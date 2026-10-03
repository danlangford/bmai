// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

pub(crate) mod ai;

use self::ai::{Bmai3, EvaluationCoordinate, RolloutPolicy};
use crate::game::{
    Action, Die, Game, Move, SwingSet, apply_attack, apply_attack_for_players,
    apply_before_roll_effects, available_dice_count, check_initiative, initiative_winner,
    optimize_dice, property, recover_dizzy_dice, restore_dice_for_new_round, roll_die,
    roll_round_dice, roll_scheduled_die, special, swing_range,
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

fn completes_native_probability_sample(native: Option<NativeEvaluation>) -> bool {
    native.is_some_and(|context| context.replay.stream_version.completes_probability_sample())
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
            stream_version: crate::native::NativeStreamVersion::CURRENT,
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
use std::sync::OnceLock;

struct TraceSettings {
    swing_list: bool,
    swing_candidate: bool,
    swing_sim: bool,
    swing_moves: bool,
    swing: bool,
    reserve: bool,
    chance: bool,
    focus: bool,
    bmai_attack: bool,
    attack_eval: bool,
    qai: bool,
    rng: bool,
    qai_moves: bool,
}

fn trace_settings() -> &'static TraceSettings {
    static QUIET: TraceSettings = TraceSettings {
        swing_list: false,
        swing_candidate: false,
        swing_sim: false,
        swing_moves: false,
        swing: false,
        reserve: false,
        chance: false,
        focus: false,
        bmai_attack: false,
        attack_eval: false,
        qai: false,
        rng: false,
        qai_moves: false,
    };
    if crate::native::native_worker_active() {
        return &QUIET;
    }
    static SETTINGS: OnceLock<TraceSettings> = OnceLock::new();
    SETTINGS.get_or_init(|| TraceSettings {
        swing_list: std::env::var_os("BMAIR_TRACE_SWING_LIST").is_some(),
        swing_candidate: std::env::var_os("BMAIR_TRACE_SWING_CANDIDATE").is_some(),
        swing_sim: std::env::var_os("BMAIR_TRACE_SWING_SIM").is_some(),
        swing_moves: std::env::var_os("BMAIR_TRACE_SWING_MOVES").is_some(),
        swing: std::env::var_os("BMAIR_TRACE_SWING").is_some(),
        reserve: std::env::var_os("BMAIR_TRACE_RESERVE").is_some(),
        chance: std::env::var_os("BMAIR_TRACE_CHANCE").is_some(),
        focus: std::env::var_os("BMAIR_TRACE_FOCUS").is_some(),
        bmai_attack: std::env::var_os("BMAIR_TRACE_BMAI_ATTACK").is_some(),
        attack_eval: std::env::var_os("BMAIR_TRACE_ATTACK_EVAL").is_some(),
        qai: std::env::var_os("BMAIR_TRACE_QAI").is_some(),
        rng: std::env::var_os("BMAIR_TRACE_RNG").is_some(),
        qai_moves: std::env::var_os("BMAIR_TRACE_QAI_MOVES").is_some(),
    })
}

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
    fn empty() -> Self {
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

mod fight;
mod initiative;
mod match_play;
mod preround;

use fight::*;
use initiative::*;
use match_play::*;
use preround::*;

pub(crate) use fight::{
    evaluate_selected_native_bmai_move, select_bmai_action_with_stats, select_maximize_action,
    select_native_bmai_action_with_stats, select_qai_action, select_random_action,
};
pub(crate) use initiative::{select_chance_action, select_focus_action};
pub use match_play::play_games;
pub(crate) use match_play::{
    Engines, play_fair_games, play_fair_games_native, play_games_with_policies,
    play_games_with_policies_native,
};
pub(crate) use preround::{
    select_bmai_auxiliary_action, select_bmai_reserve_action, select_native_bmai_auxiliary_action,
    select_native_bmai_reserve_action, select_qai_auxiliary_action, select_qai_reserve_action,
    select_qai_set_swing_action_for, select_swing_action,
};

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
mod tests;
