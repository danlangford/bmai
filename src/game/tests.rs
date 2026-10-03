// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[test]
fn cpp_die_index_set_is_bounded_and_iterates_like_bit_array() {
    let indices: BMC_DieIndexSet = [0, 3, 9].into();
    assert_eq!(indices.len(), 3);
    assert_eq!(indices.first(), Some(0));
    assert!(indices.contains(9));
    assert_eq!(indices.iter().collect::<Vec<_>>(), vec![0, 3, 9]);
}

fn die(properties: u64) -> BMC_Die {
    BMC_Die {
        m_properties: property::VALID | properties,
        m_sides: [20, 0],
        m_swing_type: [None, None],
        m_value_total: Some(8),
        m_captured: false,
        m_notset: false,
        m_dizzy: false,
        m_original_index: 0,
        m_in_reserve: false,
    }
}

fn game_with(attacker: Vec<BMC_Die>, target: Vec<BMC_Die>) -> BMC_Game {
    let mut game = BMC_Game::default();
    game.m_player[0].m_die = attacker;
    game.m_player[1].m_die = target;
    game
}

fn attacks(game: &BMC_Game, kind: BME_ATTACK) -> Vec<BMC_Move> {
    game.GenerateValidAttacks()
        .into_iter()
        .filter(|action| action.m_attack == Some(kind))
        .collect()
}

fn valued_die(value: i32, properties: u64, original_index: usize) -> BMC_Die {
    let mut result = die(properties);
    result.m_sides[0] = value.max(1) as u8;
    result.m_value_total = Some(value as u8);
    result.m_original_index = original_index;
    result
}

fn has_skill(game: &BMC_Game, attackers: &[usize], target: usize) -> bool {
    let expected: BMC_DieIndexSet = attackers.iter().copied().collect();
    game.GenerateValidAttacks().iter().any(|action| {
        action.m_attack == Some(BME_ATTACK::SKILL)
            && action.m_attackers == expected
            && action.m_targets.first() == Some(target)
    })
}

