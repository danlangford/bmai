// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

#[derive(Clone, Copy, Debug)]
pub(crate) enum SearchMode {
    Legacy,
    LegacyWithWorkers { workers: usize },
    Native { workers: Option<usize> },
}

pub(crate) const LEGACY: SearchMode = SearchMode::Legacy;
pub(crate) const NATIVE: SearchMode = SearchMode::Native { workers: None };

pub(crate) const fn legacy_with_workers(workers: usize) -> SearchMode {
    SearchMode::LegacyWithWorkers { workers }
}

pub(crate) const fn native(workers: usize) -> SearchMode {
    SearchMode::Native {
        workers: Some(workers),
    }
}

#[derive(Default)]
pub(crate) struct SearchScenario {
    phase: Option<Phase>,
    target_wins: Option<usize>,
    players: [Option<(f32, Vec<String>)>; 2],
    max_ply: Option<usize>,
    min_sims: Option<usize>,
    max_sims: Option<usize>,
    max_branch: Option<usize>,
    surrender: Option<bool>,
    fire_overshooting: Option<bool>,
    expected_player: Option<usize>,
    expected_win_percent: Option<RangeInclusive<f32>>,
    expected_action: ActionExpectation,
    modes: Vec<SearchMode>,
}

impl SearchScenario {
    pub(crate) fn phase(mut self, phase: Phase) -> Self {
        self.phase = Some(phase);
        self
    }

    pub(crate) fn target_wins(mut self, target_wins: usize) -> Self {
        self.target_wins = Some(target_wins);
        self
    }

    pub(crate) fn player(
        mut self,
        player: usize,
        score: f32,
        dice: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        assert!(player < 2, "player index must be 0 or 1");
        self.players[player] = Some((score, dice.into_iter().map(Into::into).collect()));
        self
    }

    pub(crate) fn ply(mut self, max_ply: usize) -> Self {
        self.max_ply = Some(max_ply);
        self
    }

    pub(crate) fn simulations(mut self, min: usize, max: usize) -> Self {
        self.min_sims = Some(min);
        self.max_sims = Some(max);
        self
    }

    pub(crate) fn max_branch(mut self, max_branch: usize) -> Self {
        self.max_branch = Some(max_branch);
        self
    }

    pub(crate) fn surrender(mut self, allowed: bool) -> Self {
        self.surrender = Some(allowed);
        self
    }

    pub(crate) fn fire_overshooting(mut self, enabled: bool) -> Self {
        self.fire_overshooting = Some(enabled);
        self
    }

    pub(crate) fn modes(mut self, modes: impl IntoIterator<Item = SearchMode>) -> Self {
        self.modes = modes.into_iter().collect();
        self
    }

    /// A range keeps seeded statistical scenarios honest about uncertainty.
    pub(crate) fn expect_player_win_percent(
        mut self,
        player: usize,
        expected: RangeInclusive<f32>,
    ) -> Self {
        self.expected_player = Some(player);
        self.expected_win_percent = Some(expected);
        self
    }

    pub(crate) fn expect_pass(mut self) -> Self {
        self.expected_action.pass();
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

    pub(crate) fn firing(mut self, dice: impl IntoIterator<Item = (usize, u8)>) -> Self {
        self.expected_action.fire = dice
            .into_iter()
            .map(|(die, value)| FireSelection { die, value })
            .collect();
        self
    }

    #[track_caller]
    pub(crate) fn run(self) {
        assert!(
            self.expected_win_percent.is_some() || self.expected_action.action.is_some(),
            "search scenario has no expectation"
        );
        assert!(!self.modes.is_empty(), "search scenario has no modes");

        for mode in &self.modes {
            let input = self.protocol_input(*mode);
            let mut parser = Parser::default();
            let mut output = Vec::new();
            parser
                .parse_string(&input, &mut output)
                .unwrap_or_else(|error| panic!("invalid search scenario: {error}\n{input}"));
            let output = String::from_utf8(output).expect("protocol output must be UTF-8");
            if let Some(expected) = &self.expected_win_percent {
                let player = self
                    .expected_player
                    .expect("win percentage expectation has no player");
                assert_eq!(player, 0, "getaction reports the active player 0");
                let actual = reported_win_percent(&output).unwrap_or_else(|| {
                    panic!("search scenario reported no win percentage:\n{output}")
                });
                assert!(
                    expected.contains(&actual),
                    "{mode:?} player {player} win percentage {actual} was outside {expected:?}\n{output}"
                );
            }
            if let Some(expected) = self.expected_action.protocol_action() {
                assert_eq!(
                    parser.last_action(),
                    Some(&expected),
                    "{mode:?} selected an unexpected action:\n{output}"
                );
                let expected_wire = legacy_action_suffix(&expected);
                assert!(
                    output.ends_with(&expected_wire),
                    "{mode:?} did not emit {expected_wire:?}:\n{output}"
                );
            }
        }
    }

    fn protocol_input(&self, mode: SearchMode) -> String {
        let phase = self.phase.unwrap_or(Phase::Fight);
        let target_wins = self.target_wins.unwrap_or(3);
        let mut input = String::new();
        match mode {
            SearchMode::Legacy => {}
            SearchMode::LegacyWithWorkers { workers } => {
                input.push_str(&format!("workers {workers}\n"));
            }
            SearchMode::Native { workers } => {
                input.push_str("mode native\n");
                if let Some(workers) = workers {
                    input.push_str(&format!("workers {workers}\n"));
                }
            }
        }
        input.push_str(&format!("game {target_wins}\n{}\n", phase_name(phase)));
        for player in 0..2 {
            let (score, dice) = self.players[player]
                .as_ref()
                .unwrap_or_else(|| panic!("search scenario has no player {player}"));
            input.push_str(&format!("player {player} {} {score}\n", dice.len()));
            for die in dice {
                input.push_str(die);
                input.push('\n');
            }
        }
        if let Some(value) = self.max_ply {
            input.push_str(&format!("ply {value}\n"));
        }
        if let Some(value) = self.max_sims {
            input.push_str(&format!("max_sims {value}\n"));
        }
        if let Some(value) = self.min_sims {
            input.push_str(&format!("min_sims {value}\n"));
        }
        if let Some(value) = self.max_branch {
            input.push_str(&format!("maxbranch {value}\n"));
        }
        if let Some(allowed) = self.surrender {
            input.push_str(if allowed {
                "surrender on\n"
            } else {
                "surrender off\n"
            });
        }
        if let Some(enabled) = self.fire_overshooting {
            input.push_str(if enabled {
                "fire_overshooting on\n"
            } else {
                "fire_overshooting off\n"
            });
        }
        input.push_str("getaction\nquit\n");
        input
    }
}

fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Auxiliary => "aux",
        Phase::Preround => "preround",
        Phase::Initiative => "initiative",
        Phase::Chance => "chance",
        Phase::Focus => "focus",
        Phase::Fight => "fight",
        Phase::Reserve => "reserve",
        Phase::Gameover => "gameover",
    }
}

fn reported_win_percent(output: &str) -> Option<f32> {
    output.lines().find_map(|line| {
        let (_, percent) = line.strip_prefix("l1 p0 best move (")?.rsplit_once(", ")?;
        percent.strip_suffix("% win)")?.parse().ok()
    })
}
