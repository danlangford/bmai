# BMAIR integration protocol

<!--
SPDX-License-Identifier: MIT
SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
-->

BMAIR exposes two versioned process protocols. `legacy-v1` is the line-based
text protocol inherited from C++ BMAI. `jsonl-v1` is the machine-to-machine contract for
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
Turbo (`!`), Mood (`?`), and Mad (`&`). The remaining fields describe swing types `P-Z`,
option and Twin punctuation, defined-side selection, rolled values, and the
dizzy marker. These are BMAIR wire tokens, not a claim that BMAIR parses the
Buttonweavers recipe grammar.

For example, discovery identifies `d` as Stealth, `p` as Poison, `z` as Speed,
`F` as Fire, `G` as Rage, `#` as Rush, and `b` as Boom. Consumers should use this metadata instead of
maintaining a parallel token-to-skill table.

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
  used by the last native BMAI search, or null when no native search ran,
  including when the endgame solver answered;
- `evaluation`: the last fight-search probability estimate, or null. It contains
  `player`, a zero-to-one `probability`, `simulations`, and a `source` of
  `move_selection` or `selected_move_resample`. `simulations` of 0 marks an
  exact endgame answer.

Replay metadata describes the top-level decision. Candidate/batch/simulation
coordinates are deterministically derived inside the versioned partition; see
[`SEARCH.md`](SEARCH.md). `session.execution_mode` is always `native`.
Finite settings are JSON numbers. The text protocol accepts non-finite Turbo accuracy
values, so those exceptional values serialize as `"nan"`, `"infinity"`, or
`"-infinity"` instead of becoming invalid or misleading JSON.

### Typed actions

Actions use a `type` discriminator:

- `{"type":"pass"}`
- `{"type":"surrender"}`
- `{"type":"auxiliary","die":1}`; `die` is null when Auxiliary is declined
- `{"type":"attack","attack_type":"power","attackers":[0],"targets":[1]}`;
  `attack_type` is one of the advertised `attack_types`, including `rush` and `boom`
- `{"type":"reserve","die":2}`; `die` is null when reserve is declined
- `{"type":"set_swing","swings":[{"swing":"X","value":12}],"options":[{"die":1,"value":20}]}`
- `{"type":"chance","dice":[0,2]}`
- `{"type":"focus","dice":[{"die":0,"value":4}]}`

An attack may include `turbo`. Option Turbo is
`{"kind":"option","die":0,"value":20}`; swing Turbo is
`{"kind":"swing","die":0,"swing":"X","value":12}`. ButtonWeavers lets each
attacking Turbo die that rerolls and has not just morphed choose a size, and
`setTurboSize` resizes only that die. BMAIR chooses for the first such die
that has a size to choose, which `die` names; a client submits the current
size of any other attacking Turbo die, since ButtonWeavers needs a size for
each. Legacy output's `option DIE VALUE` and `swing X VALUE` lines describe
the same die. Die numbers are original wire-protocol indices, even when
internal dice storage is optimized.
An assisted attack includes `"fire":[{"die":1,"value":3}]`, naming the
final displayed value of each Fire die turned down for the attack. The field is
omitted for attacks without Fire assistance.

Stable error codes in v1 are `invalid_json`, `invalid_request`,
`unsupported_protocol`, `invalid_params`, `method_not_found`, and
`execution_error`.

## Legacy v1

Run `bmair [FILE]` or pipe text to stdin. The startup banner and all parser
output are part of this human-oriented interface. The command set is:

`game`, phase names, `player`, `ai`, `mode`, `rng`, `workers`, `seed`, `ply`,
`max_sims`, `min_sims`, `maxbranch`, `report_sims`, `turbo_accuracy`, `fire_overshooting`, `special`,
`surrender`, `getaction`,
`playgame`, `playfair`, `compare`, `debug`, `debugply`, and `quit`.

Standard input is incremental. BMAIR flushes the four-line banner before
waiting for input, executes each single-line command as it arrives, and
executes a `game` after receiving its phase and both complete player/die
blocks. Output is flushed after every complete top-level command. `quit`
terminates immediately without waiting for EOF. This preserves the original
engine's subprocess contract used by clients that write and flush a request, keep
stdin open, and then read the response. File arguments remain batch inputs.
After trimming whitespace, a whole top-level line beginning with `#` is ignored.
Inline comments and comments within a `game` phase/player/die block are invalid.
Inside a `game` block every die line is a die definition, so a Rush die such as
`#6:6` is never mistaken for a comment.

