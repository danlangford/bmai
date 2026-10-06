# C++ → Rust full-parity ledger

Authority: C++ `main` at `1fcb826`, Konstant PR #82 at `4813530`, and the
contract in `AGENTS.md`. PR #82 is treated as the reference for its signed
Konstant behavior until it lands upstream.

Rust types took Rust naming in 0.17.0 (for example `BMC_Game` became `Game`
and `BME_ATTACK::POWER` became `Attack::Power`), and functions and fields took
it in 0.19.0: C++ `GetValueTotal` is `value_total`, `ApplyAttack` is
`apply_attack`, and `m_die` is `dice`. Names in the C++ columns below are the
C++ names.

## Regression oracle policy

`tests/golden/` holds the expected output of every `tests/fixtures/*in*.txt`,
checked by `tests/fixture_golden.rs`. Each file records the normalized
protocol output, the exit status, and the RNG draw count and hash, so a change
to candidate order, simulation counts, or randomness fails even when the final
move survives it. The files were generated from the 0.14.0 source, whose
fixtures had matched the C++ reference at `4813530`; that historical evidence
is recorded below.

No reference binary is needed. The ordinary test run checks every fixture
except the long searches in `SLOW_FIXTURES`, which CI checks for releases.
When a change intentionally alters a fixture, regenerate the files with
`BMAIR_UPDATE_GOLDEN=1` and review the diff in the pull request. New skills
follow ButtonWeavers, not C++, so a correct rule may change a golden file.

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
  `game::tests::pr82_signed_konstant_skill_attack_matrix` and
  `pr82_variable_skill_stack_disables_legacy_value_pruning`.
- [x] Konstant/Stinger/Warrior range, sign, gap, full-value, later-target, and
  unused-pool cases are individually named in the table-driven Rust test, so
  failures report the corresponding upstream GoogleTest name.
- [x] Trip and Chance Mighty/Weak/Maximum sequencing maps to
  `konstant_trip_targets_keep_their_size_and_value`,
  `pr82_trip_target_before_roll_effect_triggers_once`,
  `pr82_chance_rerolls_resize_mighty_and_weak_dice_once`, and
  `konstant_chance_dice_keep_their_size_and_value`. The Konstant cases follow
  the ButtonWeavers engine, where Konstant also blocks Mighty and Weak,
  instead of C++.
- [x] Participating/nonparticipating Ornery, Konstant Mighty/Weak, Mood, and
  pass behavior maps to
  `konstant_ornery_mighty_and_weak_dice_keep_their_size_while_others_reroll`,
  `ornery_mood_dice_change_after_an_attack_but_not_a_pass`,
  `pr82_participating_ornery_before_roll_effect_triggers_once`, and
  `konstant_ornery_mood_die_keeps_its_size_and_value`,
  which follows the ButtonWeavers engine (Konstant blocks the Mood resize)
  instead of C++;
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
  `game::tests::cpp_player_copy_constructor_is_independent`.
- [x] `ParserTests.ParseString` ->
  `protocol::legacy::tests::cpp_parser_multiline_fight_string`.
- [x] NoSkill, MultiDieSkillAttack, SingleDieSkillAttack,
  KonstantSingleDieSkillAttack, StealthSingleDieSkillAttack, and
  StealthMultiDieSkillAttack -> `cpp_basic_power_and_skill_attack_generation`
  plus `copied_cpp_skill_restrictions_match_konstant_and_stealth_cases`.
- [x] MaximumSkill -> `cpp_maximum_die_always_rolls_its_maximum` and
  `cpp_speed_generation_and_property_score_combinations`.
- [x] Konstant Trip, Chance, Skill, and Warrior tests -> the four
  `search::tests::core::cpp_konstant_*`/`copied_cpp_konstant_*` tests.
- [x] Insult and all nine Stealth tests ->
  `game::tests::cpp_insult_and_stealth_restrictions` plus basic generation.
- [x] Null, Value, NullValue, Poison, PoisonValue, and PoisonNull ->
  `score_matches_cpp_property_branches` and
  `cpp_speed_generation_and_property_score_combinations`.
- [x] SpeedSkill -> `cpp_speed_generation_and_property_score_combinations`.
- [x] MorphingSkill, MorphingTwinSkill, MorphingSpeedSkill ->
  `cpp_morphing_copies_single_and_twin_target_sizes` and
  `copied_cpp_multi_target_speed_attack_does_not_morph`.
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
  Since 0.23.0 BMAIR defaults to `mode native` with a wider search budget, so
  a C++ protocol input reproduces C++ only after `mode legacy`, `ply 1`,
  `max_sims 500`, `min_sims 10`, and `maxbranch 5000`. The golden test sends
  those before each fixture, which stays a byte-identical C++ input.
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
- [x] `playfair`, with initiative-split reporting. Intentional difference
  since 0.20.0: C++ modes 0-3 picked random, maximizer, or a non-culling BMAI
  with one of two rollout policies for both players. BMAIR's `playfair GAMES`
  plays the engines the players already have, so modes 0, 1, and 3 are `ai`
  and setting choices (`random`, `maximize`, and `montecarlo` with
  `cull off`). Mode 2's mixed Maximize-or-Random playout was dropped; no
  client used it, and a randomized playout can return if the harness shows
  it helps.
