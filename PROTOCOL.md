# BMAIR integration protocol

<!--
SPDX-License-Identifier: MIT
SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
-->

BMAIR exposes two versioned process protocols. `legacy-v1` is the permanent
C++-compatible text protocol. `jsonl-v1` is the machine-to-machine contract for
long-lived clients such as Python services. JSONL is additive: it executes the
same parser and search paths and returns both a typed action and the exact
legacy response.

## Discover capabilities

`bmair --capabilities` writes one JSON document to stdout and no banner. A
running JSONL session also accepts a `capabilities` request. Discovery reports
the build identity, protocols, parser commands, phases, typed actions, attack
types, implemented skills, parsing-only upstream skills, execution modes, RNGs,
and worker constraints.

```json
{"protocol":"jsonl-v1","id":"caps-1","method":"capabilities"}
```

## JSON Lines v1

Start a session with `bmair --protocol jsonl-v1`. Send exactly one UTF-8 JSON
object per line and read exactly one response per nonblank line. The process
flushes each response immediately, preserves session state across requests, and
ends normally at EOF. Protocol output is exclusively stdout; diagnostics and
fatal process errors belong on stderr.

Every request has these fields:

| Field | Type | Meaning |
|---|---|---|
| `protocol` | string | Must be `jsonl-v1`. |
| `id` | string, number, or null | Returned unchanged for correlation. |
| `method` | string | Operation name. |
| `params` | object or null | Method parameters; omitted means null. |

Every response repeats `protocol` and `id`. Success has `ok: true` and a
`result`; failure has `ok: false` and an `error` containing stable `code`, human
`message`, and `recoverable`. A malformed line or rejected operation does not
terminate the session. State-changing requests are transactional: a rejected
script leaves the previous session intact.

### Methods

`capabilities` takes no parameters. `session.reset` takes no parameters and
restores a fresh default parser. `session.execute` accepts:

```json
{"script":"seed 17\nply 1\n"}
```

The script is the migration bridge from the legacy command language. Its
success result contains:

- `legacy_output`: exact text emitted by the legacy parser;
- `action`: the typed result of the last `getaction`, or null;
- `session`: phase, execution/RNG modes, next native decision index, worker
  count, and search settings after execution;
- `replay`: the native stream partition, root seed, and decision index actually
  used by the last native BMAI search, or null when no native search ran.

Replay metadata describes the top-level decision. Candidate/batch/simulation
coordinates are deterministically derived inside the versioned partition.
Legacy searches deliberately return null because reproducing an arbitrary
legacy continuation requires the preceding RNG stream, not merely its seed.

### Typed actions

Actions use a `type` discriminator:

- `{"type":"pass"}`
- `{"type":"surrender"}`
- `{"type":"attack","attack_type":"power","attackers":[0],"targets":[1]}`
- `{"type":"reserve","die":2}`; `die` is null when reserve is declined
- `{"type":"set_swing","swings":[{"swing":"X","value":12}],"options":[{"die":1,"value":20}]}`
- `{"type":"chance","dice":[0,2]}`
- `{"type":"focus","dice":[{"die":0,"value":4}]}`

An attack may include `turbo`. Option Turbo is
`{"kind":"option","die":0,"value":20}`; swing Turbo is
`{"kind":"swing","swing":"X","value":12}`. Die numbers are original
wire-protocol indices, even when internal dice storage is optimized.

Stable error codes in v1 are `invalid_json`, `invalid_request`,
`unsupported_protocol`, `invalid_params`, `method_not_found`, and
`execution_error`.

## Legacy v1

Run `bmair [FILE]` or pipe text to stdin. The startup banner and all parser
output are part of this human-oriented interface. The command set is:

`game`, phase names, `player`, `ai`, `mode`, `rng`, `workers`, `seed`, `ply`,
`max_sims`, `min_sims`, `maxbranch`, `turbo_accuracy`, `surrender`, `getaction`,
`playgame`, `playfair`, `compare`, `debug`, `debugply`, and `quit`.

Game-state syntax and multiline action examples live in
[`tests/fixtures/`](tests/fixtures/); deterministic native examples live in
[`tests/native-fixtures/`](tests/native-fixtures/). The exact compatibility and
differential evidence is maintained in [`PARITY.md`](PARITY.md).

## Compatibility policy

Released protocol identifiers are immutable contracts. Within `jsonl-v1`, new
methods, optional response fields, capability entries, action variants, and
error codes may be added. Existing field meaning, discriminator meaning, and
required fields will not change. Clients must ignore unknown object fields and
use capability discovery before relying on optional behavior. Removing or
retyping existing behavior requires a new protocol identifier.

`legacy-v1` remains frozen to the upstream C++ compatibility contract described
in `PARITY.md`. Native execution may intentionally evolve, but its algorithms
and replay partitions are explicitly versioned and opt-in.

