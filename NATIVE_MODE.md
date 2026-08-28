# Native-mode deterministic parallel-search experiment

## Objective

Reduce wall-clock search time without changing Button Men mechanics or making
results depend on thread scheduling. Legacy mode remains untouched and is the
mechanics and compatibility oracle.

The first experiment is successful only if native mode is:

- deterministic for a complete replay key;
- independent of worker count and task completion order;
- mechanically legal under focused and property tests;
- measurably faster on `bmai_in`, `bmsim_in`, `bug11_in`, and `bug16_in`;
- no weaker than the legacy baseline within a declared statistical interval.

## Proposed boundary

Parallelize independent candidate simulations, not parser operations, game
state transitions, candidate enumeration, culling decisions, or final move
selection. The coordinator retains canonical candidate order and is the only
component allowed to update aggregate scores or cull candidates.

Each simulation receives a deterministic task key containing:

1. replay/search version;
2. root seed and decision sequence number;
3. canonical candidate index;
4. evaluation batch and simulation index.

That key initializes an independent native RNG stream. A simulation may consume
a variable number of draws without shifting another simulation's stream.
Workers return immutable results. The coordinator reduces them in task-key
order using the existing score and tie-breaking rules. Worker count, scheduling,
and completion order are therefore absent from the result.

## Experiment sequence

1. Add a native replay key and deterministic stream derivation with known-answer
   tests. Do not add threads yet.
2. Run native search sequentially with one independent stream per simulation.
   Record the expected deterministic actions and score summaries.
3. Introduce a bounded worker pool using scoped standard-library threads. Avoid
   a dependency until measurements demonstrate a need for one.
4. Prove identical native results with 1, 2, and available-CPU worker counts,
   including randomized scheduling delays in test-only orchestration.
5. Benchmark the four representative fixtures and record CPU time, wall time,
   peak memory, worker count, version, and complete replay key.
6. Run paired native-versus-legacy matches with swapped player positions and
   seeds. Define the sample size and confidence interval before inspecting the
   result.

## Explicit non-goals for the first experiment

- no changes to legacy RNG, enumeration, simulation counts, or output;
- no parallel mutation of a shared game or RNG;
- no nondeterministic floating-point accumulation;
- no work stealing whose order affects culling;
- no claim that equal fixture actions imply equal playing strength.

The experiment may be discarded without migration because native mode is
opt-in and has no compatibility promise before a separately versioned release.
