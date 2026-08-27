// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::env;
use std::fs;
use std::io::{self, Read};

use bmair::BMC_Parser;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("BMAIR: the Button Men AI in Rust");
    println!("Based on BMAI, Copyright © 2001-2024 Denis Papp.");
    println!(
        "Rust port Copyright © 2026 Dan Langford <721364+danlangford@users.noreply.github.com>."
    );
    println!("Version: {}", env!("CARGO_PKG_VERSION"));

    let input = if let Some(path) = env::args().nth(1) {
        println!("Reading from {path}");
        fs::read_to_string(path)?
    } else {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        input
    };

    let mut parser = BMC_Parser::default();
    parser.ParseString(&input, &mut io::stdout())?;
    Ok(())
}
