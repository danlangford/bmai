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

- Stronger QAI rollouts within QAI's time and memory budget, measured with the
  harness. Candidate rules: prefer attacks that leave the opponent's capture
  options smallest, and avoid rerolling a die the keep-threshold says must
  survive. The 0.21.0 sweep found neither depth nor simulation count improves
  on BMAIBagels' settings, so rollout quality is the likeliest limit.
- A `minimax` AI policy (expectiminimax), built in measured steps:
  - Exact reroll odds instead of sampling, starting with a Turbo pre-screen
    that drops dominated sizes before simulating the rest.
  - Exact Chance and Focus decisions.
  - A short search with exact chance nodes, so forced attacks and pressure on
    an opponent's critical die emerge on their own.
  - Leaf scoring from the keep-threshold (two-thirds of the side-total
    difference), critical dice, and capture odds, after the Cheapass Games
    strategy articles.
  - Exact endgame solving for small positions (retrograde analysis).
  - Swing-size ranking from the same threshold math.
- Review the protocol and JSONL schema while they can still change, then
  freeze them for 1.0.
- Start Berserk, Morphing, and Mighty dice at their recipe size each round,
  as ButtonWeavers does. They still keep a size changed in one round for the
  rest of the match.
- Wildcard (`C`) remains deferred until the protocol can carry deck state.
- Pass search a single randomness source, the sequential generator or a
  native replay key, so no search takes a generator it ignores. Offer a
  modern generator (PCG or xoshiro) beside Park-Miller for strength runs.
- Move the Monte Carlo search from `search/` into `engines/montecarlo/`, and
  split the helpers other engines share out of it.
- Finish tuning BMAIBagels' settings. The 0.21.0 sweep changed one setting
  at a time over 300 pairs, so it could only find large effects. Still open:
  `maxbranch` 800 and above (it leaned better), combinations of settings,
  native execution, and larger samples that can resolve a few points.

### Tabled

Tried or considered, and set aside because the strength harness showed no
clear benefit. Each could return if that changes.

- A per-decision `time_limit` for Monte Carlo. A prototype bounded every
  phase, but ply 3 limited to ply 2's time lost clearly, and BMAIBagels
  already retries a lower ply after an hour. The code is at tag
  `prototype/time-limit`.
- BMAIBagels at ply 1 with more simulations. `4000/200/16000` tied ply 2 at a
  quarter of the time, but the bot is not short of time.
- Monte Carlo defaults that depend on `ply`. BMAIBagels sets every value, so
  nothing uses the defaults.
- Exposing the fixed depth decay and cull thresholds as settings, given how
  little the exposed settings moved strength.
- A Quick variant that charges each attack for the opponent's best reply
  capture. It beat Quick in 53% of 2,400 games but played about 10 times
  slower on many-option buttons, too slow for a playout.
- Cheaper Quick changes as Monte Carlo's playout: a rules-aware reroll value,
  a smaller random bonus, charging exposed dice or Fire turned down,
  rewarding dangerous captures, counting a Value reroll once, random opponent
  swing sizes, and Chance and Focus for initiative in simulated rounds. None
  beat the C++ QAI in confirmation runs; STRENGTH.md has the numbers.

## [0.28.0] - 2026-10-08

### Added

- A web site you can host anywhere. Each release includes
  `bmair-VERSION-web-release.zip`, a static site that runs BMAIR in the
  browser: unzip it onto GitHub Pages, Netlify, Cloudflare Pages, S3, or any
  web server. The page takes the same arguments and standard input as the
  command line, offers examples, and shows output as the engine writes it.
  Nothing leaves the browser.
- Each release also includes the WebAssembly engine on its own,
  `bmair-VERSION-webassembly-wasm32-wasip1-release.wasm`, for WASI runtimes
  such as Wasmtime. It is the same file the web site runs.
- `bmair gauntlet "RECIPE" -` reads the opponents from standard input.

### Changed

