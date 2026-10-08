// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::process::{Command, Output};

fn gauntlet(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bmair"))
        .arg("gauntlet")
        .args(arguments)
        .output()
        .unwrap()
}

#[test]
fn the_default_field_reports_every_opponent_and_a_total() {
    let output = gauntlet(&[
        "--games",
        "2",
        "--engine",
        "quick",
        "dk(1) k(V) k(V) k(V) dmMH(4)",
    ]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("Copyright"));
    assert!(stdout.starts_with("Button: dk(1) k(V) k(V) k(V) dmMH(4)\nEngine: quick\n"));
    let rows = stdout
        .lines()
        .skip_while(|line| !line.starts_with("Opponent"))
        .skip(1)
        .map(|line| line.split("  ").next().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        [
            "Lucky",
            "Vincent",
            "Sailor Jupiter",
            "Hammer",
            "Monkeys",
            "Lady K",
            "Konami",
            "Wolfman",
            "Overall"
        ]
    );
    assert!(stdout.lines().last().unwrap().contains("/16 "));
}

#[test]
fn an_opponents_file_replaces_the_default_field() {
    let path = std::env::temp_dir().join(format!("bmair-gauntlet-{}.txt", std::process::id()));
    std::fs::write(&path, "# classic\nAvis: (4) (4) (10) (12) (X)\n").unwrap();
    let output = gauntlet(&[
        "--games",
        "3",
        "--engine",
        "quick",
        "6 12 20 20 X",
        path.to_str().unwrap(),
    ]);
    std::fs::remove_file(&path).unwrap();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("4 games per opponent: seeds 1-2, each played from both seats"));
    assert!(stdout.contains("\nAvis "));
    assert!(!stdout.contains("Lucky"));
}

#[test]
fn the_default_field_prints_in_the_opponents_file_format() {
    let output = gauntlet(&["--field"]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 8);
    assert!(stdout.contains("\nHammer: (6) (12) (20) (20) (X)\n"));
}

#[test]
fn an_unquoted_recipe_is_refused_before_any_play() {
    let output = gauntlet(&["dk1", "kV", "kV"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("inside quotes")
    );
}

#[test]
fn bad_arguments_fail_with_a_reason_before_any_play() {
    for (arguments, reason) in [
        (&["-g", "10", "(4)"][..], "unknown option -g"),
        (
            &["--games", "0", "(4)"],
            "--games takes a whole number above zero, not 0",
        ),
        (
            &["--threads", "x", "(4)"],
            "--threads takes a whole number above zero, not x",
        ),
        (
            &["--seed", "4294967295", "--games", "4", "(4)"],
            "runs past the last seed",
        ),
        (&["--games"], "--games needs a value"),
        (&["(4)", "no-such-opponents.txt"], "no-such-opponents.txt: "),
        (&[""], "the button: player 0 has no dice"),
    ] {
        let output = gauntlet(arguments);
        assert!(!output.status.success(), "{arguments:?}");
        assert!(output.stdout.is_empty(), "{arguments:?}");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(reason), "{arguments:?}: {stderr}");
    }
}
