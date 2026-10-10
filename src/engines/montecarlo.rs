// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{Choice, DecisionContext, Engine, SearchSummary, Setting};
use crate::Bmai3;
use crate::game::{Game, Move};
use crate::search::{
    ChanceMove, FocusMove, SwingMove, select_chance_action, select_focus_action,
    select_native_bmai_action, select_native_bmai_auxiliary_action,
    select_native_bmai_reserve_action, select_swing_action,
};

/// Estimates each candidate's win rate by playing simulated games.
#[derive(Clone, Debug, Default)]
pub(crate) struct MonteCarlo {
    pub(crate) search: Bmai3,
}

impl MonteCarlo {
    pub(crate) fn new(search: Bmai3) -> Self {
        Self { search }
    }
}

impl Engine for MonteCarlo {
    fn name(&self) -> &'static str {
        "montecarlo"
    }

    fn clone_box(&self) -> Box<dyn Engine> {
        Box::new(self.clone())
    }

    fn set(&mut self, setting: Setting) -> Result<(), String> {
        match setting {
            // Ply 0 always searched exactly as ply 1, so it only misled.
            Setting::Ply(0) => return Err("montecarlo ply must be at least 1".into()),
            Setting::Ply(ply) => self.search.max_ply = ply,
            Setting::MaxSims(sims) => self.search.max_sims = sims,
            Setting::MinSims(sims) => self.search.min_sims = sims,
            Setting::MaxBranch(branch) => self.search.max_branch = branch,
            Setting::Cull(cull) => self.search.cull_moves = cull,
            Setting::Playout(playout) => self.search.playout = playout,
            Setting::Endgame(dice) => self.search.endgame_dice = dice,
        }
        Ok(())
    }

    fn montecarlo(&self) -> Option<&Bmai3> {
        Some(&self.search)
    }

    fn swing(
        &self,
        game: &Game,
        player: usize,
        context: &mut DecisionContext<'_, '_>,
    ) -> SwingMove {
        let native = context.native();
        select_swing_action(game, player, context.rng, &self.search, 1, Some(native)).0
    }

    fn chance(
        &self,
        game: &Game,
        player: usize,
        initiative: usize,
        context: &mut DecisionContext<'_, '_>,
    ) -> ChanceMove {
        let native = context.native();
        select_chance_action(
            game,
            player,
            context.rng,
            &self.search,
            1,
            initiative,
            Some(native),
        )
        .0
    }

    fn focus(
        &self,
        game: &Game,
        player: usize,
        initiative: usize,
        context: &mut DecisionContext<'_, '_>,
    ) -> FocusMove {
        let native = context.native();
        select_focus_action(
            game,
            player,
            context.rng,
            &self.search,
            1,
            initiative,
            Some(native),
        )
        .0
    }

    fn attack(&self, game: &Game, context: &mut DecisionContext<'_, '_>) -> Choice<Move> {
        if let Some((choice, probability)) = solve_endgame(game, &self.search) {
            if std::env::var_os("BMAIR_TRACE_ENDGAME").is_some() {
                let sampled = select_native_bmai_action(game, context.native(), &self.search);
                trace_endgame(game, &self.search, &sampled.best_move, probability);
            }
            return Choice {
                search: Some(SearchSummary {
                    score: probability as f32,
                    win_probability: probability as f32,
                    simulations: 0,
                }),
                choice,
            };
        }
        let result = select_native_bmai_action(game, context.native(), &self.search);
        Choice {
            search: Some(SearchSummary {
                score: result.best_score,
                win_probability: result.win_probability(),
                simulations: result.sims_run,
            }),
            choice: result.best_move,
        }
    }

    fn reserve(&self, game: &Game, context: &mut DecisionContext<'_, '_>) -> Option<usize> {
        select_native_bmai_reserve_action(game, context.native(), &self.search)
    }

    fn auxiliary(
        &self,
        game: &Game,
        context: &mut DecisionContext<'_, '_>,
    ) -> Choice<Option<usize>> {
        let result = select_native_bmai_auxiliary_action(game, context.native(), &self.search);
        Choice {
            choice: result.die,
            search: Some(SearchSummary {
                score: result.score,
                win_probability: result.probability(),
                simulations: result.simulations,
            }),
        }
    }
}

/// Big enough for every position measured so far; larger ones go to Monte
/// Carlo rather than slow the move down.
const ENDGAME_NODE_LIMIT: usize = 200_000;

fn solve_endgame(game: &Game, search: &Bmai3) -> Option<(Move, f64)> {
    if search.endgame_dice == 0 || crate::search::dice_in_play(game) > search.endgame_dice {
        return None;
    }
    crate::search::Solver::new(search.fire_candidate_limit(), ENDGAME_NODE_LIMIT).best_move(game)
}

/// Logs how far Monte Carlo's choice falls short of the exact best move.
fn trace_endgame(game: &Game, search: &Bmai3, sampled: &Move, best: f64) {
    let mut solver = crate::search::Solver::new(search.fire_candidate_limit(), ENDGAME_NODE_LIMIT);
    let Some(values) = solver.move_values(game) else {
        return;
    };
    let same = |action: &Move| {
        action.action == sampled.action
            && action.attack == sampled.attack
            && action.attackers == sampled.attackers
            && action.targets == sampled.targets
            && action.turbo_option == sampled.turbo_option
            && action.fire == sampled.fire
    };
    // Surrendering loses the round, so it is worth nothing.
    let sampled_value = if sampled.action == crate::Action::Surrender {
        Some(0.0)
    } else {
        values
            .iter()
            .find(|(action, _)| same(action))
            .map(|(_, value)| *value)
    };
    eprintln!(
        "ENDGAME dice={} moves={} best={best:.6} montecarlo={}",
        crate::search::dice_in_play(game),
        values.len(),
        sampled_value.map_or("unknown".into(), |value| format!("{value:.6}"))
    );
}
