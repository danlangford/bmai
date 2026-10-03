// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{Choice, DecisionContext, Engine, Quick};
use crate::game::{Game, Move};
use crate::search::{ChanceMove, FocusMove, SwingMove, select_maximize_action};

/// Attacks for the most points this turn. Outside the fight it decides as
/// [`Quick`] does.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Maximize;

impl Engine for Maximize {
    fn name(&self) -> &'static str {
        "maximize"
    }

    fn clone_box(&self) -> Box<dyn Engine> {
        Box::new(*self)
    }

    fn swing(
        &self,
        game: &Game,
        player: usize,
        context: &mut DecisionContext<'_, '_>,
    ) -> SwingMove {
        Quick.swing(game, player, context)
    }

    fn chance(
        &self,
        game: &Game,
        player: usize,
        initiative: usize,
        context: &mut DecisionContext<'_, '_>,
    ) -> ChanceMove {
        Quick.chance(game, player, initiative, context)
    }

    fn focus(
        &self,
        game: &Game,
        player: usize,
        initiative: usize,
        context: &mut DecisionContext<'_, '_>,
    ) -> FocusMove {
        Quick.focus(game, player, initiative, context)
    }

    fn attack(&self, game: &Game, context: &mut DecisionContext<'_, '_>) -> Choice<Move> {
        let fire_limit = crate::Bmai3::default().fire_candidate_limit();
        Choice::unsearched(select_maximize_action(game, context.rng, fire_limit))
    }

    fn reserve(&self, game: &Game, context: &mut DecisionContext<'_, '_>) -> Option<usize> {
        Quick.reserve(game, context)
    }

    fn auxiliary(
        &self,
        game: &Game,
        context: &mut DecisionContext<'_, '_>,
    ) -> Choice<Option<usize>> {
        Quick.auxiliary(game, context)
    }
}
