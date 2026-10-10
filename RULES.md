# ButtonWeavers rules coverage

<!--
SPDX-License-Identifier: MIT
SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>
-->

BMAIR plays by ButtonWeavers' rules: the engine in
[danlangford/buttonmen](https://github.com/danlangford/buttonmen)
(`src/engine/`) and its skills page. When the two disagree, BMAIR changes.
This file maps each rule to the code that implements it and the tests that
pin it, and lists the places BMAIR knowingly differs or falls short.

The golden outputs in `tests/golden/` catch unintended changes but prove no
rule; the tests named here do.

## Phases

| Phase or decision | BMAIR support |
|---|---|
| Auxiliary (`aux`) | Parsing, lifecycle, search, and text and typed actions; full matches decide it before round 1. |
| Preround, Reserve, Chance, Focus, Fight | Action selection and simulation. |
| Initiative | Parsed and simulated; it has no `getaction` decision of its own. |
| Game over | Parsed terminal state; it has no `getaction` decision. |
| Turbo selection | Returned as part of a Fight attack rather than as a separate phase. |
| Fire adjustment | Generated and applied with Fight actions; text and typed output give each assisting die's final value. |

ButtonWeavers' server states for joining games, loading buttons, and starting
and ending rounds and turns are not BMAIR decisions. The caller supplies the
resulting position, and BMAIR applies the rules when it simulates.

## Rules

| ButtonWeavers rule | Implementation | Evidence |
|---|---|---|
| Die notation, including a shared `-N` size for every swing half of a Twin | `protocol::legacy::die` | `defined_swing_size_applies_to_every_swing_half_of_a_twin`, `zero_does_not_lock_a_swing_definition`, `forced_win_is_reported_as_certain`, the parser tests, and every fixture |
| Swing and option selection and rolling | `apply_swing_move`, `roll_die` | swing, option, Turbo, and Unique tests |
| Scoring for ordinary, Poison, Value, Null, and Warrior dice | `Die::score` | the score tests in `game::tests` and each skill's scoring scenarios |
| Power, Skill, Speed, Trip, Shadow, and Berserk attacks | `Game::valid_attacks` | the attack, Stealth, and Insult tests |
| Konstant, Stealth, Warrior, Stinger, Unskilled, and Queer attack restrictions | `can_do_attack`, `can_be_attacked`, `skill_stack_can_hit` | the signed Konstant, Stinger, and Warrior matrix and the Stealth and Insult regressions |
| Attacker effects: Berserk, Mighty, Weak, Morphing, Warrior, and Ornery | `apply_attack_player_effects` | the participating and nonparticipating Ornery, Morphing, Twin, Speed, and Warrior tests |
| Reroll effects: nature rerolls, Mood, Mad, and Trip's single before-roll pass | `apply_before_roll_effects`, `apply_mood`, `apply_attacker_nature_roll`, `roll_scheduled_die` | the Trip, Chance, Ornery, Konstant, Mood, and Mad tests |
| Captured Null and Value dice | target mutation in `apply_attack_for_players` before the capture is scored | the property score tests |
| Time and Space extra turns and dizzy recovery | `apply_attack_for_players`, `recover_dizzy_dice` | the Time and Space and Focus tests |
| Jolt attacker consumption and extra turns | `apply_attack_for_players` | the Jolt, Trip, Konstant, multiple-die, and Time and Space tests |
| Doppelganger Power captures and round reset | `apply_attack_player_effects`, `restore_dice_for_new_round` | the Doppelganger section below and `search::tests::doppelganger` |
| Rage initiative, participation, replacement, and round reset | Rage handling in `game::mechanics` | the Rage section below |
| Each round deals every die fresh from the recipe; swing and option selections carry over until a round loser makes them again; reserve dice are offered after the deal | `restore_dice_for_new_round` with the `RoundSelections` that `play_preround_with_policies` returns, and the reserve step of `play_match_with_policies` | next-round scenarios in `search::tests::{berserk,morphing,mighty,weak,warrior,turbo,mad,mood,jolt,rage,null,value,radioactive}`, `null_captures_do_not_tie_every_later_round`, `a_round_losers_option_die_comes_back_at_its_last_choice`, `the_next_round_deals_swing_dice_at_the_sizes_chosen_before_the_fight`, `a_reserve_die_added_after_a_loss_stays_in_play_in_later_rounds`, `the_round_loser_picks_a_reserve_die_from_freshly_dealt_dice` |
| Fire-assisted Power and Skill attacks, with turndowns that last | `FireAdjustment` and Fire candidate expansion | the Fire section below |
| Radioactive decay of the attacker in every one-attacker, one-target attack | `radioactive_decay_applies`, `apply_radioactive_attack_effects`, `split_radioactive_attacker` | the Radioactive section below |
| Rush two-target attacks by or against Rush dice | `Attack::Rush` in `Game::valid_attacks` | the Rush section below |
| Initiative, Focus values, and dizzy dice | `check_initiative` and `search::initiative` | the Konstant Chance and parser initiative tests |
| A Chance reroll gains the initiative only when the roller then holds it alone (`react_to_initiative_chance`) | `apply_chance_move` | `a_chance_reroll_succeeds_only_when_the_roller_wins_initiative` |
| Simultaneous preround choices across swing and option dice, and Unique swing | `search::preround` | the locked swing and option regressions and the Unique tests |
| Auxiliary dice: mutual accept or decline, the courtesy copy, and one decision per full match written into the recipe | `offer_courtesy_auxiliary`, `apply_auxiliary_decision`, `choose_auxiliary_dice` | `search::tests::auxiliary`, `auxiliary_phase_appends_the_courtesy_copy_after_the_receivers_dice`, `a_declined_auxiliary_die_stays_out_of_every_round` |
| Reserve dice after a round loss | the reserve step of `play_match_with_policies` | `complete_native_match_uses_reserve_after_a_round_loss`, `the_round_loser_picks_a_reserve_die_from_freshly_dealt_dice`, `no_reserve_die_is_offered_after_the_round_that_ends_the_match` |
| Round and match standings, ties, and the loser's swing reset | `search::match_play` | `tied_round_has_no_loser`, `a_round_losers_option_die_comes_back_at_its_last_choice` |
| Turbo: each attacking Turbo die that rerolls, has a size to choose, and has not just morphed may choose a size (a Trip asks before its roll); `setTurboSize` resizes that die alone, both halves of a Twin; an option die after Berserk takes one of its option sizes | `turbo_attacker`, `chooses_turbo_size`, `resized_by_turbo`, `expand_turbo_moves`, `trip_reachable_at_some_turbo_size`, and the Turbo step of `apply_attack_player_effects` | `search::tests::turbo`, `twin_turbo_trip_reaches_with_both_halves_resized`, `a_morphing_turbo_die_chooses_a_size_before_it_trips`, `turbo_output_names_the_die_that_attacked` |
| A game is cancelled when its 200th round ends, before that round is scored (`BMGame::do_next_step_end_round`) | `MAX_ROUNDS` in `play_match_with_policies` | `a_match_that_can_only_tie_is_cancelled_at_round_200`, `a_cancelled_match_plays_its_200th_round`, `play_games_writes_each_game_to_its_output`, `playfair_counts_games_cancelled_at_the_round_limit` |

## Defined Twin Swing regression

On 2026-09-02 BMAIBagels sent `(T,T)-2:2`, and BMAIR applied the `-2` to only
the first half, reading the die as `(2,0):2` and halving its score and capture
value before search began. `defined_swing_size_applies_to_every_swing_half_of_a_twin`
covers repeated, distinct, and fixed-and-swing Twin combinations, with Turbo
and Mood on either side of the definition, and
`forced_win_is_reported_as_certain` drives the parser and search at one and
four workers. The fixtures `parity_defined_twin_swing_in.txt` and
`parity_defined_swing_postfix_in.txt` keep the original wire input.

## Doppelganger interaction coverage

The three distinct Doppelganger interaction notes in ButtonWeavers'
`skills.html` are mapped explicitly here so none is hidden inside a generic
mechanics test:

| Documented interaction | Evidence |
|---|---|
| Radioactive decays first; both decay products copy the captured die | `radioactive_doppelganger_decays_before_both_products_copy_the_target` |
| A Doppelganger that captures a Rage die retains Rage after copying it | `doppelganger_that_captures_rage_retains_rage_after_transforming` |
| Copied Turbo does not resize the Doppelganger during the triggering attack | `copied_mighty_grows_but_copied_turbo_does_not_resize` |

## Rage rule and interaction coverage

The live ButtonWeavers skills page describes three core Rage rules and repeats
one named Doppelganger interaction under both skills. Each distinct behavior is
mapped explicitly:

| Documented behavior | Evidence |
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

## Fire rule and interaction coverage

The ButtonWeavers Fire description and interaction metadata are mapped to
explicit scenarios:

| Documented behavior | Evidence |
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
boundary, and both text and typed action output. Fire-assisted attacks
involving an attacking Turbo die use its displayed size (see Known gaps).
Search materializes at most
`max(1, maxbranch / min_sims)` Fire-assisted candidates per state so a large
button cannot exhaust time and memory enumerating allocations before its
configured branch budget applies; `fire_candidate_construction_obeys_the_search_budget`
covers that boundary.

## Radioactive rule and interaction coverage

Source: ButtonWeavers `BMSkillRadioactive`, `BMDie::split`, `BMDieTwin::split`,
`BMAttack::commit_attack`, and the hook order in `BMSkill::skill_order_array`
at `a2d2a1fac12bcffd3453bb0dfe1282b733d23a5b`. Decay comes from the attacker's
`capture` hook or the target's `be_captured` hook, whichever die is
Radioactive. Because the target hook runs after every attacker hook, a
Radioactive attacker decays before Doppelganger copies, while a Doppelganger
copy of a Radioactive target is itself what decays. Cases marked "responder log"
reproduce dice from ButtonWeavers' own `responder0*Test.php` action logs.

| skills.html interaction | Evidence |
|---|---|
| Berserk halves, then decays | `berserk_halves_before_it_decays` (responder log), `radioactive_berserk_attacker_halves_before_it_decays` |
| Doppelganger decays, then each product copies the target | `radioactive_doppelganger_decays_before_each_product_copies_the_target`, `radioactive_doppelganger_decays_before_both_products_copy_the_target`; the Radioactive-target order is `doppelganger_copy_of_a_radioactive_target_decays` (responder log) |
| Mad is lost on decay | `decay_removes_mad` |
| Mood is lost on decay | `decay_removes_mood_so_the_products_keep_their_halved_size` |
| Morphing morphs, then decays | `morphing_attacker_morphs_before_a_radioactive_target_decays_it`; a Radioactive Morphing attacker follows the engine instead: `radioactive_morphing_attacker_decays_into_two_full_size_morphs` |
| Time and Space is lost on decay | `decay_removes_time_and_space_so_an_odd_reroll_grants_no_extra_turn` |
| Turbo is lost on decay | `decay_removes_turbo_and_turbo_sizes_are_not_offered`; Trip keeps its sizes because it rolls first: `turbo_trip_still_offers_sizes_because_it_rolls_before_decaying` |

| Description rule | Evidence |
|---|---|
| The attacker splits into near-equal halves summing to its size | `radioactive_attacker_decays_into_two_halves_that_sum_to_its_size`, `a_one_sided_die_decays_into_a_one_sided_and_a_zero_sided_die` (responder log), `twin_dice_decay_into_alternating_halves` (responder log), `odd_twin_halves_give_each_product_one_rounded_up_subdie` (skills.html example) |
| Either die being Radioactive triggers one decay | `attacker_decays_when_only_the_target_is_radioactive_and_the_target_keeps_radioactive_when_captured`, `two_radioactive_dice_decay_the_attacker_only_once` (responder log) |
| Only attacks with one attacker and one target decay, of any type | `single_die_skill_attack_decays` (responder log), `shadow_attack_decays_and_keeps_shadow` (skills.html example), `single_target_speed_attack_decays`, `multi_target_speed_attack_does_not_decay`, `multi_die_skill_attack_does_not_decay` |
| Involved dice remaining in play lose Radioactive | `failed_trip_still_decays_and_the_surviving_target_loses_radioactive` (responder log), `successful_trip_decays_after_the_trip_roll` (responder log) |
| Decayed dice lose Jolt | `decay_removes_jolt_after_jolt_grants_its_extra_turn` |

Further scenarios cover Konstant and Weak products, Rage on both sides, Null,
scoring, next-round restoration, the dice-pool limit, and search.
Decay products always roll fresh values, including Konstant and Trip
attackers. Mighty and Weak resize the products of ordinary attackers and of a
Doppelganger copy of a Radioactive target, but not Konstant products.

Same-die combinations were settled by running the ButtonWeavers engine
(`BMAttack::commit_attack` under PHP 8.5), because its by-reference attacker
loop contradicts two documented interactions. BMAIR follows the engine:

| Engine probe | Result | Evidence |
|---|---|---|
| `D(20)` captures `H(6)` / `h(12)` / `kH(4)` | `H(8)` / `h(10)` / rerolled `kH(6)` | `copied_mighty_grows_but_copied_turbo_does_not_resize`, `copied_weak_shrinks_on_the_doppelganger_reroll`, `copied_konstant_still_resizes_and_rerolls` |
| `%D(9)` captures `H(4):4` | `H(4):4` never rerolled, plus a rerolled `H(6)` | `radioactive_doppelganger_keeps_the_first_copy_unrolled` |
| `%m(4)` captures `(6,6)` | two full-size `m(6,6)`, not halves | `radioactive_morphing_attacker_decays_into_two_full_size_morphs` |
| `%tm(4)` Trips `(6)` | two full-size `tm(6)` | `radioactive_morphing_trip_decays_into_two_full_size_morphs` |
| `%B(12)` Berserk vs `(6)` | `(3)` and `(3)` | `radioactive_berserk_attacker_halves_before_it_decays` |
| `m(4)` captures `%(10)` | `m(5)` and `m(5)` | `morphing_attacker_morphs_before_a_radioactive_target_decays_it` |

No current button has same-die Radioactive Morphing or Doppelganger dice.
A decay that would exceed the 20-die pool is skipped instead of panicking.

## Button specials and rule corrections

Source: ButtonWeavers `BMBtnSkillUniqueSwing`, `BMBtnSkillGordo`,
`BMBtnSkillLargo`, `BMBtnSkillTheFlyingSquirrel`, `BMAttackSkill::
are_button_skills_compatible` (The Japanese Beetle), `BMBtnSkillGiant` with
`BMGame::is_button_slow`, `BMAttackTrip::validate_attack`, and
`BMSkillMorphing::capture`. Clients name specials with the `special` command
because the wire state carries no button identity.

| ButtonWeavers rule | Evidence |
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

## Rush rule and interaction coverage

Rush follows ButtonWeavers
`BMSkillRush`, `BMAttackRush`, and its parent `BMAttackSpeed` at
`a2d2a1fac12bcffd3453bb0dfe1282b733d23a5b`. The skill has no documented
interactions, so the evidence below maps the description and the attack
validator, then every implemented hook that could see a Rush attack.

| ButtonWeavers behavior | Evidence |
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

## Boom and Mad rule and interaction coverage

Boom and Mad follow ButtonWeavers `BMAttackBoom`, `BMSkillBoom`, and
`BMSkillMad`. Rows marked "probe" were checked by calling
`BMAttack::commit_attack` in the ButtonWeavers engine under PHP.

| ButtonWeavers behavior | Evidence |
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

## Deliberate differences

These are BMAIR choices, not rules, and none changes a move ButtonWeavers
would accept.

| Difference | Why |
|---|---|
| When several Turbo dice make one Skill attack, search offers sizes for the first only; the others keep their size | Offering every combination multiplies candidates (five `Y` Turbo dice give 20⁵). ButtonWeavers needs a size for each attacking Turbo die but accepts the current one. |
| A Speed die's two-target attack is offered once, as Speed, not again as Rush | The two attacks have the same legality and resolution (`speed_rush_die_offers_one_speed_attack_instead_of_a_duplicate_rush`). |
| Available dice are kept ordered by value, ties in their existing order; they keep their end-of-round positions instead of returning to recipe order; and a loser's option dice come back at its last choice | Positions set roll order and candidate indices but no rule. |
| Search builds at most `max(1, maxbranch / min_sims)` Fire-assisted candidates per position | A large button cannot spend the whole budget enumerating allocations (`fire_candidate_construction_obeys_the_search_budget`). |

## Known differences real buttons reach

These predate the move to ButtonWeavers and are queued to fix.

- **One Chance die per action.** ButtonWeavers rerolls a single Chance die
  each time (`react_to_initiative_chance` takes one `rerolledDieIdx`); BMAIR
  offers every subset. FuzzFace, John Kovalic, Pikathulhu, and Ulthar each
  have two Chance dice.
- **Initiative ties.** ButtonWeavers breaks a tie at random
  (`do_next_step_determine_initiative`); BMAIR gives it to seat 0, in
  simulations and full matches alike.

## Known gaps

No current ButtonWeavers button reaches these; each waits for one that does.

- **Mighty or Weak with Turbo.** ButtonWeavers resizes a Turbo die and then
  grows or shrinks the new size on its reroll, except on a round's first turn
  (`pre_roll` skips a die whose value is unset when `turnNumberInRound <= 1`).
  BMAIR grows or shrinks first, then resizes.
- **Fire with an attacking Turbo die.** A Fire plan assumes the Turbo die's
  displayed size, so only that size is offered with Fire.
- **Doppelganger Turbo dice.** A `DX!` attacker gets duplicate Turbo
  candidates and can emit a selection ButtonWeavers would refuse.
- **Mixed Twin Turbo dice.** A `(4,X)!` die is offered no sizes, though
  `BMDieTwin` takes its swing type from either half.
- **Morphing swing type.** A morphed die keeps its own swing type, while
  ButtonWeavers morphs into a copy of the target's, which matters for later
  Turbo choices.
- **Courtesy Option dice.** An accepted Auxiliary Option die never reopens its
  owner's swing choice, because a `Die` cannot tell a chosen option from an
  unchosen one.
- **Chance and Giant.** ButtonWeavers' Chance check ignores button specials,
  so Giant's no-initiative rule does not apply there; BMAIR applies it. Giant
  has no Chance dice.
- **Chaotic.** BMAIR does not implement Chaotic, so Rage gained during a
  Chaotic reroll is untested.