- [x] Intentional difference since 0.20.0: C++ `ai <player> <type>` types 0
  (fixed-simulation BMAI), 1 (QAI), and 2 (culling BMAI3) are the named
  engines `montecarlo` with `cull off`, `quick`, and `montecarlo`.
- [x] Per-player `ply`, `max_sims`, `min_sims`, and `maxbranch` parsing.
  Intentional difference since 0.20.0: C++ players share AI objects by
  pointer, so a per-player setting also reaches the other player and later
  games. BMAIR gives each player its own engine: a per-player setting copies
  the global settings for that player only, `ai PLAYER NAME` selects a named
  engine with its own defaults, and `game` returns both players to the global
  settings. `each_player_owns_its_engine_settings` and
  `per_player_ai_settings_in.txt` cover it.
- [x] `debug` category validation/state and `debugply` parsing/state, including
  C++'s exact uppercase category and boolean-setting behavior.
- [x] Error messages, invalid inputs, phase restrictions, and exit behavior:
  focused executable differential covers unknown commands, invalid AI/debug,
  invalid phase/player setup, invalid `getaction`, and simulation phase errors.
- [x] Public Rust types/functions are mapped against reachable C++ game/AI
  behavior below. Rust is behavior-compatible, not C++ ABI/source-compatible.

### Public API mapping

- `BMC_DieData`/`BMC_Die` state and accessors map to public `Die` fields,
  `has_property`, `sides_max`, `value_total`, `is_available`, `score`,
  `roll`, `on_swing_set`, and `on_dizzy_recovered`; remaining event methods are
  invoked by the game engine so their ordering cannot be bypassed accidentally.
- `BMC_Player` getters/events map to public fields plus `optimize_dice`; dice
  setup is direct construction/parsing instead of the inert C++ `BMC_Man`
  container. `BMC_Man` has no independently configurable public behavior in
  C++ and therefore needs no Rust runtime type.
- `BMC_Game::GenerateValidAttacks`, `SimulateAttack`, `CheckInitiative`, and
  `RecoverDizzyDice` map to `generate_valid_attacks`, `simulate_attack`,
  `check_initiative`, and `recover_dizzy_dice`. Full round/game play,
  preround, reserve, Chance, and Focus are exposed through `Parser` and
  `play_games`, with `Game` serving as the explicit game template instead
  of C++ `PlayGame(BMC_Man*, BMC_Man*)` setup pointers.
- `BMC_AI`, Maximizer, QAI, legacy BMAI, and BMAI3 virtual dispatch maps to
  the crate-internal `Engine` trait and its `random`, `maximize`, `quick`,
  and `montecarlo` engines; `Bmai3` keeps the public search settings,
  simulation-count computation, evaluator, and last probability, and
  `Playout` selects the simulated games' engine.
- `BMC_Move`'s tagged union maps to `Move` for fight actions and internal
  typed Swing/Chance/Focus/Reserve moves. Protocol clients observe those move
  types through the same parser action output rather than union field access.
- `BMC_RNG` maps directly to public `reseed`, `rand`, `rand_below`, and
  `rand_f32`. Logger and stats presentation are transport diagnostics; parser
  debug settings and stable stats/action output are externally preserved.

## Feature and search matrix

Status meanings: **covered** has implementation and direct test evidence;
**source-audited** has a mapped implementation but still needs the differential
case named in the final column.

