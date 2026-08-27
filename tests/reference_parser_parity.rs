// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::io::Write;
use std::process::{Command, Stdio};

#[test]
#[ignore = "requires BMAI_CPP_REFERENCE"]
fn invalid_commands_match_cpp_exit_status_and_error() {
    let reference = std::env::var_os("BMAI_CPP_REFERENCE")
        .expect("set BMAI_CPP_REFERENCE to the C++ bmai executable");
    let rust = env!("CARGO_BIN_EXE_bmair");
    let cases = [
        "unrecognized\n",
        "ai 0 3\n",
        "ai 2 0\n",
        "debug invalid 0\n",
        "game\ninvalid-phase\n",
        "game\nfight\nnot-a-player\n",
        "game\ngameover\nplayer 0 0 0\nplayer 1 0 0\ngetaction\n",
        "game\nfight\nplayer 0 1 0\n6:6\nplayer 1 1 0\n6:6\nplaygame 1\n",
    ];

    for input in cases {
        let cpp = run(&reference, input);
        let rust = run(rust, input);
        assert_eq!(cpp.0, rust.0, "exit status differs for {input:?}");
        assert_eq!(cpp.1, rust.1, "error differs for {input:?}");
    }
}

fn run(executable: impl AsRef<std::ffi::OsStr>, input: &str) -> (bool, String) {
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start implementation");
    child
        .stdin
        .take()
        .expect("child stdin")
        .write_all(input.as_bytes())
        .expect("write input");
    let output = child.wait_with_output().expect("wait for implementation");
    let combined = [output.stdout, output.stderr].concat();
    let message = String::from_utf8_lossy(&combined)
        .lines()
        .filter(|line| {
            !line.starts_with("BMAI:")
                && !line.starts_with("BMAIR:")
                && !line.starts_with("Copyright")
                && !line.starts_with("Based on BMAI")
                && !line.starts_with("Rust port Copyright")
                && !line.starts_with("For information")
                && !line.starts_with("Version:")
        })
        .collect::<Vec<_>>()
        .join("\n");
    (output.status.success(), message)
}
