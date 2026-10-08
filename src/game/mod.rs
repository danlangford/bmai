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
    apply_attack, apply_attack_for_players, apply_before_roll_effects, available_dice_count,
    check_initiative, initiative_winner, optimize_dice, record_round_sides, recover_dizzy_dice,
    restore_dice_for_new_round, roll_die, roll_round_dice, roll_scheduled_die, swing_range,
};
#[cfg(test)]
pub(crate) use mechanics::{apply_attack_player_effects, split_radioactive_attacker};
pub use player::Player;
pub use state::Game;
pub(crate) use state::{MAX_DICE, MAX_INPUT_DICE};
pub use types::{Action, Attack, Phase, SwingSet};

#[cfg(test)]
mod tests;
