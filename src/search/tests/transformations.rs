// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;
use crate::BME_ATTACK::SHADOW;
use crate::game::ApplyRadioactiveDecay;

#[test]
fn radioactive_doppelganger_can_transfer_the_full_twenty_die_pool() {
    let mut template = BMC_Game::default();
    for original_index in 0..10 {
        let mut attacker = swing_die(
            'P',
            property::RADIOACTIVE | property::DOPPELGANGER,
            original_index,
        );
        attacker.m_sides = [20, 0];
        attacker.m_value_total = Some(20);
        template.m_player[0].m_die.push(attacker);

        let mut target = swing_die('P', 0, original_index);
        target.m_sides = [1, 0];
        target.m_value_total = Some(1);
        template.m_player[1].m_die.push(target);
    }
    let mut game = template.clone();

    for _ in 0..10 {
        let attacker = game.m_player[0]
            .m_die
            .iter()
            .position(|die| die.HasProperty(property::RADIOACTIVE | property::DOPPELGANGER))
            .expect("an untransformed Radioactive Doppelganger remains");
        let action = BMC_Move::attack(POWER, [attacker], [0], 0.0);
        apply_generated_attack(&mut game, &action, &mut BMC_RNG::default());
    }

    assert_eq!(game.m_player[0].m_die.len(), 20);
    assert!(game.m_player[1].m_die.iter().all(|die| die.m_captured));
    RestoreDiceForNewRound(&mut game, &template);
    assert_eq!(game.m_player[0].m_die.len(), 10);
}

#[test]
#[should_panic(
    expected = "Radioactive+Doppelganger decay exceeds the transformed dice capacity of 20"
)]
fn radioactive_doppelganger_reports_transformed_capacity_exhaustion() {
    let mut game = BMC_Game::default();
    for original_index in 0..20 {
        let properties = if original_index == 0 {
            property::RADIOACTIVE | property::DOPPELGANGER
        } else {
            0
        };
        game.m_player[0]
            .m_die
            .push(swing_die('P', properties, original_index));
    }
    game.m_player[1].m_die.push(swing_die('P', 0, 0));
    let action = BMC_Move::attack(POWER, [0], [0], 0.0);

    ApplyRadioactiveDecay(&mut game, &action, 0, 1);
}

#[test]
fn radioactive_doppelganger_decay_is_limited_to_power_attacks() {
    let mut game = BMC_Game::default();
    game.m_player[0].m_die.push(swing_die(
        'P',
        property::RADIOACTIVE | property::DOPPELGANGER | property::SHADOW,
        0,
    ));
    game.m_player[1].m_die.push(swing_die('P', 0, 0));
    let action = BMC_Move::attack(SHADOW, [0], [0], 0.0);

    assert!(ApplyRadioactiveDecay(&mut game, &action, 0, 1).is_none());
    assert_eq!(game.m_player[0].m_die.len(), 1);
}

#[test]
fn standalone_radioactive_remains_parsing_only() {
    let mut game = BMC_Game::default();
    let mut attacker = swing_die('P', property::RADIOACTIVE, 0);
    attacker.m_sides = [20, 0];
    attacker.m_value_total = Some(20);
    game.m_player[0].m_die = vec![attacker];
    let mut target = swing_die('P', 0, 0);
    target.m_sides = [2, 0];
    target.m_value_total = Some(2);
    game.m_player[1].m_die = vec![target];
    let action = BMC_Move::attack(POWER, [0], [0], 0.0);

    assert!(ApplyRadioactiveDecay(&mut game, &action, 0, 1).is_none());
    assert_eq!(game.m_player[0].m_die.len(), 1);
}

#[test]
fn doppelganger_recipe_returns_at_the_start_of_the_next_round() {
    let mut template = BMC_Game::default();
    let mut original = swing_die('P', property::DOPPELGANGER | property::TRIP, 0);
    original.m_sides = [20, 0];
    original.m_value_total = Some(20);
    template.m_player[0].m_die = vec![original];
    let mut target = swing_die('V', property::POISON | property::TWIN, 0);
    target.m_sides = [8, 10];
    target.m_swing_type = [Some('V'), Some('X')];
    target.m_value_total = Some(12);
    template.m_player[1].m_die = vec![target];

    let mut game = template.clone();
    game.m_player[0].m_die[0].m_sides = [18, 0];
    let action = BMC_Move::attack(POWER, [0], [0], 0.0);
    apply_generated_attack(&mut game, &action, &mut BMC_RNG::default());
    assert_eq!(game.m_player[0].m_die[0].m_sides, [8, 10]);

    RestoreDiceForNewRound(&mut game, &template);

    assert_eq!(game.m_player[0].m_die[0].m_sides, [18, 0]);
    assert_eq!(game.m_player[0].m_die[0].m_swing_type, [Some('P'), None]);
    assert!(game.m_player[0].m_die[0].HasProperty(property::DOPPELGANGER | property::TRIP));
    assert!(!game.m_player[0].m_die[0].HasProperty(property::POISON | property::TWIN));
}

#[test]
fn repeated_doppelganger_captures_restore_the_rounds_original_recipe() {
    let mut template = BMC_Game::default();
    let mut original = swing_die('P', property::DOPPELGANGER, 0);
    original.m_sides = [18, 0];
    original.m_value_total = Some(18);
    template.m_player[0].m_die = vec![original];
    let mut first_target = swing_die('P', property::DOPPELGANGER | property::MAXIMUM, 0);
    first_target.m_sides = [12, 0];
    first_target.m_value_total = Some(12);
    let mut second_target = swing_die('P', property::POISON, 1);
    second_target.m_sides = [6, 0];
    second_target.m_value_total = Some(6);
    template.m_player[1].m_die = vec![first_target, second_target];

    let mut game = template.clone();
    let action = BMC_Move::attack(POWER, [0], [0], 0.0);
    apply_generated_attack(&mut game, &action, &mut BMC_RNG::default());
    assert!(game.m_player[0].m_die[0].HasProperty(property::DOPPELGANGER));
    apply_generated_attack(&mut game, &action, &mut BMC_RNG::default());
    assert!(game.m_player[0].m_die[0].HasProperty(property::POISON));

    RestoreDiceForNewRound(&mut game, &template);

    assert_eq!(game.m_player[0].m_die[0].m_sides, [18, 0]);
    assert!(game.m_player[0].m_die[0].HasProperty(property::DOPPELGANGER));
    assert!(!game.m_player[0].m_die[0].HasProperty(property::POISON));
}

#[test]
fn doppelganger_round_reset_does_not_preserve_mighty_side_changes() {
    let mut template = BMC_Game::default();
    let mut original = swing_die('P', property::DOPPELGANGER | property::MIGHTY, 0);
    original.m_swing_type = [None, None];
    original.m_sides = [6, 0];
    original.m_value_total = Some(6);
    template.m_player[0].m_die = vec![original];
    let mut target = swing_die('P', property::MAXIMUM, 0);
    target.m_swing_type = [None, None];
    target.m_sides = [4, 0];
    target.m_value_total = Some(4);
    template.m_player[1].m_die = vec![target];

    let mut game = template.clone();
    let action = BMC_Move::attack(POWER, [0], [0], 0.0);
    apply_generated_attack(&mut game, &action, &mut BMC_RNG::default());
    RestoreDiceForNewRound(&mut game, &template);

    assert_eq!(game.m_player[0].m_die[0].m_sides, [6, 0]);
    assert!(game.m_player[0].m_die[0].HasProperty(property::DOPPELGANGER | property::MIGHTY));
}
