// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{Attack, Die, DieIndexSet, Game, MAX_DICE, Move, Player, property};
use crate::rng::Rng;

pub(crate) fn apply_attack(game: &mut Game, action: &Move, rng: &mut Rng) -> bool {
    apply_attack_for_players(game, action, 0, 1, rng)
}

pub(crate) fn apply_attack_for_players(
    game: &mut Game,
    action: &Move,
    attacker_player: usize,
    target_player: usize,
    rng: &mut Rng,
) -> bool {
    if action.attack == Some(Attack::Boom) {
        return apply_boom_attack(game, action, attacker_player, target_player, rng);
    }
    apply_fire_adjustments(game, action, attacker_player);
    // A cached count, so attackers marked NOTSET below still count.
    let mut available_attackers = available_dice_count(&game.players[attacker_player]);
    let is_trip = action.attack == Some(Attack::Trip);
    let attacking_jolt_dice = action
        .attackers
        .iter()
        .filter(|index| game.players[attacker_player].dice[*index].has_property(property::JOLT))
        .collect::<DieIndexSet>();
    let attacking_jolt = !attacking_jolt_dice.is_empty();
    let attacking_rage = action
        .attackers
        .iter()
        .any(|index| game.players[attacker_player].dice[index].has_property(property::RAGE));
    let captured_jolt = action
        .targets
        .iter()
        .any(|index| game.players[target_player].dice[index].has_property(property::JOLT));
    // ButtonWeavers keeps these hooks even after Doppelganger replaces the attacker.
    let null_attacker = action
        .attackers
        .iter()
        .any(|index| game.players[attacker_player].dice[index].has_property(property::NULL));
    let value_attacker = action
        .attackers
        .iter()
        .any(|index| game.players[attacker_player].dice[index].has_property(property::VALUE));
    let mut actual_attackers = action.attackers;
    let decays = radioactive_decay_applies(game, action, attacker_player, target_player);
    // Decay strips Jolt after ButtonWeavers' Jolt hook has granted the turn.
    let jolt_dice_to_consume = if decays {
        DieIndexSet::default()
    } else {
        attacking_jolt_dice
    };
    let rerolling_attackers = actual_attackers
        .iter()
        .filter(|index| {
            !game.players[attacker_player].dice[*index].has_property(property::KONSTANT)
        })
        .collect::<DieIndexSet>();
    if decays && !is_trip {
        let attacker = action.attackers.first().expect("Radioactive attacker");
        actual_attackers = apply_radioactive_attack_effects(
            game,
            action,
            attacker_player,
            target_player,
            attacker,
        );
        available_attackers += 1;
    } else {
        for attacker in actual_attackers.iter() {
            apply_attack_player_effects(
                game,
                action,
                attacker_player,
                target_player,
                attacker,
                true,
            );
        }
    }

    if is_trip {
        let target = action.targets.first().expect("Trip target");
        if !game.players[target_player].dice[target].has_property(property::KONSTANT) {
            game.players[target_player].dice[target].not_set = true;
        }
        apply_before_roll_effects(game, target_player, target);
    }

    // Ornery dice reroll after every attack their player makes.
    if action.attack.is_some() {
        for attacker in 0..available_attackers {
            let die = &game.players[attacker_player].dice[attacker];
            if !rerolls_when_ornery(die) || die.not_set {
                continue;
            }
            if !die.has_property(property::KONSTANT) {
                game.players[attacker_player].dice[attacker].not_set = true;
            }
            apply_before_roll_effects(game, attacker_player, attacker);
        }
    }

    // Trip consumes these after its rolls instead, below.
    if !is_trip {
        consume_attacking_jolt(game, attacker_player, jolt_dice_to_consume);
        consume_attacking_rage(game, attacker_player, actual_attackers, attacking_rage);
    }
    // This roll order determines RNG consumption.
    for attacker in actual_attackers.iter() {
        apply_attacker_nature_roll(game, attacker_player, attacker, rng);
    }
    if action.attack.is_some() {
        for attacker in 0..available_attackers {
            let die = &game.players[attacker_player].dice[attacker];
            if rerolls_when_ornery(die) && !actual_attackers.contains(attacker) {
                apply_attacker_nature_roll(game, attacker_player, attacker, rng);
            }
        }
    }
    if is_trip {
        let target = action.targets.first().expect("Trip target");
        if game.players[target_player].dice[target].not_set {
            roll_scheduled_die(game, target_player, target, rng);
        }
        consume_attacking_jolt(game, attacker_player, jolt_dice_to_consume);
        consume_attacking_rage(game, attacker_player, actual_attackers, attacking_rage);
    }

    // Konstant attackers never reroll, so they cannot trigger it.
    let time_and_space_extra_turn = actual_attackers.iter().any(|index| {
        let die = &game.players[attacker_player].dice[index];
        die.has_property(property::TIME_AND_SPACE)
            && rerolling_attackers.contains(index)
            && die.value_total() % 2 == 1
    });

    let mut morph_extra_turn = false;
    if is_trip {
        let target = action.targets.first().expect("Trip target");
        let attacker = action.attackers.first().expect("Trip attacker");
        let trip_failed = game.players[attacker_player].dice[attacker].value_total()
            < game.players[target_player].dice[target].value_total();
        let morphs = !trip_failed
            && game.players[attacker_player].dice[attacker].has_property(property::MORPHING);
        let attacker_is_radioactive =
            game.players[attacker_player].dice[attacker].has_property(property::RADIOACTIVE);
        if morphs {
            // ButtonWeavers morphs after the Trip roll and gives the new die
            // no value, so it rerolls at the captured die's size.
            morph_into_target(game, attacker_player, target_player, attacker, target);
            if !decays {
                let die = &mut game.players[attacker_player].dice[attacker];
                die.not_set = true;
                roll_die(die, rng);
                morph_extra_turn =
                    die.has_property(property::TIME_AND_SPACE) && die.value_total() % 2 == 1;
            }
        }
        if decays {
            // Trip rolls resolve before ButtonWeavers' capture hooks, so decay
            // happens after them, even when the Trip fails.
            let products = split_radioactive_attacker(game, attacker_player, attacker);
            for product in products.iter() {
                if morphs && attacker_is_radioactive {
                    // ButtonWeavers runs each product's Morphing hook again.
                    morph_into_target(game, attacker_player, target_player, product, target);
                }
                roll_die(&mut game.players[attacker_player].dice[product], rng);
            }
            if trip_failed {
                let die = &mut game.players[target_player].dice[target];
                die.properties &= !property::RADIOACTIVE;
            }
        }
        if trip_failed {
            optimize_dice(&mut game.players[attacker_player]);
            optimize_dice(&mut game.players[target_player]);
            return attacking_jolt || time_and_space_extra_turn;
        }
    }

    let mut created_rage_replacement = false;
    for (removed, original_target) in action.targets.iter().enumerate() {
        let target = original_target - removed;
        let rage_replacement = create_and_roll_rage_replacement(game, target_player, target, rng);
        let own_score = game.players[target_player].dice[target].score(true);
        game.players[target_player].score -= own_score;
        if null_attacker {
            game.players[target_player].dice[target].properties |= property::NULL;
        }
        if value_attacker {
            game.players[target_player].dice[target].properties |= property::VALUE;
        }
        let captured_score = game.players[target_player].dice[target].score(false);
        game.players[attacker_player].score += captured_score;
        on_die_lost(&mut game.players[target_player], target);
        if let Some(replacement) = rage_replacement {
            add_rage_replacement(game, target_player, replacement);
            created_rage_replacement = true;
        }
    }
    optimize_dice(&mut game.players[attacker_player]);
    if created_rage_replacement {
        optimize_dice(&mut game.players[target_player]);
    }
    attacking_jolt || captured_jolt || time_and_space_extra_turn || morph_extra_turn
}

