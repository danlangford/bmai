// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BME_PHASE {
    AUXILIARY,
    PREROUND,
    RESERVE,
    INITIATIVE,
    CHANCE,
    FOCUS,
    FIGHT,
    GAMEOVER,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BME_ACTION {
    USE_AUXILIARY,
    SET_SWING_AND_OPTION,
    USE_RESERVE,
    ATTACK,
    PASS,
    SURRENDER,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum BME_SWING_SET {
    #[default]
    NOT,
    READY,
    LOCKED,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BME_ATTACK {
    POWER,
    SKILL,
    BERSERK,
    SPEED,
    TRIP,
    SHADOW,
    RUSH,
}

impl BME_ATTACK {
    pub fn protocol(self) -> &'static str {
        match self {
            Self::POWER => "power",
            Self::SKILL => "skill",
            Self::BERSERK => "berserk",
            Self::SPEED => "speed",
            Self::TRIP => "trip",
            Self::SHADOW => "shadow",
            Self::RUSH => "rush",
        }
    }
}
