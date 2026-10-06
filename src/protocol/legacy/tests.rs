// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;
use crate::Attack::{Power, Skill};
use crate::Phase::{Auxiliary, Chance, Focus};
use crate::search::test_support::{LEGACY, parser_scenario, search_scenario};

#[test]
fn cpp_parser_multiline_fight_string() {
    let input = "\n\
game\n\
fight\n\
player 0 3 0\n\
30:16\n\
30:16\n\
6/30-6:5\n\
player 1 3 0\n\
5/7-5:4\n\
7/9-7:3\n\
9/11-9:3\n\
getaction\n";
    Parser::default()
        .parse_string(input, &mut Vec::new())
        .unwrap();
}

#[test]
fn streamed_legacy_commands_match_batched_parsing() {
    let input = "game\nfight\nplayer 0 1 1\n1:1\nplayer 1 2 30\n1:1\n(30,30):60\nseed 17\nsurrender off\ngetaction\nquit\n";
    let mut batched = Parser::default();
    let mut batched_output = Vec::new();
    batched.parse_string(input, &mut batched_output).unwrap();

    let mut streamed = Parser::default();
    let mut streamed_output = Vec::new();
    streamed
        .parse_stream(&mut std::io::Cursor::new(input), &mut streamed_output)
        .unwrap();

    assert_eq!(streamed_output, batched_output);
    assert_eq!(streamed.session_metadata(), batched.session_metadata());
    assert_eq!(streamed.last_action(), batched.last_action());
    assert_eq!(streamed.last_replay(), batched.last_replay());
}

#[test]
fn preround_returns_already_selected_swing_without_resimulating_it() {
    let input = "game\n\
preround\n\
player 0 5 0\n\
6:4\n\
8:6\n\
8:7\n\
12:1\n\
X-4:3\n\
player 1 5 0\n\
4\n\
12\n\
20\n\
20\n\
X\n\
getaction\n";
    let mut output = Vec::new();

    Parser::default().parse_string(input, &mut output).unwrap();

    assert!(
        String::from_utf8(output)
            .unwrap()
            .ends_with("action\nswing X 4\n")
    );
}

#[test]
fn preround_returns_already_selected_option_without_resimulating_it() {
    let input = "game\n\
preround\n\
player 0 1 0\n\
4/8-8:3\n\
player 1 1 0\n\
6\n\
getaction\n";
    let mut output = Vec::new();

    Parser::default().parse_string(input, &mut output).unwrap();

    assert!(
        String::from_utf8(output)
            .unwrap()
            .ends_with("action\noption 0 8\n")
    );
}

#[test]
fn whole_line_comments_are_ignored_between_top_level_commands() {
    let game = "game\nfight\nplayer 0 1 1\n1:1\nplayer 1 1 1\n1:1\n";
    let plain = format!("{game}seed 17\nsurrender off\nquit\n");
    let commented = format!(
        "  # generated game state\n{game}\t# search policy\nseed 17\nsurrender off\n# done\nquit\n"
    );

    let parse_batched = |input: &str| {
        let mut parser = Parser::default();
        let mut output = Vec::new();
        parser.parse_string(input, &mut output).unwrap();
        (output, parser.session_metadata())
    };
    let parse_streamed = |input: &str| {
        let mut parser = Parser::default();
        let mut output = Vec::new();
        parser
            .parse_stream(&mut std::io::Cursor::new(input), &mut output)
            .unwrap();
        (output, parser.session_metadata())
    };

    let expected = parse_batched(&plain);
    assert_eq!(parse_batched(&commented), expected);
    assert_eq!(parse_streamed(&commented), expected);
}

#[test]
fn comments_inside_game_blocks_remain_invalid() {
    let input = "game\n# phase comment\nfight\n";
    let batched = Parser::default()
        .parse_string(input, &mut Vec::new())
        .unwrap_err();
    let streamed = Parser::default()
        .parse_stream(&mut std::io::Cursor::new(input), &mut Vec::new())
        .unwrap_err();

    assert_eq!(batched.to_string(), "phase not found");
    assert_eq!(streamed.to_string(), batched.to_string());
}

#[test]
fn inline_comments_remain_invalid() {
    let error = Parser::default()
        .parse_string("seed 17 # comment\n", &mut Vec::new())
        .unwrap_err();

    assert_eq!(error.to_string(), "invalid integer: 17 # comment");
}

