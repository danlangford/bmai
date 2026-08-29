// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::env;
use std::process::Command;

#[path = "src/build_version.rs"]
mod build_version;

fn main() {
    println!("cargo:rerun-if-env-changed=BMAIR_GIT_DESCRIBE");
    watch_git_path("HEAD");
    watch_git_path("index");
    watch_git_path("packed-refs");
    watch_git_path("refs/tags");

    let describe = env::var("BMAIR_GIT_DESCRIBE")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(git_describe)
        .unwrap_or_else(|| "unknown".to_owned());
    let version = build_version::derive(env!("CARGO_PKG_VERSION"), &describe);
    println!("cargo:rustc-env=BMAIR_GIT_DESCRIBE={describe}");
    println!("cargo:rustc-env=BMAIR_BUILD_VERSION={version}");
    println!(
        "cargo:rustc-env=BMAIR_BUILD_PROFILE={}",
        env::var("PROFILE").unwrap_or_else(|_| "unknown".to_owned())
    );
}

fn git_describe() -> Option<String> {
    let output = Command::new("git")
        .args([
            "describe",
            "--dirty",
            "--always",
            "--long",
            "--tags",
            "--match=bmair-v[0-9]*",
        ])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn watch_git_path(path: &str) {
    let Ok(output) = Command::new("git")
        .args(["rev-parse", "--git-path", path])
        .output()
    else {
        return;
    };
    if output.status.success() {
        println!(
            "cargo:rerun-if-changed={}",
            String::from_utf8_lossy(&output.stdout).trim()
        );
    }
}
