# C++ → Rust full-parity ledger

Authority: C++ `main` at `1fcb826`, Konstant PR #82 at `4813530`, and the
contract in `AGENTS.md`. PR #82 is treated as the reference for its signed
Konstant behavior until it lands upstream.

## Regression oracle policy

The adopted Konstant reference at `4813530` remains the source-provenance
oracle even though that patch has not been merged upstream. Its implementation
and regression suite were deliberately accepted as part of BMAIR's mechanics
contract.

The published `bmair-v0.4.1` legacy executable is the routine regression oracle
for later releases: its complete C++ and adopted-Konstant parity was established
before post-C++ mechanics such as Jolt were added. Current legacy builds should
match it on every historical fixture. Re-run the C++ reference gates after
changes to mechanics, parsing, RNG consumption, candidate generation, or search
control flow, and periodically as a provenance audit; the Rust baseline does
not replace that historical evidence chain.

## Completion gates

- [x] All current `*in*.txt` fixtures have materially identical outputs.
  Evidence: release differential passed on 2026-08-23; PR #82 differentials
  and the GitHub artifact comparison passed on 2026-08-27/28; seeded internal
  traces also matched the large preround/reserve searches.
- [x] Every meaningful C++ test is mapped to an equivalent default Rust test;
  demo/developer-only exclusions are explicitly justified below.
- [x] Every C++ parser command/API path is implemented or explicitly mapped.
- [x] Feature/property/search matrix is complete with code and test evidence.
- [x] Differential cases cover meaningful mechanics absent from shipped inputs.
- [x] Final release differential, unit tests, clippy, and source audit pass on
  the final commit.
- [x] Rust-native structural parity is complete: bounded state, simulation
  reuse, direct candidate enumeration, compact moves, cached-state lifecycle,
  and search control flow correspond to C++ or have an explicit justification.

## Upstream test mapping

Source files: `test/LegacyFunctions.cpp`, `PlayerTest.cpp`, `ParserTest.cpp`,
`SkillTest.cpp`, `BMAI3Tests.cpp`, and `DemoTest.cpp`.

### Konstant PR #82 additions

- [x] All PR #82 signed-Konstant generation cases (including the four
  parameterized signed targets) and the ten-Konstant upper bound map to
  `model::tests::pr82_signed_konstant_skill_attack_matrix` and
  `pr82_variable_skill_stack_disables_legacy_value_pruning`.
- [x] Konstant/Stinger/Warrior range, sign, gap, full-value, later-target, and
  unused-pool cases are individually named in the table-driven Rust test, so
  failures report the corresponding upstream GoogleTest name.
- [x] Trip and Chance Mighty/Weak/Maximum sequencing maps to
  `pr82_konstant_trip_target_retains_value_and_changes_sides_once`,
  `pr82_trip_target_before_roll_effect_triggers_once`, and
  `pr82_chance_effects_run_once_while_konstant_retains_value`.
- [x] Participating/nonparticipating Ornery, Konstant Mighty/Weak, Mood, and
  pass behavior maps to the four `simulation::tests::pr82_*ornery*` tests;
  `OrdinarySideChangeInvalidatesValue` maps to
  `pr82_ordinary_side_change_invalidates_value`.
- [x] Konstant Time-and-Space, Morphing, Berserk, Skill, Trip, and Warrior
  lifecycle cases map to `pr82_konstant_time_and_space_never_grants_extra_turn`,
  `pr82_konstant_attack_side_changes_preserve_value`, and the existing focused
  Konstant Skill/Trip/Warrior tests.
- [x] `bug105372_in.txt` is copied into `tests/fixtures` and therefore runs in
  the material-output and exhaustive RNG differential gates.

- [x] Inventory all 48 registered upstream test cases: 42 functional cases,
  two debug assertion contracts, three demo/framework cases, and one disabled
  developer setup case. PR #82 adds 60 registered executions (56 named Skill
  tests with one parameterized over four targets, plus `bug105372`), bringing
  its reference suite to 108.
- [x] `LegacyMembers.TestRNG` -> `rng::tests::cpp_legacy_rng_distribution`.
- [x] `PlayerTests.CopyConstructor` ->
  `model::tests::cpp_player_copy_constructor_is_independent`.
