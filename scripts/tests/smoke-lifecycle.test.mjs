import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { executeOwnedFile } from "../lib/smoke-lifecycle.mjs";

async function captureFailure(run) {
  let failure;
  try {
    await run();
  } catch (error) {
    failure = error;
  }
  assert.ok(failure instanceof Error);
  return failure;
}

function processExists(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    if (error?.code === "ESRCH") {
      return false;
    }
    throw error;
  }
}

function processGroupExists(pgid) {
  try {
    process.kill(-pgid, 0);
    return true;
  } catch (error) {
    if (error?.code === "ESRCH") {
      return false;
    }
    throw error;
  }
}

async function waitForPath(path, timeoutMs = 10_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (existsSync(path)) {
      return;
    }
    await new Promise((resolveWait) => setTimeout(resolveWait, 5));
  }
  throw new Error(`timed out waiting for ${path}`);
}

async function waitForProcessExit(pid, timeoutMs = 1_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (!processExists(pid)) {
      return true;
    }
    await new Promise((resolveWait) => setTimeout(resolveWait, 10));
  }
  return !processExists(pid);
}

function writeFastExitTree(
  root,
  { descendantOutputBytes = 0, exitCode = 0 } = {},
) {
  const directInfoFile = join(root, "direct.json");
  const descendantInfoFile = join(root, "descendant.json");
  const descendantSource = join(root, "stubborn-descendant");
  const targetSource = join(root, "fast-exit-target");
  writeFileSync(
    descendantSource,
    [
      "#!/usr/bin/env node",
      'const { execFileSync } = require("node:child_process");',
      'const { writeFileSync } = require("node:fs");',
      "const pgid = Number(execFileSync(",
      '  "/bin/ps",',
      '  ["-o", "pgid=", "-p", String(process.pid)],',
      '  { encoding: "utf8" },',
      ").trim());",
      "writeFileSync(",
      "  process.env.SMOKE_DESCENDANT_INFO_FILE,",
      "  JSON.stringify({ pgid, pid: process.pid, ppid: process.ppid }),",
      ");",
      "const outputBytes = Number(process.env.SMOKE_DESCENDANT_OUTPUT_BYTES || 0);",
      "if (outputBytes > 0) {",
      "  setTimeout(() => process.stdout.write(Buffer.alloc(outputBytes, 120)), 50);",
      "}",
      'for (const signal of ["SIGHUP", "SIGINT", "SIGTERM"]) {',
      "  process.on(signal, () => {});",
      "}",
      "setInterval(() => {}, 60_000);",
      "",
    ].join("\n"),
  );
  chmodSync(descendantSource, 0o755);
  writeFileSync(
    targetSource,
    [
      "#!/usr/bin/env node",
      'const { execFileSync, spawn } = require("node:child_process");',
      'const { writeFileSync } = require("node:fs");',
      "const pgid = Number(execFileSync(",
      '  "/bin/ps",',
      '  ["-o", "pgid=", "-p", String(process.pid)],',
      '  { encoding: "utf8" },',
      ").trim());",
      "writeFileSync(",
      "  process.env.SMOKE_DIRECT_INFO_FILE,",
      "  JSON.stringify({ pgid, pid: process.pid, ppid: process.ppid }),",
      ");",
      "const descendant = spawn(",
      "  process.execPath,",
      "  [process.env.SMOKE_DESCENDANT_SOURCE],",
      '  { stdio: ["ignore", "inherit", "inherit"] },',
      ");",
      "descendant.unref();",
      `process.exit(${exitCode});`,
      "",
    ].join("\n"),
  );
  chmodSync(targetSource, 0o755);
  return {
    descendantInfoFile,
    descendantOutputBytes,
    descendantSource,
    directInfoFile,
    targetSource,
  };
}