/// ButtonWeavers removes the Boom die unscored and rerolls the target, which
/// is never captured; only the attacker's Jolt and Ornery hooks still fire.
fn apply_boom_attack(
    game: &mut Game,
    action: &Move,
    attacker_player: usize,
    target_player: usize,
    rng: &mut Rng,
) -> bool {
    let attacker = action.attackers.first().expect("Boom attacker");
    let target = action.targets.first().expect("Boom target");
    let extra_turn = game.players[attacker_player].dice[attacker].has_property(property::JOLT);
    game.players[attacker_player].score -= game.players[attacker_player].dice[attacker].score(true);
    on_die_lost(&mut game.players[attacker_player], attacker);

    if !game.players[target_player].dice[target].has_property(property::KONSTANT) {
        game.players[target_player].dice[target].not_set = true;
        apply_before_roll_effects(game, target_player, target);
        reroll_and_rescore(game, target_player, target, rng);
    }
    for index in 0..game.players[attacker_player].dice.len() {
        let die = &game.players[attacker_player].dice[index];
        if !die.is_available() || !rerolls_when_ornery(die) {
            continue;
        }
        if !die.has_property(property::KONSTANT) {
            game.players[attacker_player].dice[index].not_set = true;
        }
        apply_before_roll_effects(game, attacker_player, index);
        apply_attacker_nature_roll(game, attacker_player, index, rng);
    }
    optimize_dice(&mut game.players[attacker_player]);
    optimize_dice(&mut game.players[target_player]);
    extra_turn
}

