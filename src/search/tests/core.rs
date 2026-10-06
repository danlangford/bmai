// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[test]
fn unique_rejects_equal_values_on_lower_swing_types() {
    let player = Player {
        dice: vec![swing_die('P', 0, 0), swing_die('Q', property::UNIQUE, 1)],
        ..Default::default()
    };

    let moves = generate_swing_moves(&player);
    assert_eq!(moves.len(), 30 * 19 - 19);
    assert!(moves.iter().all(|candidate| {
        let p = candidate
            .values()
            .iter()
            .find(|(swing, _)| *swing == 'P')
            .unwrap()
            .1;
        let q = candidate
            .values()
            .iter()
            .find(|(swing, _)| *swing == 'Q')
            .unwrap()
            .1;
        p != q
    }));
}

#[test]
fn turbo_swing_changes_all_matching_dice_before_the_reroll() {
    scenario()
        .attackers(["kX!-10:10", "3:3", "kX-10:7"])
        .attacks(Skill)
        .using([0, 1])
        .defender("13:13")
        .turbo(20)
        .expect_attacker_die(2, "kX-20:7")
        .run();
}

#[test]
fn pr82_trip_target_before_roll_effect_triggers_once() {
    for (target, expected) in [("H6:6", "H8:8"), ("h20:20", "h16:16")] {
        scenario()
            .attacker("kt4:1")
            .seed(2)
            .attacks(Trip)
            .defender(target)
            .expect_defender_dice([expected])
            .run();
    }
}

#[test]
fn pr82_participating_ornery_before_roll_effect_triggers_once() {
    scenario()
        .attacker("oH4:4")
        .attacks(Power)
        .defender("1:1")
        .expect_attacker_dice(["Ho6:5"])
        .run();
}

#[test]
fn pr82_ordinary_side_change_invalidates_value() {
    let mut game = Game::default();
    let mut attacker = swing_die('P', property::MIGHTY, 0);
    attacker.sides[0] = 6;
    attacker.value = Some(3);
    let mut target = swing_die('P', 0, 0);
    target.sides[0] = 1;
    target.value = Some(1);
    game.players[0].dice = vec![attacker];
    game.players[1].dice = vec![target];
    let action = Move::new_attack(Power, [0], [0], 0.0);

    apply_attack_player_effects(&mut game, &action, 0, 1, 0, true);
    assert!(game.players[0].dice[0].not_set);
    assert_eq!(game.players[0].dice[0].sides[0], 8);
}

#[test]
fn copied_cpp_konstant_skill_attacker_keeps_its_value() {
    scenario()
        .attackers(["k20:13", "7:7"])
        .attacks(Skill)
        .using([0, 1])
        .defender("20:20")
        .expect_attacker_die(0, "k20:13")
        .run();
}

#[test]
fn copied_cpp_multi_target_speed_attack_does_not_morph() {
    scenario()
        .attacker("mz10:8")
        .attacks(Speed)
        .defenders(["4:3", "6:5"])
        .targeting([0, 1])
        .expect_attacker_dice(["zm10:1"])
        .run();
}

#[test]
fn copied_cpp_konstant_chance_die_keeps_its_value() {
    initiative_scenario()
        .player(["ck100:7"])
        .opponent(["20:20"])
        .chance_rerolls([0])
        .expect_player_dice(["ck100:7"])
        .run();
}

#[test]
fn cpp_chance_success_is_keyed_to_player_zero_initiative() {
    initiative_scenario()
        .seated_as(1)
        .player(["ck20:6"])
        .opponent(["20:5"])
        .chance_rerolls([0])
        .expect_initiative(Some(0))
        .expect_chance_success(true)
        .expect_next_initiative(1)
        .run();
}

#[test]
fn cpp_focus_marks_dice_dizzy_until_turn_recovery() {
    initiative_scenario()
        .player(["f20:12"])
        .opponent(["20:20"])
        .focuses([(0, 7)])
        .expect_player_dice(["f20:7d"])
        .expect_player_dice_next_turn(["f20:7"])
        .run();
}

#[test]
fn cpp_maximum_die_always_rolls_its_maximum() {
    roll("M6").times(10).expect_values([6]).run();
}

#[test]
#[should_panic(expected = "Die::Roll requires NOTSET state")]
fn cpp_roll_requires_notset_state() {
    let mut die = swing_die('P', 0, 0);
    die.sides[0] = 6;
    die.value = Some(1);
    die.not_set = false;
    roll_die(&mut die, &mut Rng::default());
}

