import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { once } from "node:events";
import {
  existsSync,
  chmodSync,
  mkdirSync,
  mkdtempSync,
  renameSync,
  readdirSync,
  rmSync,
  statSync,
  symlinkSync,
  unlinkSync,
  utimesSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";

import {
  assertBoundedPathIdentityUnchanged,
  initializeAllocatedFixture,
  snapshotBoundedPathIdentity,
} from "../lib/smoke-fixture-safety.mjs";

async function withTemporaryDirectory(prefix, run) {
  const root = mkdtempSync(join(tmpdir(), prefix));
  try {
    await run(root);
  } finally {
    rmSync(root, { force: true, recursive: true });
  }
}

test("detects an in-place config rewrite while the parent directory mtime is preserved", async () => {
  await withTemporaryDirectory("smoke-tree-identity-", async (root) => {
    const configRoot = join(root, "opencode");
    const configFile = join(configRoot, "opencode.json");
    mkdirSync(configRoot);
    writeFileSync(configFile, "secret-before");

    const stableTimestampSeconds = Math.floor(Date.now() / 1_000) - 60;
    utimesSync(configRoot, stableTimestampSeconds, stableTimestampSeconds);
    const parentMtimeBefore = statSync(configRoot).mtimeMs;
    const before = await snapshotBoundedPathIdentity(configRoot);

    writeFileSync(configFile, "secret-after!");
    utimesSync(configRoot, stableTimestampSeconds, stableTimestampSeconds);
    assert.equal(statSync(configRoot).mtimeMs, parentMtimeBefore);

    let detected;
    try {
      await assertBoundedPathIdentityUnchanged(before);
    } catch (error) {
      detected = error;
    }
    assert.match(detected?.message ?? "", /real opencode config changed/);
    assert.equal(detected.message.includes(root), false);
    assert.equal(detected.message.includes("secret-before"), false);
    assert.equal(detected.message.includes("secret-after"), false);
    assert.equal(JSON.stringify(before).includes("secret-before"), false);
  });
});

for (const [label, seed, mutate] of [
  [
    "entry addition",
    () => {},
    (root) => writeFileSync(join(root, "added.json"), "added"),
  ],
  [
    "entry removal",
    (root) => writeFileSync(join(root, "removed.json"), "removed"),
    (root) => unlinkSync(join(root, "removed.json")),
  ],
  [
    "entry rename",
    (root) => writeFileSync(join(root, "before.json"), "same"),
    (root) => renameSync(join(root, "before.json"), join(root, "after.json")),
  ],
  [
    "entry type change",
    (root) => writeFileSync(join(root, "entry"), "file"),
    (root) => {
      unlinkSync(join(root, "entry"));
      mkdirSync(join(root, "entry"));
    },
  ],
  [
    "symlink target change",
    (root) => {
      writeFileSync(join(root, "first"), "same");
      writeFileSync(join(root, "second"), "same");
      symlinkSync("first", join(root, "config-link"));
    },
    (root) => {
      unlinkSync(join(root, "config-link"));
      symlinkSync("second", join(root, "config-link"));
    },
  ],
]) {
  test(`detects ${label} without exposing tree details`, async () => {
    await withTemporaryDirectory("smoke-tree-mutation-", async (root) => {
      seed(root);
      const before = await snapshotBoundedPathIdentity(root);
      mutate(root);

      await assert.rejects(
        () => assertBoundedPathIdentityUnchanged(before),
        /real opencode config changed/,
      );
    });
  });
}

test("bounds file reads and does not disclose the path or content", async () => {
  await withTemporaryDirectory("smoke-tree-bounds-", async (root) => {
    const content = "private-config-content";
    writeFileSync(join(root, "opencode.json"), content);

    let failure;
    try {
      await snapshotBoundedPathIdentity(root, {
        maxDepth: 4,
        maxEntries: 8,
        maxFileBytes: 4,
        maxTotalBytes: 8,
      });
    } catch (error) {
      failure = error;
    }

    assert.match(failure?.message ?? "", /bounded real opencode config snapshot/);
    assert.equal(failure.message.includes(root), false);
    assert.equal(failure.message.includes(content), false);
  });
});

for (const field of [
  "maxDepth",
  "maxEntries",
  "maxFileBytes",
  "maxTotalBytes",
]) {
  for (const invalid of [
    -1,
    Number.POSITIVE_INFINITY,
    Number.NaN,
    1.5,
    Number.MAX_SAFE_INTEGER + 1,
    "4",
  ]) {
    test(`rejects invalid ${field} bound ${String(invalid)}`, async () => {
      await withTemporaryDirectory("smoke-tree-invalid-bound-", async (root) => {
        await assert.rejects(
          () => snapshotBoundedPathIdentity(root, { [field]: invalid }),
          new RegExp(`invalid smoke snapshot bound ${field}`),
        );
      });
    });
  }
}

test("copies and freezes validated bounds", async () => {
  await withTemporaryDirectory("smoke-tree-frozen-bounds-", async (root) => {
    const customBounds = {
      maxDepth: 4,
      maxEntries: 8,
      maxFileBytes: 16,
      maxTotalBytes: 32,
    };
    const snapshot = await snapshotBoundedPathIdentity(root, customBounds);
    customBounds.maxEntries = Number.MAX_SAFE_INTEGER;

    assert.equal(Object.isFrozen(snapshot.bounds), true);
    assert.equal(snapshot.bounds.maxEntries, 8);
    assert.throws(() => {
      snapshot.bounds.maxEntries = Number.MAX_SAFE_INTEGER;
    }, TypeError);
  });
});

test("zero byte bounds can snapshot an empty regular file", async () => {
  await withTemporaryDirectory("smoke-tree-zero-byte-bound-", async (root) => {
    writeFileSync(join(root, "empty"), "");

    const snapshot = await snapshotBoundedPathIdentity(root, {
      maxDepth: 1,
      maxEntries: 2,
      maxFileBytes: 0,
      maxTotalBytes: 0,
    });

    assert.equal(snapshot.totalFileBytes, 0);
  });
});

test("does not follow a symlink outside the explicitly scoped config root", async () => {
  await withTemporaryDirectory("smoke-tree-symlink-scope-", async (root) => {
    const configRoot = join(root, "opencode");
    const outside = join(root, "outside-private-config");
    mkdirSync(configRoot);
    writeFileSync(outside, "content larger than the four byte file bound");
    symlinkSync(outside, join(configRoot, "external-link"));

    const snapshot = await snapshotBoundedPathIdentity(configRoot, {
      maxDepth: 4,
      maxEntries: 8,
      maxFileBytes: 4,
      maxTotalBytes: 8,
    });

    assert.equal(snapshot.exists, true);
  });
});

test("removes an allocated fixture root when initialization throws", async () => {
  let allocatedRoot;
  const failure = new Error("injected fixture initialization failure");

  await assert.rejects(
    () =>
      initializeAllocatedFixture({
        allocateRoot() {
          allocatedRoot = mkdtempSync(join(tmpdir(), "smoke-init-cleanup-"));
          return allocatedRoot;
        },
        initializeRoot(root) {
          mkdirSync(join(root, "partially-created"));
          writeFileSync(join(root, "partially-created", "state"), "partial");
          throw failure;
        },
      }),
    (error) => error === failure,
  );

  assert.equal(existsSync(allocatedRoot), false);
});

test(
  "native descriptor walker fails closed across file, directory, and FIFO swaps",
  { skip: process.platform !== "darwin" },
  async () => {
    await withTemporaryDirectory("smoke-native-race-test-", async (buildRoot) => {
      const binary = join(buildRoot, "smoke-fixture-identity-race-tests");
      const compile = spawnSync(
        "swiftc",
        [
          resolve(
            "scripts/lib/smoke-fixture-identity/Snapshot.swift",
          ),
          resolve(
            "scripts/tests/fixtures/smoke-fixture-identity-races/main.swift",
          ),
          "-o",
          binary,
        ],
        { encoding: "utf8", timeout: 30_000 },
      );
      assert.equal(
        compile.status,
        0,
        compile.error?.message || compile.stderr || compile.stdout,
      );

      const run = spawnSync(binary, [], {
        encoding: "utf8",
        timeout: 10_000,
      });
      assert.equal(
        run.status,
        0,
        run.error?.message || run.stderr || run.stdout,
      );
      assert.equal(
        run.stdout,
        "native fixture identity race tests passed\n",
      );
      assert.equal(run.stderr, "");
    });
  },
);

async function waitForHelperAllocation(helperTmp, child) {
  const deadline = Date.now() + 10_000;
  while (Date.now() < deadline) {
    const helpers = readdirSync(helperTmp).filter((name) =>
      name.startsWith("smoke-fixture-identity-helper-"),
    );
    if (helpers.length > 0) {
      assert.equal(helpers.length, 1);
      return;
    }
    if (child.exitCode !== null || child.signalCode !== null) {
      throw new Error("compile-window child exited before helper allocation");
    }
    await new Promise((resolveWait) => setTimeout(resolveWait, 20));
  }
  throw new Error("timed out waiting for helper allocation");
}

for (const signal of ["SIGHUP", "SIGINT", "SIGTERM"]) {
  test(
    `${signal} during helper compilation cleans the directory and preserves the signal`,
    { skip: process.platform !== "darwin", timeout: 20_000 },
    async () => {
      const root = mkdtempSync(join(tmpdir(), "smoke-helper-compile-signal-"));
      const configRoot = join(root, "config");
      const helperTmp = join(root, "helper-tmp");
      const fakeBin = join(root, "fake-bin");
      mkdirSync(configRoot);
      mkdirSync(helperTmp);
      mkdirSync(fakeBin);
      writeFileSync(join(configRoot, "opencode.json"), "{}\n");
      const fakeSwiftc = join(fakeBin, "swiftc");
      writeFileSync(
        fakeSwiftc,
        [
          "#!/bin/sh",
          "while kill -0 \"$PPID\" 2>/dev/null; do",
          "  /bin/sleep 0.1",
          "done",
          "exit 1",
          "",
        ].join("\n"),
      );
      chmodSync(fakeSwiftc, 0o755);

      const child = spawn(
        process.execPath,
        [
          resolve(
            "scripts/tests/fixtures/smoke-fixture-signal-child.mjs",
          ),
        ],
        {
          env: {
            ...process.env,
            PATH: `${fakeBin}:${process.env.PATH ?? ""}`,
            SKILLS_COPILOT_SIGNAL_TEST_ROOT: configRoot,
            TMPDIR: `${helperTmp}/`,
          },
          stdio: ["ignore", "ignore", "pipe"],
        },
      );
      let stderr = "";
      child.stderr.setEncoding("utf8");
      child.stderr.on("data", (chunk) => {
        stderr += chunk;
      });

      try {
        await waitForHelperAllocation(helperTmp, child);
        const exit = once(child, "exit");
        assert.equal(child.kill(signal), true);
        const [code, receivedSignal] = await exit;
        assert.equal(code, null);
        assert.equal(receivedSignal, signal);
        assert.equal(stderr, "");
        assert.deepEqual(readdirSync(helperTmp), []);
      } finally {
        if (child.exitCode === null && child.signalCode === null) {
          child.kill("SIGKILL");
          await once(child, "exit");
        }
        rmSync(root, { force: true, recursive: true });
      }
    },
  );
}

test(
  "signal termination removes the private helper and preserves SIGTERM",
  { skip: process.platform !== "darwin", timeout: 30_000 },
  async () => {
    const root = mkdtempSync(join(tmpdir(), "smoke-helper-signal-test-"));
    const configRoot = join(root, "config");
    const helperTmp = join(root, "helper-tmp");
    mkdirSync(configRoot);
    mkdirSync(helperTmp);
    writeFileSync(join(configRoot, "opencode.json"), "{}\n");

    const child = spawn(
      process.execPath,
      [
        resolve(
          "scripts/tests/fixtures/smoke-fixture-signal-child.mjs",
        ),
      ],
      {
        env: {
          ...process.env,
          SKILLS_COPILOT_SIGNAL_TEST_ROOT: configRoot,
          TMPDIR: `${helperTmp}/`,
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
      const readiness = Promise.race([
        once(child.stdout, "data"),
        once(child, "exit").then(([code, signal]) => {
          throw new Error(
            `signal child exited before readiness: code=${code} signal=${signal}`,
          );
        }),
        new Promise((_, reject) => {
          setTimeout(
            () => reject(new Error("signal child readiness timeout")),
            15_000,
          ).unref();
        }),
      ]);
      const [readyChunk] = await readiness;
      assert.equal(String(readyChunk), "ready\n");
      assert.equal(
        readdirSync(helperTmp).filter((name) =>
          name.startsWith("smoke-fixture-identity-helper-"),
        ).length,
        1,
      );

      const exit = once(child, "exit");
      assert.equal(child.kill("SIGTERM"), true);
      const [code, signal] = await exit;
      assert.equal(code, null);
      assert.equal(signal, "SIGTERM");
      assert.equal(stderr, "");
      assert.deepEqual(readdirSync(helperTmp), []);
    } finally {
      if (child.exitCode === null && child.signalCode === null) {
        child.kill("SIGKILL");
        await once(child, "exit");
      }
      rmSync(root, { force: true, recursive: true });
    }
  },
);
