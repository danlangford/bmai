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
        for (index, die) in player.m_die.iter().enumerate() {
            if die.IsAvailable() {
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
        self.value_total += dice[index].1.GetValueTotal();
    }

    fn pop(&mut self, dice: &AvailableDiceList<'_>) {
        self.value_total -= dice[self.indices[self.len - 1]].1.GetValueTotal();
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
            self.value_total -= dice[self.indices[top]].1.GetValueTotal();
            self.indices[top] += 1;
            self.value_total += dice[self.indices[top]].1.GetValueTotal();
        }
        false
    }
}

fn DieCount(die: &Die) -> i32 {
    if die.HasProperty(property::TWIN) {
        2
    } else {
        1
    }
}

/// Konstant dice may add or subtract their value in a Skill attack.
fn SkillStackCanHit(stack: &DieIndexStack, available: &AvailableDiceList<'_>, target: u16) -> bool {
    SkillStackCanHitWithFire(stack, available, target, &[0; MAX_DICE])
}

fn SkillStackCanHitWithFire(
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
            die.HasProperty(property::KONSTANT) && !die.HasProperty(property::WARRIOR)
        })
        .count();

    for signs in 0..(1usize << subtractable) {
        let mut minimum = 0i32;
        let mut maximum = 0i32;
        let mut sign_bit = 0usize;
        for position in stack.values() {
            let (die_index, die) = available[*position];
            let value = i32::from(die.GetValueTotal()) + i32::from(increases[die_index]);
            let is_konstant = die.HasProperty(property::KONSTANT);
            let variable_stinger =
                die.HasProperty(property::STINGER) && !die.HasProperty(property::WARRIOR);
            let term_minimum = if variable_stinger {
                DieCount(die)
            } else {
                value
            };

            if is_konstant {
                let may_subtract = !die.HasProperty(property::WARRIOR);
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
fn TripRollMax(die: &Die, smallest_mood_size: bool) -> u16 {
    if die.HasProperty(property::MOOD)
        && let Some(swing) = die.m_swing_type[0]
    {
        let (minimum, maximum) = super::SwingRange(swing);
        return u16::from(if smallest_mood_size { minimum } else { maximum });
    }
    let dice = DieCount(die) as usize;
    die.m_sides[..dice]
        .iter()
        .map(|sides| {
            let mut sides = *sides;
            if die.HasProperty(property::WEAK) {
                sides = super::mechanics::WeakSides(sides);
            }
            if die.HasProperty(property::MIGHTY) {
                sides = super::mechanics::MightySides(sides);
            }
            u16::from(sides)
        })
        .sum()
}

/// Mirrors ButtonWeavers `BMAttackTrip::validate_attack`.
fn TripCanCapture(attacker: &Die, target: &Die) -> bool {
    let target_minimum = DieCount(target) as u16;
    let attacker_maximum = if attacker.HasProperty(property::KONSTANT) {
        attacker.GetValueTotal()
    } else {
        TripRollMax(attacker, false)
    };
    if target.HasProperty(property::KONSTANT) && attacker_maximum < target.GetValueTotal() {
        return false;
    }
    if target.HasProperty(property::MAXIMUM) && attacker_maximum < TripRollMax(target, true) {
        return false;
    }
    attacker_maximum >= target_minimum
}

fn TurboSizes(die: &Die, accuracy: f32) -> Vec<(i16, Die)> {
    if !die.HasProperty(property::TURBO) {
        return vec![(-1, *die)];
    }
    if die.HasProperty(property::OPTION) {
        let mut swapped = *die;
        swapped.m_sides.swap(0, 1);
        return vec![(0, *die), (1, swapped)];
    }
    let Some(swing) = die.m_swing_type[0] else {
        return vec![(-1, *die)];
    };
    let (minimum, maximum) = turbo_swing_range(swing);
    let mut choices = vec![die.m_sides[0], minimum, maximum];
    let step = TurboStep(accuracy);
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
        let mut resized = *die;
        resized.m_sides[0] = sides;
        sizes.push((i16::from(sides), resized));
    }
    sizes
}

/// Only sizes `ExpandTurboMoves` can submit count, so every Trip keeps one.
fn TripReachableAtSomeTurboSize(
    attacker: &Die,
    target: &Die,
    expandable_turbo: bool,
    accuracy: f32,
) -> bool {
    if !expandable_turbo {
        return TripCanCapture(attacker, target);
    }
    TurboSizes(attacker, accuracy)
        .iter()
        .any(|(_, resized)| TripCanCapture(resized, target))
}

fn FireHelperCapacities(player: &Player, attackers: DieIndexSet) -> Vec<(usize, u8)> {
    player
        .m_die
        .iter()
        .enumerate()
        .filter(|(index, die)| {
            !attackers.contains(*index) && die.IsAvailable() && die.HasProperty(property::FIRE)
        })
        .filter_map(|(index, die)| {
            let minimum = DieCount(die) as u16;
            let capacity = die.GetValueTotal().saturating_sub(minimum);
            (capacity > 0).then_some((index, capacity as u8))
        })
        .collect()
}

fn AttackerFireCapacities(player: &Player, attackers: DieIndexSet) -> Vec<(usize, u8)> {
    attackers
        .iter()
        .filter_map(|index| {
            let die = &player.m_die[index];
            let capacity = die
                .GetSidesMax()
                .min(u16::from(u8::MAX))
                .saturating_sub(die.GetValueTotal());
            (capacity > 0).then_some((index, capacity as u8))
        })
        .collect()
}

fn FireAllocations(entries: &[(usize, u8)], total: u16, limit: usize) -> Vec<[u8; MAX_DICE]> {
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

fn FirePlansForPower(
    player: &Player,
    attacker: usize,
    minimum: u16,
    limit: usize,
) -> Vec<FireAdjustment> {
    let attackers = DieIndexSet::from([attacker]);
    let die = &player.m_die[attacker];
    let attacker_capacity = die
        .GetSidesMax()
        .min(u16::from(u8::MAX))
        .saturating_sub(die.GetValueTotal());
    let helper_capacities = FireHelperCapacities(player, attackers);
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
            FireAllocations(&helper_capacities, amount, remaining)
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

fn FirePlansForSkill(
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
    let attacker_capacities = AttackerFireCapacities(player, attackers);
    let helper_capacities = FireHelperCapacities(player, attackers);
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
        let reductions = FireAllocations(&helper_capacities, amount, limit - plans.len());
        if reductions.is_empty() {
            continue;
        }
        for increases in FireAllocations(&attacker_capacities, amount, limit - plans.len()) {
            if !SkillStackCanHitWithFire(stack, available, target, &increases) {
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
    fn GenerateValidAttackCandidatesInCppOrder(&self, fire_limit: usize) -> Vec<Move> {
        let attacker = &self.m_player[0];
        let target = &self.m_player[1];
        let available = AvailableDiceList::new(attacker);
        let targets = AvailableDiceList::new(target);
        let target_max = targets.first().map_or(0, |(_, die)| die.GetValueTotal());
        let target_min = targets.last().map_or(0, |(_, die)| die.GetValueTotal());
        let player_has_variable_skill_value = available.iter().any(|(_, die)| {
            !die.HasProperty(property::WARRIOR)
                && die.HasProperty(property::STINGER | property::KONSTANT)
        });
        let first_turbo = FirstTurboDie(attacker).map(|(index, _)| index);
        let skill_attacks_allowed = attacker.m_specials & special::NO_SKILL_ATTACKS == 0
            && target.m_specials & special::SKILL_IMMUNE == 0;
        let player_has_fire = available.iter().any(|(_, die)| {
            die.HasProperty(property::FIRE) && die.GetValueTotal() > DieCount(die) as u16
        });
        let targets_have_rush = targets
            .iter()
            .any(|(_, die)| die.HasProperty(property::RUSH));
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
                        if !attacker_die.CanDoAttack(attack, 1) {
                            continue;
                        }
                        for (target_index, target_die) in targets.iter().rev() {
                            if !target_die.CanBeAttacked(attack, 1) {
                                continue;
                            }
                            let legal = match attack {
                                Attack::Power => {
                                    attacker_die.GetValueTotal() >= target_die.GetValueTotal()
                                }
                                Attack::Shadow => {
                                    attacker_die.GetValueTotal() <= target_die.GetValueTotal()
                                        && attacker_die.GetSidesMax() >= target_die.GetValueTotal()
                                }
                                Attack::Trip => TripReachableAtSomeTurboSize(
                                    attacker_die,
                                    target_die,
                                    first_turbo == Some(attacker_index),
                                    self.m_turbo_accuracy,
                                ),
                                _ => unreachable!(),
                            };
                            if legal {
                                let score = match attack {
                                    Attack::Power => {
                                        target_die.GetScore(false)
                                            - if attacker_die.HasProperty(property::VALUE) {
                                                attacker_die.GetValueTotal() as f32 * 0.02
                                            } else {
                                                0.0
                                            }
                                    }
                                    Attack::Trip => target_die.GetScore(false) * 0.2,
                                    Attack::Shadow => target_die.GetScore(false),
                                    _ => unreachable!(),
                                };
                                moves.push(Move::attack(
                                    attack,
                                    [attacker_index],
                                    [*target_index],
                                    score,
                                ));
                            }
                            if player_has_fire && attack == Attack::Power && !legal {
                                let minimum = target_die
                                    .GetValueTotal()
                                    .saturating_sub(attacker_die.GetValueTotal())
                                    .max(1);
                                for fire in FirePlansForPower(
                                    attacker,
                                    attacker_index,
                                    minimum,
                                    fire_remaining,
                                ) {
                                    let mut candidate = Move::attack(
                                        attack,
                                        [attacker_index],
                                        [*target_index],
                                        target_die.GetScore(false),
                                    );
                                    candidate.m_fire = fire;
                                    moves.push(candidate);
                                    fire_remaining -= 1;
                                }
                            } else if player_has_fire
                                && attack == Attack::Power
                                && self.m_fire_overshooting
                            {
                                for fire in FirePlansForPower(
                                    attacker,
                                    attacker_index,
                                    1,
                                    optional_fire_remaining,
                                ) {
                                    let mut candidate = Move::attack(
                                        attack,
                                        [attacker_index],
                                        [*target_index],
                                        target_die.GetScore(false),
                                    );
                                    candidate.m_fire = fire;
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
                                available[*position].1.CanDoAttack(Attack::Skill, stack_len)
                            });
                            let warriors = stack
                                .values()
                                .iter()
                                .filter(|position| {
                                    available[**position].1.HasProperty(property::WARRIOR)
                                })
                                .count();
                            let single_konstant = stack_len == 1
                                && available[stack.values()[0]]
                                    .1
                                    .HasProperty(property::KONSTANT);
                            // ButtonWeavers requires a non-Warrior participant.
                            if dice_legal
                                && warriors <= 1
                                && warriors < stack_len
                                && !single_konstant
                            {
                                let stack_has_stinger = stack.values().iter().any(|position| {
                                    let die = available[*position].1;
                                    !die.HasProperty(property::WARRIOR)
                                        && die.HasProperty(property::STINGER)
                                });
                                let stack_has_konstant = stack.values().iter().any(|position| {
                                    let die = available[*position].1;
                                    !die.HasProperty(property::WARRIOR)
                                        && die.HasProperty(property::KONSTANT)
                                });
                                let minimum = stack
                                    .values()
                                    .iter()
                                    .map(|position| {
                                        let die = available[*position].1;
                                        if die.HasProperty(property::STINGER) {
                                            1
                                        } else {
                                            die.GetValueTotal()
                                        }
                                    })
                                    .sum::<u16>();
                                let flexible_stinger =
                                    stack_has_stinger && !stack_has_konstant && stack_len > 1;
                                for (target_index, target_die) in targets.iter() {
                                    if flexible_stinger && target_die.GetValueTotal() < minimum {
                                        break;
                                    }
                                    if !stack_has_konstant
                                        && !flexible_stinger
                                        && target_die.GetValueTotal() < stack.value_total
                                    {
                                        break;
                                    }
                                    let candidate_value = if stack_has_konstant {
                                        true
                                    } else if flexible_stinger {
                                        target_die.GetValueTotal() <= stack.value_total
                                    } else {
                                        target_die.GetValueTotal() == stack.value_total
                                    };
                                    let direct = candidate_value
                                        && SkillStackCanHit(
                                            &stack,
                                            &available,
                                            target_die.GetValueTotal(),
                                        );
                                    if direct && target_die.CanBeAttacked(Attack::Skill, stack_len)
                                    {
                                        moves.push(Move::attack(
                                            attack,
                                            stack
                                                .values()
                                                .iter()
                                                .map(|position| available[*position].0)
                                                .collect::<DieIndexSet>(),
                                            [*target_index],
                                            target_die.GetScore(false),
                                        ));
                                    }
                                    if player_has_fire
                                        && !direct
                                        && target_die.CanBeAttacked(Attack::Skill, stack_len)
                                    {
                                        let attacker_indices = stack
                                            .values()
                                            .iter()
                                            .map(|position| available[*position].0)
                                            .collect::<DieIndexSet>();
                                        for fire in FirePlansForSkill(
                                            attacker,
                                            &available,
                                            &stack,
                                            target_die.GetValueTotal(),
                                            fire_remaining,
                                        ) {
                                            let mut candidate = Move::attack(
                                                attack,
                                                attacker_indices,
                                                [*target_index],
                                                target_die.GetScore(false),
                                            );
                                            candidate.m_fire = fire;
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
                        if !attacker_die.CanDoAttack(attack, 1) {
                            continue;
                        }
                        for (target_index, target_die) in targets.iter() {
                            if target_die.CanBeAttacked(attack, 1) {
                                moves.push(Move::attack(
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
                        let attacker_has_rush = attacker_die.HasProperty(property::RUSH);
                        if !attacker_has_rush && !targets_have_rush
                            || !attacker_die.CanDoAttack(attack, 1)
                            || attacker_die.CanDoAttack(Attack::Speed, 1)
                        {
                            continue;
                        }
                        for first in 0..targets.len() {
                            let (first_index, first_die) = targets[first];
                            if !first_die.CanBeAttacked(attack, 1) {
                                continue;
                            }
                            for &(second_index, second_die) in targets.iter().skip(first + 1) {
                                if first_die.GetValueTotal() + second_die.GetValueTotal()
                                    != attacker_die.GetValueTotal()
                                    || !second_die.CanBeAttacked(attack, 1)
                                {
                                    continue;
                                }
                                if !attacker_has_rush
                                    && !first_die.HasProperty(property::RUSH)
                                    && !second_die.HasProperty(property::RUSH)
                                {
                                    continue;
                                }
                                moves.push(Move::attack(
                                    attack,
                                    [attacker_index],
                                    [first_index, second_index],
                                    first_die.GetScore(false) + second_die.GetScore(false),
                                ));
                            }
                        }
                    }
                    Attack::Berserk | Attack::Speed => {
                        if !attacker_die.CanDoAttack(attack, 1) || targets.is_empty() {
                            continue;
                        }
                        let mut stack = DieIndexStack::new();
                        stack.push(0, &targets);
                        loop {
                            if attacker_die.GetValueTotal() == stack.value_total
                                && stack
                                    .values()
                                    .iter()
                                    .all(|position| targets[*position].1.CanBeAttacked(attack, 1))
                            {
                                let target_indices = stack
                                    .values()
                                    .iter()
                                    .map(|position| targets[*position].0)
                                    .collect::<DieIndexSet>();
                                let score = stack
                                    .values()
                                    .iter()
                                    .map(|position| targets[*position].1.GetScore(false))
                                    .sum();
                                moves.push(Move::attack(
                                    attack,
                                    [attacker_index],
                                    target_indices,
                                    score,
                                ));
                            }
                            if stack.len == targets.len()
                                && attacker_die.GetValueTotal() >= stack.value_total
                            {
                                break;
                            }
                            let finished = if attacker_die.GetValueTotal() <= stack.value_total {
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

    pub fn GenerateValidAttacks(&self) -> Vec<Move> {
        // QAI and protocol users depend on this API's score order.
        self.GenerateValidAttacksForSearch(usize::MAX)
    }

    pub fn GenerateValidAttacksInCppOrder(&self) -> Vec<Move> {
        let mut moves = self.GenerateValidAttackCandidatesInCppOrder(usize::MAX);
        ExpandTurboMoves(self, &mut moves);
        moves
    }

    fn GenerateValidAttacksForSearch(&self, fire_limit: usize) -> Vec<Move> {
        let mut moves = self.GenerateValidAttackCandidatesInCppOrder(fire_limit);
        moves.sort_by(|a, b| {
            b.m_score
                .total_cmp(&a.m_score)
                .then_with(|| attack_preference(a.m_attack).cmp(&attack_preference(b.m_attack)))
        });
        ExpandTurboMoves(self, &mut moves);
        moves
    }

    pub(crate) fn GenerateValidAttacksInCppOrderForSearch(&self, fire_limit: usize) -> Vec<Move> {
        let mut moves = self.GenerateValidAttackCandidatesInCppOrder(fire_limit);
        ExpandTurboMoves(self, &mut moves);
        moves
    }

    pub fn GetAttackAction(&self) -> Move {
        let moves = self.GenerateValidAttacksForSearch(DEFAULT_FIRE_CANDIDATE_LIMIT);
        if self.m_surrender_allowed && self.m_player[1].m_score - self.m_player[0].m_score >= 20.0 {
            return Move {
                m_action: Action::Surrender,
                m_attack: None,
                m_attackers: Vec::new().into(),
                m_targets: Vec::new().into(),
                m_score: 0.0,
                m_turbo_option: -1,
                m_fire: FireAdjustment::default(),
            };
        }
        if let Some(best) = moves.first() {
            return best.clone();
        }
        Move {
            m_action: if self.m_surrender_allowed {
                Action::Surrender
            } else {
                Action::Pass
            },
            m_attack: None,
            m_attackers: Vec::new().into(),
            m_targets: Vec::new().into(),
            m_score: 0.0,
            m_turbo_option: -1,
            m_fire: FireAdjustment::default(),
        }
    }

    pub fn GetAttackActionDeep(&self) -> Move {
        let moves = self.GenerateValidAttacksForSearch(DEFAULT_FIRE_CANDIDATE_LIMIT);
        moves
            .into_iter()
            .filter(|candidate| candidate.m_action == Action::Attack)
            .min_by(|a, b| {
                let a_target = a
                    .m_targets
                    .iter()
                    .map(|index| self.m_player[1].m_die[index].GetScore(false))
                    .sum::<f32>();
                let b_target = b
                    .m_targets
                    .iter()
                    .map(|index| self.m_player[1].m_die[index].GetScore(false))
                    .sum::<f32>();
                let a_attacker = a.m_attackers.first().unwrap_or(0);
                let b_attacker = b.m_attackers.first().unwrap_or(0);
                a_target
                    .total_cmp(&b_target)
                    .then_with(|| {
                        self.m_player[0].m_die[b_attacker]
                            .GetSidesMax()
                            .cmp(&self.m_player[0].m_die[a_attacker].GetSidesMax())
                    })
                    .then_with(|| b_attacker.cmp(&a_attacker))
            })
            .unwrap_or_else(|| self.GetAttackAction())
    }
}

fn FirstTurboDie(player: &Player) -> Option<(usize, &Die)> {
    player
        .m_die
        .iter()
        .enumerate()
        .find(|(_, die)| die.IsAvailable() && die.HasProperty(property::TURBO))
}

fn MoveInvolvesDie(action: &Move, die: usize) -> bool {
    match action.m_attack {
        Some(Attack::Power | Attack::Shadow | Attack::Trip)
        | Some(Attack::Berserk | Attack::Speed | Attack::Rush) => {
            action.m_attackers.first() == Some(die)
        }
        Some(Attack::Skill) => action.m_attackers.contains(die),
        // The Boom die leaves play without rolling, so Turbo never applies.
        Some(Attack::Boom) | None => false,
    }
}

fn ExpandTurboMoves(game: &Game, moves: &mut Vec<Move>) {
    let player = &game.m_player[0];
    let accuracy = game.m_turbo_accuracy;
    let Some((turbo_index, turbo_die)) = FirstTurboDie(player) else {
        return;
    };
    let original_move_count = moves.len();
    for move_index in 0..original_move_count {
        if !MoveInvolvesDie(&moves[move_index], turbo_index) {
            continue;
        }
        // Decay strips Turbo before the attack reroll, so sizes only matter for
        // Trip, which ButtonWeavers rolls at the chosen size before decaying.
        if moves[move_index].m_attack != Some(Attack::Trip)
            && super::mechanics::RadioactiveDecayApplies(game, &moves[move_index], 0, 1)
        {
            continue;
        }
        if moves[move_index].m_attack == Some(Attack::Trip) {
            let target = moves[move_index].m_targets.first().expect("Trip target");
            let target_die = &game.m_player[1].m_die[target];
            let legal = TurboSizes(turbo_die, accuracy)
                .into_iter()
                .filter(|(_, resized)| TripCanCapture(resized, target_die))
                .map(|(size, _)| size)
                .collect::<Vec<_>>();
            let (first, rest) = legal
                .split_first()
                .expect("generated Trips have a legal Turbo size");
            moves[move_index].m_turbo_option = *first;
            for size in rest {
                let mut changed = moves[move_index].clone();
                changed.m_turbo_option = *size;
                moves.push(changed);
            }
            continue;
        }
        if turbo_die.HasProperty(property::OPTION) {
            moves[move_index].m_turbo_option = 0;
            // The Fire plan assumed the current size and may not fit another.
            if !moves[move_index].m_fire.is_empty() {
                continue;
            }
            let mut changed = moves[move_index].clone();
            changed.m_turbo_option = 1;
            moves.push(changed);
        } else if let Some(swing) = turbo_die.m_swing_type[0] {
            let current = i16::from(turbo_die.m_sides[0]);
            moves[move_index].m_turbo_option = current;
            if !moves[move_index].m_fire.is_empty() {
                continue;
            }
            // Unlike `TurboSizes`, this keeps C++'s duplicate sizes at
            // accuracies above 1 so non-Trip candidate order is unchanged.
            let (minimum, maximum) = turbo_swing_range(swing);
            let mut choices = vec![minimum, maximum];
            let step = TurboStep(accuracy);
            let mut candidate = f32::from(minimum + 1);
            while candidate < f32::from(maximum) {
                choices.push(candidate as u8);
                candidate += step;
            }
            for sides in choices {
                let sides = i16::from(sides);
                if sides == current {
                    continue;
                }
                let mut changed = moves[move_index].clone();
                changed.m_turbo_option = sides;
                moves.push(changed);
            }
        }
    }
}

/// An infinite accuracy would make the step zero and never advance.
fn TurboStep(accuracy: f32) -> f32 {
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
