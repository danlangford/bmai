// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

impl Parser {
    pub(super) fn ParseStringCommands<W: Write>(
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
                let algorithm = RngAlgorithm::Parse(value).ok_or_else(|| {
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
                self.m_player_ai[player] = AiSlot::Type(value);
                writeln!(output, "Setting AI for player {player} to type {value}")
                    .map_err(io_error)?;
            } else if let Some((player, value)) = two_usize_arguments(line, "ply")? {
                if self.SetPlayerAI(player, |ai| ai.m_max_ply = value)? {
                    writeln!(output, "Setting max ply for player {player} to {value}")
                        .map_err(io_error)?;
                }
            } else if let Some(value) = argument(line, "ply") {
                self.m_ai.m_max_ply = value?;
                writeln!(output, "Setting max ply to {}", self.m_ai.m_max_ply).map_err(io_error)?;
            } else if let Some((player, value)) = two_usize_arguments(line, "max_sims")? {
                if self.SetPlayerAI(player, |ai| ai.m_max_sims = value)? {
                    writeln!(output, "Setting max sims for player {player} to {value}")
                        .map_err(io_error)?;
                }
            } else if let Some(value) = argument(line, "max_sims") {
                self.m_ai.m_max_sims = value?;
                writeln!(
                    output,
                    "Setting max # simulations to {}",
                    self.m_ai.m_max_sims
                )
                .map_err(io_error)?;
            } else if let Some((player, value)) = two_usize_arguments(line, "min_sims")? {
                if self.SetPlayerAI(player, |ai| ai.m_min_sims = value)? {
                    writeln!(output, "Setting min sims for player {player} to {value}")
                        .map_err(io_error)?;
                }
            } else if let Some(value) = argument(line, "min_sims") {
                self.m_ai.m_min_sims = value?;
                writeln!(
                    output,
                    "Setting min # simulations to {}",
                    self.m_ai.m_min_sims
                )
                .map_err(io_error)?;
            } else if let Some((player, value)) = two_usize_arguments(line, "maxbranch")? {
                if self.SetPlayerAI(player, |ai| ai.m_max_branch = value)? {
                    writeln!(output, "Setting max branch for player {player} to {value}")
                        .map_err(io_error)?;
                }
            } else if let Some(value) = argument(line, "maxbranch") {
                self.m_ai.m_max_branch = value?;
                writeln!(output, "Setting max branch to {}", self.m_ai.m_max_branch)
                    .map_err(io_error)?;
            } else if let Some(value) = argument(line, "report_sims") {
                self.m_report_sims = value?;
                writeln!(
                    output,
                    "Setting selected-move report simulations to {}",
                    self.m_report_sims
                )
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
            } else if let Some(value) = line.strip_prefix("fire_overshooting ") {
                self.m_game.m_fire_overshooting = parse_on_off("fire_overshooting", value)?;
                writeln!(
                    output,
                    "Setting Fire overshooting {}",
                    if self.m_game.m_fire_overshooting {
                        "on"
                    } else {
                        "off"
                    }
                )
                .map_err(io_error)?;
            } else if let Some(arguments) = line.strip_prefix("special ") {
                let mut fields = arguments.split_whitespace();
                let player = parse_usize(fields.next().unwrap_or_default())?;
                if player > 1 {
                    return Err(ParseError(format!(
                        "invalid special player number: {player}"
                    )));
                }
                let names = fields.collect::<Vec<_>>();
                let mut specials = 0;
                for name in &names {
                    specials |=
                        crate::protocol::notation::button_special(name).ok_or_else(|| {
                            ParseError(format!(
                                "unknown button special: {name} (expected one of {})",
                                crate::protocol::notation::BUTTON_SPECIALS
                                    .iter()
                                    .map(|special| special.id)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ))
                        })?;
                }
                self.m_game.m_player[player].m_specials = specials;
                writeln!(
                    output,
                    "Setting specials for player {player} to {}",
                    if names.is_empty() {
                        "none".to_string()
                    } else {
                        names.join(" ")
                    }
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
                        0 => AiPolicy::Random,
                        1 => AiPolicy::Maximize,
                        2 | 3 => {
                            let ai = Bmai3 {
                                m_cull_moves: false,
                                m_max_ply: self.m_ai.m_max_ply,
                                m_rollout_policy: if mode == 2 {
                                    RolloutPolicy::MaximizeOrRandom(probability)
                                } else {
                                    RolloutPolicy::Qai
                                },
                                ..Default::default()
                            };
                            AiPolicy::Bmai(Box::new(ai))
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

    /// C++ holds NULL before any `game` or `ai`; `g_ai` is the closest stand-in.
    pub(super) fn PlayerAI(&self, player: usize) -> &Bmai3 {
        match self.m_player_ai[player] {
            AiSlot::Unbound | AiSlot::Global => &self.m_ai,
            AiSlot::Type(ai_type) => &self.m_type_ai[ai_type],
        }
    }

    pub(super) fn AIType(&self, player: usize) -> usize {
        match self.m_player_ai[player] {
            AiSlot::Unbound | AiSlot::Global => 2,
            AiSlot::Type(ai_type) => ai_type,
        }
    }

    /// Returns whether C++ prints a confirmation. Before any `game` or `ai`, C++
    /// dereferences NULL; BMAIR keeps earlier releases' message-only behavior.
    pub(super) fn SetPlayerAI(
        &mut self,
        player: usize,
        update: impl FnOnce(&mut Bmai3),
    ) -> Result<bool, ParseError> {
        let slot = *self
            .m_player_ai
            .get(player)
            .ok_or_else(|| ParseError(format!("invalid setting for ai player number: {player}")))?;
        match slot {
            AiSlot::Unbound => {}
            AiSlot::Global => update(&mut self.m_ai),
            AiSlot::Type(1) => return Ok(false),
            AiSlot::Type(ai_type) => update(&mut self.m_type_ai[ai_type]),
        }
        Ok(true)
    }

    pub(super) fn RequirePreround(&self) -> Result<(), ParseError> {
        if self.m_game.m_phase == Phase::Preround {
            Ok(())
        } else {
            Err(ParseError("Cannot PlayGame unless it is preround".into()))
        }
    }

    pub(super) fn Policies(&self) -> [AiPolicy; 2] {
        std::array::from_fn(|player| match self.AIType(player) {
            0 => {
                let mut ai = self.PlayerAI(player).clone();
                ai.m_cull_moves = false;
                AiPolicy::Bmai(Box::new(ai))
            }
            1 => AiPolicy::Qai,
            2 => AiPolicy::Bmai(Box::new(self.PlayerAI(player).clone())),
            _ => unreachable!(),
        })
    }

    pub(super) fn ParseGame<W: Write>(
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
            // A later undefined swing unlocks it again, as in C++.
            let mut swing_set = SwingSet::Not;
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
                        SwingSet::Locked
                    } else {
                        SwingSet::Not
                    };
                }
                dice.push(die);
            }
            self.m_game.m_player[id].m_die = dice;
            self.m_game.m_player[id].m_round_original_sides = [[0; 2]; crate::game::MAX_DICE];
            self.m_game.m_player[id].m_round_transformed = 0;
            self.m_game.m_player[id].m_radioactive_products = 0;
            self.m_game.m_player[id].m_rage_replacements = 0;
            self.m_game.m_player[id].m_specials = 0;
            self.m_game.m_player[id].m_score = if matches!(
                self.m_game.m_phase,
                Phase::Initiative | Phase::Chance | Phase::Focus
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
                self.m_game.m_phase == Phase::Preround,
                output,
            )?;
        }
        if self.m_game.m_phase == Phase::Auxiliary {
            PrepareAuxiliaryPhase(&mut self.m_game)?;
        }
        // The `ai` type objects keep their settings across games, as in C++.
        self.m_player_ai = [AiSlot::Global; 2];
        Ok(pos)
    }
}
