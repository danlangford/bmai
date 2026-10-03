// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;
use crate::engines::{DecisionContext, Engine, MonteCarlo};

pub(crate) type Engines = [Box<dyn Engine>; 2];

pub fn play_games(template: &Game, games: usize, rng: &mut Rng, ai: &Bmai3) -> [usize; 2] {
    let engine = || -> Box<dyn Engine> { Box::new(MonteCarlo::new(ai.clone())) };
    play_games_with_policies(template, games, rng, &[engine(), engine()])
}

pub(crate) fn play_games_with_policies(
    template: &Game,
    games: usize,
    rng: &mut Rng,
    policies: &Engines,
) -> [usize; 2] {
    play_games_with_policies_internal(template, games, rng, policies, None)
}

pub(crate) fn play_games_with_policies_native(
    template: &Game,
    games: usize,
    rng: &mut Rng,
    policies: &Engines,
    root_seed: u64,
    workers: usize,
    decision_index: &mut u64,
) -> [usize; 2] {
    let mut native = NativeReplaySequence {
        algorithm: rng.algorithm(),
        root_seed,
        workers,
        decision_index,
    };
    play_games_with_policies_internal(template, games, rng, policies, Some(&mut native))
}

pub(super) fn play_games_with_policies_internal(
    template: &Game,
    games: usize,
    rng: &mut Rng,
    policies: &Engines,
    mut native: Option<&mut NativeReplaySequence<'_>>,
) -> [usize; 2] {
    let mut matches = [0, 0];
    for _ in 0..games {
        let result = play_match_with_policies(template, rng, policies, native.as_deref_mut());
        matches[result.winner] += 1;
        println!(
            "game over {} - {} - {}",
            result.wins[0], result.wins[1], result.ties
        );
    }
    matches
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct MatchResult {
    pub(super) winner: usize,
    pub(super) wins: [u8; 2],
    pub(super) ties: usize,
    pub(super) initiative_winner: usize,
    pub(super) reserves_used: usize,
}

pub(super) fn play_match_with_policies(
    template: &Game,
    rng: &mut Rng,
    policies: &Engines,
    mut native: Option<&mut NativeReplaySequence<'_>>,
) -> MatchResult {
    let mut game = template.clone();
    let mut wins = [0u8, 0u8];
    let mut ties = 0usize;
    let mut initiative = 0;
    let mut reserves_used = 0;
    while wins[0] < template.target_wins && wins[1] < template.target_wins {
        restore_dice_for_new_round(&mut game, template);
        let round = play_round_with_policies(&mut game, rng, policies, native.as_deref_mut());
        let Some(winner) = round.0 else {
            ties += 1;
            continue;
        };
        initiative = round.1;
        wins[winner] += 1;
        let loser = 1 - winner;
        game.players[loser].swing_set = SwingSet::Not;
        for die in &mut game.players[loser].dice {
            if die.swing_type.iter().any(Option::is_some) {
                die.not_set = true;
            }
        }
        if wins[0] < template.target_wins
            && wins[1] < template.target_wins
            && game.players[loser].dice.iter().any(|die| die.in_reserve)
        {
            let mut oriented = game.clone();
            if loser == 1 {
                oriented.players.swap(0, 1);
            }
            let selected = policies[loser].reserve(
                &oriented,
                &mut DecisionContext::new(rng, native.as_deref_mut()),
            );
            if let Some(index) = selected {
                apply_use_reserve(&mut game.players[loser].dice[index]);
                reserves_used += 1;
            }
        }
    }
    MatchResult {
        winner: usize::from(wins[1] > wins[0]),
        wins,
        ties,
        initiative_winner: initiative,
        reserves_used,
    }
}

pub(crate) fn play_fair_games(
    template: &Game,
    games: usize,
    rng: &mut Rng,
    policies: &Engines,
) -> [[usize; 2]; 2] {
    play_fair_games_internal(template, games, rng, policies, None)
}

pub(crate) fn play_fair_games_native(
    template: &Game,
    games: usize,
    rng: &mut Rng,
    policies: &Engines,
    root_seed: u64,
    workers: usize,
    decision_index: &mut u64,
) -> [[usize; 2]; 2] {
    let mut native = NativeReplaySequence {
        algorithm: rng.algorithm(),
        root_seed,
        workers,
        decision_index,
    };
    play_fair_games_internal(template, games, rng, policies, Some(&mut native))
}

pub(super) fn play_fair_games_internal(
    template: &Game,
    games: usize,
    rng: &mut Rng,
    policies: &Engines,
    mut native: Option<&mut NativeReplaySequence<'_>>,
) -> [[usize; 2]; 2] {
    let mut wins = [[0usize; 2]; 2];
    for _ in 0..games {
        let result = play_match_with_policies(template, rng, policies, native.as_deref_mut());
        wins[result.initiative_winner][result.winner] += 1;
    }
    wins
}

pub(super) fn play_round_with_policies(
    game: &mut Game,
    rng: &mut Rng,
    policies: &Engines,
    mut native: Option<&mut NativeReplaySequence<'_>>,
) -> (Option<usize>, usize) {
    play_preround_with_policies(game, rng, policies, native.as_deref_mut());
    for player in &mut game.players {
        player.score = 0.0;
        for die in &mut player.dice {
            die.not_set = true;
            roll_die(die, rng);
        }
        player.score = player
            .dice
            .iter()
            .filter(|die| die.is_available())
            .map(|die| die.score(true))
            .sum();
        optimize_dice(player);
    }
    let mut phase_player = initiative_winner(game);
    let initiative_winner = phase_player;
    loop {
        let player = 1 - phase_player;
        if !has_available_property(&game.players[player], property::CHANCE) {
            break;
        }
        let action = policies[player].chance(
            game,
            player,
            phase_player,
            &mut DecisionContext::new(rng, native.as_deref_mut()),
        );
        let (next_phase, continues) = apply_chance_move(game, player, phase_player, &action, rng);
        if trace_settings().chance {
            eprintln!(
                "CHANCE_APPLY player={player} previous={phase_player} next={next_phase} continues={continues} values={:?}",
                game.players[player]
                    .dice
                    .iter()
                    .map(Die::value_total)
                    .collect::<Vec<_>>()
            );
        }
        phase_player = next_phase;
        if !continues {
            break;
        }
    }
    loop {
        let player = 1 - phase_player;
        if !has_available_property(&game.players[player], property::FOCUS) {
            break;
        }
        let action = policies[player].focus(
            game,
            player,
            phase_player,
            &mut DecisionContext::new(rng, native.as_deref_mut()),
        );
        if action.values.is_empty() {
            break;
        }
        apply_focus_move(game, player, &action);
        phase_player = player;
    }
    let mut consecutive_passes = 0;
    for _ in 0..256 {
        if fight_over(game) {
            break;
        }
        let mut oriented = game.clone();
        if phase_player == 1 {
            oriented.players.swap(0, 1);
        }
        let action = policies[phase_player]
            .attack(
                &oriented,
                &mut DecisionContext::new(rng, native.as_deref_mut()),
            )
            .choice;
        if action.action == Action::Surrender {
            game.players[phase_player].score = -1000.0;
            break;
        } else if action.action != Action::Attack {
            consecutive_passes += 1;
            if consecutive_passes == 2 {
                break;
            }
            recover_dizzy_dice(&mut game.players[phase_player]);
        } else {
            consecutive_passes = 0;
            let extra_turn =
                apply_attack_for_players(game, &action, phase_player, 1 - phase_player, rng);
            recover_dizzy_dice(&mut game.players[phase_player]);
            if extra_turn {
                continue;
            }
        }
        phase_player = 1 - phase_player;
    }
    (round_winner(game), initiative_winner)
}

pub(super) fn round_winner(game: &Game) -> Option<usize> {
    match game.players[1].score.partial_cmp(&game.players[0].score) {
        Some(std::cmp::Ordering::Greater) => Some(1),
        Some(std::cmp::Ordering::Less) => Some(0),
        Some(std::cmp::Ordering::Equal) => None,
        None => panic!("round score is NaN"),
    }
}

pub(super) fn play_preround_with_policies(
    game: &mut Game,
    rng: &mut Rng,
    policies: &Engines,
    mut native: Option<&mut NativeReplaySequence<'_>>,
) {
    for (player, policy) in policies.iter().enumerate() {
        if game.players[player].swing_set != SwingSet::Not
            || !needs_set_swing(&game.players[player])
        {
            game.players[player].swing_set = SwingSet::Locked;
            continue;
        }
        let selected = policy.swing(
            game,
            player,
            &mut DecisionContext::new(rng, native.as_deref_mut()),
        );
        apply_swing_move(&mut game.players[player], &selected);
        game.players[player].swing_set = SwingSet::Locked;
    }
}

pub(super) fn play_preround(game: &mut Game, rng: &mut Rng, ai: &Bmai3, level: usize) -> usize {
    // C++ never restores the level after a simulated preround, so the next
    // player's choice starts a ply deeper.
    let mut player_level = level;
    for player in 0..2 {
        if game.players[player].swing_set != SwingSet::Not
            || !needs_set_swing(&game.players[player])
        {
            game.players[player].swing_set = SwingSet::Locked;
            continue;
        }
        let (selected, _) = select_swing_action(game, player, rng, ai, player_level, None);
        apply_swing_move(&mut game.players[player], &selected);
        game.players[player].swing_set = SwingSet::Locked;
        if level > 1 {
            player_level += 1;
        }
    }
    player_level
}
