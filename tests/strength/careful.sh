#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
#
# Careful against Quick, head to head and as Monte Carlo's playout, on the
# classic and the many-option matchups. Seeds 301 on are fresh.
# Usage: tests/strength/careful.sh [OUT_DIR]   (from the repo root)
set -euo pipefail

BMAIR_STRENGTH_SEEDS=${BMAIR_STRENGTH_SEEDS:-301..400}
source tests/strength/lib.sh "${1:-strength-runs/$(date +%Y-%m-%d-%H%M)-careful}"

options=tests/strength/matchups-options.txt
mc="montecarlo ply=1 max_sims=4000 min_sims=200 maxbranch=16000"

run 01-classic-quick-vs-careful quick careful
run 02-options-quick-vs-careful quick careful "$options"
run 03-options-playout-quick-vs-careful "$mc playout=quick" "$mc playout=careful" "$options"
run 04-classic-playout-quick-vs-careful "$mc playout=quick" "$mc playout=careful"

echo "$(date +%H:%M) all done" | tee -a "$out/progress.log"
