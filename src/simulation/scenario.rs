// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! Human-readable mechanics scenarios.
//!
//! This is deliberately a thin test adapter. Game setup goes through the
//! production parser, legality goes through production attack enumeration,
//! and resolution goes through the production simulator.

use super::{ApplyAttack, RestoreDiceForNewRound};
use crate::protocol::{OptionSelection, ProtocolAction, SwingSelection};
use crate::{BMC_Die, BMC_Game, BMC_Move, BMC_Parser, BMC_RNG, BME_ATTACK, BME_PHASE, property};
use std::ops::RangeInclusive;

pub(crate) fn scenario() -> Scenario {
    Scenario::default()
}

pub(crate) fn search_scenario() -> SearchScenario {
    SearchScenario::default()
}

pub(crate) fn parser_scenario(input: impl Into<String>) -> ParserScenario {
    ParserScenario {
        input: input.into(),
        ..Default::default()
    }
}

#[derive(Default)]
struct ActionExpectation {
    action: Option<ExpectedAction>,
    attackers: Option<Vec<usize>>,
    targets: Option<Vec<usize>>,
}

enum ExpectedAction {
    Pass,
    Surrender,
    Attack(BME_ATTACK),
    Reserve(Option<usize>),
    SetSwing {
        swings: Vec<SwingSelection>,
        options: Vec<OptionSelection>,
    },
}

impl ActionExpectation {
    fn pass(&mut self) {
        self.action = Some(ExpectedAction::Pass);
    }

    fn surrender(&mut self) {
        self.action = Some(ExpectedAction::Surrender);
    }

    fn attack(&mut self, attack: BME_ATTACK) {
        self.action = Some(ExpectedAction::Attack(attack));
    }

    fn protocol_action(&self) -> Option<ProtocolAction> {
        match self.action.as_ref()? {
            ExpectedAction::Pass => Some(ProtocolAction::Pass),
            ExpectedAction::Surrender => Some(ProtocolAction::Surrender),
            ExpectedAction::Attack(attack) => Some(ProtocolAction::Attack {
                attack_type: attack.protocol(),
                attackers: self
                    .attackers
                    .clone()
                    .expect("attack expectation has no attackers"),
                targets: self
                    .targets
                    .clone()
                    .expect("attack expectation has no targets"),
                turbo: None,
            }),
            ExpectedAction::Reserve(die) => Some(ProtocolAction::Reserve { die: *die }),
            ExpectedAction::SetSwing { swings, options } => Some(ProtocolAction::SetSwing {
                swings: swings.clone(),
                options: options.clone(),
            }),
        }
    }
}

/// Runs existing wire input through the production parser while keeping action
/// expectations in the vocabulary used by a game transcript.
#[derive(Default)]
pub(crate) struct ParserScenario {
    input: String,
    expected_action: ActionExpectation,
}

impl ParserScenario {
    pub(crate) fn expect_pass(mut self) -> Self {
        self.expected_action.pass();
        self
    }

    pub(crate) fn expect_surrender(mut self) -> Self {
        self.expected_action.surrender();
        self
    }

    pub(crate) fn expect_attack(mut self, attack: BME_ATTACK) -> Self {
        self.expected_action.attack(attack);
        self
    }

    pub(crate) fn expect_reserve(mut self, die: Option<usize>) -> Self {
        self.expected_action.action = Some(ExpectedAction::Reserve(die));
        self
    }

    pub(crate) fn expect_swings(mut self, swings: impl IntoIterator<Item = (char, u8)>) -> Self {
        self.expected_action.action = Some(ExpectedAction::SetSwing {
            swings: swings
                .into_iter()
                .map(|(swing, value)| SwingSelection { swing, value })
                .collect(),
            options: Vec::new(),
        });
        self
    }

    pub(crate) fn using(mut self, dice: impl IntoIterator<Item = usize>) -> Self {
        self.expected_action.attackers = Some(dice.into_iter().collect());
        self
    }

    pub(crate) fn targeting(mut self, dice: impl IntoIterator<Item = usize>) -> Self {
        self.expected_action.targets = Some(dice.into_iter().collect());
        self
    }

