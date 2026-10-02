// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::fmt;
use std::io::{BufRead, Write};

use crate::game::{
    BMC_Die, BMC_DieIndexSet, BMC_Game, BMC_Move, BMD_MAX_DICE, BME_ACTION, BME_PHASE,
    BME_SWING_SET, property,
};
use crate::search::{
    BMC_AI_POLICY, EvaluateSelectedNativeBMAIMove, PlayFairGames, PlayFairGamesNative,
    PlayGamesWithPolicies, PlayGamesWithPoliciesNative, SelectBMAIActionWithStats,
    SelectBMAIAuxiliaryAction, SelectBMAIChanceAction, SelectBMAIFocusAction,
    SelectBMAIReserveAction, SelectBMAISetSwingAction, SelectNativeBMAIActionWithStats,
    SelectNativeBMAIAuxiliaryAction, SelectNativeBMAIChanceAction, SelectNativeBMAIFocusAction,
    SelectNativeBMAIReserveAction, SelectNativeBMAISetSwingAction, SelectQAIAction,
    SelectQAIAuxiliaryAction, SelectQAIReserveAction, SelectQAISetSwingAction, SwingMove,
};
use crate::{BMC_BMAI3, BMC_RNG, BME_RNG_ALGORITHM, BME_ROLLOUT_POLICY, ExecutionMode};

