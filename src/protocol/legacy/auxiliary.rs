// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

pub(super) fn PrepareAuxiliaryPhase(game: &mut BMC_Game) -> Result<(), ParseError> {
    let auxiliary = game.m_player.each_ref().map(|player| {
        player
            .m_die
            .iter()
            .filter(|die| die.HasProperty(property::AUXILIARY))
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
        [1, 0] => AddCourtesyAuxiliary(game, 0, 1),
        [0, 1] => AddCourtesyAuxiliary(game, 1, 0),
        _ => Ok(()),
    }
}

pub(super) fn AddCourtesyAuxiliary(
    game: &mut BMC_Game,
    source_player: usize,
    target_player: usize,
) -> Result<(), ParseError> {
    if game.m_player[target_player].m_die.len() >= BMD_MAX_DICE {
        return Err(ParseError(format!(
            "courtesy Auxiliary die exceeds player {target_player} capacity {BMD_MAX_DICE}"
        )));
    }
    let mut die = *game.m_player[source_player]
        .m_die
        .iter()
        .find(|die| die.HasProperty(property::AUXILIARY))
        .expect("source player has one Auxiliary die");
    die.m_original_index = game.m_player[target_player].m_die.len();
    die.m_value_total = None;
    die.m_notset = true;
    game.m_player[target_player].m_die.push(die);
    game.m_player[target_player].m_swing_set = BME_SWING_SET::NOT;
    Ok(())
}
