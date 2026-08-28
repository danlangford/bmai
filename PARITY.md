# C++ → Rust full-parity ledger

Authority: C++ `main` at `1fcb826` and the contract in `AGENTS.md`.

## Completion gates

- [x] All 15 shipped `*in*.txt` fixtures have materially identical outputs.
  Evidence: release differential passed on 2026-08-23; seeded internal traces
  also matched the large preround/reserve searches.
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

- [x] Inventory all 48 registered upstream test cases: 42 functional cases,
  two debug assertion contracts, three demo/framework cases, and one disabled
  developer setup case.
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
| `BMC_Parser::ParseDie*` | `parser::ParseDie`, `parse_side`, `prefix_property` | `parses_twin_option_and_properties`, every shipped fixture | covered |
| `BMC_Die::OnSwingSet`, `SetOption`, `Roll`, `Reset`; `BMC_Player::Reset`, `RollDice`, `OptimizeDice` | `ApplySwingMove`, `RollDie`, match reset, `BMC_Player::OptimizeDice` | both lifecycle panic ports, Turbo/Unique tests, exact seeded fixture traces | covered |
| `BMC_Die::GetScore` ordinary/Poison/Value/Null/Warrior | `BMC_Die::GetScore` | score branch tests and all upstream skill score ports | covered |
| `BMC_Game::GenerateValidAttacks`, `ValidAttack` for Power/Skill/Speed/Trip/Shadow/Berserk | `GenerateValidAttacks`, `GenerateValidAttacksInCppOrder` | upstream attack/Stealth/Insult tests | covered |
| Konstant, Stealth, Warrior, Stinger, Unskilled, Queer attack restrictions | `CanDoAttack`, `CanBeAttacked`, subset/minimum skill enumeration | upstream ports, Stinger pruning/Stealth+Insult regressions, combined seeded differential | covered |
| `BMC_Die::OnApplyAttackPlayer` Berserk, Mighty, Weak, Morphing, Turbo, Warrior and Ornery second pass | `ApplyAttackPlayerEffects` | Ornery/Mighty, Morphing/Twin/Speed, Turbo, Warrior tests and combined differentials | covered |
| `OnBeforeRollInGame`, nature rerolls, Mood, Trip double before-roll pass | `ApplyBeforeRollEffects`, `ApplyMood`, `ApplyAttackerNatureRoll`, `RollScheduledDie` | Trip/Weak/Konstant tests and combined seeded differentials | covered |
| captured Null/Value mutation and scoring | target mutation in `ApplyAttackForPlayers` before captured score | property score tests and combined seeded differential | covered |
| Time and Space odd-roll extra turn and dizzy recovery | `ApplyAttackForPlayers` extra-turn result, `RecoverDizzyDice` | combined seeded differential and exact QAI/RNG trace | covered |
| `CheckInitiative`, Chance chain, Focus values, dizzy state | `CheckInitiative`, `ApplyChanceMove`, `ApplyFocusMove`, initiative evaluators | Konstant Chance, C++ player-index asymmetry regression, parser initiative tests, and chained seeded differential | covered |
| simultaneous preround evaluation, option/swing Cartesian product, `UNIQUE` | `GenerateSwingMoves`, `EvaluateSwingMove`, `ApplySwingMove` | exact bug11/preround traces, Unique unit test | covered |
| reserve activation and BMAI/BMAI3 evaluation | `ApplyUseReserve`, `SelectBMAIReserveAction` | exact bug16 candidate/simulation/RNG trace | covered |
| base random AI, Maximizer, QAI, legacy BMAI, BMAI3 | policy dispatch, `SelectRandomAction`, `SelectMaximizeAction`, `SelectQAIAction`, fixed/culling evaluators | seeded `ai` and all four `playfair` mode comparisons | covered |
| max ply, QAI transition, BMAI3 batches/culling/Trip threshold, surrender | `EvaluateMove`, `PlayFightQAI`, `BMC_BMAI3::EvaluateMoves`/`CullMoves` | exact ply-2 and full bug16 traces, evaluator tests | covered |
| round/match standings, loser swing reset, initiative fairness matrix | `PlayRoundWithPolicies`, `PlayMatchWithPolicies`, `PlayGames`, `PlayFairGames` | bmsim fixture and four playfair mode comparisons | covered |
| `BMC_RNG` seed expansion, integer/float output, consumption order | `BMC_RNG`; RNG passed through all stochastic operations | exact sequence/distribution tests and multi-million-event fixture traces | covered |

Parsing-only parity is intentional for `AUXILIARY`, `DOPPLEGANGER`,
`RADIOACTIVE`, and `RAGE`: upstream C++ only assigns their property bits in
`BMC_Parser::ParseDie` and implements no game behavior. `UNSKILLED` is marked
TODO upstream but both engines enforce its existing no-Skill-attack behavior.

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
- [x] Full release C++/Rust fixture and added differential suite passes
  after the Value lifecycle correction (1,208.03 seconds on 2026-08-27).
- [x] Worktree committed locally and not pushed.

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
| `BMC_Move` attacker/target bit arrays | `BMC_DieIndexSet(u16)` | aligned; no per-move participant allocation |
| `BMC_DieIndexStack` direct attack walk | fixed `[usize; 10]` `BMC_DieIndexStack`, stack-backed available-dice views, and direct outer attacker/attack traversal | aligned; safe bounds replace raw array access and transient index vectors are eliminated |
| cached `m_sides_max` | sum of at most two `u8` sides in `GetSidesMax` | intentionally computed; cheaper invariant surface than synchronizing another field |
| cached attack/vulnerability bits | property branches in `CanDoAttack`/`CanBeAttacked` | intentionally computed; preserves Stealth's skill-dice-count rule explicitly and avoids stale masks after property mutation |
| cached available/min/max player values | bounded scans or first/last values after exact `OptimizeDice` ordering | intentionally computed over at most ten dice; capture/Trip/Chance/Focus already invoke the corresponding optimize points |
| property-presence lookup | bounded `HasAvailableProperty`/iterator scans | matches C++ `HasDieWithProperty`, which also scans rather than caching |
| preround/reserve BMAI batches and culling | `SelectSwingAction`/`SelectBMAIReserveAction` plus `BMC_BMAI3` evaluator settings | aligned, including static-level quirks and direct scratch restoration |
| Chance/Focus phase recursion | `SelectChanceAction`, `SelectFocusAction`, `EvaluateNextInitiativeAction` | aligned candidate batches, culling, phase transitions, and POV inversion |
| fight BMAI/BMAI3/QAI | direct ordered generation, `EvaluateMove`, `PlayFightQAI`, `SelectQAIAction` | aligned simulation lifecycle, ply transition, culling, and RNG order |
