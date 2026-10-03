// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[test]
fn unique_rejects_equal_values_on_lower_swing_types() {
    let player = Player {
        m_die: vec![swing_die('P', 0, 0), swing_die('Q', property::UNIQUE, 1)],
        ..Default::default()
    };

    let moves = GenerateSwingMoves(&player);
    assert_eq!(moves.len(), 30 * 19 - 19);
    assert!(moves.iter().all(|candidate| {
        let p = candidate
            .values()
            .iter()
            .find(|(swing, _)| *swing == 'P')
            .unwrap()
            .1;
        let q = candidate
            .values()
            .iter()
            .find(|(swing, _)| *swing == 'Q')
            .unwrap()
            .1;
        p != q
    }));
}

#[test]
fn turbo_swing_changes_all_matching_dice_before_the_reroll() {
    let mut game = Game::default();
    let mut turbo = swing_die('X', property::TURBO | property::KONSTANT, 0);
    turbo.m_sides[0] = 10;
    turbo.m_value_total = Some(10);
    let mut companion = swing_die('X', property::KONSTANT, 1);
    companion.m_sides[0] = 10;
    companion.m_value_total = Some(7);
    game.m_player[0].m_die = vec![turbo, companion];
    let mut target = swing_die('P', 0, 0);
    target.m_sides[0] = 6;
    target.m_value_total = Some(6);
    game.m_player[1].m_die = vec![target];
    let action = Move {
        m_action: Attack,
        m_attack: Some(Power),
        m_attackers: vec![0].into(),
        m_targets: vec![0].into(),
        m_score: 0.0,
        m_turbo_option: 20,
        m_fire: crate::game::FireAdjustment::default(),
    };

    let mut rng = Rng::default();
    ApplyAttack(&mut game, &action, &mut rng);
    assert!(
        game.m_player[0]
            .m_die
            .iter()
            .all(|die| die.m_swing_type[0] != Some('X') || die.m_sides[0] == 20)
    );
}

#[test]
fn pr82_trip_target_before_roll_effect_triggers_once() {
    for (effect, starting_sides, expected_sides) in
        [(property::MIGHTY, 6, 8), (property::WEAK, 20, 16)]
    {
        let mut game = Game::default();
        let mut attacker = swing_die('P', property::TRIP | property::KONSTANT, 0);
        attacker.m_sides[0] = 4;
        attacker.m_value_total = Some(1);
        game.m_player[0].m_die = vec![attacker];
        let mut target = swing_die('P', effect, 0);
        target.m_sides[0] = starting_sides;
        target.m_value_total = Some(starting_sides);
        game.m_player[1].m_die = vec![target];
        let action = Move::attack(Trip, [0], [0], 0.0);

        ApplyAttack(&mut game, &action, &mut Rng::default());
        assert_eq!(game.m_player[1].m_die[0].m_sides[0], expected_sides);
    }
}

#[test]
fn pr82_participating_ornery_before_roll_effect_triggers_once() {
    let mut game = Game::default();
    let mut attacker = swing_die('P', property::ORNERY | property::MIGHTY, 0);
    attacker.m_sides[0] = 4;
    attacker.m_value_total = Some(4);
    game.m_player[0].m_die = vec![attacker];
    let mut target = swing_die('P', 0, 0);
    target.m_sides[0] = 1;
    target.m_value_total = Some(1);
    game.m_player[1].m_die = vec![target];
    let action = Move {
        m_action: Attack,
        m_attack: Some(Power),
        m_attackers: vec![0].into(),
        m_targets: vec![0].into(),
        m_score: 0.0,
        m_turbo_option: -1,
        m_fire: crate::game::FireAdjustment::default(),
    };

    ApplyAttack(&mut game, &action, &mut Rng::default());
    assert_eq!(game.m_player[0].m_die[0].m_sides[0], 6);
}

#[test]
fn pr82_ordinary_side_change_invalidates_value() {
    let mut game = Game::default();
    let mut attacker = swing_die('P', property::MIGHTY, 0);
    attacker.m_sides[0] = 6;
    attacker.m_value_total = Some(3);
    let mut target = swing_die('P', 0, 0);
    target.m_sides[0] = 1;
    target.m_value_total = Some(1);
    game.m_player[0].m_die = vec![attacker];
    game.m_player[1].m_die = vec![target];
    let action = Move::attack(Power, [0], [0], 0.0);

    ApplyAttackPlayerEffects(&mut game, &action, 0, 1, 0, true);
    assert!(game.m_player[0].m_die[0].m_notset);
    assert_eq!(game.m_player[0].m_die[0].m_sides[0], 8);
}

