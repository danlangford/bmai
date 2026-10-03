// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[test]
fn konstant_chance_dice_keep_their_size_and_value() {
    // Engine probe.
    for die in ["ckH6:3", "ckh6:3", "ckM6:3"] {
        initiative_scenario()
            .player([die])
            .opponent(["20:20"])
            .chance_rerolls([0])
            .seed(1)
            .expect_player_dice([die])
            .run();
    }
}

#[test]
fn pr82_chance_rerolls_resize_mighty_and_weak_dice_once() {
    for (die, expected) in [("cH6:1", "cH8:8"), ("ch6:1", "ch4:4")] {
        initiative_scenario()
            .player([die])
            .opponent(["20:20"])
            .chance_rerolls([0])
            .seed(1)
            .expect_player_dice([expected])
            .run();
    }
}

#[test]
fn konstant_trip_targets_keep_their_size_and_value() {
    // Engine probe.
    for target in ["kH6:3", "kh6:3"] {
        scenario()
            .attacker("t6:6")
            .attacks(Trip)
            .defender(target)
            .seed(1)
            .expect_captured_defender_dice([target])
            .run();
    }
}

#[test]
fn konstant_ornery_mighty_and_weak_dice_keep_their_size_while_others_reroll() {
    // Engine probe.
    for ornery in ["okH6:3", "okh6:3"] {
        scenario()
            .attackers(["6:6", ornery])
            .attacks(Power)
            .using([0])
            .defender("1:1")
            .expect_attacker_die(1, ornery)
            .run();
    }
    scenario()
        .attackers(["6:6", "o100:100"])
        .attacks(Power)
        .using([0])
        .defender("1:1")
        .seed(1)
        .expect_attacker_die(1, "o100:74")
        .run();
}

#[test]
fn ornery_mood_dice_change_after_an_attack_but_not_a_pass() {
    scenario()
        .attackers(["6:6", "oX?-6:3"])
        .passes()
        .defender("1:1")
        .seed(3)
        .expect_attacker_die(1, "oX-6?:3")
        .run();
    scenario()
        .attackers(["6:6", "oX?-6:3"])
        .attacks(Power)
        .using([0])
        .defender("1:1")
        .seed(3)
        .expect_attacker_die(1, "oX-8?:4")
        .run();
}

#[test]
fn konstant_ornery_mood_die_keeps_its_size_and_value() {
    // Engine probe: Konstant blocks Mood's resize (`doesReroll`).
    scenario()
        .attackers(["6:6", "okX?-6:3"])
        .passes()
        .defender("1:1")
        .seed(3)
        .expect_attacker_die(1, "okX-6?:3")
        .run();
    scenario()
        .attackers(["6:6", "okX?-6:3"])
        .attacks(Power)
        .using([0])
        .defender("1:1")
        .seed(3)
        .expect_attacker_die(1, "okX-6?:3")
        .run();
}

#[test]
fn pr82_konstant_time_and_space_never_grants_extra_turn() {
    scenario()
        .attacker("k^t6:3")
        .attacks(Trip)
        .defender("1:1")
        .expect_extra_turn(false)
        .run();
    scenario()
        .attackers(["k^6:3", "2:2"])
        .attacks(Skill)
        .using([0, 1])
        .defender("5:5")
        .expect_extra_turn(false)
        .run();
}

#[test]
fn pr82_ordinary_time_and_space_uses_its_rerolled_value() {
    scenario()
        .attacker("^6:1")
        .attacks(Power)
        .defender("1:1")
        .seed(3)
        .expect_attacker_dice(["^6:3"])
        .expect_extra_turn(true)
        .run();
}

#[test]
fn warrior_dice_ignore_ornery_rerolls() {
    // Engine probe.
    scenario()
        .attackers(["6:6", "o`10:3", "o10:3"])
        .attacks(Power)
        .using([0])
        .defender("1:1")
        .seed(1)
        .expect_attacker_die(1, "o`10:3")
        .expect_attacker_die(2, "o10:4")
        .run();
}

#[test]
fn konstant_mighty_and_weak_attackers_keep_their_size() {
    // Engine probe.
    for (attacker, expected) in [("kH6:3", "Hk6:3"), ("kh12:3", "hk12:3")] {
        scenario()
            .attackers([attacker, "1:1"])
            .attacks(Skill)
            .using([0, 1])
            .defender("4:4")
            .expect_attacker_die(0, expected)
            .run();
    }
}

#[test]
fn boom_skips_warrior_and_konstant_effects_on_ornery_bystanders() {
    scenario()
        .attackers(["b4:2", "o`10:3", "Hok6:3"])
        .attacks(crate::Attack::Boom)
        .using([0])
        .defender("6:5")
        .expect_attacker_dice(["o`10:3", "Hok6:3"])
        .run();
}
