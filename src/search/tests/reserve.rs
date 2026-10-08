// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;
use crate::engines::{Choice, DecisionContext, Engine, Quick};
use std::sync::{Arc, Mutex};

#[test]
fn a_reserve_die_added_after_a_loss_stays_in_play_in_later_rounds() {
    let template =
        native_fixture_game("game\npreround\nplayer 0 3 0\n6\nr8\nr10\nplayer 1 1 0\n4\n");
    let mut game = template.clone();
    apply_use_reserve(&mut game.players[0].dice[1]);

    for _ in 0..2 {
        restore_dice_for_new_round(&mut game, &template);
        let [_, added, held] = &game.players[0].dice[..] else {
            panic!("the reserve dice changed count");
        };
        assert!(!added.in_reserve && !added.has_property(property::RESERVE) && added.not_set);
        assert!(held.in_reserve && held.has_property(property::RESERVE));
    }
}

#[derive(Clone, Debug, Default)]
struct ReserveSpy(Arc<Mutex<Vec<Game>>>);

impl Engine for ReserveSpy {
    fn name(&self) -> &'static str {
        "reserve spy"
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
        Quick.attack(game, context)
    }

    fn reserve(&self, game: &Game, context: &mut DecisionContext<'_, '_>) -> Option<usize> {
        self.0.lock().unwrap().push(game.clone());
        Quick.reserve(game, context)
    }

    fn auxiliary(
        &self,
        game: &Game,
        context: &mut DecisionContext<'_, '_>,
    ) -> Choice<Option<usize>> {
        Quick.auxiliary(game, context)
    }
}

#[test]
fn the_round_loser_picks_reserve_dice_from_the_next_rounds_dice() {
    let game = native_fixture_game(
        "game 3\npreround\nplayer 0 3 0\n12\nr4\nr20\nplayer 1 3 0\n12\nr4\nr20\n",
    );
    let spy = ReserveSpy::default();
    let policies: Engines = [Box::new(spy.clone()), Box::new(spy.clone())];
    let mut rng = Rng::default();
    for _ in 0..10 {
        play_match_with_policies(&game, &mut rng, &policies, None);
    }

    let seen = spy.0.lock().unwrap();
    assert!(!seen.is_empty(), "no round loser had a reserve die to pick");
    for position in seen.iter() {
        for die in position.players.iter().flat_map(|player| &player.dice) {
            assert!(die.value.is_none() && !die.captured, "{die:?}");
        }
    }
}
