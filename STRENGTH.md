# Native-mode strength experiment

<!--
SPDX-License-Identifier: MIT
SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
-->

Each section records how its runs were made. Runs marked legacy execution used
the C++-compatible search BMAIR has since removed; the strength harness now
plays natively, as `bmair gauntlet` does.

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
won that pairing by about 4.5 percentage points while taking roughly 140 times as long
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


## 0.21.0 settings sweep

### Design

Each row plays BMAIBagels' settings (`montecarlo ply=2 max_sims=100
min_sims=5 maxbranch=400`) against one change, on the six matchups in
`tests/strength/matchups.txt`, first to one round, each pair from both
seats, eight threads, legacy execution.

- `tests/strength/sweep.sh`, seeds 1 through 50 (300 pairs per row): ply 3,
  ply 1 with ten times the simulations, and one ply 2 setting at a time. Not
  preregistered; it was exploratory. One more row, ply 3 with a time limit
  equal to ply 2's measured time per decision, ran on a `time_limit`
  prototype that was not released.
- `tests/strength/ply1-width.sh`, seeds 1 through 100 (600 pairs per row): ply
  1 with simulations scaled to a quarter of, the same as, and four times ply
  2's time per decision. Ratios between `max_sims`, `min_sims`, and
  `maxbranch` match the sweep's ply 1 row, because `maxbranch / min_sims`
  also caps Fire candidates. The decision rule was stated before results:
  ply 1 replaces ply 2 only if the same-time row's interval lies wholly
  below 0.5.

### Result

Run on 2026-10-04 from `182b829`, the code at tag `prototype/time-limit`. The
first contestant is always ply 2 above, so a score below 0.5 favours the change.

| Change | Pairs | Ply 2 score (95% CI) | Ply 2 ms/decision | Change ms/decision |
|---|---:|---|---:|---:|
| ply 3 | 300 | 0.477 (0.437–0.516) | 993 | 65253 |
| ply 3, `time_limit=0.993` | 300 | **0.633 (0.596–0.671)** | 931 | 779 |
| ply 1, `1000/50/4000` | 300 | 0.502 (0.462–0.541) | 944 | 62 |
| `maxbranch=800` | 300 | 0.463 (0.423–0.504) | 975 | 3045 |
| `maxbranch=200` | 300 | **0.542 (0.503–0.581)** | 986 | 285 |
| `max_sims=200` | 300 | 0.482 (0.451–0.513) | 1008 | 1026 |
| `min_sims=20` | 300 | 0.488 (0.448–0.528) | 1004 | 1017 |
| `cull=off` | 300 | 0.468 (0.427–0.509) | 1004 | 1412 |
| `playout=maximize` | 300 | 0.527 (0.486–0.567) | 990 | 948 |
| ply 1, `4000/200/16000` | 600 | 0.512 (0.486–0.539) | 972 | 258 |
| ply 1, `16000/800/64000` | 600 | 0.499 (0.472–0.527) | 1023 | 1084 |
| ply 1, `64000/3200/256000` | 600 | 0.515 (0.487–0.543) | 985 | 4135 |

Ply 1 settings are `max_sims/min_sims/maxbranch`.

- Ply 3 cut off at ply 2's time loses clearly, and ply 3 given 65 times the
  time does not measurably win. A time limit could only cap rare slow
  decisions, not buy depth, so the prototype was set aside.
- Ply 1 ties ply 2 from 62 ms to 4.1 s per decision, a 65-fold range of
  simulations. The same-time row sits at 0.499, so under the stated rule ply
  1 does not replace ply 2. It does match ply 2 at a quarter of the time or
  less, where the 0.20.0 ladder's ply 1 at ply 2's settings lost narrowly.
- Halving `maxbranch` loses narrowly; no other single setting change is
  distinguishable from the current settings.
- Neither depth nor simulation count moves strength much past these
  settings, so the playouts that score every simulation are the likelier
  limit.

Caveats:

- Twelve comparisons with no correction for multiplicity; only the timed ply
  3 row would survive one. The `maxbranch=200` row would not.
- Every row measures strength against BMAIBagels' own settings, not against
  people, and only on these six matchups.
- Intervals of about ±0.027 (600 pairs) or ±0.04 (300 pairs) cannot resolve
  smaller differences.
- Legacy execution is single-threaded per game; the bot runs native mode
  across all cores, which these rows do not test.
- Milliseconds per decision are wall time under eight-way concurrency on a
  shared machine.

## 0.23.0 defaults and Quick experiments

All runs use the harness's paired seats, eight threads unless noted, and the
first contestant's score, so below 0.5 favours the second. Builds and seeds
are each run's `build.txt` and `seeds.txt`. Fire overshooting follows each
build's default: off before 0.23.0, on from 511d37b.

### Many-option matchups

`tests/strength/matchups-options.txt`: six pairings with Turbo, Fire, Poison,
Value, and Trip dice, three from games 121248, 121297, and 121298. Build
3b09a01 (Fire overshooting off), seeds 201 through 300, 600 pairs per row,
against BMAIBagels' ply 2 (`max_sims 100`, `min_sims 5`, `maxbranch 400`):

| Challenger | Ply 2 score (95% CI) | Ply 2 ms | Challenger ms |
|---|---|---:|---:|
| ply 1, `16000/800/64000` | 0.509 (0.485–0.533) | 4292 | 3734 |
| ply 1, `4000/200/16000` | **0.463 (0.438–0.488)** | 3273 | 729 |
| ply 2, `400/20/1600` | **0.448 (0.422–0.473)** | 3226 | 35680 |

With the classic-matchup tie at the same setting (0.512, 0.486–0.539), ply 1
at `4000/200/16000` became BMAIBagels' setting and BMAIR's default: no worse on
classic buttons, better on many-option ones, and a quarter of ply 2's time.
Four times ply 2's budget also wins here, at about fifty times that cost. A
fourth row, that budget on classic matchups, was stopped because no result
could have changed the choice.

### Quick changes, none adopted

Each change ran as Monte Carlo's playout at ply 1 `4000/200/16000` against the
C++ QAI's, on both matchup sets. A screen of 150 pairs per row (seeds 401
through 425) picked candidates; 600-pair confirmations on fresh seeds decided.

| Change | Many-option | Classic | Playout time |
|---|---|---|---|
| Charge the opponent's best reply capture (`careful`, head to head, 600 pairs, 317ec18) | 0.469 (0.451–0.487) | 0.471 (0.454–0.487) | about 10 times |
| Rules-aware reroll value (Konstant, Maximum, Mighty, Weak, Mood, Mad, Twin; uncommitted) | 0.480 (0.444–0.516) | 0.500, identical | +20%, unoptimized |
| Random bonus 0 to 0, 1, or 2 instead of 4 | 0.493 to 0.520 | 0.473 to 0.487 | same |
| Exposure: charge my most valuable Power-capturable die (screen) | 0.437 (0.384–0.489) | 0.493 (0.436–0.550) | +11% |
| Exposure, confirmed (seeds 501 through 600) | 0.494 (0.469–0.519) | 0.516 (0.489–0.543) | +10% |
| Danger: reward capturing dice that threaten mine, or Fire and Stinger (screen) | 0.483 (0.431–0.536) | 0.480 (0.426–0.534) | same |
| Value once: skip the reroll guess for Value attackers (screen) | 0.483 (0.462–0.505) | 0.500, identical | same |
| Fire cost: charge each pip of Fire turned down (screen) | 0.507 (0.470–0.544) | 0.500, identical | same |
| Exposure, danger, and value once (seeds 501 through 600) | 0.465 (0.440–0.490) | 0.510 (0.483–0.537) | +15% |
| Danger and value once, confirmed (seeds 601 through 700) | 0.495 (0.470–0.520) | 0.512 (0.484–0.540) | +4% |
| Random legal opponent swing sizes instead of every minimum (d25ae73, four threads) | 0.493 (0.451–0.536) | 0.533 (0.478–0.589) | same |
| Chance and Focus for initiative in simulated rounds (d25ae73, four threads) | 0.500 (0.481–0.519) | 0.500, identical | same |

`careful` beat the C++ QAI head to head but was too slow for a playout, so its
playout rows were stopped. Exposure's screen result did not survive
confirmation, and the three-change bundle's narrow win did not survive without
exposure, so it is treated as chance after this many comparisons. The code
was removed; the Tabled list in CHANGELOG.md records the ideas.

Caveats: screens of 150 pairs resolve about ±0.04 and confirmations about
±0.027, so smaller effects read as ties; playout changes are diluted across
thousands of simulated moves; and every run pits Monte Carlo against Monte
Carlo over single rounds on twelve matchups.

## 0.24.0 endgame solver

Fresh seeds 901 through 950, 300 pairs per matchup set, Monte Carlo at
BMAIBagels' settings (ply 1, `4000/200/16000`) against the same with
`endgame 4`, surrender allowed. With `BMAIR_TRACE_ENDGAME`, every decision the
solver took also ran Monte Carlo and scored its choice exactly.

| | Classic | Many-option |
|---|---:|---:|
| Endgame decisions (4 dice or fewer) | 523 | 428 |
| Already certainly lost | 215 | 192 |
| Monte Carlo chose a worse move | 4 (0.8%) | 7 (1.6%) |
| Its mean and largest shortfall | 0.019, 0.039 | 0.046, 0.150 |
| Monte Carlo surrendered a round it could win | 13 | 2 |
| Mean and largest chance it gave up | 0.063, 0.102 | 0.045, 0.052 |
| Harness score of plain Monte Carlo (95% CI) | 0.500 (0.495–0.505) | 0.495 (0.489–0.501) |

Monte Carlo already plays four-die endgames well, so the solver moves the
harness score little; its value is exact play and exact reported odds in
those positions, at about 10% more time per decision. Its surrenders in live
rounds led to surrender's new default of off. The harness timings above
include the trace's extra Monte Carlo searches, so they overstate the cost.
