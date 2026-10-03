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
fn no_skill_attacks_removes_only_skill_attacks() {
    for (attack, allowed) in [(SKILL, false), (POWER, true)] {
        scenario()
            .attackers(["6:3", "6:4"])
            .attacker_special("no_skill_attacks")
            .attacks(attack)
            .using(if attack == SKILL { vec![0, 1] } else { vec![1] })
            .defender(if attack == SKILL { "8:7" } else { "8:4" })
            .expect_allowed(allowed)
            .run();
    }
}

#[test]
fn skill_immune_dice_cannot_be_skill_attacked() {
    for (attack, allowed) in [(SKILL, false), (POWER, true)] {
        scenario()
            .attackers(["6:3", "6:4"])
            .attacks(attack)
            .using(if attack == SKILL { vec![0, 1] } else { vec![1] })
            .defender(if attack == SKILL { "8:7" } else { "8:4" })
            .defender_special("skill_immune")
            .expect_allowed(allowed)
            .run();
    }
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
fn unique_sizes_declines_an_auxiliary_swing_die_and_so_do_both_players() {
    let gordo = |auxiliary: &str| {
        parsed(&format!(
            "game 3\naux\nplayer 0 2 0\nV\n{auxiliary}\nplayer 1 2 0\n8\n+6\nspecial 0 unique_sizes\nquit\n"
        ))
        .m_game
    };
    assert_eq!(SelectQAIAuxiliaryAction(&gordo("+X")), None);
    assert_eq!(SelectQAIAuxiliaryAction(&gordo("+4")), Some(1));

    let mut game = gordo("+X");
    ApplyAuxiliaryDecision(&mut game, true);
    assert_eq!(game.m_player[0].m_die.len(), 1);
    assert_eq!(game.m_player[1].m_die.len(), 1);
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
fn trip_must_be_able_to_reach_a_konstant_or_maximum_target() {
    for (target, allowed) in [
        ("k6:5", false),
        ("k6:4", true),
        ("M6:2", false),
        ("M4:2", true),
    ] {
        scenario()
            .attacker("t4:1")
            .attacks(TRIP)
            .defender(target)
            .expect_allowed(allowed)
            .run();
    }
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

#[test]
fn turbo_trip_sizes_too_small_for_the_target_are_not_offered() {
    let sizes = attacks_by(&["tP!-4:1"], &["(3,3):4"])
        .into_iter()
        .filter(|candidate| candidate.m_attack == Some(TRIP))
        .map(|candidate| candidate.m_turbo_option)
        .collect::<Vec<_>>();
    assert!(!sizes.contains(&1), "{sizes:?}");
    assert!(sizes.contains(&2) && sizes.contains(&30), "{sizes:?}");
}

// Morphing follows `BMSkillMorphing::capture`: any single-target attack, and
// never after a failed one.

#[test]
fn single_target_berserk_and_speed_attacks_morph() {
    for (attack, attacker) in [(BERSERK, "mB12:6"), (SPEED, "mz12:6")] {
        scenario()
            .attacker(attacker)
            .attacks(attack)
            .defender("20:6")
            .expect_attacker_dice([if attack == BERSERK { "m20:1" } else { "zm20:1" }])
            .run();
    }
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

#[test]
fn successful_trip_rolls_at_its_own_size_then_morphs_and_rerolls() {
    // A d20 Trip roll beats a d2 target almost always; the default seed does.
    scenario()
        .attacker("tm20:1")
        .attacks(TRIP)
        .defender("2:1")
        .expect_no_defender_dice()
        .expect_attacker_dice(["tm2:1"])
        .run();
}
