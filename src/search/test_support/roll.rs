// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::mechanics::parse_game;
use super::*;
use crate::game::RollScheduledDie;
use std::collections::BTreeSet;

pub(crate) struct RollScenario {
    die: String,
    times: usize,
    expected_sizes: Option<BTreeSet<u8>>,
    expected_values: Option<BTreeSet<u16>>,
    expect_twin_halves_match: bool,
}

impl RollScenario {
    pub(super) fn new(die: String) -> Self {
        Self {
            die,
            times: 1,
            expected_sizes: None,
            expected_values: None,
            expect_twin_halves_match: false,
        }
    }

    /// One RNG stream across every reroll, so the draws are independent.
    pub(crate) fn times(mut self, times: usize) -> Self {
        self.times = times;
        self
    }

    pub(crate) fn expect_sizes(mut self, sizes: impl IntoIterator<Item = u8>) -> Self {
        self.expected_sizes = Some(sizes.into_iter().collect());
        self
    }

    pub(crate) fn expect_values(mut self, values: impl IntoIterator<Item = u16>) -> Self {
        self.expected_values = Some(values.into_iter().collect());
        self
    }

    pub(crate) fn expect_twin_halves_match(mut self) -> Self {
        self.expect_twin_halves_match = true;
        self
    }

    #[track_caller]
    pub(crate) fn run(self) {
        let template = parse_game(std::slice::from_ref(&self.die), &["1:1".to_string()]);
        let mut rng = Rng::default();
        let mut sizes = BTreeSet::new();
        let mut values = BTreeSet::new();
        for _ in 0..self.times {
            let mut game = template.clone();
            game.m_player[0].m_die[0].m_notset = true;
            RollScheduledDie(&mut game, 0, 0, &mut rng);
            let die = &game.m_player[0].m_die[0];
            if self.expect_twin_halves_match {
                assert_eq!(die.m_sides[0], die.m_sides[1], "Twin halves differ");
            }
            sizes.insert(die.m_sides[0]);
            values.insert(die.GetValueTotal());
        }
        if let Some(expected) = self.expected_sizes {
            assert_eq!(sizes, expected, "unexpected rerolled sizes of {}", self.die);
        }
        if let Some(expected) = self.expected_values {
            assert_eq!(values, expected, "unexpected rolled values of {}", self.die);
        }
    }
}
