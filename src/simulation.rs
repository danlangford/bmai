// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

#![allow(non_snake_case)]

use crate::model::{BMC_Die, BMC_Game, BMC_Move, BME_ACTION, BME_ATTACK, BME_SWING_SET, property};
use crate::{BMC_BMAI3, BMC_RNG, BME_ROLLOUT_POLICY};

#[derive(Clone, Copy)]
struct NativeEvaluation {
    algorithm: crate::BME_RNG_ALGORITHM,
    replay: crate::native::NativeReplayKey,
    workers: usize,
}

struct NativeReplaySequence<'a> {
    algorithm: crate::BME_RNG_ALGORITHM,
    root_seed: u64,
    workers: usize,
    decision_index: &'a mut u64,
}

impl NativeReplaySequence<'_> {
    fn next(&mut self) -> NativeEvaluation {
        let replay = crate::native::NativeReplayKey {
            stream_version: crate::native::NativeStreamVersion::V1,
            root_seed: self.root_seed,
            decision_index: *self.decision_index,
        };
        *self.decision_index = self.decision_index.wrapping_add(1);
        NativeEvaluation {
            algorithm: self.algorithm,
            replay,
            workers: self.workers,
        }
    }
}

const NATIVE_ENUMERATION_STREAM: u64 = u64::MAX;
use std::sync::OnceLock;

#[derive(Clone, Debug)]
#[allow(clippy::upper_case_acronyms)]
pub enum BMC_AI_POLICY {
    BMAI(Box<BMC_BMAI3>),
    QAI,
    RANDOM,
    MAXIMIZE,
}

struct TraceSettings {
    swing_list: bool,
    swing_candidate: bool,
    swing_sim: bool,
    swing_moves: bool,
    swing: bool,
    reserve: bool,
    chance: bool,
    focus: bool,
    bmai_attack: bool,
    attack_eval: bool,
    qai: bool,
    rng: bool,
    qai_moves: bool,
}

fn TraceSettings() -> &'static TraceSettings {
    static SETTINGS: OnceLock<TraceSettings> = OnceLock::new();
    SETTINGS.get_or_init(|| TraceSettings {
        swing_list: std::env::var_os("BMAIR_TRACE_SWING_LIST").is_some(),
        swing_candidate: std::env::var_os("BMAIR_TRACE_SWING_CANDIDATE").is_some(),
        swing_sim: std::env::var_os("BMAIR_TRACE_SWING_SIM").is_some(),
        swing_moves: std::env::var_os("BMAIR_TRACE_SWING_MOVES").is_some(),
        swing: std::env::var_os("BMAIR_TRACE_SWING").is_some(),
        reserve: std::env::var_os("BMAIR_TRACE_RESERVE").is_some(),
        chance: std::env::var_os("BMAIR_TRACE_CHANCE").is_some(),
        focus: std::env::var_os("BMAIR_TRACE_FOCUS").is_some(),
        bmai_attack: std::env::var_os("BMAIR_TRACE_BMAI_ATTACK").is_some(),
        attack_eval: std::env::var_os("BMAIR_TRACE_ATTACK_EVAL").is_some(),
        qai: std::env::var_os("BMAIR_TRACE_QAI").is_some(),
        rng: std::env::var_os("BMAIR_TRACE_RNG").is_some(),
        qai_moves: std::env::var_os("BMAIR_TRACE_QAI_MOVES").is_some(),
    })
}

#[derive(Clone, Copy)]
pub(crate) struct SwingMove {
    values: [(char, u8); 10],
    value_len: u8,
    options: [(usize, bool); 10],
    option_len: u8,
}

#[derive(Clone, Debug)]
pub(crate) struct FocusMove {
    pub(crate) values: Vec<(usize, u8)>,
}

#[derive(Clone, Debug)]
pub(crate) struct ChanceMove {
    pub(crate) reroll: Vec<usize>,
}

#[derive(Clone, Copy)]
enum InitiativeStage {
    Chance,
    Focus,
    Fight,
}

impl SwingMove {
    fn empty() -> Self {
        Self {
            values: [('\0', 0); 10],
            value_len: 0,
            options: [(0, false); 10],
            option_len: 0,
        }
    }

    pub(crate) fn values(&self) -> &[(char, u8)] {
        &self.values[..usize::from(self.value_len)]
    }

    pub(crate) fn options(&self) -> &[(usize, bool)] {
        &self.options[..usize::from(self.option_len)]
    }

    fn push_value(&mut self, value: (char, u8)) {
        self.values[usize::from(self.value_len)] = value;
        self.value_len += 1;
    }

    fn push_option(&mut self, value: (usize, bool)) {
        self.options[usize::from(self.option_len)] = value;
        self.option_len += 1;
    }
}

pub fn PlayGames(
    template: &BMC_Game,
    games: usize,
    rng: &mut BMC_RNG,
    ai: &BMC_BMAI3,
) -> [usize; 2] {
    PlayGamesWithPolicies(
        template,
        games,
        rng,
        &[
            BMC_AI_POLICY::BMAI(Box::new(ai.clone())),
            BMC_AI_POLICY::BMAI(Box::new(ai.clone())),
        ],
    )
}

pub(crate) fn PlayGamesWithPolicies(
    template: &BMC_Game,
    games: usize,
    rng: &mut BMC_RNG,
    policies: &[BMC_AI_POLICY; 2],
) -> [usize; 2] {
    PlayGamesWithPoliciesInternal(template, games, rng, policies, None)
}

pub(crate) fn PlayGamesWithPoliciesNative(
    template: &BMC_Game,
    games: usize,
    rng: &mut BMC_RNG,
    policies: &[BMC_AI_POLICY; 2],
    root_seed: u64,
    workers: usize,
    decision_index: &mut u64,
) -> [usize; 2] {
    let mut native = NativeReplaySequence {
        algorithm: rng.Algorithm(),
        root_seed,
        workers,
        decision_index,
    };
    PlayGamesWithPoliciesInternal(template, games, rng, policies, Some(&mut native))
}

fn PlayGamesWithPoliciesInternal(
    template: &BMC_Game,
    games: usize,
    rng: &mut BMC_RNG,
    policies: &[BMC_AI_POLICY; 2],
    mut native: Option<&mut NativeReplaySequence<'_>>,
) -> [usize; 2] {
    let mut matches = [0, 0];
    for _ in 0..games {
        let (winner, wins, _) =
            PlayMatchWithPolicies(template, rng, policies, native.as_deref_mut());
        matches[winner] += 1;
        println!("game over {} - {} - 0", wins[0], wins[1]);
    }
    matches
}

fn PlayMatchWithPolicies(
    template: &BMC_Game,
    rng: &mut BMC_RNG,
    policies: &[BMC_AI_POLICY; 2],
    mut native: Option<&mut NativeReplaySequence<'_>>,
) -> (usize, [u8; 2], usize) {
    let mut game = template.clone();
    let mut wins = [0u8, 0u8];
    let mut initiative = 0;
    while wins[0] < template.m_target_wins && wins[1] < template.m_target_wins {
        let round = PlayRoundWithPolicies(&mut game, rng, policies, native.as_deref_mut());
        let winner = round.0;
        initiative = round.1;
        wins[winner] += 1;
        let loser = 1 - winner;
        game.m_player[loser].m_swing_set = BME_SWING_SET::NOT;
        for die in &mut game.m_player[loser].m_die {
            if die.m_swing_type.iter().any(Option::is_some) {
                die.m_notset = true;
            }
        }
    }
    (usize::from(wins[1] > wins[0]), wins, initiative)
}

pub(crate) fn PlayFairGames(
    template: &BMC_Game,
    games: usize,
    rng: &mut BMC_RNG,
    policies: &[BMC_AI_POLICY; 2],
) -> [[usize; 2]; 2] {
    PlayFairGamesInternal(template, games, rng, policies, None)
}

pub(crate) fn PlayFairGamesNative(
    template: &BMC_Game,
    games: usize,
    rng: &mut BMC_RNG,
    policies: &[BMC_AI_POLICY; 2],
    root_seed: u64,
    workers: usize,
    decision_index: &mut u64,
) -> [[usize; 2]; 2] {
    let mut native = NativeReplaySequence {
        algorithm: rng.Algorithm(),
        root_seed,
        workers,
        decision_index,
    };
    PlayFairGamesInternal(template, games, rng, policies, Some(&mut native))
}

fn PlayFairGamesInternal(
    template: &BMC_Game,
    games: usize,
    rng: &mut BMC_RNG,
    policies: &[BMC_AI_POLICY; 2],
    mut native: Option<&mut NativeReplaySequence<'_>>,
) -> [[usize; 2]; 2] {
    let mut wins = [[0usize; 2]; 2];
    for _ in 0..games {
        let (winner, _, initiative) =
            PlayMatchWithPolicies(template, rng, policies, native.as_deref_mut());
        wins[initiative][winner] += 1;
    }
    wins
}

