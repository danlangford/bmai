# Testing BMAIR mechanics

<!--
SPDX-License-Identifier: MIT
SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
-->

BMAIR's mechanics tests may use a small scenario DSL when a complete game
script would obscure the rule being tested. A scenario reads like a Button Men
position:

```rust
use crate::Attack::Power;
use crate::Phase::Fight;

scenario()
    .phase(Fight)
    .attacker("n30:27")
    .attacks(Power)
    .defender("20:19")
    .expect_allowed(true)
    .expect_scores(0.0, 0.0)
    .expect_attacker_dice(["n30:30"])
    .expect_no_defender_dice()
    .run();
```

Use `.attackers([...]).using([...])` or
`.defenders([...]).targeting([...])` when an attack involves multiple dice.
The indices refer to recipe declaration order even when production parsing
reorders dice for play; the default attacker and target are declaration index
zero. Expectations can cover attack legality, scores, extra turns, active dice,
captured defender dice, and either side's next-round dice. The scenario's dice
hold the round's swing and option selections, and next-round dice are dealt but
not yet rolled, so they are written without values: `B20:10` halved by a
Berserk attack comes back as `B20`. Rerolls
use BMAIR's stable test default; `.seed(...)` selects a specific replay seed
when the exact roll matters. `.turbo(...)` chooses an option-die branch (`0` or
`1`) or a Turbo swing size. `.with_scores(...)` overrides the scores derived
from the starting dice when a scoring rule needs a specific baseline.
`.attacker_special(id)` and `.defender_special(id)` apply a `special` button
rule to either side.
`.expect_attacker_die(index, recipe)` checks one surviving die by declaration
index when other rerolled dice are irrelevant to the rule under test.
`.expect_no_defender_dice()` keeps an empty defending side equally readable.
For Fire attacks, `.boosting([(die, value)])` names each participating die's
fired-up value and `.firing([(die, value)])` names each assisting Fire die's
final value. Both use recipe declaration indices, making the transferred points
and the persistent state visible in the scenario.

`.passes()` replaces `.attacks(...)` to check what a Pass leaves behind, such
as Ornery dice that stay put; it skips the legality check and takes no attack
options. Expected dice may list skills in any order: each expectation the
parser accepts is printed the same way as the actual die, and the rest, such
as Radioactive products below their swing range, are compared as written.

Chance and Focus have their own scenario:

```rust
initiative_scenario()
    .player(["cX?-13:1", "1:1"])
    .opponent(["20:20"])
    .chance_rerolls([0])
    .expect_player_dice(["cX?-12:1", "1:1"])
    .run();

initiative_scenario()
    .player(["f20:12"])
    .opponent(["20:20"])
    .focuses([(0, 7)])
    .expect_player_dice(["f20:7d"])
    .expect_player_dice_next_turn(["f20:7"])
    .run();
```

`.seated_as(1)` puts the player in seat 1, for C++'s seat-keyed Chance rule.
`.expect_initiative(...)` checks who wins initiative on the dice, and
`.expect_chance_success(...)` and `.expect_next_initiative(...)` check what
`apply_chance_move` reports.

`roll(die)` rerolls one die many times on a single RNG stream, for rules about
which sizes or values a reroll can produce:

```rust
roll("(Y,Y)&-13:13")
    .times(200)
    .expect_twin_halves_match()
    .expect_sizes([2, 4, 6, 8, 10, 12, 14, 16, 18, 20])
    .run();
```

The DSL is deliberately test-only and dependency-free. It is not a second game
implementation: setup is parsed by `Parser`, attack legality comes from
`generate_valid_attacks_in_cpp_order` (`.passes()` skips it), resolution comes from
`apply_attack`, round restoration comes from `restore_dice_for_new_round`, and
Chance, Focus, and rerolls use `apply_chance_move`, `apply_focus_move`, and
`roll_scheduled_die`. Expected dice are written using the protocol notation and
failures show canonical expected and actual recipes.

Skill tests live in `src/search/tests/<skill>.rs`, one file per skill. A test
of an interaction between skills goes with the skill whose rule decides the
outcome: Radioactive decay removing Mad is Mad's rule, so it is in `mad.rs`,
and a Mighty target growing on a Boom reroll is Boom's rule, so it is in
`boom.rs`. When both skills' rules shape the outcome, prefer the skill whose
ButtonWeavers skills-page entry describes the interaction from its side.
`parity.rs` holds tests mapped to upstream C++ tests, including the ones whose
expectations were corrected by ButtonWeavers engine probes.

Prefer a scenario when its recipe and outcome tell the whole rules story. Keep
a lower-level test when it needs to inspect an intermediate state, exercise a
phase the DSL does not cover (swing selection, reserve, auxiliary), or prove a
particular internal ordering or RNG-consumption boundary.
