#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
#
# Monte Carlo at BMAIBagels' settings against the same with the exact endgame
# solver, on the classic and many-option matchups. Below 0.5 favours the solver.
# Usage: tests/strength/endgame.sh [OUT_DIR] [DICE...]   (from the repo root)
set -euo pipefail

BMAIR_STRENGTH_SEEDS=${BMAIR_STRENGTH_SEEDS:-801..825}
source tests/strength/lib.sh "${1:-strength-runs/$(date +%Y-%m-%d-%H%M)-endgame}"
shift || true
dice=("$@")
[[ ${#dice[@]} -gt 0 ]] || dice=(4 6)

options=tests/strength/matchups-options.txt
mc="montecarlo ply=1 max_sims=4000 min_sims=200 maxbranch=16000"

for count in "${dice[@]}"; do
  run "options-endgame$count" "$mc" "$mc endgame=$count" "$options"
  run "classic-endgame$count" "$mc" "$mc endgame=$count"
done

echo "$(date +%H:%M) all done" | tee -a "$out/progress.log"
