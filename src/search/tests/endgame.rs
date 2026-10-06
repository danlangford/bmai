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
    let outcomes =
        attack_outcomes(&game, &only_attack(&game, crate::Attack::Power), 1_000).unwrap();
    assert_eq!(outcomes.len(), 6);
    for outcome in &outcomes {
        assert!((outcome.probability - 1.0 / 6.0).abs() < 1e-12);
    }
}

#[test]
fn a_trip_succeeds_with_article_nines_odds() {
    // A Trip d6 against a d10 succeeds (6 + 1) / (2 * 10) of the time.
    let game = fight("t6:3", "10:7");
    let outcomes = attack_outcomes(&game, &only_attack(&game, crate::Attack::Trip), 1_000).unwrap();
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

#[test]
fn a_trip_that_can_fail_forever_is_left_to_monte_carlo() {
    // Article 10: the Shadow die can never take the 1, so its owner passes,
    // and each failed Trip rerolls the Shadow die into a repeated position.
    let game = fight("t1:1", "s6:4");
    assert!(Solver::new(500, 100_000).best_move(&game).is_none());
}

#[test]
fn too_many_ornery_rerolls_are_left_to_monte_carlo() {
    let game = fight("o20:20\no20:5\no20:5\no20:5", "20:3");
    assert!(Solver::new(500, 100_000).best_move(&game).is_none());
}

#[test]
fn scripted_draws_refuse_other_randomness() {
    let result = std::panic::catch_unwind(|| crate::Rng::scripted(Vec::new()).rand());
    assert!(result.is_err());
}

/// An independent solver for checking the real one: no cache, no merging of
/// outcomes, its own walk through the draws. Too slow for anything else.
fn naive_value(game: &Game, passed: bool, depth: usize) -> f64 {
    assert!(depth < 40, "corpus positions must not cycle");
    if crate::search::fight_over(game) {
        return f64::from(crate::search::win_probability(game));
    }
    crate::search::moves_including_pass(game, 500)
        .iter()
        .map(|action| naive_move_value(game, action, passed, depth))
        .fold(0.0, f64::max)
}

fn naive_move_value(game: &Game, action: &Move, passed: bool, depth: usize) -> f64 {
    if action.action != crate::Action::Attack {
        if passed {
            return f64::from(crate::search::win_probability(game));
        }
        let mut next = game.clone();
        next.players.swap(0, 1);
        return 1.0 - naive_value(&next, true, depth + 1);
    }
    let mut total = 0.0;
    let mut stack = vec![Vec::<u32>::new()];
    while let Some(faces) = stack.pop() {
        let mut rng = crate::Rng::scripted(faces.clone());
        let mut next = game.clone();
        let extra_turn = apply_attack(&mut next, action, &mut rng);
        let ranges = rng.scripted_ranges().unwrap().to_vec();
        if faces.len() < ranges.len() {
            // This path drew more than it scripted: branch on that draw.
            for face in 0..ranges[faces.len()] {
                let mut longer = faces.clone();
                longer.push(face);
                stack.push(longer);
            }
            continue;
        }
        let probability: f64 = ranges.iter().map(|&range| 1.0 / f64::from(range)).product();
        let after = if extra_turn {
            naive_value(&next, false, depth + 1)
        } else {
            next.players.swap(0, 1);
            1.0 - naive_value(&next, false, depth + 1)
        };
        total += probability * after;
    }
    total
}

/// Seeded small positions with common skills; no Trip, whose failures can
/// repeat a position, which the naive solver cannot handle.
fn corpus(count: usize) -> Vec<Game> {
    let mut rng = crate::Rng::untraced_default();
    rng.reseed(20_261_006);
    // Time and Space and Morphing grant extra turns; Shadow and Konstant
    // leave sides with no attack, so passes occur.
    let kinds = [
        "", "", "v", "p", "z", "s", "s", "f", "k", "k", "H", "w", "^", "^", "m",
    ];
    let sizes = [2u32, 4, 6, 8];
    let die = |rng: &mut crate::Rng| {
        let kind = kinds[rng.rand_below(kinds.len() as u32) as usize];
        let size = sizes[rng.rand_below(sizes.len() as u32) as usize];
        format!("{kind}{size}:{}", 1 + rng.rand_below(size))
    };
    (0..count)
        .map(|_| {
            let mine = 1 + rng.rand_below(2) as usize;
            let theirs = 1 + rng.rand_below(2) as usize;
            let player0 = (0..mine).map(|_| die(&mut rng)).collect::<Vec<_>>();
            let player1 = (0..theirs).map(|_| die(&mut rng)).collect::<Vec<_>>();
            let score = |rng: &mut crate::Rng| rng.rand_below(20);
            let (score0, score1) = (score(&mut rng), score(&mut rng));
            native_fixture_game(&format!(
                "game\nfight\nplayer 0 {mine} {score0}\n{}\nplayer 1 {theirs} {score1}\n{}\n",
                player0.join("\n"),
                player1.join("\n")
            ))
        })
        .collect()
}

#[test]
fn the_solver_matches_a_naive_solver_on_three_hundred_positions() {
    let mut solved = 0;
    for (index, game) in corpus(300).iter().enumerate() {
        let Some((_, value)) = Solver::new(500, 1_000_000).best_move(game) else {
            continue;
        };
        let naive = naive_value(game, false, 0);
        assert!(
            (value - naive).abs() < 1e-12,
            "position {index}: solver {value}, naive {naive}\n{:?}",
            game.players
        );
        solved += 1;
    }
    assert_eq!(
        solved, 300,
        "every corpus position is small enough to solve"
    );
}