/// Unlike an attacker's reroll, a Boom target's new value counts for Value.
fn reroll_and_rescore(game: &mut Game, player: usize, index: usize, rng: &mut Rng) {
    let old_score = game.players[player].dice[index].score(true);
    let die = &mut game.players[player].dice[index];
    apply_mood(die, rng);
    roll_die(die, rng);
    game.players[player].score += die.score(true) - old_score;
}

pub(super) fn apply_fire_adjustments(game: &mut Game, action: &Move, player: usize) {
    if action.fire.is_empty() {
        return;
    }
    let (increase_total, reduction_total) = action.fire.amounts.iter().enumerate().fold(
        (0u16, 0u16),
        |(increases, reductions), (index, amount)| {
            if action.attackers.contains(index) {
                (increases + u16::from(*amount), reductions)
            } else {
                (increases, reductions + u16::from(*amount))
            }
        },
    );
    assert_eq!(
        increase_total, reduction_total,
        "Fire increases and reductions must balance"
    );
    for index in 0..crate::game::MAX_DICE {
        let amount = action.fire.amounts[index];
        if amount == 0 {
            continue;
        }
        assert!(
            index < game.players[player].dice.len(),
            "invalid Fire die index"
        );
        let die = &mut game.players[player].dice[index];
        let old_score = die.score(true);
        let value = die.value_total();
        if action.attackers.contains(index) {
            let new_value = value + u16::from(amount);
            assert!(new_value <= die.sides_max() && new_value <= u16::from(u8::MAX));
            die.value = Some(new_value as u8);
        } else {
            assert!(die.has_property(property::FIRE));
            let minimum = if die.has_property(property::TWIN) {
                2
            } else {
                1
            };
            let new_value = value - u16::from(amount);
            assert!(new_value >= minimum);
            die.value = Some(new_value as u8);
        }
        game.players[player].score += die.score(true) - old_score;
    }
}

pub(crate) fn radioactive_decay_applies(
    game: &Game,
    action: &Move,
    attacker_player: usize,
    target_player: usize,
) -> bool {
    // A Boom die leaves play before ButtonWeavers' decay hook can split it.
    if matches!(action.attack, None | Some(Attack::Boom))
        || action.attackers.len() != 1
        || action.targets.len() != 1
    {
        return false;
    }
    let (Some(attacker), Some(target)) = (action.attackers.first(), action.targets.first()) else {
        return false;
    };
    // Rage replacements and failed Trips can keep decay going past the
    // 20-slot pool; skipping the split there beats panicking mid-search.
    if game.players[attacker_player].dice.len() >= MAX_DICE {
        return false;
    }
    game.players[attacker_player].dice[attacker].has_property(property::RADIOACTIVE)
        || game.players[target_player].dice[target].has_property(property::RADIOACTIVE)
}

