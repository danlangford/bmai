// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

fn response(session: &mut BmairSession, request: Value) -> Value {
    serde_json::from_str(&session.handle_line(&request.to_string())).unwrap()
}

#[test]
fn capabilities_are_discoverable_without_mutating_the_session() {
    let value = response(
        &mut BmairSession::default(),
        json!({
            "protocol": "jsonl-v1",
            "id": "python-client",
            "method": "capabilities"
        }),
    );
    assert_eq!(value["id"], "python-client");
    assert_eq!(value["ok"], true);
    assert_eq!(value["result"]["protocols"][1], "jsonl-v1");
}

#[test]
fn rejected_scripts_are_transactional_and_the_session_recovers() {
    let mut session = BmairSession::default();
    let initial = response(
        &mut session,
        json!({
            "protocol": "jsonl-v1",
            "id": 0,
            "method": "session.execute",
            "params": { "script": "workers 4" }
        }),
    );
    assert_eq!(initial["result"]["session"]["workers"], 4);

    let rejected = response(
        &mut session,
        json!({
            "protocol": "jsonl-v1",
            "id": 1,
            "method": "session.execute",
            "params": { "script": "workers 2\nnot-a-command" }
        }),
    );
    assert_eq!(rejected["error"]["code"], "execution_error");
    assert_eq!(session.metadata().workers, 4);

    let accepted = response(
        &mut session,
        json!({
            "protocol": "jsonl-v1",
            "id": 2,
            "method": "session.execute",
            "params": { "script": "ply 2" }
        }),
    );
    assert_eq!(accepted["ok"], true);
    assert_eq!(accepted["result"]["session"]["workers"], 4);
    assert_eq!(accepted["result"]["session"]["max_ply"], 2);
}

#[test]
fn malformed_input_does_not_end_a_multi_request_stream() {
    let input = concat!(
        "not-json\n",
        "{\"protocol\":\"jsonl-v1\",\"id\":2,\"method\":\"capabilities\"}\n"
    );
    let mut output = Vec::new();
    run_jsonl(input.as_bytes(), &mut output).unwrap();
    let responses = String::from_utf8(output).unwrap();
    let values = responses
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(values.len(), 2);
    assert_eq!(values[0]["error"]["code"], "invalid_json");
    assert_eq!(values[1]["ok"], true);
}

#[test]
fn typed_actions_share_the_exact_legacy_execution_path() {
    let cases = [
        (
            "fight",
            include_str!("../../../tests/native-fixtures/fight.txt"),
            "attack",
        ),
        (
            "preround",
            include_str!("../../../tests/native-fixtures/preround.txt"),
            "set_swing",
        ),
        (
            "reserve",
            include_str!("../../../tests/native-fixtures/reserve.txt"),
            "reserve",
        ),
        (
            "chance",
            include_str!("../../../tests/native-fixtures/chance.txt"),
            "chance",
        ),
        (
            "focus",
            include_str!("../../../tests/native-fixtures/focus.txt"),
            "pass",
        ),
    ];

    for (id, script, action_type) in cases {
        let mut legacy_parser = Parser::default();
        let mut legacy_output = Vec::new();
        legacy_parser
            .parse_string(script, &mut legacy_output)
            .unwrap();

        let value = response(
            &mut BmairSession::default(),
            json!({
                "protocol": "jsonl-v1",
                "id": id,
                "method": "session.execute",
                "params": { "script": script }
            }),
        );
        assert_eq!(value["ok"], true, "{id}");
        assert_eq!(value["result"]["action"]["type"], action_type, "{id}");
        assert_eq!(
            value["result"]["legacy_output"],
            String::from_utf8(legacy_output).unwrap(),
            "{id}"
        );
    }
}

#[test]
fn native_search_reports_the_exact_decision_replay_key() {
    let value = response(
        &mut BmairSession::default(),
        json!({
            "protocol": "jsonl-v1",
            "id": "replay",
            "method": "session.execute",
            "params": { "script": include_str!("../../../tests/native-fixtures/fight.txt") }
        }),
    );
    assert_eq!(value["ok"], true);
    assert_eq!(
        value["result"]["replay"]["stream_partition"],
        crate::native::NATIVE_STREAM_PARTITION_ID
    );
    assert_eq!(value["result"]["replay"]["root_seed"], 17);
    assert_eq!(value["result"]["replay"]["decision_index"], 0);
    assert_eq!(value["result"]["session"]["native_decision_index"], 1);
}

