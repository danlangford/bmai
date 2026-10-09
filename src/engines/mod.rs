// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! Every AI implements [`Engine`], so match play, the legacy protocol, and the
//! strength harness pick engines by name instead of matching on each one.

pub(crate) mod maximize;
mod montecarlo;
pub(crate) mod quick;
pub(crate) mod random;

pub(crate) use maximize::Maximize;
pub(crate) use montecarlo::MonteCarlo;
pub(crate) use quick::Quick;
pub(crate) use random::Random;

use crate::Rng;
use crate::game::{Game, Move};
use crate::search::{ChanceMove, FocusMove, NativeEvaluation, NativeReplaySequence, SwingMove};

/// Fixed, so Monte Carlo's search settings never change how these engines
/// play.
pub(crate) const SIMPLE_FIRE_CANDIDATES: usize = 500;

/// The names `ai PLAYER NAME` accepts, in the order capabilities list them.
pub const ENGINE_NAMES: [&str; 4] = ["random", "maximize", "quick", "montecarlo"];

/// Capabilities list these; a test checks `MonteCarlo::set` accepts exactly them.
pub(crate) const MONTECARLO_SETTINGS: &[&str] = &[
    "ply",
    "max_sims",
    "min_sims",
    "maxbranch",
    "cull",
    "playout",
    "endgame",
];

pub(crate) fn engine(name: &str) -> Option<Box<dyn Engine>> {
    match name {
        "random" => Some(Box::new(Random)),
        "maximize" => Some(Box::new(Maximize)),
        "quick" => Some(Box::new(Quick)),
        "montecarlo" => Some(Box::new(MonteCarlo::default())),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Setting {
    Ply(usize),
    MaxSims(usize),
    MinSims(usize),
    MaxBranch(usize),
    Cull(bool),
    Playout(crate::Playout),
    Endgame(usize),
}

impl Setting {
    /// Parses `name=value`, as harness specs write settings.
    pub fn parse(name: &str, value: &str) -> Result<Self, String> {
        let number = || {
            value
                .parse::<usize>()
                .map_err(|_| format!("{name} needs a whole number, not {value}"))
        };
        match name {
            "ply" => number().map(Self::Ply),
            "max_sims" => number().map(Self::MaxSims),
            "min_sims" => number().map(Self::MinSims),
            "maxbranch" => number().map(Self::MaxBranch),
            "cull" => match value {
                "on" => Ok(Self::Cull(true)),
                "off" => Ok(Self::Cull(false)),
                _ => Err(format!("cull needs on or off, not {value}")),
            },
            "endgame" => value
                .parse()
                .map(Self::Endgame)
                .map_err(|_| format!("endgame needs a dice count, not {value}")),
            "playout" => match value {
                "quick" => Ok(Self::Playout(crate::Playout::Quick)),
                "maximize" => Ok(Self::Playout(crate::Playout::Maximize)),
                "random" => Ok(Self::Playout(crate::Playout::Random)),
                _ => Err(format!(
                    "playout needs quick, maximize, or random, not {value}"
                )),
            },
            _ => Err(format!("unknown setting {name}")),
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Ply(_) => "ply",
            Self::MaxSims(_) => "max_sims",
            Self::MinSims(_) => "min_sims",
            Self::MaxBranch(_) => "maxbranch",
            Self::Cull(_) => "cull",
            Self::Playout(_) => "playout",
            Self::Endgame(_) => "endgame",
        }
    }
}

/// A decision an engine made, with the search figures the protocol reports
/// when the engine searched.
#[derive(Clone, Debug)]
pub(crate) struct Choice<T> {
    pub choice: T,
    pub search: Option<SearchSummary>,
}

impl<T> Choice<T> {
    pub(crate) const fn unsearched(choice: T) -> Self {
        Self {
            choice,
            search: None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SearchSummary {
    pub score: f32,
    pub win_probability: f32,
    pub simulations: usize,
}

/// Engines decide for `player`, or for player 0 of an already oriented game.
pub(crate) trait Engine: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &'static str;
    fn clone_box(&self) -> Box<dyn Engine>;

    /// Rejects settings the engine does not use, so experiments cannot
    /// silently run with a setting that did nothing.
    fn set(&mut self, setting: Setting) -> Result<(), String> {
        Err(format!("{} has no {} setting", self.name(), setting.name()))
    }

    /// The Monte Carlo search settings, for engines that have them.
    fn montecarlo(&self) -> Option<&crate::Bmai3> {
        None
    }

    fn swing(&self, game: &Game, player: usize, context: &mut DecisionContext<'_, '_>)
    -> SwingMove;
    fn chance(
        &self,
        game: &Game,
        player: usize,
        initiative: usize,
        context: &mut DecisionContext<'_, '_>,
    ) -> ChanceMove;
    fn focus(
        &self,
        game: &Game,
        player: usize,
        initiative: usize,
        context: &mut DecisionContext<'_, '_>,
    ) -> FocusMove;
    fn attack(&self, game: &Game, context: &mut DecisionContext<'_, '_>) -> Choice<Move>;
    fn reserve(&self, game: &Game, context: &mut DecisionContext<'_, '_>) -> Option<usize>;
    fn auxiliary(
        &self,
        game: &Game,
        context: &mut DecisionContext<'_, '_>,
    ) -> Choice<Option<usize>>;
}

impl Clone for Box<dyn Engine> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// The randomness a decision may draw on. Native replay keys are issued only
/// when an engine asks for one, so engines that never search leave the
/// native decision index where it was.
pub(crate) struct DecisionContext<'a, 'b> {
    pub(crate) rng: &'a mut Rng,
    native: Option<&'a mut NativeReplaySequence<'b>>,
    issued: Option<crate::native::NativeReplayKey>,
}

impl<'a, 'b> DecisionContext<'a, 'b> {
    pub(crate) fn new(rng: &'a mut Rng, native: Option<&'a mut NativeReplaySequence<'b>>) -> Self {
        Self {
            rng,
            native,
            issued: None,
        }
    }

    /// Only playout engines, which never search, decide without a sequence.
    pub(crate) fn native(&mut self) -> NativeEvaluation {
        let evaluation = self
            .native
            .as_deref_mut()
            .map(NativeReplaySequence::next)
            .expect("a searching engine decides only where a replay sequence exists");
        self.issued = Some(evaluation.replay);
        evaluation
    }

    /// The replay key this decision used, if it searched natively.
    pub(crate) const fn issued_replay(&self) -> Option<crate::native::NativeReplayKey> {
        self.issued
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(name: &str) -> Setting {
        let value = match name {
            "cull" => "off",
            "playout" => "random",
            _ => "2",
        };
        Setting::parse(name, value).unwrap()
    }

    #[test]
    fn each_engine_accepts_exactly_the_settings_capabilities_list() {
        for name in ENGINE_NAMES {
            let listed: &[&str] = if name == "montecarlo" {
                MONTECARLO_SETTINGS
            } else {
                &[]
            };
            for setting in MONTECARLO_SETTINGS {
                let accepted = engine(name).unwrap().set(sample(setting)).is_ok();
                assert_eq!(accepted, listed.contains(setting), "{name} {setting}");
            }
        }
    }
}