#[test]
fn streamed_game_rejects_malformed_structure_without_waiting_for_more_input() {
    for (input, expected) in [
        ("game\ninvalid-phase\n", "phase not found"),
        ("game\nfight\nplayer 0 1\n", "missing player: player 0 1"),
        (
            "game\nfight\nplayer 0 1 invalid\n",
            "invalid score: invalid",
        ),
        (
            "game\nfight\nplayer 0 2 1\nnot-a-die\n",
            "error parsing die not-a-die at -",
        ),
    ] {
        let mut parser = Parser::default();
        let error = parser
            .parse_stream(&mut std::io::Cursor::new(input), &mut Vec::new())
            .unwrap_err();
        assert_eq!(error.to_string(), expected);
    }
}

#[test]
fn game_rejects_more_than_ten_input_dice_in_both_parsing_paths() {
    let input = "game\nfight\nplayer 0 11 0\n";
    let expected = "player dice count 11 exceeds maximum 10";

    let batched = Parser::default()
        .parse_string(input, &mut Vec::new())
        .unwrap_err();
    let streamed = Parser::default()
        .parse_stream(&mut std::io::Cursor::new(input), &mut Vec::new())
        .unwrap_err();

    assert_eq!(batched.to_string(), expected);
    assert_eq!(streamed.to_string(), expected);
}

#[test]
fn ai_names_an_engine_and_per_player_settings_apply_to_that_player() {
    let input = "ai 0 montecarlo\nply 0 3\nmax_sims 0 40\nmin_sims 0 4\nmaxbranch 0 400\ncull 0 off\ndebugply 2\nseed 17\nquit\n";
    let mut parser = Parser::default();
    let mut output = Vec::new();
    parser.parse_string(input, &mut output).unwrap();
    let engine = parser.player_engine(0);
    let search = engine.montecarlo().unwrap();
    assert_eq!(engine.name(), "montecarlo");
    assert_eq!(
        (
            search.max_ply,
            search.max_sims,
            search.min_sims,
            search.max_branch,
            search.cull_moves
        ),
        (3, 40, 4, 400, false)
    );
    assert_eq!(parser.player_engine(1).montecarlo().unwrap().max_ply, 1);
    assert_eq!(parser.debug_ply, 2);
    assert_eq!(
        String::from_utf8(output).unwrap(),
        "Setting AI for player 0 to montecarlo\n\
Setting max ply for player 0 to 3\n\
Setting max sims for player 0 to 40\n\
Setting min sims for player 0 to 4\n\
Setting max branch for player 0 to 400\n\
Setting cull for player 0 to off\n\
Setting debug ply to 2\n\
Seeding with 17\n"
    );
}

#[test]
fn each_player_owns_its_engine_settings() {
    const GAME: &str = "game\nfight\nplayer 0 3 0\n6:6\n4:2\n8:5\nplayer 1 3 0\n4:4\n3:3\n10:7\n";
    let cases: [(&str, String, &[&str]); 6] = [
        (
            "a player-1 setting leaves player 0 and the global settings alone",
            format!("{GAME}max_sims 1 3\ngetaction\n"),
            &["l1 p0 Valid Moves 5 Sims 500", "stats 1/10-500/5000/0.50"],
        ),
        (
            "a per-player setting starts from the global settings",
            format!("min_sims 2\nmax_sims 40\n{GAME}max_sims 0 3\ngetaction\n"),
            &["l1 p0 Valid Moves 5 Sims 3", "stats 1/2-40/5000/0.50"],
        ),
        (
            "ai montecarlo starts from the global settings",
            format!("ply 2\nmax_sims 7\n{GAME}ai 0 montecarlo\ngetaction\n"),
            &["l1 p0 Valid Moves 5 Sims 7", "stats 2/10-7/5000/0.50"],
        ),
        (
            "game returns both players to the global settings",
            format!(
                "min_sims 2\nmax_sims 7\n{GAME}ai 0 montecarlo\nmax_sims 0 3\n{GAME}getaction\n"
            ),
            &["l1 p0 Valid Moves 5 Sims 7", "stats 1/2-7/5000/0.50"],
        ),
        (
            "two montecarlo players do not share settings",
            format!("{GAME}ai 0 montecarlo\nai 1 montecarlo\nmax_sims 1 3\ngetaction\n"),
            &["l1 p0 Valid Moves 5 Sims 500", "stats 1/10-500/5000/0.50"],
        ),
        (
            "global settings do not reach a player with its own engine",
            format!("{GAME}ai 0 montecarlo\nmax_sims 7\ngetaction\n"),
            &["l1 p0 Valid Moves 5 Sims 500", "stats 1/10-7/5000/0.50"],
        ),
    ];
    for (name, input, expected) in cases {
        let mut output = Vec::new();
        Parser::default()
            .parse_string(
                &format!(
                    "seed 17\nmode legacy\nmax_sims 500\nmin_sims 10\nmaxbranch 5000\n{input}"
                ),
                &mut output,
            )
            .unwrap();
        let output = String::from_utf8(output).unwrap();
        let observed = output
            .lines()
            .filter(|line| line.contains("Valid Moves") || line.starts_with("stats "))
            .collect::<Vec<_>>();
        assert_eq!(observed, expected, "{name}\n{output}");
    }
}