- Where threads can't start, as in the WebAssembly release, `workers` above 1
  and gauntlet `--threads` run their work in turn instead of crashing. Results
  match native builds; only the speed differs. CI runs every golden fixture through the browser's
  WebAssembly host and compares output and RNG fingerprints with native.

## [0.26.1] - 2026-10-08

### Fixed

- Skills a round adds or removes no longer carry into the next round, since
  ButtonWeavers deals each round's dice from the recipe:
  - A die captured by a Null or Value die kept that skill. Null spread from
    round to round until every die scored zero, every round tied, and
    `playgame` replayed tied rounds forever. Avis against `6 n8 n12 20 X`,
    and the other Null buttons that hung, now finish with every engine.
  - A Warrior die that attacked stayed an ordinary die.
  - A Radioactive die that survived a failed Trip stayed non-Radioactive.
- A game is cancelled when its 200th round ends, whatever that round's
  result, as on ButtonWeavers, so a game that can only tie still ends. Two
  all-Null buttons are one example. `playgame` and `compare` print
  `game cancelled` instead of `game over` and count it for neither player,
  and `playfair` leaves it out. The strength harness and `bmair gauntlet`
  score it as a draw.

## [0.26.0] - 2026-10-08

### Added

- `bmair gauntlet "RECIPE" [OPPONENTS]` plays one button against a field of
  opponents and reports its match wins, win rate with a 95% interval, and
  share of rounds won against each, then overall. Every seed is played from
  both seats with one engine, by default `montecarlo` at its default
  settings. Each match runs native search on one worker, and matches run in
  parallel across every core. Without an opponents file it plays eight
  established ButtonWeavers buttons between 40% and 60%; `--field` prints
  them for editing.
- Gauntlet recipes and strength-harness matchups accept ButtonWeavers
  notation (`p(12)`, `(X=12)`, `(X)?`, `p?(X)`) as well as BMAIR's. All 547
  recipes in BMAIBagels' OPERATION LOOKING GLASS list convert and parse.

## [0.25.0] - 2026-10-07

### Changed

- Searches use about 36% less CPU, measured on 100 of BMAIBagels' logged
  decisions at its settings, with the same moves and reported odds:
  - The endgame solver keys positions by their packed fields instead of
    debug text, which had been a third of all CPU. Its outcomes now sum in a
    different order, so internal values can differ in the last bit; tests
    pin them to the bit.
  - Attacks skip the hooks for Jolt, Rage, Null, Value, Konstant,
    Radioactive, Ornery, and Time and Space when no die on the board has
    the skill.
  - Dice rolls avoid a 128-bit division and two divisions they did not need,
    and dice without Mighty or Weak skip resizing.

## [0.24.0] - 2026-10-06

### Added

- `endgame N`, a Monte Carlo setting that solves the end of a round exactly
  once N or fewer dice remain (default 4). It replays each attack with every
  face of every random draw, so every skill's odds are exact, and plays both
  sides at their best. A position over its budget, with too many rerolls
  (Ornery), or that can repeat (a Trip that keeps failing) is searched as
  before. An exact answer skips the `report_sims` resampling, so clients see
  the exact odds. Game 121248's position, once reported as a certain win,
  solves to a 30.4% chance for the opponent. `BMAIR_TRACE_ENDGAME` logs how
  far Monte Carlo's choice falls short of the solver's.

### Changed

- **Breaking:** `surrender` defaults to `off`. With it on, Monte Carlo
  surrendered rounds it could still win: the solver found 15 such
  surrenders in 951 endgame decisions, with chances up to 10%. Clients
  expecting C++ BMAI's results now also send `surrender on` and `endgame 0`;
  the golden test does.

## [0.23.0] - 2026-10-06

### Changed