/// ButtonWeavers runs a Radioactive target's decay hook after every attacker
/// hook, so only a Radioactive attacker decays before Doppelganger copies.
pub(crate) fn apply_radioactive_attack_effects(
    game: &mut Game,
    action: &Move,
    attacker_player: usize,
    target_player: usize,
    attacker: usize,
) -> DieIndexSet {
    let target = action.targets.first().expect("Radioactive target");
    let original = game.players[attacker_player].dice[attacker];
    let attacker_is_radioactive = original.has_property(property::RADIOACTIVE);
    let copies_target =
        action.attack == Some(Attack::Power) && original.has_property(property::DOPPELGANGER);

    // Warrior dice never attack alone, and decay strips Turbo before it
    // could resize, so neither effect from apply_attack_player_effects applies.
    if action.attack == Some(Attack::Berserk) {
        halve_berserk_attacker(game, attacker_player, attacker);
    }
    if morphing_applies(action) && original.has_property(property::MORPHING) {
        morph_into_target(game, attacker_player, target_player, attacker, target);
    }
    if copies_target && !attacker_is_radioactive {
        copy_doppelganger_target(game, attacker_player, target_player, attacker, target);
    }

    let products = split_radioactive_attacker(game, attacker_player, attacker);
    for (position, product) in products.iter().enumerate() {
        if copies_target && attacker_is_radioactive {
            copy_doppelganger_target(game, attacker_player, target_player, product, target);
            if position == 0 {
                // ButtonWeavers' attacker loop never rerolls the first copy,
                // which keeps the captured die's value and size.
                game.players[attacker_player].dice[product].not_set = false;
            } else {
                resize_mighty_and_weak(game, attacker_player, product);
            }
        } else if attacker_is_radioactive
            && morphing_applies(action)
            && original.has_property(property::MORPHING)
        {
            // ButtonWeavers runs each product's Morphing hook again.
            morph_into_target(game, attacker_player, target_player, product, target);
        } else if copies_target {
            // ButtonWeavers resets doesReroll on Doppelganger copies.
            resize_mighty_and_weak(game, attacker_player, product);
        } else {
            apply_before_roll_effects(game, attacker_player, product);
        }
    }
    products
}

/// Matches ButtonWeavers `BMDie::split` and `BMDieTwin::split`, whose order
/// decides which product keeps each rounded-up half.
pub(crate) fn split_radioactive_attacker(
    game: &mut Game,
    attacker_player: usize,
    attacker: usize,
) -> DieIndexSet {
    assert!(
        game.players[attacker_player].dice.len() < MAX_DICE,
        "Radioactive decay exceeds the transformed dice capacity of {MAX_DICE}"
    );

    let original = game.players[attacker_player].dice[attacker];
    let original_index = original.original_index;
    let used_indices = game.players[attacker_player]
        .dice
        .iter()
        .map(|die| die.original_index)
        .collect::<DieIndexSet>();
    let synthetic_index = (0..MAX_DICE)
        .find(|index| !used_indices.contains(*index))
        .expect("Radioactive decay has no free stable die index");
    let mut first = original;
    let mut second = original;
    let removed = property::RADIOACTIVE
        | property::TURBO
        | property::MOOD
        | property::MAD
        | property::JOLT
        | property::TIME_AND_SPACE;
    first.properties &= !removed;
    second.properties &= !removed;
    if original.has_property(property::TWIN) {
        first.sides = [original.sides[0].div_ceil(2), original.sides[1] / 2];
        second.sides = [original.sides[0] / 2, original.sides[1].div_ceil(2)];
    } else {
        first.sides[0] = original.sides[0].div_ceil(2);
        second.sides[0] = original.sides[0] / 2;
    }
    for die in [&mut first, &mut second] {
        die.value = None;
        die.not_set = true;
        die.captured = false;
        die.dizzy = false;
    }
    second.original_index = synthetic_index;

    let transformed = 1 << original_index;
    if game.players[attacker_player].round_transformed & transformed == 0 {
        game.players[attacker_player].round_original_sides[original_index] = original.sides;
        game.players[attacker_player].round_transformed |= transformed;
    }
    game.players[attacker_player].radioactive_products |= 1 << synthetic_index;
    let old_score = original.score(true);
    game.players[attacker_player].dice[attacker] = first;
    game.players[attacker_player]
        .dice
        .insert(attacker + 1, second);
    let new_score = first.score(true) + second.score(true);
    game.players[attacker_player].score += new_score - old_score;
    [attacker, attacker + 1].into()
}

