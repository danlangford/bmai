// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::io::Write;
use std::process::Stdio;

mod common;

/// BMAIBagels' request in game 121248 just before it claimed 100%; the
/// opponent's best line wins about 17.5% by hand.
const GAME_121248: &str = "mode native\nworkers 2\nseed 7\nfire_overshooting on\ngame 3\nfight\n\
player 0 2 23\n2/8-2:2\n6/10-6:6\nplayer 1 4 26\nwHF4:2\n8:1\n10:2\nvz20:4\n\
ply 2\nmax_sims 100\nmin_sims 5\nmaxbranch 400\nreport_sims 1000\nsurrender off\ngetaction\nquit\n";

#[test]
fn a_rerolled_value_die_leaves_the_opponent_a_chance() {
    let mut child = common::bmair()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(GAME_121248.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let text = String::from_utf8_lossy(&output.stdout) + String::from_utf8_lossy(&output.stderr);
    let report = text
        .lines()
        .find(|line| line.contains("selected move report"))
        .unwrap_or_else(|| panic!("no report in:\n{text}"));
    let percent: f32 = report
        .rsplit_once(", ")
        .and_then(|(_, rest)| rest.split('%').next())
        .and_then(|number| number.parse().ok())
        .unwrap_or_else(|| panic!("unreadable report: {report}"));
    assert!((60.0..90.0).contains(&percent), "{report}");
}
