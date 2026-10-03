// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;
use crate::Action::{Attack, Pass};
use crate::Attack::{Berserk, Power, Skill, Speed, Trip};
use crate::Phase::Fight;
use crate::RngAlgorithm::LegacyParkMillerV1;
use crate::game::{Die, DieIndexSet, Player};
use test_support::scenario;

fn auxiliary_game() -> Game {
    let input = "game 3\naux\nplayer 0 2 0\n6\n+Y\nplayer 1 2 0\n8\n+p12\nquit\n";
    let mut parser = crate::Parser::default();
    parser.ParseString(input, &mut Vec::new()).unwrap();
    parser.m_game
}

#[test]
fn mutual_auxiliary_acceptance_keeps_each_die_and_removes_the_skill() {
    let mut game = auxiliary_game();

    ApplyAuxiliaryDecision(&mut game, true);

    assert_eq!(game.m_player[0].m_die.len(), 2);
    assert_eq!(game.m_player[1].m_die.len(), 2);
    assert!(game.m_player.iter().all(|player| {
        player
            .m_die
            .iter()
            .all(|die| !die.HasProperty(property::AUXILIARY))
    }));
    assert_eq!(game.m_player[0].m_die[1].m_swing_type[0], Some('Y'));
    assert!(game.m_player[1].m_die[1].HasProperty(property::POISON));
}

#[test]
fn either_auxiliary_decline_removes_both_dice() {
    let mut game = auxiliary_game();

    ApplyAuxiliaryDecision(&mut game, false);

    assert_eq!(game.m_player[0].m_die.len(), 1);
    assert_eq!(game.m_player[1].m_die.len(), 1);
    assert_eq!(game.m_player[0].m_die[0].GetSidesMax(), 6);
    assert_eq!(game.m_player[1].m_die[0].GetSidesMax(), 8);
}