fn PlayRoundWithPolicies(
    game: &mut BMC_Game,
    rng: &mut BMC_RNG,
    policies: &[BMC_AI_POLICY; 2],
    mut native: Option<&mut NativeReplaySequence<'_>>,
) -> (usize, usize) {
    PlayPreroundWithPolicies(game, rng, policies, native.as_deref_mut());
    for player in &mut game.m_player {
        player.m_score = 0.0;
        for die in &mut player.m_die {
            die.m_notset = true;
            RollDie(die, rng);
        }
        player.m_score = player
            .m_die
            .iter()
            .filter(|die| die.IsAvailable())
            .map(|die| die.GetScore(true))
            .sum();
        OptimizeDice(player);
    }
    let mut phase_player = InitiativeWinner(game);
    let initiative_winner = phase_player;
    loop {
        let player = 1 - phase_player;
        if !HasAvailableProperty(&game.m_player[player], property::CHANCE) {
            break;
        }
        let action = match &policies[player] {
            BMC_AI_POLICY::BMAI(ai) => {
                let native_evaluation = native.as_deref_mut().map(NativeReplaySequence::next);
                SelectChanceAction(game, player, rng, ai, 1, phase_player, native_evaluation).0
            }
            BMC_AI_POLICY::QAI | BMC_AI_POLICY::RANDOM | BMC_AI_POLICY::MAXIMIZE => {
                ChanceMove { reroll: Vec::new() }
            }
        };
        let (next_phase, continues) = ApplyChanceMove(game, player, phase_player, &action, rng);
        if TraceSettings().chance {
            eprintln!(
                "CHANCE_APPLY player={player} previous={phase_player} next={next_phase} continues={continues} values={:?}",
                game.m_player[player]
                    .m_die
                    .iter()
                    .map(BMC_Die::GetValueTotal)
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
        if !HasAvailableProperty(&game.m_player[player], property::FOCUS) {
            break;
        }
        let action = match &policies[player] {
            BMC_AI_POLICY::BMAI(ai) => {
                let native_evaluation = native.as_deref_mut().map(NativeReplaySequence::next);
                SelectFocusAction(game, player, rng, ai, 1, phase_player, native_evaluation).0
            }
            BMC_AI_POLICY::QAI | BMC_AI_POLICY::RANDOM | BMC_AI_POLICY::MAXIMIZE => {
                FocusMove { values: Vec::new() }
            }
        };
        if action.values.is_empty() {
            break;
        }
        ApplyFocusMove(game, player, &action);
        phase_player = player;
    }
    let mut consecutive_passes = 0;
    for _ in 0..256 {
        if FightOver(game) {
            break;
        }
        let mut oriented = game.clone();
        if phase_player == 1 {
            oriented.m_player.swap(0, 1);
        }
        let action = match &policies[phase_player] {
            BMC_AI_POLICY::BMAI(ai) => {
                if let Some(context) = native.as_deref_mut().map(NativeReplaySequence::next) {
                    SelectNativeBMAIAction(
                        &oriented,
                        context.algorithm,
                        context.replay,
                        context.workers,
                        ai,
                    )
                } else {
                    SelectBMAIAction(&oriented, rng, ai)
                }
            }
            BMC_AI_POLICY::QAI => SelectQAIAction(&oriented, rng),
            BMC_AI_POLICY::RANDOM => SelectRandomAction(&oriented, rng),
            BMC_AI_POLICY::MAXIMIZE => SelectMaximizeAction(&oriented, rng),
        };
        if action.m_action == BME_ACTION::SURRENDER {
            game.m_player[phase_player].m_score = -1000.0;
            break;
        } else if action.m_action != BME_ACTION::ATTACK {
            consecutive_passes += 1;
            if consecutive_passes == 2 {
                break;
            }
            RecoverDizzyDice(&mut game.m_player[phase_player]);
        } else {
            consecutive_passes = 0;
            let extra_turn =
                ApplyAttackForPlayers(game, &action, phase_player, 1 - phase_player, rng);
            RecoverDizzyDice(&mut game.m_player[phase_player]);
            if extra_turn {
                continue;
            }
        }
        phase_player = 1 - phase_player;
    }
    (
        usize::from(game.m_player[1].m_score > game.m_player[0].m_score),
        initiative_winner,
    )
}

fn PlayPreroundWithPolicies(
    game: &mut BMC_Game,
    rng: &mut BMC_RNG,
    policies: &[BMC_AI_POLICY; 2],
    mut native: Option<&mut NativeReplaySequence<'_>>,
) {
    for (player, policy) in policies.iter().enumerate() {
        if game.m_player[player].m_swing_set != BME_SWING_SET::NOT
            || !NeedsSetSwing(&game.m_player[player])
        {
            game.m_player[player].m_swing_set = BME_SWING_SET::LOCKED;
            continue;
        }
        let selected = match policy {
            BMC_AI_POLICY::BMAI(ai) => {
                let native_evaluation = native.as_deref_mut().map(NativeReplaySequence::next);
                SelectSwingAction(game, player, rng, ai, 1, native_evaluation).0
            }
            BMC_AI_POLICY::QAI | BMC_AI_POLICY::RANDOM | BMC_AI_POLICY::MAXIMIZE => {
                GenerateSwingMoves(&game.m_player[player])
                    .into_iter()
                    .next()
                    .unwrap_or_else(SwingMove::empty)
            }
        };
        ApplySwingMove(&mut game.m_player[player], &selected);
        game.m_player[player].m_swing_set = BME_SWING_SET::LOCKED;
    }
}

fn PlayPreround(game: &mut BMC_Game, rng: &mut BMC_RNG, ai: &BMC_BMAI3, level: usize) -> usize {
    // In a simulated game BMAI's static search level is not restored after a
    // preround evaluation.  Consequently, the next player's preround choice
    // starts one ply deeper.  Top-level games restore the level after each
    // choice, so both players remain at level 1 there.
    let mut player_level = level;
    for player in 0..2 {
        if game.m_player[player].m_swing_set != BME_SWING_SET::NOT
            || !NeedsSetSwing(&game.m_player[player])
        {
            game.m_player[player].m_swing_set = BME_SWING_SET::LOCKED;
            continue;
        }
        let (selected, _) = SelectSwingAction(game, player, rng, ai, player_level, None);
        ApplySwingMove(&mut game.m_player[player], &selected);
        game.m_player[player].m_swing_set = BME_SWING_SET::LOCKED;
        if level > 1 {
            player_level += 1;
        }
    }
    player_level
}

fn SelectSwingAction(
    game: &BMC_Game,
    player: usize,
    rng: &mut BMC_RNG,
    ai: &BMC_BMAI3,
    level: usize,
    native: Option<NativeEvaluation>,
) -> (SwingMove, f32) {
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
        if sims_run >= sims || moves.len() == 1 || !ai.m_cull_moves {
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
        if moves.len() == 1 {
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

fn ApplyUseReserve(die: &mut BMC_Die) {
    die.m_in_reserve = false;
    die.m_properties &= !property::RESERVE;
    die.m_value_total = None;
    die.m_notset = true;
}

fn RandomlySelectSwingMoves(
    moves: &mut Vec<SwingMove>,
    player: &crate::model::BMC_Player,
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

fn EvaluateSwingMove(
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

fn PlayRoundToNextBMAIAction(
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

fn RollRoundDice(game: &mut BMC_Game, rng: &mut BMC_RNG) {
    for player in &mut game.m_player {
        player.m_score = 0.0;
        for die in &mut player.m_die {
            die.m_notset = true;
            RollDie(die, rng);
        }
        player.m_score = player
            .m_die
            .iter()
            .filter(|d| d.IsAvailable())
            .map(|d| d.GetScore(true))
            .sum();
        player.OptimizeDice();
    }
}

fn HasAvailableProperty(player: &crate::model::BMC_Player, property: u64) -> bool {
    player
        .m_die
        .iter()
        .any(|die| die.IsAvailable() && die.HasProperty(property))
}

fn GenerateChanceMoves(game: &BMC_Game, player: usize) -> Vec<ChanceMove> {
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

fn ApplyChanceMove(
    game: &mut BMC_Game,
    player: usize,
    previous_initiative: usize,
    action: &ChanceMove,
    rng: &mut BMC_RNG,
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
    // Preserve BMC_Game::ApplyUseChance exactly. The C++ implementation tests
    // `initiative != 0`, rather than comparing initiative with the acting
    // player. Consequently a reroll is considered successful precisely when
    // player 0 wins initiative, even when player 1 is the Chance user.
    if CheckInitiative(game) == Some(0) {
        (player, true)
    } else {
        (previous_initiative, false)
    }
}

fn SelectChanceAction(
    game: &BMC_Game,
    player: usize,
    rng: &mut BMC_RNG,
    ai: &BMC_BMAI3,
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
        if sims_run >= sims || moves.len() == 1 || !ai.m_cull_moves {
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
        if moves.len() == 1 {
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

fn EvaluateChanceSimulation(
    simulation: &mut BMC_Game,
    player: usize,
    initiative: usize,
    action: &ChanceMove,
    rng: &mut BMC_RNG,
    ai: &BMC_BMAI3,
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

pub(crate) fn SelectBMAIChanceAction(
    game: &BMC_Game,
    rng: &mut BMC_RNG,
    ai: &BMC_BMAI3,
) -> ChanceMove {
    SelectChanceAction(game, 0, rng, ai, 1, 1, None).0
}

pub(crate) fn SelectNativeBMAIChanceAction(
    game: &BMC_Game,
    rng_algorithm: crate::BME_RNG_ALGORITHM,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    ai: &BMC_BMAI3,
) -> ChanceMove {
    let mut unused_legacy_rng = BMC_RNG::UntracedDefault();
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

fn GenerateFocusMoves(game: &BMC_Game, player: usize) -> Vec<FocusMove> {
    let focus = game.m_player[player]
        .m_die
        .iter()
        .enumerate()
        .filter(|(_, die)| {
            die.IsAvailable() && die.HasProperty(property::FOCUS) && die.GetValueTotal() > 1
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

fn ApplyFocusMove(game: &mut BMC_Game, player: usize, action: &FocusMove) {
    for (index, value) in &action.values {
        let die = &mut game.m_player[player].m_die[*index];
        die.m_value_total = Some(*value);
        die.m_dizzy = true;
    }
    game.m_player[player].OptimizeDice();
}

fn SelectFocusAction(
    game: &BMC_Game,
    player: usize,
    rng: &mut BMC_RNG,
    ai: &BMC_BMAI3,
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
        if sims_run >= sims || moves.len() == 1 || !ai.m_cull_moves {
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
        if moves.len() == 1 {
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

fn EvaluateFocusSimulation(
    simulation: &mut BMC_Game,
    player: usize,
    initiative: usize,
    action: &FocusMove,
    rng: &mut BMC_RNG,
    ai: &BMC_BMAI3,
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

pub(crate) fn SelectBMAIFocusAction(
    game: &BMC_Game,
    rng: &mut BMC_RNG,
    ai: &BMC_BMAI3,
) -> FocusMove {
    SelectFocusAction(game, 0, rng, ai, 1, 1, None).0
}

pub(crate) fn SelectNativeBMAIFocusAction(
    game: &BMC_Game,
    rng_algorithm: crate::BME_RNG_ALGORITHM,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    ai: &BMC_BMAI3,
) -> FocusMove {
    let mut unused_legacy_rng = BMC_RNG::UntracedDefault();
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

fn EvaluateNextInitiativeAction(
    game: &mut BMC_Game,
    rng: &mut BMC_RNG,
    ai: &BMC_BMAI3,
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

pub(crate) fn RecoverDizzyDice(player: &mut crate::model::BMC_Player) {
    for die in &mut player.m_die {
        die.m_dizzy = false;
    }
}

// Play a round inside a BMAI simulation. The simulation retains BMAI until an
// evaluation reaches max ply; OnEndEvaluation then replaces both AIs with QAI
// for the remainder of this round.
fn PlaySimulatedRound(
    game: &mut BMC_Game,
    rng: &mut BMC_RNG,
    ai: &BMC_BMAI3,
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
        if action.m_action == BME_ACTION::SURRENDER {
            game.m_player[phase].m_score = -1000.0;
            break;
        }
        if action.m_action != BME_ACTION::ATTACK {
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
        if action.m_action != BME_ACTION::ATTACK {
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

fn PlayRoundQAI(game: &mut BMC_Game, rng: &mut BMC_RNG, pov: usize, ai: &BMC_BMAI3) -> f32 {
    RollRoundDice(game, rng);
    let phase = InitiativeWinner(game);
    PlayFightQAIFromPhase(game, rng, phase, pov, false, ai)
}

fn PlayFightQAIFromPhase(
    game: &mut BMC_Game,
    rng: &mut BMC_RNG,
    mut phase: usize,
    pov: usize,
    mut passed: bool,
    ai: &BMC_BMAI3,
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
        if action.m_action != BME_ACTION::ATTACK {
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
        if action.m_action != BME_ACTION::ATTACK {
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

fn NeedsSetSwing(player: &crate::model::BMC_Player) -> bool {
    player
        .m_die
        .iter()
        .filter(|die| !die.m_in_reserve)
        .any(|d| d.m_swing_type.iter().any(Option::is_some) || d.HasProperty(property::OPTION))
}

fn GenerateSwingMoves(player: &crate::model::BMC_Player) -> Vec<SwingMove> {
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

fn ApplySwingMove(player: &mut crate::model::BMC_Player, action: &SwingMove) {
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

pub(crate) fn SelectBMAIAction(
    game: &BMC_Game,
    rng: &mut BMC_RNG,
    settings: &BMC_BMAI3,
) -> BMC_Move {
    SelectBMAIActionAtLevel(game, rng, settings, 1, false).0
}

pub(crate) fn SelectNativeBMAIAction(
    game: &BMC_Game,
    rng_algorithm: crate::BME_RNG_ALGORITHM,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    settings: &BMC_BMAI3,
) -> BMC_Move {
    SelectBMAIActionAtLevelNative(game, rng_algorithm, replay, workers, settings).0
}

fn SelectBMAIActionAtLevelNative(
    game: &BMC_Game,
    rng_algorithm: crate::BME_RNG_ALGORITHM,
    replay: crate::native::NativeReplayKey,
    workers: usize,
    settings: &BMC_BMAI3,
) -> (BMC_Move, f32) {
    let mut moves = game.GenerateValidAttacksInCppOrder();
    if moves.is_empty() {
        moves.push(PassMove());
    }
    let mut evaluator = settings.clone();
    let policy = settings.clone();
    let selected = evaluator.EvaluateMovesBatched(moves, 1, |requests| {
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
        }
    } else {
        selected
    };
    (selected, probability)
}

fn NativeSimulationRng(
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
    let stream_seed = crate::native::NativeSimulationKey {
        replay,
        candidate_index,
        batch_index: batch_index as u64,
        simulation_index: simulation_index as u64,
    }
    .derive_stream_seed();
    BMC_RNG::FromNativeStream(algorithm, stream_seed)
}

fn SelectBMAIActionAtLevel(
    game: &BMC_Game,
    rng: &mut BMC_RNG,
    settings: &BMC_BMAI3,
    level: usize,
    previous_pass: bool,
) -> (BMC_Move, f32) {
    let trace = TraceSettings().bmai_attack;
    let trace_evaluation = TraceSettings().attack_eval;
    let mut moves = game.GenerateValidAttacksInCppOrder();
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
    (selected, probability)
}

fn EvaluateMove(
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

fn PlayFightQAI(game: &mut BMC_Game, rng: &mut BMC_RNG, mut passed: bool, ai: &BMC_BMAI3) -> f32 {
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

fn PassMove() -> BMC_Move {
    BMC_Move {
        m_action: BME_ACTION::PASS,
        m_attack: None,
        m_attackers: Vec::new().into(),
        m_targets: Vec::new().into(),
        m_score: 0.0,
        m_turbo_option: -1,
    }
}

fn FightOver(game: &BMC_Game) -> bool {
    game.m_player
        .iter()
        .any(|player| AvailableDice(player) == 0)
}

fn WinProbability(game: &BMC_Game) -> f32 {
    match game.m_player[0]
        .m_score
        .total_cmp(&game.m_player[1].m_score)
    {
        std::cmp::Ordering::Greater => 1.0,
        std::cmp::Ordering::Equal => 0.5,
        std::cmp::Ordering::Less => 0.0,
    }
}

fn MovesIncludingPass(game: &BMC_Game) -> Vec<BMC_Move> {
    let mut moves = game.GenerateValidAttacksInCppOrder();
    if moves.is_empty() {
        moves.push(PassMove());
    }
    moves
}

fn SelectRandomAction(game: &BMC_Game, rng: &mut BMC_RNG) -> BMC_Move {
    let moves = MovesIncludingPass(game);
    moves[rng.GetRandMax(moves.len() as u32) as usize].clone()
}

fn SelectMaximizeAction(game: &BMC_Game, rng: &mut BMC_RNG) -> BMC_Move {
    let moves = MovesIncludingPass(game);
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

fn SelectRolloutAction(game: &BMC_Game, rng: &mut BMC_RNG, ai: &BMC_BMAI3) -> BMC_Move {
    match ai.m_rollout_policy {
        BME_ROLLOUT_POLICY::QAI => SelectQAIAction(game, rng),
        BME_ROLLOUT_POLICY::MAXIMIZE_OR_RANDOM(probability) => {
            if rng.GetFRand() < probability {
                SelectMaximizeAction(game, rng)
            } else {
                SelectRandomAction(game, rng)
            }
        }
    }
}

pub(crate) fn SelectQAIAction(game: &BMC_Game, rng: &mut BMC_RNG) -> BMC_Move {
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
    for candidate in game.GenerateValidAttacksInCppOrder() {
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
fn RestoreSimulation(simulation: &mut BMC_Game, source: &BMC_Game) {
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
    }
    simulation.m_phase = source.m_phase;
    simulation.m_surrender_allowed = source.m_surrender_allowed;
    simulation.m_target_wins = source.m_target_wins;
    simulation.m_turbo_accuracy = source.m_turbo_accuracy;
}

pub(crate) fn ApplyAttack(game: &mut BMC_Game, action: &BMC_Move, rng: &mut BMC_RNG) -> bool {
    ApplyAttackForPlayers(game, action, 0, 1, rng)
}

fn ApplyAttackForPlayers(
    game: &mut BMC_Game,
    action: &BMC_Move,
    attacker_player: usize,
    target_player: usize,
    rng: &mut BMC_RNG,
) -> bool {
    // C++ GetAvailableDice() is a cached boundary and does not shrink merely
    // because an attacker is marked NOTSET during this phase.
    let available_attackers = AvailableDice(&game.m_player[attacker_player]);
    let is_trip = action.m_attack == Some(BME_ATTACK::TRIP);
    let null_attacker = action
        .m_attackers
        .iter()
        .any(|index| game.m_player[attacker_player].m_die[index].HasProperty(property::NULL));
    let value_attacker = action
        .m_attackers
        .iter()
        .any(|index| game.m_player[attacker_player].m_die[index].HasProperty(property::VALUE));

    let actual_attackers = action.m_attackers;
    for attacker in actual_attackers.iter() {
        ApplyAttackPlayerEffects(game, action, attacker_player, target_player, attacker, true);
    }

    if is_trip {
        let target = action.m_targets.first().expect("Trip target");
        if !game.m_player[target_player].m_die[target].HasProperty(property::KONSTANT) {
            game.m_player[target_player].m_die[target].m_notset = true;
        }
        ApplyBeforeRollEffects(game, target_player, target);
    }

    // C++ handles Ornery dice that were not already scheduled by the attack.
    if action.m_attack.is_some() {
        for attacker in 0..available_attackers {
            let die = &game.m_player[attacker_player].m_die[attacker];
            if !die.HasProperty(property::ORNERY) || die.m_notset {
                continue;
            }
            if !die.HasProperty(property::KONSTANT) {
                game.m_player[attacker_player].m_die[attacker].m_notset = true;
            }
            ApplyBeforeRollEffects(game, attacker_player, attacker);
        }
    }

    // Match ApplyAttackNatureRoll: actual attackers first, then Ornery dice
    // that did not participate, and finally a Trip target.
    for attacker in actual_attackers.iter() {
        ApplyAttackerNatureRoll(game, attacker_player, attacker, rng);
    }
    if action.m_attack.is_some() {
        for attacker in 0..available_attackers {
            let die = &game.m_player[attacker_player].m_die[attacker];
            if die.HasProperty(property::ORNERY) && !actual_attackers.contains(attacker) {
                ApplyAttackerNatureRoll(game, attacker_player, attacker, rng);
            }
        }
    }
    if is_trip {
        let target = action.m_targets.first().expect("Trip target");
        if game.m_player[target_player].m_die[target].m_notset {
            RollScheduledDie(game, target_player, target, rng);
        }
        let attacker = action.m_attackers.first().expect("Trip attacker");
        if game.m_player[attacker_player].m_die[attacker].GetValueTotal()
            < game.m_player[target_player].m_die[target].GetValueTotal()
        {
            OptimizeDice(&mut game.m_player[attacker_player]);
            OptimizeDice(&mut game.m_player[target_player]);
            return false;
        }
    }

    for (removed, original_target) in action.m_targets.iter().enumerate() {
        let target = original_target - removed;
        let own_score = game.m_player[target_player].m_die[target].GetScore(true);
        game.m_player[target_player].m_score -= own_score;
        if null_attacker {
            game.m_player[target_player].m_die[target].m_properties |= property::NULL;
        }
        if value_attacker {
            game.m_player[target_player].m_die[target].m_properties |= property::VALUE;
        }
        let captured_score = game.m_player[target_player].m_die[target].GetScore(false);
        game.m_player[attacker_player].m_score += captured_score;
        OnDieLost(&mut game.m_player[target_player], target);
    }
    let extra_turn = action.m_attackers.iter().any(|index| {
        let die = &game.m_player[attacker_player].m_die[index];
        die.HasProperty(property::TIME_AND_SPACE)
            && !die.HasProperty(property::KONSTANT)
            && die.GetValueTotal() % 2 == 1
    });
    OptimizeDice(&mut game.m_player[attacker_player]);
    extra_turn
}

fn ApplyAttackPlayerEffects(
    game: &mut BMC_Game,
    action: &BMC_Move,
    attacker_player: usize,
    target_player: usize,
    attacker: usize,
    actually_attacking: bool,
) {
    if !game.m_player[attacker_player].m_die[attacker].HasProperty(property::KONSTANT) {
        game.m_player[attacker_player].m_die[attacker].m_notset = true;
    }

    if actually_attacking && action.m_attack == Some(BME_ATTACK::BERSERK) {
        let die = &mut game.m_player[attacker_player].m_die[attacker];
        let old_score = die.GetScore(true);
        die.m_sides[0] = die.m_sides[0].div_ceil(2);
        die.m_properties &= !property::BERSERK;
        game.m_player[attacker_player].m_score += die.GetScore(true) - old_score;
    }

    if game.m_player[attacker_player].m_die[attacker].m_notset {
        ApplyBeforeRollEffects(game, attacker_player, attacker);
    }

    // Morphing is implemented by C++ only for its 1_1 and N_1 attack types.
    if action.m_targets.len() == 1
        && !matches!(
            action.m_attack,
            Some(BME_ATTACK::BERSERK | BME_ATTACK::SPEED)
        )
        && game.m_player[attacker_player].m_die[attacker].HasProperty(property::MORPHING)
    {
        let target = action.m_targets.first().expect("Morphing target");
        let target_die = game.m_player[target_player].m_die[target];
        let die = &mut game.m_player[attacker_player].m_die[attacker];
        let old_score = die.GetScore(true);
        if target_die.HasProperty(property::TWIN) {
            die.m_properties |= property::TWIN;
            die.m_sides = target_die.m_sides;
        } else {
            die.m_properties &= !property::TWIN;
            die.m_sides = [target_die.GetSidesMax() as u8, 0];
        }
        game.m_player[attacker_player].m_score += die.GetScore(true) - old_score;
    }

    if game.m_player[attacker_player].m_die[attacker].HasProperty(property::TURBO)
        && action.m_turbo_option >= 0
    {
        if game.m_player[attacker_player].m_die[attacker].HasProperty(property::OPTION) {
            if action.m_turbo_option == 1 {
                let die = &mut game.m_player[attacker_player].m_die[attacker];
                let old_score = die.GetScore(true);
                die.m_sides.swap(0, 1);
                game.m_player[attacker_player].m_score += die.GetScore(true) - old_score;
            }
        } else if action.m_turbo_option > 0
            && let Some(swing) = game.m_player[attacker_player].m_die[attacker].m_swing_type[0]
        {
            let mut score_delta = 0.0;
            for die in &mut game.m_player[attacker_player].m_die {
                let old_score = die.GetScore(true);
                for side in 0..2 {
                    if die.m_swing_type[side] == Some(swing) {
                        die.m_sides[side] = action.m_turbo_option as u8;
                    }
                }
                score_delta += die.GetScore(true) - old_score;
            }
            game.m_player[attacker_player].m_score += score_delta;
        }
    }

    if game.m_player[attacker_player].m_die[attacker].HasProperty(property::WARRIOR) {
        let die = &mut game.m_player[attacker_player].m_die[attacker];
        let old_score = die.GetScore(true);
        die.m_properties &= !property::WARRIOR;
        game.m_player[attacker_player].m_score += die.GetScore(true) - old_score;
    }
}

fn ApplyBeforeRollEffects(game: &mut BMC_Game, player: usize, index: usize) {
    let die = &mut game.m_player[player].m_die[index];
    let old_score = die.GetScore(true);
    let dice = if die.HasProperty(property::TWIN) {
        2
    } else {
        1
    };
    if die.HasProperty(property::MIGHTY) {
        for sides in die.m_sides.iter_mut().take(dice) {
            *sides = MightySides(*sides);
        }
    }
    if die.HasProperty(property::WEAK) {
        for sides in die.m_sides.iter_mut().take(dice) {
            *sides = WeakSides(*sides);
        }
    }
    game.m_player[player].m_score += die.GetScore(true) - old_score;
}

fn ApplyAttackerNatureRoll(game: &mut BMC_Game, player: usize, index: usize, rng: &mut BMC_RNG) {
    let die = &mut game.m_player[player].m_die[index];
    let old_score = die.GetScore(true);
    ApplyMood(die, rng);
    // C++ score bookkeeping surrounds side/property changes, but Roll itself
    // does not notify the owner. In particular, a Value die keeps the score
    // contributed by its pre-attack value after the nature reroll.
    game.m_player[player].m_score += die.GetScore(true) - old_score;
    if die.m_notset {
        RollDie(die, rng);
    }
}

fn RollScheduledDie(game: &mut BMC_Game, player: usize, index: usize, rng: &mut BMC_RNG) {
    let die = &mut game.m_player[player].m_die[index];
    RollDie(die, rng);
}

fn OnDieLost(player: &mut crate::model::BMC_Player, index: usize) {
    let available = AvailableDice(player);
    let mut lost = player.m_die.remove(index);
    lost.m_captured = true;
    player.m_die.insert(available - 1, lost);
}

fn ApplyMood(die: &mut BMC_Die, rng: &mut BMC_RNG) {
    if die.HasProperty(property::MOOD) {
        for index in 0..die.m_sides.len() {
            if let Some(swing) = die.m_swing_type[index] {
                die.m_sides[index] = match swing {
                    'X' => [4, 6, 8, 10, 12, 20][rng.GetRandMax(6) as usize],
                    'V' => [6, 8, 10, 12][rng.GetRandMax(4) as usize],
                    _ => {
                        let (min, max) = SwingRange(swing);
                        min + rng.GetRandMax(u32::from(max - min + 1)) as u8
                    }
                };
            }
        }
    }
}

fn MightySides(sides: u8) -> u8 {
    const VALUES: [u8; 20] = [
        1, 2, 4, 4, 6, 6, 8, 8, 10, 10, 12, 12, 16, 16, 16, 16, 20, 20, 20, 20,
    ];
    if sides >= 20 {
        30
    } else {
        VALUES[sides as usize]
    }
}

fn WeakSides(sides: u8) -> u8 {
    const VALUES: [u8; 20] = [
        1, 1, 1, 2, 2, 4, 4, 6, 6, 8, 8, 10, 10, 12, 12, 12, 12, 16, 16, 16,
    ];
    match sides {
        31.. => 30,
        21..=30 => 20,
        20 => 16,
        _ => VALUES[sides as usize],
    }
}

fn OptimizeDice(player: &mut crate::model::BMC_Player) {
    player.OptimizeDice();
}

pub(crate) fn RollDie(die: &mut BMC_Die, rng: &mut BMC_RNG) {
    assert!(die.m_notset, "BMC_Die::Roll requires NOTSET state");
    die.m_captured = false;
    die.m_notset = false;
    die.m_dizzy = false;
    if die.m_in_reserve {
        die.m_value_total = None;
        return;
    }
    let dice = if die.HasProperty(property::TWIN) {
        2
    } else {
        1
    };
    let mut value = 0u16;
    for sides in die.m_sides.iter().take(dice) {
        if *sides > 0 {
            if die.HasProperty(property::WARRIOR | property::MAXIMUM) {
                value += u16::from(*sides);
            } else {
                value += u16::from(rng.GetRandMax(u32::from(*sides)) as u8 + 1);
            }
        }
    }
    die.m_value_total = Some(value as u8);
}

fn InitiativeWinner(game: &BMC_Game) -> usize {
    CheckInitiative(game).unwrap_or(0)
}

pub(crate) fn CheckInitiative(game: &BMC_Game) -> Option<usize> {
    let mut values = [Vec::new(), Vec::new()];
    for (player, output) in values.iter_mut().enumerate() {
        *output = game.m_player[player]
            .m_die
            .iter()
            .filter(|die| {
                die.IsAvailable()
                    && !die.HasProperty(property::TRIP | property::SLOW | property::STINGER)
            })
            .map(BMC_Die::GetValueTotal)
            .collect();
        output.sort_unstable();
    }
    for index in 0..values[0].len().max(values[1].len()) {
        match (values[0].get(index), values[1].get(index)) {
            (Some(a), Some(b)) if a != b => return Some(usize::from(a > b)),
            (None, Some(_)) => return Some(1),
            (Some(_), None) => return Some(0),
            _ => {}
        }
    }
    None
}

fn AvailableDice(player: &crate::model::BMC_Player) -> usize {
    player.m_die.iter().filter(|die| die.IsAvailable()).count()
}

fn SwingRange(swing: char) -> (u8, u8) {
    match swing {
        'P' => (1, 30),
        'Q' => (2, 20),
        'R' => (2, 16),
        'S' => (6, 20),
        'T' => (2, 12),
        'U' => (8, 30),
        'V' => (6, 12),
        'W' => (4, 12),
        'X' => (4, 20),
        'Y' => (1, 20),
        'Z' => (4, 30),
        _ => (0, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BMC_Die, BMC_DieIndexSet, BMC_Player};

    #[test]
    fn native_fight_score_summary_is_stable() {
        let input = include_str!("../tests/native-fixtures/fight.txt");
        let setup = input.split_once("getaction").unwrap().0;
        let mut parser = crate::BMC_Parser::default();
        parser.ParseString(setup, &mut Vec::new()).unwrap();
        let settings = BMC_BMAI3 {
            m_min_sims: 20,
            m_max_sims: 20,
            m_max_branch: 100,
            ..Default::default()
        };
        let replay = crate::native::NativeReplayKey {
            stream_version: crate::native::NativeStreamVersion::V1,
            root_seed: 17,
            decision_index: 0,
        };

        let available = std::thread::available_parallelism().map_or(1, usize::from);
        let mut expected: Option<(BMC_Move, f32)> = None;
        for workers in [1, 2, available] {
            let result = SelectBMAIActionAtLevelNative(
                &parser.m_game,
                crate::BME_RNG_ALGORITHM::LEGACY_PARK_MILLER_V1,
                replay,
                workers,
                &settings,
            );
            if let Some(expected) = &expected {
                assert_eq!(result.0.m_action, expected.0.m_action);
                assert_eq!(result.0.m_attack, expected.0.m_attack);
                assert_eq!(result.0.m_attackers, expected.0.m_attackers);
                assert_eq!(result.0.m_targets, expected.0.m_targets);
                assert_eq!(result.0.m_score, expected.0.m_score);
                assert_eq!(result.0.m_turbo_option, expected.0.m_turbo_option);
                assert_eq!(result.1, expected.1);
            } else {
                expected = Some(result);
            }
        }

        let (action, probability) = expected.unwrap();
        assert_eq!(action.m_action, BME_ACTION::ATTACK);
        assert_eq!(action.m_attack, Some(BME_ATTACK::POWER));
        assert_eq!(action.m_attackers, vec![0]);
        assert_eq!(action.m_targets, vec![1]);
        assert_eq!(probability, 0.0);
    }

    fn swing_die(swing: char, properties: u64, original_index: usize) -> BMC_Die {
        BMC_Die {
            m_properties: property::VALID | properties,
            m_sides: [0, 0],
            m_swing_type: [Some(swing), None],
            m_value_total: None,
            m_captured: false,
            m_notset: false,
            m_dizzy: false,
            m_original_index: original_index,
            m_in_reserve: false,
        }
    }

    #[test]
    fn unique_rejects_equal_values_on_lower_swing_types() {
        let player = BMC_Player {
            m_die: vec![swing_die('P', 0, 0), swing_die('Q', property::UNIQUE, 1)],
            ..Default::default()
        };

        let moves = GenerateSwingMoves(&player);
        assert_eq!(moves.len(), 30 * 19 - 19);
        assert!(moves.iter().all(|candidate| {
            let p = candidate
                .values()
                .iter()
                .find(|(swing, _)| *swing == 'P')
                .unwrap()
                .1;
            let q = candidate
                .values()
                .iter()
                .find(|(swing, _)| *swing == 'Q')
                .unwrap()
                .1;
            p != q
        }));
    }

    #[test]
    fn turbo_swing_changes_all_matching_dice_before_the_reroll() {
        let mut game = BMC_Game::default();
        let mut turbo = swing_die('X', property::TURBO | property::KONSTANT, 0);
        turbo.m_sides[0] = 10;
        turbo.m_value_total = Some(10);
        let mut companion = swing_die('X', property::KONSTANT, 1);
        companion.m_sides[0] = 10;
        companion.m_value_total = Some(7);
        game.m_player[0].m_die = vec![turbo, companion];
        let mut target = swing_die('P', 0, 0);
        target.m_sides[0] = 6;
        target.m_value_total = Some(6);
        game.m_player[1].m_die = vec![target];
        let action = BMC_Move {
            m_action: BME_ACTION::ATTACK,
            m_attack: Some(BME_ATTACK::POWER),
            m_attackers: vec![0].into(),
            m_targets: vec![0].into(),
            m_score: 0.0,
            m_turbo_option: 20,
        };

        let mut rng = BMC_RNG::default();
        ApplyAttack(&mut game, &action, &mut rng);
        assert!(
            game.m_player[0]
                .m_die
                .iter()
                .all(|die| die.m_swing_type[0] != Some('X') || die.m_sides[0] == 20)
        );
    }

    #[test]
    fn pr82_trip_target_before_roll_effect_triggers_once() {
        for (effect, starting_sides, expected_sides) in
            [(property::MIGHTY, 6, 8), (property::WEAK, 20, 16)]
        {
            let mut game = BMC_Game::default();
            let mut attacker = swing_die('P', property::TRIP | property::KONSTANT, 0);
            attacker.m_sides[0] = 4;
            attacker.m_value_total = Some(1);
            game.m_player[0].m_die = vec![attacker];
            let mut target = swing_die('P', effect, 0);
            target.m_sides[0] = starting_sides;
            target.m_value_total = Some(starting_sides);
            game.m_player[1].m_die = vec![target];
            let action = BMC_Move::attack(BME_ATTACK::TRIP, [0], [0], 0.0);

            ApplyAttack(&mut game, &action, &mut BMC_RNG::default());
            assert_eq!(game.m_player[1].m_die[0].m_sides[0], expected_sides);
        }
    }

    #[test]
    fn pr82_participating_ornery_before_roll_effect_triggers_once() {
        let mut game = BMC_Game::default();
        let mut attacker = swing_die('P', property::ORNERY | property::MIGHTY, 0);
        attacker.m_sides[0] = 4;
        attacker.m_value_total = Some(4);
        game.m_player[0].m_die = vec![attacker];
        let mut target = swing_die('P', 0, 0);
        target.m_sides[0] = 1;
        target.m_value_total = Some(1);
        game.m_player[1].m_die = vec![target];
        let action = BMC_Move {
            m_action: BME_ACTION::ATTACK,
            m_attack: Some(BME_ATTACK::POWER),
            m_attackers: vec![0].into(),
            m_targets: vec![0].into(),
            m_score: 0.0,
            m_turbo_option: -1,
        };

        ApplyAttack(&mut game, &action, &mut BMC_RNG::default());
        assert_eq!(game.m_player[0].m_die[0].m_sides[0], 6);
    }

    /// Ports OrdinarySideChangeInvalidatesValue through the attack-side-change
    /// path that owns the corresponding lifecycle transition in Rust.
    #[test]
    fn pr82_ordinary_side_change_invalidates_value() {
        let mut game = BMC_Game::default();
        let mut attacker = swing_die('P', property::MIGHTY, 0);
        attacker.m_sides[0] = 6;
        attacker.m_value_total = Some(3);
        let mut target = swing_die('P', 0, 0);
        target.m_sides[0] = 1;
        target.m_value_total = Some(1);
        game.m_player[0].m_die = vec![attacker];
        game.m_player[1].m_die = vec![target];
        let action = BMC_Move::attack(BME_ATTACK::POWER, [0], [0], 0.0);

        ApplyAttackPlayerEffects(&mut game, &action, 0, 1, 0, true);
        assert!(game.m_player[0].m_die[0].m_notset);
        assert_eq!(game.m_player[0].m_die[0].m_sides[0], 8);
    }

    #[test]
    fn copied_cpp_konstant_skill_attacker_keeps_its_value() {
        let mut game = BMC_Game::default();
        let mut konstant = swing_die('P', property::KONSTANT, 0);
        konstant.m_sides[0] = 20;
        konstant.m_value_total = Some(13);
        let mut ordinary = swing_die('P', 0, 1);
        ordinary.m_sides[0] = 7;
        ordinary.m_value_total = Some(7);
        game.m_player[0].m_die = vec![konstant, ordinary];
        let mut target = swing_die('P', 0, 0);
        target.m_sides[0] = 20;
        target.m_value_total = Some(20);
        game.m_player[1].m_die = vec![target];
        let action = BMC_Move {
            m_action: BME_ACTION::ATTACK,
            m_attack: Some(BME_ATTACK::SKILL),
            m_attackers: vec![0, 1].into(),
            m_targets: vec![0].into(),
            m_score: 0.0,
            m_turbo_option: -1,
        };

        ApplyAttack(&mut game, &action, &mut BMC_RNG::default());
        assert_eq!(
            game.m_player[0]
                .m_die
                .iter()
                .find(|die| die.m_original_index == 0)
                .unwrap()
                .GetValueTotal(),
            13
        );
    }

    #[test]
    fn copied_cpp_morphing_speed_attack_does_not_morph() {
        let mut game = BMC_Game::default();
        let mut attacker = swing_die('P', property::MORPHING | property::SPEED, 0);
        attacker.m_sides[0] = 10;
        attacker.m_value_total = Some(8);
        game.m_player[0].m_die = vec![attacker];
        let mut first = swing_die('P', 0, 0);
        first.m_sides[0] = 4;
        first.m_value_total = Some(3);
        let mut second = swing_die('P', 0, 1);
        second.m_sides[0] = 6;
        second.m_value_total = Some(5);
        game.m_player[1].m_die = vec![first, second];
        let action = BMC_Move {
            m_action: BME_ACTION::ATTACK,
            m_attack: Some(BME_ATTACK::SPEED),
            m_attackers: vec![0].into(),
            m_targets: vec![0, 1].into(),
            m_score: 0.0,
            m_turbo_option: -1,
        };

        ApplyAttack(&mut game, &action, &mut BMC_RNG::default());
        assert_eq!(game.m_player[0].m_die[0].m_sides[0], 10);
    }

    #[test]
    fn copied_cpp_konstant_chance_die_keeps_its_value() {
        let mut game = BMC_Game::default();
        let mut chance = swing_die('P', property::CHANCE | property::KONSTANT, 0);
        chance.m_sides[0] = 100;
        chance.m_value_total = Some(7);
        game.m_player[0].m_die = vec![chance];
        let mut opponent = swing_die('P', 0, 0);
        opponent.m_sides[0] = 20;
        opponent.m_value_total = Some(20);
        game.m_player[1].m_die = vec![opponent];

        ApplyChanceMove(
            &mut game,
            0,
            1,
            &ChanceMove { reroll: vec![0] },
            &mut BMC_RNG::default(),
        );
        assert_eq!(game.m_player[0].m_die[0].GetValueTotal(), 7);
    }

    /// Regression for BMC_Game::ApplyUseChance's literal `initiative != 0`
    /// check. This intentionally preserves the C++ player-index asymmetry.
    #[test]
    fn cpp_chance_success_is_keyed_to_player_zero_initiative() {
        let mut game = BMC_Game::default();
        let mut zero = swing_die('P', 0, 0);
        zero.m_sides[0] = 20;
        zero.m_value_total = Some(5);
        let mut one = swing_die('P', property::CHANCE | property::KONSTANT, 0);
        one.m_sides[0] = 20;
        one.m_value_total = Some(6);
        game.m_player[0].m_die = vec![zero];
        game.m_player[1].m_die = vec![one];

        let result = ApplyChanceMove(
            &mut game,
            1,
            0,
            &ChanceMove { reroll: vec![0] },
            &mut BMC_RNG::default(),
        );
        assert_eq!(CheckInitiative(&game), Some(0));
        assert_eq!(result, (1, true));
    }

    #[test]
    fn cpp_focus_marks_dice_dizzy_until_turn_recovery() {
        let mut game = BMC_Game::default();
        let mut focus = swing_die('P', property::FOCUS, 0);
        focus.m_sides[0] = 20;
        focus.m_value_total = Some(12);
        game.m_player[0].m_die = vec![focus];

        ApplyFocusMove(
            &mut game,
            0,
            &FocusMove {
                values: vec![(0, 7)],
            },
        );
        assert_eq!(game.m_player[0].m_die[0].GetValueTotal(), 7);
        assert!(game.m_player[0].m_die[0].m_dizzy);
        RecoverDizzyDice(&mut game.m_player[0]);
        assert!(!game.m_player[0].m_die[0].m_dizzy);
    }

    #[test]
    fn cpp_value_attacker_score_retains_its_pre_reroll_value() {
        let mut game = BMC_Game::default();
        let mut attacker = swing_die('P', property::VALUE, 0);
        attacker.m_sides[0] = 20;
        attacker.m_value_total = Some(15);
        game.m_player[0].m_die = vec![attacker];
        game.m_player[0].m_score = 7.5;
        let mut target = swing_die('P', 0, 0);
        target.m_sides[0] = 6;
        target.m_value_total = Some(5);
        game.m_player[1].m_die = vec![target];
        game.m_player[1].m_score = 3.0;
        let action = BMC_Move {
            m_action: BME_ACTION::ATTACK,
            m_attack: Some(BME_ATTACK::POWER),
            m_attackers: vec![0].into(),
            m_targets: vec![0].into(),
            m_score: 0.0,
            m_turbo_option: -1,
        };
        let mut rng = BMC_RNG::default();
        rng.SRand(1);

        ApplyAttack(&mut game, &action, &mut rng);
        assert_ne!(game.m_player[0].m_die[0].GetValueTotal(), 15);
        assert_eq!(game.m_player[0].m_score, 12.5);
    }

    /// Port of SkillTests.MaximumSkill's roll contract.
    #[test]
    fn cpp_maximum_die_always_rolls_its_maximum() {
        let mut maximum = swing_die('P', property::MAXIMUM, 0);
        maximum.m_sides[0] = 6;
        for seed in 1..=10 {
            maximum.m_value_total = None;
            maximum.m_notset = true;
            let mut rng = BMC_RNG::default();
            rng.SRand(seed);
            RollDie(&mut maximum, &mut rng);
            assert_eq!(maximum.GetValueTotal(), 6);
        }
    }

    /// Port of RollRequiresNotSetState. Rust assertions are active in debug
    /// test builds, matching the upstream test's non-NDEBUG branch.
    #[test]
    #[should_panic(expected = "BMC_Die::Roll requires NOTSET state")]
    fn cpp_roll_requires_notset_state() {
        let mut die = swing_die('P', 0, 0);
        die.m_sides[0] = 6;
        die.m_value_total = Some(1);
        die.m_notset = false;
        RollDie(&mut die, &mut BMC_RNG::default());
    }

    /// Port of SwingSetRequiresNotSetState.
    #[test]
    #[should_panic(expected = "BMC_Die::OnSwingSet requires NOTSET state")]
    fn cpp_swing_set_requires_notset_state() {
        let mut die = swing_die('X', 0, 0);
        die.m_sides[0] = 6;
        die.m_value_total = Some(1);
        die.m_notset = false;
        let mut player = crate::model::BMC_Player {
            m_die: vec![die],
            ..Default::default()
        };
        let mut action = SwingMove::empty();
        action.push_value(('X', 8));
        ApplySwingMove(&mut player, &action);
    }

    /// Port of KonstantRetainsValueWhenTripped.
    #[test]
    fn cpp_konstant_target_retains_value_when_tripped() {
        let mut game = BMC_Game::default();
        let mut trip = swing_die('P', property::TRIP | property::KONSTANT, 0);
        trip.m_sides[0] = 8;
        trip.m_value_total = Some(8);
        game.m_player[0].m_die = vec![trip];
        let mut target = swing_die('P', property::KONSTANT, 0);
        target.m_sides[0] = 100;
        target.m_value_total = Some(7);
        game.m_player[1].m_die = vec![target];
        let action = BMC_Move {
            m_action: BME_ACTION::ATTACK,
            m_attack: Some(BME_ATTACK::TRIP),
            m_attackers: vec![0].into(),
            m_targets: vec![0].into(),
            m_score: 0.0,
            m_turbo_option: -1,
        };
        let mut rng = BMC_RNG::default();
        rng.SRand(1);
        ApplyAttack(&mut game, &action, &mut rng);
        assert_eq!(game.m_player[1].m_die[0].GetValueTotal(), 7);
        assert!(game.m_player[1].m_die[0].m_captured);
    }

    /// Port of KonstantWarriorRetainsValueWhenUsedInSkillAttack.
    #[test]
    fn cpp_konstant_warrior_keeps_value_and_loses_warrior_after_skill() {
        let mut game = BMC_Game::default();
        let mut warrior = swing_die('P', property::WARRIOR | property::KONSTANT, 0);
        warrior.m_sides[0] = 41;
        warrior.m_value_total = Some(17);
        let mut ordinary = swing_die('P', 0, 1);
        ordinary.m_sides[0] = 11;
        ordinary.m_value_total = Some(11);
        game.m_player[0].m_die = vec![warrior, ordinary];
        let mut target = swing_die('P', 0, 0);
        target.m_sides[0] = 20;
        target.m_value_total = Some(28);
        game.m_player[1].m_die = vec![target];
        let action = BMC_Move {
            m_action: BME_ACTION::ATTACK,
            m_attack: Some(BME_ATTACK::SKILL),
            m_attackers: vec![0, 1].into(),
            m_targets: vec![0].into(),
            m_score: 0.0,
            m_turbo_option: -1,
        };
        let mut rng = BMC_RNG::default();
        rng.SRand(1);
        ApplyAttack(&mut game, &action, &mut rng);
        let warrior = game.m_player[0]
            .m_die
            .iter()
            .find(|die| die.m_original_index == 0)
            .unwrap();
        assert_eq!(warrior.GetValueTotal(), 17);
        assert!(!warrior.HasProperty(property::WARRIOR));
    }

    /// Port of MorphingSkill and MorphingTwinSkill in both directions.
    #[test]
    fn cpp_morphing_copies_single_and_twin_target_sizes() {
        let cases = [([9, 0], [7, 0], [7, 0]), ([7, 0], [10, 11], [10, 11])];
        for (attacker_sides, target_sides, expected) in cases {
            let mut game = BMC_Game::default();
            let mut attacker = swing_die('P', property::MORPHING, 0);
            attacker.m_sides = attacker_sides;
            attacker.m_value_total = Some(8);
            if attacker_sides[1] > 0 {
                attacker.m_properties |= property::TWIN;
            }
            let mut target = swing_die('P', property::MORPHING, 0);
            target.m_sides = target_sides;
            target.m_value_total = Some(6);
            if target_sides[1] > 0 {
                target.m_properties |= property::TWIN;
            }
            game.m_player[0].m_die = vec![attacker];
            game.m_player[1].m_die = vec![target];
            let action = BMC_Move {
                m_action: BME_ACTION::ATTACK,
                m_attack: Some(BME_ATTACK::POWER),
                m_attackers: vec![0].into(),
                m_targets: vec![0].into(),
                m_score: 0.0,
                m_turbo_option: -1,
            };
            ApplyAttack(&mut game, &action, &mut BMC_RNG::default());
            assert_eq!(game.m_player[0].m_die[0].m_sides, expected);
            assert_eq!(
                game.m_player[0].m_die[0].HasProperty(property::TWIN),
                expected[1] > 0
            );
        }
    }

    /// Ports PR #82's Chance Mighty/Weak/Maximum Konstant regressions.
    #[test]
    fn pr82_chance_effects_run_once_while_konstant_retains_value() {
        for (properties, expected_sides) in [
            (property::MIGHTY, 8),
            (property::WEAK, 4),
            (property::MAXIMUM, 6),
        ] {
            let mut game = BMC_Game::default();
            let mut chance = swing_die('P', property::CHANCE | property::KONSTANT | properties, 0);
            chance.m_sides[0] = 6;
            chance.m_value_total = Some(3);
            game.m_player[0].m_die = vec![chance];
            let mut opponent = swing_die('P', 0, 0);
            opponent.m_sides[0] = 20;
            opponent.m_value_total = Some(20);
            game.m_player[1].m_die = vec![opponent];

            let mut rng = BMC_RNG::default();
            rng.SRand(1);
            ApplyChanceMove(&mut game, 0, 1, &ChanceMove { reroll: vec![0] }, &mut rng);
            assert_eq!(game.m_player[0].m_die[0].GetValueTotal(), 3);
            assert_eq!(game.m_player[0].m_die[0].m_sides[0], expected_sides);
        }

        for (effect, expected_sides) in [(property::MIGHTY, 8), (property::WEAK, 4)] {
            let mut game = BMC_Game::default();
            let mut chance = swing_die('P', property::CHANCE | effect, 0);
            chance.m_sides[0] = 6;
            chance.m_value_total = Some(1);
            game.m_player[0].m_die = vec![chance];
            let mut opponent = swing_die('P', 0, 0);
            opponent.m_sides[0] = 20;
            opponent.m_value_total = Some(20);
            game.m_player[1].m_die = vec![opponent];
            let mut rng = BMC_RNG::default();
            rng.SRand(1);
            ApplyChanceMove(&mut game, 0, 1, &ChanceMove { reroll: vec![0] }, &mut rng);
            assert_eq!(game.m_player[0].m_die[0].m_sides[0], expected_sides);
        }
    }

    /// Ports the PR #82 Konstant Trip Mighty/Weak cases.
    #[test]
    fn pr82_konstant_trip_target_retains_value_and_changes_sides_once() {
        for (effect, expected_sides) in [(property::MIGHTY, 8), (property::WEAK, 4)] {
            let mut game = BMC_Game::default();
            let mut attacker = swing_die('P', property::TRIP, 0);
            attacker.m_sides[0] = 6;
            attacker.m_value_total = Some(6);
            let mut target = swing_die('P', property::KONSTANT | effect, 0);
            target.m_sides[0] = 6;
            target.m_value_total = Some(3);
            game.m_player[0].m_die = vec![attacker];
            game.m_player[1].m_die = vec![target];
            let action = BMC_Move::attack(BME_ATTACK::TRIP, [0], [0], 0.0);

            let mut rng = BMC_RNG::default();
            rng.SRand(1);
            ApplyAttack(&mut game, &action, &mut rng);
            assert_eq!(game.m_player[1].m_die[0].GetValueTotal(), 3);
            assert_eq!(game.m_player[1].m_die[0].m_sides[0], expected_sides);
        }
    }

    /// Ports KonstantOrneryMighty/Weak and NonparticipatingOrneryDieRerolls.
    #[test]
    fn pr82_nonparticipating_ornery_effects_and_rolls_match_cpp() {
        for (effect, expected_sides) in [(property::MIGHTY, 8), (property::WEAK, 4)] {
            let mut game = BMC_Game::default();
            let mut attacker = swing_die('P', 0, 0);
            attacker.m_sides[0] = 6;
            attacker.m_value_total = Some(6);
            let mut ornery = swing_die('P', property::ORNERY | property::KONSTANT | effect, 1);
            ornery.m_sides[0] = 6;
            ornery.m_value_total = Some(3);
            let mut target = swing_die('P', 0, 0);
            target.m_sides[0] = 1;
            target.m_value_total = Some(1);
            game.m_player[0].m_die = vec![attacker, ornery];
            game.m_player[1].m_die = vec![target];

            ApplyAttack(
                &mut game,
                &BMC_Move::attack(BME_ATTACK::POWER, [0], [0], 0.0),
                &mut BMC_RNG::default(),
            );
            let ornery = game.m_player[0]
                .m_die
                .iter()
                .find(|die| die.m_original_index == 1)
                .unwrap();
            assert_eq!(ornery.GetValueTotal(), 3);
            assert_eq!(ornery.m_sides[0], expected_sides);
        }

        let mut game = BMC_Game::default();
        let mut attacker = swing_die('P', 0, 0);
        attacker.m_sides[0] = 6;
        attacker.m_value_total = Some(6);
        let mut ornery = swing_die('P', property::ORNERY, 1);
        ornery.m_sides[0] = 100;
        ornery.m_value_total = Some(100);
        let mut target = swing_die('P', 0, 0);
        target.m_sides[0] = 1;
        target.m_value_total = Some(1);
        game.m_player[0].m_die = vec![attacker, ornery];
        game.m_player[1].m_die = vec![target];
        let mut rng = BMC_RNG::default();
        rng.SRand(1);
        ApplyAttack(
            &mut game,
            &BMC_Move::attack(BME_ATTACK::POWER, [0], [0], 0.0),
            &mut rng,
        );
        let ornery = game.m_player[0]
            .m_die
            .iter()
            .find(|die| die.m_original_index == 1)
            .unwrap();
        assert_ne!(ornery.GetValueTotal(), 100);
    }

    /// Ports OrneryMoodDoesNotChangeOnPass and the Konstant Mood attack case.
    #[test]
    fn pr82_ornery_mood_only_changes_on_an_attack() {
        let mood_die = || {
            let mut die = swing_die(
                'X',
                property::ORNERY | property::MOOD | property::KONSTANT,
                0,
            );
            die.m_sides[0] = 6;
            die.m_value_total = Some(3);
            die
        };

        let mut pass_game = BMC_Game::default();
        pass_game.m_player[0].m_die = vec![mood_die()];
        ApplyAttack(
            &mut pass_game,
            &BMC_Move {
                m_action: BME_ACTION::PASS,
                m_attack: None,
                m_attackers: BMC_DieIndexSet::default(),
                m_targets: BMC_DieIndexSet::default(),
                m_score: 0.0,
                m_turbo_option: -1,
            },
            &mut BMC_RNG::default(),
        );
        assert_eq!(
            (
                pass_game.m_player[0].m_die[0].m_sides[0],
                pass_game.m_player[0].m_die[0].GetValueTotal()
            ),
            (6, 3)
        );

        let mut attack_game = BMC_Game::default();
        let mut attacker = swing_die('P', 0, 1);
        attacker.m_sides[0] = 6;
        attacker.m_value_total = Some(6);
        let mut target = swing_die('P', 0, 0);
        target.m_sides[0] = 1;
        target.m_value_total = Some(1);
        attack_game.m_player[0].m_die = vec![attacker, mood_die()];
        attack_game.m_player[1].m_die = vec![target];
        let mut rng = BMC_RNG::default();
        rng.SRand(3);
        ApplyAttack(
            &mut attack_game,
            &BMC_Move::attack(BME_ATTACK::POWER, [0], [0], 0.0),
            &mut rng,
        );
        let mood = attack_game.m_player[0]
            .m_die
            .iter()
            .find(|die| die.m_original_index == 0)
            .unwrap();
        assert_ne!(mood.m_sides[0], 6);
        assert_eq!(mood.GetValueTotal(), 3);
    }

    /// Ports both Konstant Time-and-Space no-extra-turn cases.
    #[test]
    fn pr82_konstant_time_and_space_never_grants_extra_turn() {
        for attack in [BME_ATTACK::TRIP, BME_ATTACK::SKILL] {
            let mut game = BMC_Game::default();
            let mut konstant = swing_die(
                'P',
                property::KONSTANT
                    | property::TIME_AND_SPACE
                    | if attack == BME_ATTACK::TRIP {
                        property::TRIP
                    } else {
                        0
                    },
                0,
            );
            konstant.m_sides[0] = 6;
            konstant.m_value_total = Some(3);
            game.m_player[0].m_die = vec![konstant];
            let mut target = swing_die('P', 0, 0);
            target.m_sides[0] = 1;
            target.m_value_total = Some(1);
            game.m_player[1].m_die = vec![target];
            let action = if attack == BME_ATTACK::TRIP {
                BMC_Move::attack(attack, [0], [0], 0.0)
            } else {
                let mut ordinary = swing_die('P', 0, 1);
                ordinary.m_sides[0] = 2;
                ordinary.m_value_total = Some(2);
                game.m_player[0].m_die.push(ordinary);
                game.m_player[1].m_die[0].m_sides[0] = 5;
                game.m_player[1].m_die[0].m_value_total = Some(5);
                BMC_Move::attack(attack, [0, 1], [0], 0.0)
            };
            assert!(!ApplyAttack(&mut game, &action, &mut BMC_RNG::default()));
        }
    }

    /// Ports TimeAndSpaceOddRerollGrantsExtraTurn.
    #[test]
    fn pr82_ordinary_time_and_space_uses_its_rerolled_value() {
        let mut game = BMC_Game::default();
        let mut attacker = swing_die('P', property::TIME_AND_SPACE, 0);
        attacker.m_sides[0] = 6;
        attacker.m_value_total = Some(1);
        let mut target = swing_die('P', 0, 0);
        target.m_sides[0] = 1;
        target.m_value_total = Some(1);
        game.m_player[0].m_die = vec![attacker];
        game.m_player[1].m_die = vec![target];
        let mut rng = BMC_RNG::default();
        rng.SRand(3);
        let extra_turn = ApplyAttack(
            &mut game,
            &BMC_Move::attack(BME_ATTACK::POWER, [0], [0], 0.0),
            &mut rng,
        );
        assert_eq!(game.m_player[0].m_die[0].GetValueTotal() % 2, 1);
        assert!(extra_turn);
    }

    /// Ports the Konstant Morphing and Berserk side-change regressions.
    #[test]
    fn pr82_konstant_attack_side_changes_preserve_value() {
        let mut morph_game = BMC_Game::default();
        let mut morph = swing_die('P', property::MORPHING | property::KONSTANT, 0);
        morph.m_sides[0] = 9;
        morph.m_value_total = Some(6);
        let mut ordinary = swing_die('P', 0, 1);
        ordinary.m_sides[0] = 1;
        ordinary.m_value_total = Some(1);
        let mut target = swing_die('P', 0, 0);
        target.m_sides[0] = 7;
        target.m_value_total = Some(7);
        morph_game.m_player[0].m_die = vec![morph, ordinary];
        morph_game.m_player[1].m_die = vec![target];
        ApplyAttack(
            &mut morph_game,
            &BMC_Move::attack(BME_ATTACK::SKILL, [0, 1], [0], 0.0),
            &mut BMC_RNG::default(),
        );
        let morph = morph_game.m_player[0]
            .m_die
            .iter()
            .find(|die| die.m_original_index == 0)
            .unwrap();
        assert_eq!((morph.m_sides[0], morph.GetValueTotal()), (7, 6));

        let mut berserk_game = BMC_Game::default();
        let mut berserk = swing_die('P', property::BERSERK | property::KONSTANT, 0);
        berserk.m_sides[0] = 9;
        berserk.m_value_total = Some(8);
        let mut first = swing_die('P', 0, 0);
        first.m_sides[0] = 3;
        first.m_value_total = Some(3);
        let mut second = swing_die('P', 0, 1);
        second.m_sides[0] = 5;
        second.m_value_total = Some(5);
        berserk_game.m_player[0].m_die = vec![berserk];
        berserk_game.m_player[1].m_die = vec![first, second];
        ApplyAttack(
            &mut berserk_game,
            &BMC_Move::attack(BME_ATTACK::BERSERK, [0], [0, 1], 0.0),
            &mut BMC_RNG::default(),
        );
        let berserk = &berserk_game.m_player[0].m_die[0];
        assert_eq!((berserk.m_sides[0], berserk.GetValueTotal()), (5, 8));
        assert!(!berserk.HasProperty(property::BERSERK));
    }
}
