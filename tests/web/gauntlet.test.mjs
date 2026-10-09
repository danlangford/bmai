// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

import assert from "node:assert/strict";
import { test } from "node:test";
import { ShardProgress, gauntletWorkers, shardArguments } from "../../web/gauntlet.js";

const recipe = "(4) (6) (8) (10) (X)";

test("a gauntlet uses every core unless --threads says otherwise", () => {
  assert.equal(gauntletWorkers(["gauntlet", recipe], 8), 8);
  assert.equal(gauntletWorkers(["gauntlet", "--threads", "3", recipe, "-"], 8), 3);
  assert.equal(gauntletWorkers(["gauntlet", recipe], 0), 1);
});

test("anything that isn't a whole gauntlet runs as one engine", () => {
  for (const args of [
    [],
    ["--capabilities"],
    ["--protocol", "jsonl-v1"],
    ["gauntlet", "--field"],
    ["gauntlet", "--help"],
    ["gauntlet", "--merge"],
    ["gauntlet", "--shard", "1/2", recipe],
    // The engine explains a bad count better than a guess would.
    ["gauntlet", "--threads", "x", recipe],
    ["gauntlet", "--threads", "0", recipe],
  ]) {
    assert.equal(gauntletWorkers(args, 8), 0, args.join(" "));
  }
});

test("each shard gets the run's arguments and its part", () => {
  assert.deepEqual(shardArguments(["gauntlet", recipe, "-"], 2, 5), [
    "gauntlet",
    recipe,
    "-",
    "--shard",
    "2/5",
  ]);
});

test("progress counts matches across chunks that split lines", () => {
  const progress = new ShardProgress();
  assert.equal(progress.pairs, null);
  progress.add('{"type":"shard","format":1,"pairs":3,"field":["A"]}\n{"type":"pa');
  assert.deepEqual([progress.pairs, progress.played], [3, 0]);
  progress.add('ir","opponent":0}\n{"type":"pair","opponent":0}\n');
  assert.deepEqual([progress.pairs, progress.played], [3, 2]);
});
