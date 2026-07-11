import { readFile } from "node:fs/promises";

export async function loadPerformanceBudgets(path) {
  let manifest;
  try {
    manifest = JSON.parse(await readFile(path, "utf8"));
  } catch (error) {
    throw new Error(`performance budget manifest is unavailable: ${path}`, {
      cause: error,
    });
  }

  if (
    manifest?.schema_version !== 1 ||
    !isPositiveNumber(manifest?.scan_10k?.max_elapsed_ms) ||
    !isPositiveNumber(manifest?.scan_10k?.max_rss_mb) ||
    !isPositiveNumber(manifest?.native_list?.max_p95_ms)
  ) {
    throw new Error(`performance budget manifest is invalid: ${path}`);
  }
  return manifest;
}

export function parseTenKMetrics(output) {
  const benchmark = output.match(
    /skills-copilot-bench scanned=(\d+) records=(\d+) budget_exhausted=(true|false) elapsed_ms=(\d+)/,
  );
  if (!benchmark) {
    throw new Error("missing skills-copilot-bench runtime metrics");
  }
  const rss = output.match(/benchmark-runtime: max_rss_mb=([0-9]+(?:\.[0-9]+)?)/);
  if (!rss) {
    throw new Error("missing benchmark-runtime max_rss_mb");
  }

  const metrics = {
    scanned: Number(benchmark[1]),
    records: Number(benchmark[2]),
    budgetExhausted: benchmark[3] === "true",
    elapsedMs: Number(benchmark[4]),
    maxRssMb: Number(rss[1]),
  };
  if (
    metrics.scanned !== 10_000 ||
    metrics.records !== 10_000 ||
    metrics.budgetExhausted
  ) {
    throw new Error(
      "expected scanned=10000 records=10000 budget_exhausted=false",
    );
  }
  return metrics;
}

export function checkPerformanceBudget(metrics, budget) {
  const errors = [];
  if (metrics.elapsedMs > budget.max_elapsed_ms) {
    errors.push(
      `elapsed_ms ${metrics.elapsedMs} exceeds ${budget.max_elapsed_ms}`,
    );
  }
  if (metrics.maxRssMb > budget.max_rss_mb) {
    errors.push(
      `max_rss_mb ${metrics.maxRssMb} exceeds ${budget.max_rss_mb}`,
    );
  }
  return errors;
}

export function effectiveMaximum(manifestMaximum, override, ci) {
  if (override === undefined || override === "") {
    return manifestMaximum;
  }
  const parsed = Number(override);
  if (!isPositiveNumber(parsed)) {
    throw new Error(`invalid performance budget override: ${override}`);
  }
  if (ci && parsed > manifestMaximum) {
    throw new Error(
      `performance budget override ${parsed} cannot loosen CI budget ${manifestMaximum}`,
    );
  }
  return parsed;
}

function isPositiveNumber(value) {
  return Number.isFinite(value) && value > 0;
}