#[test]
fn copied_cpp_konstant_skill_attacker_keeps_its_value() {
    let mut game = Game::default();
    let mut konstant = swing_die('P', property::KONSTANT, 0);
    konstant.m_sides[0] = 20;
    konstant.m_value_total = Some(13);
    let mut ordinary = swing_die('P', 0, 1);
    ordinary.m_sides[0] = 7;
    ordinary.m_value_total = Some(7);
    game.m_player[0].m_die = vec![konstant, ordinary];
    let mut target = swing_die('P', 0, 0);
    target.m_sides[0] = 20;
    target.m_value_total = Some(20);
    game.m_player[1].m_die = vec![target];
    let action = Move {
        m_action: Attack,
        m_attack: Some(Skill),
        m_attackers: vec![0, 1].into(),
        m_targets: vec![0].into(),
        m_score: 0.0,
        m_turbo_option: -1,
        m_fire: crate::game::FireAdjustment::default(),
    };

    ApplyAttack(&mut game, &action, &mut Rng::default());
    assert_eq!(
        game.m_player[0]
            .m_die
            .iter()
            .find(|die| die.m_original_index == 0)
            .unwrap()
            .GetValueTotal(),
        13
    );
}

#[test]
fn copied_cpp_multi_target_speed_attack_does_not_morph() {
    let mut game = Game::default();
    let mut attacker = swing_die('P', property::MORPHING | property::SPEED, 0);
    attacker.m_sides[0] = 10;
    attacker.m_value_total = Some(8);
    game.m_player[0].m_die = vec![attacker];
    let mut first = swing_die('P', 0, 0);
    first.m_sides[0] = 4;
    first.m_value_total = Some(3);
    let mut second = swing_die('P', 0, 1);
    second.m_sides[0] = 6;
    second.m_value_total = Some(5);
    game.m_player[1].m_die = vec![first, second];
    let action = Move {
        m_action: Attack,
        m_attack: Some(Speed),
        m_attackers: vec![0].into(),
        m_targets: vec![0, 1].into(),
        m_score: 0.0,
        m_turbo_option: -1,
        m_fire: crate::game::FireAdjustment::default(),
    };

    ApplyAttack(&mut game, &action, &mut Rng::default());
    assert_eq!(game.m_player[0].m_die[0].m_sides[0], 10);
}

#[test]
fn copied_cpp_konstant_chance_die_keeps_its_value() {
    let mut game = Game::default();
    let mut chance = swing_die('P', property::CHANCE | property::KONSTANT, 0);
    chance.m_sides[0] = 100;
    chance.m_value_total = Some(7);
    game.m_player[0].m_die = vec![chance];
    let mut opponent = swing_die('P', 0, 0);
    opponent.m_sides[0] = 20;
    opponent.m_value_total = Some(20);
    game.m_player[1].m_die = vec![opponent];

    ApplyChanceMove(
        &mut game,
        0,
        1,
        &ChanceMove { reroll: vec![0] },
        &mut Rng::default(),
    );
    assert_eq!(game.m_player[0].m_die[0].GetValueTotal(), 7);
}

/// C++'s `initiative != 0` check makes Chance asymmetric by player index.
#[test]
fn cpp_chance_success_is_keyed_to_player_zero_initiative() {
    let mut game = Game::default();
    let mut zero = swing_die('P', 0, 0);
    zero.m_sides[0] = 20;
    zero.m_value_total = Some(5);
    let mut one = swing_die('P', property::CHANCE | property::KONSTANT, 0);
    one.m_sides[0] = 20;
    one.m_value_total = Some(6);
    game.m_player[0].m_die = vec![zero];
    game.m_player[1].m_die = vec![one];

    let result = ApplyChanceMove(
        &mut game,
        1,
        0,
        &ChanceMove { reroll: vec![0] },
        &mut Rng::default(),
    );
    assert_eq!(CheckInitiative(&game), Some(0));
    assert_eq!(result, (1, true));
}

