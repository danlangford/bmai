// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! "Engine probe" cases ran ButtonWeavers' `BMAttack::commit_attack` under PHP.

use super::*;
use crate::Attack::{Boom, Shadow};
use test_support::{LEGACY, NATIVE, native, search_scenario};

const REROLL_SEED: u32 = 3;

fn boom_choices(attacker: &str, target: &str) -> Vec<Move> {
    attacks_by(&[attacker], &[target])
        .into_iter()
        .filter(|candidate| candidate.m_attack == Some(Boom))
        .collect()
}

fn mood_sizes(die: &str) -> Vec<u8> {
    let mut rng = Rng::default();
    let mut sizes = (0..200)
        .map(|_| {
            let mut game = native_fixture_game(&format!(
                "game\nfight\nplayer 0 1 0\n{die}\nplayer 1 1 0\n1:1\n"
            ));
            game.m_player[0].m_die[0].m_notset = true;
            RollScheduledDie(&mut game, 0, 0, &mut rng);
            game.m_player[0].m_die[0].m_sides[0]
        })
        .collect::<Vec<_>>();
    sizes.sort_unstable();
    sizes.dedup();
    sizes
}

#[test]
fn boom_removes_the_boom_die_unscored_and_rerolls_the_target() {
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("6:5")
        .expect_attacker_dice(Vec::<&str>::new())
        .expect_defender_dice(["6:3"])
        .expect_captured_defender_dice(Vec::<&str>::new())
        .expect_scores(0.0, 3.0)
        .expect_next_round_attacker_dice(["b4:2"])
        .run();
}

#[test]
fn only_boom_dice_can_boom() {
    scenario()
        .attacker("4:2")
        .attacks(Boom)
        .defender("6:5")
        .expect_allowed(false)
        .run();
}

#[test]
fn stealth_dice_may_be_targeted_by_boom_attacks() {
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("d6:5")
        .expect_defender_dice(["d6:3"])
        .run();
}

#[test]
fn stealth_and_warrior_dice_cannot_boom() {
    for attacker in ["db4:2", "`b4:4"] {
        scenario()
            .attacker(attacker)
            .attacks(Boom)
            .defender("6:5")
            .expect_allowed(false)
            .run();
    }
}

#[test]
fn warrior_dice_cannot_be_boomed() {
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .defender("`6:6")
        .expect_allowed(false)
        .run();
}

#[test]
fn dizzy_boom_dice_cannot_boom() {
    scenario()
        .attacker("bf4:2d")
        .attacks(Boom)
        .defender("6:5")
        .expect_allowed(false)
        .run();
}

#[test]
fn konstant_boom_dice_may_boom() {
    scenario()
        .attacker("kb4:2")
        .attacks(Boom)
        .defender("6:5")
        .expect_allowed(true)
        .run();
}

#[test]
fn a_konstant_target_keeps_its_value() {
    // Engine probe.
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .defender("k6:5")
        .expect_defender_dice(["k6:5"])
        .run();
}

#[test]
fn a_mighty_target_grows_on_its_reroll() {
    // Engine probe.
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("H6:5")
        .expect_defender_dice(["H8:7"])
        .run();
}

#[test]
fn a_weak_target_shrinks_on_its_reroll() {
    // Engine probe.
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("h12:5")
        .expect_defender_dice(["h10:1"])
        .run();
}

#[test]
fn a_value_target_rescores_after_its_reroll() {
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("v6:5")
        .expect_defender_dice(["v6:3"])
        .expect_scores(0.0, 1.5)
        .run();
}

#[test]
fn a_mad_target_resizes_to_an_even_size() {
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("X&-12:5")
        .expect_defender_dice(["X-14&:5"])
        .run();
}

#[test]
fn a_rage_target_is_not_replaced_because_it_is_not_captured() {
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("G6:5")
        .expect_defender_dice(["G6:3"])
        .expect_captured_defender_dice(Vec::<&str>::new())
        .run();
}

#[test]
fn only_a_jolt_boom_die_grants_an_extra_turn() {
    // Engine probe.
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .defender("J6:5")
        .expect_extra_turn(false)
        .run();
    scenario()
        .attacker("Jb4:2")
        .attacks(Boom)
        .defender("6:5")
        .expect_extra_turn(true)
        .run();
}

#[test]
fn time_and_space_boom_dice_never_grant_an_extra_turn() {
    for seed in 1..=8 {
        scenario()
            .attacker("^b4:2")
            .attacks(Boom)
            .defender("6:5")
            .seed(seed)
            .expect_extra_turn(false)
            .run();
    }
}