async function runFastExitTree(mode) {
  const root = mkdtempSync(join(tmpdir(), "smoke-fast-exit-tree-"));
  const tree = writeFastExitTree(root, {
    descendantOutputBytes: mode === "maxBuffer" ? 4_096 : 0,
    exitCode: mode === "exit7" ? 7 : 0,
  });
  const controller = mode === "abort" ? new AbortController() : null;
  const execution = executeOwnedFile(tree.targetSource, [], {
    encoding: "utf8",
    env: {
      ...process.env,
      SMOKE_DESCENDANT_INFO_FILE: tree.descendantInfoFile,
      SMOKE_DESCENDANT_OUTPUT_BYTES: String(tree.descendantOutputBytes),
      SMOKE_DESCENDANT_SOURCE: tree.descendantSource,
      SMOKE_DIRECT_INFO_FILE: tree.directInfoFile,
    },
    maxBuffer: mode === "maxBuffer" ? 64 : 1_024,
    signal: controller?.signal,
    timeout: mode === "timeout" ? 1_500 : 0,
  }).then(
    (value) => ({ kind: "success", value }),
    (error) => ({ error, kind: "failure" }),
  );
  let direct;
  let descendant;
  let abortTimer;
  try {
    await Promise.all([
      waitForPath(tree.directInfoFile),
      waitForPath(tree.descendantInfoFile),
    ]);
    direct = JSON.parse(readFileSync(tree.directInfoFile, "utf8"));
    descendant = JSON.parse(readFileSync(tree.descendantInfoFile, "utf8"));
    if (controller) {
      abortTimer = setTimeout(
        () => controller.abort(new Error("injected fast-exit abort")),
        25,
      );
    }
    const observed = await Promise.race([
      execution,
      new Promise((resolveTimeout) =>
        setTimeout(() => resolveTimeout({ kind: "deadline" }), 3_500),
      ),
    ]);
    const directExited = await waitForProcessExit(direct.pid);
    const descendantExited = await waitForProcessExit(descendant.pid);
    return {
      descendant,
      descendantExited,
      direct,
      directExited,
      groupExists: processGroupExists(direct.pgid),
      observed,
    };
  } finally {
    clearTimeout(abortTimer);
    for (const pid of [direct?.pid, descendant?.pid]) {
      if (pid && processExists(pid)) {
        process.kill(pid, "SIGKILL");
        await waitForProcessExit(pid);
      }
    }
    await Promise.race([
      execution,
      new Promise((resolveTimeout) => setTimeout(resolveTimeout, 250)),
    ]);
    rmSync(root, { force: true, recursive: true });
  }
}

test("owned supervisor anchors the target process group", async () => {
  const source = [
    'const { execFileSync } = require("node:child_process");',
    "const pgid = Number(execFileSync(",
    '  "/bin/ps",',
    '  ["-o", "pgid=", "-p", String(process.pid)],',
    '  { encoding: "utf8" },',
    ").trim());",
    "process.stdout.write(JSON.stringify({ pgid, pid: process.pid, ppid: process.ppid }));",
  ].join("\n");

  const { stdout } = await executeOwnedFile(
    process.execPath,
    ["-e", source],
    { encoding: "utf8", maxBuffer: 1_024, timeout: 5_000 },
  );
  const identity = JSON.parse(stdout);
  assert.equal(identity.pgid, identity.ppid);
  assert.notEqual(identity.pgid, identity.pid);
});

test("owned execution preserves argv, cwd, and env without invoking a shell", async () => {
  const root = mkdtempSync(join(tmpdir(), "smoke-owned-exec-argv-"));
  const marker = join(root, "shell-marker");
  const argument = `$(touch ${marker}) ; literal value`;
  const source = [
    "process.stdout.write(JSON.stringify({",
    "  argument: process.argv[1],",
    "  cwd: process.cwd(),",
    "  env: process.env.SMOKE_OWNED_EXEC_TEST,",
    "}));",
  ].join("\n");

  try {
    const { stdout } = await executeOwnedFile(
      process.execPath,
      ["-e", source, argument],
      {
        cwd: root,
        encoding: "utf8",
        env: { ...process.env, SMOKE_OWNED_EXEC_TEST: "exact-value" },
        maxBuffer: 4_096,
        timeout: 5_000,
      },
    );
    assert.deepEqual(JSON.parse(stdout), {
      argument,
      cwd: realpathSync(root),
      env: "exact-value",
    });
    assert.equal(existsSync(marker), false);
  } finally {
    rmSync(root, { force: true, recursive: true });
  }
});

test("a natural exit 7 propagates output without signaling a stale process group", async () => {
  const originalKill = process.kill;
  const negativeSignals = [];
  process.kill = (pid, signal) => {
    if (pid < 0 && signal !== 0) {
      let groupExisted = true;
      try {
        originalKill.call(process, pid, 0);
      } catch (error) {
        groupExisted = error?.code !== "ESRCH";
      }
      negativeSignals.push({ groupExisted, pid, signal });
    }
    return originalKill.call(process, pid, signal);
  };

  try {
    const failure = await captureFailure(() =>
      executeOwnedFile(
        process.execPath,
        [
          "-e",
          'process.stdout.write("bounded-out"); process.stderr.write("bounded-err"); process.exit(7);',
        ],
        { encoding: "utf8", maxBuffer: 1_024, timeout: 5_000 },
      ),
    );
    assert.equal(failure.code, 7);
    assert.equal(failure.killed, false);
    assert.equal(failure.signal, null);
    assert.equal(failure.stdout, "bounded-out");
    assert.equal(failure.stderr, "bounded-err");
    assert.equal(
      negativeSignals.every((entry) => entry.groupExisted),
      true,
    );
  } finally {
    process.kill = originalKill;
  }
});

