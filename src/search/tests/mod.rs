// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;
use crate::Action::Attack;
use crate::Attack::{Berserk, Power, Skill, Speed, Trip};
use crate::Phase::Fight;
use crate::RngAlgorithm::LegacyParkMillerV1;
use crate::engines::{Choice, DecisionContext, Engine, Quick};
use crate::game::{Die, Player};
use std::sync::{Arc, Mutex};
use test_support::{initiative_scenario, roll, scenario};

#[test]
fn native_fight_score_summary_is_stable() {
    let input = include_str!("../../../tests/native-fixtures/fight.txt");
    let setup = input.split_once("getaction").unwrap().0;
    let mut parser = crate::Parser::default();
    parser.parse_string(setup, &mut Vec::new()).unwrap();
    let settings = Bmai3 {
        min_sims: 20,
        max_sims: 20,
        max_branch: 100,
        ..Default::default()
    };
    let replay = crate::native::NativeReplayKey {
        root_seed: 17,
        decision_index: 0,
    };

    let available = std::thread::available_parallelism().map_or(1, usize::from);
    let mut expected: Option<(Move, f32)> = None;
    for workers in [1, 2, available] {
        let native = NativeEvaluation {
            algorithm: LegacyParkMillerV1,
            replay,
            workers,
        };
        let search = select_native_bmai_action(&parser.game, native, &settings);
        let probability = search.win_probability();
        let result = (search.best_move, probability);
        if let Some(expected) = &expected {
            assert_eq!(result.0.action, expected.0.action);
            assert_eq!(result.0.attack, expected.0.attack);
            assert_eq!(result.0.attackers, expected.0.attackers);
            assert_eq!(result.0.targets, expected.0.targets);
            assert_eq!(result.0.score, expected.0.score);
            assert_eq!(result.0.turbo_option, expected.0.turbo_option);
            assert_eq!(result.1, expected.1);
        } else {
            expected = Some(result);
        }
    }

    let (action, probability) = expected.unwrap();
    assert_eq!(action.action, Attack);
    assert_eq!(action.attack, Some(Power));
    assert_eq!(action.attackers, vec![0]);
    assert_eq!(action.targets, vec![1]);
    assert_eq!(probability, 0.0);
}

