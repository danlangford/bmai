// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

pub(crate) mod ai;

use self::ai::{Bmai3, EvaluationCoordinate, Playout};
use crate::game::{
    Action, Die, Game, Move, RoundSelections, SwingSet, apply_attack, apply_attack_for_players,
    apply_before_roll_effects, available_dice_count, check_initiative, initiative_winner,
    optimize_dice, property, record_round_selections, recover_dizzy_dice,
    restore_dice_for_new_round, roll_die, roll_round_dice, roll_scheduled_die, special,
    swing_range,
};
#[cfg(test)]
use crate::game::{MAX_DICE, apply_attack_player_effects};
use crate::rng::Rng;

#[derive(Clone, Copy)]
pub(crate) struct NativeEvaluation {
    pub(crate) algorithm: crate::RngAlgorithm,
    pub(crate) replay: crate::native::NativeReplayKey,
    pub(crate) workers: usize,
}

impl NativeEvaluation {
    pub(crate) fn simulation_rng(
        self,
        candidate_index: impl TryInto<u64>,
        batch_index: usize,
        simulation_index: usize,
    ) -> Rng {
        let candidate_index = candidate_index
            .try_into()
            .ok()
            .expect("candidate index must fit in the replay format");
        let key = crate::native::NativeSimulationKey {
            replay: self.replay,
            candidate_index,
            batch_index: batch_index as u64,
            simulation_index: simulation_index as u64,
        };
        Rng::from_native_stream(self.algorithm, key.derive_stream_seed(), key.stratum())
    }
}

fn completes_native_probability_sample(native: Option<NativeEvaluation>) -> bool {
    native.is_some()
}

pub(crate) struct NativeReplaySequence<'a> {
    pub(crate) algorithm: crate::RngAlgorithm,
    pub(crate) root_seed: u64,
    pub(crate) workers: usize,
    pub(crate) decision_index: &'a mut u64,
}

impl NativeReplaySequence<'_> {
    pub(crate) fn next(&mut self) -> NativeEvaluation {
        let replay = crate::native::NativeReplayKey {
            root_seed: self.root_seed,
            decision_index: *self.decision_index,
        };
        *self.decision_index = self.decision_index.wrapping_add(1);
        NativeEvaluation {
            algorithm: self.algorithm,
            replay,
            workers: self.workers,
        }
    }
}

const NATIVE_ENUMERATION_STREAM: u64 = u64::MAX;
// Never a candidate position, so report draws cannot overlap search draws.
const NATIVE_REPORTING_STREAM: usize = 0xffff_fffe;

#[derive(Clone, Copy)]
pub(crate) struct SwingMove {
    values: [(char, u8); 10],
    value_len: u8,
    options: [(usize, bool); 10],
    option_len: u8,
}

#[derive(Clone, Debug)]
pub(crate) struct FocusMove {
    pub(crate) values: Vec<(usize, u8)>,
}

#[derive(Clone, Debug)]
pub(crate) struct ChanceMove {
    pub(crate) reroll: Vec<usize>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AuxiliarySearchResult {
    pub(crate) die: Option<usize>,
    pub(crate) score: f32,
    pub(crate) simulations: usize,
}

impl AuxiliarySearchResult {
    pub(crate) fn probability(&self) -> f32 {
        if self.simulations == 0 {
            0.0
        } else {
            self.score / self.simulations as f32
        }
    }
}

#[derive(Clone, Copy)]
enum InitiativeStage {
    Chance,
    Focus,
    Fight,
}

impl SwingMove {
    pub(crate) fn empty() -> Self {
        Self {
            values: [('\0', 0); 10],
            value_len: 0,
            options: [(0, false); 10],
            option_len: 0,
        }
    }

    pub(crate) fn values(&self) -> &[(char, u8)] {
        &self.values[..usize::from(self.value_len)]
    }

    pub(crate) fn options(&self) -> &[(usize, bool)] {
        &self.options[..usize::from(self.option_len)]
    }

    fn push_value(&mut self, value: (char, u8)) {
        self.values[usize::from(self.value_len)] = value;
        self.value_len += 1;
    }

    fn push_option(&mut self, value: (usize, bool)) {
        self.options[usize::from(self.option_len)] = value;
        self.option_len += 1;
    }
}

mod endgame;
pub(crate) use endgame::{Solver, dice_in_play};
mod fight;
mod initiative;
mod match_play;
mod preround;

use fight::*;
use initiative::*;
use match_play::*;
use preround::*;

pub(crate) use fight::{
    ScratchGame, evaluate_selected_native_bmai_move, moves_including_pass, pass_move,
    restore_simulation, select_native_bmai_action,
};
pub(crate) use initiative::{select_chance_action, select_focus_action};
pub(crate) use match_play::{
    Engines, MatchResult, play_fair_games, play_games_with_policies, play_match_with_policies,
};
pub(crate) use preround::{
    acceptable_auxiliary_die, first_swing_move, offer_courtesy_auxiliary,
    select_native_bmai_auxiliary_action, select_native_bmai_reserve_action, select_swing_action,
};

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
mod tests;
