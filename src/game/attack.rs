// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

const DEFAULT_FIRE_CANDIDATE_LIMIT: usize = 500;

struct AvailableDiceList<'a> {
    dice: [Option<(usize, &'a Die)>; MAX_DICE],
    len: usize,
}

impl<'a> AvailableDiceList<'a> {
    fn new(player: &'a Player) -> Self {
        let mut available = Self {
            dice: [None; MAX_DICE],
            len: 0,
        };
        for (index, die) in player.dice.iter().enumerate() {
            if die.is_available() {
                available.dice[available.len] = Some((index, die));
                available.len += 1;
            }
        }
        available
    }

    fn iter(&self) -> impl DoubleEndedIterator<Item = &(usize, &'a Die)> {
        self.dice[..self.len]
            .iter()
            .map(|entry| entry.as_ref().expect("initialized available die"))
    }

    fn len(&self) -> usize {
        self.len
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn first(&self) -> Option<&(usize, &'a Die)> {
        self.dice[..self.len].first().and_then(Option::as_ref)
    }

    fn last(&self) -> Option<&(usize, &'a Die)> {
        self.dice[..self.len].last().and_then(Option::as_ref)
    }
}

impl<'a> std::ops::Index<usize> for AvailableDiceList<'a> {
    type Output = (usize, &'a Die);

    fn index(&self, index: usize) -> &Self::Output {
        self.dice[index]
            .as_ref()
            .expect("initialized available die")
    }
}

#[derive(Clone, Copy)]
struct DieIndexStack {
    indices: [usize; MAX_DICE],
    len: usize,
    value_total: u16,
}

impl DieIndexStack {
    fn new() -> Self {
        Self {
            indices: [0; MAX_DICE],
            len: 0,
            value_total: 0,
        }
    }

    fn values(&self) -> &[usize] {
        &self.indices[..self.len]
    }

    fn push(&mut self, index: usize, dice: &AvailableDiceList<'_>) {
        self.indices[self.len] = index;
        self.len += 1;
        self.value_total += dice[index].1.value_total();
    }

    fn pop(&mut self, dice: &AvailableDiceList<'_>) {
        self.value_total -= dice[self.indices[self.len - 1]].1.value_total();
        self.len -= 1;
    }

    fn cycle(&mut self, mut add_die: bool, dice: &AvailableDiceList<'_>) -> bool {
        if self.indices[self.len - 1] == dice.len() - 1 {
            self.pop(dice);
            if self.len == 0 {
                return true;
            }
            add_die = false;
        }
        if add_die {
            self.push(self.indices[self.len - 1] + 1, dice);
        } else {
            let top = self.len - 1;
            self.value_total -= dice[self.indices[top]].1.value_total();
            self.indices[top] += 1;
            self.value_total += dice[self.indices[top]].1.value_total();
        }
        false
    }
}

fn die_count(die: &Die) -> i32 {
    if die.has_property(property::TWIN) {
        2
    } else {
        1
    }
}

/// Konstant dice may add or subtract their value in a Skill attack.
fn skill_stack_can_hit(
    stack: &DieIndexStack,
    available: &AvailableDiceList<'_>,
    target: u16,
) -> bool {
    skill_stack_can_hit_with_fire(stack, available, target, &[0; MAX_DICE])
}

fn skill_stack_can_hit_with_fire(
    stack: &DieIndexStack,
    available: &AvailableDiceList<'_>,
    target: u16,
    increases: &[u8; MAX_DICE],
) -> bool {
    let subtractable = stack
        .values()
        .iter()
        .filter(|position| {
            let die = available[**position].1;
            die.has_property(property::KONSTANT) && !die.has_property(property::WARRIOR)
        })
        .count();

    for signs in 0..(1usize << subtractable) {
        let mut minimum = 0i32;
        let mut maximum = 0i32;
        let mut sign_bit = 0usize;
        for position in stack.values() {
            let (die_index, die) = available[*position];
            let value = i32::from(die.value_total()) + i32::from(increases[die_index]);
            let is_konstant = die.has_property(property::KONSTANT);
            let variable_stinger =
                die.has_property(property::STINGER) && !die.has_property(property::WARRIOR);
            let term_minimum = if variable_stinger {
                die_count(die)
            } else {
                value
            };

            if is_konstant {
                let may_subtract = !die.has_property(property::WARRIOR);
                let subtract = may_subtract && signs & (1 << sign_bit) != 0;
                if may_subtract {
                    sign_bit += 1;
                }
                if subtract {
                    minimum -= value;
                    maximum -= term_minimum;
                } else {
                    minimum += term_minimum;
                    maximum += value;
                }
            } else {
                minimum += term_minimum;
                maximum += value;
            }
        }
        if i32::from(target) >= minimum && i32::from(target) <= maximum {
            return true;
        }
    }
    false
}

/// Mirrors ButtonWeavers `post_trip_roll_max`, including its Mood Twin quirk.
fn trip_roll_max(die: &Die, smallest_mood_size: bool) -> u16 {
    if die.has_property(property::MOOD)
        && let Some(swing) = die.swing_type[0]
    {
        let (minimum, maximum) = super::swing_range(swing);
        return u16::from(if smallest_mood_size { minimum } else { maximum });
    }
    let dice = die_count(die) as usize;
    die.sides[..dice]
        .iter()
        .map(|sides| {
            let mut sides = *sides;
            if die.has_property(property::WEAK) {
                sides = super::mechanics::weak_sides(sides);
            }
            if die.has_property(property::MIGHTY) {
                sides = super::mechanics::mighty_sides(sides);
            }
            u16::from(sides)
        })
        .sum()
}

/// Mirrors ButtonWeavers `BMAttackTrip::validate_attack`.
fn trip_can_capture(attacker: &Die, target: &Die) -> bool {
    let target_minimum = die_count(target) as u16;
    let attacker_maximum = if attacker.has_property(property::KONSTANT) {
        attacker.value_total()
    } else {
        trip_roll_max(attacker, false)
    };
    if target.has_property(property::KONSTANT) && attacker_maximum < target.value_total() {
        return false;
    }
    if target.has_property(property::MAXIMUM) && attacker_maximum < trip_roll_max(target, true) {
        return false;
    }
    attacker_maximum >= target_minimum
}

fn turbo_sizes(die: &Die, accuracy: f32) -> Vec<(i16, Die)> {
    if !chooses_turbo_size(die) {
        return vec![(-1, *die)];
    }
    if die.has_property(property::OPTION) {
        let mut swapped = *die;
        swapped.sides.swap(0, 1);
        return vec![(0, *die), (1, swapped)];
    }
    let Some(swing) = die.swing_type[0] else {
        return vec![(-1, *die)];
    };
    let (minimum, maximum) = turbo_swing_range(swing);
    let mut choices = vec![die.sides[0], minimum, maximum];
    let step = turbo_step(accuracy);
    let mut candidate = f32::from(minimum + 1);
    while candidate < f32::from(maximum) {
        choices.push(candidate as u8);
        candidate += step;
    }
    let mut sizes = Vec::with_capacity(choices.len());
    for sides in choices {
        if sizes.iter().any(|(chosen, _)| *chosen == i16::from(sides)) {
            continue;
        }
        sizes.push((i16::from(sides), resized_by_turbo(die, sides)));
    }
    sizes
}

/// ButtonWeavers' `setTurboSize` resizes this die alone, both halves of a
/// Twin.
pub(crate) fn resized_by_turbo(die: &Die, sides: u8) -> Die {
    let mut resized = *die;
    for side in 0..2 {
        if die.swing_type[side].is_some() && die.swing_type[side] == die.swing_type[0] {
            resized.sides[side] = sides;
        }
    }
    resized
}

/// Only sizes `expand_turbo_moves` can submit count, so every Trip keeps one.
fn trip_reachable_at_some_turbo_size(attacker: &Die, target: &Die, accuracy: f32) -> bool {
    turbo_sizes(attacker, accuracy)
        .iter()
        .any(|(_, resized)| trip_can_capture(resized, target))
}

fn fire_helper_capacities(player: &Player, attackers: DieIndexSet) -> Vec<(usize, u8)> {
    player
        .dice
        .iter()
        .enumerate()
        .filter(|(index, die)| {
            !attackers.contains(*index) && die.is_available() && die.has_property(property::FIRE)
        })
        .filter_map(|(index, die)| {
            let minimum = die_count(die) as u16;
            let capacity = die.value_total().saturating_sub(minimum);
            (capacity > 0).then_some((index, capacity as u8))
        })
        .collect()
}

fn attacker_fire_capacities(player: &Player, attackers: DieIndexSet) -> Vec<(usize, u8)> {
    attackers
        .iter()
        .filter_map(|index| {
            let die = &player.dice[index];
            let capacity = die
                .sides_max()
                .min(u16::from(u8::MAX))
                .saturating_sub(die.value_total());
            (capacity > 0).then_some((index, capacity as u8))
        })
        .collect()
}

fn fire_allocations(entries: &[(usize, u8)], total: u16, limit: usize) -> Vec<[u8; MAX_DICE]> {
    fn visit(
        entries: &[(usize, u8)],
        position: usize,
        remaining: u16,
        allocation: &mut [u8; MAX_DICE],
        results: &mut Vec<[u8; MAX_DICE]>,
        limit: usize,
    ) {
        if results.len() >= limit {
            return;
        }
        if position == entries.len() {
            if remaining == 0 {
                results.push(*allocation);
            }
            return;
        }
        let remaining_capacity: u16 = entries[position + 1..]
            .iter()
            .map(|(_, capacity)| u16::from(*capacity))
            .sum();
        let (index, capacity) = entries[position];
        let minimum = remaining.saturating_sub(remaining_capacity);
        let maximum = remaining.min(u16::from(capacity));
        for amount in minimum..=maximum {
            allocation[index] = amount as u8;
            visit(
                entries,
                position + 1,
                remaining - amount,
                allocation,
                results,
                limit,
            );
            if results.len() == limit {
                break;
            }
        }
        allocation[index] = 0;
    }

    if entries.is_empty() || limit == 0 {
        return Vec::new();
    }
    let capacity: u16 = entries
        .iter()
        .map(|(_, capacity)| u16::from(*capacity))
        .sum();
    if total == 0 || total > capacity {
        return Vec::new();
    }
    let mut results = Vec::new();
    visit(entries, 0, total, &mut [0; MAX_DICE], &mut results, limit);
    results
}

fn fire_plans_for_power(
    player: &Player,
    attacker: usize,
    minimum: u16,
    limit: usize,
) -> Vec<FireAdjustment> {
    let attackers = DieIndexSet::from([attacker]);
    let die = &player.dice[attacker];
    let attacker_capacity = die
        .sides_max()
        .min(u16::from(u8::MAX))
        .saturating_sub(die.value_total());
    let helper_capacities = fire_helper_capacities(player, attackers);
    let helper_capacity = helper_capacities
        .iter()
        .map(|(_, capacity)| u16::from(*capacity))
        .sum::<u16>();
    let maximum = attacker_capacity.min(helper_capacity);
    if minimum == 0 || minimum > maximum || limit == 0 {
        return Vec::new();
    }
    let mut plans = Vec::new();
    for amount in minimum..=maximum {
        let remaining = limit - plans.len();
        plans.extend(
            fire_allocations(&helper_capacities, amount, remaining)
                .into_iter()
                .map(|reductions| {
                    let mut increases = [0; MAX_DICE];
                    increases[attacker] = amount as u8;
                    FireAdjustment::from_allocations(increases, reductions)
                }),
        );
        if plans.len() == limit {
            break;
        }
    }
    plans
}

fn fire_plans_for_skill(
    player: &Player,
    available: &AvailableDiceList<'_>,
    stack: &DieIndexStack,
    target: u16,
    limit: usize,
) -> Vec<FireAdjustment> {
    let attackers = stack
        .values()
        .iter()
        .map(|position| available[*position].0)
        .collect::<DieIndexSet>();
    let attacker_capacities = attacker_fire_capacities(player, attackers);
    let helper_capacities = fire_helper_capacities(player, attackers);
    let maximum: u16 = attacker_capacities
        .iter()
        .map(|(_, capacity)| u16::from(*capacity))
        .sum::<u16>()
        .min(
            helper_capacities
                .iter()
                .map(|(_, capacity)| u16::from(*capacity))
                .sum(),
        );
    if limit == 0 {
        return Vec::new();
    }
    let mut plans = Vec::new();
    for amount in 1..=maximum {
        let reductions = fire_allocations(&helper_capacities, amount, limit - plans.len());
        if reductions.is_empty() {
            continue;
        }
        for increases in fire_allocations(&attacker_capacities, amount, limit - plans.len()) {
            if !skill_stack_can_hit_with_fire(stack, available, target, &increases) {
                continue;
            }
            let remaining = limit - plans.len();
            plans.extend(
                reductions
                    .iter()
                    .take(remaining)
                    .map(|reduction| FireAdjustment::from_allocations(increases, *reduction)),
            );
            if plans.len() == limit {
                return plans;
            }
        }
    }
    plans
}

impl Game {
    fn attack_candidates(&self, fire_limit: usize) -> Vec<Move> {
        let attacker = &self.players[0];
        let target = &self.players[1];
        let available = AvailableDiceList::new(attacker);
        let targets = AvailableDiceList::new(target);
        let target_max = targets.first().map_or(0, |(_, die)| die.value_total());
        let target_min = targets.last().map_or(0, |(_, die)| die.value_total());
        let player_has_variable_skill_value = available.iter().any(|(_, die)| {
            !die.has_property(property::WARRIOR)
                && die.has_property(property::STINGER | property::KONSTANT)
        });
        let skill_attacks_allowed = attacker.specials & special::NO_SKILL_ATTACKS == 0
            && target.specials & special::SKILL_IMMUNE == 0;
        let player_has_fire = available.iter().any(|(_, die)| {
            die.has_property(property::FIRE) && die.value_total() > die_count(die) as u16
        });
        let targets_have_rush = targets
            .iter()
            .any(|(_, die)| die.has_property(property::RUSH));
        let mut moves = Vec::with_capacity(32);
        // Reserve the bounded Fire budget for required attacks before optional overshoots.
        let mut optional_fire_moves = Vec::new();
        let mut optional_fire_remaining = fire_limit;
        let mut fire_remaining = fire_limit;
        for attacker_position in 0..available.len() {
            let (attacker_index, attacker_die) = available[attacker_position];
            for attack in [
                Attack::Power,
                Attack::Skill,
                Attack::Berserk,
                Attack::Speed,
                Attack::Trip,
                Attack::Shadow,
                Attack::Rush,
                Attack::Boom,
            ] {
                match attack {
                    Attack::Power | Attack::Trip | Attack::Shadow => {
                        if !attacker_die.can_do_attack(attack, 1) {
                            continue;
                        }
                        for (target_index, target_die) in targets.iter().rev() {
                            if !target_die.can_be_attacked(attack, 1) {
                                continue;
                            }
                            let legal = match attack {
                                Attack::Power => {
                                    attacker_die.value_total() >= target_die.value_total()
                                }
                                Attack::Shadow => {
                                    attacker_die.value_total() <= target_die.value_total()
                                        && attacker_die.sides_max() >= target_die.value_total()
                                }
                                Attack::Trip => trip_reachable_at_some_turbo_size(
                                    attacker_die,
                                    target_die,
                                    self.turbo_accuracy,
                                ),
                                _ => unreachable!(),
                            };
                            if legal {
                                let score = match attack {
                                    Attack::Power => {
                                        target_die.score(false)
                                            - if attacker_die.has_property(property::VALUE) {
                                                attacker_die.value_total() as f32 * 0.02
                                            } else {
                                                0.0
                                            }
                                    }
                                    Attack::Trip => target_die.score(false) * 0.2,
                                    Attack::Shadow => target_die.score(false),
                                    _ => unreachable!(),
                                };
                                moves.push(Move::new_attack(
                                    attack,
                                    [attacker_index],
                                    [*target_index],
                                    score,
                                ));
                            }
                            if player_has_fire && attack == Attack::Power && !legal {
                                let minimum = target_die
                                    .value_total()
                                    .saturating_sub(attacker_die.value_total())
                                    .max(1);
                                for fire in fire_plans_for_power(
                                    attacker,
                                    attacker_index,
                                    minimum,
                                    fire_remaining,
                                ) {
                                    let mut candidate = Move::new_attack(
                                        attack,
                                        [attacker_index],
                                        [*target_index],
                                        target_die.score(false),
                                    );
                                    candidate.fire = fire;
                                    moves.push(candidate);
                                    fire_remaining -= 1;
                                }
                            } else if player_has_fire
                                && attack == Attack::Power
                                && self.fire_overshooting
                            {
                                for fire in fire_plans_for_power(
                                    attacker,
                                    attacker_index,
                                    1,
                                    optional_fire_remaining,
                                ) {
                                    let mut candidate = Move::new_attack(
                                        attack,
                                        [attacker_index],
                                        [*target_index],
                                        target_die.score(false),
                                    );
                                    candidate.fire = fire;
                                    optional_fire_moves.push(candidate);
                                    optional_fire_remaining -= 1;
                                }
                            }
                        }
                    }
                    Attack::Skill if !skill_attacks_allowed => {}
                    Attack::Skill => {
                        let mut stack = DieIndexStack::new();
                        stack.push(attacker_position, &available);
                        loop {
                            let stack_len = stack.len;
                            let dice_legal = stack.values().iter().all(|position| {
                                available[*position]
                                    .1
                                    .can_do_attack(Attack::Skill, stack_len)
                            });
                            let warriors = stack
                                .values()
                                .iter()
                                .filter(|position| {
                                    available[**position].1.has_property(property::WARRIOR)
                                })
                                .count();
                            let single_konstant = stack_len == 1
                                && available[stack.values()[0]]
                                    .1
                                    .has_property(property::KONSTANT);
                            // ButtonWeavers requires a non-Warrior participant.
                            if dice_legal
                                && warriors <= 1
                                && warriors < stack_len
                                && !single_konstant
                            {
                                let stack_has_stinger = stack.values().iter().any(|position| {
                                    let die = available[*position].1;
                                    !die.has_property(property::WARRIOR)
                                        && die.has_property(property::STINGER)
                                });
                                let stack_has_konstant = stack.values().iter().any(|position| {
                                    let die = available[*position].1;
                                    !die.has_property(property::WARRIOR)
                                        && die.has_property(property::KONSTANT)
                                });
                                let minimum = stack
                                    .values()
                                    .iter()
                                    .map(|position| {
                                        let die = available[*position].1;
                                        if die.has_property(property::STINGER) {
                                            1
                                        } else {
                                            die.value_total()
                                        }
                                    })
                                    .sum::<u16>();
                                let flexible_stinger =
                                    stack_has_stinger && !stack_has_konstant && stack_len > 1;
                                for (target_index, target_die) in targets.iter() {
                                    if flexible_stinger && target_die.value_total() < minimum {
                                        break;
                                    }
                                    if !stack_has_konstant
                                        && !flexible_stinger
                                        && target_die.value_total() < stack.value_total
                                    {
                                        break;
                                    }
                                    let candidate_value = if stack_has_konstant {
                                        true
                                    } else if flexible_stinger {
                                        target_die.value_total() <= stack.value_total
                                    } else {
                                        target_die.value_total() == stack.value_total
                                    };
                                    let direct = candidate_value
                                        && skill_stack_can_hit(
                                            &stack,
                                            &available,
                                            target_die.value_total(),
                                        );
                                    if direct
                                        && target_die.can_be_attacked(Attack::Skill, stack_len)
                                    {
                                        moves.push(Move::new_attack(
                                            attack,
                                            stack
                                                .values()
                                                .iter()
                                                .map(|position| available[*position].0)
                                                .collect::<DieIndexSet>(),
                                            [*target_index],
                                            target_die.score(false),
                                        ));
                                    }
                                    if player_has_fire
                                        && !direct
                                        && target_die.can_be_attacked(Attack::Skill, stack_len)
                                    {
                                        let attacker_indices = stack
                                            .values()
                                            .iter()
                                            .map(|position| available[*position].0)
                                            .collect::<DieIndexSet>();
                                        for fire in fire_plans_for_skill(
                                            attacker,
                                            &available,
                                            &stack,
                                            target_die.value_total(),
                                            fire_remaining,
                                        ) {
                                            let mut candidate = Move::new_attack(
                                                attack,
                                                attacker_indices,
                                                [*target_index],
                                                target_die.score(false),
                                            );
                                            candidate.fire = fire;
                                            moves.push(candidate);
                                            fire_remaining -= 1;
                                        }
                                    }
                                }
                            }
                            if !player_has_variable_skill_value
                                && stack.len == available.len()
                                && stack.value_total <= target_min
                            {
                                break;
                            }
                            let finished = if !player_has_variable_skill_value
                                && stack.value_total >= target_max
                            {
                                stack.cycle(false, &available)
                            } else {
                                stack.cycle(true, &available)
                            };
                            if finished || stack.len == 1 {
                                break;
                            }
                        }
                    }
                    Attack::Boom => {
                        if !attacker_die.can_do_attack(attack, 1) {
                            continue;
                        }
                        for (target_index, target_die) in targets.iter() {
                            if target_die.can_be_attacked(attack, 1) {
                                moves.push(Move::new_attack(
                                    attack,
                                    [attacker_index],
                                    [*target_index],
                                    0.0,
                                ));
                            }
                        }
                    }
                    Attack::Rush => {
                        // A Speed die's two-target Speed attack resolves identically.
                        let attacker_has_rush = attacker_die.has_property(property::RUSH);
                        if !attacker_has_rush && !targets_have_rush
                            || !attacker_die.can_do_attack(attack, 1)
                            || attacker_die.can_do_attack(Attack::Speed, 1)
                        {
                            continue;
                        }
                        for first in 0..targets.len() {
                            let (first_index, first_die) = targets[first];
                            if !first_die.can_be_attacked(attack, 1) {
                                continue;
                            }
                            for &(second_index, second_die) in targets.iter().skip(first + 1) {
                                if first_die.value_total() + second_die.value_total()
                                    != attacker_die.value_total()
                                    || !second_die.can_be_attacked(attack, 1)
                                {
                                    continue;
                                }
                                if !attacker_has_rush
                                    && !first_die.has_property(property::RUSH)
                                    && !second_die.has_property(property::RUSH)
                                {
                                    continue;
                                }
                                moves.push(Move::new_attack(
                                    attack,
                                    [attacker_index],
                                    [first_index, second_index],
                                    first_die.score(false) + second_die.score(false),
                                ));
                            }
                        }
                    }
                    Attack::Berserk | Attack::Speed => {
                        if !attacker_die.can_do_attack(attack, 1) || targets.is_empty() {
                            continue;
                        }
                        let mut stack = DieIndexStack::new();
                        stack.push(0, &targets);
                        loop {
                            if attacker_die.value_total() == stack.value_total
                                && stack
                                    .values()
                                    .iter()
                                    .all(|position| targets[*position].1.can_be_attacked(attack, 1))
                            {
                                let target_indices = stack
                                    .values()
                                    .iter()
                                    .map(|position| targets[*position].0)
                                    .collect::<DieIndexSet>();
                                let score = stack
                                    .values()
                                    .iter()
                                    .map(|position| targets[*position].1.score(false))
                                    .sum();
                                moves.push(Move::new_attack(
                                    attack,
                                    [attacker_index],
                                    target_indices,
                                    score,
                                ));
                            }
                            if stack.len == targets.len()
                                && attacker_die.value_total() >= stack.value_total
                            {
                                break;
                            }
                            let finished = if attacker_die.value_total() <= stack.value_total {
                                stack.cycle(false, &targets)
                            } else {
                                stack.cycle(true, &targets)
                            };
                            if finished {
                                break;
                            }
                        }
                    }
                }
            }
        }
        optional_fire_moves.truncate(fire_remaining);
        moves.extend(optional_fire_moves);
        moves
    }

    /// Best-scoring first, which the score-driven policies rely on.
    pub fn valid_attacks_by_score(&self) -> Vec<Move> {
        self.valid_attacks_by_score_within(usize::MAX)
    }

    fn valid_attacks_by_score_within(&self, fire_limit: usize) -> Vec<Move> {
        let mut moves = self.attack_candidates(fire_limit);
        moves.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| attack_preference(a.attack).cmp(&attack_preference(b.attack)))
        });
        expand_turbo_moves(self, &mut moves);
        moves
    }

    /// In generation order, which search keeps so a candidate's index, and
    /// so its replay stream, stays put.
    pub(crate) fn valid_attacks(&self, fire_limit: usize) -> Vec<Move> {
        let mut moves = self.attack_candidates(fire_limit);
        expand_turbo_moves(self, &mut moves);
        moves
    }

    pub fn attack_action(&self) -> Move {
        let moves = self.valid_attacks_by_score_within(DEFAULT_FIRE_CANDIDATE_LIMIT);
        if self.surrender_allowed && self.players[1].score - self.players[0].score >= 20.0 {
            return Move {
                action: Action::Surrender,
                attack: None,
                attackers: Vec::new().into(),
                targets: Vec::new().into(),
                score: 0.0,
                turbo_option: -1,
                fire: FireAdjustment::default(),
            };
        }
        if let Some(best) = moves.first() {
            return best.clone();
        }
        Move {
            action: if self.surrender_allowed {
                Action::Surrender
            } else {
                Action::Pass
            },
            attack: None,
            attackers: Vec::new().into(),
            targets: Vec::new().into(),
            score: 0.0,
            turbo_option: -1,
            fire: FireAdjustment::default(),
        }
    }

    pub fn attack_action_deep(&self) -> Move {
        let moves = self.valid_attacks_by_score_within(DEFAULT_FIRE_CANDIDATE_LIMIT);
        moves
            .into_iter()
            .filter(|candidate| candidate.action == Action::Attack)
            .min_by(|a, b| {
                let a_target = a
                    .targets
                    .iter()
                    .map(|index| self.players[1].dice[index].score(false))
                    .sum::<f32>();
                let b_target = b
                    .targets
                    .iter()
                    .map(|index| self.players[1].dice[index].score(false))
                    .sum::<f32>();
                let a_attacker = a.attackers.first().unwrap_or(0);
                let b_attacker = b.attackers.first().unwrap_or(0);
                a_target
                    .total_cmp(&b_target)
                    .then_with(|| {
                        self.players[0].dice[b_attacker]
                            .sides_max()
                            .cmp(&self.players[0].dice[a_attacker].sides_max())
                    })
                    .then_with(|| b_attacker.cmp(&a_attacker))
            })
            .unwrap_or_else(|| self.attack_action())
    }
}

