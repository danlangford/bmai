# Native-mode strength experiment

<!--
SPDX-License-Identifier: MIT
SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
-->

## Preregistered design

This design was committed before inspecting match results. Native and legacy
BMAI cannot currently oppose each other inside one process because execution
mode is a parser-wide setting. Each mode will therefore play the same QAI
opponent under paired initial conditions.

- Matchup: the two button sets from `tests/fixtures/bmsim_in.txt`.
- Search settings: ply 1, minimum 10 simulations, maximum 20 simulations, and
  maximum branch 500.
- Seeds: integers 1 through 200.
- Positions: two strata. The second swaps both button sets and which player is
  controlled by BMAI, so each BMAI button set occupies both player slots.
- Sample size: 400 games per execution mode, 800 total.
- Native scheduling: eight workers using `bmair-native-stream-v1`.
- Legacy scheduling: one worker; the setting has no effect on legacy search.

For every seed-position pair, record native and legacy BMAI wins as zero or
one and calculate their paired difference. Report the mean difference and a
two-sided 95% normal interval using the sample standard deviation of paired
differences. Native is declared noninferior only when the interval's lower
bound is greater than -0.10. The ten-percentage-point margin and normal
approximation are deliberately modest for this first experiment; failure is
reported as inconclusive or weaker, never rounded into a pass.

This measures performance against one fixed opponent and matchup. Passing does
not establish universal playing-strength equivalence.

## Result

Run on 2026-08-28 from `7463e34` using the release profile. Every cell produced
the preregistered 200 games:

| Mode | Original position | Swapped position | Total |
|---|---:|---:|---:|
| Legacy BMAI wins | 151/200 | 149/200 | 300/400 |
| Native BMAI wins | 150/200 | 145/200 | 295/400 |

The paired native-minus-legacy difference was -0.0125. Its preregistered 95%
interval was [-0.0732, 0.0482]. The lower bound exceeds -0.10, so this experiment
declares native mode noninferior for this fixed matchup and QAI opponent. The
point estimate is not evidence that native is stronger; it was 1.25 percentage
points lower. Broader matchup coverage is follow-up work rather than a claim
supported by this result.

## 0.20.0 engine ladder

### Preregistered design

Committed before any ladder results were inspected.

- Contestants: `random`, `maximize`, `quick`, and `montecarlo` at ply 1 and
  ply 2. Monte Carlo uses BMAIBagels' settings at the time (`max_sims=100
  min_sims=5 maxbranch=400`, culling on), which have never been tuned.
- Matchups: the six button pairs in `tests/strength/matchups.txt`, first to
  one round, so every game is one independent round.
- Seeds 1 through 50 for every matchup: 300 pairs per pairing, each played
  from both seats (600 games), every contestant against every other.
- Legacy execution, eight threads; milliseconds per decision are wall time on
  the deciding thread.
- Command, from `6c02b14`:
  `ladder --engine random --engine maximize --engine quick --engine
  "montecarlo ply=1 max_sims=100 min_sims=5 maxbranch=400" --engine
  "montecarlo ply=2 max_sims=100 min_sims=5 maxbranch=400" --seeds 1..50
  --threads 8`.

For each pairing report the first contestant's mean paired score (0, 0.5, or
1 per pair) with a two-sided 95% normal interval. The ladder is confirmed
for a pair of adjacent contestants only when the interval excludes 0.5 in
the expected direction (`random` < `maximize` < `quick` < ply 1 < ply 2).
An interval containing 0.5 is reported as inconclusive, never as a tie or a
pass. Ply 3 is not laddered: one ply-3 decision at these settings (`bmai_in.txt`)
takes about 5 s, against about 0.8 s at ply 2 and 5 ms at ply 1.

### Result

Run on 2026-10-03 from `6c02b14` with the command above; every pairing played
its 300 pairs. Monte Carlo rows use `max_sims=100 min_sims=5 maxbranch=400`.

| First | Second | Pairs | First wins | First score (95% CI) | First ms/decision | Second ms/decision |
|---|---|---:|---:|---|---:|---:|
| `random` | `maximize` | 300 | 193/600 | 0.322 (0.284–0.359) | 0.00 | 0.01 |
| `random` | `quick` | 300 | 113/600 | 0.188 (0.157–0.219) | 0.00 | 0.00 |
| `random` | `montecarlo ply=1` | 300 | 100/600 | 0.167 (0.139–0.195) | 0.00 | 5.74 |
| `random` | `montecarlo ply=2` | 300 | 84/600 | 0.140 (0.111–0.169) | 0.01 | 1160.44 |
| `maximize` | `quick` | 300 | 210/600 | 0.350 (0.316–0.384) | 0.01 | 0.01 |
| `maximize` | `montecarlo ply=1` | 300 | 160/600 | 0.267 (0.231–0.302) | 0.01 | 7.06 |
| `maximize` | `montecarlo ply=2` | 300 | 143/600 | 0.238 (0.205–0.272) | 0.01 | 1282.56 |
| `quick` | `montecarlo ply=1` | 300 | 242/600 | 0.403 (0.366–0.440) | 0.01 | 7.96 |
| `quick` | `montecarlo ply=2` | 300 | 230/600 | 0.383 (0.345–0.421) | 0.01 | 1153.29 |
| `montecarlo ply=1` | `montecarlo ply=2` | 300 | 273/600 | 0.455 (0.415–0.495) | 8.30 | 1174.38 |

All four adjacent comparisons exclude 0.5, so under the preregistered rule
the ladder `random` < `maximize` < `quick` < `montecarlo ply=1` <
`montecarlo ply=2` is confirmed. Each step is larger the further apart the
contestants are, as a ladder should be.

The ply 2 step is the weakest: its interval ends at 0.495. It is not robust
to a correction for testing four steps; a Bonferroni-adjusted interval
(98.75%) for that row reaches about 0.506 and would be inconclusive. Ply 2
won that pairing by about 4.5 points while taking roughly 140 times as long
per decision (about 1.2 s against 8 ms), so whether ply 2 is worth its cost
is a question for a time-budgeted comparison, not this ladder.

Caveats, disclosed after the run:

- The intervals describe these six matchups at one round each, not Button
  Men in general, and carry no correction for testing four steps.
- Seeds 1 through 50 went to Park-Miller unmixed, and consecutive seeds roll
  nearly the same opening dice (the first d6 cycles 6, 4, 3, 1). Both seats
  of a pair saw the same skew, so the comparison is not biased, but the
  sampled positions are not a random sample. The harness now mixes seeds.
- In legacy mode Monte Carlo search draws from the dice generator, so the
  two games of a pair share buttons and seats but not dice.
- Milliseconds per decision are wall time under eight-way concurrency on a
  shared, heavily loaded machine (load averages near 100 during parts of the
  run). They describe relative cost, not latency.

