// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{Choice, DecisionContext, Engine, Quick, SIMPLE_FIRE_CANDIDATES};
use crate::game::{Game, Move};
use crate::search::{ChanceMove, FocusMove, SwingMove};

/// Quick that weighs what each attack leaves the opponent to capture. A
/// separate engine, since Quick must keep playing like the C++ QAI.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Careful;

impl Engine for Careful {
    fn name(&self) -> &'static str {
        "careful"
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
        Choice::unsearched(super::quick::careful_attack(
            game,
            context.rng,
            SIMPLE_FIRE_CANDIDATES,
        ))
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
