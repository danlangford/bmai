#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
#
# Long strength sweep: ply 2 against ply 3, then one-setting-at-a-time
# tuning against BMAIBagels' settings.
# Usage: tests/strength/sweep.sh [OUT_DIR]   (from the repo root)
set -euo pipefail

BMAIR_STRENGTH_SEEDS=${BMAIR_STRENGTH_SEEDS:-1..50}
source tests/strength/lib.sh "${1:-strength-runs/$(date +%Y-%m-%d-%H%M)-sweep}"

base="montecarlo max_sims=100 min_sims=5 maxbranch=400"
ply2="$base ply=2"

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
