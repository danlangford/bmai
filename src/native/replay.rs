// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

/// Change this whenever the derivation changes, so recorded searches replay.
pub const NATIVE_STREAM_PARTITION_ID: &str = "bmair-native-stream-v2";

const ROOT_SALT: u64 = 0x524f_4f54_5345_4544; // "ROOTSEED"
const DECISION_SALT: u64 = 0x4445_4349_5349_4f4e; // "DECISION"
const CANDIDATE_SALT: u64 = 0x4341_4e44_4944_4154; // "CANDIDAT"
const BATCH_SALT: u64 = 0x4241_5443_485f_5f5f; // "BATCH___"
const SIMULATION_SALT: u64 = 0x5349_4d55_4c41_5445; // "SIMULATE"
const STATE_DOMAIN: u64 = 0x5354_4154_455f_5f5f; // "STATE___"
const STREAM_DOMAIN: u64 = 0x5354_5245_414d_5f5f; // "STREAM__"

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NativeReplayKey {
    pub root_seed: u64,
    pub decision_index: u64,
}

/// No worker identity, so the worker count cannot change a stream.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NativeSimulationKey {
    pub replay: NativeReplayKey,
    pub candidate_index: u64,
    pub batch_index: u64,
    pub simulation_index: u64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NativeStreamSeed {
    pub state: u64,
    pub stream: u64,
}

/// Spreads each candidate's first draws across outcomes; the offset keeps
/// candidates from sharing an ordering.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct NativeStratum {
    pub index: u64,
    pub offset: u64,
    pub radix: u64,
}

impl NativeStreamSeed {
    /// Zero and the modulus are invalid Park-Miller states.
    #[must_use]
    pub const fn legacy_park_miller_state(self) -> u32 {
        const MAX_STATE: u64 = 2_147_483_646;
        ((self.state ^ self.stream.rotate_left(29)) % MAX_STATE + 1) as u32
    }
}

impl NativeSimulationKey {
    /// Rust's default hashing is not a stable replay format.
    #[must_use]
    pub fn derive_stream_seed(self) -> NativeStreamSeed {
        const DOMAIN: u64 = 0x424d_4149_525f_4e32; // "BMAIR_N2"
        let mut accumulator = mix64(DOMAIN);
        accumulator = fold(accumulator, self.replay.root_seed, ROOT_SALT);
        accumulator = fold(accumulator, self.replay.decision_index, DECISION_SALT);
        accumulator = fold(accumulator, self.candidate_index, CANDIDATE_SALT);
        accumulator = fold(accumulator, self.batch_index, BATCH_SALT);
        accumulator = fold(accumulator, self.simulation_index, SIMULATION_SALT);

        NativeStreamSeed {
            state: mix64(accumulator ^ STATE_DOMAIN),
            stream: mix64(accumulator ^ STREAM_DOMAIN),
        }
    }

    #[must_use]
    pub(crate) fn stratum(self) -> NativeStratum {
        const DOMAIN: u64 = 0x5354_5241_5455_4d32; // "STRATUM2"
        let mut accumulator = mix64(DOMAIN);
        accumulator = fold(accumulator, self.replay.root_seed, ROOT_SALT);
        accumulator = fold(accumulator, self.replay.decision_index, DECISION_SALT);
        NativeStratum {
            index: self.simulation_index,
            offset: fold(accumulator, self.candidate_index, CANDIDATE_SALT),
            radix: 1,
        }
    }
}

fn fold(accumulator: u64, value: u64, salt: u64) -> u64 {
    mix64(accumulator ^ mix64(value ^ salt))
}

pub(super) fn mix64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
