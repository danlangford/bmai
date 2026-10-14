# Fixture parity benchmark

Measured on 2026-08-23 on macOS. The C++ executable was built in Release mode
from `main` at `1fcb826`; Rust was built with `cargo build --release`. Each
meaningful fixture was run once, sequentially, with wall time measured by
`/usr/bin/time -p` and no competing BMAI processes.

| Fixture | C++ | Rust | Rust / C++ |
|---|---:|---:|---:|
| `Insult_in.txt` | <0.01s | <0.01s | n/a |
| `SurrenderDefault-Pass-in.txt` | <0.01s | <0.01s | n/a |
| `SurrenderOff-Attack-in.txt` | <0.01s | <0.01s | n/a |
| `SurrenderOff-Pass-in.txt` | <0.01s | <0.01s | n/a |
| `SurrenderOn-Attack-in.txt` | <0.01s | 0.01s | n/a |
| `SurrenderOn-Pass-in.txt` | <0.01s | <0.01s | n/a |
| `Value1_in.txt` | <0.01s | <0.01s | n/a |
| `Value2_in.txt` | <0.01s | <0.01s | n/a |
| `bmai_in.txt` | 5.99s | 20.42s | 3.41x |
| `bmsim_in.txt` | 17.19s | 71.96s | 4.19x |
| `bug11_in.txt` | 34.58s | 146.18s | 4.23x |
| `bug16_in.txt` | 163.89s | 677.68s | 4.13x |
| `bug55_a_in.txt` | <0.01s | <0.01s | n/a |
| `bug55_b_in.txt` | <0.01s | <0.01s | n/a |
| `test_in.txt` | <0.01s | <0.01s | n/a |

The four expensive rows execute the same search policies in both languages;
there are no fixture-specific direct-action paths. Material protocol output and
match results were identical. In addition, instrumented seeded comparisons
matched rollout boundaries, move counts, culling results, candidate scores, and
terminal RNG seeds. In the largest reserve search (`bug16_in.txt`), all five
candidates matched individually across 8,449,351 simulations.

Those absolute Rust measurements predate the structural optimization work and
are retained as the original baseline. A same-machine paired study on
2026-08-27 measured the individual retained changes using user CPU time:

| Change | `bmai_in` | `bmsim_in` | `bug11_in` | `bug16_in` |
|---|---:|---:|---:|---:|
| stack-backed temporary die indices | about 21–22% faster | 13.6% faster | 17.2% faster | 30.8% faster |
| direct same-length simulation restoration | 9.1% faster | 3.2% faster | 3.4% faster | 10.0% faster |
| fat LTO versus Thin LTO | 7.1% faster | 0.5% slower | 0.6% slower | 19.2% faster |

Candidate-list reuse was tested and rejected because it regressed `bug16_in`
user CPU by about 18%. PGO improved its three training fixtures by roughly
5–9% but was also rejected because the untrained `bug16_in` case regressed by
about 1.4%. Exact all-fixture material output and RNG fingerprints passed after
the retained changes; that gate evidence lived in `PARITY.md`, which git
history keeps.

## 0.1.0 release artifact comparison

Measured on 2026-08-28 on an Intel Mac using the x86_64 slices from GitHub
Actions. The C++ reference was the universal macOS Release artifact from
Konstant PR #82 at `4813530`; Rust was the macOS x86_64 Release artifact at
`f2f56db`. All 24 `*in*.txt` fixtures had materially identical output.

| Fixture | C++ | Rust | Rust / C++ |
|---|---:|---:|---:|
| `bmai_in.txt` | 6.23s | 7.30s | 1.17x |
| `bmsim_in.txt` | 18.19s | 19.16s | 1.05x |
| `bug11_in.txt` | 43.94s | 46.13s | 1.05x |
| `bug16_in.txt` | 139.95s | 226.90s | 1.62x |
| all 24 fixtures | 209.00s | 300.16s | 1.44x |

The aggregate difference is dominated by `bug16_in.txt`; the other meaningful
searches are within 5–17% of C++. Sub-10ms protocol fixtures are omitted from
the table because process startup dominates their ratios.

## Native deterministic parallel-search experiment

Measured on 2026-08-28 on the same Intel Mac at `3640c4c` with Rust 1.98.0 and
the release profile. `scripts/benchmark_native.sh` captured raw protocol output,
wall/user/system time, peak resident memory, build metadata, and SHA-256 hashes.
The native replay used stream version `bmair-native-stream-v1`; each fixture's
existing seed and search settings were retained. Output after removing only the
reported `Setting native workers to N` line was byte-identical between one and
eight workers for all four fixtures.

