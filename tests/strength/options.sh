#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
#
# Ply 1 with many more simulations, and ply 2 with four times as many,
# against BMAIBagels' ply 2 on buttons with many choices per move, plus ply 2
# at four times on the classic matchups. Seeds 201 on are fresh.
# Usage: tests/strength/options.sh [OUT_DIR]   (from the repo root)
set -euo pipefail

BMAIR_STRENGTH_SEEDS=${BMAIR_STRENGTH_SEEDS:-201..300}
source tests/strength/lib.sh "${1:-strength-runs/$(date +%Y-%m-%d-%H%M)-options}"

options=tests/strength/matchups-options.txt
ply2="montecarlo max_sims=100 min_sims=5 maxbranch=400 ply=2"
ply1_quarter="montecarlo max_sims=4000 min_sims=200 maxbranch=16000 ply=1"
ply1_same="montecarlo max_sims=16000 min_sims=800 maxbranch=64000 ply=1"
ply2_4x="montecarlo max_sims=400 min_sims=20 maxbranch=1600 ply=2"

run 01-options-ply2-vs-ply1-same-time "$ply2" "$ply1_same" "$options"
run 02-options-ply2-vs-ply1-quarter-time "$ply2" "$ply1_quarter" "$options"
run 03-options-ply2-vs-ply2-4x "$ply2" "$ply2_4x" "$options"
run 04-classic-ply2-vs-ply2-4x "$ply2" "$ply2_4x"

echo "$(date +%H:%M) all done" | tee -a "$out/progress.log"
