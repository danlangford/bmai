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

Native parallel search is the default since 0.23.0 and is deterministic across
worker counts, but it does not promise legacy search decisions or RNG
consumption. Eight workers raise peak memory substantially for `bug16_in.txt`
(about 497MB to 1.86GB), so `workers 1` suits machines short of memory. [`BENCHMARKS.md`](BENCHMARKS.md) records commit
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
[`ARCHITECTURE.md`](ARCHITECTURE.md) explains the game, search, protocol, and
runtime boundaries and where new behavior belongs.

Protocol samples under `tests/fixtures/` have golden outputs in
`tests/golden/`, recording each fixture's output and RNG fingerprint.

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
`min_sims`, `maxbranch`, `report_sims`, `turbo_accuracy`, `fire_overshooting`,
`debug`, `debugply`, and `quit`. See
[`tests/fixtures/`](tests/fixtures/) for complete game-state examples.
Whole lines whose first non-whitespace character is `#` may be used as comments
between top-level commands. Inline comments and comments inside `game` blocks
are not supported, so a Rush die line such as `#6:6` inside a `game` block is
always a die.

Each player is driven by a named engine: `random`, `maximize`, `quick` (the
C++ Quick AI), or `montecarlo` (BMAI's simulation search, the default).
`ai PLAYER NAME` selects one. `ply`, `max_sims`, `min_sims`, `maxbranch`,
`cull`, `playout`, and `endgame` take an optional player. Without one they set the global
Monte Carlo settings every player starts each `game` with; with one they change only that
player's engine. An engine rejects settings it does not use, and Monte Carlo
`ply` must be at least 1.

### Testing a button against a field

`bmair gauntlet` plays one button against a list of opponents and reports how
often it wins:

```shell
bmair gauntlet --engine "montecarlo max_sims=20 min_sims=5" "dk(1) k(V) k(V) k(V) dmMH(4)"
```

```text
Button: dk(1) k(V) k(V) k(V) dmMH(4)
Engine: montecarlo max_sims=20 min_sims=5
500 games per opponent: seeds 1-250, each played from both seats

Opponent        Won           Win %  95% CI       Rounds won
Lucky           334/500       66.8%  62.8-70.8%        59.3%
Vincent         390/500       78.0%  74.6-81.4%        65.6%
Sailor Jupiter  194/500       38.8%  34.8-42.8%        49.4%
Hammer          261/500       52.2%  47.7-56.7%        52.4%
Monkeys         65/500        13.0%  10.1-15.9%        26.7%
Lady K          213/500       42.6%  38.3-46.9%        45.6%
Konami          7/500          1.4%  0.4-2.4%          15.4%
Wolfman         318/500       63.6%  59.5-67.7%        57.4%
Overall         1782/4000     44.5%  42.9-46.2%        47.2%
```

Recipes may use ButtonWeavers notation, such as `p(12)`, `(X=12)`, and `(X)?`,
or BMAIR's own, such as `p12`, `X-12`, and `X?`. Quote the recipe so the shell
passes it as one argument.

Without an opponents file, the gauntlet plays a default field of eight
established buttons that win between 40% and 60% on ButtonWeavers.
`bmair gauntlet --field` prints that field as an opponents file, one
`Name: recipe` per line, ready to copy and edit. Pass the edited file after the
recipe.

Each opponent gets 500 matches, first to three, with every seed played once
from each seat. One engine plays both seats: `montecarlo` at its default
settings, unless `--engine` names another in the strength harness's
`name setting=value` form. The default takes minutes for the whole field;
`montecarlo max_sims=20 min_sims=5`, as above, takes seconds. Each match runs
native search on one worker, and matches run in parallel across every core.
`--games`, `--seed`, and `--threads` set the rest, and `--help` lists them. The same seeds give the same results at any thread count. At 500 matches
the 95% interval is about four points either way.

### Skills

BMAIR implements the C++ engine's skills plus these ButtonWeavers skills that
the C++ engine predates or only parses: Auxiliary, Boom, Doppelganger, Fire,
Jolt, Mad, Radioactive, Rage, and Rush. No advertised die skill is parsing-only; known
gaps are listed under Planned in the CHANGELOG. Clients should discover the
exact die tokens, skills, and attack types (including `rush` and `boom`) through
capabilities rather than hard-coding them.

Some buttons carry a button special, a rule for the whole button. The wire
format has no button names, so clients name the rule after the `game` block,
for example `special 0 unique_sizes` for Gordo. Capabilities `button_specials`
lists each rule and the buttons that use it.

### Python and service integration

Long-lived clients should start `bmair --protocol jsonl-v1` and exchange one
request and response per line. This interface has request IDs, typed actions,
structured recoverable errors, capability discovery, transactional session
updates, native replay metadata, and structured probability evaluations; it
emits no human banner on stdout.

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

`mode native` is the default since 0.23.0: deterministic per-simulation RNG
streams plus bounded parallel candidate evaluation. `mode legacy` selects the
exact C++ compatibility contract, and `mode parity` is an alias.

Native search defaults to `workers auto`. Set an explicit positive count or use
`workers auto` to resolve the logical CPU parallelism available to the process.
The resolved count is reported and included in replay metadata; worker settings
do not affect legacy search.

For a user-visible probability estimate, `report_sims N` keeps normal bounded
search responsible for choosing the fight move, then evaluates only that move
with exactly `N` fresh native samples. It defaults to zero, requires native
BMAI fight search to produce a report, and does not change the selected action
or consume a later decision stream. Clients should discover the command through
capabilities before using it.

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
search tests, plus golden output for every fixture. The longest fixture
searches are ignored by default and run in CI on every pull request:

```shell
cargo test --release --test fixture_golden -- --include-ignored
```

After an intentional behavior change, regenerate the golden files with
`BMAIR_UPDATE_GOLDEN=1` and review their diff. New mechanics
tests can use the recipe-based scenario DSL described in
[`TESTING.md`](TESTING.md).
