// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[test]
fn attacking_jolt_grants_an_extra_turn_and_loses_jolt() {
    scenario()
        .attacker("JM6:6")
        .attacks(Power)
        .defender("1:1")
        .expect_extra_turn(true)
        .expect_attacker_dice(["M6:6"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn consumed_jolt_returns_at_the_start_of_the_next_round() {
    let mut template = Game::default();
    let mut jolt = swing_die('P', property::JOLT, 0);
    jolt.m_sides[0] = 6;
    jolt.m_value_total = Some(6);
    template.m_player[0].m_die = vec![jolt];

    let mut game = template.clone();
    game.m_player[0].m_die[0].m_properties &= !property::JOLT;
    RestoreDiceForNewRound(&mut game, &template);

    assert!(game.m_player[0].m_die[0].HasProperty(property::JOLT));
}

#[test]
fn every_attacking_jolt_loses_the_skill_but_grants_only_one_extra_turn() {
    scenario()
        .attackers(["JM2:2", "JM3:3"])
        .attacks(Skill)
        .using([0, 1])
        .defender("5:5")
        .expect_extra_turn(true)
        .expect_attacker_dice(["M3:3", "M2:2"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn nonparticipating_jolt_keeps_the_skill_and_does_not_grant_an_extra_turn() {
    scenario()
        .attackers(["M6:6", "JM4:4"])
        .attacks(Power)
        .using([0])
        .defender("1:1")
        .expect_extra_turn(false)
        .expect_attacker_dice(["M6:6", "MJ4:4"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn capturing_a_jolt_die_grants_the_capturer_an_extra_turn() {
    scenario()
        .attacker("M6:6")
        .attacks(Power)
        .defender("J1:1")
        .expect_extra_turn(true)
        .expect_no_defender_dice()
        .expect_captured_defender_dice(["J1:1"])
        .run();
}

#[test]
fn unsuccessful_jolt_trip_still_grants_an_extra_turn_and_consumes_jolt() {
    scenario()
        .attacker("tJ6:6")
        .attacks(Trip)
        .defender("6:6")
        .seed(4)
        .expect_extra_turn(true)
        .expect_attacker_dice(["t6:1"])
        .expect_defender_dice(["6:3"])
        .run();
}

#[test]
fn unsuccessful_trip_does_not_trigger_defending_jolt() {
    scenario()
        .attacker("t6:6")
        .attacks(Trip)
        .defender("J6:6")
        .seed(4)
        .expect_extra_turn(false)
        .expect_attacker_dice(["t6:1"])
        .expect_defender_dice(["J6:3"])
        .run();
}

#[test]
fn unsuccessful_time_and_space_trip_grants_an_extra_turn_after_odd_reroll() {
    scenario()
        .attacker("t^6:6")
        .attacks(Trip)
        .defender("6:6")
        .seed(4)
        .expect_extra_turn(true)
        .expect_attacker_dice(["^t6:1"])
        .expect_defender_dice(["6:3"])
        .run();
}

#[test]
fn jolt_and_time_and_space_still_produce_only_one_extra_turn() {
    scenario()
        .attacker("J^1:1")
        .attacks(Power)
        .defender("1:1")
        .expect_extra_turn(true)
        .expect_attacker_dice(["^1:1"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn konstant_jolt_grants_an_extra_turn_without_rerolling() {
    scenario()
        .attackers(["Jk6:3", "2:2"])
        .attacks(Skill)
        .using([0, 1])
        .defender("5:5")
        .expect_extra_turn(true)
        .expect_attacker_dice(["k6:3", "2:1"])
        .expect_no_defender_dice()
        .run();
}

#[test]
fn pr82_konstant_attack_side_changes_preserve_value() {
    let mut morph_game = Game::default();
    let mut morph = swing_die('P', property::MORPHING | property::KONSTANT, 0);
    morph.m_sides[0] = 9;
    morph.m_value_total = Some(6);
    let mut ordinary = swing_die('P', 0, 1);
    ordinary.m_sides[0] = 1;
    ordinary.m_value_total = Some(1);
    let mut target = swing_die('P', 0, 0);
    target.m_sides[0] = 7;
    target.m_value_total = Some(7);
    morph_game.m_player[0].m_die = vec![morph, ordinary];
    morph_game.m_player[1].m_die = vec![target];
    ApplyAttack(
        &mut morph_game,
        &Move::attack(Skill, [0, 1], [0], 0.0),
        &mut Rng::default(),
    );
    let morph = morph_game.m_player[0]
        .m_die
        .iter()
        .find(|die| die.m_original_index == 0)
        .unwrap();
    assert_eq!((morph.m_sides[0], morph.GetValueTotal()), (7, 6));

    let mut berserk_game = Game::default();
    let mut berserk = swing_die('P', property::BERSERK | property::KONSTANT, 0);
    berserk.m_sides[0] = 9;
    berserk.m_value_total = Some(8);
    let mut first = swing_die('P', 0, 0);
    first.m_sides[0] = 3;
    first.m_value_total = Some(3);
    let mut second = swing_die('P', 0, 1);
    second.m_sides[0] = 5;
    second.m_value_total = Some(5);
    berserk_game.m_player[0].m_die = vec![berserk];
    berserk_game.m_player[1].m_die = vec![first, second];
    ApplyAttack(
        &mut berserk_game,
        &Move::attack(Berserk, [0], [0, 1], 0.0),
        &mut Rng::default(),
    );
    let berserk = &berserk_game.m_player[0].m_die[0];
    assert_eq!((berserk.m_sides[0], berserk.GetValueTotal()), (5, 8));
    assert!(!berserk.HasProperty(property::BERSERK));
}