- New defaults, so BMAIR plays well with no settings: `mode native`,
  `workers auto`, `fire_overshooting on`, and Monte Carlo `max_sims 4000`,
  `min_sims 200`, and `maxbranch 16000` at ply 1. The strength harness found
  this budget ties ply 2 on classic buttons and beats it on Turbo, Fire, and
  Poison buttons in a quarter of the time. `workers 1` suits machines short
  of memory, and ButtonWeavers players who have not turned on Fire
  overshooting should send `fire_overshooting off`. Clients expecting C++
  BMAI's results must now send `mode legacy`, `ply 1`, `max_sims 500`,
  `min_sims 10`, `maxbranch 5000`, and `fire_overshooting off`.
- Quick, Maximize, and Random use a fixed Fire candidate limit of 500, so
  Monte Carlo's search settings never change how they play.
- The golden test sends C++ BMAI's old defaults before each fixture, so the
  golden decisions and RNG fingerprints are unchanged.

### Added

- `tests/strength/options.sh` and `tests/strength/matchups-options.txt`: the
  many-option matchups (Turbo, Fire, Poison, Value, Trip) behind the new
  budget.

## [0.22.0] - 2026-10-05

### Fixed

- A Value die scores its current value after it rerolls, and after Focus
  turns it down. BMAIR had kept C++'s pre-reroll score, so simulations never
  credited an opponent's high Value reroll: in game 121248 BMAIBagels
  claimed 100.0% from a position the opponent wins about 17.5% of the time.
  Rerolls after a Trip Morph and of Radioactive products are rescored too.
  The `Value1` and `Value2` golden outputs changed.

## [0.21.0] - 2026-10-05

### Changed

- Quick and every playout pick their swing settings without listing every
  combination, which made playouts with many swing dice slow. The setting
  chosen is unchanged.

### Added

- `tests/strength/sweep.sh` and `tests/strength/ply1-width.sh` play
  BMAIBagels' settings against ply 3, wider ply 1, and one-setting changes.
  STRENGTH.md records the results: no change beat the current settings.

## [0.20.0] - 2026-10-03

### Added

- Named AI engines behind one `Engine` trait: `random`, `maximize`, `quick`
  (the C++ Quick AI), and `montecarlo` (BMAI's simulation search, the
  default). `ai PLAYER NAME` selects one, and capabilities `engines` lists
  each with the settings it takes.
- `cull [PLAYER] on|off` makes Monte Carlo culling a setting instead of a
  separate AI type.
- `playout [PLAYER] quick|maximize|random` chooses the engine that plays
  Monte Carlo's simulated games, replacing the C++ rollout policies; the
  default, `quick`, keeps every search unchanged.
- A strength harness: `strength::play_pairing` plays two engine
  configurations over the same seeds from both seats and reports the paired
  win rate with a 95% interval and milliseconds per decision.
  `cargo run --release --example ladder` runs a round-robin over
  `tests/strength/matchups.txt`. Seeds are mixed before seeding Park-Miller,
  whose consecutive seeds roll nearly the same opening dice.

### Changed

- The numeric AI types `0`, `1`, and `2` are gone; use `montecarlo` with
  `cull off`, `quick`, and `montecarlo`.
- `playfair GAMES` plays the engines the players already have. The C++ modes
  and probability argument are gone. Modes 0, 1, and 3 are `random`,
  `maximize`, and `montecarlo` with `cull off`; mode 2's mixed
  Maximize-or-Random playout has no equivalent.
- CI checks the slow golden fixtures on every pull request instead of only
  when building a release. The lint and slow-test jobs cache their Rust builds.
- Quick, Random, and Maximize move logic lives in their engine files, and
  Monte Carlo playouts call those engines directly.
- Native-mode traces of swing, Chance, and Focus searches print the session
  seed instead of a placeholder generator's.
- Each player owns its engine. A per-player setting, or `ai PLAYER
  montecarlo`, copies the global settings for that player only, instead of
  changing an AI object C++ shares between players and games; `game` returns
  both players to the global settings.
- An engine rejects settings it does not use, so `ply` on `quick` is an
  error rather than silently ignored, and Monte Carlo `ply 0` is an error.
