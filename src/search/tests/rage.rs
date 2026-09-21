// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[test]
fn rage_dice_do_not_contribute_to_initiative() {
    let game = native_fixture_game("game\ninitiative\nplayer 0 1 0\nG20:20\nplayer 1 1 0\n6:6\n");
    assert_eq!(CheckInitiative(&game), Some(1));
}

#[test]
fn rage_slow_die_is_still_excluded_from_initiative() {
    let game = native_fixture_game("game\ninitiative\nplayer 0 1 0\nGw20:20\nplayer 1 1 0\n6:6\n");
    assert_eq!(CheckInitiative(&game), Some(1));
}

#[test]
fn rage_focus_die_cannot_be_used_for_focus() {
    let game = native_fixture_game("game\nfocus\nplayer 0 1 0\nGf20:20\nplayer 1 1 0\n6:6\n");
    assert_eq!(GenerateFocusMoves(&game, 0).len(), 1);
    assert!(GenerateFocusMoves(&game, 0)[0].values.is_empty());
}

#[test]
fn attacking_rage_die_loses_rage() {
    scenario()
        .attacker("G6:6")
        .attacks(POWER)
        .defender("1:1")
        .expect_attacker_dice(["6:5"])
        .expect_no_defender_dice()
        .expect_next_round_attacker_dice(["G6:5"])
        .run();
}

