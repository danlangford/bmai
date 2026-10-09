// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! Measures one button against a field of opponents. One engine plays both
//! seats and every seed is played with the button in each seat, so the result
//! reflects the buttons rather than the AI or the seat.

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::io::{self, Write};
use std::ops::RangeInclusive;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};

use serde::{Deserialize, Serialize};

use crate::engines::Engine;
use crate::native::drain_with_workers;
use crate::search::Engines;
use crate::strength::{
    Contestant, Matchup, mean_with_interval, pair_result, parse_game, play_seeded_match,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    button: String,
    field: Vec<Button>,
    engine_spec: String,
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
            button: recipe.to_owned(),
            field: field.to_vec(),
            engine_spec: contestant.spec().to_owned(),
            engine: contestant.engine,
            opponents,
        })
    }

    pub fn opponents(&self) -> usize {
        self.opponents.len()
    }

    pub fn play(&self, opponent: usize, seeds: RangeInclusive<u32>, threads: usize) -> Record {
        let seeds = seeds.collect::<Vec<_>>();
        let Ok(pairs) = self.play_seeds::<Infallible>(opponent, &seeds, threads, &|_| Ok(()));
        Record::new(&self.field[opponent].name, &pairs)
    }

    /// Plays this shard's part of the gauntlet and writes it as JSON lines:
    /// a header naming the run, then each pair as it finishes.
    pub fn play_shard(
        &self,
        seeds: RangeInclusive<u32>,
        shard: Shard,
        threads: usize,
        output: impl Write + Send,
    ) -> io::Result<()> {
        let header = ShardHeader {
            format: SHARD_FORMAT,
            bmair: BUILD_VERSION.to_owned(),
            button: self.button.clone(),
            engine: self.engine_spec.clone(),
            field: self.field.clone(),
            seeds: [*seeds.start(), *seeds.end()],
            shard: [shard.part(), shard.count()],
            pairs: shard.pairs(seeds.clone(), self.opponents()),
        };
        let output = Mutex::new(output);
        let write = |line: &ShardLine| -> io::Result<()> {
            let mut output = output.lock().unwrap_or_else(PoisonError::into_inner);
            serde_json::to_writer(&mut *output, line)?;
            writeln!(output)?;
            output.flush()
        };
        write(&ShardLine::Shard(header))?;
        for opponent in 0..self.opponents() {
            let seeds = shard.seeds(seeds.clone(), opponent);
            self.play_seeds(opponent, &seeds, threads, &|pair| {
                write(&ShardLine::Pair(*pair))
            })?;
        }
        Ok(())
    }

    /// Returns the pairs in `seeds` order. `report` sees each pair as soon as
    /// it finishes; its first error stops the play and is returned.
    pub fn play_seeds<E: Send + Sync>(
        &self,
        opponent: usize,
        seeds: &[u32],
        threads: usize,
        report: &(dyn Fn(&Pair) -> Result<(), E> + Sync),
    ) -> Result<Vec<Pair>, E> {
        let seats = &self.opponents[opponent];
        let next = AtomicUsize::new(0);
        let failure = OnceLock::new();
        let completed = drain_with_workers(threads, || {
            let engines: Engines = [self.engine.clone(), self.engine.clone()];
            let mut results = Vec::new();
            loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some(&seed) = seeds.get(index) else {
                    break;
                };
                let pair = play_pair(seats, opponent, seed, &engines);
                if let Err(error) = report(&pair) {
                    let _ = failure.set(error);
                    next.store(seeds.len(), Ordering::Relaxed);
                    break;
                }
                results.push((index, pair));
            }
            results
        });
        if let Some(error) = failure.into_inner() {
            return Err(error);
        }
        let mut pairs = vec![Pair::default(); seeds.len()];
        for (index, pair) in completed.into_iter().flatten() {
            pairs[index] = pair;
        }
        Ok(pairs)
    }
}

