#!/usr/bin/env node

import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const repoRoot = resolve(new URL("..", import.meta.url).pathname);
const trendsPath = join(repoRoot, "docs", "verification", "benchmark-trends.md");
const packageJsonPath = join(repoRoot, "package.json");

function fail(message) {
  console.error(`benchmark trend verification failed: ${message}`);
  process.exit(1);
}

function readRequired(path) {
  if (!existsSync(path)) {
    fail(`missing required file: ${path}`);
  }
  return readFileSync(path, "utf8");
}

const trends = readRequired(trendsPath);
const packageJson = readRequired(packageJsonPath);

for (const snippet of [
  "# Benchmark Trends",
  "Current Baselines",
  "Large catalog scan",
  "`pnpm benchmark:10k`",
  "10,000 synthetic command-catalog records",
  "Native list model",
  "`pnpm benchmark:macos-list-model`",
  "Maintenance",
]) {
  if (!trends.includes(snippet)) {
    fail(`docs/verification/benchmark-trends.md missing required benchmark trend text: ${snippet}`);
  }
}

for (const scriptName of [
  '"benchmark:10k"',
  '"benchmark:macos-list-model"',
  '"verify:benchmark-trends"',
]) {
  if (!packageJson.includes(scriptName)) {
    fail(`package.json missing script reference: ${scriptName}`);
  }
}

if (trends.includes("Pending reproducible fixture benchmark")) {
  fail("docs/verification/benchmark-trends.md still contains pending benchmark placeholders");
}

console.log("benchmark trend verification passed");
