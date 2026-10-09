// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;
use crate::Attack::Power;
use crate::search::initiative::{apply_chance_move, apply_focus_move};
use crate::search::{ChanceMove, FocusMove};
use test_support::scenario;

#[test]
fn an_attacking_value_die_scores_its_new_value() {
    scenario()
        .attacker("v20:4")
        .attacks(Power)
        .defender("6:4")
        .with_scores(2.0, 3.0)
        .seed(1)
        .expect_attacker_dice(["v20:20"])
        // Half of 20 replaces half of 4; the capture counts its value, 4.
        .expect_scores(14.0, 0.0)
        .run();
}

#[test]
fn a_value_capture_makes_the_die_value_only_for_the_round() {
    scenario()
        .attacker("v8:8")
        .attacks(Power)
        .defender("20:5")
        .expect_captured_defender_dice(["v20:5"])
        .expect_next_round_defender_dice(["20"])
        .run();
}

#[test]
fn a_focused_value_die_scores_its_lowered_value() {
    let mut game = native_fixture_game("game\nfight\nplayer 0 1 4\nvf20:8\nplayer 1 1 3\n6:3\n");
    apply_focus_move(
        &mut game,
        0,
        &FocusMove {
            values: vec![(0, 2)],
        },
    );
    assert_eq!(game.players[0].score, 1.0);
}

fn score_from_dice(game: &Game, player: usize) -> f32 {
    let own: f32 = game.players[player]
        .dice
        .iter()
        .filter(|die| die.is_available())
        .map(|die| die.score(true))
        .sum();
    // Boom marks its own lost die captured, so it needs excluding here.
    let captured: f32 = game.players[1 - player]
        .dice
        .iter()
        .filter(|die| die.captured)
        .map(|die| die.score(false))
        .sum();
    own + captured
}

fn assert_score_follows_dice(before: &Game, after: &Game, context: &str) {
    for player in 0..2 {
        let offset = before.players[player].score - score_from_dice(before, player);
        assert_eq!(
            after.players[player].score,
            score_from_dice(after, player) + offset,
            "player {player} {context}"
        );
    }
}

#[test]
fn every_attack_keeps_the_score_equal_to_the_dice() {
    for (name, attacker, defender) in [
        // Game 121248, where a stale Value score showed as a certain win.
        ("121248", "wHF4:2\n8:1\nvz20:4", "2/8-2:1\n6/10-6:6"),
        ("Trip Morph reroll", "tmv6:2", "20:10"),
        ("Radioactive products", "t%mv8:5", "12:3"),
    ] {
        let count = |dice: &str| dice.lines().count();
        let game = native_fixture_game(&format!(
            "game\nfight\nplayer 0 {} 0\n{attacker}\nplayer 1 {} 0\n{defender}\n",
            count(attacker),
            count(defender)
        ));
        for action in game.valid_attacks(80) {
            for seed in 1..=20 {
                let mut after = game.clone();
                let mut rng = Rng::default();
                rng.reseed(seed);
                apply_attack(&mut after, &action, &mut rng);
                assert_score_follows_dice(
                    &game,
                    &after,
                    &format!("{name}: {action:?} seed {seed}"),
                );
            }
        }
    }
}

#[test]
fn a_chance_reroll_keeps_the_score_equal_to_the_dice() {
    let game = native_fixture_game("game\nfight\nplayer 0 2 0\ncv20:4\n6:3\nplayer 1 1 0\n10:5\n");
    for seed in 1..=20 {
        let mut after = game.clone();
        let mut rng = Rng::default();
        rng.reseed(seed);
        apply_chance_move(&mut after, 0, 1, &ChanceMove { reroll: vec![0] }, &mut rng);
        assert_score_follows_dice(&game, &after, &format!("seed {seed}"));
    }
}
