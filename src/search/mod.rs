// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

#![allow(non_snake_case)]

pub(crate) mod ai;

use self::ai::{BMC_BMAI3, BME_ROLLOUT_POLICY, EvaluationCoordinate};
use crate::game::{
    ApplyAttack, ApplyAttackForPlayers, ApplyBeforeRollEffects, AvailableDice, BMC_Die, BMC_Game,
    BMC_Move, BME_ACTION, BME_SWING_SET, CheckInitiative, InitiativeWinner, OptimizeDice,
    RecoverDizzyDice, RestoreDiceForNewRound, RollDie, RollRoundDice, RollScheduledDie, SwingRange,
    property,
};
#[cfg(test)]
use crate::game::{ApplyAttackPlayerEffects, BMD_MAX_DICE};
use crate::rng::BMC_RNG;

#[derive(Clone, Copy)]
struct NativeEvaluation {
    algorithm: crate::BME_RNG_ALGORITHM,
    replay: crate::native::NativeReplayKey,
    workers: usize,
}

fn CompletesNativeProbabilitySample(native: Option<NativeEvaluation>) -> bool {
    native.is_some_and(|context| context.replay.stream_version.completes_probability_sample())
}

struct NativeReplaySequence<'a> {
    algorithm: crate::BME_RNG_ALGORITHM,
    root_seed: u64,
    workers: usize,
    decision_index: &'a mut u64,
}

impl NativeReplaySequence<'_> {
    fn next(&mut self) -> NativeEvaluation {
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
// Reserved candidate coordinate for a fresh selected-move probability sample.
// Search candidates always use their zero-based position, so this stream is
// disjoint from every sample consumed while choosing the move.
const NATIVE_REPORTING_STREAM: usize = 0xffff_fffe;
use std::sync::OnceLock;

#[derive(Clone, Debug)]
#[allow(clippy::upper_case_acronyms)]
pub enum BMC_AI_POLICY {
    BMAI(Box<BMC_BMAI3>),
    QAI,
    RANDOM,
    MAXIMIZE,
}

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

fn TraceSettings() -> &'static TraceSettings {
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
    EvaluateSelectedNativeBMAIMove, SelectBMAIActionWithStats, SelectNativeBMAIActionWithStats,
    SelectQAIAction,
};
pub(crate) use initiative::{
    SelectBMAIChanceAction, SelectBMAIFocusAction, SelectNativeBMAIChanceAction,
    SelectNativeBMAIFocusAction,
};
pub use match_play::PlayGames;
pub(crate) use match_play::{
    PlayFairGames, PlayFairGamesNative, PlayGamesWithPolicies, PlayGamesWithPoliciesNative,
};
pub(crate) use preround::{
    SelectBMAIAuxiliaryAction, SelectBMAIReserveAction, SelectBMAISetSwingAction,
    SelectNativeBMAIAuxiliaryAction, SelectNativeBMAIReserveAction, SelectNativeBMAISetSwingAction,
    SelectQAIAuxiliaryAction, SelectQAIReserveAction, SelectQAISetSwingAction,
};

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
mod tests;
