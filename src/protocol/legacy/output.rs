// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

pub(super) fn debug_player<W: Write>(
    player: &crate::game::Player,
    all: bool,
    output: &mut W,
) -> Result<(), ParseError> {
    write!(output, "p{} s{:.1} Dice ", player.id, player.score).map_err(io_error)?;
    for die in &player.dice {
        if !all && !die.is_available() {
            continue;
        }
        write!(output, "({:x})", die.properties & !property::VALID).map_err(io_error)?;
        if die.has_property(property::TWIN) {
            write!(output, "({},{})", die.sides[0], die.sides[1]).map_err(io_error)?;
        } else {
            write!(output, "{}", die.sides[0]).map_err(io_error)?;
        }
        if let Some(value) = die.value {
            write!(output, ":{value} ").map_err(io_error)?;
        } else {
            write!(output, " ").map_err(io_error)?;
        }
    }
    writeln!(output).map_err(io_error)
}

pub(super) fn send_attack<W: Write>(
    game: &Game,
    action: &Move,
    output: &mut W,
) -> Result<(), ParseError> {
    match action.action {
        Action::Pass => writeln!(output, "pass").map_err(io_error),
        Action::Surrender => writeln!(output, "surrender").map_err(io_error),
        Action::Attack => {
            writeln!(output, "{}", action.attack.expect("attack kind").protocol())
                .map_err(io_error)?;
            write_indices(&game.players[0], &action.attackers, output)?;
            write_indices(&game.players[1], &action.targets, output)?;
            if action.turbo_option >= 0
                && let Some(die) = game.players[0]
                    .dice
                    .iter()
                    .find(|die| die.is_available() && die.has_property(property::TURBO))
            {
                if die.has_property(property::OPTION) {
                    writeln!(
                        output,
                        "option {} {}",
                        die.original_index, die.sides[action.turbo_option as usize]
                    )
                    .map_err(io_error)?;
                } else if let Some(swing) = die.swing_type[0] {
                    writeln!(output, "swing {swing} {}", action.turbo_option).map_err(io_error)?;
                }
            }
            for index in action
                .fire
                .amounts
                .iter()
                .enumerate()
                .filter_map(|(index, amount)| {
                    (!action.attackers.contains(index) && *amount > 0).then_some((index, *amount))
                })
            {
                let (index, reduction) = index;
                let die = &game.players[0].dice[index];
                writeln!(
                    output,
                    "fire {} {}",
                    die.original_index,
                    die.value_total() - u16::from(reduction)
                )
                .map_err(io_error)?;
            }
            Ok(())
        }
        _ => Err(ParseError("invalid fight action".into())),
    }
}

pub(super) fn protocol_attack(
    game: &Game,
    action: &Move,
) -> Result<crate::protocol::ProtocolAction, ParseError> {
    match action.action {
        Action::Pass => Ok(crate::protocol::ProtocolAction::Pass),
        Action::Surrender => Ok(crate::protocol::ProtocolAction::Surrender),
        Action::Attack => {
            let attack_type = action
                .attack
                .ok_or_else(|| ParseError("attack has no attack type".into()))?
                .protocol();
            let original_indices = |player: usize, indices: &DieIndexSet| {
                indices
                    .iter()
                    .map(|index| game.players[player].dice[index].original_index)
                    .collect::<Vec<_>>()
            };
            let turbo = if action.turbo_option < 0 {
                None
            } else {
                game.players[0]
                    .dice
                    .iter()
                    .find(|die| die.is_available() && die.has_property(property::TURBO))
                    .and_then(|die| {
                        if die.has_property(property::OPTION) {
                            Some(crate::protocol::TurboSelection::Option {
                                die: die.original_index,
                                value: die.sides[action.turbo_option as usize],
                            })
                        } else {
                            die.swing_type[0].map(|swing| crate::protocol::TurboSelection::Swing {
                                swing,
                                value: action.turbo_option as u8,
                            })
                        }
                    })
            };
            let fire = action
                .fire
                .amounts
                .iter()
                .enumerate()
                .filter(|(index, amount)| !action.attackers.contains(*index) && **amount > 0)
                .map(|(index, reduction)| {
                    let die = &game.players[0].dice[index];
                    crate::protocol::FireSelection {
                        die: die.original_index,
                        value: (die.value_total() - u16::from(*reduction)) as u8,
                    }
                })
                .collect();
            Ok(crate::protocol::ProtocolAction::Attack {
                attack_type,
                attackers: original_indices(0, &action.attackers),
                targets: original_indices(1, &action.targets),
                turbo,
                fire,
            })
        }
        _ => Err(ParseError("invalid fight action".into())),
    }
}

