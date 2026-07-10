import assert from "node:assert/strict";
import { existsSync, mkdtempSync, realpathSync, rmSync } from "node:fs";
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

test("owned execution starts the direct child as its process-group leader", async () => {
  const source = [
    'const { execFileSync } = require("node:child_process");',
    "const pgid = Number(execFileSync(",
    '  "/bin/ps",',
    '  ["-o", "pgid=", "-p", String(process.pid)],',
    '  { encoding: "utf8" },',
    ").trim());",
    "process.stdout.write(JSON.stringify({ pgid, pid: process.pid }));",
  ].join("\n");

  const { stdout } = await executeOwnedFile(
    process.execPath,
    ["-e", source],
    { encoding: "utf8", maxBuffer: 1_024, timeout: 5_000 },
  );
  const identity = JSON.parse(stdout);
  assert.equal(identity.pgid, identity.pid);
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
    if (pid < 0) {
      negativeSignals.push({ pid, signal });
      return true;
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
    assert.deepEqual(negativeSignals, []);
  } finally {
    process.kill = originalKill;
  }
});

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