fn play_pair(seats: &Seats, opponent: usize, seed: u32, engines: &Engines) -> Pair {
    let play = |matchup: &Matchup| play_seeded_match(&matchup.game, seed, engines);
    let first = play(&seats.button_first);
    let second = play(&seats.button_second);
    let (wins, cancelled, score) = pair_result(first.winner, second.winner);
    Pair {
        opponent,
        seed,
        wins: wins[0],
        cancelled,
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
    /// Games of the two cancelled at the round limit, each scored as a draw.
    pub cancelled: usize,
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
            .filter(|&(position, _)| self.deals(opponent * per_opponent + position))
            .map(|(_, seed)| seed)
            .collect()
    }

    /// Pairs this part plays across `opponents` opponents.
    pub fn pairs(self, seeds: RangeInclusive<u32>, opponents: usize) -> usize {
        let total = opponents * seeds.count();
        (0..total).filter(|&pair| self.deals(pair)).count()
    }

    fn deals(self, pair: usize) -> bool {
        pair % self.count == self.index
    }

    /// The part of a `count`-part split that plays pair number `pair`.
    fn dealt(count: usize, pair: usize) -> Self {
        Self {
            index: pair % count,
            count,
        }
    }
}

const BUILD_VERSION: &str = env!("BMAIR_BUILD_VERSION");

/// Changes whenever a merge could misread another build's lines.
pub const SHARD_FORMAT: u32 = 2;

/// The first line of a shard's output: everything that decides its results,
/// so a merge can refuse parts of different gauntlets, and what the merge
/// needs to print the table.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShardHeader {
    pub format: u32,
    /// Another bmair build may play the same seed differently.
    pub bmair: String,
    pub button: String,
    pub engine: String,
    pub field: Vec<Button>,
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
    pub field: Vec<Button>,
    pub seeds: RangeInclusive<u32>,
    pub records: Vec<Record>,
}

