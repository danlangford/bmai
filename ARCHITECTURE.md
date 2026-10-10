# BMAIR architecture

<!--
SPDX-License-Identifier: MIT
SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
-->

BMAIR is organized around its domain rather than around Rust language features.
The public crate root is a compatibility façade; implementation code lives with
the behavior it owns.

## Source layout

```text
src/
├── game/                 Button Men state, rules, attacks, and round mechanics
├── search/               AI policy, phase search, and simulation
│   └── test_support/     Test-only scenario builders
├── native/               Deterministic replay streams and ordered worker runtime
├── protocol/             External contracts and their adapters
│   ├── legacy/           Line protocol inherited from C++ BMAI
│   ├── jsonl/            Typed persistent-process protocol
│   └── types/            Shared protocol response types
├── lib.rs                Stable public façade
├── main.rs               Executable composition root
└── rng.rs                Reproducible random-number generation
```

Unit tests sit beside the module they protect. End-to-end protocol and golden
tests live under `tests/`, while reusable scenario builders remain behind
`cfg(test)` in `search/test_support/`.

The browser page in `web/` is another adapter, written in JavaScript. It runs
the unchanged `wasm32-wasip1` executable through its own WASI host, so it adds
no Rust entry point. `scripts/package_web.sh` bundles it into the release's
static site, and `tests/web/` holds it to the native golden output.

## Dependency direction

```text
main → protocol → search → game
                  ↓       ↓
                native ← rng
```

- `game` owns deterministic rules, legal-action enumeration, state transitions,
  and the historical stateless baseline selectors. It never runs probabilistic
  search or emits protocol output.
- `search` asks `game` for legal actions and applies game mechanics while it
  evaluates candidates. Search phases do not parse input.
- `native` owns replay-stream partitioning and ordered worker coordination.
- `protocol` translates external input into game/search calls and translates
  results back into stable wire types.
- `main` chooses an adapter and owns process I/O. Business behavior does not
  depend on the executable.
- `lib.rs` is the stable public façade over the implementation modules.

## Change placement

- Add or change a die skill in `game/`, then cover the rule with the mechanics
  scenario DSL.
- Change candidate evaluation or a decision phase in `search/`, then show it
  stays deterministic and worker-count independent at the relevant boundary.
- Add a wire command or response in `protocol/`; keep parsing and formatting
  out of game and search modules.
- Add cross-process behavior in `tests/`, not in unit-test helpers.

New dependencies must point in the direction above. If a lower layer needs a
higher-layer type, move the shared concept down to the layer that owns it
instead of adding a circular dependency.

## Rules and search references

[`RULES.md`](RULES.md) maps each ButtonWeavers rule to the code and tests
behind it, and [`SEARCH.md`](SEARCH.md) explains how search stays
deterministic. A change that moves the golden outputs regenerates them, and
the diff belongs in review.
