// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::{Choice, DecisionContext, Engine};
use crate::Rng;
use crate::game::{Game, Move, apply_attack, property};
use crate::search::{
    ChanceMove, FocusMove, ScratchGame, SwingMove, acceptable_auxiliary_die, first_swing_move,
    restore_simulation,
};

/// The C++ "Quick AI": fast enough to play out every Monte Carlo simulation.
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
        first_swing_move(&game.players[player]).unwrap_or_else(SwingMove::empty)
    }

    fn chance(&self, _: &Game, _: usize, _: usize, _: &mut DecisionContext<'_, '_>) -> ChanceMove {
        ChanceMove { reroll: Vec::new() }
    }

    fn focus(&self, _: &Game, _: usize, _: usize, _: &mut DecisionContext<'_, '_>) -> FocusMove {
        FocusMove { values: Vec::new() }
    }

    fn attack(&self, game: &Game, context: &mut DecisionContext<'_, '_>) -> Choice<Move> {
        let fire_limit = super::SIMPLE_FIRE_CANDIDATES;
        Choice::unsearched(attack(game, context.rng, fire_limit))
    }

    fn reserve(&self, game: &Game, _: &mut DecisionContext<'_, '_>) -> Option<usize> {
        game.players[0].dice.iter().position(|die| die.in_reserve)
    }

    fn auxiliary(&self, game: &Game, _: &mut DecisionContext<'_, '_>) -> Choice<Option<usize>> {
        Choice::unsearched(acceptable_auxiliary_die(&game.players[0]))
    }
}

/// Scores each attack by one greedy look ahead. Monte Carlo playouts call
/// this directly, since they make far more moves than any search decides.
pub(crate) fn attack(game: &Game, rng: &mut Rng, fire_limit: usize) -> Move {
    let mut best: Option<(f32, Move)> = None;
    let mut simulation = ScratchGame::new(game);
    for candidate in game.valid_attacks(fire_limit) {
        restore_simulation(&mut simulation, game);
        apply_attack(&mut simulation, &candidate, rng);
        let mut score = simulation.players[0].score - simulation.players[1].score;
        for attacker in candidate.attackers.iter() {
            let die = &game.players[0].dice[attacker];
            let delta = (die.sides_max() as f32 + 1.0) * 0.5 - die.value_total() as f32;
            if !die.has_property(property::SHADOW) {
                score += if die.has_property(property::POISON) {
                    -delta
                } else {
                    delta
                };
            }
        }
        score += rng.rand_below(5) as f32;
        if best
            .as_ref()
            .is_none_or(|(best_score, _)| score > *best_score)
        {
            best = Some((score, candidate));
        }
    }
    best.map_or_else(Move::pass, |(_, action)| action)
}
