// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::Value;

mod common;

#[test]
fn jsonl_process_keeps_stdout_machine_clean_and_recovers_per_line() {
    let mut child = common::bmair()
        .args(["--protocol", "jsonl-v1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(
            concat!(
                "not-json\n",
                "{\"protocol\":\"jsonl-v1\",\"id\":\"caps\",\"method\":\"capabilities\"}\n",
                "{\"protocol\":\"jsonl-v1\",\"id\":\"state\",\"method\":\"session.execute\",\"params\":{\"script\":\"workers 2\"}}\n"
            )
            .as_bytes(),
        )
        .unwrap();
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let responses = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(responses.len(), 3);
    assert_eq!(responses[0]["error"]["code"], "invalid_json");
    assert_eq!(responses[1]["id"], "caps");
    assert_eq!(responses[1]["result"]["implementation"], "bmair");
    assert_eq!(responses[2]["id"], "state");
    assert_eq!(responses[2]["result"]["session"]["workers"], 2);

    let response_bytes = responses_to_bytes(&responses);
    let stdout = String::from_utf8_lossy(&response_bytes);
    assert!(!stdout.contains("Copyright"));
    assert!(!stdout.contains("Version:"));
}

#[test]
fn max_sims_below_the_default_min_sims_searches_instead_of_panicking() {
    // The fixture was written for these settings, not BMAIR's defaults.
    let script = format!(
        "surrender on\nendgame 0\nply 1\nmax_sims 500\nmin_sims 10\nmaxbranch 5000\n{}",
        include_str!("fixtures/parity_min_sims_exceeds_max_sims_in.txt")
    );
    let request = serde_json::json!({
        "protocol": "jsonl-v1",
        "id": "low-max",
        "method": "session.execute",
        "params": {"script": script},
    });
    let mut child = common::bmair()
        .args(["--protocol", "jsonl-v1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(child.stdin.as_mut().unwrap(), "{request}").unwrap();
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(response["ok"], true, "{response}");
    assert_eq!(response["result"]["action"]["type"], "attack");
    assert_eq!(response["result"]["action"]["attack_type"], "skill");
    assert!(
        response["result"]["legacy_output"]
            .as_str()
            .unwrap()
            .contains("stats 1/10-5/5000/0.50")
    );
}

#[test]
fn playgame_results_stay_inside_the_json_response() {
    let request = serde_json::json!({
        "protocol": "jsonl-v1",
        "id": "play",
        "method": "session.execute",
        // Null dice can only tie, so the game is cancelled whatever the seed.
        "params": {"script": "game 1\npreround\nplayer 0 2 0\nn4\nn4\nplayer 1 2 0\nn4\nn4\nai 0 quick\nai 1 quick\nplaygame 1\n"},
    });
    let mut child = common::bmair()
        .args(["--protocol", "jsonl-v1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(child.stdin.as_mut().unwrap(), "{request}").unwrap();
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    let response: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(response["id"], "play");
    assert!(
        response["result"]["legacy_output"]
            .as_str()
            .unwrap()
            .ends_with("\ngame cancelled 0 - 0 - 199\nmatches over 0 - 0\n"),
        "{response}"
    );
}

#[test]
fn documented_jsonl_session_fixture_runs_as_one_persistent_process() {
    let mut child = common::bmair()
        .args(["--protocol", "jsonl-v1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(include_bytes!("jsonl-fixtures/session.jsonl"))
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let responses = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(responses.len(), 5);
    assert_eq!(responses[0]["id"], "caps");
    assert_eq!(responses[1]["error"]["code"], "invalid_json");
    assert_eq!(responses[2]["id"], "configure");
    assert_eq!(responses[3]["id"], "fight");
    assert_eq!(responses[3]["result"]["action"]["type"], "attack");
    assert_eq!(responses[3]["result"]["replay"]["root_seed"], 17);
    assert_eq!(responses[4]["id"], "reset");
    assert_eq!(
        responses[4]["result"]["session"]["native_decision_index"],
        0
    );
}

#[test]
fn capabilities_flag_emits_one_json_document_without_a_banner() {
    let output = common::bmair().arg("--capabilities").output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["implementation"], "bmair");
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("Copyright")
    );
}

#[test]
fn unsupported_process_protocol_is_a_clean_fatal_cli_error() {
    let output = common::bmair()
        .args(["--protocol", "jsonl-v0"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "unsupported protocol: jsonl-v0\n"
    );
}

#[test]
fn the_hash_trace_restarts_with_the_session() {
    let reset = serde_json::json!({"protocol": "jsonl-v1", "id": 2, "method": "session.reset"});
    let output = run_session(
        common::bmair().env("BMAIR_TRACE_RNG_HASH", "1"),
        &[execute(1, PLAYGAME), reset, execute(3, PLAYGAME)],
    );
    let responses = strict_responses(&output.stdout);
    assert_eq!(responses.len(), 3);
    assert!(
        responses.iter().all(|response| response["ok"] == true),
        "{responses:?}"
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    let games = stderr
        .lines()
        .filter(|line| line.starts_with("RNG_HASH ") && !line.starts_with("RNG_HASH 0 "))
        .collect::<Vec<_>>();
    assert_eq!(games.len(), 2, "{stderr}");
    assert_eq!(games[0], games[1]);
}

const PLAYGAME: &str = "game 3\npreround\nplayer 0 2 0\n6\n8\nplayer 1 2 0\n4\n10\nai 0 quick\nai 1 quick\nplaygame 1\n";

fn execute(id: u32, script: &str) -> Value {
    serde_json::json!({
        "protocol": "jsonl-v1",
        "id": id,
        "method": "session.execute",
        "params": {"script": script},
    })
}

fn run_session(command: &mut Command, requests: &[Value]) -> std::process::Output {
    let mut child = command
        .args(["--protocol", "jsonl-v1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for request in requests {
        writeln!(stdin, "{request}").unwrap();
    }
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    output
}

fn strict_responses(stdout: &[u8]) -> Vec<Value> {
    String::from_utf8_lossy(stdout)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap_or_else(|error| panic!("{line}: {error}")))
        .collect()
}

fn responses_to_bytes(responses: &[Value]) -> Vec<u8> {
    serde_json::to_vec(responses).unwrap()
}
