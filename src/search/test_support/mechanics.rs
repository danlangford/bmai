// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[derive(Default)]
pub(crate) struct Scenario {
    phase: Option<Phase>,
    attacker_dice: Vec<String>,
    defender_dice: Vec<String>,
    specials: [Vec<&'static str>; 2],
    scores: Option<[f32; 2]>,
    attack: Option<Attack>,
    attackers: Option<Vec<usize>>,
    targets: Option<Vec<usize>>,
    turbo_option: Option<i16>,
    fire_values: Vec<(usize, u8)>,
    boosted_values: Vec<(usize, u8)>,
    fire_overshooting: bool,
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
    pub(crate) fn phase(mut self, phase: Phase) -> Self {
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

    pub(crate) fn attacker_special(mut self, special: &'static str) -> Self {
        self.specials[0].push(special);
        self
    }

    pub(crate) fn defender_special(mut self, special: &'static str) -> Self {
        self.specials[1].push(special);
        self
    }

    pub(crate) fn attacks(mut self, attack: Attack) -> Self {
        self.attack = Some(attack);
        self
    }

    /// Otherwise the parser derives scores from the dice.
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

    pub(crate) fn firing(mut self, dice: impl IntoIterator<Item = (usize, u8)>) -> Self {
        self.fire_values = dice.into_iter().collect();
        self
    }

    pub(crate) fn boosting(mut self, dice: impl IntoIterator<Item = (usize, u8)>) -> Self {
        self.boosted_values = dice.into_iter().collect();
        self
    }

    pub(crate) fn fire_overshooting(mut self, enabled: bool) -> Self {
        self.fire_overshooting = enabled;
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

    /// Separate from active dice so Rage tests can check both piles.
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
        let phase = self.phase.unwrap_or(Phase::Fight);
        assert_eq!(phase, Phase::Fight, "attack scenarios require FIGHT");
        assert!(
            !self.attacker_dice.is_empty(),
            "scenario has no attacker dice"
        );
        assert!(
            !self.defender_dice.is_empty(),
            "scenario has no defender dice"
        );
        let attack = self.attack.expect("scenario has no attack type");
        let mut game =
            parse_game_with_specials(&self.attacker_dice, &self.defender_dice, &self.specials);
        game.m_phase = phase;
        game.m_fire_overshooting = self.fire_overshooting;
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
        let mut move_to_apply = Move::attack(attack, attackers, targets, 0.0);
        if let Some(selection) = self.turbo_option {
            move_to_apply.m_turbo_option = selection;
        }
        for (original_index, new_value) in &self.fire_values {
            let index =
                resolve_original_indices("Fire", &game.m_player[0].m_die, &[*original_index])[0];
            let old_value = game.m_player[0].m_die[index].GetValueTotal();
            assert!(
                u16::from(*new_value) < old_value,
                "Fire dice must turn down"
            );
            move_to_apply.m_fire.m_amounts[index] = (old_value - u16::from(*new_value)) as u8;
        }
        for (original_index, new_value) in &self.boosted_values {
            let index = resolve_original_indices(
                "boosted attacker",
                &game.m_player[0].m_die,
                &[*original_index],
            )[0];
            let old_value = game.m_player[0].m_die[index].GetValueTotal();
            assert!(
                u16::from(*new_value) > old_value,
                "fired attackers must turn up"
            );
            move_to_apply.m_fire.m_amounts[index] = (u16::from(*new_value) - old_value) as u8;
        }
        let allowed = game
            .GenerateValidAttacksInCppOrder()
            .iter()
            .any(|candidate| {
                candidate.m_attack == move_to_apply.m_attack
                    && candidate.m_attackers == move_to_apply.m_attackers
                    && candidate.m_targets == move_to_apply.m_targets
                    && candidate.m_turbo_option == move_to_apply.m_turbo_option
                    && candidate.m_fire == move_to_apply.m_fire
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

        let mut rng = Rng::default();
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

pub(super) fn resolve_original_indices(
    label: &str,
    dice: &[Die],
    requested: &[usize],
) -> Vec<usize> {
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

pub(super) fn parse_game(attacker_dice: &[String], defender_dice: &[String]) -> Game {
    parse_game_with_specials(attacker_dice, defender_dice, &[Vec::new(), Vec::new()])
}

fn parse_game_with_specials(
    attacker_dice: &[String],
    defender_dice: &[String],
    specials: &[Vec<&'static str>; 2],
) -> Game {
    // INITIATIVE makes the parser derive scores; `run` sets the real phase.
    let mut input = String::from("game\ninitiative\n");
    for (player, dice) in [attacker_dice, defender_dice].into_iter().enumerate() {
        input.push_str(&format!("player {player} {} 0\n", dice.len()));
        for die in dice {
            input.push_str(die);
            input.push('\n');
        }
    }
    for (player, names) in specials.iter().enumerate() {
        if !names.is_empty() {
            input.push_str(&format!("special {player} {}\n", names.join(" ")));
        }
    }
    let mut parser = Parser::default();
    parser
        .ParseString(&input, &mut Vec::new())
        .unwrap_or_else(|error| panic!("invalid scenario setup: {error}\n{input}"));
    parser.m_game
}

#[track_caller]
fn assert_active_dice(label: &str, game: &Game, player: usize, expected: &[String]) {
    assert_dice_matching(label, game, player, expected, |die| {
        !die.m_captured && !die.m_in_reserve
    });
}

#[track_caller]
fn assert_round_dice(label: &str, game: &Game, player: usize, expected: &[String]) {
    assert_dice_matching(label, game, player, expected, |die| !die.m_in_reserve);
}

#[track_caller]
fn assert_die_by_original_index(
    label: &str,
    game: &Game,
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
    game: &Game,
    player: usize,
    expected: &[String],
    include: impl Fn(&Die) -> bool,
) {
    let actual = game.m_player[player]
        .m_die
        .iter()
        .filter(|die| include(die))
        .map(format_die)
        .collect::<Vec<_>>();
    assert_eq!(actual, expected, "unexpected {label} dice");
}

fn format_die(die: &Die) -> String {
    let mut output = String::new();
    for notation in crate::protocol::notation::DIE_PROPERTY_PREFIXES {
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

fn format_side(die: &Die, side: usize) -> String {
    die.m_swing_type[side].map_or_else(
        || die.m_sides[side].to_string(),
        |swing| format!("{swing}-{}", die.m_sides[side]),
    )
}