#[test]
fn global_cull_and_playout_settings_reach_players_following_them() {
    let mut parser = Parser::default();
    parser
        .parse_string(
            "cull off\nplayout maximize\ngame\nfight\nplayer 0 1 0\n6:6\nplayer 1 1 0\n4:4\nai 1 montecarlo\n",
            &mut Vec::new(),
        )
        .unwrap();
    for player in 0..2 {
        let engine = parser.player_engine(player);
        let search = engine.montecarlo().unwrap();
        assert!(!search.cull_moves, "player {player}");
        assert_eq!(search.playout, crate::Playout::Maximize, "player {player}");
    }
}

#[test]
fn engines_reject_settings_they_do_not_use() {
    for (input, expected) in [
        ("ply 0\n", "montecarlo ply must be at least 1"),
        ("ply 1 0\n", "player 1: montecarlo ply must be at least 1"),
        (
            "ai 0 quick\nply 0 2\n",
            "player 0: quick has no ply setting",
        ),
        (
            "ai 0 random\nmax_sims 0 9\n",
            "player 0: random has no max_sims setting",
        ),
        (
            "ai 1 maximize\ncull 1 off\n",
            "player 1: maximize has no cull setting",
        ),
        ("cull maybe\n", "cull needs on or off, not maybe"),
        (
            "playout best\n",
            "playout needs quick, careful, maximize, or random, not best",
        ),
        (
            "ai 0 quick\nplayout 0 random\n",
            "player 0: quick has no playout setting",
        ),
    ] {
        assert_eq!(
            Parser::default()
                .parse_string(input, &mut Vec::new())
                .unwrap_err()
                .to_string(),
            expected,
            "{input}"
        );
    }
}

#[test]
fn ai_rejects_unknown_engines_and_players() {
    for (input, expected) in [
        ("ai 2 quick\n", "invalid setting for ai player number: 2"),
        (
            "ai 0 1\n",
            "unknown ai 1; choose one of: random, maximize, quick, montecarlo, careful",
        ),
        ("ai 0\n", "ai takes a player and an engine name: ai 0"),
    ] {
        assert_eq!(
            Parser::default()
                .parse_string(input, &mut Vec::new())
                .unwrap_err()
                .to_string(),
            expected
        );
    }
}

#[test]
fn cpp_debug_command_validates_category_and_reports_setting() {
    let mut output = Vec::new();
    Parser::default()
        .parse_string("debug SIMULATION 0\n", &mut output)
        .unwrap();
    assert_eq!(output, b"Debug SIMULATION set to 0\n");

    let mut output = Vec::new();
    Parser::default()
        .parse_string("debug SIMULATION -1\n", &mut output)
        .unwrap();
    assert_eq!(output, b"Debug SIMULATION set to 1\n");

    let mut output = Vec::new();
    Parser::default()
        .parse_string("debug ALWAYS 0\n", &mut output)
        .unwrap();
    assert!(output.is_empty());

    let error = Parser::default()
        .parse_string("debug simulation 0\n", &mut Vec::new())
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Could not find debug category: simulation"
    );
}

