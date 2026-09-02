# Changelog

<!--
SPDX-License-Identifier: MIT
SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
-->

All notable changes to BMAIR are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned

- Complete mechanics support for the remaining parsing-only skills:
  Auxiliary (`+`) and Radioactive (`%`).
- Add parser and mechanics support for Fire and Rush. Fire work must include
  the documented Rage+Fire interaction: firing does not consume Rage.

## [0.8.2] - 2026-09-02

### Changed

- Advanced native replay partitioning to `bmair-native-stream-v2`. Native
  search now stratifies a simulation's initial consecutive bounded draws
  across mixed-radix outcome cells, reducing sampling noise for both ordinary
  and multi-die rerolls without making results depend on worker count or
  completion order.
- Continue evaluating native search's surviving best candidate through the
  declared simulation budget after weaker candidates are culled. Legacy mode
  retains the original C++ early-stop behavior.

### Fixed

- Corrected the native win estimate for the reported Poison-versus-Queer
  endgame from ButtonWeavers game 119365. The exact input now selects the same
  Power capture and reports the position's 10% win probability in both legacy
  and native modes.
- Corrected the native estimates for reconstructed ordinary-d10 and Twin-d6
  endgames by covering their one- and two-die reroll distributions evenly.
- Applied complete-survivor probability sampling consistently to native fight,
  preround, Chance, and Focus searches; native reserve search already sampled
  every candidate through its full budget.

## [0.8.1] - 2026-09-02

### Added

- Added parser/search scenario test DSLs that drive the production protocol,
  assert typed and wire-format actions, and check exact or ranged win
  percentages across execution modes and worker configurations.
- Added the reported forced-win position as a permanent C++/Rust differential
  fixture and exercised it in legacy and native modes.

### Fixed

- Applied a shared defined Swing size to every Swing half of a Twin die, so a
  recipe such as `(T,T)-2` is parsed as `(2,2)` instead of `(2,0)`.
- Preserved defined Swing sizes when Turbo or Mood postfix markers follow the
  numeric size, matching the original C++ parser.
- Kept a zero Swing suffix undefined instead of incorrectly locking it.

## [0.8.0] - 2026-09-01

### Added

- Implemented the ButtonWeavers Rage (`G`) skill. Rage dice are excluded from
  initiative, lose Rage when they participate in an attack, and produce a
  rolled same-recipe replacement without Rage when captured.
- Added readable mechanics scenarios for every Rage rule and documented
  Doppelganger interaction, plus Jolt, Time and Space, Konstant, Null, Value,
  Poison, Radioactive, Mighty, Weak, Mood, Twin, Turbo, Speed, Trip, round
  restoration, and transformed-capacity behavior.

### Changed

- Advertise Rage as implemented through machine-readable capabilities.
- Track Rage replacements as bounded round-local dice and restore attacking
  Rage properties when the next game round begins.

## [0.7.0] - 2026-09-01

### Added

- Added a dependency-free, human-readable Button Men scenario DSL for
  mechanics tests. Scenarios use production parsing, attack enumeration, and
  resolution while expressing setup and expectations as die recipes. Existing
  scoring, Konstant, Jolt, Time and Space, Doppelganger, Turbo, Rage, and
  Radioactive+Doppelganger cases exercise the DSL directly.

### Changed

- Moved scenario construction, canonical die formatting, and state assertions
  into a dedicated test-only simulation module, keeping this testing API out
  of release binaries and providing a focused home for future skill tests.

## [0.6.0] - 2026-09-01

### Added

- Implemented the ButtonWeavers Doppelganger (`D`) skill. A successful
  single-die Power attack replaces the attacker with an exact copy of the
  captured die for the rest of the round, then rerolls it.
- Added focused Doppelganger coverage for ordinary and Skill attacks, Twin and
  Swing recipes, copied Doppelganger, Jolt, Time and Space, Konstant, Mighty,
  Turbo, Rage, Radioactive decay products, and restoration of the original
  recipe for the next round.

### Changed

- Enforced BMAI's historical maximum of ten input dice per player with a clear
  parser error, while reserving twenty in-round slots for
  Radioactive+Doppelganger transfers and reporting capacity exhaustion.

### Fixed

- Corrected the internal spelling of the Doppelganger property; the existing
  user-facing `Doppelganger` name and `D` notation were already correct.

## [0.5.0] - 2026-09-01

### Added

- Implemented the Jolt (`J`) skill: an attacking Jolt die loses Jolt and
  grants another turn, while capturing a Jolt die also grants another turn.
- Added Jolt interaction coverage for multiple attackers, unsuccessful Trip
  attacks, captured defenders, Konstant, and Time and Space.
- Added `workers auto` to use the logical CPU parallelism available to the
  process for native search while recording the resolved worker count.
- Added embedded build versions to platform executable and workflow-artifact
  filenames so downloaded development and release builds remain distinguishable.
- Added whole-line `#` comments between top-level legacy protocol commands for
  both file and incremental standard-input parsing.

### Changed

- Displayed Dan Langford's BMAIR copyright before the original BMAI
  attribution so single-copyright legacy clients identify the current port.

## [0.4.1] - 2026-08-29

### Fixed

- Restored the top-level legacy fight-search `best move` diagnostic, including
  its numeric win percentage, so existing subprocess consumers such as
  BMAIBagels can continue extracting odds from C++-compatible output.

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

[Unreleased]: https://github.com/danlangford/bmai/compare/bmair-v0.8.2...HEAD
[0.8.2]: https://github.com/danlangford/bmai/compare/bmair-v0.8.1...bmair-v0.8.2
[0.8.1]: https://github.com/danlangford/bmai/compare/bmair-v0.8.0...bmair-v0.8.1
[0.8.0]: https://github.com/danlangford/bmai/compare/bmair-v0.7.0...bmair-v0.8.0
[0.7.0]: https://github.com/danlangford/bmai/compare/bmair-v0.6.0...bmair-v0.7.0
[0.6.0]: https://github.com/danlangford/bmai/compare/bmair-v0.5.0...bmair-v0.6.0
[0.5.0]: https://github.com/danlangford/bmai/compare/bmair-v0.4.1...bmair-v0.5.0
[0.4.1]: https://github.com/danlangford/bmai/compare/bmair-v0.4.0...bmair-v0.4.1
[0.4.0]: https://github.com/danlangford/bmai/compare/bmair-v0.3.0...bmair-v0.4.0
[0.3.0]: https://github.com/danlangford/bmai/compare/bmair-v0.2.0...bmair-v0.3.0
[0.2.0]: https://github.com/danlangford/bmai/compare/bmair-v0.1.0...bmair-v0.2.0
[0.1.0]: https://github.com/danlangford/bmai/releases/tag/bmair-v0.1.0