for (const mode of ["exit7", "timeout", "abort", "maxBuffer"]) {
  test(`fast-exit target with stubborn descendant settles for ${mode}`, async () => {
    const result = await runFastExitTree(mode);
    assert.equal(result.observed.kind, "failure");
    if (mode === "exit7") {
      assert.equal(result.observed.error.code, 7);
      assert.equal(result.observed.error.killed, false);
    } else if (mode === "timeout") {
      assert.equal(result.observed.error.timedOut, true);
      assert.equal(result.observed.error.killed, true);
    } else if (mode === "abort") {
      assert.equal(result.observed.error.name, "AbortError");
      assert.equal(result.observed.error.code, "ABORT_ERR");
    } else {
      assert.equal(
        result.observed.error.code,
        "ERR_CHILD_PROCESS_STDIO_MAXBUFFER",
      );
    }
    assert.equal(result.directExited, true);
    assert.equal(result.descendantExited, true);
    assert.equal(result.groupExists, false);
    assert.equal(result.direct.pgid, result.direct.ppid);
    assert.equal(result.descendant.pgid, result.direct.pgid);
  });
}

for (const stream of ["stdout", "stderr"]) {
  test(`${stream} maxBuffer is a hard bound with bounded diagnostics`, async () => {
    const source = `process.${stream}.write(Buffer.alloc(4_096, 120));`;
    const failure = await captureFailure(() =>
      executeOwnedFile(process.execPath, ["-e", source], {
        encoding: "utf8",
        maxBuffer: 64,
        timeout: 5_000,
      }),
    );

    assert.equal(failure.code, "ERR_CHILD_PROCESS_STDIO_MAXBUFFER");
    assert.ok(Buffer.byteLength(failure.stdout ?? "") <= 64);
    assert.ok(Buffer.byteLength(failure.stderr ?? "") <= 64);
    assert.ok(failure.message.length <= 512);
  });
}

test("timeout rejects within a bound and terminates the child", async () => {
  const started = Date.now();
  const failure = await captureFailure(() =>
    executeOwnedFile(
      process.execPath,
      ["-e", "setInterval(() => {}, 60_000);"],
      { encoding: "utf8", maxBuffer: 1_024, timeout: 50 },
    ),
  );

  assert.equal(failure.killed, true);
  assert.ok(Date.now() - started < 2_000);
});

test("AbortSignal rejects with AbortError and terminates the child", async () => {
  const controller = new AbortController();
  const reason = new Error("injected abort reason");
  const abort = setTimeout(() => controller.abort(reason), 50);
  try {
    const failure = await captureFailure(() =>
      executeOwnedFile(
        process.execPath,
        ["-e", "setInterval(() => {}, 60_000);"],
        {
          encoding: "utf8",
          maxBuffer: 1_024,
          signal: controller.signal,
          timeout: 5_000,
        },
      ),
    );
    assert.equal(failure.name, "AbortError");
    assert.equal(failure.code, "ABORT_ERR");
  } finally {
    clearTimeout(abort);
  }
});

test(
  "SIGTERM queued during a 25k-file rmSync cleanup keeps lifecycle hooks active",
  { timeout: 30_000 },
  async () => {
    const outer = mkdtempSync(join(tmpdir(), "smoke-cleanup-signal-"));
    const root = join(outer, "owned-root");
    const marker = join(outer, "cleanup-started");
    mkdirSync(root);
    for (let directoryIndex = 0; directoryIndex < 100; directoryIndex += 1) {
      const directory = join(root, `entries-${directoryIndex}`);
      mkdirSync(directory);
      for (let fileIndex = 0; fileIndex < 250; fileIndex += 1) {
        writeFileSync(join(directory, `entry-${fileIndex}`), "");
      }
    }

    const child = spawn(
      process.execPath,
      [
        join(
          process.cwd(),
          "scripts/tests/fixtures/smoke-lifecycle-cleanup-signal-child.mjs",
        ),
      ],
      {
        env: {
          ...process.env,
          SKILLS_COPILOT_CLEANUP_SIGNAL_MARKER: marker,
          SKILLS_COPILOT_CLEANUP_SIGNAL_ROOT: root,
        },
        stdio: ["ignore", "pipe", "pipe"],
      },
    );
    let stderr = "";
    child.stderr.setEncoding("utf8");
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });

    try {
      const [ready] = await once(child.stdout, "data");
      assert.equal(String(ready), "ready\n");
      await waitForPath(marker);
      assert.equal(existsSync(root), true, "cleanup completed before signal");
      const exit = once(child, "exit");
      assert.equal(child.kill("SIGTERM"), true);
      const [code, signal] = await Promise.race([
        exit,
        new Promise((_, reject) =>
          setTimeout(
            () => reject(new Error("cleanup signal child exit timeout")),
            5_000,
          ),
        ),
      ]);
      assert.equal(code, null);
      assert.equal(signal, "SIGTERM");
      assert.equal(existsSync(root), false);
      assert.equal(stderr, "");
    } finally {
      if (child.exitCode === null && child.signalCode === null) {
        child.kill("SIGKILL");
        await once(child, "exit");
      }
      rmSync(outer, { force: true, recursive: true });
    }
  },
);