Top-level BMAI fight searches emit the legacy `l1 p0 best move` diagnostic
before `stats` and `action`. Its parenthesized fields include the accumulated
winning score and numeric win percentage used by historical subprocess
consumers. When `report_sims` is nonzero, a separate `l1 p0 selected move
report` diagnostic follows it with the fresh score, sample count, and win
percentage. Recursive search diagnostics remain internal.
After the attacker and target index lines and any Turbo selection, an assisted
attack emits one `fire DIE VALUE` line per assisting Fire die. `DIE` is its
original input index and `VALUE` is the final displayed value to submit to
ButtonWeavers.

The stable command forms are:

| Form | Effect |
|---|---|
| `game [TARGET_WINS]` | Begin a two-player state; the following line is a phase, followed by two `player ID DICE SCORE` blocks and one die per line. |
| `ai PLAYER NAME` | Select the player's engine: `random`, `maximize`, `quick`, or `montecarlo`. |
| `mode native` | Accepted for older clients and does nothing. `mode legacy` and `mode parity` are errors: the C++-compatible search was removed. |
| `rng legacy\|park-miller` | Select the versioned BMAI Park-Miller stream. |
| `workers N` / `workers auto` | Configure at least one native worker, or use the logical CPU parallelism available to the process (default `auto`); results do not depend on it. |
| `seed N` | Seed the session generator and the native root; zero resolves from wall-clock time. |
| `ply [PLAYER] N` | Set global or per-player Monte Carlo depth, at least 1 (default 1); see below. |
| `max_sims [PLAYER] N` | Set global or per-player maximum simulations (default 4000). |
| `min_sims [PLAYER] N` | Set global or per-player minimum simulations (default 200). |
| `maxbranch [PLAYER] N` | Set global or per-player branch budget (default 16000); together with `min_sims`, this also bounds Fire-assisted candidates materialized per state. |
| `cull [PLAYER] on\|off` | Let Monte Carlo search drop clearly losing candidates early (default on). |
| `playout [PLAYER] quick\|maximize\|random` | Choose the engine that plays Monte Carlo's simulated games (default `quick`). |
| `endgame [PLAYER] N` | Once N or fewer dice remain on both sides together, choose the attack by solving the rest of the round exactly, with every reroll's exact odds, instead of sampling (default 4; 0 never). A position too large, or one that can repeat (a Trip that keeps failing), is searched as usual. An exact answer reports zero simulations and skips `report_sims`. |
| `report_sims N` | After native BMAI fight search chooses a move, evaluate only that move with exactly N fresh samples; zero disables the report and is the default. |
| `turbo_accuracy F` | Control Turbo choices considered from extremes (`0`) to all (`1`). |
| `special PLAYER [ID...]` | Apply button specials to a player for the current game; `game` clears them. IDs are listed in capabilities `button_specials` with the buttons that use each. |
| `fire_overshooting on\|off` | Permit optional Fire adjustments on Power attacks that are already legal for both sides of simulated continuations; defaults to `on`; ButtonWeavers players who have not turned on that preference should send `off`. |
| `surrender on\|off` | Enable or disable surrender selection (default `off`). |
| `getaction` | Select an action for player zero in the supplied phase. |
| `playgame N` / `compare N` | Run N complete games from a preround state. Auxiliary dice are decided once, before the first round, as in an `aux` state: player 0 chooses first, and a decline by either player removes every Auxiliary die for the whole game. A game is cancelled when its 200th round ends, whatever that round's result, and that round is not scored, as on ButtonWeavers; it prints `game cancelled` instead of `game over` and counts for neither player, so 200 tied rounds print `game cancelled 0 - 0 - 199`. |
| `playfair N` | Play N games between the players' engines and report wins split by who won initiative. Auxiliary dice are decided as for `playgame`. Cancelled games are left out of the split and counted in the header, `PlayFairGames: N games, C cancelled`; with none cancelled the header is `PlayFairGames: N games`. |
| `debug CATEGORY 0\|1` / `debugply N` | Configure text-protocol diagnostics. |
| `quit` | Stop consuming the current script. |

Unqualified settings change the global Monte Carlo settings, which the `stats`
line reports and every player starts each `game` with. A per-player setting,
or `ai PLAYER montecarlo`, gives only that player its own copy of the global
settings; neither reaches the other player. A setting the player's engine does
not use is an error, as is Monte Carlo `ply 0`. JSONL session metadata reports
each player's `engine` and, for `montecarlo`, its `montecarlo` settings.

