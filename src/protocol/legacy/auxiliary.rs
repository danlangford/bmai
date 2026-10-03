// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

pub(super) fn prepare_auxiliary_phase(game: &mut Game) -> Result<(), ParseError> {
    let auxiliary = game.players.each_ref().map(|player| {
        player
            .dice
            .iter()
            .filter(|die| die.has_property(property::AUXILIARY))
            .count()
    });
    for (player, count) in auxiliary.into_iter().enumerate() {
        if count > 1 {
            return Err(ParseError(format!(
                "player {player} has {count} Auxiliary dice; ButtonWeavers permits one"
            )));
        }
    }
    match auxiliary {
        [1, 0] => add_courtesy_auxiliary(game, 0, 1),
        [0, 1] => add_courtesy_auxiliary(game, 1, 0),
        _ => Ok(()),
    }
}

pub(super) fn add_courtesy_auxiliary(
    game: &mut Game,
    source_player: usize,
    target_player: usize,
) -> Result<(), ParseError> {
    if game.players[target_player].dice.len() >= MAX_DICE {
        return Err(ParseError(format!(
            "courtesy Auxiliary die exceeds player {target_player} capacity {MAX_DICE}"
        )));
    }
    let mut die = *game.players[source_player]
        .dice
        .iter()
        .find(|die| die.has_property(property::AUXILIARY))
        .expect("source player has one Auxiliary die");
    die.original_index = game.players[target_player].dice.len();
    die.value = None;
    die.notset = true;
    game.players[target_player].dice.push(die);
    game.players[target_player].swing_set = SwingSet::Not;
    Ok(())
}
