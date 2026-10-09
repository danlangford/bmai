// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::io::Write;
use std::process::{Command, Output, Stdio};

mod common;

fn gauntlet(arguments: &[&str]) -> Output {
    common::bmair()
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
fn a_dash_reads_the_opponents_from_standard_input() {
    let output = bmair_with_input(
        &[
            "gauntlet",
            "--games",
            "2",
            "--engine",
            "quick",
            "6 12 20 20 X",
            "-",
        ],
        b"Avis: (4) (4) (10) (12) (X)\n",
    );
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\nAvis "));
    assert!(!stdout.contains("Lucky"));
}

fn bmair_with_input(arguments: &[&str], input: &[u8]) -> Output {
    run_with_input(common::bmair().args(arguments), input)
}

fn run_with_input(command: &mut Command, input: &[u8]) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn merged_shards_print_the_single_run_table() {
    let options = ["--games", "5", "--seed", "3", "--engine", "quick"];
    let recipe = "dk(1) k(V) k(V) k(V) dmMH(4)";
    let single = gauntlet(&[&options[..], &[recipe]].concat());
    assert!(single.status.success(), "{single:?}");
    for (count, threads) in [(1, "1"), (3, "2"), (8, "1")] {
        let parts = (1..=count)
            .rev()
            .map(|part| {
                let shard = format!("{part}/{count}");
                let output = gauntlet(
                    &[
                        &options[..],
                        &["--threads", threads, "--shard", &shard, recipe],
                    ]
                    .concat(),
                );
                assert!(output.status.success(), "{output:?}");
                // The web page counts progress against this.
                let text = String::from_utf8(output.stdout.clone()).unwrap();
                let header: serde_json::Value =
                    serde_json::from_str(text.lines().next().unwrap()).unwrap();
                assert_eq!(header["pairs"], text.lines().count() - 1, "{shard}");
                output.stdout
            })
            .collect::<Vec<_>>();
        let merged = bmair_with_input(&["gauntlet", "--merge"], &parts.concat());
        assert!(merged.status.success(), "{merged:?}");
        assert_eq!(
            String::from_utf8(merged.stdout).unwrap(),
            String::from_utf8(single.stdout.clone()).unwrap(),
            "{count} shards"
        );
    }
}

#[test]
fn a_merge_reads_shard_files_and_names_what_is_missing() {
    let directory = std::env::temp_dir();
    let path = |part| directory.join(format!("bmair-shard-{}-{part}.jsonl", std::process::id()));
    for part in 1..=2 {
        let output = gauntlet(&[
            "--games",
            "2",
            "--engine",
            "quick",
            "--shard",
            &format!("{part}/2"),
            "6 12 20 20 X",
        ]);
        assert!(output.status.success(), "{output:?}");
        std::fs::write(path(part), output.stdout).unwrap();
    }
    let whole = gauntlet(&[
        "--merge",
        path(1).to_str().unwrap(),
        path(2).to_str().unwrap(),
    ]);
    let partial = gauntlet(&["--merge", path(2).to_str().unwrap()]);
    for part in 1..=2 {
        std::fs::remove_file(path(part)).unwrap();
    }
    assert!(whole.status.success(), "{whole:?}");
    assert!(
        String::from_utf8(whole.stdout)
            .unwrap()
            .contains("\nOverall ")
    );
    assert!(!partial.status.success());
    assert!(
        String::from_utf8(partial.stderr)
            .unwrap()
            .contains("missing shard 1 of 2")
    );
    let help = gauntlet(&["--merge", "--help"]);
    assert!(help.status.success());
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("--shard K/N")
    );
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
        (
            &["--shard", "3/2", "(4)"],
            "--shard takes K/N with 1 <= K <= N, not 3/2",
        ),
        (&["--games", "2", "--merge"], "--merge comes first"),
        (&["--merge", "no-such-shard.jsonl"], "no-such-shard.jsonl: "),
        (
            &["--merge", "--games"],
            "--merge takes shard files, not --games",
        ),
        (&[""], "the button: player 0 has no dice"),
    ] {
        let output = gauntlet(arguments);
        assert!(!output.status.success(), "{arguments:?}");
        assert!(output.stdout.is_empty(), "{arguments:?}");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(reason), "{arguments:?}: {stderr}");
    }
}

#[test]
fn the_hash_trace_prints_one_fingerprint_per_match() {
    let output = run_with_input(
        common::bmair().env("BMAIR_TRACE_RNG_HASH", "1").args([
            "gauntlet",
            "--games",
            "4",
            "--engine",
            "quick",
            "6 12 20 20 X",
            "-",
        ]),
        b"Avis: (4) (4) (10) (12) (X)\n",
    );
    assert!(output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    let draws = stderr
        .lines()
        .filter_map(|line| line.strip_prefix("RNG_HASH "))
        .map(|line| line.split(' ').next().unwrap().parse::<u64>().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(draws.len(), 4, "{stderr}");
    assert!(draws.iter().all(|&count| count > 0), "{stderr}");
}
