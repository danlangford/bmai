// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::env;
use std::fs;
use std::io::{self, Write};

use bmair::gauntlet::{self, Gauntlet, Record, Shard, Table};
use bmair::strength::Contestant;
use bmair::{Capabilities, Parser, run_jsonl};

const GAUNTLET_USAGE: &str = "\
usage: bmair gauntlet [options] \"RECIPE\" [OPPONENTS]
       bmair gauntlet --merge [SHARD_FILES]

Plays RECIPE against each opponent and reports how often it wins. Write the
recipe in ButtonWeavers or BMAIR notation, inside quotes. OPPONENTS is a file
of `Name: recipe` lines, or - to read them from standard input; without it,
the default field is played.

To split a long gauntlet across processes or machines, run each part with the
same options plus --shard K/N, then pass every part's output to --merge, from
files or standard input. The table matches a single run.

options:
  --games N      matches per opponent, half from each seat (default 500)
  --engine SPEC  the AI for both seats (default \"montecarlo\")
  --seed N       the first seed (default 1)
  --threads N    matches to play at once (default: every core)
  --shard K/N    play part K of N and print its matches as JSON lines
  --merge        print the table for every part's --shard output
  --field        print the default field and exit
";

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if matches!(
        arguments.first().map(String::as_str),
        Some("-V" | "--version")
    ) {
        println!(
            "bmair {} ({}; {})",
            env!("BMAIR_BUILD_VERSION"),
            env!("BMAIR_GIT_DESCRIBE"),
            env!("BMAIR_BUILD_PROFILE")
        );
        return Ok(());
    }
    if matches!(
        arguments.first().map(String::as_str),
        Some("--capabilities")
    ) {
        serde_json::to_writer(io::stdout().lock(), &Capabilities::current())?;
        println!();
        return Ok(());
    }
    if arguments.first().map(String::as_str) == Some("gauntlet") {
        return run_gauntlet(&arguments[1..]);
    }
    if arguments.first().map(String::as_str) == Some("--protocol") {
        match arguments.get(1).map(String::as_str) {
            Some("jsonl-v1") if arguments.len() == 2 => {
                run_jsonl(io::stdin().lock(), io::stdout().lock())?;
                return Ok(());
            }
            Some(protocol) => return Err(format!("unsupported protocol: {protocol}").into()),
            None => return Err("--protocol requires a protocol name".into()),
        }
    }

    let mut output = io::stdout().lock();
    writeln!(output, "BMAIR: the Button Men AI in Rust")?;
    writeln!(output, "Rust port Copyright © 2026 Dan Langford.")?;
    writeln!(output, "Original BMAI Copyright © 2001-2026 Denis Papp.")?;
    writeln!(
        output,
        "Version: {} ({}; {})",
        env!("BMAIR_BUILD_VERSION"),
        env!("BMAIR_GIT_DESCRIBE"),
        env!("BMAIR_BUILD_PROFILE")
    )?;
    output.flush()?;

    let mut parser = Parser::default();
    parser.trace_rng_from_env();
    if let Some(path) = arguments.first() {
        writeln!(output, "Reading from {path}")?;
        let input = fs::read_to_string(path)?;
        parser.parse_string(&input, &mut output)?;
    } else {
        let mut input = io::stdin().lock();
        parser.parse_stream(&mut input, &mut output)?;
    }
    Ok(())
}