#[derive(Debug, Clone)]
pub struct ParseError(String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ParseError {}

/// Which C++ AI object a player's `BMC_Game::m_ai` pointer names. C++ shares
/// these objects by pointer, so a per-player setting changes every player
/// that points at the same object, and the objects outlive `game` blocks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
enum BMC_AI_SLOT {
    /// No `game` or `ai` command has set the pointer yet; C++ holds NULL.
    UNBOUND,
    /// `g_ai`, the global BMAI3 that `ParseGame` assigns to both players.
    GLOBAL,
    /// `c_ai_type[N]`: `g_bmai`, `g_qai2`, or `g_bmai3`.
    TYPE(usize),
}

#[derive(Clone, Debug)]
pub struct BMC_Parser {
    pub m_game: BMC_Game,
    m_report_sims: usize,
    m_execution_mode: ExecutionMode,
    m_native_root_seed: u64,
    m_native_decision_index: u64,
    m_native_workers: usize,
    m_rng: BMC_RNG,
    /// C++ `g_ai`; its settings are the global settings reported by `stats`.
    m_ai: BMC_BMAI3,
    /// C++ `c_ai_type`; the QAI entry carries no search settings.
    m_type_ai: [BMC_BMAI3; 3],
    m_player_ai: [BMC_AI_SLOT; 2],
    m_debug_ply: usize,
    m_logging: [bool; 8],
    m_last_action: Option<crate::protocol::ProtocolAction>,
    m_last_replay: Option<crate::protocol::ReplayMetadata>,
    m_last_evaluation: Option<crate::protocol::ProbabilityEstimate>,
}

impl Default for BMC_Parser {
    fn default() -> Self {
        Self {
            m_game: BMC_Game::default(),
            m_report_sims: 0,
            m_execution_mode: ExecutionMode::default(),
            m_native_root_seed: 78_904_497,
            m_native_decision_index: 0,
            m_native_workers: 1,
            m_rng: BMC_RNG::default(),
            m_ai: BMC_BMAI3::default(),
            m_type_ai: std::array::from_fn(|ai_type| BMC_BMAI3 {
                m_cull_moves: ai_type == 2,
                ..Default::default()
            }),
            m_player_ai: [BMC_AI_SLOT::UNBOUND; 2],
            m_debug_ply: 0,
            m_logging: [true; 8],
            m_last_action: None,
            m_last_replay: None,
            m_last_evaluation: None,
        }
    }
}

impl BMC_Parser {
    pub const fn execution_mode(&self) -> ExecutionMode {
        self.m_execution_mode
    }

    pub const fn rng_algorithm(&self) -> BME_RNG_ALGORITHM {
        self.m_rng.Algorithm()
    }

    pub const fn rng_replay_id(&self) -> &'static str {
        self.m_rng.ReplayId()
    }

    pub fn session_metadata(&self) -> crate::protocol::SessionMetadata {
        crate::protocol::SessionMetadata {
            phase: phase_protocol(self.m_game.m_phase),
            target_wins: self.m_game.m_target_wins,
            surrender_allowed: self.m_game.m_surrender_allowed,
            turbo_accuracy: crate::protocol::ProtocolFloat::from_f32(self.m_game.m_turbo_accuracy),
            fire_overshooting: self.m_game.m_fire_overshooting,
            execution_mode: self.m_execution_mode.as_str(),
            rng: self.m_rng.ReplayId(),
            native_root_seed: self.m_native_root_seed,
            native_decision_index: self.m_native_decision_index,
            workers: self.m_native_workers,
            max_ply: self.m_ai.m_max_ply,
            min_simulations: self.m_ai.m_min_sims,
            max_simulations: self.m_ai.m_max_sims,
            max_branch: self.m_ai.m_max_branch,
            report_simulations: self.m_report_sims,
            players: std::array::from_fn(|player| {
                let ai = self.PlayerAI(player);
                crate::protocol::PlayerAiMetadata {
                    ai_type: self.AIType(player),
                    policy: match self.AIType(player) {
                        0 => "bmai",
                        1 => "qai",
                        2 => "bmai3",
                        _ => unreachable!(),
                    },
                    culls_moves: ai.m_cull_moves,
                    max_ply: ai.m_max_ply,
                    min_simulations: ai.m_min_sims,
                    max_simulations: ai.m_max_sims,
                    max_branch: ai.m_max_branch,
                }
            }),
        }
    }

    pub fn last_action(&self) -> Option<&crate::protocol::ProtocolAction> {
        self.m_last_action.as_ref()
    }

    pub fn last_replay(&self) -> Option<&crate::protocol::ReplayMetadata> {
        self.m_last_replay.as_ref()
    }

    pub fn last_evaluation(&self) -> Option<&crate::protocol::ProbabilityEstimate> {
        self.m_last_evaluation.as_ref()
    }

    pub fn ParseString<W: Write>(&mut self, data: &str, output: &mut W) -> Result<(), ParseError> {
        self.m_last_action = None;
        self.m_last_replay = None;
        self.m_last_evaluation = None;
        self.ParseStringCommands(data, output)
    }

    /// Parse the legacy protocol incrementally, matching the C++ stdin
    /// contract: each top-level command is executed as soon as its complete
    /// line or game block arrives, and `quit` terminates without waiting for
    /// EOF.
    pub fn ParseStream<R: BufRead, W: Write>(
        &mut self,
        input: &mut R,
        output: &mut W,
    ) -> Result<(), ParseError> {
        self.m_last_action = None;
        self.m_last_replay = None;
        self.m_last_evaluation = None;

        while let Some(line) = read_stream_line(input)? {
            let command = line.trim();
            if command.is_empty() || command.starts_with('#') {
                continue;
            }
            let is_game = command.starts_with("game");
            let is_quit = command == "quit";

            let mut block = line;
            if is_game {
                let phase = read_required_stream_line(input, "missing phase")?;
                parse_phase(phase.trim())?;
                block.push_str(&phase);
                for expected_player in 0..2 {
                    let header = read_required_stream_line(input, "missing player")?;
                    let fields = header.split_whitespace().collect::<Vec<_>>();
                    if fields.len() < 4 || fields[0] != "player" {
                        return Err(ParseError(format!("missing player: {}", header.trim())));
                    }
                    let player = parse_usize(fields[1])?;
                    let dice = parse_usize(fields[2])?;
                    validate_player_dice_count(dice)?;
                    fields[3]
                        .parse::<f32>()
                        .map_err(|_| ParseError(format!("invalid score: {}", fields[3])))?;
                    if player != expected_player {
                        return Err(ParseError(format!("expected player {expected_player}")));
                    }
                    block.push_str(&header);
                    for original_index in 0..dice {
                        let definition = read_required_stream_line(input, "missing die")?;
                        ParseDie(definition.trim(), original_index)?;
                        block.push_str(&definition);
                    }
                }
            }

            self.ParseStringCommands(&block, output)?;
            output.flush().map_err(io_error)?;
            if is_quit {
                break;
            }
        }
        Ok(())
    }
}

mod action;
mod auxiliary;
mod commands;
mod die;
mod input;
mod output;

use auxiliary::*;
use die::*;
use input::*;
use output::*;

#[cfg(test)]
mod tests;