#[test]
fn legacy_execution_does_not_claim_a_native_replay_key() {
    let script = include_str!("../../../tests/native-fixtures/fight.txt")
        .replace("mode native", "mode legacy");
    let value = response(
        &mut BmairSession::default(),
        json!({
            "protocol": "jsonl-v1",
            "id": "legacy",
            "method": "session.execute",
            "params": { "script": script }
        }),
    );
    assert_eq!(value["ok"], true);
    assert!(value["result"]["replay"].is_null());
    assert_eq!(value["result"]["session"]["native_decision_index"], 0);
}

#[test]
fn reset_restores_defaults_after_multiple_stateful_requests() {
    let mut session = BmairSession::default();
    let changed = response(
        &mut session,
        json!({
            "protocol": "jsonl-v1",
            "id": 1,
            "method": "session.execute",
            "params": { "script": "mode native\nworkers 4\nseed 17\nply 2\n" }
        }),
    );
    assert_eq!(changed["result"]["session"]["execution_mode"], "native");
    assert_eq!(changed["result"]["session"]["workers"], 4);
    assert_eq!(changed["result"]["session"]["max_ply"], 2);
    assert_eq!(
        changed["result"]["session"]["players"][0]["engine"],
        "montecarlo"
    );
    assert_eq!(
        changed["result"]["session"]["players"][0]["montecarlo"]["max_ply"],
        2
    );
    assert_eq!(
        changed["result"]["session"]["players"][0]["specials"],
        json!([])
    );
    assert_eq!(
        changed["result"]["build"]["version"],
        env!("BMAIR_BUILD_VERSION")
    );

    let reset = response(
        &mut session,
        json!({
            "protocol": "jsonl-v1",
            "id": 2,
            "method": "session.reset"
        }),
    );
    assert_eq!(reset["result"]["session"]["execution_mode"], "legacy");
    assert_eq!(reset["result"]["session"]["workers"], 1);
    assert_eq!(reset["result"]["session"]["max_ply"], 1);
}

#[test]
fn session_metadata_reports_only_each_players_own_specials() {
    let mut session = BmairSession::default();
    let value = response(
        &mut session,
        json!({
            "protocol": "jsonl-v1",
            "id": 1,
            "method": "session.execute",
            "params": { "script": "game\nfight\nplayer 0 1 0\n6:6\nplayer 1 1 0\n4:4\nspecial 1 skill_immune no_initiative\n" }
        }),
    );
    let players = &value["result"]["session"]["players"];
    assert_eq!(players[0]["specials"], json!([]));
    assert_eq!(
        players[1]["specials"],
        json!(["skill_immune", "no_initiative"])
    );
}

#[test]
fn session_metadata_reports_each_players_engine_and_settings() {
    let mut session = BmairSession::default();
    let game = "game\nfight\nplayer 0 1 0\n6:6\nplayer 1 1 0\n4:4\n";
    let value = response(
        &mut session,
        json!({
            "protocol": "jsonl-v1",
            "id": 1,
            "method": "session.execute",
            "params": { "script": format!("max_sims 3\n{game}max_sims 1 7\ncull 1 off\nai 0 quick\n") }
        }),
    );
    let session_state = &value["result"]["session"];
    assert_eq!(session_state["max_simulations"], 3);
    assert_eq!(session_state["cull"], true);
    assert_eq!(session_state["players"][0]["engine"], "quick");
    assert_eq!(session_state["players"][0]["montecarlo"], Value::Null);
    let player_1 = &session_state["players"][1];
    assert_eq!(player_1["engine"], "montecarlo");
    assert_eq!(player_1["montecarlo"]["max_simulations"], 7);
    assert_eq!(player_1["montecarlo"]["cull"], false);
}

