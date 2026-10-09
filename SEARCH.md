# Deterministic search and replay

<!--
SPDX-License-Identifier: MIT
SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
-->

BMAIR's searches run their simulations in parallel, yet a given position and
replay key always produce the same move, whatever the worker count or the
order in which threads finish.

## How a decision searches

Every top-level decision draws a replay key from its session: the root seed
and a decision index that advances only when a Monte Carlo search runs. Quick,
Maximize, Random and immediate passes draw none, so mixing engines cannot shift
a later search's streams.

Within a decision, each simulation gets its own generator, seeded from the
replay key, the candidate's canonical index, the evaluation batch, and the
simulation index. A simulation may draw any number of times without
moving another's stream. Workers return results, and the coordinating thread
reduces them in task order with the usual scoring and tie-breaking, culls, and
picks the move. Parsing, game-state changes, candidate enumeration, culling
and move selection never run in parallel.

Levels below the top of a search, such as an opponent's reply at ply 2, draw
from their own simulation's generator in order, with no further keys.

## Workers

`workers N` sets the worker count, and `workers auto`, the default, uses the
logical CPU parallelism available to the process. Each batch clamps its count
to its number of tasks. The resolved count goes into session metadata for
performance reproduction, but it never changes a result. WASI cannot report a
core count, so WebAssembly builds resolve `auto` to 1; where threads cannot
start, as in the `wasm32-wasip1` release, every batch runs on the
coordinating thread with the same results.

## Stream partition

The current partition is `bmair-native-stream-v2`, reported in each JSONL
response's `replay` field. A simulation's first consecutive bounded draws are
stratified across mixed-radix outcome cells by simulation index, so each
complete block of simulations covers every face of an ordinary reroll once,
and every combination of consecutive bounded draws. Once culling leaves one
candidate, search keeps evaluating it through the configured budget, so a
larger budget always buys a better estimate. Changing the derivation means a
new partition identifier, never a silent change under the old one.

`report_sims N` first finishes the ordinary search, then evaluates only the
chosen fight move for exactly `N` fresh samples on a coordinate outside the
candidate range. It does not advance the decision index, so turning it on
cannot change the action or a later decision.

## Generator

The generator is the Park-Miller minimal-standard LCG (multiplier 16807,
modulus 2^31-1) with BMAI's handling of small and zero seeds, so `minstd`
alone does not identify it. Its replay identifier is
`bmai-park-miller-16807-v1`, and `rng park-miller` (alias `rng legacy`)
selects it. Selecting a generator never reseeds it.

## Replay contract

A recorded game or search can be replayed from:

- the BMAIR version or commit;
- the generator's replay identifier and resolved seed;
- the stream partition and, for each decision, its root seed and decision
  index (JSONL's `replay`; `session.native_decision_index` is the next one);
- each player's engine and its settings: ply, minimum and maximum
  simulations, maximum branch, and Turbo accuracy.

The worker count is worth recording for performance, not for the result.