| C++ behavior | Rust implementation | Evidence | Status |
|---|---|---|---|
| `BMC_Parser::ParseDie*` | `protocol::legacy::die` | defined Twin Swing parser matrix, all advertised property prefixes, all phases and value/dizzy state, forced-win search scenario in four mode/worker combinations, `parity_defined_twin_swing_in.txt`, every shipped fixture | covered |
| `BMC_Die::OnSwingSet`, `SetOption`, `Roll`, `Reset`; `BMC_Player::Reset`, `RollDice`, `OptimizeDice` | `apply_swing_move`, `roll_die`, match reset, `Player::optimize_dice` | both lifecycle panic ports, Turbo/Unique tests, exact seeded fixture traces | covered |
| `BMC_Die::GetScore` ordinary/Poison/Value/Null/Warrior | `Die::score` | score branch tests and all upstream skill score ports | covered |
| `BMC_Game::GenerateValidAttacks`, `ValidAttack` for Power/Skill/Speed/Trip/Shadow/Berserk | `game::attack` direct ordered enumeration | upstream attack/Stealth/Insult tests | covered |
| Konstant, Stealth, Warrior, Stinger, Unskilled, Queer attack restrictions | `can_do_attack`, `can_be_attacked`, direct stack enumeration plus `skill_stack_can_hit` signed intervals | PR #82's complete signed-Konstant/Stinger/Warrior matrix, Stealth+Insult regressions, differentials | covered |
| `BMC_Die::OnApplyAttackPlayer` Berserk, Mighty, Weak, Morphing, Turbo, Warrior and Ornery scheduling | `apply_attack_player_effects`, cached attack-phase available boundary | PR #82 participating/nonparticipating Ornery, Morphing/Twin/Speed, Turbo, Warrior tests | covered |
| `OnBeforeRollInGame`, nature rerolls, Mood, and Trip's single before-roll pass | `apply_before_roll_effects`, `apply_mood`, `apply_attacker_nature_roll`, `roll_scheduled_die` | PR #82 Trip/Chance/Ornery/Konstant effect and pass tests plus seeded differentials | covered |
| captured Null/Value mutation and scoring | target mutation in `apply_attack_for_players` before captured score | property score tests and combined seeded differential | covered |
| Time and Space odd-roll extra turn and dizzy recovery | `apply_attack_for_players` extra-turn result, `recover_dizzy_dice` | combined seeded differential and exact QAI/RNG trace | covered |
| Jolt attacker consumption and attacker/captured-defender extra turns | Jolt snapshots and attacker-property removal in `apply_attack_for_players` | focused Jolt, Trip, Konstant, multiple-die, and Time-and-Space tests | covered Rust extension |
| ButtonWeavers Doppelganger Power-capture transformation and round reset | target recipe replacement in `apply_attack_player_effects`, Radioactive decay expansion, original-recipe restoration in `restore_dice_for_new_round` | focused ordinary/Skill/Twin/Swing, Jolt, Time-and-Space/Konstant, Mighty/Turbo, Rage, Radioactive, and round-lifecycle tests | covered Rust extension |
| ButtonWeavers Rage initiative, participation, replacement, and round reset | Rage initiative filtering, attacker snapshots, bounded replacement creation, and `restore_dice_for_new_round` | focused Rage core rules plus Doppelganger, Jolt, Time-and-Space, Konstant, scoring, reroll, multi-target, and capacity scenarios | covered Rust extension |
| ButtonWeavers Fire-assisted Power/Skill attacks and persistent turndowns | exact `FireAdjustment` attacker increases/helper reductions, direct candidate expansion, and pre-attack application | focused Fire rules plus Stinger, Konstant, Mighty, Weak, Rage, Jolt, Time-and-Space, Queer, Twin, multi-helper, typed-action, and legacy-wire scenarios | covered Rust extension |
| ButtonWeavers Radioactive decay of the attacker in every one-attacker, one-target attack | `radioactive_decay_applies`, `apply_radioactive_attack_effects`, `split_radioactive_attacker`, the Trip branch of `apply_attack_for_players`, and Turbo-candidate suppression in `expand_turbo_moves` | every skills.html interaction plus responder-log reproductions in `search::tests::radioactive` | covered Rust extension |
| ButtonWeavers Rush two-target attacks by or against Rush dice | `Attack::Rush` direct pair enumeration in `generate_valid_attack_candidates_in_cpp_order`, shared `can_do_attack`/`can_be_attacked` Speed restrictions, generic multi-target resolution | focused Rush rules plus Speed, Stealth, Warrior, Focus, Insult, Konstant, Stinger, Fire, Twin, Berserk, Morphing, Doppelganger/Radioactive, Jolt, Time-and-Space, Rage, Null, Value, Poison, Mighty, Weak, Mood, Ornery, Maximum, Turbo, Reserve, parser, and legacy/native search scenarios | covered Rust extension |
| `CheckInitiative`, Chance chain, Focus values, dizzy state | `game::mechanics` initiative plus `search::initiative` evaluators | Konstant Chance, C++ player-index asymmetry regression, parser initiative tests, and chained seeded differential | covered |
| simultaneous preround evaluation, option/swing Cartesian product, `UNIQUE` | `search::preround` generation, evaluation, and application | exact bug11/preround traces, locked swing/option regressions, Unique unit test | covered |
| ButtonWeavers Auxiliary mutual accept/decline lifecycle and courtesy copy | `prepare_auxiliary_phase`, `apply_auxiliary_decision`, legacy/native Auxiliary selectors | readable wire-protocol, mechanics, invalid-input, and worker-independence tests | covered Rust extension |
| reserve activation and BMAI/BMAI3 evaluation | `search::preround` selection plus `search::match_play` dispatch | exact bug16 candidate/simulation/RNG trace plus `complete_native_match_uses_reserve_after_a_round_loss` | covered |
| base random AI, Maximizer, QAI, legacy BMAI, BMAI3 | `engines` (`random`, `maximize`, `quick`, `montecarlo`) and `search::ai` evaluators | seeded `ai` and `playfair` comparisons for every engine | covered |
| max ply, QAI transition, BMAI3 batches/culling/Trip threshold, surrender | `search::fight` rollout control plus `search::ai` batching/culling | exact ply-2 and full bug16 traces, evaluator tests | covered |
| round/match standings including ties, loser swing reset, initiative fairness matrix | `search::match_play` round, match, and fairness orchestration | bmsim fixture, `playfair` for every engine, `tied_round_has_no_loser`, and complete-match reserve regression | covered |
| `BMC_RNG` seed expansion, integer/float output, consumption order | `Rng` dispatching `LegacyParkMillerV1`; RNG passed through all stochastic operations | version/name/continuity tests, exact sequence/distribution tests, and multi-million-event fixture traces | covered |

Native search deliberately advances beyond C++ BMAI3's probability-reporting
behavior under the versioned `bmair-native-stream-v2` replay contract. Culling
still removes inferior candidates, but the survivor runs through its declared
simulation budget; an opening bounded random draw is stratified across
canonical simulation indices, and consecutive bounded draws traverse
mixed-radix outcome cells. The game 119365 regression proves that the shared
Poison/Queer mechanics win only on d20 rerolls 5 and 6, while the search
scenario proves legacy and native both report 10% and select the same Power
capture. Reconstructed ordinary-d10 and Twin-d6 endgames cover single- and
multi-die distributions, including draw-as-half-win aggregation. Legacy
C++-ordered RNG consumption and early culling remain unchanged.

