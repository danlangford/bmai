// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! Versioned deterministic identities for opt-in native-mode simulations.
//!
//! This module does not alter the legacy RNG or search. It defines the stable
//! replay boundary that native search can use to give every simulation an
//! independent random stream, regardless of worker count or scheduling order.

use std::cell::Cell;

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

thread_local! {
    static NATIVE_WORKER_ACTIVE: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn native_worker_active() -> bool {
    NATIVE_WORKER_ACTIVE.get()
}

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

fn mix64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

/// Evaluates independent tasks on scoped threads and restores input order.
///
/// Static round-robin assignment keeps the implementation dependency-free.
/// Result ordering depends only on task position, never worker completion.
pub(crate) fn ordered_parallel_map<T, R, F>(tasks: Vec<T>, workers: usize, evaluate: F) -> Vec<R>
where
    T: Send,
    R: Send,
    F: Fn(T) -> R + Sync,
{
    let worker_count = workers.max(1).min(tasks.len().max(1));
    if worker_count == 1 {
        return tasks.into_iter().map(evaluate).collect();
    }

    let mut assignments = (0..worker_count)
        .map(|_| Vec::new())
        .collect::<Vec<Vec<(usize, T)>>>();
    for (index, task) in tasks.into_iter().enumerate() {
        assignments[index % worker_count].push((index, task));
    }

    let mut completed = std::thread::scope(|scope| {
        let handles = assignments
            .into_iter()
            .map(|assignment| {
                let evaluate = &evaluate;
                scope.spawn(move || {
                    NATIVE_WORKER_ACTIVE.set(true);
                    let completed = assignment
                        .into_iter()
                        .map(|(index, task)| (index, evaluate(task)))
                        .collect::<Vec<_>>();
                    NATIVE_WORKER_ACTIVE.set(false);
                    completed
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("native worker panicked"))
            .collect::<Vec<_>>()
    });
    completed.sort_unstable_by_key(|(index, _)| *index);
    completed.into_iter().map(|(_, result)| result).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE_KEY: NativeSimulationKey = NativeSimulationKey {
        replay: NativeReplayKey {
            stream_version: NativeStreamVersion::V1,
            root_seed: 0x0123_4567_89ab_cdef,
            decision_index: 42,
        },
        candidate_index: 7,
        batch_index: 3,
        simulation_index: 999,
    };

    #[test]
    fn stream_partition_identifier_is_versioned() {
        assert_eq!(
            NativeStreamVersion::V1.partition_id(),
            "bmair-native-stream-v1"
        );
        assert_eq!(
            NativeStreamVersion::CURRENT.partition_id(),
            "bmair-native-stream-v2"
        );
        assert!(!NativeStreamVersion::V1.completes_probability_sample());
        assert!(NativeStreamVersion::CURRENT.completes_probability_sample());
    }

    #[test]
    fn stream_seed_has_a_stable_known_answer() {
        assert_eq!(
            EXAMPLE_KEY.derive_stream_seed(),
            NativeStreamSeed {
                state: 13_647_275_757_561_345_854,
                stream: 4_338_030_216_732_356_548,
            }
        );
    }

    #[test]
    fn current_stream_seed_has_a_stable_known_answer() {
        let key = NativeSimulationKey {
            replay: NativeReplayKey {
                stream_version: NativeStreamVersion::CURRENT,
                ..EXAMPLE_KEY.replay
            },
            ..EXAMPLE_KEY
        };
        assert_eq!(
            key.derive_stream_seed(),
            NativeStreamSeed {
                state: 7_713_654_091_583_939_669,
                stream: 6_360_158_390_379_539_643,
            }
        );
        assert_eq!(
            key.stratum(),
            Some(NativeStratum {
                index: 999,
                offset: 13_862_872_699_400_720_889,
                radix: 1,
            })
        );
    }

    #[test]
    fn every_task_coordinate_partitions_the_stream() {
        let baseline = EXAMPLE_KEY.derive_stream_seed();
        let variants = [
            NativeSimulationKey {
                replay: NativeReplayKey {
                    root_seed: EXAMPLE_KEY.replay.root_seed + 1,
                    ..EXAMPLE_KEY.replay
                },
                ..EXAMPLE_KEY
            },
            NativeSimulationKey {
                replay: NativeReplayKey {
                    decision_index: EXAMPLE_KEY.replay.decision_index + 1,
                    ..EXAMPLE_KEY.replay
                },
                ..EXAMPLE_KEY
            },
            NativeSimulationKey {
                candidate_index: EXAMPLE_KEY.candidate_index + 1,
                ..EXAMPLE_KEY
            },
            NativeSimulationKey {
                batch_index: EXAMPLE_KEY.batch_index + 1,
                ..EXAMPLE_KEY
            },
            NativeSimulationKey {
                simulation_index: EXAMPLE_KEY.simulation_index + 1,
                ..EXAMPLE_KEY
            },
        ];

        for variant in variants {
            assert_ne!(variant.derive_stream_seed(), baseline);
        }
    }

    #[test]
    fn v2_strata_advance_by_simulation_but_keep_a_candidate_offset() {
        let key = NativeSimulationKey {
            replay: NativeReplayKey {
                stream_version: NativeStreamVersion::V2,
                ..EXAMPLE_KEY.replay
            },
            ..EXAMPLE_KEY
        };
        let baseline = key.stratum().unwrap();
        let next_simulation = NativeSimulationKey {
            simulation_index: key.simulation_index + 1,
            ..key
        }
        .stratum()
        .unwrap();
        let next_batch = NativeSimulationKey {
            batch_index: key.batch_index + 1,
            ..key
        }
        .stratum()
        .unwrap();

        assert_eq!(next_simulation.index, baseline.index + 1);
        assert_eq!(next_simulation.offset, baseline.offset);
        assert_eq!(next_simulation.radix, 1);
        assert_eq!(next_batch.index, baseline.index);
        assert_eq!(next_batch.offset, baseline.offset);
        assert_eq!(next_batch.radix, 1);
        assert!(
            NativeSimulationKey {
                replay: NativeReplayKey {
                    stream_version: NativeStreamVersion::V1,
                    ..key.replay
                },
                ..key
            }
            .stratum()
            .is_none()
        );
    }

    #[test]
    fn legacy_state_uses_both_words_and_is_in_range() {
        let seed = EXAMPLE_KEY.derive_stream_seed();
        assert_eq!(seed.legacy_park_miller_state(), 2_042_917_870);
        assert!((1..2_147_483_647).contains(&seed.legacy_park_miller_state()));
        assert_ne!(
            seed.legacy_park_miller_state(),
            NativeStreamSeed {
                stream: seed.stream + 1,
                ..seed
            }
            .legacy_park_miller_state()
        );
    }

    #[test]
    fn ordered_parallel_results_are_worker_count_independent() {
        let tasks = (0u64..257).collect::<Vec<_>>();
        let expected = ordered_parallel_map(tasks.clone(), 1, expensive_test_mapping);
        for workers in [2, 3, 8, 512] {
            assert_eq!(
                ordered_parallel_map(tasks.clone(), workers, expensive_test_mapping),
                expected
            );
        }
    }

    #[test]
    fn worker_identity_is_scoped_to_parallel_evaluation() {
        assert!(!native_worker_active());
        assert_eq!(
            ordered_parallel_map(vec![1, 2], 2, |_| native_worker_active()),
            [true, true]
        );
        assert!(!native_worker_active());
    }

    fn expensive_test_mapping(value: u64) -> u64 {
        for _ in 0..(17 - value % 17) {
            std::hint::spin_loop();
            std::thread::yield_now();
        }
        mix64(value)
    }
}
