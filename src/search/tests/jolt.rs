// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[test]
fn attacking_jolt_grants_an_extra_turn_and_loses_jolt() {
    scenario()
        .attacker("JM6:6")
        .attacks(Power)
        .defender("1:1")
        .expect_extra_turn(true)
        .expect_attacker_dice(["M6:6"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn consumed_jolt_returns_at_the_start_of_the_next_round() {
    scenario()
        .attacker("J6:6")
        .attacks(Power)
        .defender("1:1")
        .expect_attacker_dice(["6:5"])
        .expect_next_round_attacker_dice(["J6"])
        .run();
}

#[test]
fn every_attacking_jolt_loses_the_skill_but_grants_only_one_extra_turn() {
    scenario()
        .attackers(["JM2:2", "JM3:3"])
        .attacks(Skill)
        .using([0, 1])
        .defender("5:5")
        .expect_extra_turn(true)
        .expect_attacker_dice(["M3:3", "M2:2"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn nonparticipating_jolt_keeps_the_skill_and_does_not_grant_an_extra_turn() {
    scenario()
        .attackers(["M6:6", "JM4:4"])
        .attacks(Power)
        .using([0])
        .defender("1:1")
        .expect_extra_turn(false)
        .expect_attacker_dice(["M6:6", "MJ4:4"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn capturing_a_jolt_die_grants_the_capturer_an_extra_turn() {
    scenario()
        .attacker("M6:6")
        .attacks(Power)
        .defender("J1:1")
        .expect_extra_turn(true)
        .expect_no_defender_dice()
        .expect_captured_defender_dice(["J1:1"])
        .run();
}

#[test]
fn unsuccessful_jolt_trip_still_grants_an_extra_turn_and_consumes_jolt() {
    scenario()
        .attacker("tJ6:6")
        .attacks(Trip)
        .defender("6:6")
        .seed(4)
        .expect_extra_turn(true)
        .expect_attacker_dice(["t6:1"])
        .expect_defender_dice(["6:3"])
        .run();
}

#[test]
fn unsuccessful_trip_does_not_trigger_defending_jolt() {
    scenario()
        .attacker("t6:6")
        .attacks(Trip)
        .defender("J6:6")
        .seed(4)
        .expect_extra_turn(false)
        .expect_attacker_dice(["t6:1"])
        .expect_defender_dice(["J6:3"])
        .run();
}

#[test]
fn unsuccessful_time_and_space_trip_grants_an_extra_turn_after_odd_reroll() {
    scenario()
        .attacker("t^6:6")
        .attacks(Trip)
        .defender("6:6")
        .seed(4)
        .expect_extra_turn(true)
        .expect_attacker_dice(["^t6:1"])
        .expect_defender_dice(["6:3"])
        .run();
}

#[test]
fn jolt_and_time_and_space_still_produce_only_one_extra_turn() {
    scenario()
        .attacker("J^1:1")
        .attacks(Power)
        .defender("1:1")
        .expect_extra_turn(true)
        .expect_attacker_dice(["^1:1"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn konstant_jolt_grants_an_extra_turn_without_rerolling() {
    scenario()
        .attackers(["Jk6:3", "2:2"])
        .attacks(Skill)
        .using([0, 1])
        .defender("5:5")
        .expect_extra_turn(true)
        .expect_attacker_dice(["k6:3", "2:1"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn pr82_konstant_attack_side_changes_preserve_value() {
    scenario()
        .attackers(["km9:6", "1:1"])
        .attacks(Skill)
        .using([0, 1])
        .defender("7:7")
        .expect_attacker_die(0, "km7:6")
        .run();
    scenario()
        .attacker("kB9:8")
        .attacks(Berserk)
        .defenders(["3:3", "5:5"])
        .targeting([0, 1])
        .expect_attacker_dice(["k5:8"])
        .run();
}
