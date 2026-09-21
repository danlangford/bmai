// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

/// Ports PR #82's Chance Mighty/Weak/Maximum Konstant regressions.
#[test]
fn pr82_chance_effects_run_once_while_konstant_retains_value() {
    for (properties, expected_sides) in [
        (property::MIGHTY, 8),
        (property::WEAK, 4),
        (property::MAXIMUM, 6),
    ] {
        let mut game = BMC_Game::default();
        let mut chance = swing_die('P', property::CHANCE | property::KONSTANT | properties, 0);
        chance.m_sides[0] = 6;
        chance.m_value_total = Some(3);
        game.m_player[0].m_die = vec![chance];
        let mut opponent = swing_die('P', 0, 0);
        opponent.m_sides[0] = 20;
        opponent.m_value_total = Some(20);
        game.m_player[1].m_die = vec![opponent];

        let mut rng = BMC_RNG::default();
        rng.SRand(1);
        ApplyChanceMove(&mut game, 0, 1, &ChanceMove { reroll: vec![0] }, &mut rng);
        assert_eq!(game.m_player[0].m_die[0].GetValueTotal(), 3);
        assert_eq!(game.m_player[0].m_die[0].m_sides[0], expected_sides);
    }

    for (effect, expected_sides) in [(property::MIGHTY, 8), (property::WEAK, 4)] {
        let mut game = BMC_Game::default();
        let mut chance = swing_die('P', property::CHANCE | effect, 0);
        chance.m_sides[0] = 6;
        chance.m_value_total = Some(1);
        game.m_player[0].m_die = vec![chance];
        let mut opponent = swing_die('P', 0, 0);
        opponent.m_sides[0] = 20;
        opponent.m_value_total = Some(20);
        game.m_player[1].m_die = vec![opponent];
        let mut rng = BMC_RNG::default();
        rng.SRand(1);
        ApplyChanceMove(&mut game, 0, 1, &ChanceMove { reroll: vec![0] }, &mut rng);
        assert_eq!(game.m_player[0].m_die[0].m_sides[0], expected_sides);
    }
}

/// Ports the PR #82 Konstant Trip Mighty/Weak cases.
#[test]
fn pr82_konstant_trip_target_retains_value_and_changes_sides_once() {
    for (effect, expected_sides) in [(property::MIGHTY, 8), (property::WEAK, 4)] {
        let mut game = BMC_Game::default();
        let mut attacker = swing_die('P', property::TRIP, 0);
        attacker.m_sides[0] = 6;
        attacker.m_value_total = Some(6);
        let mut target = swing_die('P', property::KONSTANT | effect, 0);
        target.m_sides[0] = 6;
        target.m_value_total = Some(3);
        game.m_player[0].m_die = vec![attacker];
        game.m_player[1].m_die = vec![target];
        let action = BMC_Move::attack(TRIP, [0], [0], 0.0);

        let mut rng = BMC_RNG::default();
        rng.SRand(1);
        ApplyAttack(&mut game, &action, &mut rng);
        assert_eq!(game.m_player[1].m_die[0].GetValueTotal(), 3);
        assert_eq!(game.m_player[1].m_die[0].m_sides[0], expected_sides);
    }
}

/// Ports KonstantOrneryMighty/Weak and NonparticipatingOrneryDieRerolls.
#[test]
fn pr82_nonparticipating_ornery_effects_and_rolls_match_cpp() {
    for (effect, expected_sides) in [(property::MIGHTY, 8), (property::WEAK, 4)] {
        let mut game = BMC_Game::default();
        let mut attacker = swing_die('P', 0, 0);
        attacker.m_sides[0] = 6;
        attacker.m_value_total = Some(6);
        let mut ornery = swing_die('P', property::ORNERY | property::KONSTANT | effect, 1);
        ornery.m_sides[0] = 6;
        ornery.m_value_total = Some(3);
        let mut target = swing_die('P', 0, 0);
        target.m_sides[0] = 1;
        target.m_value_total = Some(1);
        game.m_player[0].m_die = vec![attacker, ornery];
        game.m_player[1].m_die = vec![target];

        ApplyAttack(
            &mut game,
            &BMC_Move::attack(POWER, [0], [0], 0.0),
            &mut BMC_RNG::default(),
        );
        let ornery = game.m_player[0]
            .m_die
            .iter()
            .find(|die| die.m_original_index == 1)
            .unwrap();
        assert_eq!(ornery.GetValueTotal(), 3);
        assert_eq!(ornery.m_sides[0], expected_sides);
    }

    let mut game = BMC_Game::default();
    let mut attacker = swing_die('P', 0, 0);
    attacker.m_sides[0] = 6;
    attacker.m_value_total = Some(6);
    let mut ornery = swing_die('P', property::ORNERY, 1);
    ornery.m_sides[0] = 100;
    ornery.m_value_total = Some(100);
    let mut target = swing_die('P', 0, 0);
    target.m_sides[0] = 1;
    target.m_value_total = Some(1);
    game.m_player[0].m_die = vec![attacker, ornery];
    game.m_player[1].m_die = vec![target];
    let mut rng = BMC_RNG::default();
    rng.SRand(1);
    ApplyAttack(&mut game, &BMC_Move::attack(POWER, [0], [0], 0.0), &mut rng);
    let ornery = game.m_player[0]
        .m_die
        .iter()
        .find(|die| die.m_original_index == 1)
        .unwrap();
    assert_ne!(ornery.GetValueTotal(), 100);
}

