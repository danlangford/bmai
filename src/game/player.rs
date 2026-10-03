// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{Die, MAX_DICE, SwingSet};

#[derive(Clone, Debug, Default)]
pub struct Player {
    pub m_id: usize,
    pub m_score: f32,
    pub m_die: Vec<Die>,
    pub m_swing_set: SwingSet,
    pub m_round_original_sides: [[u8; 2]; MAX_DICE],
    pub m_round_transformed: u32,
    pub m_radioactive_products: u32,
    pub m_rage_replacements: u32,
    pub m_specials: u8,
}

impl Player {
    pub fn OptimizeDice(&mut self) {
        // Swapping when a later die outranks an earlier one is the C++ order,
        // so the same exchange sort runs on one cached key per die.
        let count = self.m_die.len();
        let mut keys = [0u16; MAX_DICE];
        for (key, die) in keys.iter_mut().zip(&self.m_die) {
            *key = if die.IsAvailable() {
                256 + die.GetValueTotal()
            } else {
                0
            };
        }
        let keys = &mut keys[..count];
        if keys.windows(2).all(|pair| pair[0] >= pair[1]) {
            return;
        }
        // Not a stable sort: C++'s swap order breaks ties, and search order
        // depends on it.
        for i in 0..count {
            for j in (i + 1)..count {
                if keys[j] > keys[i] {
                    keys.swap(i, j);
                    self.m_die.swap(i, j);
                }
            }
        }
    }
}
