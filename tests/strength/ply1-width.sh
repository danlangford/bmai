#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
#
# Ply 1 with many more simulations against BMAIBagels' ply 2, at a quarter,
# the same, and four times ply 2's time per move. Settings keep the sweep's
# ratios, since maxbranch / min_sims also caps the Fire candidates.
# Usage: tests/strength/ply1-width.sh [OUT_DIR]   (from the repo root)
set -euo pipefail

source tests/strength/lib.sh "${1:-strength-runs/$(date +%Y-%m-%d-%H%M)-ply1-width}"

ply2="montecarlo max_sims=100 min_sims=5 maxbranch=400 ply=2"

run 01-ply2-vs-ply1-same-time "$ply2" "montecarlo max_sims=16000 min_sims=800 maxbranch=64000 ply=1"
run 02-ply2-vs-ply1-quarter-time "$ply2" "montecarlo max_sims=4000 min_sims=200 maxbranch=16000 ply=1"
run 03-ply2-vs-ply1-4x-time "$ply2" "montecarlo max_sims=64000 min_sims=3200 maxbranch=256000 ply=1"

echo "$(date +%H:%M) all done" | tee -a "$out/progress.log"
