// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! Expected values were captured from the C++ reference `4813530`, which
//! agreed on every case except that it prints a Twin die's total size.

use std::io::Write;
use std::process::Stdio;

mod common;

#[test]
fn invalid_commands_fail_with_the_cpp_error() {
    let cases = [
        ("unrecognized\n", "unrecognized command: unrecognized"),
        (
            "debug invalid 0\n",
            "Could not find debug category: invalid",
        ),
        ("game\ninvalid-phase\n", "phase not found"),
        (
            "game\nfight\nnot-a-player\n",
            "missing player: not-a-player",
        ),
        (
            "game\ngameover\nplayer 0 0 0\nplayer 1 0 0\ngetaction\n",
            "p0 s0.0 Dice \np1 s0.0 Dice \nGetAction(): Unrecognized phase",
        ),
        (
            "game\nfight\nplayer 0 1 0\n6:6\nplayer 1 1 0\n6:6\nplaygame 1\n",
            "p0 s0.0 Dice (0)6:6 \np1 s0.0 Dice (0)6:6 \nCannot PlayGame unless it is preround",
        ),
    ];
    for (input, message) in cases {
        let output = execute(input);
        assert!(!output.status.success(), "{input:?} should fail");
        let combined = [output.stdout, output.stderr].concat();
        let text = String::from_utf8_lossy(&combined);
        let observed = text
            .lines()
            .filter(|line| !is_banner(line))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(observed, message, "error differs for {input:?}");
    }
}

#[test]
fn invalid_engine_selection_fails_with_the_engine_names() {
    for (input, message) in [
        (
            "ai 0 3\n",
            "unknown ai 3; choose one of: random, maximize, quick, montecarlo",
        ),
        ("ai 2 quick\n", "invalid setting for ai player number: 2"),
    ] {
        let output = execute(input);
        assert!(!output.status.success(), "{input:?} should fail");
        let combined = [output.stdout, output.stderr].concat();
        let text = String::from_utf8_lossy(&combined);
        let observed = text
            .lines()
            .filter(|line| !is_banner(line))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(observed, message, "error differs for {input:?}");
    }
}

#[test]
fn defined_swing_and_option_forms_parse_to_the_cpp_dice() {
    let cases = [
        ("T-2:2", "(0)2:2"),
        ("(T,T)-2:2", "(2000)(2,2):2"),
        ("(X,Y)-6:6", "(2000)(6,6):6"),
        ("(T,4)-2:2", "(2000)(2,4):2"),
        ("(4,T)-2:2", "(2000)(4,2):2"),
        ("T-20?:20", "(400)20:20"),
        ("T?-20:20", "(400)20:20"),
        ("(T,T)-2!:2", "(2800)(2,2):2"),
        ("(T,T)!-2:2", "(2800)(2,2):2"),
        ("6/20-6:6", "(1000)6:6"),
        ("6/20-20:20", "(1000)20:20"),
        ("T/20-20:20", "(1000)20:20"),
        ("T/U-6:6", "(1000)6:6"),
    ];
    for (recipe, parsed) in cases {
        let input = format!("game 3\nfight\nplayer 0 1 0\n30:30\nplayer 1 1 0\n{recipe}\nquit\n");
        let output = execute(&input);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout
            .lines()
            .find(|line| line.starts_with("p1 "))
            .unwrap_or_else(|| panic!("no player 1 line for {recipe}:\n{stdout}"));
        assert_eq!(
            line.trim_end(),
            format!("p1 s0.0 Dice {parsed}"),
            "{recipe}"
        );
    }
}

fn is_banner(line: &str) -> bool {
    [
        "BMAIR:",
        "Copyright",
        "Original BMAI Copyright",
        "Rust port Copyright",
        "Version:",
    ]
    .iter()
    .any(|prefix| line.starts_with(prefix))
}

fn execute(input: &str) -> std::process::Output {
    let mut child = common::bmair()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start bmair");
    child
        .stdin
        .take()
        .expect("child stdin")
        .write_all(input.as_bytes())
        .expect("write input");
    child.wait_with_output().expect("wait for bmair")
}
