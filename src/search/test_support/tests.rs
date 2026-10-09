// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;
use crate::Attack::{Berserk, Power, Shadow, Skill, Speed, Trip};
use crate::Phase::Fight;

#[test]
fn forced_win_is_reported_as_certain() {
    search_scenario()
        .phase(Fight)
        .target_wins(3)
        .player(0, 30.0, ["4:4", "p(6,6):11"])
        .player(1, 40.0, ["(2,2):3", "(T,T)-2:2"])
        .ply(2)
        .simulations(5, 100)
        .max_branch(400)
        .surrender(false)
        .workers([1, 4])
        .expect_player_win_percent(0, 100.0..=100.0)
        .expect_attack(crate::Attack::Power)
        .using([1])
        .targeting([1])
        .run();
}

#[test]
fn poison_versus_queer_endgame_reports_the_exact_win_probability() {
    search_scenario()
        .phase(Fight)
        .target_wins(3)
        .player(0, 2.0, ["p20:14"])
        .player(1, 47.0, ["q6:5", "q20:4"])
        .ply(3)
        .simulations(5, 100)
        .max_branch(400)
        .surrender(false)
        .workers([1, 4])
        .expect_player_win_percent(0, 10.0..=10.0)
        .expect_attack(Power)
        .using([0])
        .targeting([1])
        .run();
}

#[test]
fn ordinary_d10_endgame_reports_the_exact_win_probability() {
    search_scenario()
        .phase(Fight)
        .target_wins(3)
        .player(0, 29.0, ["10:9"])
        .player(1, 27.0, ["6:6", "X-6:6"])
        .ply(2)
        .simulations(5, 100)
        .max_branch(400)
        .surrender(false)
        .workers([1, 4])
        .expect_player_win_percent(0, 40.0..=40.0)
        .expect_attack(Power)
        .using([0])
        .targeting([1])
        .run();
}

#[test]
fn twin_d6_endgame_uses_the_full_two_die_distribution() {
    search_scenario()
        .phase(Fight)
        .target_wins(3)
        .player(0, 48.0, ["8:8", "(6,6):9"])
        .player(1, 66.0, ["q6:6", "q6:6"])
        .ply(2)
        .simulations(5, 36)
        .max_branch(400)
        .surrender(false)
        .workers([1, 4])
        .expect_player_win_percent(0, 29.2..=29.2)
        .expect_attack(Power)
        .using([1])
        .targeting([1])
        .run();

    search_scenario()
        .phase(Fight)
        .target_wins(3)
        .player(0, 48.0, ["8:8", "(6,6):9"])
        .player(1, 66.0, ["q6:6", "q6:6"])
        .ply(2)
        .simulations(5, 100)
        .max_branch(400)
        .surrender(false)
        .workers([1, 4])
        .expect_player_win_percent(0, 28.0..=31.0)
        .expect_attack(Power)
        .using([1])
        .targeting([0])
        .run();
}

#[test]
fn poison_versus_queer_endgame_wins_only_on_rerolls_five_and_six() {
    for poison_roll in 1..=20 {
        let mut game = parse_game(
            &["p20:14".to_owned()],
            &["q6:5".to_owned(), "q20:4".to_owned()],
        );
        game.phase = Fight;
        game.players[0].score = 2.0;
        game.players[1].score = 47.0;
        let attacker = resolve_original_indices("attacker", &game.players[0].dice, &[0]);
        let target = resolve_original_indices("target", &game.players[1].dice, &[1]);
        let mut rng = Rng::default();
        rng.reseed(seed_for_first_roll(poison_roll, 20));

        apply_attack(
            &mut game,
            &Move::new_attack(Power, attacker, target, 0.0),
            &mut rng,
        );
        assert_eq!(game.players[0].dice[0].value_total(), poison_roll);
        assert_eq!([game.players[0].score, game.players[1].score], [22.0, 37.0]);

        game.players.swap(0, 1);
        let queer_capture = game
            .valid_attacks(usize::MAX)
            .into_iter()
            .find(|candidate| {
                candidate.attack == Some(Shadow)
                    && candidate
                        .attackers
                        .iter()
                        .any(|index| game.players[0].dice[index].original_index == 0)
                    && candidate
                        .targets
                        .iter()
                        .any(|index| game.players[1].dice[index].original_index == 0)
            });

        if matches!(poison_roll, 5 | 6) {
            apply_attack(&mut game, &queer_capture.unwrap(), &mut rng);
            assert_eq!(
                [game.players[1].score, game.players[0].score],
                [42.0, 27.0],
                "unexpected final scores after Poison rolled {poison_roll}"
            );
        } else {
            assert!(
                queer_capture.is_none(),
                "q6 unexpectedly captured Poison after it rolled {poison_roll}"
            );
        }
    }
}

