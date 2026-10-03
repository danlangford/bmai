// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{Action, Attack, MAX_DICE};

#[derive(Clone, Debug)]
pub struct Move {
    pub m_action: Action,
    pub m_attack: Option<Attack>,
    pub m_attackers: DieIndexSet,
    pub m_targets: DieIndexSet,
    pub m_score: f32,
    /// -1 means no Turbo decision; option dice use 0/1; swing dice store a size.
    pub m_turbo_option: i16,
    pub m_fire: FireAdjustment,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FireAdjustment {
    /// Attackers' entries are increases; other dice's entries are reductions.
    pub m_amounts: [u8; MAX_DICE],
}

impl Default for FireAdjustment {
    fn default() -> Self {
        Self {
            m_amounts: [0; MAX_DICE],
        }
    }
}

impl FireAdjustment {
    pub fn is_empty(&self) -> bool {
        self.m_amounts.iter().all(|amount| *amount == 0)
    }

    pub(super) fn from_allocations(
        mut increases: [u8; MAX_DICE],
        reductions: [u8; MAX_DICE],
    ) -> Self {
        for index in 0..MAX_DICE {
            debug_assert!(increases[index] == 0 || reductions[index] == 0);
            increases[index] += reductions[index];
        }
        Self {
            m_amounts: increases,
        }
    }
}

impl Move {
    pub(crate) fn attack(
        kind: Attack,
        attackers: impl Into<DieIndexSet>,
        targets: impl Into<DieIndexSet>,
        score: f32,
    ) -> Self {
        Self {
            m_action: Action::Attack,
            m_attack: Some(kind),
            m_attackers: attackers.into(),
            m_targets: targets.into(),
            m_score: score,
            m_turbo_option: -1,
            m_fire: FireAdjustment::default(),
        }
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub struct DieIndexSet(u32);

impl DieIndexSet {
    pub fn iter(self) -> impl Iterator<Item = usize> {
        (0..MAX_DICE).filter(move |index| self.0 & (1 << index) != 0)
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
        index < MAX_DICE && self.0 & (1 << index) != 0
    }
}

impl std::fmt::Debug for DieIndexSet {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_list().entries(self.iter()).finish()
    }
}

impl From<Vec<usize>> for DieIndexSet {
    fn from(indices: Vec<usize>) -> Self {
        indices.as_slice().into()
    }
}

impl<const N: usize> From<[usize; N]> for DieIndexSet {
    fn from(indices: [usize; N]) -> Self {
        indices.as_slice().into()
    }
}

impl From<&[usize]> for DieIndexSet {
    fn from(indices: &[usize]) -> Self {
        let mut bits = 0u32;
        for index in indices {
            assert!(*index < MAX_DICE);
            bits |= 1 << index;
        }
        Self(bits)
    }
}

impl FromIterator<usize> for DieIndexSet {
    fn from_iter<T: IntoIterator<Item = usize>>(indices: T) -> Self {
        let mut bits = 0u32;
        for index in indices {
            assert!(index < MAX_DICE);
            bits |= 1 << index;
        }
        Self(bits)
    }
}

impl PartialEq<Vec<usize>> for DieIndexSet {
    fn eq(&self, other: &Vec<usize>) -> bool {
        self.iter().eq(other.iter().copied())
    }
}
