// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::mechanics::{assert_dice_matching, parse_game};
use super::*;
use crate::search::{ApplyChanceMove, ApplyFocusMove, ChanceMove, FocusMove};

#[derive(Default)]
pub(crate) struct InitiativeScenario {
    player_dice: Vec<String>,
    opponent_dice: Vec<String>,
    player_index: usize,
    chance_rerolls: Option<Vec<usize>>,
    focus_values: Option<Vec<(usize, u8)>>,
    seed: Option<u32>,
    expected_player_dice: Option<Vec<String>>,
    expected_dice_next_turn: Option<Vec<String>>,
    expected_chance_success: Option<bool>,
    expected_initiative: Option<Option<usize>>,
}

impl InitiativeScenario {
    pub(crate) fn player(mut self, dice: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.player_dice.extend(dice.into_iter().map(Into::into));
        self
    }

    pub(crate) fn opponent(mut self, dice: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.opponent_dice.extend(dice.into_iter().map(Into::into));
        self
    }

    /// C++ keys Chance success to player 0, so the seat matters.
    pub(crate) fn seated_as(mut self, player: usize) -> Self {
        self.player_index = player;
        self
    }

    pub(crate) fn chance_rerolls(mut self, dice: impl IntoIterator<Item = usize>) -> Self {
        self.chance_rerolls = Some(dice.into_iter().collect());
        self
    }

    pub(crate) fn focuses(mut self, dice: impl IntoIterator<Item = (usize, u8)>) -> Self {
        self.focus_values = Some(dice.into_iter().collect());
        self
    }

    pub(crate) fn seed(mut self, seed: u32) -> Self {
        self.seed = Some(seed);
        self
    }

    pub(crate) fn expect_player_dice(
        mut self,
        dice: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.expected_player_dice = Some(dice.into_iter().map(Into::into).collect());
        self
    }

    pub(crate) fn expect_player_dice_next_turn(
        mut self,
        dice: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.expected_dice_next_turn = Some(dice.into_iter().map(Into::into).collect());
        self
    }

    pub(crate) fn expect_chance_success(mut self, success: bool) -> Self {
        self.expected_chance_success = Some(success);
        self
    }

    pub(crate) fn expect_initiative(mut self, player: Option<usize>) -> Self {
        self.expected_initiative = Some(player);
        self
    }

    #[track_caller]
    pub(crate) fn run(self) {
        assert!(
            self.chance_rerolls.is_some() != self.focus_values.is_some(),
            "initiative scenario needs exactly one of chance_rerolls() or focuses()"
        );
        let player = self.player_index;
        let opponent = 1 - player;
        let mut seats = [&self.opponent_dice, &self.opponent_dice];
        seats[player] = &self.player_dice;
        let mut game = parse_game(seats[0], seats[1]);
        let mut rng = Rng::default();
        if let Some(seed) = self.seed {
            rng.SRand(seed);
        }

        if let Some(reroll) = self.chance_rerolls {
            let (_, success) = ApplyChanceMove(
                &mut game,
                player,
                opponent,
                &ChanceMove { reroll },
                &mut rng,
            );
            if let Some(expected) = self.expected_chance_success {
                assert_eq!(success, expected, "unexpected Chance result");
            }
        }
        if let Some(values) = self.focus_values {
            ApplyFocusMove(&mut game, player, &FocusMove { values });
        }

        if let Some(expected) = self.expected_initiative {
            assert_eq!(game.CheckInitiative(), expected, "unexpected initiative");
        }
        if let Some(expected) = self.expected_player_dice {
            assert_dice_matching("player", &game, player, &expected, |_| true);
        }
        if let Some(expected) = self.expected_dice_next_turn {
            game.RecoverDizzyDice(player);
            assert_dice_matching("next-turn player", &game, player, &expected, |_| true);
        }
    }
}
