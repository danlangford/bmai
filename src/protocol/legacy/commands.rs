// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

impl Parser {
    pub(super) fn parse_string_commands<W: Write>(
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
                self.execution_mode = ExecutionMode::parse(value).ok_or_else(|| {
                    ParseError(format!(
                        "invalid execution mode: {value} (expected legacy or native)"
                    ))
                })?;
                writeln!(
                    output,
                    "Setting execution mode to {}",
                    self.execution_mode.as_str()
                )
                .map_err(io_error)?;
            } else if let Some(value) = line.strip_prefix("rng ") {
                let algorithm = RngAlgorithm::parse(value).ok_or_else(|| {
                    ParseError(format!(
                        "invalid RNG algorithm: {value} (expected legacy or park-miller)"
                    ))
                })?;
                self.rng.set_algorithm(algorithm);
                writeln!(output, "Setting RNG to legacy ({})", algorithm.replay_id())
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
                self.native_workers = workers;
                if automatic {
                    writeln!(output, "Setting native workers to {workers} (auto)")
                        .map_err(io_error)?;
                } else {
                    writeln!(output, "Setting native workers to {workers}").map_err(io_error)?;
                }
            } else if line.starts_with("game") {
                if let Some(wins) = line.strip_prefix("game ") {
                    self.game.target_wins = parse_usize(wins)? as u8;
                    writeln!(output, "target wins set to {}", self.game.target_wins)
                        .map_err(io_error)?;
                }
                pos = self.parse_game(&lines, pos, output)?;
            } else if let Some(arguments) = line.strip_prefix("ai ") {
                let (player, name) = arguments.split_once(' ').ok_or_else(|| {
                    ParseError(format!("ai takes a player and an engine name: {line}"))
                })?;
                let player = parse_usize(player)?;
                if player > 1 {
                    return Err(ParseError(format!(
                        "invalid setting for ai player number: {player}"
                    )));
                }
                let engine: Box<dyn Engine> = match name.trim() {
                    "montecarlo" => Box::new(MonteCarlo::new(self.ai.clone())),
                    name => crate::engines::engine(name).ok_or_else(|| {
                        ParseError(format!(
                            "unknown ai {name}; choose one of: {}",
                            crate::engines::ENGINE_NAMES.join(", ")
                        ))
                    })?,
                };
                writeln!(
                    output,
                    "Setting AI for player {player} to {}",
                    engine.name()
                )
                .map_err(io_error)?;
                self.player_engines[player] = PlayerEngine::Own(engine);
            } else if let Some((player, value)) = two_usize_arguments(line, "ply")? {
                self.set_player_setting(player, Setting::Ply(value))?;
                writeln!(output, "Setting max ply for player {player} to {value}")
                    .map_err(io_error)?;
            } else if let Some(value) = argument(line, "ply") {
                self.set_global_setting(Setting::Ply(value?))?;
                writeln!(output, "Setting max ply to {}", self.ai.max_ply).map_err(io_error)?;
            } else if let Some((player, value)) = two_usize_arguments(line, "max_sims")? {
                self.set_player_setting(player, Setting::MaxSims(value))?;
                writeln!(output, "Setting max sims for player {player} to {value}")
                    .map_err(io_error)?;
            } else if let Some(value) = argument(line, "max_sims") {
                self.set_global_setting(Setting::MaxSims(value?))?;
                writeln!(output, "Setting max # simulations to {}", self.ai.max_sims)
                    .map_err(io_error)?;
            } else if let Some((player, value)) = two_usize_arguments(line, "min_sims")? {
                self.set_player_setting(player, Setting::MinSims(value))?;
                writeln!(output, "Setting min sims for player {player} to {value}")
                    .map_err(io_error)?;
            } else if let Some(value) = argument(line, "min_sims") {
                self.set_global_setting(Setting::MinSims(value?))?;
                writeln!(output, "Setting min # simulations to {}", self.ai.min_sims)
                    .map_err(io_error)?;
            } else if let Some((player, value)) = two_usize_arguments(line, "maxbranch")? {
                self.set_player_setting(player, Setting::MaxBranch(value))?;
                writeln!(output, "Setting max branch for player {player} to {value}")
                    .map_err(io_error)?;
            } else if let Some(value) = argument(line, "maxbranch") {
                self.set_global_setting(Setting::MaxBranch(value?))?;
                writeln!(output, "Setting max branch to {}", self.ai.max_branch)
                    .map_err(io_error)?;
            } else if let Some(arguments) = line.strip_prefix("playout ") {
                self.apply_setting_command(arguments, "playout", output)?;
            } else if let Some(arguments) = line.strip_prefix("cull ") {
                self.apply_setting_command(arguments, "cull", output)?;
            } else if let Some(arguments) = line.strip_prefix("endgame ") {
                self.apply_setting_command(arguments, "endgame", output)?;
            } else if let Some(value) = argument(line, "report_sims") {
                self.report_sims = value?;
                writeln!(
                    output,
                    "Setting selected-move report simulations to {}",
                    self.report_sims
                )
                .map_err(io_error)?;
            } else if let Some(value) = line.strip_prefix("turbo_accuracy ") {
                self.game.turbo_accuracy = value
                    .parse::<f32>()
                    .map_err(|_| ParseError(format!("invalid float: {value}")))?;
                writeln!(
                    output,
                    "Setting turbo accuracy to {:.6}",
                    self.game.turbo_accuracy
                )
                .map_err(io_error)?;
            } else if let Some(value) = line.strip_prefix("fire_overshooting ") {
                self.game.fire_overshooting = parse_on_off("fire_overshooting", value)?;
                writeln!(
                    output,
                    "Setting Fire overshooting {}",
                    if self.game.fire_overshooting {
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
                self.game.players[player].specials = specials;
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
                self.game.surrender_allowed = value == "on";
            } else if line == "getaction" {
                self.send_action(output)?;
            // C++'s compare is the same command as playgame.
            } else if let Some(command) = ["playgame ", "compare "]
                .into_iter()
                .find(|command| line.starts_with(command))
            {
                self.require_preround()?;
                let games = parse_usize(line.trim_start_matches(command))?;
                let wins = self
                    .play_matches(|game, rng, policies, native| {
                        play_games_with_policies(game, games, rng, policies, native, output)
                    })
                    .map_err(io_error)?;
                writeln!(output, "matches over {} - {}", wins[0], wins[1]).map_err(io_error)?;
            } else if let Some(games) = line.strip_prefix("playfair ") {
                self.require_preround()?;
                let games = parse_usize(games.trim()).map_err(|_| {
                    ParseError(format!(
                        "playfair takes only a game count; select engines with ai: {line}"
                    ))
                })?;
                let wins = self.play_matches(|game, rng, policies, native| {
                    play_fair_games(game, games, rng, policies, native)
                });
                let cancelled = games - wins.iter().flatten().sum::<usize>();
                // C++ has no round limit, so its header never needs the count.
                if cancelled == 0 {
                    writeln!(output, "PlayFairGames: {games} games")
                } else {
                    writeln!(
                        output,
                        "PlayFairGames: {games} games, {cancelled} cancelled"
                    )
                }
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
                self.rng.reseed(resolved);
                self.native_root_seed = u64::from(resolved);
                self.native_decision_index = 0;
                writeln!(output, "Seeding with {seed}").map_err(io_error)?;
            } else if let Some(value) = argument(line, "debugply") {
                self.debug_ply = value?;
                writeln!(output, "Setting debug ply to {}", self.debug_ply).map_err(io_error)?;
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
                self.logging[index] = enabled != 0;
                if self.logging[0] {
                    writeln!(output, "Debug {category} set to {enabled}").map_err(io_error)?;
                }
            } else {
                return Err(ParseError(format!("unrecognized command: {line}")));
            }
        }
        Ok(())
    }

