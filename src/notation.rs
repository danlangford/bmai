// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use serde::Serialize;

use crate::model::property;

/// Whether a die property is fully implemented or accepted only for upstream
/// notation compatibility.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CapabilitySupport {
    Implemented,
    ParsingOnly,
}

/// One property prefix in BMAIR's die notation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct DiePropertyNotation {
    pub token: char,
    pub id: &'static str,
    pub name: &'static str,
    pub support: CapabilitySupport,
    #[serde(skip)]
    pub(crate) property: u64,
}

/// One property suffix in BMAIR's die notation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct DiePostfixNotation {
    pub token: char,
    pub id: &'static str,
    pub name: &'static str,
}

/// Discoverable grammar elements used to describe dice on the wire.
#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct DieNotationCapabilities {
    pub property_prefixes: &'static [DiePropertyNotation],
    pub postfix_properties: &'static [DiePostfixNotation],
    pub swing_types: &'static str,
    pub option_separator: char,
    pub twin_open: char,
    pub twin_separator: char,
    pub twin_close: char,
    pub defined_side_separator: char,
    pub rolled_value_separator: char,
    pub dizzy_value_suffix: char,
}

macro_rules! die_property {
    ($token:literal, $id:literal, $name:literal, $support:ident, $property:ident) => {
        DiePropertyNotation {
            token: $token,
            id: $id,
            name: $name,
            support: CapabilitySupport::$support,
            property: property::$property,
        }
    };
}

/// The parser and capability response deliberately share this table. Adding or
/// changing a prefix therefore changes accepted syntax and discovery together.
pub(crate) const DIE_PROPERTY_PREFIXES: &[DiePropertyNotation] = &[
    die_property!(
        '^',
        "time_and_space",
        "TimeAndSpace",
        Implemented,
        TIME_AND_SPACE
    ),
    die_property!('q', "queer", "Queer", Implemented, QUEER),
    die_property!('t', "trip", "Trip", Implemented, TRIP),
    die_property!('z', "speed", "Speed", Implemented, SPEED),
    die_property!('s', "shadow", "Shadow", Implemented, SHADOW),
    die_property!('B', "berserk", "Berserk", Implemented, BERSERK),
    die_property!('d', "stealth", "Stealth", Implemented, STEALTH),
    die_property!('p', "poison", "Poison", Implemented, POISON),
    die_property!('n', "null", "Null", Implemented, NULL),
    die_property!('f', "focus", "Focus", Implemented, FOCUS),
    die_property!('H', "mighty", "Mighty", Implemented, MIGHTY),
    die_property!('h', "weak", "Weak", Implemented, WEAK),
    die_property!('r', "reserve", "Reserve", Implemented, RESERVE),
    die_property!('o', "ornery", "Ornery", Implemented, ORNERY),
    die_property!('c', "chance", "Chance", Implemented, CHANCE),
    die_property!('m', "morphing", "Morphing", Implemented, MORPHING),
    die_property!('`', "warrior", "Warrior", Implemented, WARRIOR),
    die_property!('w', "slow", "Slow", Implemented, SLOW),
    die_property!('u', "unique", "Unique", Implemented, UNIQUE),
    die_property!('~', "unskilled", "Unskilled", Implemented, UNSKILLED),
    die_property!('g', "stinger", "Stinger", Implemented, STINGER),
    die_property!('k', "konstant", "Konstant", Implemented, KONSTANT),
    die_property!('M', "maximum", "Maximum", Implemented, MAXIMUM),
    die_property!('I', "insult", "Insult", Implemented, INSULT),
    die_property!('v', "value", "Value", Implemented, VALUE),
    die_property!('J', "jolt", "Jolt", Implemented, JOLT),
    die_property!('+', "auxiliary", "Auxiliary", ParsingOnly, AUXILIARY),
    die_property!(
        'D',
        "doppelganger",
        "Doppelganger",
        Implemented,
        DOPPELGANGER
    ),
    die_property!('%', "radioactive", "Radioactive", ParsingOnly, RADIOACTIVE),
    die_property!('G', "rage", "Rage", Implemented, RAGE),
];

const DIE_POSTFIX_PROPERTIES: &[DiePostfixNotation] = &[
    DiePostfixNotation {
        token: '!',
        id: "turbo",
        name: "Turbo",
    },
    DiePostfixNotation {
        token: '?',
        id: "mood",
        name: "Mood",
    },
];

impl DieNotationCapabilities {
    pub const fn current() -> Self {
        Self {
            property_prefixes: DIE_PROPERTY_PREFIXES,
            postfix_properties: DIE_POSTFIX_PROPERTIES,
            swing_types: "P-Z",
            option_separator: '/',
            twin_open: '(',
            twin_separator: ',',
            twin_close: ')',
            defined_side_separator: '-',
            rolled_value_separator: ':',
            dizzy_value_suffix: 'd',
        }
    }
}
