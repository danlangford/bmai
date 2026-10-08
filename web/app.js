// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

import { splitArguments } from "./arguments.js";

const EXAMPLES = [
  { label: "Fight: choose an attack", args: "", file: "examples/fight.txt" },
  { label: "Fight: Value dice", args: "", file: "examples/value.txt" },
  { label: "Preround: choose swing sizes, then play 10 games", args: "", file: "examples/preround.txt" },
  { label: "JSON Lines session", args: "--protocol jsonl-v1", file: "examples/session.jsonl" },
  {
    label: "Gauntlet: one button against a field you can edit",
    args: 'gauntlet --games 2 "(4) (6) (8) (10) (X)" -',
    command: ["gauntlet", "--field"],
  },
  { label: "Capabilities", args: "--capabilities", input: "" },
];
const CUSTOM = "custom";
const STORAGE_KEY = "bmair-web";
const WRAP_KEY = "bmair-web-wrap";

const element = (id) => document.getElementById(id);
const form = element("controls");
const exampleSelect = element("example");
const argsInput = element("args");
const input = element("input");
const output = element("output");
const status = element("status");
const version = element("version");
const runButton = element("run");
const stopButton = element("stop");
const copyButton = element("copy");
const saveButton = element("save");
const wrapButton = element("wrap");
const fileInput = element("file");
const notice = element("notice");

let worker = null;
let nextRequestId = 0;
const captures = new Map();
let run = null;
let latestSelection = 0;
let pending = [];
let flushScheduled = false;

function showNotice(message) {
  notice.textContent = message;
  notice.hidden = false;
}

function setStatus(text, state) {
  status.textContent = text;
  status.dataset.state = state;
}

function load(key) {
  try {
    return JSON.parse(localStorage.getItem(key));
  } catch {
    return null;
  }
}

function store(key, value) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Blocked storage costs only the convenience of restoring the last session.
  }
}

function save() {
  store(STORAGE_KEY, { example: exampleSelect.value, args: argsInput.value, input: input.value });
}

function setWrap(wrap) {
  output.classList.toggle("wrap", wrap);
  wrapButton.setAttribute("aria-pressed", String(wrap));
}

function startWorker() {
  worker = new Worker(new URL("worker.js", import.meta.url), { type: "module" });
  worker.onmessage = ({ data }) => {
    if (captures.has(data.id)) {
      handleCapture(data);
    } else if (data.id === run?.id) {
      handleRun(data);
    }
  };
  worker.onerror = (event) => {
    event.preventDefault();
    const message = event.message || "the engine worker did not start";
    abandonCaptures(message);
    if (run) {
      finish("error", message);
    }
  };
}

function send(args, stdin) {
  nextRequestId += 1;
  worker.postMessage({ id: nextRequestId, args, stdin });
  return nextRequestId;
}

/** Runs a short command out of sight and resolves to its standard output. */
function capture(args) {
  return new Promise((resolve, reject) => {
    captures.set(send(args, ""), { args, resolve, reject, text: "", decoder: new TextDecoder() });
  });
}

function handleCapture(message) {
  const request = captures.get(message.id);
  if (message.type === "stdout") {
    request.text += request.decoder.decode(message.bytes, { stream: true });
  } else if (message.type === "exit" || message.type === "error") {
    captures.delete(message.id);
    if (message.type === "exit" && message.exitCode === 0) {
      request.resolve(request.text + request.decoder.decode());
    } else {
      request.reject(new Error(message.message ?? `exit status ${message.exitCode}`));
    }
  }
}

function abandonCaptures(reason) {
  for (const request of captures.values()) {
    request.reject(new Error(reason));
  }
  captures.clear();
}

function handleRun(message) {
  if (message.type === "stdout" || message.type === "stderr") {
    queueOutput(run.decoders[message.type].decode(message.bytes, { stream: true }), message.type);
  } else if (message.type === "exit") {
    finish("exit", message.exitCode);
  } else if (message.type === "error") {
    finish("error", message.message);
  }
}

// The engine writes line by line; drawing once per frame keeps long games responsive.
function queueOutput(text, stream) {
  if (!text) {
    return;
  }
  pending.push({ text, stream });
  if (!flushScheduled) {
    flushScheduled = true;
    requestAnimationFrame(flushOutput);
  }
}

function flushOutput() {
  flushScheduled = false;
  const following = output.scrollHeight - output.scrollTop - output.clientHeight < 32;
  for (const { text, stream } of pending) {
    const last = output.lastChild;
    if (stream === "stdout") {
      if (last?.nodeType === Node.TEXT_NODE) {
        last.appendData(text);
      } else {
        output.append(text);
      }
    } else if (last?.nodeType === Node.ELEMENT_NODE) {
      last.append(text);
    } else {
      const span = document.createElement("span");
      span.className = "stderr";
      span.textContent = text;
      output.append(span);
    }
  }
  pending = [];
  if (following) {
    output.scrollTop = output.scrollHeight;
  }
  const empty = output.textContent === "";
  copyButton.disabled = empty;
  saveButton.disabled = empty;
}

function elapsed() {
  const milliseconds = performance.now() - run.started;
  return milliseconds < 1000
    ? `${Math.round(milliseconds)} ms`
    : `${(milliseconds / 1000).toFixed(1)} s`;
}

