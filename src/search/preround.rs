// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

pub(super) fn SelectSwingAction(
    game: &BMC_Game,
    player: usize,
    rng: &mut BMC_RNG,
    ai: &BMC_BMAI3,
    level: usize,
    native: Option<NativeEvaluation>,
) -> (SwingMove, f32) {
    if game.m_player[player].m_swing_set != BME_SWING_SET::NOT {
        return (CurrentSwingMove(&game.m_player[player]), 0.0);
    }
    let traces = TraceSettings();
    let trace_list = level == 1 && traces.swing_list;
    let trace_candidate = traces.swing_candidate;
    let trace_sim = level == 2 && traces.swing_sim;
    let trace_moves = level == 1 && traces.swing_moves;
    let trace_best = traces.swing;
    let mut moves = GenerateSwingMoves(&game.m_player[player]);
    if moves.is_empty() {
        return (SwingMove::empty(), 0.0);
    }
    let max_moves = ai.m_max_branch / ai.m_min_sims;
    if moves.len() > max_moves {
        let mut selection_rng = native.map(|context| {
            NativeSimulationRng(
                context.algorithm,
                context.replay,
                NATIVE_ENUMERATION_STREAM,
                0,
                0,
            )
        });
        RandomlySelectSwingMoves(
            &mut moves,
            &game.m_player[player],
            max_moves,
            selection_rng.as_mut().unwrap_or(rng),
        );
        moves.shrink_to_fit();
    }
    if trace_list {
        for (index, action) in moves.iter().enumerate() {
            eprintln!(
                "SWING_LIST m{index} {:?} {:?}",
                action.values(),
                action.options()
            );
        }
    }
    let sims = ai.ComputeNumberSims(moves.len(), level);
    let mut scores = vec![0.0f32; moves.len()];
    let mut candidate_indices = native.map(|_| (0..moves.len()).collect::<Vec<_>>());
    let mut best_score = -1.0f32;
    let mut best = moves[0];
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
                    ApplySwingMove(&mut simulation.m_player[player], &candidate);
                    simulation.m_player[player].m_swing_set = BME_SWING_SET::LOCKED;
                    let mut simulation_rng = NativeSimulationRng(
                        context.algorithm,
                        context.replay,
                        candidate_index,
                        batch_index,
                        sims_run + simulation_index,
                    );
                    EvaluateSwingMove(&mut simulation, player, &mut simulation_rng, ai, level)
                },
            )
        });
        for (index, candidate) in moves.iter().enumerate() {
            if trace_candidate {
                eprintln!(
                    "SWING_CANDIDATE l{level} p{player} m{index} seed={} sims={} {:?}",
                    rng.DebugSeed(),
                    sims_run + batch,
                    candidate.values()
                );
            }
            if let Some(results) = &native_results {
                scores[index] += results[index * batch..(index + 1) * batch]
                    .iter()
                    .sum::<f32>();
            } else {
                for simulation_index in 0..batch {
                    if trace_sim && index == 0 {
                        eprintln!(
                            "SWING_SIM l{level} m{index} s{simulation_index} seed={}",
                            rng.DebugSeed()
                        );
                    }
                    RestoreSimulation(&mut simulation, game);
                    ApplySwingMove(&mut simulation.m_player[player], candidate);
                    simulation.m_player[player].m_swing_set = BME_SWING_SET::LOCKED;
                    scores[index] += EvaluateSwingMove(&mut simulation, player, rng, ai, level);
                }
            }
            if scores[index] > best_score {
                best_score = scores[index];
                best = *candidate;
            }
            if trace_moves {
                eprintln!(
                    "SWING_MOVE l{level} m{index} sims={} score={:.6} {:?} {:?}",
                    sims_run + batch,
                    scores[index],
                    candidate.values(),
                    candidate.options()
                );
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
    if trace_best {
        eprintln!(
            "SWING p{player} seed={} score={best_score} sims={sims_run} {:?} {:?}",
            rng.DebugSeed(),
            best.values(),
            best.options()
        );
    }
    let probability = best_score / sims_run as f32;
    (best, probability)
}

pub(crate) fn SelectBMAISetSwingAction(
    game: &BMC_Game,
    rng: &mut BMC_RNG,
    ai: &BMC_BMAI3,
) -> SwingMove {
    SelectSwingAction(game, 0, rng, ai, 1, None).0
}

pub(crate) fn SelectNativeBMAISetSwingAction(
    game: &BMC_Game,
    rng_algorithm: crate::BME_RNG_ALGORITHM,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    ai: &BMC_BMAI3,
) -> SwingMove {
    let mut unused_legacy_rng = BMC_RNG::UntracedDefault();
    SelectSwingAction(
        game,
        0,
        &mut unused_legacy_rng,
        ai,
        1,
        Some(NativeEvaluation {
            algorithm: rng_algorithm,
            replay,
            workers,
        }),
    )
    .0
}

pub(crate) fn SelectQAISetSwingAction(game: &BMC_Game) -> SwingMove {
    GenerateSwingMoves(&game.m_player[0])
        .into_iter()
        .next()
        .unwrap_or_else(SwingMove::empty)
}

pub(crate) fn SelectBMAIReserveAction(
    game: &BMC_Game,
    rng: &mut BMC_RNG,
    ai: &BMC_BMAI3,
) -> Option<usize> {
    let reserve_indices = game.m_player[0]
        .m_die
        .iter()
        .enumerate()
        .filter_map(|(index, die)| die.m_in_reserve.then_some(index))
        .collect::<Vec<_>>();
    let sims = ai.ComputeNumberSims(reserve_indices.len() + 1, 1);
    let mut best_score = -1.0f32;
    let mut best = None;
    let mut simulation = game.clone();

    for candidate in reserve_indices.into_iter().map(Some).chain([None]) {
        let mut score = 0.0f32;
        if TraceSettings().reserve {
            eprintln!(
                "RESERVE_BEGIN candidate={candidate:?} seed={} sims={sims}",
                rng.DebugSeed()
            );
        }
        for _ in 0..sims {
            RestoreSimulation(&mut simulation, game);
            if let Some(index) = candidate {
                ApplyUseReserve(&mut simulation.m_player[0].m_die[index]);
            }
            let fight_level = PlayPreround(&mut simulation, rng, ai, 2);
            score += PlaySimulatedRound(&mut simulation, rng, ai, fight_level, 0);
        }
        if TraceSettings().reserve {
            eprintln!(
                "RESERVE_END candidate={candidate:?} seed={} score={score:.1}",
                rng.DebugSeed()
            );
        }
        if score > best_score {
            best_score = score;
            best = candidate;
        }
    }
    best
}

pub(super) fn AuxiliaryDie(player: &crate::game::BMC_Player) -> Option<usize> {
    player
        .m_die
        .iter()
        .position(|die| die.HasProperty(property::AUXILIARY))
}

/// Gordo refuses a single swing die of its own V-Z types, which would force
/// two dice to share a size; ButtonWeavers lets a Twin swing die through.
fn AcceptableAuxiliaryDie(player: &crate::game::BMC_Player) -> Option<usize> {
    AuxiliaryDie(player).filter(|index| {
        let die = &player.m_die[*index];
        player.m_specials & special::UNIQUE_SIZES == 0
            || die.HasProperty(property::TWIN)
            || !die.m_swing_type[0].is_some_and(|swing| ('V'..='Z').contains(&swing))
    })
}

/// Resolve ButtonWeavers' mutual Auxiliary choice for a simulation. If both
/// players agree, each selected die becomes an ordinary die for the game. A
/// decline by either player removes every Auxiliary die from both buttons.
pub(crate) fn ApplyAuxiliaryDecision(game: &mut BMC_Game, accepted: bool) {
    let accepted = accepted
        && game.m_player.iter().all(|player| {
            AuxiliaryDie(player).is_none() || AcceptableAuxiliaryDie(player).is_some()
        });
    for player in &mut game.m_player {
        let selected = accepted.then(|| AuxiliaryDie(player)).flatten();
        let mut index = 0;
        player.m_die.retain_mut(|die| {
            let keep = !die.HasProperty(property::AUXILIARY) || Some(index) == selected;
            if keep && die.HasProperty(property::AUXILIARY) {
                die.m_properties &= !property::AUXILIARY;
                die.m_value_total = None;
                die.m_notset = true;
            }
            index += 1;
            keep
        });
    }
}

pub(super) fn EvaluateAuxiliaryDecision(game: &BMC_Game, accepted: bool, rng: &mut BMC_RNG) -> f32 {
    let mut simulation = game.clone();
    ApplyAuxiliaryDecision(&mut simulation, accepted);
    // Auxiliary is a pregame choice. Compare the resulting buttons over one
    // complete round with the inexpensive deterministic QAI rollout policy;
    // recursively invoking BMAI here would nest a new search at every move.
    let (winner, _) = PlayRoundWithPolicies(
        &mut simulation,
        rng,
        &[BMC_AI_POLICY::QAI, BMC_AI_POLICY::QAI],
        None,
    );
    match winner {
        Some(0) => 1.0,
        None => 0.5,
        Some(1) => 0.0,
        Some(_) => unreachable!("Button Men has exactly two players"),
    }
}

pub(crate) fn SelectBMAIAuxiliaryAction(
    game: &BMC_Game,
    rng: &mut BMC_RNG,
    ai: &BMC_BMAI3,
) -> AuxiliarySearchResult {
    let Some(auxiliary) = AcceptableAuxiliaryDie(&game.m_player[0]) else {
        return AuxiliarySearchResult {
            die: None,
            score: 0.0,
            simulations: 0,
        };
    };
    let simulations = ai.ComputeNumberSims(2, 1);
    let candidates = [Some(auxiliary), None];
    let mut best = AuxiliarySearchResult {
        die: None,
        score: -1.0,
        simulations,
    };
    for candidate in candidates {
        let score = (0..simulations)
            .map(|_| EvaluateAuxiliaryDecision(game, candidate.is_some(), rng))
            .sum();
        if score > best.score {
            best.die = candidate;
            best.score = score;
        }
    }
    best
}

pub(crate) fn SelectNativeBMAIAuxiliaryAction(
    game: &BMC_Game,
    rng_algorithm: crate::BME_RNG_ALGORITHM,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    ai: &BMC_BMAI3,
) -> AuxiliarySearchResult {
    let Some(auxiliary) = AcceptableAuxiliaryDie(&game.m_player[0]) else {
        return AuxiliarySearchResult {
            die: None,
            score: 0.0,
            simulations: 0,
        };
    };
    let simulations = ai.ComputeNumberSims(2, 1);
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
        workers,
        |(candidate_index, candidate, simulation_index)| {
            let mut simulation_rng =
                NativeSimulationRng(rng_algorithm, replay, candidate_index, 0, simulation_index);
            EvaluateAuxiliaryDecision(game, candidate.is_some(), &mut simulation_rng)
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

pub(crate) fn SelectQAIAuxiliaryAction(game: &BMC_Game) -> Option<usize> {
    AcceptableAuxiliaryDie(&game.m_player[0])
}

pub(crate) fn SelectNativeBMAIReserveAction(
    game: &BMC_Game,
    rng_algorithm: crate::BME_RNG_ALGORITHM,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    ai: &BMC_BMAI3,
) -> Option<usize> {
    let reserve_indices = game.m_player[0]
        .m_die
        .iter()
        .enumerate()
        .filter_map(|(index, die)| die.m_in_reserve.then_some(index))
        .collect::<Vec<_>>();
    let sims = ai.ComputeNumberSims(reserve_indices.len() + 1, 1);
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
        workers,
        |(candidate_index, candidate, simulation_index)| {
            let mut simulation = game.clone();
            if let Some(index) = candidate {
                ApplyUseReserve(&mut simulation.m_player[0].m_die[index]);
            }
            let mut simulation_rng =
                NativeSimulationRng(rng_algorithm, replay, candidate_index, 0, simulation_index);
            let fight_level = PlayPreround(&mut simulation, &mut simulation_rng, ai, 2);
            PlaySimulatedRound(&mut simulation, &mut simulation_rng, ai, fight_level, 0)
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

pub(crate) fn SelectQAIReserveAction(game: &BMC_Game) -> Option<usize> {
    game.m_player[0]
        .m_die
        .iter()
        .position(|die| die.m_in_reserve)
}

pub(super) fn ApplyUseReserve(die: &mut BMC_Die) {
    die.m_in_reserve = false;
    die.m_properties &= !property::RESERVE;
    die.m_value_total = None;
    die.m_notset = true;
}

pub(super) fn RandomlySelectSwingMoves(
    moves: &mut Vec<SwingMove>,
    player: &crate::game::BMC_Player,
    max: usize,
    rng: &mut BMC_RNG,
) {
    let swing_types = player
        .m_die
        .iter()
        .filter(|die| !die.m_in_reserve)
        .flat_map(|die| die.m_swing_type.iter().flatten().copied())
        .collect::<std::collections::BTreeSet<_>>();
    let extreme_settings = |action: &SwingMove| {
        action
            .values()
            .iter()
            .filter(|(swing, value)| {
                let (minimum, maximum) = SwingRange(*swing);
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
        let mut index = 0;
        while index < moves.len() {
            if extreme_settings(&moves[index]) == swing_dice {
                index += 1;
            } else {
                // BMC_MoveList::Remove fills the hole with the final move.
                moves.swap_remove(index);
            }
        }
        return;
    }

    while moves.len() > max {
        let index = rng.GetRandMax(moves.len() as u32) as usize;
        let percentage_extreme = extreme_settings(&moves[index]) as f32 / swing_dice as f32;
        if rng.GetFRand() >= percentage_extreme {
            moves.swap_remove(index);
        }
    }
}

pub(super) fn EvaluateSwingMove(
    game: &mut BMC_Game,
    player: usize,
    rng: &mut BMC_RNG,
    ai: &BMC_BMAI3,
    level: usize,
) -> f32 {
    let other = 1 - player;

    // At the terminal ply OnPreSimulation replaces both simulation AIs with
    // QAI. QAI's preround policy is the first valid (minimum swing / first
    // option) setting, after which it plays the fight out.
    if level >= ai.m_max_ply {
        if game.m_player[other].m_swing_set == BME_SWING_SET::NOT {
            if NeedsSetSwing(&game.m_player[other]) {
                let selected = GenerateSwingMoves(&game.m_player[other])
                    .into_iter()
                    .next()
                    .expect("a player needing a swing has a valid setting");
                ApplySwingMove(&mut game.m_player[other], &selected);
            }
            game.m_player[other].m_swing_set = BME_SWING_SET::LOCKED;
        }
        return PlayRoundQAI(game, rng, player, ai);
    }

    // Before the terminal ply, PlayRound_EvaluateMove stops at the opponent's
    // next BMAI decision and uses that decision's estimated probability.
    if game.m_player[other].m_swing_set == BME_SWING_SET::NOT {
        if !NeedsSetSwing(&game.m_player[other]) {
            game.m_player[other].m_swing_set = BME_SWING_SET::LOCKED;
            return PlayRoundToNextBMAIAction(game, rng, player, ai, level + 1);
        }
        if game.m_player[player].m_swing_set == BME_SWING_SET::READY {
            game.m_player[player].m_swing_set = BME_SWING_SET::NOT;
        }
        let (_, other_probability) = SelectSwingAction(game, other, rng, ai, level + 1, None);
        return 1.0 - other_probability;
    }

    PlayRoundToNextBMAIAction(game, rng, player, ai, level + 1)
}

pub(super) fn PlayRoundToNextBMAIAction(
    game: &mut BMC_Game,
    rng: &mut BMC_RNG,
    pov: usize,
    ai: &BMC_BMAI3,
    level: usize,
) -> f32 {
    RollRoundDice(game, rng);
    let phase = InitiativeWinner(game);
    EvaluateNextInitiativeAction(game, rng, ai, level, phase, pov, InitiativeStage::Chance)
}

pub(super) fn NeedsSetSwing(player: &crate::game::BMC_Player) -> bool {
    player
        .m_die
        .iter()
        .filter(|die| !die.m_in_reserve)
        .any(|d| d.m_swing_type.iter().any(Option::is_some) || d.HasProperty(property::OPTION))
}

pub(super) fn CurrentSwingMove(player: &crate::game::BMC_Player) -> SwingMove {
    let mut action = SwingMove::empty();
    let mut seen = [false; 26];
    for (index, die) in player.m_die.iter().enumerate() {
        if die.m_in_reserve {
            continue;
        }
        for side in 0..2 {
            if let Some(swing @ 'A'..='Z') = die.m_swing_type[side] {
                let seen_index = (swing as u8 - b'A') as usize;
                if !seen[seen_index] {
                    action.push_value((swing, die.m_sides[side]));
                    seen[seen_index] = true;
                }
            }
        }
        if die.HasProperty(property::OPTION) {
            // ParseDie keeps the selected option in slot zero.
            action.push_option((index, false));
        }
    }
    action
}

pub(super) fn GenerateSwingMoves(player: &crate::game::BMC_Player) -> Vec<SwingMove> {
    let mut actions = Vec::<(Option<char>, usize, Vec<u8>)>::new();
    let mut swings = player
        .m_die
        .iter()
        .filter(|die| !die.m_in_reserve)
        .flat_map(|d| d.m_swing_type.iter().flatten().copied())
        .collect::<Vec<_>>();
    swings.sort_unstable();
    swings.dedup();
    for swing in swings {
        let (min, max) = SwingRange(swing);
        actions.push((Some(swing), 0, (min..=max).collect()));
    }
    for (index, die) in player.m_die.iter().enumerate() {
        if !die.m_in_reserve && die.HasProperty(property::OPTION) {
            actions.push((None, index, vec![0, 1]));
        }
    }
    let mut moves = vec![SwingMove::empty()];
    for (swing, index, values) in actions {
        let mut next = Vec::new();
        for base in &moves {
            for value in &values {
                let mut m = *base;
                if let Some(s) = swing {
                    m.push_value((s, *value));
                } else {
                    m.push_option((index, *value != 0));
                }
                next.push(m);
            }
        }
        moves = next;
    }
    let unique_sizes = player.m_specials & special::UNIQUE_SIZES != 0;
    if unique_sizes || player.m_specials & special::UNIQUE_SWING != 0 {
        moves.retain(|candidate| {
            // Option dice count at the side this candidate chooses.
            let fixed_sizes = player
                .m_die
                .iter()
                .enumerate()
                .filter(|(_, die)| {
                    !die.m_in_reserve && die.m_swing_type.iter().all(Option::is_none)
                })
                .map(|(index, die)| {
                    let second = candidate
                        .options()
                        .iter()
                        .any(|(option, second)| *option == index && *second);
                    if die.HasProperty(property::OPTION) {
                        u16::from(die.m_sides[usize::from(second)])
                    } else {
                        die.GetSidesMax()
                    }
                })
                .collect::<Vec<_>>();
            let values = candidate.values();
            values.iter().enumerate().all(|(index, (_, value))| {
                !values[..index].iter().any(|(_, other)| other == value)
                    && !(unique_sizes && fixed_sizes.contains(&u16::from(*value)))
            })
        });
    }
    // BMC_Game::ValidSetSwing enforces UNIQUE after enumerating each complete
    // swing/option setting. A Unique swing die may not use the same value as
    // any lower-numbered swing type present on the same button.
    moves.retain(|candidate| {
        player
            .m_die
            .iter()
            .filter(|die| !die.m_in_reserve && die.HasProperty(property::UNIQUE))
            .all(|die| {
                let Some(unique_swing) = die.m_swing_type[0] else {
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
                        && player.m_die.iter().any(|other| {
                            !other.m_in_reserve && other.m_swing_type.contains(&Some(*swing))
                        })
                })
            })
    });
    moves
}

pub(super) fn ApplySwingMove(player: &mut crate::game::BMC_Player, action: &SwingMove) {
    for die in &mut player.m_die {
        if die.m_in_reserve {
            continue;
        }
        for side in 0..2 {
            if let Some(s) = die.m_swing_type[side]
                && let Some((_, v)) = action.values().iter().find(|(kind, _)| *kind == s)
            {
                assert!(die.m_notset, "BMC_Die::OnSwingSet requires NOTSET state");
                die.m_sides[side] = *v;
            }
        }
    }
    for (index, second) in action.options() {
        if *second {
            player.m_die[*index].m_sides.swap(0, 1);
        }
    }
}