| Fixture | 1 worker | 8 workers | Speedup | 1-worker RSS | 8-worker RSS |
|---|---:|---:|---:|---:|---:|
| `bmai_in.txt` | 6.24s | 1.81s | 3.45x | 1.0MB | 1.3MB |
| `bmsim_in.txt` | 18.27s | 10.59s | 1.73x | 1.5MB | 2.0MB |
| `bug11_in.txt` | 43.06s | 12.01s | 3.59x | 1.3MB | 1.9MB |
| `bug16_in.txt` | 227.63s | 78.75s | 2.89x | 496.5MB | 1859.0MB |

Eight workers increase aggregate CPU consumption because each culling batch
creates scoped workers and each task owns independent simulation state. The
largest reserve search is the limiting case: its 2.89x wall-time improvement
costs 3.74x peak RSS and 2.45x aggregate CPU time. Native mode therefore keeps
the default at one worker. Higher counts are explicitly opt-in until simulation
state reuse or a persistent bounded worker pool reduces this overhead.

## 0.4.0 streaming-stdin comparison

Measured sequentially on 2026-08-29 on the same Intel Mac. The 0.3.0 side is
the published macOS x86_64 Release artifact (`8ffdc1b`, Rust 1.97.1); the 0.4.0
side is a local Release build from `8ffdc1b` plus the streaming-stdin change
using Rust 1.98.0. Each fixture was run once in file-argument mode and once via
stdin redirected to EOF. Times are observational rather than a release gate.

| Fixture | 0.3 file | 0.4 file | 0.3 stdin | 0.4 stdin | stdin change |
|---|---:|---:|---:|---:|---:|
| `bmai_in.txt` | 6.86s | 6.72s | 6.72s | 6.78s | +0.9% |
| `bmsim_in.txt` | 18.95s | 17.36s | 17.44s | 17.41s | -0.2% |
| `bug11_in.txt` | 43.44s | 42.06s | 43.31s | 42.14s | -2.7% |
| `bug16_in.txt` | 241.22s | 245.46s | 236.47s | 231.31s | -2.2% |

User CPU time followed wall time closely on the first three fixtures. The
`bug16_in.txt` runs had 200.74–210.13s user and 27.91–31.60s system time, which
shows that its few-percent spread is ordinary run variance. There is no
measurable streaming penalty in this sample: framing and flushing happen only
around top-level commands, while AI search dominates runtime. Peak RSS was not
recorded because sandboxed macOS `/usr/bin/time -l` could not access
`kern.clockrate`.

## 0.4.1 legacy diagnostic sanity check

Measured once on 2026-08-29 on the same Intel Mac using `bmai_in.txt` and
Release builds. Published 0.4.0 took 7.60s wall / 7.03s user; local 0.4.1 took
6.24s wall / 6.22s user. The difference is ordinary single-run variance, not a
claimed speedup. The change records two already-computed evaluation scalars
and formats one line only after the top-level search completes, so it adds no
work to simulation, branching, culling, or RNG consumption.

## 0.18.0 search speedups

Measured on 2026-10-03 on an Intel Core i7-7820HQ Mac (load average about 4,
from other work). Each fixture ran once per binary, back to back, with user CPU
time from the shell's `time`. The baseline was 0.17.0 plus the test-only DSL
commits (`8561427`); 0.18.0 adds the four retained changes together: a reused
per-thread rollout game, set-bit `DieIndexSet` iteration, cached `OptimizeDice`
keys, and rotating captured dice into place. Output and RNG fingerprints were
identical on every golden fixture.

| Fixture | Baseline | 0.18.0 | Speedup |
|---|---:|---:|---:|
| `bmai_in.txt` | 9.51s | 5.41s | 1.76x |
| `bmsim_in.txt` | 25.24s | 14.84s | 1.70x |
| `bug11_in.txt` | 62.46s | 37.61s | 1.66x |
| `bug16_in.txt` | 294.23s | 208.63s | 1.41x |

In native mode on `bmai_in.txt`, user CPU fell from 10.71s to 6.66s with one
worker and from 17.18s to 10.41s with `workers auto`.

## 0.25.0 search speedups

Measured on 2026-10-07 on an idle Intel Celeron G5905T (2 cores, Debian 13),
with nothing else running. The input was 100 of BMAIBagels' logged fight
decisions, run one after another with their logged settings (native mode,
`workers auto`, Monte Carlo ply 1 with 4000/200/16000, `endgame 4`). Each
binary ran three times; the figure is total CPU (user plus system) across
all 100, and runs agreed within 0.02s. Each change was chosen from a `perf`
profile, and output was byte-for-byte identical after every one.

