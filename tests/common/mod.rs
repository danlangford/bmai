// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::process::Command;

/// Without the developer's trace settings, which add to stderr and can fill
/// the pipe of a test that reads it only after the process exits.
pub fn bmair() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_bmair"));
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("BMAIR_TRACE_") {
            command.env_remove(name);
        }
    }
    command
}
