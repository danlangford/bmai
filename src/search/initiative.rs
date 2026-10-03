// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

pub(super) fn HasAvailableProperty(player: &crate::game::Player, property: u64) -> bool {
    player
        .m_die
        .iter()
        .any(|die| die.IsAvailable() && die.HasProperty(property))
}

pub(super) fn GenerateChanceMoves(game: &Game, player: usize) -> Vec<ChanceMove> {
    let dice = game.m_player[player]
        .m_die
        .iter()
        .enumerate()
        .filter_map(|(index, die)| {
            (die.IsAvailable() && die.HasProperty(property::CHANCE)).then_some(index)
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

pub(super) fn ApplyChanceMove(
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
        if !game.m_player[player].m_die[*index].HasProperty(property::KONSTANT) {
            game.m_player[player].m_die[*index].m_notset = true;
        }
        ApplyBeforeRollEffects(game, player, *index);
        if game.m_player[player].m_die[*index].m_notset {
            RollScheduledDie(game, player, *index, rng);
        }
    }
    game.m_player[player].OptimizeDice();
    // C++ tests `initiative != 0`, so success means player 0 won, whoever rolled.
    if CheckInitiative(game) == Some(0) {
        (player, true)
    } else {
        (previous_initiative, false)
    }
}

pub(super) fn SelectChanceAction(
    game: &Game,
    player: usize,
    rng: &mut Rng,
    ai: &Bmai3,
    level: usize,
    initiative: usize,
    native: Option<NativeEvaluation>,
) -> (ChanceMove, f32) {
    let mut moves = GenerateChanceMoves(game, player);
    let sims = ai.ComputeNumberSims(moves.len(), level);
    let mut scores = vec![0.0f32; moves.len()];
    let mut candidate_indices = native.map(|_| (0..moves.len()).collect::<Vec<_>>());
    let mut best_score = -1.0f32;
    let mut best = moves[0].clone();
    let mut sims_run = 0usize;
    let mut simulation = game.clone();
    while sims_run < sims {
        let batch = if ai.m_cull_moves {
            ai.m_sims_per_check.min(sims - sims_run)
        } else {
            sims - sims_run
        };
        let native_results = native.map(|context| {
            let batch_index = if ai.m_cull_moves {
                sims_run / ai.m_sims_per_check.max(1)
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
                    let mut simulation_rng = NativeSimulationRng(
                        context.algorithm,
                        context.replay,
                        candidate_index,
                        batch_index,
                        sims_run + simulation_index,
                    );
                    EvaluateChanceSimulation(
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
                    RestoreSimulation(&mut simulation, game);
                    scores[index] += EvaluateChanceSimulation(
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
        if sims_run >= sims || !ai.m_cull_moves {
            break;
        }
        if moves.len() == 1 {
            if CompletesNativeProbabilitySample(native) {
                continue;
            }
            break;
        }
        let progress = sims_run as f32 / sims as f32;
        let threshold = ai.m_min_best_score_threshold
            + progress * (ai.m_max_best_score_threshold - ai.m_min_best_score_threshold);
        let mut delta_threshold = (1.0 - progress) * ai.m_sims_per_check as f32 * 0.5;
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
        if moves.len() == 1 && !CompletesNativeProbabilitySample(native) {
            break;
        }
    }
    if TraceSettings().chance {
        eprintln!(
            "CHANCE_BEST l{level} seed={} score={best_score} sims={sims_run} {:?}",
            rng.DebugSeed(),
            best.reroll
        );
    }
    (best, best_score / sims_run as f32)
}

pub(super) fn EvaluateChanceSimulation(
    simulation: &mut Game,
    player: usize,
    initiative: usize,
    action: &ChanceMove,
    rng: &mut Rng,
    ai: &Bmai3,
    level: usize,
) -> f32 {
    let (next_initiative, chance_continues) =
        ApplyChanceMove(simulation, player, initiative, action, rng);
    if level >= ai.m_max_ply {
        PlayFightQAIFromPhase(simulation, rng, next_initiative, player, false, ai)
    } else {
        EvaluateNextInitiativeAction(
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

pub(crate) fn SelectBMAIChanceAction(game: &Game, rng: &mut Rng, ai: &Bmai3) -> ChanceMove {
    SelectChanceAction(game, 0, rng, ai, 1, 1, None).0
}

pub(crate) fn SelectNativeBMAIChanceAction(
    game: &Game,
    rng_algorithm: crate::RngAlgorithm,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    ai: &Bmai3,
) -> ChanceMove {
    let mut unused_legacy_rng = Rng::UntracedDefault();
    SelectChanceAction(
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

pub(super) fn GenerateFocusMoves(game: &Game, player: usize) -> Vec<FocusMove> {
    let focus = game.m_player[player]
        .m_die
        .iter()
        .enumerate()
        .filter(|(_, die)| {
            die.IsAvailable()
                && die.HasProperty(property::FOCUS)
                && !die.HasProperty(property::RAGE)
                && die.GetValueTotal() > 1
        })
        .map(|(index, die)| (index, die.GetValueTotal() as u8))
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
        RestoreSimulation(&mut trial, game);
        ApplyFocusMove(
            &mut trial,
            player,
            &FocusMove {
                values: values.clone(),
            },
        );
        if CheckInitiative(&trial) == Some(player) {
            moves.push(FocusMove { values });
        }
    }
    moves
}

pub(super) fn ApplyFocusMove(game: &mut Game, player: usize, action: &FocusMove) {
    for (index, value) in &action.values {
        let die = &mut game.m_player[player].m_die[*index];
        die.m_value_total = Some(*value);
        die.m_dizzy = true;
    }
    game.m_player[player].OptimizeDice();
}

pub(super) fn SelectFocusAction(
    game: &Game,
    player: usize,
    rng: &mut Rng,
    ai: &Bmai3,
    level: usize,
    initiative: usize,
    native: Option<NativeEvaluation>,
) -> (FocusMove, f32) {
    let trace = TraceSettings().focus;
    let mut moves = GenerateFocusMoves(game, player);
    let sims = ai.ComputeNumberSims(moves.len(), level);
    if trace {
        eprintln!(
            "FOCUS_BEGIN l{level} seed={} moves={} sims={sims}",
            rng.DebugSeed(),
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
        let batch = if ai.m_cull_moves {
            ai.m_sims_per_check.min(sims - sims_run)
        } else {
            sims - sims_run
        };
        let native_results = native.map(|context| {
            let batch_index = if ai.m_cull_moves {
                sims_run / ai.m_sims_per_check.max(1)
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
                    let mut simulation_rng = NativeSimulationRng(
                        context.algorithm,
                        context.replay,
                        candidate_index,
                        batch_index,
                        sims_run + simulation_index,
                    );
                    EvaluateFocusSimulation(
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
                    RestoreSimulation(&mut simulation, game);
                    scores[index] += EvaluateFocusSimulation(
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
                    rng.DebugSeed(),
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
        if sims_run >= sims || !ai.m_cull_moves {
            break;
        }
        if moves.len() == 1 {
            if CompletesNativeProbabilitySample(native) {
                continue;
            }
            break;
        }
        let progress = sims_run as f32 / sims as f32;
        let threshold = ai.m_min_best_score_threshold
            + progress * (ai.m_max_best_score_threshold - ai.m_min_best_score_threshold);
        let mut delta_threshold = (1.0 - progress) * ai.m_sims_per_check as f32 * 0.5;
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
        if moves.len() == 1 && !CompletesNativeProbabilitySample(native) {
            break;
        }
    }
    if trace {
        eprintln!(
            "FOCUS_BEST l{level} seed={} score={best_score} sims={sims_run} {:?}",
            rng.DebugSeed(),
            best.values
        );
    }
    (best, best_score / sims_run as f32)
}

pub(super) fn EvaluateFocusSimulation(
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
        ApplyFocusMove(simulation, player, action);
        player
    };
    if level >= ai.m_max_ply {
        PlayFightQAIFromPhase(simulation, rng, phase, player, false, ai)
    } else {
        EvaluateNextInitiativeAction(
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

pub(crate) fn SelectBMAIFocusAction(game: &Game, rng: &mut Rng, ai: &Bmai3) -> FocusMove {
    SelectFocusAction(game, 0, rng, ai, 1, 1, None).0
}

pub(crate) fn SelectNativeBMAIFocusAction(
    game: &Game,
    rng_algorithm: crate::RngAlgorithm,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    ai: &Bmai3,
) -> FocusMove {
    let mut unused_legacy_rng = Rng::UntracedDefault();
    SelectFocusAction(
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

pub(super) fn EvaluateNextInitiativeAction(
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
        if HasAvailableProperty(&game.m_player[player], property::CHANCE) {
            let (_, probability) =
                SelectChanceAction(game, player, rng, ai, level, initiative, None);
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
        if HasAvailableProperty(&game.m_player[player], property::FOCUS) {
            let (_, probability) =
                SelectFocusAction(game, player, rng, ai, level, initiative, None);
            return if player == pov {
                probability
            } else {
                1.0 - probability
            };
        }
    }
    let mut oriented = game.clone();
    if initiative == 1 {
        oriented.m_player.swap(0, 1);
    }
    let (_, probability) = SelectBMAIActionAtLevel(&oriented, rng, ai, level, false);
    if initiative == pov {
        probability
    } else {
        1.0 - probability
    }
}

// As in C++, BMAI plays until max ply and QAI finishes the round.
pub(super) fn PlaySimulatedRound(
    game: &mut Game,
    rng: &mut Rng,
    ai: &Bmai3,
    mut level: usize,
    pov: usize,
) -> f32 {
    RollRoundDice(game, rng);
    let mut phase = InitiativeWinner(game);
    let mut passed = false;
    let mut use_qai = level > ai.m_max_ply;
    let mut oriented = game.clone();
    loop {
        let player = 1 - phase;
        if !HasAvailableProperty(&game.m_player[player], property::CHANCE) || use_qai {
            break;
        }
        let action = SelectChanceAction(game, player, rng, ai, level, phase, None).0;
        if level >= ai.m_max_ply {
            use_qai = true;
        } else {
            level += 1;
        }
        let (next_phase, continues) = ApplyChanceMove(game, player, phase, &action, rng);
        phase = next_phase;
        if !continues {
            break;
        }
    }
    loop {
        let player = 1 - phase;
        if !HasAvailableProperty(&game.m_player[player], property::FOCUS) || use_qai {
            break;
        }
        let action = SelectFocusAction(game, player, rng, ai, level, phase, None).0;
        if level >= ai.m_max_ply {
            use_qai = true;
        } else {
            level += 1;
        }
        if action.values.is_empty() {
            break;
        }
        ApplyFocusMove(game, player, &action);
        phase = player;
    }
    for _ in 0..256 {
        if FightOver(game) {
            break;
        }
        RestoreSimulation(&mut oriented, game);
        if phase == 1 {
            oriented.m_player.swap(0, 1);
        }
        let action = if use_qai {
            SelectRolloutAction(&oriented, rng, ai)
        } else {
            let action = SelectBMAIActionAtLevel(&oriented, rng, ai, level, passed).0;
            if level >= ai.m_max_ply {
                use_qai = true;
            } else {
                level += 1;
            }
            action
        };
        if action.m_action == Action::Surrender {
            game.m_player[phase].m_score = -1000.0;
            break;
        }
        if action.m_action != Action::Attack {
            if passed {
                break;
            }
            passed = true;
        } else {
            passed = false;
            let extra = ApplyAttackForPlayers(game, &action, phase, 1 - phase, rng);
            RecoverDizzyDice(&mut game.m_player[phase]);
            if extra {
                continue;
            }
        }
        if action.m_action != Action::Attack {
            RecoverDizzyDice(&mut game.m_player[phase]);
        }
        phase = 1 - phase;
    }
    match game.m_player[pov]
        .m_score
        .total_cmp(&game.m_player[1 - pov].m_score)
    {
        std::cmp::Ordering::Greater => 1.0,
        std::cmp::Ordering::Equal => 0.5,
        std::cmp::Ordering::Less => 0.0,
    }
}

pub(super) fn PlayRoundQAI(game: &mut Game, rng: &mut Rng, pov: usize, ai: &Bmai3) -> f32 {
    RollRoundDice(game, rng);
    let phase = InitiativeWinner(game);
    PlayFightQAIFromPhase(game, rng, phase, pov, false, ai)
}

pub(super) fn PlayFightQAIFromPhase(
    game: &mut Game,
    rng: &mut Rng,
    mut phase: usize,
    pov: usize,
    mut passed: bool,
    ai: &Bmai3,
) -> f32 {
    let mut oriented = game.clone();
    for _ in 0..256 {
        if game.m_player.iter().any(|p| AvailableDice(p) == 0) {
            break;
        }
        RestoreSimulation(&mut oriented, game);
        if phase == 1 {
            oriented.m_player.swap(0, 1);
        }
        let action = SelectRolloutAction(&oriented, rng, ai);
        if action.m_action != Action::Attack {
            if passed {
                break;
            }
            passed = true;
        } else {
            passed = false;
            let extra = ApplyAttackForPlayers(game, &action, phase, 1 - phase, rng);
            RecoverDizzyDice(&mut game.m_player[phase]);
            if extra {
                continue;
            }
        }
        if action.m_action != Action::Attack {
            RecoverDizzyDice(&mut game.m_player[phase]);
        }
        phase = 1 - phase;
    }
    match game.m_player[pov]
        .m_score
        .total_cmp(&game.m_player[1 - pov].m_score)
    {
        std::cmp::Ordering::Greater => 1.0,
        std::cmp::Ordering::Equal => 0.5,
        std::cmp::Ordering::Less => 0.0,
    }
}
