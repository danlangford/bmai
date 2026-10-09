// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[test]
fn a_berserk_die_returns_whole_the_next_round() {
    scenario()
        .attacker("B20:10")
        .attacks(Berserk)
        .defenders(["4:4", "6:6"])
        .targeting([0, 1])
        .expect_attacker_dice(["10:1"])
        .expect_next_round_attacker_dice(["B20"])
        .run();
}
