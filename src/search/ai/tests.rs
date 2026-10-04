// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;
use crate::{Action, Attack};

fn test_move(score: f32) -> Move {
    Move {
        action: Action::Attack,
        attack: Some(Attack::Power),
        attackers: vec![0].into(),
        targets: vec![0].into(),
        score,
        turbo_option: -1,
        fire: crate::game::FireAdjustment::default(),
    }
}

#[test]
fn simulation_count_matches_cpp_decay_and_clamps() {
    let ai = Bmai3::default();
    assert_eq!(ai.fire_candidate_limit(), 500);
    assert_eq!(ai.compute_number_sims(1, 1), 500);
    assert_eq!(ai.compute_number_sims(12, 1), 416);
    assert_eq!(ai.compute_number_sims(12, 2), 208);
    assert_eq!(ai.compute_number_sims(1000, 1), 10);
}

#[test]
fn simulation_minimum_wins_when_it_exceeds_the_maximum_like_cpp() {
    let ai = Bmai3 {
        max_sims: 5,
        ..Default::default()
    };
    assert_eq!(ai.compute_number_sims(1000, 1), 10);
    assert_eq!(ai.compute_number_sims(1, 1), 5);
}

#[test]
fn evaluation_selects_the_highest_probability_move() {
    let mut ai = Bmai3 {
        max_sims: 20,
        max_branch: 40,
        ..Default::default()
    };
    let selected = ai.evaluate_moves(vec![test_move(0.2), test_move(0.8)], 1, |m, _| m.score);
    assert_eq!(selected.score, 0.8);
    assert!(ai.last_sims_run > 0);
    assert!((ai.last_best_score / ai.last_sims_run as f32 - 0.8).abs() < f32::EPSILON * 2.0);
    assert!((ai.last_probability_win - 0.8).abs() < f32::EPSILON * 2.0);
}

#[test]
fn legacy_bmai_evaluates_each_move_to_completion_without_culling() {
    let mut ai = Bmai3 {
        cull_moves: false,
        min_sims: 20,
        max_sims: 20,
        max_branch: 100,
        ..Default::default()
    };
    let mut order = Vec::new();
    let selected = ai.evaluate_moves(vec![test_move(0.2), test_move(0.8)], 1, |m, coordinate| {
        order.push((m.score, coordinate));
        m.score
    });
    assert_eq!(selected.score, 0.8);
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
    let mut ai = Bmai3 {
        min_sims: 20,
        max_sims: 20,
        max_branch: 100,
        sims_per_check: 10,
        ..Default::default()
    };
    let mut coordinates = Vec::new();
    ai.evaluate_moves(
        vec![test_move(1.0), test_move(0.8)],
        1,
        |candidate, coordinate| {
            coordinates.push((candidate.score, coordinate));
            candidate.score
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
    let mut ai = Bmai3 {
        max_sims: 100,
        max_branch: 200,
        min_sims: 5,
        ..Default::default()
    };
    let mut evaluations = [0usize; 2];
    let selected = ai.evaluate_moves_batched_to_completion(
        vec![test_move(1.0), test_move(0.0)],
        1,
        |requests| {
            requests
                .iter()
                .map(|request| {
                    evaluations[request.coordinate.candidate_index] += 1;
                    request.candidate.score
                })
                .collect()
        },
    );

    assert_eq!(selected.score, 1.0);
    assert_eq!(ai.last_sims_run, 100);
    assert_eq!(ai.last_probability_win, 1.0);
    assert_eq!(evaluations[0], 100);
    assert!(evaluations[1] < evaluations[0]);
}

fn expired() -> Bmai3 {
    Bmai3 {
        time_limit: Some(std::time::Duration::ZERO),
        ..Default::default()
    }
    .timed()
}

#[test]
fn an_expired_deadline_gives_every_candidate_exactly_one_round_at_any_depth() {
    for (cull_moves, level) in [(true, 1), (false, 1), (true, 2), (false, 3)] {
        let mut ai = Bmai3 {
            cull_moves,
            ..expired()
        };
        let mut calls = [0usize; 3];
        let moves = vec![test_move(1.0), test_move(2.0), test_move(3.0)];
        ai.evaluate_moves(moves, level, |_, coordinate| {
            calls[coordinate.candidate_index] += 1;
            if coordinate.candidate_index == 2 {
                1.0
            } else {
                0.0
            }
        });
        let round = ai.sims_per_check;
        assert_eq!(calls, [round; 3], "cull {cull_moves} level {level}");
        assert_eq!(ai.last_sims_run, round);
        assert_eq!(ai.last_best_score, round as f32);
        assert_eq!(ai.last_probability_win, 1.0);
    }
}

#[test]
fn without_a_deadline_the_whole_budget_runs() {
    let mut ai = Bmai3::default().timed();
    let mut calls = 0usize;
    ai.evaluate_moves(vec![test_move(1.0), test_move(2.0)], 1, |_, _| {
        calls += 1;
        0.5
    });
    assert_eq!(calls, 2 * ai.compute_number_sims(2, 1));
}

#[test]
fn past_the_deadline_the_search_stops_looking_ahead() {
    let ai = Bmai3 {
        max_ply: 3,
        ..expired()
    };
    assert!(ai.stops_looking_ahead(1));
    let untimed = Bmai3 {
        max_ply: 3,
        ..Default::default()
    };
    assert!(!untimed.stops_looking_ahead(1));
    assert!(untimed.stops_looking_ahead(3));
}
