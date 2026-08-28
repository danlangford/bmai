// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn native_playgame_wire_fixture_is_deterministic() {
    let input = include_str!("native-fixtures/playgame.txt");
    let expected = "Setting execution mode to native\n\
        Setting RNG to legacy (bmai-park-miller-16807-v1)\n\
        Seeding with 17\n\
        Setting max ply to 1\n\
        Setting max # simulations to 1\n\
        Setting min # simulations to 1\n\
        Setting max branch to 10\n\
        target wins set to 1\n\
        p0 s0.0 Dice (0)6 \n\
        p1 s0.0 Dice (0)6 \n\
        game over 1 - 0 - 0\n\
        matches over 1 - 0\n\
        Seeding with 17\n\
        game over 1 - 0 - 0\n\
        matches over 1 - 0\n";

    for _ in 0..2 {
        let mut child = Command::new(env!("CARGO_BIN_EXE_bmair"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8(output.stdout).unwrap();
        let protocol = stdout.split_once("Setting execution mode").unwrap().1;
        assert_eq!(format!("Setting execution mode{protocol}"), expected);
    }
}
