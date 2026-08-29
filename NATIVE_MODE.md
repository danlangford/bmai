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

1. **Implemented:** Add a native replay key and deterministic stream derivation
   with known-answer tests. No threads or legacy-search changes were added. See
   `src/native.rs`; its versioned `bmair-native-stream-v1` contract deliberately
   excludes worker identity.
2. **Implemented:** Run native search sequentially with one independent stream
   per simulation. Every direct `getaction` phase has a deterministic input and
   expected-output fixture, and a subprocess fixture covers replay sequencing
   through a complete `playgame` command.
3. **Implemented:** Introduce bounded scoped standard-library workers without a
   dependency. The ordered task coordinator drives fight, reserve,
   preround/swing, Chance, and Focus evaluations. Match-driven native searches
   use the same worker count.
4. **Implemented:** Prove identical native results with 1, 2, and available-CPU
   worker counts. Every direct phase has protocol-output equivalence coverage;
   fight and the post-round reserve path have complete-match coverage; and the
   coordinator has test-only scheduling perturbation coverage.
5. **Implemented:** Benchmark the four representative fixtures and record CPU
   time, wall time, peak memory, worker count, version, and complete replay key.
   See `BENCHMARKS.md`; eight workers improve wall time by 1.73x to 3.59x, with
   the large reserve case's 1.86GB peak RSS recorded as a limitation.
6. **Implemented:** Run paired native-versus-legacy matches with swapped player
   positions and seeds. `STRENGTH.md` preregistered the sample and interval,
   then recorded noninferiority for the declared fixed matchup.

## Validation evidence

On 2026-08-28, the completed implementation passed 108 upstream PR #82 C++
tests (three expected upstream skips), all Rust default tests, the parser error
differential, the 487.65-second all-fixture material differential, the
153.57-second representative raw RNG stream differential, and the 470.82-second
exhaustive RNG count/fingerprint differential. The C++ oracle was PR #82 commit
`4813530bca328231535c2c0853a7b239be064794`.

After adding complete-match reserve dispatch and explicit tied-round standings,
the affected gates were repeated: 72 default Rust tests passed (three
intentional long-running ignores), the all-fixture material differential passed
in 459.81 seconds, and the exhaustive RNG fingerprint differential passed in
521.08 seconds against the same oracle.

Worker closures never print diagnostics. The coordinator marks scoped worker
threads, and trace settings resolve to a quiet configuration there. Candidate
scores and any diagnostics are reduced and emitted only in canonical order on
the coordinating thread. Legacy logging and protocol output are unchanged.

## Explicit non-goals for the first experiment

- no changes to legacy RNG, enumeration, simulation counts, or output;
- no parallel mutation of a shared game or RNG;
- no nondeterministic floating-point accumulation;
- no work stealing whose order affects culling;
- no claim that equal fixture actions imply equal playing strength.

The experiment may be discarded without migration because native mode is
opt-in and has no compatibility promise before a separately versioned release.
