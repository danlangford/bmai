// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::cell::Cell;

thread_local! {
    static NATIVE_WORKER_ACTIVE: Cell<bool> = const { Cell::new(false) };
}

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
