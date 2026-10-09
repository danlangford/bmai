// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{Die, SwingSet};

#[derive(Clone, Debug, Default)]
pub struct Player {
    pub id: usize,
    pub score: f32,
    pub dice: Vec<Die>,
    pub swing_set: SwingSet,
    pub radioactive_products: u32,
    pub rage_replacements: u32,
    pub specials: u8,
}

impl Player {
    /// No rule orders tied dice; a stable sort keeps their candidate indices,
    /// and so their replay streams, where they were.
    pub fn optimize_dice(&mut self) {
        self.dice
            .sort_by_key(|die| std::cmp::Reverse(die.is_available().then(|| die.value_total())));
    }
}
