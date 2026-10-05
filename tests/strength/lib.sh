# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
#
# Shared by the strength scripts: `source tests/strength/lib.sh OUT_DIR`, then
# call `run NAME FIRST SECOND` once per pairing. Rerunning resumes.

out=$1
seeds=${BMAIR_STRENGTH_SEEDS:-1..100}
threads=${BMAIR_STRENGTH_THREADS:-8}
mkdir -p "$out"

build=$(git describe --tags --always --dirty)
if [[ -e "$out/ladder" ]]; then
  # Resuming must not mix builds, nor replace a binary a running copy uses.
  if [[ "$(cat "$out/build.txt")" != "$build" ]]; then
    echo "$out was started from $(cat "$out/build.txt"), not $build" >&2
    exit 1
  fi
else
  cargo build --release --example ladder
  # A private copy, so switching branches mid-run can't change the binary.
  cp target/release/examples/ladder "$out/ladder"
  echo "$build" > "$out/build.txt"
  echo "$seeds" > "$out/seeds.txt"
fi

[[ -s "$out/summary.md" ]] || cat > "$out/summary.md" <<'TABLE'
| First | Second | Pairs | First wins | First score (95% CI) | First ms/decision | Second ms/decision |
|---|---|---:|---:|---|---:|---:|
TABLE

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
  tail -n 1 "$out/$name.tmp" >> "$out/summary.md"
  mv "$out/$name.tmp" "$out/$name.md"
  echo "$(date +%H:%M) done  $name" | tee -a "$out/progress.log"
}
