// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{Attack, Die, DieIndexSet, Game, MAX_DICE, Move, Player, property};
use crate::rng::Rng;

pub(crate) fn ApplyAttack(game: &mut Game, action: &Move, rng: &mut Rng) -> bool {
    ApplyAttackForPlayers(game, action, 0, 1, rng)
}

pub(crate) fn ApplyAttackForPlayers(
    game: &mut Game,
    action: &Move,
    attacker_player: usize,
    target_player: usize,
    rng: &mut Rng,
) -> bool {
    if action.m_attack == Some(Attack::Boom) {
        return ApplyBoomAttack(game, action, attacker_player, target_player, rng);
    }
    ApplyFireAdjustments(game, action, attacker_player);
    // A cached count, so attackers marked NOTSET below still count.
    let mut available_attackers = AvailableDice(&game.m_player[attacker_player]);
    let is_trip = action.m_attack == Some(Attack::Trip);
    let attacking_jolt_dice = action
        .m_attackers
        .iter()
        .filter(|index| game.m_player[attacker_player].m_die[*index].HasProperty(property::JOLT))
        .collect::<DieIndexSet>();
    let attacking_jolt = !attacking_jolt_dice.is_empty();
    let attacking_rage = action
        .m_attackers
        .iter()
        .any(|index| game.m_player[attacker_player].m_die[index].HasProperty(property::RAGE));
    let captured_jolt = action
        .m_targets
        .iter()
        .any(|index| game.m_player[target_player].m_die[index].HasProperty(property::JOLT));
    // ButtonWeavers keeps these hooks even after Doppelganger replaces the attacker.
    let null_attacker = action
        .m_attackers
        .iter()
        .any(|index| game.m_player[attacker_player].m_die[index].HasProperty(property::NULL));
    let value_attacker = action
        .m_attackers
        .iter()
        .any(|index| game.m_player[attacker_player].m_die[index].HasProperty(property::VALUE));
    let mut actual_attackers = action.m_attackers;
    let decays = RadioactiveDecayApplies(game, action, attacker_player, target_player);
    // Decay strips Jolt after ButtonWeavers' Jolt hook has granted the turn.
    let jolt_dice_to_consume = if decays {
        DieIndexSet::default()
    } else {
        attacking_jolt_dice
    };
    let rerolling_attackers = actual_attackers
        .iter()
        .filter(|index| {
            !game.m_player[attacker_player].m_die[*index].HasProperty(property::KONSTANT)
        })
        .collect::<DieIndexSet>();
    if decays && !is_trip {
        let attacker = action.m_attackers.first().expect("Radioactive attacker");
        actual_attackers =
            ApplyRadioactiveAttackEffects(game, action, attacker_player, target_player, attacker);
        available_attackers += 1;
    } else {
        for attacker in actual_attackers.iter() {
            ApplyAttackPlayerEffects(game, action, attacker_player, target_player, attacker, true);
        }
    }

    if is_trip {
        let target = action.m_targets.first().expect("Trip target");
        if !game.m_player[target_player].m_die[target].HasProperty(property::KONSTANT) {
            game.m_player[target_player].m_die[target].m_notset = true;
        }
        ApplyBeforeRollEffects(game, target_player, target);
    }

    // Ornery dice reroll after every attack their player makes.
    if action.m_attack.is_some() {
        for attacker in 0..available_attackers {
            let die = &game.m_player[attacker_player].m_die[attacker];
            if !RerollsWhenOrnery(die) || die.m_notset {
                continue;
            }
            if !die.HasProperty(property::KONSTANT) {
                game.m_player[attacker_player].m_die[attacker].m_notset = true;
            }
            ApplyBeforeRollEffects(game, attacker_player, attacker);
        }
    }

    // Trip consumes these after its rolls instead, below.
    if !is_trip {
        ConsumeAttackingJolt(game, attacker_player, jolt_dice_to_consume);
        ConsumeAttackingRage(game, attacker_player, actual_attackers, attacking_rage);
    }
    // This roll order determines RNG consumption.
    for attacker in actual_attackers.iter() {
        ApplyAttackerNatureRoll(game, attacker_player, attacker, rng);
    }
    if action.m_attack.is_some() {
        for attacker in 0..available_attackers {
            let die = &game.m_player[attacker_player].m_die[attacker];
            if RerollsWhenOrnery(die) && !actual_attackers.contains(attacker) {
                ApplyAttackerNatureRoll(game, attacker_player, attacker, rng);
            }
        }
    }
    if is_trip {
        let target = action.m_targets.first().expect("Trip target");
        if game.m_player[target_player].m_die[target].m_notset {
            RollScheduledDie(game, target_player, target, rng);
        }
        ConsumeAttackingJolt(game, attacker_player, jolt_dice_to_consume);
        ConsumeAttackingRage(game, attacker_player, actual_attackers, attacking_rage);
    }

    // Konstant attackers never reroll, so they cannot trigger it.
    let time_and_space_extra_turn = actual_attackers.iter().any(|index| {
        let die = &game.m_player[attacker_player].m_die[index];
        die.HasProperty(property::TIME_AND_SPACE)
            && rerolling_attackers.contains(index)
            && die.GetValueTotal() % 2 == 1
    });

    let mut morph_extra_turn = false;
    if is_trip {
        let target = action.m_targets.first().expect("Trip target");
        let attacker = action.m_attackers.first().expect("Trip attacker");
        let trip_failed = game.m_player[attacker_player].m_die[attacker].GetValueTotal()
            < game.m_player[target_player].m_die[target].GetValueTotal();
        let morphs = !trip_failed
            && game.m_player[attacker_player].m_die[attacker].HasProperty(property::MORPHING);
        let attacker_is_radioactive =
            game.m_player[attacker_player].m_die[attacker].HasProperty(property::RADIOACTIVE);
        if morphs {
            // ButtonWeavers morphs after the Trip roll and gives the new die
            // no value, so it rerolls at the captured die's size.
            MorphIntoTarget(game, attacker_player, target_player, attacker, target);
            if !decays {
                let die = &mut game.m_player[attacker_player].m_die[attacker];
                die.m_notset = true;
                RollDie(die, rng);
                morph_extra_turn =
                    die.HasProperty(property::TIME_AND_SPACE) && die.GetValueTotal() % 2 == 1;
            }
        }
        if decays {
            // Trip rolls resolve before ButtonWeavers' capture hooks, so decay
            // happens after them, even when the Trip fails.
            let products = SplitRadioactiveAttacker(game, attacker_player, attacker);
            for product in products.iter() {
                if morphs && attacker_is_radioactive {
                    // ButtonWeavers runs each product's Morphing hook again.
                    MorphIntoTarget(game, attacker_player, target_player, product, target);
                }
                RollDie(&mut game.m_player[attacker_player].m_die[product], rng);
            }
            if trip_failed {
                let die = &mut game.m_player[target_player].m_die[target];
                die.m_properties &= !property::RADIOACTIVE;
            }
        }
        if trip_failed {
            OptimizeDice(&mut game.m_player[attacker_player]);
            OptimizeDice(&mut game.m_player[target_player]);
            return attacking_jolt || time_and_space_extra_turn;
        }
    }

    let mut created_rage_replacement = false;
    for (removed, original_target) in action.m_targets.iter().enumerate() {
        let target = original_target - removed;
        let rage_replacement = CreateAndRollRageReplacement(game, target_player, target, rng);
        let own_score = game.m_player[target_player].m_die[target].GetScore(true);
        game.m_player[target_player].m_score -= own_score;
        if null_attacker {
            game.m_player[target_player].m_die[target].m_properties |= property::NULL;
        }
        if value_attacker {
            game.m_player[target_player].m_die[target].m_properties |= property::VALUE;
        }
        let captured_score = game.m_player[target_player].m_die[target].GetScore(false);
        game.m_player[attacker_player].m_score += captured_score;
        OnDieLost(&mut game.m_player[target_player], target);
        if let Some(replacement) = rage_replacement {
            AddRageReplacement(game, target_player, replacement);
            created_rage_replacement = true;
        }
    }
    OptimizeDice(&mut game.m_player[attacker_player]);
    if created_rage_replacement {
        OptimizeDice(&mut game.m_player[target_player]);
    }
    attacking_jolt || captured_jolt || time_and_space_extra_turn || morph_extra_turn
}

