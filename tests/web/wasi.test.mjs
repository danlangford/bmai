// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

// Runs bmair.wasm through the browser's WASI host, holding the web package to
// the native golden output and RNG fingerprints.
//
//   BMAIR_WASM=target/wasm32-wasip1/release/bmair.wasm node --test tests/web/*.test.mjs
//
// BMAIR_WEB_SLOW_FIXTURES=1 adds the fixtures that take minutes.

import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { test } from "node:test";
import { ShardProgress, shardArguments } from "../../web/gauntlet.js";
import { runCommand } from "../../web/wasi.js";

const root = new URL("../../", import.meta.url);
const wasmPath = process.env.BMAIR_WASM;
if (!wasmPath) {
  throw new Error("set BMAIR_WASM to the bmair.wasm to test");
}
const module = await WebAssembly.compile(await readFile(wasmPath));

// Keep in step with tests/fixture_golden.rs.
const SLOW_FIXTURES = ["bmai_in.txt", "bmsim_in.txt", "bug11_in.txt", "bug16_in.txt"];
const UNSTABLE_PREFIXES = [
  "BMAIR:",
  "Rust port Copyright",
  "Original BMAI Copyright",
  "Version:",
  "Reading from ",
];

async function bmair(args, stdin = "", env = {}) {
  const decoders = { stdout: new TextDecoder(), stderr: new TextDecoder() };
  const result = { stdout: "", stderr: "" };
  result.exitCode = await runCommand(module, {
    args,
    env,
    stdin: new TextEncoder().encode(stdin),
    onStdout: (bytes) => {
      result.stdout += decoders.stdout.decode(bytes, { stream: true });
    },
    onStderr: (bytes) => {
      result.stderr += decoders.stderr.decode(bytes, { stream: true });
    },
  });
  result.stdout += decoders.stdout.decode();
  result.stderr += decoders.stderr.decode();
  return result;
}

function normalizeLikeGolden({ exitCode, stdout, stderr }) {
  let normalized = `exit ${exitCode}\n`;
  for (const [name, text] of [["stdout", stdout], ["stderr", stderr]]) {
    normalized += `--- ${name}\n`;
    const lines = text.split(/\r?\n/);
    if (lines.at(-1) === "") {
      lines.pop();
    }
    for (const line of lines) {
      if (!UNSTABLE_PREFIXES.some((prefix) => line.startsWith(prefix))) {
        const timed = line.indexOf("Time:");
        normalized += `${(timed === -1 ? line : line.slice(0, timed)).trimEnd()}\n`;
      }
    }
  }
  return normalized;
}

const fixtureNames = (await readdir(new URL("tests/fixtures/", root)))
  .filter((name) => name.includes("in") && name.endsWith(".txt"))
  .filter((name) => process.env.BMAIR_WEB_SLOW_FIXTURES || !SLOW_FIXTURES.includes(name))
  .sort();

test("every golden fixture matches native output", async (context) => {
  for (const name of fixtureNames) {
    await context.test(name, async () => {
      const fixture = await readFile(new URL(`tests/fixtures/${name}`, root), "utf8");
      const expected = await readFile(new URL(`tests/golden/${name}`, root), "utf8");
      const result = await bmair([], fixture, { BMAIR_TRACE_RNG_HASH: "1" });
      assert.equal(normalizeLikeGolden(result), expected);
    });
  }
});

