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
    /// False selects the original BMC_BMAI fixed-simulation evaluator; true
    /// selects BMC_BMAI3's batched evaluator and culling.
    pub m_cull_moves: bool,
    pub m_rollout_policy: BME_ROLLOUT_POLICY,
    pub m_max_ply: usize,
    pub m_max_branch: usize,
    pub m_min_sims: usize,
    pub m_max_sims: usize,
    pub m_sims_per_check: usize,
    pub m_min_best_score_threshold: f32,
    pub m_max_best_score_threshold: f32,
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
            m_last_probability_win: 0.0,
            m_ply_decay: 0.5,
            m_stats: BMC_Stats::default(),
        }
    }
}

impl BMC_BMAI3 {
    pub fn ComputeNumberSims(&self, moves: usize, level: usize) -> usize {
        assert!(moves > 0);
        assert!(level > 0);
        let decay = self.m_ply_decay.powi(level as i32 - 1);
        let sims = (self.m_max_branch as f32 * decay / moves as f32) as usize;
        let minimum = (self.m_min_sims as f32 * decay + 0.99) as usize;
        let maximum = (self.m_max_sims as f32 * decay + 0.99) as usize;
        sims.clamp(minimum.max(1), maximum.max(1))
    }

    /// BMAI3's batched evaluation and culling loop. The callback returns the
    /// probability that the player whose move is being evaluated wins.
    pub fn EvaluateMoves<F>(
        &mut self,
        moves: Vec<BMC_Move>,
        level: usize,
        mut evaluate: F,
    ) -> BMC_Move
    where
        F: FnMut(&BMC_Move, usize) -> f32,
    {
        assert!(!moves.is_empty());
        let sims = self.ComputeNumberSims(moves.len(), level);
        self.m_stats.OnPlyAction(level, moves.len(), sims);
        if !self.m_cull_moves {
            let mut best = moves[0].clone();
            let mut best_score = -1.0_f32;
            for candidate in &moves {
                let mut score = 0.0;
                for simulation in 0..sims {
                    score += evaluate(candidate, simulation);
                    self.m_stats.OnFullSimulation();
                }
                if score > best_score {
                    best_score = score;
                    best = candidate.clone();
                }
            }
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
            for index in 0..state.movelist.len() {
                for simulation in 0..check_sims {
                    state.score[index] += evaluate(&state.movelist[index], simulation);
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
            if state.sims_run >= state.sims || !self.CullMoves(&mut state) {
                break;
            }
        }

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
            best_score: -1.0,
            best_move,
            movelist,
        }
    }
}

#[cfg(test)]
mod tests {
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
        }
    }

    #[test]
    fn simulation_count_matches_cpp_decay_and_clamps() {
        let ai = BMC_BMAI3::default();
        assert_eq!(ai.ComputeNumberSims(1, 1), 500);
        assert_eq!(ai.ComputeNumberSims(12, 1), 416);
        assert_eq!(ai.ComputeNumberSims(12, 2), 208);
        assert_eq!(ai.ComputeNumberSims(1000, 1), 10);
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
        let selected =
            ai.EvaluateMoves(vec![test_move(0.2), test_move(0.8)], 1, |m, simulation| {
                order.push((m.m_score, simulation));
                m.m_score
            });
        assert_eq!(selected.m_score, 0.8);
        assert_eq!(order[..20], (0..20).map(|i| (0.2, i)).collect::<Vec<_>>());
        assert_eq!(order[20..], (0..20).map(|i| (0.8, i)).collect::<Vec<_>>());
    }
}
