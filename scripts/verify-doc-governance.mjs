#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
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

function trackedPaths(pathspec) {
  const args = ["ls-files", "-z"];
  if (pathspec) args.push("--", pathspec);
  const output = execFileSync("git", args, {
    cwd: repoRoot,
    encoding: "utf8",
    maxBuffer: 32 * 1024 * 1024,
  });
  return output.split("\0").filter(Boolean);
}

function pathExists(relativePath, trackedFiles) {
  if (existsSync(join(repoRoot, relativePath))) return true;
  if (trackedFiles.has(relativePath)) return true;
  const prefix = relativePath.endsWith("/")
    ? relativePath
    : `${relativePath}/`;
  for (const tracked of trackedFiles) {
    if (tracked.startsWith(prefix)) return true;
  }
  return false;
}

function loadJson(path, label, errors) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch (error) {
    errors.push(`${label} is unreadable or invalid JSON: ${error.message}`);
    return undefined;
  }
}

function readRepositoryFile(relativePath, errors) {
  try {
    return readFileSync(join(repoRoot, relativePath), "utf8");
  } catch (error) {
    errors.push(`${relativePath} is unreadable: ${error.message}`);
    return undefined;
  }
}

export function verifyRepositoryGovernance() {
  const errors = [];
  const manifest = loadJson(manifestPath, "scripts/repository-governance.json", errors);
  if (!manifest) return errors;
  if (manifest.schema_version !== 1) {
    errors.push(
      `scripts/repository-governance.json has unsupported schema_version ${String(manifest.schema_version)}`,
    );
    return errors;
  }

  let allTrackedPaths;
  let markdownPaths;
  try {
    allTrackedPaths = trackedPaths();
    markdownPaths = trackedPaths("*.md");
  } catch (error) {
    errors.push(`git ls-files failed: ${error.message}`);
    return errors;
  }
  const trackedFiles = new Set(allTrackedPaths);

  const policyDocuments = new Map();
  for (const relativePath of manifest.policy_documents) {
    if (!trackedFiles.has(relativePath)) {
      errors.push(`policy document is not tracked: ${relativePath}`);
      continue;
    }
    const text = readRepositoryFile(relativePath, errors);
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
      pattern = new RegExp(patternText, "u");
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
    const text = readRepositoryFile(relativePath, errors);
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