/// Rebuilds a gauntlet from every part's `--shard` output, in any order and
/// split across any sources, each named for error messages. Records keep seed
/// order so the totals and intervals match a single run.
pub fn merge<'a>(sources: impl IntoIterator<Item = (&'a str, &'a str)>) -> Result<Merged, String> {
    let mut headers = Vec::new();
    let mut pair_lines = Vec::new();
    for (source, text) in sources {
        for (index, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let at = format!("{source}:{}", index + 1);
            match serde_json::from_str::<ShardLine>(line) {
                // Before the part's pair lines, which an older format may lack fields for.
                Ok(ShardLine::Shard(header)) if header.format != SHARD_FORMAT => {
                    let [part, count] = header.shard;
                    return Err(format!(
                        "{at}: shard {part}/{count} has format {}, not {SHARD_FORMAT}; merge it with the bmair that played it",
                        header.format
                    ));
                }
                Ok(ShardLine::Shard(header)) => headers.push((at, header)),
                Ok(ShardLine::Pair(pair)) => pair_lines.push((at, pair)),
                Err(error) => return Err(format!("{at}: {}", without_position(&error))),
            }
        }
    }
    let Some((_, first)) = headers.first() else {
        return Err("no shard headers; merge the output of bmair gauntlet --shard".into());
    };
    let first = first.clone();
    let count = first.shard[1];
    let seeds = first.seeds[0]..=first.seeds[1];
    let mut parts = BTreeMap::new();
    for (at, header) in &headers {
        let [part, its_count] = header.shard;
        let name = format!("shard {part}/{its_count}");
        if header.bmair != first.bmair {
            return Err(format!(
                "{at}: {name} was played by bmair {}, not {}",
                header.bmair, first.bmair
            ));
        }
        if its_count != count {
            return Err(format!(
                "{at}: {name} is from a {its_count}-part split, not {count}"
            ));
        }
        let differs = [
            ("button", header.button != first.button),
            ("engine", header.engine != first.engine),
            ("opponents", header.field != first.field),
            ("seeds", header.seeds != first.seeds),
        ]
        .into_iter()
        .find(|(_, differs)| *differs);
        if let Some((what, _)) = differs {
            return Err(format!(
                "{at}: {name} is from a different gauntlet; its {what} differ"
            ));
        }
        if part == 0 || part > count {
            return Err(format!("{at}: {name} is not one of {count} parts"));
        }
        let expected = Shard::dealt(count, part - 1).pairs(seeds.clone(), first.field.len());
        if header.pairs != expected {
            return Err(format!(
                "{at}: {name} says it plays {} pairs, but its part has {expected}",
                header.pairs
            ));
        }
        if let Some(earlier) = parts.insert(part, at) {
            return Err(format!("{at}: {name} appears twice (first at {earlier})"));
        }
    }
    let missing = (1..=count)
        .filter(|part| !parts.contains_key(part))
        .map(|part| part.to_string())
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!("missing shard {} of {count}", missing.join(", ")));
    }

    let mut pairs = BTreeMap::new();
    for (at, pair) in pair_lines {
        let Some(opponent) = first.field.get(pair.opponent) else {
            return Err(format!(
                "{at}: opponent index {} is past the field's {} buttons",
                pair.opponent,
                first.field.len()
            ));
        };
        if !seeds.contains(&pair.seed) {
            return Err(format!(
                "{at}: seed {} is outside seeds {}-{}",
                pair.seed,
                seeds.start(),
                seeds.end()
            ));
        }
        if let Some((earlier, _)) = pairs.insert((pair.opponent, pair.seed), (at.clone(), pair)) {
            return Err(format!(
                "{at}: {} seed {} appears twice (first at {earlier})",
                opponent.name, pair.seed
            ));
        }
    }
    let per_opponent = seeds.clone().count();
    let mut records = Vec::new();
    for (opponent, button) in first.field.iter().enumerate() {
        let played = seeds
            .clone()
            .enumerate()
            .map(|(position, seed)| {
                pairs
                    .remove(&(opponent, seed))
                    .map(|(_, pair)| pair)
                    .ok_or_else(|| {
                        let shard = Shard::dealt(count, opponent * per_opponent + position);
                        format!(
                            "{} seed {seed} is missing; rerun shard {}/{count}",
                            button.name,
                            shard.part()
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        records.push(Record::new(&button.name, &played));
    }
    Ok(Merged {
        button: first.button,
        engine: first.engine,
        field: first.field,
        seeds,
        records,
    })
}

/// serde places errors within the line; the caller already names the line.
fn without_position(error: &serde_json::Error) -> String {
    let message = error.to_string();
    message
        .rsplit_once(" at line ")
        .map_or(message.clone(), |(text, _)| text.to_owned())
}

#[derive(Clone, Debug)]
pub struct Record {
    pub opponent: String,
    pub games: usize,
    pub wins: usize,
    /// Matches cancelled at the round limit, each half a win in the win rate.
    pub cancelled: usize,
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
            cancelled: pairs.iter().map(|pair| pair.cancelled).sum(),
            rounds: [0, 1].map(|side| pairs.iter().map(|pair| pair.rounds[side]).sum()),
            pair_scores: pairs.iter().map(|pair| pair.score).collect(),
        }
    }

    pub fn total(records: &[Record]) -> Record {
        Record {
            opponent: "Overall".into(),
            games: records.iter().map(|record| record.games).sum(),
            wins: records.iter().map(|record| record.wins).sum(),
            cancelled: records.iter().map(|record| record.cancelled).sum(),
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
        self.line(
            "Opponent",
            "Won",
            "Cancelled",
            "Win %",
            "95% CI",
            "Rounds won",
        )
    }

    pub fn row(&self, record: &Record) -> String {
        let (low, high) = record.interval();
        self.line(
            &record.opponent,
            &format!("{}/{}", record.wins, record.games),
            &record.cancelled.to_string(),
            &percent(record.win_rate()),
            &format!("{:.1}-{:.1}%", 100.0 * low, 100.0 * high),
            &percent(record.round_rate()),
        )
    }

    fn line(
        &self,
        name: &str,
        won: &str,
        cancelled: &str,
        rate: &str,
        interval: &str,
        rounds: &str,
    ) -> String {
        let width = self.name_width;
        format!(
            "{name:<width$}  {won:<11}  {cancelled:>9}  {rate:>6}  {interval:<11}  {rounds:>10}"
        )
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
        assert_eq!(
            (record.wins, record.cancelled, record.games, record.rounds),
            (0, 4, 4, [0, 0])
        );
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
                cancelled: 0,
                rounds: [9, 4],
                pair_scores: vec![1.0, 0.5],
            },
            Record {
                opponent: "B".into(),
                games: 2,
                wins: 0,
                cancelled: 1,
                rounds: [2, 6],
                pair_scores: vec![0.25],
            },
        ];
        let total = Record::total(&records);
        assert_eq!(
            (total.wins, total.cancelled, total.games, total.rounds),
            (3, 1, 6, [11, 10])
        );
        assert!((total.win_rate() - 1.75 / 3.0).abs() < 1e-9);
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

    #[test]
    fn shards_get_an_even_share_of_the_whole_field() {
        // The web page's gauntlet example: 8 opponents, 5 seeds, 18 workers.
        for (opponents, seeds, count) in [(8, 1..=5, 18), (8, 1..=250, 7), (3, 1..=4, 5)] {
            let total = opponents * seeds.clone().count();
            for part in 1..=count {
                let shard = Shard::parse(&format!("{part}/{count}")).unwrap();
                let pairs = (0..opponents)
                    .map(|opponent| shard.seeds(seeds.clone(), opponent).len())
                    .sum::<usize>();
                assert!(
                    pairs == total / count || pairs == total.div_ceil(count),
                    "part {part}/{count} plays {pairs} of {total}"
                );
                assert_eq!(shard.pairs(seeds.clone(), opponents), pairs);
            }
        }
    }

    fn header(part: usize, count: usize) -> ShardHeader {
        let field = vec![button("A", "(6) (X)"), button("B", "(8) (X)")];
        let shard = Shard::parse(&format!("{part}/{count}")).unwrap();
        ShardHeader {
            format: SHARD_FORMAT,
            bmair: BUILD_VERSION.into(),
            button: "(4) (X)".into(),
            engine: "quick".into(),
            pairs: shard.pairs(3..=4, field.len()),
            field,
            seeds: [3, 4],
            shard: [part, count],
        }
    }

    fn pair(opponent: usize, seed: u32) -> Pair {
        Pair {
            opponent,
            seed,
            wins: 1,
            cancelled: usize::from(seed == 4),
            // Distinct scores show the order a merge puts them in.
            score: f64::from(seed) / 10.0,
            rounds: [3, 3],
        }
    }

    fn lines(headers: &[ShardHeader], pairs: &[(usize, u32)]) -> String {
        let headers = headers.iter().cloned().map(ShardLine::Shard);
        let pairs = pairs
            .iter()
            .map(|&(opponent, seed)| ShardLine::Pair(pair(opponent, seed)));
        headers
            .chain(pairs)
            .map(|line| serde_json::to_string(&line).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
    }

    const EVERY_PAIR: [(usize, u32); 4] = [(1, 4), (0, 3), (1, 3), (0, 4)];

    #[test]
    fn a_merge_puts_pairs_back_in_seed_order() {
        let first = lines(&[header(2, 2)], &EVERY_PAIR[..2]);
        let second = lines(&[header(1, 2)], &EVERY_PAIR[2..]);
        let merged = merge([("b", first.as_str()), ("a", second.as_str())]).unwrap();
        assert_eq!(
            (merged.button.as_str(), merged.engine.as_str()),
            ("(4) (X)", "quick")
        );
        assert_eq!(merged.seeds, 3..=4);
        let opponents = merged.records.iter().map(|record| record.opponent.as_str());
        assert_eq!(opponents.collect::<Vec<_>>(), ["A", "B"]);
        assert_eq!(
            (
                merged.records[1].wins,
                merged.records[1].cancelled,
                merged.records[1].games
            ),
            (2, 1, 4)
        );
        assert_eq!(merged.records[1].pair_scores, [0.3, 0.4]);
    }

    #[test]
    fn a_merge_refuses_anything_but_one_whole_gauntlet() {
        let whole = |change: fn(&mut ShardHeader)| {
            let mut second = header(2, 2);
            change(&mut second);
            lines(&[header(1, 2), second], &EVERY_PAIR)
        };
        for (text, error) in [
            (String::new(), "no shard headers"),
            (
                "{\"type\":\"pair\"}".into(),
                "x:1: missing field `opponent`",
            ),
            (
                "{\"type\":\"pair\",".into(),
                "x:1: EOF while parsing a value",
            ),
            (
                "{\"type\":\"pair\",\"opponent\":0,\"seed\":3,\"wins\":1,\"score\":0.5,\"rounds\":[3,3]}"
                    .into(),
                "x:1: missing field `cancelled`",
            ),
            (
                whole(|header| header.format = 0),
                "x:2: shard 2/2 has format 0, not 2",
            ),
            (
                whole(|header| header.bmair = "0.1.0".into()),
                "x:2: shard 2/2 was played by bmair 0.1.0",
            ),
            (
                whole(|header| header.shard = [2, 3]),
                "x:2: shard 2/3 is from a 3-part split, not 2",
            ),
            (
                whole(|header| header.button = "(20)".into()),
                "x:2: shard 2/2 is from a different gauntlet; its button differ",
            ),
            (
                whole(|header| header.engine = "random".into()),
                "its engine differ",
            ),
            (
                whole(|header| header.field[1].recipe = "(20)".into()),
                "its opponents differ",
            ),
            (whole(|header| header.seeds = [3, 5]), "its seeds differ"),
            (
                whole(|header| header.shard = [3, 2]),
                "x:2: shard 3/2 is not one of 2 parts",
            ),
            (
                whole(|header| header.pairs = 0),
                "x:2: shard 2/2 says it plays 0 pairs, but its part has 2",
            ),
            (
                lines(&[header(1, 2), header(1, 2)], &EVERY_PAIR),
                "x:2: shard 1/2 appears twice (first at x:1)",
            ),
            (
                lines(&[header(1, 3)], &EVERY_PAIR),
                "missing shard 2, 3 of 3",
            ),
            (
                lines(&[header(1, 2), header(2, 2)], &EVERY_PAIR[1..]),
                "B seed 4 is missing; rerun shard 2/2",
            ),
            (
                lines(
                    &[header(1, 1)],
                    &[EVERY_PAIR.as_slice(), &[(0, 3)]].concat(),
                ),
                "x:6: A seed 3 appears twice (first at x:3)",
            ),
            (
                lines(
                    &[header(1, 1)],
                    &[EVERY_PAIR.as_slice(), &[(2, 3)]].concat(),
                ),
                "x:6: opponent index 2 is past the field's 2 buttons",
            ),
            (
                lines(
                    &[header(1, 1)],
                    &[EVERY_PAIR.as_slice(), &[(0, 9)]].concat(),
                ),
                "x:6: seed 9 is outside seeds 3-4",
            ),
        ] {
            let actual = merge([("x", text.as_str())]).unwrap_err();
            assert!(actual.contains(error), "expected {error:?}, got {actual:?}");
            assert!(!actual.contains(" at line "), "{actual}");
        }
    }

    #[test]
    fn a_merge_refuses_a_part_played_before_cancelled_counts() {
        let mut old = header(1, 1);
        old.format = 1;
        let text = [
            serde_json::to_string(&ShardLine::Shard(old)).unwrap(),
            "{\"type\":\"pair\",\"opponent\":0,\"seed\":3,\"wins\":1,\"score\":0.5,\"rounds\":[3,3]}"
                .into(),
        ]
        .join("\n");
        assert_eq!(
            merge([("old", text.as_str())]).unwrap_err(),
            "old:1: shard 1/1 has format 1, not 2; merge it with the bmair that played it"
        );
    }

    #[test]
    fn rows_line_up_under_the_header() {
        let table = Table::new(["Sailor Jupiter"]);
        let record = Record {
            opponent: "Lucky".into(),
            games: 4,
            wins: 2,
            cancelled: 1,
            rounds: [9, 3],
            pair_scores: vec![1.0, 0.25],
        };
        assert_eq!(
            table.header(),
            "Opponent        Won          Cancelled   Win %  95% CI       Rounds won"
        );
        assert_eq!(
            table.row(&record),
            "Lucky           2/4                  1   62.5%  0.0-100.0%        75.0%"
        );
    }
}
