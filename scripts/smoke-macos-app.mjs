#!/usr/bin/env node

import { execFileSync, spawnSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  readdirSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { platform, tmpdir } from "node:os";
import { join, resolve } from "node:path";
import {
  assertBoundedPathIdentityUnchanged,
  initializeAllocatedFixture,
  snapshotBoundedPathIdentity,
} from "./lib/smoke-fixture-safety.mjs";
import { runSmokeFlow } from "./lib/smoke-flow.mjs";
import { parseSmokeOptions } from "./lib/smoke-options.mjs";
import { sameFilesystemEntry } from "./lib/path-identity.mjs";
import { cleanupOwnedTemporaryRoot } from "./lib/smoke-lifecycle.mjs";
import { formatValidationBlocker } from "./validation-blockers.mjs";

const appName = "AgentCopilot";
const bundleId = "dev.agent-copilot.native";
const legacyAppName = "SkillsCopilot";
const legacyBundleId = "dev.skills-copilot.native";
const processName = appName;
const appPath = resolve(process.env.SKILLS_COPILOT_APP ?? "dist/AgentCopilot.app");
const appBinary = join(appPath, "Contents", "MacOS", appName);
const serviceBinary = join(appPath, "Contents", "Resources", "skills-copilot-service");
const smokeActionPreviewSecret = randomBytes(32).toString("hex");
const screenshotPath = resolve(
  process.env.SKILLS_COPILOT_SMOKE_SCREENSHOT ??
    join(tmpdir(), "agent-copilot-smoke-completed.png"),
);
const knownBenignLogPatterns = [
  /appintents/i,
  /StateRestoration.*restoreWindowWithIdentifier/i,
  /CFPasteboard/i,
  /Connection invalid/i,
  /XPC_ERROR_CONNECTION_INVALID/i,
  /TCCAccessRequest/i,
  /TCC:access/i,
  /SkyLight.*not a valid connection ID/i,
  /launchservicesd/i,
  /RunningBoard/i,
  /CoreServices\.coreservicesd/i,
  /Missing .* entitlement/i,
  /\[com\.apple\.AppKit:General\] <private>/i,
];

class SmokeFailure extends Error {
  constructor(message) {
    super(message);
    this.name = "SmokeFailure";
  }
}

function fail(message) {
  throw new SmokeFailure(message);
}

function note(message) {
  console.log(`smoke: ${message}`);
}

function expectedServiceProtocolVersion() {
  const fixturePath = resolve(
    "fixtures/service-protocol/service.status.response.json",
  );
  let fixture;
  try {
    fixture = JSON.parse(readFileSync(fixturePath, "utf8"));
  } catch {
    fail("service status protocol fixture is missing or invalid");
  }
  const version = fixture?.result?.protocol_version;
  if (!Number.isSafeInteger(version) || version < 1) {
    fail("service status protocol fixture has an invalid protocol version");
  }
  return version;
}

function run(command, args, options = {}) {
  return execFileSync(command, args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    ...options,
  }).trim();
}

function tryRun(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    stdio: [options.input === undefined ? "ignore" : "pipe", "pipe", "pipe"],
    ...options,
  });
  return {
    ok: result.status === 0,
    stdout: (result.stdout ?? "").trim(),
    stderr: (result.stderr ?? "").trim(),
    status: result.status,
    signal: result.signal,
    error: result.error?.message ?? "",
  };
}

function sleepMs(milliseconds) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, milliseconds);
}

function canonicalPath(path) {
  try {
    return realpathSync(path);
  } catch {
    return resolve(path);
  }
}

function targetBundlePath() {
  return canonicalPath(appPath);
}

function runSwift(script, args = []) {
  return tryRun("swift", ["-Xfrontend", "-disable-availability-checking", "-e", script, ...args]);
}

function queryRunningApps() {
  const swift = `
import AppKit
import Foundation

let args = Array(CommandLine.arguments.dropFirst())
let bundleId = args.indices.contains(0) ? args[0] : ""
let appName = args.indices.contains(1) ? args[1] : ""
let legacyBundleId = args.indices.contains(2) ? args[2] : ""
let legacyAppName = args.indices.contains(3) ? args[3] : ""
var rows: [[String: Any]] = []

for app in NSWorkspace.shared.runningApplications {
    let identifierMatches = app.bundleIdentifier == bundleId || app.bundleIdentifier == legacyBundleId
    let nameMatches = app.localizedName == appName || app.localizedName == legacyAppName
    guard identifierMatches || nameMatches else { continue }
    rows.append([
        "pid": Int(app.processIdentifier),
        "bundleIdentifier": app.bundleIdentifier ?? "",
        "localizedName": app.localizedName ?? "",
        "bundlePath": app.bundleURL?.resolvingSymlinksInPath().standardizedFileURL.path ?? "",
        "executablePath": app.executableURL?.resolvingSymlinksInPath().standardizedFileURL.path ?? "",
        "isActive": app.isActive,
        "isTerminated": app.isTerminated,
    ])
}

let data = try JSONSerialization.data(withJSONObject: rows, options: [])
print(String(data: data, encoding: .utf8)!)
`;
  const result = runSwift(swift, [bundleId, appName, legacyBundleId, legacyAppName]);
  if (!result.ok) {
    fail(formatValidationBlocker(
      result.stderr || result.stdout || "tool-layer-unknown: unable to query running macOS applications",
      "query-running-apps",
    ));
  }
  try {
    return JSON.parse(result.stdout || "[]");
  } catch {
    fail(`tool-layer-unknown: invalid running app query JSON: ${result.stdout}`);
  }
}

function targetRunningApps() {
  const target = targetBundlePath();
  return queryRunningApps().filter((app) => sameFilesystemEntry(app.bundlePath, target));
}

function staleSameBundleApps() {
  const target = targetBundlePath();
  return queryRunningApps().filter(
    (app) => app.bundlePath && !sameFilesystemEntry(app.bundlePath, target),
  );
}

function verifyBundle() {
  if (platform() !== "darwin") {
    fail("macOS app smoke only runs on darwin");
  }
  const infoPlist = join(appPath, "Contents", "Info.plist");
  const icon = join(appPath, "Contents", "Resources", "AppIcon.icns");
  for (const file of [appPath, infoPlist, appBinary, serviceBinary, icon]) {
    if (!existsSync(file)) {
      fail(`bundle is missing ${file}`);
    }
  }
  const identifier = run("/usr/libexec/PlistBuddy", [
    "-c",
    "Print :CFBundleIdentifier",
    infoPlist,
  ]);
  if (identifier !== bundleId) {
    fail(`unexpected CFBundleIdentifier ${identifier}`);
  }
  const iconFile = run("/usr/libexec/PlistBuddy", [
    "-c",
    "Print :CFBundleIconFile",
    infoPlist,
  ]);
  if (iconFile !== "AppIcon") {
    fail(`unexpected CFBundleIconFile ${iconFile}`);
  }
  note(`bundle ok: ${appPath}`);
}

function verifyBundleFreshness(allowStaleApp) {
  if (allowStaleApp) {
    note("bundle freshness check skipped by --allow-stale-app");
    return;
  }

  const appBinary = join(appPath, "Contents", "MacOS", appName);
  const bundledIcon = join(appPath, "Contents", "Resources", "AppIcon.icns");
  const bundledLocalizable = join(appPath, "Contents", "Resources", "en.lproj", "Localizable.strings");
  const infoPlist = join(appPath, "Contents", "Info.plist");
  assertTargetFresh(
    "Swift app binary",
    appBinary,
    [
      "apps/macos/Package.swift",
      "script/build_and_run.sh",
      ...filesUnder("apps/macos/Sources", [".swift"]),
    ],
  );
  assertTargetFresh(
    "Rust service sidecar",
    serviceBinary,
    [
      "Cargo.toml",
      "Cargo.lock",
      "script/build_and_run.sh",
      ...filesUnder("crates", [".rs", ".toml"]),
    ],
  );
  assertTargetFresh("bundled app icon", bundledIcon, [
    "apps/macos/Sources/SkillsCopilot/Resources/AppIcon.icns",
    "script/build_and_run.sh",
  ]);
  assertTargetFresh("bundled localized strings", bundledLocalizable, [
    "script/build_and_run.sh",
    ...filesUnder("apps/macos/Sources/SkillsCopilot/Resources", [".strings"]),
  ]);
  assertTargetFresh("Info.plist", infoPlist, [
    "crates/service/Cargo.toml",
    "script/build_and_run.sh",
  ]);
  note("bundle freshness ok");
}