The C++ source assigns the `AUXILIARY` property and declares an Auxiliary AI
action, but does not implement the phase or selector. BMAIR's complete
Auxiliary lifecycle is therefore an intentional post-C++ extension based on
the ButtonWeavers engine at `a2d2a1fac12bcffd3453bb0dfe1282b733d23a5b`.
The wire state has no button identity, so site-specific eligibility such as
Gordo's restriction remains the caller's responsibility. Upstream C++ only
assigns the `RADIOACTIVE` property bit; BMAIR 0.15.0 implements the complete
ButtonWeavers decay rule as an intentional post-C++ extension (see "Radioactive
rule and interaction coverage" below). Doppelganger is an intentional post-C++ extension
based on ButtonWeavers engine source at `a2d2a1fac12bcffd3453bb0dfe1282b733d23a5b`.
Rage is also an intentional
post-C++ extension based on the ButtonWeavers engine at the same pinned source
revision and its live skill contract.
Fire is an intentional post-C++ extension based on that pinned ButtonWeavers
source. Unlike ButtonWeavers' aggregate-only `firingAmount`, BMAIR records exact
attacker increases so the documented Fire+Konstant value retention has an
unambiguous simulation state. Wildcard Fire is excluded because BMAIR does not
implement Wildcard dice. Odd Queer attack-type eligibility follows the current
ButtonWeavers ordering and is tested explicitly.
`UNSKILLED` is marked TODO upstream but both engines enforce its existing
no-Skill-attack behavior. Rust accepts the legacy C++ maximum of ten input dice
per player and uses a compact 20-bit in-round index space. Twenty slots cover
both every Radioactive+Doppelganger distribution of the original two-player
pool and one round-local Rage replacement for every original die. Both the
input and transformed limits fail explicitly rather than silently dropping
dice or skill behavior.

Mechanics and search scenarios in `src/search/test_support/` are test-only
adapters over the production parser, C++-ordered legality enumeration, attack
resolution, RNG, search, and round restoration used by the executable. The
adapters do not provide alternate rules or search implementations. Canonical
die-recipe state assertions and protocol-level win-percentage ranges keep the
coverage reviewable while preserving existing parity evidence and production
control flow.

## Phase coverage inventory

| Phase or decision | BMAIR status |
|---|---|
| Auxiliary (`aux`) | Full parser, lifecycle, legacy/native search, text action, and typed action support. |
| Preround, Reserve, Chance, Focus, Fight | Full action-selection and simulation support. |
| Initiative | Parsed and simulated automatically; it has no direct `getaction` decision. |
| Game over | Parsed terminal state; it has no direct `getaction` decision. |
| Turbo selection | Returned atomically as part of a Fight attack rather than exposed as a separate phase. |
| Fire adjustment | Generated and applied atomically with Fight actions; legacy and typed outputs include each assisting die's final value. |

ButtonWeavers' server orchestration states—joining games, custom recipes,
loading buttons, starting/ending rounds and turns, and committing attacks—are
not independent BMAIR decision phases. The caller supplies the resulting game
state, while BMAIR performs the applicable rules transition during simulation.

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
  every material-output and RNG-fingerprint differential run;
  `parity_defined_swing_postfix_in.txt` makes the postfix form affect a
  terminal win result so that branch also has a C++ oracle.

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
| A Rage+Fire die does not lose Rage when it only fires | `rage_fire_keeps_rage_when_it_only_assists` |

Additional ButtonWeavers-source and edge-case coverage exercises failed Trip,
Speed multi-capture, Jolt, Time and Space, Konstant, Null, Value, Poison,
Radioactive, Mighty, Weak, Mood, Twin, Turbo, next-round restoration, and the
ten-original-to-twenty-round-dice capacity boundary. The older Rage issue
clarifications for Slow, Focus, and the initial roll of a Konstant replacement
also have direct tests. Rage gained during a Chaotic attacking reroll remains
deferred because BMAIR does not implement Chaotic. Single-attacker/single-target
Radioactive+Rage ordering is covered by
`captured_radioactive_rage_target_is_replaced_and_still_decays_the_attacker`.

### Fire rule and interaction coverage

The ButtonWeavers Fire description and interaction metadata are mapped to
explicit scenarios:

| Documented behavior | Rust evidence |
|---|---|
| Fire cannot Power Attack | `fire_dice_cannot_power_attack` |
| Fire can assist ordinary Power and Skill attacks by transferring displayed value | `fire_assists_a_power_attack_and_stays_turned_down`, `fire_assists_a_skill_attack` |
| Fire cannot assist other attack types | `fire_does_not_assist_nonstandard_attack_types` |
| Neither helper nor attacker may leave its normal value range | `fire_at_its_minimum_cannot_assist`, `fire_cannot_raise_an_attacker_past_its_maximum`, `twin_fire_cannot_turn_down_below_one_per_component` |
| Mighty+Fire grows only when rolling, not when firing | `mighty_fire_does_not_grow_when_it_only_assists` |
| Weak+Fire shrinks only when rolling, not when firing | `weak_fire_does_not_shrink_when_it_only_assists` |
| A fired-up Konstant die retains the changed value | `fired_up_konstant_keeps_its_new_value_after_a_skill_attack` |

Additional scenarios cover multiple Fire helpers, ButtonWeavers' default-off
Fire-overshooting preference and BMAIR's explicit opt-in command, Stinger's
flexible current-value contribution, Rage retention, nonparticipating Jolt and
Time-and-Space, Fire participating in Skill attacks, current ButtonWeavers
Queer eligibility, the Ornery helper reroll in
`assisting_ornery_fire_die_still_rerolls_after_the_attack`, the initial Turbo
boundary, and both legacy and typed action output. Fire-assisted attacks
involving an attacking Turbo die currently use its displayed size; alternate
Turbo sizes are an explicit follow-up rather than reusing a plan calculated
for a different maximum. Search materializes at most
`max(1, maxbranch / min_sims)` Fire-assisted candidates per state so a large
button cannot exhaust time and memory enumerating allocations before its
configured branch budget applies; `fire_candidate_construction_obeys_the_search_budget`
covers that boundary.

### Radioactive rule and interaction coverage

Source: ButtonWeavers `BMSkillRadioactive`, `BMDie::split`, `BMDieTwin::split`,
`BMAttack::commit_attack`, and the hook order in `BMSkill::skill_order_array`
at `a2d2a1fac12bcffd3453bb0dfe1282b733d23a5b`. Decay comes from the attacker's
`capture` hook or the target's `be_captured` hook, whichever die is
Radioactive. Because the target hook runs after every attacker hook, a
Radioactive attacker decays before Doppelganger copies, while a Doppelganger
copy of a Radioactive target is itself what decays. Cases marked "responder log"
reproduce dice from ButtonWeavers' own `responder0*Test.php` action logs.

| skills.html interaction | Rust evidence |
|---|---|
| Berserk halves, then decays | `berserk_halves_before_it_decays` (responder log), `radioactive_berserk_attacker_halves_before_it_decays` |
| Doppelganger decays, then each product copies the target | `radioactive_doppelganger_decays_before_each_product_copies_the_target`, `radioactive_doppelganger_decays_before_both_products_copy_the_target`; the Radioactive-target order is `doppelganger_copy_of_a_radioactive_target_decays` (responder log) |
| Mad is lost on decay | not applicable: BMAIR does not implement Mad, and BMAIBagels refuses Mad games |
| Mood is lost on decay | `decay_removes_mood_so_the_products_keep_their_halved_size` |
| Morphing morphs, then decays | `morphing_attacker_morphs_before_a_radioactive_target_decays_it`; a Radioactive Morphing attacker follows the engine instead: `radioactive_morphing_attacker_decays_into_two_full_size_morphs` |
| Time and Space is lost on decay | `decay_removes_time_and_space_so_an_odd_reroll_grants_no_extra_turn` |
| Turbo is lost on decay | `decay_removes_turbo_and_turbo_sizes_are_not_offered`; Trip keeps its sizes because it rolls first: `turbo_trip_still_offers_sizes_because_it_rolls_before_decaying` |

| Description rule | Rust evidence |
|---|---|
| The attacker splits into near-equal halves summing to its size | `radioactive_attacker_decays_into_two_halves_that_sum_to_its_size`, `a_one_sided_die_decays_into_a_one_sided_and_a_zero_sided_die` (responder log), `twin_dice_decay_into_alternating_halves` (responder log), `odd_twin_halves_give_each_product_one_rounded_up_subdie` (skills.html example) |
| Either die being Radioactive triggers one decay | `attacker_decays_when_only_the_target_is_radioactive_and_the_target_keeps_radioactive_when_captured`, `two_radioactive_dice_decay_the_attacker_only_once` (responder log) |
| Only attacks with one attacker and one target decay, of any type | `single_die_skill_attack_decays` (responder log), `shadow_attack_decays_and_keeps_shadow` (skills.html example), `single_target_speed_attack_decays`, `multi_target_speed_attack_does_not_decay`, `multi_die_skill_attack_does_not_decay` |
| Involved dice remaining in play lose Radioactive | `failed_trip_still_decays_and_the_surviving_target_loses_radioactive` (responder log), `successful_trip_decays_after_the_trip_roll` (responder log) |
| Decayed dice lose Jolt | `decay_removes_jolt_after_jolt_grants_its_extra_turn` |

Further scenarios cover Konstant and Weak products, Rage on both sides, Null,
scoring, next-round restoration, the dice-pool limit, and legacy/native search.
Decay products always roll fresh values, including Konstant and Trip
attackers. Mighty and Weak resize the products of ordinary attackers and of a
Doppelganger copy of a Radioactive target, but not Konstant products.

Same-die combinations were settled by running the ButtonWeavers engine
(`BMAttack::commit_attack` under PHP 8.5), because its by-reference attacker
loop contradicts two documented interactions. BMAIR follows the engine:

| Engine probe | Result | Rust evidence |
|---|---|---|
| `D(20)` captures `H(6)` / `h(12)` / `kH(4)` | `H(8)` / `h(10)` / rerolled `kH(6)` | `copied_mighty_grows_but_copied_turbo_does_not_resize`, `copied_weak_shrinks_on_the_doppelganger_reroll`, `copied_konstant_still_resizes_and_rerolls` |
| `%D(9)` captures `H(4):4` | `H(4):4` never rerolled, plus a rerolled `H(6)` | `radioactive_doppelganger_keeps_the_first_copy_unrolled` |
| `%m(4)` captures `(6,6)` | two full-size `m(6,6)`, not halves | `radioactive_morphing_attacker_decays_into_two_full_size_morphs` |
| `%tm(4)` Trips `(6)` | two full-size `tm(6)` | `radioactive_morphing_trip_decays_into_two_full_size_morphs` |
| `%B(12)` Berserk vs `(6)` | `(3)` and `(3)` | `radioactive_berserk_attacker_halves_before_it_decays` |
| `m(4)` captures `%(10)` | `m(5)` and `m(5)` | `morphing_attacker_morphs_before_a_radioactive_target_decays_it` |

No current button has same-die Radioactive Morphing or Doppelganger dice.
A decay that would exceed the 20-die pool is skipped instead of panicking.

### Button specials and rule corrections

Source: ButtonWeavers `BMBtnSkillUniqueSwing`, `BMBtnSkillGordo`,
`BMBtnSkillLargo`, `BMBtnSkillTheFlyingSquirrel`, `BMAttackSkill::
are_button_skills_compatible` (The Japanese Beetle), `BMBtnSkillGiant` with
`BMGame::is_button_slow`, `BMAttackTrip::validate_attack`, and
`BMSkillMorphing::capture`. Clients name specials with the `special` command
because the wire state carries no button identity.

| ButtonWeavers rule | Rust evidence |
|---|---|
| `special` sets, reports, validates, resets per game, and survives simulation side swaps | `special_command_sets_and_reports_each_players_specials`, `each_game_block_clears_specials`, `unknown_specials_and_players_are_rejected`, `simulations_keep_each_players_specials_after_a_side_swap`, `largo_search_reports_a_power_attack_over_the_wire` |
| Largo and The Flying Squirrel cannot Skill attack | `largo_cannot_skill_attack`, `largo_may_still_power_attack` |
| The Japanese Beetle cannot be Skill attacked | `japanese_beetle_cannot_be_skill_attacked`, `japanese_beetle_may_still_be_power_attacked` |
| Giant cannot win initiative, even against a button without initiative dice | `no_initiative_loses_to_lower_dice_and_to_a_button_with_no_initiative_dice` |
| Guillermo and Oregon assign different swing types different sizes | `unique_swing_assigns_different_swing_types_different_sizes` |
| Gordo also avoids fixed die sizes, comparing option dice at their chosen side, and declines a single V-Z Auxiliary swing die for both players | `unique_sizes_also_avoids_fixed_die_sizes`, `unique_sizes_compares_option_dice_at_their_chosen_side`, `gordo_declines_a_v_to_z_auxiliary_swing_die`, `gordo_accepts_other_auxiliary_dice`, `either_players_gordo_decline_removes_both_auxiliary_dice` |
| A Trip needs only to reach the target's minimum, with Konstant, Maximum, Mighty, Weak, Mood, and Turbo adjustments | `single_trip_dice_may_trip_twin_dice_they_can_reach`, `trip_must_reach_a_konstant_targets_value`, `trip_must_reach_a_maximum_targets_size`, `konstant_trip_dice_reach_only_their_value`, `mighty_trip_dice_reach_further`, `weak_trip_dice_reach_less_far`, `mood_trip_dice_reach_their_largest_swing_size`, `mood_twin_trip_dice_reach_one_subdies_swing_size`, `a_mood_maximum_target_counts_at_its_smallest_swing_size`, `turbo_trip_sizes_too_small_for_the_target_are_not_offered`, `turbo_trip_is_offered_when_only_a_larger_size_reaches_the_target`, `option_turbo_trip_offers_only_the_side_that_reaches_the_target` |
| Morphing applies to any single-target attack, and only after a successful one | `single_target_berserk_attack_morphs`, `single_target_speed_attack_morphs`, `failed_trip_does_not_morph`, `successful_trip_rolls_at_its_own_size_then_morphs_and_rerolls`, `time_and_space_counts_the_reroll_after_a_trip_morph`, `radioactive_trip_target_decays_the_attacker_after_it_morphs` |
| A Value die scores its current value after every reroll and Focus change | `an_attacking_value_die_scores_its_new_value`, `a_focused_value_die_scores_its_lowered_value`, `every_attack_keeps_the_score_equal_to_the_dice` (Trip Morph and Radioactive rerolls too), `a_chance_reroll_keeps_the_score_equal_to_the_dice`, `a_rerolled_value_die_leaves_the_opponent_a_chance` |

The Trip and Morphing rows intentionally depart from C++, which forbade a
non-Twin Trip against a Twin die and limited Morphing to its 1_1 and N_1
attack types. `parity_trip_morphing_in.txt`'s golden output changed
accordingly in 0.16.0. The Value row departs from C++, which kept an
attacker's pre-reroll score; `Value1_in.txt` and `Value2_in.txt` changed in
0.22.0.

### Rush rule and interaction coverage

Rush is an intentional post-C++ extension based on ButtonWeavers
`BMSkillRush`, `BMAttackRush`, and its parent `BMAttackSpeed` at
`a2d2a1fac12bcffd3453bb0dfe1282b733d23a5b`. The skill has no documented
interactions, so the evidence below maps the description and the attack
validator, then every implemented hook that could see a Rush attack.

| ButtonWeavers behavior | Rust evidence |
|---|---|
| A Rush die captures exactly two dice whose values sum to its value | `rush_die_captures_two_dice_whose_values_sum_to_its_value`, `rush_targets_must_sum_exactly_to_the_attacker_value`, `rush_requires_exactly_two_targets`, `rush_enumerates_every_qualifying_target_pair_once` |
| Any die may Rush when a target is a Rush die; otherwise Rush is illegal | `any_die_may_rush_when_a_target_is_a_rush_die`, `rush_requires_a_rush_attacker_or_target` |
| Speed validation rejects Stealth and Warrior attackers and targets and dizzy attackers | `stealth_dice_cannot_rush_or_be_rushed`, `warrior_dice_cannot_rush_or_be_rushed`, `dizzy_focus_die_cannot_rush`, `insult_and_dizzy_dice_can_be_rushed` |
| Only Power/Skill incompatibilities apply to other skills | `attack_restricted_skills_can_still_rush`, `shadow_rush_die_offers_both_attack_types` |
| Fire assists only Power and Skill; Konstant and Stinger alter only Skill values | `fire_cannot_assist_a_rush_attack`, `stinger_rush_attacker_must_match_the_sum_exactly`, `konstant_rush_attacker_keeps_its_value_and_konstant_targets_are_captured` |
| Berserk, Doppelganger, Morphing, and Radioactive capture hooks require their own type, Power, or a single defender | `berserk_rush_attacker_keeps_berserk_and_its_size`, `doppelganger_radioactive_rush_neither_copies_nor_decays`, `morphing_rush_attacker_does_not_morph_after_two_captures` |
| Generic capture and reroll hooks apply to every Rush participant | Jolt, Time-and-Space, Rage, Null, Value/Poison, Mighty/Weak, Mood/Ornery, Maximum, Twin, and Turbo scenarios in `search::tests::rush` |

ButtonWeavers lists Speed and Rush separately, but a Speed die's two-target
Speed attack has exactly the same legality and resolution as its Rush attack.
BMAIR therefore omits the duplicate Rush candidate for Speed dice
(`speed_rush_die_offers_one_speed_attack_instead_of_a_duplicate_rush`).
Rush candidates are enumerated after Shadow for each attacker, so inputs without
Rush dice keep their C++ candidate order and RNG consumption.

### Boom and Mad rule and interaction coverage

Boom and Mad follow ButtonWeavers `BMAttackBoom`, `BMSkillBoom`, and
`BMSkillMad`. Rows marked "probe" were checked by calling
`BMAttack::commit_attack` in the ButtonWeavers engine under PHP.

| ButtonWeavers behavior | Rust evidence |
|---|---|
| Boom die leaves play unscored and returns next round; the target rerolls in place | `boom_removes_the_boom_die_unscored_and_rerolls_the_target`, `only_boom_dice_can_boom` |
| Documented: Stealth dice may be targeted by Boom | `stealth_dice_may_be_targeted_by_boom_attacks` |
| Stealth, Warrior, and dizzy dice cannot Boom; Warrior dice cannot be boomed; Konstant dice may Boom | `stealth_and_warrior_dice_cannot_boom`, `warrior_dice_cannot_be_boomed`, `dizzy_boom_dice_cannot_boom`, `konstant_boom_dice_may_boom` |
| Probe: the target's reroll applies Konstant, Mighty, Weak, Mad, and Twin, and Value rescores it | `a_konstant_target_keeps_its_value`, `a_mighty_target_grows_on_its_reroll`, `a_weak_target_shrinks_on_its_reroll`, `a_value_target_rescores_after_its_reroll`, `a_mad_target_resizes_to_an_even_size`, `a_twin_target_rerolls_both_halves` |
| Probe: nothing is captured, so Rage, Null and Value Boom dice, Time and Space, and Radioactive never trigger | `a_rage_target_is_not_replaced_because_it_is_not_captured`, `null_and_value_boom_dice_change_nothing_because_nothing_is_captured`, `time_and_space_boom_dice_never_grant_an_extra_turn`, `radioactive_never_decays_on_a_boom` |
| Probe: only a Jolt Boom die grants an extra turn | `only_a_jolt_boom_die_grants_an_extra_turn` |
| Ornery rerolls follow any attack; Fire and Turbo do not apply | `ornery_dice_reroll_after_a_boom`, `fire_cannot_assist_a_boom`, `turbo_boom_dice_offer_no_turbo_sizes` |
| Search returns Boom moves | `search_reports_a_boom_when_it_is_the_only_attack` |
| Mad picks an even size on every reroll; the opening swing size may be odd | `mad_resizes_to_even_sizes_in_its_swing_range`, `a_mad_die_may_start_at_an_odd_size`, `mad_parses_before_or_after_the_swing_size` |
| Documented: Ornery rerolls randomize a Mad die's size | `ornery_rerolls_randomize_a_mad_die` |
| Documented: Radioactive decay removes Mad | `decay_removes_mad` |
| Probe: a Mad Twin shares one size; Konstant blocks the resize; Trip attackers and targets resize | `a_mad_twin_shares_one_size`, `konstant_mad_dice_keep_their_size`, `trip_attackers_and_targets_resize_when_mad` |
| Probe: Mood picks only standard die sizes, including on Chance rerolls | `mood_resizes_to_standard_die_sizes_in_its_swing_range`, `chance_rerolls_resize_mood_dice`, `konstant_ornery_mood_die_keeps_its_size_and_value` |

C++ resizes Mood to any size in range, per Twin half, ignoring Konstant;
ButtonWeavers does not. Boom
candidates are enumerated after Rush, so inputs without Boom dice keep their C++
candidate order.

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
- [x] Morphing/Twin and multi-target Morphing Speed non-effect
  (`parity_trip_morphing_in.txt` plus unit tests). Single-target Speed now
  morphs, as ButtonWeavers does.
- [x] Combined Stealth+Insult precedence, Stinger stack pruning, Null+Value,
  Poison, Queer, Morphing Twin, Time and Space, Ornery, Mood, Mighty and Weak
  seeded game coverage (`parity_combined_mechanics_in.txt`).
- [x] Time and Space extra-turn behavior (combined seeded fixture).
- [x] Mood, Mighty, and Weak RNG/state ordering (combined seeded fixture).
- [x] `min_sims` above `max_sims` follows C++ `ComputeNumberSims` minimum-first
  ordering instead of panicking (`parity_min_sims_exceeds_max_sims_in.txt`).
- [x] Per-player settings reach only that player, unlike C++'s shared AI
  objects (`per_player_ai_settings_in.txt` plus the six-case parser test).

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

- [x] The retired `tests/reference_trace_parity.rs` compared raw RNG streams for a routine
  representative gate and count+FNV fingerprints for every input fixture
  against an instrumented C++ build; the golden files now carry those
  fingerprints. The
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
  retaining ergonomic protocol/public access where needed. `DieIndexSet`
  is a copyable ten-bit value with ascending iteration and protocol mapping.
- [x] Align bounded game/player/die storage with C++ fixed-capacity state where
  practical, or document measured reasons for retaining dynamic storage. Move
  and combination state is fixed-capacity. Player dice remain a `Vec` bounded
  by the C++ protocol's ten-die contract: this is the idiomatic initialized-prefix
  representation, and reusable simulations retain its allocation. A fixed
  `[Option<Die>; 10]` would enlarge/complicate the public model without
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
| fixed `BMC_Game` assignment into one `sim` | `restore_simulation` into one scratch game per evaluator | aligned; same-length dice use direct slice copying, with allocation-retaining `Vec::clone_from` for shape changes |
| `BMC_Move` attacker/target bit arrays | `DieIndexSet(u32)` | aligned; no per-move participant allocation |
| `BMC_DieIndexStack` direct attack walk | fixed `[usize; 10]` `DieIndexStack`, stack-backed available-dice views, and direct outer attacker/attack traversal | aligned; safe bounds replace raw array access and transient index vectors are eliminated |
| cached `m_sides_max` | sum of at most two `u8` sides in `sides_max` | intentionally computed; cheaper invariant surface than synchronizing another field |
| cached attack/vulnerability bits | property branches in `can_do_attack`/`can_be_attacked` | intentionally computed; preserves Stealth's skill-dice-count rule explicitly and avoids stale masks after property mutation |
| cached available/min/max player values | bounded scans or first/last values after exact `optimize_dice` ordering | intentionally computed over at most ten dice; capture/Trip/Chance/Focus already invoke the corresponding optimize points |
| property-presence lookup | bounded `has_available_property`/iterator scans | matches C++ `HasDieWithProperty`, which also scans rather than caching |
| preround/reserve BMAI batches and culling | `select_swing_action`/`select_bmai_reserve_action` plus `Bmai3` evaluator settings | aligned, including static-level quirks and direct scratch restoration |
| Chance/Focus phase recursion | `select_chance_action`, `select_focus_action`, `evaluate_next_initiative_action` | aligned candidate batches, culling, phase transitions, and POV inversion |
| fight BMAI/BMAI3/QAI | direct ordered generation, `evaluate_move`, `play_fight_qai`, `select_qai_action` | aligned simulation lifecycle, ply transition, culling, and RNG order |

## 0.3 integration-boundary revalidation

The JSONL work observes the existing parser/search result; it does not parse
legacy output to reconstruct actions and does not introduce an alternate game
or AI path. `BmairSession::execute` runs `Parser::parse_string` against a
clone and commits that exact state only on success. `Parser::action`
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
  `Parser::parse_stream` now consumes complete commands with `BufRead`
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

## 0.13 selected-move probability reporting

- [x] `report_sims N` is an opt-in Rust-native extension. Its default of zero
  preserves the C++ move-selection budget, candidate ordering, RNG use, action,
  and legacy diagnostic output.
- [x] When enabled in native BMAI fight search, the already-selected move is
  evaluated on a reserved, architecture-stable native stream for exactly `N`
  fresh samples. It does not repartition the move-selection budget or affect
  the selected action.
- [x] JSONL `session.execute` exposes the result as a typed `evaluation` with
  player, probability, simulation count, and source. Legacy output retains its
  historical best-move line for BMAIBagels compatibility.
- [x] `selected_move_report_is_structured_and_does_not_change_the_action`
  reconstructs game 120813, asserts the same action with reporting disabled and
  enabled, and recovers ElihuRoot's 70/30 endgame. The game 120810 regression
  recovers its 50/50 endgame.
- [x] The native mixed-radix stream exhaustively enumerates one or two initial
  bounded rolls whenever the sample count covers their outcome space. The
  reporting path reuses that stream rather than adding a second rules engine;
  `native_strata_enumerate_two_die_outcomes_before_repeating`,
  `ordinary_d10_endgame_preserves_legacy_estimate_and_native_is_exact`, and
  `twin_d6_endgame_uses_the_full_two_die_distribution` cover the mechanism.
  Positions with later conditional randomness remain stratified estimates.