#[test]
#[should_panic(expected = "Die::OnSwingSet requires NOTSET state")]
fn cpp_swing_set_requires_notset_state() {
    let mut die = swing_die('X', 0, 0);
    die.sides[0] = 6;
    die.value = Some(1);
    die.not_set = false;
    let mut player = crate::game::Player {
        dice: vec![die],
        ..Default::default()
    };
    let mut action = SwingMove::empty();
    action.push_value(('X', 8));
    apply_swing_move(&mut player, &action);
}

#[test]
fn cpp_konstant_target_retains_value_when_tripped() {
    scenario()
        .attacker("kt8:8")
        .attacks(Trip)
        .defender("k100:7")
        .seed(1)
        .expect_no_defender_dice()
        .expect_captured_defender_dice(["k100:7"])
        .run();
}

#[test]
fn cpp_konstant_warrior_keeps_value_and_loses_warrior_after_skill() {
    scenario()
        .attackers(["`k41:17", "11:11"])
        .attacks(Skill)
        .using([0, 1])
        .defender("20:28")
        .seed(1)
        .expect_attacker_die(0, "k41:17")
        .expect_no_defender_dice()
        .run();
}

#[test]
fn cpp_morphing_copies_single_and_twin_target_sizes() {
    for (attacker, target, expected) in [
        ("m9:8", "m7:6", "m7:7"),
        ("m7:8", "m(10,11):6", "m(10,11):9"),
    ] {
        scenario()
            .attacker(attacker)
            .attacks(Power)
            .defender(target)
            .expect_attacker_dice([expected])
            .run();
    }
}

#[test]
fn each_playout_plays_its_own_engines_move() {
    let game = native_fixture_game(
        "game\nfight\nplayer 0 3 0\n6:6\n8:2\n10:9\nplayer 1 3 0\n6:6\n5:5\n4:4\n",
    );
    let fire_limit = Bmai3::default().fire_candidate_limit();
    let own_move = |playout, rng: &mut Rng| match playout {
        crate::Playout::Quick => crate::engines::quick::attack(&game, rng, fire_limit),
        crate::Playout::Careful => crate::engines::quick::careful_attack(&game, rng, fire_limit),
        crate::Playout::Maximize => crate::engines::maximize::attack(&game, rng, fire_limit),
        crate::Playout::Random => crate::engines::random::attack(&game, rng, fire_limit),
    };
    let mut picks = std::collections::BTreeSet::new();
    for playout in [
        crate::Playout::Quick,
        crate::Playout::Maximize,
        crate::Playout::Random,
    ] {
        let ai = Bmai3 {
            playout,
            ..Default::default()
        };
        for seed in 1..=20 {
            let (mut search_rng, mut engine_rng) = (Rng::default(), Rng::default());
            search_rng.reseed(seed);
            engine_rng.reseed(seed);
            let played = select_rollout_action(&game, &mut search_rng, &ai);
            let expected = own_move(playout, &mut engine_rng);
            assert_eq!(
                format!("{played:?}"),
                format!("{expected:?}"),
                "{playout:?} {seed}"
            );
            picks.insert((playout.name(), format!("{played:?}")));
        }
    }
    // The engines must disagree somewhere, or routing to the wrong one passes.
    let distinct = |name| picks.iter().filter(|(playout, _)| *playout == name).count();
    assert!(distinct("random") > distinct("maximize"), "{picks:?}");
    assert!(
        picks
            .iter()
            .any(|(playout, pick)| *playout == "quick"
                && !picks.contains(&("maximize", pick.clone()))),
        "{picks:?}"
    );
}

#[test]
fn the_first_swing_move_matches_the_first_generated_one() {
    for (dice, special) in [
        ("4\nX\nY\nZ\nV", ""),
        ("6/12\nX\n(Y,Y)", ""),
        ("uX\nuY\nZ", ""),
        ("4\nX\nX\nY", "special 0 unique_swing\n"),
        ("4\n6\nX\nY\nZ", "special 0 unique_sizes\n"),
        ("8\n10", ""),
        ("4/6\nX", "special 0 unique_sizes\n"),
        ("6\n7\n8\n9\n10\n11\n12\nV", "special 0 unique_sizes\n"),
    ] {
        let count = dice.lines().count();
        let game = native_fixture_game(&format!(
            "game\npreround\nplayer 0 {count} 0\n{dice}\nplayer 1 1 0\n6\n{special}"
        ));
        let player = &game.players[0];
        assert_eq!(
            first_swing_move(player).map(|m| (m.values().to_vec(), m.options().to_vec())),
            generate_swing_moves(player)
                .first()
                .map(|m| (m.values().to_vec(), m.options().to_vec())),
            "{dice} {special}"
        );
    }
}
