// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! Measures one button against a field of opponents. One engine plays both
//! seats and every seed is played with the button in each seat, so the result
//! reflects the buttons rather than the AI or the seat.

use std::ops::RangeInclusive;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::Rng;
use crate::engines::Engine;
use crate::search::{Engines, NativeReplaySequence, play_match_with_policies};
use crate::strength::{Contestant, Matchup, game_seed, mean_with_interval, parse_game};

pub const DEFAULT_ENGINE: &str = "montecarlo";
pub const DEFAULT_GAMES: usize = 500;
const TARGET_WINS: u8 = 3;

/// Established buttons between 40% and 60% on ButtonWeavers, suggested on its
/// forum as a balance benchmark for new buttons.
pub const DEFAULT_FIELD: &str = "\
Lucky: (6) (10) p(12) (20) (X)
Vincent: (30) (30) (30) (6/30)
Sailor Jupiter: (6) (10) (12) (20) r(6) r(12) r(12) r(20)
Hammer: (6) (12) (20) (20) (X)
Monkeys: z(6) z(6) z(6) z(10) z(T)
Lady K: (4) s(6) (8) s(8) (X)
Konami: (6) (8) f(10) f(10) (X)
Wolfman: (6) p(10) (12) z(16) (X)
";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Button {
    pub name: String,
    pub recipe: String,
}

/// Reads `Name: recipe` lines, skipping blank lines and `#` or `//` comments.
pub fn parse_field(text: &str) -> Result<Vec<Button>, String> {
    let mut field = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let (name, recipe) = line
            .split_once(':')
            .map(|(name, recipe)| (name.trim(), recipe.trim()))
            .filter(|(name, recipe)| !name.is_empty() && !recipe.is_empty())
            .ok_or_else(|| format!("line {}: expected Name: recipe, not {line}", index + 1))?;
        field.push(Button {
            name: name.to_owned(),
            recipe: recipe.to_owned(),
        });
    }
    if field.is_empty() {
        return Err("the field has no opponents".into());
    }
    Ok(field)
}

#[derive(Debug)]
pub struct Gauntlet {
    engine: Box<dyn Engine>,
    opponents: Vec<Seats>,
}

#[derive(Debug)]
struct Seats {
    button_first: Matchup,
    button_second: Matchup,
}