fn halve_berserk_attacker(game: &mut Game, attacker_player: usize, attacker: usize) {
    let die = &mut game.players[attacker_player].dice[attacker];
    let old_score = die.score(true);
    die.sides[0] = die.sides[0].div_ceil(2);
    die.properties &= !property::BERSERK;
    game.players[attacker_player].score += die.score(true) - old_score;
}

/// Trip morphs only once its roll has succeeded, so it is handled separately.
fn morphing_applies(action: &Move) -> bool {
    action.targets.len() == 1 && action.attack != Some(Attack::Trip)
}

fn morph_into_target(
    game: &mut Game,
    attacker_player: usize,
    target_player: usize,
    attacker: usize,
    target: usize,
) {
    let target_die = game.players[target_player].dice[target];
    let die = &mut game.players[attacker_player].dice[attacker];
    let old_score = die.score(true);
    if target_die.has_property(property::TWIN) {
        die.properties |= property::TWIN;
        die.sides = target_die.sides;
    } else {
        die.properties &= !property::TWIN;
        die.sides = [target_die.sides_max() as u8, 0];
    }
    game.players[attacker_player].score += die.score(true) - old_score;
}

fn copy_doppelganger_target(
    game: &mut Game,
    attacker_player: usize,
    target_player: usize,
    attacker: usize,
    target: usize,
) {
    let mut copied = game.players[target_player].dice[target];
    let original = game.players[attacker_player].dice[attacker];
    let original_index = original.original_index;
    let old_score = original.score(true);
    let transformed = 1 << original_index;
    if game.players[attacker_player].round_transformed & transformed == 0 {
        game.players[attacker_player].round_original_sides[original_index] = original.sides;
        game.players[attacker_player].round_transformed |= transformed;
    }
    copied.captured = false;
    copied.not_set = true;
    copied.dizzy = false;
    copied.original_index = original_index;
    copied.in_reserve = false;
    game.players[attacker_player].dice[attacker] = copied;
    game.players[attacker_player].score += copied.score(true) - old_score;
}

pub(super) fn consume_attacking_jolt(game: &mut Game, player: usize, attackers: DieIndexSet) {
    for attacker in attackers.iter() {
        game.players[player].dice[attacker].properties &= !property::JOLT;
    }
}

pub(super) fn consume_attacking_rage(
    game: &mut Game,
    player: usize,
    attackers: DieIndexSet,
    participated_with_rage: bool,
) {
    if !participated_with_rage {
        return;
    }
    for attacker in attackers.iter() {
        game.players[player].dice[attacker].properties &= !property::RAGE;
    }
}

pub(super) fn create_and_roll_rage_replacement(
    game: &Game,
    player: usize,
    target: usize,
    rng: &mut Rng,
) -> Option<Die> {
    let original = game.players[player].dice[target];
    if !original.has_property(property::RAGE) {
        return None;
    }
    assert!(
        game.players[player].dice.len() < MAX_DICE,
        "Rage replacement exceeds the transformed dice capacity of {MAX_DICE}"
    );
    let used_indices = game.players[player]
        .dice
        .iter()
        .map(|die| die.original_index)
        .collect::<DieIndexSet>();
    let synthetic_index = (0..MAX_DICE)
        .find(|index| !used_indices.contains(*index))
        .expect("Rage replacement has no free stable die index");
    let mut replacement = original;
    replacement.properties &= !property::RAGE;
    replacement.value = None;
    replacement.captured = false;
    replacement.not_set = true;
    replacement.dizzy = false;
    replacement.original_index = synthetic_index;
    // ButtonWeavers' replacement roll skips Mighty, Weak, and Mood.
    roll_die(&mut replacement, rng);
    Some(replacement)
}

