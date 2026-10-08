// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;
use crate::Attack::Power;
use test_support::scenario;

#[test]
fn a_null_capture_nulls_the_die_only_for_the_round() {
    scenario()
        .attacker("n8:8")
        .attacks(Power)
        .defender("20:5")
        .expect_scores(0.0, 0.0)
        .expect_captured_defender_dice(["n20:5"])
        .expect_next_round_defender_dice(["20"])
        .run();
}
