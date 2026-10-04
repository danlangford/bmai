// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::fmt;
use std::io::{BufRead, Write};

use crate::engines::{DecisionContext, Engine, MonteCarlo, Setting};
use crate::game::{Action, Die, DieIndexSet, Game, MAX_DICE, Move, Phase, SwingSet, property};
use crate::search::{
    Engines, NativeReplaySequence, SwingMove, evaluate_selected_native_bmai_move, play_fair_games,
    play_fair_games_native, play_games_with_policies, play_games_with_policies_native,
};
use crate::{Bmai3, ExecutionMode, Rng, RngAlgorithm};

#[derive(Debug, Clone)]
pub struct ParseError(String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ParseError {}

/// Players follow the global Monte Carlo settings until `ai` or a per-player
/// setting gives them an engine of their own.
#[derive(Clone, Debug)]
enum PlayerEngine {
    Global,
    Own(Box<dyn Engine>),
}

#[derive(Clone, Debug)]
pub struct Parser {
    pub game: Game,
    report_sims: usize,
    execution_mode: ExecutionMode,
    native_root_seed: u64,
    native_decision_index: u64,
    native_workers: usize,
    rng: Rng,
    ai: Bmai3,
    player_engines: [PlayerEngine; 2],
    debug_ply: usize,
    logging: [bool; 8],
    last_action: Option<crate::protocol::ProtocolAction>,
    last_replay: Option<crate::protocol::ReplayMetadata>,
    last_evaluation: Option<crate::protocol::ProbabilityEstimate>,
}

impl Default for Parser {
    fn default() -> Self {
        Self {
            game: Game::default(),
            report_sims: 0,
            execution_mode: ExecutionMode::default(),
            native_root_seed: 78_904_497,
            native_decision_index: 0,
            native_workers: 1,
            rng: Rng::default(),
            ai: Bmai3::default(),
            player_engines: [PlayerEngine::Global, PlayerEngine::Global],
            debug_ply: 0,
            logging: [true; 8],
            last_action: None,
            last_replay: None,
            last_evaluation: None,
        }
    }
}

impl Parser {
    pub const fn execution_mode(&self) -> ExecutionMode {
        self.execution_mode
    }

    pub const fn rng_algorithm(&self) -> RngAlgorithm {
        self.rng.algorithm()
    }

    pub const fn rng_replay_id(&self) -> &'static str {
        self.rng.replay_id()
    }

    pub fn session_metadata(&self) -> crate::protocol::SessionMetadata {
        crate::protocol::SessionMetadata {
            phase: phase_protocol(self.game.phase),
            target_wins: self.game.target_wins,
            surrender_allowed: self.game.surrender_allowed,
            turbo_accuracy: crate::protocol::ProtocolFloat::from_f32(self.game.turbo_accuracy),
            fire_overshooting: self.game.fire_overshooting,
            execution_mode: self.execution_mode.as_str(),
            rng: self.rng.replay_id(),
            native_root_seed: self.native_root_seed,
            native_decision_index: self.native_decision_index,
            workers: self.native_workers,
            max_ply: self.ai.max_ply,
            min_simulations: self.ai.min_sims,
            max_simulations: self.ai.max_sims,
            max_branch: self.ai.max_branch,
            cull: self.ai.cull_moves,
            report_simulations: self.report_sims,
            players: std::array::from_fn(|player| {
                let engine = self.player_engine(player);
                crate::protocol::PlayerAiMetadata {
                    engine: engine.name(),
                    montecarlo: engine.montecarlo().map(|search| {
                        crate::protocol::MonteCarloMetadata {
                            max_ply: search.max_ply,
                            min_simulations: search.min_sims,
                            max_simulations: search.max_sims,
                            max_branch: search.max_branch,
                            cull: search.cull_moves,
                            playout: search.playout.name(),
                            time_limit_ms: search
                                .time_limit
                                .map(|limit| u64::try_from(limit.as_millis()).unwrap_or(u64::MAX)),
                        }
                    }),
                    specials: crate::protocol::notation::BUTTON_SPECIALS
                        .iter()
                        .filter(|special| self.game.players[player].specials & special.special != 0)
                        .map(|special| special.id)
                        .collect(),
                }
            }),
        }
    }

    pub fn last_action(&self) -> Option<&crate::protocol::ProtocolAction> {
        self.last_action.as_ref()
    }

    pub fn last_replay(&self) -> Option<&crate::protocol::ReplayMetadata> {
        self.last_replay.as_ref()
    }

    pub fn last_evaluation(&self) -> Option<&crate::protocol::ProbabilityEstimate> {
        self.last_evaluation.as_ref()
    }

    pub fn parse_string<W: Write>(&mut self, data: &str, output: &mut W) -> Result<(), ParseError> {
        self.last_action = None;
        self.last_replay = None;
        self.last_evaluation = None;
        self.parse_string_commands(data, output)
    }

    /// Clients keep stdin open, so each command runs as soon as it arrives.
    pub fn parse_stream<R: BufRead, W: Write>(
        &mut self,
        input: &mut R,
        output: &mut W,
    ) -> Result<(), ParseError> {
        self.last_action = None;
        self.last_replay = None;
        self.last_evaluation = None;

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
                        parse_die(definition.trim(), original_index)?;
                        block.push_str(&definition);
                    }
                }
            }

            self.parse_string_commands(&block, output)?;
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