function assertTargetFresh(label, targetPath, inputPaths) {
  if (!existsSync(targetPath)) {
    fail(`${label} is missing: ${targetPath}`);
  }
  const targetMtime = statSync(targetPath).mtimeMs;
  const staleInputs = inputPaths
    .filter((path) => existsSync(path))
    .map((path) => ({ path, mtime: statSync(path).mtimeMs }))
    .filter((input) => input.mtime > targetMtime + 1_000)
    .sort((a, b) => b.mtime - a.mtime);

  if (staleInputs.length === 0) {
    return;
  }

  const examples = staleInputs
    .slice(0, 8)
    .map((input) => `  - ${input.path}`)
    .join("\n");
  fail(
    `stale-bundle: ${label} is older than source inputs.\n` +
      `${examples}\n` +
      "Run pnpm build:macos before Smoke App Run; use pnpm verify:macos-launch only for interactive launch/window proof.",
  );
}

function filesUnder(dir, extensions) {
  if (!existsSync(dir)) {
    return [];
  }

  const files = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...filesUnder(path, extensions));
    } else if (extensions.some((extension) => path.endsWith(extension))) {
      files.push(path);
    }
  }
  return files;
}

async function createFixtureEnvironment() {
  return initializeAllocatedFixture({
    allocateRoot: () =>
      mkdtempSync(join(tmpdir(), "skills-copilot-native-smoke-")),
    initializeRoot: initializeFixtureEnvironment,
  });
}

async function initializeFixtureEnvironment(root) {
  const realOpencodeConfigSnapshot = await snapshotRealOpencodeConfig();
  const home = join(root, "home");
  const appData = join(root, "app-data");
  const claudeSkillsRoot = join(home, ".claude", "skills");
  const codexSkillsRoot = join(home, ".agents", "skills");
  const opencodeSkillsRoot = join(home, ".config", "opencode", "skills");
  const opencodeConfiguredSkillsRoot = join(root, "opencode-configured-skills");
  const opencodeUnavailableSkillsRoot = join(root, "opencode-unavailable-skills");
  const piSkillsRoot = join(home, ".pi", "agent", "skills");
  const projectRoot = join(root, "fixture-project");
  const projectCwd = join(projectRoot, "nested", "workspace");
  const projectCodexSkillsRoot = join(projectRoot, ".agents", "skills");
  const projectOpencodeSkillsRoot = join(projectRoot, ".opencode", "skills");
  const projectPiSkillsRoot = join(projectCwd, ".pi", "skills");
  const projectPiSettings = join(projectRoot, ".pi", "settings.json");
  const codexUserConfig = join(home, ".codex", "config.toml");
  const codexPluginRoot = join(
    home,
    ".codex",
    "plugins",
    "cache",
    "smoke-publisher",
    "smoke-plugin",
    "1.0.0",
  );
  const sessionRoot = join(root, "authorized-sessions");
  const sessionPath = join(sessionRoot, "claude-session.jsonl");
  const projectCodexConfig = join(projectRoot, ".codex", "config.toml");
  const projectOpencodeConfig = join(projectRoot, "opencode.json");
  mkdirSync(claudeSkillsRoot, { recursive: true });
  mkdirSync(codexSkillsRoot, { recursive: true });
  mkdirSync(opencodeSkillsRoot, { recursive: true });
  mkdirSync(opencodeConfiguredSkillsRoot, { recursive: true });
  mkdirSync(piSkillsRoot, { recursive: true });
  mkdirSync(projectCodexSkillsRoot, { recursive: true });
  mkdirSync(projectOpencodeSkillsRoot, { recursive: true });
  mkdirSync(projectPiSkillsRoot, { recursive: true });
  mkdirSync(join(projectRoot, ".pi"), { recursive: true });
  mkdirSync(join(projectRoot, ".git"), { recursive: true });
  mkdirSync(projectCwd, { recursive: true });
  mkdirSync(join(codexPluginRoot, ".codex-plugin"), { recursive: true });
  mkdirSync(sessionRoot, { recursive: true });
  mkdirSync(appData, { recursive: true });
  mkdirSync(join(home, ".claude"), { recursive: true });
  mkdirSync(join(home, ".codex"), { recursive: true });
  writeFileSync(join(home, ".claude", "settings.json"), "{}\n");

  writeSkill(
    claudeSkillsRoot,
    "alpha-review",
    "---\nname: alpha-review\ndescription: Review fixture for native smoke.\n---\nAlpha body.\n",
  );
  writeSkill(
    claudeSkillsRoot,
    "content-drift-a",
    "---\nname: shared-name\ndescription: First conflicting fixture.\n---\nUse version A.\n",
  );
  writeSkill(
    claudeSkillsRoot,
    "content-drift-b",
    "---\nname: shared-name\ndescription: Second conflicting fixture.\n---\nUse version B.\n",
  );
  writeSkill(
    codexSkillsRoot,
    "codex-user-smoke",
    "---\nname: codex-user-smoke\ndescription: User Codex fixture for native smoke.\n---\nUser Codex body.\n",
  );
  writeFileSync(
    join(codexPluginRoot, ".codex-plugin", "plugin.json"),
    JSON.stringify(
      {
        name: "smoke-plugin",
        version: "1.0.0",
        skills: "./skills/",
      },
      null,
      2,
    ) + "\n",
  );
  writeSkill(
    join(codexPluginRoot, "skills"),
    "codex-plugin-smoke",
    "---\nname: codex-plugin-smoke\ndescription: Enabled Codex plugin fixture for native smoke.\n---\nPlugin body.\n",
  );
  writeSkill(
    piSkillsRoot,
    "pi-global-smoke",
    "---\nname: pi-global-smoke\ndescription: Global Pi native fixture for native smoke.\n---\nGlobal Pi body.\n",
  );
  writeSkill(
    codexSkillsRoot,
    "pi-agent-global-smoke",
    "---\nname: pi-agent-global-smoke\ndescription: Global Pi compatibility fixture for native smoke.\n---\nGlobal Pi compatibility body.\n",
  );
  writeSkill(
    projectCodexSkillsRoot,
    "codex-project-smoke",
    "---\nname: codex-project-smoke\ndescription: Project Codex fixture for native smoke.\n---\nProject Codex body.\n",
  );
  writeSkill(
    projectPiSkillsRoot,
    "pi-project-smoke",
    "---\nname: pi-project-smoke\ndescription: Project Pi native fixture for native smoke.\n---\nProject Pi body.\n",
  );
  writeSkill(
    projectCodexSkillsRoot,
    "pi-agent-project-smoke",
    "---\nname: pi-agent-project-smoke\ndescription: Project Pi compatibility fixture for native smoke.\n---\nProject Pi compatibility body.\n",
  );
  writeFileSync(
    projectPiSettings,
    JSON.stringify({ skills: [] }, null, 2) + "\n",
  );
  writeSkill(
    opencodeSkillsRoot,
    "opencode-global-smoke",
    "---\nname: opencode-global-smoke\ndescription: Global opencode fixture for native smoke.\n---\nGlobal opencode body.\n",
  );
  writeSkill(
    opencodeConfiguredSkillsRoot,
    "opencode-configured-smoke",
    "---\nname: opencode-configured-smoke\ndescription: Configured opencode fixture for native smoke.\n---\nConfigured opencode body.\n",
  );
  writeFileSync(
    join(home, ".config", "opencode", "opencode.json"),
    JSON.stringify(
      {
        skills: {
          paths: [
            opencodeConfiguredSkillsRoot,
            opencodeUnavailableSkillsRoot,
          ],
          urls: ["https://example.invalid/skills/index.json"],
        },
      },
      null,
      2,
    ) + "\n",
  );
  writeSkill(
    projectOpencodeSkillsRoot,
    "opencode-project-smoke",
    "---\nname: opencode-project-smoke\ndescription: Project opencode fixture for native smoke.\n---\nProject opencode body.\n",
  );
  writeFileSync(
    sessionPath,
    [
      JSON.stringify({
        type: "user",
        isSidechain: false,
        sessionId: "claude-native-smoke",
        cwd: projectRoot,
        timestamp: "2026-07-24T08:00:00.000Z",
        message: {
          role: "user",
          content: "Continue the isolated product projection smoke.",
        },
      }),
      JSON.stringify({
        type: "assistant",
        sessionId: "claude-native-smoke",
        timestamp: "2026-07-24T08:00:01.000Z",
        message: {
          role: "assistant",
          content: "The isolated session is ready for a read-only continuation preview.",
        },
      }),
    ].join("\n") + "\n",
  );
  const codexTargetSkillPath = realpathSync(
    join(projectCodexSkillsRoot, "codex-project-smoke", "SKILL.md"),
  );
  const codexNonTargetSkillPath = join(root, "unrelated-codex-skill", "SKILL.md");
  writeFileSync(
    codexUserConfig,
    [
      "# fixture comment preserved by Codex config patch",
      'model = "fixture-model"',
      "",
      '[plugins."smoke-plugin@smoke-publisher"]',
      "enabled = true",
      "",
      "[sandbox]",
      'mode = "read-only"',
      "",
      "[[skills.config]]",
      `path = "${escapeTomlBasicString(codexTargetSkillPath)}"`,
      "enabled = true",
      "",
      "[[skills.config]]",
      `path = "${escapeTomlBasicString(codexNonTargetSkillPath)}"`,
      "enabled = false",
      "",
      "[[skills.config]]",
      `path = "${escapeTomlBasicString(codexTargetSkillPath)}"`,
      "enabled = true",
      "",
      "[[skills.config]]",
      `path = "${escapeTomlBasicString(codexNonTargetSkillPath)}"`,
      "enabled = false",
      "",
    ].join("\n"),
  );
  return {
    appData,
    codexNonTargetSkillPath,
    codexTargetSkillPath,
    codexUserConfig,
    home,
    projectCodexConfig,
    projectCwd,
    projectOpencodeConfig,
    projectPiSettings,
    projectRoot,
    realOpencodeConfigSnapshot,
    root,
    sessionPath,
    sessionRoot,
  };
}

