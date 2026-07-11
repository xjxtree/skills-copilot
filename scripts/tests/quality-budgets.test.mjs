import assert from "node:assert/strict";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  checkPerformanceBudget,
  effectiveMaximum,
  effectiveSampleCount,
  loadPerformanceBudgets,
  parseTenKMetrics,
} from "../lib/quality-budgets.mjs";

test("missing performance budget manifest fails closed", async () => {
  await assert.rejects(
    loadPerformanceBudgets(join(import.meta.dirname, "missing-performance-budgets.json")),
    /performance budget manifest is unavailable/,
  );
});

test("malformed performance budget manifest fails closed", async () => {
  const directory = await mkdtemp(join(tmpdir(), "agent-copilot-budget-test-"));
  const path = join(directory, "performance-budgets.json");
  try {
    await writeFile(path, JSON.stringify({
      schema_version: 1,
      scan_10k: { max_elapsed_ms: 8000, max_rss_mb: 640 },
      native_list: { max_p95_ms: 80 },
    }));
    await assert.rejects(
      loadPerformanceBudgets(path),
      /performance budget manifest is invalid/,
    );
  } finally {
    await rm(directory, { force: true, recursive: true });
  }
});

test("parses exact 10k runtime metrics", () => {
  const metrics = parseTenKMetrics([
    "skills-copilot-bench scanned=10000 records=10000 budget_exhausted=false elapsed_ms=3460 elapsed_s=3.460",
    "benchmark-runtime: max_rss_mb=404.5",
  ].join("\n"));

  assert.deepEqual(metrics, {
    scanned: 10_000,
    records: 10_000,
    budgetExhausted: false,
    elapsedMs: 3460,
    maxRssMb: 404.5,
  });
});

test("rejects non-exact 10k counts", () => {
  assert.throws(
    () => parseTenKMetrics([
      "skills-copilot-bench scanned=9999 records=9999 budget_exhausted=false elapsed_ms=3460 elapsed_s=3.460",
      "benchmark-runtime: max_rss_mb=404.5",
    ].join("\n")),
    /expected scanned=10000 records=10000 budget_exhausted=false/,
  );
});

test("rejects exhausted 10k source budget", () => {
  assert.throws(
    () => parseTenKMetrics([
      "skills-copilot-bench scanned=10000 records=10000 budget_exhausted=true elapsed_ms=3460 elapsed_s=3.460",
      "benchmark-runtime: max_rss_mb=404.5",
    ].join("\n")),
    /expected scanned=10000 records=10000 budget_exhausted=false/,
  );
});

test("rejects missing benchmark RSS", () => {
  assert.throws(
    () => parseTenKMetrics(
      "skills-copilot-bench scanned=10000 records=10000 budget_exhausted=false elapsed_ms=3460 elapsed_s=3.460",
    ),
    /missing benchmark-runtime max_rss_mb/,
  );
});

test("rejects elapsed and RSS overages", () => {
  assert.deepEqual(
    checkPerformanceBudget(
      { elapsedMs: 8001, maxRssMb: 640.1 },
      { max_elapsed_ms: 8000, max_rss_mb: 640 },
    ),
    ["elapsed_ms 8001 exceeds 8000", "max_rss_mb 640.1 exceeds 640"],
  );
});

test("accepts exact counts within elapsed and RSS budgets", () => {
  const metrics = parseTenKMetrics([
    "skills-copilot-bench scanned=10000 records=10000 budget_exhausted=false elapsed_ms=7999 elapsed_s=7.999",
    "benchmark-runtime: max_rss_mb=639.9",
  ].join("\n"));

  assert.deepEqual(
    checkPerformanceBudget(metrics, { max_elapsed_ms: 8000, max_rss_mb: 640 }),
    [],
  );
});

test("CI performance maximum overrides can only tighten", () => {
  assert.equal(effectiveMaximum(80, undefined, true), 80);
  assert.equal(effectiveMaximum(80, "70", true), 70);
  assert.throws(
    () => effectiveMaximum(80, "81", true),
    /cannot loosen CI budget/,
  );
  for (const value of ["0", "-1", "NaN"]) {
    assert.throws(
      () => effectiveMaximum(80, value, true),
      /invalid performance budget override/,
    );
  }
});

test("native benchmark samples default to checked-in minimums", () => {
  assert.equal(effectiveSampleCount(80, undefined, true, "iterations"), 80);
  assert.equal(effectiveSampleCount(12, undefined, true, "warmups"), 12);
});

test("CI rejects invalid or reduced native benchmark samples", () => {
  for (const value of ["0", "-1", "NaN", "79"]) {
    assert.throws(
      () => effectiveSampleCount(80, value, true, "iterations"),
      /native benchmark iterations override/,
    );
  }
  assert.equal(effectiveSampleCount(80, "81", true, "iterations"), 81);
});
