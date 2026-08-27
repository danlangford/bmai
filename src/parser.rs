// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::fmt;
use std::io::Write;

use crate::model::{
    BMC_Die, BMC_DieIndexSet, BMC_Game, BMC_Move, BME_ACTION, BME_PHASE, BME_SWING_SET, property,
};
use crate::simulation::{
    BMC_AI_POLICY, PlayFairGames, PlayGamesWithPolicies, SelectBMAIAction, SelectBMAIChanceAction,
    SelectBMAIFocusAction, SelectBMAIReserveAction, SelectBMAISetSwingAction, SelectQAIAction,
    SelectQAIReserveAction, SelectQAISetSwingAction, SwingMove,
};
use crate::{BMC_BMAI3, BMC_RNG, BME_ROLLOUT_POLICY};

#[derive(Debug, Clone)]
pub struct ParseError(String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ParseError {}

#[derive(Debug)]
pub struct BMC_Parser {
    pub m_game: BMC_Game,
    m_max_ply: usize,
    m_min_sims: usize,
    m_max_sims: usize,
    m_max_branch: usize,
    m_rng: BMC_RNG,
    m_ai: BMC_BMAI3,
    m_player_ai: [BMC_BMAI3; 2],
    m_ai_type: [usize; 2],
    m_ai_explicit: [bool; 2],
    m_debug_ply: usize,
    m_logging: [bool; 8],
}

impl Default for BMC_Parser {
    fn default() -> Self {
        Self {
            m_game: BMC_Game::default(),
            m_max_ply: 1,
            m_min_sims: 10,
            m_max_sims: 500,
            m_max_branch: 5000,
            m_rng: BMC_RNG::default(),
            m_ai: BMC_BMAI3::default(),
            m_player_ai: std::array::from_fn(|_| BMC_BMAI3::default()),
            m_ai_type: [2, 2],
            m_ai_explicit: [false, false],
            m_debug_ply: 0,
            m_logging: [true; 8],
        }
    }
}

impl BMC_Parser {
    pub fn ParseString<W: Write>(&mut self, data: &str, output: &mut W) -> Result<(), ParseError> {
        let lines: Vec<_> = data.lines().collect();
        let mut pos = 0;
        while pos < lines.len() {
            let line = lines[pos].trim();
            pos += 1;
            if line.is_empty() {
                continue;
            }
            if line.starts_with("game") {
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
                let wins = PlayGamesWithPolicies(&self.m_game, games, &mut self.m_rng, &policies);
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
                let wins = PlayFairGames(&self.m_game, games, &mut self.m_rng, &policies);
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
                let wins = PlayGamesWithPolicies(&self.m_game, games, &mut self.m_rng, &policies);
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
        self.m_game.m_phase = match phase {
            "preround" => BME_PHASE::PREROUND,
            "reserve" => BME_PHASE::RESERVE,
            "initiative" => BME_PHASE::INITIATIVE,
            "chance" => BME_PHASE::CHANCE,
            "focus" => BME_PHASE::FOCUS,
            "fight" => BME_PHASE::FIGHT,
            "gameover" => BME_PHASE::GAMEOVER,
            _ => return Err(ParseError("phase not found".into())),
        };
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
                    swing_set = if definition
                        .split_once(':')
                        .map_or(definition, |(text, _)| text)
                        .rsplit_once('-')
                        .is_some_and(|(_, value)| value.parse::<u8>().is_ok())
                    {
                        BME_SWING_SET::LOCKED
                    } else {
                        BME_SWING_SET::NOT
                    };
                }
                dice.push(die);
            }
            self.m_game.m_player[id].m_die = dice;
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
                let action = if self.m_ai_type[0] == 1 {
                    SelectQAIAction(&self.m_game, &mut self.m_rng)
                } else {
                    SelectBMAIAction(&self.m_game, &mut self.m_rng, &self.m_player_ai[0])
                };
                self.SendStats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                SendAttack(&self.m_game, &action, output)
            }
            BME_PHASE::RESERVE => {
                let reserve = if self.m_ai_type[0] == 1 {
                    SelectQAIReserveAction(&self.m_game)
                } else {
                    SelectBMAIReserveAction(&self.m_game, &mut self.m_rng, &self.m_player_ai[0])
                };
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
                } else {
                    SelectBMAISetSwingAction(&self.m_game, &mut self.m_rng, &self.m_player_ai[0])
                };
                self.SendStats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                SendSetSwing(&self.m_game, &action, output)
            }
            BME_PHASE::CHANCE => {
                if self.m_ai_type[0] == 1 {
                    self.SendStats(output)?;
                    writeln!(output, "action\npass").map_err(io_error)?;
                    return Ok(());
                }
                let action =
                    SelectBMAIChanceAction(&self.m_game, &mut self.m_rng, &self.m_player_ai[0]);
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
                    self.SendStats(output)?;
                    writeln!(output, "action\npass").map_err(io_error)?;
                    return Ok(());
                }
                let action =
                    SelectBMAIFocusAction(&self.m_game, &mut self.m_rng, &self.m_player_ai[0]);
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
    // Defined swing/option side follows '-'.
    if let Some((_, defined)) = definition.rsplit_once('-')
        && let Ok(value) = defined.parse::<u8>()
    {
        if swings[0].is_some() {
            sides[0] = value;
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

fn prefix_property(ch: char) -> Option<u64> {
    Some(match ch {
        '^' => property::TIME_AND_SPACE,
        'q' => property::QUEER,
        't' => property::TRIP,
        'z' => property::SPEED,
        's' => property::SHADOW,
        'B' => property::BERSERK,
        'd' => property::STEALTH,
        'p' => property::POISON,
        'n' => property::NULL,
        'f' => property::FOCUS,
        'H' => property::MIGHTY,
        'h' => property::WEAK,
        'r' => property::RESERVE,
        'o' => property::ORNERY,
        'c' => property::CHANCE,
        'm' => property::MORPHING,
        '`' => property::WARRIOR,
        'w' => property::SLOW,
        'u' => property::UNIQUE,
        '~' => property::UNSKILLED,
        'g' => property::STINGER,
        'k' => property::KONSTANT,
        'M' => property::MAXIMUM,
        'I' => property::INSULT,
        'v' => property::VALUE,
        '+' => property::AUXILIARY,
        'D' => property::DOPPLEGANGER,
        '%' => property::RADIOACTIVE,
        'G' => property::RAGE,
        _ => return None,
    })
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
fn io_error(error: std::io::Error) -> ParseError {
    ParseError(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let option = ParseDie("zU?-30", 1).unwrap();
        assert!(option.HasProperty(property::SPEED | property::MOOD));
        assert_eq!(option.m_sides[0], 30);
    }

    #[test]
    fn initiative_chance_and_focus_use_bmai_search() {
        for (phase, die, expected) in [
            ("chance", "c10:10", "action\npass\n"),
            ("focus", "f10:10", "action\npass\n"),
        ] {
            let input = format!(
                "game 3\n{phase}\nplayer 0 1 0\n{die}\nplayer 1 1 0\n6:6\nply 1\nmax_sims 10\nmin_sims 1\nmaxbranch 20\ngetaction\nquit\n"
            );
            let mut output = Vec::new();
            BMC_Parser::default()
                .ParseString(&input, &mut output)
                .unwrap();
            let output = String::from_utf8(output).unwrap();
            assert!(output.ends_with(expected), "{output}");
        }
    }

    #[test]
    fn insult_fixture_emits_reference_protocol_action() {
        let input = include_str!("../tests/fixtures/Insult_in.txt");
        let mut output = Vec::new();
        BMC_Parser::default()
            .ParseString(input, &mut output)
            .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.ends_with("action\npower\n0\n1\n"), "{output}");
    }

    #[test]
    fn value_fixtures_emit_reference_protocol_actions() {
        let cases = [
            (
                include_str!("../tests/fixtures/Value1_in.txt"),
                "action\nskill\n0 1\n1\n",
            ),
            (
                include_str!("../tests/fixtures/Value2_in.txt"),
                "action\npower\n1\n0\n",
            ),
        ];
        for (input, expected) in cases {
            let mut output = Vec::new();
            BMC_Parser::default()
                .ParseString(input, &mut output)
                .unwrap();
            let output = String::from_utf8(output).unwrap();
            assert!(output.ends_with(expected), "{output}");
        }
    }

    #[test]
    fn deterministic_fight_fixtures_emit_reference_protocol_actions() {
        let cases = [
            (
                include_str!("../tests/fixtures/bug55_a_in.txt"),
                "action\nskill\n2 0\n0\n",
            ),
            (
                include_str!("../tests/fixtures/bug55_b_in.txt"),
                "action\nskill\n3 0 1\n2\n",
            ),
            (
                include_str!("../tests/fixtures/SurrenderDefault-Pass-in.txt"),
                "action\nsurrender\n",
            ),
            (
                include_str!("../tests/fixtures/SurrenderOff-Attack-in.txt"),
                "action\npower\n0\n0\n",
            ),
            (
                include_str!("../tests/fixtures/SurrenderOff-Pass-in.txt"),
                "action\npass\n",
            ),
            (
                include_str!("../tests/fixtures/SurrenderOn-Attack-in.txt"),
                "action\nsurrender\n",
            ),
            (
                include_str!("../tests/fixtures/SurrenderOn-Pass-in.txt"),
                "action\nsurrender\n",
            ),
        ];
        for (input, expected) in cases {
            let mut output = Vec::new();
            BMC_Parser::default()
                .ParseString(input, &mut output)
                .unwrap();
            let output = String::from_utf8(output).unwrap();
            assert!(output.ends_with(expected), "{output}");
        }
    }

    #[test]
    #[ignore = "full BMAI3 searches; run in the release parity suite"]
    fn deeper_reference_fixtures_emit_reference_protocol_actions() {
        let cases = [
            (
                include_str!("../tests/fixtures/bmai_in.txt"),
                "action\npower\n1\n0\n",
            ),
            (
                include_str!("../tests/fixtures/bug11_in.txt"),
                "action\nswing T 2\nswing W 4\n",
            ),
        ];
        for (input, expected) in cases {
            let mut output = Vec::new();
            BMC_Parser::default()
                .ParseString(input, &mut output)
                .unwrap();
            let output = String::from_utf8(output).unwrap();
            assert!(output.ends_with(expected), "{output}");
        }
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
        let input = include_str!("../tests/fixtures/bug16_in.txt");
        let mut output = Vec::new();
        BMC_Parser::default()
            .ParseString(input, &mut output)
            .unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.ends_with("action\nreserve 6\n"), "{output}");
    }
}
