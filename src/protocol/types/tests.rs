// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[test]
fn capabilities_have_stable_protocol_names_and_serialize() {
    assert_eq!(ProtocolVersion::LegacyV1.as_str(), "legacy-v1");
    assert_eq!(ProtocolVersion::JsonlV1.as_str(), "jsonl-v1");

    let value = serde_json::to_value(Capabilities::current()).unwrap();
    assert_eq!(value["implementation"], "bmair");
    let engines = value["engines"].as_array().unwrap();
    let names = engines
        .iter()
        .map(|engine| engine["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(names, crate::engines::ENGINE_NAMES);
    assert_eq!(
        engines[3]["settings"],
        serde_json::json!(["ply", "max_sims", "min_sims", "maxbranch", "cull"])
    );
    assert_eq!(value["protocols"][0], "legacy-v1");
    assert_eq!(value["protocols"][1], "jsonl-v1");
    assert_eq!(value["native"]["automatic_workers"], true);
    assert!(
        value["commands"]
            .as_array()
            .unwrap()
            .contains(&"getaction".into())
    );
    assert_eq!(
        value["commands"],
        serde_json::json!([
            "mode",
            "rng",
            "workers",
            "game",
            "player",
            "ai",
            "ply",
            "max_sims",
            "min_sims",
            "maxbranch",
            "cull",
            "report_sims",
            "turbo_accuracy",
            "fire_overshooting",
            "special",
            "surrender",
            "getaction",
            "playgame",
            "playfair",
            "compare",
            "seed",
            "debugply",
            "debug",
            "quit"
        ])
    );
    assert!(
        value["skills"]
            .as_array()
            .unwrap()
            .contains(&"Konstant".into())
    );
    assert_eq!(
        value["actions"],
        serde_json::json!([
            "attack",
            "auxiliary",
            "chance",
            "focus",
            "pass",
            "reserve",
            "set_swing",
            "surrender"
        ])
    );
    assert!(
        value["skills"]
            .as_array()
            .unwrap()
            .contains(&"Auxiliary".into())
    );
    assert_eq!(
        value["attack_types"],
        serde_json::json!([
            "power", "skill", "berserk", "speed", "trip", "shadow", "rush", "boom"
        ])
    );
    assert_eq!(value["parsing_only_skills"], serde_json::json!([]));
    assert_eq!(
        value["button_specials"][3],
        serde_json::json!({
            "id": "skill_immune",
            "buttons": ["The Japanese Beetle"],
            "rule": "Cannot be attacked by skill attacks."
        })
    );
}

#[test]
fn die_notation_is_complete_unique_and_machine_readable() {
    use std::collections::HashSet;

    use crate::protocol::notation::CapabilitySupport;

    let capabilities = Capabilities::current();
    let tokens: Vec<_> = capabilities
        .die_notation
        .property_prefixes
        .iter()
        .map(|entry| entry.token)
        .collect();
    assert_eq!(
        tokens,
        vec![
            '^', 'q', 't', 'z', 's', 'B', 'd', 'p', 'n', 'f', 'H', 'h', 'r', 'o', 'c', 'm', '`',
            'w', 'u', '~', 'g', 'k', 'M', 'I', 'v', 'J', 'F', '+', 'D', '%', 'G', '#', 'b'
        ]
    );
    assert_eq!(
        tokens.iter().copied().collect::<HashSet<_>>().len(),
        tokens.len()
    );
    let implemented_skills: HashSet<_> = capabilities.skills.iter().copied().collect();
    let parsing_only_skills: HashSet<_> =
        capabilities.parsing_only_skills.iter().copied().collect();
    for entry in capabilities.die_notation.property_prefixes {
        let advertised = match entry.support {
            CapabilitySupport::Implemented => &implemented_skills,
            CapabilitySupport::ParsingOnly => &parsing_only_skills,
        };
        assert!(
            advertised.contains(entry.name),
            "{} is absent from its compatibility skill list",
            entry.name
        );
    }
    assert!(parsing_only_skills.is_empty());
    assert!(implemented_skills.contains("Radioactive"));
    for entry in capabilities.die_notation.postfix_properties {
        assert!(implemented_skills.contains(entry.name));
    }

    let value = serde_json::to_value(capabilities).unwrap();
    let prefixes = value["die_notation"]["property_prefixes"]
        .as_array()
        .unwrap();
    assert_eq!(
        prefixes.iter().find(|entry| entry["token"] == "d").unwrap(),
        &serde_json::json!({
            "token": "d",
            "id": "stealth",
            "name": "Stealth",
            "support": "implemented"
        })
    );
    assert_eq!(
        prefixes.iter().find(|entry| entry["token"] == "G").unwrap(),
        &serde_json::json!({
            "token": "G",
            "id": "rage",
            "name": "Rage",
            "support": "implemented"
        })
    );
    assert_eq!(
        prefixes.iter().find(|entry| entry["token"] == "#").unwrap(),
        &serde_json::json!({
            "token": "#",
            "id": "rush",
            "name": "Rush",
            "support": "implemented"
        })
    );
    assert_eq!(
        prefixes.iter().find(|entry| entry["token"] == "F").unwrap(),
        &serde_json::json!({
            "token": "F",
            "id": "fire",
            "name": "Fire",
            "support": "implemented"
        })
    );
    assert_eq!(
        value["die_notation"]["postfix_properties"],
        serde_json::json!([
            {"token": "!", "id": "turbo", "name": "Turbo"},
            {"token": "?", "id": "mood", "name": "Mood"},
            {"token": "&", "id": "mad", "name": "Mad"}
        ])
    );
    assert_eq!(value["die_notation"]["swing_types"], "P-Z");
    assert_eq!(value["die_notation"]["option_separator"], "/");
    assert_eq!(value["die_notation"]["rolled_value_separator"], ":");
    assert_eq!(value["die_notation"]["dizzy_value_suffix"], "d");
}

#[test]
fn every_typed_action_shape_has_a_stable_discriminator() {
    let actions = [
        serde_json::to_value(ProtocolAction::Pass).unwrap(),
        serde_json::to_value(ProtocolAction::Surrender).unwrap(),
        serde_json::to_value(ProtocolAction::Auxiliary { die: Some(1) }).unwrap(),
        serde_json::to_value(ProtocolAction::Attack {
            attack_type: "skill",
            attackers: vec![0, 2],
            targets: vec![1],
            turbo: Some(TurboSelection::Swing {
                swing: 'X',
                value: 12,
            }),
            fire: Vec::new(),
        })
        .unwrap(),
        serde_json::to_value(ProtocolAction::Reserve { die: None }).unwrap(),
        serde_json::to_value(ProtocolAction::SetSwing {
            swings: vec![SwingSelection {
                swing: 'X',
                value: 12,
            }],
            options: vec![OptionSelection { die: 1, value: 20 }],
        })
        .unwrap(),
        serde_json::to_value(ProtocolAction::Chance { dice: vec![0, 2] }).unwrap(),
        serde_json::to_value(ProtocolAction::Focus {
            dice: vec![FocusSelection { die: 0, value: 4 }],
        })
        .unwrap(),
    ];
    assert_eq!(
        actions
            .iter()
            .map(|action| action["type"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "pass",
            "surrender",
            "auxiliary",
            "attack",
            "reserve",
            "set_swing",
            "chance",
            "focus"
        ]
    );
    assert_eq!(actions[2]["die"], 1);
    assert_eq!(actions[3]["turbo"]["kind"], "swing");
}

#[test]
fn fire_turndowns_are_an_additive_attack_field() {
    let ordinary = serde_json::to_value(ProtocolAction::Attack {
        attack_type: "power",
        attackers: vec![0],
        targets: vec![0],
        turbo: None,
        fire: Vec::new(),
    })
    .unwrap();
    assert!(ordinary.get("fire").is_none());

    let assisted = serde_json::to_value(ProtocolAction::Attack {
        attack_type: "power",
        attackers: vec![0],
        targets: vec![0],
        turbo: None,
        fire: vec![FireSelection { die: 1, value: 1 }],
    })
    .unwrap();
    assert_eq!(
        assisted["fire"],
        serde_json::json!([{"die": 1, "value": 1}])
    );
}

#[test]
fn protocol_floats_preserve_non_finite_legacy_settings_without_invalid_json() {
    assert_eq!(
        serde_json::to_value(ProtocolFloat::from_f32(0.5)).unwrap(),
        0.5
    );
    assert_eq!(
        serde_json::to_value(ProtocolFloat::from_f32(f32::NAN)).unwrap(),
        "nan"
    );
    assert_eq!(
        serde_json::to_value(ProtocolFloat::from_f32(f32::INFINITY)).unwrap(),
        "infinity"
    );
}

#[test]
fn probability_estimate_has_a_stable_structured_shape() {
    let value = serde_json::to_value(ProbabilityEstimate {
        player: 0,
        probability: ProtocolFloat::Finite(0.7),
        simulations: 1000,
        source: "selected_move_resample",
    })
    .unwrap();
    assert_eq!(value["player"], 0);
    assert_eq!(value["simulations"], 1000);
    assert_eq!(value["source"], "selected_move_resample");
    assert!((value["probability"].as_f64().unwrap() - 0.7).abs() < 1e-6);
}