#[test]
fn pr82_signed_konstant_skill_attack_matrix() {
    const K: u64 = property::KONSTANT;
    const G: u64 = property::STINGER;
    const W: u64 = property::WARRIOR;
    const M: u64 = property::MAXIMUM;

    struct Case {
        name: &'static str,
        dice: &'static [(i32, u64)],
        target: i32,
        target_properties: u64,
        selected: &'static [usize],
        expected: bool,
    }

    let cases = [
        Case {
            name: "KonstantMultiDieSkillAttackWithSubtraction",
            dice: &[(1, M | K), (1, M | K), (3, M | K)],
            target: 3,
            target_properties: 0,
            selected: &[0, 1, 2],
            expected: true,
        },
        Case {
            name: "KonstantMixedMultiDieSkillAttackWithSubtraction",
            dice: &[(8, 0), (1, M | K)],
            target: 7,
            target_properties: property::STEALTH,
            selected: &[0, 1],
            expected: true,
        },
        Case {
            name: "KonstantMultiDieSkillAttackNoMatchingAssignment",
            dice: &[(1, M | K), (1, M | K), (3, M | K)],
            target: 6,
            target_properties: 0,
            selected: &[0, 1, 2],
            expected: false,
        },
        Case {
            name: "KonstantWarriorCannotSubtractInSkillAttack",
            dice: &[(3, W | K), (5, M | K)],
            target: 2,
            target_properties: property::STEALTH,
            selected: &[0, 1],
            expected: false,
        },
        Case {
            name: "OnlyOneWarriorMayParticipateInSkillAttack",
            dice: &[(5, 0), (2, W | K), (3, W | K)],
            target: 10,
            target_properties: property::STEALTH,
            selected: &[0, 1, 2],
            expected: false,
        },
        Case {
            name: "WarriorCannotAttackWithoutParticipatingNonWarrior",
            dice: &[(5, W)],
            target: 5,
            target_properties: 0,
            selected: &[0],
            expected: false,
        },
        Case {
            name: "KonstantMultiDieSkillAttackWithoutSubtraction",
            dice: &[(1, M | K), (2, M | K)],
            target: 3,
            target_properties: property::STEALTH,
            selected: &[0, 1],
            expected: true,
        },
        Case {
            name: "KonstantSkillAttackWithUnusedStingerInPool",
            dice: &[(5, 0), (2, M | K), (3, M | K), (6, G)],
            target: 4,
            target_properties: 0,
            selected: &[0, 1, 2],
            expected: true,
        },
        Case {
            name: "StingerAndKonstantBothInAttack",
            dice: &[(6, G), (3, M | K)],
            target: 7,
            target_properties: 0,
            selected: &[0, 1],
            expected: true,
        },
        Case {
            name: "StingerAndKonstantWithSubtraction",
            dice: &[(8, G), (5, M | K)],
            target: 3,
            target_properties: 0,
            selected: &[0, 1],
            expected: true,
        },
        Case {
            name: "StingerSkillAttackInRange",
            dice: &[(4, 0), (6, G)],
            target: 7,
            target_properties: 0,
            selected: &[0, 1],
            expected: true,
        },
        Case {
            name: "StingerSkillAttackAtMinimumRange",
            dice: &[(4, 0), (6, G)],
            target: 5,
            target_properties: 0,
            selected: &[0, 1],
            expected: true,
        },
        Case {
            name: "StingerSkillAttackAtMaximumRange",
            dice: &[(4, 0), (6, G)],
            target: 10,
            target_properties: 0,
            selected: &[0, 1],
            expected: true,
        },
        Case {
            name: "StingerSkillAttackBelowRange",
            dice: &[(4, 0), (6, G)],
            target: 4,
            target_properties: 0,
            selected: &[0, 1],
            expected: false,
        },
        Case {
            name: "TwoStingersSkillAttackRange",
            dice: &[(10, G), (10, G)],
            target: 2,
            target_properties: 0,
            selected: &[0, 1],
            expected: true,
        },
        Case {
            name: "TwoStingersCannotHitBelowMinimum",
            dice: &[(10, G), (10, G)],
            target: 1,
            target_properties: 0,
            selected: &[0, 1],
            expected: false,
        },
        Case {
            name: "StingerAtValueOneHasNoFlexibility",
            dice: &[(4, 0), (1, G)],
            target: 5,
            target_properties: 0,
            selected: &[0, 1],
            expected: true,
        },
        Case {
            name: "StingerAtValueOneCannotHitLowerTarget",
            dice: &[(4, 0), (1, G)],
            target: 4,
            target_properties: 0,
            selected: &[0, 1],
            expected: false,
        },
        Case {
            name: "NormalStingerKonstantThreeDieAttack",
            dice: &[(4, 0), (6, G), (3, M | K)],
            target: 5,
            target_properties: 0,
            selected: &[0, 1, 2],
            expected: true,
        },
        Case {
            name: "KonstantWarriorCanAddInSkillAttack",
            dice: &[(5, 0), (3, W | K)],
            target: 8,
            target_properties: 0,
            selected: &[0, 1],
            expected: true,
        },
        Case {
            name: "StingerWarriorMustUseFullValue",
            dice: &[(4, 0), (6, W | G)],
            target: 7,
            target_properties: 0,
            selected: &[0, 1],
            expected: false,
        },
        Case {
            name: "StingerWarriorAtFullValueIsValid",
            dice: &[(4, 0), (6, W | G)],
            target: 10,
            target_properties: 0,
            selected: &[0, 1],
            expected: true,
        },
        Case {
            name: "StingerAndKonstantCombinedFlexibility",
            dice: &[(8, G), (5, M | K)],
            target: 2,
            target_properties: 0,
            selected: &[0, 1],
            expected: true,
        },
        Case {
            name: "StingerKonstantOnSameDieWithSubtraction",
            dice: &[(4, 0), (5, G | K)],
            target: 2,
            target_properties: property::STEALTH,
            selected: &[0, 1],
            expected: true,
        },
        Case {
            name: "StingerKonstantOnSameDieWithAddition",
            dice: &[(4, 0), (5, G | K)],
            target: 6,
            target_properties: property::STEALTH,
            selected: &[0, 1],
            expected: true,
        },
        Case {
            name: "StingerKonstantOnSameDieCannotHitGapBetweenSigns",
            dice: &[(4, 0), (5, G | K)],
            target: 4,
            target_properties: property::STEALTH,
            selected: &[0, 1],
            expected: false,
        },
        Case {
            name: "TwoStingerKonstantDiceCannotHitGapBetweenSignedValues",
            dice: &[(1, G | K), (1, G | K)],
            target: 1,
            target_properties: property::STEALTH,
            selected: &[0, 1],
            expected: false,
        },
        Case {
            name: "StingerWithKonstantWarriorUsesStingerFlexibility",
            dice: &[(6, G), (3, W | K)],
            target: 7,
            target_properties: property::STEALTH,
            selected: &[0, 1],
            expected: true,
        },
        Case {
            name: "StingerWarriorWithKonstantUsesKonstantSubtraction",
            dice: &[(6, W | G), (3, M | K)],
            target: 3,
            target_properties: property::STEALTH,
            selected: &[0, 1],
            expected: true,
        },
        Case {
            name: "StingerKonstantWarriorUsesFullPositiveValue",
            dice: &[(4, 0), (5, W | G | K)],
            target: 9,
            target_properties: property::STEALTH,
            selected: &[0, 1],
            expected: true,
        },
        Case {
            name: "StingerKonstantWarriorCannotUsePartialValue",
            dice: &[(4, 0), (5, W | G | K)],
            target: 6,
            target_properties: property::STEALTH,
            selected: &[0, 1],
            expected: false,
        },
        Case {
            name: "StingerKonstantWarriorCannotSubtract",
            dice: &[(4, 0), (5, W | G | K)],
            target: 2,
            target_properties: property::STEALTH,
            selected: &[0, 1],
            expected: false,
        },
    ];

    for case in cases {
        let attacker = case
            .dice
            .iter()
            .enumerate()
            .map(|(index, &(value, properties))| valued_die(value, properties, index))
            .collect();
        let game = game_with(
            attacker,
            vec![valued_die(case.target, case.target_properties, 0)],
        );
        assert_eq!(
            has_skill(&game, case.selected, 0),
            case.expected,
            "{}",
            case.name
        );
    }

    for target in [10, 4, 6, 0] {
        let game = game_with(
            vec![
                valued_die(5, 0, 0),
                valued_die(2, M | K, 1),
                valued_die(3, M | K, 2),
            ],
            vec![valued_die(target, property::STEALTH, 0)],
        );
        assert!(has_skill(&game, &[0, 1, 2], 0), "signed target {target}");
    }

    let ten = (1..=10)
        .enumerate()
        .map(|(index, value)| valued_die(value, K, index))
        .collect();
    assert!(has_skill(
        &game_with(ten, vec![valued_die(53, 0, 0)]),
        &(0..10).collect::<Vec<_>>(),
        0
    ));

    let game = game_with(
        vec![valued_die(8, 0, 0), valued_die(1, M | K, 1)],
        vec![
            valued_die(8, property::STEALTH, 0),
            valued_die(7, property::STEALTH, 1),
        ],
    );
    assert!(!has_skill(&game, &[0, 1], 0));
    assert!(has_skill(&game, &[0, 1], 1));
}

