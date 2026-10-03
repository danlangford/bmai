// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! Button specials (ButtonWeavers `BMBtnSkill*`) and the Trip and Morphing
//! corrections that ship with them.

use super::*;

fn parsed(input: &str) -> crate::BMC_Parser {
    let mut parser = crate::BMC_Parser::default();
    parser.ParseString(input, &mut Vec::new()).unwrap();
    parser
}

const GAME: &str = "game\nfight\nplayer 0 1 0\n6:6\nplayer 1 1 0\n6:6\n";

#[test]
fn special_command_sets_and_reports_each_players_specials() {
    let mut output = Vec::new();
    let mut parser = crate::BMC_Parser::default();
    parser
        .ParseString(
            &format!("{GAME}special 1 no_skill_attacks skill_immune\n"),
            &mut output,
        )
        .unwrap();
    assert_eq!(
        parser.m_game.m_player[1].m_specials,
        special::NO_SKILL_ATTACKS | special::SKILL_IMMUNE
    );
    assert_eq!(parser.m_game.m_player[0].m_specials, 0);
    assert!(
        String::from_utf8(output)
            .unwrap()
            .ends_with("Setting specials for player 1 to no_skill_attacks skill_immune\n")
    );
}

#[test]
fn each_game_block_clears_specials() {
    let parser = parsed(&format!("{GAME}special 0 no_initiative\n{GAME}"));
    assert_eq!(parser.m_game.m_player[0].m_specials, 0);
}

#[test]
fn unknown_specials_and_players_are_rejected() {
    let error = crate::BMC_Parser::default()
        .ParseString(&format!("{GAME}special 0 flying\n"), &mut Vec::new())
        .unwrap_err()
        .to_string();
    assert!(error.contains("unknown button special: flying"), "{error}");
    assert!(
        crate::BMC_Parser::default()
            .ParseString(&format!("{GAME}special 2 no_initiative\n"), &mut Vec::new())
            .is_err()
    );
}

#[test]
fn largo_cannot_skill_attack() {
    scenario()
        .attackers(["6:3", "6:4"])
        .attacker_special("no_skill_attacks")
        .attacks(SKILL)
        .using([0, 1])
        .defender("8:7")
        .expect_allowed(false)
        .run();
}

#[test]
fn largo_may_still_power_attack() {
    scenario()
        .attacker("6:4")
        .attacker_special("no_skill_attacks")
        .attacks(POWER)
        .defender("8:4")
        .expect_allowed(true)
        .run();
}

#[test]
fn japanese_beetle_cannot_be_skill_attacked() {
    scenario()
        .attackers(["6:3", "6:4"])
        .attacks(SKILL)
        .using([0, 1])
        .defender("8:7")
        .defender_special("skill_immune")
        .expect_allowed(false)
        .run();
}

#[test]
fn japanese_beetle_may_still_be_power_attacked() {
    scenario()
        .attacker("6:4")
        .attacks(POWER)
        .defender("8:4")
        .defender_special("skill_immune")
        .expect_allowed(true)
        .run();
}

#[test]
fn largo_search_reports_a_power_attack_over_the_wire() {
    test_support::parser_scenario(
        "game 3\nfight\nplayer 0 2 0\n6:3\n6:4\nplayer 1 2 0\n8:7\n8:4\nspecial 0 no_skill_attacks\nply 1\nmax_sims 5\nmin_sims 5\nsurrender off\ngetaction\n",
    )
    .expect_attack(POWER)
    .using([1])
    .targeting([1])
    .run();
}

#[test]
fn simulations_keep_each_players_specials_after_a_side_swap() {
    let game = parsed(&format!(
        "{GAME}special 0 unique_swing\nspecial 1 no_initiative\n"
    ))
    .m_game;
    let mut simulation = game.clone();
    simulation.m_player.swap(0, 1);
    super::fight::RestoreSimulation(&mut simulation, &game);
    assert_eq!(simulation.m_player[0].m_specials, special::UNIQUE_SWING);
    assert_eq!(simulation.m_player[1].m_specials, special::NO_INITIATIVE);
}

