// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;
use crate::search::endgame::{Solver, attack_outcomes};

fn fight(player0: &str, player1: &str) -> Game {
    let count = |dice: &str| dice.lines().count();
    native_fixture_game(&format!(
        "game\nfight\nplayer 0 {} 0\n{player0}\nplayer 1 {} 0\n{player1}\n",
        count(player0),
        count(player1)
    ))
}

fn only_attack(game: &Game, attack: crate::Attack) -> Move {
    let mut moves = game
        .generate_valid_attacks_in_cpp_order_for_search(500)
        .into_iter()
        .filter(|candidate| candidate.attack == Some(attack));
    let found = moves.next().expect("the attack is legal");
    assert!(moves.next().is_none(), "the attack is unique");
    found
}

#[test]
fn a_power_attack_rerolls_each_face_with_equal_odds() {
    let game = fight("6:5", "4:2");
    let outcomes = attack_outcomes(&game, &only_attack(&game, crate::Attack::Power));
    assert_eq!(outcomes.len(), 6);
    for outcome in &outcomes {
        assert!((outcome.probability - 1.0 / 6.0).abs() < 1e-12);
    }
}

#[test]
fn a_trip_succeeds_with_article_nines_odds() {
    // A Trip d6 against a d10 succeeds (6 + 1) / (2 * 10) of the time.
    let game = fight("t6:3", "10:7");
    let outcomes = attack_outcomes(&game, &only_attack(&game, crate::Attack::Trip));
    let total: f64 = outcomes.iter().map(|outcome| outcome.probability).sum();
    let captured: f64 = outcomes
        .iter()
        .filter(|outcome| outcome.game.players[1].dice.iter().any(|die| die.captured))
        .map(|outcome| outcome.probability)
        .sum();
    assert!((total - 1.0).abs() < 1e-12, "{total}");
    assert!((captured - 7.0 / 20.0).abs() < 1e-12, "{captured}");
}

#[test]
fn game_121248_leaves_the_opponent_a_real_chance() {
    // After BMAIBagels' 18:44 move, ElihuRoot to move, where BMAIR once said
    // he had no chance. His best line also rerolls the Value die when he
    // takes the last die, which a hand count puts near 31%.
    let game = native_fixture_game(
        "game\nfight\nplayer 0 3 21\nwHF4:2\n8:1\nvz20:4\nplayer 1 2 33\n2/8-2:1\n6/10-6:6\n",
    );
    let (_, value) = Solver::new(500, 1_000_000)
        .best_move(&game)
        .expect("small enough to solve");
    assert!((value - 0.304_062_5).abs() < 1e-9, "{value}");
}
