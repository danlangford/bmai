// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::fmt;
use std::io::{BufRead, Write};

use crate::model::{
    BMC_Die, BMC_DieIndexSet, BMC_Game, BMC_Move, BME_ACTION, BME_PHASE, BME_SWING_SET, property,
};
use crate::simulation::{
    BMC_AI_POLICY, PlayFairGames, PlayFairGamesNative, PlayGamesWithPolicies,
    PlayGamesWithPoliciesNative, SelectBMAIActionWithStats, SelectBMAIChanceAction,
    SelectBMAIFocusAction, SelectBMAIReserveAction, SelectBMAISetSwingAction,
    SelectNativeBMAIActionWithStats, SelectNativeBMAIChanceAction, SelectNativeBMAIFocusAction,
    SelectNativeBMAIReserveAction, SelectNativeBMAISetSwingAction, SelectQAIAction,
    SelectQAIReserveAction, SelectQAISetSwingAction, SwingMove,
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

#[derive(Clone, Debug)]
pub struct BMC_Parser {
    pub m_game: BMC_Game,
    m_max_ply: usize,
    m_min_sims: usize,
    m_max_sims: usize,
    m_max_branch: usize,
    m_execution_mode: ExecutionMode,
    m_native_root_seed: u64,
    m_native_decision_index: u64,
    m_native_workers: usize,
    m_rng: BMC_RNG,
    m_ai: BMC_BMAI3,
    m_player_ai: [BMC_BMAI3; 2],
    m_ai_type: [usize; 2],
    m_ai_explicit: [bool; 2],
    m_debug_ply: usize,
    m_logging: [bool; 8],
    m_last_action: Option<crate::protocol::ProtocolAction>,
    m_last_replay: Option<crate::protocol::ReplayMetadata>,
}

impl Default for BMC_Parser {
    fn default() -> Self {
        Self {
            m_game: BMC_Game::default(),
            m_max_ply: 1,
            m_min_sims: 10,
            m_max_sims: 500,
            m_max_branch: 5000,
            m_execution_mode: ExecutionMode::default(),
            m_native_root_seed: 78_904_497,
            m_native_decision_index: 0,
            m_native_workers: 1,
            m_rng: BMC_RNG::default(),
            m_ai: BMC_BMAI3::default(),
            m_player_ai: std::array::from_fn(|_| BMC_BMAI3::default()),
            m_ai_type: [2, 2],
            m_ai_explicit: [false, false],
            m_debug_ply: 0,
            m_logging: [true; 8],
            m_last_action: None,
            m_last_replay: None,
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
            execution_mode: self.m_execution_mode.as_str(),
            rng: self.m_rng.ReplayId(),
            native_root_seed: self.m_native_root_seed,
            native_decision_index: self.m_native_decision_index,
            workers: self.m_native_workers,
            max_ply: self.m_max_ply,
            min_simulations: self.m_min_sims,
            max_simulations: self.m_max_sims,
            max_branch: self.m_max_branch,
            players: std::array::from_fn(|player| {
                let ai = &self.m_player_ai[player];
                crate::protocol::PlayerAiMetadata {
                    ai_type: self.m_ai_type[player],
                    policy: match self.m_ai_type[player] {
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

    pub fn ParseString<W: Write>(&mut self, data: &str, output: &mut W) -> Result<(), ParseError> {
        self.m_last_action = None;
        self.m_last_replay = None;
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

    fn ParseStringCommands<W: Write>(
        &mut self,
        data: &str,
        output: &mut W,
    ) -> Result<(), ParseError> {
        let lines: Vec<_> = data.lines().collect();
        let mut pos = 0;
        while pos < lines.len() {
            let line = lines[pos].trim();
            pos += 1;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(value) = line.strip_prefix("mode ") {
                self.m_execution_mode = ExecutionMode::parse(value).ok_or_else(|| {
                    ParseError(format!(
                        "invalid execution mode: {value} (expected legacy or native)"
                    ))
                })?;
                writeln!(
                    output,
                    "Setting execution mode to {}",
                    self.m_execution_mode.as_str()
                )
                .map_err(io_error)?;
            } else if let Some(value) = line.strip_prefix("rng ") {
                let algorithm = BME_RNG_ALGORITHM::Parse(value).ok_or_else(|| {
                    ParseError(format!(
                        "invalid RNG algorithm: {value} (expected legacy or park-miller)"
                    ))
                })?;
                self.m_rng.SetAlgorithm(algorithm);
                writeln!(output, "Setting RNG to legacy ({})", algorithm.ReplayId())
                    .map_err(io_error)?;
            } else if let Some(value) = line.strip_prefix("workers ") {
                let (workers, automatic) = if value == "auto" {
                    (available_workers(), true)
                } else {
                    (parse_usize(value)?, false)
                };
                if workers == 0 {
                    return Err(ParseError("native worker count must be at least 1".into()));
                }
                self.m_native_workers = workers;
                if automatic {
                    writeln!(output, "Setting native workers to {workers} (auto)")
                        .map_err(io_error)?;
                } else {
                    writeln!(output, "Setting native workers to {workers}").map_err(io_error)?;
                }
            } else if line.starts_with("game") {
                if let Some(wins) = line.strip_prefix("game ") {
                    self.m_game.m_target_wins = parse_usize(wins)? as u8;
                    writeln!(output, "target wins set to {}", self.m_game.m_target_wins)
                        .map_err(io_error)?;
                }
                pos = self.ParseGame(&lines, pos, output)?;
            } else if let Some((player, value)) = two_usize_arguments(line, "ai")? {
                if value > 2 {
                    return Err(ParseError(format!("invalid setting for ai type: {value}")));
                }
                if player > 1 {
                    return Err(ParseError(format!(
                        "invalid setting for ai player number: {player}"
                    )));
                }
                self.m_ai_type[player] = value;
                self.m_ai_explicit[player] = true;
                self.m_player_ai[player].m_cull_moves = value == 2;
                writeln!(output, "Setting AI for player {player} to type {value}")
                    .map_err(io_error)?;
            } else if let Some((player, value)) = two_usize_arguments(line, "ply")? {
                if self.PlayerIsBMAI(player)? {
                    self.m_player_ai[player].m_max_ply = value;
                    writeln!(output, "Setting max ply for player {player} to {value}")
                        .map_err(io_error)?;
                }
            } else if let Some(value) = argument(line, "ply") {
                self.m_max_ply = value?;
                self.m_ai.m_max_ply = self.m_max_ply;
                let setting = self.m_max_ply;
                self.SyncDefaultAI(|ai| ai.m_max_ply = setting);
                writeln!(output, "Setting max ply to {}", self.m_max_ply).map_err(io_error)?;
            } else if let Some((player, value)) = two_usize_arguments(line, "max_sims")? {
                if self.PlayerIsBMAI(player)? {
                    self.m_player_ai[player].m_max_sims = value;
                    writeln!(output, "Setting max sims for player {player} to {value}")
                        .map_err(io_error)?;
                }
            } else if let Some(value) = argument(line, "max_sims") {
                self.m_max_sims = value?;
                self.m_ai.m_max_sims = self.m_max_sims;
                let setting = self.m_max_sims;
                self.SyncDefaultAI(|ai| ai.m_max_sims = setting);
                writeln!(output, "Setting max # simulations to {}", self.m_max_sims)
                    .map_err(io_error)?;
            } else if let Some((player, value)) = two_usize_arguments(line, "min_sims")? {
                if self.PlayerIsBMAI(player)? {
                    self.m_player_ai[player].m_min_sims = value;
                    writeln!(output, "Setting min sims for player {player} to {value}")
                        .map_err(io_error)?;
                }
            } else if let Some(value) = argument(line, "min_sims") {
                self.m_min_sims = value?;
                self.m_ai.m_min_sims = self.m_min_sims;
                let setting = self.m_min_sims;
                self.SyncDefaultAI(|ai| ai.m_min_sims = setting);
                writeln!(output, "Setting min # simulations to {}", self.m_min_sims)
                    .map_err(io_error)?;
            } else if let Some((player, value)) = two_usize_arguments(line, "maxbranch")? {
                if self.PlayerIsBMAI(player)? {
                    self.m_player_ai[player].m_max_branch = value;
                    writeln!(output, "Setting max branch for player {player} to {value}")
                        .map_err(io_error)?;
                }
            } else if let Some(value) = argument(line, "maxbranch") {
                self.m_max_branch = value?;
                self.m_ai.m_max_branch = self.m_max_branch;
                let setting = self.m_max_branch;
                self.SyncDefaultAI(|ai| ai.m_max_branch = setting);
                writeln!(output, "Setting max branch to {}", self.m_max_branch)
                    .map_err(io_error)?;
            } else if let Some(value) = line.strip_prefix("turbo_accuracy ") {
                self.m_game.m_turbo_accuracy = value
                    .parse::<f32>()
                    .map_err(|_| ParseError(format!("invalid float: {value}")))?;
                writeln!(
                    output,
                    "Setting turbo accuracy to {:.6}",
                    self.m_game.m_turbo_accuracy
                )
                .map_err(io_error)?;
            } else if let Some(value) = line.strip_prefix("surrender ") {
                self.m_game.m_surrender_allowed = value == "on";
            } else if line == "getaction" {
                self.GetAction(output)?;
            } else if line.starts_with("playgame ") {
                self.RequirePreround()?;
                let games = parse_usize(line.trim_start_matches("playgame "))?;
                let policies = self.Policies();
                let wins = if self.m_execution_mode == ExecutionMode::Native {
                    PlayGamesWithPoliciesNative(
                        &self.m_game,
                        games,
                        &mut self.m_rng,
                        &policies,
                        self.m_native_root_seed,
                        self.m_native_workers,
                        &mut self.m_native_decision_index,
                    )
                } else {
                    PlayGamesWithPolicies(&self.m_game, games, &mut self.m_rng, &policies)
                };
                writeln!(output, "matches over {} - {}", wins[0], wins[1]).map_err(io_error)?;
            } else if let Some((games, mode, probability)) = playfair_arguments(line)? {
                self.RequirePreround()?;
                // C++ accepts out-of-range modes and simply leaves the game's
                // currently selected AIs unchanged.
                let policies = if mode > 3 {
                    self.Policies()
                } else {
                    std::array::from_fn(|_| match mode {
                        0 => BMC_AI_POLICY::RANDOM,
                        1 => BMC_AI_POLICY::MAXIMIZE,
                        2 | 3 => {
                            let ai = BMC_BMAI3 {
                                m_cull_moves: false,
                                m_max_ply: self.m_ai.m_max_ply,
                                m_rollout_policy: if mode == 2 {
                                    BME_ROLLOUT_POLICY::MAXIMIZE_OR_RANDOM(probability)
                                } else {
                                    BME_ROLLOUT_POLICY::QAI
                                },
                                ..Default::default()
                            };
                            BMC_AI_POLICY::BMAI(Box::new(ai))
                        }
                        _ => unreachable!(),
                    })
                };
                let wins = if self.m_execution_mode == ExecutionMode::Native {
                    PlayFairGamesNative(
                        &self.m_game,
                        games,
                        &mut self.m_rng,
                        &policies,
                        self.m_native_root_seed,
                        self.m_native_workers,
                        &mut self.m_native_decision_index,
                    )
                } else {
                    PlayFairGames(&self.m_game, games, &mut self.m_rng, &policies)
                };
                writeln!(
                    output,
                    "PlayFairGames: {games} games, mode {mode}, p {probability:.6}"
                )
                .map_err(io_error)?;
                for player in 0..2 {
                    let initiatives = if player == 0 { [0, 1] } else { [1, 0] };
                    for initiative in initiatives {
                        let won = wins[initiative][player];
                        let lost = wins[initiative][1 - player];
                        let total = won + lost;
                        let percent = if total == 0 {
                            "nan".to_owned()
                        } else {
                            format!("{:.1}", won as f32 * 100.0 / total as f32)
                        };
                        writeln!(
                            output,
                            "P{player} stats: initiative P{initiative} games {total} wins {won} losses {lost} percent {percent}%"
                        )
                        .map_err(io_error)?;
                    }
                }
            } else if line.starts_with("compare ") {
                self.RequirePreround()?;
                let games = parse_usize(line.trim_start_matches("compare "))?;
                let policies = self.Policies();
                let wins = if self.m_execution_mode == ExecutionMode::Native {
                    PlayGamesWithPoliciesNative(
                        &self.m_game,
                        games,
                        &mut self.m_rng,
                        &policies,
                        self.m_native_root_seed,
                        self.m_native_workers,
                        &mut self.m_native_decision_index,
                    )
                } else {
                    PlayGamesWithPolicies(&self.m_game, games, &mut self.m_rng, &policies)
                };
                writeln!(output, "matches over {} - {}", wins[0], wins[1]).map_err(io_error)?;
            } else if line == "quit" {
                break;
            } else if let Some(value) = argument(line, "seed") {
                let seed = value?;
                let resolved = if seed == 0 {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|error| ParseError(error.to_string()))?
                        .as_secs() as u32
                } else {
                    seed as u32
                };
                self.m_rng.SRand(resolved);
                self.m_native_root_seed = u64::from(resolved);
                self.m_native_decision_index = 0;
                writeln!(output, "Seeding with {seed}").map_err(io_error)?;
            } else if let Some(value) = argument(line, "debugply") {
                self.m_debug_ply = value?;
                writeln!(output, "Setting debug ply to {}", self.m_debug_ply).map_err(io_error)?;
            } else if line.starts_with("debug ") {
                let values = line.split_whitespace().collect::<Vec<_>>();
                if values.len() != 3 {
                    return Err(ParseError(format!("unrecognized command: {line}")));
                }
                let category = values[1];
                let categories = [
                    "ALWAYS",
                    "WARNING",
                    "PARSER",
                    "SIMULATION",
                    "ROUND",
                    "GAME",
                    "QAI",
                    "BMAI",
                ];
                let Some(index) = categories.iter().position(|name| *name == category) else {
                    return Err(ParseError(format!(
                        "Could not find debug category: {category}"
                    )));
                };
                let enabled = values[2]
                    .parse::<i32>()
                    .map_err(|_| ParseError(format!("invalid integer: {}", values[2])))?
                    != 0;
                let enabled = usize::from(enabled);
                self.m_logging[index] = enabled != 0;
                if self.m_logging[0] {
                    writeln!(output, "Debug {category} set to {enabled}").map_err(io_error)?;
                }
            } else {
                return Err(ParseError(format!("unrecognized command: {line}")));
            }
        }
        Ok(())
    }

    fn PlayerIsBMAI(&self, player: usize) -> Result<bool, ParseError> {
        self.m_ai_type
            .get(player)
            .map(|kind| *kind != 1)
            .ok_or_else(|| ParseError(format!("invalid setting for ai player number: {player}")))
    }

    fn SyncDefaultAI(&mut self, update: impl Fn(&mut BMC_BMAI3) + Copy) {
        for player in 0..2 {
            if !self.m_ai_explicit[player] {
                update(&mut self.m_player_ai[player]);
            }
        }
    }

    fn RequirePreround(&self) -> Result<(), ParseError> {
        if self.m_game.m_phase == BME_PHASE::PREROUND {
            Ok(())
        } else {
            Err(ParseError("Cannot PlayGame unless it is preround".into()))
        }
    }

    fn Policies(&self) -> [BMC_AI_POLICY; 2] {
        std::array::from_fn(|player| match self.m_ai_type[player] {
            0 => {
                let mut ai = self.m_player_ai[player].clone();
                ai.m_cull_moves = false;
                BMC_AI_POLICY::BMAI(Box::new(ai))
            }
            1 => BMC_AI_POLICY::QAI,
            2 => BMC_AI_POLICY::BMAI(Box::new(self.m_player_ai[player].clone())),
            _ => unreachable!(),
        })
    }

    fn ParseGame<W: Write>(
        &mut self,
        lines: &[&str],
        mut pos: usize,
        output: &mut W,
    ) -> Result<usize, ParseError> {
        let phase = lines
            .get(pos)
            .ok_or_else(|| ParseError("missing phase".into()))?
            .trim();
        pos += 1;
        self.m_game.m_phase = parse_phase(phase)?;
        for expected in 0..2 {
            let header = lines
                .get(pos)
                .ok_or_else(|| ParseError("missing player".into()))?
                .split_whitespace()
                .collect::<Vec<_>>();
            pos += 1;
            if header.len() < 4 || header[0] != "player" {
                return Err(ParseError(format!(
                    "missing player: {}",
                    lines[pos - 1].trim()
                )));
            }
            let id = parse_usize(header[1])?;
            let count = parse_usize(header[2])?;
            validate_player_dice_count(count)?;
            let score: f32 = header[3]
                .parse()
                .map_err(|_| ParseError(format!("invalid score: {}", header[3])))?;
            if id != expected {
                return Err(ParseError(format!("expected player {expected}")));
            }
            let mut dice = Vec::with_capacity(count);
            // BMC_Parser::ParseDieSides updates this status as each die is
            // parsed.  A defined swing/option locks it; a later undefined one
            // changes it back to NOT.  The order matters for exact parser
            // state parity.
            let mut swing_set = BME_SWING_SET::NOT;
            for original_index in 0..count {
                let definition = lines
                    .get(pos)
                    .ok_or_else(|| ParseError("missing die".into()))?
                    .trim();
                pos += 1;
                let die = ParseDie(definition, original_index)?;
                if die.m_swing_type.iter().any(Option::is_some) || die.HasProperty(property::OPTION)
                {
                    swing_set = if ParseDieDefinedSides(
                        definition
                            .split_once(':')
                            .map_or(definition, |(text, _)| text),
                    )
                    .is_some_and(|value| value > 0)
                    {
                        BME_SWING_SET::LOCKED
                    } else {
                        BME_SWING_SET::NOT
                    };
                }
                dice.push(die);
            }
            self.m_game.m_player[id].m_die = dice;
            self.m_game.m_player[id].m_round_original_sides = [[0; 2]; crate::model::BMD_MAX_DICE];
            self.m_game.m_player[id].m_round_transformed = 0;
            self.m_game.m_player[id].m_radioactive_products = 0;
            self.m_game.m_player[id].m_rage_replacements = 0;
            self.m_game.m_player[id].m_score = if matches!(
                self.m_game.m_phase,
                BME_PHASE::INITIATIVE | BME_PHASE::CHANCE | BME_PHASE::FOCUS
            ) {
                self.m_game.m_player[id]
                    .m_die
                    .iter()
                    .filter(|die| !die.m_in_reserve)
                    .map(|die| die.GetScore(true))
                    .sum()
            } else {
                score
            };
            self.m_game.m_player[id].m_swing_set = swing_set;
            self.m_game.m_player[id].OptimizeDice();
            DebugPlayer(
                &self.m_game.m_player[id],
                self.m_game.m_phase == BME_PHASE::PREROUND,
                output,
            )?;
        }
        // BMC_Parser::ParseGame always restores both game AI pointers to the
        // global BMAI3 instance, while retaining its global search settings.
        self.m_ai_type = [2, 2];
        self.m_ai_explicit = [false, false];
        self.m_player_ai = std::array::from_fn(|_| self.m_ai.clone());
        Ok(pos)
    }

    fn GetAction<W: Write>(&mut self, output: &mut W) -> Result<(), ParseError> {
        match self.m_game.m_phase {
            BME_PHASE::FIGHT => {
                let moves = self.m_game.GenerateValidAttacks();
                if self.m_ai_type[0] != 1 {
                    writeln!(
                        output,
                        "l1 p0 Valid Moves {} Sims {}",
                        moves.len(),
                        self.m_player_ai[0].ComputeNumberSims(moves.len().max(1), 1)
                    )
                    .map_err(io_error)?;
                }
                let (action, search) = if self.m_ai_type[0] == 1 {
                    (SelectQAIAction(&self.m_game, &mut self.m_rng), None)
                } else if self.m_execution_mode == ExecutionMode::Native {
                    let replay = self.NextNativeReplay();
                    let result = SelectNativeBMAIActionWithStats(
                        &self.m_game,
                        self.m_rng.Algorithm(),
                        replay,
                        self.m_native_workers,
                        &self.m_player_ai[0],
                    );
                    let summary = (result.m_best_score, result.ProbabilityWin());
                    (result.m_move, Some(summary))
                } else {
                    let result = SelectBMAIActionWithStats(
                        &self.m_game,
                        &mut self.m_rng,
                        &self.m_player_ai[0],
                    );
                    let summary = (result.m_best_score, result.ProbabilityWin());
                    (result.m_move, Some(summary))
                };
                self.m_last_action = Some(protocol_attack(&self.m_game, &action)?);
                if let Some((best_score, probability_win)) = search {
                    writeln!(
                        output,
                        "l1 p0 best move ({:.1} points, {:.1}% win)",
                        best_score,
                        probability_win * 100.0
                    )
                    .map_err(io_error)?;
                }
                self.SendStats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                SendAttack(&self.m_game, &action, output)
            }
            BME_PHASE::RESERVE => {
                let reserve = if self.m_ai_type[0] == 1 {
                    SelectQAIReserveAction(&self.m_game)
                } else if self.m_execution_mode == ExecutionMode::Native {
                    let replay = self.NextNativeReplay();
                    SelectNativeBMAIReserveAction(
                        &self.m_game,
                        self.m_rng.Algorithm(),
                        replay,
                        self.m_native_workers,
                        &self.m_player_ai[0],
                    )
                } else {
                    SelectBMAIReserveAction(&self.m_game, &mut self.m_rng, &self.m_player_ai[0])
                };
                self.m_last_action = Some(crate::protocol::ProtocolAction::Reserve {
                    die: reserve.map(|index| self.m_game.m_player[0].m_die[index].m_original_index),
                });
                self.SendStats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                if let Some(index) = reserve {
                    writeln!(
                        output,
                        "reserve {}",
                        self.m_game.m_player[0].m_die[index].m_original_index
                    )
                    .map_err(io_error)
                } else {
                    writeln!(output, "reserve -1").map_err(io_error)
                }
            }
            BME_PHASE::PREROUND => {
                let action = if self.m_ai_type[0] == 1 {
                    SelectQAISetSwingAction(&self.m_game)
                } else if self.m_execution_mode == ExecutionMode::Native {
                    let replay = self.NextNativeReplay();
                    SelectNativeBMAISetSwingAction(
                        &self.m_game,
                        self.m_rng.Algorithm(),
                        replay,
                        self.m_native_workers,
                        &self.m_player_ai[0],
                    )
                } else {
                    SelectBMAISetSwingAction(&self.m_game, &mut self.m_rng, &self.m_player_ai[0])
                };
                self.m_last_action = Some(protocol_swing(&self.m_game, &action));
                self.SendStats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                SendSetSwing(&self.m_game, &action, output)
            }
            BME_PHASE::CHANCE => {
                if self.m_ai_type[0] == 1 {
                    self.m_last_action = Some(crate::protocol::ProtocolAction::Pass);
                    self.SendStats(output)?;
                    writeln!(output, "action\npass").map_err(io_error)?;
                    return Ok(());
                }
                let action = if self.m_execution_mode == ExecutionMode::Native {
                    let replay = self.NextNativeReplay();
                    SelectNativeBMAIChanceAction(
                        &self.m_game,
                        self.m_rng.Algorithm(),
                        replay,
                        self.m_native_workers,
                        &self.m_player_ai[0],
                    )
                } else {
                    SelectBMAIChanceAction(&self.m_game, &mut self.m_rng, &self.m_player_ai[0])
                };
                self.m_last_action = Some(protocol_chance(&self.m_game, &action));
                self.SendStats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                if action.reroll.is_empty() {
                    writeln!(output, "pass").map_err(io_error)
                } else {
                    for index in action.reroll {
                        writeln!(
                            output,
                            "chance {}",
                            self.m_game.m_player[0].m_die[index].m_original_index
                        )
                        .map_err(io_error)?;
                    }
                    Ok(())
                }
            }
            BME_PHASE::FOCUS => {
                if self.m_ai_type[0] == 1 {
                    self.m_last_action = Some(crate::protocol::ProtocolAction::Pass);
                    self.SendStats(output)?;
                    writeln!(output, "action\npass").map_err(io_error)?;
                    return Ok(());
                }
                let action = if self.m_execution_mode == ExecutionMode::Native {
                    let replay = self.NextNativeReplay();
                    SelectNativeBMAIFocusAction(
                        &self.m_game,
                        self.m_rng.Algorithm(),
                        replay,
                        self.m_native_workers,
                        &self.m_player_ai[0],
                    )
                } else {
                    SelectBMAIFocusAction(&self.m_game, &mut self.m_rng, &self.m_player_ai[0])
                };
                self.m_last_action = Some(protocol_focus(&self.m_game, &action));
                self.SendStats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                if action.values.is_empty() {
                    writeln!(output, "pass").map_err(io_error)
                } else {
                    for (index, value) in action.values {
                        writeln!(
                            output,
                            "focus {} {value}",
                            self.m_game.m_player[0].m_die[index].m_original_index
                        )
                        .map_err(io_error)?;
                    }
                    Ok(())
                }
            }
            _ => Err(ParseError("GetAction(): Unrecognized phase".into())),
        }
    }

    fn NextNativeReplay(&mut self) -> crate::native::NativeReplayKey {
        debug_assert_eq!(self.m_execution_mode, ExecutionMode::Native);
        let replay = crate::native::NativeReplayKey {
            stream_version: crate::native::NativeStreamVersion::CURRENT,
            root_seed: self.m_native_root_seed,
            decision_index: self.m_native_decision_index,
        };
        self.m_last_replay = Some(crate::protocol::ReplayMetadata {
            stream_partition: replay.stream_version.partition_id(),
            root_seed: replay.root_seed,
            decision_index: replay.decision_index,
        });
        self.m_native_decision_index = self.m_native_decision_index.wrapping_add(1);
        replay
    }

    fn SendStats<W: Write>(&self, output: &mut W) -> Result<(), ParseError> {
        writeln!(
            output,
            "stats {}/{}-{}/{}/0.50",
            self.m_max_ply, self.m_min_sims, self.m_max_sims, self.m_max_branch
        )
        .map_err(io_error)
    }
}

fn ParseDie(input: &str, original_index: usize) -> Result<BMC_Die, ParseError> {
    let (definition, value_part) = input
        .split_once(':')
        .map_or((input, None), |(a, b)| (a, Some(b)));
    let mut properties = property::VALID;
    let mut pos = 0;
    let chars: Vec<char> = definition.chars().collect();
    while pos < chars.len()
        && !chars[pos].is_ascii_digit()
        && !(('P'..='Z').contains(&chars[pos]))
        && chars[pos] != '('
    {
        properties |= prefix_property(chars[pos])
            .ok_or_else(|| ParseError(format!("error parsing die {input} at {}", chars[pos])))?;
        pos += 1;
    }
    let mut sides = [0u8; 2];
    let mut swings = [None; 2];
    if chars.get(pos) == Some(&'(') {
        properties |= property::TWIN;
        pos += 1;
        (sides[0], swings[0], pos) = parse_side(&chars, pos)?;
        if chars.get(pos) != Some(&',') {
            return Err(ParseError(format!("invalid twin die: {input}")));
        }
        pos += 1;
        (sides[1], swings[1], pos) = parse_side(&chars, pos)?;
        if chars.get(pos) != Some(&')') {
            return Err(ParseError(format!("invalid twin die: {input}")));
        }
        pos += 1;
    } else {
        (sides[0], swings[0], pos) = parse_side(&chars, pos)?;
        if chars.get(pos) == Some(&'/') {
            properties |= property::OPTION;
            pos += 1;
            (sides[1], swings[1], pos) = parse_side(&chars, pos)?;
        }
    }
    while pos < chars.len() {
        match chars[pos] {
            '!' => properties |= property::TURBO,
            '?' => properties |= property::MOOD,
            '-' => {
                pos += 1;
                while pos < chars.len() && chars[pos].is_ascii_digit() {
                    pos += 1;
                }
                continue;
            }
            ch => return Err(ParseError(format!("error parsing die {input} at {ch}"))),
        }
        pos += 1;
    }
    // C++ ParseDieSides looks ahead to the shared `-N` suffix separately for
    // every Swing side. This matters for Twin Swing dice such as `(T,T)-2`:
    // both halves are d2, not only the first half.
    if let Some(value) = ParseDieDefinedSides(definition).filter(|value| *value > 0) {
        for side in 0..2 {
            if swings[side].is_some() {
                sides[side] = value;
            }
        }
        if properties & property::OPTION != 0 && sides[1] == value {
            sides.swap(0, 1);
            swings.swap(0, 1);
        }
    }
    let m_value_total = value_part
        .map(|v| {
            v.trim_end_matches('d')
                .parse::<u8>()
                .map_err(|_| ParseError(format!("invalid die value: {input}")))
        })
        .transpose()?;
    Ok(BMC_Die {
        m_properties: properties,
        m_sides: sides,
        m_swing_type: swings,
        m_value_total,
        m_captured: false,
        m_notset: m_value_total.is_none() && properties & property::RESERVE == 0,
        m_dizzy: value_part.is_some_and(|value| value.ends_with('d')),
        m_original_index: original_index,
        m_in_reserve: properties & property::RESERVE != 0,
    })
}

/// Mirrors C++ `ParseDieDefinedSides`: find the shared `-N` suffix and parse
/// its leading digits. Postfix properties may appear before or after it.
fn ParseDieDefinedSides(definition: &str) -> Option<u8> {
    let (_, suffix) = definition.split_once('-')?;
    let digit_count = suffix.bytes().take_while(u8::is_ascii_digit).count();
    suffix.get(..digit_count)?.parse().ok()
}

fn parse_side(chars: &[char], mut pos: usize) -> Result<(u8, Option<char>, usize), ParseError> {
    if let Some(ch @ 'P'..='Z') = chars.get(pos).copied() {
        return Ok((0, Some(ch), pos + 1));
    }
    let start = pos;
    while pos < chars.len() && chars[pos].is_ascii_digit() {
        pos += 1;
    }
    if start == pos {
        return Err(ParseError("expected die sides".into()));
    }
    let value = chars[start..pos]
        .iter()
        .collect::<String>()
        .parse()
        .map_err(|_| ParseError("invalid die sides".into()))?;
    Ok((value, None, pos))
}

fn read_stream_line<R: BufRead>(input: &mut R) -> Result<Option<String>, ParseError> {
    let mut line = String::new();
    match input.read_line(&mut line).map_err(io_error)? {
        0 => Ok(None),
        _ => Ok(Some(line)),
    }
}

fn read_required_stream_line<R: BufRead>(
    input: &mut R,
    missing: &'static str,
) -> Result<String, ParseError> {
    read_stream_line(input)?.ok_or_else(|| ParseError(missing.into()))
}

fn prefix_property(ch: char) -> Option<u64> {
    crate::notation::DIE_PROPERTY_PREFIXES
        .iter()
        .find(|notation| notation.token == ch)
        .map(|notation| notation.property)
}

fn DebugPlayer<W: Write>(
    player: &crate::model::BMC_Player,
    all: bool,
    output: &mut W,
) -> Result<(), ParseError> {
    write!(output, "p{} s{:.1} Dice ", player.m_id, player.m_score).map_err(io_error)?;
    for die in &player.m_die {
        if !all && !die.IsAvailable() {
            continue;
        }
        write!(output, "({:x})", die.m_properties & !property::VALID).map_err(io_error)?;
        if die.HasProperty(property::TWIN) {
            write!(output, "({},{})", die.m_sides[0], die.m_sides[1]).map_err(io_error)?;
        } else {
            write!(output, "{}", die.m_sides[0]).map_err(io_error)?;
        }
        if let Some(value) = die.m_value_total {
            write!(output, ":{value} ").map_err(io_error)?;
        } else {
            write!(output, " ").map_err(io_error)?;
        }
    }
    writeln!(output).map_err(io_error)
}

fn SendAttack<W: Write>(
    game: &BMC_Game,
    action: &BMC_Move,
    output: &mut W,
) -> Result<(), ParseError> {
    match action.m_action {
        BME_ACTION::PASS => writeln!(output, "pass").map_err(io_error),
        BME_ACTION::SURRENDER => writeln!(output, "surrender").map_err(io_error),
        BME_ACTION::ATTACK => {
            writeln!(
                output,
                "{}",
                action.m_attack.expect("attack kind").protocol()
            )
            .map_err(io_error)?;
            write_indices(&game.m_player[0], &action.m_attackers, output)?;
            write_indices(&game.m_player[1], &action.m_targets, output)?;
            if action.m_turbo_option >= 0
                && let Some(die) = game.m_player[0]
                    .m_die
                    .iter()
                    .find(|die| die.IsAvailable() && die.HasProperty(property::TURBO))
            {
                if die.HasProperty(property::OPTION) {
                    writeln!(
                        output,
                        "option {} {}",
                        die.m_original_index, die.m_sides[action.m_turbo_option as usize]
                    )
                    .map_err(io_error)?;
                } else if let Some(swing) = die.m_swing_type[0] {
                    writeln!(output, "swing {swing} {}", action.m_turbo_option)
                        .map_err(io_error)?;
                }
            }
            Ok(())
        }
        _ => Err(ParseError("invalid fight action".into())),
    }
}

fn protocol_attack(
    game: &BMC_Game,
    action: &BMC_Move,
) -> Result<crate::protocol::ProtocolAction, ParseError> {
    match action.m_action {
        BME_ACTION::PASS => Ok(crate::protocol::ProtocolAction::Pass),
        BME_ACTION::SURRENDER => Ok(crate::protocol::ProtocolAction::Surrender),
        BME_ACTION::ATTACK => {
            let attack_type = action
                .m_attack
                .ok_or_else(|| ParseError("attack has no attack type".into()))?
                .protocol();
            let original_indices = |player: usize, indices: &BMC_DieIndexSet| {
                indices
                    .iter()
                    .map(|index| game.m_player[player].m_die[index].m_original_index)
                    .collect::<Vec<_>>()
            };
            let turbo = if action.m_turbo_option < 0 {
                None
            } else {
                game.m_player[0]
                    .m_die
                    .iter()
                    .find(|die| die.IsAvailable() && die.HasProperty(property::TURBO))
                    .and_then(|die| {
                        if die.HasProperty(property::OPTION) {
                            Some(crate::protocol::TurboSelection::Option {
                                die: die.m_original_index,
                                value: die.m_sides[action.m_turbo_option as usize],
                            })
                        } else {
                            die.m_swing_type[0].map(|swing| {
                                crate::protocol::TurboSelection::Swing {
                                    swing,
                                    value: action.m_turbo_option as u8,
                                }
                            })
                        }
                    })
            };
            Ok(crate::protocol::ProtocolAction::Attack {
                attack_type,
                attackers: original_indices(0, &action.m_attackers),
                targets: original_indices(1, &action.m_targets),
                turbo,
            })
        }
        _ => Err(ParseError("invalid fight action".into())),
    }
}

fn protocol_swing(game: &BMC_Game, action: &SwingMove) -> crate::protocol::ProtocolAction {
    let swings = action
        .values()
        .iter()
        .map(|(swing, value)| crate::protocol::SwingSelection {
            swing: *swing,
            value: *value,
        })
        .collect::<Vec<_>>();
    let options = action
        .options()
        .iter()
        .map(|(index, second)| {
            let die = &game.m_player[0].m_die[*index];
            crate::protocol::OptionSelection {
                die: die.m_original_index,
                value: die.m_sides[usize::from(*second)],
            }
        })
        .collect::<Vec<_>>();
    if swings.is_empty() && options.is_empty() {
        crate::protocol::ProtocolAction::Pass
    } else {
        crate::protocol::ProtocolAction::SetSwing { swings, options }
    }
}

fn protocol_chance(
    game: &BMC_Game,
    action: &crate::simulation::ChanceMove,
) -> crate::protocol::ProtocolAction {
    if action.reroll.is_empty() {
        crate::protocol::ProtocolAction::Pass
    } else {
        crate::protocol::ProtocolAction::Chance {
            dice: action
                .reroll
                .iter()
                .map(|index| game.m_player[0].m_die[*index].m_original_index)
                .collect(),
        }
    }
}

fn protocol_focus(
    game: &BMC_Game,
    action: &crate::simulation::FocusMove,
) -> crate::protocol::ProtocolAction {
    if action.values.is_empty() {
        crate::protocol::ProtocolAction::Pass
    } else {
        crate::protocol::ProtocolAction::Focus {
            dice: action
                .values
                .iter()
                .map(|(index, value)| crate::protocol::FocusSelection {
                    die: game.m_player[0].m_die[*index].m_original_index,
                    value: *value,
                })
                .collect(),
        }
    }
}

fn phase_protocol(phase: BME_PHASE) -> &'static str {
    match phase {
        BME_PHASE::PREROUND => "preround",
        BME_PHASE::RESERVE => "reserve",
        BME_PHASE::INITIATIVE => "initiative",
        BME_PHASE::CHANCE => "chance",
        BME_PHASE::FOCUS => "focus",
        BME_PHASE::FIGHT => "fight",
        BME_PHASE::GAMEOVER => "gameover",
    }
}

fn parse_phase(phase: &str) -> Result<BME_PHASE, ParseError> {
    match phase {
        "preround" => Ok(BME_PHASE::PREROUND),
        "reserve" => Ok(BME_PHASE::RESERVE),
        "initiative" => Ok(BME_PHASE::INITIATIVE),
        "chance" => Ok(BME_PHASE::CHANCE),
        "focus" => Ok(BME_PHASE::FOCUS),
        "fight" => Ok(BME_PHASE::FIGHT),
        "gameover" => Ok(BME_PHASE::GAMEOVER),
        _ => Err(ParseError("phase not found".into())),
    }
}

fn write_indices<W: Write>(
    player: &crate::model::BMC_Player,
    indices: &BMC_DieIndexSet,
    output: &mut W,
) -> Result<(), ParseError> {
    for (n, index) in indices.iter().enumerate() {
        if n > 0 {
            write!(output, " ").map_err(io_error)?;
        }
        write!(output, "{}", player.m_die[index].m_original_index).map_err(io_error)?;
    }
    writeln!(output).map_err(io_error)
}

fn SendSetSwing<W: Write>(
    game: &BMC_Game,
    action: &SwingMove,
    output: &mut W,
) -> Result<(), ParseError> {
    let player = &game.m_player[0];
    let mut sent = false;
    for (swing, value) in action.values() {
        writeln!(output, "swing {swing} {value}").map_err(io_error)?;
        sent = true;
    }
    for (index, second) in action.options() {
        let die = &player.m_die[*index];
        let selected = die.m_sides[usize::from(*second)];
        writeln!(output, "option {} {selected}", die.m_original_index).map_err(io_error)?;
        sent = true;
    }
    if !sent {
        writeln!(output, "pass").map_err(io_error)?;
    }
    Ok(())
}

fn argument(line: &str, command: &str) -> Option<Result<usize, ParseError>> {
    line.strip_prefix(command)
        .and_then(|rest| rest.strip_prefix(' '))
        .map(parse_usize)
}
fn two_usize_arguments(line: &str, command: &str) -> Result<Option<(usize, usize)>, ParseError> {
    let Some(rest) = line
        .strip_prefix(command)
        .and_then(|rest| rest.strip_prefix(' '))
    else {
        return Ok(None);
    };
    let values = rest.split_whitespace().collect::<Vec<_>>();
    if values.len() != 2 {
        return Ok(None);
    }
    Ok(Some((parse_usize(values[0])?, parse_usize(values[1])?)))
}
fn playfair_arguments(line: &str) -> Result<Option<(usize, usize, f32)>, ParseError> {
    let Some(rest) = line.strip_prefix("playfair ") else {
        return Ok(None);
    };
    let values = rest.split_whitespace().collect::<Vec<_>>();
    if values.len() != 3 {
        return Ok(None);
    }
    let probability = values[2]
        .parse()
        .map_err(|_| ParseError(format!("invalid float: {}", values[2])))?;
    Ok(Some((
        parse_usize(values[0])?,
        parse_usize(values[1])?,
        probability,
    )))
}
fn parse_usize(input: &str) -> Result<usize, ParseError> {
    input
        .parse()
        .map_err(|_| ParseError(format!("invalid integer: {input}")))
}
fn validate_player_dice_count(count: usize) -> Result<(), ParseError> {
    if count > crate::model::BMD_MAX_INPUT_DICE {
        return Err(ParseError(format!(
            "player dice count {count} exceeds maximum {}",
            crate::model::BMD_MAX_INPUT_DICE
        )));
    }
    Ok(())
}
fn available_workers() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
}
fn io_error(error: std::io::Error) -> ParseError {
    ParseError(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BME_ATTACK::{POWER, SKILL};
    use crate::BME_PHASE::{CHANCE, FOCUS};
    use crate::simulation::scenario::{LEGACY, parser_scenario, search_scenario};

    /// Port of ParserTests.ParseString.
    #[test]
    fn cpp_parser_multiline_fight_string() {
        let input = "\n\
game\n\
fight\n\
player 0 3 0\n\
30:16\n\
30:16\n\
6/30-6:5\n\
player 1 3 0\n\
5/7-5:4\n\
7/9-7:3\n\
9/11-9:3\n\
getaction\n";
        BMC_Parser::default()
            .ParseString(input, &mut Vec::new())
            .unwrap();
    }

    #[test]
    fn streamed_legacy_commands_match_batched_parsing() {
        let input = "game\nfight\nplayer 0 1 1\n1:1\nplayer 1 2 30\n1:1\n(30,30):60\nseed 17\nsurrender off\ngetaction\nquit\n";
        let mut batched = BMC_Parser::default();
        let mut batched_output = Vec::new();
        batched.ParseString(input, &mut batched_output).unwrap();

        let mut streamed = BMC_Parser::default();
        let mut streamed_output = Vec::new();
        streamed
            .ParseStream(&mut std::io::Cursor::new(input), &mut streamed_output)
            .unwrap();

        assert_eq!(streamed_output, batched_output);
        assert_eq!(streamed.session_metadata(), batched.session_metadata());
        assert_eq!(streamed.last_action(), batched.last_action());
        assert_eq!(streamed.last_replay(), batched.last_replay());
    }

    #[test]
    fn whole_line_comments_are_ignored_between_top_level_commands() {
        let game = "game\nfight\nplayer 0 1 1\n1:1\nplayer 1 1 1\n1:1\n";
        let plain = format!("{game}seed 17\nsurrender off\nquit\n");
        let commented = format!(
            "  # generated game state\n{game}\t# search policy\nseed 17\nsurrender off\n# done\nquit\n"
        );

        let parse_batched = |input: &str| {
            let mut parser = BMC_Parser::default();
            let mut output = Vec::new();
            parser.ParseString(input, &mut output).unwrap();
            (output, parser.session_metadata())
        };
        let parse_streamed = |input: &str| {
            let mut parser = BMC_Parser::default();
            let mut output = Vec::new();
            parser
                .ParseStream(&mut std::io::Cursor::new(input), &mut output)
                .unwrap();
            (output, parser.session_metadata())
        };

        let expected = parse_batched(&plain);
        assert_eq!(parse_batched(&commented), expected);
        assert_eq!(parse_streamed(&commented), expected);
    }

    #[test]
    fn comments_inside_game_blocks_remain_invalid() {
        let input = "game\n# phase comment\nfight\n";
        let batched = BMC_Parser::default()
            .ParseString(input, &mut Vec::new())
            .unwrap_err();
        let streamed = BMC_Parser::default()
            .ParseStream(&mut std::io::Cursor::new(input), &mut Vec::new())
            .unwrap_err();

        assert_eq!(batched.to_string(), "phase not found");
        assert_eq!(streamed.to_string(), batched.to_string());
    }

    #[test]
    fn inline_comments_remain_invalid() {
        let error = BMC_Parser::default()
            .ParseString("seed 17 # comment\n", &mut Vec::new())
            .unwrap_err();

        assert_eq!(error.to_string(), "invalid integer: 17 # comment");
    }

    #[test]
    fn streamed_game_rejects_malformed_structure_without_waiting_for_more_input() {
        for (input, expected) in [
            ("game\ninvalid-phase\n", "phase not found"),
            ("game\nfight\nplayer 0 1\n", "missing player: player 0 1"),
            (
                "game\nfight\nplayer 0 1 invalid\n",
                "invalid score: invalid",
            ),
            (
                "game\nfight\nplayer 0 2 1\nnot-a-die\n",
                "error parsing die not-a-die at -",
            ),
        ] {
            let mut parser = BMC_Parser::default();
            let error = parser
                .ParseStream(&mut std::io::Cursor::new(input), &mut Vec::new())
                .unwrap_err();
            assert_eq!(error.to_string(), expected);
        }
    }

    #[test]
    fn game_rejects_more_than_ten_input_dice_in_both_parsing_paths() {
        let input = "game\nfight\nplayer 0 11 0\n";
        let expected = "player dice count 11 exceeds maximum 10";

        let batched = BMC_Parser::default()
            .ParseString(input, &mut Vec::new())
            .unwrap_err();
        let streamed = BMC_Parser::default()
            .ParseStream(&mut std::io::Cursor::new(input), &mut Vec::new())
            .unwrap_err();

        assert_eq!(batched.to_string(), expected);
        assert_eq!(streamed.to_string(), expected);
    }

    #[test]
    fn cpp_parser_ai_and_per_player_search_settings() {
        let input = "ai 0 2\nply 0 3\nmax_sims 0 40\nmin_sims 0 4\nmaxbranch 0 400\ndebugply 2\nseed 17\nquit\n";
        let mut parser = BMC_Parser::default();
        let mut output = Vec::new();
        parser.ParseString(input, &mut output).unwrap();
        assert_eq!(parser.m_ai_type[0], 2);
        assert_eq!(parser.m_player_ai[0].m_max_ply, 3);
        assert_eq!(parser.m_player_ai[0].m_max_sims, 40);
        assert_eq!(parser.m_player_ai[0].m_min_sims, 4);
        assert_eq!(parser.m_player_ai[0].m_max_branch, 400);
        assert_eq!(parser.m_debug_ply, 2);
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "Setting AI for player 0 to type 2\n\
Setting max ply for player 0 to 3\n\
Setting max sims for player 0 to 40\n\
Setting min sims for player 0 to 4\n\
Setting max branch for player 0 to 400\n\
Setting debug ply to 2\n\
Seeding with 17\n"
        );
    }

    #[test]
    fn cpp_parser_rejects_invalid_ai_selection() {
        for (input, expected) in [
            ("ai 2 0\n", "invalid setting for ai player number: 2"),
            ("ai 0 3\n", "invalid setting for ai type: 3"),
            ("ai 2 3\n", "invalid setting for ai type: 3"),
        ] {
            assert_eq!(
                BMC_Parser::default()
                    .ParseString(input, &mut Vec::new())
                    .unwrap_err()
                    .to_string(),
                expected
            );
        }
    }

    #[test]
    fn cpp_playfair_out_of_range_mode_retains_current_ai() {
        let input = "game 1\npreround\nplayer 0 1 0\n6\nplayer 1 1 0\n6\nplayfair 0 4 0.5\nquit\n";
        let mut output = Vec::new();
        BMC_Parser::default()
            .ParseString(input, &mut output)
            .unwrap();
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("PlayFairGames: 0 games, mode 4")
        );
    }

    #[test]
    fn cpp_debug_command_validates_category_and_reports_setting() {
        let mut output = Vec::new();
        BMC_Parser::default()
            .ParseString("debug SIMULATION 0\n", &mut output)
            .unwrap();
        assert_eq!(output, b"Debug SIMULATION set to 0\n");

        let mut output = Vec::new();
        BMC_Parser::default()
            .ParseString("debug SIMULATION -1\n", &mut output)
            .unwrap();
        assert_eq!(output, b"Debug SIMULATION set to 1\n");

        let mut output = Vec::new();
        BMC_Parser::default()
            .ParseString("debug ALWAYS 0\n", &mut output)
            .unwrap();
        assert!(output.is_empty());

        let error = BMC_Parser::default()
            .ParseString("debug simulation 0\n", &mut Vec::new())
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Could not find debug category: simulation"
        );
    }

    #[test]
    fn cpp_qai_selection_drives_getaction_and_ignores_bmai_settings() {
        let input = "game 1\nfight\nplayer 0 2 0\n6:5\n6:1\nplayer 1 1 0\n20:6\nai 0 1\nmax_sims 0 5\nmin_sims 0 1\nmaxbranch 0 20\nseed 17\ngetaction\nquit\n";
        let mut output = Vec::new();
        BMC_Parser::default()
            .ParseString(input, &mut output)
            .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(!output.contains("Setting max sims for player"), "{output}");
        assert!(!output.contains("Valid Moves"), "{output}");
        assert!(output.ends_with("action\nskill\n0 1\n0\n"), "{output}");
    }

    #[test]
    fn cpp_playfair_modes_report_initiative_split_stats() {
        for mode in 0..=3 {
            let input = format!(
                "game 1\npreround\nplayer 0 1 0\n2\nplayer 1 1 0\n2\nseed 17\nplayfair 2 {mode} 0.5\nquit\n"
            );
            let mut output = Vec::new();
            BMC_Parser::default()
                .ParseString(&input, &mut output)
                .unwrap();
            let output = String::from_utf8(output).unwrap();
            assert!(
                output.contains(&format!("PlayFairGames: 2 games, mode {mode}, p 0.500000")),
                "{output}"
            );
            assert_eq!(output.matches(" stats: initiative ").count(), 4, "{output}");
        }
    }

    #[test]
    fn cpp_game_parse_restores_default_ai_and_accepts_gameover_phase() {
        let input = "ai 0 1\ngame 3\ngameover\nplayer 0 0 0\nplayer 1 0 0\nquit\n";
        let mut parser = BMC_Parser::default();
        parser.ParseString(input, &mut Vec::new()).unwrap();
        assert_eq!(parser.m_game.m_phase, BME_PHASE::GAMEOVER);
        assert_eq!(parser.m_ai_type, [2, 2]);
        assert_eq!(parser.m_ai_explicit, [false, false]);
    }

    #[test]
    fn cpp_game_simulation_commands_require_preround() {
        let prefix = "game 1\nfight\nplayer 0 1 0\n6:6\nplayer 1 1 0\n6:6\n";
        for command in ["playgame 1", "compare 1", "playfair 1 0 0.5"] {
            let error = BMC_Parser::default()
                .ParseString(&format!("{prefix}{command}\n"), &mut Vec::new())
                .unwrap_err();
            assert_eq!(error.to_string(), "Cannot PlayGame unless it is preround");
        }
    }

    #[test]
    fn parses_twin_option_and_properties() {
        let twin = ParseDie("p(4,4):4", 0).unwrap();
        assert!(twin.HasProperty(property::TWIN));
        assert!(twin.HasProperty(property::POISON));
        assert_eq!(twin.m_sides, [4, 4]);

        let first_option = ParseDie("6/20-6:4", 1).unwrap();
        assert!(first_option.HasProperty(property::OPTION));
        assert_eq!(first_option.m_sides, [6, 20]);

        let second_option = ParseDie("6/20-20:17", 2).unwrap();
        assert!(second_option.HasProperty(property::OPTION));
        assert_eq!(second_option.m_sides, [20, 6]);

        let mood_swing = ParseDie("zU?-30", 3).unwrap();
        assert!(mood_swing.HasProperty(property::SPEED | property::MOOD));
        assert_eq!(mood_swing.m_sides[0], 30);

        let jolt = ParseDie("J^6:3", 4).unwrap();
        assert!(jolt.HasProperty(property::JOLT | property::TIME_AND_SPACE));
    }

    #[test]
    fn every_advertised_property_prefix_is_accepted_by_the_die_parser() {
        for notation in crate::notation::DIE_PROPERTY_PREFIXES {
            let recipe = format!("{}6", notation.token);
            let die = ParseDie(&recipe, 0)
                .unwrap_or_else(|error| panic!("{} did not parse: {error}", notation.name));
            assert!(
                die.HasProperty(notation.property),
                "{} ({:?}) was not retained by {recipe}",
                notation.name,
                notation.token
            );
        }
    }

    #[test]
    fn every_wire_phase_name_maps_to_the_expected_game_phase() {
        for (name, expected) in [
            ("preround", BME_PHASE::PREROUND),
            ("reserve", BME_PHASE::RESERVE),
            ("initiative", BME_PHASE::INITIATIVE),
            ("chance", BME_PHASE::CHANCE),
            ("focus", BME_PHASE::FOCUS),
            ("fight", BME_PHASE::FIGHT),
            ("gameover", BME_PHASE::GAMEOVER),
        ] {
            let input = format!("game 5\n{name}\nplayer 0 0 0\nplayer 1 0 0\nquit\n");
            let mut parser = BMC_Parser::default();
            parser
                .ParseString(&input, &mut Vec::new())
                .unwrap_or_else(|error| panic!("{name} did not parse: {error}"));
            assert_eq!(parser.m_game.m_phase, expected, "{name}");
            assert_eq!(parser.m_game.m_target_wins, 5, "{name}");
        }
    }

    #[test]
    fn current_value_and_focus_dizzy_suffix_are_parsed_explicitly() {
        let ready = ParseDie("6:4", 2).unwrap();
        assert_eq!(ready.m_value_total, Some(4));
        assert!(!ready.m_notset);
        assert!(!ready.m_dizzy);
        assert_eq!(ready.m_original_index, 2);

        let dizzy = ParseDie("f12:7d", 4).unwrap();
        assert_eq!(dizzy.m_value_total, Some(7));
        assert!(!dizzy.m_notset);
        assert!(dizzy.m_dizzy);
        assert_eq!(dizzy.m_original_index, 4);
    }

    #[test]
    fn defined_swing_size_applies_to_every_swing_half_of_a_twin() {
        for (recipe, expected_sides) in [
            ("(T,T)-2:2", [2, 2]),
            ("(X,Y)-6:6", [6, 6]),
            ("(T,4)-2:2", [2, 4]),
            ("(4,T)-2:2", [4, 2]),
            ("(T,T)-2!:2", [2, 2]),
            ("(T,T)!-2:2", [2, 2]),
            ("(T,T)-2?:2", [2, 2]),
            ("(T,T)?-2:2", [2, 2]),
        ] {
            let die = ParseDie(recipe, 0).unwrap();
            assert!(die.HasProperty(property::TWIN), "{recipe}");
            assert_eq!(die.m_sides, expected_sides, "{recipe}");
        }
    }

    #[test]
    fn zero_does_not_lock_a_swing_definition() {
        let input = "game 3\npreround\nplayer 0 1 0\nT-0\nplayer 1 1 0\n6\nquit\n";
        let mut parser = BMC_Parser::default();
        parser.ParseString(input, &mut Vec::new()).unwrap();
        assert_eq!(parser.m_game.m_player[0].m_swing_set, BME_SWING_SET::NOT);
    }

    #[test]
    fn positive_defined_swing_and_option_values_lock_player_state() {
        for recipe in ["T-2", "(T,T)-2", "6/20-20"] {
            let input =
                format!("game 3\npreround\nplayer 0 1 0\n{recipe}\nplayer 1 1 0\n6\nquit\n");
            let mut parser = BMC_Parser::default();
            parser
                .ParseString(&input, &mut Vec::new())
                .unwrap_or_else(|error| panic!("{recipe} did not parse: {error}"));
            assert_eq!(
                parser.m_game.m_player[0].m_swing_set,
                BME_SWING_SET::LOCKED,
                "{recipe}"
            );
        }
    }

    #[test]
    fn initiative_chance_and_focus_use_bmai_search() {
        for (phase, die) in [(CHANCE, "c10:10"), (FOCUS, "f10:10")] {
            search_scenario()
                .phase(phase)
                .player(0, 0.0, [die])
                .player(1, 0.0, ["6:6"])
                .ply(1)
                .simulations(1, 10)
                .max_branch(20)
                .modes([LEGACY])
                .expect_pass()
                .run();
        }
    }

    #[test]
    fn typed_initiative_actions_use_original_die_indices() {
        let mut parser = BMC_Parser::default();
        parser
            .ParseString(
                "game 3\nfocus\nplayer 0 2 0\nf10:10\nc8:8\nplayer 1 1 0\n6:6\n",
                &mut Vec::new(),
            )
            .unwrap();

        assert_eq!(
            protocol_focus(
                &parser.m_game,
                &crate::simulation::FocusMove {
                    values: vec![(0, 4)]
                }
            ),
            crate::protocol::ProtocolAction::Focus {
                dice: vec![crate::protocol::FocusSelection { die: 0, value: 4 }]
            }
        );
        assert_eq!(
            protocol_chance(
                &parser.m_game,
                &crate::simulation::ChanceMove { reroll: vec![1] }
            ),
            crate::protocol::ProtocolAction::Chance { dice: vec![1] }
        );
    }

    #[test]
    fn insult_fixture_emits_reference_protocol_action() {
        parser_scenario(include_str!("../tests/fixtures/Insult_in.txt"))
            .expect_attack(POWER)
            .using([0])
            .targeting([1])
            .run();
    }

    #[test]
    fn value_fixtures_emit_reference_protocol_actions() {
        let cases = [
            (
                include_str!("../tests/fixtures/Value1_in.txt"),
                SKILL,
                vec![0, 1],
                vec![1],
            ),
            (
                include_str!("../tests/fixtures/Value2_in.txt"),
                POWER,
                vec![1],
                vec![0],
            ),
        ];
        for (input, attack, attackers, targets) in cases {
            parser_scenario(input)
                .expect_attack(attack)
                .using(attackers)
                .targeting(targets)
                .run();
        }
    }

    #[test]
    fn deterministic_fight_fixtures_emit_reference_protocol_actions() {
        parser_scenario(include_str!("../tests/fixtures/bug55_a_in.txt"))
            .expect_attack(SKILL)
            .using([2, 0])
            .targeting([0])
            .run();
        parser_scenario(include_str!("../tests/fixtures/bug55_b_in.txt"))
            .expect_attack(SKILL)
            .using([3, 0, 1])
            .targeting([2])
            .run();
        parser_scenario(include_str!("../tests/fixtures/bug105372_in.txt"))
            .expect_attack(SKILL)
            .using([2, 1, 0])
            .targeting([0])
            .run();
        parser_scenario(include_str!("../tests/fixtures/SurrenderOff-Attack-in.txt"))
            .expect_attack(POWER)
            .using([0])
            .targeting([0])
            .run();
    }

    #[test]
    fn surrender_policy_fixtures_emit_reference_protocol_actions() {
        parser_scenario(include_str!(
            "../tests/fixtures/SurrenderDefault-Pass-in.txt"
        ))
        .expect_surrender()
        .run();
        parser_scenario(include_str!("../tests/fixtures/SurrenderOff-Pass-in.txt"))
            .expect_pass()
            .run();
        parser_scenario(include_str!("../tests/fixtures/SurrenderOn-Attack-in.txt"))
            .expect_surrender()
            .run();
        parser_scenario(include_str!("../tests/fixtures/SurrenderOn-Pass-in.txt"))
            .expect_surrender()
            .run();
    }

    #[test]
    #[ignore = "full BMAI3 searches; run in the release parity suite"]
    fn deeper_reference_fixtures_emit_reference_protocol_actions() {
        parser_scenario(include_str!("../tests/fixtures/bmai_in.txt"))
            .expect_attack(POWER)
            .using([1])
            .targeting([0])
            .run();

        parser_scenario(include_str!("../tests/fixtures/bug11_in.txt"))
            .expect_swings([('T', 2), ('W', 4)])
            .run();
    }

    #[test]
    fn obsolete_sims_command_is_rejected_like_reference_binary() {
        let input = include_str!("../tests/fixtures/test_in.txt");
        let error = BMC_Parser::default()
            .ParseString(input, &mut Vec::new())
            .unwrap_err();
        assert_eq!(error.to_string(), "unrecognized command: sims 150");
    }

    #[test]
    fn rust_execution_and_rng_modes_are_independent_and_versioned() {
        let mut parser = BMC_Parser::default();
        assert_eq!(parser.execution_mode(), ExecutionMode::Legacy);
        assert_eq!(
            parser.rng_algorithm(),
            BME_RNG_ALGORITHM::LEGACY_PARK_MILLER_V1
        );

        let mut output = Vec::new();
        parser
            .ParseString(
                "mode native\nrng park-miller\nseed 17\nmode parity\nrng legacy\nquit\n",
                &mut output,
            )
            .unwrap();
        assert_eq!(parser.execution_mode(), ExecutionMode::Legacy);
        assert_eq!(parser.rng_replay_id(), "bmai-park-miller-16807-v1");
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "Setting execution mode to native\n\
             Setting RNG to legacy (bmai-park-miller-16807-v1)\n\
             Seeding with 17\n\
             Setting execution mode to legacy\n\
             Setting RNG to legacy (bmai-park-miller-16807-v1)\n"
        );
    }

    #[test]
    fn rust_execution_and_rng_modes_reject_unknown_values() {
        let engine = BMC_Parser::default()
            .ParseString("mode experimental\n", &mut Vec::new())
            .unwrap_err();
        assert_eq!(
            engine.to_string(),
            "invalid execution mode: experimental (expected legacy or native)"
        );

        let rng = BMC_Parser::default()
            .ParseString("rng xoshiro\n", &mut Vec::new())
            .unwrap_err();
        assert_eq!(
            rng.to_string(),
            "invalid RNG algorithm: xoshiro (expected legacy or park-miller)"
        );
    }

    #[test]
    fn native_worker_setting_validates_input_and_does_not_change_legacy_search() {
        let zero = BMC_Parser::default()
            .ParseString("workers 0\n", &mut Vec::new())
            .unwrap_err();
        assert_eq!(zero.to_string(), "native worker count must be at least 1");

        let malformed = BMC_Parser::default()
            .ParseString("workers many\n", &mut Vec::new())
            .unwrap_err();
        assert_eq!(malformed.to_string(), "invalid integer: many");

        let fixture = include_str!("../tests/native-fixtures/fight.txt")
            .replace("mode native", "mode legacy");
        let run = |workers: usize| {
            let input = fixture.replace("workers 3", &format!("workers {workers}"));
            let mut output = Vec::new();
            BMC_Parser::default()
                .ParseString(&input, &mut output)
                .unwrap();
            String::from_utf8(output).unwrap().replace(
                &format!("Setting native workers to {workers}"),
                "Setting native workers to N",
            )
        };
        assert_eq!(run(1), run(64));
    }

    #[test]
    fn native_worker_auto_uses_available_logical_parallelism() {
        let expected = available_workers();
        let mut parser = BMC_Parser::default();
        let mut output = Vec::new();

        parser.ParseString("workers auto\n", &mut output).unwrap();

        assert_eq!(parser.session_metadata().workers, expected);
        assert_eq!(
            String::from_utf8(output).unwrap(),
            format!("Setting native workers to {expected} (auto)\n")
        );
    }

    #[test]
    fn native_replay_index_advances_only_for_native_bmai_searches() {
        let fixture = include_str!("../tests/native-fixtures/fight.txt");

        let mut qai = BMC_Parser::default();
        qai.ParseString(
            &fixture.replace("getaction", "ai 0 1\ngetaction"),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(qai.m_native_decision_index, 0);

        let mut bmai = BMC_Parser::default();
        bmai.ParseString(fixture, &mut Vec::new()).unwrap();
        assert_eq!(bmai.m_native_decision_index, 1);
    }

    #[test]
    fn native_wire_fixtures_are_deterministic() {
        let cases = [
            (
                include_str!("../tests/native-fixtures/fight.txt"),
                include_str!("../tests/native-fixtures/fight.out.txt"),
            ),
            (
                include_str!("../tests/native-fixtures/reserve.txt"),
                include_str!("../tests/native-fixtures/reserve.out.txt"),
            ),
            (
                include_str!("../tests/native-fixtures/preround.txt"),
                include_str!("../tests/native-fixtures/preround.out.txt"),
            ),
            (
                include_str!("../tests/native-fixtures/chance.txt"),
                include_str!("../tests/native-fixtures/chance.out.txt"),
            ),
            (
                include_str!("../tests/native-fixtures/focus.txt"),
                include_str!("../tests/native-fixtures/focus.out.txt"),
            ),
        ];

        for (input, expected) in cases {
            // Git may check text fixtures out with CRLF on Windows, while the
            // protocol writer deliberately emits `\n` on every platform.
            let expected = expected.replace("\r\n", "\n");
            for _ in 0..2 {
                let mut output = Vec::new();
                BMC_Parser::default()
                    .ParseString(input, &mut output)
                    .unwrap();
                assert_eq!(String::from_utf8(output).unwrap(), expected);
            }
        }
    }

    #[test]
    fn native_phases_are_worker_count_independent() {
        let run = |input: &str, workers: usize| {
            let input = input.replace("workers 3", &format!("workers {workers}"));
            let mut output = Vec::new();
            BMC_Parser::default()
                .ParseString(&input, &mut output)
                .unwrap();
            String::from_utf8(output).unwrap().replace(
                &format!("Setting native workers to {workers}"),
                "Setting native workers to N",
            )
        };
        let available = std::thread::available_parallelism().map_or(1, usize::from);
        for input in [
            include_str!("../tests/native-fixtures/reserve.txt"),
            include_str!("../tests/native-fixtures/preround.txt"),
            include_str!("../tests/native-fixtures/chance.txt"),
            include_str!("../tests/native-fixtures/focus.txt"),
        ] {
            let expected = run(input, 1);
            for workers in [2, available] {
                assert_eq!(run(input, workers), expected);
            }
        }
    }

    #[test]
    #[ignore = "full default BMAI3 simulation; run in the release parity suite"]
    fn simulation_fixture_emits_reference_match_result() {
        let input = include_str!("../tests/fixtures/bmsim_in.txt");
        let mut output = Vec::new();
        BMC_Parser::default()
            .ParseString(input, &mut output)
            .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.ends_with("matches over 12 - 8\n"), "{output}");
    }

    #[test]
    #[ignore = "full reserve BMAI3 search; run in the release parity suite"]
    fn reserve_fixture_emits_reference_protocol_action() {
        parser_scenario(include_str!("../tests/fixtures/bug16_in.txt"))
            .expect_reserve(Some(6))
            .run();
    }
}
