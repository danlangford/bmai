// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

pub(super) fn has_available_property(player: &crate::game::Player, property: u64) -> bool {
    player
        .dice
        .iter()
        .any(|die| die.is_available() && die.has_property(property))
}

pub(super) fn generate_chance_moves(game: &Game, player: usize) -> Vec<ChanceMove> {
    let dice = game.players[player]
        .dice
        .iter()
        .enumerate()
        .filter_map(|(index, die)| {
            (die.is_available() && die.has_property(property::CHANCE)).then_some(index)
        })
        .collect::<Vec<_>>();
    let mut moves = Vec::with_capacity(1usize << dice.len());
    moves.push(ChanceMove { reroll: Vec::new() });
    for mask in 1usize..(1usize << dice.len()) {
        moves.push(ChanceMove {
            reroll: dice
                .iter()
                .enumerate()
                .filter_map(|(bit, index)| (mask & (1 << bit) != 0).then_some(*index))
                .collect(),
        });
    }
    moves
}

pub(super) fn apply_chance_move(
    game: &mut Game,
    player: usize,
    previous_initiative: usize,
    action: &ChanceMove,
    rng: &mut Rng,
) -> (usize, bool) {
    if action.reroll.is_empty() {
        return (previous_initiative, false);
    }
    for index in &action.reroll {
        if !game.players[player].dice[*index].has_property(property::KONSTANT) {
            game.players[player].dice[*index].notset = true;
        }
        apply_before_roll_effects(game, player, *index);
        if game.players[player].dice[*index].notset {
            roll_scheduled_die(game, player, *index, rng);
        }
    }
    game.players[player].optimize_dice();
    // C++ tests `initiative != 0`, so success means player 0 won, whoever rolled.
    if check_initiative(game) == Some(0) {
        (player, true)
    } else {
        (previous_initiative, false)
    }
}

