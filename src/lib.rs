// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

#![allow(non_camel_case_types, non_snake_case)]

mod game;
mod mode;
pub mod native;
pub mod protocol;
mod rng;
mod search;

pub use game::{
    BMC_Die, BMC_DieIndexSet, BMC_Game, BMC_Move, BMC_Player, BME_ACTION, BME_ATTACK, BME_PHASE,
    BME_SWING_SET, property,
};
pub use mode::ExecutionMode;
pub use protocol::jsonl;
pub use protocol::jsonl::{BmairSession, SessionExecuteResult, run_jsonl};
pub use protocol::legacy::{BMC_Parser, ParseError};
pub use protocol::notation;
pub use protocol::notation::{
    CapabilitySupport, DieNotationCapabilities, DiePostfixNotation, DiePropertyNotation,
};
pub use protocol::{
    BuildIdentity, Capabilities, PlayerAiMetadata, ProtocolAction, ProtocolFloat, ProtocolVersion,
    ReplayMetadata, SessionMetadata,
};
pub use rng::{BMC_RNG, BME_RNG_ALGORITHM};
pub use search::ai::{BMC_BMAI3, BMC_Stats, BME_ROLLOUT_POLICY, EvaluationCoordinate};
pub use search::{BMC_AI_POLICY, PlayGames};
#[cfg(test)]
mod build_metadata;