    #[track_caller]
    pub(crate) fn run(self) {
        let expected = self
            .expected_action
            .protocol_action()
            .expect("parser scenario has no expected action");
        let mut parser = BMC_Parser::default();
        let mut output = Vec::new();
        parser
            .ParseString(&self.input, &mut output)
            .unwrap_or_else(|error| panic!("invalid parser scenario: {error}\n{}", self.input));
        assert_eq!(
            parser.last_action(),
            Some(&expected),
            "unexpected action for parser scenario:\n{}\n{}",
            self.input,
            String::from_utf8_lossy(&output)
        );
        let expected_wire = legacy_action_suffix(&expected);
        let output = String::from_utf8(output).expect("protocol output must be UTF-8");
        assert!(
            output.ends_with(&expected_wire),
            "parser scenario did not emit {expected_wire:?}:\n{output}"
        );
    }
}

fn legacy_action_suffix(action: &ProtocolAction) -> String {
    match action {
        ProtocolAction::Pass => "action\npass\n".into(),
        ProtocolAction::Surrender => "action\nsurrender\n".into(),
        ProtocolAction::Attack {
            attack_type,
            attackers,
            targets,
            turbo: None,
        } => format!(
            "action\n{attack_type}\n{}\n{}\n",
            joined_indices(attackers),
            joined_indices(targets)
        ),
        ProtocolAction::Reserve { die } => {
            let die = die.map_or_else(|| "-1".into(), |value| value.to_string());
            format!("action\nreserve {die}\n")
        }
        ProtocolAction::SetSwing { swings, options } => {
            let mut output = String::from("action\n");
            for selection in swings {
                output.push_str(&format!("swing {} {}\n", selection.swing, selection.value));
            }
            for selection in options {
                output.push_str(&format!("option {} {}\n", selection.die, selection.value));
            }
            output
        }
        _ => panic!("parser scenario cannot yet render {action:?}"),
    }
}