#[test]
fn random_and_maximize_attack_in_the_fight_and_decide_as_quick_elsewhere() {
    let fight = "game 1\nfight\nplayer 0 2 0\n6:5\n6:1\nplayer 1 1 0\n20:6\n";
    let preround = "game 1\npreround\nplayer 0 2 0\n6\nX\nplayer 1 1 0\n20\n";
    let action = |input: String| {
        let mut parser = Parser::default();
        let mut output = Vec::new();
        parser.parse_string(&input, &mut output).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(!output.contains("Valid Moves"), "{output}");
        parser.last_action().cloned()
    };
    let quick_swing = action(format!("{preround}ai 0 quick\ngetaction\n"));
    for engine in ["random", "maximize"] {
        assert_eq!(
            action(format!("{preround}ai 0 {engine}\ngetaction\n")),
            quick_swing,
            "{engine}"
        );
        assert!(matches!(
            action(format!("{fight}ai 0 {engine}\nseed 17\ngetaction\n")),
            Some(crate::protocol::ProtocolAction::Attack { .. })
        ));
    }
    assert_eq!(
        action(format!("{fight}ai 0 maximize\nseed 17\ngetaction\n")),
        Some(crate::protocol::ProtocolAction::Attack {
            attack_type: "skill",
            attackers: vec![0, 1],
            targets: vec![0],
            turbo: None,
            fire: Vec::new(),
        })
    );
}

#[test]
fn quick_engine_drives_getaction_without_search_output() {
    let input = "game 1\nfight\nplayer 0 2 0\n6:5\n6:1\nplayer 1 1 0\n20:6\nai 0 quick\nseed 17\ngetaction\nquit\n";
    let mut output = Vec::new();
    Parser::default().parse_string(input, &mut output).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(!output.contains("Valid Moves"), "{output}");
    assert!(output.ends_with("action\nskill\n0 1\n0\n"), "{output}");
}

#[test]
fn playfair_plays_the_selected_engines_and_reports_initiative_split_stats() {
    for engine in crate::engines::ENGINE_NAMES {
        let input = format!(
            "game 1\npreround\nplayer 0 1 0\n2\nplayer 1 1 0\n2\nai 0 {engine}\nai 1 {engine}\nseed 17\nplayfair 2\nquit\n"
        );
        let mut output = Vec::new();
        Parser::default().parse_string(&input, &mut output).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("PlayFairGames: 2 games\n"), "{output}");
        assert_eq!(output.matches(" stats: initiative ").count(), 4, "{output}");
    }
}

#[test]
fn cpp_game_parse_restores_default_ai_and_accepts_gameover_phase() {
    let input = "ai 0 quick\ngame 3\ngameover\nplayer 0 0 0\nplayer 1 0 0\nquit\n";
    let mut parser = Parser::default();
    parser.parse_string(input, &mut Vec::new()).unwrap();
    assert_eq!(parser.game.phase, Phase::Gameover);
    assert!(
        parser
            .player_engines
            .iter()
            .all(|engine| matches!(engine, PlayerEngine::Global))
    );
}

#[test]
fn cpp_game_simulation_commands_require_preround() {
    let prefix = "game 1\nfight\nplayer 0 1 0\n6:6\nplayer 1 1 0\n6:6\n";
    for command in ["playgame 1", "compare 1", "playfair 1"] {
        let error = Parser::default()
            .parse_string(&format!("{prefix}{command}\n"), &mut Vec::new())
            .unwrap_err();
        assert_eq!(error.to_string(), "Cannot PlayGame unless it is preround");
    }
}

#[test]
fn parses_twin_option_and_properties() {
    let twin = parse_die("p(4,4):4", 0).unwrap();
    assert!(twin.has_property(property::TWIN));
    assert!(twin.has_property(property::POISON));
    assert_eq!(twin.sides, [4, 4]);

    let first_option = parse_die("6/20-6:4", 1).unwrap();
    assert!(first_option.has_property(property::OPTION));
    assert_eq!(first_option.sides, [6, 20]);

    let second_option = parse_die("6/20-20:17", 2).unwrap();
    assert!(second_option.has_property(property::OPTION));
    assert_eq!(second_option.sides, [20, 6]);

    let mood_swing = parse_die("zU?-30", 3).unwrap();
    assert!(mood_swing.has_property(property::SPEED | property::MOOD));
    assert_eq!(mood_swing.sides[0], 30);

    let jolt = parse_die("J^6:3", 4).unwrap();
    assert!(jolt.has_property(property::JOLT | property::TIME_AND_SPACE));
}

#[test]
fn every_advertised_property_prefix_is_accepted_by_the_die_parser() {
    for notation in crate::protocol::notation::DIE_PROPERTY_PREFIXES {
        let recipe = format!("{}6", notation.token);
        let die = parse_die(&recipe, 0)
            .unwrap_or_else(|error| panic!("{} did not parse: {error}", notation.name));
        assert!(
            die.has_property(notation.property),
            "{} ({:?}) was not retained by {recipe}",
            notation.name,
            notation.token
        );
    }
}

