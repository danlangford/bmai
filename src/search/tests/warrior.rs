// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;
use crate::Attack::Skill;
use test_support::scenario;

#[test]
fn a_warrior_die_regains_warrior_for_the_next_round() {
    scenario()
        .attackers(["`6:6", "4:2"])
        .attacks(Skill)
        .using([0, 1])
        .defender("8:8")
        .expect_attacker_die(0, "6:5")
        .expect_next_round_attacker_dice(["`6", "4"])
        .run();
}
