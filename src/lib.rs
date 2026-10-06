// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

mod engines;
mod game;
mod mode;
pub mod native;
pub mod protocol;
mod rng;
mod search;
pub mod strength;

pub use engines::{ENGINE_NAMES, Setting};
pub use game::{Action, Attack, Die, DieIndexSet, Game, Move, Phase, Player, SwingSet, property};
pub use mode::ExecutionMode;
pub use protocol::jsonl;
pub use protocol::jsonl::{BmairSession, SessionExecuteResult, run_jsonl};
pub use protocol::legacy::{ParseError, Parser};
pub use protocol::notation;
pub use protocol::notation::{
    CapabilitySupport, DieNotationCapabilities, DiePostfixNotation, DiePropertyNotation,
};
pub use protocol::{
    BuildIdentity, Capabilities, MonteCarloMetadata, PlayerAiMetadata, ProbabilityEstimate,
    ProtocolAction, ProtocolFloat, ProtocolVersion, ReplayMetadata, SessionMetadata,
};
pub use rng::{Rng, RngAlgorithm};
pub use search::ai::{Bmai3, EvaluationCoordinate, Playout, QuickTweaks, Stats};
pub use search::play_games;
#[cfg(test)]
mod build_metadata;