/// Ports OrneryMoodDoesNotChangeOnPass and the Konstant Mood attack case.
#[test]
fn pr82_ornery_mood_only_changes_on_an_attack() {
    let mood_die = || {
        let mut die = swing_die(
            'X',
            property::ORNERY | property::MOOD | property::KONSTANT,
            0,
        );
        die.m_sides[0] = 6;
        die.m_value_total = Some(3);
        die
    };

    let mut pass_game = BMC_Game::default();
    pass_game.m_player[0].m_die = vec![mood_die()];
    ApplyAttack(
        &mut pass_game,
        &BMC_Move {
            m_action: PASS,
            m_attack: None,
            m_attackers: BMC_DieIndexSet::default(),
            m_targets: BMC_DieIndexSet::default(),
            m_score: 0.0,
            m_turbo_option: -1,
            m_fire: crate::game::BMC_FireAdjustment::default(),
        },
        &mut BMC_RNG::default(),
    );
    assert_eq!(
        (
            pass_game.m_player[0].m_die[0].m_sides[0],
            pass_game.m_player[0].m_die[0].GetValueTotal()
        ),
        (6, 3)
    );

    let mut attack_game = BMC_Game::default();
    let mut attacker = swing_die('P', 0, 1);
    attacker.m_sides[0] = 6;
    attacker.m_value_total = Some(6);
    let mut target = swing_die('P', 0, 0);
    target.m_sides[0] = 1;
    target.m_value_total = Some(1);
    attack_game.m_player[0].m_die = vec![attacker, mood_die()];
    attack_game.m_player[1].m_die = vec![target];
    let mut rng = BMC_RNG::default();
    rng.SRand(3);
    ApplyAttack(
        &mut attack_game,
        &BMC_Move::attack(POWER, [0], [0], 0.0),
        &mut rng,
    );
    let mood = attack_game.m_player[0]
        .m_die
        .iter()
        .find(|die| die.m_original_index == 0)
        .unwrap();
    assert_ne!(mood.m_sides[0], 6);
    assert_eq!(mood.GetValueTotal(), 3);
}

/// Ports both Konstant Time-and-Space no-extra-turn cases.
#[test]
fn pr82_konstant_time_and_space_never_grants_extra_turn() {
    for attack in [TRIP, SKILL] {
        let mut game = BMC_Game::default();
        let mut konstant = swing_die(
            'P',
            property::KONSTANT
                | property::TIME_AND_SPACE
                | if attack == TRIP { property::TRIP } else { 0 },
            0,
        );
        konstant.m_sides[0] = 6;
        konstant.m_value_total = Some(3);
        game.m_player[0].m_die = vec![konstant];
        let mut target = swing_die('P', 0, 0);
        target.m_sides[0] = 1;
        target.m_value_total = Some(1);
        game.m_player[1].m_die = vec![target];
        let action = if attack == TRIP {
            BMC_Move::attack(attack, [0], [0], 0.0)
        } else {
            let mut ordinary = swing_die('P', 0, 1);
            ordinary.m_sides[0] = 2;
            ordinary.m_value_total = Some(2);
            game.m_player[0].m_die.push(ordinary);
            game.m_player[1].m_die[0].m_sides[0] = 5;
            game.m_player[1].m_die[0].m_value_total = Some(5);
            BMC_Move::attack(attack, [0, 1], [0], 0.0)
        };
        assert!(!ApplyAttack(&mut game, &action, &mut BMC_RNG::default()));
    }
}

/// Ports TimeAndSpaceOddRerollGrantsExtraTurn.
#[test]
fn pr82_ordinary_time_and_space_uses_its_rerolled_value() {
    let mut game = BMC_Game::default();
    let mut attacker = swing_die('P', property::TIME_AND_SPACE, 0);
    attacker.m_sides[0] = 6;
    attacker.m_value_total = Some(1);
    let mut target = swing_die('P', 0, 0);
    target.m_sides[0] = 1;
    target.m_value_total = Some(1);
    game.m_player[0].m_die = vec![attacker];
    game.m_player[1].m_die = vec![target];
    let mut rng = BMC_RNG::default();
    rng.SRand(3);
    let extra_turn = ApplyAttack(&mut game, &BMC_Move::attack(POWER, [0], [0], 0.0), &mut rng);
    assert_eq!(game.m_player[0].m_die[0].GetValueTotal() % 2, 1);
    assert!(extra_turn);
}