- [x] `ParserTests.ParseString` ->
  `parser::tests::cpp_parser_multiline_fight_string`.
- [x] NoSkill, MultiDieSkillAttack, SingleDieSkillAttack,
  KonstantSingleDieSkillAttack, StealthSingleDieSkillAttack, and
  StealthMultiDieSkillAttack -> `cpp_basic_power_and_skill_attack_generation`
  plus `copied_cpp_skill_restrictions_match_konstant_and_stealth_cases`.
- [x] MaximumSkill -> `cpp_maximum_die_always_rolls_its_maximum` and
  `cpp_speed_generation_and_property_score_combinations`.
- [x] Konstant Trip, Chance, Skill, and Warrior tests -> the four
  `simulation::tests::cpp_konstant_*`/`copied_cpp_konstant_*` tests.
- [x] Insult and all nine Stealth tests ->
  `model::tests::cpp_insult_and_stealth_restrictions` plus basic generation.
- [x] Null, Value, NullValue, Poison, PoisonValue, and PoisonNull ->
  `score_matches_cpp_property_branches` and
  `cpp_speed_generation_and_property_score_combinations`.
- [x] SpeedSkill -> `cpp_speed_generation_and_property_score_combinations`.
- [x] MorphingSkill, MorphingTwinSkill, MorphingSpeedSkill ->
  `cpp_morphing_copies_single_and_twin_target_sizes` and
  `copied_cpp_morphing_speed_attack_does_not_morph`.
- [x] All ten `BMAIActionTests` parameter cases -> parser fixture tests.
- [x] Debug-only RollRequiresNotSetState and SwingSetRequiresNotSetState ->
  `cpp_roll_requires_notset_state` and `cpp_swing_set_requires_notset_state`,
  with assertions enforced by the production Rust lifecycle operations.
- [x] DemoTest's three arithmetic/demo cases are excluded: they test GoogleTest
  itself and a test-local factorial, not BMAI production behavior.
- [x] `LegacyMembers.SetupDevGame` is excluded: upstream marks the fixture
  disabled and it is developer scaffolding without assertions.

## Parser and externally reachable API

- [x] `game [wins]`, `playgame`, `getaction`, `seed`, `surrender`, `quit`.
- [x] Rust-only `mode legacy|parity|native` and
  `rng legacy|park-miller|bmai-park-miller-16807-v1` are explicit extensions.
  They default to legacy behavior and therefore add no output or state change
  to C++ protocol inputs. `mode native` is currently a named evolution seam,
  not a behavior change.
- [x] Whole-line `#` comments between top-level commands are an explicit Rust
  parser extension. Batched-file and incremental-stdin tests prove identical
  behavior after comments are removed. Inline comments and comments within a
  structured `game` block remain invalid; the C++ parser accepts no comments.
- [x] Jolt (`J`) is an explicit post-C++ mechanics extension based on the
  ButtonWeavers rules engine. The C++ reference predates Jolt and has no Jolt
  parser or behavior to map. Existing C++ inputs remain unchanged; focused Rust
  tests cover attacking, successful and unsuccessful capture, unsuccessful
  Trip, Konstant, multiple-Jolt, and Time-and-Space post-roll interactions. The
  attack lifecycle was audited against ButtonWeavers `master` at
  `a2d2a1fac12bcffd3453bb0dfe1282b733d23a5b`, including `BMAttack`,
  `BMSkillTrip`, `BMSkillJolt`, `BMSkillTimeAndSpace`, and `BMSkillKonstant`;
  the seeded Trip and Konstant interaction tests also assert that their attacks
  are produced by BMAIR's legal-attack generator.
- [x] Global `ply`, `max_sims`, `min_sims`, `maxbranch`, `turbo_accuracy`.
- [x] `compare` (the upstream implementation is currently identical to
  `playgame`, despite its stale OLD-AI comment).
- [x] `playfair` modes 0 random, 1 maximizer, 2 legacy BMAI with
  Maximize-or-Random rollout policy, and 3 legacy BMAI with QAI, including
  initiative-split reporting.
- [x] `ai <player> <type>`: type 0 legacy fixed-simulation BMAI, type 1 QAI,
  and type 2 batched/culling BMAI3 are wired for parser actions and games.
