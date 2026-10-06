# Minimax engine design

<!--
SPDX-License-Identifier: MIT
SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
-->

Status: draft for discussion. Nothing here is built yet.

## Why a new engine

Monte Carlo has stopped improving with effort. Ply 3, 65 times the
simulations, and a dozen changes to its Quick playouts all measured as ties
(STRENGTH.md). Its estimates are noisy where choices are many: in game 121298
it chose a Trip over a capture because 15 samples flattered the Trip.

`minimax` is a different kind of player: it looks at the opponent's best reply
and at exact dice odds instead of sampled ones, and judges positions with the
strategy published by Cheapass Games (James Ernest and contributors, 1999 to
2002; eleven articles indexed at
<https://web.archive.org/web/20150910034130/http://www.cheapass.com/bpu/bmstrat.html>).
Article numbers below refer to that index.

It is a new `Engine` beside `random`, `maximize`, `quick`, and `montecarlo`,
selected with `ai PLAYER minimax`. It shares the game rules (move generation
and `apply_attack`) and nothing else.

## Principles from the articles

| # | Principle | Use in the engine |
|---|---|---|
| 3, 4 | **Keep-threshold.** The bigger side must end the round keeping at least two thirds of the difference in total sides, `2(A-B)/3`. | Evaluation: whether each side can still keep enough, and which dice are *critical* to that. Swing choice. |
| 1 | **Highest lower number.** Your opponent takes your highest die, so after rerolling, what matters is your *second* highest. | Falls out of the search: exact reroll odds plus the opponent's best reply. Also a cheap evaluation term. |
| 3 | **Shade.** The chance a rerolled die lands on a value the opponent cannot reach with any Power or Skill combination. | Evaluation: safety of each die, from the opponent's *reach set*. |
| 3 | **Change killing and the keystone.** Capture the one die that every combination threatening your critical die needs. Leave the opponent duplicates that cover fewer sums. | Falls out of a two-move search; the reach set makes it cheap to evaluate too. |
| 2 | **Lock.** Leave the opponent no legal attack; big characters win by it. Keep a wide spread of values you can take. | Evaluation: reach-set size for each side; zero means locked. |
| 2 | **Take heavy dice.** Capturing high-sided dice makes your locks likelier and theirs rarer. | Evaluation, through points and reach. |
| 9 | **Field of attack** and exact **Trip odds**: `(N+1)/(2X)` when the target is at least as big, else `(2N-X+1)/(2N)`. Mixed Shadow and regular dice cover more values than either alone. | Exact Trip chance nodes; reach sets that count Shadow, Queer, and mixed dice correctly. |
| 10 | **Blind, active, and hosed** Shadow dice, and rounds that end when both players pass. | Evaluation of Shadow dice; the search handles mutual passing as a round end. |
| 8 | **Poison:** roll your big Poison die until it is small enough that it gets taken; take your opponent's only when you must. | Exact scoring handles points; the search sees the reroll. |
| 6 | **Speed and Focus:** range of sizes counters Speed; Focus can lower a die to deny a Speed attack. | Focus and initiative decisions (later milestone). |
| 4, 5, 11 | **Swing choice.** A small character picks the largest swing that keeps the big one's threshold; a big one the largest that does not raise its own; equals go small to win initiative. | A swing chooser built on the keep-threshold, checked by search. |
| 7 | Twin dice beat a single die of the same size against smaller dice. | Falls out of exact reroll distributions. |
| 2 | Small endgames can be solved exhaustively. | An exact endgame solver. |

## The core idea: sides to capture or protect

The engine plays to win the round, seen through the keep-threshold:

1. **How many sides must I capture, or protect, to win?** From the
   keep-threshold and the points already banked.
2. **Which combinations of dice deliver that?** The opponent's dice I could
   capture, and my own dice I must keep, each as minimal sets.
3. **Attack a die from a combination I need, preferring the combinations with
   the fewest dice**, weighed against the risk of rerolling dice from my own
   protect combinations into the opponent's reach.

The search checks these judgments against the opponent's best reply; the
evaluation and move ordering are built from them.

## Shape of the search

Expectiminimax over one round:

- **My move:** every legal attack and pass, from the existing generator.
- **Chance:** the rerolls that attack causes (attackers; Trip's two rolls),
  enumerated exactly with their probabilities.
- **Opponent's move:** the same, minimizing my result.
- **Leaves:** an evaluation of my chance to win the round (below), or an
  exact value from the endgame solver. Never Monte Carlo playouts.

Depth counts moves, starting at two (my move and the reply). It deepens while
a deterministic node budget allows, so results reproduce across machines; no
wall-clock limits (the time limit was tabled in 0.21.0).

### Keeping chance nodes small

A Skill attack with three d20s has 8,000 outcomes, but most are equivalent.
What matters about a new value is only where it falls among the thresholds
that change anything: the opponent's die values that can capture it and the
sums its own side can reach. The chance node therefore groups outcomes into
**value classes** between those thresholds, with each class's probability.
Article 1's triangular-number arithmetic is the one-die case of this.

Exact odds come from the rules themselves, not from per-skill code: every
random draw in BMAIR goes through `Rng::rand_below`, so the engine replays an
attack with a scripted generator that enumerates each draw's faces in turn.
That yields every outcome with its exact probability for any skill (Mood, Mad,
Trip, Twin, Radioactive, and the rest), and identical resulting positions are
merged. The engine never samples: it calculates the odds or it does not play
the position.

### Ordering and pruning

Moves are ordered by the evaluation of their expected outcome, best first, so
alpha-beta pruning cuts more. Expectiminimax can prune chance nodes only with
bounds on the evaluation (*-search), which the win-probability scale gives:
every value lies in 0 to 1.

## Evaluation

A leaf estimates my probability of winning the round, from features computed
in one pass over at most twenty dice:

1. **Points now and points at stake:** current score difference, and the
   largest swing still possible from dice in play.
2. **Keep-threshold margin:** sides each side must keep against what it can
   still keep; which of its dice are critical.
3. **Reach sets:** the values each side can capture by Power (each die's
   value or more, or less for Shadow), by Skill (subset sums, with Fire
   help), Speed, and Berserk. Bitsets of at most 64 values, so one pass.
4. **Safety (shade):** for each critical die, the chance it survives the
   opponent's reach after its expected reroll.
5. **Lock:** an empty reach set for the side to move.
6. **Initiative:** whose move it is.

The features combine into a probability through weights that start from the
articles' rules of thumb and are then **fitted with the strength harness**,
the only part of the engine that is tuned rather than derived.

## Exact endgame solver

When few dice remain (start at four in total, raise it as speed allows), solve
the rest of the round exactly: every move, every reroll class, both players
optimal. Results are cached by a canonical position, with dice sorted and
values bucketed. Article 2 noted five-die rounds were already brute-forceable
in 1999. These are the positions where 121248 and 121297 were decided.

## Decisions besides attacks

- **Attacks:** the search above.
- **Swing:** candidates from the keep-threshold model (articles 4 and 11),
  each scored by the search from the first move. Starts as Monte Carlo's.
- **Reserve, auxiliary, Chance, Focus:** delegate to Monte Carlo at first,
  then move to the search one at a time, each measured.

## Building it in measured steps

Each step ships only if the strength harness, against Monte Carlo at
BMAIBagels' settings on the classic and many-option matchups, shows it is no
worse, and the step's claimed gain shows up.

1. **Endgame solver**, as a mode of `montecarlo` that switches to it below the
   dice threshold. Smallest step, and the positions ElihuRoot reported.
2. **`minimax` engine** at depth two with exact chance nodes and evaluation
   version 1 (points, keep-threshold, safety). Monte Carlo for everything but
   attacks.
3. **Reach sets** in the evaluation: change killing, keystones, locks.
4. **Fit the weights** with the harness.
5. **Deeper search** within the node budget, with move ordering and *-search
   pruning.
6. **Swing chooser** from the keep-threshold model.
7. **Remaining decisions** one at a time.

## Decisions

- **One round is the horizon.** Every round counts the same toward the match,
  and the only link between rounds, the loser retuning swing dice, never makes
  losing a round worthwhile.
- **Exact odds for every skill**, through the scripted generator. Where the
  evaluation needs skill-specific judgment (reach, safety), skills are added
  in order of how often they appear in recently played games.
- **No simulation anywhere:** no playouts, no sampled rerolls.
