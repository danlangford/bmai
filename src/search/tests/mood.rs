// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

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

#[test]
fn a_round_winner_keeps_the_mood_size_it_chose() {
    scenario()
        .attacker("X?-13:13")
        .attacks(Power)
        .defender("1:1")
        .expect_attacker_dice(["X-12?:1"])
        .expect_next_round_attacker_dice(["X?-13"])
        .run();
}
