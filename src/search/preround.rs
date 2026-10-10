// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

pub(crate) fn select_swing_action(
    game: &Game,
    player: usize,
    rng: &mut Rng,
    ai: &Bmai3,
    level: usize,
    native: Option<NativeEvaluation>,
) -> (SwingMove, f32) {
    if game.players[player].swing_set != SwingSet::Not {
        return (current_swing_move(&game.players[player]), 0.0);
    }
    let mut moves = generate_swing_moves(&game.players[player]);
    if moves.is_empty() {
        return (SwingMove::empty(), 0.0);
    }
    let max_moves = ai.max_branch / ai.min_sims;
    if moves.len() > max_moves {
        let mut selection_rng =
            native.map(|context| context.simulation_rng(NATIVE_ENUMERATION_STREAM, 0, 0));
        randomly_select_swing_moves(
            &mut moves,
            &game.players[player],
            max_moves,
            selection_rng.as_mut().unwrap_or(rng),
        );
        moves.shrink_to_fit();
    }
    let sims = ai.compute_number_sims(moves.len(), level);
    let mut scores = vec![0.0f32; moves.len()];
    let mut candidate_indices = native.map(|_| (0..moves.len()).collect::<Vec<_>>());
    let mut best_score = -1.0f32;
    let mut best = moves[0];
    let mut sims_run = 0usize;
    let mut simulation = game.clone();
    while sims_run < sims {
        let batch = if ai.cull_moves {
            ai.sims_per_check.min(sims - sims_run)
        } else {
            sims - sims_run
        };
        let native_results = native.map(|context| {
            let batch_index = if ai.cull_moves {
                sims_run / ai.sims_per_check.max(1)
            } else {
                0
            };
            let tasks = moves
                .iter()
                .copied()
                .enumerate()
                .flat_map(|(index, candidate)| {
                    let candidate_index = candidate_indices.as_ref().unwrap()[index];
                    (0..batch)
                        .map(move |simulation_index| (candidate, candidate_index, simulation_index))
                })
                .collect();
            crate::native::ordered_parallel_map(
                tasks,
                context.workers,
                |(candidate, candidate_index, simulation_index)| {
                    let mut simulation = game.clone();
                    apply_swing_move(&mut simulation.players[player], &candidate);
                    simulation.players[player].swing_set = SwingSet::Locked;
                    let mut simulation_rng = context.simulation_rng(
                        candidate_index,
                        batch_index,
                        sims_run + simulation_index,
                    );
                    evaluate_swing_move(&mut simulation, player, &mut simulation_rng, ai, level)
                },
            )
        });
        for (index, candidate) in moves.iter().enumerate() {
            if let Some(results) = &native_results {
                scores[index] += results[index * batch..(index + 1) * batch]
                    .iter()
                    .sum::<f32>();
            } else {
                for _ in 0..batch {
                    restore_simulation(&mut simulation, game);
                    apply_swing_move(&mut simulation.players[player], candidate);
                    simulation.players[player].swing_set = SwingSet::Locked;
                    scores[index] += evaluate_swing_move(&mut simulation, player, rng, ai, level);
                }
            }
            if scores[index] > best_score {
                best_score = scores[index];
                best = *candidate;
            }
        }
        sims_run += batch;
        if sims_run >= sims || !ai.cull_moves {
            break;
        }
        if moves.len() == 1 {
            if completes_native_probability_sample(native) {
                continue;
            }
            break;
        }
        let progress = sims_run as f32 / sims as f32;
        let threshold = ai.min_best_score_threshold
            + progress * (ai.max_best_score_threshold - ai.min_best_score_threshold);
        let mut delta_threshold = (1.0 - progress) * ai.sims_per_check as f32 * 0.5;
        if best_score > 1.0 && delta_threshold >= best_score {
            delta_threshold = best_score;
        }
        let mut index = 0;
        while index < moves.len() {
            let delta = best_score - scores[index];
            if delta >= (sims - sims_run) as f32
                || scores[index] < best_score * threshold && delta >= delta_threshold
            {
                moves.swap_remove(index);
                scores.swap_remove(index);
                if let Some(indices) = &mut candidate_indices {
                    indices.swap_remove(index);
                }
            } else {
                index += 1;
            }
        }
        if moves.len() == 1 && !completes_native_probability_sample(native) {
            break;
        }
    }
    let probability = best_score / sims_run as f32;
    (best, probability)
}

