// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

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

    // Keep stdin open while reading, exactly as BMAIBagels does. The child
    // must recognize `quit`, emit the action, and exit without waiting for EOF.
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

fn spawn_bmair() -> Child {
    Command::new(env!("CARGO_BIN_EXE_bmair"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
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
