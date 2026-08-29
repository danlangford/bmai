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

#[derive(Debug, Serialize)]
pub struct Capabilities {
    pub implementation: &'static str,
    pub build: BuildIdentity,
    pub protocols: &'static [ProtocolVersion],
    pub commands: &'static [&'static str],
    pub phases: &'static [&'static str],
    pub actions: &'static [&'static str],
    pub skills: &'static [&'static str],
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
                "auxiliary",
                "chance",
                "focus",
                "option",
                "pass",
                "reserve",
                "surrender",
                "swing",
                "turbo",
            ],
            skills: &[
                "Auxiliary",
                "Berserk",
                "Chance",
                "Doppelganger",
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
                "Radioactive",
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
                "Value",
                "Warrior",
                "Weak",
            ],
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
    }
}
