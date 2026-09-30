// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

/// Selects the compatibility contract used by the game/search engine.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ExecutionMode {
    /// Exact C++ behavior, including candidate order and RNG consumption.
    #[default]
    Legacy,
    /// Rust-native evolution point. It intentionally shares the legacy
    /// implementation until a separately tested native behavior is introduced.
    Native,
}

impl ExecutionMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Legacy => "legacy",
            Self::Native => "native",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "legacy" | "parity" => Some(Self::Legacy),
            "native" => Some(Self::Native),
            _ => None,
        }
    }
}