async function snapshotRealOpencodeConfig() {
  const realHome = process.env.HOME;
  if (!realHome) {
    fail("HOME is not set; cannot verify real opencode config isolation");
  }
  return await snapshotBoundedPathIdentity(
    join(realHome, ".config", "opencode"),
  );
}

async function assertRealOpencodeConfigUntouched(snapshot) {
  try {
    await assertBoundedPathIdentityUnchanged(snapshot);
  } catch {
    fail("fixture run modified real opencode config");
  }
  note("fixture opencode isolation passed: real HOME config paths unchanged");
}

function writeSkill(skillsRoot, name, content) {
  const dir = join(skillsRoot, name);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "SKILL.md"), content);
}

function escapeTomlBasicString(value) {
  return value
    .replaceAll("\\", "\\\\")
    .replaceAll("\b", "\\b")
    .replaceAll("\t", "\\t")
    .replaceAll("\n", "\\n")
    .replaceAll("\f", "\\f")
    .replaceAll("\r", "\\r")
    .replaceAll('"', '\\"');
}

function setLaunchEnv(env) {
  for (const [key, value] of Object.entries(env)) {
    const result = tryRun("launchctl", ["setenv", key, value]);
    if (!result.ok) {
      fail(result.stderr || `failed to set launch env ${key}`);
    }
  }
}

function unsetLaunchEnv(keys) {
  for (const key of keys) {
    tryRun("launchctl", ["unsetenv", key]);
  }
}

function terminateExistingApp() {
  const apps = queryRunningApps();
  if (apps.length === 0) {
    return;
  }

  const target = targetBundlePath();
  for (const app of apps) {
    if (app.bundlePath && !sameFilesystemEntry(app.bundlePath, target)) {
      note(
        `terminating stale same-bundle ${processName} pid ${app.pid} from ${app.bundlePath}`,
      );
    }
    try {
      process.kill(app.pid, "SIGTERM");
    } catch {
      // The app may have exited between NSWorkspace query and termination.
    }
  }

  const startedAt = Date.now();
  while (Date.now() - startedAt < 5_000) {
    if (queryRunningApps().length === 0) {
      return;
    }
    sleepMs(250);
  }

  for (const app of queryRunningApps()) {
    try {
      process.kill(app.pid, "SIGKILL");
    } catch {
      // Best effort cleanup before reporting any remaining ambiguity.
    }
  }

  const remaining = queryRunningApps();
  if (remaining.length > 0) {
    const examples = remaining
      .map((app) => `pid=${app.pid} bundle=${app.bundlePath || "<unknown>"}`)
      .join("; ");
    fail(`stale-bundle: unable to terminate existing ${processName} instances: ${examples}`);
  }
}

function launchApp(env) {
  setLaunchEnv(env);
  const result = tryRun("open", ["-n", appPath]);
  unsetLaunchEnv(Object.keys(env));
  if (!result.ok) {
    fail(formatValidationBlocker(result.stderr || "activation-failed: failed to launch app with open"));
  }
  const pid = waitForProcess();
  activateApp(pid);
  const windowId = waitForWindow(pid);
  note(`launched ${processName} pid ${pid} window ${windowId}`);
  return { pid, windowId };
}

function waitForProcess(timeoutMs = 10_000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const apps = targetRunningApps();
    if (apps.length === 1) {
      return apps[0].pid;
    }
    if (apps.length > 1) {
      const examples = apps.map((app) => `pid=${app.pid}`).join(", ");
      fail(`activation-failed: duplicate current bundle processes for ${targetBundlePath()}: ${examples}`);
    }
    sleepMs(250);
  }
  const staleApps = staleSameBundleApps();
  if (staleApps.length > 0) {
    const examples = staleApps
      .map((app) => `pid=${app.pid} bundle=${app.bundlePath || "<unknown>"}`)
      .join("; ");
    fail(`stale-bundle: running ${processName} instances are from different bundle path than target ${targetBundlePath()}: ${examples}`);
  }
  fail(`activation-failed: timed out waiting for ${processName} to start from ${targetBundlePath()}`);
}

function activateApp(pid) {
  const swift = `
import AppKit
import Foundation

let rawPid = CommandLine.arguments.dropFirst().first ?? ""
guard let pid = Int32(rawPid),
      let app = NSRunningApplication(processIdentifier: pid_t(pid)) else {
    fputs("activation-failed: unable to resolve running app pid \\(rawPid).\\n", stderr)
    exit(2)
}

let deadline = Date().addingTimeInterval(5)
while Date() < deadline {
    if app.isActive || app.activate(options: [.activateAllWindows, .activateIgnoringOtherApps]) {
        exit(0)
    }
    Thread.sleep(forTimeInterval: 0.25)
}
fputs("activation-failed: failed to activate \\(app.localizedName ?? "target app") pid \\(pid).\\n", stderr)
exit(3)
`;
  const result = runSwift(swift, [String(pid)]);
  if (!result.ok) {
    fail(formatValidationBlocker(result.stderr || result.stdout || "activation-failed: failed to activate app"));
  }
}

function waitForWindow(pid, timeoutMs = 10_000) {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const windows = visibleWindowsForPid(pid);
    if (windows.length === 1) {
      return windows[0].id;
    }
    if (windows.length > 1) {
      const examples = windows.map((window) => `window=${window.id}`).join(", ");
      fail(`window-not-found: multiple visible ${appName} windows create window ambiguity for pid ${pid}: ${examples}`);
    }
    sleepMs(250);
  }
  const sessionBlocker = currentSessionBlocker();
  fail(sessionBlocker ?? `window-not-found: timed out waiting for visible ${appName} window for pid ${pid}`);
}