fn seed_for_first_roll(value: u16, sides: u32) -> u32 {
    (1..=u32::MAX)
        .find(|seed| {
            let mut rng = Rng::default();
            rng.reseed(*seed);
            rng.rand_below(sides) + 1 == u32::from(value)
        })
        .expect("every die face must be reachable from the legacy RNG")
}

#[test]
fn scenario_uses_production_legality_and_null_scoring() {
    scenario()
        .phase(Fight)
        .attackers(["n30:27"])
        .attacks(Power)
        .defenders(["20:19"])
        .using([0])
        .targeting([0])
        .expect_allowed(true)
        .expect_scores(0.0, 0.0)
        .expect_attacker_dice(["n30:30"])
        .expect_no_defender_dice()
        .seed(1)
        .run();
}

#[test]
fn scenario_can_assert_extra_turns_and_next_round_state() {
    scenario()
        .attacker("JM6:6")
        .attacks(Power)
        .defender("1:1")
        .expect_extra_turn(true)
        .expect_attacker_dice(["M6:6"])
        .expect_next_round_attacker_dice(["MJ6"])
        .run();
}

#[test]
fn illegal_scenario_requires_an_explicit_expectation() {
    scenario()
        .attacker("6:1")
        .attacks(Power)
        .defender("20:20")
        .expect_allowed(false)
        .run();
}

#[test]
fn scenario_failures_show_expected_and_actual_die_recipes() {
    let failure = std::panic::catch_unwind(|| {
        scenario()
            .attacker("M6:6")
            .attacks(Power)
            .defender("1:1")
            .expect_attacker_dice(["M8:8"])
            .run();
    })
    .expect_err("the intentionally incorrect recipe should fail");
    let message = failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied())
        .expect("assertion panic should contain text");
    assert!(message.contains("unexpected attacker dice"));
    assert!(message.contains("M6:6"));
    assert!(message.contains("M8:8"));
}

#[test]
fn scenario_indices_follow_recipe_order_after_production_optimization() {
    let game = parse_game(&["6:1".to_owned(), "20:20".to_owned()], &["1:1".to_owned()]);
    assert_eq!(game.players[0].dice[0].original_index, 1);
    assert_eq!(
        resolve_original_indices("attacker", &game.players[0].dice, &[0]),
        [1]
    );
}

#[test]
fn scenario_selects_a_specific_turbo_branch() {
    scenario()
        .attacker("M6/10!-10:10")
        .attacks(Power)
        .defender("1:1")
        .turbo(1)
        .expect_attacker_dice(["M6/10!:6"])
        .run();
}

#[test]
fn fire_dice_cannot_power_attack() {
    scenario()
        .attacker("F6:6")
        .attacks(Power)
        .defender("1:1")
        .expect_allowed(false)
        .run();
}

#[test]
fn fire_assists_a_power_attack_and_stays_turned_down() {
    scenario()
        .attackers(["6:2", "F6:4"])
        .attacks(Power)
        .using([0])
        .defender("6:5")
        .targeting([0])
        .boosting([(0, 5)])
        .firing([(1, 1)])
        .expect_attacker_die(1, "F6:1")
        .run();
}

