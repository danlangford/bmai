// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

pub(crate) fn SelectBMAIAction(
    game: &BMC_Game,
    rng: &mut BMC_RNG,
    settings: &BMC_BMAI3,
) -> BMC_Move {
    SelectBMAIActionWithStats(game, rng, settings).m_move
}

#[derive(Clone, Debug)]
pub(crate) struct BMC_SearchResult {
    pub m_move: BMC_Move,
    pub m_best_score: f32,
    pub m_sims_run: usize,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BMC_ProbabilityEstimate {
    pub score: f32,
    pub simulations: usize,
}

impl BMC_ProbabilityEstimate {
    pub fn ProbabilityWin(self) -> f32 {
        self.score / self.simulations as f32
    }
}

impl BMC_SearchResult {
    pub fn ProbabilityWin(&self) -> f32 {
        self.m_best_score / self.m_sims_run as f32
    }
}

pub(crate) fn SelectBMAIActionWithStats(
    game: &BMC_Game,
    rng: &mut BMC_RNG,
    settings: &BMC_BMAI3,
) -> BMC_SearchResult {
    SelectBMAIActionAtLevelWithStats(game, rng, settings, 1, false)
}

pub(crate) fn SelectNativeBMAIAction(
    game: &BMC_Game,
    rng_algorithm: crate::BME_RNG_ALGORITHM,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    settings: &BMC_BMAI3,
) -> BMC_Move {
    SelectNativeBMAIActionWithStats(game, rng_algorithm, replay, workers, settings).m_move
}

pub(crate) fn SelectNativeBMAIActionWithStats(
    game: &BMC_Game,
    rng_algorithm: crate::BME_RNG_ALGORITHM,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    settings: &BMC_BMAI3,
) -> BMC_SearchResult {
    SelectBMAIActionAtLevelNativeWithStats(game, rng_algorithm, replay, workers, settings)
}

#[cfg(test)]
pub(super) fn SelectBMAIActionAtLevelNative(
    game: &BMC_Game,
    rng_algorithm: crate::BME_RNG_ALGORITHM,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    settings: &BMC_BMAI3,
) -> (BMC_Move, f32) {
    let result =
        SelectBMAIActionAtLevelNativeWithStats(game, rng_algorithm, replay, workers, settings);
    let probability = result.ProbabilityWin();
    (result.m_move, probability)
}

pub(super) fn SelectBMAIActionAtLevelNativeWithStats(
    game: &BMC_Game,
    rng_algorithm: crate::BME_RNG_ALGORITHM,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    settings: &BMC_BMAI3,
) -> BMC_SearchResult {
    let mut moves = game.GenerateValidAttacksInCppOrderForSearch(settings.FireCandidateLimit());
    if moves.is_empty() {
        moves.push(PassMove());
    }
    let mut evaluator = settings.clone();
    let policy = settings.clone();
    let mut evaluate_batch = |requests: &[crate::search::ai::EvaluationRequest<'_>]| {
        let tasks = requests
            .iter()
            .map(|request| (request.candidate.clone(), request.coordinate))
            .collect();
        crate::native::ordered_parallel_map(tasks, workers, |(candidate, coordinate)| {
            let mut simulation = game.clone();
            let mut simulation_rng = NativeSimulationRng(
                rng_algorithm,
                replay,
                coordinate.candidate_index,
                coordinate.batch_index,
                coordinate.simulation_index,
            );
            EvaluateMove(
                &mut simulation,
                &candidate,
                &mut simulation_rng,
                &policy,
                1,
                false,
                false,
            )
        })
    };
    let selected = if replay.stream_version.completes_probability_sample() {
        evaluator.EvaluateMovesBatchedToCompletion(moves, 1, &mut evaluate_batch)
    } else {
        evaluator.EvaluateMovesBatched(moves, 1, &mut evaluate_batch)
    };
    let probability = evaluator.m_last_probability_win;
    let selected = if probability == 0.0 && game.m_surrender_allowed {
        BMC_Move {
            m_action: BME_ACTION::SURRENDER,
            m_attack: None,
            m_attackers: Vec::new().into(),
            m_targets: Vec::new().into(),
            m_score: 0.0,
            m_turbo_option: -1,
            m_fire: crate::game::BMC_FireAdjustment::default(),
        }
    } else {
        selected
    };
    BMC_SearchResult {
        m_move: selected,
        m_best_score: evaluator.m_last_best_score,
        m_sims_run: evaluator.m_last_sims_run,
    }
}

pub(crate) fn EvaluateSelectedNativeBMAIMove(
    game: &BMC_Game,
    selected: &BMC_Move,
    rng_algorithm: crate::BME_RNG_ALGORITHM,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    settings: &BMC_BMAI3,
    simulations: usize,
) -> BMC_ProbabilityEstimate {
    assert!(simulations > 0);
    let tasks = (0..simulations)
        .map(|simulation_index| EvaluationCoordinate {
            candidate_index: NATIVE_REPORTING_STREAM,
            batch_index: 0,
            simulation_index,
        })
        .collect();
    let scores = crate::native::ordered_parallel_map(tasks, workers, |coordinate| {
        let mut simulation = game.clone();
        let mut simulation_rng = NativeSimulationRng(
            rng_algorithm,
            replay,
            coordinate.candidate_index,
            coordinate.batch_index,
            coordinate.simulation_index,
        );
        EvaluateMove(
            &mut simulation,
            selected,
            &mut simulation_rng,
            settings,
            1,
            false,
            false,
        )
    });
    BMC_ProbabilityEstimate {
        score: scores.iter().sum(),
        simulations,
    }
}

pub(super) fn NativeSimulationRng(
    algorithm: crate::BME_RNG_ALGORITHM,
    replay: crate::native::NativeReplayKey,
    candidate_index: impl TryInto<u64>,
    batch_index: usize,
    simulation_index: usize,
) -> BMC_RNG {
    let candidate_index = candidate_index
        .try_into()
        .ok()
        .expect("candidate index must fit in the replay format");
    let key = crate::native::NativeSimulationKey {
        replay,
        candidate_index,
        batch_index: batch_index as u64,
        simulation_index: simulation_index as u64,
    };
    BMC_RNG::FromNativeStream(algorithm, key.derive_stream_seed(), key.stratum())
}

pub(super) fn SelectBMAIActionAtLevel(
    game: &BMC_Game,
    rng: &mut BMC_RNG,
    settings: &BMC_BMAI3,
    level: usize,
    previous_pass: bool,
) -> (BMC_Move, f32) {
    let result = SelectBMAIActionAtLevelWithStats(game, rng, settings, level, previous_pass);
    let probability = result.ProbabilityWin();
    (result.m_move, probability)
}

pub(super) fn SelectBMAIActionAtLevelWithStats(
    game: &BMC_Game,
    rng: &mut BMC_RNG,
    settings: &BMC_BMAI3,
    level: usize,
    previous_pass: bool,
) -> BMC_SearchResult {
    let trace = TraceSettings().bmai_attack;
    let trace_evaluation = TraceSettings().attack_eval;
    let mut moves = game.GenerateValidAttacksInCppOrderForSearch(settings.FireCandidateLimit());
    if moves.is_empty() {
        moves.push(PassMove());
    }
    if trace {
        eprintln!(
            "BMAI_BEGIN l{level} seed={} moves={} pass={previous_pass}",
            rng.DebugSeed(),
            moves.len()
        );
    }
    let mut evaluator = settings.clone();
    let policy = settings.clone();
    let mut simulation = game.clone();
    let selected = evaluator.EvaluateMoves(moves, level, |candidate, _coordinate| {
        RestoreSimulation(&mut simulation, game);
        EvaluateMove(
            &mut simulation,
            candidate,
            rng,
            &policy,
            level,
            previous_pass,
            trace_evaluation,
        )
    });
    let probability = evaluator.m_last_probability_win;
    let selected = if probability == 0.0 && game.m_surrender_allowed {
        BMC_Move {
            m_action: BME_ACTION::SURRENDER,
            m_attack: None,
            m_attackers: Vec::new().into(),
            m_targets: Vec::new().into(),
            m_score: 0.0,
            m_turbo_option: -1,
            m_fire: crate::game::BMC_FireAdjustment::default(),
        }
    } else {
        selected
    };
    if trace {
        eprintln!(
            "BMAI_END l{level} seed={} probability={probability:.6} action={:?} attack={:?} {:?}->{:?}",
            rng.DebugSeed(),
            selected.m_action,
            selected.m_attack,
            selected.m_attackers,
            selected.m_targets
        );
    }
    BMC_SearchResult {
        m_move: selected,
        m_best_score: evaluator.m_last_best_score,
        m_sims_run: evaluator.m_last_sims_run,
    }
}

pub(super) fn EvaluateMove(
    simulation: &mut BMC_Game,
    candidate: &BMC_Move,
    rng: &mut BMC_RNG,
    settings: &BMC_BMAI3,
    level: usize,
    previous_pass: bool,
    trace: bool,
) -> f32 {
    if trace {
        eprintln!(
            "ATTACK_EVAL l{level} seed={} {:?} {:?}->{:?}",
            rng.DebugSeed(),
            candidate.m_attack,
            candidate.m_attackers,
            candidate.m_targets
        );
    }
    // BMC_Game::PlayFight_EvaluateMove returns immediately for surrender.  It
    // must not be treated like a pass: doing so consumes an entire QAI rollout
    // and, at ply 2+, changes both the result and all subsequent RNG state.
    if candidate.m_action == BME_ACTION::SURRENDER {
        return 0.0;
    }
    if candidate.m_action == BME_ACTION::PASS && previous_pass {
        return WinProbability(simulation);
    }
    let extra_turn = if candidate.m_action == BME_ACTION::ATTACK {
        ApplyAttack(simulation, candidate, rng)
    } else {
        false
    };
    if FightOver(simulation) {
        return WinProbability(simulation);
    }
    if !extra_turn {
        simulation.m_player.swap(0, 1);
    }
    let result = if level >= settings.m_max_ply {
        let probability = PlayFightQAI(
            simulation,
            rng,
            candidate.m_action == BME_ACTION::PASS,
            settings,
        );
        if extra_turn {
            probability
        } else {
            1.0 - probability
        }
    } else {
        let (_, next_probability) = SelectBMAIActionAtLevel(
            simulation,
            rng,
            settings,
            level + 1,
            candidate.m_action == BME_ACTION::PASS,
        );
        if extra_turn {
            next_probability
        } else {
            1.0 - next_probability
        }
    };
    if trace {
        eprintln!(
            "ATTACK_RESULT l{level} seed={} score={result} totals={:.1},{:.1} dice={},{}",
            rng.DebugSeed(),
            simulation.m_player[0].m_score,
            simulation.m_player[1].m_score,
            AvailableDice(&simulation.m_player[0]),
            AvailableDice(&simulation.m_player[1])
        );
    }
    result
}

pub(super) fn PlayFightQAI(
    game: &mut BMC_Game,
    rng: &mut BMC_RNG,
    mut passed: bool,
    ai: &BMC_BMAI3,
) -> f32 {
    let mut initial_player_is_zero = true;
    for _ in 0..256 {
        if FightOver(game) {
            break;
        }
        let action = SelectRolloutAction(game, rng, ai);
        if action.m_action != BME_ACTION::ATTACK {
            if passed {
                break;
            }
            passed = true;
        } else {
            passed = false;
            let extra_turn = ApplyAttack(game, &action, rng);
            if extra_turn {
                continue;
            }
        }
        game.m_player.swap(0, 1);
        initial_player_is_zero = !initial_player_is_zero;
    }
    let current_zero_probability = WinProbability(game);
    if initial_player_is_zero {
        current_zero_probability
    } else {
        1.0 - current_zero_probability
    }
}

pub(super) fn PassMove() -> BMC_Move {
    BMC_Move {
        m_action: BME_ACTION::PASS,
        m_attack: None,
        m_attackers: Vec::new().into(),
        m_targets: Vec::new().into(),
        m_score: 0.0,
        m_turbo_option: -1,
        m_fire: crate::game::BMC_FireAdjustment::default(),
    }
}

pub(super) fn FightOver(game: &BMC_Game) -> bool {
    game.m_player
        .iter()
        .any(|player| AvailableDice(player) == 0)
}

pub(super) fn WinProbability(game: &BMC_Game) -> f32 {
    match game.m_player[0]
        .m_score
        .total_cmp(&game.m_player[1].m_score)
    {
        std::cmp::Ordering::Greater => 1.0,
        std::cmp::Ordering::Equal => 0.5,
        std::cmp::Ordering::Less => 0.0,
    }
}

pub(super) fn MovesIncludingPass(game: &BMC_Game, fire_limit: usize) -> Vec<BMC_Move> {
    let mut moves = game.GenerateValidAttacksInCppOrderForSearch(fire_limit);
    if moves.is_empty() {
        moves.push(PassMove());
    }
    moves
}

pub(super) fn SelectRandomAction(
    game: &BMC_Game,
    rng: &mut BMC_RNG,
    fire_limit: usize,
) -> BMC_Move {
    let moves = MovesIncludingPass(game, fire_limit);
    moves[rng.GetRandMax(moves.len() as u32) as usize].clone()
}

pub(super) fn SelectMaximizeAction(
    game: &BMC_Game,
    rng: &mut BMC_RNG,
    fire_limit: usize,
) -> BMC_Move {
    let moves = MovesIncludingPass(game, fire_limit);
    let mut best = moves[0].clone();
    let mut best_score = f32::NEG_INFINITY;
    let mut simulation = game.clone();
    for candidate in moves {
        if candidate.m_action != BME_ACTION::ATTACK {
            return candidate;
        }
        RestoreSimulation(&mut simulation, game);
        ApplyAttack(&mut simulation, &candidate, rng);
        let score = simulation.m_player[0].m_score - simulation.m_player[1].m_score;
        if score > best_score {
            best_score = score;
            best = candidate;
        }
    }
    best
}

pub(super) fn SelectRolloutAction(game: &BMC_Game, rng: &mut BMC_RNG, ai: &BMC_BMAI3) -> BMC_Move {
    match ai.m_rollout_policy {
        BME_ROLLOUT_POLICY::QAI => SelectQAIActionWithFireLimit(game, rng, ai.FireCandidateLimit()),
        BME_ROLLOUT_POLICY::MAXIMIZE_OR_RANDOM(probability) => {
            if rng.GetFRand() < probability {
                SelectMaximizeAction(game, rng, ai.FireCandidateLimit())
            } else {
                SelectRandomAction(game, rng, ai.FireCandidateLimit())
            }
        }
    }
}

pub(crate) fn SelectQAIAction(game: &BMC_Game, rng: &mut BMC_RNG) -> BMC_Move {
    SelectQAIActionWithFireLimit(game, rng, BMC_BMAI3::default().FireCandidateLimit())
}

pub(super) fn SelectQAIActionWithFireLimit(
    game: &BMC_Game,
    rng: &mut BMC_RNG,
    fire_limit: usize,
) -> BMC_Move {
    let traces = TraceSettings();
    let trace = traces.qai;
    let trace_rng = traces.rng;
    let trace_moves = traces.qai_moves;
    if trace {
        eprintln!(
            "QAI_BEGIN seed={} scores={:.1},{:.1}",
            rng.DebugSeed(),
            game.m_player[0].m_score,
            game.m_player[1].m_score
        );
    }
    let mut best: Option<(f32, BMC_Move)> = None;
    let mut move_count = 0usize;
    // Match C++'s `BMC_Game sim(true); sim = *_game` lifecycle while letting
    // Vec::clone_from reuse the players' dice allocations safely.
    let mut simulation = game.clone();
    for candidate in game.GenerateValidAttacksInCppOrderForSearch(fire_limit) {
        move_count += 1;
        if trace_rng {
            eprintln!(
                "QAI_RNG before={} move={} attack={:?} attacker={} target={} scores={:.2},{:.2}",
                rng.DebugSeed(),
                move_count - 1,
                candidate.m_attack,
                candidate.m_attackers.first().unwrap_or(usize::MAX),
                candidate.m_targets.first().unwrap_or(usize::MAX),
                game.m_player[0].m_score,
                game.m_player[1].m_score
            );
        }
        RestoreSimulation(&mut simulation, game);
        ApplyAttack(&mut simulation, &candidate, rng);
        let mut score = simulation.m_player[0].m_score - simulation.m_player[1].m_score;
        for attacker in candidate.m_attackers.iter() {
            let die = &game.m_player[0].m_die[attacker];
            let delta = (die.GetSidesMax() as f32 + 1.0) * 0.5 - die.GetValueTotal() as f32;
            if !die.HasProperty(property::SHADOW) {
                score += if die.HasProperty(property::POISON) {
                    -delta
                } else {
                    delta
                };
            }
        }
        score += rng.GetRandMax(5) as f32;
        if trace_rng {
            eprintln!("QAI_RNG after={} score={score:.2}", rng.DebugSeed());
        }
        if trace_moves {
            eprintln!(
                "QAI_MOVE {score:.2} {:?} {:?}->{:?}",
                candidate.m_attack, candidate.m_attackers, candidate.m_targets
            );
        }
        if best
            .as_ref()
            .is_none_or(|(best_score, _)| score > *best_score)
        {
            best = Some((score, candidate));
        }
    }
    let selected = best.map_or_else(PassMove, |(_, action)| action);
    if trace {
        let values = |player: usize| {
            game.m_player[player]
                .m_die
                .iter()
                .filter(|die| die.IsAvailable())
                .map(BMC_Die::GetValueTotal)
                .collect::<Vec<_>>()
        };
        eprintln!(
            "QAI_BEST seed={} moves={move_count} {:?}|{:?} action={:?} attack={:?} {:?}->{:?}",
            rng.DebugSeed(),
            values(0),
            values(1),
            selected.m_action,
            selected.m_attack,
            selected.m_attackers,
            selected.m_targets
        );
    }
    selected
}

/// Rust-native equivalent of C++ `sim = *_game`: restore every game field
/// while retaining the scratch players' existing dice allocations.
pub(super) fn RestoreSimulation(simulation: &mut BMC_Game, source: &BMC_Game) {
    for player in 0..simulation.m_player.len() {
        simulation.m_player[player].m_id = source.m_player[player].m_id;
        simulation.m_player[player].m_score = source.m_player[player].m_score;
        let simulation_dice = &mut simulation.m_player[player].m_die;
        let source_dice = &source.m_player[player].m_die;
        if simulation_dice.len() == source_dice.len() {
            simulation_dice.copy_from_slice(source_dice);
        } else {
            simulation_dice.clone_from(source_dice);
        }
        simulation.m_player[player].m_swing_set = source.m_player[player].m_swing_set;
        simulation.m_player[player].m_round_original_sides =
            source.m_player[player].m_round_original_sides;
        simulation.m_player[player].m_round_transformed =
            source.m_player[player].m_round_transformed;
        simulation.m_player[player].m_radioactive_products =
            source.m_player[player].m_radioactive_products;
        simulation.m_player[player].m_rage_replacements =
            source.m_player[player].m_rage_replacements;
        simulation.m_player[player].m_specials = source.m_player[player].m_specials;
    }
    simulation.m_phase = source.m_phase;
    simulation.m_surrender_allowed = source.m_surrender_allowed;
    simulation.m_target_wins = source.m_target_wins;
    simulation.m_turbo_accuracy = source.m_turbo_accuracy;
    simulation.m_fire_overshooting = source.m_fire_overshooting;
}
