// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! Measures one engine configuration against another. Every seed is played
//! with each contestant in each seat, so neither gains from a seat or button
//! advantage. The dice are not shared: in legacy mode a search draws from the
//! same generator as the dice, so the two games of a pair diverge at the
//! first search.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

use crate::engines::{Choice, DecisionContext, Engine, Setting};
use crate::game::{Game, Move};
use crate::search::{ChanceMove, Engines, FocusMove, SwingMove, play_match_with_policies};
use crate::{Parser, Rng};

/// An engine and its settings, written as `montecarlo ply=2 cull=off`.
#[derive(Clone, Debug)]
pub struct Contestant {
    spec: String,
    engine: Box<dyn Engine>,
}

impl Contestant {
    pub fn parse(spec: &str) -> Result<Self, String> {
        let mut words = spec.split_whitespace();
        let name = words.next().ok_or("a contestant needs an engine name")?;
        let mut engine = crate::engines::engine(name).ok_or_else(|| {
            format!(
                "unknown engine {name}; choose one of: {}",
                crate::engines::ENGINE_NAMES.join(", ")
            )
        })?;
        for word in words {
            let (setting, value) = word
                .split_once('=')
                .ok_or_else(|| format!("settings are written name=value, not {word}"))?;
            engine.set(Setting::parse(setting, value)?)?;
        }
        Ok(Self {
            spec: spec.split_whitespace().collect::<Vec<_>>().join(" "),
            engine,
        })
    }

    pub fn spec(&self) -> &str {
        &self.spec
    }
}

/// Two buttons, written `label | dice | dice` with dice in BMAIR notation.
#[derive(Clone, Debug)]
pub struct Matchup {
    pub label: String,
    game: Game,
}

impl Matchup {
    pub fn parse(line: &str, target_wins: u8) -> Result<Self, String> {
        let fields = line.split('|').map(str::trim).collect::<Vec<_>>();
        let [label, first, second] = fields[..] else {
            return Err(format!("a matchup is label | dice | dice, not {line}"));
        };
        let mut input = format!("game {target_wins}\npreround\n");
        for (player, dice) in [first, second].into_iter().enumerate() {
            let dice = dice.split_whitespace().collect::<Vec<_>>();
            input.push_str(&format!("player {player} {} 0\n", dice.len()));
            for die in dice {
                input.push_str(die);
                input.push('\n');
            }
        }
        let mut parser = Parser::default();
        parser
            .parse_string(&input, &mut Vec::new())
            .map_err(|error| format!("matchup {label}: {error}"))?;
        Ok(Self {
            label: label.to_string(),
            game: parser.game,
        })
    }
}

#[derive(Clone, Debug)]
pub struct PairingResult {
    pub first: String,
    pub second: String,
    /// Matchup and seed pairs, each played once from each seat.
    pub pairs: usize,
    pub first_wins: usize,
    pub second_wins: usize,
    /// The first contestant's mean score per pair, with a 95% interval.
    pub first_score: f64,
    pub interval: (f64, f64),
    pub first_ms_per_decision: f64,
    pub second_ms_per_decision: f64,
}

