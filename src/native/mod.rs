// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! Deterministic replay identities and ordered native worker execution.

mod replay;
mod worker;

pub(crate) use replay::NativeStratum;
pub use replay::{
    NATIVE_STREAM_PARTITION_ID, NATIVE_STREAM_PARTITION_V1_ID, NativeReplayKey,
    NativeSimulationKey, NativeStreamSeed, NativeStreamVersion,
};
pub(crate) use worker::{native_worker_active, ordered_parallel_map};

#[cfg(test)]
use replay::mix64;
#[cfg(test)]
mod tests;