    pub(super) fn player_engine(&self, player: usize) -> Box<dyn Engine> {
        match &self.player_engines[player] {
            PlayerEngine::Global => Box::new(MonteCarlo::new(self.ai.clone())),
            PlayerEngine::Own(engine) => engine.clone(),
        }
    }

    /// A per-player setting gives that player its own copy of the global
    /// settings, so it never changes the other player's search.
    pub(super) fn set_player_setting(
        &mut self,
        player: usize,
        setting: Setting,
    ) -> Result<(), ParseError> {
        if player > 1 {
            return Err(ParseError(format!(
                "invalid setting for ai player number: {player}"
            )));
        }
        if matches!(self.player_engines[player], PlayerEngine::Global) {
            self.player_engines[player] = PlayerEngine::Own(self.player_engine(player));
        }
        let PlayerEngine::Own(engine) = &mut self.player_engines[player] else {
            unreachable!("the player was just given its own engine");
        };
        engine
            .set(setting)
            .map_err(|error| ParseError(format!("player {player}: {error}")))
    }

    /// `NAME [PLAYER] VALUE` for settings that have no C++ confirmation text.
    pub(super) fn apply_setting_command<W: Write>(
        &mut self,
        arguments: &str,
        name: &str,
        output: &mut W,
    ) -> Result<(), ParseError> {
        let parse = |value: &str| Setting::parse(name, value).map_err(ParseError);
        match arguments.split_once(' ') {
            Some((player, value)) => {
                let player = parse_usize(player)?;
                self.set_player_setting(player, parse(value)?)?;
                writeln!(output, "Setting {name} for player {player} to {value}").map_err(io_error)
            }
            None => {
                self.set_global_setting(parse(arguments)?)?;
                writeln!(output, "Setting {name} to {arguments}").map_err(io_error)
            }
        }
    }

