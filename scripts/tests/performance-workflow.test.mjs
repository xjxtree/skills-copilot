import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { workflowJobBody } from "../lib/performance-workflow.mjs";

test("macOS CI runs both live performance gates after build and before smoke", async () => {
  const workflow = await readFile(
    new URL("../../.github/workflows/ci.yml", import.meta.url),
    "utf8",
  );
  const macosJob = workflowJobBody(workflow, "macos-app");

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

test("macOS performance assertions ignore matching commands in later jobs", () => {
  const workflow = `
jobs:
  macos-app:
    runs-on: macos-latest
    steps:
      - run: pnpm build:macos
  later-job:
    runs-on: macos-latest
    steps:
      - run: pnpm benchmark:10k
      - run: pnpm benchmark:macos-list-model
      - run: pnpm smoke:macos-app -- --fixture-data --headless-sidecar
`;

  const macosJob = workflowJobBody(workflow, "macos-app");
  assert.match(macosJob, /run: pnpm build:macos/);
  assert.doesNotMatch(macosJob, /benchmark:10k/);
  assert.doesNotMatch(macosJob, /later-job:/);
});