#[test]
fn only_participating_rage_dice_lose_rage() {
    scenario()
        .attackers(["G4:4", "G6:6", "8:8"])
        .attacks(SKILL)
        .defender("12:12")
        .using([0, 2])
        .expect_attacker_dice(["G6:6", "8:1", "4:1"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn unsuccessful_trip_consumes_attacking_rage_without_replacing_target() {
    scenario()
        .attacker("Gt6:1")
        .attacks(TRIP)
        .defender("G20:20")
        .expect_attacker_dice(["t6:5"])
        .expect_defender_dice(["G20:13"])
        .run();
}

#[test]
fn successful_trip_captures_and_replaces_a_rage_target() {
    scenario()
        .attacker("Gt20:20")
        .attacks(TRIP)
        .defender("G6:1")
        .expect_attacker_dice(["t20:1"])
        .expect_defender_dice(["6:3"])
        .expect_captured_defender_dice(["G6:1"])
        .run();
}

#[test]
fn captured_rage_die_is_replaced_until_the_round_ends() {
    scenario()
        .attacker("20:20")
        .attacks(POWER)
        .defender("G6:6")
        .expect_attacker_dice(["20:1"])
        .expect_defender_dice(["6:1"])
        .expect_captured_defender_dice(["G6:6"])
        .expect_scores(16.0, 3.0)
        .expect_next_round_defender_dice(["G6:6"])
        .run();
}

#[test]
fn rage_replacement_does_not_apply_mighty_before_its_initial_roll() {
    scenario()
        .attacker("20:20")
        .attacks(POWER)
        .defender("GH6:6")
        .expect_defender_dice(["H6:1"])
        .expect_captured_defender_dice(["HG6:6"])
        .run();
}

#[test]
fn rage_replacement_does_not_apply_weak_before_its_initial_roll() {
    scenario()
        .attacker("20:20")
        .attacks(POWER)
        .defender("Gh10:10")
        .expect_defender_dice(["h10:3"])
        .expect_captured_defender_dice(["hG10:10"])
        .run();
}

#[test]
fn rage_konstant_replacement_receives_an_initial_roll() {
    scenario()
        .attacker("20:20")
        .attacks(POWER)
        .defender("Gk10:10")
        .expect_defender_dice(["k10:3"])
        .expect_captured_defender_dice(["kG10:10"])
        .run();
}

#[test]
fn rage_replacement_keeps_jolt_and_captured_jolt_grants_an_extra_turn() {
    scenario()
        .attacker("20:20")
        .attacks(POWER)
        .defender("GJ6:6")
        .expect_extra_turn(true)
        .expect_defender_dice(["J6:1"])
        .expect_captured_defender_dice(["JG6:6"])
        .run();
}

#[test]
fn time_and_space_on_a_rage_replacement_does_not_grant_an_extra_turn() {
    scenario()
        .attacker("20:20")
        .attacks(POWER)
        .defender("G^6:6")
        .expect_extra_turn(false)
        .expect_defender_dice(["^6:1"])
        .expect_captured_defender_dice(["^G6:6"])
        .run();
}

#[test]
fn attacking_rage_jolt_loses_both_skills_and_grants_one_extra_turn() {
    scenario()
        .attacker("GJ6:6")
        .attacks(POWER)
        .defender("1:1")
        .expect_extra_turn(true)
        .expect_attacker_dice(["6:5"])
        .run();
}

#[test]
fn null_capture_does_not_add_null_to_the_rage_replacement() {
    scenario()
        .attacker("n20:20")
        .attacks(POWER)
        .defender("G6:6")
        .expect_defender_dice(["6:1"])
        .expect_captured_defender_dice(["nG6:6"])
        .run();
}

#[test]
fn value_capture_does_not_add_value_to_the_rage_replacement() {
    scenario()
        .attacker("v20:20")
        .attacks(POWER)
        .defender("G6:6")
        .expect_defender_dice(["6:1"])
        .expect_captured_defender_dice(["vG6:6"])
        .run();
}

#[test]
fn multi_target_rage_replacement_keeps_radioactive_and_poison() {
    scenario()
        .attacker("z8:8")
        .attacks(SPEED)
        .defenders(["pG%7:7", "1:1"])
        .targeting([0, 1])
        .expect_defender_dice(["p%7:6"])
        .expect_captured_defender_dice(["1:1", "p%G7:7"])
        .run();
}

#[test]
fn rage_replacement_preserves_mood_sides_without_triggering_mood() {
    scenario()
        .attacker("30:30")
        .attacks(POWER)
        .defender("GX?-20:20")
        .expect_defender_dice(["X-20?:13"])
        .expect_captured_defender_dice(["GX-20?:20"])
        .run();
}

#[test]
fn attacking_rage_time_and_space_loses_rage_and_can_grant_an_extra_turn() {
    scenario()
        .attacker("G^6:6")
        .attacks(POWER)
        .defender("1:1")
        .expect_extra_turn(true)
        .expect_attacker_dice(["^6:5"])
        .run();
}

#[test]
fn attacking_konstant_rage_loses_rage_without_rerolling() {
    scenario()
        .attackers(["Gk6:6", "8:8"])
        .attacks(SKILL)
        .defender("2:2")
        .using([0, 1])
        .expect_attacker_dice(["k6:6", "8:1"])
        .run();
}

#[test]
fn rage_replacement_preserves_twin_and_turbo_abilities() {
    scenario()
        .attacker("30:30")
        .attacks(POWER)
        .defender("G(4,6)!:10")
        .expect_defender_dice(["(4,6)!:4"])
        .expect_captured_defender_dice(["G(4,6)!:10"])
        .run();
}

#[test]
fn ten_captured_rage_dice_fit_the_bounded_twenty_die_round_pool() {
    scenario()
        .attacker("z10:10")
        .attacks(SPEED)
        .defenders(["G1:1"; 10])
        .targeting(0..10)
        .expect_defender_dice(["1:1"; 10])
        .expect_captured_defender_dice(["G1:1"; 10])
        .run();
}

#[test]
#[should_panic(expected = "Rage replacement exceeds the transformed dice capacity of 20")]
fn rage_replacement_reports_transformed_capacity_exhaustion() {
    let mut game = BMC_Game::default();
    let mut attacker = swing_die('P', 0, 0);
    attacker.m_sides[0] = 20;
    attacker.m_value_total = Some(20);
    game.m_player[0].m_die.push(attacker);
    for original_index in 0..BMD_MAX_DICE {
        let mut target = swing_die('P', 0, original_index);
        target.m_sides[0] = 1;
        target.m_value_total = Some(1);
        if original_index == 0 {
            target.m_properties |= property::RAGE;
        }
        game.m_player[1].m_die.push(target);
    }
    let action = BMC_Move::attack(POWER, [0], [0], 0.0);

    ApplyAttack(&mut game, &action, &mut BMC_RNG::default());
}

#[test]
fn speed_attack_replaces_each_captured_rage_die() {
    scenario()
        .attacker("z10:10")
        .attacks(SPEED)
        .defenders(["G4:4", "G6:6"])
        .targeting([0, 1])
        .expect_defender_dice(["4:3", "6:1"])
        .expect_captured_defender_dice(["G4:4", "G6:6"])
        .run();
}
