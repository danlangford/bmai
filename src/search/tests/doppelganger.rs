// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[test]
fn doppelganger_power_attack_copies_the_captured_die_recipe() {
    scenario()
        .phase(Fight)
        .attacker("tD20:20")
        .attacks(Power)
        .defender("pM12:7")
        .expect_allowed(true)
        .expect_attacker_die(0, "pM12:12")
        .expect_no_defender_dice()
        .expect_next_round_attacker_dice(["tD20"])
        .run();
}

#[test]
fn doppelganger_does_not_copy_a_skill_attack_target() {
    scenario()
        .attackers(["Dk6:3", "k6:3"])
        .attacks(Skill)
        .using([0, 1])
        .defender("p(4,2):6")
        .expect_attacker_dice(["kD6:3", "k6:3"])
        .expect_no_defender_dice()
        .expect_captured_defender_dice(["p(4,2):6"])
        .run();
}

#[test]
fn doppelganger_copies_twin_swing_shape_and_doppelganger_skill() {
    // No recipe gives two Twin swing types different sizes.
    let mut game = Game::default();
    let mut attacker = swing_die('P', property::DOPPELGANGER, 0);
    attacker.sides = [20, 0];
    attacker.value = Some(20);
    let mut target = swing_die('X', property::DOPPELGANGER | property::TWIN, 0);
    target.sides = [8, 10];
    target.swing_type = [Some('X'), Some('V')];
    target.value = Some(12);
    game.players[0].dice = vec![attacker];
    game.players[1].dice = vec![target];

    let action = Move::new_attack(Power, [0], [0], 0.0);
    apply_generated_attack(&mut game, &action, &mut Rng::default());

    let copied = &game.players[0].dice[0];
    assert_eq!(copied.sides, [8, 10]);
    assert_eq!(copied.swing_type, [Some('X'), Some('V')]);
    assert!(copied.has_property(property::TWIN | property::DOPPELGANGER));
}

#[test]
fn capturing_jolt_copies_and_retains_it_after_the_extra_turn_trigger() {
    scenario()
        .attacker("D20:20")
        .attacks(Power)
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
        .attacks(Power)
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
        .attacks(Power)
        .defender("HM6!:6")
        .expect_attacker_dice(["HM8!:8"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn radioactive_doppelganger_decays_before_both_products_copy_the_target() {
    scenario()
        .attacker("%DJ^17!?:17")
        .attacks(Power)
        .defender("pM8:8")
        .expect_extra_turn(true)
        .expect_attacker_dice(["pM8:8", "pM8:8"])
        .expect_no_defender_dice()
        .expect_next_round_attacker_dice(["^JD%17!?"])
        .run();
}

#[test]
fn doppelganger_that_captures_rage_retains_rage_after_transforming() {
    scenario()
        .attacker("D20:20")
        .attacks(Power)
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
        .attacks(Power)
        .defender("6:6")
        .expect_captured_defender_dice(["n6:6"])
        .run();
}

#[test]
fn value_doppelganger_still_values_its_capture_after_transforming() {
    scenario()
        .attacker("vD20:20")
        .attacks(Power)
        .defender("6:6")
        .expect_captured_defender_dice(["v6:6"])
        .run();
}

#[test]
fn copied_weak_shrinks_on_the_doppelganger_reroll() {
    // ButtonWeavers engine probe: D(20) capturing h(12) leaves h(10).
    scenario()
        .attacker("D20:20")
        .attacks(Power)
        .defender("h12:12")
        .expect_attacker_dice(["h10:1"])
        .run();
}

#[test]
fn copied_konstant_still_resizes_and_rerolls() {
    // ButtonWeavers engine probe: D(20) capturing kH(4) leaves kH(6) rerolled.
    scenario()
        .attacker("D20:20")
        .attacks(Power)
        .defender("kH4:4")
        .expect_attacker_dice(["Hk6:5"])
        .run();
}

#[test]
fn split_copies_of_a_radioactive_konstant_target_still_resize() {
    // Engine probe.
    scenario()
        .attacker("D9:9")
        .attacks(Power)
        .defender("%kH4:3")
        .expect_attacker_dice(["Hk4:1", "Hk4:1"])
        .run();
}

#[test]
fn a_radioactive_doppelgangers_second_konstant_copy_still_resizes() {
    // Engine probe.
    scenario()
        .attacker("%D9:9")
        .attacks(Power)
        .defender("kH4:4")
        .expect_attacker_dice(["Hk6:5", "Hk4:4"])
        .run();
}
