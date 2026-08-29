#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

set -euo pipefail

binary=${1:-target/release/bmair}
output_dir=${2:-native-strength-artifacts}
seeds=${BMAIR_STRENGTH_SEEDS:-200}

mkdir -p "$output_dir"
git describe --tags --always --dirty >"$output_dir/git-describe.txt"

write_game() {
    local orientation=$1
    local bmai_player
    if [[ $orientation == original ]]; then
        bmai_player=0
        printf '%s\n' \
            'player 0 5 0' 6 8 z20 z20 S \
            'player 1 5 0' 4 20 4/8 6/12 6/20
    else
        bmai_player=1
        printf '%s\n' \
            'player 0 5 0' 4 20 4/8 6/12 6/20 \
            'player 1 5 0' 6 8 z20 z20 S
    fi
    printf 'ai 0 %s\nai 1 %s\n' \
        "$([[ $bmai_player == 0 ]] && echo 2 || echo 1)" \
        "$([[ $bmai_player == 1 ]] && echo 2 || echo 1)"
}

run_cell() {
    local mode=$1
    local orientation=$2
    local input="$output_dir/$mode-$orientation.input.txt"
    local output="$output_dir/$mode-$orientation.output.txt"
    {
        printf 'mode %s\nrng legacy\nworkers %s\n' \
            "$mode" "$([[ $mode == native ]] && echo 8 || echo 1)"
        printf '%s\n' 'ply 1' 'min_sims 10' 'max_sims 20' 'maxbranch 500'
        printf '%s\n' 'game 1' preround
        write_game "$orientation"
        for ((seed = 1; seed <= seeds; seed++)); do
            printf 'seed %d\nplaygame 1\n' "$seed"
        done
        printf 'quit\n'
    } >"$input"
    "$binary" <"$input" >"$output"

    local bmai_field=3
    [[ $orientation == swapped ]] && bmai_field=5
    awk -v field="$bmai_field" '/^matches over / { print ($field == 1 ? 1 : 0) }' \
        "$output" >"$output_dir/$mode-$orientation.wins.txt"
}

for mode in legacy native; do
    for orientation in original swapped; do
        run_cell "$mode" "$orientation"
    done
done

paste \
    "$output_dir/native-original.wins.txt" \
    "$output_dir/legacy-original.wins.txt" \
    "$output_dir/native-swapped.wins.txt" \
    "$output_dir/legacy-swapped.wins.txt" |
    awk '
        { d[++n] = $1 - $2; d[++n] = $3 - $4; sum += $1 - $2 + $3 - $4 }
        END {
            mean = sum / n
            for (i = 1; i <= n; i++) squared += (d[i] - mean)^2
            sd = sqrt(squared / (n - 1))
            half = 1.96 * sd / sqrt(n)
            lower = mean - half
            upper = mean + half
            verdict = lower > -0.10 ? "noninferior" : "not established"
            printf "pairs\t%d\nmean_difference\t%.6f\nci95_lower\t%.6f\nci95_upper\t%.6f\nverdict\t%s\n", n, mean, lower, upper, verdict
        }
    ' | tee "$output_dir/result.tsv"