test("arguments reach the engine", async () => {
  const { exitCode, stdout } = await bmair(["--version"]);
  assert.equal(exitCode, 0);
  assert.match(stdout, /^bmair \d+\.\d+\.\d+\S* \(/);
});

test("a failing command reports its exit status and error", async () => {
  const { exitCode, stdout, stderr } = await bmair(["--protocol", "nope"]);
  assert.equal(exitCode, 1);
  assert.equal(stdout, "");
  assert.equal(stderr, "unsupported protocol: nope\n");
});

test("the engine cannot open files", async () => {
  const { exitCode, stderr } = await bmair(["tests/fixtures/Insult_in.txt"]);
  assert.equal(exitCode, 1);
  assert.match(stderr, /No such file or directory/);
});

test("worker counts above one give the one-worker result", async () => {
  const fixture = await readFile(new URL("tests/fixtures/Value1_in.txt", root), "utf8");
  const play = async (workers) => {
    const input = `mode native\nseed 3\nworkers ${workers}\nmax_sims 200\n${fixture}`;
    const { exitCode, stdout } = await bmair([], input);
    assert.equal(exitCode, 0);
    assert.match(stdout, /Valid Moves 4 /);
    return stdout.replace(`Setting native workers to ${workers}\n`, "");
  };
  assert.equal(await play(4), await play(1));
});

// The fixtures pin C++ defaults; the page runs BMAIR's, whose endgame solver
// recurses deeper than anything else on WebAssembly's 1 MiB stack.
test("search traces stay out of worker evaluations, as on native", async () => {
  const fixture = await readFile(new URL("tests/fixtures/Value1_in.txt", root), "utf8");
  const play = (workers) =>
    bmair([], `mode native\nseed 3\nworkers ${workers}\nmax_sims 100\n${fixture}`, {
      BMAIR_TRACE_RNG: "1",
    });
  const alone = await play(1);
  const shared = await play(2);
  assert.equal(shared.exitCode, 0);
  assert.match(alone.stderr, /QAI_RNG/);
  assert.doesNotMatch(shared.stderr, /QAI_RNG/);
});

test("BMAIR's own defaults run, endgame solver included", async () => {
  const fixture = await readFile(new URL("tests/fixtures/Insult_in.txt", root), "utf8");
  const fight = await bmair([], fixture);
  assert.equal(fight.exitCode, 0, fight.stderr);
  assert.match(fight.stdout, /\naction\n/);

  const gauntlet = await bmair(
    ["gauntlet", "--games", "2", "(4) (6) (8) (10) (X)", "-"],
    "Avis: (4) (4) (10) (12) (X)\n",
  );
  assert.equal(gauntlet.exitCode, 0, gauntlet.stderr);
  assert.match(gauntlet.stdout, /\nEngine: montecarlo\n/);
  assert.match(gauntlet.stdout, /\nOverall +\d\/2 /);
});

test("a gauntlet reads its opponents from standard input", async () => {
  const play = (threads) =>
    bmair(
      ["gauntlet", "--games", "4", "--engine", "quick", "--threads", threads, "6 12 20 20 X", "-"],
      "Avis: (4) (4) (10) (12) (X)\n",
    );
  const [alone, shared] = [await play("1"), await play("4")];
  assert.equal(shared.exitCode, 0);
  assert.match(shared.stdout, /\nAvis +\d\/4 /);
  assert.match(shared.stdout, /\nOverall +\d\/4 /);
  assert.equal(shared.stdout, alone.stdout);
});

test("gauntlet shards merge to the single-run table", async () => {
  const field = "Avis: (4) (4) (10) (12) (X)\nHammer: (6) (12) (20) (20) (X)\n";
  const args = ["gauntlet", "--games", "6", "--engine", "quick", "6 12 20 20 X", "-"];
  const single = await bmair(args, field);
  assert.equal(single.exitCode, 0, single.stderr);
  let parts = "";
  for (let part = 3; part >= 1; part -= 1) {
    const shard = await bmair(shardArguments(args, part, 3), field);
    assert.equal(shard.exitCode, 0, shard.stderr);
    const progress = new ShardProgress();
    progress.add(shard.stdout);
    assert.equal(progress.played, progress.pairs, `shard ${part}/3`);
    parts += shard.stdout;
  }
  const merged = await bmair(["gauntlet", "--merge"], parts);
  assert.equal(merged.exitCode, 0, merged.stderr);
  assert.equal(merged.stdout, single.stdout);
});

test("JSON Lines answers every request in order", async () => {
  const session = await readFile(new URL("tests/jsonl-fixtures/session.jsonl", root), "utf8");
  const requests = session.split("\n").filter((line) => line.trim() !== "");
  const { exitCode, stdout } = await bmair(["--protocol", "jsonl-v1"], session);
  assert.equal(exitCode, 0);
  const responses = stdout.trimEnd().split("\n").map((line) => JSON.parse(line));
  assert.equal(responses.length, requests.length);
  responses.forEach((response, index) => {
    let request = null;
    try {
      request = JSON.parse(requests[index]);
    } catch {
      assert.equal(response.ok, false);
      assert.equal(response.error.recoverable, true);
      return;
    }
    assert.equal(response.id, request.id);
    assert.equal(response.ok, true, JSON.stringify(response.error));
  });
});

test("a module that needs other WASI calls is refused before it runs", async () => {
  const name = (text) => [text.length, ...new TextEncoder().encode(text)];
  const importEntry = [...name("wasi_snapshot_preview1"), ...name("sock_accept"), 0x00, 0x00];
  const bytes = new Uint8Array([
    ...[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00],
    ...[0x01, 0x04, 0x01, 0x60, 0x00, 0x00],
    ...[0x02, importEntry.length + 1, 0x01, ...importEntry],
  ]);
  const unsupported = await WebAssembly.compile(bytes);
  await assert.rejects(runCommand(unsupported), /unsupported WASI calls: sock_accept/);
});
