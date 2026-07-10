import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  validateCompletionLog,
  verifyNativeTestRegistry,
} from "../verify-macos-native-test-registry.mjs";

const repoRoot = fileURLToPath(new URL("../..", import.meta.url));

test("reports missing native model suites", () => {
  assert.deepEqual(
    verifyNativeTestRegistry({
      discoveredTypes: ["FindingDisplayModelTests", "SkillListModelTests"],
      registeredMainTypes: ["FindingDisplayModelTests"],
      serviceTypes: [],
      shardedType: "SkillStoreTests",
    }),
    ["unregistered native test types: SkillListModelTests"],
  );
});

test("reports duplicate registrations", () => {
  assert.deepEqual(
    verifyNativeTestRegistry({
      discoveredTypes: ["FindingDisplayModelTests"],
      registeredMainTypes: ["FindingDisplayModelTests", "FindingDisplayModelTests"],
      serviceTypes: [],
      shardedType: "SkillStoreTests",
    }),
    ["duplicate native test registrations: FindingDisplayModelTests"],
  );
});

test("requires the exact full-suite completion sentinel once", () => {
  const sentinel =
    "SkillsCopilotTests: full-suite-complete service=2 main=21 skill-store-groups=64 named=87";
  assert.deepEqual(validateCompletionLog(`${sentinel}\n`), []);
  assert.deepEqual(validateCompletionLog("partial output\n"), [
    "full-suite completion line must appear exactly once; found 0",
  ]);
  assert.deepEqual(validateCompletionLog(`${sentinel}\n${sentinel}\n`), [
    "full-suite completion line must appear exactly once; found 2",
  ]);
});

test("reports a missing canonical full-suite entrypoint", () => {
  assert.deepEqual(
    verifyNativeTestRegistry({
      discoveredTypes: ["FindingDisplayModelTests"],
      registeredMainTypes: ["FindingDisplayModelTests"],
      serviceTypes: [],
      shardedType: "SkillStoreTests",
      requiredHarnessType: "FullNativeModelSuiteTests",
    }),
    ["missing full-suite native test entrypoint: FullNativeModelSuiteTests"],
  );
});

test("importing the verifier has no CLI side effects", () => {
  const result = spawnSync(
    process.execPath,
    [
      "--input-type=module",
      "--eval",
      'await import("./scripts/verify-macos-native-test-registry.mjs")',
    ],
    { cwd: repoRoot, encoding: "utf8" },
  );
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout, "");
  assert.equal(result.stderr, "");
});