#[test]
fn fire_search_reports_the_required_turndown_in_legacy_and_json_actions() {
    search_scenario()
        .phase(Fight)
        .player(0, 6.0, ["6:2", "F6:4"])
        .player(1, 3.0, ["6:5"])
        .ply(1)
        .simulations(1, 4)
        .max_branch(100)
        .surrender(false)
        .workers([1, 4])
        .expect_attack(Power)
        .using([0])
        .targeting([0])
        .firing([(1, 1)])
        .run();
}

#[test]
fn optional_fire_overshooting_protects_a_fire_die_when_enabled() {
    search_scenario()
        .phase(Fight)
        .player(0, 15.0, ["~10:1", "F20:20"])
        .player(1, 10.5, ["1:1", "s20:15"])
        .ply(2)
        .simulations(20, 200)
        .max_branch(1_000)
        .surrender(false)
        .fire_overshooting(false)
        .workers([1, 4])
        .expect_player_win_percent(0, 25.0..=40.0)
        .expect_attack(Power)
        .using([0])
        .targeting([0])
        .run();

    search_scenario()
        .phase(Fight)
        .player(0, 15.0, ["~10:1", "F20:20"])
        .player(1, 10.5, ["1:1", "s20:15"])
        .ply(2)
        .simulations(20, 200)
        .max_branch(1_000)
        .surrender(false)
        .fire_overshooting(true)
        .workers([1, 4])
        .expect_player_win_percent(0, 100.0..=100.0)
        .expect_attack(Power)
        .using([0])
        .targeting([0])
        .firing([(1, 14)])
        .run();
}

#[test]
fn fire_assists_a_skill_attack() {
    scenario()
        .attackers(["4:1", "4:1", "F6:4"])
        .attacks(Skill)
        .using([0, 1])
        .defender("8:5")
        .boosting([(0, 4)])
        .firing([(2, 1)])
        .expect_attacker_die(2, "F6:1")
        .run();
}

#[test]
fn fire_cannot_raise_an_attacker_past_its_maximum() {
    scenario()
        .attackers(["4:4", "F6:6"])
        .attacks(Power)
        .using([0])
        .defender("6:5")
        .expect_allowed(false)
        .run();
}

#[test]
fn fire_at_its_minimum_cannot_assist() {
    scenario()
        .attackers(["6:2", "F6:1"])
        .attacks(Power)
        .using([0])
        .defender("6:3")
        .expect_allowed(false)
        .run();
}

#[test]
fn fire_overshooting_requires_an_explicit_player_preference() {
    scenario()
        .attackers(["6:2", "F6:5"])
        .attacks(Power)
        .using([0])
        .defender("1:1")
        .boosting([(0, 3)])
        .firing([(1, 4)])
        .expect_allowed(false)
        .run();
}

#[test]
fn fire_overshooting_is_available_when_the_player_enables_it() {
    scenario()
        .attackers(["6:2", "F6:5"])
        .attacks(Power)
        .using([0])
        .defender("1:1")
        .boosting([(0, 3)])
        .firing([(1, 4)])
        .fire_overshooting(true)
        .expect_allowed(true)
        .run();
}

#[test]
fn fire_does_not_assist_nonstandard_attack_types() {
    for (attack, attacker) in [
        (Berserk, "B6:2"),
        (Speed, "z6:2"),
        (Shadow, "s6:2"),
        (Trip, "t6:2"),
    ] {
        scenario()
            .attackers([attacker, "F6:2"])
            .attacks(attack)
            .using([0])
            .defender("2:2")
            .targeting([0])
            .boosting([(0, 3)])
            .firing([(1, 1)])
            .expect_allowed(false)
            .run();
    }
}