/// ButtonWeavers removes the Boom die unscored and rerolls the target, which
/// is never captured; only the attacker's Jolt and Ornery hooks still fire.
fn ApplyBoomAttack(
    game: &mut Game,
    action: &Move,
    attacker_player: usize,
    target_player: usize,
    rng: &mut Rng,
) -> bool {
    let attacker = action.m_attackers.first().expect("Boom attacker");
    let target = action.m_targets.first().expect("Boom target");
    let extra_turn = game.m_player[attacker_player].m_die[attacker].HasProperty(property::JOLT);
    game.m_player[attacker_player].m_score -=
        game.m_player[attacker_player].m_die[attacker].GetScore(true);
    OnDieLost(&mut game.m_player[attacker_player], attacker);

    if !game.m_player[target_player].m_die[target].HasProperty(property::KONSTANT) {
        game.m_player[target_player].m_die[target].m_notset = true;
        ApplyBeforeRollEffects(game, target_player, target);
        RerollAndRescore(game, target_player, target, rng);
    }
    for index in 0..game.m_player[attacker_player].m_die.len() {
        let die = &game.m_player[attacker_player].m_die[index];
        if !die.IsAvailable() || !RerollsWhenOrnery(die) {
            continue;
        }
        if !die.HasProperty(property::KONSTANT) {
            game.m_player[attacker_player].m_die[index].m_notset = true;
        }
        ApplyBeforeRollEffects(game, attacker_player, index);
        ApplyAttackerNatureRoll(game, attacker_player, index, rng);
    }
    OptimizeDice(&mut game.m_player[attacker_player]);
    OptimizeDice(&mut game.m_player[target_player]);
    extra_turn
}