impl Gauntlet {
    /// Checks every recipe up front, so a typo fails before any match is played.
    pub fn new(recipe: &str, field: &[Button], contestant: Contestant) -> Result<Self, String> {
        parse_game(recipe, recipe, TARGET_WINS).map_err(|error| format!("the button: {error}"))?;
        let opponents = field
            .iter()
            .map(|opponent| {
                Ok(Seats {
                    button_first: Matchup::new(
                        &opponent.name,
                        recipe,
                        &opponent.recipe,
                        TARGET_WINS,
                    )?,
                    button_second: Matchup::new(
                        &opponent.name,
                        &opponent.recipe,
                        recipe,
                        TARGET_WINS,
                    )?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self {
            engine: contestant.engine,
            opponents,
        })
    }

    pub fn opponents(&self) -> usize {
        self.opponents.len()
    }

    pub fn play(&self, opponent: usize, seeds: RangeInclusive<u32>, threads: usize) -> Record {
        let seats = &self.opponents[opponent];
        let seeds = seeds.collect::<Vec<_>>();
        let next = AtomicUsize::new(0);
        let mut pairs = vec![Pair::default(); seeds.len()];
        std::thread::scope(|scope| {
            let workers = (0..threads.max(1))
                .map(|_| {
                    scope.spawn(|| {
                        let engines: Engines = [self.engine.clone(), self.engine.clone()];
                        let mut results = Vec::new();
                        loop {
                            let index = next.fetch_add(1, Ordering::Relaxed);
                            let Some(&seed) = seeds.get(index) else {
                                break;
                            };
                            results.push((index, play_pair(seats, seed, &engines)));
                        }
                        results
                    })
                })
                .collect::<Vec<_>>();
            for worker in workers {
                for (index, pair) in worker.join().expect("a gauntlet worker panicked") {
                    pairs[index] = pair;
                }
            }
        });
        Record {
            opponent: seats.button_first.label.clone(),
            games: 2 * pairs.len(),
            wins: pairs.iter().map(|pair| pair.wins).sum(),
            rounds: [0, 1].map(|side| pairs.iter().map(|pair| pair.rounds[side]).sum()),
            pair_scores: pairs.iter().map(|pair| pair.wins as f64 / 2.0).collect(),
        }
    }
}

fn play_pair(seats: &Seats, seed: u32, engines: &Engines) -> Pair {
    let play = |matchup: &Matchup| {
        let mut rng = Rng::default();
        rng.reseed(game_seed(seed));
        let mut decision_index = 0;
        // Matches already fill every core, so each match's search stays on one.
        let mut native = NativeReplaySequence {
            algorithm: rng.algorithm(),
            root_seed: u64::from(game_seed(seed)),
            workers: 1,
            decision_index: &mut decision_index,
        };
        play_match_with_policies(&matchup.game, &mut rng, engines, Some(&mut native))
    };
    let first = play(&seats.button_first);
    let second = play(&seats.button_second);
    Pair {
        wins: usize::from(first.winner == 0) + usize::from(second.winner == 1),
        rounds: [
            usize::from(first.wins[0] + second.wins[1]),
            usize::from(first.wins[1] + second.wins[0]),
        ],
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Pair {
    wins: usize,
    rounds: [usize; 2],
}

#[derive(Clone, Debug)]
pub struct Record {
    pub opponent: String,
    pub games: usize,
    pub wins: usize,
    /// Rounds the button won and lost. Tied rounds are replayed, so they are not counted.
    pub rounds: [usize; 2],
    pair_scores: Vec<f64>,
}

impl Record {
    pub fn total(records: &[Record]) -> Record {
        Record {
            opponent: "Overall".into(),
            games: records.iter().map(|record| record.games).sum(),
            wins: records.iter().map(|record| record.wins).sum(),
            rounds: [0, 1].map(|side| records.iter().map(|record| record.rounds[side]).sum()),
            pair_scores: records
                .iter()
                .flat_map(|record| record.pair_scores.iter().copied())
                .collect(),
        }
    }

    pub fn win_rate(&self) -> f64 {
        mean_with_interval(&self.pair_scores).0
    }

    /// A 95% interval over the seat-swapped pairs.
    pub fn interval(&self) -> (f64, f64) {
        let (low, high) = mean_with_interval(&self.pair_scores).1;
        (low.max(0.0), high.min(1.0))
    }

    pub fn round_rate(&self) -> f64 {
        self.rounds[0] as f64 / (self.rounds[0] + self.rounds[1]) as f64
    }
}

/// Plain text that lines up in a terminal and survives a paste into a forum.
#[derive(Clone, Copy, Debug)]
pub struct Table {
    name_width: usize,
}

impl Table {
    pub fn new(field: &[Button]) -> Self {
        let name_width = field
            .iter()
            .map(|button| button.name.chars().count())
            .chain(["Opponent".len()])
            .max()
            .unwrap_or_default();
        Self { name_width }
    }

    pub fn header(&self) -> String {
        self.line("Opponent", "Won", "Win %", "95% CI", "Rounds won")
    }

    pub fn row(&self, record: &Record) -> String {
        let (low, high) = record.interval();
        self.line(
            &record.opponent,
            &format!("{}/{}", record.wins, record.games),
            &percent(record.win_rate()),
            &format!("{:.1}-{:.1}%", 100.0 * low, 100.0 * high),
            &percent(record.round_rate()),
        )
    }

    fn line(&self, name: &str, won: &str, rate: &str, interval: &str, rounds: &str) -> String {
        let width = self.name_width;
        format!("{name:<width$}  {won:<11}  {rate:>6}  {interval:<11}  {rounds:>10}")
            .trim_end()
            .to_owned()
    }
}

fn percent(rate: f64) -> String {
    if rate.is_nan() {
        "-".into()
    } else {
        format!("{:.1}%", 100.0 * rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn button(name: &str, recipe: &str) -> Button {
        Button {
            name: name.into(),
            recipe: recipe.into(),
        }
    }

    #[test]
    fn the_default_field_parses_and_builds() {
        let field = parse_field(DEFAULT_FIELD).unwrap();
        assert_eq!(field.len(), 8);
        assert_eq!(
            field[2],
            button(
                "Sailor Jupiter",
                "(6) (10) (12) (20) r(6) r(12) r(12) r(20)"
            )
        );
        let engine = Contestant::parse(DEFAULT_ENGINE).unwrap();
        let gauntlet = Gauntlet::new("dk(1) k(V) k(V) k(V) dmMH(4)", &field, engine).unwrap();
        assert_eq!(gauntlet.opponents(), 8);
    }

    #[test]
    fn fields_skip_comments_and_report_bad_lines() {
        let field = parse_field("# heading\n\n// note\nAvis: 4 4 10 12 X\n").unwrap();
        assert_eq!(field, [button("Avis", "4 4 10 12 X")]);
        assert_eq!(
            parse_field("Avis: 4 4 10 12 X\nHammer 6 12 20 20 X").unwrap_err(),
            "line 2: expected Name: recipe, not Hammer 6 12 20 20 X"
        );
        assert_eq!(
            parse_field("# only\n").unwrap_err(),
            "the field has no opponents"
        );
    }

    #[test]
    fn a_bad_recipe_names_its_owner() {
        let quick = || Contestant::parse("quick").unwrap();
        let broken = [button("Broken", "(6) y(7)")];
        let error = Gauntlet::new("(6) (X)", &broken, quick()).unwrap_err();
        assert!(error.starts_with("matchup Broken: "), "{error}");
        let field = [button("Fine", "(6) (X)")];
        let error = Gauntlet::new("(6) y(7)", &field, quick()).unwrap_err();
        assert!(error.starts_with("the button: "), "{error}");
    }

    #[test]
    fn a_stronger_button_wins_most_games_from_either_seat() {
        let quick = || Contestant::parse("quick").unwrap();
        let big = "(20) (20) (20) (20) (20)";
        let tiny = "(1) (1) (2) (2) (4)";
        let strong = Gauntlet::new(big, &[button("Tiny", tiny)], quick())
            .unwrap()
            .play(0, 1..=10, 1);
        let weak = Gauntlet::new(tiny, &[button("Big", big)], quick())
            .unwrap()
            .play(0, 1..=10, 1);
        assert!(strong.wins >= 16, "{strong:?}");
        assert!(weak.wins <= 4, "{weak:?}");
        assert!(strong.rounds[0] > strong.rounds[1], "{strong:?}");
        assert!(weak.rounds[0] < weak.rounds[1], "{weak:?}");
    }

    #[test]
    fn a_button_needs_dice() {
        let quick = || Contestant::parse("quick").unwrap();
        let field = [button("Fine", "(6) (X)")];
        for recipe in ["", "   "] {
            let error = Gauntlet::new(recipe, &field, quick()).unwrap_err();
            assert!(error.starts_with("the button: "), "{error}");
        }
    }

    #[test]
    fn a_button_against_itself_wins_exactly_half() {
        let recipe = "(4) (8) (12) (20) (X)";
        let quick = Contestant::parse("quick").unwrap();
        let gauntlet = Gauntlet::new(recipe, &[button("Mirror", recipe)], quick).unwrap();
        let record = gauntlet.play(0, 1..=10, 2);
        assert_eq!((record.wins, record.games), (10, 20));
        assert_eq!(record.rounds[0], record.rounds[1]);
        assert!((record.win_rate() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn thread_count_does_not_change_the_result() {
        let field = parse_field(DEFAULT_FIELD).unwrap();
        let quick = Contestant::parse("quick").unwrap();
        let gauntlet = Gauntlet::new("dk(1) k(V) k(V) k(V) dmMH(4)", &field[..2], quick).unwrap();
        let one = gauntlet.play(1, 1..=8, 1);
        let three = gauntlet.play(1, 1..=8, 3);
        assert_eq!((one.wins, one.rounds), (three.wins, three.rounds));
        assert_eq!(one.opponent, "Vincent");
    }

    #[test]
    fn the_total_pools_every_pair() {
        let records = [
            Record {
                opponent: "A".into(),
                games: 4,
                wins: 3,
                rounds: [9, 4],
                pair_scores: vec![1.0, 0.5],
            },
            Record {
                opponent: "B".into(),
                games: 2,
                wins: 0,
                rounds: [2, 6],
                pair_scores: vec![0.0],
            },
        ];
        let total = Record::total(&records);
        assert_eq!((total.wins, total.games, total.rounds), (3, 6, [11, 10]));
        assert!((total.win_rate() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn rows_line_up_under_the_header() {
        let table = Table::new(&[button("Sailor Jupiter", "6")]);
        let record = Record {
            opponent: "Lucky".into(),
            games: 4,
            wins: 3,
            rounds: [9, 3],
            pair_scores: vec![1.0, 0.5],
        };
        assert_eq!(
            table.header(),
            "Opponent        Won           Win %  95% CI       Rounds won"
        );
        assert_eq!(
            table.row(&record),
            "Lucky           3/4           75.0%  26.0-100.0%       75.0%"
        );
    }
}
