# BMAIR

<!--
SPDX-License-Identifier: MIT
SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
-->

BMAIR is the Rust implementation of the Button Men AI. The `bmair` executable
accepts the same line-oriented game protocol and parser commands as the
original engine.

The original C++ engine is maintained separately at
[pappde/bmai](https://github.com/pappde/bmai). Its behavior and test cases are
the reference specification for this port.

## Lineage

BMAIR is a source-language port and derivative of Denis Papp's MIT-licensed
BMAI. This repository preserves the original Git history: the Rust port begins
at upstream BMAI commit
[`1fcb826`](https://github.com/pappde/bmai/commit/1fcb826c923a4b01a4a8b97e05f8b5cd0b3ce0d1),
and the Rust commits descend directly from it. The upstream and port copyright
notices are retained under the MIT license.

The parity record in [`PARITY.md`](PARITY.md) maps the C++ implementation and
tests to their Rust equivalents.

## Performance snapshot

Release-build wall times on the same Intel Mac are shown below. The C++ and
Rust 0.1.0 columns use the release artifacts; the parallel column uses
the current deterministic native search with eight workers. Lower is better.

| Fixture | C++ Release | Rust 0.1.0 legacy | Current native, 8 workers | Native vs. C++ | Native vs. Rust 0.1.0 |
|---|---:|---:|---:|---:|---:|
| `bmai_in.txt` | 6.23s | 7.30s | 1.81s | 3.44x faster | 4.03x faster |
| `bmsim_in.txt` | 18.19s | 19.16s | 10.59s | 1.72x faster | 1.81x faster |
| `bug11_in.txt` | 43.94s | 46.13s | 12.01s | 3.66x faster | 3.84x faster |
| `bug16_in.txt` | 139.95s | 226.90s | 78.75s | 1.78x faster | 2.88x faster |
| **Four-fixture total** | **208.31s** | **299.49s** | **103.16s** | **2.02x faster** | **2.90x faster** |

Native parallel search is opt-in and deterministic across worker counts, but it
does not promise legacy search decisions or RNG consumption. Eight workers also
raise peak memory substantially for `bug16_in.txt` (about 497MB to 1.86GB), so
the default remains one worker. [`BENCHMARKS.md`](BENCHMARKS.md) records commit
identities, build details, CPU time, memory, output checks, and methodology.

## Versioning

BMAIR preserves BMAI's source and rules lineage but starts its own semantic
version series at `bmair-v0.1.0`. A language port changes the executable,
packaging, public Rust API, and release lifecycle, so continuing BMAI's version
number would imply compatibility beyond the intentionally preserved protocol
and game behavior. Tag-derived build versions and Git descriptions remain in
every build for traceability. Release history follows the
[Keep a Changelog](CHANGELOG.md) convention.

## Development

Install the pinned Rust toolchain through mise, then use Cargo for project
tasks. If mise is activated in your shell, the `mise exec --` prefix is
optional.

```shell
mise install
mise exec -- cargo build --locked
mise exec -- cargo test --locked --all-targets --all-features
mise exec -- cargo fmt --check
mise exec -- cargo clippy --locked --all-targets --all-features -- -D warnings
mise exec -- cargo build --release --locked
```

Open this repository directly in RustRover. `Cargo.toml` is at the repository
root and the Rust sources follow the standard Cargo layout under `src/`.

Existing protocol samples are retained under `tests/fixtures/` for parity and
differential tests against the C++ implementation.

Pull requests validate the release declaration independently from Rust format,
lint, extended-test, and platform-build checks, so one failure does not hide
unrelated evidence. The shared build workflow tests native `x86_64` and ARM64
binaries for Linux, Windows, and macOS. Each platform/architecture pair is
uploaded as a separate workflow artifact; macOS Intel and Apple Silicon builds
are not combined into a universal binary. Release artifacts include build
metadata and SHA-256 checksums. Executable filenames use the embedded version,
platform, architecture, and profile, such as
`bmair-0.5.0-macos-arm64-release` for an exact release or
`bmair-0.5.0-dev.2+gabcdef0-macos-arm64-release` for a development build.

Every merge-ready pull request must increase the Cargo version and add its dated
`CHANGELOG.md` entry. The required PR gate fails if that version is already
tagged, is not greater than the base branch version, or lacks the changelog
entry. Release publication does not appear in pull-request checks. After the
squash merge and every release check passes, the release workflow creates the
annotated `bmair-v*` tag, stages and verifies every already-tested artifact on a
draft GitHub release, and publishes it once. This is compatible with immutable
releases: interrupted drafts can resume, while published tags and assets are
never modified. Pushing a matching tag remains a supported recovery/manual
release trigger.

Pull requests, default-branch pushes, and tags use Release builds by default so
release-only optimizer or linker failures are caught before tagging. Manual
workflow runs may select Debug for diagnostics.

## Running BMAIR

Pass a protocol file to the release executable:

```shell
cargo run --release --locked -- tests/fixtures/Insult_in.txt
```

Or pipe protocol input through standard input:

```shell
cargo run --release --locked < tests/fixtures/Insult_in.txt
```

`bmair --version` derives its displayed version from Cargo and
`git describe`. An exact `bmair-v0.5.0` tag reports `0.5.0`; development builds
report the upcoming Cargo version plus the number of commits since the previous
release, abbreviated commit SHA, and a `dirty` suffix when appropriate.

The supported top-level commands are `game`, `playgame`, `compare`, `playfair`,
`getaction`, `ai`, `mode`, `rng`, `workers`, `seed`, `surrender`, `ply`, `max_sims`,
`min_sims`, `maxbranch`, `turbo_accuracy`, `debug`, `debugply`, and `quit`. See
[`tests/fixtures/`](tests/fixtures/) for complete game-state examples.
Whole lines whose first non-whitespace character is `#` may be used as comments
between top-level commands. Inline comments and comments inside `game` blocks
are not supported.

### Python and service integration

Long-lived clients should start `bmair --protocol jsonl-v1` and exchange one
request and response per line. This interface has request IDs, typed actions,
structured recoverable errors, capability discovery, transactional session
updates, and native replay metadata; it emits no human banner on stdout.

The complete wire contract and compatibility policy are in
[`PROTOCOL.md`](PROTOCOL.md). A dependency-free persistent Python client is in
[`examples/python/bmair_jsonl.py`](examples/python/bmair_jsonl.py). It is
intended to support Python consumers such as bmaibagels without requiring those
consumers to move engine logic into Python or Rust.

Existing subprocess clients may continue using the C++-compatible legacy
protocol. BMAIR flushes its banner, processes complete stdin commands without
waiting for EOF, and treats `quit` as immediate termination. This supports the
historical BMAIBagels `Popen` pattern of writing and flushing a complete legacy
request while keeping the pipe open to read the action.

### Execution and RNG modes

`mode legacy` selects the exact C++ compatibility contract and remains the
default. `mode parity` is an alias. `mode native` selects the explicit
Rust-native evolution point and currently enables deterministic per-simulation
RNG streams plus bounded parallel candidate evaluation.

Native search defaults to `workers 1`. Set an explicit positive count or use
`workers auto` to resolve the logical CPU parallelism available to the process.
The resolved count is reported and included in replay metadata; worker settings
do not affect legacy search.

`rng legacy` selects BMAI's Park-Miller minimal-standard generator (multiplier
16807, modulus 2^31-1) with BMAI's historical seed expansion. `rng park-miller`
is an alias. Its stable replay identifier is
`bmai-park-miller-16807-v1`. Selecting an RNG does not reseed it; use `seed`
separately. Protocols intended for durable replay should record the execution
mode, RNG replay identifier, seed, BMAIR version, and all search
settings. The compatibility and replay contracts are defined in
[`MODES.md`](MODES.md).

The first Rust-native search experiment is specified in
[`NATIVE_MODE.md`](NATIVE_MODE.md). It targets deterministic parallel
candidate simulation while keeping legacy mode as the compatibility oracle.

## Verification

The default Rust test suite includes unit, parser, game-mechanics, and structural
search tests. Expensive reference-binary differential tests are marked ignored
because they require separately supplied C++ reference executables; their setup
and recorded evidence are documented in [`PARITY.md`](PARITY.md). New mechanics
tests can use the recipe-based scenario DSL described in
[`TESTING.md`](TESTING.md).
