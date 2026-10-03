// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

#![allow(non_camel_case_types, non_snake_case)]

use crate::BMC_Move;
use std::sync::OnceLock;

const BMD_DEFAULT_SIMS: usize = 500;
const BMD_MIN_SIMS: usize = 10;
const BMD_DEFAULT_MAX_BRANCH: usize = 5000;

#[derive(Clone, Copy, Debug)]
pub enum BME_ROLLOUT_POLICY {
    QAI,
    MAXIMIZE_OR_RANDOM(f32),
}

#[derive(Clone, Debug, Default)]
pub struct BMC_Stats {
    pub m_sims: usize,
    pub m_total_sims: [usize; 10],
    pub m_total_moves: [usize; 10],
    pub m_total_samples: [usize; 10],
}

/// Culling reorders candidates, so these keep their original indices.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvaluationCoordinate {
    pub candidate_index: usize,
    pub batch_index: usize,
    pub simulation_index: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct EvaluationRequest<'a> {
    pub candidate: &'a BMC_Move,
    pub coordinate: EvaluationCoordinate,
}

impl BMC_Stats {
    pub fn OnFullSimulation(&mut self) {
        self.m_sims += 1;
    }

    pub fn OnPlyAction(&mut self, ply: usize, moves: usize, sims: usize) {
        if let Some(total) = self.m_total_sims.get_mut(ply) {
            *total += sims;
            self.m_total_moves[ply] += moves;
            self.m_total_samples[ply] += 1;
        }
    }
}

#[derive(Clone, Debug)]
pub struct BMC_BMAI3 {
    pub m_cull_moves: bool,
    pub m_rollout_policy: BME_ROLLOUT_POLICY,
    pub m_max_ply: usize,
    pub m_max_branch: usize,
    pub m_min_sims: usize,
    pub m_max_sims: usize,
    pub m_sims_per_check: usize,
    pub m_min_best_score_threshold: f32,
    pub m_max_best_score_threshold: f32,
    pub m_last_best_score: f32,
    pub m_last_sims_run: usize,
    pub m_last_probability_win: f32,
    pub m_ply_decay: f32,
    pub m_stats: BMC_Stats,
}

impl Default for BMC_BMAI3 {
    fn default() -> Self {
        Self {
            m_cull_moves: true,
            m_rollout_policy: BME_ROLLOUT_POLICY::QAI,
            m_max_ply: 1,
            m_max_branch: BMD_DEFAULT_MAX_BRANCH,
            m_min_sims: BMD_MIN_SIMS,
            m_max_sims: BMD_DEFAULT_SIMS,
            m_sims_per_check: 10,
            m_min_best_score_threshold: 0.25,
            m_max_best_score_threshold: 0.90,
            m_last_best_score: 0.0,
            m_last_sims_run: 0,
            m_last_probability_win: 0.0,
            m_ply_decay: 0.5,
            m_stats: BMC_Stats::default(),
        }
    }
}

impl BMC_BMAI3 {
    pub(crate) fn FireCandidateLimit(&self) -> usize {
        (self.m_max_branch / self.m_min_sims.max(1)).max(1)
    }

    pub fn ComputeNumberSims(&self, moves: usize, level: usize) -> usize {
        assert!(moves > 0);
        assert!(level > 0);
        let decay = self.m_ply_decay.powi(level as i32 - 1);
        let sims = (self.m_max_branch as f32 * decay / moves as f32) as usize;
        // C++ lets the minimum win over a smaller maximum; `clamp` would panic.
        let minimum = ((self.m_min_sims as f32 * decay + 0.99) as usize).max(1);
        if sims < minimum {
            return minimum;
        }
        let maximum = ((self.m_max_sims as f32 * decay + 0.99) as usize).max(1);
        sims.min(maximum)
    }

    /// The callback returns the mover's win probability.
    pub fn EvaluateMoves<F>(
        &mut self,
        moves: Vec<BMC_Move>,
        level: usize,
        mut evaluate: F,
    ) -> BMC_Move
    where
        F: FnMut(&BMC_Move, EvaluationCoordinate) -> f32,
    {
        self.EvaluateMovesBatched(moves, level, |requests| {
            requests
                .iter()
                .map(|request| evaluate(request.candidate, request.coordinate))
                .collect()
        })
    }

