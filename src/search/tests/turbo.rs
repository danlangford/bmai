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

#[test]
fn a_turbo_swing_resizes_only_the_attacking_die() {
    scenario()
        .attackers(["X!-10:10", "X-10:7", "X!-10:4"])
        .attacks(Power)
        .defender("4:4")
        .turbo(20)
        .expect_attacker_die(0, "X-20!:1")
        .expect_attacker_die(1, "X-10:7")
        .expect_attacker_die(2, "X-10!:4")
        .run();
}

#[test]
fn a_twin_turbo_swing_resizes_both_halves() {
    scenario()
        .attacker("(X,X)!-10:10")
        .attacks(Power)
        .defender("4:4")
        .turbo(20)
        .expect_attacker_dice(["(X-20,X-20)!:14"])
        .run();
}

#[test]
fn a_second_turbo_die_chooses_a_size_when_it_attacks_alone() {
    scenario()
        .attackers(["X!-10:10", "X!-10:9"])
        .attacks(Power)
        .using([1])
        .defender("4:4")
        .turbo(20)
        .expect_attacker_die(0, "X-10!:10")
        .expect_attacker_die(1, "X-20!:1")
        .run();
}

#[test]
fn a_skill_attack_resizes_its_first_turbo_attacker() {
    scenario()
        .attackers(["X!-10:3", "X!-10:2"])
        .attacks(Skill)
        .using([0, 1])
        .defender("5:5")
        .turbo(20)
        .expect_attacker_die(0, "X-20!:1")
        .expect_attacker_die(1, "X-10!:3")
        .run();
}

#[test]
fn a_konstant_turbo_die_is_not_asked_for_a_size() {
    scenario()
        .attackers(["kX!-10:10", "3:3"])
        .attacks(Skill)
        .using([0, 1])
        .defender("13:13")
        .turbo(20)
        .expect_allowed(false)
        .run();
}

#[test]
fn a_turbo_die_that_morphs_is_not_asked_for_a_size() {
    scenario()
        .attacker("mX!-10:10")
        .attacks(Power)
        .defender("6:6")
        .turbo(20)
        .expect_allowed(false)
        .run();
}

#[test]
fn a_fixed_turbo_die_leaves_the_choice_to_a_swing_turbo_die() {
    scenario()
        .attackers(["16!:8", "X!-10:2"])
        .attacks(Skill)
        .using([0, 1])
        .defender("10:10")
        .turbo(20)
        .expect_attacker_die(0, "16!:1")
        .expect_attacker_die(1, "X-20!:13")
        .run();
}

#[test]
fn a_berserk_turbo_option_die_keeps_a_full_option_size() {
    for (selection, expected) in [(0, "30/1!:11"), (1, "1/30!:1")] {
        scenario()
            .attacker("B1/30!-30:30")
            .attacks(Berserk)
            .defender("30:30")
            .turbo(selection)
            .expect_attacker_dice([expected])
            .run();
    }
}