pub(super) fn auxiliary_die(player: &crate::game::Player) -> Option<usize> {
    player
        .dice
        .iter()
        .position(|die| die.has_property(property::AUXILIARY))
}

/// Gordo refuses a single swing die of its own V-Z types, which would force
/// two dice to share a size; ButtonWeavers lets a Twin swing die through.
pub(crate) fn acceptable_auxiliary_die(player: &crate::game::Player) -> Option<usize> {
    auxiliary_die(player).filter(|index| {
        let die = &player.dice[*index];
        player.specials & special::UNIQUE_SIZES == 0
            || die.has_property(property::TWIN)
            || !die.swing_type[0].is_some_and(|swing| ('V'..='Z').contains(&swing))
    })
}

/// ButtonWeavers gives a player without an Auxiliary die a copy of the
/// opponent's, so both players choose.
pub(crate) fn offer_courtesy_auxiliary(game: &mut Game) {
    let (source, index, target) = match game.players.each_ref().map(auxiliary_die) {
        [Some(index), None] => (0, index, 1),
        [None, Some(index)] => (1, index, 0),
        _ => return,
    };
    assert!(
        game.players[target].dice.len() < crate::game::MAX_DICE,
        "courtesy Auxiliary die exceeds player {target} capacity {}",
        crate::game::MAX_DICE
    );
    let mut die = game.players[source].dice[index];
    die.original_index = game.players[target].dice.len();
    die.value = None;
    die.not_set = true;
    game.players[target].dice.push(die);
}

/// On ButtonWeavers a decline by either player removes every Auxiliary die.
pub(crate) fn apply_auxiliary_decision(game: &mut Game, accepted: bool) {
    let accepted = accepted
        && game.players.iter().all(|player| {
            auxiliary_die(player).is_none() || acceptable_auxiliary_die(player).is_some()
        });
    for player in &mut game.players {
        let selected = accepted.then(|| auxiliary_die(player)).flatten();
        // A lock keeps the swing sizes a position gave, so only an added die
        // that has none may lift it.
        if selected.is_some_and(|index| {
            let die = &player.dice[index];
            (0..2).any(|side| die.swing_type[side].is_some() && die.sides[side] == 0)
        }) {
            player.swing_set = SwingSet::Not;
        }
        let mut index = 0;
        player.dice.retain_mut(|die| {
            let keep = !die.has_property(property::AUXILIARY) || Some(index) == selected;
            if keep && die.has_property(property::AUXILIARY) {
                die.properties &= !property::AUXILIARY;
                die.value = None;
                die.not_set = true;
            }
            index += 1;
            keep
        });
    }
}

pub(super) fn evaluate_auxiliary_decision(game: &Game, accepted: bool, rng: &mut Rng) -> f32 {
    let mut simulation = game.clone();
    apply_auxiliary_decision(&mut simulation, accepted);
    // QAI, because BMAI here would nest a new search at every move.
    let quick = || -> Box<dyn crate::engines::Engine> { Box::new(crate::engines::Quick) };
    let round = play_round_with_policies(&mut simulation, rng, &[quick(), quick()], None);
    match round.winner {
        Some(0) => 1.0,
        None => 0.5,
        Some(1) => 0.0,
        Some(_) => unreachable!("Button Men has exactly two players"),
    }
}

