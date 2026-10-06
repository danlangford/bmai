// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! Exact play for the end of a round: every move, every reroll with its exact
//! probability, both players at their best. No sampling, so a position the
//! solver cannot finish within its budget is left to Monte Carlo.

use std::collections::{BTreeMap, HashMap, HashSet};

use super::{fight_over, moves_including_pass, win_probability};
use crate::Rng;
use crate::game::{Action, Game, Move, apply_attack, available_dice_count};

/// One distinct result of an attack and how likely it is.
pub(crate) struct Outcome {
    pub(crate) probability: f64,
    pub(crate) extra_turn: bool,
    pub(crate) game: Game,
}

/// Every result of `action`, found by replaying it with each face of each
/// random draw, or `None` past `limit` replays (Ornery dice multiply them).
/// Draws can depend on earlier ones (a Trip that fails rolls no Morph), so
/// this walks the tree of draws rather than a fixed grid. Sorted, so sums and
/// ties come out the same in every process.
pub(crate) fn attack_outcomes(game: &Game, action: &Move, limit: usize) -> Option<Vec<Outcome>> {
    let mut merged: BTreeMap<String, Outcome> = BTreeMap::new();
    let mut faces = Vec::<u32>::new();
    for _ in 0..limit {
        let mut rng = Rng::scripted(faces.clone());
        let mut result = game.clone();
        let extra_turn = apply_attack(&mut result, action, &mut rng);
        let ranges = rng.scripted_ranges().unwrap_or_default().to_vec();
        let probability = ranges.iter().map(|&range| 1.0 / f64::from(range)).product();
        let key = format!("{extra_turn}{:?}", result.players);
        merged
            .entry(key)
            .and_modify(|outcome| outcome.probability += probability)
            .or_insert(Outcome {
                probability,
                extra_turn,
                game: result,
            });
        faces.resize(ranges.len(), 0);
        let Some(position) = (0..ranges.len()).rev().find(|&i| faces[i] + 1 < ranges[i]) else {
            return Some(merged.into_values().collect());
        };
        faces.truncate(position + 1);
        faces[position] += 1;
    }
    None
}

/// Moves deep a line may run before the solver gives the move back; failed
/// Trips can make long lines of distinct positions, and each costs stack.
const DEPTH_LIMIT: usize = 48;

/// Replays allowed for one attack before the solver gives the move back.
const OUTCOME_LIMIT: usize = 20_000;

/// Solves the rest of the round for player 0, who is to move.
pub(crate) struct Solver {
    fire_limit: usize,
    node_limit: usize,
    nodes: usize,
    memo: HashMap<String, f64>,
    // Positions still being solved; meeting one again is a cycle (a Trip
    // that keeps failing), which this solver leaves to Monte Carlo.
    open: HashSet<String>,
}

impl Solver {
    pub(crate) fn new(fire_limit: usize, node_limit: usize) -> Self {
        Self {
            fire_limit,
            node_limit,
            nodes: 0,
            memo: HashMap::new(),
            open: HashSet::new(),
        }
    }

    /// The best move and its exact chance of winning the round, or `None`
    /// when the position needs more than the node budget.
    pub(crate) fn best_move(&mut self, game: &Game) -> Option<(Move, f64)> {
        let mut best: Option<(Move, f64)> = None;
        for action in moves_including_pass(game, self.fire_limit) {
            let value = self.move_value(game, &action, false)?;
            if best.as_ref().is_none_or(|(_, best)| value > *best) {
                best = Some((action, value));
            }
        }
        best
    }

    fn position_value(&mut self, game: &Game, passed: bool) -> Option<f64> {
        if fight_over(game) {
            return Some(f64::from(win_probability(game)));
        }
        let key = format!("{passed}{:?}", game.players);
        if let Some(&value) = self.memo.get(&key) {
            return Some(value);
        }
        self.nodes += 1;
        if self.nodes > self.node_limit
            || self.open.len() >= DEPTH_LIMIT
            || !self.open.insert(key.clone())
        {
            return None;
        }
        let mut best = 0.0f64;
        for action in moves_including_pass(game, self.fire_limit) {
            let value = self.move_value(game, &action, passed);
            let Some(value) = value else {
                self.open.remove(&key);
                return None;
            };
            best = best.max(value);
        }
        self.open.remove(&key);
        self.memo.insert(key, best);
        Some(best)
    }

    fn move_value(&mut self, game: &Game, action: &Move, passed: bool) -> Option<f64> {
        if action.action != Action::Attack {
            // Two passes in a row end the round.
            if passed {
                return Some(f64::from(win_probability(game)));
            }
            let mut next = game.clone();
            next.players.swap(0, 1);
            return Some(1.0 - self.position_value(&next, true)?);
        }
        let mut value = 0.0;
        for outcome in attack_outcomes(game, action, OUTCOME_LIMIT)? {
            let mut next = outcome.game;
            let after = if outcome.extra_turn {
                self.position_value(&next, false)?
            } else {
                next.players.swap(0, 1);
                1.0 - self.position_value(&next, false)?
            };
            value += outcome.probability * after;
        }
        Some(value)
    }
}

/// Dice still in play on both sides, which decides when the solver takes over.
pub(crate) fn dice_in_play(game: &Game) -> usize {
    game.players.iter().map(available_dice_count).sum()
}
