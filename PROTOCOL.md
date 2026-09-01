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
worker constraints, and the complete BMAIR die-notation vocabulary.

```json
{"protocol":"jsonl-v1","id":"caps-1","method":"capabilities"}
```

The `die_notation` object lets clients translate external recipes without
hard-coding BMAIR's skill abbreviations. `property_prefixes` reports each
one-character token with a stable snake-case `id`, display `name`, and
`support` of `implemented` or `parsing_only`. `postfix_properties` reports
Turbo (`!`) and Mood (`?`). The remaining fields describe swing types `P-Z`,
option and Twin punctuation, defined-side selection, rolled values, and the
dizzy marker. These are BMAIR wire tokens, not a claim that BMAIR parses the
Buttonweavers recipe grammar.

For example, discovery identifies `d` as Stealth, `p` as Poison, `z` as Speed,
and `G` as the parsing-only Rage property. Consumers should use this metadata
instead of maintaining a parallel token-to-skill table.

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

- `build`: Cargo/tag-derived version, Git description, and build profile;
- `legacy_output`: exact text emitted by the legacy parser;
- `action`: the typed result of the last `getaction`, or null;
- `session`: phase/game rules, execution/RNG modes, next native decision index,
  worker count, global settings, and per-player AI/search settings after
  execution;
- `replay`: the native stream partition, root seed, and decision index actually
  used by the last native BMAI search, or null when no native search ran.

Replay metadata describes the top-level decision. Candidate/batch/simulation
coordinates are deterministically derived inside the versioned partition.
Legacy searches deliberately return null because reproducing an arbitrary
legacy continuation requires the preceding RNG stream, not merely its seed.
Finite settings are JSON numbers. Legacy accepts non-finite Turbo accuracy
values, so those exceptional values serialize as `"nan"`, `"infinity"`, or
`"-infinity"` instead of becoming invalid or misleading JSON.

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

Standard input is incremental. BMAIR flushes the four-line banner before
waiting for input, executes each single-line command as it arrives, and
executes a `game` after receiving its phase and both complete player/die
blocks. Output is flushed after every complete top-level command. `quit`
terminates immediately without waiting for EOF. This preserves the original
C++ subprocess contract used by clients that write and flush a request, keep
stdin open, and then read the response. File arguments remain batch inputs.
After trimming whitespace, a whole top-level line beginning with `#` is ignored.
Inline comments and comments within a `game` phase/player/die block are invalid.

Top-level BMAI fight searches emit the legacy `l1 p0 best move` diagnostic
before `stats` and `action`. Its parenthesized fields include the accumulated
winning score and numeric win percentage used by historical subprocess
consumers. Recursive search diagnostics remain internal.

The stable command forms are:

| Form | Effect |
|---|---|
| `game [TARGET_WINS]` | Begin a two-player state; the following line is a phase, followed by two `player ID DICE SCORE` blocks and one die per line. |
| `ai PLAYER TYPE` | Select unculled BMAI (`0`), QAI (`1`), or culled BMAI (`2`). |
| `mode legacy\|parity\|native` | Select compatible or opt-in native execution. |
| `rng legacy\|park-miller` | Select the versioned BMAI Park-Miller stream. |
| `workers N` / `workers auto` | Configure at least one native worker, or use the logical CPU parallelism available to the process; legacy results are unaffected. |
| `seed N` | Seed legacy RNG state and the native root; zero resolves from wall-clock time. |
| `ply [PLAYER] N` | Set global or per-player BMAI depth. |
| `max_sims [PLAYER] N` | Set global or per-player maximum simulations. |
| `min_sims [PLAYER] N` | Set global or per-player minimum simulations. |
| `maxbranch [PLAYER] N` | Set global or per-player branch budget. |
| `turbo_accuracy F` | Control Turbo choices considered from extremes (`0`) to all (`1`). |
| `surrender on\|off` | Enable or disable surrender selection. |
| `getaction` | Select an action for player zero in the supplied phase. |
| `playgame N` / `compare N` | Run N complete games from a preround state. |
| `playfair N MODE P` | Run position-swapped games with the selected rollout mode. |
| `debug CATEGORY 0\|1` / `debugply N` | Configure legacy diagnostics. |
| `quit` | Stop consuming the current script. |

Phases are `preround`, `reserve`, `initiative`, `chance`, `focus`, `fight`, and
`gameover`. `getaction` is defined for preround, reserve, Chance, Focus, and
fight; initiative/gameover are state-description phases rather than direct
action requests. Legacy parser errors terminate the process with a nonzero exit
status. JSONL converts those same errors into recoverable `execution_error`
responses and rolls back the request.

Game-state syntax and multiline action examples live in
[`tests/fixtures/`](tests/fixtures/); deterministic native examples live in
[`tests/native-fixtures/`](tests/native-fixtures/). A complete persistent JSONL
conversation is executable at
[`tests/jsonl-fixtures/session.jsonl`](tests/jsonl-fixtures/session.jsonl). The
exact compatibility and differential evidence is maintained in
[`PARITY.md`](PARITY.md).

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

The process protocol is the cross-language compatibility boundary. The public
Rust types follow Cargo semantic versioning and may gain fields or
`#[non_exhaustive]` variants independently of the JSON forward-compatibility
rules.

## Operational boundary

BMAIR is a local computation engine, not a network server. It performs no
authentication, authorization, request-size limiting, timeout enforcement, or
per-session resource accounting. A service should keep the subprocess private,
validate its own inputs, constrain worker/search settings, apply process-level
time and memory limits, and restart the process after an unexpected exit. Do
not expose `session.execute` directly to untrusted network users.
