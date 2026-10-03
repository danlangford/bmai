// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{Attack, property};

#[derive(Clone, Copy, Debug)]
pub struct Die {
    pub properties: u64,
    pub sides: [u8; 2],
    pub swing_type: [Option<char>; 2],
    pub value: Option<u8>,
    pub captured: bool,
    pub not_set: bool,
    pub dizzy: bool,
    pub original_index: usize,
    pub in_reserve: bool,
}

impl Die {
    pub fn has_property(&self, property: u64) -> bool {
        self.properties & property != 0
    }
    pub fn sides_max(&self) -> u16 {
        if self.has_property(property::TWIN) {
            self.sides.iter().map(|v| u16::from(*v)).sum()
        } else {
            u16::from(self.sides[0])
        }
    }
    pub fn value_total(&self) -> u16 {
        u16::from(self.value.unwrap_or(0))
    }
    pub fn is_available(&self) -> bool {
        self.value.is_some() && !self.captured && !self.not_set && !self.in_reserve
    }

    pub fn score(&self, own: bool) -> f32 {
        if self.has_property(property::NULL | property::WARRIOR) {
            return 0.0;
        }
        let poison = self.has_property(property::POISON);
        let is_value_die = self.has_property(property::VALUE);
        match (poison, is_value_die, own) {
            (true, true, true) => -(self.value_total() as f32),
            (true, true, false) => -(self.value_total() as f32) * 0.5,
            (true, false, true) => -(self.sides_max() as f32),
            (true, false, false) => -(self.sides_max() as f32) * 0.5,
            (false, true, true) => self.value_total() as f32 * 0.5,
            (false, true, false) => self.value_total() as f32,
            (false, false, true) => self.sides_max() as f32 * 0.5,
            (false, false, false) => self.sides_max() as f32,
        }
    }

    pub fn roll(&mut self, rng: &mut crate::rng::Rng) {
        super::roll_die(self, rng);
    }

    pub fn on_swing_set(&mut self, swing: char, value: u8) {
        assert!(self.not_set, "Die::OnSwingSet requires NOTSET state");
        for side in 0..2 {
            if self.swing_type[side] == Some(swing) {
                self.sides[side] = value;
            }
        }
    }

    pub fn on_dizzy_recovered(&mut self) {
        self.dizzy = false;
    }

    pub(super) fn can_do_attack(&self, attack: Attack, skill_dice: usize) -> bool {
        if !self.is_available() || self.dizzy {
            return false;
        }
        if self.has_property(property::WARRIOR) {
            return attack == Attack::Skill;
        }
        if self.has_property(property::STEALTH) {
            return attack == Attack::Skill && skill_dice > 1;
        }
        match attack {
            Attack::Power => {
                !self.has_property(property::SHADOW | property::KONSTANT | property::FIRE)
                    && !(self.has_property(property::QUEER) && self.value_total() % 2 == 1)
            }
            Attack::Skill => !self.has_property(property::UNSKILLED | property::BERSERK),
            Attack::Berserk => self.has_property(property::BERSERK),
            Attack::Speed => self.has_property(property::SPEED),
            Attack::Trip => self.has_property(property::TRIP),
            Attack::Shadow => {
                self.has_property(property::SHADOW)
                    || self.has_property(property::QUEER) && self.value_total() % 2 == 1
            }
            // Rush legality depends on the targets, so enumeration checks it.
            Attack::Rush => true,
            Attack::Boom => self.has_property(property::BOOM),
        }
    }

    pub(super) fn can_be_attacked(&self, attack: Attack, skill_dice: usize) -> bool {
        if self.has_property(property::WARRIOR) {
            return false;
        }
        // Stealth overrides Insult, as in C++ RecomputeAttacks.
        if self.has_property(property::STEALTH) {
            return attack == Attack::Boom || attack == Attack::Skill && skill_dice > 1;
        }
        if self.has_property(property::INSULT) && attack == Attack::Skill {
            return false;
        }
        true
    }
}
