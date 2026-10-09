// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

mod common;

const TIMEOUT: Duration = Duration::from_secs(10);
const FIGHT_REQUEST: &[u8] = b"game\nfight\nplayer 0 1 1\n1:1\nplayer 1 2 30\n1:1\n(30,30):60\nsurrender off\ngetaction\nquit\n";

#[test]
fn legacy_banner_is_flushed_before_input() {
    let mut child = spawn_bmair();
    let stdout = child.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut stdout = BufReader::new(stdout);
        let mut banner = String::new();
        for _ in 0..4 {
            stdout.read_line(&mut banner).unwrap();
        }
        sender.send(banner).ok();
    });

    let banner = receive_or_stop(
        receiver,
        &mut child,
        reader,
        "legacy banner was not flushed before input",
    );
    assert!(banner.starts_with("BMAIR: the Button Men AI in Rust\n"));
    assert_eq!(
        banner.lines().nth(1),
        Some("Rust port Copyright © 2026 Dan Langford.")
    );
    assert_eq!(
        banner.lines().nth(2),
        Some("Original BMAI Copyright © 2001-2026 Denis Papp.")
    );
    assert!(banner.contains("Version:"));
    stop(&mut child);
}

#[test]
fn legacy_stdin_matches_bmaibagels_write_flush_read_contract() {
    let mut child = spawn_bmair();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(FIGHT_REQUEST).unwrap();
    stdin.flush().unwrap();

    let stdout = child.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut response = String::new();
        BufReader::new(stdout)
            .read_to_string(&mut response)
            .unwrap();
        sender.send(response).ok();
    });

    // BMAIBagels keeps stdin open, so `quit` must exit without waiting for EOF.
    let response = receive_or_stop(
        receiver,
        &mut child,
        reader,
        "legacy response waited for EOF",
    );
    assert!(response.starts_with("BMAIR: the Button Men AI in Rust\n"));
    let best_move = response
        .lines()
        .find(|line| line.contains(" p0 best move ") && line.contains('%'))
        .expect("BMAIBagels-compatible best-move diagnostic");
    let win_percentage = best_move
        .split('%')
        .next()
        .and_then(|prefix| prefix.split_whitespace().last())
        .expect("percentage before percent sign")
        .parse::<f32>()
        .expect("numeric win percentage");
    assert!(win_percentage.is_finite(), "{best_move}");
    assert_eq!(best_move, "l1 p0 best move (0.0 points, 0.0% win)");
    assert!(response.contains("action\npower\n0\n0\n"), "{response}");
    assert!(child.wait().unwrap().success());
    drop(stdin);

    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(stderr.is_empty(), "{stderr}");
}

#[test]
fn null_captures_do_not_tie_every_later_round() {
    let output = run_to_quit(
        b"game 3\npreround\nplayer 0 5 0\n6\nn8\nn12\n20\nX\nplayer 1 5 0\n4\n4\n10\n12\nX\nai 0 quick\nai 1 quick\nplaygame 1\nquit\n",
        "a match with Null dice never finished",
    );
    assert!(output.contains("\ngame over "), "{output}");
}

#[test]
fn a_match_that_can_only_tie_is_cancelled_at_round_200() {
    let output = run_to_quit(
        b"game 3\npreround\nplayer 0 2 0\nn4\nn4\nplayer 1 2 0\nn4\nn4\nai 0 quick\nai 1 quick\nplaygame 1\nquit\n",
        "a match that can only tie never finished",
    );
    assert!(
        output.contains("\ngame cancelled 0 - 0 - 199\nmatches over 0 - 0\n"),
        "{output}"
    );
}

#[test]
fn a_declined_auxiliary_die_stays_out_of_every_round() {
    // Gordo cannot add the courtesy X, so only Null dice play and every round
    // ties; a player 0 X would win the match.
    let output = run_to_quit(
        b"game 3\npreround\nplayer 0 3 0\nn4\nn4\n+X\nplayer 1 2 0\nn4\nn4\nspecial 1 unique_sizes\nai 0 quick\nai 1 quick\nplaygame 1\nquit\n",
        "a match with a declined Auxiliary die never finished",
    );
    assert!(
        output.contains("\ngame cancelled 0 - 0 - 199\nmatches over 0 - 0\n"),
        "{output}"
    );
}

#[test]
fn rng_hash_trace_skips_endgame_solver_replays() {
    let stderr = run_traced(
        "BMAIR_TRACE_RNG_HASH",
        b"endgame 4\ngame 3\nfight\nplayer 0 2 10\n^6:3\nf8:5\nplayer 1 2 12\nv10:7\nz6:2\ngetaction\nquit\n",
    );
    let hash_lines = stderr.lines().filter(|line| line.starts_with("RNG_HASH "));
    let first = stderr.lines().next().unwrap_or_default();
    assert_eq!(hash_lines.count(), 1, "first stderr line: {first}");
}

fn run_traced(variable: &str, input: &[u8]) -> String {
    let mut child = piped_bmair().env(variable, "1").spawn().unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stderr).unwrap()
}

fn run_to_quit(input: &'static [u8], timeout_message: &str) -> String {
    let mut child = spawn_bmair();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(input).unwrap();
    stdin.flush().unwrap();
    let stdout = child.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut output = String::new();
        BufReader::new(stdout).read_to_string(&mut output).unwrap();
        sender.send(output).ok();
    });
    let output = receive_or_stop(receiver, &mut child, reader, timeout_message);
    assert!(child.wait().unwrap().success());
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    output
}

fn spawn_bmair() -> Child {
    piped_bmair().spawn().unwrap()
}

fn piped_bmair() -> Command {
    let mut command = common::bmair();
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn receive_or_stop<T: Send + 'static>(
    receiver: mpsc::Receiver<T>,
    child: &mut Child,
    reader: JoinHandle<()>,
    timeout_message: &str,
) -> T {
    match receiver.recv_timeout(TIMEOUT) {
        Ok(value) => {
            reader.join().unwrap();
            value
        }
        Err(error) => {
            stop(child);
            reader.join().ok();
            panic!("{timeout_message}: {error}");
        }
    }
}

fn stop(child: &mut Child) {
    child.kill().ok();
    child.wait().ok();
}