pub(crate) fn select_native_bmai_auxiliary_action(
    game: &Game,
    native: NativeEvaluation,
    ai: &Bmai3,
) -> AuxiliarySearchResult {
    let Some(auxiliary) = acceptable_auxiliary_die(&game.players[0]) else {
        return AuxiliarySearchResult {
            die: None,
            score: 0.0,
            simulations: 0,
        };
    };
    let simulations = ai.compute_number_sims(2, 1);
    let candidates = [Some(auxiliary), None];
    let tasks = candidates
        .iter()
        .copied()
        .enumerate()
        .flat_map(|(candidate_index, candidate)| {
            (0..simulations)
                .map(move |simulation_index| (candidate_index, candidate, simulation_index))
        })
        .collect();
    let results = crate::native::ordered_parallel_map(
        tasks,
        native.workers,
        |(candidate_index, candidate, simulation_index)| {
            let mut simulation_rng = native.simulation_rng(candidate_index, 0, simulation_index);
            evaluate_auxiliary_decision(game, candidate.is_some(), &mut simulation_rng)
        },
    );
    let mut best = AuxiliarySearchResult {
        die: None,
        score: -1.0,
        simulations,
    };
    for (candidate, scores) in candidates
        .into_iter()
        .zip(results.chunks_exact(simulations))
    {
        let score = scores.iter().sum();
        if score > best.score {
            best.die = candidate;
            best.score = score;
        }
    }
    best
}

pub(crate) fn select_native_bmai_reserve_action(
    game: &Game,
    native: NativeEvaluation,
    ai: &Bmai3,
) -> Option<usize> {
    let reserve_indices = game.players[0]
        .dice
        .iter()
        .enumerate()
        .filter_map(|(index, die)| die.in_reserve.then_some(index))
        .collect::<Vec<_>>();
    let sims = ai.compute_number_sims(reserve_indices.len() + 1, 1);
    let candidates = reserve_indices
        .into_iter()
        .map(Some)
        .chain([None])
        .collect::<Vec<_>>();
    let tasks = candidates
        .iter()
        .copied()
        .enumerate()
        .flat_map(|(candidate_index, candidate)| {
            (0..sims).map(move |simulation_index| (candidate_index, candidate, simulation_index))
        })
        .collect();
    let results = crate::native::ordered_parallel_map(
        tasks,
        native.workers,
        |(candidate_index, candidate, simulation_index)| {
            let mut simulation = game.clone();
            if let Some(index) = candidate {
                apply_use_reserve(&mut simulation.players[0].dice[index]);
            }
            let mut simulation_rng = native.simulation_rng(candidate_index, 0, simulation_index);
            let fight_level = play_preround(&mut simulation, &mut simulation_rng, ai, 2);
            play_simulated_round(&mut simulation, &mut simulation_rng, ai, fight_level, 0)
        },
    );

    let mut best_score = -1.0f32;
    let mut best = None;
    for (candidate, scores) in candidates.into_iter().zip(results.chunks_exact(sims)) {
        let score = scores.iter().sum();
        if score > best_score {
            best_score = score;
            best = candidate;
        }
    }
    best
}

pub(super) fn apply_use_reserve(die: &mut Die) {
    die.in_reserve = false;
    die.properties &= !property::RESERVE;
    die.value = None;
    die.not_set = true;
}

pub(super) fn randomly_select_swing_moves(
    moves: &mut Vec<SwingMove>,
    player: &crate::game::Player,
    max: usize,
    rng: &mut Rng,
) {
    let swing_types = player
        .dice
        .iter()
        .filter(|die| !die.in_reserve)
        .flat_map(|die| die.swing_type.iter().flatten().copied())
        .collect::<std::collections::BTreeSet<_>>();
    let extreme_settings = |action: &SwingMove| {
        action
            .values()
            .iter()
            .filter(|(swing, value)| {
                let (minimum, maximum) = swing_range(*swing);
                *value == minimum || *value == maximum
            })
            .count()
    };
    let swing_dice = swing_types.len();
    let extreme_moves = moves
        .iter()
        .filter(|action| extreme_settings(action) == swing_dice)
        .count();

    if extreme_moves >= max {
        moves.retain(|action| extreme_settings(action) == swing_dice);
        return;
    }

    while moves.len() > max {
        let index = rng.rand_below(moves.len() as u32) as usize;
        let percentage_extreme = extreme_settings(&moves[index]) as f32 / swing_dice as f32;
        if rng.rand_f32() >= percentage_extreme {
            moves.swap_remove(index);
        }
    }
}

