#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { platform } from "node:os";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  checkPerformanceBudget,
  effectiveMaximum,
  loadPerformanceBudgets,
  parseTenKMetrics,
} from "./lib/quality-budgets.mjs";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const budgetPath = join(repoRoot, "scripts/performance-budgets.json");
const manifest = await loadPerformanceBudgets(budgetPath);
const budget = {
  max_elapsed_ms: effectiveMaximum(
    manifest.scan_10k.max_elapsed_ms,
    process.env.TEN_K_BENCH_MAX_ELAPSED_MS,
    process.env.CI === "true",
  ),
  max_rss_mb: effectiveMaximum(
    manifest.scan_10k.max_rss_mb,
    process.env.TEN_K_BENCH_MAX_RSS_MB,
    process.env.CI === "true",
  ),
};

const compile = run("cargo", [
  "test",
  "-p",
  "skills-copilot-commands",
  "benchmark_10k_scan_to_catalog",
  "--no-run",
  "--message-format=json",
]);
if (compile.stderr) process.stderr.write(compile.stderr);
if (compile.status !== 0) process.exit(compile.status ?? 1);

const candidates = compilerExecutables(compile.stdout);
const matches = [];
for (const executable of candidates) {
  const listed = run(executable, ["--list"]);
  if (listed.status !== 0) continue;
  for (const line of listed.stdout.split("\n")) {
    const testName = line.match(/^(.*::benchmark_10k_scan_to_catalog): test$/)?.[1];
    if (testName) matches.push({ executable, testName });
  }
}
if (matches.length !== 1) {
  throw new Error(`expected one 10k benchmark test executable, found ${matches.length}`);
}

const [{ executable, testName }] = matches;
const timeArgs = platform() === "darwin"
  ? ["-l", executable, testName, "--ignored", "--nocapture", "--exact"]
  : ["-v", executable, testName, "--ignored", "--nocapture", "--exact"];
const measured = run("/usr/bin/time", timeArgs);
printOutput(measured);
if (measured.status !== 0) process.exit(measured.status ?? 1);

const combined = `${measured.stdout}\n${measured.stderr}`;
const rssBytes = parseRssBytes(combined);
if (rssBytes === undefined) {
  throw new Error("missing timed test-binary maximum resident set size");
}
const maxRssMb = rssBytes / 1024 / 1024;
const normalized = `${combined}\nbenchmark-runtime: max_rss_mb=${maxRssMb.toFixed(1)}`;
const metrics = parseTenKMetrics(normalized);
const errors = checkPerformanceBudget(metrics, budget);

console.log(
  `benchmark: scanned=${metrics.scanned} records=${metrics.records} elapsed_ms=${metrics.elapsedMs} max_rss_mb=${metrics.maxRssMb.toFixed(1)}`,
);
console.log(
  `benchmark: budget max_elapsed_ms=${budget.max_elapsed_ms} max_rss_mb=${budget.max_rss_mb}`,
);
if (errors.length > 0) {
  for (const error of errors) console.error(`benchmark: ${error}`);
  process.exit(1);
}

function compilerExecutables(output) {
  const executables = new Set();
  for (const line of output.split("\n")) {
    let message;
    try {
      message = JSON.parse(line);
    } catch {
      continue;
    }
    if (
      message.reason !== "compiler-artifact" ||
      !message.package_id?.includes("skills-copilot-commands") ||
      !message.executable ||
      !message.target?.kind?.some((kind) => kind === "lib" || kind === "test")
    ) {
      continue;
    }
    executables.add(
      isAbsolute(message.executable)
        ? message.executable
        : resolve(repoRoot, message.executable),
    );
  }
  return [...executables];
}

function parseRssBytes(output) {
  if (platform() === "darwin") {
    const match = output.match(/(\d+)\s+maximum resident set size/);
    return match ? Number(match[1]) : undefined;
  }
  const match = output.match(/Maximum resident set size \(kbytes\):\s+(\d+)/);
  return match ? Number(match[1]) * 1024 : undefined;
}

function run(command, args) {
  return spawnSync(command, args, {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    env: {
      ...process.env,
      PATH: `${process.env.HOME}/.cargo/bin:${process.env.PATH}`,
    },
  });
}

function printOutput(result) {
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
}
