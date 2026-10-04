// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

//! Round-robin strength ladder:
//! `cargo run --release --example ladder -- --engine quick --engine "montecarlo ply=2"`

use bmair::strength::{Contestant, Matchup, markdown_table, play_pairing};

/// The preregistered 0.20.0 ladder in STRENGTH.md.
const DEFAULT_ENGINES: [&str; 5] = [
    "random",
    "maximize",
    "quick",
    "montecarlo ply=1 max_sims=100 min_sims=5 maxbranch=400",
    "montecarlo ply=2 max_sims=100 min_sims=5 maxbranch=400",
];

fn main() -> Result<(), String> {
    let mut specs = Vec::new();
    let mut matchups_path = String::from("tests/strength/matchups.txt");
    let mut seeds = 1..=50u32;
    let mut target_wins = 1u8;
    let mut threads = std::thread::available_parallelism().map_or(1, usize::from);
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--engine" => specs.push(value()?),
            "--matchups" => matchups_path = value()?,
            "--seeds" => {
                let range = value()?;
                let (low, high) = range
                    .split_once("..")
                    .ok_or(format!("--seeds takes LOW..HIGH, not {range}"))?;
                let parse = |text: &str| text.parse::<u32>().map_err(|error| error.to_string());
                seeds = parse(low)?..=parse(high)?;
            }
            "--target-wins" => target_wins = value()?.parse().map_err(|_| "bad --target-wins")?,
            "--threads" => threads = value()?.parse().map_err(|_| "bad --threads")?,
            _ => return Err(format!("unknown flag {flag}")),
        }
    }
    if specs.is_empty() {
        specs = DEFAULT_ENGINES.map(String::from).to_vec();
    }
    if seeds.contains(&0) {
        return Err("seed 0 reseeds from the clock; start seeds at 1".into());
    }
    let contestants = specs
        .iter()
        .map(|spec| Contestant::parse(spec))
        .collect::<Result<Vec<_>, _>>()?;
    let matchups = std::fs::read_to_string(&matchups_path)
        .map_err(|error| format!("{matchups_path}: {error}"))?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| Matchup::parse(line, target_wins))
        .collect::<Result<Vec<_>, _>>()?;

    let mut results = Vec::new();
    for (index, first) in contestants.iter().enumerate() {
        for second in &contestants[index + 1..] {
            let result = play_pairing(first, second, &matchups, seeds.clone(), threads);
            eprintln!(
                "{} vs {}: {:.3}",
                result.first, result.second, result.first_score
            );
            results.push(result);
        }
    }
    print!("{}", markdown_table(&results));
    Ok(())
}