- [x] Per-player `ply`, `max_sims`, `min_sims`, and `maxbranch` parsing/state.
- [x] `debug` category validation/state and `debugply` parsing/state, including
  C++'s exact uppercase category and boolean-setting behavior.
- [x] Error messages, invalid inputs, phase restrictions, and exit behavior:
  focused executable differential covers unknown commands, invalid AI/debug,
  invalid phase/player setup, invalid `getaction`, and simulation phase errors.
- [x] Public Rust types/functions are mapped against reachable C++ game/AI
  behavior below. Rust is behavior-compatible, not C++ ABI/source-compatible.

### Public API mapping

- `BMC_DieData`/`BMC_Die` state and accessors map to public `BMC_Die` fields,
  `HasProperty`, `GetSidesMax`, `GetValueTotal`, `IsAvailable`, `GetScore`,
  `Roll`, `OnSwingSet`, and `OnDizzyRecovered`; remaining event methods are
  invoked by the game engine so their ordering cannot be bypassed accidentally.
- `BMC_Player` getters/events map to public fields plus `OptimizeDice`; dice
  setup is direct construction/parsing instead of the inert C++ `BMC_Man`
  container. `BMC_Man` has no independently configurable public behavior in
  C++ and therefore needs no Rust runtime type.
- `BMC_Game::GenerateValidAttacks`, `SimulateAttack`, `CheckInitiative`, and
  `RecoverDizzyDice` retain corresponding Rust methods. Full round/game play,
  preround, reserve, Chance, and Focus are exposed through `BMC_Parser` and
  `PlayGames`, with `BMC_Game` serving as the explicit game template instead
  of C++ `PlayGame(BMC_Man*, BMC_Man*)` setup pointers.
- `BMC_AI`, Maximizer, QAI, legacy BMAI, and BMAI3 virtual dispatch maps to
  `BMC_AI_POLICY` plus `BMC_BMAI3`; its search settings, simulation-count
  computation, evaluator, rollout selection, and last probability are public.
- `BMC_Move`'s tagged union maps to `BMC_Move` for fight actions and internal
  typed Swing/Chance/Focus/Reserve moves. Protocol clients observe those move
  types through the same parser action output rather than union field access.
- `BMC_RNG` maps directly to public `SRand`, `GetRand`, `GetRandMax`, and
  `GetFRand`. Logger and stats presentation are transport diagnostics; parser
  debug settings and stable stats/action output are externally preserved.

## Feature and search matrix

Status meanings: **covered** has implementation and direct test evidence;
**source-audited** has a mapped implementation but still needs the differential
case named in the final column.