fn run_gauntlet(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if arguments.first().map(String::as_str) == Some("--merge") {
        return merge_gauntlet(&arguments[1..]);
    }
    let mut games = gauntlet::DEFAULT_GAMES;
    let mut engine = gauntlet::DEFAULT_ENGINE.to_owned();
    let mut first_seed = 1u32;
    let mut threads = std::thread::available_parallelism().map_or(1, usize::from);
    let mut shard = None;
    let mut positional = Vec::new();
    let mut arguments = arguments.iter();
    while let Some(argument) = arguments.next() {
        let mut value = || {
            arguments
                .next()
                .ok_or_else(|| format!("{argument} needs a value"))
        };
        match argument.as_str() {
            "-h" | "--help" => {
                print!("{GAUNTLET_USAGE}");
                return Ok(());
            }
            "--field" => {
                print!("{}", gauntlet::DEFAULT_FIELD);
                return Ok(());
            }
            "--games" => games = positive(argument, value()?)?,
            "--engine" => engine.clone_from(value()?),
            "--seed" => first_seed = positive(argument, value()?)?,
            "--threads" => threads = positive(argument, value()?)?,
            "--shard" => shard = Some(Shard::parse(value()?)?),
            "--merge" => {
                return Err("--merge comes first: bmair gauntlet --merge [SHARD_FILES]".into());
            }
            flag if flag.starts_with('-') && flag != "-" => {
                return Err(format!("unknown option {flag}\n\n{GAUNTLET_USAGE}").into());
            }
            recipe => positional.push(recipe),
        }
    }
    let (recipe, field) = match positional[..] {
        [recipe] => (recipe, gauntlet::DEFAULT_FIELD.to_owned()),
        [recipe, path] => (recipe, read_input(path)?),
        [] => return Err(GAUNTLET_USAGE.into()),
        _ => {
            return Err(
                "put the recipe inside quotes: bmair gauntlet \"(4) (6) (8) (10) (X)\"".into(),
            );
        }
    };
    let pairs = games.div_ceil(2);
    let last_seed = u32::try_from(pairs - 1)
        .ok()
        .and_then(|offset| first_seed.checked_add(offset))
        .ok_or("--seed plus --games runs past the last seed")?;
    let seeds = first_seed..=last_seed;
    let field = gauntlet::parse_field(&field)?;
    let engine = Contestant::parse(&engine)?;
    let spec = engine.spec().to_owned();
    let gauntlet = Gauntlet::new(recipe, &field, engine)?;

    if let Some(shard) = shard {
        gauntlet.play_shard(seeds, shard, threads, io::stdout())?;
        return Ok(());
    }

    let table = Table::new(field.iter().map(|button| button.name.as_str()));
    let mut output = io::stdout().lock();
    write_gauntlet_heading(&mut output, recipe, &spec, &seeds, &table)?;
    let mut records = Vec::new();
    for opponent in 0..gauntlet.opponents() {
        let record = gauntlet.play(opponent, seeds.clone(), threads);
        writeln!(output, "{}", table.row(&record))?;
        output.flush()?;
        records.push(record);
    }
    writeln!(output, "{}", table.row(&Record::total(&records)))?;
    Ok(())
}

fn merge_gauntlet(arguments: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(flag) = arguments
        .iter()
        .find(|argument| argument.starts_with('-') && *argument != "-")
    {
        if matches!(flag.as_str(), "-h" | "--help") {
            print!("{GAUNTLET_USAGE}");
            return Ok(());
        }
        return Err(format!("--merge takes shard files, not {flag}").into());
    }
    let paths = if arguments.is_empty() {
        vec!["-".to_owned()]
    } else {
        arguments.to_vec()
    };
    let sources = paths
        .iter()
        .map(|path| {
            let name = if path == "-" { "standard input" } else { path };
            read_input(path).map(|text| (name, text))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let merged = gauntlet::merge(sources.iter().map(|(name, text)| (*name, text.as_str())))?;
    let table = Table::new(merged.field.iter().map(|button| button.name.as_str()));
    let mut output = io::stdout().lock();
    write_gauntlet_heading(
        &mut output,
        &merged.button,
        &merged.engine,
        &merged.seeds,
        &table,
    )?;
    for record in &merged.records {
        writeln!(output, "{}", table.row(record))?;
    }
    writeln!(output, "{}", table.row(&Record::total(&merged.records)))?;
    Ok(())
}

fn write_gauntlet_heading(
    output: &mut impl Write,
    recipe: &str,
    engine: &str,
    seeds: &std::ops::RangeInclusive<u32>,
    table: &Table,
) -> io::Result<()> {
    writeln!(output, "Button: {recipe}")?;
    writeln!(output, "Engine: {engine}")?;
    writeln!(
        output,
        "{} games per opponent: seeds {}-{}, each played from both seats",
        2 * seeds.clone().count(),
        seeds.start(),
        seeds.end()
    )?;
    writeln!(output)?;
    writeln!(output, "{}", table.header())
}

fn read_input(path: &str) -> Result<String, String> {
    if path == "-" {
        io::read_to_string(io::stdin())
    } else {
        fs::read_to_string(path)
    }
    .map_err(|error| format!("{path}: {error}"))
}

fn positive<T: std::str::FromStr + Default + PartialEq>(
    option: &str,
    value: &str,
) -> Result<T, String> {
    value
        .parse()
        .ok()
        .filter(|parsed| *parsed != T::default())
        .ok_or_else(|| format!("{option} takes a whole number above zero, not {value}"))
}
