// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use serde::Serialize;

use crate::game::{property, special};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CapabilitySupport {
    Implemented,
    ParsingOnly,
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct DiePostfixNotation {
    pub token: char,
    pub id: &'static str,
    pub name: &'static str,
}

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

/// Shared by the parser and capabilities so syntax and discovery cannot drift.
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
    die_property!('F', "fire", "Fire", Implemented, FIRE),
    die_property!('+', "auxiliary", "Auxiliary", Implemented, AUXILIARY),
    die_property!(
        'D',
        "doppelganger",
        "Doppelganger",
        Implemented,
        DOPPELGANGER
    ),
    die_property!('%', "radioactive", "Radioactive", Implemented, RADIOACTIVE),
    die_property!('G', "rage", "Rage", Implemented, RAGE),
    die_property!('#', "rush", "Rush", Implemented, RUSH),
    die_property!('b', "boom", "Boom", Implemented, BOOM),
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
    DiePostfixNotation {
        token: '&',
        id: "mad",
        name: "Mad",
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ButtonSpecialNotation {
    pub id: &'static str,
    pub buttons: &'static [&'static str],
    pub rule: &'static str,
    #[serde(skip)]
    pub(crate) special: u8,
}

pub(crate) const BUTTON_SPECIALS: &[ButtonSpecialNotation] = &[
    ButtonSpecialNotation {
        id: "unique_swing",
        buttons: &["Guillermo", "Oregon"],
        rule: "Different swing types must be assigned unique values.",
        special: special::UNIQUE_SWING,
    },
    ButtonSpecialNotation {
        id: "unique_sizes",
        buttons: &["Gordo"],
        rule: "No two dice may be the same size, and no Auxiliary swing die may be added.",
        special: special::UNIQUE_SIZES,
    },
    ButtonSpecialNotation {
        id: "no_skill_attacks",
        buttons: &["Largo", "The Flying Squirrel"],
        rule: "Cannot perform skill attacks.",
        special: special::NO_SKILL_ATTACKS,
    },
    ButtonSpecialNotation {
        id: "skill_immune",
        buttons: &["The Japanese Beetle"],
        rule: "Cannot be attacked by skill attacks.",
        special: special::SKILL_IMMUNE,
    },
    ButtonSpecialNotation {
        id: "no_initiative",
        buttons: &["Giant"],
        rule: "Cannot win initiative.",
        special: special::NO_INITIATIVE,
    },
];

pub(crate) fn button_special(id: &str) -> Option<u8> {
    BUTTON_SPECIALS
        .iter()
        .find(|special| special.id == id)
        .map(|special| special.special)
}

/// Splits a recipe into BMAIR die tokens, accepting ButtonWeavers notation
/// (`p(12)`, `(X=12)`, `(X)?`) as well as BMAIR's own (`p12`, `X-12`, `X?`).
pub fn recipe_dice(recipe: &str) -> Vec<String> {
    recipe.split_whitespace().map(bmair_die).collect()
}

/// Rewrites one ButtonWeavers die in BMAIR notation; BMAIR dice pass through.
pub fn bmair_die(die: &str) -> String {
    let Some((before, rest)) = die.split_once('(') else {
        return die.replacen('=', "-", 1);
    };
    let Some((inside, after)) = rest.split_once(')') else {
        return die.to_owned();
    };
    // The site sometimes writes postfix skills ahead of the parentheses: p?(X).
    let prefix = before.replace(['!', '?', '&'], "");
    let postfix = before
        .chars()
        .filter(|ch| matches!(ch, '!' | '?' | '&'))
        .collect::<String>();
    if inside.contains(',') {
        return format!("{prefix}({inside}){postfix}{after}");
    }
    let (sides, defined) = inside
        .split_once('=')
        .map_or((inside, None), |(sides, size)| (sides, Some(size)));
    let defined = defined.map(|size| format!("-{size}")).unwrap_or_default();
    format!("{prefix}{sides}{postfix}{after}{defined}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttonweavers_dice_become_bmair_dice() {
        for (site, bmair) in [
            ("(6)", "6"),
            ("p(12)", "p12"),
            ("(X)", "X"),
            ("(X=12)", "X-12"),
            ("(6/30)", "6/30"),
            ("(4,4)", "(4,4)"),
            ("p(8,8)", "p(8,8)"),
            ("(Z,Z)?", "(Z,Z)?"),
            ("(X)?", "X?"),
            ("(X)!", "X!"),
            ("g(10/20)!", "g10/20!"),
            ("p?(X)", "pX?"),
            ("o!(Z)", "oZ!"),
            ("dk(1)", "dk1"),
            ("dmMH(4)", "dmMH4"),
            ("(X=12)!", "X!-12"),
        ] {
            assert_eq!(bmair_die(site), bmair, "{site}");
        }
    }

    #[test]
    fn bmair_dice_pass_through() {
        for die in [
            "dk1", "kV", "X-12", "X!-12", "6/30", "(T,T)-2", "p(8,8)", "cX?-13",
        ] {
            assert_eq!(bmair_die(die), die);
        }
        assert_eq!(bmair_die("X=12"), "X-12");
    }

    #[test]
    fn recipes_split_on_any_whitespace() {
        assert_eq!(
            recipe_dice(" (6)  p(10)\t(X) "),
            ["6", "p10", "X"].map(String::from)
        );
    }
}