#[test]
fn null_and_value_boom_dice_change_nothing_because_nothing_is_captured() {
    for attacker in ["nb4:2", "vb4:2"] {
        scenario()
            .attacker(attacker)
            .attacks(Boom)
            .seed(REROLL_SEED)
            .defender("6:5")
            .expect_defender_dice(["6:3"])
            .run();
    }
}

#[test]
fn radioactive_never_decays_on_a_boom() {
    // Engine probe.
    scenario()
        .attacker("%b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("6:5")
        .expect_attacker_dice(Vec::<&str>::new())
        .run();
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("%6:5")
        .expect_defender_dice(["%6:3"])
        .run();
}

#[test]
fn ornery_dice_reroll_after_a_boom() {
    scenario()
        .attackers(["b4:2", "o10:3"])
        .attacks(Boom)
        .seed(5)
        .using([0])
        .defender("6:5")
        .expect_attacker_dice(["o10:2"])
        .run();
}

#[test]
fn a_twin_target_rerolls_both_halves() {
    scenario()
        .attacker("b4:2")
        .attacks(Boom)
        .seed(REROLL_SEED)
        .defender("(3,3):5")
        .expect_defender_dice(["(3,3):6"])
        .run();
}

#[test]
fn fire_cannot_assist_a_boom() {
    scenario()
        .attackers(["b4:2", "F6:4"])
        .attacks(Boom)
        .using([0])
        .defender("6:5")
        .boosting([(0, 3)])
        .firing([(1, 3)])
        .expect_allowed(false)
        .run();
}

#[test]
fn turbo_boom_dice_offer_no_turbo_sizes() {
    let choices = boom_choices("bX!-6:2", "6:5")
        .into_iter()
        .map(|candidate| candidate.m_turbo_option)
        .collect::<Vec<_>>();
    assert_eq!(choices, vec![-1]);
}

#[test]
fn search_reports_a_boom_when_it_is_the_only_attack() {
    search_scenario()
        .phase(Fight)
        .player(0, 0.5, ["b1:1"])
        .player(1, 10.0, ["20:20"])
        .ply(1)
        .simulations(5, 20)
        .max_branch(100)
        .surrender(false)
        .modes([LEGACY, NATIVE, native(4)])
        .expect_attack(Boom)
        .using([0])
        .targeting([0])
        .run();
}

#[test]
fn mad_resizes_to_even_sizes_in_its_swing_range() {
    assert_eq!(
        mood_sizes("Y&-13:5"),
        vec![2, 4, 6, 8, 10, 12, 14, 16, 18, 20]
    );
}

#[test]
fn mood_resizes_to_standard_die_sizes_in_its_swing_range() {
    assert_eq!(mood_sizes("R?-11:5"), vec![2, 4, 6, 8, 10, 12]);
}

#[test]
fn a_mad_twin_shares_one_size() {
    // Engine probe.
    let mut rng = Rng::default();
    for _ in 0..20 {
        let mut game =
            native_fixture_game("game\nfight\nplayer 0 1 0\n(Y,Y)&-13:13\nplayer 1 1 0\n1:1\n");
        game.m_player[0].m_die[0].m_notset = true;
        RollScheduledDie(&mut game, 0, 0, &mut rng);
        let sides = game.m_player[0].m_die[0].m_sides;
        assert_eq!(sides[0], sides[1]);
        assert_eq!(sides[0] % 2, 0);
    }
}

#[test]
fn chance_rerolls_resize_mood_dice() {
    let mut game =
        native_fixture_game("game\nchance\nplayer 0 2 0\ncX?-13:1\n1:1\nplayer 1 1 0\n20:20\n");
    ApplyChanceMove(
        &mut game,
        0,
        1,
        &ChanceMove { reroll: vec![0] },
        &mut Rng::default(),
    );
    let die = game.m_player[0]
        .m_die
        .iter()
        .find(|die| die.m_original_index == 0)
        .unwrap();
    assert_ne!(die.m_sides[0], 13);
    assert!([4, 6, 8, 10, 12, 20].contains(&die.m_sides[0]));
}

#[test]
fn a_mad_die_may_start_at_an_odd_size() {
    let mut parser = crate::Parser::default();
    parser
        .ParseString(
            "game\npreround\nplayer 0 1 0\nX&\nplayer 1 1 0\n6\n",
            &mut Vec::new(),
        )
        .unwrap();
    let sizes = GenerateSwingMoves(&parser.m_game.m_player[0])
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
        assert!(
            game.m_player[0].m_die[0].HasProperty(property::MAD),
            "{die}"
        );
    }
}
