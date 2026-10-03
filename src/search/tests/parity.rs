// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[test]
fn pr82_chance_effects_run_once_while_konstant_retains_value() {
    for (properties, expected_sides) in [
        (property::MIGHTY, 8),
        (property::WEAK, 4),
        (property::MAXIMUM, 6),
    ] {
        let mut game = Game::default();
        let mut chance = swing_die('P', property::CHANCE | property::KONSTANT | properties, 0);
        chance.m_sides[0] = 6;
        chance.m_value_total = Some(3);
        game.m_player[0].m_die = vec![chance];
        let mut opponent = swing_die('P', 0, 0);
        opponent.m_sides[0] = 20;
        opponent.m_value_total = Some(20);
        game.m_player[1].m_die = vec![opponent];

        let mut rng = Rng::default();
        rng.SRand(1);
        ApplyChanceMove(&mut game, 0, 1, &ChanceMove { reroll: vec![0] }, &mut rng);
        assert_eq!(game.m_player[0].m_die[0].GetValueTotal(), 3);
        assert_eq!(game.m_player[0].m_die[0].m_sides[0], expected_sides);
    }

    for (effect, expected_sides) in [(property::MIGHTY, 8), (property::WEAK, 4)] {
        let mut game = Game::default();
        let mut chance = swing_die('P', property::CHANCE | effect, 0);
        chance.m_sides[0] = 6;
        chance.m_value_total = Some(1);
        game.m_player[0].m_die = vec![chance];
        let mut opponent = swing_die('P', 0, 0);
        opponent.m_sides[0] = 20;
        opponent.m_value_total = Some(20);
        game.m_player[1].m_die = vec![opponent];
        let mut rng = Rng::default();
        rng.SRand(1);
        ApplyChanceMove(&mut game, 0, 1, &ChanceMove { reroll: vec![0] }, &mut rng);
        assert_eq!(game.m_player[0].m_die[0].m_sides[0], expected_sides);
    }
}

#[test]
fn pr82_konstant_trip_target_retains_value_and_changes_sides_once() {
    for (effect, expected_sides) in [(property::MIGHTY, 8), (property::WEAK, 4)] {
        let mut game = Game::default();
        let mut attacker = swing_die('P', property::TRIP, 0);
        attacker.m_sides[0] = 6;
        attacker.m_value_total = Some(6);
        let mut target = swing_die('P', property::KONSTANT | effect, 0);
        target.m_sides[0] = 6;
        target.m_value_total = Some(3);
        game.m_player[0].m_die = vec![attacker];
        game.m_player[1].m_die = vec![target];
        let action = Move::attack(Trip, [0], [0], 0.0);

        let mut rng = Rng::default();
        rng.SRand(1);
        ApplyAttack(&mut game, &action, &mut rng);
        assert_eq!(game.m_player[1].m_die[0].GetValueTotal(), 3);
        assert_eq!(game.m_player[1].m_die[0].m_sides[0], expected_sides);
    }
}

#[test]
fn pr82_nonparticipating_ornery_effects_and_rolls_match_cpp() {
    for (effect, expected_sides) in [(property::MIGHTY, 8), (property::WEAK, 4)] {
        let mut game = Game::default();
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
            &Move::attack(Power, [0], [0], 0.0),
            &mut Rng::default(),
        );
        let ornery = game.m_player[0]
            .m_die
            .iter()
            .find(|die| die.m_original_index == 1)
            .unwrap();
        assert_eq!(ornery.GetValueTotal(), 3);
        assert_eq!(ornery.m_sides[0], expected_sides);
    }

    let mut game = Game::default();
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
    let mut rng = Rng::default();
    rng.SRand(1);
    ApplyAttack(&mut game, &Move::attack(Power, [0], [0], 0.0), &mut rng);
    let ornery = game.m_player[0]
        .m_die
        .iter()
        .find(|die| die.m_original_index == 1)
        .unwrap();
    assert_ne!(ornery.GetValueTotal(), 100);
}

