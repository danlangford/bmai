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
        .ParseString(input, &mut Vec::new())
        .unwrap();
}

#[test]
fn streamed_legacy_commands_match_batched_parsing() {
    let input = "game\nfight\nplayer 0 1 1\n1:1\nplayer 1 2 30\n1:1\n(30,30):60\nseed 17\nsurrender off\ngetaction\nquit\n";
    let mut batched = Parser::default();
    let mut batched_output = Vec::new();
    batched.ParseString(input, &mut batched_output).unwrap();

    let mut streamed = Parser::default();
    let mut streamed_output = Vec::new();
    streamed
        .ParseStream(&mut std::io::Cursor::new(input), &mut streamed_output)
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

    Parser::default().ParseString(input, &mut output).unwrap();

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

    Parser::default().ParseString(input, &mut output).unwrap();

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
        parser.ParseString(input, &mut output).unwrap();
        (output, parser.session_metadata())
    };
    let parse_streamed = |input: &str| {
        let mut parser = Parser::default();
        let mut output = Vec::new();
        parser
            .ParseStream(&mut std::io::Cursor::new(input), &mut output)
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
        .ParseString(input, &mut Vec::new())
        .unwrap_err();
    let streamed = Parser::default()
        .ParseStream(&mut std::io::Cursor::new(input), &mut Vec::new())
        .unwrap_err();

    assert_eq!(batched.to_string(), "phase not found");
    assert_eq!(streamed.to_string(), batched.to_string());
}

#[test]
fn inline_comments_remain_invalid() {
    let error = Parser::default()
        .ParseString("seed 17 # comment\n", &mut Vec::new())
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
            .ParseStream(&mut std::io::Cursor::new(input), &mut Vec::new())
            .unwrap_err();
        assert_eq!(error.to_string(), expected);
    }
}

#[test]
fn game_rejects_more_than_ten_input_dice_in_both_parsing_paths() {
    let input = "game\nfight\nplayer 0 11 0\n";
    let expected = "player dice count 11 exceeds maximum 10";

    let batched = Parser::default()
        .ParseString(input, &mut Vec::new())
        .unwrap_err();
    let streamed = Parser::default()
        .ParseStream(&mut std::io::Cursor::new(input), &mut Vec::new())
        .unwrap_err();

    assert_eq!(batched.to_string(), expected);
    assert_eq!(streamed.to_string(), expected);
}

#[test]
fn cpp_parser_ai_and_per_player_search_settings() {
    let input = "ai 0 2\nply 0 3\nmax_sims 0 40\nmin_sims 0 4\nmaxbranch 0 400\ndebugply 2\nseed 17\nquit\n";
    let mut parser = Parser::default();
    let mut output = Vec::new();
    parser.ParseString(input, &mut output).unwrap();
    assert_eq!(parser.AIType(0), 2);
    assert_eq!(parser.PlayerAI(0).m_max_ply, 3);
    assert_eq!(parser.PlayerAI(0).m_max_sims, 40);
    assert_eq!(parser.PlayerAI(0).m_min_sims, 4);
    assert_eq!(parser.PlayerAI(0).m_max_branch, 400);
    assert_eq!(parser.m_debug_ply, 2);
    assert_eq!(
        String::from_utf8(output).unwrap(),
        "Setting AI for player 0 to type 2\n\
Setting max ply for player 0 to 3\n\
Setting max sims for player 0 to 40\n\
Setting min sims for player 0 to 4\n\
Setting max branch for player 0 to 400\n\
Setting debug ply to 2\n\
Seeding with 17\n"
    );
}