#[test]
fn every_wire_phase_name_maps_to_the_expected_game_phase() {
    for (name, expected) in [
        ("aux", Phase::Auxiliary),
        ("preround", Phase::Preround),
        ("reserve", Phase::Reserve),
        ("initiative", Phase::Initiative),
        ("chance", Phase::Chance),
        ("focus", Phase::Focus),
        ("fight", Phase::Fight),
        ("gameover", Phase::Gameover),
    ] {
        let input = format!("game 5\n{name}\nplayer 0 0 0\nplayer 1 0 0\nquit\n");
        let mut parser = Parser::default();
        parser
            .parse_string(&input, &mut Vec::new())
            .unwrap_or_else(|error| panic!("{name} did not parse: {error}"));
        assert_eq!(parser.game.phase, expected, "{name}");
        assert_eq!(parser.game.target_wins, 5, "{name}");
    }
}

#[test]
fn auxiliary_qai_emits_the_buttonweavers_action_contract() {
    parser_scenario(
        "game 3\naux\nplayer 0 2 0\n6\n+20\nplayer 1 2 0\n6\n+4\nai 0 quick\ngetaction\nquit\n",
    )
    .expect_auxiliary(Some(1))
    .run();
}

#[test]
fn auxiliary_phase_adds_the_buttonweavers_courtesy_copy() {
    parser_scenario(
        "game 3\naux\nplayer 0 1 0\n6\nplayer 1 2 0\n6\n+12\nai 0 quick\ngetaction\nquit\n",
    )
    .expect_auxiliary(Some(1))
    .run();
}

#[test]
fn auxiliary_phase_declines_when_no_auxiliary_die_exists() {
    parser_scenario("game 3\naux\nplayer 0 1 0\n6\nplayer 1 1 0\n6\nai 0 quick\ngetaction\nquit\n")
        .expect_auxiliary(None)
        .run();
}

#[test]
fn auxiliary_phase_rejects_a_buttonweavers_invalid_second_auxiliary_die() {
    let error = Parser::default()
        .parse_string(
            "game 3\naux\nplayer 0 2 0\n+6\n+8\nplayer 1 1 0\n+6\nquit\n",
            &mut Vec::new(),
        )
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "player 0 has 2 Auxiliary dice; ButtonWeavers permits one"
    );
}

#[test]
fn native_auxiliary_search_is_worker_count_independent() {
    search_scenario()
        .phase(Auxiliary)
        .player(0, 0.0, ["6", "+M100"])
        .player(1, 0.0, ["6", "+pM100"])
        .ply(1)
        .simulations(10, 40)
        .max_branch(80)
        .modes([
            crate::search::test_support::NATIVE,
            crate::search::test_support::native(4),
        ])
        .expect_auxiliary(None)
        .run();
}

#[test]
fn current_value_and_focus_dizzy_suffix_are_parsed_explicitly() {
    let ready = parse_die("6:4", 2).unwrap();
    assert_eq!(ready.value, Some(4));
    assert!(!ready.not_set);
    assert!(!ready.dizzy);
    assert_eq!(ready.original_index, 2);

    let dizzy = parse_die("f12:7d", 4).unwrap();
    assert_eq!(dizzy.value, Some(7));
    assert!(!dizzy.not_set);
    assert!(dizzy.dizzy);
    assert_eq!(dizzy.original_index, 4);
}

#[test]
fn defined_swing_size_applies_to_every_swing_half_of_a_twin() {
    for (recipe, expected_sides) in [
        ("(T,T)-2:2", [2, 2]),
        ("(X,Y)-6:6", [6, 6]),
        ("(T,4)-2:2", [2, 4]),
        ("(4,T)-2:2", [4, 2]),
        ("(T,T)-2!:2", [2, 2]),
        ("(T,T)!-2:2", [2, 2]),
        ("(T,T)-2?:2", [2, 2]),
        ("(T,T)?-2:2", [2, 2]),
    ] {
        let die = parse_die(recipe, 0).unwrap();
        assert!(die.has_property(property::TWIN), "{recipe}");
        assert_eq!(die.sides, expected_sides, "{recipe}");
    }
}

