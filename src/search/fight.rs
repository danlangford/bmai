// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

pub(crate) fn select_bmai_action(game: &Game, rng: &mut Rng, settings: &Bmai3) -> Move {
    select_bmai_action_with_stats(game, rng, settings).best_move
}

#[derive(Clone, Debug)]
pub(crate) struct SearchResult {
    pub best_move: Move,
    pub best_score: f32,
    pub sims_run: usize,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SearchProbability {
    pub score: f32,
    pub simulations: usize,
}

impl SearchProbability {
    pub fn probability_win(self) -> f32 {
        self.score / self.simulations as f32
    }
}

impl SearchResult {
    pub fn probability_win(&self) -> f32 {
        self.best_score / self.sims_run as f32
    }
}

pub(crate) fn select_bmai_action_with_stats(
    game: &Game,
    rng: &mut Rng,
    settings: &Bmai3,
) -> SearchResult {
    select_bmai_action_at_level_with_stats(game, rng, settings, 1, false)
}

pub(crate) fn select_native_bmai_action(
    game: &Game,
    rng_algorithm: crate::RngAlgorithm,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    settings: &Bmai3,
) -> Move {
    select_native_bmai_action_with_stats(game, rng_algorithm, replay, workers, settings).best_move
}

pub(crate) fn select_native_bmai_action_with_stats(
    game: &Game,
    rng_algorithm: crate::RngAlgorithm,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    settings: &Bmai3,
) -> SearchResult {
    select_bmai_action_at_level_native_with_stats(game, rng_algorithm, replay, workers, settings)
}

#[cfg(test)]
pub(super) fn select_bmai_action_at_level_native(
    game: &Game,
    rng_algorithm: crate::RngAlgorithm,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    settings: &Bmai3,
) -> (Move, f32) {
    let result = select_bmai_action_at_level_native_with_stats(
        game,
        rng_algorithm,
        replay,
        workers,
        settings,
    );
    let probability = result.probability_win();
    (result.best_move, probability)
}

pub(super) fn select_bmai_action_at_level_native_with_stats(
    game: &Game,
    rng_algorithm: crate::RngAlgorithm,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    settings: &Bmai3,
) -> SearchResult {
    let mut moves =
        game.generate_valid_attacks_in_cpp_order_for_search(settings.FireCandidateLimit());
    if moves.is_empty() {
        moves.push(pass_move());
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
            let mut simulation_rng = native_simulation_rng(
                rng_algorithm,
                replay,
                coordinate.candidate_index,
                coordinate.batch_index,
                coordinate.simulation_index,
            );
            evaluate_move(
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
    let probability = evaluator.last_probability_win;
    let selected = if probability == 0.0 && game.surrender_allowed {
        Move {
            action: Action::Surrender,
            attack: None,
            attackers: Vec::new().into(),
            targets: Vec::new().into(),
            score: 0.0,
            turbo_option: -1,
            fire: crate::game::FireAdjustment::default(),
        }
    } else {
        selected
    };
    SearchResult {
        best_move: selected,
        best_score: evaluator.last_best_score,
        sims_run: evaluator.last_sims_run,
    }
}

pub(crate) fn evaluate_selected_native_bmai_move(
    game: &Game,
    selected: &Move,
    rng_algorithm: crate::RngAlgorithm,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    settings: &Bmai3,
    simulations: usize,
) -> SearchProbability {
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
        let mut simulation_rng = native_simulation_rng(
            rng_algorithm,
            replay,
            coordinate.candidate_index,
            coordinate.batch_index,
            coordinate.simulation_index,
        );
        evaluate_move(
            &mut simulation,
            selected,
            &mut simulation_rng,
            settings,
            1,
            false,
            false,
        )
    });
    SearchProbability {
        score: scores.iter().sum(),
        simulations,
    }
}

pub(super) fn native_simulation_rng(
    algorithm: crate::RngAlgorithm,
    replay: crate::native::NativeReplayKey,
    candidate_index: impl TryInto<u64>,
    batch_index: usize,
    simulation_index: usize,
) -> Rng {
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
    Rng::from_native_stream(algorithm, key.derive_stream_seed(), key.stratum())
}

pub(super) fn select_bmai_action_at_level(
    game: &Game,
    rng: &mut Rng,
    settings: &Bmai3,
    level: usize,
    previous_pass: bool,
) -> (Move, f32) {
    let result = select_bmai_action_at_level_with_stats(game, rng, settings, level, previous_pass);
    let probability = result.probability_win();
    (result.best_move, probability)
}

pub(super) fn select_bmai_action_at_level_with_stats(
    game: &Game,
    rng: &mut Rng,
    settings: &Bmai3,
    level: usize,
    previous_pass: bool,
) -> SearchResult {
    let trace = trace_settings().bmai_attack;
    let trace_evaluation = trace_settings().attack_eval;
    let mut moves =
        game.generate_valid_attacks_in_cpp_order_for_search(settings.FireCandidateLimit());
    if moves.is_empty() {
        moves.push(pass_move());
    }
    if trace {
        eprintln!(
            "BMAI_BEGIN l{level} seed={} moves={} pass={previous_pass}",
            rng.debug_seed(),
            moves.len()
        );
    }
    let mut evaluator = settings.clone();
    let policy = settings.clone();
    let mut simulation = game.clone();
    let selected = evaluator.EvaluateMoves(moves, level, |candidate, _coordinate| {
        restore_simulation(&mut simulation, game);
        evaluate_move(
            &mut simulation,
            candidate,
            rng,
            &policy,
            level,
            previous_pass,
            trace_evaluation,
        )
    });
    let probability = evaluator.last_probability_win;
    let selected = if probability == 0.0 && game.surrender_allowed {
        Move {
            action: Action::Surrender,
            attack: None,
            attackers: Vec::new().into(),
            targets: Vec::new().into(),
            score: 0.0,
            turbo_option: -1,
            fire: crate::game::FireAdjustment::default(),
        }
    } else {
        selected
    };
    if trace {
        eprintln!(
            "BMAI_END l{level} seed={} probability={probability:.6} action={:?} attack={:?} {:?}->{:?}",
            rng.debug_seed(),
            selected.action,
            selected.attack,
            selected.attackers,
            selected.targets
        );
    }
    SearchResult {
        best_move: selected,
        best_score: evaluator.last_best_score,
        sims_run: evaluator.last_sims_run,
    }
}

pub(super) fn evaluate_move(
    simulation: &mut Game,
    candidate: &Move,
    rng: &mut Rng,
    settings: &Bmai3,
    level: usize,
    previous_pass: bool,
    trace: bool,
) -> f32 {
    if trace {
        eprintln!(
            "ATTACK_EVAL l{level} seed={} {:?} {:?}->{:?}",
            rng.debug_seed(),
            candidate.attack,
            candidate.attackers,
            candidate.targets
        );
    }
    // Treating surrender as a pass would waste a rollout and shift later RNG.
    if candidate.action == Action::Surrender {
        return 0.0;
    }
    if candidate.action == Action::Pass && previous_pass {
        return win_probability(simulation);
    }
    let extra_turn = if candidate.action == Action::Attack {
        apply_attack(simulation, candidate, rng)
    } else {
        false
    };
    if fight_over(simulation) {
        return win_probability(simulation);
    }
    if !extra_turn {
        simulation.players.swap(0, 1);
    }
    let result = if level >= settings.max_ply {
        let probability =
            play_fight_qai(simulation, rng, candidate.action == Action::Pass, settings);
        if extra_turn {
            probability
        } else {
            1.0 - probability
        }
    } else {
        let (_, next_probability) = select_bmai_action_at_level(
            simulation,
            rng,
            settings,
            level + 1,
            candidate.action == Action::Pass,
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
            rng.debug_seed(),
            simulation.players[0].score,
            simulation.players[1].score,
            available_dice(&simulation.players[0]),
            available_dice(&simulation.players[1])
        );
    }
    result
}

pub(super) fn play_fight_qai(game: &mut Game, rng: &mut Rng, mut passed: bool, ai: &Bmai3) -> f32 {
    let mut initial_player_is_zero = true;
    for _ in 0..256 {
        if fight_over(game) {
            break;
        }
        let action = select_rollout_action(game, rng, ai);
        if action.action != Action::Attack {
            if passed {
                break;
            }
            passed = true;
        } else {
            passed = false;
            let extra_turn = apply_attack(game, &action, rng);
            if extra_turn {
                continue;
            }
        }
        game.players.swap(0, 1);
        initial_player_is_zero = !initial_player_is_zero;
    }
    let current_zero_probability = win_probability(game);
    if initial_player_is_zero {
        current_zero_probability
    } else {
        1.0 - current_zero_probability
    }
}

pub(super) fn pass_move() -> Move {
    Move {
        action: Action::Pass,
        attack: None,
        attackers: Vec::new().into(),
        targets: Vec::new().into(),
        score: 0.0,
        turbo_option: -1,
        fire: crate::game::FireAdjustment::default(),
    }
}

pub(super) fn fight_over(game: &Game) -> bool {
    game.players
        .iter()
        .any(|player| available_dice(player) == 0)
}

pub(super) fn win_probability(game: &Game) -> f32 {
    match game.players[0].score.total_cmp(&game.players[1].score) {
        std::cmp::Ordering::Greater => 1.0,
        std::cmp::Ordering::Equal => 0.5,
        std::cmp::Ordering::Less => 0.0,
    }
}

pub(super) fn moves_including_pass(game: &Game, fire_limit: usize) -> Vec<Move> {
    let mut moves = game.generate_valid_attacks_in_cpp_order_for_search(fire_limit);
    if moves.is_empty() {
        moves.push(pass_move());
    }
    moves
}

pub(super) fn select_random_action(game: &Game, rng: &mut Rng, fire_limit: usize) -> Move {
    let moves = moves_including_pass(game, fire_limit);
    moves[rng.rand_max(moves.len() as u32) as usize].clone()
}

pub(super) fn select_maximize_action(game: &Game, rng: &mut Rng, fire_limit: usize) -> Move {
    let moves = moves_including_pass(game, fire_limit);
    let mut best = moves[0].clone();
    let mut best_score = f32::NEG_INFINITY;
    let mut simulation = ScratchGame::new(game);
    for candidate in moves {
        if candidate.action != Action::Attack {
            return candidate;
        }
        restore_simulation(&mut simulation, game);
        apply_attack(&mut simulation, &candidate, rng);
        let score = simulation.players[0].score - simulation.players[1].score;
        if score > best_score {
            best_score = score;
            best = candidate;
        }
    }
    best
}

pub(super) fn select_rollout_action(game: &Game, rng: &mut Rng, ai: &Bmai3) -> Move {
    match ai.rollout_policy {
        RolloutPolicy::Qai => select_qai_action_with_fire_limit(game, rng, ai.FireCandidateLimit()),
        RolloutPolicy::MaximizeOrRandom(probability) => {
            if rng.frand() < probability {
                select_maximize_action(game, rng, ai.FireCandidateLimit())
            } else {
                select_random_action(game, rng, ai.FireCandidateLimit())
            }
        }
    }
}

pub(crate) fn select_qai_action(game: &Game, rng: &mut Rng) -> Move {
    select_qai_action_with_fire_limit(game, rng, Bmai3::default().FireCandidateLimit())
}

pub(super) fn select_qai_action_with_fire_limit(
    game: &Game,
    rng: &mut Rng,
    fire_limit: usize,
) -> Move {
    let traces = trace_settings();
    let trace = traces.qai;
    let trace_rng = traces.rng;
    let trace_moves = traces.qai_moves;
    if trace {
        eprintln!(
            "QAI_BEGIN seed={} scores={:.1},{:.1}",
            rng.debug_seed(),
            game.players[0].score,
            game.players[1].score
        );
    }
    let mut best: Option<(f32, Move)> = None;
    let mut move_count = 0usize;
    let mut simulation = ScratchGame::new(game);
    for candidate in game.generate_valid_attacks_in_cpp_order_for_search(fire_limit) {
        move_count += 1;
        if trace_rng {
            eprintln!(
                "QAI_RNG before={} move={} attack={:?} attacker={} target={} scores={:.2},{:.2}",
                rng.debug_seed(),
                move_count - 1,
                candidate.attack,
                candidate.attackers.first().unwrap_or(usize::MAX),
                candidate.targets.first().unwrap_or(usize::MAX),
                game.players[0].score,
                game.players[1].score
            );
        }
        restore_simulation(&mut simulation, game);
        apply_attack(&mut simulation, &candidate, rng);
        let mut score = simulation.players[0].score - simulation.players[1].score;
        for attacker in candidate.attackers.iter() {
            let die = &game.players[0].dice[attacker];
            let delta = (die.sides_max() as f32 + 1.0) * 0.5 - die.value_total() as f32;
            if !die.has_property(property::SHADOW) {
                score += if die.has_property(property::POISON) {
                    -delta
                } else {
                    delta
                };
            }
        }
        score += rng.rand_max(5) as f32;
        if trace_rng {
            eprintln!("QAI_RNG after={} score={score:.2}", rng.debug_seed());
        }
        if trace_moves {
            eprintln!(
                "QAI_MOVE {score:.2} {:?} {:?}->{:?}",
                candidate.attack, candidate.attackers, candidate.targets
            );
        }
        if best
            .as_ref()
            .is_none_or(|(best_score, _)| score > *best_score)
        {
            best = Some((score, candidate));
        }
    }
    let selected = best.map_or_else(pass_move, |(_, action)| action);
    if trace {
        let values = |player: usize| {
            game.players[player]
                .dice
                .iter()
                .filter(|die| die.is_available())
                .map(Die::value_total)
                .collect::<Vec<_>>()
        };
        eprintln!(
            "QAI_BEST seed={} moves={move_count} {:?}|{:?} action={:?} attack={:?} {:?}->{:?}",
            rng.debug_seed(),
            values(0),
            values(1),
            selected.action,
            selected.attack,
            selected.attackers,
            selected.targets
        );
    }
    selected
}

thread_local! {
    static SCRATCH_GAME: std::cell::Cell<Option<Box<Game>>> = const { std::cell::Cell::new(None) };
}

/// Rollouts evaluate every candidate on a copy; reusing one per thread avoids
/// allocating dice for each rollout step. Dropping it hands it back.
struct ScratchGame(Option<Box<Game>>);

impl ScratchGame {
    fn new(source: &Game) -> Self {
        let scratch = match SCRATCH_GAME.take() {
            Some(mut scratch) => {
                restore_simulation(&mut scratch, source);
                scratch
            }
            None => Box::new(source.clone()),
        };
        Self(Some(scratch))
    }
}

impl std::ops::Deref for ScratchGame {
    type Target = Game;

