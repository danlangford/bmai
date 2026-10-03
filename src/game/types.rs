// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Auxiliary,
    Preround,
    Reserve,
    Initiative,
    Chance,
    Focus,
    Fight,
    Gameover,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    UseAuxiliary,
    SetSwingAndOption,
    UseReserve,
    Attack,
    Pass,
    Surrender,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SwingSet {
    #[default]
    Not,
    Ready,
    Locked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Attack {
    Power,
    Skill,
    Berserk,
    Speed,
    Trip,
    Shadow,
    Rush,
    Boom,
}

impl Attack {
    pub fn protocol(self) -> &'static str {
        match self {
            Self::Power => "power",
            Self::Skill => "skill",
            Self::Berserk => "berserk",
            Self::Speed => "speed",
            Self::Trip => "trip",
            Self::Shadow => "shadow",
            Self::Rush => "rush",
            Self::Boom => "boom",
        }
    }
}
