// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

pub(super) fn DebugPlayer<W: Write>(
    player: &crate::game::Player,
    all: bool,
    output: &mut W,
) -> Result<(), ParseError> {
    write!(output, "p{} s{:.1} Dice ", player.m_id, player.m_score).map_err(io_error)?;
    for die in &player.m_die {
        if !all && !die.IsAvailable() {
            continue;
        }
        write!(output, "({:x})", die.m_properties & !property::VALID).map_err(io_error)?;
        if die.HasProperty(property::TWIN) {
            write!(output, "({},{})", die.m_sides[0], die.m_sides[1]).map_err(io_error)?;
        } else {
            write!(output, "{}", die.m_sides[0]).map_err(io_error)?;
        }
        if let Some(value) = die.m_value_total {
            write!(output, ":{value} ").map_err(io_error)?;
        } else {
            write!(output, " ").map_err(io_error)?;
        }
    }
    writeln!(output).map_err(io_error)
}

pub(super) fn SendAttack<W: Write>(
    game: &Game,
    action: &Move,
    output: &mut W,
) -> Result<(), ParseError> {
    match action.m_action {
        Action::Pass => writeln!(output, "pass").map_err(io_error),
        Action::Surrender => writeln!(output, "surrender").map_err(io_error),
        Action::Attack => {
            writeln!(
                output,
                "{}",
                action.m_attack.expect("attack kind").protocol()
            )
            .map_err(io_error)?;
            write_indices(&game.m_player[0], &action.m_attackers, output)?;
            write_indices(&game.m_player[1], &action.m_targets, output)?;
            if action.m_turbo_option >= 0
                && let Some(die) = game.m_player[0]
                    .m_die
                    .iter()
                    .find(|die| die.IsAvailable() && die.HasProperty(property::TURBO))
            {
                if die.HasProperty(property::OPTION) {
                    writeln!(
                        output,
                        "option {} {}",
                        die.m_original_index, die.m_sides[action.m_turbo_option as usize]
                    )
                    .map_err(io_error)?;
                } else if let Some(swing) = die.m_swing_type[0] {
                    writeln!(output, "swing {swing} {}", action.m_turbo_option)
                        .map_err(io_error)?;
                }
            }
            for index in action
                .m_fire
                .m_amounts
                .iter()
                .enumerate()
                .filter_map(|(index, amount)| {
                    (!action.m_attackers.contains(index) && *amount > 0).then_some((index, *amount))
                })
            {
                let (index, reduction) = index;
                let die = &game.m_player[0].m_die[index];
                writeln!(
                    output,
                    "fire {} {}",
                    die.m_original_index,
                    die.GetValueTotal() - u16::from(reduction)
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
    match action.m_action {
        Action::Pass => Ok(crate::protocol::ProtocolAction::Pass),
        Action::Surrender => Ok(crate::protocol::ProtocolAction::Surrender),
        Action::Attack => {
            let attack_type = action
                .m_attack
                .ok_or_else(|| ParseError("attack has no attack type".into()))?
                .protocol();
            let original_indices = |player: usize, indices: &DieIndexSet| {
                indices
                    .iter()
                    .map(|index| game.m_player[player].m_die[index].m_original_index)
                    .collect::<Vec<_>>()
            };
            let turbo = if action.m_turbo_option < 0 {
                None
            } else {
                game.m_player[0]
                    .m_die
                    .iter()
                    .find(|die| die.IsAvailable() && die.HasProperty(property::TURBO))
                    .and_then(|die| {
                        if die.HasProperty(property::OPTION) {
                            Some(crate::protocol::TurboSelection::Option {
                                die: die.m_original_index,
                                value: die.m_sides[action.m_turbo_option as usize],
                            })
                        } else {
                            die.m_swing_type[0].map(|swing| {
                                crate::protocol::TurboSelection::Swing {
                                    swing,
                                    value: action.m_turbo_option as u8,
                                }
                            })
                        }
                    })
            };
            let fire = action
                .m_fire
                .m_amounts
                .iter()
                .enumerate()
                .filter(|(index, amount)| !action.m_attackers.contains(*index) && **amount > 0)
                .map(|(index, reduction)| {
                    let die = &game.m_player[0].m_die[index];
                    crate::protocol::FireSelection {
                        die: die.m_original_index,
                        value: (die.GetValueTotal() - u16::from(*reduction)) as u8,
                    }
                })
                .collect();
            Ok(crate::protocol::ProtocolAction::Attack {
                attack_type,
                attackers: original_indices(0, &action.m_attackers),
                targets: original_indices(1, &action.m_targets),
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
            let die = &game.m_player[0].m_die[*index];
            crate::protocol::OptionSelection {
                die: die.m_original_index,
                value: die.m_sides[usize::from(*second)],
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
                .map(|index| game.m_player[0].m_die[*index].m_original_index)
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
                    die: game.m_player[0].m_die[*index].m_original_index,
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
        write!(output, "{}", player.m_die[index].m_original_index).map_err(io_error)?;
    }
    writeln!(output).map_err(io_error)
}

pub(super) fn SendSetSwing<W: Write>(
    game: &Game,
    action: &SwingMove,
    output: &mut W,
) -> Result<(), ParseError> {
    let player = &game.m_player[0];
    let mut sent = false;
    for (swing, value) in action.values() {
        writeln!(output, "swing {swing} {value}").map_err(io_error)?;
        sent = true;
    }
    for (index, second) in action.options() {
        let die = &player.m_die[*index];
        let selected = die.m_sides[usize::from(*second)];
        writeln!(output, "option {} {selected}", die.m_original_index).map_err(io_error)?;
        sent = true;
    }
    if !sent {
        writeln!(output, "pass").map_err(io_error)?;
    }
    Ok(())
}