#[test]
fn cpp_player_copy_constructor_is_independent() {
    let first = BMC_Player {
        m_id: 1,
        ..Default::default()
    };
    let mut second = first.clone();
    assert_eq!(second.m_id, 1);
    second.m_id = 2;
    assert_eq!(second.m_id, 2);
    assert_eq!(first.m_id, 1);
}

#[test]
fn score_matches_cpp_property_branches() {
    assert_eq!(die(0).GetScore(true), 10.0);
    assert_eq!(die(0).GetScore(false), 20.0);
    assert_eq!(die(property::POISON).GetScore(true), -20.0);
    assert_eq!(die(property::POISON).GetScore(false), -10.0);
    assert_eq!(die(property::VALUE).GetScore(true), 4.0);
    assert_eq!(die(property::VALUE).GetScore(false), 8.0);
    assert_eq!(die(property::POISON | property::VALUE).GetScore(true), -8.0);
    assert_eq!(
        die(property::POISON | property::VALUE).GetScore(false),
        -4.0
    );
    assert_eq!(die(property::NULL).GetScore(false), 0.0);
    assert_eq!(die(property::WARRIOR).GetScore(true), 0.0);
}

#[test]
fn turbo_swing_expands_an_attack_to_every_default_accuracy_choice() {
    let mut game = BMC_Game::default();
    let mut turbo = die(property::TURBO);
    turbo.m_sides[0] = 10;
    turbo.m_swing_type[0] = Some('X');
    turbo.m_value_total = Some(10);
    game.m_player[0].m_die = vec![turbo];
    let mut target = die(0);
    target.m_value_total = Some(8);
    game.m_player[1].m_die = vec![target];

    let moves = game.GenerateValidAttacks();
    let mut choices = moves
        .iter()
        .filter(|action| action.m_attack == Some(BME_ATTACK::POWER))
        .map(|action| action.m_turbo_option)
        .collect::<Vec<_>>();
    choices.sort_unstable();
    assert_eq!(choices, (4_i16..=20).collect::<Vec<_>>());
}

