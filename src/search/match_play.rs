// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::io::{self, Write};

use super::*;
use crate::engines::{DecisionContext, Engine};

pub(crate) type Engines = [Box<dyn Engine>; 2];

/// ButtonWeavers cancels a game when its 200th round ends, so a game that can
/// only tie still ends.
const MAX_ROUNDS: usize = 200;

/// A game cancelled at the round limit counts for neither player.
pub(crate) fn play_games<W: Write>(
    template: &Game,
    games: usize,
    rng: &mut Rng,
    policies: &Engines,
    native: &mut NativeReplaySequence<'_>,
    output: &mut W,
) -> io::Result<[usize; 2]> {
    let mut matches = [0, 0];
    for _ in 0..games {
        let result = play_match_with_policies(template, rng, policies, native);
        let outcome = match result.winner {
            Some(winner) => {
                matches[winner] += 1;
                "over"
            }
            None => "cancelled",
        };
        writeln!(
            output,
            "game {outcome} {} - {} - {}",
            result.wins[0], result.wins[1], result.ties
        )?;
    }
    Ok(matches)
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MatchResult {
    /// `None` when the match is cancelled at the round limit.
    pub(crate) winner: Option<usize>,
    pub(crate) wins: [u8; 2],
    pub(super) ties: usize,
    pub(super) initiative_winner: usize,
    pub(super) reserves_used: usize,
}

pub(crate) fn play_match_with_policies(
    template: &Game,
    rng: &mut Rng,
    policies: &Engines,
    native: &mut NativeReplaySequence<'_>,
) -> MatchResult {
    let recipe = choose_auxiliary_dice(template, rng, policies, native);
    let mut game = recipe.clone();
    let mut wins = [0u8, 0u8];
    let mut ties = 0usize;
    let mut initiative = 0;
    let mut reserves_used = 0;
    let mut cancelled = false;
    while wins[0] < template.target_wins && wins[1] < template.target_wins {
        let round_number = usize::from(wins[0]) + usize::from(wins[1]) + ties + 1;
        let round = play_round_with_policies(&mut game, rng, policies, Some(native));
        if round_number >= MAX_ROUNDS {
            cancelled = true;
            break;
        }
        // ButtonWeavers deals the next round before offering reserve dice.
        restore_dice_for_new_round(&mut game, &recipe, &round.selections);
        let Some(winner) = round.winner else {
            ties += 1;
            continue;
        };
        initiative = round.initiative_winner;
        wins[winner] += 1;
        let loser = 1 - winner;
        game.players[loser].swing_set = SwingSet::Not;
        if wins[0] < template.target_wins
            && wins[1] < template.target_wins
            && game.players[loser].dice.iter().any(|die| die.in_reserve)
        {
            let mut oriented = game.clone();
            if loser == 1 {
                oriented.players.swap(0, 1);
            }
            let selected =
                policies[loser].reserve(&oriented, &mut DecisionContext::new(rng, Some(native)));
            if let Some(index) = selected {
                apply_use_reserve(&mut game.players[loser].dice[index]);
                reserves_used += 1;
            }
        }
    }
    MatchResult {
        winner: (!cancelled).then(|| usize::from(wins[1] > wins[0])),
        wins,
        ties,
        initiative_winner: initiative,
        reserves_used,
    }
}

/// ButtonWeavers decides Auxiliary dice once per game and rewrites both
/// recipes, so every later round deals the outcome too.
pub(super) fn choose_auxiliary_dice(
    template: &Game,
    rng: &mut Rng,
    policies: &Engines,
    native: &mut NativeReplaySequence<'_>,
) -> Game {
    let mut recipe = template.clone();
    offer_courtesy_auxiliary(&mut recipe);
    if recipe
        .players
        .iter()
        .all(|player| auxiliary_die(player).is_none())
    {
        return recipe;
    }
    // A decline ends the choice for both players on ButtonWeavers.
    let accepted = (0..2).all(|player| {
        let mut oriented = recipe.clone();
        if player == 1 {
            oriented.players.swap(0, 1);
        }
        policies[player]
            .auxiliary(&oriented, &mut DecisionContext::new(rng, Some(native)))
            .choice
            .is_some()
    });
    apply_auxiliary_decision(&mut recipe, accepted);
    recipe
}

pub(crate) fn play_fair_games(
    template: &Game,
    games: usize,
    rng: &mut Rng,
    policies: &Engines,
    native: &mut NativeReplaySequence<'_>,
) -> [[usize; 2]; 2] {
    let mut wins = [[0usize; 2]; 2];
    for _ in 0..games {
        let result = play_match_with_policies(template, rng, policies, native);
        if let Some(winner) = result.winner {
            wins[result.initiative_winner][winner] += 1;
        }
    }
    wins
}

pub(super) struct Round {
    pub(super) winner: Option<usize>,
    pub(super) initiative_winner: usize,
    pub(super) selections: RoundSelections,
}

pub(super) fn play_round_with_policies(
    game: &mut Game,
    rng: &mut Rng,
    policies: &Engines,
    mut native: Option<&mut NativeReplaySequence<'_>>,
) -> Round {
    let selections = play_preround_with_policies(game, rng, policies, native.as_deref_mut());
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
    Round {
        winner: round_winner(game),
        initiative_winner,
        selections,
    }
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
) -> RoundSelections {
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
    record_round_selections(game)
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
        let (selected, _, _) = select_swing_action(game, player, rng, ai, player_level, None);
        apply_swing_move(&mut game.players[player], &selected);
        game.players[player].swing_set = SwingSet::Locked;
        if level > 1 {
            player_level += 1;
        }
    }
    player_level
}