function currentSessionBlocker() {
  const swift = `
import CoreGraphics
import Foundation

if let session = CGSessionCopyCurrentDictionary() as? [String: Any],
   let locked = session["CGSSessionScreenIsLocked"] as? Bool,
   locked {
    print("locked-session: macOS session is locked; refusing UI evidence.")
    exit(6)
}
exit(0)
`;
  const result = runSwift(swift);
  if (!result.ok) {
    return result.stderr || result.stdout || "tool-layer-unknown: unable to read macOS session state";
  }
  if (result.stdout) {
    return result.stdout;
  }
  return null;
}

function visibleWindowsForPid(pid) {
  const swift = `
import AppKit
import CoreGraphics
import Foundation

let rawPid = CommandLine.arguments.dropFirst().first ?? ""
guard let expectedPID = Int32(rawPid) else {
    fputs("window-not-found: invalid app pid \\(rawPid).\\n", stderr)
    exit(2)
}
guard let windows = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as? [[String: Any]] else {
    exit(2)
}

var rows: [[String: Any]] = []
for window in windows {
    guard let layer = window[kCGWindowLayer as String] as? Int, layer == 0 else { continue }
    guard let ownerPID = window[kCGWindowOwnerPID as String] as? Int32, ownerPID == expectedPID else { continue }
    guard let id = window[kCGWindowNumber as String] as? UInt32 else { continue }
    guard let bounds = window[kCGWindowBounds as String] as? [String: Any],
          let width = bounds["Width"] as? Double,
          let height = bounds["Height"] as? Double,
          width > 0,
          height > 0 else {
        continue
    }
    rows.append(["id": Int(id), "pid": Int(ownerPID), "width": width, "height": height])
}
let data = try JSONSerialization.data(withJSONObject: rows, options: [])
print(String(data: data, encoding: .utf8)!)
`;
  const result = runSwift(swift, [String(pid)]);
  if (!result.ok) {
    fail(formatValidationBlocker(result.stderr || result.stdout || "window-not-found: failed to query app windows"));
  }
  try {
    return JSON.parse(result.stdout || "[]");
  } catch {
    fail(`tool-layer-unknown: invalid window query JSON: ${result.stdout}`);
  }
}

function captureAppWindow(pid, windowId) {
  const sessionBlocker = currentSessionBlocker();
  if (sessionBlocker) {
    fail(formatValidationBlocker(sessionBlocker, "capture-window"));
  }
  tryRun("caffeinate", ["-u", "-t", "3"]);
  sleepMs(1_000);
  const result = tryRun("./script/capture_app_window.sh", [
    appPath,
    screenshotPath,
    String(pid),
    String(windowId),
  ]);
  if (!result.ok) {
    fail(formatValidationBlocker(
      result.error ||
        result.stderr ||
        result.stdout ||
        `capture-window failed with status ${result.status ?? "unknown"} signal ${result.signal ?? "none"}`,
      "capture-window",
    ));
  }
  note(result.stdout);
}

function callService(method, params, env) {
  const envelope = callServiceEnvelope(method, params, env);
  if (!envelope.ok) {
    fail(`${method} returned ${envelope.error?.code}: ${envelope.error?.message}`);
  }
  return envelope.result;
}

function expectServiceError(method, params, env, matcher, label) {
  const envelope = callServiceEnvelope(method, params, env);
  if (envelope.ok) {
    fail(`${label} unexpectedly succeeded`);
  }
  const message = `${envelope.error?.code ?? ""}: ${envelope.error?.message ?? ""}`;
  if (!matcher.test(message)) {
    fail(`${label} returned unexpected error: ${message}`);
  }
  return envelope.error;
}

function callServiceEnvelope(method, params, env) {
  const request = JSON.stringify({
    id: `smoke-${method}`,
    method,
    params,
  });
  const result = tryRun(serviceBinary, [], {
    input: request,
    env: {
      ...process.env,
      ...env,
      SKILLS_COPILOT_ACTION_PREVIEW_SECRET: smokeActionPreviewSecret,
    },
  });
  if (!result.ok) {
    fail(result.stderr || `service failed for ${method}`);
  }
  let envelope;
  try {
    envelope = JSON.parse(result.stdout);
  } catch {
    fail(`invalid service JSON for ${method}: ${result.stdout}`);
  }
  return envelope;
}

function actionConfirmationFromPreview(preview, label) {
  const action = preview?.action;
  if (
    !action ||
    typeof action.id !== "string" ||
    action.id.length === 0 ||
    typeof action.source_revision !== "string" ||
    action.source_revision.length === 0 ||
    !action.target ||
    typeof preview.preview_token !== "string" ||
    preview.preview_token.length === 0
  ) {
    fail(`${label} did not return a complete typed action confirmation`);
  }
  const reference = {
    action_id: action.id,
    source_revision: action.source_revision,
    target: action.target,
  };
  if (typeof action.project_id === "string" && action.project_id.length > 0) {
    reference.project_id = action.project_id;
  }
  return {
    reference,
    preview_token: preview.preview_token,
    confirmed: true,
  };
}

function assertVerifiedActionReadback(result, preview, label) {
  if (result?.action?.id !== preview?.action?.id) {
    fail(`${label} returned an action different from the reviewed preview`);
  }
  if (
    result.action.source_revision !== preview.action.source_revision ||
    JSON.stringify(result.action.target) !== JSON.stringify(preview.action.target)
  ) {
    fail(`${label} returned a mismatched action binding`);
  }
  if (
    !result.readback ||
    result.readback.verified !== true ||
    result.readback.action_id !== preview.action.id ||
    !Array.isArray(result.readback.observations) ||
    result.readback.observations.length === 0
  ) {
    fail(`${label} did not return verified semantic read-back`);
  }
}

function refreshCatalog(method, env, expectedContextRevision = undefined) {
  const contextRevision =
    expectedContextRevision === undefined
      ? callService("project.getContext", {}, env).revision
      : expectedContextRevision;
  const params = { explicit_refresh: true };
  if (typeof contextRevision === "string" && contextRevision.length > 0) {
    params.expected_context_revision = contextRevision;
  }
  const scan = callService(method, params, env);
  if (
    typeof scan.accepted_context_revision !== "string" ||
    scan.accepted_context_revision.length === 0 ||
    typeof scan.catalog_scan_revision !== "string" ||
    scan.catalog_scan_revision.length === 0 ||
    scan.readback?.verified !== true ||
    scan.readback.accepted_context_revision !== scan.accepted_context_revision ||
    scan.readback.catalog_scan_revision !== scan.catalog_scan_revision
  ) {
    fail(`${method} did not return a verified revision-bound catalog read-back`);
  }
  if (
    typeof contextRevision === "string" &&
    scan.accepted_context_revision !== contextRevision
  ) {
    fail(`${method} accepted a project context revision other than the requested revision`);
  }
  return scan;
}

function applySkillToggle(instanceID, targetEnabled, env, label) {
  const preview = callService(
    "batch.previewSkillToggles",
    {
      instance_ids: [instanceID],
      target_enabled: targetEnabled,
    },
    env,
  );
  if (
    preview.writes_allowed !== true ||
    preview.writable_count !== 1 ||
    preview.requested_count !== 1
  ) {
    fail(`${label} did not return one writable toggle preview`);
  }
  const result = callService(
    "batch.applySkillToggles",
    {
      instance_ids: [instanceID],
      target_enabled: targetEnabled,
      confirmation: actionConfirmationFromPreview(preview, `${label} preview`),
    },
    env,
  );
  assertVerifiedActionReadback(result, preview, `${label} apply`);
  const updated = result.updated_records?.find((record) => record.id === instanceID);
  if (!updated || updated.enabled !== targetEnabled) {
    fail(`${label} did not return the requested skill state`);
  }
  return updated;
}