Phases are `aux`, `preround`, `reserve`, `initiative`, `chance`, `focus`,
`fight`, and `gameover`. `getaction` is defined for Auxiliary, preround,
reserve, Chance, Focus, and fight; initiative/gameover are state-description
phases rather than direct action requests. In an Auxiliary state, the action is
`aux DIE` to accept the indexed die or `aux -1` to decline. When only one
player supplies an Auxiliary die, BMAIR creates ButtonWeavers' courtesy copy
for the other player before evaluating the choice. Swing sizes the position
gives stand unless an accepted Auxiliary die has a swing size still to choose;
then its owner chooses all its swing sizes again. An accepted Option die never
reopens them, so against given sizes it plays the size the position selected,
or its first-listed size.

Gordo's restriction is applied when the caller sends `special N unique_sizes`;
other button-specific eligibility is the caller's responsibility. BMAIR
rejects a second Auxiliary die for a player in any phase.
Legacy parser errors terminate the process with a nonzero exit status. JSONL
converts those same errors into recoverable `execution_error` responses and
rolls back the request.

A nonzero `report_sims` produces a report only for Monte Carlo fight search;
other engines' fight requests reject it, while other phases retain the setting
without producing an evaluation. The reporting samples use
the same versioned native mechanics and rollout policy as root candidate
evaluation, but a reserved stream keeps them independent from move selection.
The selected action and next decision replay key are therefore identical with
reporting on or off.

Game-state syntax and multiline action examples live in
[`tests/fixtures/`](tests/fixtures/), each with its recorded output in
[`tests/golden/`](tests/golden/). A complete persistent JSONL conversation is
executable at
[`tests/jsonl-fixtures/session.jsonl`](tests/jsonl-fixtures/session.jsonl).

## Gauntlet shards

`bmair gauntlet --shard K/N` writes one JSON object per line instead of a
table, and `bmair gauntlet --merge` reads every part back. The first line of a
part describes the run:

```json
{"type":"shard","format":2,"bmair":"0.29.0","button":"(4) (6) (8) (10) (X)","engine":"montecarlo","field":[{"name":"Avis","recipe":"(4) (4) (10) (12) (X)"}],"seeds":[1,250],"shard":[1,4],"pairs":63}
```

| Field | Meaning |
|---|---|
| `format` | Shard format version, currently `2`. |
| `bmair` | Build version that played the part. Another build may play a seed differently. |
| `button`, `engine` | The recipe under test and the engine spec for both seats. |
| `field` | Opponents in table order, each with `name` and `recipe`. |
| `seeds` | First and last seed of the whole gauntlet, inclusive. |
| `shard` | This part and the number of parts, counting from 1. |
| `pairs` | How many pair lines this part writes. |

Every later line is one seed played against one opponent with the button in
each seat, written as soon as it finishes, so pairs arrive in completion
order:

```json
{"type":"pair","opponent":0,"seed":17,"wins":2,"cancelled":0,"score":1.0,"rounds":[6,2]}
```

`opponent` is a 0-based index into `field`. `wins` counts the button's wins of
the two games, `cancelled` the games cancelled at the 200-round limit, and
`score` is the button's share of the pair, with a cancelled match counting as
half. `rounds` is `[won, lost]` for the button.

Pairs are dealt to parts in turn: the pair at position `p` of opponent `o`
belongs to part `(o * seeds_per_opponent + p) % N + 1`. A merge requires
headers that agree on everything but `shard` and `pairs`, every part exactly
once, and every opponent and seed exactly once, then prints the table a single
run would. Readers ignore unknown fields. A change that would make a merge
misread another build's lines bumps `format`: format 2 added `cancelled`,
which format 1 parts lack.

## Compatibility policy

Released protocol identifiers are immutable contracts. Within `jsonl-v1`, new
methods, optional response fields, capability entries, action variants, and
error codes may be added. Existing field meaning, discriminator meaning, and
required fields will not change. Clients must ignore unknown object fields and
use capability discovery before relying on optional behavior. Removing or
retyping existing behavior requires a new protocol identifier.

Search may intentionally evolve, but its replay partitions are explicitly
versioned. BMAIR no longer reproduces C++ BMAI's search: removing `mode legacy`
is the one deliberate exception to the policy above, marked breaking in the
CHANGELOG. A client sending input written for C++ BMAI should also send the
settings it assumed: `ply 1`, `max_sims 500`, `min_sims 10`, `maxbranch 5000`,
`surrender on`, `endgame 0`, and `fire_overshooting off`.

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
