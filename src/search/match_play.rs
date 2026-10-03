// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

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

pub(super) fn PlayGamesWithPoliciesInternal(
    template: &BMC_Game,
    games: usize,
    rng: &mut BMC_RNG,
    policies: &[BMC_AI_POLICY; 2],
    mut native: Option<&mut NativeReplaySequence<'_>>,
) -> [usize; 2] {
    let mut matches = [0, 0];
    for _ in 0..games {
        let result = PlayMatchWithPolicies(template, rng, policies, native.as_deref_mut());
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

pub(super) fn PlayMatchWithPolicies(
    template: &BMC_Game,
    rng: &mut BMC_RNG,
    policies: &[BMC_AI_POLICY; 2],
    mut native: Option<&mut NativeReplaySequence<'_>>,
) -> MatchResult {
    let mut game = template.clone();
    let mut wins = [0u8, 0u8];
    let mut ties = 0usize;
    let mut initiative = 0;
    let mut reserves_used = 0;
    while wins[0] < template.m_target_wins && wins[1] < template.m_target_wins {
        RestoreDiceForNewRound(&mut game, template);
        let round = PlayRoundWithPolicies(&mut game, rng, policies, native.as_deref_mut());
        let Some(winner) = round.0 else {
            ties += 1;
            continue;
        };
        initiative = round.1;
        wins[winner] += 1;
        let loser = 1 - winner;
        game.m_player[loser].m_swing_set = BME_SWING_SET::NOT;
        for die in &mut game.m_player[loser].m_die {
            if die.m_swing_type.iter().any(Option::is_some) {
                die.m_notset = true;
            }
        }
        if wins[0] < template.m_target_wins
            && wins[1] < template.m_target_wins
            && game.m_player[loser]
                .m_die
                .iter()
                .any(|die| die.m_in_reserve)
        {
            let mut oriented = game.clone();
            if loser == 1 {
                oriented.m_player.swap(0, 1);
            }
            let selected = match &policies[loser] {
                BMC_AI_POLICY::BMAI(ai) => {
                    if let Some(context) = native.as_deref_mut().map(NativeReplaySequence::next) {
                        SelectNativeBMAIReserveAction(
                            &oriented,
                            context.algorithm,
                            context.replay,
                            context.workers,
                            ai,
                        )
                    } else {
                        SelectBMAIReserveAction(&oriented, rng, ai)
                    }
                }
                BMC_AI_POLICY::QAI | BMC_AI_POLICY::RANDOM | BMC_AI_POLICY::MAXIMIZE => {
                    SelectQAIReserveAction(&oriented)
                }
            };
            if let Some(index) = selected {
                ApplyUseReserve(&mut game.m_player[loser].m_die[index]);
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

pub(super) fn PlayFairGamesInternal(
    template: &BMC_Game,
    games: usize,
    rng: &mut BMC_RNG,
    policies: &[BMC_AI_POLICY; 2],
    mut native: Option<&mut NativeReplaySequence<'_>>,
) -> [[usize; 2]; 2] {
    let mut wins = [[0usize; 2]; 2];
    for _ in 0..games {
        let result = PlayMatchWithPolicies(template, rng, policies, native.as_deref_mut());
        wins[result.initiative_winner][result.winner] += 1;
    }
    wins
}

pub(super) fn PlayRoundWithPolicies(
    game: &mut BMC_Game,
    rng: &mut BMC_RNG,
    policies: &[BMC_AI_POLICY; 2],
    mut native: Option<&mut NativeReplaySequence<'_>>,
) -> (Option<usize>, usize) {
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
            BMC_AI_POLICY::RANDOM => {
                SelectRandomAction(&oriented, rng, BMC_BMAI3::default().FireCandidateLimit())
            }
            BMC_AI_POLICY::MAXIMIZE => {
                SelectMaximizeAction(&oriented, rng, BMC_BMAI3::default().FireCandidateLimit())
            }
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
    (RoundWinner(game), initiative_winner)
}

pub(super) fn RoundWinner(game: &BMC_Game) -> Option<usize> {
    match game.m_player[1]
        .m_score
        .partial_cmp(&game.m_player[0].m_score)
    {
        Some(std::cmp::Ordering::Greater) => Some(1),
        Some(std::cmp::Ordering::Less) => Some(0),
        Some(std::cmp::Ordering::Equal) => None,
        None => panic!("round score is NaN"),
    }
}

pub(super) fn PlayPreroundWithPolicies(
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

pub(super) fn PlayPreround(
    game: &mut BMC_Game,
    rng: &mut BMC_RNG,
    ai: &BMC_BMAI3,
    level: usize,
) -> usize {
    // C++ never restores the level after a simulated preround, so the next
    // player's choice starts a ply deeper.
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
