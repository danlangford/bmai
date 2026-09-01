// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

pub mod property {
    pub const TIME_AND_SPACE: u64 = 0x0001;
    pub const AUXILIARY: u64 = 0x0002;
    pub const QUEER: u64 = 0x0004;
    pub const TRIP: u64 = 0x0008;
    pub const SPEED: u64 = 0x0010;
    pub const SHADOW: u64 = 0x0020;
    pub const BERSERK: u64 = 0x0040;
    pub const STEALTH: u64 = 0x0080;
    pub const POISON: u64 = 0x0100;
    pub const NULL: u64 = 0x0200;
    pub const MOOD: u64 = 0x0400;
    pub const TURBO: u64 = 0x0800;
    pub const OPTION: u64 = 0x1000;
    pub const TWIN: u64 = 0x2000;
    pub const FOCUS: u64 = 0x4000;
    pub const VALID: u64 = 0x8000;
    pub const MIGHTY: u64 = 0x1_0000;
    pub const WEAK: u64 = 0x2_0000;
    pub const RESERVE: u64 = 0x4_0000;
    pub const ORNERY: u64 = 0x8_0000;
    pub const DOPPELGANGER: u64 = 0x10_0000;
    pub const CHANCE: u64 = 0x20_0000;
    pub const MORPHING: u64 = 0x40_0000;
    pub const RADIOACTIVE: u64 = 0x80_0000;
    pub const WARRIOR: u64 = 0x100_0000;
    pub const SLOW: u64 = 0x200_0000;
    pub const UNIQUE: u64 = 0x400_0000;
    pub const UNSKILLED: u64 = 0x800_0000;
    pub const STINGER: u64 = 0x1000_0000;
    pub const RAGE: u64 = 0x2000_0000;
    pub const KONSTANT: u64 = 0x4000_0000;
    pub const MAXIMUM: u64 = 0x8000_0000;
    pub const INSULT: u64 = 0x1_0000_0000;
    pub const VALUE: u64 = 0x2_0000_0000;
    pub const JOLT: u64 = 0x4_0000_0000;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BME_PHASE {
    PREROUND,
    RESERVE,
    INITIATIVE,
    CHANCE,
    FOCUS,
    FIGHT,
    GAMEOVER,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BME_ACTION {
    SET_SWING_AND_OPTION,
    USE_RESERVE,
    ATTACK,
    PASS,
    SURRENDER,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum BME_SWING_SET {
    #[default]
    NOT,
    READY,
    LOCKED,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BME_ATTACK {
    POWER,
    SKILL,
    BERSERK,
    SPEED,
    TRIP,
    SHADOW,
}

impl BME_ATTACK {
    pub fn protocol(self) -> &'static str {
        match self {
            Self::POWER => "power",
            Self::SKILL => "skill",
            Self::BERSERK => "berserk",
            Self::SPEED => "speed",
            Self::TRIP => "trip",
            Self::SHADOW => "shadow",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BMC_Die {
    pub m_properties: u64,
    pub m_sides: [u8; 2],
    pub m_swing_type: [Option<char>; 2],
    pub m_value_total: Option<u8>,
    pub m_captured: bool,
    pub m_notset: bool,
    pub m_dizzy: bool,
    pub m_original_index: usize,
    pub m_in_reserve: bool,
}

impl BMC_Die {
    pub fn HasProperty(&self, property: u64) -> bool {
        self.m_properties & property != 0
    }
    pub fn GetSidesMax(&self) -> u16 {
        if self.HasProperty(property::TWIN) {
            self.m_sides.iter().map(|v| u16::from(*v)).sum()
        } else {
            u16::from(self.m_sides[0])
        }
    }
    pub fn GetValueTotal(&self) -> u16 {
        u16::from(self.m_value_total.unwrap_or(0))
    }
    pub fn IsAvailable(&self) -> bool {
        self.m_value_total.is_some() && !self.m_captured && !self.m_notset && !self.m_in_reserve
    }

    pub fn GetScore(&self, own: bool) -> f32 {
        if self.HasProperty(property::NULL | property::WARRIOR) {
            return 0.0;
        }
        let poison = self.HasProperty(property::POISON);
        let value = self.HasProperty(property::VALUE);
        match (poison, value, own) {
            (true, true, true) => -(self.GetValueTotal() as f32),
            (true, true, false) => -(self.GetValueTotal() as f32) * 0.5,
            (true, false, true) => -(self.GetSidesMax() as f32),
            (true, false, false) => -(self.GetSidesMax() as f32) * 0.5,
            (false, true, true) => self.GetValueTotal() as f32 * 0.5,
            (false, true, false) => self.GetValueTotal() as f32,
            (false, false, true) => self.GetSidesMax() as f32 * 0.5,
            (false, false, false) => self.GetSidesMax() as f32,
        }
    }

    pub fn Roll(&mut self, rng: &mut crate::BMC_RNG) {
        crate::simulation::RollDie(self, rng);
    }

    pub fn OnSwingSet(&mut self, swing: char, value: u8) {
        assert!(self.m_notset, "BMC_Die::OnSwingSet requires NOTSET state");
        for side in 0..2 {
            if self.m_swing_type[side] == Some(swing) {
                self.m_sides[side] = value;
            }
        }
    }

    pub fn OnDizzyRecovered(&mut self) {
        self.m_dizzy = false;
    }

    fn CanDoAttack(&self, attack: BME_ATTACK, skill_dice: usize) -> bool {
        if !self.IsAvailable() || self.m_dizzy {
            return false;
        }
        if self.HasProperty(property::WARRIOR) {
            return attack == BME_ATTACK::SKILL;
        }
        if self.HasProperty(property::STEALTH) {
            return attack == BME_ATTACK::SKILL && skill_dice > 1;
        }
        match attack {
            BME_ATTACK::POWER => {
                !self.HasProperty(property::SHADOW | property::KONSTANT)
                    && !(self.HasProperty(property::QUEER) && self.GetValueTotal() % 2 == 1)
            }
            BME_ATTACK::SKILL => !self.HasProperty(property::UNSKILLED | property::BERSERK),
            BME_ATTACK::BERSERK => self.HasProperty(property::BERSERK),
            BME_ATTACK::SPEED => self.HasProperty(property::SPEED),
            BME_ATTACK::TRIP => self.HasProperty(property::TRIP),
            BME_ATTACK::SHADOW => {
                self.HasProperty(property::SHADOW)
                    || self.HasProperty(property::QUEER) && self.GetValueTotal() % 2 == 1
            }
        }
    }

    fn CanBeAttacked(&self, attack: BME_ATTACK, skill_dice: usize) -> bool {
        if self.HasProperty(property::WARRIOR) {
            return false;
        }
        // RecomputeAttacks applies INSULT before STEALTH. STEALTH clears all
        // vulnerabilities and then restores multi-die Skill, so it overrides
        // Insult when both properties are present.
        if self.HasProperty(property::STEALTH) {
            return attack == BME_ATTACK::SKILL && skill_dice > 1;
        }
        if self.HasProperty(property::INSULT) && attack == BME_ATTACK::SKILL {
            return false;
        }
        true
    }
}

#[derive(Clone, Debug, Default)]
pub struct BMC_Player {
    pub m_id: usize,
    pub m_score: f32,
    pub m_die: Vec<BMC_Die>,
    pub m_swing_set: BME_SWING_SET,
    /// Dynamic sides of original recipes replaced by Doppelganger this round.
    pub m_doppelganger_original_sides: [[u8; 2]; BMD_MAX_DICE],
    /// Bit set keyed by stable die index for each transformed Doppelganger.
    pub m_doppelganger_transformed: u16,
}

impl BMC_Player {
    pub fn OptimizeDice(&mut self) {
        // Preserve BMC_Player::OptimizeDice exactly. This is deliberately not
        // equivalent to a stable descending sort: an available die can swap
        // through several positions while the outer index is held, changing
        // the relative order of equal-valued dice after captures and rerolls.
        for i in 0..self.m_die.len() {
            for j in (i + 1)..self.m_die.len() {
                let swap = if !self.m_die[i].IsAvailable() && self.m_die[j].IsAvailable() {
                    true
                } else {
                    self.m_die[j].IsAvailable()
                        && self.m_die[i].GetValueTotal() < self.m_die[j].GetValueTotal()
                };
                if swap {
                    self.m_die.swap(i, j);
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct BMC_Move {
    pub m_action: BME_ACTION,
    pub m_attack: Option<BME_ATTACK>,
    pub m_attackers: BMC_DieIndexSet,
    pub m_targets: BMC_DieIndexSet,
    pub m_score: f32,
    /// C++ BMC_MoveAttack::m_turbo_option. -1 means no Turbo decision;
    /// option dice use 0/1 and swing dice store the selected side count.
    pub m_turbo_option: i16,
}

impl BMC_Move {
    pub(crate) fn attack(
        kind: BME_ATTACK,
        attackers: impl Into<BMC_DieIndexSet>,
        targets: impl Into<BMC_DieIndexSet>,
        score: f32,
    ) -> Self {
        Self {
            m_action: BME_ACTION::ATTACK,
            m_attack: Some(kind),
            m_attackers: attackers.into(),
            m_targets: targets.into(),
            m_score: score,
            m_turbo_option: -1,
        }
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub struct BMC_DieIndexSet(u16);

impl BMC_DieIndexSet {
    pub fn iter(self) -> impl Iterator<Item = usize> {
        (0..BMD_MAX_DICE).filter(move |index| self.0 & (1 << index) != 0)
    }

    pub fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn first(self) -> Option<usize> {
        (!self.is_empty()).then(|| self.0.trailing_zeros() as usize)
    }

    pub fn contains(self, index: usize) -> bool {
        index < BMD_MAX_DICE && self.0 & (1 << index) != 0
    }
}

impl std::fmt::Debug for BMC_DieIndexSet {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_list().entries(self.iter()).finish()
    }
}

impl From<Vec<usize>> for BMC_DieIndexSet {
    fn from(indices: Vec<usize>) -> Self {
        indices.as_slice().into()
    }
}

impl<const N: usize> From<[usize; N]> for BMC_DieIndexSet {
    fn from(indices: [usize; N]) -> Self {
        indices.as_slice().into()
    }
}

impl From<&[usize]> for BMC_DieIndexSet {
    fn from(indices: &[usize]) -> Self {
        let mut bits = 0u16;
        for index in indices {
            assert!(*index < BMD_MAX_DICE);
            bits |= 1 << index;
        }
        Self(bits)
    }
}

impl FromIterator<usize> for BMC_DieIndexSet {
    fn from_iter<T: IntoIterator<Item = usize>>(indices: T) -> Self {
        let mut bits = 0u16;
        for index in indices {
            assert!(index < BMD_MAX_DICE);
            bits |= 1 << index;
        }
        Self(bits)
    }
}

impl PartialEq<Vec<usize>> for BMC_DieIndexSet {
    fn eq(&self, other: &Vec<usize>) -> bool {
        self.iter().eq(other.iter().copied())
    }
}

#[derive(Clone, Debug)]
pub struct BMC_Game {
    pub m_player: [BMC_Player; 2],
    pub m_phase: BME_PHASE,
    pub m_surrender_allowed: bool,
    pub m_target_wins: u8,
    pub m_turbo_accuracy: f32,
}

pub(crate) const BMD_MAX_DICE: usize = 10;

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
            let die = available[*position].1;
            let value = i32::from(die.GetValueTotal());
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

impl Default for BMC_Game {
    fn default() -> Self {
        Self {
            m_player: [
                BMC_Player {
                    m_id: 0,
                    ..Default::default()
                },
                BMC_Player {
                    m_id: 1,
                    ..Default::default()
                },
            ],
            m_phase: BME_PHASE::PREROUND,
            m_surrender_allowed: true,
            m_target_wins: 3,
            m_turbo_accuracy: 1.0,
        }
    }
}

impl BMC_Game {
    pub fn SimulateAttack(&mut self, action: &BMC_Move, rng: &mut crate::BMC_RNG) -> bool {
        crate::simulation::ApplyAttack(self, action, rng)
    }

    pub fn CheckInitiative(&self) -> Option<usize> {
        crate::simulation::CheckInitiative(self)
    }

    pub fn RecoverDizzyDice(&mut self, player: usize) {
        crate::simulation::RecoverDizzyDice(&mut self.m_player[player]);
    }

    fn GenerateValidAttackCandidatesInCppOrder(&self) -> Vec<BMC_Move> {
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
        let mut moves = Vec::with_capacity(32);
        for attacker_position in 0..available.len() {
            let (attacker_index, attacker_die) = available[attacker_position];
            for attack in [
                BME_ATTACK::POWER,
                BME_ATTACK::SKILL,
                BME_ATTACK::BERSERK,
                BME_ATTACK::SPEED,
                BME_ATTACK::TRIP,
                BME_ATTACK::SHADOW,
            ] {
                match attack {
                    BME_ATTACK::POWER | BME_ATTACK::TRIP | BME_ATTACK::SHADOW => {
                        if !attacker_die.CanDoAttack(attack, 1) {
                            continue;
                        }
                        for (target_index, target_die) in targets.iter().rev() {
                            let legal = target_die.CanBeAttacked(attack, 1)
                                && match attack {
                                    BME_ATTACK::POWER => {
                                        attacker_die.GetValueTotal() >= target_die.GetValueTotal()
                                    }
                                    BME_ATTACK::SHADOW => {
                                        attacker_die.GetValueTotal() <= target_die.GetValueTotal()
                                            && attacker_die.GetSidesMax()
                                                >= target_die.GetValueTotal()
                                    }
                                    BME_ATTACK::TRIP => {
                                        attacker_die.HasProperty(property::TWIN)
                                            || !target_die.HasProperty(property::TWIN)
                                    }
                                    _ => unreachable!(),
                                };
                            if !legal {
                                continue;
                            }
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
                            if dice_legal && warriors <= 1 && !single_konstant {
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
                                    if candidate_value
                                        && SkillStackCanHit(
                                            &stack,
                                            &available,
                                            target_die.GetValueTotal(),
                                        )
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
        moves
    }

    pub fn GenerateValidAttacks(&self) -> Vec<BMC_Move> {
        // Use the direct C++ enumeration for the complete candidate set, then
        // retain this API's historical score ordering for QAI/protocol users.
        let mut moves = self.GenerateValidAttackCandidatesInCppOrder();
        moves.sort_by(|a, b| {
            b.m_score
                .total_cmp(&a.m_score)
                .then_with(|| attack_preference(a.m_attack).cmp(&attack_preference(b.m_attack)))
        });
        ExpandTurboMoves(&self.m_player[0], self.m_turbo_accuracy, &mut moves);
        moves
    }

    pub fn GenerateValidAttacksInCppOrder(&self) -> Vec<BMC_Move> {
        let mut moves = self.GenerateValidAttackCandidatesInCppOrder();
        ExpandTurboMoves(&self.m_player[0], self.m_turbo_accuracy, &mut moves);
        moves
    }

    pub fn GetAttackAction(&self) -> BMC_Move {
        let moves = self.GenerateValidAttacks();
        if self.m_surrender_allowed && self.m_player[1].m_score - self.m_player[0].m_score >= 20.0 {
            return BMC_Move {
                m_action: BME_ACTION::SURRENDER,
                m_attack: None,
                m_attackers: Vec::new().into(),
                m_targets: Vec::new().into(),
                m_score: 0.0,
                m_turbo_option: -1,
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
        }
    }

    pub fn GetAttackActionDeep(&self) -> BMC_Move {
        let moves = self.GenerateValidAttacks();
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
        | Some(BME_ATTACK::BERSERK | BME_ATTACK::SPEED) => action.m_attackers.first() == Some(die),
        Some(BME_ATTACK::SKILL) => action.m_attackers.contains(die),
        None => false,
    }
}

fn ExpandTurboMoves(player: &BMC_Player, accuracy: f32, moves: &mut Vec<BMC_Move>) {
    let Some((turbo_index, turbo_die)) = FirstTurboDie(player) else {
        return;
    };
    let original_move_count = moves.len();
    for move_index in 0..original_move_count {
        if !MoveInvolvesDie(&moves[move_index], turbo_index) {
            continue;
        }
        if turbo_die.HasProperty(property::OPTION) {
            moves[move_index].m_turbo_option = 0;
            let mut changed = moves[move_index].clone();
            changed.m_turbo_option = 1;
            moves.push(changed);
        } else if let Some(swing) = turbo_die.m_swing_type[0] {
            let current = i16::from(turbo_die.m_sides[0]);
            moves[move_index].m_turbo_option = current;
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
        None => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpp_die_index_set_is_bounded_and_iterates_like_bit_array() {
        let indices: BMC_DieIndexSet = [0, 3, 9].into();
        assert_eq!(indices.len(), 3);
        assert_eq!(indices.first(), Some(0));
        assert!(indices.contains(9));
        assert_eq!(indices.iter().collect::<Vec<_>>(), vec![0, 3, 9]);
    }

    fn die(properties: u64) -> BMC_Die {
        BMC_Die {
            m_properties: property::VALID | properties,
            m_sides: [20, 0],
            m_swing_type: [None, None],
            m_value_total: Some(8),
            m_captured: false,
            m_notset: false,
            m_dizzy: false,
            m_original_index: 0,
            m_in_reserve: false,
        }
    }

    fn game_with(attacker: Vec<BMC_Die>, target: Vec<BMC_Die>) -> BMC_Game {
        let mut game = BMC_Game::default();
        game.m_player[0].m_die = attacker;
        game.m_player[1].m_die = target;
        game
    }

    fn attacks(game: &BMC_Game, kind: BME_ATTACK) -> Vec<BMC_Move> {
        game.GenerateValidAttacks()
            .into_iter()
            .filter(|action| action.m_attack == Some(kind))
            .collect()
    }

    fn valued_die(value: i32, properties: u64, original_index: usize) -> BMC_Die {
        let mut result = die(properties);
        result.m_sides[0] = value.max(1) as u8;
        result.m_value_total = Some(value as u8);
        result.m_original_index = original_index;
        result
    }

    fn has_skill(game: &BMC_Game, attackers: &[usize], target: usize) -> bool {
        let expected: BMC_DieIndexSet = attackers.iter().copied().collect();
        game.GenerateValidAttacks().iter().any(|action| {
            action.m_attack == Some(BME_ATTACK::SKILL)
                && action.m_attackers == expected
                && action.m_targets.first() == Some(target)
        })
    }

    /// Ports PR #82's signed-Konstant and Stinger/Warrior skill matrix.
    #[test]
    fn pr82_signed_konstant_skill_attack_matrix() {
        const K: u64 = property::KONSTANT;
        const G: u64 = property::STINGER;
        const W: u64 = property::WARRIOR;
        const M: u64 = property::MAXIMUM;

        struct Case {
            name: &'static str,
            dice: &'static [(i32, u64)],
            target: i32,
            target_properties: u64,
            selected: &'static [usize],
            expected: bool,
        }

        let cases = [
            Case {
                name: "KonstantMultiDieSkillAttackWithSubtraction",
                dice: &[(1, M | K), (1, M | K), (3, M | K)],
                target: 3,
                target_properties: 0,
                selected: &[0, 1, 2],
                expected: true,
            },
            Case {
                name: "KonstantMixedMultiDieSkillAttackWithSubtraction",
                dice: &[(8, 0), (1, M | K)],
                target: 7,
                target_properties: property::STEALTH,
                selected: &[0, 1],
                expected: true,
            },
            Case {
                name: "KonstantMultiDieSkillAttackNoMatchingAssignment",
                dice: &[(1, M | K), (1, M | K), (3, M | K)],
                target: 6,
                target_properties: 0,
                selected: &[0, 1, 2],
                expected: false,
            },
            Case {
                name: "KonstantWarriorCannotSubtractInSkillAttack",
                dice: &[(3, W | K), (5, M | K)],
                target: 2,
                target_properties: property::STEALTH,
                selected: &[0, 1],
                expected: false,
            },
            Case {
                name: "OnlyOneWarriorMayParticipateInSkillAttack",
                dice: &[(5, 0), (2, W | K), (3, W | K)],
                target: 10,
                target_properties: property::STEALTH,
                selected: &[0, 1, 2],
                expected: false,
            },
            Case {
                name: "KonstantMultiDieSkillAttackWithoutSubtraction",
                dice: &[(1, M | K), (2, M | K)],
                target: 3,
                target_properties: property::STEALTH,
                selected: &[0, 1],
                expected: true,
            },
            Case {
                name: "KonstantSkillAttackWithUnusedStingerInPool",
                dice: &[(5, 0), (2, M | K), (3, M | K), (6, G)],
                target: 4,
                target_properties: 0,
                selected: &[0, 1, 2],
                expected: true,
            },
            Case {
                name: "StingerAndKonstantBothInAttack",
                dice: &[(6, G), (3, M | K)],
                target: 7,
                target_properties: 0,
                selected: &[0, 1],
                expected: true,
            },
            Case {
                name: "StingerAndKonstantWithSubtraction",
                dice: &[(8, G), (5, M | K)],
                target: 3,
                target_properties: 0,
                selected: &[0, 1],
                expected: true,
            },
            Case {
                name: "StingerSkillAttackInRange",
                dice: &[(4, 0), (6, G)],
                target: 7,
                target_properties: 0,
                selected: &[0, 1],
                expected: true,
            },
            Case {
                name: "StingerSkillAttackAtMinimumRange",
                dice: &[(4, 0), (6, G)],
                target: 5,
                target_properties: 0,
                selected: &[0, 1],
                expected: true,
            },
            Case {
                name: "StingerSkillAttackAtMaximumRange",
                dice: &[(4, 0), (6, G)],
                target: 10,
                target_properties: 0,
                selected: &[0, 1],
                expected: true,
            },
            Case {
                name: "StingerSkillAttackBelowRange",
                dice: &[(4, 0), (6, G)],
                target: 4,
                target_properties: 0,
                selected: &[0, 1],
                expected: false,
            },
            Case {
                name: "TwoStingersSkillAttackRange",
                dice: &[(10, G), (10, G)],
                target: 2,
                target_properties: 0,
                selected: &[0, 1],
                expected: true,
            },
            Case {
                name: "TwoStingersCannotHitBelowMinimum",
                dice: &[(10, G), (10, G)],
                target: 1,
                target_properties: 0,
                selected: &[0, 1],
                expected: false,
            },
            Case {
                name: "StingerAtValueOneHasNoFlexibility",
                dice: &[(4, 0), (1, G)],
                target: 5,
                target_properties: 0,
                selected: &[0, 1],
                expected: true,
            },
            Case {
                name: "StingerAtValueOneCannotHitLowerTarget",
                dice: &[(4, 0), (1, G)],
                target: 4,
                target_properties: 0,
                selected: &[0, 1],
                expected: false,
            },
            Case {
                name: "NormalStingerKonstantThreeDieAttack",
                dice: &[(4, 0), (6, G), (3, M | K)],
                target: 5,
                target_properties: 0,
                selected: &[0, 1, 2],
                expected: true,
            },
            Case {
                name: "KonstantWarriorCanAddInSkillAttack",
                dice: &[(5, 0), (3, W | K)],
                target: 8,
                target_properties: 0,
                selected: &[0, 1],
                expected: true,
            },
            Case {
                name: "StingerWarriorMustUseFullValue",
                dice: &[(4, 0), (6, W | G)],
                target: 7,
                target_properties: 0,
                selected: &[0, 1],
                expected: false,
            },
            Case {
                name: "StingerWarriorAtFullValueIsValid",
                dice: &[(4, 0), (6, W | G)],
                target: 10,
                target_properties: 0,
                selected: &[0, 1],
                expected: true,
            },
            Case {
                name: "StingerAndKonstantCombinedFlexibility",
                dice: &[(8, G), (5, M | K)],
                target: 2,
                target_properties: 0,
                selected: &[0, 1],
                expected: true,
            },
            Case {
                name: "StingerKonstantOnSameDieWithSubtraction",
                dice: &[(4, 0), (5, G | K)],
                target: 2,
                target_properties: property::STEALTH,
                selected: &[0, 1],
                expected: true,
            },
            Case {
                name: "StingerKonstantOnSameDieWithAddition",
                dice: &[(4, 0), (5, G | K)],
                target: 6,
                target_properties: property::STEALTH,
                selected: &[0, 1],
                expected: true,
            },
            Case {
                name: "StingerKonstantOnSameDieCannotHitGapBetweenSigns",
                dice: &[(4, 0), (5, G | K)],
                target: 4,
                target_properties: property::STEALTH,
                selected: &[0, 1],
                expected: false,
            },
            Case {
                name: "TwoStingerKonstantDiceCannotHitGapBetweenSignedValues",
                dice: &[(1, G | K), (1, G | K)],
                target: 1,
                target_properties: property::STEALTH,
                selected: &[0, 1],
                expected: false,
            },
            Case {
                name: "StingerWithKonstantWarriorUsesStingerFlexibility",
                dice: &[(6, G), (3, W | K)],
                target: 7,
                target_properties: property::STEALTH,
                selected: &[0, 1],
                expected: true,
            },
            Case {
                name: "StingerWarriorWithKonstantUsesKonstantSubtraction",
                dice: &[(6, W | G), (3, M | K)],
                target: 3,
                target_properties: property::STEALTH,
                selected: &[0, 1],
                expected: true,
            },
            Case {
                name: "StingerKonstantWarriorUsesFullPositiveValue",
                dice: &[(4, 0), (5, W | G | K)],
                target: 9,
                target_properties: property::STEALTH,
                selected: &[0, 1],
                expected: true,
            },
            Case {
                name: "StingerKonstantWarriorCannotUsePartialValue",
                dice: &[(4, 0), (5, W | G | K)],
                target: 6,
                target_properties: property::STEALTH,
                selected: &[0, 1],
                expected: false,
            },
            Case {
                name: "StingerKonstantWarriorCannotSubtract",
                dice: &[(4, 0), (5, W | G | K)],
                target: 2,
                target_properties: property::STEALTH,
                selected: &[0, 1],
                expected: false,
            },
        ];

        for case in cases {
            let attacker = case
                .dice
                .iter()
                .enumerate()
                .map(|(index, &(value, properties))| valued_die(value, properties, index))
                .collect();
            let game = game_with(
                attacker,
                vec![valued_die(case.target, case.target_properties, 0)],
            );
            assert_eq!(
                has_skill(&game, case.selected, 0),
                case.expected,
                "{}",
                case.name
            );
        }

        for target in [10, 4, 6, 0] {
            let game = game_with(
                vec![
                    valued_die(5, 0, 0),
                    valued_die(2, M | K, 1),
                    valued_die(3, M | K, 2),
                ],
                vec![valued_die(target, property::STEALTH, 0)],
            );
            assert!(has_skill(&game, &[0, 1, 2], 0), "signed target {target}");
        }

        let ten = (1..=10)
            .enumerate()
            .map(|(index, value)| valued_die(value, K, index))
            .collect();
        assert!(has_skill(
            &game_with(ten, vec![valued_die(53, 0, 0)]),
            &(0..10).collect::<Vec<_>>(),
            0
        ));

        let game = game_with(
            vec![valued_die(8, 0, 0), valued_die(1, M | K, 1)],
            vec![
                valued_die(8, property::STEALTH, 0),
                valued_die(7, property::STEALTH, 1),
            ],
        );
        assert!(!has_skill(&game, &[0, 1], 0));
        assert!(has_skill(&game, &[0, 1], 1));
    }

    /// Port of PlayerTests.CopyConstructor.
    #[test]
    fn cpp_player_copy_constructor_is_independent() {
        let first = BMC_Player {
            m_id: 1,
            ..Default::default()
        };
        let mut second = first.clone();
        assert_eq!(second.m_id, 1);
        second.m_id = 2;
        assert_eq!(second.m_id, 2);
        assert_eq!(first.m_id, 1);
    }

    #[test]
    fn score_matches_cpp_property_branches() {
        assert_eq!(die(0).GetScore(true), 10.0);
        assert_eq!(die(0).GetScore(false), 20.0);
        assert_eq!(die(property::POISON).GetScore(true), -20.0);
        assert_eq!(die(property::POISON).GetScore(false), -10.0);
        assert_eq!(die(property::VALUE).GetScore(true), 4.0);
        assert_eq!(die(property::VALUE).GetScore(false), 8.0);
        assert_eq!(die(property::POISON | property::VALUE).GetScore(true), -8.0);
        assert_eq!(
            die(property::POISON | property::VALUE).GetScore(false),
            -4.0
        );
        assert_eq!(die(property::NULL).GetScore(false), 0.0);
        assert_eq!(die(property::WARRIOR).GetScore(true), 0.0);
    }

    #[test]
    fn turbo_swing_expands_an_attack_to_every_default_accuracy_choice() {
        let mut game = BMC_Game::default();
        let mut turbo = die(property::TURBO);
        turbo.m_sides[0] = 10;
        turbo.m_swing_type[0] = Some('X');
        turbo.m_value_total = Some(10);
        game.m_player[0].m_die = vec![turbo];
        let mut target = die(0);
        target.m_value_total = Some(8);
        game.m_player[1].m_die = vec![target];

        let moves = game.GenerateValidAttacks();
        let mut choices = moves
            .iter()
            .filter(|action| action.m_attack == Some(BME_ATTACK::POWER))
            .map(|action| action.m_turbo_option)
            .collect::<Vec<_>>();
        choices.sort_unstable();
        assert_eq!(choices, (4_i16..=20).collect::<Vec<_>>());
    }

    #[test]
    fn cpp_order_appends_turbo_alternatives_after_every_base_attack() {
        let mut game = BMC_Game::default();
        let mut turbo = die(property::TURBO);
        turbo.m_sides[0] = 10;
        turbo.m_swing_type[0] = Some('X');
        turbo.m_value_total = Some(10);
        let mut ordinary = die(0);
        ordinary.m_value_total = Some(9);
        ordinary.m_original_index = 1;
        game.m_player[0].m_die = vec![turbo, ordinary];
        let mut target = die(0);
        target.m_value_total = Some(8);
        game.m_player[1].m_die = vec![target];

        let moves = game.GenerateValidAttacksInCppOrder();
        let first_alternative = moves
            .iter()
            .position(|action| action.m_turbo_option >= 0 && action.m_turbo_option != 10)
            .unwrap();
        let last_base = moves
            .iter()
            .rposition(|action| action.m_turbo_option < 0 || action.m_turbo_option == 10)
            .unwrap();
        assert!(first_alternative > last_base);
    }

    #[test]
    fn copied_cpp_skill_restrictions_match_konstant_and_stealth_cases() {
        let target = die(0);

        let mut konstant_game = BMC_Game::default();
        let mut konstant = die(property::KONSTANT);
        konstant.m_value_total = Some(8);
        konstant_game.m_player[0].m_die = vec![konstant];
        konstant_game.m_player[1].m_die = vec![target];
        assert!(konstant_game.GenerateValidAttacks().is_empty());

        let mut stealth_game = BMC_Game::default();
        let mut stealth = die(property::STEALTH);
        stealth.m_value_total = Some(7);
        stealth_game.m_player[0].m_die = vec![stealth];
        stealth_game.m_player[1].m_die = vec![target];
        assert!(stealth_game.GenerateValidAttacks().is_empty());

        let mut ordinary = die(0);
        ordinary.m_original_index = 1;
        ordinary.m_value_total = Some(1);
        stealth_game.m_player[0].m_die = vec![stealth, ordinary];
        assert!(stealth_game.GenerateValidAttacks().iter().any(|action| {
            action.m_attack == Some(BME_ATTACK::SKILL) && action.m_attackers.len() == 2
        }));
    }

    /// Ports NoSkill, MultiDieSkillAttack, SingleDieSkillAttack,
    /// KonstantSingleDieSkillAttack, and StealthMultiDieSkillAttack.
    #[test]
    fn cpp_basic_power_and_skill_attack_generation() {
        let mut a = die(0);
        a.m_sides[0] = 9;
        let mut t = die(0);
        t.m_sides[0] = 7;
        t.m_value_total = Some(6);
        let game = game_with(vec![a], vec![t]);
        assert_eq!(attacks(&game, BME_ATTACK::POWER).len(), 1);

        let mut five = die(0);
        five.m_sides[0] = 6;
        five.m_value_total = Some(5);
        let mut one = five;
        one.m_value_total = Some(1);
        one.m_original_index = 1;
        let mut twenty = die(0);
        twenty.m_value_total = Some(6);
        let game = game_with(vec![five, one], vec![twenty]);
        let skill = attacks(&game, BME_ATTACK::SKILL);
        assert_eq!(skill.len(), 1);
        assert_eq!(skill[0].m_attackers, vec![0, 1]);

        let mut six = die(0);
        six.m_sides[0] = 6;
        six.m_value_total = Some(6);
        let mut twenty = die(0);
        twenty.m_value_total = Some(6);
        let game = game_with(vec![six], vec![twenty]);
        assert_eq!(attacks(&game, BME_ATTACK::POWER).len(), 1);
        assert_eq!(attacks(&game, BME_ATTACK::SKILL).len(), 1);

        let mut konstant = die(property::KONSTANT);
        konstant.m_value_total = Some(6);
        let mut target = die(0);
        target.m_value_total = Some(6);
        assert!(
            game_with(vec![konstant], vec![target])
                .GenerateValidAttacks()
                .is_empty()
        );
    }

    /// Ports InsultSkill and the Stealth attack/vulnerability tests.
    #[test]
    fn cpp_insult_and_stealth_restrictions() {
        let insult = die(property::INSULT);
        assert!(insult.CanBeAttacked(BME_ATTACK::POWER, 1));
        assert!(!insult.CanBeAttacked(BME_ATTACK::SKILL, 2));
        let stealth_insult = die(property::STEALTH | property::INSULT);
        assert!(stealth_insult.CanBeAttacked(BME_ATTACK::SKILL, 2));

        for active in [property::TRIP, property::SHADOW, property::BERSERK] {
            let stealth = die(property::STEALTH | active);
            assert!(stealth.CanDoAttack(BME_ATTACK::SKILL, 2));
            assert!(!stealth.CanDoAttack(BME_ATTACK::POWER, 1));
            assert!(!stealth.CanDoAttack(BME_ATTACK::TRIP, 1));
            assert!(!stealth.CanDoAttack(BME_ATTACK::SHADOW, 1));
            assert!(!stealth.CanDoAttack(BME_ATTACK::BERSERK, 1));
        }

        let mut ordinary = die(0);
        ordinary.m_value_total = Some(6);
        let mut stealth_target = die(property::STEALTH);
        stealth_target.m_value_total = Some(6);
        assert!(
            game_with(vec![ordinary], vec![stealth_target])
                .GenerateValidAttacks()
                .is_empty()
        );
        let mut second = ordinary;
        second.m_original_index = 1;
        second.m_value_total = Some(1);
        ordinary.m_value_total = Some(5);
        assert_eq!(
            attacks(
                &game_with(vec![ordinary, second], vec![stealth_target]),
                BME_ATTACK::SKILL
            )
            .len(),
            1
        );
    }

    #[test]
    fn pr82_variable_skill_stack_disables_legacy_value_pruning() {
        let mut stinger = die(property::STINGER);
        stinger.m_value_total = Some(5);
        let mut three = die(0);
        three.m_value_total = Some(3);
        three.m_original_index = 1;
        let mut two = die(0);
        two.m_value_total = Some(2);
        two.m_original_index = 2;
        let mut target = die(0);
        target.m_value_total = Some(8);

        let skill = attacks(
            &game_with(vec![stinger, three, two], vec![target]),
            BME_ATTACK::SKILL,
        );
        assert!(skill.iter().any(|action| action.m_attackers.len() == 2));
        assert!(skill.iter().any(|action| action.m_attackers.len() == 3));
    }

    /// Ports SpeedSkill and the score assertions from Maximum, Null, Value,
    /// NullValue, Morphing, Poison, PoisonValue, and PoisonNull.
    #[test]
    fn cpp_speed_generation_and_property_score_combinations() {
        let mut speed = die(property::SPEED);
        speed.m_sides[0] = 10;
        speed.m_value_total = Some(8);
        let mut four = die(0);
        four.m_sides[0] = 4;
        four.m_value_total = Some(3);
        let mut six = die(0);
        six.m_sides[0] = 6;
        six.m_value_total = Some(5);
        six.m_original_index = 1;
        let game = game_with(vec![speed], vec![four, six]);
        let speed_attacks = attacks(&game, BME_ATTACK::SPEED);
        assert_eq!(speed_attacks.len(), 1);
        assert_eq!(speed_attacks[0].m_targets, vec![0, 1]);

        let mut scored = die(0);
        scored.m_sides[0] = 30;
        assert_eq!(
            (scored.GetScore(true), scored.GetScore(false)),
            (15.0, 30.0)
        );
        scored.m_properties |= property::MORPHING | property::MAXIMUM;
        assert_eq!(
            (scored.GetScore(true), scored.GetScore(false)),
            (15.0, 30.0)
        );
        scored.m_properties |= property::VALUE;
        assert_eq!((scored.GetScore(true), scored.GetScore(false)), (4.0, 8.0));
        scored.m_properties |= property::POISON;
        assert_eq!(
            (scored.GetScore(true), scored.GetScore(false)),
            (-8.0, -4.0)
        );
        scored.m_properties |= property::NULL;
        assert_eq!((scored.GetScore(true), scored.GetScore(false)), (0.0, 0.0));
    }
}