fn joined_indices(indices: &[usize]) -> String {
    indices
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum SearchMode {
    Legacy,
    LegacyWithWorkers { workers: usize },
    Native { workers: Option<usize> },
}

pub(crate) const LEGACY: SearchMode = SearchMode::Legacy;
pub(crate) const NATIVE: SearchMode = SearchMode::Native { workers: None };

pub(crate) const fn legacy_with_workers(workers: usize) -> SearchMode {
    SearchMode::LegacyWithWorkers { workers }
}

pub(crate) const fn native(workers: usize) -> SearchMode {
    SearchMode::Native {
        workers: Some(workers),
    }
}

#[derive(Default)]
pub(crate) struct SearchScenario {
    phase: Option<BME_PHASE>,
    target_wins: Option<usize>,
    players: [Option<(f32, Vec<String>)>; 2],
    max_ply: Option<usize>,
    min_sims: Option<usize>,
    max_sims: Option<usize>,
    max_branch: Option<usize>,
    surrender: Option<bool>,
    expected_player: Option<usize>,
    expected_win_percent: Option<RangeInclusive<f32>>,
    expected_action: ActionExpectation,
    modes: Vec<SearchMode>,
}

impl SearchScenario {
    pub(crate) fn phase(mut self, phase: BME_PHASE) -> Self {
        self.phase = Some(phase);
        self
    }

    pub(crate) fn target_wins(mut self, target_wins: usize) -> Self {
        self.target_wins = Some(target_wins);
        self
    }

    pub(crate) fn player(
        mut self,
        player: usize,
        score: f32,
        dice: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        assert!(player < 2, "player index must be 0 or 1");
        self.players[player] = Some((score, dice.into_iter().map(Into::into).collect()));
        self
    }

    pub(crate) fn ply(mut self, max_ply: usize) -> Self {
        self.max_ply = Some(max_ply);
        self
    }

    pub(crate) fn simulations(mut self, min: usize, max: usize) -> Self {
        self.min_sims = Some(min);
        self.max_sims = Some(max);
        self
    }

    pub(crate) fn max_branch(mut self, max_branch: usize) -> Self {
        self.max_branch = Some(max_branch);
        self
    }

    pub(crate) fn surrender(mut self, allowed: bool) -> Self {
        self.surrender = Some(allowed);
        self
    }

    pub(crate) fn modes(mut self, modes: impl IntoIterator<Item = SearchMode>) -> Self {
        self.modes = modes.into_iter().collect();
        self
    }

    /// Asserts the percentage printed for the player whose action is requested.
    /// A range keeps seeded statistical scenarios readable without hiding their
    /// acceptable uncertainty.
    pub(crate) fn expect_player_win_percent(
        mut self,
        player: usize,
        expected: RangeInclusive<f32>,
    ) -> Self {
        self.expected_player = Some(player);
        self.expected_win_percent = Some(expected);
        self
    }

    pub(crate) fn expect_pass(mut self) -> Self {
        self.expected_action.pass();
        self
    }

    pub(crate) fn expect_attack(mut self, attack: BME_ATTACK) -> Self {
        self.expected_action.attack(attack);
        self
    }

    pub(crate) fn using(mut self, dice: impl IntoIterator<Item = usize>) -> Self {
        self.expected_action.attackers = Some(dice.into_iter().collect());
        self
    }

    pub(crate) fn targeting(mut self, dice: impl IntoIterator<Item = usize>) -> Self {
        self.expected_action.targets = Some(dice.into_iter().collect());
        self
    }

    #[track_caller]
    pub(crate) fn run(self) {
        assert!(
            self.expected_win_percent.is_some() || self.expected_action.action.is_some(),
            "search scenario has no expectation"
        );
        assert!(!self.modes.is_empty(), "search scenario has no modes");

        for mode in &self.modes {
            let input = self.protocol_input(*mode);
            let mut parser = BMC_Parser::default();
            let mut output = Vec::new();
            parser
                .ParseString(&input, &mut output)
                .unwrap_or_else(|error| panic!("invalid search scenario: {error}\n{input}"));
            let output = String::from_utf8(output).expect("protocol output must be UTF-8");
            if let Some(expected) = &self.expected_win_percent {
                let player = self
                    .expected_player
                    .expect("win percentage expectation has no player");
                assert_eq!(player, 0, "getaction reports the active player 0");
                let actual = reported_win_percent(&output).unwrap_or_else(|| {
                    panic!("search scenario reported no win percentage:\n{output}")
                });
                assert!(
                    expected.contains(&actual),
                    "{mode:?} player {player} win percentage {actual} was outside {expected:?}\n{output}"
                );
            }
            if let Some(expected) = self.expected_action.protocol_action() {
                assert_eq!(
                    parser.last_action(),
                    Some(&expected),
                    "{mode:?} selected an unexpected action:\n{output}"
                );
                let expected_wire = legacy_action_suffix(&expected);
                assert!(
                    output.ends_with(&expected_wire),
                    "{mode:?} did not emit {expected_wire:?}:\n{output}"
                );
            }
        }
    }

    fn protocol_input(&self, mode: SearchMode) -> String {
        let phase = self.phase.unwrap_or(BME_PHASE::FIGHT);
        let target_wins = self.target_wins.unwrap_or(3);
        let mut input = String::new();
        match mode {
            SearchMode::Legacy => {}
            SearchMode::LegacyWithWorkers { workers } => {
                input.push_str(&format!("workers {workers}\n"));
            }
            SearchMode::Native { workers } => {
                input.push_str("mode native\n");
                if let Some(workers) = workers {
                    input.push_str(&format!("workers {workers}\n"));
                }
            }
        }
        input.push_str(&format!("game {target_wins}\n{}\n", phase_name(phase)));
        for player in 0..2 {
            let (score, dice) = self.players[player]
                .as_ref()
                .unwrap_or_else(|| panic!("search scenario has no player {player}"));
            input.push_str(&format!("player {player} {} {score}\n", dice.len()));
            for die in dice {
                input.push_str(die);
                input.push('\n');
            }
        }
        if let Some(value) = self.max_ply {
            input.push_str(&format!("ply {value}\n"));
        }
        if let Some(value) = self.max_sims {
            input.push_str(&format!("max_sims {value}\n"));
        }
        if let Some(value) = self.min_sims {
            input.push_str(&format!("min_sims {value}\n"));
        }
        if let Some(value) = self.max_branch {
            input.push_str(&format!("maxbranch {value}\n"));
        }
        if let Some(allowed) = self.surrender {
            input.push_str(if allowed {
                "surrender on\n"
            } else {
                "surrender off\n"
            });
        }
        input.push_str("getaction\nquit\n");
        input
    }
}

fn phase_name(phase: BME_PHASE) -> &'static str {
    match phase {
        BME_PHASE::PREROUND => "preround",
        BME_PHASE::INITIATIVE => "initiative",
        BME_PHASE::CHANCE => "chance",
        BME_PHASE::FOCUS => "focus",
        BME_PHASE::FIGHT => "fight",
        BME_PHASE::RESERVE => "reserve",
        BME_PHASE::GAMEOVER => "gameover",
    }
}

