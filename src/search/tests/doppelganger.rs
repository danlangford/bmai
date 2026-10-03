// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[test]
fn doppelganger_power_attack_copies_the_captured_die_recipe() {
    scenario()
        .phase(FIGHT)
        .attacker("tD20:20")
        .attacks(POWER)
        .defender("pM12:7")
        .expect_allowed(true)
        .expect_attacker_die(0, "pM12:12")
        .expect_no_defender_dice()
        .expect_next_round_attacker_dice(["tD20:20"])
        .run();
}

#[test]
fn doppelganger_does_not_copy_a_skill_attack_target() {
    scenario()
        .attackers(["Dk6:3", "k6:3"])
        .attacks(SKILL)
        .using([0, 1])
        .defender("p(4,2):6")
        .expect_attacker_dice(["kD6:3", "k6:3"])
        .expect_no_defender_dice()
        .expect_captured_defender_dice(["p(4,2):6"])
        .run();
}

#[test]
fn doppelganger_copies_twin_swing_shape_and_doppelganger_skill() {
    // This state cannot be expressed by one parser recipe because its two
    // Twin swing types have independently selected sizes. Keep this as a
    // lower-level test of the in-round transformation.
    let mut game = BMC_Game::default();
    let mut attacker = swing_die('P', property::DOPPELGANGER, 0);
    attacker.m_sides = [20, 0];
    attacker.m_value_total = Some(20);
    let mut target = swing_die('X', property::DOPPELGANGER | property::TWIN, 0);
    target.m_sides = [8, 10];
    target.m_swing_type = [Some('X'), Some('V')];
    target.m_value_total = Some(12);
    game.m_player[0].m_die = vec![attacker];
    game.m_player[1].m_die = vec![target];

    let action = BMC_Move::attack(POWER, [0], [0], 0.0);
    apply_generated_attack(&mut game, &action, &mut BMC_RNG::default());

    let copied = &game.m_player[0].m_die[0];
    assert_eq!(copied.m_sides, [8, 10]);
    assert_eq!(copied.m_swing_type, [Some('X'), Some('V')]);
    assert!(copied.HasProperty(property::TWIN | property::DOPPELGANGER));
}

#[test]
fn capturing_jolt_copies_and_retains_it_after_the_extra_turn_trigger() {
    scenario()
        .attacker("D20:20")
        .attacks(POWER)
        .defender("JM5:5")
        .expect_extra_turn(true)
        .expect_attacker_dice(["MJ5:5"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn copied_time_and_space_runs_after_the_doppelganger_reroll() {
    scenario()
        .attacker("D20:20")
        .attacks(POWER)
        .defender("^kM5:5")
        .expect_extra_turn(true)
        .expect_attacker_dice(["^kM5:5"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn copied_mighty_grows_but_copied_turbo_does_not_resize() {
    // ButtonWeavers engine probe: D(20) capturing H(6) leaves H(8).
    scenario()
        .attacker("D20:20")
        .attacks(POWER)
        .defender("HM6!:6")
        .expect_attacker_dice(["HM8!:8"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn radioactive_doppelganger_decays_before_both_products_copy_the_target() {
    scenario()
        .attacker("%DJ^17!?:17")
        .attacks(POWER)
        .defender("pM8:8")
        .expect_extra_turn(true)
        .expect_attacker_dice(["pM8:8", "pM8:8"])
        .expect_no_defender_dice()
        .expect_next_round_attacker_dice(["^JD%17!?:17"])
        .run();
}

#[test]
fn doppelganger_that_captures_rage_retains_rage_after_transforming() {
    scenario()
        .attacker("D20:20")
        .attacks(POWER)
        .defender("GM7:7")
        .expect_attacker_dice(["MG7:7"])
        .expect_defender_dice(["M7:7"])
        .expect_captured_defender_dice(["MG7:7"])
        .run();
}

#[test]
fn null_doppelganger_still_nulls_its_capture_after_transforming() {
    scenario()
        .attacker("nD20:20")
        .attacks(POWER)
        .defender("6:6")
        .expect_captured_defender_dice(["n6:6"])
        .run();
}

#[test]
fn value_doppelganger_still_values_its_capture_after_transforming() {
    scenario()
        .attacker("vD20:20")
        .attacks(POWER)
        .defender("6:6")
        .expect_captured_defender_dice(["v6:6"])
        .run();
}

#[test]
fn copied_weak_shrinks_on_the_doppelganger_reroll() {
    // ButtonWeavers engine probe: D(20) capturing h(12) leaves h(10).
    scenario()
        .attacker("D20:20")
        .attacks(POWER)
        .defender("h12:12")
        .expect_attacker_dice(["h10:1"])
        .run();
}

#[test]
fn copied_konstant_still_resizes_and_rerolls() {
    // ButtonWeavers engine probe: D(20) capturing kH(4) leaves kH(6) rerolled.
    scenario()
        .attacker("D20:20")
        .attacks(POWER)
        .defender("kH4:4")
        .expect_attacker_dice(["Hk6:5"])
        .run();
}