pub fn play_pairing(
    first: &Contestant,
    second: &Contestant,
    matchups: &[Matchup],
    seeds: std::ops::RangeInclusive<u32>,
    threads: usize,
) -> PairingResult {
    let units = matchups
        .iter()
        .flat_map(|matchup| seeds.clone().map(move |seed| (matchup, seed)))
        .collect::<Vec<_>>();
    let clocks = [Clock::default(), Clock::default()];
    let next = AtomicUsize::new(0);
    let mut scores = vec![0.0f64; units.len()];
    let mut wins = [0usize; 2];
    std::thread::scope(|scope| {
        let workers = (0..threads.max(1))
            .map(|_| {
                scope.spawn(|| {
                    let timed = |contestant: &Contestant, clock: &Clock| -> Box<dyn Engine> {
                        Box::new(Timed {
                            inner: contestant.engine.clone(),
                            clock: clock.clone(),
                        })
                    };
                    let first = timed(first, &clocks[0]);
                    let second = timed(second, &clocks[1]);
                    let mut results = Vec::new();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some((matchup, seed)) = units.get(index) else {
                            break;
                        };
                        let seated = [
                            [first.clone(), second.clone()],
                            [second.clone(), first.clone()],
                        ];
                        let [as_seat_0, as_seat_1] = seated.map(|engines: Engines| {
                            let mut rng = Rng::default();
                            rng.reseed(game_seed(*seed));
                            play_match_with_policies(&matchup.game, &mut rng, &engines, None).winner
                        });
                        results.push((index, [as_seat_0 == 0, as_seat_1 == 1]));
                    }
                    results
                })
            })
            .collect::<Vec<_>>();
        for worker in workers {
            for (index, first_won) in worker.join().expect("a pairing worker panicked") {
                let first_count = first_won.iter().filter(|won| **won).count();
                wins[0] += first_count;
                wins[1] += 2 - first_count;
                scores[index] = first_count as f64 / 2.0;
            }
        }
    });
    let (first_score, interval) = mean_with_interval(&scores);
    PairingResult {
        first: first.spec.clone(),
        second: second.spec.clone(),
        pairs: units.len(),
        first_wins: wins[0],
        second_wins: wins[1],
        first_score,
        interval,
        first_ms_per_decision: clocks[0].ms_per_decision(),
        second_ms_per_decision: clocks[1].ms_per_decision(),
    }
}

/// Park-Miller seeded with consecutive integers rolls nearly the same opening
/// dice for neighbouring seeds, so each seed is mixed first (murmur3 fmix32).
fn game_seed(seed: u32) -> u32 {
    let mut mixed = seed;
    mixed = (mixed ^ (mixed >> 16)).wrapping_mul(0x85eb_ca6b);
    mixed = (mixed ^ (mixed >> 13)).wrapping_mul(0xc2b2_ae35);
    mixed ^= mixed >> 16;
    // Seed 0 asks the generator for the clock.
    mixed.max(1)
}

/// A normal 95% interval over the paired scores.
fn mean_with_interval(scores: &[f64]) -> (f64, (f64, f64)) {
    if scores.is_empty() {
        return (f64::NAN, (f64::NAN, f64::NAN));
    }
    let n = scores.len() as f64;
    let mean = scores.iter().sum::<f64>() / n;
    if scores.len() < 2 {
        return (mean, (mean, mean));
    }
    let variance = scores
        .iter()
        .map(|score| (score - mean).powi(2))
        .sum::<f64>()
        / (n - 1.0);
    let half_width = 1.96 * (variance / n).sqrt();
    (mean, (mean - half_width, mean + half_width))
}

pub fn markdown_table(results: &[PairingResult]) -> String {
    let mut table = String::from(
        "| First | Second | Pairs | First wins | First score (95% CI) | First ms/decision | Second ms/decision |\n\
         |---|---|---:|---:|---|---:|---:|\n",
    );
    for result in results {
        table.push_str(&format!(
            "| `{}` | `{}` | {} | {}/{} | {:.3} ({:.3}–{:.3}) | {:.2} | {:.2} |\n",
            result.first,
            result.second,
            result.pairs,
            result.first_wins,
            result.first_wins + result.second_wins,
            result.first_score,
            result.interval.0,
            result.interval.1,
            result.first_ms_per_decision,
            result.second_ms_per_decision,
        ));
    }
    table
}

#[derive(Clone, Debug, Default)]
struct Clock {
    nanos: Arc<AtomicU64>,
    decisions: Arc<AtomicU64>,
}

impl Clock {
    fn time<T>(&self, decision: impl FnOnce() -> T) -> T {
        let start = Instant::now();
        let result = decision();
        self.nanos
            .fetch_add(start.elapsed().as_nanos() as u64, Ordering::Relaxed);
        self.decisions.fetch_add(1, Ordering::Relaxed);
        result
    }

    fn ms_per_decision(&self) -> f64 {
        let decisions = self.decisions.load(Ordering::Relaxed);
        if decisions == 0 {
            0.0
        } else {
            self.nanos.load(Ordering::Relaxed) as f64 / 1e6 / decisions as f64
        }
    }
}

/// Times the decisions of the engine it wraps, so the harness can report
/// cost alongside strength without engines knowing they are measured.
#[derive(Clone, Debug)]
struct Timed {
    inner: Box<dyn Engine>,
    clock: Clock,
}