| C++ behavior | Rust implementation | Evidence | Status |
|---|---|---|---|
| `BMC_Parser::ParseDie*` | `parser::ParseDie`, `ParseDieDefinedSides`, `parse_side`, `prefix_property` | defined Twin Swing parser matrix, forced-win search scenario in four mode/worker combinations, `parity_defined_twin_swing_in.txt`, every shipped fixture | covered |
| `BMC_Die::OnSwingSet`, `SetOption`, `Roll`, `Reset`; `BMC_Player::Reset`, `RollDice`, `OptimizeDice` | `ApplySwingMove`, `RollDie`, match reset, `BMC_Player::OptimizeDice` | both lifecycle panic ports, Turbo/Unique tests, exact seeded fixture traces | covered |
| `BMC_Die::GetScore` ordinary/Poison/Value/Null/Warrior | `BMC_Die::GetScore` | score branch tests and all upstream skill score ports | covered |
| `BMC_Game::GenerateValidAttacks`, `ValidAttack` for Power/Skill/Speed/Trip/Shadow/Berserk | `GenerateValidAttacks`, `GenerateValidAttacksInCppOrder` | upstream attack/Stealth/Insult tests | covered |
| Konstant, Stealth, Warrior, Stinger, Unskilled, Queer attack restrictions | `CanDoAttack`, `CanBeAttacked`, direct stack enumeration plus `SkillStackCanHit` signed intervals | PR #82's complete signed-Konstant/Stinger/Warrior matrix, Stealth+Insult regressions, differentials | covered |
| `BMC_Die::OnApplyAttackPlayer` Berserk, Mighty, Weak, Morphing, Turbo, Warrior and Ornery scheduling | `ApplyAttackPlayerEffects`, cached attack-phase available boundary | PR #82 participating/nonparticipating Ornery, Morphing/Twin/Speed, Turbo, Warrior tests | covered |
| `OnBeforeRollInGame`, nature rerolls, Mood, and Trip's single before-roll pass | `ApplyBeforeRollEffects`, `ApplyMood`, `ApplyAttackerNatureRoll`, `RollScheduledDie` | PR #82 Trip/Chance/Ornery/Konstant effect and pass tests plus seeded differentials | covered |
| captured Null/Value mutation and scoring | target mutation in `ApplyAttackForPlayers` before captured score | property score tests and combined seeded differential | covered |
| Time and Space odd-roll extra turn and dizzy recovery | `ApplyAttackForPlayers` extra-turn result, `RecoverDizzyDice` | combined seeded differential and exact QAI/RNG trace | covered |
| Jolt attacker consumption and attacker/captured-defender extra turns | Jolt snapshots and attacker-property removal in `ApplyAttackForPlayers` | focused Jolt, Trip, Konstant, multiple-die, and Time-and-Space tests | covered Rust extension |
| ButtonWeavers Doppelganger Power-capture transformation and round reset | target recipe replacement in `ApplyAttackPlayerEffects`, Radioactive decay expansion, original-recipe restoration in `RestoreDiceForNewRound` | focused ordinary/Skill/Twin/Swing, Jolt, Time-and-Space/Konstant, Mighty/Turbo, Rage, Radioactive, and round-lifecycle tests | covered Rust extension |
| ButtonWeavers Rage initiative, participation, replacement, and round reset | Rage initiative filtering, attacker snapshots, bounded replacement creation, and `RestoreDiceForNewRound` | focused Rage core rules plus Doppelganger, Jolt, Time-and-Space, Konstant, scoring, reroll, multi-target, and capacity scenarios | covered Rust extension |
| `CheckInitiative`, Chance chain, Focus values, dizzy state | `CheckInitiative`, `ApplyChanceMove`, `ApplyFocusMove`, initiative evaluators | Konstant Chance, C++ player-index asymmetry regression, parser initiative tests, and chained seeded differential | covered |
| simultaneous preround evaluation, option/swing Cartesian product, `UNIQUE` | `GenerateSwingMoves`, `EvaluateSwingMove`, `ApplySwingMove` | exact bug11/preround traces, Unique unit test | covered |
| reserve activation and BMAI/BMAI3 evaluation | `ApplyUseReserve`, `SelectBMAIReserveAction`, post-round dispatch in `PlayMatchWithPolicies` | exact bug16 candidate/simulation/RNG trace plus `complete_native_match_uses_reserve_after_a_round_loss` | covered |
| base random AI, Maximizer, QAI, legacy BMAI, BMAI3 | policy dispatch, `SelectRandomAction`, `SelectMaximizeAction`, `SelectQAIAction`, fixed/culling evaluators | seeded `ai` and all four `playfair` mode comparisons | covered |
| max ply, QAI transition, BMAI3 batches/culling/Trip threshold, surrender | `EvaluateMove`, `PlayFightQAI`, `BMC_BMAI3::EvaluateMoves`/`CullMoves` | exact ply-2 and full bug16 traces, evaluator tests | covered |
| round/match standings including ties, loser swing reset, initiative fairness matrix | `PlayRoundWithPolicies`, `PlayMatchWithPolicies`, `PlayGames`, `PlayFairGames` | bmsim fixture, four playfair mode comparisons, `tied_round_has_no_loser`, and complete-match reserve regression | covered |
| `BMC_RNG` seed expansion, integer/float output, consumption order | `BMC_RNG` dispatching `LEGACY_PARK_MILLER_V1`; RNG passed through all stochastic operations | version/name/continuity tests, exact sequence/distribution tests, and multi-million-event fixture traces | covered |

