// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{Move, Phase, Player};

#[derive(Clone, Debug)]
pub struct Game {
    pub m_player: [Player; 2],
    pub m_phase: Phase,
    pub m_surrender_allowed: bool,
    pub m_target_wins: u8,
    pub m_turbo_accuracy: f32,
    pub m_fire_overshooting: bool,
}

// Ten input dice; twenty slots leave room for decay and Rage growth while
// keeping move indices compact.
pub(crate) const MAX_INPUT_DICE: usize = 10;
pub(crate) const MAX_DICE: usize = MAX_INPUT_DICE * 2;
impl Default for Game {
    fn default() -> Self {
        Self {
            m_player: [
                Player {
                    m_id: 0,
                    ..Default::default()
                },
                Player {
                    m_id: 1,
                    ..Default::default()
                },
            ],
            m_phase: Phase::Preround,
            m_surrender_allowed: true,
            m_target_wins: 3,
            m_turbo_accuracy: 1.0,
            m_fire_overshooting: false,
        }
    }
}

impl Game {
    pub fn SimulateAttack(&mut self, action: &Move, rng: &mut crate::rng::Rng) -> bool {
        super::ApplyAttack(self, action, rng)
    }

    pub fn CheckInitiative(&self) -> Option<usize> {
        super::CheckInitiative(self)
    }

    pub fn RecoverDizzyDice(&mut self, player: usize) {
        super::RecoverDizzyDice(&mut self.m_player[player]);
    }
}
