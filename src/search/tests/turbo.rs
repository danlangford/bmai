// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[test]
fn a_turbo_swing_die_returns_to_its_chosen_size_the_next_round() {
    scenario()
        .attacker("X!-10:10")
        .attacks(Power)
        .defender("4:4")
        .turbo(20)
        .expect_attacker_dice(["X-20!:1"])
        .expect_next_round_attacker_dice(["X-10!"])
        .run();
}

#[test]
fn a_turbo_option_die_returns_to_its_chosen_side_the_next_round() {
    scenario()
        .attacker("4/12!-12:12")
        .attacks(Power)
        .defender("4:4")
        .turbo(1)
        .expect_attacker_dice(["4/12!:1"])
        .expect_next_round_attacker_dice(["4/12!-12"])
        .run();
}