Parsing-only parity is intentional for `AUXILIARY` and `RADIOACTIVE`:
upstream C++ only assigns their property bits in `BMC_Parser::ParseDie` and
implements no game behavior. Doppelganger is an intentional post-C++ extension
based on ButtonWeavers engine source at `a2d2a1fac12bcffd3453bb0dfe1282b733d23a5b`.
The Radioactive decay path required by its documented Doppelganger interaction
is implemented, including decay-product replacement and round restoration;
unrelated Radioactive mechanics remain parsing-only. Rage is also an intentional
post-C++ extension based on the ButtonWeavers engine at the same pinned source
revision and its live skill contract.
`UNSKILLED` is marked TODO upstream but both engines enforce its existing
no-Skill-attack behavior. Rust accepts the legacy C++ maximum of ten input dice
per player and uses a compact 20-bit in-round index space. Twenty slots cover
both every Radioactive+Doppelganger distribution of the original two-player
pool and one round-local Rage replacement for every original die. Both the
input and transformed limits fail explicitly rather than silently dropping
dice or skill behavior.

Mechanics and search scenarios in `src/simulation/scenario.rs` are test-only
adapters over the production parser, C++-ordered legality enumeration, attack
resolution, RNG, search, and round restoration used by the executable. The
adapters do not provide alternate rules or search implementations. Canonical
die-recipe state assertions and protocol-level win-percentage ranges keep the
coverage reviewable while preserving existing parity evidence and production
control flow.

## Defined Twin Swing forced-win regression

The 2026-09-02 BMAIBagels incident exposed an uncovered parser branch: C++
`ParseDieSides` applies a shared `-N` definition to every Swing half, while
Rust applied it only to the first half. Rust therefore read `(T,T)-2:2` as
`(2,0):2`, halving its score and capture value before search began.

- `defined_swing_size_applies_to_every_swing_half_of_a_twin` covers repeated,
  distinct, and fixed/Swing Twin combinations, with Turbo and Mood appearing
  on either side of the shared definition.
- `zero_does_not_lock_a_swing_definition` preserves C++'s `sides > 0` lock
  condition.
- `forced_win_is_reported_as_certain_in_legacy_and_native_search` drives the
  production parser and search through legacy with and without a `workers`
  command, plus native with its default and four workers. Every form must
  report player 0 at exactly 100%.
- `parity_defined_twin_swing_in.txt` keeps the original legacy wire input in
  every material-output and RNG-fingerprint differential run.

### Doppelganger interaction coverage

The three distinct Doppelganger interaction notes in ButtonWeavers'
`skills.html` are mapped explicitly here so none is hidden inside a generic
mechanics test:

| Documented interaction | Rust evidence |
|---|---|
| Radioactive decays first; both decay products copy the captured die | `radioactive_doppelganger_decays_before_both_products_copy_the_target` |
| A Doppelganger that captures a Rage die retains Rage after copying it | `doppelganger_that_captures_rage_retains_rage_after_transforming` |
| Copied Turbo does not resize the Doppelganger during the triggering attack | `copied_mighty_and_turbo_do_not_run_before_the_doppelganger_reroll` |

### Rage rule and interaction coverage

The live ButtonWeavers skills page describes three core Rage rules and repeats
one named Doppelganger interaction under both skills. Each distinct behavior is
mapped explicitly:

| Documented behavior | Rust evidence |
|---|---|
| Rage dice do not count for initiative | `rage_dice_do_not_contribute_to_initiative` |
| A participating Rage attacker loses Rage | `attacking_rage_die_loses_rage`, `only_participating_rage_dice_lose_rage` |
| A captured Rage die produces a rolled same-ability replacement without Rage | `captured_rage_die_is_replaced_until_the_round_ends` |
| A Doppelganger capturing Rage retains Rage after transforming | `doppelganger_that_captures_rage_retains_rage_after_transforming` |

Additional ButtonWeavers-source and edge-case coverage exercises failed Trip,
Speed multi-capture, Jolt, Time and Space, Konstant, Null, Value, Poison,
Radioactive, Mighty, Weak, Mood, Twin, Turbo, next-round restoration, and the
ten-original-to-twenty-round-dice capacity boundary. The older Rage issue
clarifications for Slow, Focus, and the initial roll of a Konstant replacement
also have direct tests. Rage+Fire (firing does not consume Rage) and Rage gained
during a Chaotic attacking reroll are recorded as deferred because BMAIR does
not yet parse or implement Fire or Chaotic. Single-attacker/single-target
Radioactive+Rage ordering remains part of the explicitly parsing-only
Radioactive work; the current Rage test uses a multi-target Speed attack, where
Radioactive does not trigger, to prove the replacement retains its other
properties without pretending that standalone Radioactive is complete.

