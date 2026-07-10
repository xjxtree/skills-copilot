import assert from "node:assert/strict";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  renameSync,
  rmSync,
  statSync,
  symlinkSync,
  unlinkSync,
  utimesSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  assertBoundedPathIdentityUnchanged,
  initializeAllocatedFixture,
  snapshotBoundedPathIdentity,
} from "../lib/smoke-fixture-safety.mjs";

function withTemporaryDirectory(prefix, run) {
  const root = mkdtempSync(join(tmpdir(), prefix));
  try {
    run(root);
  } finally {
    rmSync(root, { force: true, recursive: true });
  }
}

test("detects an in-place config rewrite while the parent directory mtime is preserved", () => {
  withTemporaryDirectory("smoke-tree-identity-", (root) => {
    const configRoot = join(root, "opencode");
    const configFile = join(configRoot, "opencode.json");
    mkdirSync(configRoot);
    writeFileSync(configFile, "secret-before");

    const stableTimestampSeconds = Math.floor(Date.now() / 1_000) - 60;
    utimesSync(configRoot, stableTimestampSeconds, stableTimestampSeconds);
    const parentMtimeBefore = statSync(configRoot).mtimeMs;
    const before = snapshotBoundedPathIdentity(configRoot);

    writeFileSync(configFile, "secret-after!");
    utimesSync(configRoot, stableTimestampSeconds, stableTimestampSeconds);
    assert.equal(statSync(configRoot).mtimeMs, parentMtimeBefore);

    let detected;
    try {
      assertBoundedPathIdentityUnchanged(before);
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
  test(`detects ${label} without exposing tree details`, () => {
    withTemporaryDirectory("smoke-tree-mutation-", (root) => {
      seed(root);
      const before = snapshotBoundedPathIdentity(root);
      mutate(root);

      assert.throws(
        () => assertBoundedPathIdentityUnchanged(before),
        /real opencode config changed/,
      );
    });
  });
}

test("bounds file reads and does not disclose the path or content", () => {
  withTemporaryDirectory("smoke-tree-bounds-", (root) => {
    const content = "private-config-content";
    writeFileSync(join(root, "opencode.json"), content);

    let failure;
    try {
      snapshotBoundedPathIdentity(root, {
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

test("does not follow a symlink outside the explicitly scoped config root", () => {
  withTemporaryDirectory("smoke-tree-symlink-scope-", (root) => {
    const configRoot = join(root, "opencode");
    const outside = join(root, "outside-private-config");
    mkdirSync(configRoot);
    writeFileSync(outside, "content larger than the four byte file bound");
    symlinkSync(outside, join(configRoot, "external-link"));

    const snapshot = snapshotBoundedPathIdentity(configRoot, {
      maxDepth: 4,
      maxEntries: 8,
      maxFileBytes: 4,
      maxTotalBytes: 8,
    });

    assert.equal(snapshot.exists, true);
  });
});

test("removes an allocated fixture root when initialization throws", () => {
  let allocatedRoot;
  const failure = new Error("injected fixture initialization failure");

  assert.throws(
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
