// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

// The engine gets one thread in a browser, so a gauntlet runs as several
// engines in separate workers, each playing one `--shard`, and the engine's
// own `--merge` prints the table a single run would.

const NOT_A_RUN = new Set(["-h", "--help", "--field", "--shard", "--merge"]);

/**
 * How many workers should share a gauntlet: its `--threads` value, as on the
 * command line, or else every core. More workers than cores cost memory and
 * gain nothing. Anything but a whole gauntlet runs as one engine.
 */
export function gauntletWorkers(args, cores) {
  const available = Math.max(1, cores || 1);
  if (args[0] !== "gauntlet" || args.some((arg) => NOT_A_RUN.has(arg))) {
    return 1;
  }
  const threads = args.lastIndexOf("--threads");
  if (threads === -1) {
    return available;
  }
  // Only what the engine's own parser accepts; it explains anything else.
  const value = args[threads + 1] ?? "";
  if (!/^\+?\d+$/.test(value) || Number(value) === 0) {
    return 1;
  }
  return Math.min(Number(value), available);
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
      let parsed;
      try {
        parsed = JSON.parse(line);
      } catch {
        // The merge names any line it can't read; progress just skips it.
        continue;
      }
      const { type, pairs } = parsed;
      if (type === "pair") {
        this.played += 1;
      } else if (type === "shard") {
        this.pairs = pairs;
      }
    }
  }
}
