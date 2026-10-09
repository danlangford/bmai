// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::sync::OnceLock;

/// No worker count changes a result, so where threads can't start the same
/// work runs on one thread. Stable Rust gives a threaded WebAssembly build the
/// same `cfg` as an unthreaded one, and a host may refuse to spawn, so
/// WebAssembly asks once at run time.
fn threads_available() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    !cfg!(target_family = "wasm")
        || *AVAILABLE.get_or_init(|| {
            std::thread::Builder::new()
                .spawn(|| {})
                .is_ok_and(|probe| probe.join().is_ok())
        })
}

/// Results follow task position, never completion order, so output is
/// deterministic.
pub(crate) fn ordered_parallel_map<T, R, F>(tasks: Vec<T>, workers: usize, evaluate: F) -> Vec<R>
where
    T: Send,
    R: Send,
    F: Fn(T) -> R + Sync,
{
    let worker_count = workers.max(1).min(tasks.len().max(1));
    if worker_count == 1 || !threads_available() {
        return tasks.into_iter().map(evaluate).collect();
    }

    let mut assignments = (0..worker_count)
        .map(|_| Vec::new())
        .collect::<Vec<Vec<(usize, T)>>>();
    for (index, task) in tasks.into_iter().enumerate() {
        assignments[index % worker_count].push((index, task));
    }

    let mut completed = std::thread::scope(|scope| {
        let handles = assignments
            .into_iter()
            .map(|assignment| {
                let evaluate = &evaluate;
                scope.spawn(move || {
                    assignment
                        .into_iter()
                        .map(|(index, task)| (index, evaluate(task)))
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("native worker panicked"))
            .collect::<Vec<_>>()
    });
    completed.sort_unstable_by_key(|(index, _)| *index);
    completed.into_iter().map(|(_, result)| result).collect()
}

/// Runs up to `count` copies of `work` at once and returns each copy's result.
/// Where threads can't start, a single copy runs, so each copy must keep
/// taking work from a shared queue until none is left.
pub(crate) fn drain_with_workers<R, F>(count: usize, work: F) -> Vec<R>
where
    R: Send,
    F: Fn() -> R + Sync,
{
    if count <= 1 || !threads_available() {
        return vec![work()];
    }
    std::thread::scope(|scope| {
        let handles = (0..count).map(|_| scope.spawn(&work)).collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("a worker panicked"))
            .collect()
    })
}
