// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! "Engine probe" cases ran ButtonWeavers' `BMAttack::commit_attack` under PHP.

use super::*;

#[test]
fn mood_resizes_to_standard_die_sizes_in_its_swing_range() {
    roll("R?-11:5")
        .times(200)
        .expect_sizes([2, 4, 6, 8, 10, 12])
        .run();
}

#[test]
fn chance_rerolls_resize_mood_dice() {
    initiative_scenario()
        .player(["cX?-13:1", "1:1"])
        .opponent(["20:20"])
        .chance_rerolls([0])
        .expect_player_dice(["cX?-12:1", "1:1"])
        .run();
}
