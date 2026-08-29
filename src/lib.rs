// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

#![allow(non_camel_case_types, non_snake_case)]

mod ai;
mod engine;
pub mod jsonl;
mod model;
pub mod native;
mod parser;
pub mod protocol;
mod rng;
mod simulation;

pub use ai::{BMC_BMAI3, BMC_Stats, BME_ROLLOUT_POLICY, EvaluationCoordinate};
pub use engine::ExecutionMode;
pub use jsonl::{BmairSession, SessionExecuteResult, run_jsonl};
pub use model::{
    BMC_Die, BMC_DieIndexSet, BMC_Game, BMC_Move, BMC_Player, BME_ACTION, BME_ATTACK, BME_PHASE,
    BME_SWING_SET, property,
};
pub use parser::{BMC_Parser, ParseError};
pub use protocol::{
    Capabilities, ProtocolAction, ProtocolVersion, ReplayMetadata, SessionMetadata,
};
pub use rng::{BMC_RNG, BME_RNG_ALGORITHM};
pub use simulation::{BMC_AI_POLICY, PlayGames};
#[cfg(test)]
mod build_version;
