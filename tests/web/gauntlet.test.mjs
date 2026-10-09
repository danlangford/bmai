// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

import assert from "node:assert/strict";
import { test } from "node:test";
import { ShardProgress, gauntletWorkers, shardArguments } from "../../web/gauntlet.js";

const recipe = "(4) (6) (8) (10) (X)";

test("a gauntlet uses every core unless --threads says otherwise", () => {
  assert.equal(gauntletWorkers(["gauntlet", recipe], 8), 8);
  assert.equal(gauntletWorkers(["gauntlet", "--threads", "3", recipe, "-"], 8), 3);
  assert.equal(gauntletWorkers(["gauntlet", "--threads", "+3", recipe], 8), 3);
  assert.equal(gauntletWorkers(["gauntlet", recipe], 0), 1);
  assert.equal(gauntletWorkers(["gauntlet", recipe], undefined), 1);
});

test("the last --threads wins, as in the engine", () => {
  assert.equal(gauntletWorkers(["gauntlet", "--threads", "2", "--threads", "5", recipe], 8), 5);
});

test("workers never outnumber cores", () => {
  assert.equal(gauntletWorkers(["gauntlet", "--threads", "100", recipe], 8), 8);
  assert.equal(gauntletWorkers(["gauntlet", "--threads", "4294967296", recipe], 8), 8);
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
  ]) {
    assert.equal(gauntletWorkers(args, 8), 1, args.join(" "));
  }
});

test("a --threads value the engine refuses goes to one engine to explain", () => {
  for (const value of ["x", "0", "00", "1e2", "0x10", " 2", "2.0", "-3", ""]) {
    assert.equal(gauntletWorkers(["gauntlet", "--threads", value, recipe], 8), 1, value);
  }
  assert.equal(gauntletWorkers(["gauntlet", recipe, "--threads"], 8), 1);
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
  progress.add("not json\n");
  assert.deepEqual([progress.pairs, progress.played], [3, 2]);
});
