// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{
    BMC_Die, BMC_DieIndexSet, BMC_Game, BMC_Move, BMC_Player, BMD_MAX_DICE, BME_ATTACK, property,
};
use crate::rng::BMC_RNG;

pub(crate) fn ApplyAttack(game: &mut BMC_Game, action: &BMC_Move, rng: &mut BMC_RNG) -> bool {
    ApplyAttackForPlayers(game, action, 0, 1, rng)
}

pub(crate) fn ApplyAttackForPlayers(
    game: &mut BMC_Game,
    action: &BMC_Move,
    attacker_player: usize,
    target_player: usize,
    rng: &mut BMC_RNG,
) -> bool {
    ApplyFireAdjustments(game, action, attacker_player);
    // C++ GetAvailableDice() is a cached boundary and does not shrink merely
    // because an attacker is marked NOTSET during this phase.
    let mut available_attackers = AvailableDice(&game.m_player[attacker_player]);
    let is_trip = action.m_attack == Some(BME_ATTACK::TRIP);
    let attacking_jolt_dice = action
        .m_attackers
        .iter()
        .filter(|index| game.m_player[attacker_player].m_die[*index].HasProperty(property::JOLT))
        .collect::<BMC_DieIndexSet>();
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
        BMC_DieIndexSet::default()
    } else {
        attacking_jolt_dice
    };
    let rerolling_attackers = actual_attackers
        .iter()
        .filter(|index| {
            !game.m_player[attacker_player].m_die[*index].HasProperty(property::KONSTANT)
        })
        .collect::<BMC_DieIndexSet>();
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

    // C++ handles Ornery dice that were not already scheduled by the attack.
    if action.m_attack.is_some() {
        for attacker in 0..available_attackers {
            let die = &game.m_player[attacker_player].m_die[attacker];
            if !die.HasProperty(property::ORNERY) || die.m_notset {
                continue;
            }
            if !die.HasProperty(property::KONSTANT) {
                game.m_player[attacker_player].m_die[attacker].m_notset = true;
            }
            ApplyBeforeRollEffects(game, attacker_player, attacker);
        }
    }

    // ButtonWeavers consumes ordinary attacking Jolt before the attack reroll.
    // Trip resolves both rerolls first and consumes attacking Jolt below.
    if !is_trip {
        ConsumeAttackingJolt(game, attacker_player, jolt_dice_to_consume);
        ConsumeAttackingRage(game, attacker_player, actual_attackers, attacking_rage);
    }
    // Match ApplyAttackNatureRoll: actual attackers first, then Ornery dice
    // that did not participate, and finally a Trip target.
    for attacker in actual_attackers.iter() {
        ApplyAttackerNatureRoll(game, attacker_player, attacker, rng);
    }
    if action.m_attack.is_some() {
        for attacker in 0..available_attackers {
            let die = &game.m_player[attacker_player].m_die[attacker];
            if die.HasProperty(property::ORNERY) && !actual_attackers.contains(attacker) {
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

    // Time and Space is a post-roll effect. It applies even when a Trip fails,
    // but Konstant attackers cannot trigger it because they do not reroll.
    let time_and_space_extra_turn = actual_attackers.iter().any(|index| {
        let die = &game.m_player[attacker_player].m_die[index];
        die.HasProperty(property::TIME_AND_SPACE)
            && rerolling_attackers.contains(index)
            && die.GetValueTotal() % 2 == 1
    });

    if is_trip {
        let target = action.m_targets.first().expect("Trip target");
        let attacker = action.m_attackers.first().expect("Trip attacker");
        let trip_failed = game.m_player[attacker_player].m_die[attacker].GetValueTotal()
            < game.m_player[target_player].m_die[target].GetValueTotal();
        if decays {
            // Trip rolls resolve before ButtonWeavers' capture hooks, so decay
            // happens after them, even when the Trip fails.
            let products = SplitRadioactiveAttacker(game, attacker_player, attacker);
            for product in products.iter() {
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
    attacking_jolt || captured_jolt || time_and_space_extra_turn
}

pub(super) fn ApplyFireAdjustments(game: &mut BMC_Game, action: &BMC_Move, player: usize) {
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
    for index in 0..crate::game::BMD_MAX_DICE {
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
    game: &BMC_Game,
    action: &BMC_Move,
    attacker_player: usize,
    target_player: usize,
) -> bool {
    if action.m_attack.is_none() || action.m_attackers.len() != 1 || action.m_targets.len() != 1 {
        return false;
    }
    let (Some(attacker), Some(target)) = (action.m_attackers.first(), action.m_targets.first())
    else {
        return false;
    };
    // Rage replacements and failed Trips can keep decay going past the
    // 20-slot pool; skipping the split there beats panicking mid-search.
    if game.m_player[attacker_player].m_die.len() >= BMD_MAX_DICE {
        return false;
    }
    game.m_player[attacker_player].m_die[attacker].HasProperty(property::RADIOACTIVE)
        || game.m_player[target_player].m_die[target].HasProperty(property::RADIOACTIVE)
}

/// ButtonWeavers runs a Radioactive target's decay hook after every attacker
/// hook, so only a Radioactive attacker decays before Doppelganger copies.
pub(crate) fn ApplyRadioactiveAttackEffects(
    game: &mut BMC_Game,
    action: &BMC_Move,
    attacker_player: usize,
    target_player: usize,
    attacker: usize,
) -> BMC_DieIndexSet {
    let target = action.m_targets.first().expect("Radioactive target");
    let original = game.m_player[attacker_player].m_die[attacker];
    let attacker_is_radioactive = original.HasProperty(property::RADIOACTIVE);
    let copies_target =
        action.m_attack == Some(BME_ATTACK::POWER) && original.HasProperty(property::DOPPELGANGER);

    // Warrior dice never attack alone, and decay strips Turbo before it
    // could resize, so neither effect from ApplyAttackPlayerEffects applies.
    if action.m_attack == Some(BME_ATTACK::BERSERK) {
        HalveBerserkAttacker(game, attacker_player, attacker);
    }
    if MorphingApplies(action) && original.HasProperty(property::MORPHING) {
        MorphIntoTarget(game, attacker_player, target_player, attacker, target);
    }
    if copies_target && !attacker_is_radioactive {
        CopyDoppelgangerTarget(game, attacker_player, target_player, attacker, target);
    }

    let products = SplitRadioactiveAttacker(game, attacker_player, attacker);
    for product in products.iter() {
        if copies_target && attacker_is_radioactive {
            CopyDoppelgangerTarget(game, attacker_player, target_player, product, target);
        // ButtonWeavers resets doesReroll on Doppelganger copies, so only the
        // original die's Konstant can stop the resize.
        } else if !original.HasProperty(property::KONSTANT) {
            ApplyBeforeRollEffects(game, attacker_player, product);
        }
    }
    products
}

/// Matches ButtonWeavers `BMDie::split` and `BMDieTwin::split`, whose order
/// decides which product keeps each rounded-up half.
pub(crate) fn SplitRadioactiveAttacker(
    game: &mut BMC_Game,
    attacker_player: usize,
    attacker: usize,
) -> BMC_DieIndexSet {
    assert!(
        game.m_player[attacker_player].m_die.len() < BMD_MAX_DICE,
        "Radioactive decay exceeds the transformed dice capacity of {BMD_MAX_DICE}"
    );

    let original = game.m_player[attacker_player].m_die[attacker];
    let original_index = original.m_original_index;
    let used_indices = game.m_player[attacker_player]
        .m_die
        .iter()
        .map(|die| die.m_original_index)
        .collect::<BMC_DieIndexSet>();
    let synthetic_index = (0..BMD_MAX_DICE)
        .find(|index| !used_indices.contains(*index))
        .expect("Radioactive decay has no free stable die index");
    let mut first = original;
    let mut second = original;
    let removed = property::RADIOACTIVE
        | property::TURBO
        | property::MOOD
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

fn HalveBerserkAttacker(game: &mut BMC_Game, attacker_player: usize, attacker: usize) {
    let die = &mut game.m_player[attacker_player].m_die[attacker];
    let old_score = die.GetScore(true);
    die.m_sides[0] = die.m_sides[0].div_ceil(2);
    die.m_properties &= !property::BERSERK;
    game.m_player[attacker_player].m_score += die.GetScore(true) - old_score;
}

fn MorphingApplies(action: &BMC_Move) -> bool {
    action.m_targets.len() == 1
        && !matches!(
            action.m_attack,
            Some(BME_ATTACK::BERSERK | BME_ATTACK::SPEED)
        )
}

fn MorphIntoTarget(
    game: &mut BMC_Game,
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
    game: &mut BMC_Game,
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

pub(super) fn ConsumeAttackingJolt(game: &mut BMC_Game, player: usize, attackers: BMC_DieIndexSet) {
    for attacker in attackers.iter() {
        game.m_player[player].m_die[attacker].m_properties &= !property::JOLT;
    }
}

pub(super) fn ConsumeAttackingRage(
    game: &mut BMC_Game,
    player: usize,
    attackers: BMC_DieIndexSet,
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
    game: &BMC_Game,
    player: usize,
    target: usize,
    rng: &mut BMC_RNG,
) -> Option<BMC_Die> {
    let original = game.m_player[player].m_die[target];
    if !original.HasProperty(property::RAGE) {
        return None;
    }
    assert!(
        game.m_player[player].m_die.len() < BMD_MAX_DICE,
        "Rage replacement exceeds the transformed dice capacity of {BMD_MAX_DICE}"
    );
    let used_indices = game.m_player[player]
        .m_die
        .iter()
        .map(|die| die.m_original_index)
        .collect::<BMC_DieIndexSet>();
    let synthetic_index = (0..BMD_MAX_DICE)
        .find(|index| !used_indices.contains(*index))
        .expect("Rage replacement has no free stable die index");
    let mut replacement = original;
    replacement.m_properties &= !property::RAGE;
    replacement.m_value_total = None;
    replacement.m_captured = false;
    replacement.m_notset = true;
    replacement.m_dizzy = false;
    replacement.m_original_index = synthetic_index;
    // ButtonWeavers marks Rage replacements specially: Mighty and Weak do not
    // change size on this initial roll, and Mood does not trigger because the
    // newly cloned die has no value yet.
    RollDie(&mut replacement, rng);
    Some(replacement)
}

pub(super) fn AddRageReplacement(game: &mut BMC_Game, player: usize, replacement: BMC_Die) {
    game.m_player[player].m_rage_replacements |= 1 << replacement.m_original_index;
    game.m_player[player].m_score += replacement.GetScore(true);
    let available = AvailableDice(&game.m_player[player]);
    game.m_player[player].m_die.insert(available, replacement);
}

pub(crate) fn ApplyAttackPlayerEffects(
    game: &mut BMC_Game,
    action: &BMC_Move,
    attacker_player: usize,
    target_player: usize,
    attacker: usize,
    actually_attacking: bool,
) {
    if !game.m_player[attacker_player].m_die[attacker].HasProperty(property::KONSTANT) {
        game.m_player[attacker_player].m_die[attacker].m_notset = true;
    }

    if actually_attacking && action.m_attack == Some(BME_ATTACK::BERSERK) {
        HalveBerserkAttacker(game, attacker_player, attacker);
    }

    if game.m_player[attacker_player].m_die[attacker].m_notset {
        ApplyBeforeRollEffects(game, attacker_player, attacker);
    }

    // Morphing is implemented by C++ only for its 1_1 and N_1 attack types.
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

    // ButtonWeavers runs Doppelganger after Mighty, Weak, and Turbo, but
    // before Warrior and the attack reroll. A successful single-die Power
    // attack replaces the attacking recipe with an exact copy of its target.
    if actually_attacking
        && action.m_attack == Some(BME_ATTACK::POWER)
        && action.m_attackers.len() == 1
        && action.m_targets.len() == 1
        && game.m_player[attacker_player].m_die[attacker].HasProperty(property::DOPPELGANGER)
    {
        let target = action.m_targets.first().expect("Doppelganger target");
        CopyDoppelgangerTarget(game, attacker_player, target_player, attacker, target);
    }

    if game.m_player[attacker_player].m_die[attacker].HasProperty(property::WARRIOR) {
        let die = &mut game.m_player[attacker_player].m_die[attacker];
        let old_score = die.GetScore(true);
        die.m_properties &= !property::WARRIOR;
        game.m_player[attacker_player].m_score += die.GetScore(true) - old_score;
    }
}

pub(crate) fn ApplyBeforeRollEffects(game: &mut BMC_Game, player: usize, index: usize) {
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

pub(super) fn ApplyAttackerNatureRoll(
    game: &mut BMC_Game,
    player: usize,
    index: usize,
    rng: &mut BMC_RNG,
) {
    let die = &mut game.m_player[player].m_die[index];
    let old_score = die.GetScore(true);
    ApplyMood(die, rng);
    // C++ score bookkeeping surrounds side/property changes, but Roll itself
    // does not notify the owner. In particular, a Value die keeps the score
    // contributed by its pre-attack value after the nature reroll.
    game.m_player[player].m_score += die.GetScore(true) - old_score;
    if die.m_notset {
        RollDie(die, rng);
    }
}

pub(crate) fn RollScheduledDie(
    game: &mut BMC_Game,
    player: usize,
    index: usize,
    rng: &mut BMC_RNG,
) {
    let die = &mut game.m_player[player].m_die[index];
    RollDie(die, rng);
}

pub(super) fn OnDieLost(player: &mut crate::game::BMC_Player, index: usize) {
    let available = AvailableDice(player);
    let mut lost = player.m_die.remove(index);
    lost.m_captured = true;
    player.m_die.insert(available - 1, lost);
}

pub(super) fn ApplyMood(die: &mut BMC_Die, rng: &mut BMC_RNG) {
    if die.HasProperty(property::MOOD) {
        for index in 0..die.m_sides.len() {
            if let Some(swing) = die.m_swing_type[index] {
                die.m_sides[index] = match swing {
                    'X' => [4, 6, 8, 10, 12, 20][rng.GetRandMax(6) as usize],
                    'V' => [6, 8, 10, 12][rng.GetRandMax(4) as usize],
                    _ => {
                        let (min, max) = SwingRange(swing);
                        min + rng.GetRandMax(u32::from(max - min + 1)) as u8
                    }
                };
            }
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

pub(crate) fn OptimizeDice(player: &mut BMC_Player) {
    player.OptimizeDice();
}

pub(crate) fn RollDie(die: &mut BMC_Die, rng: &mut BMC_RNG) {
    assert!(die.m_notset, "BMC_Die::Roll requires NOTSET state");
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

pub(crate) fn InitiativeWinner(game: &BMC_Game) -> usize {
    CheckInitiative(game).unwrap_or(0)
}

pub(crate) fn CheckInitiative(game: &BMC_Game) -> Option<usize> {
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
            .map(BMC_Die::GetValueTotal)
            .collect();
        output.sort_unstable();
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

pub(crate) fn AvailableDice(player: &BMC_Player) -> usize {
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

pub(crate) fn RestoreDiceForNewRound(game: &mut BMC_Game, template: &BMC_Game) {
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

pub(crate) fn RollRoundDice(game: &mut BMC_Game, rng: &mut BMC_RNG) {
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

pub(crate) fn RecoverDizzyDice(player: &mut BMC_Player) {
    for die in &mut player.m_die {
        die.m_dizzy = false;
    }
}