fn reported_win_percent(output: &str) -> Option<f32> {
    output.lines().find_map(|line| {
        let (_, percent) = line.strip_prefix("l1 p0 best move (")?.rsplit_once(", ")?;
        percent.strip_suffix("% win)")?.parse().ok()
    })
}

#[derive(Default)]
pub(crate) struct Scenario {
    phase: Option<BME_PHASE>,
    attacker_dice: Vec<String>,
    defender_dice: Vec<String>,
    scores: Option<[f32; 2]>,
    attack: Option<BME_ATTACK>,
    attackers: Option<Vec<usize>>,
    targets: Option<Vec<usize>>,
    turbo_option: Option<i16>,
    seed: Option<u32>,
    expected_allowed: Option<bool>,
    expected_extra_turn: Option<bool>,
    expected_scores: Option<[f32; 2]>,
    expected_attacker_dice: Option<Vec<String>>,
    expected_attacker_dice_by_original_index: Vec<(usize, String)>,
    expected_defender_dice: Option<Vec<String>>,
    expected_captured_defender_dice: Option<Vec<String>>,
    expected_next_round_attacker_dice: Option<Vec<String>>,
    expected_next_round_defender_dice: Option<Vec<String>>,
}

impl Scenario {
    pub(crate) fn phase(mut self, phase: BME_PHASE) -> Self {
        self.phase = Some(phase);
        self
    }

    pub(crate) fn attacker(mut self, die: impl Into<String>) -> Self {
        self.attacker_dice.push(die.into());
        self
    }

    pub(crate) fn attackers(mut self, dice: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.attacker_dice.extend(dice.into_iter().map(Into::into));
        self
    }

    pub(crate) fn defender(mut self, die: impl Into<String>) -> Self {
        self.defender_dice.push(die.into());
        self
    }

    pub(crate) fn defenders(mut self, dice: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.defender_dice.extend(dice.into_iter().map(Into::into));
        self
    }

    pub(crate) fn attacks(mut self, attack: BME_ATTACK) -> Self {
        self.attack = Some(attack);
        self
    }

    /// Sets the scores before the attack. Without this call, production parsing
    /// derives each score from the active dice as it does for initiative.
    pub(crate) fn with_scores(mut self, attacker: f32, defender: f32) -> Self {
        self.scores = Some([attacker, defender]);
        self
    }

    pub(crate) fn using(mut self, attackers: impl IntoIterator<Item = usize>) -> Self {
        self.attackers = Some(attackers.into_iter().collect());
        self
    }

    pub(crate) fn targeting(mut self, targets: impl IntoIterator<Item = usize>) -> Self {
        self.targets = Some(targets.into_iter().collect());
        self
    }

    /// Selects an option-die branch (`0` or `1`) or a Turbo swing size.
    pub(crate) fn turbo(mut self, selection: i16) -> Self {
        self.turbo_option = Some(selection);
        self
    }

    pub(crate) fn seed(mut self, seed: u32) -> Self {
        self.seed = Some(seed);
        self
    }

    pub(crate) fn expect_allowed(mut self, allowed: bool) -> Self {
        self.expected_allowed = Some(allowed);
        self
    }

    pub(crate) fn expect_extra_turn(mut self, extra_turn: bool) -> Self {
        self.expected_extra_turn = Some(extra_turn);
        self
    }

    pub(crate) fn expect_scores(mut self, attacker: f32, defender: f32) -> Self {
        self.expected_scores = Some([attacker, defender]);
        self
    }