pub(super) fn add_rage_replacement(game: &mut Game, player: usize, replacement: Die) {
    game.players[player].rage_replacements |= 1 << replacement.original_index;
    game.players[player].score += replacement.score(true);
    let available = available_dice_count(&game.players[player]);
    game.players[player].dice.insert(available, replacement);
}

pub(crate) fn apply_attack_player_effects(
    game: &mut Game,
    action: &Move,
    attacker_player: usize,
    target_player: usize,
    attacker: usize,
    actually_attacking: bool,
) {
    if !game.players[attacker_player].dice[attacker].has_property(property::KONSTANT) {
        game.players[attacker_player].dice[attacker].not_set = true;
    }

    if actually_attacking && action.attack == Some(Attack::Berserk) {
        halve_berserk_attacker(game, attacker_player, attacker);
    }

    if game.players[attacker_player].dice[attacker].not_set {
        apply_before_roll_effects(game, attacker_player, attacker);
    }

    if morphing_applies(action)
        && game.players[attacker_player].dice[attacker].has_property(property::MORPHING)
    {
        let target = action.targets.first().expect("Morphing target");
        morph_into_target(game, attacker_player, target_player, attacker, target);
    }

    if game.players[attacker_player].dice[attacker].has_property(property::TURBO)
        && action.turbo_option >= 0
    {
        if game.players[attacker_player].dice[attacker].has_property(property::OPTION) {
            if action.turbo_option == 1 {
                let die = &mut game.players[attacker_player].dice[attacker];
                let old_score = die.score(true);
                die.sides.swap(0, 1);
                game.players[attacker_player].score += die.score(true) - old_score;
            }
        } else if action.turbo_option > 0
            && let Some(swing) = game.players[attacker_player].dice[attacker].swing_type[0]
        {
            let mut score_delta = 0.0;
            for die in &mut game.players[attacker_player].dice {
                let old_score = die.score(true);
                for side in 0..2 {
                    if die.swing_type[side] == Some(swing) {
                        die.sides[side] = action.turbo_option as u8;
                    }
                }
                score_delta += die.score(true) - old_score;
            }
            game.players[attacker_player].score += score_delta;
        }
    }

    // ButtonWeavers copies before the attack reroll, so the copy's own Mighty
    // or Weak applies; Warrior is lost after.
    if actually_attacking
        && action.attack == Some(Attack::Power)
        && action.attackers.len() == 1
        && action.targets.len() == 1
        && game.players[attacker_player].dice[attacker].has_property(property::DOPPELGANGER)
    {
        let target = action.targets.first().expect("Doppelganger target");
        copy_doppelganger_target(game, attacker_player, target_player, attacker, target);
        // The copy rerolls even if Konstant, so Mighty and Weak resize it.
        resize_mighty_and_weak(game, attacker_player, attacker);
    }

    if game.players[attacker_player].dice[attacker].has_property(property::WARRIOR) {
        let die = &mut game.players[attacker_player].dice[attacker];
        let old_score = die.score(true);
        die.properties &= !property::WARRIOR;
        game.players[attacker_player].score += die.score(true) - old_score;
    }
}

// ButtonWeavers Konstant clears `doesReroll`, which Mighty and Weak require.
pub(crate) fn apply_before_roll_effects(game: &mut Game, player: usize, index: usize) {
    if !game.players[player].dice[index].has_property(property::KONSTANT) {
        resize_mighty_and_weak(game, player, index);
    }
}

fn rerolls_when_ornery(die: &Die) -> bool {
    die.has_property(property::ORNERY) && !die.has_property(property::WARRIOR)
}

fn resize_mighty_and_weak(game: &mut Game, player: usize, index: usize) {
    let die = &mut game.players[player].dice[index];
    let old_score = die.score(true);
    let dice = if die.has_property(property::TWIN) {
        2
    } else {
        1
    };
    if die.has_property(property::MIGHTY) {
        for sides in die.sides.iter_mut().take(dice) {
            *sides = mighty_sides(*sides);
        }
    }
    if die.has_property(property::WEAK) {
        for sides in die.sides.iter_mut().take(dice) {
            *sides = weak_sides(*sides);
        }
    }
    game.players[player].score += die.score(true) - old_score;
}

