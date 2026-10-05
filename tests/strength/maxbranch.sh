#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
#
# BMAIBagels' ply 2 maxbranch against twice and four times as much.
# Usage: tests/strength/maxbranch.sh [OUT_DIR]   (from the repo root)
set -euo pipefail

out=${1:-strength-runs/$(date +%Y-%m-%d-%H%M)-maxbranch}
seeds=${SEEDS:-1..100}
threads=${THREADS:-8}
mkdir -p "$out"

cargo build --release --example ladder
# A private copy, so switching branches mid-run can't change the binary.
cp target/release/examples/ladder "$out/ladder"
git rev-parse --short HEAD > "$out/commit.txt"

ply2="montecarlo max_sims=100 min_sims=5 maxbranch=400 ply=2"

run() {
  local name=$1 first=$2 second=$3
  if [[ -s "$out/$name.md" ]]; then
    echo "skip $name (done)"
    return
  fi
  echo "$(date +%H:%M) start $name: $first vs $second" | tee -a "$out/progress.log"
  "$out/ladder" --engine "$first" --engine "$second" \
    --seeds "$seeds" --threads "$threads" \
    > "$out/$name.tmp" 2>> "$out/progress.log"
  mv "$out/$name.tmp" "$out/$name.md"
  echo "$(date +%H:%M) done  $name" | tee -a "$out/progress.log"
  tail -n 1 "$out/$name.md" >> "$out/summary.md"
}

[[ -s "$out/summary.md" ]] || cat > "$out/summary.md" <<'EOF'
| First | Second | Pairs | First wins | First score (95% CI) | First ms/decision | Second ms/decision |
|---|---|---:|---:|---|---:|---:|
EOF


run 01-ply2-vs-maxbranch800 "$ply2" "montecarlo max_sims=100 min_sims=5 maxbranch=800 ply=2"
run 02-ply2-vs-maxbranch1600 "$ply2" "montecarlo max_sims=100 min_sims=5 maxbranch=1600 ply=2"

echo "$(date +%H:%M) all done" | tee -a "$out/progress.log"
