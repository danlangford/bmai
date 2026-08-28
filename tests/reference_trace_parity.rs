// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Strong search-parity gate. The C++ binary must be built with the local RNG
/// fingerprint hook used during porting. Count plus a rolling hash covers every
/// generated state without serializing multi-million-event traces.
#[test]
#[ignore = "requires instrumented BMAI_CPP_TRACE_REFERENCE; exhaustive and long-running"]
fn every_input_fixture_has_the_identical_rng_fingerprint() {
    let reference = std::env::var_os("BMAI_CPP_TRACE_REFERENCE")
        .expect("set BMAI_CPP_TRACE_REFERENCE to an instrumented C++ bmai executable");
    let rust = env!("CARGO_BIN_EXE_bmair");
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut fixtures = input_fixtures(&fixture_dir);
    fixtures.sort();

    for fixture in fixtures {
        let cpp = run_with_rng_fingerprint(&reference, &fixture);
        let rust = run_with_rng_fingerprint(rust, &fixture);
        assert_eq!(
            cpp,
            rust,
            "RNG fingerprint differs for {}",
            fixture.display()
        );
    }
}

#[test]
#[ignore = "requires instrumented BMAI_CPP_TRACE_REFERENCE"]
fn representative_searches_consume_the_identical_rng_stream() {
    let reference = std::env::var_os("BMAI_CPP_TRACE_REFERENCE")
        .expect("set BMAI_CPP_TRACE_REFERENCE to an instrumented C++ bmai executable");
    let rust = env!("CARGO_BIN_EXE_bmair");
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let fixtures = [
        "Value1_in.txt",
        "bmai_in.txt",
        "parity_chance_chain_in.txt",
        "parity_combined_mechanics_in.txt",
        "parity_trip_morphing_in.txt",
    ]
    .map(|name| fixture_dir.join(name));
    compare_fixtures(&reference, rust, fixtures);
}

fn compare_fixtures(
    reference: impl AsRef<std::ffi::OsStr>,
    rust: impl AsRef<std::ffi::OsStr>,
    fixtures: impl IntoIterator<Item = PathBuf>,
) {
    let reference = reference.as_ref();
    let rust_executable = rust.as_ref();
    for fixture in fixtures {
        let cpp = run_with_raw_rng_trace(reference, &fixture);
        let rust = run_with_raw_rng_trace(rust_executable, &fixture);
        if cpp != rust {
            let mismatch = cpp
                .iter()
                .zip(&rust)
                .position(|(cpp, rust)| cpp != rust)
                .unwrap_or(cpp.len().min(rust.len()));
            panic!(
                "raw RNG stream differs for {} at event {mismatch}: C++ {:?}, Rust {:?}; lengths {} vs {}",
                fixture.display(),
                cpp.get(mismatch),
                rust.get(mismatch),
                cpp.len(),
                rust.len()
            );
        }
    }
}

fn run_with_raw_rng_trace(executable: impl AsRef<std::ffi::OsStr>, fixture: &Path) -> Vec<u32> {
    let result = Command::new(executable)
        .arg(fixture)
        .env("BMAIR_TRACE_RAW_RNG", "1")
        .output()
        .expect("run traced implementation");
    assert!(
        result.status.success(),
        "{} failed: {}",
        fixture.display(),
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8_lossy(&result.stderr)
        .lines()
        .filter_map(|line| line.strip_prefix("RNG "))
        .map(|seed| seed.parse().expect("numeric RNG trace state"))
        .collect()
}

fn run_with_rng_fingerprint(
    executable: impl AsRef<std::ffi::OsStr>,
    fixture: &Path,
) -> (bool, u64, u64) {
    let result = Command::new(executable)
        .arg(fixture)
        .env("BMAIR_TRACE_RNG_HASH", "1")
        .output()
        .expect("run fingerprinted implementation");
    let stderr = String::from_utf8_lossy(&result.stderr);
    let line = stderr
        .lines()
        .find(|line| line.starts_with("RNG_HASH "))
        .expect("RNG fingerprint output");
    let values = line
        .split_whitespace()
        .skip(1)
        .map(|value| value.parse::<u64>().expect("numeric RNG fingerprint"))
        .collect::<Vec<_>>();
    (result.status.success(), values[0], values[1])
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

#[cfg(test)]
mod tests {
    #[test]
    fn extracts_only_raw_rng_states() {
        let values = "QAI begin\nRNG 17\nRNG 42\n"
            .lines()
            .filter_map(|line| line.strip_prefix("RNG "))
            .map(|seed| seed.parse::<u32>().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(values, [17, 42]);
    }
}
