#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { lstatSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  collectDeclaredCreatePaths,
  collectHeadingSlugs,
  collectMarkdownReferences,
  parseGateMembers,
  validateGateMembers,
  validateReferences,
} from "./lib/repository-governance.mjs";

const scriptPath = fileURLToPath(import.meta.url);
const scriptDir = dirname(scriptPath);
const repoRoot = resolve(scriptDir, "..");
const manifestPath = join(scriptDir, "repository-governance.json");
const MANIFEST_LABEL = "scripts/repository-governance.json";
const MANIFEST_KEYS = [
  "schema_version",
  "policy_documents",
  "required_text",
  "forbidden_patterns",
  "forbidden_paths",
  "gate",
];
const GATE_KEYS = ["script", "members"];
const SAFE_GIT_CONFIG = [
  "-c",
  "core.fsmonitor=false",
  "-c",
  "core.untrackedCache=false",
  "-c",
  "core.excludesFile=/dev/null",
];

function isPlainObject(value) {
  return (
    value !== null &&
    typeof value === "object" &&
    !Array.isArray(value) &&
    (Object.getPrototypeOf(value) === Object.prototype ||
      Object.getPrototypeOf(value) === null)
  );
}

function validateExactKeys(value, expected, label, errors) {
  const keys = Object.keys(value);
  for (const key of expected) {
    if (!Object.hasOwn(value, key)) {
      errors.push(`${label} is missing required key: ${key}`);
    }
  }
  for (const key of keys.filter((key) => !expected.includes(key)).sort()) {
    errors.push(`${label} has unexpected key: ${key}`);
  }
}

function validateStringArray(value, label, errors, { nonempty = false } = {}) {
  if (
    !Array.isArray(value) ||
    (nonempty && value.length === 0) ||
    value.some((entry) => typeof entry !== "string" || !entry.trim())
  ) {
    errors.push(`${label} must be an array of nonempty strings`);
    return false;
  }

  const seen = new Set();
  for (const entry of value) {
    if (seen.has(entry)) {
      errors.push(`${label} contains duplicate value: ${entry}`);
    } else {
      seen.add(entry);
    }
  }
  return true;
}

export function validateGovernanceManifest(manifest) {
  const errors = [];
  if (!isPlainObject(manifest)) {
    return [`${MANIFEST_LABEL} must be a plain object`];
  }

  validateExactKeys(manifest, MANIFEST_KEYS, MANIFEST_LABEL, errors);
  if (
    Object.hasOwn(manifest, "schema_version") &&
    manifest.schema_version !== 1
  ) {
    errors.push(
      `${MANIFEST_LABEL} has unsupported schema_version ${String(manifest.schema_version)}`,
    );
  }
  if (Object.hasOwn(manifest, "policy_documents")) {
    validateStringArray(
      manifest.policy_documents,
      `${MANIFEST_LABEL}.policy_documents`,
      errors,
      { nonempty: true },
    );
  }
  if (Object.hasOwn(manifest, "required_text")) {
    if (!isPlainObject(manifest.required_text)) {
      errors.push(`${MANIFEST_LABEL}.required_text must be a plain object`);
    } else {
      for (const relativePath of Object.keys(manifest.required_text).sort()) {
        if (!relativePath.trim()) {
          errors.push(`${MANIFEST_LABEL}.required_text has an empty document path`);
          continue;
        }
        validateStringArray(
          manifest.required_text[relativePath],
          `${MANIFEST_LABEL}.required_text.${relativePath}`,
          errors,
          { nonempty: true },
        );
      }
    }
  }
  for (const key of ["forbidden_patterns", "forbidden_paths"]) {
    if (Object.hasOwn(manifest, key)) {
      validateStringArray(manifest[key], `${MANIFEST_LABEL}.${key}`, errors);
    }
  }
  if (Object.hasOwn(manifest, "gate")) {
    if (!isPlainObject(manifest.gate)) {
      errors.push(`${MANIFEST_LABEL}.gate must be a plain object`);
    } else {
      validateExactKeys(manifest.gate, GATE_KEYS, `${MANIFEST_LABEL}.gate`, errors);
      if (
        Object.hasOwn(manifest.gate, "script") &&
        (typeof manifest.gate.script !== "string" || !manifest.gate.script.trim())
      ) {
        errors.push(`${MANIFEST_LABEL}.gate.script must be a nonempty string`);
      }
      if (Object.hasOwn(manifest.gate, "members")) {
        validateStringArray(
          manifest.gate.members,
          `${MANIFEST_LABEL}.gate.members`,
          errors,
          { nonempty: true },
        );
      }
    }
  }
  return errors;
}

