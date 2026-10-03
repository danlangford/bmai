// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{BMC_Move, BMC_Player, BME_PHASE};

#[derive(Clone, Debug)]
pub struct BMC_Game {
    pub m_player: [BMC_Player; 2],
    pub m_phase: BME_PHASE,
    pub m_surrender_allowed: bool,
    pub m_target_wins: u8,
    pub m_turbo_accuracy: f32,
    pub m_fire_overshooting: bool,
}

// Ten input dice; twenty slots leave room for decay and Rage growth while
// keeping move indices compact.
pub(crate) const BMD_MAX_INPUT_DICE: usize = 10;
pub(crate) const BMD_MAX_DICE: usize = BMD_MAX_INPUT_DICE * 2;
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
            m_fire_overshooting: false,
        }
    }
}

impl BMC_Game {
    pub fn SimulateAttack(&mut self, action: &BMC_Move, rng: &mut crate::rng::BMC_RNG) -> bool {
        super::ApplyAttack(self, action, rng)
    }

    pub fn CheckInitiative(&self) -> Option<usize> {
        super::CheckInitiative(self)
    }

    pub fn RecoverDizzyDice(&mut self, player: usize) {
        super::RecoverDizzyDice(&mut self.m_player[player]);
    }
}