pub(super) fn apply_attacker_nature_roll(
    game: &mut Game,
    player: usize,
    index: usize,
    rng: &mut Rng,
) {
    let die = &mut game.players[player].dice[index];
    let old_score = die.score(true);
    apply_mood(die, rng);
    // C++ never rescores after the reroll, so a Value die keeps its old score.
    game.players[player].score += die.score(true) - old_score;
    if die.not_set {
        roll_die(die, rng);
    }
}

/// ButtonWeavers resizes Mood and Mad dice on every reroll, including Trip
/// targets and Chance rerolls.
pub(crate) fn roll_scheduled_die(game: &mut Game, player: usize, index: usize, rng: &mut Rng) {
    apply_attacker_nature_roll(game, player, index, rng);
}

pub(super) fn on_die_lost(player: &mut crate::game::Player, index: usize) {
    let available = available_dice_count(player);
    player.dice[index].captured = true;
    player.dice[index..available].rotate_left(1);
}

// ButtonWeavers `standard_die_sizes`; unlike Mighty's list, no 16.
const MOOD_DIE_SIZES: [u8; 9] = [1, 2, 4, 6, 8, 10, 12, 20, 30];

pub(super) fn apply_mood(die: &mut Die, rng: &mut Rng) {
    let mad = die.has_property(property::MAD);
    if !mad && !die.has_property(property::MOOD) || die.has_property(property::KONSTANT) {
        return;
    }
    let Some(swing) = die.swing_type.iter().flatten().next().copied() else {
        return;
    };
    let (minimum, maximum) = swing_range(swing);
    let sizes = (minimum..=maximum)
        .filter(|size| {
            if mad {
                size % 2 == 0
            } else {
                MOOD_DIE_SIZES.contains(size)
            }
        })
        .collect::<Vec<_>>();
    let size = sizes[rng.rand_below(sizes.len() as u32) as usize];
    for index in 0..die.sides.len() {
        if die.swing_type[index].is_some() {
            die.sides[index] = size;
        }
    }
}

pub(super) fn mighty_sides(sides: u8) -> u8 {
    const VALUES: [u8; 20] = [
        1, 2, 4, 4, 6, 6, 8, 8, 10, 10, 12, 12, 16, 16, 16, 16, 20, 20, 20, 20,
    ];
    if sides >= 20 {
        30
    } else {
        VALUES[sides as usize]
    }
}

pub(super) fn weak_sides(sides: u8) -> u8 {
    const VALUES: [u8; 20] = [
        1, 1, 1, 2, 2, 4, 4, 6, 6, 8, 8, 10, 10, 12, 12, 12, 12, 16, 16, 16,
    ];
    match sides {
        31.. => 30,
        21..=30 => 20,
        20 => 16,
        _ => VALUES[sides as usize],
    }
}

pub(crate) fn optimize_dice(player: &mut Player) {
    player.optimize_dice();
}

pub(crate) fn roll_die(die: &mut Die, rng: &mut Rng) {
    assert!(die.not_set, "Die::Roll requires NOTSET state");
    die.captured = false;
    die.not_set = false;
    die.dizzy = false;
    if die.in_reserve {
        die.value = None;
        return;
    }
    let dice = if die.has_property(property::TWIN) {
        2
    } else {
        1
    };
    let mut value = 0u16;
    for sides in die.sides.iter().take(dice) {
        if *sides > 0 {
            if die.has_property(property::WARRIOR | property::MAXIMUM) {
                value += u16::from(*sides);
            } else {
                value += u16::from(rng.rand_below(u32::from(*sides)) as u8 + 1);
            }
        }
    }
    die.value = Some(value as u8);
}

pub(crate) fn initiative_winner(game: &Game) -> usize {
    check_initiative(game).unwrap_or(0)
}