function applyProjectSetContext(env, state, context) {
  const preview = callService(
    "project.previewSetContext",
    {
      ...context,
      expected_revision: state.revision,
    },
    env,
  );
  const candidateLastUsedAt = preview.candidate?.active?.last_used_at;
  if (!Number.isSafeInteger(candidateLastUsedAt)) {
    fail("set project context preview did not return a candidate timestamp");
  }
  const result = callService(
    "project.setContext",
    {
      ...context,
      candidate_last_used_at: candidateLastUsedAt,
      action_confirmation: actionConfirmationFromPreview(
        preview,
        "set project context preview",
      ),
    },
    env,
  );
  assertVerifiedActionReadback(result, preview, "set project context apply");
  if (result.state?.revision !== preview.candidate?.revision) {
    fail("set project context read-back did not match the reviewed candidate");
  }
  return result.state;
}

function applyProjectRemoveRecentContext(env, state, id) {
  const preview = callService(
    "project.previewRemoveRecentContext",
    {
      id,
      expected_revision: state.revision,
    },
    env,
  );
  const result = callService(
    "project.removeRecentContext",
    {
      id,
      action_confirmation: actionConfirmationFromPreview(
        preview,
        "remove recent project preview",
      ),
    },
    env,
  );
  assertVerifiedActionReadback(result, preview, "remove recent project apply");
  if (result.state?.revision !== preview.candidate?.revision) {
    fail("remove recent project read-back did not match the reviewed candidate");
  }
  return result.state;
}

function applyProjectClearRecentContexts(env, state) {
  const preview = callService(
    "project.previewClearRecentContexts",
    { expected_revision: state.revision },
    env,
  );
  const result = callService(
    "project.clearRecentContexts",
    {
      action_confirmation: actionConfirmationFromPreview(
        preview,
        "clear recent projects preview",
      ),
    },
    env,
  );
  assertVerifiedActionReadback(result, preview, "clear recent projects apply");
  if (result.state?.revision !== preview.candidate?.revision) {
    fail("clear recent projects read-back did not match the reviewed candidate");
  }
  return result.state;
}

function applyProjectClearContext(env, state) {
  const preview = callService(
    "project.previewClearContext",
    { expected_revision: state.revision },
    env,
  );
  const result = callService(
    "project.clearContext",
    {
      action_confirmation: actionConfirmationFromPreview(
        preview,
        "clear project context preview",
      ),
    },
    env,
  );
  assertVerifiedActionReadback(result, preview, "clear project context apply");
  if (result.state?.revision !== preview.candidate?.revision) {
    fail("clear project context read-back did not match the reviewed candidate");
  }
  return result.state;
}

function runFixtureServiceSmoke(env) {
  const status = callService("service.status", {}, env);
  const expectedProtocolVersion = expectedServiceProtocolVersion();
  if (status.protocol_version !== expectedProtocolVersion) {
    fail(
      `unexpected protocol version ${status.protocol_version}; expected ${expectedProtocolVersion}`,
    );
  }
  const scan = refreshCatalog("catalog.scanClaude", env);
  if (scan.scanned_count !== 3) {
    fail(`expected 3 scanned skills, got ${scan.scanned_count}`);
  }
  const skills = callService("catalog.listSkills", {}, env);
  const alpha = skills.find((skill) => skill.name === "alpha-review");
  if (!alpha) {
    fail("alpha-review fixture missing after scan");
  }
  const disabled = applySkillToggle(alpha.id, false, env, "alpha-review disable");
  if (disabled.enabled !== false) {
    fail("toggle off did not disable alpha-review");
  }
  const enabled = applySkillToggle(alpha.id, true, env, "alpha-review enable");
  if (enabled.enabled !== true) {
    fail("toggle on did not re-enable alpha-review");
  }
  const staleTogglePreview = callService(
    "batch.previewSkillToggles",
    {
      instance_ids: [alpha.id],
      target_enabled: false,
    },
    env,
  );
  const settings = callService("config.readClaudeSettings", {}, env);
  if (typeof settings.revision !== "string" || settings.revision.length === 0) {
    fail("settings read did not return a config revision");
  }
  let parsedSettings;
  try {
    parsedSettings = JSON.parse(settings.content);
  } catch {
    fail("settings read returned invalid JSON");
  }
  const candidateContent =
    `${JSON.stringify(
      {
        ...parsedSettings,
        smokeLifecycleFixture: { verified: true },
      },
      null,
      2,
    )}\n`;
  const savePreview = callService(
    "config.previewSaveClaudeSettings",
    {
      content: candidateContent,
      expected_revision: settings.revision,
    },
    env,
  );
  if (savePreview.changed !== true) {
    fail("settings save preview did not describe an actual fixture write");
  }
  const saved = callService(
    "config.saveClaudeSettings",
    {
      content: candidateContent,
      confirmation: actionConfirmationFromPreview(
        savePreview,
        "settings save preview",
      ),
    },
    env,
  );
  assertVerifiedActionReadback(saved, savePreview, "settings save apply");
  if (
    saved.document?.exists !== true ||
    saved.document.content !== candidateContent ||
    readFileSync(settings.target, "utf8") !== candidateContent
  ) {
    fail("settings save read-back did not match the reviewed candidate");
  }
  const contentBeforeStaleApply = readFileSync(settings.target, "utf8");
  expectServiceError(
    "batch.applySkillToggles",
    {
      instance_ids: [alpha.id],
      target_enabled: false,
      confirmation: actionConfirmationFromPreview(
        staleTogglePreview,
        "stale alpha-review preview",
      ),
    },
    env,
    /stale_action_reference|action reference is stale/i,
    "stale skill action",
  );
  if (readFileSync(settings.target, "utf8") !== contentBeforeStaleApply) {
    fail("rejected stale skill action mutated Claude settings");
  }
  const alphaAfterStaleApply = callService("catalog.listSkills", {}, env).find(
    (skill) => skill.id === alpha.id,
  );
  if (!alphaAfterStaleApply || alphaAfterStaleApply.enabled !== true) {
    fail("rejected stale skill action changed alpha-review state");
  }
  const snapshots = callService("snapshot.list", {}, env);
  if (!Array.isArray(snapshots) || snapshots.length === 0) {
    fail("expected snapshots after toggle/settings write flow");
  }
  const snapshot = snapshots.find((record) => record.id === saved.snapshot_id);
  if (!snapshot) {
    fail("settings save snapshot was not present in snapshot.list");
  }
  const preview = callService(
    "snapshot.previewRollback",
    { snapshot_id: snapshot.id },
    env,
  );
  if (preview.snapshot?.id !== snapshot.id) {
    fail("snapshot preview did not return the requested snapshot payload");
  }
  if (
    typeof preview.current_revision !== "string" ||
    preview.current_revision.length === 0 ||
    typeof preview.preview_token !== "string" ||
    preview.preview_token.length === 0 ||
    preview.changed !== true
  ) {
    fail("snapshot preview did not return a complete protocol-v2 rollback binding");
  }
  const rolledBack = callService(
    "snapshot.rollback",
    {
      snapshot_id: preview.snapshot.id,
      confirmation: actionConfirmationFromPreview(preview, "snapshot rollback preview"),
    },
    env,
  );
  assertVerifiedActionReadback(rolledBack, preview, "snapshot rollback apply");
  if (
    rolledBack.document?.content !== settings.content ||
    readFileSync(settings.target, "utf8") !== settings.content
  ) {
    fail("snapshot rollback did not restore the exact pre-save settings bytes");
  }
  note(
    "fixture service smoke passed: explicit scan, confirmed toggles, " +
      "stale-action rejection, confirmed settings save, snapshot preview, confirmed rollback",
  );
  return status;
}