    pub(crate) fn expect_attacker_dice(
        mut self,
        dice: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.expected_attacker_dice = Some(dice.into_iter().map(Into::into).collect());
        self
    }

    /// Asserts one surviving attacker by its recipe declaration index.
    pub(crate) fn expect_attacker_die(
        mut self,
        original_index: usize,
        die: impl Into<String>,
    ) -> Self {
        self.expected_attacker_dice_by_original_index
            .push((original_index, die.into()));
        self
    }

    pub(crate) fn expect_defender_dice(
        mut self,
        dice: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.expected_defender_dice = Some(dice.into_iter().map(Into::into).collect());
        self
    }

    pub(crate) fn expect_no_defender_dice(mut self) -> Self {
        self.expected_defender_dice = Some(Vec::new());
        self
    }

    /// Asserts the defender's captured pile after the attack. This is kept
    /// separate from active dice so transformations such as Rage can describe
    /// both results without reaching into `BMC_Game` internals.
    pub(crate) fn expect_captured_defender_dice(
        mut self,
        dice: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.expected_captured_defender_dice = Some(dice.into_iter().map(Into::into).collect());
        self
    }

    pub(crate) fn expect_next_round_attacker_dice(
        mut self,
        dice: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.expected_next_round_attacker_dice = Some(dice.into_iter().map(Into::into).collect());
        self
    }

    pub(crate) fn expect_next_round_defender_dice(
        mut self,
        dice: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.expected_next_round_defender_dice = Some(dice.into_iter().map(Into::into).collect());
        self
    }

    #[track_caller]
    pub(crate) fn run(self) {
        let phase = self.phase.unwrap_or(BME_PHASE::FIGHT);
        assert_eq!(phase, BME_PHASE::FIGHT, "attack scenarios require FIGHT");
        assert!(
            !self.attacker_dice.is_empty(),
            "scenario has no attacker dice"
        );
        assert!(
            !self.defender_dice.is_empty(),
            "scenario has no defender dice"
        );
        let attack = self.attack.expect("scenario has no attack type");
        let mut game = parse_game(&self.attacker_dice, &self.defender_dice);
        game.m_phase = phase;
        if let Some(scores) = self.scores {
            game.m_player[0].m_score = scores[0];
            game.m_player[1].m_score = scores[1];
        }
        let template = game.clone();
        let attackers = resolve_original_indices(
            "attacker",
            &game.m_player[0].m_die,
            self.attackers.as_deref().unwrap_or(&[0]),
        );
        let targets = resolve_original_indices(
            "target",
            &game.m_player[1].m_die,
            self.targets.as_deref().unwrap_or(&[0]),
        );
        let mut move_to_apply = BMC_Move::attack(attack, attackers, targets, 0.0);
        if let Some(selection) = self.turbo_option {
            move_to_apply.m_turbo_option = selection;
        }
        let allowed = game
            .GenerateValidAttacksInCppOrder()
            .iter()
            .any(|candidate| {
                candidate.m_attack == move_to_apply.m_attack
                    && candidate.m_attackers == move_to_apply.m_attackers
                    && candidate.m_targets == move_to_apply.m_targets
                    && candidate.m_turbo_option == move_to_apply.m_turbo_option
            });

        if let Some(expected) = self.expected_allowed {
            assert_eq!(allowed, expected, "unexpected attack legality");
        }
        if !allowed {
            assert_eq!(
                self.expected_allowed,
                Some(false),
                "scenario attack is illegal; use expect_allowed(false) when intentional"
            );
            return;
        }

        let mut rng = BMC_RNG::default();
        if let Some(seed) = self.seed {
            rng.SRand(seed);
        }
        let extra_turn = ApplyAttack(&mut game, &move_to_apply, &mut rng);

        if let Some(expected) = self.expected_extra_turn {
            assert_eq!(extra_turn, expected, "unexpected extra-turn result");
        }
        if let Some(expected) = self.expected_scores {
            assert_eq!(
                [game.m_player[0].m_score, game.m_player[1].m_score],
                expected,
                "unexpected player scores"
            );
        }
        if let Some(expected) = self.expected_attacker_dice {
            assert_active_dice("attacker", &game, 0, &expected);
        }
        for (original_index, expected) in self.expected_attacker_dice_by_original_index {
            assert_die_by_original_index("attacker", &game, 0, original_index, &expected);
        }
        if let Some(expected) = self.expected_defender_dice {
            assert_active_dice("defender", &game, 1, &expected);
        }
        if let Some(expected) = self.expected_captured_defender_dice {
            assert_dice_matching("captured defender", &game, 1, &expected, |die| {
                die.m_captured
            });
        }
        if self.expected_next_round_attacker_dice.is_some()
            || self.expected_next_round_defender_dice.is_some()
        {
            RestoreDiceForNewRound(&mut game, &template);
            if let Some(expected) = self.expected_next_round_attacker_dice {
                assert_round_dice("next-round attacker", &game, 0, &expected);
            }
            if let Some(expected) = self.expected_next_round_defender_dice {
                assert_round_dice("next-round defender", &game, 1, &expected);
            }
            for (label, player) in [("attacker", 0), ("defender", 1)] {
                assert_eq!(
                    game.m_player[player].m_round_transformed, 0,
                    "next-round {label} still has transformed-recipe bookkeeping"
                );
                assert_eq!(
                    game.m_player[player].m_radioactive_products, 0,
                    "next-round {label} still has Radioactive-product bookkeeping"
                );
                assert_eq!(
                    game.m_player[player].m_rage_replacements, 0,
                    "next-round {label} still has Rage-replacement bookkeeping"
                );
            }
        }
    }
}

