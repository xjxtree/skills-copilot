import assert from "node:assert/strict";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
  allocateCheckMacOSCaches,
  cargoCacheHealth,
  checkCacheKey,
} from "../check-macos-cache.mjs";

function temporaryRoot(t) {
  const root = mkdtempSync(join(tmpdir(), "agent-copilot-check-cache-test-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  return root;
}

test("shared caches are repository-scoped and reused after release", (t) => {
  const root = temporaryRoot(t);
  const repoRoot = join(root, "repo");
  const first = allocateCheckMacOSCaches({ repoRoot, tempRoot: root, env: {} });
  const cargoPath = first.cargoTargetDir;
  const swiftPath = first.swiftScratchRoot;
  assert.match(cargoPath, new RegExp(`target-${checkCacheKey(repoRoot)}$`));
  assert.match(swiftPath, new RegExp(`swift-${checkCacheKey(repoRoot)}$`));
  first.release();

  const second = allocateCheckMacOSCaches({ repoRoot, tempRoot: root, env: {} });
  assert.equal(second.cargoTargetDir, cargoPath);
  assert.equal(second.swiftScratchRoot, swiftPath);
  second.release();
});

test("parallel checks receive isolated caches that are removed on release", (t) => {
  const root = temporaryRoot(t);
  const repoRoot = join(root, "repo");
  const first = allocateCheckMacOSCaches({ repoRoot, tempRoot: root, env: {} });
  const second = allocateCheckMacOSCaches({ repoRoot, tempRoot: root, env: {} });

  assert.notEqual(second.cargoTargetDir, first.cargoTargetDir);
  assert.notEqual(second.swiftScratchRoot, first.swiftScratchRoot);
  assert.ok(existsSync(second.cargoTargetDir));
  assert.ok(existsSync(second.swiftScratchRoot));
  assert.ok(second.messages.some((message) => message.includes("isolated cargo")));

  second.release();
  assert.equal(existsSync(second.cargoTargetDir), false);
  assert.equal(existsSync(second.swiftScratchRoot), false);
  first.release();
});

test("an owned Cargo cache with missing bindgen output is rebuilt", (t) => {
  const root = temporaryRoot(t);
  const repoRoot = join(root, "repo");
  const first = allocateCheckMacOSCaches({ repoRoot, tempRoot: root, env: {} });
  const cargoPath = first.cargoTargetDir;
  first.release();

  const brokenOutput = join(
    cargoPath,
    "debug",
    "build",
    "libsqlite3-sys-broken",
    "out",
  );
  mkdirSync(brokenOutput, { recursive: true });
  writeFileSync(join(cargoPath, "stale-sentinel"), "stale", "utf8");
  assert.equal(cargoCacheHealth(cargoPath).healthy, false);

  const repaired = allocateCheckMacOSCaches({ repoRoot, tempRoot: root, env: {} });
  assert.equal(repaired.cargoTargetDir, cargoPath);
  assert.equal(existsSync(join(cargoPath, "stale-sentinel")), false);
  assert.equal(cargoCacheHealth(cargoPath).healthy, true);
  assert.ok(repaired.messages.some((message) => message.includes("Rebuilt unhealthy cargo")));
  repaired.release();
});

test("unowned shared paths are preserved and bypassed", (t) => {
  const root = temporaryRoot(t);
  const repoRoot = join(root, "repo");
  const cacheKey = checkCacheKey(repoRoot);
  const foreignCargo = join(root, `agent-copilot-check-v2-target-${cacheKey}`);
  mkdirSync(foreignCargo, { recursive: true });
  writeFileSync(join(foreignCargo, "keep-me"), "foreign", "utf8");

  const session = allocateCheckMacOSCaches({ repoRoot, tempRoot: root, env: {} });
  assert.notEqual(session.cargoTargetDir, foreignCargo);
  assert.equal(readFileSync(join(foreignCargo, "keep-me"), "utf8"), "foreign");
  assert.ok(session.messages.some((message) => message.includes("Ignored unowned cargo")));
  session.release();
  assert.equal(readFileSync(join(foreignCargo, "keep-me"), "utf8"), "foreign");
});

test("explicit cache overrides are never marked, repaired, or removed", (t) => {
  const root = temporaryRoot(t);
  const cargoOverride = join(root, "caller-cargo");
  const swiftOverride = join(root, "caller-swift");
  mkdirSync(cargoOverride);
  mkdirSync(swiftOverride);
  writeFileSync(join(cargoOverride, "sentinel"), "cargo", "utf8");
  writeFileSync(join(swiftOverride, "sentinel"), "swift", "utf8");

  const session = allocateCheckMacOSCaches({
    repoRoot: join(root, "repo"),
    tempRoot: root,
    env: {
      CARGO_TARGET_DIR: cargoOverride,
      SWIFTPM_SCRATCH_PATH: swiftOverride,
    },
  });
  assert.equal(session.cargoTargetDir, cargoOverride);
  assert.equal(session.swiftScratchRoot, swiftOverride);
  session.release();
  assert.equal(readFileSync(join(cargoOverride, "sentinel"), "utf8"), "cargo");
  assert.equal(readFileSync(join(swiftOverride, "sentinel"), "utf8"), "swift");
});
