// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;
use crate::engines::MonteCarlo;

fn auxiliary_game() -> Game {
    let input = "game 3\naux\nplayer 0 2 0\n6\n+Y\nplayer 1 2 0\n8\n+p12\nquit\n";
    let mut parser = crate::Parser::default();
    parser.parse_string(input, &mut Vec::new()).unwrap();
    parser.game
}

#[test]
fn mutual_auxiliary_acceptance_keeps_each_die_and_removes_the_skill() {
    let mut game = auxiliary_game();

    apply_auxiliary_decision(&mut game, true);

    assert_eq!(game.players[0].dice.len(), 2);
    assert_eq!(game.players[1].dice.len(), 2);
    assert!(game.players.iter().all(|player| {
        player
            .dice
            .iter()
            .all(|die| !die.has_property(property::AUXILIARY))
    }));
    assert_eq!(game.players[0].dice[1].swing_type[0], Some('Y'));
    assert!(game.players[1].dice[1].has_property(property::POISON));
}

#[test]
fn either_auxiliary_decline_removes_both_dice() {
    let mut game = auxiliary_game();

    apply_auxiliary_decision(&mut game, false);

    assert_eq!(game.players[0].dice.len(), 1);
    assert_eq!(game.players[1].dice.len(), 1);
    assert_eq!(game.players[0].dice[0].sides_max(), 6);
    assert_eq!(game.players[1].dice[0].sides_max(), 8);
}

fn play_spied(input: &str, accepts: [bool; 2]) -> (MatchResult, QuickSpy) {
    let game = native_fixture_game(input);
    let spy = QuickSpy::default();
    let policies: Engines = accepts.map(|accepts| -> Box<dyn Engine> {
        Box::new(QuickSpy {
            declines_auxiliary: !accepts,
            ..spy.clone()
        })
    });
    let mut rng = Rng::default();
    rng.reseed(17);
    let result = play_match_with_policies(&game, &mut rng, &policies, None);
    (result, spy)
}

/// Player 1 has no Auxiliary die, so it is offered a copy of player 0's.
const COURTESY_MATCH: &str = "game 3\npreround\nplayer 0 2 0\n4\n+6\nplayer 1 1 0\n8\n";

fn rounds(result: &MatchResult) -> usize {
    usize::from(result.wins[0]) + usize::from(result.wins[1]) + result.ties
}

fn sizes_and_auxiliary(dice: &[Die]) -> Vec<(u16, bool)> {
    let mut dice = dice
        .iter()
        .map(|die| (die.sides_max(), die.has_property(property::AUXILIARY)))
        .collect::<Vec<_>>();
    dice.sort_unstable();
    dice
}

fn dice_shown_in_question_order(spy: &QuickSpy) -> Vec<Vec<(u16, bool)>> {
    spy.auxiliaries
        .lock()
        .unwrap()
        .iter()
        .map(|game| sizes_and_auxiliary(&game.players[0].dice))
        .collect()
}

fn attacked_hands(spy: &QuickSpy) -> Vec<Vec<Die>> {
    spy.attacks
        .lock()
        .unwrap()
        .iter()
        .flat_map(|game| game.players.iter().map(|player| player.dice.clone()))
        .collect()
}

fn has_auxiliary(dice: &[Die]) -> bool {
    dice.iter().any(|die| die.has_property(property::AUXILIARY))
}

#[test]
fn mutually_accepted_auxiliary_dice_play_every_round_without_the_skill() {
    let (result, spy) = play_spied(COURTESY_MATCH, [true, true]);

    assert_eq!(
        dice_shown_in_question_order(&spy),
        [vec![(4, false), (6, true)], vec![(6, true), (8, false)]]
    );
    assert!(rounds(&result) >= 3, "{result:?}");
    for dice in attacked_hands(&spy) {
        assert_eq!(dice.len(), 2);
        assert!(!has_auxiliary(&dice));
    }
}

#[test]
fn an_accepted_auxiliary_die_restored_from_the_recipe_stays_without_the_skill() {
    let (result, spy) = play_spied(
        "game 3\npreround\nplayer 0 2 0\n4\n+%8\nplayer 1 1 0\n4\n",
        [true, true],
    );

    assert!(rounds(&result) >= 3, "{result:?}");
    let hands = attacked_hands(&spy);
    assert!(
        hands.iter().any(|dice| dice.len() > 2),
        "the Radioactive Auxiliary die never decayed"
    );
    assert!(!hands.iter().any(|dice| has_auxiliary(dice)));
}