fn resolve_original_indices(label: &str, dice: &[BMC_Die], requested: &[usize]) -> Vec<usize> {
    requested
        .iter()
        .map(|original_index| {
            dice.iter()
                .position(|die| {
                    die.m_original_index == *original_index && !die.m_captured && !die.m_in_reserve
                })
                .unwrap_or_else(|| panic!("scenario has no active {label} die {original_index}"))
        })
        .collect()
}

fn parse_game(attacker_dice: &[String], defender_dice: &[String]) -> BMC_Game {
    // INITIATIVE makes the production parser derive scores from the dice. The
    // requested phase is applied by Scenario::run after parsing.
    let mut input = String::from("game\ninitiative\n");
    for (player, dice) in [attacker_dice, defender_dice].into_iter().enumerate() {
        input.push_str(&format!("player {player} {} 0\n", dice.len()));
        for die in dice {
            input.push_str(die);
            input.push('\n');
        }
    }
    let mut parser = BMC_Parser::default();
    parser
        .ParseString(&input, &mut Vec::new())
        .unwrap_or_else(|error| panic!("invalid scenario setup: {error}\n{input}"));
    parser.m_game
}

#[track_caller]
fn assert_active_dice(label: &str, game: &BMC_Game, player: usize, expected: &[String]) {
    assert_dice_matching(label, game, player, expected, |die| {
        !die.m_captured && !die.m_in_reserve
    });
}

#[track_caller]
fn assert_round_dice(label: &str, game: &BMC_Game, player: usize, expected: &[String]) {
    assert_dice_matching(label, game, player, expected, |die| !die.m_in_reserve);
}

#[track_caller]
fn assert_die_by_original_index(
    label: &str,
    game: &BMC_Game,
    player: usize,
    original_index: usize,
    expected: &str,
) {
    let die = game.m_player[player]
        .m_die
        .iter()
        .find(|die| die.m_original_index == original_index && !die.m_captured && !die.m_in_reserve)
        .unwrap_or_else(|| panic!("no active {label} die has declaration index {original_index}"));
    assert_eq!(
        format_die(die),
        expected,
        "unexpected {label} die {original_index}"
    );
}

#[track_caller]
fn assert_dice_matching(
    label: &str,
    game: &BMC_Game,
    player: usize,
    expected: &[String],
    include: impl Fn(&BMC_Die) -> bool,
) {
    let actual = game.m_player[player]
        .m_die
        .iter()
        .filter(|die| include(die))
        .map(format_die)
        .collect::<Vec<_>>();
    assert_eq!(actual, expected, "unexpected {label} dice");
}