#[test]
fn no_initiative_loses_to_lower_dice_and_to_a_button_with_no_initiative_dice() {
    let input = |dice: &str| {
        format!(
            "game\ninitiative\nplayer 0 1 0\n1:1\nplayer 1 1 0\n{dice}\nspecial 0 no_initiative\n"
        )
    };
    assert_eq!(CheckInitiative(&parsed(&input("20:20")).m_game), Some(1));
    // A Trip die does not count for initiative, but only Giant is ranked last.
    assert_eq!(CheckInitiative(&parsed(&input("t4:4")).m_game), Some(1));

    let both = parsed(
        "game\ninitiative\nplayer 0 1 0\n1:1\nplayer 1 1 0\n20:20\nspecial 0 no_initiative\nspecial 1 no_initiative\n",
    );
    assert_eq!(CheckInitiative(&both.m_game), None);
}

#[test]
fn unique_swing_assigns_different_swing_types_different_sizes() {
    let parser =
        parsed("game\npreround\nplayer 0 3 0\nX\nY\nY\nplayer 1 1 0\n6\nspecial 0 unique_swing\n");
    let moves = GenerateSwingMoves(&parser.m_game.m_player[0]);
    // X is 4-20 and Y is 1-20, so 17 of the 17 * 20 pairs share a size.
    assert_eq!(moves.len(), 17 * 20 - 17);
    assert!(moves.iter().all(|candidate| {
        let values = candidate.values();
        values[0].1 != values[1].1
    }));
}

#[test]
fn unique_sizes_also_avoids_fixed_die_sizes() {
    let parser =
        parsed("game\npreround\nplayer 0 2 0\n8\nV\nplayer 1 1 0\n6\nspecial 0 unique_sizes\n");
    let sizes = GenerateSwingMoves(&parser.m_game.m_player[0])
        .iter()
        .map(|candidate| candidate.values()[0].1)
        .collect::<Vec<_>>();
    assert_eq!(sizes, vec![6, 7, 9, 10, 11, 12]);
}

#[test]
fn unique_sizes_compares_option_dice_at_their_chosen_side() {
    let parser =
        parsed("game\npreround\nplayer 0 2 0\n8/10\nV\nplayer 1 1 0\n6\nspecial 0 unique_sizes\n");
    let moves = GenerateSwingMoves(&parser.m_game.m_player[0]);
    let chose = |size: u8, second: bool| {
        moves
            .iter()
            .any(|candidate| candidate.values()[0].1 == size && candidate.options()[0].1 == second)
    };
    // V is 6-12 and the option is 8 or 10, so two of the 14 pairs collide.
    assert_eq!(moves.len(), 12);
    assert!(!chose(8, false) && !chose(10, true));
    assert!(chose(8, true) && chose(10, false));
}

fn gordo_auxiliary(player_0: &str, player_1: &str, gordo: usize) -> BMC_Game {
    parsed(&format!(
        "game 3\naux\nplayer 0 2 0\nV\n{player_0}\nplayer 1 2 0\n8\n{player_1}\nspecial {gordo} unique_sizes\nquit\n"
    ))
    .m_game
}

#[test]
fn gordo_declines_a_v_to_z_auxiliary_swing_die() {
    assert_eq!(
        SelectQAIAuxiliaryAction(&gordo_auxiliary("+X", "+6", 0)),
        None
    );
}

#[test]
fn gordo_accepts_other_auxiliary_dice() {
    for auxiliary in ["+4", "+Q", "+(X,X)"] {
        assert_eq!(
            SelectQAIAuxiliaryAction(&gordo_auxiliary(auxiliary, "+6", 0)),
            Some(1),
            "{auxiliary}"
        );
    }
}

