// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::Value;

#[test]
fn jsonl_process_keeps_stdout_machine_clean_and_recovers_per_line() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_bmair"))
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
fn capabilities_flag_emits_one_json_document_without_a_banner() {
    let output = Command::new(env!("CARGO_BIN_EXE_bmair"))
        .arg("--capabilities")
        .output()
        .unwrap();
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

fn responses_to_bytes(responses: &[Value]) -> Vec<u8> {
    serde_json::to_vec(responses).unwrap()
}