#[test]
fn native_initiative_phase_scores_are_worker_count_independent() {
    let replay = crate::native::NativeReplayKey {
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
        min_sims: 1,
        max_sims: 1,
        max_branch: 20,
        ..Default::default()
    };
    let swing = contexts.map(|context| {
        let mut rng = Rng::default();
        let (action, score) = select_swing_action(&game, 0, &mut rng, &settings, 1, Some(context));
        (action.values().to_vec(), action.options().to_vec(), score)
    });
    assert_eq!(swing[1], swing[0]);
    assert_eq!(swing[2], swing[0]);

    let game = native_fixture_game(include_str!("../../../tests/native-fixtures/chance.txt"));
    let settings = Bmai3 {
        min_sims: 1,
        max_sims: 2,
        max_branch: 10,
        ..Default::default()
    };
    let chance = contexts.map(|context| {
        let mut rng = Rng::default();
        let (action, score) =
            select_chance_action(&game, 0, &mut rng, &settings, 1, 1, Some(context));
        (action.reroll, score)
    });
    assert_eq!(chance[1], chance[0]);
    assert_eq!(chance[2], chance[0]);

    let game = native_fixture_game(include_str!("../../../tests/native-fixtures/focus.txt"));
    let settings = Bmai3 {
        min_sims: 1,
        max_sims: 2,
        max_branch: 40,
        ..Default::default()
    };
    let focus = contexts.map(|context| {
        let mut rng = Rng::default();
        let (action, score) =
            select_focus_action(&game, 0, &mut rng, &settings, 1, 1, Some(context));
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
        min_sims: 1,
        max_sims: 1,
        max_branch: 10,
        ..Default::default()
    };
    let policies: Engines = [
        Box::new(crate::engines::MonteCarlo::new(ai.clone())),
        Box::new(crate::engines::MonteCarlo::new(ai)),
    ];
    let run = |workers| {
        let mut rng = Rng::default();
        rng.reseed(17);
        let mut decision_index = 0;
        let mut native = NativeReplaySequence {
            algorithm: rng.algorithm(),
            root_seed: 17,
            workers,
            decision_index: &mut decision_index,
        };
        let result = play_match_with_policies(&game, &mut rng, &policies, &mut native);
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
    game.players[0].score = 12.0;
    game.players[1].score = 12.0;
    assert_eq!(round_winner(&game), None);
}

#[test]
fn a_cancelled_match_plays_its_200th_round() {
    let game = null_mirror();
    let quick = || -> Box<dyn crate::engines::Engine> { Box::new(crate::engines::Quick) };
    let policies: Engines = [quick(), quick()];
    let mut rng = Rng::default();
    let result = play_match_with_policies(&game, &mut rng, &policies, &mut unsearched(&mut 0));
    assert_eq!((result.winner, result.ties), (None, 199));

    // Only the generator's position shows that round 200 was played.
    let mut replay = Rng::default();
    let mut replayed = game.clone();
    let mut next_draw_after = |rounds| {
        for _ in 0..rounds {
            let round = play_round_with_policies(&mut replayed, &mut replay, &policies, None);
            restore_dice_for_new_round(&mut replayed, &game, &round.selections);
        }
        replay.clone().rand()
    };
    let after_199 = next_draw_after(199);
    let after_200 = next_draw_after(1);
    assert_ne!(after_199, after_200);
    assert_eq!(rng.rand(), after_200);
}

#[test]
fn play_games_writes_each_game_to_its_output() {
    let quick = || -> Box<dyn crate::engines::Engine> { Box::new(crate::engines::Quick) };
    let mut output = Vec::new();
    let wins = play_games_with_policies(
        &null_mirror(),
        2,
        &mut Rng::default(),
        &[quick(), quick()],
        &mut unsearched(&mut 0),
        &mut output,
    )
    .unwrap();
    assert_eq!(wins, [0, 0]);
    assert_eq!(
        String::from_utf8(output).unwrap(),
        "game cancelled 0 - 0 - 199\n".repeat(2)
    );
}

/// For engines that never search, so no replay key is ever drawn from it.
fn unsearched(decision_index: &mut u64) -> NativeReplaySequence<'_> {
    NativeReplaySequence {
        algorithm: LegacyParkMillerV1,
        root_seed: 0,
        workers: 1,
        decision_index,
    }
}

/// Null dice score nothing, so every round ties.
fn null_mirror() -> Game {
    native_fixture_game("game 1\npreround\nplayer 0 2 0\nn4\nn4\nplayer 1 2 0\nn4\nn4\n")
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
    native_fixture_game(&input).generate_valid_attacks_in_cpp_order()
}

fn native_fixture_game(input: &str) -> Game {
    let setup = input.split_once("getaction").map_or(input, |parts| parts.0);
    let mut parser = crate::Parser::default();
    parser.parse_string(setup, &mut Vec::new()).unwrap();
    parser.game
}

fn apply_generated_attack(game: &mut Game, action: &Move, rng: &mut Rng) -> bool {
    assert!(
        game.generate_valid_attacks_in_cpp_order()
            .iter()
            .any(|candidate| {
                candidate.attack == action.attack
                    && candidate.attackers == action.attackers
                    && candidate.targets == action.targets
            })
    );
    apply_attack(game, action, rng)
}

fn swing_die(swing: char, properties: u64, original_index: usize) -> Die {
    Die {
        properties: property::VALID | properties,
        sides: [0, 0],
        swing_type: [Some(swing), None],
        value: None,
        captured: false,
        not_set: false,
        dizzy: false,
        original_index,
        in_reserve: false,
    }
}

#[derive(Clone, Debug, Default)]
struct QuickSpy {
    attacks: Arc<Mutex<Vec<Game>>>,
    reserves: Arc<Mutex<Vec<Game>>>,
    auxiliaries: Arc<Mutex<Vec<Game>>>,
    declines_auxiliary: bool,
}

impl Engine for QuickSpy {
    fn name(&self) -> &'static str {
        "quick spy"
    }

    fn clone_box(&self) -> Box<dyn Engine> {
        Box::new(self.clone())
    }

    fn swing(
        &self,
        game: &Game,
        player: usize,
        context: &mut DecisionContext<'_, '_>,
    ) -> SwingMove {
        Quick.swing(game, player, context)
    }

    fn chance(
        &self,
        game: &Game,
        player: usize,
        initiative: usize,
        context: &mut DecisionContext<'_, '_>,
    ) -> ChanceMove {
        Quick.chance(game, player, initiative, context)
    }

    fn focus(
        &self,
        game: &Game,
        player: usize,
        initiative: usize,
        context: &mut DecisionContext<'_, '_>,
    ) -> FocusMove {
        Quick.focus(game, player, initiative, context)
    }

    fn attack(&self, game: &Game, context: &mut DecisionContext<'_, '_>) -> Choice<Move> {
        self.attacks.lock().unwrap().push(game.clone());
        Quick.attack(game, context)
    }

    fn reserve(&self, game: &Game, context: &mut DecisionContext<'_, '_>) -> Option<usize> {
        self.reserves.lock().unwrap().push(game.clone());
        Quick.reserve(game, context)
    }

    fn auxiliary(
        &self,
        game: &Game,
        context: &mut DecisionContext<'_, '_>,
    ) -> Choice<Option<usize>> {
        self.auxiliaries.lock().unwrap().push(game.clone());
        if self.declines_auxiliary {
            Choice::unsearched(None)
        } else {
            Quick.auxiliary(game, context)
        }
    }
}

mod auxiliary;
mod berserk;
mod boom;
mod core;
mod doppelganger;
mod endgame;
mod jolt;
mod mad;
mod mighty;
mod mood;
mod morphing;
mod null;
mod parity;
mod radioactive;
mod rage;
mod reserve;
mod rush;
mod specials;
mod transformations;
mod turbo;
mod value;
mod warrior;
mod weak;
