// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

// A search can run for minutes, so it runs here rather than on the page's
// thread, and the page stops it by terminating this worker.

import { runCommand } from "./wasi.js";

const compiled = compile();

async function compile() {
  const response = await fetch(new URL("bmair.wasm", import.meta.url));
  if (!response.ok) {
    throw new Error(`could not load bmair.wasm: HTTP ${response.status}`);
  }
  // Streaming compilation refuses hosts that mislabel .wasm files.
  if (response.headers.get("content-type")?.startsWith("application/wasm")) {
    return WebAssembly.compileStreaming(response);
  }
  return WebAssembly.compile(await response.arrayBuffer());
}

self.onmessage = async ({ data: { id, args, stdin } }) => {
  const forward = (stream) => (bytes) =>
    self.postMessage({ id, type: stream, bytes }, [bytes.buffer]);
  try {
    const exitCode = await runCommand(await compiled, {
      args,
      stdin: new TextEncoder().encode(stdin),
      onStdout: forward("stdout"),
      onStderr: forward("stderr"),
    });
    self.postMessage({ id, type: "exit", exitCode });
  } catch (error) {
    self.postMessage({ id, type: "error", message: String(error?.message ?? error) });
  }
};
