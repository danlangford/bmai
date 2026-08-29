# Changelog

<!--
SPDX-License-Identifier: MIT
SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
-->

All notable changes to BMAIR are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - 2026-08-29

### Added

- Incremental `legacy-v1` standard-input execution for long-lived subprocess
  callers such as BMAIBagels, including banner flushing and `quit` termination
  without requiring the caller to close stdin.
- Process-level regression coverage for the BMAIBagels write, flush, read, and
  submit workflow.

### Changed

- Standard-input legacy commands now execute as soon as a complete command or
  game block arrives, matching the original C++ parser. File inputs, JSONL, AI
  decisions, output syntax, and RNG behavior remain unchanged.

## [0.3.0] - 2026-08-28

### Added

- A versioned, Python-friendly machine-to-machine integration contract for
  capability discovery, structured JSON Lines requests and responses, typed
  actions, replay metadata, and reusable multi-request sessions.
- A public transactional `BmairSession` Rust API and dependency-free persistent
  Python subprocess example.
- Protocol specifications, compatibility guarantees, consumer examples, and
  cross-protocol contract tests while retaining `legacy-v1` unchanged.
- Discoverable BMAIR die-notation tokens, stable skill identifiers, and
  implementation-support labels for recipe translators.

### Changed

- Machine responses now carry build identity, complete global/per-player search
  settings, original die indices, and the exact native decision replay key.

## [0.2.0] - 2026-08-28

### Added

- Opt-in native execution mode with versioned, deterministic per-simulation
  random streams.
- Bounded parallel candidate evaluation for fight, preround/swing, reserve,
  Chance, and Focus search through the `workers` protocol command.
- Native wire-protocol fixtures, deterministic replay tests, performance
  benchmarks, and a preregistered paired playing-strength experiment.
- README performance comparison covering C++, Rust 0.1.0 legacy mode, and
  eight-worker native mode.

### Changed

- Build versions are derived from `bmair-v*` tags with commit distance, SHA,
  and dirty state retained for development builds.
- A release-ready default-branch merge builds and tests all six
  platform/architecture targets before creating its version tag and publishing
  binaries, checksums, build metadata, and changelog notes.
- Pull-request release-policy checks run independently from format, lint, test,
  and platform builds; release-publication jobs appear only on release runs.
- GitHub releases stage and verify their complete asset set as resumable drafts
  before one-way publication, making the pipeline compatible with immutable
  releases.

### Fixed

- Complete matches now mirror C++ tied-round standings and offer the losing
  player a reserve decision between nonterminal rounds.
- Native replay indices advance only when a native BMAI search actually runs.

## [0.1.0] - 2026-08-28

### Added

- Initial Rust port release of BMAI with compatible parser commands, game
  mechanics, AI search, candidate ordering, simulation counts, culling, and
  legacy Park-Miller RNG consumption.
- Rust mappings for the upstream C++ tests and seeded differential fixtures,
  including Konstant PR #82 mechanics and regression coverage.
- Release builds for ARM64 and x86_64 Linux, Windows, and macOS with SHA-256
  checksums and build metadata.

### Changed

- Established BMAIR's independent semantic-version series while preserving the
  original BMAI Git history, MIT license, and source lineage.
- Applied parity-preserving storage, simulation-reuse, enumeration, restoration,
  and compiler/linker optimizations.

[Unreleased]: https://github.com/danlangford/bmai/compare/bmair-v0.4.0...HEAD
[0.4.0]: https://github.com/danlangford/bmai/compare/bmair-v0.3.0...bmair-v0.4.0
[0.3.0]: https://github.com/danlangford/bmai/compare/bmair-v0.2.0...bmair-v0.3.0
[0.2.0]: https://github.com/danlangford/bmai/compare/bmair-v0.1.0...bmair-v0.2.0
[0.1.0]: https://github.com/danlangford/bmai/releases/tag/bmair-v0.1.0