pub(super) fn evaluate_swing_move(
    game: &mut Game,
    player: usize,
    rng: &mut Rng,
    ai: &Bmai3,
    level: usize,
) -> f32 {
    let other = 1 - player;

    // C++ switches both sides to QAI at the terminal ply.
    if level >= ai.max_ply {
        if game.players[other].swing_set == SwingSet::Not {
            if needs_set_swing(&game.players[other]) {
                let selected = first_swing_move(&game.players[other])
                    .expect("a player needing a swing has a valid setting");
                apply_swing_move(&mut game.players[other], &selected);
            }
            game.players[other].swing_set = SwingSet::Locked;
        }
        return play_round_qai(game, rng, player, ai);
    }

    // Before the terminal ply, C++ uses the opponent's next decision estimate.
    if game.players[other].swing_set == SwingSet::Not {
        if !needs_set_swing(&game.players[other]) {
            game.players[other].swing_set = SwingSet::Locked;
            return play_round_to_next_bmai_action(game, rng, player, ai, level + 1);
        }
        if game.players[player].swing_set == SwingSet::Ready {
            game.players[player].swing_set = SwingSet::Not;
        }
        let (_, other_probability) = select_swing_action(game, other, rng, ai, level + 1, None);
        return 1.0 - other_probability;
    }

    play_round_to_next_bmai_action(game, rng, player, ai, level + 1)
}

pub(super) fn play_round_to_next_bmai_action(
    game: &mut Game,
    rng: &mut Rng,
    pov: usize,
    ai: &Bmai3,
    level: usize,
) -> f32 {
    roll_round_dice(game, rng);
    let phase = initiative_winner(game);
    evaluate_next_initiative_action(game, rng, ai, level, phase, pov, InitiativeStage::Chance)
}

pub(super) fn needs_set_swing(player: &crate::game::Player) -> bool {
    player
        .dice
        .iter()
        .filter(|die| !die.in_reserve)
        .any(|d| d.swing_type.iter().any(Option::is_some) || d.has_property(property::OPTION))
}

pub(super) fn current_swing_move(player: &crate::game::Player) -> SwingMove {
    let mut action = SwingMove::empty();
    let mut seen = [false; 26];
    for (index, die) in player.dice.iter().enumerate() {
        if die.in_reserve {
            continue;
        }
        for side in 0..2 {
            if let Some(swing @ 'A'..='Z') = die.swing_type[side] {
                let seen_index = (swing as u8 - b'A') as usize;
                if !seen[seen_index] {
                    action.push_value((swing, die.sides[side]));
                    seen[seen_index] = true;
                }
            }
        }
        if die.has_property(property::OPTION) {
            // parse_die keeps the selected option in slot zero.
            action.push_option((index, false));
        }
    }
    action
}

struct SwingAction {
    swing: Option<char>,
    die: usize,
    values: Vec<u8>,
}

fn swing_actions(player: &crate::game::Player) -> Vec<SwingAction> {
    let mut actions = Vec::new();
    let mut swings = player
        .dice
        .iter()
        .filter(|die| !die.in_reserve)
        .flat_map(|d| d.swing_type.iter().flatten().copied())
        .collect::<Vec<_>>();
    swings.sort_unstable();
    swings.dedup();
    for swing in swings {
        let (min, max) = swing_range(swing);
        actions.push(SwingAction {
            swing: Some(swing),
            die: 0,
            values: (min..=max).collect(),
        });
    }
    for (index, die) in player.dice.iter().enumerate() {
        if !die.in_reserve && die.has_property(property::OPTION) {
            actions.push(SwingAction {
                swing: None,
                die: index,
                values: vec![0, 1],
            });
        }
    }
    actions
}