pub(crate) fn check_initiative(game: &Game) -> Option<usize> {
    let mut values = [Vec::new(), Vec::new()];
    for (player, output) in values.iter_mut().enumerate() {
        *output = game.players[player]
            .dice
            .iter()
            .filter(|die| {
                die.is_available()
                    && !die.has_property(
                        property::TRIP | property::SLOW | property::STINGER | property::RAGE,
                    )
            })
            .map(Die::value_total)
            .collect();
        output.sort_unstable();
    }
    // ButtonWeavers ranks a no-initiative button below every other button,
    // even one with no initiative dice, by giving the others a sentinel.
    let no_initiative =
        [0, 1].map(|player| game.players[player].specials & super::special::NO_INITIATIVE != 0);
    if no_initiative.contains(&true) {
        for (player, output) in values.iter_mut().enumerate() {
            if no_initiative[player] {
                output.clear();
            } else {
                output.push(u16::MAX);
            }
        }
    }
    for index in 0..values[0].len().max(values[1].len()) {
        match (values[0].get(index), values[1].get(index)) {
            (Some(a), Some(b)) if a != b => return Some(usize::from(a > b)),
            (None, Some(_)) => return Some(1),
            (Some(_), None) => return Some(0),
            _ => {}
        }
    }
    None
}

pub(crate) fn available_dice_count(player: &Player) -> usize {
    player.dice.iter().filter(|die| die.is_available()).count()
}

pub(crate) fn swing_range(swing: char) -> (u8, u8) {
    match swing {
        'P' => (1, 30),
        'Q' => (2, 20),
        'R' => (2, 16),
        'S' => (6, 20),
        'T' => (2, 12),
        'U' => (8, 30),
        'V' => (6, 12),
        'W' => (4, 12),
        'X' => (4, 20),
        'Y' => (1, 20),
        'Z' => (4, 30),
        _ => (0, 0),
    }
}

pub(crate) fn restore_dice_for_new_round(game: &mut Game, template: &Game) {
    for player in 0..game.players.len() {
        let products =
            game.players[player].radioactive_products | game.players[player].rage_replacements;
        game.players[player]
            .dice
            .retain(|die| products & (1 << die.original_index) == 0);
        game.players[player].round_transformed &= !products;
        game.players[player].radioactive_products = 0;
        game.players[player].rage_replacements = 0;
        for index in 0..game.players[player].dice.len() {
            let original_index = game.players[player].dice[index].original_index;
            let transformed = game.players[player].round_transformed & (1 << original_index) != 0;
            if transformed {
                let sides = game.players[player].round_original_sides[original_index];
                let in_reserve = game.players[player].dice[index].in_reserve;
                if let Some(original) = template.players[player]
                    .dice
                    .iter()
                    .find(|original| original.original_index == original_index)
                {
                    game.players[player].dice[index] = *original;
                    // Swing and Option selections persist for a round winner;
                    // fixed Mighty/Weak side changes do not.
                    if original.has_property(property::OPTION) {
                        game.players[player].dice[index].sides = sides;
                    } else {
                        for (side, saved) in sides.iter().enumerate() {
                            if original.swing_type[side].is_some() {
                                game.players[player].dice[index].sides[side] = *saved;
                            }
                        }
                    }
                    game.players[player].dice[index].in_reserve = in_reserve;
                }
                game.players[player].round_transformed &= !(1 << original_index);
            }
            let die = &mut game.players[player].dice[index];
            let Some(original) = template.players[player]
                .dice
                .iter()
                .find(|original| original.original_index == die.original_index)
            else {
                continue;
            };

            if original.has_property(property::JOLT) {
                die.properties |= property::JOLT;
            } else {
                die.properties &= !property::JOLT;
            }
            if original.has_property(property::RAGE) {
                die.properties |= property::RAGE;
            } else {
                die.properties &= !property::RAGE;
            }
        }
    }
}

pub(crate) fn roll_round_dice(game: &mut Game, rng: &mut Rng) {
    for player in &mut game.players {
        player.score = 0.0;
        for die in &mut player.dice {
            die.not_set = true;
            roll_die(die, rng);
        }
        player.score = player
            .dice
            .iter()
            .filter(|d| d.is_available())
            .map(|d| d.score(true))
            .sum();
        player.optimize_dice();
    }
}

pub(crate) fn recover_dizzy_dice(player: &mut Player) {
    for die in &mut player.dice {
        die.dizzy = false;
    }
}
