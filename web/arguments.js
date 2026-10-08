// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

/**
 * Splits a command line into arguments the way a POSIX shell treats words,
 * quotes, and backslashes, so commands copied from a terminal work here.
 */
export function splitArguments(text) {
  const args = [];
  let word = null;
  let quote = null;
  for (let index = 0; index < text.length; index += 1) {
    const character = text[index];
    if (quote === "'") {
      if (character === "'") {
        quote = null;
      } else {
        word += character;
      }
    } else if (quote === '"') {
      if (character === '"') {
        quote = null;
      } else if (character === "\\" && /["\\$`]/.test(text[index + 1] ?? "")) {
        index += 1;
        word += text[index];
      } else {
        word += character;
      }
    } else if (character === '"' || character === "'") {
      quote = character;
      word ??= "";
    } else if (character === "\\" && index + 1 < text.length) {
      index += 1;
      word = (word ?? "") + text[index];
    } else if (/\s/.test(character)) {
      if (word !== null) {
        args.push(word);
        word = null;
      }
    } else {
      word = (word ?? "") + character;
    }
  }
  if (quote) {
    throw new Error(`Arguments have an unclosed ${quote} quote`);
  }
  if (word !== null) {
    args.push(word);
  }
  return args;
}
