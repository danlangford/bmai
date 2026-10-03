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

pub(crate) use action::BMC_FireAdjustment;
pub use action::{BMC_DieIndexSet, BMC_Move};
pub use die::BMC_Die;
pub(crate) use mechanics::{
    ApplyAttack, ApplyAttackForPlayers, ApplyBeforeRollEffects, AvailableDice, CheckInitiative,
    InitiativeWinner, OptimizeDice, RecoverDizzyDice, RestoreDiceForNewRound, RollDie,
    RollRoundDice, RollScheduledDie, SwingRange,
};
#[cfg(test)]
pub(crate) use mechanics::{ApplyAttackPlayerEffects, SplitRadioactiveAttacker};
pub use player::BMC_Player;
pub use state::BMC_Game;
pub(crate) use state::{BMD_MAX_DICE, BMD_MAX_INPUT_DICE};
pub use types::{BME_ACTION, BME_ATTACK, BME_PHASE, BME_SWING_SET};

#[cfg(test)]
mod tests;