function runFixtureProjectContextSmoke(env, fixture, status) {
  const baseScan = refreshCatalog("catalog.scanAll", env);
  assertSkillPresent(
    baseScan.skills,
    "codex",
    "codex-user-smoke",
    "user Codex fixture missing from scanAll",
  );
  assertSkillNotCurrentVisible(
    baseScan.skills,
    "codex-project-smoke",
    "project Codex fixture should not be visible before project context is active",
  );
  assertFixtureOpencodeGlobalSmoke(baseScan.skills);
  assertFixturePiGlobalSmoke(baseScan.skills);

  const methods = new Set(status.supported_methods ?? []);
  const hasProjectContextApi =
    methods.has("project.getContext") &&
    methods.has("project.previewSetContext") &&
    methods.has("project.setContext") &&
    methods.has("project.previewClearContext") &&
    methods.has("project.clearContext") &&
    methods.has("project.previewRemoveRecentContext") &&
    methods.has("project.removeRecentContext") &&
    methods.has("project.previewClearRecentContexts") &&
    methods.has("project.clearRecentContexts");

  if (!hasProjectContextApi) {
    const projectEnv = {
      ...env,
      SKILLS_COPILOT_PROJECT_CWD: fixture.projectCwd,
      SKILLS_COPILOT_PROJECT_ROOT: fixture.projectRoot,
    };
    const projectScan = refreshCatalog("catalog.scanAll", projectEnv, null);
    assertSkillPresent(
      projectScan.skills,
      "codex",
      "codex-project-smoke",
      "project Codex fixture missing from env project scanAll fallback",
    );
    runFixtureOpencodeReadOnlySmoke(projectScan.skills, projectEnv);
    runFixturePiCompatibilitySmoke(projectScan.skills, projectEnv, fixture);
    runFixtureCodexConfigHardeningSmoke(projectEnv, fixture, projectScan.skills);
    note(
      "project context API unavailable; verified env project scanAll fallback only " +
        "(waiting for the complete project context and recent-project API)",
    );
    return;
  }

  const initialContext = callService("project.getContext", {}, env);
  assertProjectContextState(initialContext, false, "initial project context");

  const fixtureContext = {
    current_cwd: fixture.projectCwd,
    name: "Smoke Fixture Project",
    root_path: fixture.projectRoot,
  };
  const setContext = applyProjectSetContext(env, initialContext, fixtureContext);
  assertProjectContextState(setContext, true, "set project context");

  const activeContext = callService("project.getContext", {}, env);
  assertProjectContextState(activeContext, true, "active project context");

  const removeRecentContext = applyProjectRemoveRecentContext(
    env,
    activeContext,
    activeContext.active.id,
  );
  assertProjectContextState(removeRecentContext, true, "remove recent project context");
  if (removeRecentContext.recent.length !== 0) {
    fail("removing the active project from recents should preserve active and empty recents");
  }

  const restoredRecentContext = applyProjectSetContext(
    env,
    removeRecentContext,
    fixtureContext,
  );
  const clearRecentContexts = applyProjectClearRecentContexts(env, restoredRecentContext);
  assertProjectContextState(clearRecentContexts, true, "clear recent project contexts");
  if (clearRecentContexts.recent.length !== 0) {
    fail("clearing recent projects should preserve active and empty recents");
  }

  const projectScan = refreshCatalog(
    "catalog.scanAll",
    env,
    clearRecentContexts.revision,
  );
  assertSkillPresent(
    projectScan.skills,
    "codex",
    "codex-project-smoke",
    "project Codex fixture missing after project.setContext -> scanAll",
  );
  runFixtureProductSurfaceSmoke(
    env,
    fixture,
    clearRecentContexts,
  );
  runFixtureOpencodeWritableSmoke(projectScan.skills, env, fixture);
  runFixturePiCompatibilitySmoke(projectScan.skills, env, fixture);
  runFixtureCodexConfigHardeningSmoke(env, fixture, projectScan.skills);

  const clearContext = applyProjectClearContext(env, clearRecentContexts);
  assertProjectContextState(clearContext, false, "clear project context");

  const clearedContext = callService("project.getContext", {}, env);
  assertProjectContextState(clearedContext, false, "cleared project context");

  const clearedScan = refreshCatalog("catalog.scanAll", env, clearedContext.revision);
  assertSkillPresent(
    clearedScan.skills,
    "codex",
    "codex-user-smoke",
    "user Codex fixture missing after project.clearContext -> scanAll",
  );
  assertSkillNotCurrentVisible(
    clearedScan.skills,
    "codex-project-smoke",
    "project Codex fixture remained current/visible after project.clearContext -> scanAll",
  );
  assertSkillNotCurrentVisible(
    clearedScan.skills,
    "pi-project-smoke",
    "project Pi fixture remained current/visible after project.clearContext -> scanAll",
    "pi",
  );
  note(
    "fixture project context smoke passed: setContext, recent remove/clear, " +
      "scanAll project visibility, clearContext",
  );
}

