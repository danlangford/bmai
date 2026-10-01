// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;
use crate::BME_ATTACK::{RUSH, SHADOW};
use test_support::{LEGACY, NATIVE, native, search_scenario};

fn rush_attacks(attacker_dice: &[&str], defender_dice: &[&str]) -> Vec<BMC_Move> {
    let mut input = String::from("game\nfight\n");
    for (player, dice) in [attacker_dice, defender_dice].into_iter().enumerate() {
        input.push_str(&format!("player {player} {} 0\n", dice.len()));
        for die in dice {
            input.push_str(die);
            input.push('\n');
        }
    }
    native_fixture_game(&input)
        .GenerateValidAttacksInCppOrder()
        .into_iter()
        .filter(|candidate| candidate.m_attack == Some(RUSH))
        .collect()
}

// Core rules from the ButtonWeavers skill description and BMAttackRush.

#[test]
fn rush_die_captures_two_dice_whose_values_sum_to_its_value() {
    scenario()
        .attacker("#6:6")
        .attacks(RUSH)
        .defenders(["2:2", "4:4"])
        .targeting([0, 1])
        .expect_extra_turn(false)
        .expect_attacker_dice(["#6:5"])
        .expect_no_defender_dice()
        .expect_captured_defender_dice(["2:2", "4:4"])
        .expect_scores(9.0, 0.0)
        .expect_next_round_attacker_dice(["#6:5"])
        .run();
}