## New differential coverage

- [x] Turbo option and Turbo swing enumeration/application/protocol output
  (`parity_turbo_option_in.txt`, `parity_turbo_swing_in.txt`, and unit tests).
- [x] Unique swing rejection (`parity_unique_qai_in.txt` plus unit test).
- [x] Chance success/failure chain and Konstant Chance
  (`parity_chance_in.txt`, `parity_chance_chain_in.txt`, and unit regressions).
- [x] Focus action/pass and Chance-to-Focus chaining
  (`parity_focus_in.txt`, `parity_chance_chain_in.txt`, and dizzy recovery tests).
- [x] Ornery with Mighty/Weak/Morphing combinations (combined and
  `parity_trip_morphing_in.txt` seeded fixtures plus unit tests).
- [x] Trip with Mighty/Weak and Konstant targets (`parity_trip_morphing_in.txt`
  plus unit tests).
- [x] Morphing/Twin and Morphing Speed non-effect
  (`parity_trip_morphing_in.txt` plus unit tests).
- [x] Combined Stealth+Insult precedence, Stinger stack pruning, Null+Value,
  Poison, Queer, Morphing Twin, Time and Space, Ornery, Mood, Mighty and Weak
  seeded game coverage (`parity_combined_mechanics_in.txt`).
- [x] Time and Space extra-turn behavior (combined seeded fixture).
- [x] Mood, Mighty, and Weak RNG/state ordering (combined seeded fixture).

## Final verification

- [x] `cargo fmt --check`.
- [x] `cargo test`; three expensive fixture searches are intentionally ignored
  in debug and covered by the release differential.
- [x] `cargo clippy --all-targets -- -D warnings`.
- [x] `cargo build --release`.
- [x] Clean C++ upstream test suite passes: 48/48 registered on 2026-08-26,
  with the one disabled developer fixture and two NDEBUG assertion deaths
  reported as the expected skips.
- [x] Konstant PR #82 C++ reference at `4813530` passes all 108 registered
  tests on 2026-08-27; the disabled developer fixture and two release-only
  assertion deaths remain the three expected skips.
- [x] Against the PR #82 reference, all 24 `*in*.txt` fixtures (including
  `bug105372_in.txt`) pass the material-output differential in 826.45 seconds,
  the representative raw RNG stream gate in 157.17 seconds, and the exhaustive
  RNG count/fingerprint gate in 901.65 seconds on 2026-08-27.
- [x] After native-mode parallel-search work, the legacy contract was rechecked
  on 2026-08-28 against PR #82 reference `4813530`: all current input fixtures
  passed material comparison in 487.65 seconds, representative raw RNG streams
  matched in 153.57 seconds, and exhaustive RNG fingerprints matched in 470.82
  seconds. Parser error parity and all 108 registered upstream tests also
  passed, with the same three expected upstream skips.
- [x] Full release C++/Rust fixture and added differential suite passes
  after the Value lifecycle correction (1,208.03 seconds on 2026-08-27).
- [x] The 0.5.0 Jolt implementation left every historical fixture unchanged:
  the complete material-output differential passed against the adopted
  Konstant reference at `4813530` in 567.54 seconds on 2026-09-01. Jolt is
  covered separately because the C++ reference predates it.
- [x] The `rust` branch through `ccc5ff5` is published with the complete parity
  implementation and REUSE-compliant attribution.

## Internal search proof

- [x] `tests/reference_trace_parity.rs` compares raw RNG streams for a routine
  representative gate and count+FNV fingerprints for every input fixture. The
  exhaustive 2026-08-27 run matched every stochastic fixture; its final
  intentional-error `test_in.txt` case was separately confirmed at exit status
  1 with the identical zero-event fingerprint after correcting the harness to
  accept matching nonzero exits. This covers hidden search work that the
  material-output normalizer cannot observe directly.

## Rust-native structural parity

Behavioral parity remains proven. These gates track meaningful implementation
correspondence without requiring unsafe Rust, C++ ABI compatibility, unions,
raw pointers, or literal byte copying.