#[test]
fn cpp_focus_marks_dice_dizzy_until_turn_recovery() {
    let mut game = Game::default();
    let mut focus = swing_die('P', property::FOCUS, 0);
    focus.m_sides[0] = 20;
    focus.m_value_total = Some(12);
    game.m_player[0].m_die = vec![focus];

    ApplyFocusMove(
        &mut game,
        0,
        &FocusMove {
            values: vec![(0, 7)],
        },
    );
    assert_eq!(game.m_player[0].m_die[0].GetValueTotal(), 7);
    assert!(game.m_player[0].m_die[0].m_dizzy);
    RecoverDizzyDice(&mut game.m_player[0]);
    assert!(!game.m_player[0].m_die[0].m_dizzy);
}

#[test]
fn cpp_value_attacker_score_retains_its_pre_reroll_value() {
    scenario()
        .attacker("v20:15")
        .attacks(Power)
        .defender("6:5")
        .with_scores(7.5, 3.0)
        .seed(1)
        .expect_scores(12.5, 0.0)
        .expect_attacker_dice(["v20:20"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn cpp_maximum_die_always_rolls_its_maximum() {
    let mut maximum = swing_die('P', property::MAXIMUM, 0);
    maximum.m_sides[0] = 6;
    for seed in 1..=10 {
        maximum.m_value_total = None;
        maximum.m_notset = true;
        let mut rng = Rng::default();
        rng.SRand(seed);
        RollDie(&mut maximum, &mut rng);
        assert_eq!(maximum.GetValueTotal(), 6);
    }
}

#[test]
#[should_panic(expected = "Die::Roll requires NOTSET state")]
fn cpp_roll_requires_notset_state() {
    let mut die = swing_die('P', 0, 0);
    die.m_sides[0] = 6;
    die.m_value_total = Some(1);
    die.m_notset = false;
    RollDie(&mut die, &mut Rng::default());
}

#[test]
#[should_panic(expected = "Die::OnSwingSet requires NOTSET state")]
fn cpp_swing_set_requires_notset_state() {
    let mut die = swing_die('X', 0, 0);
    die.m_sides[0] = 6;
    die.m_value_total = Some(1);
    die.m_notset = false;
    let mut player = crate::game::Player {
        m_die: vec![die],
        ..Default::default()
    };
    let mut action = SwingMove::empty();
    action.push_value(('X', 8));
    ApplySwingMove(&mut player, &action);
}

#[test]
fn cpp_konstant_target_retains_value_when_tripped() {
    scenario()
        .attacker("kt8:8")
        .attacks(Trip)
        .defender("k100:7")
        .seed(1)
        .expect_no_defender_dice()
        .expect_captured_defender_dice(["k100:7"])
        .run();
}

#[test]
fn cpp_konstant_warrior_keeps_value_and_loses_warrior_after_skill() {
    scenario()
        .attackers(["`k41:17", "11:11"])
        .attacks(Skill)
        .using([0, 1])
        .defender("20:28")
        .seed(1)
        .expect_attacker_die(0, "k41:17")
        .expect_no_defender_dice()
        .run();
}

#[test]
fn cpp_morphing_copies_single_and_twin_target_sizes() {
    let cases = [([9, 0], [7, 0], [7, 0]), ([7, 0], [10, 11], [10, 11])];
    for (attacker_sides, target_sides, expected) in cases {
        let mut game = Game::default();
        let mut attacker = swing_die('P', property::MORPHING, 0);
        attacker.m_sides = attacker_sides;
        attacker.m_value_total = Some(8);
        if attacker_sides[1] > 0 {
            attacker.m_properties |= property::TWIN;
        }
        let mut target = swing_die('P', property::MORPHING, 0);
        target.m_sides = target_sides;
        target.m_value_total = Some(6);
        if target_sides[1] > 0 {
            target.m_properties |= property::TWIN;
        }
        game.m_player[0].m_die = vec![attacker];
        game.m_player[1].m_die = vec![target];
        let action = Move {
            m_action: Attack,
            m_attack: Some(Power),
            m_attackers: vec![0].into(),
            m_targets: vec![0].into(),
            m_score: 0.0,
            m_turbo_option: -1,
            m_fire: crate::game::FireAdjustment::default(),
        };
        ApplyAttack(&mut game, &action, &mut Rng::default());
        assert_eq!(game.m_player[0].m_die[0].m_sides, expected);
        assert_eq!(
            game.m_player[0].m_die[0].HasProperty(property::TWIN),
            expected[1] > 0
        );
    }
}