- JSONL session metadata reports each player's `engine` and, for
  `montecarlo`, its `montecarlo` settings, replacing `ai_type`, `policy`, and
  `culls_moves`. The session also reports the global `cull`.

## [0.19.0] - 2026-10-03

### Changed

- Functions, methods, and fields use Rust naming, completing the 0.17.0
  type rename. CamelCase functions are snake_case (`ApplyAttack` is
  `apply_attack`), `Get` prefixes are gone (`GetValueTotal` is `value_total`),
  and fields drop `m_` (`m_die` is `dice`, `m_player` is `players`). The crate
  no longer allows `non_snake_case`. Library callers must update; protocol
  output is unchanged.
- Some names now say what they do instead of mirroring C++:
  - `Die::m_value_total` is `value`, since `value_total()` is the method.
  - `Rng::GetRandMax`, `SRand`, and `GetFRand` are `rand_below`, `reseed`, and
    `rand_f32`.
  - `Parser::GetAction`, which runs the search, is `send_action`, and
    `Bmai3::CullMoves` is `cull`.
  - `AvailableDice` is `available_dice_count`, `m_notset` is `not_set`,
    `ProbabilityWin` is `win_probability`, and the `Move::attack`
    constructor is `Move::new_attack`.
- `BMAIR_TRACE_AI` prints moves with the new field names.

## [0.18.0] - 2026-10-03

### Changed

- Search is about 1.4–1.8× faster with identical results; see BENCHMARKS.md.
  All golden RNG fingerprints are unchanged.
  - Rollouts reuse one scratch game per thread instead of cloning the
    dice for every step.
  - Die index sets iterate only their members.
  - Dice ordering caches each die's sort key and skips dice already in
    order.
  - Captured dice rotate into place instead of being removed and
    reinserted.

- Dependabot pull requests skip the version and changelog check, and a merge
  without a version bump no longer fails the release workflow; it waits for
  the next release.
- Tests:
  - Scenario tests now read like game positions. Expected dice may list
    skills in any order, as input dice do.
  - Added `passes()`, an `initiative_scenario()` for Chance and Focus, and a
    `roll()` scenario for die-size sampling.
  - Hand-built dice and moves in the core, parity, Jolt, Doppelganger,
    transformation, and Boom/Mad tests now use these scenarios.
  - Boom, Mad, and Mood tests are in separate files. TESTING.md says which
    file a test of an interaction between skills belongs in.

## [0.17.0] - 2026-10-03

### Added

- Boom (`b`): the Boom die leaves play unscored and rerolls one target, which
  stays in play. Boom may target Stealth dice. A Jolt Boom die grants an
  extra turn; Time and Space and Radioactive never trigger, and a Null or
  Value Boom die converts nothing.
- Mad (`&`) swing dice: like Mood, but every reroll picks an even size in the
  swing range. Radioactive decay removes Mad.
- Advertised the `boom` attack type and the `b` and `&` tokens in capabilities.

### Changed

- Rust types now use Rust naming (`Game`, `Move`, `Phase`, `Attack`, and
  so on) instead of the C++ `BMC_`, `BME_`, and `BMD_` prefixes.

### Fixed

- Checked against the running ButtonWeavers engine:
  - Mood dice resize only to standard die sizes in their swing range.
  - A Mood Twin die picks one size for both halves.
  - Konstant Mood dice keep their size and value.
  - Mood dice also resize when tripped and on Chance rerolls.
  - Konstant Mighty and Weak dice keep their size on every reroll,
    including Ornery, Trip-target, and Chance rerolls. Doppelganger copies
    still resize.
  - Warrior dice ignore Ornery rerolls.

## [0.16.0] - 2026-10-02

### Added

- Added the `special PLAYER [ID...]` command for button
  specials, which the wire format cannot express per die:
  - `unique_swing` (Guillermo, Oregon): different swing types take different
    sizes.
  - `unique_sizes` (Gordo): no two dice share a size, and an Auxiliary swing
    die is declined.
  - `no_skill_attacks` (Largo, The Flying Squirrel).
  - `skill_immune` (The Japanese Beetle).
  - `no_initiative` (Giant): ranked below every other button for initiative.