#[test]
fn cpp_order_appends_turbo_alternatives_after_every_base_attack() {
    let mut game = BMC_Game::default();
    let mut turbo = die(property::TURBO);
    turbo.m_sides[0] = 10;
    turbo.m_swing_type[0] = Some('X');
    turbo.m_value_total = Some(10);
    let mut ordinary = die(0);
    ordinary.m_value_total = Some(9);
    ordinary.m_original_index = 1;
    game.m_player[0].m_die = vec![turbo, ordinary];
    let mut target = die(0);
    target.m_value_total = Some(8);
    game.m_player[1].m_die = vec![target];

    let moves = game.GenerateValidAttacksInCppOrder();
    let first_alternative = moves
        .iter()
        .position(|action| action.m_turbo_option >= 0 && action.m_turbo_option != 10)
        .unwrap();
    let last_base = moves
        .iter()
        .rposition(|action| action.m_turbo_option < 0 || action.m_turbo_option == 10)
        .unwrap();
    assert!(first_alternative > last_base);
}

#[test]
fn already_legal_power_attack_does_not_offer_unrequested_fire_overshoot() {
    let mut attacker = die(0);
    attacker.m_value_total = Some(8);
    let mut helper = die(property::FIRE);
    helper.m_sides[0] = 6;
    helper.m_value_total = Some(5);
    helper.m_original_index = 1;
    let mut target = die(0);
    target.m_sides[0] = 6;
    target.m_value_total = Some(5);
    let game = game_with(vec![attacker, helper], vec![target]);

    let matching = game
        .GenerateValidAttacks()
        .into_iter()
        .filter(|action| {
            action.m_attack == Some(BME_ATTACK::POWER)
                && action.m_attackers == BMC_DieIndexSet::from([0])
                && action.m_targets == BMC_DieIndexSet::from([0])
        })
        .collect::<Vec<_>>();

    assert_eq!(matching.len(), 1);
    assert!(matching[0].m_fire.is_empty());
}

#[test]
fn optional_overshoots_cannot_exhaust_the_required_fire_budget() {
    let mut attacker = die(0);
    attacker.m_sides[0] = 10;
    attacker.m_value_total = Some(5);
    let mut helper = die(property::FIRE);
    helper.m_sides[0] = 20;
    helper.m_value_total = Some(20);
    helper.m_original_index = 1;
    let mut high_target = die(0);
    high_target.m_value_total = Some(8);
    let mut low_target = die(0);
    low_target.m_value_total = Some(4);
    low_target.m_original_index = 1;
    let mut game = game_with(vec![attacker, helper], vec![high_target, low_target]);
    game.m_fire_overshooting = true;

    let assisted = game
        .GenerateValidAttacksInCppOrderForSearch(1)
        .into_iter()
        .filter(|action| !action.m_fire.is_empty())
        .collect::<Vec<_>>();

    assert_eq!(assisted.len(), 1);
    assert_eq!(assisted[0].m_targets, BMC_DieIndexSet::from([0]));
}