pub(super) fn select_chance_action(
    game: &Game,
    player: usize,
    rng: &mut Rng,
    ai: &Bmai3,
    level: usize,
    initiative: usize,
    native: Option<NativeEvaluation>,
) -> (ChanceMove, f32) {
    let mut moves = generate_chance_moves(game, player);
    let sims = ai.ComputeNumberSims(moves.len(), level);
    let mut scores = vec![0.0f32; moves.len()];
    let mut candidate_indices = native.map(|_| (0..moves.len()).collect::<Vec<_>>());
    let mut best_score = -1.0f32;
    let mut best = moves[0].clone();
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
                .cloned()
                .enumerate()
                .flat_map(|(index, action)| {
                    let candidate_index = candidate_indices.as_ref().unwrap()[index];
                    (0..batch).map(move |simulation_index| {
                        (action.clone(), candidate_index, simulation_index)
                    })
                })
                .collect();
            crate::native::ordered_parallel_map(
                tasks,
                context.workers,
                |(action, candidate_index, simulation_index)| {
                    let mut simulation = game.clone();
                    let mut simulation_rng = native_simulation_rng(
                        context.algorithm,
                        context.replay,
                        candidate_index,
                        batch_index,
                        sims_run + simulation_index,
                    );
                    evaluate_chance_simulation(
                        &mut simulation,
                        player,
                        initiative,
                        &action,
                        &mut simulation_rng,
                        ai,
                        level,
                    )
                },
            )
        });
        for (index, action) in moves.iter().enumerate() {
            if let Some(results) = &native_results {
                scores[index] += results[index * batch..(index + 1) * batch]
                    .iter()
                    .sum::<f32>();
            } else {
                for _ in 0..batch {
                    restore_simulation(&mut simulation, game);
                    scores[index] += evaluate_chance_simulation(
                        &mut simulation,
                        player,
                        initiative,
                        action,
                        rng,
                        ai,
                        level,
                    );
                }
            }
            if scores[index] > best_score {
                best_score = scores[index];
                best = action.clone();
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
    if trace_settings().chance {
        eprintln!(
            "CHANCE_BEST l{level} seed={} score={best_score} sims={sims_run} {:?}",
            rng.debug_seed(),
            best.reroll
        );
    }
    (best, best_score / sims_run as f32)
}

pub(super) fn evaluate_chance_simulation(
    simulation: &mut Game,
    player: usize,
    initiative: usize,
    action: &ChanceMove,
    rng: &mut Rng,
    ai: &Bmai3,
    level: usize,
) -> f32 {
    let (next_initiative, chance_continues) =
        apply_chance_move(simulation, player, initiative, action, rng);
    if level >= ai.max_ply {
        play_fight_qai_from_phase(simulation, rng, next_initiative, player, false, ai)
    } else {
        evaluate_next_initiative_action(
            simulation,
            rng,
            ai,
            level + 1,
            next_initiative,
            player,
            if chance_continues {
                InitiativeStage::Chance
            } else {
                InitiativeStage::Focus
            },
        )
    }
}

pub(crate) fn select_bmai_chance_action(game: &Game, rng: &mut Rng, ai: &Bmai3) -> ChanceMove {
    select_chance_action(game, 0, rng, ai, 1, 1, None).0
}

pub(crate) fn select_native_bmai_chance_action(
    game: &Game,
    rng_algorithm: crate::RngAlgorithm,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    ai: &Bmai3,
) -> ChanceMove {
    let mut unused_legacy_rng = Rng::untraced_default();
    select_chance_action(
        game,
        0,
        &mut unused_legacy_rng,
        ai,
        1,
        1,
        Some(NativeEvaluation {
            algorithm: rng_algorithm,
            replay,
            workers,
        }),
    )
    .0
}

pub(super) fn generate_focus_moves(game: &Game, player: usize) -> Vec<FocusMove> {
    let focus = game.players[player]
        .dice
        .iter()
        .enumerate()
        .filter(|(_, die)| {
            die.is_available()
                && die.has_property(property::FOCUS)
                && !die.has_property(property::RAGE)
                && die.value_total() > 1
        })
        .map(|(index, die)| (index, die.value_total() as u8))
        .collect::<Vec<_>>();
    let combinations = focus
        .iter()
        .fold(1usize, |total, (_, value)| total * usize::from(*value));
    let mut moves = vec![FocusMove { values: Vec::new() }];
    let mut trial = game.clone();
    for combination in 0..combinations.saturating_sub(1) {
        let mut divisor = 1usize;
        let mut values = Vec::new();
        for (index, current) in &focus {
            let value = ((combination / divisor) % usize::from(*current)) as u8 + 1;
            if value < *current {
                values.push((*index, value));
            }
            divisor *= usize::from(*current);
        }
        restore_simulation(&mut trial, game);
        apply_focus_move(
            &mut trial,
            player,
            &FocusMove {
                values: values.clone(),
            },
        );
        if check_initiative(&trial) == Some(player) {
            moves.push(FocusMove { values });
        }
    }
    moves
}

pub(super) fn apply_focus_move(game: &mut Game, player: usize, action: &FocusMove) {
    for (index, value) in &action.values {
        let die = &mut game.players[player].dice[*index];
        die.value = Some(*value);
        die.dizzy = true;
    }
    game.players[player].optimize_dice();
}

pub(super) fn select_focus_action(
    game: &Game,
    player: usize,
    rng: &mut Rng,
    ai: &Bmai3,
    level: usize,
    initiative: usize,
    native: Option<NativeEvaluation>,
) -> (FocusMove, f32) {
    let trace = trace_settings().focus;
    let mut moves = generate_focus_moves(game, player);
    let sims = ai.ComputeNumberSims(moves.len(), level);
    if trace {
        eprintln!(
            "FOCUS_BEGIN l{level} seed={} moves={} sims={sims}",
            rng.debug_seed(),
            moves.len()
        );
    }
    let mut scores = vec![0.0f32; moves.len()];
    let mut candidate_indices = native.map(|_| (0..moves.len()).collect::<Vec<_>>());
    let mut best_score = -1.0f32;
    let mut best = moves[0].clone();
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
                .cloned()
                .enumerate()
                .flat_map(|(index, action)| {
                    let candidate_index = candidate_indices.as_ref().unwrap()[index];
                    (0..batch).map(move |simulation_index| {
                        (action.clone(), candidate_index, simulation_index)
                    })
                })
                .collect();
            crate::native::ordered_parallel_map(
                tasks,
                context.workers,
                |(action, candidate_index, simulation_index)| {
                    let mut simulation = game.clone();
                    let mut simulation_rng = native_simulation_rng(
                        context.algorithm,
                        context.replay,
                        candidate_index,
                        batch_index,
                        sims_run + simulation_index,
                    );
                    evaluate_focus_simulation(
                        &mut simulation,
                        player,
                        initiative,
                        &action,
                        &mut simulation_rng,
                        ai,
                        level,
                    )
                },
            )
        });
        for (index, action) in moves.iter().enumerate() {
            if let Some(results) = &native_results {
                scores[index] += results[index * batch..(index + 1) * batch]
                    .iter()
                    .sum::<f32>();
            } else {
                for _ in 0..batch {
                    restore_simulation(&mut simulation, game);
                    scores[index] += evaluate_focus_simulation(
                        &mut simulation,
                        player,
                        initiative,
                        action,
                        rng,
                        ai,
                        level,
                    );
                }
            }
            if trace {
                eprintln!(
                    "FOCUS_MOVE l{level} m{index} seed={} sims={} score={} {:?}",
                    rng.debug_seed(),
                    sims_run + batch,
                    scores[index],
                    action.values
                );
            }
            if scores[index] > best_score {
                best_score = scores[index];
                best = action.clone();
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
    if trace {
        eprintln!(
            "FOCUS_BEST l{level} seed={} score={best_score} sims={sims_run} {:?}",
            rng.debug_seed(),
            best.values
        );
    }
    (best, best_score / sims_run as f32)
}

pub(super) fn evaluate_focus_simulation(
    simulation: &mut Game,
    player: usize,
    initiative: usize,
    action: &FocusMove,
    rng: &mut Rng,
    ai: &Bmai3,
    level: usize,
) -> f32 {
    let phase = if action.values.is_empty() {
        initiative
    } else {
        apply_focus_move(simulation, player, action);
        player
    };
    if level >= ai.max_ply {
        play_fight_qai_from_phase(simulation, rng, phase, player, false, ai)
    } else {
        evaluate_next_initiative_action(
            simulation,
            rng,
            ai,
            level + 1,
            phase,
            player,
            if action.values.is_empty() {
                InitiativeStage::Fight
            } else {
                InitiativeStage::Focus
            },
        )
    }
}

pub(crate) fn select_bmai_focus_action(game: &Game, rng: &mut Rng, ai: &Bmai3) -> FocusMove {
    select_focus_action(game, 0, rng, ai, 1, 1, None).0
}

pub(crate) fn select_native_bmai_focus_action(
    game: &Game,
    rng_algorithm: crate::RngAlgorithm,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    ai: &Bmai3,
) -> FocusMove {
    let mut unused_legacy_rng = Rng::untraced_default();
    select_focus_action(
        game,
        0,
        &mut unused_legacy_rng,
        ai,
        1,
        1,
        Some(NativeEvaluation {
            algorithm: rng_algorithm,
            replay,
            workers,
        }),
    )
    .0
}

pub(super) fn evaluate_next_initiative_action(
    game: &mut Game,
    rng: &mut Rng,
    ai: &Bmai3,
    level: usize,
    initiative: usize,
    pov: usize,
    mut stage: InitiativeStage,
) -> f32 {
    if matches!(stage, InitiativeStage::Chance) {
        let player = 1 - initiative;
        if has_available_property(&game.players[player], property::CHANCE) {
            let (_, probability) =
                select_chance_action(game, player, rng, ai, level, initiative, None);
            return if player == pov {
                probability
            } else {
                1.0 - probability
            };
        }
        stage = InitiativeStage::Focus;
    }
    if matches!(stage, InitiativeStage::Focus) {
        let player = 1 - initiative;
        if has_available_property(&game.players[player], property::FOCUS) {
            let (_, probability) =
                select_focus_action(game, player, rng, ai, level, initiative, None);
            return if player == pov {
                probability
            } else {
                1.0 - probability
            };
        }
    }
    let mut oriented = game.clone();
    if initiative == 1 {
        oriented.players.swap(0, 1);
    }
    let (_, probability) = select_bmai_action_at_level(&oriented, rng, ai, level, false);
    if initiative == pov {
        probability
    } else {
        1.0 - probability
    }
}

// As in C++, BMAI plays until max ply and QAI finishes the round.
pub(super) fn play_simulated_round(
    game: &mut Game,
    rng: &mut Rng,
    ai: &Bmai3,
    mut level: usize,
    pov: usize,
) -> f32 {
    roll_round_dice(game, rng);
    let mut phase = initiative_winner(game);
    let mut passed = false;
    let mut use_qai = level > ai.max_ply;
    let mut oriented = game.clone();
    loop {
        let player = 1 - phase;
        if !has_available_property(&game.players[player], property::CHANCE) || use_qai {
            break;
        }
        let action = select_chance_action(game, player, rng, ai, level, phase, None).0;
        if level >= ai.max_ply {
            use_qai = true;
        } else {
            level += 1;
        }
        let (next_phase, continues) = apply_chance_move(game, player, phase, &action, rng);
        phase = next_phase;
        if !continues {
            break;
        }
    }
    loop {
        let player = 1 - phase;
        if !has_available_property(&game.players[player], property::FOCUS) || use_qai {
            break;
        }
        let action = select_focus_action(game, player, rng, ai, level, phase, None).0;
        if level >= ai.max_ply {
            use_qai = true;
        } else {
            level += 1;
        }
        if action.values.is_empty() {
            break;
        }
        apply_focus_move(game, player, &action);
        phase = player;
    }
    for _ in 0..256 {
        if fight_over(game) {
            break;
        }
        restore_simulation(&mut oriented, game);
        if phase == 1 {
            oriented.players.swap(0, 1);
        }
        let action = if use_qai {
            select_rollout_action(&oriented, rng, ai)
        } else {
            let action = select_bmai_action_at_level(&oriented, rng, ai, level, passed).0;
            if level >= ai.max_ply {
                use_qai = true;
            } else {
                level += 1;
            }
            action
        };
        if action.action == Action::Surrender {
            game.players[phase].score = -1000.0;
            break;
        }
        if action.action != Action::Attack {
            if passed {
                break;
            }
            passed = true;
        } else {
            passed = false;
            let extra = apply_attack_for_players(game, &action, phase, 1 - phase, rng);
            recover_dizzy_dice(&mut game.players[phase]);
            if extra {
                continue;
            }
        }
        if action.action != Action::Attack {
            recover_dizzy_dice(&mut game.players[phase]);
        }
        phase = 1 - phase;
    }
    match game.players[pov]
        .score
        .total_cmp(&game.players[1 - pov].score)
    {
        std::cmp::Ordering::Greater => 1.0,
        std::cmp::Ordering::Equal => 0.5,
        std::cmp::Ordering::Less => 0.0,
    }
}

pub(super) fn play_round_qai(game: &mut Game, rng: &mut Rng, pov: usize, ai: &Bmai3) -> f32 {
    roll_round_dice(game, rng);
    let phase = initiative_winner(game);
    play_fight_qai_from_phase(game, rng, phase, pov, false, ai)
}

pub(super) fn play_fight_qai_from_phase(
    game: &mut Game,
    rng: &mut Rng,
    mut phase: usize,
    pov: usize,
    mut passed: bool,
    ai: &Bmai3,
) -> f32 {
    let mut oriented = game.clone();
    for _ in 0..256 {
        if game.players.iter().any(|p| available_dice(p) == 0) {
            break;
        }
        restore_simulation(&mut oriented, game);
        if phase == 1 {
            oriented.players.swap(0, 1);
        }
        let action = select_rollout_action(&oriented, rng, ai);
        if action.action != Action::Attack {
            if passed {
                break;
            }
            passed = true;
        } else {
            passed = false;
            let extra = apply_attack_for_players(game, &action, phase, 1 - phase, rng);
            recover_dizzy_dice(&mut game.players[phase]);
            if extra {
                continue;
            }
        }
        if action.action != Action::Attack {
            recover_dizzy_dice(&mut game.players[phase]);
        }
        phase = 1 - phase;
    }
    match game.players[pov]
        .score
        .total_cmp(&game.players[1 - pov].score)
    {
        std::cmp::Ordering::Greater => 1.0,
        std::cmp::Ordering::Equal => 0.5,
        std::cmp::Ordering::Less => 0.0,
    }
}