function controlledGitEnvironment() {
  const environment = {};
  for (const [key, value] of Object.entries(process.env)) {
    if (!/^GIT_/u.test(key)) environment[key] = value;
  }
  return {
    ...environment,
    GIT_CONFIG_GLOBAL: "/dev/null",
    GIT_CONFIG_NOSYSTEM: "1",
    GIT_CONFIG_SYSTEM: "/dev/null",
    GIT_NO_LAZY_FETCH: "1",
    GIT_NO_REPLACE_OBJECTS: "1",
    GIT_OPTIONAL_LOCKS: "0",
  };
}

function runGit(args) {
  return execFileSync(
    "git",
    ["-C", repoRoot, ...SAFE_GIT_CONFIG, ...args],
    {
      cwd: repoRoot,
      encoding: "utf8",
      env: controlledGitEnvironment(),
      maxBuffer: 32 * 1024 * 1024,
    },
  );
}

function trackedEntries() {
  const topLevel = runGit(["rev-parse", "--show-toplevel"]).trim();
  if (resolve(topLevel) !== repoRoot) {
    throw new Error("Git repository root does not match the verifier root");
  }
  const output = runGit(["ls-files", "--stage", "-z"]);
  return output
    .split("\0")
    .filter(Boolean)
    .map((record) => {
      const separator = record.indexOf("\t");
      const header = separator === -1 ? "" : record.slice(0, separator);
      const relativePath = separator === -1 ? "" : record.slice(separator + 1);
      const match = header.match(/^(\d{6}) ([0-9a-f]+) ([0-3])$/u);
      if (!match || !relativePath) {
        throw new Error("git ls-files returned an invalid stage record");
      }
      return {
        mode: match[1],
        object: match[2],
        stage: Number.parseInt(match[3], 10),
        relativePath,
      };
    });
}

function pathExists(relativePath, trackedFiles) {
  if (trackedFiles.has(relativePath)) return true;
  const prefix = relativePath.endsWith("/")
    ? relativePath
    : `${relativePath}/`;
  for (const tracked of trackedFiles) {
    if (tracked.startsWith(prefix)) return true;
  }

  let current = repoRoot;
  for (const component of relativePath.split("/").filter(Boolean)) {
    let entries;
    try {
      entries = readdirSync(current);
    } catch {
      return false;
    }
    if (!entries.includes(component)) return false;
    current = join(current, component);
  }
  return current !== repoRoot;
}

function loadJson(path, label, errors) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch (error) {
    errors.push(
      error instanceof SyntaxError
        ? `${label} is invalid JSON`
        : `${label} is unreadable`,
    );
    return undefined;
  }
}

function entriesByPath(entries) {
  const grouped = new Map();
  for (const entry of entries) {
    const existing = grouped.get(entry.relativePath) ?? [];
    existing.push(entry);
    grouped.set(entry.relativePath, existing);
  }
  return grouped;
}

function createRepositoryReader(indexEntries, errors) {
  const cache = new Map();
  const reported = new Set();

  function fail(relativePath, message) {
    if (!reported.has(relativePath)) {
      errors.push(message);
      reported.add(relativePath);
    }
    cache.set(relativePath, undefined);
    return undefined;
  }

  return function readRepositoryFile(relativePath) {
    if (cache.has(relativePath)) return cache.get(relativePath);
    const entries = indexEntries.get(relativePath) ?? [];
    if (
      entries.length !== 1 ||
      entries[0].stage !== 0
    ) {
      return fail(
        relativePath,
        `${relativePath} is not a regular tracked file: unmerged index entries`,
      );
    }
    const [{ mode }] = entries;
    if (!mode.startsWith("100")) {
      return fail(
        relativePath,
        `${relativePath} is not a regular tracked file: index mode ${mode}`,
      );
    }

    let parentPath = repoRoot;
    for (const component of relativePath.split("/").slice(0, -1)) {
      parentPath = join(parentPath, component);
      let parentMetadata;
      try {
        parentMetadata = lstatSync(parentPath);
      } catch (error) {
        return fail(
          relativePath,
          error?.code === "ENOENT"
            ? `${relativePath} is missing from the working tree`
            : `${relativePath} is unreadable in the working tree`,
        );
      }
      if (!parentMetadata.isDirectory()) {
        return fail(
          relativePath,
          `${relativePath} is not a regular working-tree file`,
        );
      }
    }

    let metadata;
    try {
      metadata = lstatSync(join(repoRoot, relativePath));
    } catch (error) {
      if (error?.code === "ENOENT") {
        return fail(
          relativePath,
          `${relativePath} is missing from the working tree`,
        );
      }
      return fail(
        relativePath,
        `${relativePath} is unreadable in the working tree`,
      );
    }
    if (!metadata.isFile()) {
      return fail(
        relativePath,
        `${relativePath} is not a regular working-tree file`,
      );
    }
    if ((metadata.mode & 0o444) === 0) {
      return fail(
        relativePath,
        `${relativePath} is unreadable in the working tree`,
      );
    }

    try {
      const text = readFileSync(join(repoRoot, relativePath), "utf8");
      cache.set(relativePath, text);
      return text;
    } catch {
      return fail(
        relativePath,
        `${relativePath} is unreadable in the working tree`,
      );
    }
  };
}