#[test]
fn either_players_gordo_decline_removes_both_auxiliary_dice() {
    for (player_0, player_1, gordo) in [("+X", "+6", 0), ("+6", "+X", 1)] {
        let mut game = gordo_auxiliary(player_0, player_1, gordo);
        ApplyAuxiliaryDecision(&mut game, true);
        assert_eq!(game.m_player[0].m_die.len(), 1, "Gordo is player {gordo}");
        assert_eq!(game.m_player[1].m_die.len(), 1, "Gordo is player {gordo}");
    }
}

// Trip legality follows `BMAttackTrip::validate_attack`.

#[test]
fn single_trip_dice_may_trip_twin_dice_they_can_reach() {
    scenario()
        .attacker("t4:1")
        .attacks(TRIP)
        .defender("(3,3):4")
        .expect_allowed(true)
        .run();

    scenario()
        .attacker("t1:1")
        .attacks(TRIP)
        .defender("(3,3):4")
        .expect_allowed(false)
        .run();
}

#[test]
fn trip_must_reach_a_konstant_targets_value() {
    for (target, allowed) in [("k6:5", false), ("k6:4", true)] {
        scenario()
            .attacker("t4:1")
            .attacks(TRIP)
            .defender(target)
            .expect_allowed(allowed)
            .run();
    }
}

#[test]
fn trip_must_reach_a_maximum_targets_size() {
    for (target, allowed) in [("M6:2", false), ("M4:2", true)] {
        scenario()
            .attacker("t4:1")
            .attacks(TRIP)
            .defender(target)
            .expect_allowed(allowed)
            .run();
    }
}

#[test]
fn konstant_trip_dice_reach_only_their_value() {
    for (attacker, allowed) in [("tk4:1", false), ("tk4:2", true)] {
        scenario()
            .attacker(attacker)
            .attacks(TRIP)
            .defender("(3,3):4")
            .expect_allowed(allowed)
            .run();
    }
}

#[test]
fn weak_trip_dice_reach_less_far() {
    for (attacker, allowed) in [("th4:1", true), ("th2:1", false)] {
        scenario()
            .attacker(attacker)
            .attacks(TRIP)
            .defender("(3,3):4")
            .expect_allowed(allowed)
            .run();
    }
}

#[test]
fn mood_trip_dice_reach_their_largest_swing_size() {
    scenario()
        .attacker("tX?-4:1")
        .attacks(TRIP)
        .defender("k15:15")
        .expect_allowed(true)
        .run();
}

#[test]
fn mood_twin_trip_dice_reach_one_subdies_swing_size() {
    // ButtonWeavers reads a Twin's swing range from a single subdie.
    scenario()
        .attacker("t(X,X)?-15:2")
        .attacks(TRIP)
        .defender("k25:25")
        .expect_allowed(false)
        .run();
}

#[test]
fn a_mood_maximum_target_counts_at_its_smallest_swing_size() {
    scenario()
        .attacker("t4:1")
        .attacks(TRIP)
        .defender("MX?-20:20")
        .expect_allowed(true)
        .run();
}

#[test]
fn mighty_trip_dice_reach_further() {
    scenario()
        .attacker("Ht1:1")
        .attacks(TRIP)
        .defender("(3,3):4")
        .expect_allowed(true)
        .run();
}

fn trip_turbo_choices(attacker: &str, target: &str) -> Vec<i16> {
    attacks_by(&[attacker], &[target])
        .into_iter()
        .filter(|candidate| candidate.m_attack == Some(TRIP))
        .map(|candidate| candidate.m_turbo_option)
        .collect()
}

#[test]
fn turbo_trip_sizes_too_small_for_the_target_are_not_offered() {
    let sizes = trip_turbo_choices("tP!-4:1", "(3,3):4");
    assert!(!sizes.contains(&1), "{sizes:?}");
    assert!(sizes.contains(&2) && sizes.contains(&30), "{sizes:?}");
}

#[test]
fn turbo_trip_is_offered_when_only_a_larger_size_reaches_the_target() {
    assert_eq!(trip_turbo_choices("tX!-4:1", "M20:3"), vec![20]);
}