/// Expected lines come from running each case through the C++ reference `4813530`.
#[test]
fn per_player_settings_change_the_shared_cpp_ai_object() {
    const GAME: &str = "game\nfight\nplayer 0 3 0\n6:6\n4:2\n8:5\nplayer 1 3 0\n4:4\n3:3\n10:7\n";
    let cases: [(&str, String, &[&str]); 7] = [
        (
            "a player-1 setting after game also changes player 0 and the global stats",
            format!("{GAME}max_sims 1 3\ngetaction\n"),
            &["l1 p0 Valid Moves 5 Sims 3", "stats 1/10-3/5000/0.50"],
        ),
        (
            "an ai type keeps its own defaults instead of the global settings",
            format!("ply 2\nmin_sims 2\n{GAME}ai 0 2\ngetaction\n"),
            &["l1 p0 Valid Moves 5 Sims 500", "stats 2/2-500/5000/0.50"],
        ),
        (
            "game restores g_ai but the ai type object keeps its settings",
            format!("min_sims 2\n{GAME}ai 0 2\nmax_sims 0 3\n{GAME}getaction\nai 0 2\ngetaction\n"),
            &[
                "l1 p0 Valid Moves 5 Sims 500",
                "stats 1/2-500/5000/0.50",
                "l1 p0 Valid Moves 5 Sims 3",
                "stats 1/2-500/5000/0.50",
            ],
        ),
        (
            "players with the same ai type share one object",
            format!("min_sims 2\n{GAME}ai 0 0\nai 1 0\nmax_sims 1 3\ngetaction\n"),
            &["l1 p0 Valid Moves 5 Sims 3", "stats 1/2-500/5000/0.50"],
        ),
        (
            "QAI ignores per-player search settings silently",
            format!("min_sims 2\n{GAME}ai 0 1\nmax_sims 0 3\ngetaction\n"),
            &["stats 1/2-500/5000/0.50"],
        ),
        (
            "global settings change only g_ai",
            format!("min_sims 2\n{GAME}ai 0 2\nmax_sims 7\ngetaction\n"),
            &["l1 p0 Valid Moves 5 Sims 500", "stats 1/2-7/5000/0.50"],
        ),
        (
            "an ai type selected before game keeps settings made before game",
            format!("min_sims 2\nai 0 2\nmax_sims 0 3\n{GAME}ai 0 2\ngetaction\n"),
            &["l1 p0 Valid Moves 5 Sims 3", "stats 1/2-500/5000/0.50"],
        ),
    ];
    for (name, input, expected) in cases {
        let mut output = Vec::new();
        Parser::default()
            .ParseString(&format!("seed 17\n{input}"), &mut output)
            .unwrap();
        let output = String::from_utf8(output).unwrap();
        let observed = output
            .lines()
            .filter(|line| line.contains("Valid Moves") || line.starts_with("stats "))
            .collect::<Vec<_>>();
        assert_eq!(observed, expected, "{name}\n{output}");
        assert!(
            !(name.starts_with("QAI") && output.contains("max sims for player")),
            "{name}\n{output}"
        );
    }
}

#[test]
fn cpp_parser_rejects_invalid_ai_selection() {
    for (input, expected) in [
        ("ai 2 0\n", "invalid setting for ai player number: 2"),
        ("ai 0 3\n", "invalid setting for ai type: 3"),
        ("ai 2 3\n", "invalid setting for ai type: 3"),
    ] {
        assert_eq!(
            Parser::default()
                .ParseString(input, &mut Vec::new())
                .unwrap_err()
                .to_string(),
            expected
        );
    }
}

#[test]
fn cpp_playfair_out_of_range_mode_retains_current_ai() {
    let input = "game 1\npreround\nplayer 0 1 0\n6\nplayer 1 1 0\n6\nplayfair 0 4 0.5\nquit\n";
    let mut output = Vec::new();
    Parser::default().ParseString(input, &mut output).unwrap();
    assert!(
        String::from_utf8(output)
            .unwrap()
            .contains("PlayFairGames: 0 games, mode 4")
    );
}

