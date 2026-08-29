// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use serde::Serialize;

/// A public protocol identifier is permanent once released. Add a variant for
/// incompatible future contracts instead of changing an existing contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
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

#[derive(Debug, Serialize)]
pub struct BuildIdentity {
    pub version: &'static str,
    pub git_describe: &'static str,
    pub profile: &'static str,
}

#[derive(Debug, Serialize)]
pub struct NativeCapabilities {
    pub execution_modes: &'static [&'static str],
    pub rng_algorithms: &'static [&'static str],
    pub minimum_workers: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SessionMetadata {
    pub phase: &'static str,
    pub execution_mode: &'static str,
    pub rng: &'static str,
    pub native_root_seed: u64,
    pub native_decision_index: u64,
    pub workers: usize,
    pub max_ply: usize,
    pub min_simulations: usize,
    pub max_simulations: usize,
    pub max_branch: usize,
}

/// Complete identity of the native decision stream used by the most recent
/// search. Candidate and simulation coordinates are derived from this key.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReplayMetadata {
    pub stream_partition: &'static str,
    pub root_seed: u64,
    pub decision_index: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SwingSelection {
    pub swing: char,
    pub value: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OptionSelection {
    pub die: usize,
    pub value: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FocusSelection {
    pub die: usize,
    pub value: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TurboSelection {
    Option { die: usize, value: u8 },
    Swing { swing: char, value: u8 },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProtocolAction {
    Pass,
    Surrender,
    Attack {
        attack_type: &'static str,
        attackers: Vec<usize>,
        targets: Vec<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        turbo: Option<TurboSelection>,
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
pub struct Capabilities {
    pub implementation: &'static str,
    pub build: BuildIdentity,
    pub protocols: &'static [ProtocolVersion],
    pub commands: &'static [&'static str],
    pub phases: &'static [&'static str],
    pub actions: &'static [&'static str],
    pub attack_types: &'static [&'static str],
    pub skills: &'static [&'static str],
    pub parsing_only_skills: &'static [&'static str],
    pub native: NativeCapabilities,
}

impl Capabilities {
    pub const fn current() -> Self {
        Self {
            implementation: "bmair",
            build: BuildIdentity {
                version: env!("BMAIR_BUILD_VERSION"),
                git_describe: env!("BMAIR_GIT_DESCRIBE"),
                profile: env!("BMAIR_BUILD_PROFILE"),
            },
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
                "chance",
                "focus",
                "pass",
                "reserve",
                "set_swing",
                "surrender",
            ],
            attack_types: &["power", "skill", "berserk", "speed", "trip", "shadow"],
            skills: &[
                "Berserk",
                "Chance",
                "Focus",
                "Insult",
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
            parsing_only_skills: &["Auxiliary", "Doppelganger", "Radioactive", "Rage"],
            native: NativeCapabilities {
                execution_modes: &["legacy", "native"],
                rng_algorithms: &["legacy", "park-miller"],
                minimum_workers: 1,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_have_stable_protocol_names_and_serialize() {
        assert_eq!(ProtocolVersion::LegacyV1.as_str(), "legacy-v1");
        assert_eq!(ProtocolVersion::JsonlV1.as_str(), "jsonl-v1");

        let value = serde_json::to_value(Capabilities::current()).unwrap();
        assert_eq!(value["implementation"], "bmair");
        assert_eq!(value["protocols"][0], "legacy-v1");
        assert_eq!(value["protocols"][1], "jsonl-v1");
        assert!(
            value["commands"]
                .as_array()
                .unwrap()
                .contains(&"getaction".into())
        );
        assert!(
            value["skills"]
                .as_array()
                .unwrap()
                .contains(&"Konstant".into())
        );
        assert_eq!(
            value["actions"],
            serde_json::json!([
                "attack",
                "chance",
                "focus",
                "pass",
                "reserve",
                "set_swing",
                "surrender"
            ])
        );
        assert!(
            value["parsing_only_skills"]
                .as_array()
                .unwrap()
                .contains(&"Auxiliary".into())
        );
    }

    #[test]
    fn every_typed_action_shape_has_a_stable_discriminator() {
        let actions = [
            serde_json::to_value(ProtocolAction::Pass).unwrap(),
            serde_json::to_value(ProtocolAction::Surrender).unwrap(),
            serde_json::to_value(ProtocolAction::Attack {
                attack_type: "skill",
                attackers: vec![0, 2],
                targets: vec![1],
                turbo: Some(TurboSelection::Swing {
                    swing: 'X',
                    value: 12,
                }),
            })
            .unwrap(),
            serde_json::to_value(ProtocolAction::Reserve { die: None }).unwrap(),
            serde_json::to_value(ProtocolAction::SetSwing {
                swings: vec![SwingSelection {
                    swing: 'X',
                    value: 12,
                }],
                options: vec![OptionSelection { die: 1, value: 20 }],
            })
            .unwrap(),
            serde_json::to_value(ProtocolAction::Chance { dice: vec![0, 2] }).unwrap(),
            serde_json::to_value(ProtocolAction::Focus {
                dice: vec![FocusSelection { die: 0, value: 4 }],
            })
            .unwrap(),
        ];
        assert_eq!(
            actions
                .iter()
                .map(|action| action["type"].as_str().unwrap())
                .collect::<Vec<_>>(),
            [
                "pass",
                "surrender",
                "attack",
                "reserve",
                "set_swing",
                "chance",
                "focus"
            ]
        );
        assert_eq!(actions[2]["turbo"]["kind"], "swing");
    }
}
