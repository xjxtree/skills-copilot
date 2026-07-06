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

function rejectPath(relativePath, reason) {
  if (existsSync(join(repoRoot, relativePath))) fail(`${relativePath} should not exist: ${reason}`);
}

const roadmap = read("docs/plans/roadmap.md");
const tasks = read("docs/plans/development-tasks.md");
const agents = read("AGENTS.md");
const readme = read("README.md");
const packageJson = read("package.json");
const workflow = read("docs/ai-agent-workflow.md");
const releaseChecklist = read("docs/runbooks/release-checklist.md");
const distributionRunbook = read("docs/runbooks/distribution-runbook.md");
const uiArtifactsReadme = read("docs/ui-artifacts/README.md");
const legacyReleaseHistoryPattern = new RegExp(
  ["CHANGE" + "LOG\\.md", "docs/" + "verification", "verification " + "checklists"].join("|"),
  "i"
);

function rejectPattern(text, label, pattern, reason) {
  if (pattern.test(text)) fail(`${label} contains ${reason}`);
}

for (const [text, label] of [
  [readme, "README.md"],
  [agents, "AGENTS.md"],
  [roadmap, "docs/plans/roadmap.md"],
  [tasks, "docs/plans/development-tasks.md"],
  [workflow, "docs/ai-agent-workflow.md"],
  [releaseChecklist, "docs/runbooks/release-checklist.md"],
  [distributionRunbook, "docs/runbooks/distribution-runbook.md"],
  [uiArtifactsReadme, "docs/ui-artifacts/README.md"],
]) {
  rejectPattern(text, label, /\bV\d+\.\d+\b/, "legacy internal version history; use GitHub Releases and tags for releases");
  rejectPattern(text, label, legacyReleaseHistoryPattern, "legacy docs-based release history references");
  rejectPattern(text, label, /Current (Status|State|Baseline)|Completed baseline|Current phase/i, "status snapshot wording");
}

rejectPath("CHANGE" + "LOG.md", "release history belongs in GitHub Releases and tags");
rejectPath(["docs", "verification"].join("/"), "release verification plans are retired");

requireText(readme, "README.md", "## App Features");
requireText(readme, "README.md", "GitHub Releases");
requireText(agents, "AGENTS.md", "## Safety Boundaries");
requireText(roadmap, "docs/plans/roadmap.md", "## Near-Term Work");
requireText(tasks, "docs/plans/development-tasks.md", "## Active Task Rules");
requireText(distributionRunbook, "docs/runbooks/distribution-runbook.md", "GitHub tags and GitHub Releases");
requireText(packageJson, "package.json", "\"verify:doc-governance\"");
requireText(
  workflow,
  "docs/ai-agent-workflow.md",
  "`verify:macos-ui-layout` is intentionally reached through `pnpm check:macos`"
);

console.log("doc governance verification passed");