#[test]
fn a_fire_die_can_participate_in_a_skill_attack_but_cannot_assist_itself() {
    scenario()
        .attacker("F6:4")
        .attacks(Skill)
        .defender("6:4")
        .expect_allowed(true)
        .run();

    scenario()
        .attacker("F6:4")
        .attacks(Skill)
        .defender("6:5")
        .expect_allowed(false)
        .run();
}

#[test]
fn multiple_fire_dice_can_split_assistance() {
    scenario()
        .attackers(["6:2", "F6:3", "F8:5"])
        .attacks(Power)
        .using([0])
        .defender("6:6")
        .boosting([(0, 6)])
        .firing([(1, 1), (2, 3)])
        .expect_attacker_die(1, "F6:1")
        .expect_attacker_die(2, "F8:3")
        .run();
}

#[test]
fn mighty_fire_does_not_grow_when_it_only_assists() {
    scenario()
        .attackers(["6:2", "HF6:4"])
        .attacks(Power)
        .using([0])
        .defender("6:5")
        .boosting([(0, 5)])
        .firing([(1, 1)])
        .expect_attacker_die(1, "HF6:1")
        .run();
}

#[test]
fn weak_fire_does_not_shrink_when_it_only_assists() {
    scenario()
        .attackers(["6:2", "hF6:4"])
        .attacks(Power)
        .using([0])
        .defender("6:5")
        .boosting([(0, 5)])
        .firing([(1, 1)])
        .expect_attacker_die(1, "hF6:1")
        .run();
}

#[test]
fn rage_fire_keeps_rage_when_it_only_assists() {
    scenario()
        .attackers(["6:2", "GF6:4"])
        .attacks(Power)
        .using([0])
        .defender("6:5")
        .boosting([(0, 5)])
        .firing([(1, 1)])
        .expect_attacker_die(1, "FG6:1")
        .run();
}

#[test]
fn assisting_jolt_and_time_and_space_dice_do_not_grant_an_extra_turn() {
    for helper in ["JF6:4", "^F6:4"] {
        scenario()
            .attackers(["6:2", helper])
            .attacks(Power)
            .using([0])
            .defender("6:5")
            .boosting([(0, 5)])
            .firing([(1, 1)])
            .expect_extra_turn(false)
            .run();
    }
}

#[test]
fn assisting_ornery_fire_die_still_rerolls_after_the_attack() {
    scenario()
        .attackers(["6:2", "oF6:4"])
        .attacks(Power)
        .using([0])
        .defender("6:5")
        .boosting([(0, 5)])
        .firing([(1, 1)])
        .seed(1)
        .expect_attacker_die(1, "oF6:2")
        .run();
}

#[test]
fn fired_up_konstant_keeps_its_new_value_after_a_skill_attack() {
    scenario()
        .attackers(["k10:3", "4:1", "F6:4"])
        .attacks(Skill)
        .using([0, 1])
        .defender("10:7")
        .boosting([(0, 6)])
        .firing([(2, 1)])
        .expect_attacker_die(0, "k10:6")
        .expect_attacker_die(2, "F6:1")
        .run();
}

#[test]
fn fire_extends_a_stinger_skill_attack_from_its_current_value() {
    scenario()
        .attackers(["g6:2", "F6:3"])
        .attacks(Skill)
        .using([0])
        .defender("6:4")
        .boosting([(0, 4)])
        .firing([(1, 1)])
        .expect_allowed(true)
        .run();
}

#[test]
fn odd_queer_cannot_be_fired_even_to_unlock_a_power_attack() {
    scenario()
        .attackers(["q6:3", "F6:2"])
        .attacks(Power)
        .using([0])
        .defender("4:4")
        .expect_allowed(false)
        .run();
}

#[test]
fn twin_fire_cannot_turn_down_below_one_per_component() {
    scenario()
        .attackers(["6:2", "F(6,6):3"])
        .attacks(Power)
        .using([0])
        .defender("6:3")
        .boosting([(0, 3)])
        .firing([(1, 2)])
        .expect_attacker_die(1, "F(6,6):2")
        .run();
}
