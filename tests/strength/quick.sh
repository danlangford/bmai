#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
#
# Screens each quick_tweaks idea as Monte Carlo's playout against the C++ QAI,
# on the classic and the many-option matchups. Below 0.5 favours the tweak.
# Usage: tests/strength/quick.sh [OUT_DIR] [TWEAK...]   (from the repo root)
set -euo pipefail

BMAIR_STRENGTH_SEEDS=${BMAIR_STRENGTH_SEEDS:-401..425}
source tests/strength/lib.sh "${1:-strength-runs/$(date +%Y-%m-%d-%H%M)-quick}"
shift || true
tweaks=("$@")
[[ ${#tweaks[@]} -gt 0 ]] || tweaks=(noise0 noise1 noise2 exposure10 danger5 valueonce firecost10)

options=tests/strength/matchups-options.txt
mc="montecarlo ply=1 max_sims=4000 min_sims=200 maxbranch=16000 playout=quick"

for tweak in "${tweaks[@]}"; do
  run "options-$tweak" "$mc" "$mc quick_tweaks=$tweak" "$options"
  run "classic-$tweak" "$mc" "$mc quick_tweaks=$tweak"
done

echo "$(date +%H:%M) all done" | tee -a "$out/progress.log"