- Advertised the specials, with the buttons that use them, in capabilities
  `button_specials`.
- Reported each player's specials in JSONL session metadata.

### Fixed

- Trip legality now follows ButtonWeavers instead of C++: a Trip die may Trip
  any die it can roll at least the minimum of, including Twin dice, and
  Konstant and Maximum targets raise that bar. A Turbo Trip is offered when
  any Turbo size reaches the target, and only sizes that do are offered.
  Mood Twin dice reach one subdie's swing size, as in ButtonWeavers.
- An infinite `turbo_accuracy` now considers every Turbo size instead of
  hanging the search.
- Morphing now applies to single-target Berserk and Speed attacks.
- Checked against the running ButtonWeavers engine:
  - A Doppelganger copy of a Mighty or Weak die now resizes on the attack
    reroll, even if Konstant.
  - A Radioactive Doppelganger's first copy keeps the captured die's value
    without rerolling.
  - A Radioactive Morphing attacker, including a Trip die, decays into two
    full-size morphs.
- A Trip die morphs only after a successful Trip: it rolls at its own size
  first, then rerolls at the captured die's size. A failed Trip never morphs.

## [0.15.0] - 2026-10-02

### Added

- Implemented the complete ButtonWeavers Radioactive (`%`) rule. In any attack
  with exactly one attacker and one target, by any attack type, the attacker
  decays into two near-equal dice when either die is Radioactive. Previously
  only Radioactive Doppelganger Power attacks decayed.
- Decay products lose Radioactive, Turbo, Mood, Jolt, and Time and Space, roll
  fresh values, and resize for Mighty or Weak unless Konstant. A target that
  survives a failed Trip loses Radioactive.
- Applied ButtonWeavers' ordering: Berserk and Morphing transform the attacker
  before it decays, a Radioactive Doppelganger decays before both products
  copy the target, and a Doppelganger copy of a Radioactive target decays.
- Advertised Radioactive as implemented; `parsing_only_skills` is now empty.
- Added a scenario for every Radioactive interaction on the ButtonWeavers
  skills page, plus reproductions of ButtonWeavers responder-test logs.

### Changed

- Replaced the reference-binary differential tests with golden outputs in
  `tests/golden/`. Each records a fixture's normalized output and RNG
  fingerprint, so no C++ or previous-release binary is needed. Most fixtures
  run in every `cargo test`; the longest searches run in CI for releases.
- Moves whose attacker will decay no longer expand into Turbo sizes, since the
  decaying die loses Turbo before its reroll. Trip keeps its sizes, because
  ButtonWeavers rolls the Trip at the chosen size before the decay.
- A decay that would exceed the 20-die pool is skipped rather than panicking.

### Fixed

- A Null or Value attacker that transforms during the attack, such as a
  Doppelganger, still makes its captured die Null or Value, as ButtonWeavers
  runs those effects from the attacker's original skills.
- Twin decay products now take ButtonWeavers' subdie order: the first product
  keeps the rounded-up first half and the rounded-down second half.

## [0.14.0] - 2026-10-01

### Added

- Implemented the ButtonWeavers Rush (`#`) skill. A die makes a `rush` attack
  by capturing exactly two dice whose values sum to its value; the attacker or
  at least one target must be a Rush die, so any die may Rush a Rush die.
- Applied ButtonWeavers' shared Speed-attack restrictions: Stealth and Warrior
  dice cannot make or receive Rush attacks, dizzy dice cannot Rush, and Fire
  cannot assist. Skills that forbid only Power or Skill attacks still Rush.
- Advertised Rush and the `rush` attack type through machine-readable
  capabilities and emitted `rush` in legacy and typed actions.
