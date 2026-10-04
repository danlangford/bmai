// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use crate::Move;
use std::sync::OnceLock;

const DEFAULT_SIMS: usize = 500;
const MIN_SIMS: usize = 10;
const DEFAULT_MAX_BRANCH: usize = 5000;

/// The engine that plays Monte Carlo's simulated games. Playouts call it
/// directly instead of through the engine trait, as they make millions of moves.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Playout {
    Quick,
    Maximize,
    Random,
}

impl Playout {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Maximize => "maximize",
            Self::Random => "random",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub sims: usize,
    pub total_sims: [usize; 10],
    pub total_moves: [usize; 10],
    pub total_samples: [usize; 10],
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
    pub candidate: &'a Move,
    pub coordinate: EvaluationCoordinate,
}

impl Stats {
    pub fn on_full_simulation(&mut self) {
        self.sims += 1;
    }

    pub fn on_ply_action(&mut self, ply: usize, moves: usize, sims: usize) {
        if let Some(total) = self.total_sims.get_mut(ply) {
            *total += sims;
            self.total_moves[ply] += moves;
            self.total_samples[ply] += 1;
        }
    }
}

#[derive(Clone, Debug)]
pub struct Bmai3 {
    pub cull_moves: bool,
    pub playout: Playout,
    pub max_ply: usize,
    pub max_branch: usize,
    pub min_sims: usize,
    pub max_sims: usize,
    pub sims_per_check: usize,
    /// Each decision stops sampling once this passes and plays its best move
    /// so far; the simulation settings still cap the work.
    pub time_limit: Option<std::time::Duration>,
    pub min_best_score_threshold: f32,
    pub max_best_score_threshold: f32,
    pub last_best_score: f32,
    pub last_sims_run: usize,
    pub last_probability_win: f32,
    pub ply_decay: f32,
    pub stats: Stats,
}

impl Default for Bmai3 {
    fn default() -> Self {
        Self {
            cull_moves: true,
            playout: Playout::Quick,
            max_ply: 1,
            max_branch: DEFAULT_MAX_BRANCH,
            min_sims: MIN_SIMS,
            max_sims: DEFAULT_SIMS,
            sims_per_check: 10,
            time_limit: None,
            min_best_score_threshold: 0.25,
            max_best_score_threshold: 0.90,
            last_best_score: 0.0,
            last_sims_run: 0,
            last_probability_win: 0.0,
            ply_decay: 0.5,
            stats: Stats::default(),
        }
    }
}

impl Bmai3 {
    pub(crate) fn fire_candidate_limit(&self) -> usize {
        (self.max_branch / self.min_sims.max(1)).max(1)
    }

    pub fn compute_number_sims(&self, moves: usize, level: usize) -> usize {
        assert!(moves > 0);
        assert!(level > 0);
        let decay = self.ply_decay.powi(level as i32 - 1);
        let sims = (self.max_branch as f32 * decay / moves as f32) as usize;
        // C++ lets the minimum win over a smaller maximum; `clamp` would panic.
        let minimum = ((self.min_sims as f32 * decay + 0.99) as usize).max(1);
        if sims < minimum {
            return minimum;
        }
        let maximum = ((self.max_sims as f32 * decay + 0.99) as usize).max(1);
        sims.min(maximum)
    }

    /// The callback returns the mover's win probability.
    pub fn evaluate_moves<F>(&mut self, moves: Vec<Move>, level: usize, mut evaluate: F) -> Move
    where
        F: FnMut(&Move, EvaluationCoordinate) -> f32,
    {
        self.evaluate_moves_batched(moves, level, |requests| {
            requests
                .iter()
                .map(|request| evaluate(request.candidate, request.coordinate))
                .collect()
        })
    }

