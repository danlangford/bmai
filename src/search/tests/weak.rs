// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[test]
fn a_weak_die_returns_to_its_recipe_size_the_next_round() {
    scenario()
        .attacker("h8:8")
        .attacks(Power)
        .defender("4:4")
        .expect_attacker_dice(["h6:5"])
        .expect_next_round_attacker_dice(["h8"])
        .run();
}
