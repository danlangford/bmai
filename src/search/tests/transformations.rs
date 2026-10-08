// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;
use crate::game::split_radioactive_attacker;

#[test]
fn radioactive_doppelganger_can_transfer_the_full_twenty_die_pool() {
    let mut template = Game::default();
    for original_index in 0..10 {
        let mut attacker = swing_die(
            'P',
            property::RADIOACTIVE | property::DOPPELGANGER,
            original_index,
        );
        attacker.sides = [20, 0];
        attacker.value = Some(20);
        template.players[0].dice.push(attacker);

        let mut target = swing_die('P', 0, original_index);
        target.sides = [1, 0];
        target.value = Some(1);
        template.players[1].dice.push(target);
    }
    let mut game = template.clone();

    for _ in 0..10 {
        let attacker = game.players[0]
            .dice
            .iter()
            .position(|die| die.has_property(property::RADIOACTIVE | property::DOPPELGANGER))
            .expect("an untransformed Radioactive Doppelganger remains");
        let action = Move::new_attack(Power, [attacker], [0], 0.0);
        apply_generated_attack(&mut game, &action, &mut Rng::default());
    }

    assert_eq!(game.players[0].dice.len(), 20);
    assert!(game.players[1].dice.iter().all(|die| die.captured));
    restore_dice_for_new_round(&mut game, &template);
    assert_eq!(game.players[0].dice.len(), 10);
}

#[test]
#[should_panic(expected = "Radioactive decay exceeds the transformed dice capacity of 20")]
fn radioactive_decay_reports_transformed_capacity_exhaustion() {
    let mut game = Game::default();
    for original_index in 0..20 {
        let properties = if original_index == 0 {
            property::RADIOACTIVE | property::DOPPELGANGER
        } else {
            0
        };
        game.players[0]
            .dice
            .push(swing_die('P', properties, original_index));
    }
    game.players[1].dice.push(swing_die('P', 0, 0));

    split_radioactive_attacker(&mut game, 0, 0);
}

#[test]
fn doppelganger_recipe_returns_at_the_start_of_the_next_round() {
    let mut template = Game::default();
    let mut original = swing_die('P', property::DOPPELGANGER | property::TRIP, 0);
    original.sides = [20, 0];
    original.value = Some(20);
    template.players[0].dice = vec![original];
    let mut target = swing_die('V', property::POISON | property::TWIN, 0);
    target.sides = [8, 10];
    target.swing_type = [Some('V'), Some('X')];
    target.value = Some(12);
    template.players[1].dice = vec![target];

    let mut game = template.clone();
    game.players[0].dice[0].sides = [18, 0];
    record_round_sides(&mut game);
    let action = Move::new_attack(Power, [0], [0], 0.0);
    apply_generated_attack(&mut game, &action, &mut Rng::default());
    assert_eq!(game.players[0].dice[0].sides, [8, 10]);

    restore_dice_for_new_round(&mut game, &template);

    assert_eq!(game.players[0].dice[0].sides, [18, 0]);
    assert_eq!(game.players[0].dice[0].swing_type, [Some('P'), None]);
    assert!(game.players[0].dice[0].has_property(property::DOPPELGANGER | property::TRIP));
    assert!(!game.players[0].dice[0].has_property(property::POISON | property::TWIN));
}

#[test]
fn repeated_doppelganger_captures_restore_the_rounds_original_recipe() {
    let mut template = Game::default();
    let mut original = swing_die('P', property::DOPPELGANGER, 0);
    original.sides = [18, 0];
    original.value = Some(18);
    template.players[0].dice = vec![original];
    let mut first_target = swing_die('P', property::DOPPELGANGER | property::MAXIMUM, 0);
    first_target.sides = [12, 0];
    first_target.value = Some(12);
    let mut second_target = swing_die('P', property::POISON, 1);
    second_target.sides = [6, 0];
    second_target.value = Some(6);
    template.players[1].dice = vec![first_target, second_target];

    let mut game = template.clone();
    record_round_sides(&mut game);
    let action = Move::new_attack(Power, [0], [0], 0.0);
    apply_generated_attack(&mut game, &action, &mut Rng::default());
    assert!(game.players[0].dice[0].has_property(property::DOPPELGANGER));
    apply_generated_attack(&mut game, &action, &mut Rng::default());
    assert!(game.players[0].dice[0].has_property(property::POISON));

    restore_dice_for_new_round(&mut game, &template);

    assert_eq!(game.players[0].dice[0].sides, [18, 0]);
    assert!(game.players[0].dice[0].has_property(property::DOPPELGANGER));
    assert!(!game.players[0].dice[0].has_property(property::POISON));
}

#[test]
fn doppelganger_round_reset_does_not_preserve_mighty_side_changes() {
    scenario()
        .attacker("DH6:6")
        .attacks(Power)
        .defender("M4:4")
        .expect_next_round_attacker_dice(["HD6"])
        .run();
}
