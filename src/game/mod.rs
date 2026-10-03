// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

mod action;
mod attack;
mod die;
mod mechanics;
mod player;
pub mod property;
pub mod special;
mod state;
mod types;

pub(crate) use action::FireAdjustment;
pub use action::{DieIndexSet, Move};
pub use die::Die;
pub(crate) use mechanics::{
    ApplyAttack, ApplyAttackForPlayers, ApplyBeforeRollEffects, AvailableDice, CheckInitiative,
    InitiativeWinner, OptimizeDice, RecoverDizzyDice, RestoreDiceForNewRound, RollDie,
    RollRoundDice, RollScheduledDie, SwingRange,
};
#[cfg(test)]
pub(crate) use mechanics::{ApplyAttackPlayerEffects, SplitRadioactiveAttacker};
pub use player::Player;
pub use state::Game;
pub(crate) use state::{MAX_DICE, MAX_INPUT_DICE};
pub use types::{Action, Attack, Phase, SwingSet};

#[cfg(test)]
mod tests;