#[test]
fn fire_plan_is_not_reused_for_a_different_turbo_option_size() {
    let mut turbo = die(property::TURBO | property::OPTION);
    turbo.m_sides = [20, 6];
    turbo.m_value_total = Some(10);
    let mut helper = die(property::FIRE);
    helper.m_sides[0] = 6;
    helper.m_value_total = Some(3);
    helper.m_original_index = 1;
    let mut target = die(0);
    target.m_value_total = Some(12);
    let game = game_with(vec![turbo, helper], vec![target]);

    let assisted = game
        .GenerateValidAttacksInCppOrder()
        .into_iter()
        .filter(|action| action.m_attack == Some(BME_ATTACK::POWER) && !action.m_fire.is_empty())
        .collect::<Vec<_>>();

    assert!(!assisted.is_empty());
    assert!(assisted.iter().all(|action| action.m_turbo_option == 0));
}

#[test]
fn fire_candidate_construction_obeys_the_search_budget() {
    let mut attacker = die(0);
    attacker.m_value_total = Some(1);
    let mut dice = vec![attacker];
    for original_index in 1..10 {
        let mut helper = die(property::FIRE);
        helper.m_value_total = Some(20);
        helper.m_original_index = original_index;
        dice.push(helper);
    }
    let mut target = die(0);
    target.m_sides[0] = 200;
    target.m_value_total = Some(50);
    let game = game_with(dice, vec![target]);

    let moves = game.GenerateValidAttacksInCppOrderForSearch(3);
    assert_eq!(
        moves
            .iter()
            .filter(|action| !action.m_fire.is_empty())
            .count(),
        3
    );
}

#[test]
fn copied_cpp_skill_restrictions_match_konstant_and_stealth_cases() {
    let target = die(0);

    let mut konstant_game = BMC_Game::default();
    let mut konstant = die(property::KONSTANT);
    konstant.m_value_total = Some(8);
    konstant_game.m_player[0].m_die = vec![konstant];
    konstant_game.m_player[1].m_die = vec![target];
    assert!(konstant_game.GenerateValidAttacks().is_empty());

    let mut stealth_game = BMC_Game::default();
    let mut stealth = die(property::STEALTH);
    stealth.m_value_total = Some(7);
    stealth_game.m_player[0].m_die = vec![stealth];
    stealth_game.m_player[1].m_die = vec![target];
    assert!(stealth_game.GenerateValidAttacks().is_empty());

    let mut ordinary = die(0);
    ordinary.m_original_index = 1;
    ordinary.m_value_total = Some(1);
    stealth_game.m_player[0].m_die = vec![stealth, ordinary];
    assert!(stealth_game.GenerateValidAttacks().iter().any(|action| {
        action.m_attack == Some(BME_ATTACK::SKILL) && action.m_attackers.len() == 2
    }));
}

#[test]
fn cpp_basic_power_and_skill_attack_generation() {
    let mut a = die(0);
    a.m_sides[0] = 9;
    let mut t = die(0);
    t.m_sides[0] = 7;
    t.m_value_total = Some(6);
    let game = game_with(vec![a], vec![t]);
    assert_eq!(attacks(&game, BME_ATTACK::POWER).len(), 1);

    let mut five = die(0);
    five.m_sides[0] = 6;
    five.m_value_total = Some(5);
    let mut one = five;
    one.m_value_total = Some(1);
    one.m_original_index = 1;
    let mut twenty = die(0);
    twenty.m_value_total = Some(6);
    let game = game_with(vec![five, one], vec![twenty]);
    let skill = attacks(&game, BME_ATTACK::SKILL);
    assert_eq!(skill.len(), 1);
    assert_eq!(skill[0].m_attackers, vec![0, 1]);

    let mut six = die(0);
    six.m_sides[0] = 6;
    six.m_value_total = Some(6);
    let mut twenty = die(0);
    twenty.m_value_total = Some(6);
    let game = game_with(vec![six], vec![twenty]);
    assert_eq!(attacks(&game, BME_ATTACK::POWER).len(), 1);
    assert_eq!(attacks(&game, BME_ATTACK::SKILL).len(), 1);

    let mut konstant = die(property::KONSTANT);
    konstant.m_value_total = Some(6);
    let mut target = die(0);
    target.m_value_total = Some(6);
    assert!(
        game_with(vec![konstant], vec![target])
            .GenerateValidAttacks()
            .is_empty()
    );
}