- [x] Replace fresh deep game clones in hot evaluation loops with reusable
  simulation storage analogous to C++'s `BMC_Game sim; sim = *_game`, using
  safe Rust allocation reuse and complete state restoration. Swing, Reserve,
  Chance, Focus, BMAI, QAI, Maximizer, and fight orientation now use one scratch
  game plus explicit full-state restoration. `bmai_in` improved from 21.56 to
  18.98 seconds across the reuse work; `bug11_in` remains materially identical.
- [x] Generate fight candidates directly in canonical C++ order with one Turbo
  expansion pass. Rust now ports `BMC_DieIndexStack::Cycle` with a fixed safe
  stack and emits the C++ attacker/attack/target traversal without sorting.
- [x] Replace heap-backed attacker/target lists in hot moves with a bounded,
  compact Rust representation corresponding to C++ `BMC_BitArray`, while
  retaining ergonomic protocol/public access where needed. `BMC_DieIndexSet`
  is a copyable ten-bit value with ascending iteration and protocol mapping.
- [x] Align bounded game/player/die storage with C++ fixed-capacity state where
  practical, or document measured reasons for retaining dynamic storage. Move
  and combination state is fixed-capacity. Player dice remain a `Vec` bounded
  by the C++ protocol's ten-die contract: this is the idiomatic initialized-prefix
  representation, and reusable simulations retain its allocation. A fixed
  `[Option<BMC_Die>; 10]` would enlarge/complicate the public model without
  removing hot-path allocation after simulation reuse.
- [x] Map C++ cached die/player state (`m_sides_max`, attack/vulnerability bits,
  available dice, min/max value, property presence) and each invalidation/update
  point to an efficient Rust equivalent or a measured justification. See the
  structural state audit below.
- [x] Re-audit preround, reserve, initiative, Chance, Focus, fight, BMAI, BMAI3,
  QAI, culling, and rollout paths for corresponding control flow rather than
  output-only equivalence; intentional Rust-native deviations are recorded in
  the structural state and control-flow audit below.
- [x] After each structural change, pass unit/clippy/release checks, material
  fixture differential, representative raw RNG comparison, and exhaustive RNG
  fingerprints for changes capable of affecting enumeration or search. Final
  structural tree: 49 Rust tests (46 passed, three expected expensive ignores),
  clippy and debug/release all-target tests pass; after the final performance
  changes all material fixtures match in 537.04 seconds and every fixture RNG
  count/fingerprint matches in 440.54 seconds (2026-08-27).

### Compiler/linker optimization

- [x] Release builds use fat LTO and one codegen unit. This preserves normal
  Rust arithmetic/RNG semantics. Against Thin LTO, fat LTO was neutral on
  `bmsim_in` and `bug11_in`, reduced `bmai_in` user CPU by about 7%, and reduced
  `bug16_in` user CPU from 231.06 to 186.78 seconds in paired runs. PGO was
  evaluated last and deliberately not retained: it improved its three training
  fixtures but slightly regressed the untrained `bug16_in` case.

### Structural state and control-flow audit

| C++ state/path | Rust-native equivalent | Decision |
|---|---|---|
| fixed `BMC_Game` assignment into one `sim` | `RestoreSimulation` into one scratch game per evaluator | aligned; same-length dice use direct slice copying, with allocation-retaining `Vec::clone_from` for shape changes |
| `BMC_Move` attacker/target bit arrays | `BMC_DieIndexSet(u32)` | aligned; no per-move participant allocation |
| `BMC_DieIndexStack` direct attack walk | fixed `[usize; 10]` `BMC_DieIndexStack`, stack-backed available-dice views, and direct outer attacker/attack traversal | aligned; safe bounds replace raw array access and transient index vectors are eliminated |
| cached `m_sides_max` | sum of at most two `u8` sides in `GetSidesMax` | intentionally computed; cheaper invariant surface than synchronizing another field |
| cached attack/vulnerability bits | property branches in `CanDoAttack`/`CanBeAttacked` | intentionally computed; preserves Stealth's skill-dice-count rule explicitly and avoids stale masks after property mutation |
| cached available/min/max player values | bounded scans or first/last values after exact `OptimizeDice` ordering | intentionally computed over at most ten dice; capture/Trip/Chance/Focus already invoke the corresponding optimize points |
| property-presence lookup | bounded `HasAvailableProperty`/iterator scans | matches C++ `HasDieWithProperty`, which also scans rather than caching |
| preround/reserve BMAI batches and culling | `SelectSwingAction`/`SelectBMAIReserveAction` plus `BMC_BMAI3` evaluator settings | aligned, including static-level quirks and direct scratch restoration |
| Chance/Focus phase recursion | `SelectChanceAction`, `SelectFocusAction`, `EvaluateNextInitiativeAction` | aligned candidate batches, culling, phase transitions, and POV inversion |
| fight BMAI/BMAI3/QAI | direct ordered generation, `EvaluateMove`, `PlayFightQAI`, `SelectQAIAction` | aligned simulation lifecycle, ply transition, culling, and RNG order |

