# BMAIR

<!--
SPDX-License-Identifier: MIT
SPDX-FileCopyrightText: Copyright 2001 Denis Papp
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

The Rust CI workflow builds and tests native `x86_64` and ARM64 binaries for
Linux, Windows, and macOS. Each platform/architecture pair is uploaded as a
separate workflow artifact; macOS Intel and Apple Silicon builds are not
combined into a universal binary.

## Running BMAIR

Pass a protocol file to the release executable:

```shell
cargo run --release --locked -- tests/fixtures/Insult_in.txt
```

Or pipe protocol input through standard input:

```shell
cargo run --release --locked < tests/fixtures/Insult_in.txt
```

The supported top-level commands are `game`, `playgame`, `compare`, `playfair`,
`getaction`, `ai`, `seed`, `surrender`, `ply`, `max_sims`, `min_sims`,
`maxbranch`, `turbo_accuracy`, `debug`, `debugply`, and `quit`. See
[`tests/fixtures/`](tests/fixtures/) for complete game-state examples.

## Verification

The default Rust test suite includes unit, parser, game-mechanics, and structural
search tests. Expensive reference-binary differential tests are marked ignored
because they require separately supplied C++ reference executables; their setup
and recorded evidence are documented in [`PARITY.md`](PARITY.md).
