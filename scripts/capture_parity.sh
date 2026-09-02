#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
# Capture a complete, durable C++/Rust parity run for later inspection.

set -u
set -o pipefail

repo_dir=$(cd "$(dirname "$0")/.." && pwd)
cpp_binary=${BMAI_CPP_REFERENCE:-}
cpp_build=${BMAI_CPP_BUILD:-}
trace_binary=${BMAI_CPP_TRACE_REFERENCE:-}
artifact_root=${BMAI_PARITY_OUTPUT:-"$repo_dir/parity-artifacts"}
run_id=$(date -u +%Y%m%dT%H%M%SZ)
run_dir="$artifact_root/$run_id"
rust_binary="$repo_dir/target/release/bmair"

usage() {
    echo "usage: $0 --cpp PATH [--cpp-build DIR] [--trace-cpp PATH] [--output DIR]"
    echo "PATH must contain the adopted Konstant behavior (currently C++ 4813530)."
    echo "The same values may be supplied with BMAI_CPP_REFERENCE, BMAI_CPP_BUILD,"
    echo "BMAI_CPP_TRACE_REFERENCE, and BMAI_PARITY_OUTPUT."
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --cpp) cpp_binary=$2; shift 2 ;;
        --cpp-build) cpp_build=$2; shift 2 ;;
        --trace-cpp) trace_binary=$2; shift 2 ;;
        --output) artifact_root=$2; run_dir="$artifact_root/$run_id"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "unknown argument: $1" >&2; usage >&2; exit 2 ;;
    esac
done

if [[ -z "$cpp_binary" || ! -x "$cpp_binary" ]]; then
    echo "--cpp must name an executable adopted-Konstant C++ reference binary" >&2
    exit 2
fi

mkdir -p "$run_dir/fixtures/cpp" "$run_dir/fixtures/rust" "$run_dir/logs"
summary="$run_dir/summary.tsv"
printf 'check\tstatus\telapsed_seconds\n' > "$summary"
performance="$run_dir/fixture-performance.tsv"
printf 'fixture\tcpp_status\tcpp_seconds\trust_status\trust_seconds\trust_over_cpp\n' > "$performance"

run_logged() {
    local name=$1
    shift
    local started ended status
    started=$(date +%s)
    set +e
    "$@" >"$run_dir/logs/$name.stdout" 2>"$run_dir/logs/$name.stderr"
    status=$?
    set -e
    ended=$(date +%s)
    printf '%s\t%s\t%s\n' "$name" "$status" "$((ended - started))" >> "$summary"
    return "$status"
}

set -e
{
    echo "run_id=$run_id"
    echo "repository=$repo_dir"
    echo "cpp_binary=$cpp_binary"
    echo "cpp_build=$cpp_build"
    echo "trace_binary=$trace_binary"
    git -C "$repo_dir" rev-parse HEAD
    rustc --version
    cargo --version
} > "$run_dir/environment.txt" 2>&1
git -C "$repo_dir" status --short > "$run_dir/git-status.txt"

overall=0
if [[ -n "$cpp_build" ]]; then
    run_logged cpp_tests ctest --test-dir "$cpp_build" --output-on-failure || overall=1
else
    printf 'cpp_tests\tSKIPPED\t0\n' >> "$summary"
fi
run_logged rust_tests cargo test --manifest-path "$repo_dir/Cargo.toml" || overall=1
run_logged rust_clippy cargo clippy --manifest-path "$repo_dir/Cargo.toml" --all-targets -- -D warnings || overall=1
run_logged rust_release cargo build --manifest-path "$repo_dir/Cargo.toml" --release || overall=1

while IFS= read -r fixture; do
    name=$(basename "$fixture" .txt)
    set +e
    started=$(date +%s)
    "$cpp_binary" "$fixture" >"$run_dir/fixtures/cpp/$name.stdout" 2>"$run_dir/fixtures/cpp/$name.stderr"
    cpp_status=$?
    ended=$(date +%s)
    cpp_seconds=$((ended - started))
    started=$(date +%s)
    "$rust_binary" "$fixture" >"$run_dir/fixtures/rust/$name.stdout" 2>"$run_dir/fixtures/rust/$name.stderr"
    rust_status=$?
    ended=$(date +%s)
    rust_seconds=$((ended - started))
    set -e
    if [[ "$cpp_seconds" -eq 0 ]]; then
        ratio=NA
    else
        ratio=$(awk -v rust="$rust_seconds" -v cpp="$cpp_seconds" 'BEGIN { printf "%.3f", rust / cpp }')
    fi
    printf 'fixture_%s_cpp\t%s\t%s\n' "$name" "$cpp_status" "$cpp_seconds" >> "$summary"
    printf 'fixture_%s_rust\t%s\t%s\n' "$name" "$rust_status" "$rust_seconds" >> "$summary"
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$name" "$cpp_status" "$cpp_seconds" "$rust_status" "$rust_seconds" "$ratio" \
        >> "$performance"
    if [[ "$cpp_status" -ne "$rust_status" ]]; then overall=1; fi
done < <(find "$repo_dir/tests/fixtures" -type f -name '*in*.txt' | sort)

run_logged fixture_material_differential env BMAI_CPP_REFERENCE="$cpp_binary" \
    cargo test --manifest-path "$repo_dir/Cargo.toml" --release --test reference_parity \
    every_input_fixture_matches_cpp_reference -- --ignored || overall=1
run_logged parser_differential env BMAI_CPP_REFERENCE="$cpp_binary" \
    cargo test --manifest-path "$repo_dir/Cargo.toml" --release --test reference_parser_parity \
    -- --ignored || overall=1

if [[ -n "$trace_binary" ]]; then
    run_logged rng_representative_differential env BMAI_CPP_TRACE_REFERENCE="$trace_binary" \
        cargo test --manifest-path "$repo_dir/Cargo.toml" --release --test reference_trace_parity \
        representative_searches_consume_the_identical_rng_stream -- --ignored || overall=1
    run_logged rng_exhaustive_differential env BMAI_CPP_TRACE_REFERENCE="$trace_binary" \
        cargo test --manifest-path "$repo_dir/Cargo.toml" --release --test reference_trace_parity \
        every_input_fixture_has_the_identical_rng_fingerprint -- --ignored || overall=1
else
    printf 'rng_representative_differential\tSKIPPED\t0\n' >> "$summary"
    printf 'rng_exhaustive_differential\tSKIPPED\t0\n' >> "$summary"
fi

echo "Parity artifacts: $run_dir"
echo "Overall status: $overall"
exit "$overall"
