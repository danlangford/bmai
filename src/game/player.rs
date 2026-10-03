// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{Die, MAX_DICE, SwingSet};

#[derive(Clone, Debug, Default)]
pub struct Player {
    pub id: usize,
    pub score: f32,
    pub dice: Vec<Die>,
    pub swing_set: SwingSet,
    pub round_original_sides: [[u8; 2]; MAX_DICE],
    pub round_transformed: u32,
    pub radioactive_products: u32,
    pub rage_replacements: u32,
    pub specials: u8,
}

impl Player {
    pub fn optimize_dice(&mut self) {
        let count = self.dice.len();
        let mut keys = [0u16; MAX_DICE];
        for (key, die) in keys.iter_mut().zip(&self.dice) {
            *key = order_key(die);
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
                    self.dice.swap(i, j);
                }
            }
        }
    }
}

// Values fit in a u8, so 256 ranks every available die above every
// unavailable one, and unavailable dice tie, as in C++.
fn order_key(die: &Die) -> u16 {
    if die.is_available() {
        256 + die.value_total()
    } else {
        0
    }
}