- Added Rush rule and interaction scenarios for Speed, Stealth, Warrior,
  Focus, Insult, Konstant, Stinger, Fire, Twin, Berserk, Morphing,
  Doppelganger, Radioactive, Jolt, Time and Space, Rage, Null, Value, Poison,
  Mighty, Weak, Mood, Ornery, Maximum, Turbo, Reserve, comment parsing, and
  legacy/native search.
- A Speed die that is also a Rush die offers its two-target captures only as
  Speed attacks. Both attack types have identical legality and resolution, so
  this avoids searching duplicate candidates. Inputs without Rush dice keep
  their candidate order, RNG consumption, and output, and skip Rush pair
  enumeration entirely.

### Fixed

- Stopped search from panicking when `max_sims` is below `min_sims` (for
  example `max_sims 5` with the default minimum of 10) in both the legacy and
  JSONL protocols. Simulation counts now follow C++ `ComputeNumberSims`, which
  checks the minimum first, and a seeded fixture matches the C++ reference.
- Matched C++ per-player search settings (`ply`, `max_sims`, `min_sims`,
  `maxbranch` with a player). C++ players point at shared AI objects, so a
  per-player setting changes the global AI after `game`, or the shared `ai`
  type object, which keeps default settings of its own and persists across
  games. BMAIR previously gave each player a private copy, so it diverged from
  C++ whenever a script combined per-player settings with `game` or `ai`.
  Seven scenarios now match the C++ reference, and JSONL session metadata
  reports the shared objects. Scripts that use only global settings, including
  BMAIBagels, are unchanged.

## [0.13.0] - 2026-09-30

### Added

- Added opt-in `report_sims N` selected-move probability reporting for native
  BMAI fight search. Normal bounded search still chooses the move; the chosen
  move is then evaluated with exactly `N` fresh samples on a reserved,
  deterministic stream.
- Added a typed JSONL `evaluation` result containing the evaluated player,
  probability, simulation count, and whether the estimate came from move
  selection or selected-move resampling.
- Added reconstructed regressions for ButtonWeavers games 120810 and 120813,
  including their exact 50/50 and 70/30 bounded-roll endgames.

### Changed

- Preserved the historical legacy `best move` diagnostic and emit a separate
  `selected move report` diagnostic only when resampling is requested.

## [0.12.0] - 2026-09-20

### Changed

- Organized the Rust source by game, search, and protocol responsibilities
  while preserving the public API, protocol output, search order, and RNG use.
- Isolated each search phase, each protocol adapter, and test-only scenario
  support into focused modules.
- Added an architecture guide documenting dependency direction and where new
  rules, search behavior, protocols, and tests belong.

## [0.11.0] - 2026-09-18

### Added

- Added the default-off `fire_overshooting on|off` client option. When enabled,
  BMAIR may spend Fire on a Power attack that was already legal, allowing the
  search to value both the stronger attacker and the safer turned-down Fire
  dice.
- Added paired mechanics, parser, and search regressions for disabled and
  enabled Fire overshooting.

## [0.10.1] - 2026-09-17

### Fixed

- Prevented Warrior dice from making a Skill attack unless at least one
  non-Warrior die participates, matching ButtonWeavers validation.
- Returned locked preround Swing and Option selections directly instead of
  attempting to apply them again to already-rolled dice.

## [0.10.0] - 2026-09-16

### Added

- Implemented the ButtonWeavers Fire (`F`) skill for Power and Skill attacks,
  including required assistance, multiple Fire dice, attacker maxima, and
  persistent Fire-die turndowns.
- Added exact Fire adjustments to search moves so Konstant attackers retain
  their fired-up values while ordinary attackers reroll normally.
- Added `fire DIE VALUE` lines to assisted legacy actions and an optional
  `fire` array to typed JSON attack actions.
- Added readable mechanics scenarios for Fire with Stinger, Konstant, Mighty,
  Weak, Rage, Jolt, Time and Space, Queer, Twin, and unsupported attack types.
- Bound assisted-candidate construction to the search budget before allocating
  every possible Fire distribution on large buttons.

### Changed

- Advertise Fire as implemented through machine-readable capabilities.