#[test]
fn cpp_debug_command_validates_category_and_reports_setting() {
    let mut output = Vec::new();
    Parser::default()
        .ParseString("debug SIMULATION 0\n", &mut output)
        .unwrap();
    assert_eq!(output, b"Debug SIMULATION set to 0\n");

    let mut output = Vec::new();
    Parser::default()
        .ParseString("debug SIMULATION -1\n", &mut output)
        .unwrap();
    assert_eq!(output, b"Debug SIMULATION set to 1\n");

    let mut output = Vec::new();
    Parser::default()
        .ParseString("debug ALWAYS 0\n", &mut output)
        .unwrap();
    assert!(output.is_empty());

    let error = Parser::default()
        .ParseString("debug simulation 0\n", &mut Vec::new())
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Could not find debug category: simulation"
    );
}

#[test]
fn cpp_qai_selection_drives_getaction_and_ignores_bmai_settings() {
    let input = "game 1\nfight\nplayer 0 2 0\n6:5\n6:1\nplayer 1 1 0\n20:6\nai 0 1\nmax_sims 0 5\nmin_sims 0 1\nmaxbranch 0 20\nseed 17\ngetaction\nquit\n";
    let mut output = Vec::new();
    Parser::default().ParseString(input, &mut output).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(!output.contains("Setting max sims for player"), "{output}");
    assert!(!output.contains("Valid Moves"), "{output}");
    assert!(output.ends_with("action\nskill\n0 1\n0\n"), "{output}");
}

#[test]
fn cpp_playfair_modes_report_initiative_split_stats() {
    for mode in 0..=3 {
        let input = format!(
            "game 1\npreround\nplayer 0 1 0\n2\nplayer 1 1 0\n2\nseed 17\nplayfair 2 {mode} 0.5\nquit\n"
        );
        let mut output = Vec::new();
        Parser::default().ParseString(&input, &mut output).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(
            output.contains(&format!("PlayFairGames: 2 games, mode {mode}, p 0.500000")),
            "{output}"
        );
        assert_eq!(output.matches(" stats: initiative ").count(), 4, "{output}");
    }
}

#[test]
fn cpp_game_parse_restores_default_ai_and_accepts_gameover_phase() {
    let input = "ai 0 1\ngame 3\ngameover\nplayer 0 0 0\nplayer 1 0 0\nquit\n";
    let mut parser = Parser::default();
    parser.ParseString(input, &mut Vec::new()).unwrap();
    assert_eq!(parser.m_game.m_phase, Phase::Gameover);
    assert_eq!(parser.m_player_ai, [AiSlot::Global; 2]);
}

#[test]
fn cpp_game_simulation_commands_require_preround() {
    let prefix = "game 1\nfight\nplayer 0 1 0\n6:6\nplayer 1 1 0\n6:6\n";
    for command in ["playgame 1", "compare 1", "playfair 1 0 0.5"] {
        let error = Parser::default()
            .ParseString(&format!("{prefix}{command}\n"), &mut Vec::new())
            .unwrap_err();
        assert_eq!(error.to_string(), "Cannot PlayGame unless it is preround");
    }
}

#[test]
fn parses_twin_option_and_properties() {
    let twin = ParseDie("p(4,4):4", 0).unwrap();
    assert!(twin.HasProperty(property::TWIN));
    assert!(twin.HasProperty(property::POISON));
    assert_eq!(twin.m_sides, [4, 4]);

    let first_option = ParseDie("6/20-6:4", 1).unwrap();
    assert!(first_option.HasProperty(property::OPTION));
    assert_eq!(first_option.m_sides, [6, 20]);

    let second_option = ParseDie("6/20-20:17", 2).unwrap();
    assert!(second_option.HasProperty(property::OPTION));
    assert_eq!(second_option.m_sides, [20, 6]);

    let mood_swing = ParseDie("zU?-30", 3).unwrap();
    assert!(mood_swing.HasProperty(property::SPEED | property::MOOD));
    assert_eq!(mood_swing.m_sides[0], 30);

    let jolt = ParseDie("J^6:3", 4).unwrap();
    assert!(jolt.HasProperty(property::JOLT | property::TIME_AND_SPACE));
}