#[test]
fn zero_does_not_lock_a_swing_definition() {
    let input = "game 3\npreround\nplayer 0 1 0\nT-0\nplayer 1 1 0\n6\nquit\n";
    let mut parser = Parser::default();
    parser.parse_string(input, &mut Vec::new()).unwrap();
    assert_eq!(parser.game.players[0].swing_set, SwingSet::Not);
}

#[test]
fn positive_defined_swing_and_option_values_lock_player_state() {
    for recipe in ["T-2", "(T,T)-2", "6/20-20"] {
        let input = format!("game 3\npreround\nplayer 0 1 0\n{recipe}\nplayer 1 1 0\n6\nquit\n");
        let mut parser = Parser::default();
        parser
            .parse_string(&input, &mut Vec::new())
            .unwrap_or_else(|error| panic!("{recipe} did not parse: {error}"));
        assert_eq!(
            parser.game.players[0].swing_set,
            SwingSet::Locked,
            "{recipe}"
        );
    }
}

#[test]
fn initiative_chance_and_focus_use_bmai_search() {
    for (phase, die) in [(Chance, "c10:10"), (Focus, "f10:10")] {
        search_scenario()
            .phase(phase)
            .player(0, 0.0, [die])
            .player(1, 0.0, ["6:6"])
            .ply(1)
            .simulations(1, 10)
            .max_branch(20)
            .modes([LEGACY])
            .expect_pass()
            .run();
    }
}

#[test]
fn typed_initiative_actions_use_original_die_indices() {
    let mut parser = Parser::default();
    parser
        .parse_string(
            "game 3\nfocus\nplayer 0 2 0\nf10:10\nc8:8\nplayer 1 1 0\n6:6\n",
            &mut Vec::new(),
        )
        .unwrap();

    assert_eq!(
        protocol_focus(
            &parser.game,
            &crate::search::FocusMove {
                values: vec![(0, 4)]
            }
        ),
        crate::protocol::ProtocolAction::Focus {
            dice: vec![crate::protocol::FocusSelection { die: 0, value: 4 }]
        }
    );
    assert_eq!(
        protocol_chance(&parser.game, &crate::search::ChanceMove { reroll: vec![1] }),
        crate::protocol::ProtocolAction::Chance { dice: vec![1] }
    );
}

#[test]
fn insult_fixture_emits_reference_protocol_action() {
    parser_scenario(include_str!("../../../tests/fixtures/Insult_in.txt"))
        .expect_attack(Power)
        .using([0])
        .targeting([1])
        .run();
}

#[test]
fn value_fixtures_emit_reference_protocol_actions() {
    let cases = [
        (
            include_str!("../../../tests/fixtures/Value1_in.txt"),
            Skill,
            vec![0, 1],
            vec![1],
        ),
        (
            include_str!("../../../tests/fixtures/Value2_in.txt"),
            Power,
            vec![1],
            vec![0],
        ),
    ];
    for (input, attack, attackers, targets) in cases {
        parser_scenario(input)
            .expect_attack(attack)
            .using(attackers)
            .targeting(targets)
            .run();
    }
}

#[test]
fn deterministic_fight_fixtures_emit_reference_protocol_actions() {
    parser_scenario(include_str!("../../../tests/fixtures/bug55_a_in.txt"))
        .expect_attack(Skill)
        .using([2, 0])
        .targeting([0])
        .run();
    parser_scenario(include_str!("../../../tests/fixtures/bug55_b_in.txt"))
        .expect_attack(Skill)
        .using([3, 0, 1])
        .targeting([2])
        .run();
    parser_scenario(include_str!("../../../tests/fixtures/bug105372_in.txt"))
        .expect_attack(Skill)
        .using([2, 1, 0])
        .targeting([0])
        .run();
    parser_scenario(include_str!(
        "../../../tests/fixtures/SurrenderOff-Attack-in.txt"
    ))
    .expect_attack(Power)
    .using([0])
    .targeting([0])
    .run();
}

#[test]
fn surrender_policy_fixtures_emit_reference_protocol_actions() {
    parser_scenario(include_str!(
        "../../../tests/fixtures/SurrenderDefault-Pass-in.txt"
    ))
    .expect_surrender()
    .run();
    parser_scenario(include_str!(
        "../../../tests/fixtures/SurrenderOff-Pass-in.txt"
    ))
    .expect_pass()
    .run();
    parser_scenario(include_str!(
        "../../../tests/fixtures/SurrenderOn-Attack-in.txt"
    ))
    .expect_surrender()
    .run();
    parser_scenario(include_str!(
        "../../../tests/fixtures/SurrenderOn-Pass-in.txt"
    ))
    .expect_surrender()
    .run();
}

