#!/usr/bin/env node

import { existsSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const repoRoot = resolve(new URL("..", import.meta.url).pathname);

const versions = {
  "v2.73": {
    title: "# V2.73 Verification Checklist",
    required: [
      "Task Preflight timeout recovery",
      "Task Cockpit cannot remain in `Preparing...`",
      "timeout/fallback/cancel/retry",
      "docs/ui-artifacts/v2.73-task-cockpit-timeout-recovery/completed.png",
      "task_cockpit",
    ],
  },
  "v2.74": {
    title: "# V2.74 Verification Checklist",
    required: [
      "Real-local launch and window targeting stability",
      "exact current-bundle launch",
      "duplicate same-bundle",
      "PID `52193`",
      "docs/ui-artifacts/v2.74-launch-window-targeting/completed.png",
      "No formal signing",
    ],
  },
  "v2.75": {
    title: "# V2.75 Verification Checklist",
    required: [
      "Task input and input-method resilience",
      "AX-settable",
      "skills-copilot.task-cockpit.input",
      "PID `43079`",
      "docs/ui-artifacts/v2.75-task-input-resilience/completed.png",
      "No raw prompt persistence",
    ],
  },
  "v2.76": {
    title: "# V2.76 Verification Checklist",
    required: [
      "Progressive Cockpit feedback",
      "skills-copilot.task-cockpit.stage-progress",
      "PID: `39728`",
      "docs/ui-artifacts/v2.76-progressive-cockpit-feedback/completed.png",
      "No new provider default calls",
      "No hidden task state",
    ],
  },
  "v2.77": {
    title: "# V2.77 Verification Checklist",
    required: [
      "real-local validation workbench",
      "skills-copilot.validation-workbench",
      "Running PID: `34909`",
      "docs/ui-artifacts/v2.77-validation-workbench/completed.png",
      "canonical blockers",
      "preserves unlocked manual visual review",
    ],
  },
  "v2.78": {
    title: "# V2.78 Verification Checklist",
    required: [
      "protocol / validation gate parity",
      "SUPPORTED_METHODS",
      "V2.46-V2.64 verification history",
      "pnpm verify:service-protocol-drift",
      "No protocol method rename",
      "No replacement of unlocked manual visual review",
    ],
  },
  "v2.79": {
    title: "# V2.79 Verification Checklist",
    required: [
      "privacy fixture and evidence-surface localization sweep",
      "literal loopback host-port",
      "path redaction/collapse/reveal",
      "PID: `68064`",
      "docs/ui-artifacts/v2.79-privacy-localization/completed.png",
      "No credential reads",
    ],
  },
  "v2.80": {
    title: "# V2.80 Verification Checklist",
    required: [
      "Detail navigation and visual density polish",
      "skills-copilot.detail.top",
      "DenseDisclosureList",
      "PID: `82571`",
      "docs/ui-artifacts/v2.80-detail-density/completed.png",
      "No provider default calls",
    ],
  },
  "v2.81": {
    title: "# V2.81 Verification Checklist",
    required: [
      "Swift stdio sidecar cancellation cleanup",
      "ServiceProcessRunner.swift",
      "SIGKILL",
      "swift test --package-path apps/macos",
      "No daemon or socket redesign",
      "Real-local validation decision",
    ],
  },
  "v2.82": {
    title: "# V2.82 Verification Checklist",
    required: [
      "test isolation and core model test floor",
      "serialized RAII guard",
      "cargo test -p skills-copilot-core",
      "locked-session",
      "No credential reads beyond existing explicitly confirmed provider tests",
      "Real-local validation decision",
    ],
  },
  "v2.83": {
    title: "# V2.83 Verification Checklist",
    required: [
      "continued module splitting",
      "crates/service/src/protocol.rs",
      "DetailOverviewSection.swift",
      "FakeServiceScript.swift",
      "No service protocol method or payload changes",
      "Real-local validation decision",
    ],
  },
  "v2.84": {
    title: "# V2.84 Verification Checklist",
    required: [
      "Swift Detail section splitting",
      "TaskCockpitPanel.swift",
      "verify:module-size",
      "DetailView.swift",
      "Final status decision: completed",
    ],
  },
  "v2.85": {
    title: "# V2.85 Verification Checklist",
    required: [
      "Rust RPC domain module splitting",
      "service_host.rs",
      "service_app_search.rs",
      "pnpm verify:service-protocol-drift",
      "slimmed method set",
      "Final status decision: completed",
    ],
  },
  "v2.86": {
    title: "# V2.86 Verification Checklist",
    required: [
      "Rust helper/test split and module-size gate",
      "service_support_helpers.rs",
      "crates/service/src/tests/",
      "verify:module-size",
      "cargo clippy --workspace",
      "Final status decision: completed",
    ],
  },
  "v2.87": {
    title: "# V2.87 Verification Checklist",
    required: [
      "Agent Copilot first pass",
      "current read-only navigation surfaces",
      "Agent-focused detail surfaces",
      "session.previewLocalSessions",
      "slimmed method set",
      "docs/ui-artifacts/native-macos-shell/completed.png",
      "Final status decision: completed",
    ],
  },
  "v2.88": {
    title: "# V2.88 Verification Checklist",
    required: [
      "handoff and current evidence closeout",
      "Navigation",
      "Agent detail",
      "Provider observability",
      "legacy per-surface screenshots were pruned",
      "docs/ui-artifacts/native-macos-shell/completed.png",
      "Final status decision: completed",
    ],
  },
  "v2.89": {
    title: "# V2.89 Verification Checklist",
    required: [
      "Brand asset refresh",
      "AppIcon.icns",
      "AppIcon.svg",
      "Agent Copilot display brand",
      "unchanged internal identifiers",
      "docs/ui-artifacts/v2.89-brand-assets",
      "pnpm generate:app-icon",
      "Final status decision: completed",
    ],
  },
  "v2.90": {
    title: "# V2.90 Verification Checklist",
    required: [
      "Internal identifier migration",
      "dist/AgentCopilot.app",
      "dev.agent-copilot.native",
      "dev.skills-copilot.native",
      "app-data compatibility migration",
      "agent-copilot-app-data-migration.json",
      "docs/ui-artifacts/v2.90-identifier-migration",
      "Final status decision: completed",
    ],
  },
  "v2.91": {
    title: "# V2.91 Verification Checklist",
    required: [
      "Model-task matching history",
      "llm.listModelTaskMatches",
      "llm.recordModelTaskMatch",
      "llm.deleteModelTaskMatch",
      "model-task-matches.json",
      "model_task_history_rows",
      "docs/ui-artifacts/v2.91-model-task-history",
      "Final status decision: completed",
    ],
  },
  "v2.92": {
    title: "# V2.92 Verification Checklist",
    required: [
      "Codex expanded roots",
      "RootSource::Compatibility",
      "RootSource::Admin",
      "RootSource::Plugin",
      ".codex/config.toml",
      "/etc/codex/skills",
      "$CODEX_HOME/skills",
      "plugin marketplace",
      "docs/ui-artifacts/v2.92-codex-expanded-roots",
      "Final status decision: completed",
    ],
  },
  "v2.93": {
    title: "# V2.93 Verification Checklist",
    required: [
      "opencode custom roots",
      "RootSource::Configured",
      "skills.paths",
      "skills.urls",
      "canonicalization/dedupe",
      "no uncontrolled network",
      "metadata-only",
      "configured local paths",
      "docs/ui-artifacts/v2.93-opencode-custom-roots",
      "Final status decision: completed",
    ],
  },
  "v2.94": {
    title: "# V2.94 Verification Checklist",
    required: [
      "Pi install and compatibility-root writes",
      "RootSource::Compatibility",
      ".agents/skills",
      "~/.pi/agent/skills",
      ".pi/skills",
      "explicit untrusted",
      "native-root install",
      "package install/remove remains blocked",
      "docs/ui-artifacts/v2.94-pi-install-compat-writes",
      "Final status decision: completed",
    ],
  },
  "v2.95": {
    title: "# V2.95 Verification Checklist",
    required: [
      "Hermes native-root install",
      "~/.hermes/skills",
      "install-only-v2.95",
      "verified-native-root-v2.95",
      "config toggles remain blocked",
      "external_dirs remain read-only",
      "hub, URL, tap, update, uninstall",
      "docs/ui-artifacts/v2.95-hermes-native-install",
      "Final status decision: completed",
    ],
  },
  "v2.96": {
    title: "# V2.96 Verification Checklist",
    required: [
      "OpenClaw native/workspace install",
      "~/.openclaw/skills",
      "<workspace>/skills",
      "install-only-v2.96",
      "verified-native-workspace-v2.96",
      ".agents roots remain scan-only",
      "ClawHub, Git, update, verify, workshop",
      "docs/ui-artifacts/v2.96-openclaw-native-workspace-install",
      "Final status decision: completed",
    ],
  },
  "v2.97": {
    title: "# V2.97 Verification Checklist",
    required: [
      "Agent Config center",
      "primary sidebar Config workflow",
      "skills.disabled",
      "skills.entries.<key>.enabled",
      "JSON5 input",
      "strict JSON write-back",
      "snapshot/read-back/rollback",
      "Safe Batch",
      "Final status decision: completed",
    ],
  },
  "v2.98": {
    title: "# V2.98 Verification Checklist",
    required: [
      "Automatic local session discovery",
      "session.previewLocalSessions",
      "Claude Code, Codex, opencode, and Pi",
      "content_items",
      "skill_usage_rows",
      "explicit local invocation markers",
      "Hermes/OpenClaw session parsing remains deferred",
      "pnpm check:macos",
      "pnpm check:privacy",
      "Final status decision: completed",
    ],
  },
};

const staleStatus = [
  "Status: planned",
  "Status: in progress",
  "Final status decision: pending",
  "remains planned",
  "not completed",
  "coordinator verification pending",
];

const commonSafety = [
  ["No provider", "provider calls", "provider default calls"],
  ["No write", "write path", "write paths", "write action", "write/apply path"],
  ["No script execution", "script execution"],
  ["No credential", "credential read", "credential handling"],
  ["No cloud sync", "cloud sync"],
  ["No telemetry", "telemetry"],
];

function fail(message) {
  console.error(`version validation docs verification failed: ${message}`);
  process.exit(1);
}

function readRequired(relativePath) {
  const path = join(repoRoot, relativePath);
  if (!existsSync(path)) {
    fail(`missing required file: ${relativePath}`);
  }
  return readFileSync(path, "utf8");
}

function requireText(text, label, snippet) {
  if (!text.includes(snippet)) {
    fail(`${label} missing required text: ${snippet}`);
  }
}

function rejectText(text, label, snippet) {
  if (text.includes(snippet)) {
    fail(`${label} contains stale text: ${snippet}`);
  }
}

function requireAnyText(text, label, snippets) {
  if (!snippets.some((snippet) => text.includes(snippet))) {
    fail(`${label} missing one of: ${snippets.join(" | ")}`);
  }
}

function verifyVersion(version) {
  const config = versions[version];
  if (!config) {
    fail(`unknown version '${version}'. Expected one of: ${Object.keys(versions).join(", ")}`);
  }

  const versionNumber = version.replace("v", "").toUpperCase();
  const checklistPath = `docs/verification/${version}-verification-checklist.md`;
  const checklist = readRequired(checklistPath);
  const packageJson = readRequired("package.json");
  requireText(checklist, checklistPath, config.title);
  requireText(checklist, checklistPath, "Status: completed");
  requireText(checklist, checklistPath, "pnpm check:privacy");
  requireText(checklist, checklistPath, `pnpm verify:${version}-docs`);

  for (const snippets of commonSafety) {
    requireAnyText(checklist, checklistPath, snippets);
  }
  for (const snippet of config.required) {
    requireText(checklist, checklistPath, snippet);
  }
  for (const snippet of staleStatus) {
    rejectText(checklist, checklistPath, snippet);
  }
  rejectText(checklist, checklistPath, "- [ ]");

  const checkedItems = checklist.match(/- \[x\]/g) ?? [];
  if (checkedItems.length < 6) {
    fail(`${checklistPath} has too few completed evidence items: ${checkedItems.length}`);
  }

  const expectedScript = `"verify:${version}-docs": "node scripts/verify-version-validation-docs.mjs ${version}"`;
  requireText(packageJson, "package.json", expectedScript);
  requireText(
    packageJson,
    "package.json",
    `pnpm verify:${version}-docs`,
  );

  console.log(`${versionNumber} validation docs verification passed`);
}

const requestedVersions = process.argv.slice(2);
const versionsToVerify =
  requestedVersions.length > 0 ? requestedVersions : Object.keys(versions);

for (const version of versionsToVerify) {
  verifyVersion(version);
}
