// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

// The engine gets one thread in a browser, so a gauntlet runs as several
// engines in separate workers, each playing one `--shard`, and the engine's
// own `--merge` prints the table a single run would.

const NOT_A_RUN = new Set(["-h", "--help", "--field", "--shard", "--merge"]);

/**
 * How many workers should share a gauntlet: its `--threads` value, as on the
 * command line, or else every core. Anything else runs as one engine (0).
 */
export function gauntletWorkers(args, cores) {
  if (args[0] !== "gauntlet" || args.some((arg) => NOT_A_RUN.has(arg))) {
    return 0;
  }
  const threads = args.indexOf("--threads");
  if (threads !== -1) {
    const value = Number(args[threads + 1]);
    return Number.isInteger(value) && value > 0 ? value : 0;
  }
  return Math.max(1, cores);
}

export function shardArguments(args, part, count) {
  return [...args, "--shard", `${part}/${count}`];
}

/** Counts a shard's matches from its JSON lines as they arrive. */
export class ShardProgress {
  pairs = null;
  played = 0;
  #pending = "";

  add(text) {
    const lines = (this.#pending + text).split("\n");
    this.#pending = lines.pop();
    for (const line of lines) {
      if (line.startsWith('{"type":"pair"')) {
        this.played += 1;
      } else if (line.startsWith('{"type":"shard"')) {
        this.pairs = JSON.parse(line).pairs;
      }
    }
  }
}
