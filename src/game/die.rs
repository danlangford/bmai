// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{BME_ATTACK, property};

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

    pub fn Roll(&mut self, rng: &mut crate::rng::BMC_RNG) {
        super::RollDie(self, rng);
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

    pub(super) fn CanDoAttack(&self, attack: BME_ATTACK, skill_dice: usize) -> bool {
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
                !self.HasProperty(property::SHADOW | property::KONSTANT | property::FIRE)
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
            // Rush legality depends on the targets, so enumeration checks it.
            BME_ATTACK::RUSH => true,
        }
    }

    pub(super) fn CanBeAttacked(&self, attack: BME_ATTACK, skill_dice: usize) -> bool {
        if self.HasProperty(property::WARRIOR) {
            return false;
        }
        // Stealth overrides Insult, as in C++ RecomputeAttacks.
        if self.HasProperty(property::STEALTH) {
            return attack == BME_ATTACK::SKILL && skill_dice > 1;
        }
        if self.HasProperty(property::INSULT) && attack == BME_ATTACK::SKILL {
            return false;
        }
        true
    }
}