| Change | CPU | Saving |
|---|---:|---:|
| 0.24.0 | 8.93s | |
| Solver keys from packed fields, not debug text | 6.31s | 29% |
| Stratified roll without a 128-bit division | 6.18s | 2% |
| Skip Mighty and Weak resizing for other dice | 6.06s | 2% |
| Skip skill hooks when no die has the skill | 5.81s | 4% |
| Two fewer divisions in the stratified roll | 5.75s | 1% |

Tried and dropped: sorting dice through an index array instead of swapping
them was 2% slower, since copying the dice back cost more than the swaps.

## 0.27.0 gauntlet thread contention

Measured on 2026-10-08 on an 18-core Apple M5 Pro (macOS 26), every core in
use, against 0.26.1. Before the change, Hammer spent most of its time in the
kernel: `sample` showed its threads waiting on the lock inside `getenv`,
which every endgame-solver replay called twice to read the RNG trace
settings. Gauntlet output was byte-for-byte identical after the change, as
was the `BMAIR_TRACE_RNG_HASH` trace.

| Gauntlet, default field | 0.26.1 | 0.27.0 |
|---|---:|---:|
| Hammer, 20/5, 500 games: wall | 78.8s | 16.7s |
| Hammer, 20/5, 500 games: user + system | 331s + 953s | 252s + 1.1s |
| `dk(1) k(V) k(V) k(V) dmMH(4)`, 20/5, 500 games: wall | 7.6s | 7.8s |
| Hammer, default `montecarlo`, 16 games: wall | 27.0s | 26.9s |

Hammer is `(6) (12) (20) (20) (X)`, and 20/5 is `--engine "montecarlo
max_sims=20 min_sims=5"`. The Hammer 20/5 rows are single runs. The last two
rows average alternating runs and are within noise, so neither button got
slower. Hammer against Vincent alone at 20/5 (72 games, one run each) shows
the scaling in wall time. Even on one thread, the reads took about a fifth
of the time:

| Threads | 0.26.1 | 0.27.0 |
|---:|---:|---:|
| 1 | 3.05s | 2.45s |
| 6 | 1.96s | 0.73s |
| 18 | 3.24s | 0.45s |

## 0.37.0 search speedups

Measured on 2026-10-14 on an 18-core Apple M5 Pro (macOS 26.6) with Rust
1.97.1 and the release profile. Other work kept the load average near 3, so
the figures are instructions retired, as `/usr/bin/time -l` reports them,
which load hardly changes. Each fixture ran with `workers 1` as its first
line, twice per binary, and the tables give the mean. Every run printed the
golden `RNG_HASH` and `SEARCH_HASH`.

Each change alone on 0.35.0, in billions of instructions:

| Change | `bmsim_in.txt` | `bug16_in.txt` |
|---|---:|---:|
| 0.35.0 | 323.1 | 904.1 |
| Swing settings cached as indices | 322.5 (−0.2%) | 786.1 (−13.1%) |
| Mover put first in place | 317.0 (−1.9%) | 898.7 (−0.6%) |
| Quick's attack list reused | 310.7 (−3.8%) | 880.0 (−2.7%) |
| No re-sort in score-only look aheads | 301.2 (−6.8%) | 855.1 (−5.4%) |
| All four | 283.2 (−12.3%) | 703.1 (−22.2%) |

Without the swing cache, `bug16_in.txt` counts vary by up to 1.2% between
runs of one binary: each simulated preround built a list of about a million
settings at a quarter kilobyte each, and a run faulted in 3.8 million pages.
With the cache, repeat runs agree within 0.1%. The second change's
`bug16_in.txt` figure is inside that noise; added last to the other three,
it saved 1.2% there and 2.2% on `bmsim_in.txt`.

All four on 0.36.0, the release they build on:

| | 0.36.0 | 0.37.0 | Change |
|---|---:|---:|---:|
| `bmsim_in.txt` instructions | 322.7G | 282.8G | −12.4% |
| `bmsim_in.txt` cycles | 54.4G | 45.1G | −17.0% |
| `bmsim_in.txt` user time | 12.45s | 10.37s | −16.7% |
| `bug16_in.txt` instructions | 901.3G | 699.8G | −22.4% |
| `bug16_in.txt` cycles | 247.6G | 135.5G | −45.3% |
| `bug16_in.txt` user time | 51.91s | 30.91s | −40.5% |

Cycles fell further than instructions on `bug16_in.txt` because most of what
the swing cache removed was memory traffic.

Tried and dropped: a per-thread pool of scratch games, so that nested
searches and each native simulation reuse one instead of cloning the game.
Every version lost somewhere. Holding the games inline cost `bug16_in.txt`
1.4% more instructions, because Quick takes a scratch game at every playout
move and fetching one got slower; boxing them cut that to 0.2%. Keeping
Quick's single slot in front of a boxed pool saved 1.6% of `bmsim_in.txt`
instructions but took 11% more cycles, for a reason the counts do not show.
