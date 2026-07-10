import assert from "node:assert/strict";
import { Buffer } from "node:buffer";
import { spawnSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  filesystemIdentity,
  sameFilesystemEntry,
} from "../lib/path-identity.mjs";
import test from "node:test";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const shellHelper = join(repoRoot, "script", "path_identity.sh");

function withPathFixtures(run) {
  const root = mkdtempSync(join(tmpdir(), "agent-copilot-path-identity-"));
  const real = join(root, "real directory");
  const alias = join(root, "alias directory");
  const distinct = join(root, "distinct directory");
  const realFile = join(root, "real file");
  const fileAlias = join(root, "file alias");
  mkdirSync(real);
  mkdirSync(distinct);
  symlinkSync(real, alias, "dir");
  writeFileSync(realFile, "fixture");
  symlinkSync(realFile, fileAlias, "file");

  try {
    run({ root, real, alias, distinct, realFile, fileAlias });
  } finally {
    rmSync(root, { force: true, recursive: true });
  }
}

function runShellComparison(left, right) {
  return spawnSync(
    "bash",
    [
      "-c",
      'source "$1"\nsame_filesystem_entry "$2" "$3"',
      "path-identity-test",
      shellHelper,
      left,
      right,
    ],
    { encoding: "utf8" },
  );
}

function assertShellComparison(left, right, expectedStatus) {
  const result = runShellComparison(left, right);
  assert.equal(
    result.status,
    expectedStatus,
    `unexpected comparison for ${JSON.stringify(left)} and ${JSON.stringify(right)}: ${result.stderr}`,
  );
  assert.equal(result.stderr, "");
}

function shellIdentity(path) {
  return spawnSync(
    "bash",
    [
      "-c",
      'source "$1"\nfilesystem_identity "$2"',
      "path-identity-test",
      shellHelper,
      path,
    ],
    { encoding: "utf8" },
  );
}

function fallbackIdentity(path) {
  return `path:${Buffer.from(resolve(path)).toString("hex")}`;
}

test("Node follows file and directory aliases when comparing filesystem identity", () => {
  withPathFixtures(({ real, alias, distinct, realFile, fileAlias }) => {
    assert.notEqual(real, alias);
    assert.equal(filesystemIdentity(real), filesystemIdentity(alias));
    assert.notEqual(filesystemIdentity(real), filesystemIdentity(distinct));
    assert.equal(filesystemIdentity(realFile), filesystemIdentity(fileAlias));
    assert.equal(sameFilesystemEntry(real, alias), true);
    assert.equal(sameFilesystemEntry(real, distinct), false);
    assert.equal(sameFilesystemEntry(realFile, fileAlias), true);
  });
});

test("Node compares missing paths by normalized absolute path", () => {
  withPathFixtures(({ root }) => {
    const missing = join(root, "missing entry");
    const normalizedAlias = join(root, "missing parent", "..", "missing entry");
    const distinctMissing = join(root, "other missing entry");

    assert.equal(sameFilesystemEntry(missing, missing), true);
    assert.equal(sameFilesystemEntry(missing, normalizedAlias), true);
    assert.equal(sameFilesystemEntry(missing, distinctMissing), false);
  });
});

test("shell helpers follow aliases and fall back for missing paths", () => {
  withPathFixtures(({ root, real, alias, distinct, realFile, fileAlias }) => {
    const missing = join(root, "missing entry");
    const normalizedAlias = join(root, "missing parent", "..", "missing entry");
    const distinctMissing = join(root, "other missing entry");

    const realIdentity = shellIdentity(real);
    const aliasIdentity = shellIdentity(alias);
    const missingIdentity = shellIdentity(missing);
    assert.equal(realIdentity.status, 0, realIdentity.stderr);
    assert.equal(aliasIdentity.status, 0, aliasIdentity.stderr);
    assert.equal(missingIdentity.status, 0, missingIdentity.stderr);
    assert.equal(realIdentity.stdout, aliasIdentity.stdout);
    assert.match(missingIdentity.stdout, /^path:/);

    assertShellComparison(real, alias, 0);
    assertShellComparison(real, distinct, 1);
    assertShellComparison(realFile, fileAlias, 0);
    assertShellComparison(missing, missing, 0);
    assertShellComparison(missing, normalizedAlias, 0);
    assertShellComparison(missing, distinctMissing, 1);
  });
});

test("fallback identities preserve special path bytes without collisions", () => {
  withPathFixtures(({ root }) => {
    const specialName = "line one\nline two\t*?[]'\"$`\\\n";
    const special = join(root, specialName);
    const normalizedAlias = join(root, "missing parent", "..", specialName);
    const collisionPairs = [
      [join(root, "missing\none"), join(root, "missing\ntwo")],
      [join(root, "missing"), join(root, "missing\n")],
    ];

    for (const path of [special, normalizedAlias, ...collisionPairs.flat()]) {
      const expected = fallbackIdentity(path);
      assert.match(expected, /^path:[0-9a-f]+$/);
      assert.equal(filesystemIdentity(path), expected);

      const shellResult = shellIdentity(path);
      assert.equal(shellResult.status, 0, shellResult.stderr);
      assert.equal(shellResult.stderr, "");
      assert.equal(shellResult.stdout, `${expected}\n`);
    }

    assert.equal(sameFilesystemEntry(special, normalizedAlias), true);
    assertShellComparison(special, normalizedAlias, 0);
    for (const [left, right] of collisionPairs) {
      assert.equal(sameFilesystemEntry(left, right), false);
      assertShellComparison(left, right, 1);
    }
  });
});