/// Unlike an attacker's reroll, a Boom target's new value counts for Value.
fn RerollAndRescore(game: &mut Game, player: usize, index: usize, rng: &mut Rng) {
    let old_score = game.m_player[player].m_die[index].GetScore(true);
    let die = &mut game.m_player[player].m_die[index];
    ApplyMood(die, rng);
    RollDie(die, rng);
    game.m_player[player].m_score += die.GetScore(true) - old_score;
}

pub(super) fn ApplyFireAdjustments(game: &mut Game, action: &Move, player: usize) {
    if action.m_fire.is_empty() {
        return;
    }
    let (increase_total, reduction_total) = action.m_fire.m_amounts.iter().enumerate().fold(
        (0u16, 0u16),
        |(increases, reductions), (index, amount)| {
            if action.m_attackers.contains(index) {
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
        let amount = action.m_fire.m_amounts[index];
        if amount == 0 {
            continue;
        }
        assert!(
            index < game.m_player[player].m_die.len(),
            "invalid Fire die index"
        );
        let die = &mut game.m_player[player].m_die[index];
        let old_score = die.GetScore(true);
        let value = die.GetValueTotal();
        if action.m_attackers.contains(index) {
            let new_value = value + u16::from(amount);
            assert!(new_value <= die.GetSidesMax() && new_value <= u16::from(u8::MAX));
            die.m_value_total = Some(new_value as u8);
        } else {
            assert!(die.HasProperty(property::FIRE));
            let minimum = if die.HasProperty(property::TWIN) {
                2
            } else {
                1
            };
            let new_value = value - u16::from(amount);
            assert!(new_value >= minimum);
            die.m_value_total = Some(new_value as u8);
        }
        game.m_player[player].m_score += die.GetScore(true) - old_score;
    }
}

pub(crate) fn RadioactiveDecayApplies(
    game: &Game,
    action: &Move,
    attacker_player: usize,
    target_player: usize,
) -> bool {
    // A Boom die leaves play before ButtonWeavers' decay hook can split it.
    if matches!(action.m_attack, None | Some(Attack::Boom))
        || action.m_attackers.len() != 1
        || action.m_targets.len() != 1
    {
        return false;
    }
    let (Some(attacker), Some(target)) = (action.m_attackers.first(), action.m_targets.first())
    else {
        return false;
    };
    // Rage replacements and failed Trips can keep decay going past the
    // 20-slot pool; skipping the split there beats panicking mid-search.
    if game.m_player[attacker_player].m_die.len() >= MAX_DICE {
        return false;
    }
    game.m_player[attacker_player].m_die[attacker].HasProperty(property::RADIOACTIVE)
        || game.m_player[target_player].m_die[target].HasProperty(property::RADIOACTIVE)
}

/// ButtonWeavers runs a Radioactive target's decay hook after every attacker
/// hook, so only a Radioactive attacker decays before Doppelganger copies.
pub(crate) fn ApplyRadioactiveAttackEffects(
    game: &mut Game,
    action: &Move,
    attacker_player: usize,
    target_player: usize,
    attacker: usize,
) -> DieIndexSet {
    let target = action.m_targets.first().expect("Radioactive target");
    let original = game.m_player[attacker_player].m_die[attacker];
    let attacker_is_radioactive = original.HasProperty(property::RADIOACTIVE);
    let copies_target =
        action.m_attack == Some(Attack::Power) && original.HasProperty(property::DOPPELGANGER);

    // Warrior dice never attack alone, and decay strips Turbo before it
    // could resize, so neither effect from ApplyAttackPlayerEffects applies.
    if action.m_attack == Some(Attack::Berserk) {
        HalveBerserkAttacker(game, attacker_player, attacker);
    }
    if MorphingApplies(action) && original.HasProperty(property::MORPHING) {
        MorphIntoTarget(game, attacker_player, target_player, attacker, target);
    }
    if copies_target && !attacker_is_radioactive {
        CopyDoppelgangerTarget(game, attacker_player, target_player, attacker, target);
    }

    let products = SplitRadioactiveAttacker(game, attacker_player, attacker);
    for (position, product) in products.iter().enumerate() {
        if copies_target && attacker_is_radioactive {
            CopyDoppelgangerTarget(game, attacker_player, target_player, product, target);
            if position == 0 {
                // ButtonWeavers' attacker loop never rerolls the first copy,
                // which keeps the captured die's value and size.
                game.m_player[attacker_player].m_die[product].m_notset = false;
            } else {
                ResizeMightyAndWeak(game, attacker_player, product);
            }
        } else if attacker_is_radioactive
            && MorphingApplies(action)
            && original.HasProperty(property::MORPHING)
        {
            // ButtonWeavers runs each product's Morphing hook again.
            MorphIntoTarget(game, attacker_player, target_player, product, target);
        } else if copies_target {
            // ButtonWeavers resets doesReroll on Doppelganger copies.
            ResizeMightyAndWeak(game, attacker_player, product);
        } else {
            ApplyBeforeRollEffects(game, attacker_player, product);
        }
    }
    products
}

/// Matches ButtonWeavers `BMDie::split` and `BMDieTwin::split`, whose order
/// decides which product keeps each rounded-up half.
pub(crate) fn SplitRadioactiveAttacker(
    game: &mut Game,
    attacker_player: usize,
    attacker: usize,
) -> DieIndexSet {
    assert!(
        game.m_player[attacker_player].m_die.len() < MAX_DICE,
        "Radioactive decay exceeds the transformed dice capacity of {MAX_DICE}"
    );

    let original = game.m_player[attacker_player].m_die[attacker];
    let original_index = original.m_original_index;
    let used_indices = game.m_player[attacker_player]
        .m_die
        .iter()
        .map(|die| die.m_original_index)
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
    first.m_properties &= !removed;
    second.m_properties &= !removed;
    if original.HasProperty(property::TWIN) {
        first.m_sides = [original.m_sides[0].div_ceil(2), original.m_sides[1] / 2];
        second.m_sides = [original.m_sides[0] / 2, original.m_sides[1].div_ceil(2)];
    } else {
        first.m_sides[0] = original.m_sides[0].div_ceil(2);
        second.m_sides[0] = original.m_sides[0] / 2;
    }
    for die in [&mut first, &mut second] {
        die.m_value_total = None;
        die.m_notset = true;
        die.m_captured = false;
        die.m_dizzy = false;
    }
    second.m_original_index = synthetic_index;

    let transformed = 1 << original_index;
    if game.m_player[attacker_player].m_round_transformed & transformed == 0 {
        game.m_player[attacker_player].m_round_original_sides[original_index] = original.m_sides;
        game.m_player[attacker_player].m_round_transformed |= transformed;
    }
    game.m_player[attacker_player].m_radioactive_products |= 1 << synthetic_index;
    let old_score = original.GetScore(true);
    game.m_player[attacker_player].m_die[attacker] = first;
    game.m_player[attacker_player]
        .m_die
        .insert(attacker + 1, second);
    let new_score = first.GetScore(true) + second.GetScore(true);
    game.m_player[attacker_player].m_score += new_score - old_score;
    [attacker, attacker + 1].into()
}

fn HalveBerserkAttacker(game: &mut Game, attacker_player: usize, attacker: usize) {
    let die = &mut game.m_player[attacker_player].m_die[attacker];
    let old_score = die.GetScore(true);
    die.m_sides[0] = die.m_sides[0].div_ceil(2);
    die.m_properties &= !property::BERSERK;
    game.m_player[attacker_player].m_score += die.GetScore(true) - old_score;
}

/// Trip morphs only once its roll has succeeded, so it is handled separately.
fn MorphingApplies(action: &Move) -> bool {
    action.m_targets.len() == 1 && action.m_attack != Some(Attack::Trip)
}

fn MorphIntoTarget(
    game: &mut Game,
    attacker_player: usize,
    target_player: usize,
    attacker: usize,
    target: usize,
) {
    let target_die = game.m_player[target_player].m_die[target];
    let die = &mut game.m_player[attacker_player].m_die[attacker];
    let old_score = die.GetScore(true);
    if target_die.HasProperty(property::TWIN) {
        die.m_properties |= property::TWIN;
        die.m_sides = target_die.m_sides;
    } else {
        die.m_properties &= !property::TWIN;
        die.m_sides = [target_die.GetSidesMax() as u8, 0];
    }
    game.m_player[attacker_player].m_score += die.GetScore(true) - old_score;
}

fn CopyDoppelgangerTarget(
    game: &mut Game,
    attacker_player: usize,
    target_player: usize,
    attacker: usize,
    target: usize,
) {
    let mut copied = game.m_player[target_player].m_die[target];
    let original = game.m_player[attacker_player].m_die[attacker];
    let original_index = original.m_original_index;
    let old_score = original.GetScore(true);
    let transformed = 1 << original_index;
    if game.m_player[attacker_player].m_round_transformed & transformed == 0 {
        game.m_player[attacker_player].m_round_original_sides[original_index] = original.m_sides;
        game.m_player[attacker_player].m_round_transformed |= transformed;
    }
    copied.m_captured = false;
    copied.m_notset = true;
    copied.m_dizzy = false;
    copied.m_original_index = original_index;
    copied.m_in_reserve = false;
    game.m_player[attacker_player].m_die[attacker] = copied;
    game.m_player[attacker_player].m_score += copied.GetScore(true) - old_score;
}

pub(super) fn ConsumeAttackingJolt(game: &mut Game, player: usize, attackers: DieIndexSet) {
    for attacker in attackers.iter() {
        game.m_player[player].m_die[attacker].m_properties &= !property::JOLT;
    }
}

pub(super) fn ConsumeAttackingRage(
    game: &mut Game,
    player: usize,
    attackers: DieIndexSet,
    participated_with_rage: bool,
) {
    if !participated_with_rage {
        return;
    }
    for attacker in attackers.iter() {
        game.m_player[player].m_die[attacker].m_properties &= !property::RAGE;
    }
}

pub(super) fn CreateAndRollRageReplacement(
    game: &Game,
    player: usize,
    target: usize,
    rng: &mut Rng,
) -> Option<Die> {
    let original = game.m_player[player].m_die[target];
    if !original.HasProperty(property::RAGE) {
        return None;
    }
    assert!(
        game.m_player[player].m_die.len() < MAX_DICE,
        "Rage replacement exceeds the transformed dice capacity of {MAX_DICE}"
    );
    let used_indices = game.m_player[player]
        .m_die
        .iter()
        .map(|die| die.m_original_index)
        .collect::<DieIndexSet>();
    let synthetic_index = (0..MAX_DICE)
        .find(|index| !used_indices.contains(*index))
        .expect("Rage replacement has no free stable die index");
    let mut replacement = original;
    replacement.m_properties &= !property::RAGE;
    replacement.m_value_total = None;
    replacement.m_captured = false;
    replacement.m_notset = true;
    replacement.m_dizzy = false;
    replacement.m_original_index = synthetic_index;
    // ButtonWeavers' replacement roll skips Mighty, Weak, and Mood.
    RollDie(&mut replacement, rng);
    Some(replacement)
}

pub(super) fn AddRageReplacement(game: &mut Game, player: usize, replacement: Die) {
    game.m_player[player].m_rage_replacements |= 1 << replacement.m_original_index;
    game.m_player[player].m_score += replacement.GetScore(true);
    let available = AvailableDice(&game.m_player[player]);
    game.m_player[player].m_die.insert(available, replacement);
}

pub(crate) fn ApplyAttackPlayerEffects(
    game: &mut Game,
    action: &Move,
    attacker_player: usize,
    target_player: usize,
    attacker: usize,
    actually_attacking: bool,
) {
    if !game.m_player[attacker_player].m_die[attacker].HasProperty(property::KONSTANT) {
        game.m_player[attacker_player].m_die[attacker].m_notset = true;
    }

    if actually_attacking && action.m_attack == Some(Attack::Berserk) {
        HalveBerserkAttacker(game, attacker_player, attacker);
    }

    if game.m_player[attacker_player].m_die[attacker].m_notset {
        ApplyBeforeRollEffects(game, attacker_player, attacker);
    }

    if MorphingApplies(action)
        && game.m_player[attacker_player].m_die[attacker].HasProperty(property::MORPHING)
    {
        let target = action.m_targets.first().expect("Morphing target");
        MorphIntoTarget(game, attacker_player, target_player, attacker, target);
    }

    if game.m_player[attacker_player].m_die[attacker].HasProperty(property::TURBO)
        && action.m_turbo_option >= 0
    {
        if game.m_player[attacker_player].m_die[attacker].HasProperty(property::OPTION) {
            if action.m_turbo_option == 1 {
                let die = &mut game.m_player[attacker_player].m_die[attacker];
                let old_score = die.GetScore(true);
                die.m_sides.swap(0, 1);
                game.m_player[attacker_player].m_score += die.GetScore(true) - old_score;
            }
        } else if action.m_turbo_option > 0
            && let Some(swing) = game.m_player[attacker_player].m_die[attacker].m_swing_type[0]
        {
            let mut score_delta = 0.0;
            for die in &mut game.m_player[attacker_player].m_die {
                let old_score = die.GetScore(true);
                for side in 0..2 {
                    if die.m_swing_type[side] == Some(swing) {
                        die.m_sides[side] = action.m_turbo_option as u8;
                    }
                }
                score_delta += die.GetScore(true) - old_score;
            }
            game.m_player[attacker_player].m_score += score_delta;
        }
    }

    // ButtonWeavers copies before the attack reroll, so the copy's own Mighty
    // or Weak applies; Warrior is lost after.
    if actually_attacking
        && action.m_attack == Some(Attack::Power)
        && action.m_attackers.len() == 1
        && action.m_targets.len() == 1
        && game.m_player[attacker_player].m_die[attacker].HasProperty(property::DOPPELGANGER)
    {
        let target = action.m_targets.first().expect("Doppelganger target");
        CopyDoppelgangerTarget(game, attacker_player, target_player, attacker, target);
        // The copy rerolls even if Konstant, so Mighty and Weak resize it.
        ResizeMightyAndWeak(game, attacker_player, attacker);
    }

    if game.m_player[attacker_player].m_die[attacker].HasProperty(property::WARRIOR) {
        let die = &mut game.m_player[attacker_player].m_die[attacker];
        let old_score = die.GetScore(true);
        die.m_properties &= !property::WARRIOR;
        game.m_player[attacker_player].m_score += die.GetScore(true) - old_score;
    }
}

// ButtonWeavers Konstant clears `doesReroll`, which Mighty and Weak require.
pub(crate) fn ApplyBeforeRollEffects(game: &mut Game, player: usize, index: usize) {
    if !game.m_player[player].m_die[index].HasProperty(property::KONSTANT) {
        ResizeMightyAndWeak(game, player, index);
    }
}

fn RerollsWhenOrnery(die: &Die) -> bool {
    die.HasProperty(property::ORNERY) && !die.HasProperty(property::WARRIOR)
}

fn ResizeMightyAndWeak(game: &mut Game, player: usize, index: usize) {
    let die = &mut game.m_player[player].m_die[index];
    let old_score = die.GetScore(true);
    let dice = if die.HasProperty(property::TWIN) {
        2
    } else {
        1
    };
    if die.HasProperty(property::MIGHTY) {
        for sides in die.m_sides.iter_mut().take(dice) {
            *sides = MightySides(*sides);
        }
    }
    if die.HasProperty(property::WEAK) {
        for sides in die.m_sides.iter_mut().take(dice) {
            *sides = WeakSides(*sides);
        }
    }
    game.m_player[player].m_score += die.GetScore(true) - old_score;
}

pub(super) fn ApplyAttackerNatureRoll(game: &mut Game, player: usize, index: usize, rng: &mut Rng) {
    let die = &mut game.m_player[player].m_die[index];
    let old_score = die.GetScore(true);
    ApplyMood(die, rng);
    // C++ never rescores after the reroll, so a Value die keeps its old score.
    game.m_player[player].m_score += die.GetScore(true) - old_score;
    if die.m_notset {
        RollDie(die, rng);
    }
}

/// ButtonWeavers resizes Mood and Mad dice on every reroll, including Trip
/// targets and Chance rerolls.
pub(crate) fn RollScheduledDie(game: &mut Game, player: usize, index: usize, rng: &mut Rng) {
    ApplyAttackerNatureRoll(game, player, index, rng);
}

pub(super) fn OnDieLost(player: &mut crate::game::Player, index: usize) {
    let available = AvailableDice(player);
    player.m_die[index].m_captured = true;
    player.m_die[index..available].rotate_left(1);
}

// ButtonWeavers `standard_die_sizes`; unlike Mighty's list, no 16.
const MOOD_DIE_SIZES: [u8; 9] = [1, 2, 4, 6, 8, 10, 12, 20, 30];

pub(super) fn ApplyMood(die: &mut Die, rng: &mut Rng) {
    let mad = die.HasProperty(property::MAD);
    if !mad && !die.HasProperty(property::MOOD) || die.HasProperty(property::KONSTANT) {
        return;
    }
    let Some(swing) = die.m_swing_type.iter().flatten().next().copied() else {
        return;
    };
    let (minimum, maximum) = SwingRange(swing);
    let sizes = (minimum..=maximum)
        .filter(|size| {
            if mad {
                size % 2 == 0
            } else {
                MOOD_DIE_SIZES.contains(size)
            }
        })
        .collect::<Vec<_>>();
    let size = sizes[rng.GetRandMax(sizes.len() as u32) as usize];
    for index in 0..die.m_sides.len() {
        if die.m_swing_type[index].is_some() {
            die.m_sides[index] = size;
        }
    }
}

pub(super) fn MightySides(sides: u8) -> u8 {
    const VALUES: [u8; 20] = [
        1, 2, 4, 4, 6, 6, 8, 8, 10, 10, 12, 12, 16, 16, 16, 16, 20, 20, 20, 20,
    ];
    if sides >= 20 {
        30
    } else {
        VALUES[sides as usize]
    }
}

pub(super) fn WeakSides(sides: u8) -> u8 {
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

pub(crate) fn OptimizeDice(player: &mut Player) {
    player.OptimizeDice();
}

pub(crate) fn RollDie(die: &mut Die, rng: &mut Rng) {
    assert!(die.m_notset, "Die::Roll requires NOTSET state");
    die.m_captured = false;
    die.m_notset = false;
    die.m_dizzy = false;
    if die.m_in_reserve {
        die.m_value_total = None;
        return;
    }
    let dice = if die.HasProperty(property::TWIN) {
        2
    } else {
        1
    };
    let mut value = 0u16;
    for sides in die.m_sides.iter().take(dice) {
        if *sides > 0 {
            if die.HasProperty(property::WARRIOR | property::MAXIMUM) {
                value += u16::from(*sides);
            } else {
                value += u16::from(rng.GetRandMax(u32::from(*sides)) as u8 + 1);
            }
        }
    }
    die.m_value_total = Some(value as u8);
}

pub(crate) fn InitiativeWinner(game: &Game) -> usize {
    CheckInitiative(game).unwrap_or(0)
}

pub(crate) fn CheckInitiative(game: &Game) -> Option<usize> {
    let mut values = [Vec::new(), Vec::new()];
    for (player, output) in values.iter_mut().enumerate() {
        *output = game.m_player[player]
            .m_die
            .iter()
            .filter(|die| {
                die.IsAvailable()
                    && !die.HasProperty(
                        property::TRIP | property::SLOW | property::STINGER | property::RAGE,
                    )
            })
            .map(Die::GetValueTotal)
            .collect();
        output.sort_unstable();
    }
    // ButtonWeavers ranks a no-initiative button below every other button,
    // even one with no initiative dice, by giving the others a sentinel.
    let no_initiative =
        [0, 1].map(|player| game.m_player[player].m_specials & super::special::NO_INITIATIVE != 0);
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

pub(crate) fn AvailableDice(player: &Player) -> usize {
    player.m_die.iter().filter(|die| die.IsAvailable()).count()
}

pub(crate) fn SwingRange(swing: char) -> (u8, u8) {
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

pub(crate) fn RestoreDiceForNewRound(game: &mut Game, template: &Game) {
    for player in 0..game.m_player.len() {
        let products = game.m_player[player].m_radioactive_products
            | game.m_player[player].m_rage_replacements;
        game.m_player[player]
            .m_die
            .retain(|die| products & (1 << die.m_original_index) == 0);
        game.m_player[player].m_round_transformed &= !products;
        game.m_player[player].m_radioactive_products = 0;
        game.m_player[player].m_rage_replacements = 0;
        for index in 0..game.m_player[player].m_die.len() {
            let original_index = game.m_player[player].m_die[index].m_original_index;
            let transformed =
                game.m_player[player].m_round_transformed & (1 << original_index) != 0;
            if transformed {
                let sides = game.m_player[player].m_round_original_sides[original_index];
                let in_reserve = game.m_player[player].m_die[index].m_in_reserve;
                if let Some(original) = template.m_player[player]
                    .m_die
                    .iter()
                    .find(|original| original.m_original_index == original_index)
                {
                    game.m_player[player].m_die[index] = *original;
                    // Swing and Option selections persist for a round winner;
                    // fixed Mighty/Weak side changes do not.
                    if original.HasProperty(property::OPTION) {
                        game.m_player[player].m_die[index].m_sides = sides;
                    } else {
                        for (side, saved) in sides.iter().enumerate() {
                            if original.m_swing_type[side].is_some() {
                                game.m_player[player].m_die[index].m_sides[side] = *saved;
                            }
                        }
                    }
                    game.m_player[player].m_die[index].m_in_reserve = in_reserve;
                }
                game.m_player[player].m_round_transformed &= !(1 << original_index);
            }
            let die = &mut game.m_player[player].m_die[index];
            let Some(original) = template.m_player[player]
                .m_die
                .iter()
                .find(|original| original.m_original_index == die.m_original_index)
            else {
                continue;
            };

            if original.HasProperty(property::JOLT) {
                die.m_properties |= property::JOLT;
            } else {
                die.m_properties &= !property::JOLT;
            }
            if original.HasProperty(property::RAGE) {
                die.m_properties |= property::RAGE;
            } else {
                die.m_properties &= !property::RAGE;
            }
        }
    }
}

pub(crate) fn RollRoundDice(game: &mut Game, rng: &mut Rng) {
    for player in &mut game.m_player {
        player.m_score = 0.0;
        for die in &mut player.m_die {
            die.m_notset = true;
            RollDie(die, rng);
        }
        player.m_score = player
            .m_die
            .iter()
            .filter(|d| d.IsAvailable())
            .map(|d| d.GetScore(true))
            .sum();
        player.OptimizeDice();
    }
}

pub(crate) fn RecoverDizzyDice(player: &mut Player) {
    for die in &mut player.m_die {
        die.m_dizzy = false;
    }
}
