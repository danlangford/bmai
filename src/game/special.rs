// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! ButtonWeavers button specials: rules that belong to a button, not a die.

/// Guillermo, Oregon: different swing types take different sizes.
pub const UNIQUE_SWING: u8 = 1;
/// Gordo: no two dice share a size, and no Auxiliary swing die.
pub const UNIQUE_SIZES: u8 = 1 << 1;
/// Largo, The Flying Squirrel.
pub const NO_SKILL_ATTACKS: u8 = 1 << 2;
/// The Japanese Beetle.
pub const SKILL_IMMUNE: u8 = 1 << 3;
/// Giant.
pub const NO_INITIATIVE: u8 = 1 << 4;
