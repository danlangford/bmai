// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! Human-readable mechanics scenarios.
//!
//! This is deliberately a thin test adapter. Game setup goes through the
//! production parser, legality goes through production attack enumeration,
//! and resolution goes through the production simulator.

use super::{ApplyAttack, RestoreDiceForNewRound};
use crate::{BMC_Die, BMC_Game, BMC_Move, BMC_Parser, BMC_RNG, BME_ATTACK, BME_PHASE, property};

pub(crate) fn scenario() -> Scenario {
    Scenario::default()
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
}

impl Scenario {
    pub(crate) fn during(mut self, phase: BME_PHASE) -> Self {
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
        if let Some(expected) = self.expected_next_round_attacker_dice {
            RestoreDiceForNewRound(&mut game, &template);
            assert_active_dice("next-round attacker", &game, 0, &expected);
            assert_eq!(
                game.m_player[0].m_round_transformed, 0,
                "next-round attacker still has transformed-recipe bookkeeping"
            );
            assert_eq!(
                game.m_player[0].m_radioactive_products, 0,
                "next-round attacker still has Radioactive-product bookkeeping"
            );
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
    fn scenario_uses_production_legality_and_null_scoring() {
        scenario()
            .during(FIGHT)
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
