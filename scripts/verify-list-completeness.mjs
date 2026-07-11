#!/usr/bin/env node
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  findUndeclaredPrefixLists,
  loadListCompletenessManifest,
  verifyListSurfaceInventory,
} from "./lib/list-completeness.mjs";

const scriptPath = fileURLToPath(import.meta.url);
const repoRoot = resolve(dirname(scriptPath), "..");
const nativeSourceRoot = join(
  repoRoot,
  "apps/macos/Sources/SkillsCopilot",
);

function swiftFiles(root) {
  const paths = [];
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const path = join(root, entry.name);
    if (entry.isDirectory()) {
      paths.push(...swiftFiles(path));
    } else if (entry.isFile() && entry.name.endsWith(".swift")) {
      paths.push(path);
    }
  }
  return paths.sort();
}

export function verifyRepositoryListCompleteness() {
  const manifest = loadListCompletenessManifest({ repoRoot });
  const errors = verifyListSurfaceInventory(manifest, { repoRoot });
  for (const path of swiftFiles(nativeSourceRoot)) {
    const relativePath = relative(repoRoot, path);
    for (const finding of findUndeclaredPrefixLists(readFileSync(path, "utf8"), {
      relativePath,
    })) {
      errors.push(`${relativePath}: ${finding}`);
    }
  }
  return errors;
}

function main() {
  const errors = verifyRepositoryListCompleteness();
  if (errors.length > 0) {
    console.error("list completeness verification failed:");
    for (const error of errors) console.error(`- ${error}`);
    process.exit(1);
  }
  console.log("list completeness verification passed");
}

if (resolve(process.argv[1] ?? "") === scriptPath) main();
