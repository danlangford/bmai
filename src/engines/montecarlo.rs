// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{Choice, DecisionContext, Engine, SearchSummary, Setting};
use crate::Bmai3;
use crate::game::{Game, Move};
use crate::search::{
    ChanceMove, FocusMove, SwingMove, select_bmai_action_with_stats, select_bmai_auxiliary_action,
    select_bmai_reserve_action, select_chance_action, select_focus_action,
    select_native_bmai_action_with_stats, select_native_bmai_auxiliary_action,
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
            Setting::QuickTweaks(tweaks) => self.search.quick_tweaks = tweaks,
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
        select_swing_action(game, player, context.rng, &self.search, 1, native).0
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
            native,
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
            native,
        )
        .0
    }

    fn attack(&self, game: &Game, context: &mut DecisionContext<'_, '_>) -> Choice<Move> {
        let result = match context.native() {
            Some(native) => select_native_bmai_action_with_stats(
                game,
                native.algorithm,
                native.replay,
                native.workers,
                &self.search,
            ),
            None => select_bmai_action_with_stats(game, context.rng, &self.search),
        };
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
        match context.native() {
            Some(native) => select_native_bmai_reserve_action(
                game,
                native.algorithm,
                native.replay,
                native.workers,
                &self.search,
            ),
            None => select_bmai_reserve_action(game, context.rng, &self.search),
        }
    }

    fn auxiliary(
        &self,
        game: &Game,
        context: &mut DecisionContext<'_, '_>,
    ) -> Choice<Option<usize>> {
        let result = match context.native() {
            Some(native) => select_native_bmai_auxiliary_action(
                game,
                native.algorithm,
                native.replay,
                native.workers,
                &self.search,
            ),
            None => select_bmai_auxiliary_action(game, context.rng, &self.search),
        };
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
