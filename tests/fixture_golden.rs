// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! The RNG fingerprint catches search changes even when the chosen move
//! survives them. After an intentional change, regenerate with
//! `BMAIR_UPDATE_GOLDEN=1 cargo test --release --test fixture_golden -- --include-ignored`
//! and review the diff.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Searches too long for every `cargo test`; CI runs them for releases.
const SLOW_FIXTURES: &[&str] = &[
    "bmai_in.txt",
    "bmsim_in.txt",
    "bug11_in.txt",
    "bug16_in.txt",
];

#[test]
fn fast_fixtures_match_their_golden_output() {
    check_fixtures(|name| !SLOW_FIXTURES.contains(&name));
}

#[test]
#[ignore = "long searches; run for releases with --ignored"]
fn slow_fixtures_match_their_golden_output() {
    check_fixtures(|name| SLOW_FIXTURES.contains(&name));
}

#[test]
fn every_slow_fixture_exists() {
    let fixtures = input_fixtures();
    for name in SLOW_FIXTURES {
        assert!(
            fixtures.iter().any(|path| file_name(path) == *name),
            "{name} is listed as slow but is not a fixture"
        );
    }
}

#[test]
fn every_golden_output_has_a_fixture() {
    let fixtures = input_fixtures();
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    for entry in fs::read_dir(directory).expect("read golden directory") {
        let golden = entry.expect("golden entry").path();
        assert!(
            fixtures
                .iter()
                .any(|path| file_name(path) == file_name(&golden)),
            "{} has no fixture; delete it",
            golden.display()
        );
    }
}

fn check_fixtures(include: impl Fn(&str) -> bool) {
    let update = std::env::var_os("BMAIR_UPDATE_GOLDEN").is_some();
    // Regenerating always passes, so it must never stand in for the CI check.
    assert!(
        !(update && std::env::var_os("CI").is_some()),
        "BMAIR_UPDATE_GOLDEN must not be set in CI"
    );
    let mut failures = Vec::new();
    for fixture in input_fixtures() {
        let name = file_name(&fixture);
        if !include(name) {
            continue;
        }
        let actual = run_fixture(&fixture);
        let golden = golden_path(name);
        if update {
            fs::write(&golden, &actual).expect("write golden output");
            continue;
        }
        let expected = fs::read_to_string(&golden).unwrap_or_else(|_| {
            panic!(
                "{} has no golden output; generate it with BMAIR_UPDATE_GOLDEN=1",
                golden.display()
            )
        });
        if let Some(difference) = first_difference(&expected, &actual) {
            failures.push(format!("{name}: {difference}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} fixture(s) differ from their golden output:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn run_fixture(fixture: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_bmair"))
        .arg(fixture)
        .env("BMAIR_TRACE_RNG_HASH", "1")
        .output()
        .expect("run bmair");
    let mut normalized = format!("exit {}\n", output.status.code().unwrap_or(-1));
    for (stream, bytes) in [("stdout", &output.stdout), ("stderr", &output.stderr)] {
        normalized.push_str(&format!("--- {stream}\n"));
        for line in String::from_utf8_lossy(bytes).lines() {
            if is_unstable(line) {
                continue;
            }
            let stable = line.split_once("Time:").map_or(line, |(stable, _)| stable);
            normalized.push_str(stable.trim_end());
            normalized.push('\n');
        }
    }
    normalized
}

/// The banner carries the build version and copyright year.
fn is_unstable(line: &str) -> bool {
    [
        "BMAIR:",
        "Rust port Copyright",
        "Original BMAI Copyright",
        "Version:",
        "Reading from ",
    ]
    .iter()
    .any(|prefix| line.starts_with(prefix))
}

fn first_difference(expected: &str, actual: &str) -> Option<String> {
    let mut expected_lines = expected.lines();
    let mut actual_lines = actual.lines();
    for line in 1.. {
        match (expected_lines.next(), actual_lines.next()) {
            (None, None) => return None,
            (expected, actual) if expected != actual => {
                return Some(format!(
                    "line {line}: expected {expected:?}, got {actual:?}"
                ));
            }
            _ => {}
        }
    }
    unreachable!()
}

fn input_fixtures() -> Vec<PathBuf> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut fixtures = fs::read_dir(directory)
        .expect("read fixture directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            let name = file_name(path);
            name.contains("in") && name.ends_with(".txt")
        })
        .collect::<Vec<_>>();
    fixtures.sort();
    fixtures
}

fn golden_path(fixture_name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(fixture_name)
}

fn file_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|name| name.to_str())
        .expect("UTF-8 fixture name")
}