    fn deref(&self) -> &Game {
        self.0
            .as_deref()
            .expect("scratch game is present until dropped")
    }
}

impl std::ops::DerefMut for ScratchGame {
    fn deref_mut(&mut self) -> &mut Game {
        self.0
            .as_deref_mut()
            .expect("scratch game is present until dropped")
    }
}

impl Drop for ScratchGame {
    fn drop(&mut self) {
        SCRATCH_GAME.set(self.0.take());
    }
}

/// Field by field, so the scratch players keep their dice allocations. The
/// destructuring makes a new field a compile error here.
pub(super) fn restore_simulation(simulation: &mut Game, source: &Game) {
    let Game {
        players,
        phase,
        surrender_allowed,
        target_wins,
        turbo_accuracy,
        fire_overshooting,
    } = source;
    for (simulation, source) in simulation.players.iter_mut().zip(players) {
        let crate::game::Player {
            id,
            score,
            dice,
            swing_set,
            round_original_sides,
            round_transformed,
            radioactive_products,
            rage_replacements,
            specials,
        } = source;
        simulation.id = *id;
        simulation.score = *score;
        if simulation.dice.len() == dice.len() {
            simulation.dice.copy_from_slice(dice);
        } else {
            simulation.dice.clone_from(dice);
        }
        simulation.swing_set = *swing_set;
        simulation.round_original_sides = *round_original_sides;
        simulation.round_transformed = *round_transformed;
        simulation.radioactive_products = *radioactive_products;
        simulation.rage_replacements = *rage_replacements;
        simulation.specials = *specials;
    }
    simulation.phase = *phase;
    simulation.surrender_allowed = *surrender_allowed;
    simulation.target_wins = *target_wins;
    simulation.turbo_accuracy = *turbo_accuracy;
    simulation.fire_overshooting = *fire_overshooting;
}
