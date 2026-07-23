#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const modulePath = fileURLToPath(import.meta.url);
const repoRoot = resolve(dirname(modulePath), "..");
const runnerPath = join(
  repoRoot,
  "apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift",
);
const serviceTypes = ["ServiceClientProcessTests", "ServiceClientRPCTests"];
const shardedType = "SkillStoreTests";
const excludedHarnessType = "FullNativeModelSuiteTests";
const expectedCompletionLine =
  "SkillsCopilotTests: full-suite-complete service=2 main=28 skill-store-groups=64 named=94";

function duplicates(values) {
  const seen = new Set();
  const repeated = new Set();
  for (const value of values) {
    if (seen.has(value)) repeated.add(value);
    seen.add(value);
  }
  return [...repeated].sort();
}

export function verifyNativeTestRegistry({
  discoveredTypes,
  registeredMainTypes,
  serviceTypes: registeredServiceTypes = serviceTypes,
  shardedType: registeredShardedType = shardedType,
  requiredHarnessType,
}) {
  const errors = [];
  if (requiredHarnessType && !discoveredTypes.includes(requiredHarnessType)) {
    errors.push(`missing full-suite native test entrypoint: ${requiredHarnessType}`);
  }
  const duplicateMainTypes = duplicates(registeredMainTypes);
  if (duplicateMainTypes.length > 0) {
    errors.push(`duplicate native test registrations: ${duplicateMainTypes.join(", ")}`);
  }

  const excluded = new Set([
    ...registeredServiceTypes,
    registeredShardedType,
    excludedHarnessType,
  ]);
  const expectedMainTypes = [...new Set(discoveredTypes)]
    .filter((type) => !excluded.has(type))
    .sort();
  const registered = new Set(registeredMainTypes);
  const expected = new Set(expectedMainTypes);
  const missing = expectedMainTypes.filter((type) => !registered.has(type));
  const extra = [...registered]
    .filter((type) => !expected.has(type))
    .sort();

  if (missing.length > 0) {
    errors.push(`unregistered native test types: ${missing.join(", ")}`);
  }
  if (extra.length > 0) {
    errors.push(`unknown native test registrations: ${extra.join(", ")}`);
  }
  return errors;
}

export function discoverNativeTestTypes(entries) {
  const discovered = [];
  const declarationPattern = /^(?:struct|final class|class)\s+([A-Za-z0-9]+Tests)\b/gm;
  for (const { source } of entries) {
    for (const match of source.matchAll(declarationPattern)) {
      discovered.push(match[1]);
    }
  }
  return discovered;
}

export function registeredMainTypes(source) {
  const start = source.indexOf("private let mainNativeModelSuites:");
  const end = source.indexOf("\n]", start);
  if (start < 0 || end < 0) return [];
  const block = source.slice(start, end + 2);
  return [...block.matchAll(/^\s*\("([A-Za-z0-9]+Tests)"\s*,/gm)].map(
    (match) => match[1],
  );
}

export function validateCompletionLog(log) {
  const matches = log
    .split(/\r?\n/)
    .filter((line) => line === expectedCompletionLine).length;
  return matches === 1
    ? []
    : [`full-suite completion line must appear exactly once; found ${matches}`];
}

function trackedNativeTestFiles() {
  const output = execFileSync(
    "git",
    ["ls-files", "-z", "--", "apps/macos/Tests/SkillsCopilotTests/*.swift"],
    { cwd: repoRoot },
  ).toString("utf8");
  return output.split("\0").filter(Boolean);
}

function parseArgs(argv) {
  let logPath;
  for (let index = 0; index < argv.length; index += 1) {
    const value = argv[index];
    if (value === "--") continue;
    if (value === "--log") {
      logPath = argv[index + 1];
      if (!logPath) throw new Error("--log requires an absolute path");
      index += 1;
      continue;
    }
    throw new Error(`unknown argument: ${value}`);
  }
  if (logPath && !isAbsolute(logPath)) {
    throw new Error("--log requires an absolute path");
  }
  return { logPath };
}

function main(argv) {
  const { logPath } = parseArgs(argv);
  const testFiles = trackedNativeTestFiles();
  const entries = testFiles.map((path) => ({
    path,
    source: readFileSync(join(repoRoot, path), "utf8"),
  }));
  const runnerSource = readFileSync(runnerPath, "utf8");
  const discoveredTypes = discoverNativeTestTypes(entries);
  const mainTypes = registeredMainTypes(runnerSource);
  const errors = verifyNativeTestRegistry({
    discoveredTypes,
    registeredMainTypes: mainTypes,
    serviceTypes,
    shardedType,
    requiredHarnessType: excludedHarnessType,
  });

  for (const expectedServiceType of serviceTypes) {
    if (!discoveredTypes.includes(expectedServiceType)) {
      errors.push(`missing service native test type: ${expectedServiceType}`);
    }
  }
  if (!discoveredTypes.includes(shardedType)) {
    errors.push(`missing sharded native test type: ${shardedType}`);
  }
  if (mainTypes.length !== 28) {
    errors.push(`main native test suite count differs: expected 28; actual ${mainTypes.length}`);
  }

  if (logPath) {
    const absoluteLogPath = resolve(logPath);
    const log = readFileSync(absoluteLogPath, "utf8");
    for (const error of validateCompletionLog(log)) {
      errors.push(`${relative(repoRoot, absoluteLogPath) || absoluteLogPath}: ${error}`);
    }
  }

  if (errors.length > 0) {
    for (const error of errors) console.error(error);
    process.exitCode = 1;
    return;
  }
  console.log(
    `macOS native test registry verification passed: service=2 main=${mainTypes.length} skill-store-groups=64 named=${2 + mainTypes.length + 64}`,
  );
}

if (process.argv[1] && resolve(process.argv[1]) === modulePath) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
