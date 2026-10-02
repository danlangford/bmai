// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

const BMD_DEFAULT_FIRE_CANDIDATE_LIMIT: usize = 500;

struct BMC_AvailableDice<'a> {
    dice: [Option<(usize, &'a BMC_Die)>; BMD_MAX_DICE],
    len: usize,
}

impl<'a> BMC_AvailableDice<'a> {
    fn new(player: &'a BMC_Player) -> Self {
        let mut available = Self {
            dice: [None; BMD_MAX_DICE],
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

    fn iter(&self) -> impl DoubleEndedIterator<Item = &(usize, &'a BMC_Die)> {
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

    fn first(&self) -> Option<&(usize, &'a BMC_Die)> {
        self.dice[..self.len].first().and_then(Option::as_ref)
    }

    fn last(&self) -> Option<&(usize, &'a BMC_Die)> {
        self.dice[..self.len].last().and_then(Option::as_ref)
    }
}

impl<'a> std::ops::Index<usize> for BMC_AvailableDice<'a> {
    type Output = (usize, &'a BMC_Die);

    fn index(&self, index: usize) -> &Self::Output {
        self.dice[index]
            .as_ref()
            .expect("initialized available die")
    }
}

#[derive(Clone, Copy)]
struct BMC_DieIndexStack {
    indices: [usize; BMD_MAX_DICE],
    len: usize,
    value_total: u16,
}

impl BMC_DieIndexStack {
    fn new() -> Self {
        Self {
            indices: [0; BMD_MAX_DICE],
            len: 0,
            value_total: 0,
        }
    }

    fn values(&self) -> &[usize] {
        &self.indices[..self.len]
    }

    fn push(&mut self, index: usize, dice: &BMC_AvailableDice<'_>) {
        self.indices[self.len] = index;
        self.len += 1;
        self.value_total += dice[index].1.GetValueTotal();
    }

    fn pop(&mut self, dice: &BMC_AvailableDice<'_>) {
        self.value_total -= dice[self.indices[self.len - 1]].1.GetValueTotal();
        self.len -= 1;
    }

    /// Direct port of `BMC_DieIndexStack::Cycle` over positions in the
    /// optimized available-dice sequence.
    fn cycle(&mut self, mut add_die: bool, dice: &BMC_AvailableDice<'_>) -> bool {
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

fn DieCount(die: &BMC_Die) -> i32 {
    if die.HasProperty(property::TWIN) {
        2
    } else {
        1
    }
}

/// Port of PR #82's signed-Konstant `BMC_Game::ValidAttack` calculation.
/// For one sign assignment, non-Warrior Stinger values form a continuous
/// interval. Konstant dice may contribute either sign unless they are Warrior.
fn SkillStackCanHit(
    stack: &BMC_DieIndexStack,
    available: &BMC_AvailableDice<'_>,
    target: u16,
) -> bool {
    SkillStackCanHitWithFire(stack, available, target, &[0; BMD_MAX_DICE])
}

fn SkillStackCanHitWithFire(
    stack: &BMC_DieIndexStack,
    available: &BMC_AvailableDice<'_>,
    target: u16,
    increases: &[u8; BMD_MAX_DICE],
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

fn FireHelperCapacities(player: &BMC_Player, attackers: BMC_DieIndexSet) -> Vec<(usize, u8)> {
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

fn AttackerFireCapacities(player: &BMC_Player, attackers: BMC_DieIndexSet) -> Vec<(usize, u8)> {
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

fn FireAllocations(entries: &[(usize, u8)], total: u16, limit: usize) -> Vec<[u8; BMD_MAX_DICE]> {
    fn visit(
        entries: &[(usize, u8)],
        position: usize,
        remaining: u16,
        allocation: &mut [u8; BMD_MAX_DICE],
        results: &mut Vec<[u8; BMD_MAX_DICE]>,
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
    visit(
        entries,
        0,
        total,
        &mut [0; BMD_MAX_DICE],
        &mut results,
        limit,
    );
    results
}

fn FirePlansForPower(
    player: &BMC_Player,
    attacker: usize,
    minimum: u16,
    limit: usize,
) -> Vec<BMC_FireAdjustment> {
    let attackers = BMC_DieIndexSet::from([attacker]);
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
                    let mut increases = [0; BMD_MAX_DICE];
                    increases[attacker] = amount as u8;
                    BMC_FireAdjustment::from_allocations(increases, reductions)
                }),
        );
        if plans.len() == limit {
            break;
        }
    }
    plans
}

fn FirePlansForSkill(
    player: &BMC_Player,
    available: &BMC_AvailableDice<'_>,
    stack: &BMC_DieIndexStack,
    target: u16,
    limit: usize,
) -> Vec<BMC_FireAdjustment> {
    let attackers = stack
        .values()
        .iter()
        .map(|position| available[*position].0)
        .collect::<BMC_DieIndexSet>();
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
                    .map(|reduction| BMC_FireAdjustment::from_allocations(increases, *reduction)),
            );
            if plans.len() == limit {
                return plans;
            }
        }
    }
    plans
}

impl BMC_Game {
    fn GenerateValidAttackCandidatesInCppOrder(&self, fire_limit: usize) -> Vec<BMC_Move> {
        let attacker = &self.m_player[0];
        let target = &self.m_player[1];
        let available = BMC_AvailableDice::new(attacker);
        let targets = BMC_AvailableDice::new(target);
        let target_max = targets.first().map_or(0, |(_, die)| die.GetValueTotal());
        let target_min = targets.last().map_or(0, |(_, die)| die.GetValueTotal());
        let player_has_variable_skill_value = available.iter().any(|(_, die)| {
            !die.HasProperty(property::WARRIOR)
                && die.HasProperty(property::STINGER | property::KONSTANT)
        });
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
                BME_ATTACK::POWER,
                BME_ATTACK::SKILL,
                BME_ATTACK::BERSERK,
                BME_ATTACK::SPEED,
                BME_ATTACK::TRIP,
                BME_ATTACK::SHADOW,
                BME_ATTACK::RUSH,
            ] {
                match attack {
                    BME_ATTACK::POWER | BME_ATTACK::TRIP | BME_ATTACK::SHADOW => {
                        if !attacker_die.CanDoAttack(attack, 1) {
                            continue;
                        }
                        for (target_index, target_die) in targets.iter().rev() {
                            if !target_die.CanBeAttacked(attack, 1) {
                                continue;
                            }
                            let legal = match attack {
                                BME_ATTACK::POWER => {
                                    attacker_die.GetValueTotal() >= target_die.GetValueTotal()
                                }
                                BME_ATTACK::SHADOW => {
                                    attacker_die.GetValueTotal() <= target_die.GetValueTotal()
                                        && attacker_die.GetSidesMax() >= target_die.GetValueTotal()
                                }
                                BME_ATTACK::TRIP => {
                                    attacker_die.HasProperty(property::TWIN)
                                        || !target_die.HasProperty(property::TWIN)
                                }
                                _ => unreachable!(),
                            };
                            if legal {
                                let score = match attack {
                                    BME_ATTACK::POWER => {
                                        target_die.GetScore(false)
                                            - if attacker_die.HasProperty(property::VALUE) {
                                                attacker_die.GetValueTotal() as f32 * 0.02
                                            } else {
                                                0.0
                                            }
                                    }
                                    BME_ATTACK::TRIP => target_die.GetScore(false) * 0.2,
                                    BME_ATTACK::SHADOW => target_die.GetScore(false),
                                    _ => unreachable!(),
                                };
                                moves.push(BMC_Move::attack(
                                    attack,
                                    [attacker_index],
                                    [*target_index],
                                    score,
                                ));
                            }
                            if player_has_fire && attack == BME_ATTACK::POWER && !legal {
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
                                    let mut candidate = BMC_Move::attack(
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
                                && attack == BME_ATTACK::POWER
                                && self.m_fire_overshooting
                            {
                                for fire in FirePlansForPower(
                                    attacker,
                                    attacker_index,
                                    1,
                                    optional_fire_remaining,
                                ) {
                                    let mut candidate = BMC_Move::attack(
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
                    BME_ATTACK::SKILL => {
                        let mut stack = BMC_DieIndexStack::new();
                        stack.push(attacker_position, &available);
                        loop {
                            let stack_len = stack.len;
                            let dice_legal = stack.values().iter().all(|position| {
                                available[*position]
                                    .1
                                    .CanDoAttack(BME_ATTACK::SKILL, stack_len)
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
                                    if direct
                                        && target_die.CanBeAttacked(BME_ATTACK::SKILL, stack_len)
                                    {
                                        moves.push(BMC_Move::attack(
                                            attack,
                                            stack
                                                .values()
                                                .iter()
                                                .map(|position| available[*position].0)
                                                .collect::<BMC_DieIndexSet>(),
                                            [*target_index],
                                            target_die.GetScore(false),
                                        ));
                                    }
                                    if player_has_fire
                                        && !direct
                                        && target_die.CanBeAttacked(BME_ATTACK::SKILL, stack_len)
                                    {
                                        let attacker_indices = stack
                                            .values()
                                            .iter()
                                            .map(|position| available[*position].0)
                                            .collect::<BMC_DieIndexSet>();
                                        for fire in FirePlansForSkill(
                                            attacker,
                                            &available,
                                            &stack,
                                            target_die.GetValueTotal(),
                                            fire_remaining,
                                        ) {
                                            let mut candidate = BMC_Move::attack(
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
                    BME_ATTACK::RUSH => {
                        // A Speed die's two-target Speed attack resolves identically.
                        let attacker_has_rush = attacker_die.HasProperty(property::RUSH);
                        if !attacker_has_rush && !targets_have_rush
                            || !attacker_die.CanDoAttack(attack, 1)
                            || attacker_die.CanDoAttack(BME_ATTACK::SPEED, 1)
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
                                moves.push(BMC_Move::attack(
                                    attack,
                                    [attacker_index],
                                    [first_index, second_index],
                                    first_die.GetScore(false) + second_die.GetScore(false),
                                ));
                            }
                        }
                    }
                    BME_ATTACK::BERSERK | BME_ATTACK::SPEED => {
                        if !attacker_die.CanDoAttack(attack, 1) || targets.is_empty() {
                            continue;
                        }
                        let mut stack = BMC_DieIndexStack::new();
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
                                    .collect::<BMC_DieIndexSet>();
                                let score = stack
                                    .values()
                                    .iter()
                                    .map(|position| targets[*position].1.GetScore(false))
                                    .sum();
                                moves.push(BMC_Move::attack(
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

    pub fn GenerateValidAttacks(&self) -> Vec<BMC_Move> {
        // Use the direct C++ enumeration for the complete candidate set, then
        // retain this API's historical score ordering for QAI/protocol users.
        self.GenerateValidAttacksForSearch(usize::MAX)
    }

    pub fn GenerateValidAttacksInCppOrder(&self) -> Vec<BMC_Move> {
        let mut moves = self.GenerateValidAttackCandidatesInCppOrder(usize::MAX);
        ExpandTurboMoves(self, &mut moves);
        moves
    }

    fn GenerateValidAttacksForSearch(&self, fire_limit: usize) -> Vec<BMC_Move> {
        let mut moves = self.GenerateValidAttackCandidatesInCppOrder(fire_limit);
        moves.sort_by(|a, b| {
            b.m_score
                .total_cmp(&a.m_score)
                .then_with(|| attack_preference(a.m_attack).cmp(&attack_preference(b.m_attack)))
        });
        ExpandTurboMoves(self, &mut moves);
        moves
    }

    pub(crate) fn GenerateValidAttacksInCppOrderForSearch(
        &self,
        fire_limit: usize,
    ) -> Vec<BMC_Move> {
        let mut moves = self.GenerateValidAttackCandidatesInCppOrder(fire_limit);
        ExpandTurboMoves(self, &mut moves);
        moves
    }

    pub fn GetAttackAction(&self) -> BMC_Move {
        let moves = self.GenerateValidAttacksForSearch(BMD_DEFAULT_FIRE_CANDIDATE_LIMIT);
        if self.m_surrender_allowed && self.m_player[1].m_score - self.m_player[0].m_score >= 20.0 {
            return BMC_Move {
                m_action: BME_ACTION::SURRENDER,
                m_attack: None,
                m_attackers: Vec::new().into(),
                m_targets: Vec::new().into(),
                m_score: 0.0,
                m_turbo_option: -1,
                m_fire: BMC_FireAdjustment::default(),
            };
        }
        if let Some(best) = moves.first() {
            return best.clone();
        }
        BMC_Move {
            m_action: if self.m_surrender_allowed {
                BME_ACTION::SURRENDER
            } else {
                BME_ACTION::PASS
            },
            m_attack: None,
            m_attackers: Vec::new().into(),
            m_targets: Vec::new().into(),
            m_score: 0.0,
            m_turbo_option: -1,
            m_fire: BMC_FireAdjustment::default(),
        }
    }

    pub fn GetAttackActionDeep(&self) -> BMC_Move {
        let moves = self.GenerateValidAttacksForSearch(BMD_DEFAULT_FIRE_CANDIDATE_LIMIT);
        moves
            .into_iter()
            .filter(|candidate| candidate.m_action == BME_ACTION::ATTACK)
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

fn FirstTurboDie(player: &BMC_Player) -> Option<(usize, &BMC_Die)> {
    player
        .m_die
        .iter()
        .enumerate()
        .find(|(_, die)| die.IsAvailable() && die.HasProperty(property::TURBO))
}

fn MoveInvolvesDie(action: &BMC_Move, die: usize) -> bool {
    match action.m_attack {
        Some(BME_ATTACK::POWER | BME_ATTACK::SHADOW | BME_ATTACK::TRIP)
        | Some(BME_ATTACK::BERSERK | BME_ATTACK::SPEED | BME_ATTACK::RUSH) => {
            action.m_attackers.first() == Some(die)
        }
        Some(BME_ATTACK::SKILL) => action.m_attackers.contains(die),
        None => false,
    }
}

fn ExpandTurboMoves(game: &BMC_Game, moves: &mut Vec<BMC_Move>) {
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
        // Decay strips Turbo before the reroll, so every size resolves the same.
        if super::mechanics::RadioactiveDecayApplies(game, &moves[move_index], 0, 1) {
            continue;
        }
        if turbo_die.HasProperty(property::OPTION) {
            moves[move_index].m_turbo_option = 0;
            // Fire capacities were calculated for the current Turbo size.
            // Reusing that plan after changing size can exceed the new maximum.
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
            let (minimum, maximum) = turbo_swing_range(swing);
            let mut choices = vec![minimum, maximum];
            let step = if accuracy <= 0.0 {
                1000.0
            } else {
                1.0 / accuracy
            };
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

fn attack_preference(attack: Option<BME_ATTACK>) -> u8 {
    match attack {
        Some(BME_ATTACK::POWER) => 0,
        Some(BME_ATTACK::SKILL) => 1,
        Some(BME_ATTACK::BERSERK) => 2,
        Some(BME_ATTACK::SPEED) => 3,
        Some(BME_ATTACK::TRIP) => 4,
        Some(BME_ATTACK::SHADOW) => 5,
        Some(BME_ATTACK::RUSH) => 6,
        None => 7,
    }
}