    /// Native mode parallelizes here, so results must match requests by position.
    pub fn evaluate_moves_batched<F>(
        &mut self,
        moves: Vec<Move>,
        level: usize,
        evaluate_batch: F,
    ) -> Move
    where
        F: FnMut(&[EvaluationRequest<'_>]) -> Vec<f32>,
    {
        self.evaluate_moves_batched_inner(moves, level, false, evaluate_batch)
    }

    /// Keeps sampling the survivor so reported odds use the whole budget.
    pub(crate) fn evaluate_moves_batched_to_completion<F>(
        &mut self,
        moves: Vec<Move>,
        level: usize,
        evaluate_batch: F,
    ) -> Move
    where
        F: FnMut(&[EvaluationRequest<'_>]) -> Vec<f32>,
    {
        self.evaluate_moves_batched_inner(moves, level, true, evaluate_batch)
    }

    fn evaluate_moves_batched_inner<F>(
        &mut self,
        moves: Vec<Move>,
        level: usize,
        complete_survivor: bool,
        mut evaluate_batch: F,
    ) -> Move
    where
        F: FnMut(&[EvaluationRequest<'_>]) -> Vec<f32>,
    {
        assert!(!moves.is_empty());
        let sims = self.compute_number_sims(moves.len(), level);
        self.stats.on_ply_action(level, moves.len(), sims);
        // Only the root is timed: a cut-short deeper search would hand its
        // parent a worse estimate than a finished one.
        let deadline = self
            .time_limit
            .filter(|_| level == 1)
            .map(|limit| std::time::Instant::now() + limit);
        if !self.cull_moves && deadline.is_none() {
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
                    self.stats.on_full_simulation();
                }
                if score > best_score {
                    best_score = score;
                    best = candidate.clone();
                }
            }
            self.last_best_score = best_score;
            self.last_sims_run = sims;
            self.last_probability_win = best_score / sims as f32;
            return best;
        }
        let mut state = ThinkState::new(moves, sims);
        static TRACE_AI: OnceLock<bool> = OnceLock::new();
        let trace = *TRACE_AI.get_or_init(|| std::env::var_os("BMAIR_TRACE_AI").is_some());

        while state.sims_run < state.sims {
            let check_sims = self
                .sims_per_check
                .min(state.sims.saturating_sub(state.sims_run));
            let batch_index = state.sims_run / self.sims_per_check;
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
                    self.stats.on_full_simulation();
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
            if state.sims_run >= state.sims
                || deadline.is_some_and(|deadline| std::time::Instant::now() >= deadline)
            {
                break;
            }
            let multiple_candidates_remain = !self.cull_moves || self.cull(&mut state);
            if !multiple_candidates_remain && !complete_survivor {
                break;
            }
        }

        self.last_best_score = state.best_score;
        self.last_sims_run = state.sims_run;
        self.last_probability_win = state.best_score / state.sims_run as f32;
        state.best_move
    }

    fn cull(&self, state: &mut ThinkState) -> bool {
        if state.movelist.len() == 1 {
            return false;
        }
        let progress = state.sims_run as f32 / state.sims as f32;
        let threshold = self.min_best_score_threshold
            + progress * (self.max_best_score_threshold - self.min_best_score_threshold);
        let mut delta_threshold = (1.0 - progress) * self.sims_per_check as f32 * 0.5;
        if state.best_score > 1.0 && delta_threshold >= state.best_score {
            delta_threshold = state.best_score;
        }

        let mut index = 0;
        while index < state.movelist.len() {
            let delta = state.best_score - state.score[index];
            let mut move_delta_threshold = delta_threshold;
            if state.movelist[index].action == crate::Action::Attack
                && state.movelist[index].attack == Some(crate::Attack::Trip)
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
struct ThinkState {
    sims: usize,
    sims_run: usize,
    score: Vec<f32>,
    candidate_index: Vec<usize>,
    best_score: f32,
    best_move: Move,
    movelist: Vec<Move>,
}

impl ThinkState {
    fn new(movelist: Vec<Move>, sims: usize) -> Self {
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
