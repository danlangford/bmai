// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

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
    pub fn win_probability(self) -> f32 {
        self.score / self.simulations as f32
    }
}

impl SearchResult {
    pub fn win_probability(&self) -> f32 {
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
    let probability = result.win_probability();
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
        game.generate_valid_attacks_in_cpp_order_for_search(settings.fire_candidate_limit());
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
        evaluator.evaluate_moves_batched_to_completion(moves, 1, &mut evaluate_batch)
    } else {
        evaluator.evaluate_moves_batched(moves, 1, &mut evaluate_batch)
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
    let probability = result.win_probability();
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
        game.generate_valid_attacks_in_cpp_order_for_search(settings.fire_candidate_limit());
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
    let selected = evaluator.evaluate_moves(moves, level, |candidate, _coordinate| {
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
            available_dice_count(&simulation.players[0]),
            available_dice_count(&simulation.players[1])
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

pub(crate) fn pass_move() -> Move {
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
        .any(|player| available_dice_count(player) == 0)
}

pub(super) fn win_probability(game: &Game) -> f32 {
    match game.players[0].score.total_cmp(&game.players[1].score) {
        std::cmp::Ordering::Greater => 1.0,
        std::cmp::Ordering::Equal => 0.5,
        std::cmp::Ordering::Less => 0.0,
    }
}

pub(crate) fn moves_including_pass(game: &Game, fire_limit: usize) -> Vec<Move> {
    let mut moves = game.generate_valid_attacks_in_cpp_order_for_search(fire_limit);
    if moves.is_empty() {
        moves.push(pass_move());
    }
    moves
}

pub(super) fn select_rollout_action(game: &Game, rng: &mut Rng, ai: &Bmai3) -> Move {
    let fire_limit = ai.fire_candidate_limit();
    match ai.playout {
        Playout::Quick => crate::engines::quick::attack(game, rng, fire_limit),
        Playout::Maximize => crate::engines::maximize::attack(game, rng, fire_limit),
        Playout::Random => crate::engines::random::attack(game, rng, fire_limit),
    }
}

thread_local! {
    static SCRATCH_GAME: std::cell::Cell<Option<Box<Game>>> = const { std::cell::Cell::new(None) };
}

/// Rollouts evaluate every candidate on a copy; reusing one per thread avoids
/// allocating dice for each rollout step. Dropping it hands it back.
pub(crate) struct ScratchGame(Option<Box<Game>>);

impl ScratchGame {
    pub(crate) fn new(source: &Game) -> Self {
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
pub(crate) fn restore_simulation(simulation: &mut Game, source: &Game) {
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