#[test]
fn any_die_may_rush_when_a_target_is_a_rush_die() {
    scenario()
        .attacker("6:6")
        .attacks(RUSH)
        .defenders(["#2:2", "4:4"])
        .targeting([0, 1])
        .expect_attacker_dice(["6:5"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn rush_requires_a_rush_attacker_or_target() {
    scenario()
        .attacker("6:6")
        .attacks(RUSH)
        .defenders(["2:2", "4:4"])
        .targeting([0, 1])
        .expect_allowed(false)
        .run();
}

#[test]
fn rush_targets_must_sum_exactly_to_the_attacker_value() {
    for targets in [["2:2", "3:3"], ["3:3", "4:4"]] {
        scenario()
            .attacker("#6:6")
            .attacks(RUSH)
            .defenders(targets)
            .targeting([0, 1])
            .expect_allowed(false)
            .run();
    }
}

#[test]
fn rush_requires_exactly_two_targets() {
    scenario()
        .attacker("#6:6")
        .attacks(RUSH)
        .defender("#6:6")
        .expect_allowed(false)
        .run();

    scenario()
        .attacker("#6:6")
        .attacks(RUSH)
        .defenders(["1:1", "2:2", "3:3"])
        .targeting([0, 1, 2])
        .expect_allowed(false)
        .run();
}

#[test]
fn rush_enumerates_every_qualifying_target_pair_once() {
    let attacks = rush_attacks(&["#6:6"], &["1:1", "5:5", "2:2", "4:4", "3:3"]);
    let mut pairs = attacks
        .iter()
        .map(|candidate| {
            let mut pair = candidate.m_targets.iter().collect::<Vec<_>>();
            pair.sort_unstable();
            pair
        })
        .collect::<Vec<_>>();
    pairs.sort();
    // Optimized defender order is 5, 4, 3, 2, 1. Pairs (5,1) and (4,2)
    // sum to six; three cannot pair with itself.
    assert_eq!(pairs, vec![vec![0, 4], vec![1, 3]]);
}

// Attack-type eligibility shared with ButtonWeavers' Speed validation.

#[test]
fn stealth_dice_cannot_rush_or_be_rushed() {
    scenario()
        .attacker("#d6:6")
        .attacks(RUSH)
        .defenders(["2:2", "4:4"])
        .targeting([0, 1])
        .expect_allowed(false)
        .run();

    scenario()
        .attacker("#6:6")
        .attacks(RUSH)
        .defenders(["d2:2", "4:4"])
        .targeting([0, 1])
        .expect_allowed(false)
        .run();
}

#[test]
fn warrior_dice_cannot_rush_or_be_rushed() {
    scenario()
        .attacker("#`6:6")
        .attacks(RUSH)
        .defenders(["2:2", "4:4"])
        .targeting([0, 1])
        .expect_allowed(false)
        .run();

    scenario()
        .attacker("#6:6")
        .attacks(RUSH)
        .defenders(["`2:2", "4:4"])
        .targeting([0, 1])
        .expect_allowed(false)
        .run();
}

#[test]
fn dizzy_focus_die_cannot_rush() {
    scenario()
        .attacker("#f6:6d")
        .attacks(RUSH)
        .defenders(["2:2", "4:4"])
        .targeting([0, 1])
        .expect_allowed(false)
        .run();
}

#[test]
fn insult_and_dizzy_dice_can_be_rushed() {
    scenario()
        .attacker("#6:6")
        .attacks(RUSH)
        .defenders(["I2:2", "f4:4d"])
        .targeting([0, 1])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn attack_restricted_skills_can_still_rush() {
    // Shadow, odd Queer, Konstant, and Fire forbid Power; Berserk and
    // Unskilled forbid Skill. None of them restricts Rush.
    for attacker in [
        "#s6:6", "#q8:5", "#k6:6", "#F6:6", "#B6:6", "#~6:6", "#t6:6", "#D6:6",
    ] {
        let value = if attacker == "#q8:5" { "3:3" } else { "4:4" };
        scenario()
            .attacker(attacker)
            .attacks(RUSH)
            .defenders(["2:2", value])
            .targeting([0, 1])
            .expect_no_defender_dice()
            .run();
    }
}

#[test]
fn speed_rush_die_offers_one_speed_attack_instead_of_a_duplicate_rush() {
    assert!(rush_attacks(&["#z6:6"], &["2:2", "4:4"]).is_empty());
    let game = native_fixture_game("game\nfight\nplayer 0 1 0\n#z6:6\nplayer 1 2 0\n2:2\n4:4\n");
    let speed = game
        .GenerateValidAttacksInCppOrder()
        .into_iter()
        .filter(|candidate| candidate.m_targets.len() == 2)
        .map(|candidate| candidate.m_attack)
        .collect::<Vec<_>>();
    assert_eq!(speed, vec![Some(SPEED)]);
}

#[test]
fn rush_targets_are_also_vulnerable_to_ordinary_speed_attacks() {
    scenario()
        .attacker("z9:9")
        .attacks(SPEED)
        .defenders(["#2:2", "3:3", "4:4"])
        .targeting([0, 1, 2])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn stinger_rush_attacker_must_match_the_sum_exactly() {
    scenario()
        .attacker("#g6:6")
        .attacks(RUSH)
        .defenders(["2:2", "3:3"])
        .targeting([0, 1])
        .expect_allowed(false)
        .run();

    scenario()
        .attacker("#g6:6")
        .attacks(RUSH)
        .defenders(["2:2", "4:4"])
        .targeting([0, 1])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn fire_cannot_assist_a_rush_attack() {
    scenario()
        .attackers(["#6:4", "F6:4"])
        .attacks(RUSH)
        .using([0])
        .defenders(["2:2", "3:3"])
        .targeting([0, 1])
        .boosting([(0, 5)])
        .firing([(1, 3)])
        .expect_allowed(false)
        .run();
}

#[test]
fn twin_values_are_summed_on_both_sides_of_a_rush() {
    scenario()
        .attacker("#(4,4):7")
        .attacks(RUSH)
        .defenders(["(2,2):3", "4:4"])
        .targeting([0, 1])
        .expect_no_defender_dice()
        .run();
}

// Attack-resolution interactions.

#[test]
fn konstant_rush_attacker_keeps_its_value_and_konstant_targets_are_captured() {
    scenario()
        .attacker("#k6:6")
        .attacks(RUSH)
        .defenders(["k2:2", "4:4"])
        .targeting([0, 1])
        .expect_attacker_dice(["k#6:6"])
        .expect_captured_defender_dice(["k2:2", "4:4"])
        .run();
}

#[test]
fn berserk_rush_attacker_keeps_berserk_and_its_size() {
    scenario()
        .attacker("#B6:6")
        .attacks(RUSH)
        .defenders(["2:2", "4:4"])
        .targeting([0, 1])
        .expect_attacker_dice(["B#6:5"])
        .run();
}

#[test]
fn morphing_rush_attacker_does_not_morph_after_two_captures() {
    scenario()
        .attacker("#m6:6")
        .attacks(RUSH)
        .defenders(["2:2", "(2,2):4"])
        .targeting([0, 1])
        .expect_attacker_dice(["m#6:5"])
        .run();
}

#[test]
fn doppelganger_radioactive_rush_neither_copies_nor_decays() {
    scenario()
        .attacker("#D%6:6")
        .attacks(RUSH)
        .defenders(["2:2", "%20:4"])
        .targeting([0, 1])
        .expect_attacker_dice(["D%#6:5"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn attacking_jolt_rush_grants_an_extra_turn_and_loses_jolt() {
    scenario()
        .attacker("#J6:6")
        .attacks(RUSH)
        .defenders(["2:2", "4:4"])
        .targeting([0, 1])
        .expect_extra_turn(true)
        .expect_attacker_dice(["#6:5"])
        .run();
}

#[test]
fn rushing_a_captured_jolt_die_grants_an_extra_turn() {
    scenario()
        .attacker("6:6")
        .attacks(RUSH)
        .defenders(["#J2:2", "4:4"])
        .targeting([0, 1])
        .expect_extra_turn(true)
        .run();
}

#[test]
fn time_and_space_rush_attacker_can_grant_an_extra_turn() {
    scenario()
        .attacker("#^6:6")
        .attacks(RUSH)
        .defenders(["2:2", "4:4"])
        .targeting([0, 1])
        .expect_extra_turn(true)
        .expect_attacker_dice(["^#6:5"])
        .run();
}

#[test]
fn rage_rush_attacker_loses_rage_and_both_rage_targets_are_replaced() {
    scenario()
        .attacker("#G6:6")
        .attacks(RUSH)
        .defenders(["#G2:2", "G4:4"])
        .targeting([0, 1])
        .expect_attacker_dice(["#6:5"])
        .expect_defender_dice(["4:1", "#2:1"])
        .expect_captured_defender_dice(["G#2:2", "G4:4"])
        .expect_next_round_attacker_dice(["G#6:5"])
        .expect_next_round_defender_dice(["G#2:2", "G4:4"])
        .run();
}

#[test]
fn null_rush_attacker_nullifies_both_captured_dice() {
    scenario()
        .attacker("#n6:6")
        .attacks(RUSH)
        .defenders(["2:2", "4:4"])
        .targeting([0, 1])
        .expect_captured_defender_dice(["n2:2", "n4:4"])
        .expect_scores(0.0, 0.0)
        .run();
}

#[test]
fn value_and_poison_scoring_apply_to_each_rushed_die() {
    scenario()
        .attacker("#v6:6")
        .attacks(RUSH)
        .defenders(["2:2", "p4:4"])
        .targeting([0, 1])
        .expect_captured_defender_dice(["v2:2", "pv4:4"])
        // The Value attacker keeps its 3-point own score; a captured Value die
        // is worth 2 and a captured Poison+Value die costs half its value.
        .expect_scores(3.0, 0.0)
        .run();
}

#[test]
fn mighty_and_weak_rush_attackers_resize_before_rerolling() {
    scenario()
        .attacker("#H6:6")
        .attacks(RUSH)
        .defenders(["2:2", "4:4"])
        .targeting([0, 1])
        .expect_attacker_dice(["H#8:1"])
        .run();

    scenario()
        .attacker("#h6:6")
        .attacks(RUSH)
        .defenders(["2:2", "4:4"])
        .targeting([0, 1])
        .expect_attacker_dice(["h#4:1"])
        .run();
}

#[test]
fn mood_rush_attacker_and_nonparticipating_ornery_dice_reroll() {
    scenario()
        .attackers(["#X?-6:6", "o20:20"])
        .attacks(RUSH)
        .using([0])
        .defenders(["2:2", "4:4"])
        .targeting([0, 1])
        .expect_attacker_dice(["o20:3", "#X-12?:1"])
        .run();
}

#[test]
fn maximum_rush_attacker_rerolls_to_its_maximum() {
    scenario()
        .attacker("#M8:6")
        .attacks(RUSH)
        .defenders(["2:2", "4:4"])
        .targeting([0, 1])
        .expect_attacker_dice(["M#8:8"])
        .run();
}

#[test]
fn turbo_rush_attacker_offers_each_turbo_size() {
    let attacks = rush_attacks(&["#X!-6:6"], &["2:2", "4:4"]);
    let sizes = attacks
        .iter()
        .map(|candidate| candidate.m_turbo_option)
        .collect::<Vec<_>>();
    assert_eq!(sizes.first(), Some(&6));
    assert!(sizes.contains(&4) && sizes.contains(&20));

    scenario()
        .attacker("#X!-6:6")
        .attacks(RUSH)
        .defenders(["2:2", "4:4"])
        .targeting([0, 1])
        .turbo(20)
        .expect_attacker_dice(["#X-20!:1"])
        .run();
}

#[test]
fn reserve_rush_dice_are_not_targets() {
    assert!(rush_attacks(&["6:6"], &["#r2", "2:2", "4:4"]).is_empty());
}

// Protocol and search integration.

#[test]
fn rush_notation_inside_game_blocks_is_not_a_comment() {
    let input = "# top-level comment\ngame\nfight\nplayer 0 1 0\n#6:6\nplayer 1 2 0\n2:2\n#4:4\n";
    let game = native_fixture_game(input);
    assert!(game.m_player[0].m_die[0].HasProperty(property::RUSH));
    assert!(
        game.m_player[1]
            .m_die
            .iter()
            .any(|die| die.HasProperty(property::RUSH) && die.GetValueTotal() == 4)
    );

    let mut streamed = crate::BMC_Parser::default();
    streamed
        .ParseStream(&mut input.as_bytes(), &mut Vec::new())
        .unwrap();
    assert!(streamed.m_game.m_player[0].m_die[0].HasProperty(property::RUSH));
}

#[test]
fn search_reports_a_round_winning_rush_in_legacy_and_native_modes() {
    search_scenario()
        .phase(FIGHT)
        .player(0, 3.0, ["#6:6"])
        .player(1, 3.0, ["2:2", "4:4"])
        .ply(2)
        .simulations(5, 50)
        .max_branch(100)
        .surrender(false)
        .modes([LEGACY, NATIVE, native(4)])
        .expect_player_win_percent(0, 100.0..=100.0)
        .expect_attack(RUSH)
        .using([0])
        .targeting([1, 0])
        .run();
}

#[test]
fn shadow_rush_die_offers_both_attack_types() {
    let game =
        native_fixture_game("game\nfight\nplayer 0 1 0\n#s6:3\nplayer 1 3 0\n1:1\n2:2\n5:5\n");
    let attacks = game
        .GenerateValidAttacksInCppOrder()
        .into_iter()
        .filter_map(|candidate| candidate.m_attack)
        .collect::<Vec<_>>();
    assert!(attacks.contains(&RUSH));
    assert!(attacks.contains(&SHADOW));
}
