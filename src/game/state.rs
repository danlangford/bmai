// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{Move, Phase, Player};

#[derive(Clone, Debug)]
pub struct Game {
    pub players: [Player; 2],
    pub phase: Phase,
    pub surrender_allowed: bool,
    pub target_wins: u8,
    pub turbo_accuracy: f32,
    pub fire_overshooting: bool,
}

// Ten input dice; twenty slots leave room for decay and Rage growth while
// keeping move indices compact.
pub(crate) const MAX_INPUT_DICE: usize = 10;
pub(crate) const MAX_DICE: usize = MAX_INPUT_DICE * 2;
impl Default for Game {
    fn default() -> Self {
        Self {
            players: [
                Player {
                    id: 0,
                    ..Default::default()
                },
                Player {
                    id: 1,
                    ..Default::default()
                },
            ],
            phase: Phase::Preround,
            surrender_allowed: false,
            target_wins: 3,
            turbo_accuracy: 1.0,
            fire_overshooting: true,
        }
    }
}

impl Game {
    pub fn simulate_attack(&mut self, action: &Move, rng: &mut crate::rng::Rng) -> bool {
        super::apply_attack(self, action, rng)
    }

    pub fn check_initiative(&self) -> Option<usize> {
        super::check_initiative(self)
    }

    pub fn recover_dizzy_dice(&mut self, player: usize) {
        super::recover_dizzy_dice(&mut self.players[player]);
    }
}
