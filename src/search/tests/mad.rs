// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! "Engine probe" cases ran ButtonWeavers' `BMAttack::commit_attack` under PHP.

use super::*;
use crate::Attack::Shadow;

#[test]
fn mad_resizes_to_even_sizes_in_its_swing_range() {
    roll("Y&-13:5")
        .times(200)
        .expect_sizes([2, 4, 6, 8, 10, 12, 14, 16, 18, 20])
        .run();
}

#[test]
fn a_mad_twin_shares_one_size() {
    // Engine probe.
    roll("(Y,Y)&-13:13")
        .times(200)
        .expect_twin_halves_match()
        .expect_sizes([2, 4, 6, 8, 10, 12, 14, 16, 18, 20])
        .run();
}

#[test]
fn a_mad_die_may_start_at_an_odd_size() {
    let mut parser = crate::Parser::default();
    parser
        .parse_string(
            "game\npreround\nplayer 0 1 0\nX&\nplayer 1 1 0\n6\n",
            &mut Vec::new(),
        )
        .unwrap();
    let sizes = generate_swing_moves(&parser.game.players[0])
        .iter()
        .map(|candidate| candidate.values()[0].1)
        .collect::<Vec<_>>();
    assert!(sizes.contains(&13), "{sizes:?}");
}

#[test]
fn ornery_rerolls_randomize_a_mad_die() {
    scenario()
        .attackers(["6:6", "oY&-13:3"])
        .attacks(Power)
        .seed(1)
        .using([0])
        .defender("1:1")
        .expect_attacker_die(1, "oY-8&:2")
        .run();
}

#[test]
fn decay_removes_mad() {
    // Engine probe.
    scenario()
        .attacker("%Y&-12:12")
        .attacks(Power)
        .defender("4:3")
        .expect_attacker_dice(["Y-6:5", "Y-6:1"])
        .run();
}

#[test]
fn konstant_mad_dice_keep_their_size() {
    // Engine probe.
    scenario()
        .attacker("ksY&-12:3")
        .attacks(Shadow)
        .defender("6:5")
        .expect_attacker_dice(["skY-12&:3"])
        .run();
}

#[test]
fn trip_attackers_and_targets_resize_when_mad() {
    // Engine probe.
    scenario()
        .attacker("tY&-13:3")
        .attacks(Trip)
        .defender("20:20")
        .expect_attacker_dice(["tY-2&:1"])
        .run();
    scenario()
        .attacker("t1:1")
        .attacks(Trip)
        .defender("Y&-13:5")
        .expect_defender_dice(["Y-6&:3"])
        .run();
}

#[test]
fn mad_parses_before_or_after_the_swing_size() {
    for die in ["X&-12:5", "X-12&:5"] {
        let game = native_fixture_game(&format!(
            "game\nfight\nplayer 0 1 0\n{die}\nplayer 1 1 0\n1:1\n"
        ));
        assert!(game.players[0].dice[0].has_property(property::MAD), "{die}");
    }
}