export function verifyRepositoryGovernance() {
  const errors = [];
  const manifest = loadJson(manifestPath, MANIFEST_LABEL, errors);
  if (manifest === undefined) return errors;
  errors.push(...validateGovernanceManifest(manifest));
  if (errors.length > 0) return errors;

  let allTrackedEntries;
  try {
    allTrackedEntries = trackedEntries();
  } catch {
    errors.push("git ls-files failed");
    return errors;
  }
  const indexEntries = entriesByPath(allTrackedEntries);
  const trackedFiles = new Set(indexEntries.keys());
  const markdownPaths = [...indexEntries.keys()].filter((relativePath) =>
    relativePath.endsWith(".md")
  );
  const readRepositoryFile = createRepositoryReader(indexEntries, errors);

  const policyDocuments = new Map();
  for (const relativePath of manifest.policy_documents) {
    if (!trackedFiles.has(relativePath)) {
      errors.push(`policy document is not tracked: ${relativePath}`);
      continue;
    }
    const text = readRepositoryFile(relativePath);
    if (text !== undefined) policyDocuments.set(relativePath, text);
  }

  for (const [relativePath, snippets] of Object.entries(manifest.required_text)) {
    const text = policyDocuments.get(relativePath);
    if (text === undefined) {
      if (!manifest.policy_documents.includes(relativePath)) {
        errors.push(`required_text references a non-policy document: ${relativePath}`);
      }
      continue;
    }
    for (const snippet of snippets) {
      if (!text.includes(snippet)) {
        errors.push(`${relativePath} missing required text: ${snippet}`);
      }
    }
  }

  for (const patternText of manifest.forbidden_patterns) {
    let pattern;
    try {
      pattern = new RegExp(patternText, "iu");
    } catch (error) {
      errors.push(`invalid forbidden pattern ${patternText}: ${error.message}`);
      continue;
    }
    for (const [relativePath, text] of policyDocuments) {
      if (pattern.test(text)) {
        errors.push(`${relativePath} contains forbidden pattern: ${patternText}`);
      }
    }
  }

  for (const relativePath of manifest.forbidden_paths) {
    if (pathExists(relativePath, trackedFiles)) {
      errors.push(`forbidden path exists: ${relativePath}`);
    }
  }

  const references = [];
  const headingsByFile = new Map();
  const declaredCreates = new Map();
  for (const relativePath of markdownPaths) {
    const text = readRepositoryFile(relativePath);
    if (text === undefined) continue;
    references.push(...collectMarkdownReferences(text, relativePath));
    headingsByFile.set(relativePath, collectHeadingSlugs(text));
    if (relativePath.startsWith("docs/superpowers/plans/")) {
      declaredCreates.set(
        relativePath,
        collectDeclaredCreatePaths(text, relativePath),
      );
    }
  }
  errors.push(
    ...validateReferences({
      references,
      trackedFiles,
      headingsByFile,
      declaredCreates,
    }),
  );

  const packageJson = loadJson(join(repoRoot, "package.json"), "package.json", errors);
  const gateCommand = packageJson?.scripts?.[manifest.gate.script];
  if (typeof gateCommand !== "string") {
    errors.push(`package.json missing gate script: ${manifest.gate.script}`);
  } else {
    try {
      errors.push(
        ...validateGateMembers(
          parseGateMembers(gateCommand),
          manifest.gate.members,
        ),
      );
    } catch (error) {
      errors.push(error.message);
    }
  }

  return errors;
}

function main() {
  const errors = verifyRepositoryGovernance();
  if (errors.length > 0) {
    console.error("doc governance verification failed:");
    for (const error of errors) console.error(`- ${error}`);
    process.exit(1);
  }
  console.log("doc governance verification passed");
}

if (resolve(process.argv[1] ?? "") === scriptPath) main();
