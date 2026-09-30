// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

/// Identifies the stream-partitioning algorithm used by [`NativeSimulationKey`].
///
/// Changing the derivation requires a new identifier so recorded searches can
/// continue to be reproduced with their original semantics.
pub const NATIVE_STREAM_PARTITION_ID: &str = "bmair-native-stream-v2";
pub const NATIVE_STREAM_PARTITION_V1_ID: &str = "bmair-native-stream-v1";

const ROOT_SALT: u64 = 0x524f_4f54_5345_4544; // "ROOTSEED"
const DECISION_SALT: u64 = 0x4445_4349_5349_4f4e; // "DECISION"
const CANDIDATE_SALT: u64 = 0x4341_4e44_4944_4154; // "CANDIDAT"
const BATCH_SALT: u64 = 0x4241_5443_485f_5f5f; // "BATCH___"
const SIMULATION_SALT: u64 = 0x5349_4d55_4c41_5445; // "SIMULATE"
const STATE_DOMAIN: u64 = 0x5354_4154_455f_5f5f; // "STATE___"
const STREAM_DOMAIN: u64 = 0x5354_5245_414d_5f5f; // "STREAM__"

/// Selects the versioned native stream-partitioning contract.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NativeStreamVersion {
    V1,
    V2,
}

impl NativeStreamVersion {
    pub const CURRENT: Self = Self::V2;

    pub(crate) const fn completes_probability_sample(self) -> bool {
        matches!(self, Self::V2)
    }

    /// Returns the identifier to persist with a replay or benchmark result.
    #[must_use]
    pub const fn partition_id(self) -> &'static str {
        match self {
            Self::V1 => NATIVE_STREAM_PARTITION_V1_ID,
            Self::V2 => NATIVE_STREAM_PARTITION_ID,
        }
    }
}

/// Identifies one native-mode search decision within a seeded replay.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NativeReplayKey {
    pub stream_version: NativeStreamVersion,
    pub root_seed: u64,
    pub decision_index: u64,
}

/// Identifies one simulation independently of where or when it is executed.
///
/// Canonical indices come from the coordinator. Worker identity is deliberately
/// absent so changing the number of workers cannot change a simulation stream.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NativeSimulationKey {
    pub replay: NativeReplayKey,
    pub candidate_index: u64,
    pub batch_index: u64,
    pub simulation_index: u64,
}

/// The two words needed to initialize a future native random-number stream.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NativeStreamSeed {
    pub state: u64,
    pub stream: u64,
}

/// Deterministic strata for a simulation's initial consecutive bounded draws.
///
/// A candidate's simulations walk adjacent mixed-radix cells, while the offset
/// keeps different candidates from sharing the same ordering. `radix` records
/// the product of the bounds already sampled in this simulation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct NativeStratum {
    pub index: u64,
    pub offset: u64,
    pub radix: u64,
}

impl NativeStreamSeed {
    /// Folds both native seed words into a valid Park-Miller state.
    ///
    /// Native mode currently reuses the proven legacy generator inside each
    /// independently partitioned simulation. The range excludes zero and the
    /// modulus, which are invalid Park-Miller states.
    #[must_use]
    pub const fn legacy_park_miller_state(self) -> u32 {
        const MAX_STATE: u64 = 2_147_483_646;
        ((self.state ^ self.stream.rotate_left(29)) % MAX_STATE + 1) as u32
    }
}

impl NativeSimulationKey {
    /// Derives stable, domain-separated seed words from this simulation key.
    ///
    /// This intentionally owns its mixer instead of relying on Rust's default
    /// hashing, whose output is not a stable replay format.
    #[must_use]
    pub fn derive_stream_seed(self) -> NativeStreamSeed {
        match self.replay.stream_version {
            NativeStreamVersion::V1 => self.derive_v1_stream_seed(),
            NativeStreamVersion::V2 => self.derive_v2_stream_seed(),
        }
    }

    #[must_use]
    pub(crate) fn stratum(self) -> Option<NativeStratum> {
        match self.replay.stream_version {
            NativeStreamVersion::V1 => None,
            NativeStreamVersion::V2 => Some(NativeStratum {
                index: self.simulation_index,
                offset: self.derive_v2_stratum_offset(),
                radix: 1,
            }),
        }
    }

    fn derive_v1_stream_seed(self) -> NativeStreamSeed {
        self.derive_stream_seed_for_domain(0x424d_4149_525f_4e31) // "BMAIR_N1"
    }

    fn derive_v2_stream_seed(self) -> NativeStreamSeed {
        self.derive_stream_seed_for_domain(0x424d_4149_525f_4e32) // "BMAIR_N2"
    }

    fn derive_stream_seed_for_domain(self, domain: u64) -> NativeStreamSeed {
        let mut accumulator = mix64(domain);
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

    fn derive_v2_stratum_offset(self) -> u64 {
        const DOMAIN: u64 = 0x5354_5241_5455_4d32; // "STRATUM2"
        let mut accumulator = mix64(DOMAIN);
        accumulator = fold(accumulator, self.replay.root_seed, ROOT_SALT);
        accumulator = fold(accumulator, self.replay.decision_index, DECISION_SALT);
        fold(accumulator, self.candidate_index, CANDIDATE_SALT)
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