#[test]
fn option_turbo_trip_offers_only_the_side_that_reaches_the_target() {
    assert_eq!(trip_turbo_choices("t4/20!-20:1", "M12:3"), vec![0]);
}

// Morphing follows `BMSkillMorphing::capture`: any single-target attack, and
// never after a failed one.

#[test]
fn single_target_berserk_attack_morphs() {
    scenario()
        .attacker("mB12:6")
        .attacks(BERSERK)
        .defender("20:6")
        .expect_attacker_dice(["m20:1"])
        .run();
}

#[test]
fn single_target_speed_attack_morphs() {
    scenario()
        .attacker("mz12:6")
        .attacks(SPEED)
        .defender("20:6")
        .expect_attacker_dice(["zm20:1"])
        .run();
}

#[test]
fn failed_trip_does_not_morph() {
    scenario()
        .attacker("tm4:1")
        .attacks(TRIP)
        .defender("20:20")
        .expect_defender_dice(["20:13"])
        .expect_attacker_dice(["tm4:1"])
        .run();
}

/// The three draws a Trip-then-morph attack makes: the attacker's Trip roll,
/// the target's reroll, and the morphed die's reroll.
fn trip_morph_draws(seed: u32, attacker_sides: u32, target_sides: u32) -> [u32; 3] {
    let mut rng = BMC_RNG::default();
    rng.SRand(seed);
    let trip = rng.GetRandMax(attacker_sides) + 1;
    let target = rng.GetRandMax(target_sides) + 1;
    let morphed = rng.GetRandMax(target_sides) + 1;
    [trip, target, morphed]
}

#[test]
fn successful_trip_rolls_at_its_own_size_then_morphs_and_rerolls() {
    // A Trip roll above 2 proves the d20 rolled before morphing, a final value
    // of at most 2 proves the reroll, and a d2 rolling first would fail.
    let seed = (1..10_000)
        .find(|seed| {
            let [trip, target, _] = trip_morph_draws(*seed, 20, 2);
            let [early_trip, early_target, _] = trip_morph_draws(*seed, 2, 2);
            trip > 2 && trip >= target && early_trip < early_target
        })
        .expect("a discriminating seed");
    let [_, _, morphed] = trip_morph_draws(seed, 20, 2);
    scenario()
        .attacker("tm20:1")
        .attacks(TRIP)
        .defender("2:1")
        .seed(seed)
        .expect_no_defender_dice()
        .expect_attacker_dice([format!("tm2:{morphed}")])
        .run();
}

#[test]
fn time_and_space_counts_the_reroll_after_a_trip_morph() {
    let seed = (1..10_000)
        .find(|seed| {
            let [trip, target, morphed] = trip_morph_draws(*seed, 20, 3);
            trip % 2 == 0 && trip >= target && morphed % 2 == 1
        })
        .expect("an even Trip roll with an odd morphed reroll");
    scenario()
        .attacker("^tm20:1")
        .attacks(TRIP)
        .defender("3:1")
        .seed(seed)
        .expect_no_defender_dice()
        .expect_extra_turn(true)
        .run();
}

#[test]
fn radioactive_trip_target_decays_the_attacker_after_it_morphs() {
    scenario()
        .attacker("tm20:20")
        .attacks(TRIP)
        .defender("%4:1")
        .expect_no_defender_dice()
        .expect_attacker_dice(["tm2:2", "tm2:1"])
        .run();
}

#[test]
fn infinite_turbo_accuracy_offers_every_size_instead_of_hanging() {
    let mut game = native_fixture_game("game\nfight\nplayer 0 1 0\ntX!-4:1\nplayer 1 1 0\n20:3\n");
    game.m_turbo_accuracy = f32::INFINITY;
    let sizes = game
        .GenerateValidAttacksInCppOrder()
        .into_iter()
        .filter(|candidate| candidate.m_attack == Some(TRIP))
        .count();
    assert_eq!(sizes, 17);
}
