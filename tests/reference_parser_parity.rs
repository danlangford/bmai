// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
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
        let cpp = run_error(&reference, input);
        let rust = run_error(rust, input);
        assert_eq!(cpp.0, rust.0, "exit status differs for {input:?}");
        assert_eq!(cpp.1, rust.1, "error differs for {input:?}");
    }
}

#[test]
#[ignore = "requires BMAI_CPP_REFERENCE"]
fn defined_swing_and_option_forms_produce_the_same_cpp_search_result() {
    let reference = std::env::var_os("BMAI_CPP_REFERENCE")
        .expect("set BMAI_CPP_REFERENCE to the C++ bmai executable");
    let rust = env!("CARGO_BIN_EXE_bmair");
    let recipes = [
        "T-2:2",
        "(T,T)-2:2",
        "(X,Y)-6:6",
        "(T,4)-2:2",
        "(4,T)-2:2",
        "T-20?:20",
        "T?-20:20",
        "(T,T)-2!:2",
        "(T,T)!-2:2",
        "6/20-6:6",
        "6/20-20:20",
        "T/20-20:20",
        "T/U-6:6",
    ];

    for recipe in recipes {
        let input = format!(
            "game 3\nfight\nplayer 0 1 0\n30:30\nplayer 1 1 0\n{recipe}\nply 1\nmax_sims 1\nmin_sims 1\nmaxbranch 1\nsurrender off\ngetaction\nquit\n"
        );
        assert_eq!(
            run_material(&reference, &input),
            run_material(rust, &input),
            "defined Swing/Option form differs for {recipe}"
        );
    }
}

fn execute(executable: impl AsRef<std::ffi::OsStr>, input: &str) -> std::process::Output {
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
    child.wait_with_output().expect("wait for implementation")
}

fn run_error(executable: impl AsRef<std::ffi::OsStr>, input: &str) -> (bool, String) {
    let output = execute(executable, input);
    let combined = [output.stdout, output.stderr].concat();
    let message = String::from_utf8_lossy(&combined)
        .lines()
        .filter(|line| {
            !line.starts_with("BMAI:")
                && !line.starts_with("BMAIR:")
                && !line.starts_with("Copyright")
                && !line.starts_with("Original BMAI Copyright")
                && !line.starts_with("Rust port Copyright")
                && !line.starts_with("For information")
                && !line.starts_with("Version:")
        })
        .collect::<Vec<_>>()
        .join("\n");
    (output.status.success(), message)
}

fn run_material(executable: impl AsRef<std::ffi::OsStr>, input: &str) -> (bool, String) {
    let output = execute(executable, input);
    let text = String::from_utf8_lossy(&output.stdout);
    let lines = text
        .lines()
        .map(|line| line.split_once("Time:").map_or(line, |(stable, _)| stable))
        .map(str::trim_end)
        .collect::<Vec<_>>();
    let material = lines
        .iter()
        .rposition(|line| line.starts_with("stats "))
        .map_or_else(String::new, |stats| lines[stats..].join("\n"));
    (output.status.success(), material)
}
