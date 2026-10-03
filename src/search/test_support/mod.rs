// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! A thin adapter: setup, legality, and resolution all use production code,
//! so scenarios cannot drift from the real rules.

use super::{ApplyAttack, RestoreDiceForNewRound};
use crate::protocol::{FireSelection, OptionSelection, ProtocolAction, SwingSelection};
use crate::{Attack, Die, Game, Move, Parser, Phase, Rng, property};
use std::ops::RangeInclusive;

pub(crate) fn scenario() -> Scenario {
    Scenario::default()
}

pub(crate) fn search_scenario() -> SearchScenario {
    SearchScenario::default()
}

pub(crate) fn initiative_scenario() -> InitiativeScenario {
    InitiativeScenario::default()
}

pub(crate) fn roll(die: impl Into<String>) -> RollScenario {
    RollScenario::new(die.into())
}

pub(crate) fn parser_scenario(input: impl Into<String>) -> ParserScenario {
    ParserScenario {
        input: input.into(),
        ..Default::default()
    }
}

#[derive(Default)]
struct ActionExpectation {
    action: Option<ExpectedAction>,
    attackers: Option<Vec<usize>>,
    targets: Option<Vec<usize>>,
    fire: Vec<FireSelection>,
}

enum ExpectedAction {
    Pass,
    Surrender,
    Auxiliary(Option<usize>),
    Attack(Attack),
    Reserve(Option<usize>),
    SetSwing {
        swings: Vec<SwingSelection>,
        options: Vec<OptionSelection>,
    },
}

impl ActionExpectation {
    fn pass(&mut self) {
        self.action = Some(ExpectedAction::Pass);
    }

    fn surrender(&mut self) {
        self.action = Some(ExpectedAction::Surrender);
    }

    fn attack(&mut self, attack: Attack) {
        self.action = Some(ExpectedAction::Attack(attack));
    }

    fn protocol_action(&self) -> Option<ProtocolAction> {
        match self.action.as_ref()? {
            ExpectedAction::Pass => Some(ProtocolAction::Pass),
            ExpectedAction::Surrender => Some(ProtocolAction::Surrender),
            ExpectedAction::Auxiliary(die) => Some(ProtocolAction::Auxiliary { die: *die }),
            ExpectedAction::Attack(attack) => Some(ProtocolAction::Attack {
                attack_type: attack.protocol(),
                attackers: self
                    .attackers
                    .clone()
                    .expect("attack expectation has no attackers"),
                targets: self
                    .targets
                    .clone()
                    .expect("attack expectation has no targets"),
                turbo: None,
                fire: self.fire.clone(),
            }),
            ExpectedAction::Reserve(die) => Some(ProtocolAction::Reserve { die: *die }),
            ExpectedAction::SetSwing { swings, options } => Some(ProtocolAction::SetSwing {
                swings: swings.clone(),
                options: options.clone(),
            }),
        }
    }
}

mod initiative;
mod mechanics;
mod parser;
mod roll;
mod search;

pub(crate) use initiative::InitiativeScenario;
pub(crate) use mechanics::Scenario;
use mechanics::{parse_game, resolve_original_indices};
pub(crate) use parser::ParserScenario;
use parser::legacy_action_suffix;
pub(crate) use roll::RollScenario;
pub(crate) use search::{LEGACY, NATIVE, SearchScenario, legacy_with_workers, native};

#[cfg(test)]
mod tests;
