// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! Measures one button against a field of opponents. One engine plays both
//! seats and every seed is played with the button in each seat, so the result
//! reflects the buttons rather than the AI or the seat.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

use crate::Rng;
use crate::engines::Engine;
use crate::native::drain_with_workers;
use crate::search::{Engines, NativeReplaySequence, play_match_with_policies};
use crate::strength::{
    Contestant, Matchup, game_seed, mean_with_interval, pair_result, parse_game,
};

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
        let seeds = seeds.collect::<Vec<_>>();
        let pairs = self.play_seeds(opponent, &seeds, threads, &|_| {});
        Record::new(&self.opponents[opponent].button_first.label, &pairs)
    }

    /// Returns the pairs in `seeds` order; `report` sees each one as soon as
    /// it finishes.
    pub fn play_seeds(
        &self,
        opponent: usize,
        seeds: &[u32],
        threads: usize,
        report: &(dyn Fn(&Pair) + Sync),
    ) -> Vec<Pair> {
        let seats = &self.opponents[opponent];
        let next = AtomicUsize::new(0);
        let mut pairs = vec![Pair::default(); seeds.len()];
        let completed = drain_with_workers(threads, || {
            let engines: Engines = [self.engine.clone(), self.engine.clone()];
            let mut results = Vec::new();
            loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some(&seed) = seeds.get(index) else {
                    break;
                };
                let pair = play_pair(seats, opponent, seed, &engines);
                report(&pair);
                results.push((index, pair));
            }
            results
        });
        for (index, pair) in completed.into_iter().flatten() {
            pairs[index] = pair;
        }
        pairs
    }
}

fn play_pair(seats: &Seats, opponent: usize, seed: u32, engines: &Engines) -> Pair {
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
    let (wins, score) = pair_result(first.winner, second.winner);
    Pair {
        opponent,
        seed,
        wins: wins[0],
        score,
        rounds: [
            usize::from(first.wins[0] + second.wins[1]),
            usize::from(first.wins[1] + second.wins[0]),
        ],
    }
}

/// One seed against one opponent, played with the button in each seat.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Pair {
    /// The opponent's position in the field.
    pub opponent: usize,
    pub seed: u32,
    /// Games the button won of the two.
    pub wins: usize,
    pub score: f64,
    /// Rounds the button won and lost.
    pub rounds: [usize; 2],
}

/// Part `index + 1` of `count`. Pairs are dealt to the parts in turn,
/// opponent by opponent, so each part gets a share of every opponent and of
/// the slow seeds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shard {
    index: usize,
    count: usize,
}

impl Shard {
    /// Reads `K/N`, counting parts from 1.
    pub fn parse(text: &str) -> Result<Self, String> {
        let invalid = || format!("--shard takes K/N with 1 <= K <= N, not {text}");
        let (part, count) = text.split_once('/').ok_or_else(invalid)?;
        let part = part.parse::<usize>().map_err(|_| invalid())?;
        let count = count.parse::<usize>().map_err(|_| invalid())?;
        if part == 0 || part > count {
            return Err(invalid());
        }
        Ok(Self {
            index: part - 1,
            count,
        })
    }

    pub fn part(self) -> usize {
        self.index + 1
    }

    pub fn count(self) -> usize {
        self.count
    }

    pub fn seeds(self, seeds: RangeInclusive<u32>, opponent: usize) -> Vec<u32> {
        let per_opponent = seeds.clone().count();
        seeds
            .enumerate()
            .filter(|(position, _)| (opponent * per_opponent + position) % self.count == self.index)
            .map(|(_, seed)| seed)
            .collect()
    }
}

/// Changes whenever a merge could misread an older shard's lines.
pub const SHARD_FORMAT: u32 = 1;

/// The first line of a shard's output: enough for a merge to check that every
/// part comes from the same gauntlet and to print its table.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShardHeader {
    pub format: u32,
    pub button: String,
    pub engine: String,
    pub field: Vec<String>,
    /// The first and last seed of the whole gauntlet.
    pub seeds: [u32; 2],
    /// This part and the number of parts, counting from 1.
    pub shard: [usize; 2],
    /// Pairs this part plays.
    pub pairs: usize,
}

/// One line of `--shard` output.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ShardLine {
    Shard(ShardHeader),
    Pair(Pair),
}