fn swing_move_allowed(player: &crate::game::Player, candidate: &SwingMove) -> bool {
    let unique_sizes = player.specials & special::UNIQUE_SIZES != 0;
    if unique_sizes || player.specials & special::UNIQUE_SWING != 0 {
        // Option dice count at the side this candidate chooses.
        let fixed_sizes = player
            .dice
            .iter()
            .enumerate()
            .filter(|(_, die)| !die.in_reserve && die.swing_type.iter().all(Option::is_none))
            .map(|(index, die)| {
                let second = candidate
                    .options()
                    .iter()
                    .any(|(option, second)| *option == index && *second);
                if die.has_property(property::OPTION) {
                    u16::from(die.sides[usize::from(second)])
                } else {
                    die.sides_max()
                }
            })
            .collect::<Vec<_>>();
        let values = candidate.values();
        let distinct = values.iter().enumerate().all(|(index, (_, value))| {
            !values[..index].iter().any(|(_, other)| other == value)
                && !(unique_sizes && fixed_sizes.contains(&u16::from(*value)))
        });
        if !distinct {
            return false;
        }
    }
    // Unique only forbids matching a lower-lettered swing type, as in C++.
    player
        .dice
        .iter()
        .filter(|die| !die.in_reserve && die.has_property(property::UNIQUE))
        .all(|die| {
            let Some(unique_swing) = die.swing_type[0] else {
                return true;
            };
            let unique_value = candidate
                .values()
                .iter()
                .find_map(|(swing, value)| (*swing == unique_swing).then_some(*value));
            let Some(unique_value) = unique_value else {
                return true;
            };
            !candidate.values().iter().any(|(swing, value)| {
                *swing < unique_swing
                    && *value == unique_value
                    && player
                        .dice
                        .iter()
                        .any(|other| !other.in_reserve && other.swing_type.contains(&Some(*swing)))
            })
        })
}

fn push_swing_choice(candidate: &mut SwingMove, action: &SwingAction, value: u8) {
    if let Some(swing) = action.swing {
        candidate.push_value((swing, value));
    } else {
        candidate.push_option((action.die, value != 0));
    }
}

pub(crate) fn generate_swing_moves(player: &crate::game::Player) -> Vec<SwingMove> {
    let mut moves = vec![SwingMove::empty()];
    for action in swing_actions(player) {
        let mut next = Vec::new();
        for base in &moves {
            for value in &action.values {
                let mut m = *base;
                push_swing_choice(&mut m, &action, *value);
                next.push(m);
            }
        }
        moves = next;
    }
    moves.retain(|candidate| swing_move_allowed(player, candidate));
    moves
}

/// `generate_swing_moves(player).first()` without building every setting,
/// which playouts would otherwise do once per simulated round.
pub(crate) fn first_swing_move(player: &crate::game::Player) -> Option<SwingMove> {
    let actions = swing_actions(player);
    let mut choice = vec![0usize; actions.len()];
    loop {
        let mut candidate = SwingMove::empty();
        for (action, value) in actions.iter().zip(&choice) {
            push_swing_choice(&mut candidate, action, action.values[*value]);
        }
        if swing_move_allowed(player, &candidate) {
            return Some(candidate);
        }
        // The last action varies fastest, matching generate_swing_moves.
        let mut position = actions.len();
        loop {
            if position == 0 {
                return None;
            }
            position -= 1;
            choice[position] += 1;
            if choice[position] < actions[position].values.len() {
                break;
            }
            choice[position] = 0;
        }
    }
}

pub(super) fn apply_swing_move(player: &mut crate::game::Player, action: &SwingMove) {
    for die in &mut player.dice {
        if die.in_reserve {
            continue;
        }
        for side in 0..2 {
            if let Some(s) = die.swing_type[side]
                && let Some((_, v)) = action.values().iter().find(|(kind, _)| *kind == s)
            {
                assert!(die.not_set, "Die::OnSwingSet requires NOTSET state");
                die.sides[side] = *v;
            }
        }
    }
    for (index, second) in action.options() {
        if *second {
            player.dice[*index].sides.swap(0, 1);
        }
    }
}
