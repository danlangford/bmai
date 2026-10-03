// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

pub(super) fn parse_die(input: &str, original_index: usize) -> Result<Die, ParseError> {
    let (definition, value_part) = input
        .split_once(':')
        .map_or((input, None), |(a, b)| (a, Some(b)));
    let mut properties = property::VALID;
    let mut pos = 0;
    let chars: Vec<char> = definition.chars().collect();
    while pos < chars.len()
        && !chars[pos].is_ascii_digit()
        && !(('P'..='Z').contains(&chars[pos]))
        && chars[pos] != '('
    {
        properties |= prefix_property(chars[pos])
            .ok_or_else(|| ParseError(format!("error parsing die {input} at {}", chars[pos])))?;
        pos += 1;
    }
    let mut sides = [0u8; 2];
    let mut swings = [None; 2];
    if chars.get(pos) == Some(&'(') {
        properties |= property::TWIN;
        pos += 1;
        (sides[0], swings[0], pos) = parse_side(&chars, pos)?;
        if chars.get(pos) != Some(&',') {
            return Err(ParseError(format!("invalid twin die: {input}")));
        }
        pos += 1;
        (sides[1], swings[1], pos) = parse_side(&chars, pos)?;
        if chars.get(pos) != Some(&')') {
            return Err(ParseError(format!("invalid twin die: {input}")));
        }
        pos += 1;
    } else {
        (sides[0], swings[0], pos) = parse_side(&chars, pos)?;
        if chars.get(pos) == Some(&'/') {
            properties |= property::OPTION;
            pos += 1;
            (sides[1], swings[1], pos) = parse_side(&chars, pos)?;
        }
    }
    while pos < chars.len() {
        match chars[pos] {
            '!' => properties |= property::TURBO,
            '?' => properties |= property::MOOD,
            '&' => properties |= property::MAD,
            '-' => {
                pos += 1;
                while pos < chars.len() && chars[pos].is_ascii_digit() {
                    pos += 1;
                }
                continue;
            }
            ch => return Err(ParseError(format!("error parsing die {input} at {ch}"))),
        }
        pos += 1;
    }
    // The shared `-N` applies to every swing half: `(T,T)-2` is two d2s.
    if let Some(value) = parse_die_defined_sides(definition).filter(|value| *value > 0) {
        for side in 0..2 {
            if swings[side].is_some() {
                sides[side] = value;
            }
        }
        if properties & property::OPTION != 0 && sides[1] == value {
            sides.swap(0, 1);
            swings.swap(0, 1);
        }
    }
    let value = value_part
        .map(|v| {
            v.trim_end_matches('d')
                .parse::<u8>()
                .map_err(|_| ParseError(format!("invalid die value: {input}")))
        })
        .transpose()?;
    Ok(Die {
        properties,
        sides,
        swing_type: swings,
        value,
        captured: false,
        not_set: value.is_none() && properties & property::RESERVE == 0,
        dizzy: value_part.is_some_and(|value| value.ends_with('d')),
        original_index,
        in_reserve: properties & property::RESERVE != 0,
    })
}

/// Postfix properties may appear on either side of the `-N` suffix.
pub(super) fn parse_die_defined_sides(definition: &str) -> Option<u8> {
    let (_, suffix) = definition.split_once('-')?;
    let digit_count = suffix.bytes().take_while(u8::is_ascii_digit).count();
    suffix.get(..digit_count)?.parse().ok()
}

pub(super) fn parse_side(
    chars: &[char],
    mut pos: usize,
) -> Result<(u8, Option<char>, usize), ParseError> {
    if let Some(ch @ 'P'..='Z') = chars.get(pos).copied() {
        return Ok((0, Some(ch), pos + 1));
    }
    let start = pos;
    while pos < chars.len() && chars[pos].is_ascii_digit() {
        pos += 1;
    }
    if start == pos {
        return Err(ParseError("expected die sides".into()));
    }
    let value = chars[start..pos]
        .iter()
        .collect::<String>()
        .parse()
        .map_err(|_| ParseError("invalid die sides".into()))?;
    Ok((value, None, pos))
}

pub(super) fn prefix_property(ch: char) -> Option<u64> {
    crate::protocol::notation::DIE_PROPERTY_PREFIXES
        .iter()
        .find(|notation| notation.token == ch)
        .map(|notation| notation.property)
}