#[test]
fn native_fight_score_summary_is_stable() {
    let input = include_str!("../../../tests/native-fixtures/fight.txt");
    let setup = input.split_once("getaction").unwrap().0;
    let mut parser = crate::Parser::default();
    parser.ParseString(setup, &mut Vec::new()).unwrap();
    let settings = Bmai3 {
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
    let mut expected: Option<(Move, f32)> = None;
    for workers in [1, 2, available] {
        let result = SelectBMAIActionAtLevelNative(
            &parser.m_game,
            LegacyParkMillerV1,
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
    assert_eq!(action.m_action, Attack);
    assert_eq!(action.m_attack, Some(Power));
    assert_eq!(action.m_attackers, vec![0]);
    assert_eq!(action.m_targets, vec![1]);
    assert_eq!(probability, 0.0);
}

#[test]
fn native_initiative_phase_scores_are_worker_count_independent() {
    let replay = crate::native::NativeReplayKey {
        stream_version: crate::native::NativeStreamVersion::V1,
        root_seed: 17,
        decision_index: 0,
    };
    let available = std::thread::available_parallelism().map_or(1, usize::from);
    let contexts = [1, 2, available].map(|workers| NativeEvaluation {
        algorithm: LegacyParkMillerV1,
        replay,
        workers,
    });

    let game = native_fixture_game(include_str!("../../../tests/native-fixtures/preround.txt"));
    let settings = Bmai3 {
        m_min_sims: 1,
        m_max_sims: 1,
        m_max_branch: 20,
        ..Default::default()
    };
    let swing = contexts.map(|context| {
        let mut rng = Rng::UntracedDefault();
        let (action, score) = SelectSwingAction(&game, 0, &mut rng, &settings, 1, Some(context));
        (action.values().to_vec(), action.options().to_vec(), score)
    });
    assert_eq!(swing[1], swing[0]);
    assert_eq!(swing[2], swing[0]);

    let game = native_fixture_game(include_str!("../../../tests/native-fixtures/chance.txt"));
    let settings = Bmai3 {
        m_min_sims: 1,
        m_max_sims: 2,
        m_max_branch: 10,
        ..Default::default()
    };
    let chance = contexts.map(|context| {
        let mut rng = Rng::UntracedDefault();
        let (action, score) =
            SelectChanceAction(&game, 0, &mut rng, &settings, 1, 1, Some(context));
        (action.reroll, score)
    });
    assert_eq!(chance[1], chance[0]);
    assert_eq!(chance[2], chance[0]);

    let game = native_fixture_game(include_str!("../../../tests/native-fixtures/focus.txt"));
    let settings = Bmai3 {
        m_min_sims: 1,
        m_max_sims: 2,
        m_max_branch: 40,
        ..Default::default()
    };
    let focus = contexts.map(|context| {
        let mut rng = Rng::UntracedDefault();
        let (action, score) = SelectFocusAction(&game, 0, &mut rng, &settings, 1, 1, Some(context));
        (action.values, score)
    });
    assert_eq!(focus[1], focus[0]);
    assert_eq!(focus[2], focus[0]);
}

#[test]
fn complete_native_match_uses_reserve_after_a_round_loss() {
    let input = "mode native\nseed 17\ngame 2\npreround\n\
            player 0 2 0\nM1\nr30\nplayer 1 1 0\nM30\n";
    let game = native_fixture_game(input);
    let ai = Bmai3 {
        m_min_sims: 1,
        m_max_sims: 1,
        m_max_branch: 10,
        ..Default::default()
    };
    let policies = [
        AiPolicy::Bmai(Box::new(ai.clone())),
        AiPolicy::Bmai(Box::new(ai)),
    ];
    let run = |workers| {
        let mut rng = Rng::default();
        rng.SRand(17);
        let mut decision_index = 0;
        let mut native = NativeReplaySequence {
            algorithm: rng.Algorithm(),
            root_seed: 17,
            workers,
            decision_index: &mut decision_index,
        };
        let result = PlayMatchWithPolicies(&game, &mut rng, &policies, Some(&mut native));
        (result, decision_index)
    };

    let one = run(1);
    let two = run(2);
    assert_eq!(one, two);
    assert!(
        one.0.reserves_used > 0,
        "the losing player never used its reserve"
    );
}

#[test]
fn tied_round_has_no_loser() {
    let mut game = Game::default();
    game.m_player[0].m_score = 12.0;
    game.m_player[1].m_score = 12.0;
    assert_eq!(RoundWinner(&game), None);
}

fn attacks_by(attacker_dice: &[&str], defender_dice: &[&str]) -> Vec<Move> {
    let mut input = String::from("game\nfight\n");
    for (player, dice) in [attacker_dice, defender_dice].into_iter().enumerate() {
        input.push_str(&format!("player {player} {} 0\n", dice.len()));
        for die in dice {
            input.push_str(die);
            input.push('\n');
        }
    }
    native_fixture_game(&input).GenerateValidAttacksInCppOrder()
}

fn native_fixture_game(input: &str) -> Game {
    let setup = input.split_once("getaction").map_or(input, |parts| parts.0);
    let mut parser = crate::Parser::default();
    parser.ParseString(setup, &mut Vec::new()).unwrap();
    parser.m_game
}

fn apply_generated_attack(game: &mut Game, action: &Move, rng: &mut Rng) -> bool {
    assert!(
        game.GenerateValidAttacksInCppOrder()
            .iter()
            .any(|candidate| {
                candidate.m_attack == action.m_attack
                    && candidate.m_attackers == action.m_attackers
                    && candidate.m_targets == action.m_targets
            })
    );
    ApplyAttack(game, action, rng)
}

fn swing_die(swing: char, properties: u64, original_index: usize) -> Die {
    Die {
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

mod boom_mad;
mod core;
mod doppelganger;
mod jolt;
mod parity;
mod radioactive;
mod rage;
mod rush;
mod specials;
mod transformations;