/// ButtonWeavers asks only attacking Turbo dice that reroll, and a fixed die
/// has no other size; Konstant dice keep their value.
fn chooses_turbo_size(die: &Die) -> bool {
    die.has_property(property::TURBO)
        && !die.has_property(property::KONSTANT)
        && (die.has_property(property::OPTION) || die.swing_type[0].is_some())
}

/// The die an attack's Turbo selection resizes. ButtonWeavers lets every
/// attacking Turbo die choose a size; offering one die's sizes keeps the
/// candidates linear instead of a product of sizes, and the others keep
/// theirs, which ButtonWeavers also accepts.
pub(crate) fn turbo_attacker(player: &Player, action: &Move) -> Option<usize> {
    // The Boom die leaves play without rolling.
    if matches!(action.attack, Some(Attack::Boom) | None) {
        return None;
    }
    // A die that has just morphed is a new die and is not asked.
    let morphs = super::mechanics::morphing_applies(action);
    action.attackers.iter().find(|&index| {
        let die = &player.dice[index];
        chooses_turbo_size(die) && !(morphs && die.has_property(property::MORPHING))
    })
}

fn expand_turbo_moves(game: &Game, moves: &mut Vec<Move>) {
    let player = &game.players[0];
    if !player
        .dice
        .iter()
        .any(|die| die.is_available() && die.has_property(property::TURBO))
    {
        return;
    }
    let accuracy = game.turbo_accuracy;
    let original_move_count = moves.len();
    for move_index in 0..original_move_count {
        let Some(turbo_index) = turbo_attacker(player, &moves[move_index]) else {
            continue;
        };
        let turbo_die = &player.dice[turbo_index];
        // Decay strips Turbo before the attack reroll, so sizes only matter for
        // Trip, which ButtonWeavers rolls at the chosen size before decaying.
        if moves[move_index].attack != Some(Attack::Trip)
            && super::mechanics::radioactive_decay_applies(game, &moves[move_index], 0, 1)
        {
            continue;
        }
        if moves[move_index].attack == Some(Attack::Trip) {
            let target = moves[move_index].targets.first().expect("Trip target");
            let target_die = &game.players[1].dice[target];
            let legal = turbo_sizes(turbo_die, accuracy)
                .into_iter()
                .filter(|(_, resized)| trip_can_capture(resized, target_die))
                .map(|(size, _)| size)
                .collect::<Vec<_>>();
            let (first, rest) = legal
                .split_first()
                .expect("generated Trips have a legal Turbo size");
            moves[move_index].turbo_option = *first;
            for size in rest {
                let mut changed = moves[move_index].clone();
                changed.turbo_option = *size;
                moves.push(changed);
            }
            continue;
        }
        if turbo_die.has_property(property::OPTION) {
            moves[move_index].turbo_option = 0;
            // The Fire plan assumed the current size and may not fit another.
            if !moves[move_index].fire.is_empty() {
                continue;
            }
            let mut changed = moves[move_index].clone();
            changed.turbo_option = 1;
            moves.push(changed);
        } else if turbo_die.swing_type[0].is_some() {
            // `turbo_sizes` lists the current size first.
            let mut sizes = turbo_sizes(turbo_die, accuracy).into_iter();
            moves[move_index].turbo_option = sizes.next().expect("the current size").0;
            if !moves[move_index].fire.is_empty() {
                continue;
            }
            for (size, _) in sizes {
                let mut changed = moves[move_index].clone();
                changed.turbo_option = size;
                moves.push(changed);
            }
        }
    }
}

/// An infinite accuracy would make the step zero and never advance.
fn turbo_step(accuracy: f32) -> f32 {
    let step = if accuracy <= 0.0 {
        1000.0
    } else {
        1.0 / accuracy
    };
    if step == 0.0 { 1.0 } else { step }
}

fn turbo_swing_range(swing: char) -> (u8, u8) {
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

fn attack_preference(attack: Option<Attack>) -> u8 {
    match attack {
        Some(Attack::Power) => 0,
        Some(Attack::Skill) => 1,
        Some(Attack::Berserk) => 2,
        Some(Attack::Speed) => 3,
        Some(Attack::Trip) => 4,
        Some(Attack::Shadow) => 5,
        Some(Attack::Rush) => 6,
        Some(Attack::Boom) => 7,
        None => 8,
    }
}
