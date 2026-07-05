#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "..");

function fail(message) {
  console.error(`doc governance verification failed: ${message}`);
  process.exit(1);
}

function read(relativePath) {
  const path = join(repoRoot, relativePath);
  if (!existsSync(path)) fail(`missing ${relativePath}`);
  return readFileSync(path, "utf8");
}

function requireText(text, label, snippet) {
  if (!text.includes(snippet)) fail(`${label} missing required text: ${snippet}`);
}

const roadmap = read("docs/plans/roadmap.md");
const tasks = read("docs/plans/development-tasks.md");
const agents = read("AGENTS.md");
const readme = read("README.md");
const changelog = read("CHANGELOG.md");
const packageJson = read("package.json");
const workflow = read("docs/ai-agent-workflow.md");

function rejectPattern(text, label, pattern, reason) {
  if (pattern.test(text)) fail(`${label} contains ${reason}`);
}

for (const [text, label] of [
  [readme, "README.md"],
  [agents, "AGENTS.md"],
]) {
  rejectPattern(text, label, /\bV\d+\.\d+\b/, "version history; use CHANGELOG.md or verification checklists");
  rejectPattern(text, label, /Current (Status|State|Baseline)|Completed baseline|Current phase/i, "status snapshot wording");
}

requireText(readme, "README.md", "## What It Does");
requireText(agents, "AGENTS.md", "## Safety Boundaries");
requireText(roadmap, "docs/plans/roadmap.md", "## Near-Term Work");
requireText(tasks, "docs/plans/development-tasks.md", "## Active Task Rules");
requireText(changelog, "CHANGELOG.md", "## V2.98");
requireText(packageJson, "package.json", "\"verify:doc-governance\"");
requireText(
  workflow,
  "docs/ai-agent-workflow.md",
  "`verify:macos-ui-layout` is intentionally reached through `pnpm check:macos`"
);

console.log("doc governance verification passed");
