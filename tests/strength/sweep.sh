#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
#
# Long strength sweep: ply 2 against ply 3, then one-setting-at-a-time
# tuning against BMAIBagels' settings.
# Usage: tests/strength/sweep.sh [OUT_DIR]   (from the repo root)
set -euo pipefail

out=${1:-strength-runs/$(date +%Y-%m-%d-%H%M)}
seeds=${SEEDS:-1..50}
threads=${THREADS:-8}
mkdir -p "$out"

cargo build --release --example ladder
# A private copy, so switching branches mid-run can't change the binary.
cp target/release/examples/ladder "$out/ladder"
git rev-parse --short HEAD > "$out/commit.txt"

base="montecarlo max_sims=100 min_sims=5 maxbranch=400"
ply2="$base ply=2"

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

# 1. Does looking deeper help?
run 01-ply2-vs-ply3 "$ply2" "$base ply=3"

# 2. Ply 1 with ten times the simulations: breadth instead of depth.
run 03-ply2-vs-ply1-wide "$ply2" "montecarlo max_sims=1000 min_sims=50 maxbranch=4000 ply=1"

# 3. Ply 2 tuning, one setting at a time.
run 04-ply2-vs-maxbranch800 "$ply2" "montecarlo max_sims=100 min_sims=5 maxbranch=800 ply=2"
run 05-ply2-vs-maxbranch200 "$ply2" "montecarlo max_sims=100 min_sims=5 maxbranch=200 ply=2"
run 06-ply2-vs-maxsims200 "$ply2" "montecarlo max_sims=200 min_sims=5 maxbranch=400 ply=2"
run 07-ply2-vs-minsims20 "$ply2" "montecarlo max_sims=100 min_sims=20 maxbranch=400 ply=2"
run 08-ply2-vs-cull-off "$ply2" "$ply2 cull=off"
run 09-ply2-vs-maximize-playout "$ply2" "$ply2 playout=maximize"

echo "$(date +%H:%M) all done" | tee -a "$out/progress.log"