#[test]
fn a_declined_auxiliary_die_never_plays() {
    let (result, spy) = play_spied(COURTESY_MATCH, [true, false]);

    assert_eq!(dice_shown_in_question_order(&spy).len(), 2);
    assert!(rounds(&result) >= 3, "{result:?}");
    for dice in attacked_hands(&spy) {
        assert_eq!(dice.len(), 1);
        assert_ne!(dice[0].sides_max(), 6);
    }
}

#[test]
fn a_first_decline_ends_the_auxiliary_choice() {
    let (result, spy) = play_spied(COURTESY_MATCH, [false, true]);

    assert_eq!(
        dice_shown_in_question_order(&spy),
        [vec![(4, false), (6, true)]]
    );
    assert!(rounds(&result) >= 3, "{result:?}");
    assert!(attacked_hands(&spy).iter().all(|dice| dice.len() == 1));
}

#[test]
fn buttons_without_auxiliary_dice_skip_the_choice() {
    let (_, spy) = play_spied(
        "game 3\npreround\nplayer 0 1 0\n4\nplayer 1 1 0\n6\n",
        [true, true],
    );

    assert!(dice_shown_in_question_order(&spy).is_empty());
}

type Keys = Vec<(&'static str, Option<u64>)>;

#[derive(Clone, Debug)]
struct KeyRecorder {
    inner: MonteCarlo,
    keys: Arc<Mutex<Keys>>,
}

impl KeyRecorder {
    fn log(&self, decision: &'static str, context: &DecisionContext<'_, '_>) {
        let key = context.issued_replay().map(|key| key.decision_index);
        self.keys.lock().unwrap().push((decision, key));
    }
}

impl Engine for KeyRecorder {
    fn name(&self) -> &'static str {
        "key recorder"
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
        let choice = self.inner.swing(game, player, context);
        self.log("swing", context);
        choice
    }

    fn chance(
        &self,
        game: &Game,
        player: usize,
        initiative: usize,
        context: &mut DecisionContext<'_, '_>,
    ) -> ChanceMove {
        let choice = self.inner.chance(game, player, initiative, context);
        self.log("chance", context);
        choice
    }

    fn focus(
        &self,
        game: &Game,
        player: usize,
        initiative: usize,
        context: &mut DecisionContext<'_, '_>,
    ) -> FocusMove {
        let choice = self.inner.focus(game, player, initiative, context);
        self.log("focus", context);
        choice
    }

    fn attack(&self, game: &Game, context: &mut DecisionContext<'_, '_>) -> Choice<Move> {
        let choice = self.inner.attack(game, context);
        self.log("attack", context);
        choice
    }

    fn reserve(&self, game: &Game, context: &mut DecisionContext<'_, '_>) -> Option<usize> {
        let choice = self.inner.reserve(game, context);
        self.log("reserve", context);
        choice
    }

    fn auxiliary(
        &self,
        game: &Game,
        context: &mut DecisionContext<'_, '_>,
    ) -> Choice<Option<usize>> {
        let choice = self.inner.auxiliary(game, context);
        self.log("auxiliary", context);
        choice
    }
}

#[test]
fn native_match_decides_auxiliary_dice_before_round_one_on_any_worker_count() {
    let game = native_fixture_game(COURTESY_MATCH);
    let ai = Bmai3 {
        min_sims: 2,
        max_sims: 2,
        max_branch: 10,
        // The solver answers without a replay key, which would hide the
        // order of the keys after the Auxiliary choice.
        endgame_dice: 0,
        ..Default::default()
    };
    let run = |workers| {
        let keys = Arc::new(Mutex::new(Vec::new()));
        let policies: Engines = [0, 1].map(|_| -> Box<dyn Engine> {
            Box::new(KeyRecorder {
                inner: MonteCarlo::new(ai.clone()),
                keys: Arc::clone(&keys),
            })
        });
        let mut rng = Rng::default();
        rng.reseed(17);
        let mut decision_index = 0;
        let mut native = NativeReplaySequence {
            algorithm: rng.algorithm(),
            root_seed: 17,
            workers,
            decision_index: &mut decision_index,
        };
        let result = play_match_with_policies(&game, &mut rng, &policies, Some(&mut native));
        let keys = std::mem::take(&mut *keys.lock().unwrap());
        (result, keys, decision_index)
    };

    let one = run(1);
    assert_eq!(run(2), one);
    let (_, keys, decision_index) = one;
    assert_eq!(
        keys[..3],
        [
            ("auxiliary", Some(0)),
            ("auxiliary", Some(1)),
            ("attack", Some(2))
        ],
        "{keys:?}"
    );
    let issued = keys.iter().filter_map(|(_, key)| *key).collect::<Vec<_>>();
    assert_eq!(issued, (0..decision_index).collect::<Vec<_>>());
}
