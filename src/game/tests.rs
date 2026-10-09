// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[test]
fn cpp_die_index_set_is_bounded_and_iterates_like_bit_array() {
    let indices: DieIndexSet = [0, 3, 9].into();
    assert_eq!(indices.len(), 3);
    assert_eq!(indices.first(), Some(0));
    assert!(indices.contains(9));
    assert_eq!(indices.iter().collect::<Vec<_>>(), vec![0, 3, 9]);
}

fn die(properties: u64) -> Die {
    Die {
        properties: property::VALID | properties,
        sides: [20, 0],
        swing_type: [None, None],
        value: Some(8),
        captured: false,
        not_set: false,
        dizzy: false,
        original_index: 0,
        in_reserve: false,
    }
}

fn game_with(attacker: Vec<Die>, target: Vec<Die>) -> Game {
    let mut game = Game::default();
    game.players[0].dice = attacker;
    game.players[1].dice = target;
    game
}

fn attacks(game: &Game, kind: Attack) -> Vec<Move> {
    game.valid_attacks_by_score()
        .into_iter()
        .filter(|action| action.attack == Some(kind))
        .collect()
}

fn valued_die(value: i32, properties: u64, original_index: usize) -> Die {
    let mut result = die(properties);
    result.sides[0] = value.max(1) as u8;
    result.value = Some(value as u8);
    result.original_index = original_index;
    result
}