fn format_die(die: &BMC_Die) -> String {
    let mut output = String::new();
    for notation in crate::notation::DIE_PROPERTY_PREFIXES {
        if die.HasProperty(notation.property) {
            output.push(notation.token);
        }
    }
    if die.HasProperty(property::TWIN) {
        output.push('(');
        output.push_str(&format_side(die, 0));
        output.push(',');
        output.push_str(&format_side(die, 1));
        output.push(')');
    } else {
        output.push_str(&format_side(die, 0));
        if die.HasProperty(property::OPTION) {
            output.push('/');
            output.push_str(&format_side(die, 1));
        }
    }
    if die.HasProperty(property::TURBO) {
        output.push('!');
    }
    if die.HasProperty(property::MOOD) {
        output.push('?');
    }
    if let Some(value) = die.m_value_total {
        output.push(':');
        output.push_str(&value.to_string());
        if die.m_dizzy {
            output.push('d');
        }
    }
    output
}

fn format_side(die: &BMC_Die, side: usize) -> String {
    die.m_swing_type[side].map_or_else(
        || die.m_sides[side].to_string(),
        |swing| format!("{swing}-{}", die.m_sides[side]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BME_ATTACK::POWER;
    use crate::BME_PHASE::FIGHT;

    #[test]
    fn forced_win_is_reported_as_certain_in_legacy_and_native_search() {
        search_scenario()
            .phase(FIGHT)
            .target_wins(3)
            .player(0, 30.0, ["4:4", "p(6,6):11"])
            .player(1, 40.0, ["(2,2):3", "(T,T)-2:2"])
            .ply(2)
            .simulations(5, 100)
            .max_branch(400)
            .surrender(false)
            .modes([LEGACY, legacy_with_workers(4), NATIVE, native(4)])
            .expect_player_win_percent(0, 100.0..=100.0)
            .expect_attack(crate::BME_ATTACK::POWER)
            .using([1])
            .targeting([1])
            .run();
    }

    #[test]
    fn scenario_uses_production_legality_and_null_scoring() {
        scenario()
            .phase(FIGHT)
            .attackers(["n30:27"])
            .attacks(POWER)
            .defenders(["20:19"])
            .using([0])
            .targeting([0])
            .expect_allowed(true)
            .expect_scores(0.0, 0.0)
            .expect_attacker_dice(["n30:30"])
            .expect_no_defender_dice()
            .seed(1)
            .run();
    }

    #[test]
    fn scenario_can_assert_extra_turns_and_next_round_state() {
        scenario()
            .attacker("JM6:6")
            .attacks(POWER)
            .defender("1:1")
            .expect_extra_turn(true)
            .expect_attacker_dice(["M6:6"])
            .expect_next_round_attacker_dice(["MJ6:6"])
            .run();
    }

    #[test]
    fn illegal_scenario_requires_an_explicit_expectation() {
        scenario()
            .attacker("6:1")
            .attacks(POWER)
            .defender("20:20")
            .expect_allowed(false)
            .run();
    }

    #[test]
    fn scenario_failures_show_expected_and_actual_die_recipes() {
        let failure = std::panic::catch_unwind(|| {
            scenario()
                .attacker("M6:6")
                .attacks(POWER)
                .defender("1:1")
                .expect_attacker_dice(["M8:8"])
                .run();
        })
        .expect_err("the intentionally incorrect recipe should fail");
        let message = failure
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| failure.downcast_ref::<&str>().copied())
            .expect("assertion panic should contain text");
        assert!(message.contains("unexpected attacker dice"));
        assert!(message.contains("M6:6"));
        assert!(message.contains("M8:8"));
    }

    #[test]
    fn scenario_indices_follow_recipe_order_after_production_optimization() {
        let game = parse_game(&["6:1".to_owned(), "20:20".to_owned()], &["1:1".to_owned()]);
        assert_eq!(game.m_player[0].m_die[0].m_original_index, 1);
        assert_eq!(
            resolve_original_indices("attacker", &game.m_player[0].m_die, &[0]),
            [1]
        );
    }

    #[test]
    fn scenario_selects_a_specific_turbo_branch() {
        scenario()
            .attacker("M6/10!-10:10")
            .attacks(POWER)
            .defender("1:1")
            .turbo(1)
            .expect_attacker_dice(["M6/10!:6"])
            .run();
    }
}
