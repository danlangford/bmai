// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Runs the expensive differential suite when `BMAI_CPP_REFERENCE` points at a
/// C++ reference binary. The legacy executable's verbose diagnostic logging is
/// not part of the protocol, so compare statistics, actions, and match results.
#[test]
#[ignore = "requires BMAI_CPP_REFERENCE and includes long BMAI3 simulations"]
fn every_input_fixture_matches_cpp_reference() {
    let reference = std::env::var_os("BMAI_CPP_REFERENCE")
        .expect("set BMAI_CPP_REFERENCE to the C++ bmai executable");
    let rust = env!("CARGO_BIN_EXE_bmair");
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut fixtures = input_fixtures(&fixture_dir);
    fixtures.sort();

    let mut failures = Vec::new();
    for fixture in fixtures {
        let cpp = Command::new(&reference)
            .arg(&fixture)
            .output()
            .expect("run C++ reference");
        let rust = Command::new(rust)
            .arg(&fixture)
            .output()
            .expect("run Rust implementation");
        let cpp = normalize(&cpp.stdout, &cpp.stderr);
        let rust = normalize(&rust.stdout, &rust.stderr);
        if cpp != rust {
            failures.push(format!(
                "{}\n--- C++ ---\n{}\n--- Rust ---\n{}",
                fixture.display(),
                cpp,
                rust
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} fixture(s) differ:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

fn input_fixtures(directory: &Path) -> Vec<PathBuf> {
    fs::read_dir(directory)
        .expect("read fixture directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains("in") && name.ends_with(".txt"))
        })
        .collect()
}

fn normalize(stdout: &[u8], stderr: &[u8]) -> String {
    let combined = [stdout, stderr].concat();
    let output = String::from_utf8_lossy(&combined);
    let lines = output
        .lines()
        .filter(|line| !line.starts_with("Version:") && !line.starts_with("Reading from "))
        .map(|line| line.split_once("Time:").map_or(line, |(stable, _)| stable))
        .map(str::trim_end)
        .collect::<Vec<_>>();

    if let Some(stats) = lines.iter().rposition(|line| line.starts_with("stats ")) {
        return lines[stats..].join("\n");
    }

    lines
        .into_iter()
        .filter(|line| line.starts_with("game over ") || line.starts_with("matches over "))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_preserves_deterministic_stats_prefix() {
        let value = normalize(
            b"Version: one\nstats 1/10 Time: 3 s Sim: 7\naction\npower\n",
            b"",
        );
        assert_eq!(value, "stats 1/10\naction\npower");
    }
}
