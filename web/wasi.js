// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

// Browsers have no WASI runtime. This one covers exactly the imports the
// bmair command module declares, so the browser runs the same .wasm release
// artifact that Wasmtime runs. Files are deliberately out of reach: there
// are no preopened directories, so `path_open` can never succeed.

const SUCCESS = 0;
const BADF = 8;
const NOTCAPABLE = 76;

const CLOCK_REALTIME = 0;
const FILETYPE_CHARACTER_DEVICE = 2;
const STDIN = 0;
const STDOUT = 1;
const STDERR = 2;

class ProcessExit {
  constructor(code) {
    this.code = code;
  }
}

function encodeStrings(strings) {
  const encoder = new TextEncoder();
  return strings.map((value) => {
    const bytes = encoder.encode(value);
    const terminated = new Uint8Array(bytes.length + 1);
    terminated.set(bytes);
    return terminated;
  });
}

/**
 * Runs a compiled WASI command module to completion, like one process.
 * Resolves to the exit code; output arrives through the callbacks as the
 * module writes it.
 */
export async function runCommand(module, options = {}) {
  const {
    args = [],
    env = {},
    stdin = new Uint8Array(0),
    onStdout = () => {},
    onStderr = () => {},
  } = options;
  const argv = encodeStrings(["bmair", ...args]);
  const environ = encodeStrings(
    Object.entries(env).map(([name, value]) => `${name}=${value}`),
  );
  let inputOffset = 0;
  let memory;

  const view = () => new DataView(memory.buffer);
  const bytes = () => new Uint8Array(memory.buffer);

  const writeSizes = (strings, countPointer, sizePointer) => {
    const size = strings.reduce((total, value) => total + value.length, 0);
    view().setUint32(countPointer, strings.length, true);
    view().setUint32(sizePointer, size, true);
    return SUCCESS;
  };
  const writeStrings = (strings, pointersPointer, bufferPointer) => {
    const data = view();
    let next = bufferPointer;
    strings.forEach((value, index) => {
      data.setUint32(pointersPointer + index * 4, next, true);
      bytes().set(value, next);
      next += value.length;
    });
    return SUCCESS;
  };
  const iovecs = (pointer, count) => {
    const data = view();
    return Array.from({ length: count }, (_, index) => ({
      buffer: data.getUint32(pointer + index * 8, true),
      length: data.getUint32(pointer + index * 8 + 4, true),
    }));
  };
  const isStdio = (fd) => fd === STDIN || fd === STDOUT || fd === STDERR;

  const imports = {
    args_get: (pointers, buffer) => writeStrings(argv, pointers, buffer),
    args_sizes_get: (count, size) => writeSizes(argv, count, size),
    environ_get: (pointers, buffer) => writeStrings(environ, pointers, buffer),
    environ_sizes_get: (count, size) => writeSizes(environ, count, size),
    clock_time_get: (id, _precision, result) => {
      const nanoseconds =
        id === CLOCK_REALTIME
          ? BigInt(Date.now()) * 1_000_000n
          : BigInt(Math.round(performance.now() * 1e6));
      view().setBigUint64(result, nanoseconds, true);
      return SUCCESS;
    },
    fd_close: (fd) => (isStdio(fd) ? SUCCESS : BADF),
    fd_fdstat_get: (fd, result) => {
      if (!isStdio(fd)) {
        return BADF;
      }
      bytes().fill(0, result, result + 24);
      view().setUint8(result, FILETYPE_CHARACTER_DEVICE);
      return SUCCESS;
    },
    fd_filestat_get: (fd, result) => {
      if (!isStdio(fd)) {
        return BADF;
      }
      bytes().fill(0, result, result + 64);
      view().setUint8(result + 16, FILETYPE_CHARACTER_DEVICE);
      return SUCCESS;
    },
    fd_prestat_get: () => BADF,
    fd_prestat_dir_name: () => BADF,
    fd_read: (fd, pointer, count, result) => {
      if (fd !== STDIN) {
        return BADF;
      }
      let read = 0;
      for (const { buffer, length } of iovecs(pointer, count)) {
        const chunk = stdin.subarray(inputOffset, inputOffset + length);
        bytes().set(chunk, buffer);
        inputOffset += chunk.length;
        read += chunk.length;
        if (chunk.length < length) {
          break;
        }
      }
      view().setUint32(result, read, true);
      return SUCCESS;
    },
    fd_write: (fd, pointer, count, result) => {
      if (fd !== STDOUT && fd !== STDERR) {
        return BADF;
      }
      const sink = fd === STDOUT ? onStdout : onStderr;
      let written = 0;
      for (const { buffer, length } of iovecs(pointer, count)) {
        if (length > 0) {
          sink(bytes().slice(buffer, buffer + length));
        }
        written += length;
      }
      view().setUint32(result, written, true);
      return SUCCESS;
    },
    path_open: () => NOTCAPABLE,
    proc_exit: (code) => {
      throw new ProcessExit(code);
    },
    random_get: (buffer, length) => {
      // getRandomValues fills at most 65536 bytes per call.
      for (let offset = 0; offset < length; offset += 65536) {
        const end = Math.min(length, offset + 65536);
        crypto.getRandomValues(bytes().subarray(buffer + offset, buffer + end));
      }
      return SUCCESS;
    },
  };

  const declared = WebAssembly.Module.imports(module)
    .filter((entry) => entry.module === "wasi_snapshot_preview1")
    .map((entry) => entry.name);
  const missing = declared.filter((name) => !(name in imports));
  if (missing.length > 0) {
    throw new Error(`bmair.wasm needs unsupported WASI calls: ${missing.join(", ")}`);
  }

  const instance = await WebAssembly.instantiate(module, {
    wasi_snapshot_preview1: imports,
  });
  memory = instance.exports.memory;
  try {
    instance.exports._start();
    return 0;
  } catch (error) {
    if (error instanceof ProcessExit) {
      return error.code;
    }
    throw error;
  }
}
