#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

# usage: scripts/package_web.sh BMAIR_WASM OUTPUT_ZIP [BUILD_INFO]
#
# Writes a static site whose files sit at the root of OUTPUT_ZIP, ready to
# unzip into any web root or upload to a static host. Everything but the
# entry page lives in a folder named for its contents, so a browser holding
# a cached file from an earlier upload can never mix it with this one.

set -euo pipefail

if [[ $# -lt 2 || $# -gt 3 ]]; then
    echo "usage: $0 BMAIR_WASM OUTPUT_ZIP [BUILD_INFO]" >&2
    exit 2
fi
wasm=$1
output=$2
build_info=${3:-}
root=$(cd "$(dirname "$0")/.." && pwd)

site=$(mktemp -d)
trap 'rm -rf "$site"' EXIT
assets="$site/assets"
mkdir -p "$assets/examples"

for file in style.css app.js arguments.js worker.js wasi.js; do
    cp "$root/web/$file" "$assets/$file"
done
cp "$wasm" "$assets/bmair.wasm"
cp "$root/tests/fixtures/Insult_in.txt" "$assets/examples/fight.txt"
cp "$root/tests/fixtures/Value1_in.txt" "$assets/examples/value.txt"
cp "$root/tests/fixtures/sample-tanya-vs-tony.txt" "$assets/examples/preround.txt"
cp "$root/tests/jsonl-fixtures/session.jsonl" "$assets/examples/session.jsonl"
digest=$(cd "$assets" && find . -type f | LC_ALL=C sort | xargs shasum -a 256 \
    | shasum -a 256 | cut -c1-12)
mv "$assets" "$site/app-$digest"

sed -e "s|href=\"style.css\"|href=\"app-$digest/style.css\"|" \
    -e "s|src=\"app.js\"|src=\"app-$digest/app.js\"|" \
    "$root/web/index.html" >"$site/index.html"
if [[ $(grep -c "app-$digest/" "$site/index.html") -ne 2 ]]; then
    echo "index.html no longer links style.css and app.js as expected" >&2
    exit 1
fi
cp "$root/web/README.txt" "$site/README.txt"
cp "$root/LICENSE" "$site/LICENSE.txt"
if [[ -n "$build_info" ]]; then
    cp "$build_info" "$site/build-info.txt"
else
    echo "Git-Describe: $(git -C "$root" describe --tags --always --dirty)" >"$site/build-info.txt"
fi

# Fixed times keep the archive reproducible. They come from the commit rather
# than 1980 because a host without Cache-Control lets browsers keep a file for
# a tenth of its Last-Modified age.
export TZ=UTC
commit_time=$(git -C "$root" log -1 --format=%cd --date=format-local:%Y%m%d%H%M.%S)
find "$site" -type d -exec chmod 755 {} +
find "$site" -type f -exec chmod 644 {} +
find "$site" -exec touch -t "$commit_time" {} +
mkdir -p "$(dirname "$output")"
output=$(cd "$(dirname "$output")" && pwd)/$(basename "$output")
rm -f "$output"
(cd "$site" && find . -type f | sed 's|^\./||' | LC_ALL=C sort | zip -X -D -q "$output" -@)
echo "$output"
