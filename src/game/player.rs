// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{BMC_Die, BMD_MAX_DICE, BME_SWING_SET};

#[derive(Clone, Debug, Default)]
pub struct BMC_Player {
    pub m_id: usize,
    pub m_score: f32,
    pub m_die: Vec<BMC_Die>,
    pub m_swing_set: BME_SWING_SET,
    /// Dynamic sides of original recipes transformed during this round.
    pub m_round_original_sides: [[u8; 2]; BMD_MAX_DICE],
    /// Stable indices whose original recipe must return next round.
    pub m_round_transformed: u32,
    /// Synthetic Radioactive decay products removed before the next round.
    pub m_radioactive_products: u32,
    /// Synthetic Rage replacements removed before the next round.
    pub m_rage_replacements: u32,
    /// `game::special` flags for this player's button.
    pub m_specials: u8,
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
