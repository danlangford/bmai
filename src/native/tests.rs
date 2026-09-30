// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

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
