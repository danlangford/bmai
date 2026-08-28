// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

#![allow(non_camel_case_types, non_snake_case)]

mod ai;
mod engine;
mod model;
mod parser;
mod rng;
mod simulation;

pub use ai::{BMC_BMAI3, BMC_Stats, BME_ROLLOUT_POLICY};
pub use engine::ExecutionMode;
pub use model::{
    BMC_Die, BMC_DieIndexSet, BMC_Game, BMC_Move, BMC_Player, BME_ACTION, BME_ATTACK, BME_PHASE,
    BME_SWING_SET, property,
};
pub use parser::{BMC_Parser, ParseError};
pub use rng::{BMC_RNG, BME_RNG_ALGORITHM};
pub use simulation::{BMC_AI_POLICY, PlayGames};
