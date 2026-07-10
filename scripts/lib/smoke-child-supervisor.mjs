#!/usr/bin/env node

import { spawn } from "node:child_process";
import { closeSync, writeSync } from "node:fs";

const controlFd = 3;
const payload = JSON.parse(process.argv[2] ?? "null");
if (
  !payload ||
  typeof payload.command !== "string" ||
  !Array.isArray(payload.args)
) {
  process.exit(2);
}

let reported = false;

function bounded(value, maximum = 512) {
  return String(value ?? "").slice(0, maximum);
}

function report(message) {
  if (reported) {
    return;
  }
  reported = true;
  try {
    writeSync(controlFd, `${JSON.stringify(message)}\n`);
  } catch {
    // The owner may already be terminating the complete process group.
  }
}

let target;
try {
  target = spawn(payload.command, payload.args, {
    argv0: payload.argv0 ?? undefined,
    shell: false,
    stdio: ["ignore", "inherit", "inherit"],
  });
} catch (error) {
  report({
    error: {
      code: error?.code ?? null,
      errno: error?.errno ?? null,
      message: bounded(error?.message),
      name: bounded(error?.name, 64),
      path: bounded(error?.path),
      syscall: bounded(error?.syscall, 128),
    },
    type: "error",
  });
}

try {
  closeSync(1);
} catch {
  // The target already inherited its own copy of the output descriptor.
}
try {
  closeSync(2);
} catch {
  // The target already inherited its own copy of the output descriptor.
}

if (target) {
  target.once("error", (error) => {
    report({
      error: {
        code: error?.code ?? null,
        errno: error?.errno ?? null,
        message: bounded(error?.message),
        name: bounded(error?.name, 64),
        path: bounded(error?.path),
        syscall: bounded(error?.syscall, 128),
      },
      type: "error",
    });
  });
  target.once("exit", (code, signal) => {
    report({ code, signal, type: "exit" });
  });
}

setInterval(() => {}, 60_000);
