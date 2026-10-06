// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ExecutionMode {
    Legacy,
    /// Shares the legacy implementation until native behavior is tested apart.
    #[default]
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