    /// Native mode parallelizes here, so results must match requests by position.
    pub fn EvaluateMovesBatched<F>(
        &mut self,
        moves: Vec<BMC_Move>,
        level: usize,
        evaluate_batch: F,
    ) -> BMC_Move
    where
        F: FnMut(&[EvaluationRequest<'_>]) -> Vec<f32>,
    {
        self.EvaluateMovesBatchedInner(moves, level, false, evaluate_batch)
    }

    /// Keeps sampling the survivor so reported odds use the whole budget.
    pub(crate) fn EvaluateMovesBatchedToCompletion<F>(
        &mut self,
        moves: Vec<BMC_Move>,
        level: usize,
        evaluate_batch: F,
    ) -> BMC_Move
    where
        F: FnMut(&[EvaluationRequest<'_>]) -> Vec<f32>,
    {
        self.EvaluateMovesBatchedInner(moves, level, true, evaluate_batch)
    }

    fn EvaluateMovesBatchedInner<F>(
        &mut self,
        moves: Vec<BMC_Move>,
        level: usize,
        complete_survivor: bool,
        mut evaluate_batch: F,
    ) -> BMC_Move
    where
        F: FnMut(&[EvaluationRequest<'_>]) -> Vec<f32>,
    {
        assert!(!moves.is_empty());
        let sims = self.ComputeNumberSims(moves.len(), level);
        self.m_stats.OnPlyAction(level, moves.len(), sims);
        if !self.m_cull_moves {
            let mut best = moves[0].clone();
            let mut best_score = -1.0_f32;
            let requests = moves
                .iter()
                .enumerate()
                .flat_map(|(candidate_index, candidate)| {
                    (0..sims).map(move |simulation| EvaluationRequest {
                        candidate,
                        coordinate: EvaluationCoordinate {
                            candidate_index,
                            batch_index: 0,
                            simulation_index: simulation,
                        },
                    })
                })
                .collect::<Vec<_>>();
            let results = evaluate_batch(&requests);
            assert_eq!(results.len(), requests.len());
            for (candidate_index, candidate) in moves.iter().enumerate() {
                let start = candidate_index * sims;
                let score = results[start..start + sims].iter().sum();
                for _ in 0..sims {
                    self.m_stats.OnFullSimulation();
                }
                if score > best_score {
                    best_score = score;
                    best = candidate.clone();
                }
            }
            self.m_last_best_score = best_score;
            self.m_last_sims_run = sims;
            self.m_last_probability_win = best_score / sims as f32;
            return best;
        }
        let mut state = BMC_ThinkState::new(moves, sims);
        static TRACE_AI: OnceLock<bool> = OnceLock::new();
        let trace = *TRACE_AI.get_or_init(|| std::env::var_os("BMAIR_TRACE_AI").is_some());

        while state.sims_run < state.sims {
            let check_sims = self
                .m_sims_per_check
                .min(state.sims.saturating_sub(state.sims_run));
            let batch_index = state.sims_run / self.m_sims_per_check;
            let simulation_start = state.sims_run;
            let candidate_indices = &state.candidate_index;
            let requests = state
                .movelist
                .iter()
                .enumerate()
                .flat_map(|(index, candidate)| {
                    let candidate_index = candidate_indices[index];
                    (0..check_sims).map(move |simulation| EvaluationRequest {
                        candidate,
                        coordinate: EvaluationCoordinate {
                            candidate_index,
                            batch_index,
                            simulation_index: simulation_start + simulation,
                        },
                    })
                })
                .collect::<Vec<_>>();
            let results = evaluate_batch(&requests);
            assert_eq!(results.len(), requests.len());
            for index in 0..state.movelist.len() {
                let start = index * check_sims;
                for score in &results[start..start + check_sims] {
                    state.score[index] += score;
                    self.m_stats.OnFullSimulation();
                }
                if state.score[index] > state.best_score {
                    state.best_score = state.score[index];
                    state.best_move = state.movelist[index].clone();
                }
                if trace {
                    eprintln!(
                        "l{level} m{index} sims {check_sims} score {:.6} {:?}",
                        state.score[index], state.movelist[index]
                    );
                }
            }
            state.sims_run += check_sims;
            if state.sims_run >= state.sims {
                break;
            }
            let multiple_candidates_remain = self.CullMoves(&mut state);
            if !multiple_candidates_remain && !complete_survivor {
                break;
            }
        }

        self.m_last_best_score = state.best_score;
        self.m_last_sims_run = state.sims_run;
        self.m_last_probability_win = state.best_score / state.sims_run as f32;
        state.best_move
    }

    fn CullMoves(&self, state: &mut BMC_ThinkState) -> bool {
        if state.movelist.len() == 1 {
            return false;
        }
        let progress = state.sims_run as f32 / state.sims as f32;
        let threshold = self.m_min_best_score_threshold
            + progress * (self.m_max_best_score_threshold - self.m_min_best_score_threshold);
        let mut delta_threshold = (1.0 - progress) * self.m_sims_per_check as f32 * 0.5;
        if state.best_score > 1.0 && delta_threshold >= state.best_score {
            delta_threshold = state.best_score;
        }

        let mut index = 0;
        while index < state.movelist.len() {
            let delta = state.best_score - state.score[index];
            let mut move_delta_threshold = delta_threshold;
            if state.movelist[index].m_action == crate::BME_ACTION::ATTACK
                && state.movelist[index].m_attack == Some(crate::BME_ATTACK::TRIP)
            {
                move_delta_threshold *= 0.5;
            }
            let cannot_catch_up = delta >= state.sims.saturating_sub(state.sims_run) as f32;
            let below_threshold =
                state.score[index] < state.best_score * threshold && delta >= move_delta_threshold;
            if cannot_catch_up || below_threshold {
                state.movelist.swap_remove(index);
                state.score.swap_remove(index);
                state.candidate_index.swap_remove(index);
            } else {
                index += 1;
            }
        }
        state.movelist.len() > 1
    }
}

#[derive(Clone, Debug)]
struct BMC_ThinkState {
    sims: usize,
    sims_run: usize,
    score: Vec<f32>,
    candidate_index: Vec<usize>,
    best_score: f32,
    best_move: BMC_Move,
    movelist: Vec<BMC_Move>,
}

impl BMC_ThinkState {
    fn new(movelist: Vec<BMC_Move>, sims: usize) -> Self {
        let best_move = movelist[0].clone();
        Self {
            sims,
            sims_run: 0,
            score: vec![0.0; movelist.len()],
            candidate_index: (0..movelist.len()).collect(),
            best_score: -1.0,
            best_move,
            movelist,
        }
    }
}

#[cfg(test)]
mod tests;