### Known limitations

- Wildcard dice remain unsupported by BMAIR. ButtonWeavers' unresolved
  Wildcard-specific Fire behavior is therefore outside this release.
- An odd Queer die cannot be fired to an even value to unlock a Power attack,
  matching ButtonWeavers' current pre-assistance attack-type eligibility.
- BMAIR follows ButtonWeavers' default `fire_overshooting = false` behavior.
  Searching optional Fire adjustments for otherwise-legal attacks requires a
  future player-preference input.
- Fire-assisted attacks involving an attacking Turbo die currently use its
  displayed size; alternate Turbo sizes are not searched for that attack.

## [0.9.0] - 2026-09-16

### Added

- Implemented the ButtonWeavers Auxiliary (`+`) lifecycle and `aux` phase.
- Added mutual accept/decline resolution, courtesy Auxiliary copies when only
  one button supplies the die, and legacy `aux DIE`/`aux -1` actions.
- Added deterministic native Auxiliary evaluation across worker counts and an
  `auxiliary` typed action.
- Added readable parser and mechanics regressions derived from the
  ButtonWeavers Auxiliary engine tests.

### Changed

- Advertise Auxiliary as implemented rather than parsing-only.

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
  endgames by stratifying their one- and two-die reroll distributions.
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

[Unreleased]: https://github.com/danlangford/bmai/compare/bmair-v0.28.0...HEAD
[0.28.0]: https://github.com/danlangford/bmai/compare/bmair-v0.27.0...bmair-v0.28.0
[0.26.1]: https://github.com/danlangford/bmai/compare/bmair-v0.26.0...bmair-v0.26.1
[0.26.0]: https://github.com/danlangford/bmai/compare/bmair-v0.25.0...bmair-v0.26.0
[0.25.0]: https://github.com/danlangford/bmai/compare/bmair-v0.24.0...bmair-v0.25.0
[0.24.0]: https://github.com/danlangford/bmai/compare/bmair-v0.23.0...bmair-v0.24.0
[0.23.0]: https://github.com/danlangford/bmai/compare/bmair-v0.22.0...bmair-v0.23.0
[0.22.0]: https://github.com/danlangford/bmai/compare/bmair-v0.21.0...bmair-v0.22.0
[0.21.0]: https://github.com/danlangford/bmai/compare/bmair-v0.20.0...bmair-v0.21.0
[0.20.0]: https://github.com/danlangford/bmai/compare/bmair-v0.19.0...bmair-v0.20.0
[0.19.0]: https://github.com/danlangford/bmai/compare/bmair-v0.18.0...bmair-v0.19.0
[0.18.0]: https://github.com/danlangford/bmai/compare/bmair-v0.17.0...bmair-v0.18.0
[0.17.0]: https://github.com/danlangford/bmai/compare/bmair-v0.16.0...bmair-v0.17.0
[0.16.0]: https://github.com/danlangford/bmai/compare/bmair-v0.15.0...bmair-v0.16.0
[0.15.0]: https://github.com/danlangford/bmai/compare/bmair-v0.14.0...bmair-v0.15.0
[0.14.0]: https://github.com/danlangford/bmai/compare/bmair-v0.13.0...bmair-v0.14.0
[0.13.0]: https://github.com/danlangford/bmai/compare/bmair-v0.12.0...bmair-v0.13.0
[0.12.0]: https://github.com/danlangford/bmai/compare/bmair-v0.11.0...bmair-v0.12.0
[0.11.0]: https://github.com/danlangford/bmai/compare/bmair-v0.10.1...bmair-v0.11.0
[0.10.1]: https://github.com/danlangford/bmai/compare/bmair-v0.10.0...bmair-v0.10.1
[0.10.0]: https://github.com/danlangford/bmai/compare/bmair-v0.9.0...bmair-v0.10.0
[0.9.0]: https://github.com/danlangford/bmai/compare/bmair-v0.8.2...bmair-v0.9.0
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