#[test]
#[ignore = "full BMAI3 searches; run in the release parity suite"]
fn deeper_reference_fixtures_emit_reference_protocol_actions() {
    parser_scenario(include_str!("../../../tests/fixtures/bmai_in.txt"))
        .expect_attack(Power)
        .using([1])
        .targeting([0])
        .run();

    parser_scenario(include_str!("../../../tests/fixtures/bug11_in.txt"))
        .expect_swings([('T', 2), ('W', 4)])
        .run();
}

#[test]
fn obsolete_sims_command_is_rejected_like_reference_binary() {
    let input = include_str!("../../../tests/fixtures/test_in.txt");
    let error = Parser::default()
        .parse_string(input, &mut Vec::new())
        .unwrap_err();
    assert_eq!(error.to_string(), "unrecognized command: sims 150");
}

#[test]
fn rust_execution_and_rng_modes_are_independent_and_versioned() {
    let mut parser = Parser::default();
    assert_eq!(parser.execution_mode(), ExecutionMode::Native);
    assert_eq!(parser.rng_algorithm(), RngAlgorithm::LegacyParkMillerV1);

    let mut output = Vec::new();
    parser
        .parse_string(
            "mode native\nrng park-miller\nseed 17\nmode parity\nrng legacy\nquit\n",
            &mut output,
        )
        .unwrap();
    assert_eq!(parser.execution_mode(), ExecutionMode::Legacy);
    assert_eq!(parser.rng_replay_id(), "bmai-park-miller-16807-v1");
    assert_eq!(
        String::from_utf8(output).unwrap(),
        "Setting execution mode to native\n\
             Setting RNG to legacy (bmai-park-miller-16807-v1)\n\
             Seeding with 17\n\
             Setting execution mode to legacy\n\
             Setting RNG to legacy (bmai-park-miller-16807-v1)\n"
    );
}

#[test]
fn rust_execution_and_rng_modes_reject_unknown_values() {
    let engine = Parser::default()
        .parse_string("mode experimental\n", &mut Vec::new())
        .unwrap_err();
    assert_eq!(
        engine.to_string(),
        "invalid execution mode: experimental (expected legacy or native)"
    );

    let rng = Parser::default()
        .parse_string("rng xoshiro\n", &mut Vec::new())
        .unwrap_err();
    assert_eq!(
        rng.to_string(),
        "invalid RNG algorithm: xoshiro (expected legacy or park-miller)"
    );
}

#[test]
fn fire_overshooting_is_explicit_default_off_session_state() {
    let mut parser = Parser::default();
    assert!(!parser.game.fire_overshooting);

    let mut output = Vec::new();
    parser
        .parse_string(
            "fire_overshooting on\nfire_overshooting off\nquit\n",
            &mut output,
        )
        .unwrap();
    assert!(!parser.game.fire_overshooting);
    assert_eq!(
        String::from_utf8(output).unwrap(),
        "Setting Fire overshooting on\nSetting Fire overshooting off\n"
    );

    let error = parser
        .parse_string("fire_overshooting maybe\n", &mut Vec::new())
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "invalid fire_overshooting setting: maybe (expected on or off)"
    );
}

#[test]
fn native_worker_setting_validates_input_and_does_not_change_legacy_search() {
    let zero = Parser::default()
        .parse_string("workers 0\n", &mut Vec::new())
        .unwrap_err();
    assert_eq!(zero.to_string(), "native worker count must be at least 1");

    let malformed = Parser::default()
        .parse_string("workers many\n", &mut Vec::new())
        .unwrap_err();
    assert_eq!(malformed.to_string(), "invalid integer: many");

    let fixture = include_str!("../../../tests/native-fixtures/fight.txt")
        .replace("mode native", "mode legacy");
    let run = |workers: usize| {
        let input = fixture.replace("workers 3", &format!("workers {workers}"));
        let mut output = Vec::new();
        Parser::default().parse_string(&input, &mut output).unwrap();
        String::from_utf8(output).unwrap().replace(
            &format!("Setting native workers to {workers}"),
            "Setting native workers to N",
        )
    };
    assert_eq!(run(1), run(64));
}