function runFixtureProductSurfaceSmoke(env, fixture, contextState) {
  const projectID = contextState.active?.id;
  if (typeof projectID !== "string" || projectID.length === 0) {
    fail("product surface smoke requires an accepted active project id");
  }
  const productParams = {
    project_id: projectID,
    expected_project_context_revision: contextState.revision,
  };
  const readiness = callService("project.getReadiness", productParams, env);
  if (
    readiness.project_id !== projectID ||
    typeof readiness.source_revision !== "string" ||
    readiness.source_revision.length === 0
  ) {
    fail("project readiness did not bind the active project and product revision");
  }
  const expectedAgents = [
    "claude-code",
    "codex",
    "opencode",
    "pi",
    "hermes",
    "openclaw",
  ];
  const readinessAgents = new Map(
    (readiness.agents ?? []).map((record) => [record.agent, record]),
  );
  for (const agent of expectedAgents) {
    if (!readinessAgents.has(agent)) {
      fail(`project readiness omitted ${agent}`);
    }
  }
  const opencodeReadiness = readinessAgents.get("opencode");
  if (
    opencodeReadiness?.coverage?.completeness === "enumerable" ||
    opencodeReadiness?.health === "healthy" ||
    readiness.health === "healthy"
  ) {
    fail("deliberately unavailable opencode root was allowed to assert healthy coverage");
  }
  if (typeof opencodeReadiness?.coverage?.incomplete_reason !== "string") {
    fail("incomplete opencode readiness omitted its typed reason");
  }

  const codexAggregates = callService(
    "catalog.listSkillAggregates",
    {
      ...productParams,
      agent: "codex",
      limit: 100,
    },
    env,
  );
  assertTerminalProductAggregatePage(codexAggregates, "Codex");
  const pluginAggregate = codexAggregates.aggregates.find((aggregate) =>
    aggregate.canonical_name?.endsWith(":codex-plugin-smoke"),
  );
  if (!pluginAggregate) {
    fail("enabled Codex plugin skill missing from product aggregates");
  }
  if (
    typeof pluginAggregate.source_identity !== "string" ||
    pluginAggregate.source_identity.includes("/") ||
    pluginAggregate.source_identity.toLowerCase().includes("cache") ||
    pluginAggregate.primary_effectiveness !== "effective"
  ) {
    fail("Codex plugin aggregate leaked cache infrastructure or lost effective state");
  }

  const opencodeAggregates = callService(
    "catalog.listSkillAggregates",
    {
      ...productParams,
      agent: "opencode",
      limit: 100,
    },
    env,
  );
  assertTerminalProductAggregatePage(opencodeAggregates, "opencode");
  if (
    opencodeAggregates.coverage?.completeness === "enumerable" ||
    typeof opencodeAggregates.coverage?.incomplete_reason !== "string" ||
    opencodeAggregates.page?.total_count !== null
  ) {
    fail("incomplete opencode aggregate list asserted an exact total");
  }
  if (
    !opencodeAggregates.aggregates.some((aggregate) =>
      JSON.stringify(aggregate).toLowerCase().includes("compatibility"),
    )
  ) {
    fail("opencode compatibility source missing from product aggregates");
  }

  const sessionInventory = callService(
    "session.previewLocalSessions",
    {
      authorized_roots: [fixture.sessionRoot],
      auto_discover: false,
      agent: "claude-code",
      scope: "all",
      include_content_items: false,
      paging_mode: "keyset",
      limit: 20,
      sort: "modified_at",
      direction: "desc",
      project_root: fixture.projectRoot,
      current_cwd: fixture.projectCwd,
    },
    env,
  );
  if (
    sessionInventory.count !== 1 ||
    sessionInventory.session_rows?.length !== 1 ||
    sessionInventory.source_completeness !== "enumerable" ||
    typeof sessionInventory.source_revision !== "string"
  ) {
    fail("isolated Claude session inventory was not complete and deterministic");
  }
  const session = sessionInventory.session_rows[0];
  const messages = callService(
    "session.listLocalSessionMessages",
    {
      authorized_roots: [fixture.sessionRoot],
      auto_discover: false,
      agent: "claude-code",
      project_root: fixture.projectRoot,
      current_cwd: fixture.projectCwd,
      session_id: session.id,
      limit: 40,
    },
    env,
  );
  if (
    messages.session_id !== session.id ||
    messages.returned_count !== 2 ||
    messages.source_completeness !== "enumerable" ||
    messages.provider_request_sent === true
  ) {
    fail("session detail did not return the complete isolated read-only transcript");
  }

  const resumeParams = {
    authorized_roots: [fixture.sessionRoot],
    auto_discover: false,
    agent: "claude-code",
    project_root: fixture.projectRoot,
    current_cwd: fixture.projectCwd,
    session_id: session.id,
    expected_source_revision: sessionInventory.source_revision,
    expected_snapshot_revision: readiness.source_revision,
  };
  const continuation = callService("session.previewResume", resumeParams, env);
  if (
    continuation.resume?.state !== "supported" ||
    continuation.resume?.copy_only !== true ||
    JSON.stringify(continuation.resume?.argv) !==
      JSON.stringify(["claude", "--resume", "claude-native-smoke"])
  ) {
    fail("session continuation preview was not the documented copy-only Claude argv");
  }
  const sessionBytesBeforeStalePreview = readFileSync(fixture.sessionPath, "utf8");
  expectServiceError(
    "session.previewResume",
    {
      ...resumeParams,
      expected_source_revision: "sha256:stale-session",
    },
    env,
    /source_changed|source changed/i,
    "stale session continuation",
  );
  if (readFileSync(fixture.sessionPath, "utf8") !== sessionBytesBeforeStalePreview) {
    fail("rejected stale session continuation mutated its source");
  }

  const llmStatus = callService("llm.status", {}, env);
  if (llmStatus.enabled !== false || llmStatus.configured !== false) {
    fail("fixture provider-off state was not preserved");
  }
  const promptPreview = callService(
    "llm.previewPrompt",
    {
      action: "project_health",
      user_intent: "Review only the accepted fixture evidence.",
      source_revision: readiness.source_revision,
    },
    env,
  );
  if (
    promptPreview.allowed !== false ||
    promptPreview.provider_request_sent !== false ||
    promptPreview.response_contract?.source_revision !== readiness.source_revision ||
    promptPreview.response_contract?.project_id !== projectID
  ) {
    fail("provider-off prompt preview lost its evidence binding or attempted a request");
  }
  expectServiceError(
    "llm.previewPrompt",
    {
      action: "project_health",
      user_intent: "Reject stale fixture evidence.",
      source_revision: "sha256:stale-product",
    },
    env,
    /source_changed|source changed/i,
    "stale AI evidence",
  );
  expectServiceError(
    "project.getReadiness",
    {
      ...productParams,
      source_revision: "sha256:stale-product",
    },
    env,
    /source_changed|source changed/i,
    "stale overview evidence",
  );

  note(
    "fixture product surface smoke passed: readiness, aggregate provenance, " +
      "incomplete coverage, session detail, copy-only continuation, provider-off AI, stale rejection",
  );
}

function assertTerminalProductAggregatePage(result, label) {
  if (
    typeof result?.source_revision !== "string" ||
    !Array.isArray(result?.aggregates) ||
    result.page?.returned_count !== result.aggregates.length ||
    result.page?.has_more !== false ||
    result.page?.next_cursor != null
  ) {
    fail(`${label} aggregate projection did not return one terminal bounded page`);
  }
}

function assertFixturePiGlobalSmoke(skills) {
  assertSkillPresent(
    skills,
    "pi",
    "pi-global-smoke",
    "global Pi native fixture missing from no-project scanAll",
  );
  assertSkillPresent(
    skills,
    "pi",
    "pi-agent-global-smoke",
    "global Pi compatibility fixture missing from no-project scanAll",
  );
  assertSkillNotCurrentVisible(
    skills,
    "pi-project-smoke",
    "project Pi fixture should not be visible before project context is active",
    "pi",
  );
  note("fixture Pi global smoke passed: native and .agents compatibility roots visible without project context");
}

function assertFixtureOpencodeGlobalSmoke(skills) {
  assertSkillPresent(
    skills,
    "opencode",
    "opencode-global-smoke",
    "global opencode fixture missing from no-project scanAll",
  );
  assertSkillPresent(
    skills,
    "opencode",
    "opencode-configured-smoke",
    "configured opencode skills.paths fixture missing from no-project scanAll",
  );
  assertSkillNotCurrentVisible(
    skills,
    "opencode-project-smoke",
    "project opencode fixture should not be visible before project context is active",
    "opencode",
  );
  note("fixture opencode global smoke passed: native and configured local roots visible without project context");
}

function runFixturePiCompatibilitySmoke(skills, env, fixture) {
  assertSkillPresent(
    skills,
    "pi",
    "pi-project-smoke",
    "project Pi native fixture missing after project context scanAll",
  );
  const compatSkill = assertSkillPresent(
    skills,
    "pi",
    "pi-agent-project-smoke",
    "project Pi compatibility fixture missing after project context scanAll",
  );

  const disabled = applySkillToggle(
    compatSkill.id,
    false,
    env,
    "Pi compatibility disable",
  );
  if (disabled.agent !== "pi" || disabled.enabled !== false) {
    fail("Pi compatibility toggle did not return a disabled Pi skill");
  }
  const settings = JSON.parse(readFileSync(fixture.projectPiSettings, "utf8"));
  const skillOverrides = Array.isArray(settings?.skills) ? settings.skills : [];
  if (!skillOverrides.some((entry) => String(entry).startsWith("-") && String(entry).includes("pi-agent-project-smoke"))) {
    fail("Pi project settings missing disabled compatibility skill entry");
  }
  const rescanned = refreshCatalog("catalog.scanAll", env);
  const rescannedSkill = assertSkillPresent(
    rescanned.skills,
    "pi",
    "pi-agent-project-smoke",
    "project Pi compatibility fixture missing after toggle rescan",
    { allowDisabled: true },
  );
  if (rescannedSkill.enabled !== false || rescannedSkill.state !== "disabled") {
    fail("Pi compatibility rescan did not preserve disabled state");
  }
  note("fixture Pi smoke passed: project compatibility root visible, toggle wrote .pi/settings.json, rescan preserved disabled state");
}

function runFixtureOpencodeWritableSmoke(skills, env, fixture) {
  const projectSkill = assertSkillPresent(
    skills,
    "opencode",
    "opencode-project-smoke",
    "project opencode fixture missing after project context scanAll",
  );
  if (existsSync(fixture.projectOpencodeConfig)) {
    fail(`project opencode config should not exist before toggle: ${fixture.projectOpencodeConfig}`);
  }
  const toggled = applySkillToggle(
    projectSkill.id,
    false,
    env,
    "opencode project disable",
  );
  if (toggled.agent !== "opencode" || toggled.enabled !== false) {
    fail("opencode toggle did not return a disabled opencode skill");
  }
  const config = JSON.parse(readFileSync(fixture.projectOpencodeConfig, "utf8"));
  if (config?.permission?.skill?.["opencode-project-smoke"] !== "deny") {
    fail("opencode project config missing managed permission.skill deny");
  }
  const rescanned = refreshCatalog("catalog.scanAll", env);
  const disabled = assertSkillPresent(
    rescanned.skills,
    "opencode",
    "opencode-project-smoke",
    "project opencode fixture missing after writable toggle rescan",
    { allowDisabled: true },
  );
  if (disabled.enabled !== false || disabled.state !== "disabled") {
    fail("opencode rescan did not preserve disabled permission.skill state");
  }
  note("fixture opencode smoke passed: project root visible, toggle wrote permission.skill deny, rescan preserved disabled state");
}

