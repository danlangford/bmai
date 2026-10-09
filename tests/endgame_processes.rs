// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::io::Write;
use std::process::Stdio;

mod common;

/// Positions the endgame solver answers, including game 121248's at six dice.
const POSITIONS: [&str; 3] = [
    "game 3\nfight\nplayer 0 2 23\n2/8-2:2\n6/10-6:6\nplayer 1 4 26\nwHF4:2\n8:1\n10:2\nvz20:4\n",
    "game 3\nfight\nplayer 0 2 10\n^6:3\nf8:5\nplayer 1 2 12\nv10:7\nz6:2\n",
    "game 3\nfight\nplayer 0 3 15\n(4,4):5\np6:2\n12:9\nplayer 1 2 14\n8:8\nH6:1\n",
];

/// Every answer from one process: the action and the probability's bits.
fn answers() -> Vec<String> {
    let mut child = common::bmair()
        .args(["--protocol", "jsonl-v1"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for (index, position) in POSITIONS.iter().enumerate() {
        let script = format!("endgame 6\nsurrender off\n{position}getaction\n");
        let request = serde_json::json!({
            "protocol": "jsonl-v1",
            "id": index,
            "method": "session.execute",
            "params": {"script": script},
        });
        writeln!(stdin, "{request}").unwrap();
    }
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| {
            let response: serde_json::Value = serde_json::from_str(line).unwrap();
            let evaluation = &response["result"]["evaluation"];
            assert_eq!(evaluation["simulations"], 0, "solved exactly: {response}");
            let probability = evaluation["probability"].as_f64().unwrap();
            format!(
                "{} {:#x}",
                response["result"]["action"],
                probability.to_bits()
            )
        })
        .collect()
}

#[test]
fn separate_processes_give_bit_identical_endgame_answers() {
    let first = answers();
    assert_eq!(first.len(), POSITIONS.len());
    assert_eq!(first, answers());
}