/// A whole gauntlet rebuilt from its parts.
#[derive(Debug)]
pub struct Merged {
    pub button: String,
    pub engine: String,
    pub field: Vec<String>,
    pub seeds: RangeInclusive<u32>,
    pub records: Vec<Record>,
}

/// Rebuilds a gauntlet from every part's `--shard` output, in any order.
/// Records keep seed order so the totals and intervals match a single run.
pub fn merge(text: &str) -> Result<Merged, String> {
    let mut header: Option<ShardHeader> = None;
    let mut parts = BTreeMap::new();
    let mut pairs = BTreeMap::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let number = index + 1;
        match serde_json::from_str::<ShardLine>(line)
            .map_err(|error| format!("line {number}: {error}"))?
        {
            ShardLine::Shard(shard) => {
                if shard.format != SHARD_FORMAT {
                    return Err(format!(
                        "line {number}: shard format {} is not {SHARD_FORMAT}; merge with the bmair that played it",
                        shard.format
                    ));
                }
                let [part, count] = shard.shard;
                if let Some(first) = &header {
                    let same_run = ShardHeader {
                        shard: first.shard,
                        pairs: first.pairs,
                        ..shard.clone()
                    };
                    if &same_run != first {
                        return Err(format!(
                            "line {number}: shard {part}/{count} is from a different gauntlet"
                        ));
                    }
                }
                if parts.insert(part, number).is_some() {
                    return Err(format!("line {number}: shard {part}/{count} appears twice"));
                }
                header.get_or_insert(shard);
            }
            ShardLine::Pair(pair) => {
                if pairs.insert((pair.opponent, pair.seed), pair).is_some() {
                    return Err(format!(
                        "line {number}: opponent {} seed {} appears twice",
                        pair.opponent + 1,
                        pair.seed
                    ));
                }
            }
        }
    }
    let header = header.ok_or("no shard headers; merge the output of bmair gauntlet --shard")?;
    let count = header.shard[1];
    let missing = (1..=count)
        .filter(|part| !parts.contains_key(part))
        .map(|part| part.to_string())
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!("missing shard {} of {count}", missing.join(", ")));
    }
    let seeds = header.seeds[0]..=header.seeds[1];
    let mut records = Vec::new();
    for (opponent, name) in header.field.iter().enumerate() {
        let played = seeds
            .clone()
            .map(|seed| {
                pairs
                    .remove(&(opponent, seed))
                    .ok_or_else(|| format!("{name} seed {seed} is missing; was a shard cut short?"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        records.push(Record::new(name, &played));
    }
    if let Some(((opponent, seed), _)) = pairs.into_iter().next() {
        return Err(format!(
            "opponent {} seed {seed} is not part of this gauntlet",
            opponent + 1
        ));
    }
    Ok(Merged {
        button: header.button,
        engine: header.engine,
        field: header.field,
        seeds,
        records,
    })
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
    pub fn new(opponent: &str, pairs: &[Pair]) -> Record {
        Record {
            opponent: opponent.to_owned(),
            games: 2 * pairs.len(),
            wins: pairs.iter().map(|pair| pair.wins).sum(),
            rounds: [0, 1].map(|side| pairs.iter().map(|pair| pair.rounds[side]).sum()),
            pair_scores: pairs.iter().map(|pair| pair.score).collect(),
        }
    }

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
    pub fn new<'a>(names: impl IntoIterator<Item = &'a str>) -> Self {
        let name_width = names
            .into_iter()
            .map(|name| name.chars().count())
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
    fn a_cancelled_match_counts_as_a_draw() {
        let recipe = "n(4) n(4)";
        let quick = Contestant::parse("quick").unwrap();
        let gauntlet = Gauntlet::new(recipe, &[button("Nulls", recipe)], quick).unwrap();
        let record = gauntlet.play(0, 1..=2, 1);
        assert_eq!((record.wins, record.games, record.rounds), (0, 4, [0, 0]));
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
    fn shards_read_k_of_n() {
        assert_eq!(
            Shard::parse("2/5").map(|shard| (shard.part(), shard.count())),
            Ok((2, 5))
        );
        for bad in ["0/3", "4/3", "3", "a/3", "1/0", "/", "1/3/5"] {
            let error = Shard::parse(bad).unwrap_err();
            assert!(error.contains(bad), "{error}");
        }
    }

    #[test]
    fn shards_deal_every_pair_to_exactly_one_part() {
        for count in [1, 2, 3, 7, 40] {
            for opponent in 0..3 {
                let mut dealt = (1..=count)
                    .flat_map(|part| {
                        Shard::parse(&format!("{part}/{count}"))
                            .unwrap()
                            .seeds(5..=17, opponent)
                    })
                    .collect::<Vec<_>>();
                dealt.sort_unstable();
                assert_eq!(dealt, (5..=17).collect::<Vec<_>>(), "{count} parts");
            }
        }
    }

    fn shard_output(parts: &[usize], count: usize, pairs: &[(usize, u32)]) -> String {
        let mut lines = parts
            .iter()
            .map(|&part| {
                serde_json::to_string(&ShardLine::Shard(ShardHeader {
                    format: SHARD_FORMAT,
                    button: "(4) (X)".into(),
                    engine: "quick".into(),
                    field: vec!["A".into(), "B".into()],
                    seeds: [3, 4],
                    shard: [part, count],
                    pairs: 0,
                }))
                .unwrap()
            })
            .collect::<Vec<_>>();
        lines.extend(pairs.iter().map(|&(opponent, seed)| {
            serde_json::to_string(&ShardLine::Pair(Pair {
                opponent,
                seed,
                wins: 1,
                score: 0.5,
                rounds: [3, 3],
            }))
            .unwrap()
        }));
        lines.join("\n")
    }

    const EVERY_PAIR: [(usize, u32); 4] = [(1, 4), (0, 3), (1, 3), (0, 4)];

    #[test]
    fn a_merge_puts_pairs_back_in_seed_order() {
        let merged = merge(&shard_output(&[2, 1], 2, &EVERY_PAIR)).unwrap();
        assert_eq!(
            (merged.button.as_str(), merged.engine.as_str()),
            ("(4) (X)", "quick")
        );
        assert_eq!(merged.seeds, 3..=4);
        let opponents = merged.records.iter().map(|record| record.opponent.as_str());
        assert_eq!(opponents.collect::<Vec<_>>(), ["A", "B"]);
        assert_eq!((merged.records[1].wins, merged.records[1].games), (2, 4));
    }

    #[test]
    fn a_merge_refuses_anything_but_one_whole_gauntlet() {
        let mut different = shard_output(&[1], 2, &[]);
        different.push('\n');
        different.push_str(&shard_output(&[2], 2, &EVERY_PAIR).replace("quick", "random"));
        let mut old_format = shard_output(&[1], 1, &EVERY_PAIR);
        old_format = old_format.replace("\"format\":1", "\"format\":0");
        for (text, error) in [
            (String::new(), "no shard headers"),
            ("{\"type\":\"pair\"}".into(), "line 1: missing field"),
            (
                shard_output(&[1, 1], 2, &EVERY_PAIR),
                "shard 1/2 appears twice",
            ),
            (
                shard_output(&[1], 3, &EVERY_PAIR),
                "missing shard 2, 3 of 3",
            ),
            (different, "shard 2/2 is from a different gauntlet"),
            (old_format, "shard format 0 is not 1"),
            (
                shard_output(&[1], 1, &EVERY_PAIR[1..]),
                "B seed 4 is missing",
            ),
            (
                shard_output(&[1], 1, &[EVERY_PAIR.as_slice(), &[(0, 3)]].concat()),
                "opponent 1 seed 3 appears twice",
            ),
            (
                shard_output(&[1], 1, &[EVERY_PAIR.as_slice(), &[(2, 3)]].concat()),
                "opponent 3 seed 3 is not part of this gauntlet",
            ),
            (
                shard_output(&[1], 1, &[EVERY_PAIR.as_slice(), &[(0, 9)]].concat()),
                "opponent 1 seed 9 is not part of this gauntlet",
            ),
        ] {
            let actual = merge(&text).unwrap_err();
            assert!(actual.contains(error), "expected {error:?}, got {actual:?}");
        }
    }

    #[test]
    fn rows_line_up_under_the_header() {
        let table = Table::new(["Sailor Jupiter"]);
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
