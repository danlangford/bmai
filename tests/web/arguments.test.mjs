// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

import assert from "node:assert/strict";
import { test } from "node:test";
import { splitArguments } from "../../web/arguments.js";

test("words split on any run of whitespace", () => {
  assert.deepEqual(splitArguments("  --protocol \t jsonl-v1\n"), ["--protocol", "jsonl-v1"]);
  assert.deepEqual(splitArguments(""), []);
  assert.deepEqual(splitArguments("   "), []);
});

test("quotes keep a recipe together", () => {
  assert.deepEqual(splitArguments('gauntlet --games 2 "(4) (6) (8) (10) (X)" -'), [
    "gauntlet",
    "--games",
    "2",
    "(4) (6) (8) (10) (X)",
    "-",
  ]);
  assert.deepEqual(splitArguments("gauntlet 'p(12) z(X)'"), ["gauntlet", "p(12) z(X)"]);
});

test("quotes join with adjacent text and may be empty", () => {
  assert.deepEqual(splitArguments('a"b c"d'), ["ab cd"]);
  assert.deepEqual(splitArguments('"" x'), ["", "x"]);
});

test("backslashes escape like a POSIX shell", () => {
  assert.deepEqual(splitArguments("a\\ b"), ["a b"]);
  assert.deepEqual(splitArguments('"say \\"hi\\""'), ['say "hi"']);
  assert.deepEqual(splitArguments('"keep \\n"'), ["keep \\n"]);
  assert.deepEqual(splitArguments("'\\'"), ["\\"]);
});

test("an unclosed quote is refused", () => {
  assert.throws(() => splitArguments('gauntlet "(4) (6)'), /unclosed " quote/);
  assert.throws(() => splitArguments("'"), /unclosed ' quote/);
});