#[test]
fn every_advertised_property_prefix_is_accepted_by_the_die_parser() {
    for notation in crate::protocol::notation::DIE_PROPERTY_PREFIXES {
        let recipe = format!("{}6", notation.token);
        let die = ParseDie(&recipe, 0)
            .unwrap_or_else(|error| panic!("{} did not parse: {error}", notation.name));
        assert!(
            die.HasProperty(notation.property),
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
            .ParseString(&input, &mut Vec::new())
            .unwrap_or_else(|error| panic!("{name} did not parse: {error}"));
        assert_eq!(parser.m_game.m_phase, expected, "{name}");
        assert_eq!(parser.m_game.m_target_wins, 5, "{name}");
    }
}

#[test]
fn auxiliary_qai_emits_the_buttonweavers_action_contract() {
    parser_scenario(
        "game 3\naux\nplayer 0 2 0\n6\n+20\nplayer 1 2 0\n6\n+4\nai 0 1\ngetaction\nquit\n",
    )
    .expect_auxiliary(Some(1))
    .run();
}

#[test]
fn auxiliary_phase_adds_the_buttonweavers_courtesy_copy() {
    parser_scenario(
        "game 3\naux\nplayer 0 1 0\n6\nplayer 1 2 0\n6\n+12\nai 0 1\ngetaction\nquit\n",
    )
    .expect_auxiliary(Some(1))
    .run();
}

#[test]
fn auxiliary_phase_declines_when_no_auxiliary_die_exists() {
    parser_scenario("game 3\naux\nplayer 0 1 0\n6\nplayer 1 1 0\n6\nai 0 1\ngetaction\nquit\n")
        .expect_auxiliary(None)
        .run();
}

#[test]
fn auxiliary_phase_rejects_a_buttonweavers_invalid_second_auxiliary_die() {
    let error = Parser::default()
        .ParseString(
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
    let ready = ParseDie("6:4", 2).unwrap();
    assert_eq!(ready.m_value_total, Some(4));
    assert!(!ready.m_notset);
    assert!(!ready.m_dizzy);
    assert_eq!(ready.m_original_index, 2);

    let dizzy = ParseDie("f12:7d", 4).unwrap();
    assert_eq!(dizzy.m_value_total, Some(7));
    assert!(!dizzy.m_notset);
    assert!(dizzy.m_dizzy);
    assert_eq!(dizzy.m_original_index, 4);
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
        let die = ParseDie(recipe, 0).unwrap();
        assert!(die.HasProperty(property::TWIN), "{recipe}");
        assert_eq!(die.m_sides, expected_sides, "{recipe}");
    }
}

#[test]
fn zero_does_not_lock_a_swing_definition() {
    let input = "game 3\npreround\nplayer 0 1 0\nT-0\nplayer 1 1 0\n6\nquit\n";
    let mut parser = Parser::default();
    parser.ParseString(input, &mut Vec::new()).unwrap();
    assert_eq!(parser.m_game.m_player[0].m_swing_set, SwingSet::Not);
}

#[test]
fn positive_defined_swing_and_option_values_lock_player_state() {
    for recipe in ["T-2", "(T,T)-2", "6/20-20"] {
        let input = format!("game 3\npreround\nplayer 0 1 0\n{recipe}\nplayer 1 1 0\n6\nquit\n");
        let mut parser = Parser::default();
        parser
            .ParseString(&input, &mut Vec::new())
            .unwrap_or_else(|error| panic!("{recipe} did not parse: {error}"));
        assert_eq!(
            parser.m_game.m_player[0].m_swing_set,
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
        .ParseString(
            "game 3\nfocus\nplayer 0 2 0\nf10:10\nc8:8\nplayer 1 1 0\n6:6\n",
            &mut Vec::new(),
        )
        .unwrap();

    assert_eq!(
        protocol_focus(
            &parser.m_game,
            &crate::search::FocusMove {
                values: vec![(0, 4)]
            }
        ),
        crate::protocol::ProtocolAction::Focus {
            dice: vec![crate::protocol::FocusSelection { die: 0, value: 4 }]
        }
    );
    assert_eq!(
        protocol_chance(
            &parser.m_game,
            &crate::search::ChanceMove { reroll: vec![1] }
        ),
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
        .ParseString(input, &mut Vec::new())
        .unwrap_err();
    assert_eq!(error.to_string(), "unrecognized command: sims 150");
}

#[test]
fn rust_execution_and_rng_modes_are_independent_and_versioned() {
    let mut parser = Parser::default();
    assert_eq!(parser.execution_mode(), ExecutionMode::Legacy);
    assert_eq!(parser.rng_algorithm(), RngAlgorithm::LegacyParkMillerV1);

    let mut output = Vec::new();
    parser
        .ParseString(
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
        .ParseString("mode experimental\n", &mut Vec::new())
        .unwrap_err();
    assert_eq!(
        engine.to_string(),
        "invalid execution mode: experimental (expected legacy or native)"
    );

    let rng = Parser::default()
        .ParseString("rng xoshiro\n", &mut Vec::new())
        .unwrap_err();
    assert_eq!(
        rng.to_string(),
        "invalid RNG algorithm: xoshiro (expected legacy or park-miller)"
    );
}

#[test]
fn fire_overshooting_is_explicit_default_off_session_state() {
    let mut parser = Parser::default();
    assert!(!parser.m_game.m_fire_overshooting);

    let mut output = Vec::new();
    parser
        .ParseString(
            "fire_overshooting on\nfire_overshooting off\nquit\n",
            &mut output,
        )
        .unwrap();
    assert!(!parser.m_game.m_fire_overshooting);
    assert_eq!(
        String::from_utf8(output).unwrap(),
        "Setting Fire overshooting on\nSetting Fire overshooting off\n"
    );

    let error = parser
        .ParseString("fire_overshooting maybe\n", &mut Vec::new())
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "invalid fire_overshooting setting: maybe (expected on or off)"
    );
}

#[test]
fn native_worker_setting_validates_input_and_does_not_change_legacy_search() {
    let zero = Parser::default()
        .ParseString("workers 0\n", &mut Vec::new())
        .unwrap_err();
    assert_eq!(zero.to_string(), "native worker count must be at least 1");

    let malformed = Parser::default()
        .ParseString("workers many\n", &mut Vec::new())
        .unwrap_err();
    assert_eq!(malformed.to_string(), "invalid integer: many");

    let fixture = include_str!("../../../tests/native-fixtures/fight.txt")
        .replace("mode native", "mode legacy");
    let run = |workers: usize| {
        let input = fixture.replace("workers 3", &format!("workers {workers}"));
        let mut output = Vec::new();
        Parser::default().ParseString(&input, &mut output).unwrap();
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

    parser.ParseString("workers auto\n", &mut output).unwrap();

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
    qai.ParseString(
        &fixture.replace("getaction", "ai 0 1\ngetaction"),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(qai.m_native_decision_index, 0);

    let mut bmai = Parser::default();
    bmai.ParseString(fixture, &mut Vec::new()).unwrap();
    assert_eq!(bmai.m_native_decision_index, 1);
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
            Parser::default().ParseString(input, &mut output).unwrap();
            assert_eq!(String::from_utf8(output).unwrap(), expected);
        }
    }
}

#[test]
fn native_phases_are_worker_count_independent() {
    let run = |input: &str, workers: usize| {
        let input = input.replace("workers 3", &format!("workers {workers}"));
        let mut output = Vec::new();
        Parser::default().ParseString(&input, &mut output).unwrap();
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
    Parser::default().ParseString(input, &mut output).unwrap();
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