fn has_skill(game: &Game, attackers: &[usize], target: usize) -> bool {
    let expected: DieIndexSet = attackers.iter().copied().collect();
    game.valid_attacks_by_score().iter().any(|action| {
        action.attack == Some(Attack::Skill)
            && action.attackers == expected
            && action.targets.first() == Some(target)
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
    let first = Player {
        id: 1,
        ..Default::default()
    };
    let mut second = first.clone();
    assert_eq!(second.id, 1);
    second.id = 2;
    assert_eq!(second.id, 2);
    assert_eq!(first.id, 1);
}

#[test]
fn score_matches_cpp_property_branches() {
    assert_eq!(die(0).score(true), 10.0);
    assert_eq!(die(0).score(false), 20.0);
    assert_eq!(die(property::POISON).score(true), -20.0);
    assert_eq!(die(property::POISON).score(false), -10.0);
    assert_eq!(die(property::VALUE).score(true), 4.0);
    assert_eq!(die(property::VALUE).score(false), 8.0);
    assert_eq!(die(property::POISON | property::VALUE).score(true), -8.0);
    assert_eq!(die(property::POISON | property::VALUE).score(false), -4.0);
    assert_eq!(die(property::NULL).score(false), 0.0);
    assert_eq!(die(property::WARRIOR).score(true), 0.0);
}

#[test]
fn turbo_swing_expands_an_attack_to_every_default_accuracy_choice() {
    let mut game = Game::default();
    let mut turbo = die(property::TURBO);
    turbo.sides[0] = 10;
    turbo.swing_type[0] = Some('X');
    turbo.value = Some(10);
    game.players[0].dice = vec![turbo];
    let mut target = die(0);
    target.value = Some(8);
    game.players[1].dice = vec![target];

    let moves = game.valid_attacks_by_score();
    let mut choices = moves
        .iter()
        .filter(|action| action.attack == Some(Attack::Power))
        .map(|action| action.turbo_option)
        .collect::<Vec<_>>();
    choices.sort_unstable();
    assert_eq!(choices, (4_i16..=20).collect::<Vec<_>>());
}

#[test]
fn generation_order_appends_turbo_alternatives_after_every_base_attack() {
    let mut game = Game::default();
    let mut turbo = die(property::TURBO);
    turbo.sides[0] = 10;
    turbo.swing_type[0] = Some('X');
    turbo.value = Some(10);
    let mut ordinary = die(0);
    ordinary.value = Some(9);
    ordinary.original_index = 1;
    game.players[0].dice = vec![turbo, ordinary];
    let mut target = die(0);
    target.value = Some(8);
    game.players[1].dice = vec![target];

    let moves = game.valid_attacks(usize::MAX);
    let first_alternative = moves
        .iter()
        .position(|action| action.turbo_option >= 0 && action.turbo_option != 10)
        .unwrap();
    let last_base = moves
        .iter()
        .rposition(|action| action.turbo_option < 0 || action.turbo_option == 10)
        .unwrap();
    assert!(first_alternative > last_base);
}

#[test]
fn already_legal_power_attack_does_not_offer_unrequested_fire_overshoot() {
    let mut attacker = die(0);
    attacker.value = Some(8);
    let mut helper = die(property::FIRE);
    helper.sides[0] = 6;
    helper.value = Some(5);
    helper.original_index = 1;
    let mut target = die(0);
    target.sides[0] = 6;
    target.value = Some(5);
    let mut game = game_with(vec![attacker, helper], vec![target]);
    game.fire_overshooting = false;

    let matching = game
        .valid_attacks_by_score()
        .into_iter()
        .filter(|action| {
            action.attack == Some(Attack::Power)
                && action.attackers == DieIndexSet::from([0])
                && action.targets == DieIndexSet::from([0])
        })
        .collect::<Vec<_>>();

    assert_eq!(matching.len(), 1);
    assert!(matching[0].fire.is_empty());
}

#[test]
fn optional_overshoots_cannot_exhaust_the_required_fire_budget() {
    let mut attacker = die(0);
    attacker.sides[0] = 10;
    attacker.value = Some(5);
    let mut helper = die(property::FIRE);
    helper.sides[0] = 20;
    helper.value = Some(20);
    helper.original_index = 1;
    let mut high_target = die(0);
    high_target.value = Some(8);
    let mut low_target = die(0);
    low_target.value = Some(4);
    low_target.original_index = 1;
    let mut game = game_with(vec![attacker, helper], vec![high_target, low_target]);
    game.fire_overshooting = true;

    let assisted = game
        .valid_attacks(1)
        .into_iter()
        .filter(|action| !action.fire.is_empty())
        .collect::<Vec<_>>();

    assert_eq!(assisted.len(), 1);
    assert_eq!(assisted[0].targets, DieIndexSet::from([0]));
}

#[test]
fn fire_plan_is_not_reused_for_a_different_turbo_option_size() {
    let mut turbo = die(property::TURBO | property::OPTION);
    turbo.sides = [20, 6];
    turbo.value = Some(10);
    let mut helper = die(property::FIRE);
    helper.sides[0] = 6;
    helper.value = Some(3);
    helper.original_index = 1;
    let mut target = die(0);
    target.value = Some(12);
    let game = game_with(vec![turbo, helper], vec![target]);

    let assisted = game
        .valid_attacks(usize::MAX)
        .into_iter()
        .filter(|action| action.attack == Some(Attack::Power) && !action.fire.is_empty())
        .collect::<Vec<_>>();

    assert!(!assisted.is_empty());
    assert!(assisted.iter().all(|action| action.turbo_option == 0));
}

#[test]
fn fire_candidate_construction_obeys_the_search_budget() {
    let mut attacker = die(0);
    attacker.value = Some(1);
    let mut dice = vec![attacker];
    for original_index in 1..10 {
        let mut helper = die(property::FIRE);
        helper.value = Some(20);
        helper.original_index = original_index;
        dice.push(helper);
    }
    let mut target = die(0);
    target.sides[0] = 200;
    target.value = Some(50);
    let game = game_with(dice, vec![target]);

    let moves = game.valid_attacks(3);
    assert_eq!(
        moves
            .iter()
            .filter(|action| !action.fire.is_empty())
            .count(),
        3
    );
}

#[test]
fn copied_cpp_skill_restrictions_match_konstant_and_stealth_cases() {
    let target = die(0);

    let mut konstant_game = Game::default();
    let mut konstant = die(property::KONSTANT);
    konstant.value = Some(8);
    konstant_game.players[0].dice = vec![konstant];
    konstant_game.players[1].dice = vec![target];
    assert!(konstant_game.valid_attacks_by_score().is_empty());

    let mut stealth_game = Game::default();
    let mut stealth = die(property::STEALTH);
    stealth.value = Some(7);
    stealth_game.players[0].dice = vec![stealth];
    stealth_game.players[1].dice = vec![target];
    assert!(stealth_game.valid_attacks_by_score().is_empty());

    let mut ordinary = die(0);
    ordinary.original_index = 1;
    ordinary.value = Some(1);
    stealth_game.players[0].dice = vec![stealth, ordinary];
    assert!(
        stealth_game
            .valid_attacks_by_score()
            .iter()
            .any(|action| { action.attack == Some(Attack::Skill) && action.attackers.len() == 2 })
    );
}

#[test]
fn cpp_basic_power_and_skill_attack_generation() {
    let mut a = die(0);
    a.sides[0] = 9;
    let mut t = die(0);
    t.sides[0] = 7;
    t.value = Some(6);
    let game = game_with(vec![a], vec![t]);
    assert_eq!(attacks(&game, Attack::Power).len(), 1);

    let mut five = die(0);
    five.sides[0] = 6;
    five.value = Some(5);
    let mut one = five;
    one.value = Some(1);
    one.original_index = 1;
    let mut twenty = die(0);
    twenty.value = Some(6);
    let game = game_with(vec![five, one], vec![twenty]);
    let skill = attacks(&game, Attack::Skill);
    assert_eq!(skill.len(), 1);
    assert_eq!(skill[0].attackers, vec![0, 1]);

    let mut six = die(0);
    six.sides[0] = 6;
    six.value = Some(6);
    let mut twenty = die(0);
    twenty.value = Some(6);
    let game = game_with(vec![six], vec![twenty]);
    assert_eq!(attacks(&game, Attack::Power).len(), 1);
    assert_eq!(attacks(&game, Attack::Skill).len(), 1);

    let mut konstant = die(property::KONSTANT);
    konstant.value = Some(6);
    let mut target = die(0);
    target.value = Some(6);
    assert!(
        game_with(vec![konstant], vec![target])
            .valid_attacks_by_score()
            .is_empty()
    );
}

#[test]
fn cpp_insult_and_stealth_restrictions() {
    let insult = die(property::INSULT);
    assert!(insult.can_be_attacked(Attack::Power, 1));
    assert!(!insult.can_be_attacked(Attack::Skill, 2));
    let stealth_insult = die(property::STEALTH | property::INSULT);
    assert!(stealth_insult.can_be_attacked(Attack::Skill, 2));

    for active in [property::TRIP, property::SHADOW, property::BERSERK] {
        let stealth = die(property::STEALTH | active);
        assert!(stealth.can_do_attack(Attack::Skill, 2));
        assert!(!stealth.can_do_attack(Attack::Power, 1));
        assert!(!stealth.can_do_attack(Attack::Trip, 1));
        assert!(!stealth.can_do_attack(Attack::Shadow, 1));
        assert!(!stealth.can_do_attack(Attack::Berserk, 1));
    }

    let mut ordinary = die(0);
    ordinary.value = Some(6);
    let mut stealth_target = die(property::STEALTH);
    stealth_target.value = Some(6);
    assert!(
        game_with(vec![ordinary], vec![stealth_target])
            .valid_attacks_by_score()
            .is_empty()
    );
    let mut second = ordinary;
    second.original_index = 1;
    second.value = Some(1);
    ordinary.value = Some(5);
    assert_eq!(
        attacks(
            &game_with(vec![ordinary, second], vec![stealth_target]),
            Attack::Skill
        )
        .len(),
        1
    );
}

#[test]
fn pr82_variable_skill_stack_disables_legacy_value_pruning() {
    let mut stinger = die(property::STINGER);
    stinger.value = Some(5);
    let mut three = die(0);
    three.value = Some(3);
    three.original_index = 1;
    let mut two = die(0);
    two.value = Some(2);
    two.original_index = 2;
    let mut target = die(0);
    target.value = Some(8);

    let skill = attacks(
        &game_with(vec![stinger, three, two], vec![target]),
        Attack::Skill,
    );
    assert!(skill.iter().any(|action| action.attackers.len() == 2));
    assert!(skill.iter().any(|action| action.attackers.len() == 3));
}

#[test]
fn cpp_speed_generation_and_property_score_combinations() {
    let mut speed = die(property::SPEED);
    speed.sides[0] = 10;
    speed.value = Some(8);
    let mut four = die(0);
    four.sides[0] = 4;
    four.value = Some(3);
    let mut six = die(0);
    six.sides[0] = 6;
    six.value = Some(5);
    six.original_index = 1;
    let game = game_with(vec![speed], vec![four, six]);
    let speed_attacks = attacks(&game, Attack::Speed);
    assert_eq!(speed_attacks.len(), 1);
    assert_eq!(speed_attacks[0].targets, vec![0, 1]);

    let mut scored = die(0);
    scored.sides[0] = 30;
    assert_eq!((scored.score(true), scored.score(false)), (15.0, 30.0));
    scored.properties |= property::MORPHING | property::MAXIMUM;
    assert_eq!((scored.score(true), scored.score(false)), (15.0, 30.0));
    scored.properties |= property::VALUE;
    assert_eq!((scored.score(true), scored.score(false)), (4.0, 8.0));
    scored.properties |= property::POISON;
    assert_eq!((scored.score(true), scored.score(false)), (-8.0, -4.0));
    scored.properties |= property::NULL;
    assert_eq!((scored.score(true), scored.score(false)), (0.0, 0.0));
}
