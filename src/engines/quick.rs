// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{Choice, DecisionContext, Engine};
use crate::game::{Game, Move};
use crate::search::{
    ChanceMove, FocusMove, SwingMove, select_qai_action, select_qai_auxiliary_action,
    select_qai_reserve_action, select_qai_set_swing_action_for,
};

/// The C++ "Quick AI": each attack is scored by one greedy look ahead, so it
/// is fast enough to play out every Monte Carlo simulation.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Quick;

impl Engine for Quick {
    fn name(&self) -> &'static str {
        "quick"
    }

    fn clone_box(&self) -> Box<dyn Engine> {
        Box::new(*self)
    }

    fn swing(&self, game: &Game, player: usize, _: &mut DecisionContext<'_, '_>) -> SwingMove {
        select_qai_set_swing_action_for(game, player)
    }

    fn chance(&self, _: &Game, _: usize, _: usize, _: &mut DecisionContext<'_, '_>) -> ChanceMove {
        ChanceMove { reroll: Vec::new() }
    }

    fn focus(&self, _: &Game, _: usize, _: usize, _: &mut DecisionContext<'_, '_>) -> FocusMove {
        FocusMove { values: Vec::new() }
    }

    fn attack(&self, game: &Game, context: &mut DecisionContext<'_, '_>) -> Choice<Move> {
        Choice::unsearched(select_qai_action(game, context.rng))
    }

    fn reserve(&self, game: &Game, _: &mut DecisionContext<'_, '_>) -> Option<usize> {
        select_qai_reserve_action(game)
    }

    fn auxiliary(&self, game: &Game, _: &mut DecisionContext<'_, '_>) -> Choice<Option<usize>> {
        Choice::unsearched(select_qai_auxiliary_action(game))
    }
}
