// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! "Engine probe" cases ran ButtonWeavers' `BMAttack::commit_attack` under PHP.

use super::*;
use crate::Attack::Boom;
use test_support::search_scenario;

const REROLL_SEED: u32 = 3;

fn boom_choices(attacker: &str, target: &str) -> Vec<Move> {
    attacks_by(&[attacker], &[target])
        .into_iter()
        .filter(|candidate| candidate.attack == Some(Boom))
        .collect()
}

#[test]
fn boom_removes_the_boom_die_unscored_and_rerolls_the_target() {
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("6:5")
        .expect_attacker_dice(Vec::<&str>::new())
        .expect_defender_dice(["6:3"])
        .expect_captured_defender_dice(Vec::<&str>::new())
        .expect_scores(0.0, 3.0)
        .expect_next_round_attacker_dice(["b4"])
        .run();
}

#[test]
fn only_boom_dice_can_boom() {
    scenario()
        .attacker("4:2")
        .attacks(Boom)
        .defender("6:5")
        .expect_allowed(false)
        .run();
}

#[test]
fn stealth_dice_may_be_targeted_by_boom_attacks() {
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("d6:5")
        .expect_defender_dice(["d6:3"])
        .run();
}

#[test]
fn stealth_and_warrior_dice_cannot_boom() {
    for attacker in ["db4:2", "`b4:4"] {
        scenario()
            .attacker(attacker)
            .attacks(Boom)
            .defender("6:5")
            .expect_allowed(false)
            .run();
    }
}

#[test]
fn warrior_dice_cannot_be_boomed() {
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .defender("`6:6")
        .expect_allowed(false)
        .run();
}

#[test]
fn dizzy_boom_dice_cannot_boom() {
    scenario()
        .attacker("bf4:2d")
        .attacks(Boom)
        .defender("6:5")
        .expect_allowed(false)
        .run();
}

#[test]
fn konstant_boom_dice_may_boom() {
    scenario()
        .attacker("kb4:2")
        .attacks(Boom)
        .defender("6:5")
        .expect_allowed(true)
        .run();
}

#[test]
fn a_konstant_target_keeps_its_value() {
    // Engine probe.
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .defender("k6:5")
        .expect_defender_dice(["k6:5"])
        .run();
}

#[test]
fn a_mighty_target_grows_on_its_reroll() {
    // Engine probe.
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("H6:5")
        .expect_defender_dice(["H8:7"])
        .run();
}

#[test]
fn a_weak_target_shrinks_on_its_reroll() {
    // Engine probe.
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("h12:5")
        .expect_defender_dice(["h10:1"])
        .run();
}

#[test]
fn a_value_target_rescores_after_its_reroll() {
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("v6:5")
        .expect_defender_dice(["v6:3"])
        .expect_scores(0.0, 1.5)
        .run();
}

#[test]
fn a_mad_target_resizes_to_an_even_size() {
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("X&-12:5")
        .expect_defender_dice(["X-14&:5"])
        .run();
}

#[test]
fn a_rage_target_is_not_replaced_because_it_is_not_captured() {
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("G6:5")
        .expect_defender_dice(["G6:3"])
        .expect_captured_defender_dice(Vec::<&str>::new())
        .run();
}

#[test]
fn only_a_jolt_boom_die_grants_an_extra_turn() {
    // Engine probe.
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .defender("J6:5")
        .expect_extra_turn(false)
        .run();
    scenario()
        .attacker("Jb4:2")
        .attacks(Boom)
        .defender("6:5")
        .expect_extra_turn(true)
        .run();
}

#[test]
fn time_and_space_boom_dice_never_grant_an_extra_turn() {
    for seed in 1..=8 {
        scenario()
            .attacker("^b4:2")
            .attacks(Boom)
            .defender("6:5")
            .seed(seed)
            .expect_extra_turn(false)
            .run();
    }
}

#[test]
fn null_and_value_boom_dice_change_nothing_because_nothing_is_captured() {
    for attacker in ["nb4:2", "vb4:2"] {
        scenario()
            .attacker(attacker)
            .attacks(Boom)
            .seed(REROLL_SEED)
            .defender("6:5")
            .expect_defender_dice(["6:3"])
            .run();
    }
}

#[test]
fn radioactive_never_decays_on_a_boom() {
    // Engine probe.
    scenario()
        .attacker("%b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("6:5")
        .expect_attacker_dice(Vec::<&str>::new())
        .run();
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("%6:5")
        .expect_defender_dice(["%6:3"])
        .run();
}

#[test]
fn ornery_dice_reroll_after_a_boom() {
    scenario()
        .attackers(["b4:2", "o10:3"])
        .attacks(Boom)
        .seed(5)
        .using([0])
        .defender("6:5")
        .expect_attacker_dice(["o10:2"])
        .run();
}

#[test]
fn a_twin_target_rerolls_both_halves() {
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("(3,3):5")
        .expect_defender_dice(["(3,3):6"])
        .run();
}

#[test]
fn fire_cannot_assist_a_boom() {
    scenario()
        .attackers(["b4:2", "F6:4"])
        .attacks(Boom)
        .using([0])
        .defender("6:5")
        .boosting([(0, 3)])
        .firing([(1, 3)])
        .expect_allowed(false)
        .run();
}

#[test]
fn turbo_boom_dice_offer_no_turbo_sizes() {
    let choices = boom_choices("bX!-6:2", "6:5")
        .into_iter()
        .map(|candidate| candidate.turbo_option)
        .collect::<Vec<_>>();
    assert_eq!(choices, vec![-1]);
}

#[test]
fn search_reports_a_boom_when_it_is_the_only_attack() {
    search_scenario()
        .phase(Fight)
        .player(0, 0.5, ["b1:1"])
        .player(1, 10.0, ["20:20"])
        .ply(1)
        .simulations(5, 20)
        .max_branch(100)
        .surrender(false)
        .workers([1, 4])
        .expect_attack(Boom)
        .using([0])
        .targeting([0])
        .run();
}
