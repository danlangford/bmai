// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[derive(Default)]
pub(crate) struct ParserScenario {
    pub(super) input: String,
    pub(super) expected_action: ActionExpectation,
}

impl ParserScenario {
    pub(crate) fn expect_pass(mut self) -> Self {
        self.expected_action.pass();
        self
    }

    pub(crate) fn expect_surrender(mut self) -> Self {
        self.expected_action.surrender();
        self
    }

    pub(crate) fn expect_attack(mut self, attack: Attack) -> Self {
        self.expected_action.attack(attack);
        self
    }

    pub(crate) fn expect_auxiliary(mut self, die: Option<usize>) -> Self {
        self.expected_action.action = Some(ExpectedAction::Auxiliary(die));
        self
    }

    pub(crate) fn using(mut self, dice: impl IntoIterator<Item = usize>) -> Self {
        self.expected_action.attackers = Some(dice.into_iter().collect());
        self
    }

    pub(crate) fn targeting(mut self, dice: impl IntoIterator<Item = usize>) -> Self {
        self.expected_action.targets = Some(dice.into_iter().collect());
        self
    }

    #[track_caller]
    pub(crate) fn run(self) {
        let expected = self
            .expected_action
            .protocol_action()
            .expect("parser scenario has no expected action");
        let mut parser = Parser::default();
        let mut output = Vec::new();
        parser
            .parse_string(&self.input, &mut output)
            .unwrap_or_else(|error| panic!("invalid parser scenario: {error}\n{}", self.input));
        assert_eq!(
            parser.last_action(),
            Some(&expected),
            "unexpected action for parser scenario:\n{}\n{}",
            self.input,
            String::from_utf8_lossy(&output)
        );
        let expected_wire = legacy_action_suffix(&expected);
        let output = String::from_utf8(output).expect("protocol output must be UTF-8");
        assert!(
            output.ends_with(&expected_wire),
            "parser scenario did not emit {expected_wire:?}:\n{output}"
        );
    }
}

pub(super) fn legacy_action_suffix(action: &ProtocolAction) -> String {
    match action {
        ProtocolAction::Pass => "action\npass\n".into(),
        ProtocolAction::Surrender => "action\nsurrender\n".into(),
        ProtocolAction::Auxiliary { die } => {
            let die = die.map_or_else(|| "-1".into(), |value| value.to_string());
            format!("action\naux {die}\n")
        }
        ProtocolAction::Attack {
            attack_type,
            attackers,
            targets,
            turbo: None,
            fire,
        } => format!(
            "action\n{attack_type}\n{}\n{}\n{}",
            joined_indices(attackers),
            joined_indices(targets),
            fire.iter()
                .map(|selection| format!("fire {} {}\n", selection.die, selection.value))
                .collect::<String>()
        ),
        _ => panic!("parser scenario cannot yet render {action:?}"),
    }
}

fn joined_indices(indices: &[usize]) -> String {
    indices
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}
