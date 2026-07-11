import assert from "node:assert/strict";
import { join } from "node:path";
import test from "node:test";
import {
  checkPerformanceBudget,
  loadPerformanceBudgets,
  parseTenKMetrics,
} from "../lib/quality-budgets.mjs";

test("missing performance budget manifest fails closed", async () => {
  await assert.rejects(
    loadPerformanceBudgets(join(import.meta.dirname, "missing-performance-budgets.json")),
    /performance budget manifest is unavailable/,
  );
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
