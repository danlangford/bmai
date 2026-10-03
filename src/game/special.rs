// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! Button specials: rules that belong to a button, not a die.
//! `protocol::notation::BUTTON_SPECIALS` names them and lists their buttons.

pub const UNIQUE_SWING: u8 = 1;
pub const UNIQUE_SIZES: u8 = 1 << 1;
pub const NO_SKILL_ATTACKS: u8 = 1 << 2;
pub const SKILL_IMMUNE: u8 = 1 << 3;
pub const NO_INITIATIVE: u8 = 1 << 4;