fn ornery_mood_die_after(action: crate::Action, properties: u64) -> (u8, u16) {
    let mut die = swing_die('X', property::ORNERY | property::MOOD | properties, 1);
    die.m_sides[0] = 6;
    die.m_value_total = Some(3);
    let mut attacker = swing_die('P', 0, 0);
    attacker.m_sides[0] = 6;
    attacker.m_value_total = Some(6);
    let mut target = swing_die('P', 0, 0);
    target.m_sides[0] = 1;
    target.m_value_total = Some(1);
    let mut game = Game::default();
    game.m_player[0].m_die = vec![attacker, die];
    game.m_player[1].m_die = vec![target];
    let action = if action == Pass {
        Move {
            m_action: Pass,
            m_attack: None,
            m_attackers: DieIndexSet::default(),
            m_targets: DieIndexSet::default(),
            m_score: 0.0,
            m_turbo_option: -1,
            m_fire: crate::game::FireAdjustment::default(),
        }
    } else {
        Move::attack(Power, [0], [0], 0.0)
    };
    let mut rng = Rng::default();
    rng.SRand(3);
    ApplyAttack(&mut game, &action, &mut rng);
    let die = game.m_player[0]
        .m_die
        .iter()
        .find(|die| die.m_original_index == 1)
        .unwrap();
    (die.m_sides[0], die.GetValueTotal())
}

#[test]
fn ornery_mood_dice_change_after_an_attack_but_not_a_pass() {
    assert_eq!(ornery_mood_die_after(Pass, 0), (6, 3));
    assert_ne!(ornery_mood_die_after(Attack, 0), (6, 3));
}

#[test]
fn konstant_ornery_mood_die_keeps_its_size_and_value() {
    // Engine probe: Konstant blocks Mood's resize (`doesReroll`).
    assert_eq!(ornery_mood_die_after(Pass, property::KONSTANT), (6, 3));
    assert_eq!(ornery_mood_die_after(Attack, property::KONSTANT), (6, 3));
}

#[test]
fn pr82_konstant_time_and_space_never_grants_extra_turn() {
    for attack in [Trip, Skill] {
        let mut game = Game::default();
        let mut konstant = swing_die(
            'P',
            property::KONSTANT
                | property::TIME_AND_SPACE
                | if attack == Trip { property::TRIP } else { 0 },
            0,
        );
        konstant.m_sides[0] = 6;
        konstant.m_value_total = Some(3);
        game.m_player[0].m_die = vec![konstant];
        let mut target = swing_die('P', 0, 0);
        target.m_sides[0] = 1;
        target.m_value_total = Some(1);
        game.m_player[1].m_die = vec![target];
        let action = if attack == Trip {
            Move::attack(attack, [0], [0], 0.0)
        } else {
            let mut ordinary = swing_die('P', 0, 1);
            ordinary.m_sides[0] = 2;
            ordinary.m_value_total = Some(2);
            game.m_player[0].m_die.push(ordinary);
            game.m_player[1].m_die[0].m_sides[0] = 5;
            game.m_player[1].m_die[0].m_value_total = Some(5);
            Move::attack(attack, [0, 1], [0], 0.0)
        };
        assert!(!ApplyAttack(&mut game, &action, &mut Rng::default()));
    }
}

#[test]
fn pr82_ordinary_time_and_space_uses_its_rerolled_value() {
    let mut game = Game::default();
    let mut attacker = swing_die('P', property::TIME_AND_SPACE, 0);
    attacker.m_sides[0] = 6;
    attacker.m_value_total = Some(1);
    let mut target = swing_die('P', 0, 0);
    target.m_sides[0] = 1;
    target.m_value_total = Some(1);
    game.m_player[0].m_die = vec![attacker];
    game.m_player[1].m_die = vec![target];
    let mut rng = Rng::default();
    rng.SRand(3);
    let extra_turn = ApplyAttack(&mut game, &Move::attack(Power, [0], [0], 0.0), &mut rng);
    assert_eq!(game.m_player[0].m_die[0].GetValueTotal() % 2, 1);
    assert!(extra_turn);
}
