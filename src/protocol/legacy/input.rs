// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

pub(super) fn read_stream_line<R: BufRead>(input: &mut R) -> Result<Option<String>, ParseError> {
    let mut line = String::new();
    match input.read_line(&mut line).map_err(io_error)? {
        0 => Ok(None),
        _ => Ok(Some(line)),
    }
}

pub(super) fn parse_on_off(command: &str, value: &str) -> Result<bool, ParseError> {
    match value {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(ParseError(format!(
            "invalid {command} setting: {value} (expected on or off)"
        ))),
    }
}

pub(super) fn read_required_stream_line<R: BufRead>(
    input: &mut R,
    missing: &'static str,
) -> Result<String, ParseError> {
    read_stream_line(input)?.ok_or_else(|| ParseError(missing.into()))
}

pub(super) fn parse_phase(phase: &str) -> Result<BME_PHASE, ParseError> {
    match phase {
        "aux" => Ok(BME_PHASE::AUXILIARY),
        "preround" => Ok(BME_PHASE::PREROUND),
        "reserve" => Ok(BME_PHASE::RESERVE),
        "initiative" => Ok(BME_PHASE::INITIATIVE),
        "chance" => Ok(BME_PHASE::CHANCE),
        "focus" => Ok(BME_PHASE::FOCUS),
        "fight" => Ok(BME_PHASE::FIGHT),
        "gameover" => Ok(BME_PHASE::GAMEOVER),
        _ => Err(ParseError("phase not found".into())),
    }
}

pub(super) fn argument(line: &str, command: &str) -> Option<Result<usize, ParseError>> {
    line.strip_prefix(command)
        .and_then(|rest| rest.strip_prefix(' '))
        .map(parse_usize)
}
pub(super) fn two_usize_arguments(
    line: &str,
    command: &str,
) -> Result<Option<(usize, usize)>, ParseError> {
    let Some(rest) = line
        .strip_prefix(command)
        .and_then(|rest| rest.strip_prefix(' '))
    else {
        return Ok(None);
    };
    let values = rest.split_whitespace().collect::<Vec<_>>();
    if values.len() != 2 {
        return Ok(None);
    }
    Ok(Some((parse_usize(values[0])?, parse_usize(values[1])?)))
}
pub(super) fn playfair_arguments(line: &str) -> Result<Option<(usize, usize, f32)>, ParseError> {
    let Some(rest) = line.strip_prefix("playfair ") else {
        return Ok(None);
    };
    let values = rest.split_whitespace().collect::<Vec<_>>();
    if values.len() != 3 {
        return Ok(None);
    }
    let probability = values[2]
        .parse()
        .map_err(|_| ParseError(format!("invalid float: {}", values[2])))?;
    Ok(Some((
        parse_usize(values[0])?,
        parse_usize(values[1])?,
        probability,
    )))
}
pub(super) fn parse_usize(input: &str) -> Result<usize, ParseError> {
    input
        .parse()
        .map_err(|_| ParseError(format!("invalid integer: {input}")))
}
pub(super) fn validate_player_dice_count(count: usize) -> Result<(), ParseError> {
    if count > crate::game::BMD_MAX_INPUT_DICE {
        return Err(ParseError(format!(
            "player dice count {count} exceeds maximum {}",
            crate::game::BMD_MAX_INPUT_DICE
        )));
    }
    Ok(())
}
pub(super) fn available_workers() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
}
pub(super) fn io_error(error: std::io::Error) -> ParseError {
    ParseError(error.to_string())
}
