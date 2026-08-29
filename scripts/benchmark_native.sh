#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

set -euo pipefail

binary=${1:-target/release/bmair}
output_dir=${2:-native-benchmark-artifacts}
worker_counts=${BMAIR_BENCH_WORKERS:-1,8}
fixtures=${BMAIR_BENCH_FIXTURES:-bmai_in.txt,bmsim_in.txt,bug11_in.txt,bug16_in.txt}

mkdir -p "$output_dir"
git describe --tags --always --dirty >"$output_dir/git-describe.txt"
rustc -Vv >"$output_dir/rustc.txt"
uname -a >"$output_dir/uname.txt"
printf 'fixture\tworkers\treal_seconds\tuser_seconds\tsys_seconds\tmax_rss_bytes\toutput_sha256\n' \
    >"$output_dir/results.tsv"

IFS=, read -r -a worker_list <<<"$worker_counts"
IFS=, read -r -a fixture_list <<<"$fixtures"

for fixture_name in "${fixture_list[@]}"; do
    fixture="tests/fixtures/$fixture_name"
    if [[ ! -f "$fixture" ]]; then
        echo "missing fixture: $fixture" >&2
        exit 1
    fi
    for workers in "${worker_list[@]}"; do
        stem=${fixture_name%.txt}-w${workers}
        input="$output_dir/$stem.input.txt"
        output="$output_dir/$stem.output.txt"
        timing="$output_dir/$stem.time.txt"
        {
            printf 'mode native\nworkers %s\n' "$workers"
            sed '/^quit$/d' "$fixture"
            # Some upstream fixtures intentionally lack a trailing newline.
            printf '\nquit\n'
        } >"$input"
        /usr/bin/time -lp "$binary" <"$input" >"$output" 2>"$timing"
        real=$(awk '$1 == "real" { print $2 }' "$timing")
        user=$(awk '$1 == "user" { print $2 }' "$timing")
        sys=$(awk '$1 == "sys" { print $2 }' "$timing")
        rss=$(awk '$2 == "maximum" && $3 == "resident" { print $1 }' "$timing")
        hash=$(shasum -a 256 "$output" | awk '{ print $1 }')
        printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
            "$fixture_name" "$workers" "$real" "$user" "$sys" "${rss:-n/a}" "$hash" \
            >>"$output_dir/results.tsv"
        tail -n 1 "$output_dir/results.tsv"
    done
done
