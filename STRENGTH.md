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
