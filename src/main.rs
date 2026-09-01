// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::env;
use std::fs;
use std::io::{self, Write};

use bmair::{BMC_Parser, Capabilities, run_jsonl};

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

    let mut parser = BMC_Parser::default();
    if let Some(path) = arguments.first() {
        writeln!(output, "Reading from {path}")?;
        let input = fs::read_to_string(path)?;
        parser.ParseString(&input, &mut output)?;
    } else {
        let mut input = io::stdin().lock();
        parser.ParseStream(&mut input, &mut output)?;
    }
    Ok(())
}
