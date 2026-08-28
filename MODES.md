# BMAIR execution and RNG modes

## Independent choices

Execution mode and RNG algorithm are independent configuration axes.

- `mode legacy` (alias `parity`) is the executable C++ compatibility contract.
  Candidate enumeration, search decisions, state transitions, and RNG
  consumption must continue to satisfy `PARITY.md`.
- `mode native` is the opt-in Rust-native evolution contract. Fight, reserve,
  preround, Chance, and Focus searches give each top-level candidate simulation
  an independent, deterministically derived RNG stream. Match-driving commands
  assign the same replay keys across every internal BMAI decision. Future
  changes may alter search, pruning, scheduling, or parallelism only behind this
  mode and their own tests.
- `rng legacy` selects `bmai-park-miller-16807-v1`. It is available to either
  execution mode and remains mandatory for exact C++ replay.

Selecting a mode never implicitly selects or reseeds an RNG. Selecting an RNG
never changes execution mode and does not reset its stream.

## Legacy RNG identity

The C++ generator is the Park-Miller minimal-standard LCG with multiplier
16807 and modulus 2^31-1. BMAI adds custom handling for small and zero seeds,
so the generic name `minstd` is not precise enough for replay metadata. Its
stable identifier is `bmai-park-miller-16807-v1`.

## Replay contract

A durable recorded game or search must include:

- execution mode and BMAIR version/commit;
- RNG replay identifier and resolved initial seed;
- AI selection and rollout policy for each player;
- ply, minimum/maximum simulations, maximum branch, and Turbo accuracy;
- any future native scheduling, worker-count, or stream-partition version.

A seed without the RNG replay identifier is not a complete replay key. Native
parallel search must version its stream partitioning and deterministic result
reduction before it can claim reproducibility.

## Native-mode acceptance criteria

Native behavior does not need byte-for-byte C++ output, but each divergence
must be opt-in, deterministic for a complete replay key, mechanically legal,
and covered by focused tests plus statistical evaluation where exact expected
actions are inappropriate. Legacy gates remain mandatory and unchanged.

Executable native wire-protocol examples live in `tests/native-fixtures/`.
They are separate from `tests/fixtures/*in*.txt` because that legacy fixture set
must remain valid input to both the C++ and Rust binaries.

## Legacy stability policy

Legacy mode is the permanent executable compatibility oracle. After the
`bmair-v0.1.0` preview it is frozen against intentional search or RNG behavior
changes. Changes are limited to upstream mechanics corrections,
parity-preserving maintenance, portability fixes, and measured optimizations
that pass the complete material-output and RNG-fingerprint gates. Intentional
AI divergence belongs exclusively behind native mode.