#[test]
fn request_validation_has_stable_recoverable_error_codes() {
    let cases = [
        (
            json!({"protocol":"jsonl-v0","id":1,"method":"capabilities"}),
            "unsupported_protocol",
        ),
        (
            json!({"protocol":"jsonl-v1","id":[],"method":"capabilities"}),
            "invalid_request",
        ),
        (
            json!({"protocol":"jsonl-v1","id":2,"method":"missing"}),
            "method_not_found",
        ),
        (
            json!({"protocol":"jsonl-v1","id":3,"method":"capabilities","params":{"extra":true}}),
            "invalid_params",
        ),
        (
            json!({"protocol":"jsonl-v1","id":4,"method":"session.execute","params":{}}),
            "invalid_params",
        ),
    ];
    let mut session = BmairSession::default();
    for (request, code) in cases {
        let value = response(&mut session, request);
        assert_eq!(value["ok"], false, "{code}");
        assert_eq!(value["error"]["code"], code);
        assert_eq!(value["error"]["recoverable"], true);
    }

    let schema_error: Value = serde_json::from_str(
        &session
            .handle_line(r#"{"protocol":"jsonl-v1","id":5,"method":"capabilities","extra":true}"#),
    )
    .unwrap();
    assert_eq!(schema_error["error"]["code"], "invalid_request");
    assert_eq!(schema_error["id"], 5);
}

#[test]
fn surrender_and_turbo_are_captured_from_the_legacy_action() {
    let cases = [
        (
            include_str!("../../../tests/fixtures/SurrenderDefault-Pass-in.txt"),
            "surrender",
            None,
        ),
        (
            include_str!("../../../tests/fixtures/parity_turbo_option_in.txt"),
            "attack",
            Some("option"),
        ),
        (
            include_str!("../../../tests/fixtures/parity_turbo_swing_in.txt"),
            "attack",
            Some("swing"),
        ),
    ];
    for (script, action_type, turbo_kind) in cases {
        let result = BmairSession::default().execute(script).unwrap();
        let action = serde_json::to_value(result.action).unwrap();
        assert_eq!(action["type"], action_type);
        match turbo_kind {
            Some(kind) => assert_eq!(action["turbo"]["kind"], kind),
            None => assert!(action.get("turbo").is_none()),
        }
    }
}

#[test]
fn native_typed_actions_and_replay_keys_are_worker_count_independent() {
    let available = std::thread::available_parallelism().map_or(1, usize::from);
    for fixture in [
        include_str!("../../../tests/native-fixtures/fight.txt"),
        include_str!("../../../tests/native-fixtures/preround.txt"),
        include_str!("../../../tests/native-fixtures/reserve.txt"),
        include_str!("../../../tests/native-fixtures/chance.txt"),
        include_str!("../../../tests/native-fixtures/focus.txt"),
    ] {
        let run = |workers: usize| {
            let script = fixture.replace("workers 3", &format!("workers {workers}"));
            let result = BmairSession::default().execute(&script).unwrap();
            (result.action, result.replay)
        };
        let expected = run(1);
        assert_eq!(run(2), expected);
        assert_eq!(run(available), expected);
    }
}

#[test]
fn selected_move_report_is_structured_and_does_not_change_the_action() {
    let position = "mode native\nworkers 2\ngame 3\nfight\nplayer 0 1 20\npX!-20:5\nplayer 1 3 56.5\n20:19\nfsv13:5\ngvz30:6\nply 3\nmax_sims 100\nmin_sims 5\nmaxbranch 400\nsurrender off\nseed 1\n";
    let baseline = BmairSession::default()
        .execute(&format!("{position}getaction\n"))
        .unwrap();
    let reported = BmairSession::default()
        .execute(&format!("{position}report_sims 20\ngetaction\n"))
        .unwrap();

    assert_eq!(reported.action, baseline.action);
    assert!(baseline.legacy_output.contains("l1 p0 best move ("));
    assert!(!baseline.legacy_output.contains("selected move report"));
    let best_line = reported
        .legacy_output
        .find("l1 p0 best move (")
        .expect("historical move-selection diagnostic");
    let report_line = reported
        .legacy_output
        .find("l1 p0 selected move report (14.0/20, 70.0% win)")
        .expect("fresh selected-move diagnostic");
    assert!(best_line < report_line);
    let evaluation = reported.evaluation.unwrap();
    assert_eq!(evaluation.player, 0);
    assert_eq!(evaluation.simulations, 20);
    assert_eq!(evaluation.source, "selected_move_resample");
    assert_eq!(evaluation.probability, crate::ProtocolFloat::Finite(0.7));
}

#[test]
fn game_120810_selected_move_report_captures_the_fifty_fifty_endgame() {
    let result = BmairSession::default()
        .execute(
            "mode native\nworkers 2\ngame 3\nfight\nplayer 0 3 12\n8:6\nV?-12:4\npX!-20:19\nplayer 1 2 44\nz6:5\nzT-2:2\nply 3\nmax_sims 100\nmin_sims 5\nmaxbranch 400\nreport_sims 20\nsurrender off\nseed 1\ngetaction\n",
        )
        .unwrap();

    let evaluation = result.evaluation.unwrap();
    assert_eq!(evaluation.simulations, 20);
    assert_eq!(evaluation.probability, crate::ProtocolFloat::Finite(0.5));
}

#[test]
fn phase_specific_declines_have_unambiguous_typed_results() {
    let reserve = BmairSession::default()
        .execute("game\nreserve\nplayer 0 1 0\n6\nplayer 1 1 0\n6\nai 0 quick\ngetaction\n")
        .unwrap();
    assert_eq!(
        reserve.action,
        Some(crate::protocol::ProtocolAction::Reserve { die: None })
    );

    let preround = BmairSession::default()
        .execute("game\npreround\nplayer 0 1 0\n6\nplayer 1 1 0\n6\nai 0 quick\ngetaction\n")
        .unwrap();
    assert_eq!(preround.action, Some(crate::protocol::ProtocolAction::Pass));
}