#[test]
fn cpp_insult_and_stealth_restrictions() {
    let insult = die(property::INSULT);
    assert!(insult.CanBeAttacked(BME_ATTACK::POWER, 1));
    assert!(!insult.CanBeAttacked(BME_ATTACK::SKILL, 2));
    let stealth_insult = die(property::STEALTH | property::INSULT);
    assert!(stealth_insult.CanBeAttacked(BME_ATTACK::SKILL, 2));

    for active in [property::TRIP, property::SHADOW, property::BERSERK] {
        let stealth = die(property::STEALTH | active);
        assert!(stealth.CanDoAttack(BME_ATTACK::SKILL, 2));
        assert!(!stealth.CanDoAttack(BME_ATTACK::POWER, 1));
        assert!(!stealth.CanDoAttack(BME_ATTACK::TRIP, 1));
        assert!(!stealth.CanDoAttack(BME_ATTACK::SHADOW, 1));
        assert!(!stealth.CanDoAttack(BME_ATTACK::BERSERK, 1));
    }

    let mut ordinary = die(0);
    ordinary.m_value_total = Some(6);
    let mut stealth_target = die(property::STEALTH);
    stealth_target.m_value_total = Some(6);
    assert!(
        game_with(vec![ordinary], vec![stealth_target])
            .GenerateValidAttacks()
            .is_empty()
    );
    let mut second = ordinary;
    second.m_original_index = 1;
    second.m_value_total = Some(1);
    ordinary.m_value_total = Some(5);
    assert_eq!(
        attacks(
            &game_with(vec![ordinary, second], vec![stealth_target]),
            BME_ATTACK::SKILL
        )
        .len(),
        1
    );
}

#[test]
fn pr82_variable_skill_stack_disables_legacy_value_pruning() {
    let mut stinger = die(property::STINGER);
    stinger.m_value_total = Some(5);
    let mut three = die(0);
    three.m_value_total = Some(3);
    three.m_original_index = 1;
    let mut two = die(0);
    two.m_value_total = Some(2);
    two.m_original_index = 2;
    let mut target = die(0);
    target.m_value_total = Some(8);

    let skill = attacks(
        &game_with(vec![stinger, three, two], vec![target]),
        BME_ATTACK::SKILL,
    );
    assert!(skill.iter().any(|action| action.m_attackers.len() == 2));
    assert!(skill.iter().any(|action| action.m_attackers.len() == 3));
}

#[test]
fn cpp_speed_generation_and_property_score_combinations() {
    let mut speed = die(property::SPEED);
    speed.m_sides[0] = 10;
    speed.m_value_total = Some(8);
    let mut four = die(0);
    four.m_sides[0] = 4;
    four.m_value_total = Some(3);
    let mut six = die(0);
    six.m_sides[0] = 6;
    six.m_value_total = Some(5);
    six.m_original_index = 1;
    let game = game_with(vec![speed], vec![four, six]);
    let speed_attacks = attacks(&game, BME_ATTACK::SPEED);
    assert_eq!(speed_attacks.len(), 1);
    assert_eq!(speed_attacks[0].m_targets, vec![0, 1]);

    let mut scored = die(0);
    scored.m_sides[0] = 30;
    assert_eq!(
        (scored.GetScore(true), scored.GetScore(false)),
        (15.0, 30.0)
    );
    scored.m_properties |= property::MORPHING | property::MAXIMUM;
    assert_eq!(
        (scored.GetScore(true), scored.GetScore(false)),
        (15.0, 30.0)
    );
    scored.m_properties |= property::VALUE;
    assert_eq!((scored.GetScore(true), scored.GetScore(false)), (4.0, 8.0));
    scored.m_properties |= property::POISON;
    assert_eq!(
        (scored.GetScore(true), scored.GetScore(false)),
        (-8.0, -4.0)
    );
    scored.m_properties |= property::NULL;
    assert_eq!((scored.GetScore(true), scored.GetScore(false)), (0.0, 0.0));
}
