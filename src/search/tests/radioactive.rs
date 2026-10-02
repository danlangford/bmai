// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! "Responder log" cases reproduce ButtonWeavers `responder0*Test.php` action
//! logs, the closest thing to a rules oracle for decay ordering.

use super::*;
use crate::BME_ATTACK::{RUSH, SHADOW};
use test_support::{LEGACY, NATIVE, native, search_scenario};

fn attacks_by(attacker_dice: &[&str], defender_dice: &[&str]) -> Vec<BMC_Move> {
    let mut input = String::from("game\nfight\n");
    for (player, dice) in [attacker_dice, defender_dice].into_iter().enumerate() {
        input.push_str(&format!("player {player} {} 0\n", dice.len()));
        for die in dice {
            input.push_str(die);
            input.push('\n');
        }
    }
    native_fixture_game(&input).GenerateValidAttacksInCppOrder()
}

#[test]
fn radioactive_attacker_decays_into_two_halves_that_sum_to_its_size() {
    scenario()
        .attacker("%7:7")
        .attacks(POWER)
        .defender("3:3")
        .expect_attacker_dice(["4:1", "3:1"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn attacker_decays_when_only_the_target_is_radioactive_and_the_target_keeps_radioactive_when_captured()
 {
    scenario()
        .attacker("10:10")
        .attacks(POWER)
        .defender("%4:4")
        .expect_attacker_dice(["5:3", "5:1"])
        .expect_captured_defender_dice(["%4:4"])
        .run();
}

#[test]
fn two_radioactive_dice_decay_the_attacker_only_once() {
    // Responder log: %Ho(1,2) vs %Ho(1,2) -> Ho(2,2) and Ho(1,2).
    scenario()
        .attacker("%Ho(1,2):3")
        .attacks(POWER)
        .defender("%Ho(1,2):2")
        .expect_attacker_dice(["Ho(1,2):3", "Ho(2,2):2"])
        .expect_captured_defender_dice(["Ho%(1,2):2"])
        .run();
}

#[test]
fn a_one_sided_die_decays_into_a_one_sided_and_a_zero_sided_die() {
    // Responder log: (Y=1) vs Hog%(4) -> (Y=1):1 and (Y=0):0.
    scenario()
        .attacker("Y-1:1")
        .attacks(POWER)
        .defender("Hog%4:1")
        .expect_attacker_dice(["Y-1:1", "Y-0:0"])
        .run();
}

#[test]
fn twin_dice_decay_into_alternating_halves() {
    // Responder log: t(T=1,T=1) vs Hog%(6) -> t(T=1,T=0) and t(T=0,T=1).
    scenario()
        .attacker("t(T,T)-1:2")
        .attacks(POWER)
        .defender("Hog%6:2")
        .expect_attacker_dice(["t(T-1,T-0):1", "t(T-0,T-1):1"])
        .run();
}

#[test]
fn odd_twin_halves_give_each_product_one_rounded_up_subdie() {
    // skills.html example.
    scenario()
        .attacker("%p(7,13):20")
        .attacks(POWER)
        .defender("20:20")
        .expect_attacker_dice(["p(3,7):6", "p(4,6):2"])
        .run();
}

#[test]
fn decay_products_score_as_two_smaller_dice() {
    scenario()
        .attacker("%7:7")
        .attacks(POWER)
        .defender("3:3")
        // Own dice score half: (4 + 3) / 2, plus the captured 3.
        .expect_scores(6.5, 0.0)
        .run();
}

#[test]
fn decay_products_return_to_the_original_recipe_next_round() {
    scenario()
        .attacker("%7:7")
        .attacks(POWER)
        .defender("3:3")
        .expect_next_round_attacker_dice(["%7:7"])
        .run();
}

#[test]
fn single_die_skill_attack_decays() {
    // Responder log: %Ho(1,4) Skill vs (X=6) -> Ho(2,4) and Ho(1,4).
    scenario()
        .attacker("%Ho(1,4):4")
        .attacks(SKILL)
        .defender("X-6:4")
        .expect_attacker_dice(["Ho(1,4):5", "Ho(2,4):2"])
        .run();
}

#[test]
fn shadow_attack_decays_and_keeps_shadow() {
    // skills.html example.
    scenario()
        .attacker("sX-15!:3")
        .attacks(SHADOW)
        .defender("%10:9")
        .expect_attacker_dice(["sX-7:6", "sX-8:1"])
        .run();
}

#[test]
fn single_target_speed_attack_decays() {
    scenario()
        .attacker("%z8:8")
        .attacks(SPEED)
        .defender("8:8")
        .expect_attacker_dice(["z4:1", "z4:1"])
        .run();
}

#[test]
fn multi_target_and_multi_attacker_attacks_do_not_decay() {
    scenario()
        .attacker("%z8:8")
        .attacks(SPEED)
        .defenders(["5:5", "3:3"])
        .targeting([0, 1])
        .expect_attacker_dice(["z%8:1"])
        .run();

    scenario()
        .attackers(["%6:3", "6:4"])
        .attacks(SKILL)
        .using([0, 1])
        .defender("%8:7")
        .expect_attacker_dice(["6:5", "%6:1"])
        .expect_captured_defender_dice(["%8:7"])
        .run();

    scenario()
        .attacker("#8:8")
        .attacks(RUSH)
        .defenders(["%5:5", "3:3"])
        .targeting([0, 1])
        .expect_attacker_dice(["#8:1"])
        .run();
}

#[test]
fn successful_trip_decays_after_the_trip_roll() {
    // Responder log: t(T=2,T=2) Trips Hog%(4) -> two t(T=1,T=1). A d1 target
    // makes the capture certain.
    scenario()
        .attacker("t(T,T)-2:3")
        .attacks(TRIP)
        .defender("%1:1")
        .expect_attacker_dice(["t(T-1,T-1):2", "t(T-1,T-1):2"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn failed_trip_still_decays_and_the_surviving_target_loses_radioactive() {
    // Responder log: t(4) fails to Trip %Ho(1,2) -> t(2), t(2), and Ho(2,4).
    // BMAIR doesn't yet allow non-Twin Trips on Twin dice, so the target here
    // is single.
    scenario()
        .attacker("t4:1")
        .attacks(TRIP)
        .defender("%H20:3")
        .expect_attacker_dice(["t2:2", "t2:1"])
        .expect_defender_dice(["H30:13"])
        .run();
}

#[test]
fn berserk_halves_before_it_decays() {
    // Responder log: Bp(U=8) Berserk vs Hog%(6) -> p(U=4) -> two p(U=2).
    scenario()
        .attacker("BpU-8:5")
        .attacks(BERSERK)
        .defender("Hog%6:5")
        .expect_attacker_dice(["pU-2:1", "pU-2:1"])
        .run();
}

#[test]
fn radioactive_berserk_attacker_halves_before_it_decays() {
    scenario()
        .attacker("%B12:6")
        .attacks(BERSERK)
        .defender("6:6")
        .expect_attacker_dice(["3:2", "3:1"])
        .run();
}

#[test]
fn radioactive_doppelganger_decays_before_each_product_copies_the_target() {
    scenario()
        .attacker("%D9:9")
        .attacks(POWER)
        .defender("H4:4")
        .expect_attacker_dice(["H4:1", "H4:1"])
        .run();
}

#[test]
fn doppelganger_copy_of_a_radioactive_target_decays() {
    // Responder log: nD(R=9) vs Hog%(4) -> Hog(4) -> two Hog(2) that grow to Hog(4).
    scenario()
        .attacker("nDR-9:1")
        .attacks(POWER)
        .defender("Hog%4:1")
        .expect_attacker_dice(["Hog4:1", "Hog4:1"])
        .expect_captured_defender_dice(["nHog%4:1"])
        .run();
}

#[test]
fn morphing_morphs_before_it_decays() {
    scenario()
        .attacker("m4:4")
        .attacks(POWER)
        .defender("%10:3")
        .expect_attacker_dice(["m5:3", "m5:1"])
        .run();

    scenario()
        .attacker("%m4:4")
        .attacks(POWER)
        .defender("(6,6):3")
        .expect_attacker_dice(["m(3,3):5", "m(3,3):3"])
        .run();
}

#[test]
fn decay_removes_mood_so_the_products_keep_their_halved_size() {
    scenario()
        .attacker("%X?-20:20")
        .attacks(POWER)
        .defender("1:1")
        .seed(7)
        .expect_attacker_dice(["X-10:4", "X-10:3"])
        .run();
}

#[test]
fn decay_removes_time_and_space_so_an_odd_reroll_grants_no_extra_turn() {
    for seed in 1..=8 {
        scenario()
            .attacker("%^9:9")
            .attacks(POWER)
            .defender("1:1")
            .seed(seed)
            .expect_extra_turn(false)
            .run();
    }
}

#[test]
fn decay_removes_turbo_and_turbo_sizes_are_not_offered() {
    scenario()
        .attacker("%X!-12:12")
        .attacks(POWER)
        .defender("1:1")
        .expect_attacker_dice(["X-6:5", "X-6:1"])
        .run();

    let turbo_sizes = attacks_by(&["%X!-12:12"], &["1:1"])
        .into_iter()
        .map(|candidate| candidate.m_turbo_option)
        .collect::<Vec<_>>();
    assert_eq!(turbo_sizes, vec![-1]);
    assert!(attacks_by(&["X!-12:12"], &["1:1"]).len() > 2);
}

#[test]
fn decay_removes_jolt_after_jolt_grants_its_extra_turn() {
    scenario()
        .attacker("%J8:8")
        .attacks(POWER)
        .defender("1:1")
        .expect_extra_turn(true)
        .expect_attacker_dice(["4:1", "4:1"])
        .expect_next_round_attacker_dice(["J%8:8"])
        .run();
}

#[test]
fn konstant_decay_products_reroll_without_resizing() {
    scenario()
        .attacker("%Hks6:3")
        .attacks(SHADOW)
        .defender("5:5")
        .expect_attacker_dice(["sHk3:2", "sHk3:1"])
        .run();
}

#[test]
fn weak_decay_products_shrink_after_splitting() {
    scenario()
        .attacker("%h12:12")
        .attacks(POWER)
        .defender("1:1")
        .expect_attacker_dice(["h4:1", "h4:1"])
        .run();
}

#[test]
fn rage_attacker_loses_rage_before_it_decays() {
    scenario()
        .attacker("%G8:8")
        .attacks(POWER)
        .defender("1:1")
        .expect_attacker_dice(["4:1", "4:1"])
        .expect_next_round_attacker_dice(["%G8:8"])
        .run();
}

#[test]
fn captured_radioactive_rage_target_is_replaced_and_still_decays_the_attacker() {
    // Responder log: (X=17) vs pG%(7) -> p%(7) added; (X=9) and (X=8).
    scenario()
        .attacker("X-17:8")
        .attacks(POWER)
        .defender("pG%7:3")
        .expect_attacker_dice(["X-9:8", "X-8:5"])
        .expect_defender_dice(["p%7:3"])
        .expect_captured_defender_dice(["p%G7:3"])
        .run();
}

#[test]
fn null_and_value_decay_products_keep_their_capture_effects() {
    scenario()
        .attacker("%n8:8")
        .attacks(POWER)
        .defender("6:6")
        .expect_attacker_dice(["n4:1", "n4:1"])
        .expect_captured_defender_dice(["n6:6"])
        .run();
}

#[test]
fn search_reports_a_radioactive_attack_in_legacy_and_native_modes() {
    search_scenario()
        .phase(FIGHT)
        .player(0, 4.0, ["%8:8"])
        .player(1, 3.0, ["6:6"])
        .ply(1)
        .simulations(5, 20)
        .max_branch(100)
        .surrender(false)
        .modes([LEGACY, NATIVE, native(4)])
        .expect_player_win_percent(0, 100.0..=100.0)
        .expect_attack(POWER)
        .using([0])
        .targeting([0])
        .run();
}
