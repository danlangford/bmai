// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[test]
fn a_morphing_die_returns_to_its_recipe_size_the_next_round() {
    scenario()
        .attacker("m4:4")
        .attacks(Power)
        .defender("20:3")
        .expect_attacker_dice(["m20:1"])
        .expect_next_round_attacker_dice(["m4"])
        .run();
}
