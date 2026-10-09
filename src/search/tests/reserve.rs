// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[test]
fn a_reserve_die_added_after_a_loss_stays_in_play_in_later_rounds() {
    let template =
        native_fixture_game("game\npreround\nplayer 0 3 0\n6\nr8\nr10\nplayer 1 1 0\n4\n");
    let mut game = template.clone();
    let selections = record_round_selections(&game);
    apply_use_reserve(&mut game.players[0].dice[1]);

    for _ in 0..2 {
        restore_dice_for_new_round(&mut game, &template, &selections);
        let [_, added, held] = &game.players[0].dice[..] else {
            panic!("the reserve dice changed count");
        };
        assert!(!added.in_reserve && !added.has_property(property::RESERVE) && added.not_set);
        assert!(held.in_reserve && held.has_property(property::RESERVE));
    }
}

fn as_dealt(player: &Player) -> Vec<(usize, [u8; 2], u64)> {
    let mut dice = player
        .dice
        .iter()
        .map(|die| {
            let properties = die.properties & !property::RESERVE;
            (die.original_index, die.sides, properties)
        })
        .collect::<Vec<_>>();
    dice.sort_unstable();
    dice
}

#[test]
fn the_round_loser_picks_a_reserve_die_from_freshly_dealt_dice() {
    let recipe = native_fixture_game(
        "game 3\npreround\nplayer 0 3 0\n%10\nB20\nr4\nplayer 1 3 0\n%10\nB20\nr4\n",
    );
    let spy = QuickSpy::default();
    let policies: Engines = [Box::new(spy.clone()), Box::new(spy.clone())];
    let mut rng = Rng::default();
    for _ in 0..20 {
        play_match_with_policies(&recipe, &mut rng, &policies, None);
    }

    let seen = spy.reserves.lock().unwrap();
    assert!(!seen.is_empty(), "no round loser had a reserve die to pick");
    let dealt = as_dealt(&recipe.players[0]);
    for position in seen.iter() {
        for player in &position.players {
            assert_eq!(as_dealt(player), dealt);
        }
    }
}