## 0.3 integration-boundary revalidation

The JSONL work observes the existing parser/search result; it does not parse
legacy output to reconstruct actions and does not introduce an alternate game
or AI path. `BmairSession::execute` runs `BMC_Parser::ParseString` against a
clone and commits that exact state only on success. `BMC_Parser::GetAction`
records the already-selected move beside the unchanged legacy writer, mapping
optimized storage indices back to original protocol die indices.

- [x] Authoritative C++ reference is upstream PR #82 head `4813530` (`Cover
  Konstant skill interactions`), not the older main-branch release binary.
- [x] Upstream PR #82 C++ suite: 108 tests discovered, 105 passed, and its three
  assertion/development cases intentionally skipped (2026-08-28).
- [x] All `tests/fixtures/*in*.txt` material outputs match the PR #82 C++
  reference after timing-only normalization (523.89 seconds, 2026-08-28).
- [x] Representative raw RNG states match exactly across five search/mechanics
  fixtures (180.29 seconds, 2026-08-28).
- [x] Every input fixture has the identical RNG event count and FNV fingerprint
  against an instrumented Release build of PR #82 (515.06 seconds,
  2026-08-28).
- [x] Invalid-command exit status and error behavior match the PR #82 parser
  reference; default Rust tests, JSONL process tests, clippy, and REUSE also
  pass after the integration boundary was added.
- [x] Capability discovery and legacy parsing share one authoritative table for
  all 29 die-property prefixes. After exposing that notation, all material
  fixture outputs matched again (491.85 seconds), representative raw RNG states
  matched (155.73 seconds), and every fixture RNG fingerprint matched (469.00
  seconds) against the instrumented Release PR #82 reference on 2026-08-28.

## 0.4 streaming legacy subprocess contract

- [x] C++ `ParseStdIn` consumes commands with `fgets`, and Rust
  `BMC_Parser::ParseStream` now consumes complete commands with `BufRead`
  without waiting for EOF. Both terminate on `quit`.
- [x] `legacy_banner_is_flushed_before_input` proves banner availability, and
  `legacy_stdin_matches_bmaibagels_write_flush_read_contract` returns an action
  while the parent deliberately keeps stdin open.
- [x] `streamed_legacy_commands_match_batched_parsing` asserts identical
  output, session metadata, typed action, and replay metadata for streamed and
  batched execution of the same seeded request.
- [x] On 2026-08-29, the PR #82 Release reference passed parser-error parity;
  every material fixture matched (533.45 seconds); representative raw RNG
  states matched (168.60 seconds); and every fixture RNG fingerprint matched
  (539.06 seconds). The three ignored deep-search Rust tests also passed in
  Release mode (246.49 seconds).

## 0.4.1 legacy search diagnostic

- [x] Top-level legacy and native fight searches expose the evaluator's actual
  accumulated best score and simulations-run count without rerunning search or
  consuming RNG.
- [x] `legacy_stdin_matches_bmaibagels_write_flush_read_contract` applies
  BMAIBagels' historical ` p0 best move ` and `%` extraction to subprocess
  output and requires a finite numeric percentage.
- [x] The focused C++ comparison request reports identical `0.0 points, 0.0%
  win` fields in both implementations.
- [x] On 2026-08-29, parser-error parity passed; every material fixture
  matched (517.75 seconds); representative raw RNG states matched (154.06
  seconds); every fixture RNG fingerprint matched (469.60 seconds); and all
  three extended Release tests passed (207.99 seconds).