function finish(outcome, detail) {
  for (const stream of ["stdout", "stderr"]) {
    queueOutput(run.decoders[stream].decode(), stream);
  }
  clearInterval(run.timer);
  const time = elapsed();
  if (outcome === "exit" && detail === 0) {
    setStatus(`Finished in ${time}`, "done");
  } else if (outcome === "exit") {
    setStatus(`Exited with status ${detail} after ${time}`, "failed");
  } else if (outcome === "stopped") {
    setStatus(`Stopped after ${time}`, "failed");
  } else {
    queueOutput(`\n${detail}\n`, "stderr");
    setStatus(`Crashed after ${time}`, "failed");
  }
  run = null;
  runButton.disabled = false;
  stopButton.disabled = true;
}

function start() {
  if (run || runButton.disabled) {
    return;
  }
  let args;
  try {
    args = splitArguments(argsInput.value);
  } catch (error) {
    setStatus(error.message, "failed");
    return;
  }
  save();
  output.textContent = "";
  pending = [];
  run = {
    id: send(args, input.value),
    started: performance.now(),
    decoders: { stdout: new TextDecoder(), stderr: new TextDecoder() },
    timer: setInterval(() => setStatus(`Running… ${elapsed()}`, "running"), 100),
  };
  setStatus("Running…", "running");
  runButton.disabled = true;
  stopButton.disabled = false;
  copyButton.disabled = true;
  saveButton.disabled = true;
}

function stop() {
  if (!run) {
    return;
  }
  worker.terminate();
  finish("stopped");
  startWorker();
  // An example still loading behind the stopped run starts over on the new worker.
  const pending = [...captures.values()];
  captures.clear();
  for (const request of pending) {
    request.text = "";
    request.decoder = new TextDecoder();
    captures.set(send(request.args, ""), request);
  }
}

async function exampleInput(example) {
  if (example.command) {
    return capture(example.command);
  }
  if (example.file) {
    const response = await fetch(new URL(example.file, import.meta.url));
    if (!response.ok) {
      throw new Error(`${example.file}: HTTP ${response.status}`);
    }
    return response.text();
  }
  return "";
}

async function selectExample(example) {
  const selection = (latestSelection += 1);
  argsInput.value = example.args;
  input.value = "";
  let text = "";
  try {
    text = await exampleInput(example);
  } catch (error) {
    setStatus(`Could not load the example: ${error.message}`, "failed");
  }
  if (selection === latestSelection) {
    input.value = text;
    save();
  }
}

function download(text) {
  const link = document.createElement("a");
  link.href = URL.createObjectURL(new Blob([text], { type: "text/plain" }));
  link.download = "bmair-output.txt";
  link.click();
  setTimeout(() => URL.revokeObjectURL(link.href), 0);
}

async function copy() {
  try {
    await navigator.clipboard.writeText(output.textContent);
    copyButton.textContent = "Copied";
  } catch {
    copyButton.textContent = "Copy failed";
  }
  setTimeout(() => {
    copyButton.textContent = "Copy";
  }, 1500);
}

function initialize() {
  if (location.protocol === "file:") {
    version.textContent = "not started";
    showNotice(
      "Browsers will not start the engine from a file:// page. Serve this folder over HTTP, " +
        "for example with `python3 -m http.server`, and open it from there.",
    );
    return;
  }

  startWorker();
  capture(["--version"]).then(
    (text) => {
      version.textContent = /^bmair (\S+)/.exec(text)?.[1] ?? "unknown version";
      runButton.disabled = false;
    },
    (error) => {
      version.textContent = "unavailable";
      showNotice(`The engine could not start: ${error.message}`);
    },
  );

  EXAMPLES.forEach((example, index) => exampleSelect.add(new Option(example.label, String(index))));
  exampleSelect.add(new Option("Your own input", CUSTOM));
  const saved = load(STORAGE_KEY);
  if (saved) {
    argsInput.value = saved.args ?? "";
    input.value = saved.input ?? "";
    exampleSelect.value = EXAMPLES[saved.example] ? saved.example : CUSTOM;
  } else {
    selectExample(EXAMPLES[0]);
  }

  const markCustom = () => {
    latestSelection += 1;
    exampleSelect.value = CUSTOM;
    save();
  };
  exampleSelect.addEventListener("change", () => {
    if (exampleSelect.value !== CUSTOM) {
      selectExample(EXAMPLES[Number(exampleSelect.value)]);
    }
  });
  argsInput.addEventListener("input", markCustom);
  input.addEventListener("input", markCustom);
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    start();
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      start();
    }
  });
  stopButton.addEventListener("click", stop);
  element("open").addEventListener("click", () => fileInput.click());
  fileInput.addEventListener("change", async () => {
    const [file] = fileInput.files;
    if (file) {
      input.value = await file.text();
      markCustom();
    }
    fileInput.value = "";
  });
  if (navigator.clipboard) {
    copyButton.addEventListener("click", copy);
  } else {
    copyButton.hidden = true;
  }
  saveButton.addEventListener("click", () => download(output.textContent));
  setWrap(load(WRAP_KEY) === true);
  wrapButton.addEventListener("click", () => {
    const wrap = !output.classList.contains("wrap");
    setWrap(wrap);
    store(WRAP_KEY, wrap);
  });
}

initialize();