#[test]
fn native_worker_auto_uses_available_logical_parallelism() {
    let expected = available_workers();
    let mut parser = Parser::default();
    let mut output = Vec::new();

    parser.parse_string("workers auto\n", &mut output).unwrap();

    assert_eq!(parser.session_metadata().workers, expected);
    assert_eq!(
        String::from_utf8(output).unwrap(),
        format!("Setting native workers to {expected} (auto)\n")
    );
}

#[test]
fn native_replay_index_advances_only_for_native_bmai_searches() {
    let fixture = include_str!("../../../tests/native-fixtures/fight.txt");

    let mut qai = Parser::default();
    qai.parse_string(
        &fixture.replace("getaction", "ai 0 quick\ngetaction"),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(qai.native_decision_index, 0);

    let mut bmai = Parser::default();
    bmai.parse_string(fixture, &mut Vec::new()).unwrap();
    assert_eq!(bmai.native_decision_index, 1);
}

#[test]
fn native_wire_fixtures_are_deterministic() {
    let cases = [
        (
            include_str!("../../../tests/native-fixtures/fight.txt"),
            include_str!("../../../tests/native-fixtures/fight.out.txt"),
        ),
        (
            include_str!("../../../tests/native-fixtures/reserve.txt"),
            include_str!("../../../tests/native-fixtures/reserve.out.txt"),
        ),
        (
            include_str!("../../../tests/native-fixtures/preround.txt"),
            include_str!("../../../tests/native-fixtures/preround.out.txt"),
        ),
        (
            include_str!("../../../tests/native-fixtures/chance.txt"),
            include_str!("../../../tests/native-fixtures/chance.out.txt"),
        ),
        (
            include_str!("../../../tests/native-fixtures/focus.txt"),
            include_str!("../../../tests/native-fixtures/focus.out.txt"),
        ),
    ];

    for (input, expected) in cases {
        // Windows checkouts may use CRLF; the writer always emits `\n`.
        let expected = expected.replace("\r\n", "\n");
        for _ in 0..2 {
            let mut output = Vec::new();
            Parser::default().parse_string(input, &mut output).unwrap();
            assert_eq!(String::from_utf8(output).unwrap(), expected);
        }
    }
}

#[test]
fn native_phases_are_worker_count_independent() {
    let run = |input: &str, workers: usize| {
        let input = input.replace("workers 3", &format!("workers {workers}"));
        let mut output = Vec::new();
        Parser::default().parse_string(&input, &mut output).unwrap();
        String::from_utf8(output).unwrap().replace(
            &format!("Setting native workers to {workers}"),
            "Setting native workers to N",
        )
    };
    let available = std::thread::available_parallelism().map_or(1, usize::from);
    for input in [
        include_str!("../../../tests/native-fixtures/reserve.txt"),
        include_str!("../../../tests/native-fixtures/preround.txt"),
        include_str!("../../../tests/native-fixtures/chance.txt"),
        include_str!("../../../tests/native-fixtures/focus.txt"),
    ] {
        let expected = run(input, 1);
        for workers in [2, available] {
            assert_eq!(run(input, workers), expected);
        }
    }
}

#[test]
#[ignore = "full default BMAI3 simulation; run in the release parity suite"]
fn simulation_fixture_emits_reference_match_result() {
    let input = include_str!("../../../tests/fixtures/bmsim_in.txt");
    let mut output = Vec::new();
    Parser::default().parse_string(input, &mut output).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(output.ends_with("matches over 12 - 8\n"), "{output}");
}

#[test]
#[ignore = "full reserve BMAI3 search; run in the release parity suite"]
fn reserve_fixture_emits_reference_protocol_action() {
    parser_scenario(include_str!("../../../tests/fixtures/bug16_in.txt"))
        .expect_reserve(Some(6))
        .run();
}

#[test]
fn a_request_without_settings_gets_the_measured_defaults() {
    let mut parser = Parser::default();
    let mut output = Vec::new();
    parser
        .parse_string(
            "game\nfight\nplayer 0 1 0\n6:6\nplayer 1 1 0\n6:3\ngetaction\nquit\n",
            &mut output,
        )
        .unwrap();
    let output = String::from_utf8(output).unwrap();
    assert_eq!(parser.execution_mode(), ExecutionMode::Native);
    assert!(!parser.game.fire_overshooting);
    assert!(output.contains("stats 1/200-4000/16000/0.50"), "{output}");
}
