# Minimax engine design

<!--
SPDX-License-Identifier: MIT
SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
-->

Status: draft, revised after review. Step 1, the endgame solver, ships in
0.24.0 (STRENGTH.md); the acceptance items it has not yet met are listed under
step 1 below.

## Why a new engine

Monte Carlo has stopped improving with effort. Ply 3, 65 times the
simulations, and a dozen changes to its Quick playouts all measured as ties
(STRENGTH.md). Its estimates are noisy where choices are many: in game 121298
it chose a Trip over a capture because 15 samples flattered the Trip.

`minimax` is a different kind of player: it looks at the opponent's best reply
and at exact dice odds instead of sampled ones, and judges positions with the
strategy published by Cheapass Games (James Ernest and contributors; eleven
articles indexed at
<https://web.archive.org/web/20150910034130/http://www.cheapass.com/bpu/bmstrat.html>).
Article numbers below refer to that index.

It is a new `Engine` beside `random`, `maximize`, `quick`, and `montecarlo`,
selected with `ai PLAYER minimax`. It shares the game rules (move generation
and `apply_attack`) and nothing else.

## Principles from the articles

| # | Principle | Use in the engine |
|---|---|---|
| 3, 4 | **Keep-threshold.** With totals A and B, the bigger side must keep about `2(A-B)/3` sides to *tie*, more to win; it assumes the smaller side ends with no dice, and article 11 excludes Option, Poison, Null, Turbo, and Mood dice and stalemates. | Swing choice, within those limits. The evaluation uses exact scoring instead (below). |
| 1 | **Highest lower number.** With two dice left and the opponent protecting a die he must keep, he will take your highest, so your *second* highest after the reroll is what counts. | Falls out of the search: exact reroll odds and the opponent's best reply. |
| 3 | **Shade.** The chance a rerolled die lands on a value the opponent cannot reach with any Power or Skill combination. | Evaluation: safety of each die, from the opponent's reach set. |
| 3 | **Change killing and the keystone.** Capture the one die that every combination threatening your critical die needs. Leave the opponent duplicates that cover fewer sums. | Falls out of a two-move search; reach sets make it cheap to evaluate. |
| 2 | **Lock.** Leave the opponent no legal attack. Keep a wide spread of values you can take. | Evaluation: reach-set size for each side; zero means locked. |
| 2 | **Take heavy dice.** | Evaluation, through points and reach. |
| 9 | **Field of attack** and **Trip odds**: `(N+1)/(2X)` when the target is at least as big, else `(2N-X+1)/(2N)`. Mixed Shadow and regular dice cover more values. | A test of exact enumeration; reach sets that count Shadow, Queer, and mixed dice. |
| 10 | **Blind, active, and hosed** Shadow dice; rounds end when both players pass; a Trip die can keep attacking a Shadow die it can never be taken by. | Evaluation of Shadow dice; the cycle policy below. |
| 8 | **Poison:** roll your big Poison die small so it gets taken; take the opponent's only when you must. | Exact scoring handles points; the search sees the reroll. |
| 6 | **Speed and Focus:** a range of sizes counters Speed; Focus can lower a die to deny a Speed attack. | Focus and initiative decisions, later. |
| 4, 5, 11 | **Swing choice** models. | An analytic swing chooser, within the models' stated limits. |
| 7 | Against smaller dice, a die built from smaller ones beats a single die with the same range and mean, for Power attacks. | Falls out of exact reroll distributions. |
| 2 | Proposed brute force for five-die positions. | The endgame solver; a prototype confirms it is tractable. |

## The core idea: sides to capture or protect

The engine plays to win the round, measured in exact score:

1. **What score gap must I close, or keep?** Capturing a die changes
   (captor minus owner) by `score(false) + score(true)`: one and a half times
   its sides for a plain die, the reverse for Poison, and the right amount for
   Value and Null dice. That holds at every point in the round for every skill,
   where the keep-threshold holds only in narrow cases.
2. **Which combinations deliver it?** The smallest sets of the opponent's dice
   I could capture, and of my own I must keep.
3. **Attack a die from a combination I need, preferring combinations with the
   fewest dice**, weighed against the risk of rerolling dice from my own
   protect combinations into the opponent's reach.

The search checks these judgments against the opponent's best reply; the
evaluation and move ordering are built from them.

## Shape of the search

A round is a stochastic game over canonical states:

- **The mover's options:** every move the existing generator offers. Pass is
  offered only when no attack is legal. Fire plans are capped at 500 and Turbo
  sizes follow `turbo_accuracy`, so "exact" means exact over the generator's
  moves.
- **Chance:** the rerolls each attack causes, every outcome with its exact
  probability (below).
- **Who moves next** comes from each outcome: `apply_attack` reports an extra
  turn (Jolt, Time and Space, Morphing), so play does not simply alternate.
- **Leaves:** an evaluation of the mover's chance to win the round, or an exact
  value from the endgame solver. Never Monte Carlo playouts.

Depth deepens within a deterministic node budget. A pass cut off partway is
discarded, and the last complete depth decides.

### Exact chance nodes

Every random draw in the rules goes through `Rng::rand_below` (the Mood and
Mad size, and each face). A scripted generator replays an attack with chosen
faces, records each draw's range, and walks the tree of draws depth first, so
draws that depend on earlier ones (a Mood die's face after its size, a Morph
reroll only after a successful Trip) are covered. Each path's probability is
the product of 1/range along it. The script fails closed: any other draw
(`rand`, `rand_f32`) panics.

Outcomes that reach the same canonical state are merged. An attack whose
replays exceed a cap (Ornery dice multiply them: five Ornery d20s make 3.2
million) is handed to Monte Carlo and counted. Later, rerolled dice can be
enumerated one at a time and combined as sorted multisets (three d20s become
1,540), and value classes can merge equivalent faces, each only with a test
that it agrees with full enumeration.

### Canonical state

One key, used both to merge outcomes and to cache solved positions: each
side's available dice sorted by every field, both scores, the side to move,
and whether the last move was a pass. Fields that matter only for restoring
the next round (original index, Radioactive products, Rage replacements) are
left out. No bucketing of values: Fire, Value dice, and Skill sums make every
point of a value matter.

### Cycles

A failed Trip captures nothing, and passing is offered only without an attack,
so a round can return to a position it has already visited (article 10's Trip
against a Shadow die). Step 1 detects a position repeating on the current line
and hands the decision to Monte Carlo, counted. A later step can solve those
positions by value iteration to a stated tolerance.

### Reproducibility

- Caches live for one decision, so a position gives the same answer cold.
- Single-threaded, or a fixed split of the root with private tables.
- Outcomes are merged in a sorted map and summed in a fixed order; no float
  sum depends on hash order.

### Ordering and pruning

Moves are ordered by the evaluation of their expected outcome, best first.
Chance nodes prune with bounds on the evaluation (*-search); every value lies
in 0 to 1.

## Evaluation

A leaf estimates the mover's probability of winning the round, from features
computed in one pass over at most twenty dice:

1. **Score gap and swing at stake:** the current difference, and what captures
   still possible could change it.
2. **Capture and protect sets:** the smallest sets that close the gap, as in
   the core idea.
3. **Reach sets:** the values each side can capture by Power (its value or
   more, or less for Shadow), Skill (subset sums, with Fire help), Speed, and
   Berserk. Bitsets, so one pass.
4. **Safety (shade):** for each die in a protect set, the chance it survives
   the opponent's reach after its reroll.
5. **Lock:** an empty reach set for the side to move.
6. **Initiative:** whose move it is.

The weights are fitted offline, not with the strength harness, which needs
600 pairs per reading: against exact solver values on positions of up to
seven dice, and by regressing recorded harness games' round results on their
positions. Each evaluation version must first predict solver values better
than the last (Brier score) before it reaches the harness.

## Exact endgame solver

When few dice remain, solve the rest of the round exactly over the
generator's moves, both players at their best, with the canonical key above.
A prototype for plain dice found 2v2 at hundreds of states, 3v3 at thousands
to tens of thousands, and 4v4 at about 780,000, so step 1 starts at four or
six dice in total. Each decision has a node budget and an outcome cap, which
also bound the memory a crafted position can take. A position beyond them, or
with a cycle, goes to Monte Carlo.

An exact answer is reported as such: BMAIR skips the `report_sims`
resampling after it, so BMAIBagels posts the exact odds.

## Decisions besides attacks

- **Attacks:** the search above.
- **Swing:** an analytic chooser from articles 4 and 11 within their limits.
  Scoring every swing by searching from the opening roll would need too many
  outcomes to be exact.
- **Reserve, auxiliary, Chance, Focus:** Monte Carlo at first, then the search
  one at a time, each measured.

## Visibility

Each decision reports the states solved, cache hits, the largest chance node,
and, if it handed the decision to Monte Carlo, why: node budget, outcome cap,
or cycle. Without the hand-off count, a solver that rarely acts would read as
a tie and look like success.

## Building it in measured steps

Each step states its acceptance criteria before its runs.

1. **Endgame solver**, a `montecarlo` setting, `endgame N`, off by default.
   Accepted when:
   - outcome enumeration sums to exactly one and matches closed forms
     (article 9's Trip odds) and sampled play;
   - solver values equal a naive, uncached solver's on hundreds of positions,
     cache on equals cache off, and runs are identical across processes;
   - cycles and blowups hand off rather than hang;
   - on decisions with few dice from fresh harness games, Monte Carlo at
     BMAIBagels' settings picks a move worse than the solver's best in a share
     and by a margin set beforehand;
   - the harness shows no loss beyond a margin set beforehand, on both
     matchup sets.
2. **`minimax` engine** at depth two with evaluation version 1, fitted to
   solver values. Monte Carlo for everything but attacks.
3. **Reach sets** in the evaluation: change killing, keystones, locks.
4. **Deeper search** within the node budget, with ordering and *-search.
5. **Swing chooser.**
6. **Remaining decisions** one at a time.

## Decisions

- **One round is the horizon.** Every round counts the same toward the match.
  Between rounds the loser may retune swing and Option dice or add a Reserve
  die, but losing a round to do so is never worth it.
- **Exact odds for every skill**, through the scripted generator. Where the
  evaluation needs skill-specific judgment (reach, safety), skills are added
  in order of how often they appear in recently played games.
- **No simulation inside the engine:** no playouts, no sampled rerolls. A
  position it cannot calculate is handed to Monte Carlo, and counted.