pub(super) fn protocol_swing(game: &Game, action: &SwingMove) -> crate::protocol::ProtocolAction {
    let swings = action
        .values()
        .iter()
        .map(|(swing, value)| crate::protocol::SwingSelection {
            swing: *swing,
            value: *value,
        })
        .collect::<Vec<_>>();
    let options = action
        .options()
        .iter()
        .map(|(index, second)| {
            let die = &game.players[0].dice[*index];
            crate::protocol::OptionSelection {
                die: die.original_index,
                value: die.sides[usize::from(*second)],
            }
        })
        .collect::<Vec<_>>();
    if swings.is_empty() && options.is_empty() {
        crate::protocol::ProtocolAction::Pass
    } else {
        crate::protocol::ProtocolAction::SetSwing { swings, options }
    }
}

pub(super) fn protocol_chance(
    game: &Game,
    action: &crate::search::ChanceMove,
) -> crate::protocol::ProtocolAction {
    if action.reroll.is_empty() {
        crate::protocol::ProtocolAction::Pass
    } else {
        crate::protocol::ProtocolAction::Chance {
            dice: action
                .reroll
                .iter()
                .map(|index| game.players[0].dice[*index].original_index)
                .collect(),
        }
    }
}

pub(super) fn protocol_focus(
    game: &Game,
    action: &crate::search::FocusMove,
) -> crate::protocol::ProtocolAction {
    if action.values.is_empty() {
        crate::protocol::ProtocolAction::Pass
    } else {
        crate::protocol::ProtocolAction::Focus {
            dice: action
                .values
                .iter()
                .map(|(index, value)| crate::protocol::FocusSelection {
                    die: game.players[0].dice[*index].original_index,
                    value: *value,
                })
                .collect(),
        }
    }
}

pub(super) fn phase_protocol(phase: Phase) -> &'static str {
    match phase {
        Phase::Auxiliary => "aux",
        Phase::Preround => "preround",
        Phase::Reserve => "reserve",
        Phase::Initiative => "initiative",
        Phase::Chance => "chance",
        Phase::Focus => "focus",
        Phase::Fight => "fight",
        Phase::Gameover => "gameover",
    }
}

pub(super) fn write_indices<W: Write>(
    player: &crate::game::Player,
    indices: &DieIndexSet,
    output: &mut W,
) -> Result<(), ParseError> {
    for (n, index) in indices.iter().enumerate() {
        if n > 0 {
            write!(output, " ").map_err(io_error)?;
        }
        write!(output, "{}", player.dice[index].original_index).map_err(io_error)?;
    }
    writeln!(output).map_err(io_error)
}

pub(super) fn send_set_swing<W: Write>(
    game: &Game,
    action: &SwingMove,
    output: &mut W,
) -> Result<(), ParseError> {
    let player = &game.players[0];
    let mut sent = false;
    for (swing, value) in action.values() {
        writeln!(output, "swing {swing} {value}").map_err(io_error)?;
        sent = true;
    }
    for (index, second) in action.options() {
        let die = &player.dice[*index];
        let selected = die.sides[usize::from(*second)];
        writeln!(output, "option {} {selected}", die.original_index).map_err(io_error)?;
        sent = true;
    }
    if !sent {
        writeln!(output, "pass").map_err(io_error)?;
    }
    Ok(())
}
