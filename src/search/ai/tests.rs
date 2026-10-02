// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;
use crate::{BME_ACTION, BME_ATTACK};

fn test_move(score: f32) -> BMC_Move {
    BMC_Move {
        m_action: BME_ACTION::ATTACK,
        m_attack: Some(BME_ATTACK::POWER),
        m_attackers: vec![0].into(),
        m_targets: vec![0].into(),
        m_score: score,
        m_turbo_option: -1,
        m_fire: crate::game::BMC_FireAdjustment::default(),
    }
}

#[test]
fn simulation_count_matches_cpp_decay_and_clamps() {
    let ai = BMC_BMAI3::default();
    assert_eq!(ai.FireCandidateLimit(), 500);
    assert_eq!(ai.ComputeNumberSims(1, 1), 500);
    assert_eq!(ai.ComputeNumberSims(12, 1), 416);
    assert_eq!(ai.ComputeNumberSims(12, 2), 208);
    assert_eq!(ai.ComputeNumberSims(1000, 1), 10);
}

#[test]
fn simulation_minimum_wins_when_it_exceeds_the_maximum_like_cpp() {
    let ai = BMC_BMAI3 {
        m_max_sims: 5,
        ..Default::default()
    };
    // The default minimum is 10. C++ checks it first, so a crowded branch
    // gets the minimum; otherwise the smaller maximum caps the count.
    assert_eq!(ai.ComputeNumberSims(1000, 1), 10);
    assert_eq!(ai.ComputeNumberSims(1, 1), 5);
}

#[test]
fn evaluation_selects_the_highest_probability_move() {
    let mut ai = BMC_BMAI3 {
        m_max_sims: 20,
        m_max_branch: 40,
        ..Default::default()
    };
    let selected = ai.EvaluateMoves(vec![test_move(0.2), test_move(0.8)], 1, |m, _| m.m_score);
    assert_eq!(selected.m_score, 0.8);
    assert!(ai.m_last_sims_run > 0);
    assert!((ai.m_last_best_score / ai.m_last_sims_run as f32 - 0.8).abs() < f32::EPSILON * 2.0);
    assert!((ai.m_last_probability_win - 0.8).abs() < f32::EPSILON * 2.0);
}

#[test]
fn legacy_bmai_evaluates_each_move_to_completion_without_culling() {
    let mut ai = BMC_BMAI3 {
        m_cull_moves: false,
        m_min_sims: 20,
        m_max_sims: 20,
        m_max_branch: 100,
        ..Default::default()
    };
    let mut order = Vec::new();
    let selected = ai.EvaluateMoves(vec![test_move(0.2), test_move(0.8)], 1, |m, coordinate| {
        order.push((m.m_score, coordinate));
        m.m_score
    });
    assert_eq!(selected.m_score, 0.8);
    assert_eq!(
        order[..20],
        (0..20)
            .map(|simulation_index| (
                0.2,
                EvaluationCoordinate {
                    candidate_index: 0,
                    batch_index: 0,
                    simulation_index,
                }
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        order[20..],
        (0..20)
            .map(|simulation_index| (
                0.8,
                EvaluationCoordinate {
                    candidate_index: 1,
                    batch_index: 0,
                    simulation_index,
                }
            ))
            .collect::<Vec<_>>()
    );
}

#[test]
fn culled_evaluations_keep_canonical_candidate_and_simulation_indices() {
    let mut ai = BMC_BMAI3 {
        m_min_sims: 20,
        m_max_sims: 20,
        m_max_branch: 100,
        m_sims_per_check: 10,
        ..Default::default()
    };
    let mut coordinates = Vec::new();
    ai.EvaluateMoves(
        vec![test_move(1.0), test_move(0.8)],
        1,
        |candidate, coordinate| {
            coordinates.push((candidate.m_score, coordinate));
            candidate.m_score
        },
    );

    assert!(coordinates.contains(&(
        1.0,
        EvaluationCoordinate {
            candidate_index: 0,
            batch_index: 1,
            simulation_index: 10,
        }
    )));
    assert!(coordinates.contains(&(
        0.8,
        EvaluationCoordinate {
            candidate_index: 1,
            batch_index: 1,
            simulation_index: 10,
        }
    )));
}

#[test]
fn native_probability_evaluation_completes_the_surviving_candidate() {
    let mut ai = BMC_BMAI3 {
        m_max_sims: 100,
        m_max_branch: 200,
        m_min_sims: 5,
        ..Default::default()
    };
    let mut evaluations = [0usize; 2];
    let selected =
        ai.EvaluateMovesBatchedToCompletion(vec![test_move(1.0), test_move(0.0)], 1, |requests| {
            requests
                .iter()
                .map(|request| {
                    evaluations[request.coordinate.candidate_index] += 1;
                    request.candidate.m_score
                })
                .collect()
        });

    assert_eq!(selected.m_score, 1.0);
    assert_eq!(ai.m_last_sims_run, 100);
    assert_eq!(ai.m_last_probability_win, 1.0);
    assert_eq!(evaluations[0], 100);
    assert!(evaluations[1] < evaluations[0]);
}
