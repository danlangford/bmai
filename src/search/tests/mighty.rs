// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[test]
fn a_mighty_die_returns_to_its_recipe_size_the_next_round() {
    scenario()
        .attacker("H6:6")
        .attacks(Power)
        .defender("4:4")
        .expect_attacker_dice(["H8:1"])
        .expect_next_round_attacker_dice(["H6"])
        .run();
}

#[test]
fn a_mighty_swing_die_returns_to_its_chosen_size_the_next_round() {
    scenario()
        .attacker("HX-13:13")
        .attacks(Power)
        .defender("4:4")
        .expect_attacker_dice(["HX-16:1"])
        .expect_next_round_attacker_dice(["HX-13"])
        .run();
}