    pub(super) fn set_global_setting(&mut self, setting: Setting) -> Result<(), ParseError> {
        let mut global = MonteCarlo::new(self.ai.clone());
        global.set(setting).map_err(ParseError)?;
        self.ai = global.search;
        Ok(())
    }

    pub(super) fn require_preround(&self) -> Result<(), ParseError> {
        if self.game.phase == Phase::Preround {
            Ok(())
        } else {
            Err(ParseError("Cannot PlayGame unless it is preround".into()))
        }
    }

    pub(super) fn policies(&self) -> Engines {
        [self.player_engine(0), self.player_engine(1)]
    }

    fn play_matches<T>(
        &mut self,
        play: impl FnOnce(&Game, &mut Rng, &Engines, Option<&mut NativeReplaySequence<'_>>) -> T,
    ) -> T {
        let policies = self.policies();
        let native_mode = self.execution_mode == ExecutionMode::Native;
        let mut sequence = NativeReplaySequence {
            algorithm: self.rng.algorithm(),
            root_seed: self.native_root_seed,
            workers: self.native_workers,
            decision_index: &mut self.native_decision_index,
        };
        play(
            &self.game,
            &mut self.rng,
            &policies,
            native_mode.then_some(&mut sequence),
        )
    }

    pub(super) fn parse_game<W: Write>(
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
        self.game.phase = parse_phase(phase)?;
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
                let die = parse_die(definition, original_index)?;
                if die.swing_type.iter().any(Option::is_some) || die.has_property(property::OPTION)
                {
                    swing_set = if parse_die_defined_sides(
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
            self.game.players[id].dice = dice;
            self.game.players[id].radioactive_products = 0;
            self.game.players[id].rage_replacements = 0;
            self.game.players[id].specials = 0;
            self.game.players[id].score = if matches!(
                self.game.phase,
                Phase::Initiative | Phase::Chance | Phase::Focus
            ) {
                self.game.players[id]
                    .dice
                    .iter()
                    .filter(|die| !die.in_reserve)
                    .map(|die| die.score(true))
                    .sum()
            } else {
                score
            };
            self.game.players[id].swing_set = swing_set;
            self.game.players[id].optimize_dice();
            debug_player(
                &self.game.players[id],
                self.game.phase == Phase::Preround,
                output,
            )?;
        }
        for (id, player) in self.game.players.iter().enumerate() {
            let count = player
                .dice
                .iter()
                .filter(|die| die.has_property(property::AUXILIARY))
                .count();
            if count > 1 {
                return Err(ParseError(format!(
                    "player {id} has {count} Auxiliary dice; ButtonWeavers permits one"
                )));
            }
        }
        if self.game.phase == Phase::Auxiliary {
            offer_courtesy_auxiliary(&mut self.game);
        }
        self.player_engines = [PlayerEngine::Global, PlayerEngine::Global];
        Ok(pos)
    }
}
