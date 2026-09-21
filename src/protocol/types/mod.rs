// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use serde::Serialize;

use crate::protocol::notation::DieNotationCapabilities;

/// A public protocol identifier is permanent once released. Add a variant for
/// incompatible future contracts instead of changing an existing contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ProtocolVersion {
    LegacyV1,
    JsonlV1,
}

impl ProtocolVersion {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LegacyV1 => "legacy-v1",
            Self::JsonlV1 => "jsonl-v1",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct BuildIdentity {
    pub version: &'static str,
    pub git_describe: &'static str,
    pub profile: &'static str,
}

impl BuildIdentity {
    pub const fn current() -> Self {
        Self {
            version: env!("BMAIR_BUILD_VERSION"),
            git_describe: env!("BMAIR_GIT_DESCRIBE"),
            profile: env!("BMAIR_BUILD_PROFILE"),
        }
    }
}

#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct NativeCapabilities {
    pub execution_modes: &'static [&'static str],
    pub rng_algorithms: &'static [&'static str],
    pub minimum_workers: usize,
    pub automatic_workers: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct PlayerAiMetadata {
    pub ai_type: usize,
    pub policy: &'static str,
    pub culls_moves: bool,
    pub max_ply: usize,
    pub min_simulations: usize,
    pub max_simulations: usize,
    pub max_branch: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum ProtocolFloat {
    Finite(f32),
    NonFinite(&'static str),
}

impl ProtocolFloat {
    pub fn from_f32(value: f32) -> Self {
        if value.is_nan() {
            Self::NonFinite("nan")
        } else if value == f32::INFINITY {
            Self::NonFinite("infinity")
        } else if value == f32::NEG_INFINITY {
            Self::NonFinite("-infinity")
        } else {
            Self::Finite(value)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct SessionMetadata {
    pub phase: &'static str,
    pub target_wins: u8,
    pub surrender_allowed: bool,
    pub turbo_accuracy: ProtocolFloat,
    pub fire_overshooting: bool,
    pub execution_mode: &'static str,
    pub rng: &'static str,
    pub native_root_seed: u64,
    pub native_decision_index: u64,
    pub workers: usize,
    pub max_ply: usize,
    pub min_simulations: usize,
    pub max_simulations: usize,
    pub max_branch: usize,
    pub players: [PlayerAiMetadata; 2],
}

/// Complete identity of the native decision stream used by the most recent
/// search. Candidate and simulation coordinates are derived from this key.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ReplayMetadata {
    pub stream_partition: &'static str,
    pub root_seed: u64,
    pub decision_index: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct SwingSelection {
    pub swing: char,
    pub value: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct OptionSelection {
    pub die: usize,
    pub value: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FocusSelection {
    pub die: usize,
    pub value: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FireSelection {
    pub die: usize,
    pub value: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum TurboSelection {
    Option { die: usize, value: u8 },
    Swing { swing: char, value: u8 },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ProtocolAction {
    Pass,
    Surrender,
    Auxiliary {
        die: Option<usize>,
    },
    Attack {
        attack_type: &'static str,
        attackers: Vec<usize>,
        targets: Vec<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        turbo: Option<TurboSelection>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        fire: Vec<FireSelection>,
    },
    Reserve {
        die: Option<usize>,
    },
    SetSwing {
        swings: Vec<SwingSelection>,
        options: Vec<OptionSelection>,
    },
    Chance {
        dice: Vec<usize>,
    },
    Focus {
        dice: Vec<FocusSelection>,
    },
}

#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct Capabilities {
    pub implementation: &'static str,
    pub build: BuildIdentity,
    pub protocols: &'static [ProtocolVersion],
    pub commands: &'static [&'static str],
    pub phases: &'static [&'static str],
    pub actions: &'static [&'static str],
    pub attack_types: &'static [&'static str],
    pub ai_policies: &'static [&'static str],
    pub skills: &'static [&'static str],
    pub parsing_only_skills: &'static [&'static str],
    pub die_notation: DieNotationCapabilities,
    pub native: NativeCapabilities,
}

impl Capabilities {
    pub const fn current() -> Self {
        Self {
            implementation: "bmair",
            build: BuildIdentity::current(),
            protocols: &[ProtocolVersion::LegacyV1, ProtocolVersion::JsonlV1],
            commands: &[
                "mode",
                "rng",
                "workers",
                "game",
                "player",
                "ai",
                "ply",
                "max_sims",
                "min_sims",
                "maxbranch",
                "turbo_accuracy",
                "fire_overshooting",
                "surrender",
                "getaction",
                "playgame",
                "playfair",
                "compare",
                "seed",
                "debugply",
                "debug",
                "quit",
            ],
            phases: &[
                "aux",
                "preround",
                "reserve",
                "initiative",
                "chance",
                "focus",
                "fight",
                "gameover",
            ],
            actions: &[
                "attack",
                "auxiliary",
                "chance",
                "focus",
                "pass",
                "reserve",
                "set_swing",
                "surrender",
            ],
            attack_types: &["power", "skill", "berserk", "speed", "trip", "shadow"],
            ai_policies: &["bmai", "qai", "bmai3"],
            skills: &[
                "Auxiliary",
                "Berserk",
                "Chance",
                "Doppelganger",
                "Focus",
                "Fire",
                "Insult",
                "Jolt",
                "Konstant",
                "Maximum",
                "Mighty",
                "Mood",
                "Morphing",
                "Null",
                "Option",
                "Ornery",
                "Poison",
                "Queer",
                "Rage",
                "Reserve",
                "Shadow",
                "Slow",
                "Speed",
                "Stealth",
                "Stinger",
                "Swing",
                "TimeAndSpace",
                "Trip",
                "Turbo",
                "Twin",
                "Unique",
                "Unskilled",
                "Value",
                "Warrior",
                "Weak",
            ],
            parsing_only_skills: &["Radioactive"],
            die_notation: DieNotationCapabilities::current(),
            native: NativeCapabilities {
                execution_modes: &["legacy", "native"],
                rng_algorithms: &["legacy", "park-miller"],
                minimum_workers: 1,
                automatic_workers: true,
            },
        }
    }
}

#[cfg(test)]
mod tests;
