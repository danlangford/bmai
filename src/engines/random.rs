// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{Choice, DecisionContext, Engine, Quick};
use crate::Rng;
use crate::game::{Game, Move};
use crate::search::{ChanceMove, FocusMove, SwingMove, moves_including_pass};

/// Attacks with any legal move. A floor for the strength ladder; outside the
/// fight it decides as [`Quick`] does.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Random;

impl Engine for Random {
    fn name(&self) -> &'static str {
        "random"
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
        Choice::unsearched(attack(game, context.rng, fire_limit))
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

pub(crate) fn attack(game: &Game, rng: &mut Rng, fire_limit: usize) -> Move {
    let moves = moves_including_pass(game, fire_limit);
    moves[rng.rand_below(moves.len() as u32) as usize].clone()
}
