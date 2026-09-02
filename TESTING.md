# Testing BMAIR mechanics

<!--
SPDX-License-Identifier: MIT
SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
-->

BMAIR's mechanics tests may use a small scenario DSL when a complete game
script would obscure the rule being tested. A scenario reads like a Button Men
position:

```rust
use crate::BME_ATTACK::POWER;
use crate::BME_PHASE::FIGHT;

scenario()
    .phase(FIGHT)
    .attacker("n30:27")
    .attacks(POWER)
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
captured defender dice, and the attacker's restored next-round recipe. Rerolls
use BMAIR's stable test default; `.seed(...)` selects a specific replay seed
when the exact roll matters. `.turbo(...)` chooses an option-die branch (`0` or
`1`) or a Turbo swing size. `.with_scores(...)` overrides the scores derived
from the starting dice when a scoring rule needs a specific baseline.
`.expect_attacker_die(index, recipe)` checks one surviving die by declaration
index when other rerolled dice are irrelevant to the rule under test.
`.expect_no_defender_dice()` keeps an empty defending side equally readable.

The DSL is deliberately test-only and dependency-free. It is not a second game
implementation: setup is parsed by `BMC_Parser`, legality comes from
`GenerateValidAttacksInCppOrder`, resolution comes from `ApplyAttack`, and
round restoration comes from `RestoreDiceForNewRound`. Expected dice are
written using the protocol notation and failures show canonical expected and
actual recipes.

Prefer a scenario when its recipe and outcome tell the whole rules story. Keep
a lower-level test when it needs to inspect an intermediate state, exercise a
non-attack phase not represented by the DSL, or prove a particular internal
ordering or RNG-consumption boundary.
