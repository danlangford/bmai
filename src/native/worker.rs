// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::cell::Cell;

thread_local! {
    static NATIVE_WORKER_ACTIVE: Cell<bool> = const { Cell::new(false) };
}

/// WebAssembly without shared memory cannot start threads. No worker count
/// changes a result, so those builds do the same work on one thread.
const THREADS_AVAILABLE: bool = !cfg!(all(target_family = "wasm", not(target_feature = "atomics")));

pub(crate) fn native_worker_active() -> bool {
    NATIVE_WORKER_ACTIVE.get()
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
    if worker_count == 1 {
        return tasks.into_iter().map(evaluate).collect();
    }
    if !THREADS_AVAILABLE {
        // Traces stay as quiet as they would be inside real workers.
        let outer = NATIVE_WORKER_ACTIVE.replace(true);
        let results = tasks.into_iter().map(evaluate).collect();
        NATIVE_WORKER_ACTIVE.set(outer);
        return results;
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
                    NATIVE_WORKER_ACTIVE.set(true);
                    let completed = assignment
                        .into_iter()
                        .map(|(index, task)| (index, evaluate(task)))
                        .collect::<Vec<_>>();
                    NATIVE_WORKER_ACTIVE.set(false);
                    completed
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
/// Builds without threads run a single copy, so each copy must keep taking
/// work from a shared queue until none is left.
pub(crate) fn drain_with_workers<R, F>(count: usize, work: F) -> Vec<R>
where
    R: Send,
    F: Fn() -> R + Sync,
{
    if count <= 1 || !THREADS_AVAILABLE {
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