function runFixtureCodexConfigHardeningSmoke(env, fixture, skills) {
  if (existsSync(fixture.projectCodexConfig)) {
    fail(`project Codex config should not exist before toggle: ${fixture.projectCodexConfig}`);
  }

  const projectSkill = assertSkillPresent(
    skills,
    "codex",
    "codex-project-smoke",
    "project Codex fixture missing before config hardening toggle",
  );

  const seededConfig = readFixtureCodexConfig(fixture);
  assertCodexConfigPreserved(seededConfig, fixture, "seeded Codex config");
  assertPathOccurrence(
    seededConfig,
    fixture.codexTargetSkillPath,
    2,
    "seeded target duplicate entries",
  );
  assertPathOccurrence(
    seededConfig,
    fixture.codexNonTargetSkillPath,
    2,
    "seeded non-target duplicate entries",
  );

  const disabled = applySkillToggle(
    projectSkill.id,
    false,
    env,
    "Codex project disable",
  );
  if (disabled.enabled !== false) {
    fail("Codex project toggle off did not disable codex-project-smoke");
  }
  if (existsSync(fixture.projectCodexConfig)) {
    fail(`Codex project toggle wrote project config: ${fixture.projectCodexConfig}`);
  }
  const disabledConfig = readFixtureCodexConfig(fixture);
  assertCodexConfigPreserved(disabledConfig, fixture, "disabled Codex config");
  assertPathOccurrence(
    disabledConfig,
    fixture.codexTargetSkillPath,
    1,
    "disabled target normalized entries",
  );
  assertPathOccurrence(
    disabledConfig,
    fixture.codexNonTargetSkillPath,
    2,
    "disabled non-target preserved entries",
  );
  assertConfigBlock(
    disabledConfig,
    fixture.codexTargetSkillPath,
    "enabled = false",
    "disabled target block",
  );

  const enabled = applySkillToggle(
    projectSkill.id,
    true,
    env,
    "Codex project enable",
  );
  if (enabled.enabled !== true) {
    fail("Codex project toggle on did not re-enable codex-project-smoke");
  }
  if (existsSync(fixture.projectCodexConfig)) {
    fail(`Codex project re-enable wrote project config: ${fixture.projectCodexConfig}`);
  }
  const enabledConfig = readFixtureCodexConfig(fixture);
  assertCodexConfigPreserved(enabledConfig, fixture, "re-enabled Codex config");
  assertPathOccurrence(
    enabledConfig,
    fixture.codexTargetSkillPath,
    0,
    "re-enabled target removed entries",
  );
  assertPathOccurrence(
    enabledConfig,
    fixture.codexNonTargetSkillPath,
    2,
    "re-enabled non-target preserved entries",
  );
  note("fixture Codex config hardening smoke passed: user config only, duplicate target normalization, non-target preservation");
}

function readFixtureCodexConfig(fixture) {
  if (!existsSync(fixture.codexUserConfig)) {
    fail(`fixture Codex user config missing: ${fixture.codexUserConfig}`);
  }
  return readFileSync(fixture.codexUserConfig, "utf8");
}

function assertCodexConfigPreserved(content, fixture, label) {
  for (const expected of [
    "# fixture comment preserved by Codex config patch",
    'model = "fixture-model"',
    "[sandbox]",
    'mode = "read-only"',
  ]) {
    if (!content.includes(expected)) {
      fail(`${label} did not preserve ${expected}`);
    }
  }
  if (!existsSync(fixture.codexUserConfig)) {
    fail(`${label} did not use fixture user Codex config`);
  }
}

function assertPathOccurrence(content, path, expected, label) {
  const count = content.split(path).length - 1;
  if (count !== expected) {
    fail(`${label}: expected ${expected} occurrences of ${path}, got ${count}`);
  }
}

function assertConfigBlock(content, path, expectedLine, label) {
  const block = content
    .split("[[skills.config]]")
    .slice(1)
    .find((candidate) => candidate.includes(path));
  if (!block) {
    fail(`${label} missing for ${path}`);
  }
  if (!block.includes(expectedLine)) {
    fail(`${label} did not include ${expectedLine}`);
  }
}

function assertSkillPresent(skills, agent, name, message, options = {}) {
  const skill = findSkill(skills, agent, name);
  if (!skill) {
    fail(message);
  }
  if (skill.state && skill.state !== "loaded" && !(options.allowDisabled && skill.state === "disabled")) {
    fail(`${message}; found ${name} with state ${skill.state}`);
  }
  return skill;
}

function assertSkillNotCurrentVisible(skills, name, message, agent = "codex") {
  const skill = findSkill(skills, agent, name);
  if (!skill) {
    return;
  }
  if (skill.state === "missing" || skill.visible === false || skill.current === false) {
    note(`${name} retained in catalog as non-current (${describeSkillState(skill)})`);
    return;
  }
  fail(`${message}; found ${name} as ${describeSkillState(skill)}`);
}

function findSkill(skills, agent, name) {
  if (!Array.isArray(skills)) {
    fail("scan result did not include a skills array");
  }
  return skills.find((skill) => skill.agent === agent && skill.name === name);
}

function assertProjectContextState(state, expectActive, label) {
  if (!state || typeof state !== "object") {
    fail(`${label} did not return a project context state object`);
  }
  if (!Array.isArray(state.recent)) {
    fail(`${label} did not return recent project contexts`);
  }
  if (expectActive) {
    if (!state.active || typeof state.active !== "object") {
      fail(`${label} did not return an active project context`);
    }
    return;
  }
  if (state.active !== null && state.active !== undefined) {
    fail(`${label} should not have an active project context`);
  }
}

function describeSkillState(skill) {
  const fields = [
    `state=${skill.state ?? "unknown"}`,
    `scope=${skill.scope ?? "unknown"}`,
    `visible=${skill.visible ?? "unknown"}`,
    `current=${skill.current ?? "unknown"}`,
  ];
  return fields.join(", ");
}

function checkSystemLogs(pid) {
  const result = tryRun("/usr/bin/log", [
    "show",
    "--last",
    "5m",
    "--style",
    "compact",
    "--predicate",
    `processID == ${pid} AND (messageType == error OR messageType == fault)`,
  ]);
  if (!result.ok) {
    fail(result.stderr || "failed to read macOS unified log");
  }

  const entries = result.stdout
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("Timestamp ") && line !== "}");
  const unknown = entries.filter(
    (line) => !knownBenignLogPatterns.some((pattern) => pattern.test(line)),
  );
  note(
    `system log check: ${entries.length} error/fault lines, ${unknown.length} unknown after filters`,
  );
  if (unknown.length > 0) {
    for (const line of unknown.slice(0, 20)) {
      console.error(line);
    }
    fail("system log check found unknown app error/fault lines");
  }
}

try {
  const options = parseSmokeOptions(process.argv.slice(2), process.env);
  await runSmokeFlow(options, {
    assertRealOpencodeConfigUntouched,
    captureAppWindow,
    checkSystemLogs,
    cleanupFixture(root) {
      cleanupOwnedTemporaryRoot(root);
    },
    createFixtureEnvironment,
    launchApp,
    note,
    runFixtureProjectContextSmoke,
    runFixtureServiceSmoke,
    terminateExistingApp,
    verifyBundle,
    verifyBundleFreshness,
  });
} catch (error) {
  if (error instanceof SmokeFailure) {
    console.error(`smoke: ${error.message}`);
  } else {
    console.error(error);
  }
  process.exit(1);
}