impl Engine for Timed {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn clone_box(&self) -> Box<dyn Engine> {
        Box::new(self.clone())
    }

    fn set(&mut self, setting: Setting) -> Result<(), String> {
        self.inner.set(setting)
    }

    fn montecarlo(&self) -> Option<&crate::Bmai3> {
        self.inner.montecarlo()
    }

    fn swing(
        &self,
        game: &Game,
        player: usize,
        context: &mut DecisionContext<'_, '_>,
    ) -> SwingMove {
        self.clock.time(|| self.inner.swing(game, player, context))
    }

    fn chance(
        &self,
        game: &Game,
        player: usize,
        initiative: usize,
        context: &mut DecisionContext<'_, '_>,
    ) -> ChanceMove {
        self.clock
            .time(|| self.inner.chance(game, player, initiative, context))
    }

    fn focus(
        &self,
        game: &Game,
        player: usize,
        initiative: usize,
        context: &mut DecisionContext<'_, '_>,
    ) -> FocusMove {
        self.clock
            .time(|| self.inner.focus(game, player, initiative, context))
    }

    fn attack(&self, game: &Game, context: &mut DecisionContext<'_, '_>) -> Choice<Move> {
        self.clock.time(|| self.inner.attack(game, context))
    }

    fn reserve(&self, game: &Game, context: &mut DecisionContext<'_, '_>) -> Option<usize> {
        self.clock.time(|| self.inner.reserve(game, context))
    }

    fn auxiliary(
        &self,
        game: &Game,
        context: &mut DecisionContext<'_, '_>,
    ) -> Choice<Option<usize>> {
        self.clock.time(|| self.inner.auxiliary(game, context))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contestants_parse_engines_and_reject_unused_settings() {
        assert_eq!(
            Contestant::parse("montecarlo  ply=2 cull=off")
                .unwrap()
                .spec(),
            "montecarlo ply=2 cull=off"
        );
        assert_eq!(
            Contestant::parse("quick ply=2").unwrap_err(),
            "quick has no ply setting"
        );
        assert_eq!(
            Contestant::parse("montecarlo ply=0").unwrap_err(),
            "montecarlo ply must be at least 1"
        );
        assert!(
            Contestant::parse("expert")
                .unwrap_err()
                .starts_with("unknown engine expert")
        );
    }

    #[test]
    fn swapping_contestants_mirrors_the_result() {
        let quick = Contestant::parse("quick").unwrap();
        let random = Contestant::parse("random").unwrap();
        let matchups = [Matchup::parse("Avis vs Hammer | 4 4 10 12 X | 6 12 20 20 X", 1).unwrap()];
        let forward = play_pairing(&quick, &random, &matchups, 1..=30, 2);
        let backward = play_pairing(&random, &quick, &matchups, 1..=30, 2);
        assert_eq!(forward.first_wins, backward.second_wins);
        assert!((forward.first_score - (1.0 - backward.first_score)).abs() < 1e-9);
        assert!(forward.first_score > 0.5, "{forward:?}");
    }

    #[test]
    fn the_ladder_matchups_parse() {
        let matchups = include_str!("../tests/strength/matchups.txt")
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
            .map(|line| Matchup::parse(line, 1))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(matchups.len(), 6);
    }

    #[test]
    fn a_contestant_against_itself_splits_every_pair_evenly_by_seat() {
        let quick = Contestant::parse("quick").unwrap();
        let matchup = Matchup::parse("mirror | 4 8 12 20 X-10 | 4 8 12 20 X-10", 1).unwrap();
        let result = play_pairing(&quick, &quick, &[matchup], 1..=20, 2);
        assert_eq!(result.pairs, 20);
        assert_eq!(result.first_wins + result.second_wins, 40);
        assert_eq!(result.first_wins, result.second_wins);
        assert!((result.first_score - 0.5).abs() < 1e-9);
    }

    #[test]
    fn the_interval_narrows_as_paired_scores_agree() {
        let (mean, (low, high)) = mean_with_interval(&[1.0, 1.0, 0.5, 1.0]);
        assert!((mean - 0.875).abs() < 1e-9);
        assert!(low < mean && mean < high);
        assert_eq!(mean_with_interval(&[0.5, 0.5]), (0.5, (0.5, 0.5)));
    }
}
