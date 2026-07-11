import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("macOS CI runs both live performance gates after build and before smoke", async () => {
  const workflow = await readFile(
    new URL("../../.github/workflows/ci.yml", import.meta.url),
    "utf8",
  );
  const jobStart = workflow.indexOf("\n  macos-app:\n");
  assert.notEqual(jobStart, -1, "ci.yml must contain the macos-app job");
  const macosJob = workflow.slice(jobStart);

  const build = macosJob.indexOf("run: pnpm build:macos");
  const tenK = macosJob.indexOf("run: pnpm benchmark:10k");
  const nativeList = macosJob.indexOf("run: pnpm benchmark:macos-list-model");
  const smoke = macosJob.indexOf(
    "run: pnpm smoke:macos-app -- --fixture-data --headless-sidecar",
  );

  assert.ok(build >= 0, "macos-app must build the app before live benchmarks");
  assert.ok(tenK > build, "10k benchmark must run after the macOS build/tests");
  assert.ok(
    nativeList > tenK,
    "native list benchmark must run after the 10k benchmark",
  );
  assert.ok(smoke > nativeList, "fixture smoke must run after live benchmarks");
  assert.equal(
    macosJob.match(/run: pnpm benchmark:10k/g)?.length,
    1,
    "10k live gate must appear exactly once in macos-app",
  );
  assert.equal(
    macosJob.match(/run: pnpm benchmark:macos-list-model/g)?.length,
    1,
    "native list live gate must appear exactly once in macos-app",
  );
});
