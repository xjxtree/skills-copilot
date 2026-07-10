# Quality, CI, and Governance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Consume the config consistency protocol safely in macOS, make every Swift test entrypoint prove the complete native suite, validate the bundled Rust sidecar without a GUI, run CI on all feature pushes, and enforce documentation, link, gate, performance, and module-size contracts from checked-in manifests.

**Architecture:** The macOS client carries config revisions and preview-bound rollback tokens without automatic stale-write retries. Native model suites have one Swift registry, one XCTest entrypoint, one executable entrypoint, and a deterministic completion sentinel. CI builds the app bundle without launching it, then executes fixture RPCs directly against the sidecar embedded in that bundle. Repository governance and quality budgets live in small JSON manifests consumed by pure Node helpers with negative tests. The 10k benchmark builds first and measures only the benchmark test executable, so elapsed time and RSS exclude Cargo compilation.

**Tech Stack:** Swift 5.9/XCTest, SwiftPM, Rust sidecar bundle, Node.js 22 ESM and `node:test`, Bash, pnpm 11.5.1, GitHub Actions on macOS 15, `/usr/bin/time`, Git.

## Global Constraints

- Work only in an isolated `codex/` worktree; do not switch branches or edit the coordinator checkout.
- Task 1 consumes protocol version 2 from Task 3 of `2026-07-10-service-consistency-privacy.md`; implement those two tasks on the same integration branch before running native gates.
- Config save sends the revision paired with the currently displayed document. Rollback sends only the preview token returned for that snapshot; it never sends a naked expected revision.
- A `config_conflict` or `stale_preview_token` response must not trigger an automatic retry, silent draft replacement, or rollback without a new preview.
- `swift test --package-path apps/macos` and `pnpm test:macos-native-models` must execute the same complete suite registry.
- Hosted CI must not call `/usr/bin/open`, `launchctl`, Accessibility APIs, window capture, or any GUI activation path.
- Headless smoke must use a temporary HOME, app-data directory, and fixture roots, and must execute `dist/AgentCopilot.app/Contents/Resources/skills-copilot-service` directly.
- Real-local GUI verification remains in `pnpm check:macos`; this plan does not weaken or replace that requirement.
- Documentation policy checks apply only to manifest-declared policy documents; link and repository-path checks apply to every tracked Markdown file.
- Benchmark thresholds are maximums. Environment overrides may tighten them locally but may not raise them in CI.
- The 10k RSS and elapsed budget must cover only the warmed benchmark test process, not Cargo compilation or dependency resolution.
- Production `.rs`, `.swift`, and `.mjs` files default to 1,200 lines; test, fixture, fuzz/benchmark, and test-driver files default to 2,000 lines. Legacy overrides are exact, self-tightening ratchets, not targets. Do not raise or add a ratchet to accommodate new code; move helpers/tests into focused modules or reduce the existing file.
- All new Node helpers expose pure functions and have RED/GREEN negative tests before CI wiring.

---

## File Structure

### New files

- `apps/macos/Tests/SkillsCopilotTests/FullNativeModelSuiteTests.swift` — normal XCTest entrypoint for the complete native registry.
- `scripts/verify-macos-native-test-registry.mjs` — static registry completeness verifier.
- `scripts/tests/macos-native-test-registry.test.mjs` — missing/duplicate suite regression tests.
- `scripts/lib/smoke-options.mjs` — pure smoke CLI parsing and headless incompatibility rules.
- `scripts/tests/smoke-options.test.mjs` — option parser tests.
- `scripts/repository-governance.json` — documentation scopes, path/link rules, and exact gate membership.
- `scripts/lib/repository-governance.mjs` — pure Markdown reference and gate parsing helpers.
- `scripts/tests/repository-governance.test.mjs` — negative governance tests.
- `scripts/performance-budgets.json` — 10k elapsed/RSS and native-list p95 ceilings.
- `scripts/module-size-budgets.json` — default limits and audited per-file ratchets.
- `scripts/lib/quality-budgets.mjs` — budget parsing, metric extraction, and comparison helpers.
- `scripts/tests/quality-budgets.test.mjs` — missing/over-budget metric and module `+1` tests.

### Deleted file

- `apps/macos/Tests/SkillsCopilotTestHarness/NativeModelTestHarness.c` — constructor-based test-bundle exit path.

### Existing files modified

- Native service client/model/store/view tests — consume revision and preview-token protocol fields.
- `apps/macos/Package.swift`, `NativeModelTestRunner.swift`, and `scripts/test-macos-native-models.sh` — share one full suite registry without constructor exits.
- `scripts/smoke-macos-app.mjs` — add fixture-only headless sidecar mode.
- `package.json`, `.github/workflows/ci.yml`, `script/build_and_run.sh`, and `scripts/check-macos.mjs` — separate build from launch and wire headless CI.
- `scripts/verify-doc-governance.mjs` and governed Markdown files — enforce manifest coverage and remove stale paths/status claims.
- `scripts/benchmark-10k.mjs`, `scripts/benchmark-native-list-model.mjs`, and `scripts/verify-module-size.mjs` — consume checked-in quality budgets.

---

### Task 1: Consume config revisions and rollback preview tokens in macOS

**Files:**
- Modify: `apps/macos/Sources/SkillsCopilot/Services/ServiceClient.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Services/ServiceClientCatalogConfigRPC.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Models/SkillRecord.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/AgentConfigWorkspacePanel.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/FakeServiceScript.swift`

**Interfaces:**
- Consumes Rust protocol version 2 fields: `ConfigDocumentRecord.revision`, `SnapshotRollbackPreviewRecord.current_revision`, and `SnapshotRollbackPreviewRecord.preview_token`.
- Produces: `saveClaudeSettings(content: String, expectedRevision: String) async throws -> ConfigDocumentRecord`.
- Produces client RPC: `rollbackSnapshot(snapshotID: String, previewToken: String) async throws -> Int`.
- Changes store method to `rollbackSnapshot(snapshotID: String, previewToken: String) async -> Bool` so the view can invalidate a failed confirmation binding.
- Store save captures one `ConfigDocumentRecord` before suspension and sends its revision.
- Snapshot UI enables confirmation only for a loaded preview whose `snapshot.id` matches the selected snapshot and whose `rollbackSupported` is true.

- [ ] **Step 1: Add RED encoding and store behavior tests**

In `SkillStoreTests.swift`, add a recording service assertion for save and rollback:

Register each new async case in `run()` before adding its body:

```swift
try await runCase("configMutationsCarryFreshProtocolBindings") {
    try await configMutationsCarryFreshProtocolBindings()
}
try await runCase("configConflictsDoNotRetryOrDiscardLoadedState") {
    try await configConflictsDoNotRetryOrDiscardLoadedState()
}
try await runCase("staleRollbackTokensRequireAnotherPreview") {
    try await staleRollbackTokensRequireAnotherPreview()
}
```

```swift
private func configMutationsCarryFreshProtocolBindings() async throws {
    let fake = try FakeServiceScript()
    defer { fake.cleanup() }
    fake.activate(scenario: "config-cas")
    let store = SkillStore(service: fake.serviceClient())
    await store.loadClaudeSettings()
    let saved = await store.saveClaudeSettings(content: "{\"theme\":\"dark\"}\n")
    try expectEqual(saved, true, "fresh settings save")
    let preview = try await store.previewRollback(snapshotID: "snap-1")
    let rolledBack = await store.rollbackSnapshot(
        snapshotID: "snap-1",
        previewToken: preview.previewToken
    )
    try expectEqual(rolledBack, true, "fresh rollback")
    let calls = fake.calls()
    try expectContains(
        calls,
        "\"expected_revision\":\"sha256:settings-revision\"",
        "save revision"
    )
    try expectContains(
        calls,
        "\"preview_token\":\"sha256:rollback-preview\"",
        "rollback token"
    )
}
```

Add a conflict test where the fake returns service code `config_conflict`, then assert the store returns false, retains the existing `claudeSettings`, and does not call save a second time. Add a stale rollback test for `stale_preview_token` that asserts one call only and clears the view/store preview binding before another confirmation is allowed.

- [ ] **Step 2: Run the focused model test and verify RED**

```sh
pnpm test:macos-native-models
```

Expected: FAIL in the SkillStore group containing the new cases because the Swift records and client methods do not expose revision/token fields. The canonical script is required here because a `main`-only selection does not execute `SkillStoreTests`.

- [ ] **Step 3: Add exact Codable and Encodable fields**

Use these model/request shapes:

```swift
struct ConfigDocumentRecord: Codable, Hashable {
    let agent: String
    let scope: String
    let target: String
    let format: String
    let content: String
    let exists: Bool
    let revision: String
}

struct SaveClaudeSettingsParams: Encodable {
    let content: String
    let expectedRevision: String

    enum CodingKeys: String, CodingKey {
        case content
        case expectedRevision = "expected_revision"
    }
}

struct RollbackSnapshotParams: Encodable {
    let snapshotId: String
    let previewToken: String

    enum CodingKeys: String, CodingKey {
        case snapshotId = "snapshot_id"
        case previewToken = "preview_token"
    }
}
```

Add `currentRevision` and `previewToken` coding keys to `SnapshotRollbackPreviewRecord`; do not synthesize either value client-side.

- [ ] **Step 4: Update RPC signatures and store calls**

Implement:

```swift
func saveClaudeSettings(content: String, expectedRevision: String) async throws -> ConfigDocumentRecord {
    try await call(
        method: "config.saveClaudeSettings",
        params: SaveClaudeSettingsParams(content: content, expectedRevision: expectedRevision)
    )
}

func rollbackSnapshot(snapshotID: String, previewToken: String) async throws -> Int {
    try await call(
        method: "snapshot.rollback",
        params: RollbackSnapshotParams(snapshotId: snapshotID, previewToken: previewToken)
    )
}
```

In `SkillStore.saveClaudeSettings`, capture `guard let loaded = claudeSettings` before the await and pass `loaded.revision`. Do not reload or overwrite the editor draft inside a `config_conflict` catch. In `SkillStore.rollbackSnapshot`, require a `previewToken` parameter, return `true` only after the single RPC and refresh succeed, and return `false` from the catch path without calling preview or rollback again.

- [ ] **Step 5: Require a matching preview before the destructive button is enabled**

In `AgentConfigSnapshotDetailPanel`, use:

```swift
private var confirmedPreview: SnapshotRollbackPreviewRecord? {
    guard let preview,
          preview.snapshot.id == snapshot.id,
          preview.rollbackSupported
    else { return nil }
    return preview
}
```

Disable rollback when `confirmedPreview == nil`. On confirmation, capture both `snapshot.id` and `confirmedPreview.previewToken` before starting the task. Await the store's Boolean result; on `false`, set `preview = nil`, copy `store.errorMessage` into `previewError`, and require the user to press Preview again. This covers `stale_preview_token` without inspecting localized message text or retrying.

- [ ] **Step 6: Update fake protocol version and payloads**

Change fake `protocol_version` values to `2`. Return revision fields from config reads and both token fields from rollback preview. Record the exact `expected_revision` and `preview_token` request values so the tests in Step 1 inspect serialized wire behavior, not only Swift arguments.

- [ ] **Step 7: Run GREEN native config tests**

```sh
pnpm test:macos-native-models
pnpm verify:service-protocol-drift
```

Expected: both commands exit 0; fresh save and rollback requests carry the server-issued binding; conflict paths call the mutation RPC exactly once and do not auto-retry.

- [ ] **Step 8: Commit the native protocol consumer**

```sh
git add apps/macos/Sources/SkillsCopilot/Services/ServiceClient.swift apps/macos/Sources/SkillsCopilot/Services/ServiceClientCatalogConfigRPC.swift apps/macos/Sources/SkillsCopilot/Models/SkillRecord.swift apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift apps/macos/Sources/SkillsCopilot/Views/AgentConfigWorkspacePanel.swift apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift apps/macos/Tests/SkillsCopilotTests/FakeServiceScript.swift
git commit -m "fix: bind native config writes to service previews"
```

---

### Task 2: Replace the partial Swift test bundle with one complete suite registry

**Files:**
- Create: `apps/macos/Tests/SkillsCopilotTests/FullNativeModelSuiteTests.swift`
- Create: `scripts/verify-macos-native-test-registry.mjs`
- Create: `scripts/tests/macos-native-test-registry.test.mjs`
- Modify: `apps/macos/Package.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift`
- Modify: `scripts/test-macos-native-models.sh`
- Modify: `package.json`
- Delete: `apps/macos/Tests/SkillsCopilotTestHarness/NativeModelTestHarness.c`

**Interfaces:**
- Produces: `runAllNativeModelTestsAsync() async throws -> NativeModelSuiteSummary`.
- Produces: `NativeModelSuiteSummary { serviceSuiteCount, mainSuiteCount, skillStoreGroupCount, namedExecutionCount }`.
- Required counts: service suites `2`, main suites `18`, SkillStore groups `64`, named executions `84`.
- Completion line: `SkillsCopilotTests: full-suite-complete service=2 main=18 skill-store-groups=64 named=84`.
- Produces: `verifyNativeTestRegistry({ discoveredTypes, registeredMainTypes }): string[]` in the Node verifier.

- [ ] **Step 1: Write RED registry verifier tests**

Create `scripts/tests/macos-native-test-registry.test.mjs`:

```js
import assert from "node:assert/strict";
import test from "node:test";
import { verifyNativeTestRegistry } from "../verify-macos-native-test-registry.mjs";

test("reports missing native model suites", () => {
  assert.deepEqual(
    verifyNativeTestRegistry({
      discoveredTypes: ["FindingDisplayModelTests", "SkillListModelTests"],
      registeredMainTypes: ["FindingDisplayModelTests"],
      serviceTypes: [],
      shardedType: "SkillStoreTests",
    }),
    ["unregistered native test types: SkillListModelTests"],
  );
});

test("reports duplicate registrations", () => {
  assert.deepEqual(
    verifyNativeTestRegistry({
      discoveredTypes: ["FindingDisplayModelTests"],
      registeredMainTypes: ["FindingDisplayModelTests", "FindingDisplayModelTests"],
      serviceTypes: [],
      shardedType: "SkillStoreTests",
    }),
    ["duplicate native test registrations: FindingDisplayModelTests"],
  );
});
```

- [ ] **Step 2: Run the verifier tests and verify RED**

```sh
node --test scripts/tests/macos-native-test-registry.test.mjs
```

Expected: FAIL because the verifier module and export do not exist.

- [ ] **Step 3: Define one explicit Swift registry and full runner**

In `NativeModelTestRunner.swift`, define the 18 synchronous main entries exactly once:

```swift
private let mainNativeModelSuites: [(String, () throws -> Void)] = [
    ("FindingDisplayModelTests", { try FindingDisplayModelTests().run() }),
    ("FindingExplainabilityModelTests", { try FindingExplainabilityModelTests().run() }),
    ("RuleTuningModelTests", { try RuleTuningModelTests().run() }),
    ("ProviderObservabilityModelTests", { try ProviderObservabilityModelTests().run() }),
    ("TaskCockpitModelTests", { try TaskCockpitModelTests().run() }),
    ("TaskInputModelTests", { try TaskInputModelTests().run() }),
    ("AIProviderModelTests", { try AIProviderModelTests().run() }),
    ("LLMModelTests", { try LLMModelTests().run() }),
    ("ScriptExecutionModelTests", { try ScriptExecutionModelTests().run() }),
    ("ToolGlobalModelTests", { try ToolGlobalModelTests().run() }),
    ("SkillManagerModelTests", { try SkillManagerModelTests().run() }),
    ("AgentConfigTimelineModelTests", { try AgentConfigTimelineModelTests().run() }),
    ("ConfigContentRedactorTests", { try ConfigContentRedactorTests().run() }),
    ("LocalizationModelTests", { try LocalizationModelTests().run() }),
    ("UIOptimizationModelTests", { try UIOptimizationModelTests().run() }),
    ("MainWindowModelTests", { try MainWindowModelTests().run() }),
    ("LocalSessionPreviewModelTests", { try LocalSessionPreviewModelTests().run() }),
    ("SkillListModelTests", { try SkillListModelTests().run() }),
]

struct NativeModelSuiteSummary: Equatable {
    let serviceSuiteCount: Int
    let mainSuiteCount: Int
    let skillStoreGroupCount: Int
    let namedExecutionCount: Int
}

func runAllNativeModelTestsAsync() async throws -> NativeModelSuiteSummary {
    try await runAsyncNamed("ServiceClientProcessTests") { try await ServiceClientProcessTests().run() }
    try await runAsyncNamed("ServiceClientRPCTests") { try await ServiceClientRPCTests().run() }
    for (name, run) in mainNativeModelSuites {
        try runNamed(name, run)
    }
    let groupCount = 64
    for group in 0..<groupCount {
        try await runAsyncNamed("SkillStoreTests group \(group)") {
            try await SkillStoreTests(selectedGroup: group, groupCount: groupCount).run()
        }
    }
    let summary = NativeModelSuiteSummary(
        serviceSuiteCount: 2,
        mainSuiteCount: mainNativeModelSuites.count,
        skillStoreGroupCount: groupCount,
        namedExecutionCount: 2 + mainNativeModelSuites.count + groupCount
    )
    fputs(
        "SkillsCopilotTests: full-suite-complete service=\(summary.serviceSuiteCount) main=\(summary.mainSuiteCount) skill-store-groups=\(summary.skillStoreGroupCount) named=\(summary.namedExecutionCount)\n",
        stderr
    )
    fflush(stderr)
    return summary
}
```

Focused suite selection may remain for developer diagnosis, but the no-environment default and both canonical entrypoints must call `runAllNativeModelTestsAsync()`.

- [ ] **Step 4: Replace the constructor with a normal XCTest**

Remove the C target/dependency from `Package.swift` and delete the C file. Create:

```swift
import XCTest
@testable import SkillsCopilot

final class FullNativeModelSuiteTests: XCTestCase {
    func testCompleteNativeModelRegistry() async throws {
        let summary = try await runAllNativeModelTestsAsync()
        XCTAssertEqual(summary.serviceSuiteCount, 2)
        XCTAssertEqual(summary.mainSuiteCount, 18)
        XCTAssertEqual(summary.skillStoreGroupCount, 64)
        XCTAssertEqual(summary.namedExecutionCount, 84)
    }
}
```

Remove the exported C symbol and every success-path `_exit` from `NativeModelTestRunner.swift`.

- [ ] **Step 5: Make the executable runner call the same function once**

In the generated `main.swift` used by `scripts/test-macos-native-models.sh`, use:

```swift
import Foundation

do {
    _ = try await runAllNativeModelTestsAsync()
} catch {
    fputs("SkillsCopilotTests: \(error)\n", stderr)
    exit(1)
}
```

Exclude `FullNativeModelSuiteTests.swift` from the executable package's test-source `rsync` so the standalone target does not import XCTest. Run the executable once instead of launching 67 separate processes. Move `BUILD_ROOT` under `${TMPDIR:-/tmp}` and add a trap that removes it on success or failure.

- [ ] **Step 6: Implement static discovery and sentinel verification**

`verify-macos-native-test-registry.mjs` must discover `^(struct|final class|class) ([A-Za-z0-9]+Tests)` in test files, exclude only `FullNativeModelSuiteTests`, classify `ServiceClientProcessTests` and `ServiceClientRPCTests` as service suites and `SkillStoreTests` as the sharded suite, and compare the remaining 18 types to `mainNativeModelSuites`. It must also accept `--log /absolute/test-output.log` and require the exact completion line with counts `2/18/64/84`. Export pure helpers and invoke filesystem/CLI work only when `process.argv[1] === fileURLToPath(import.meta.url)` so `node:test` imports do not execute the verifier main function.

- [ ] **Step 7: Wire and run GREEN checks**

Add:

```json
"verify:macos-native-test-registry": "node scripts/verify-macos-native-test-registry.mjs"
```

Insert `pnpm verify:macos-native-test-registry` in `verify:gate-parity` immediately after `pnpm verify:doc-governance`; Task 5 records that exact ordering in the governance manifest.

Run:

```sh
pnpm verify:macos-native-test-registry
swift test --package-path apps/macos --scratch-path /tmp/agent-copilot-swift-full 2>&1 | tee /tmp/agent-copilot-swift-full.log
pnpm verify:macos-native-test-registry -- --log /tmp/agent-copilot-swift-full.log
pnpm test:macos-native-models
```

Expected: static registry verification passes; both test entrypoints emit the exact full-suite sentinel and execute all 84 named units.

- [ ] **Step 8: Commit the complete suite runner**

```sh
git add apps/macos/Package.swift apps/macos/Tests/SkillsCopilotTests/FullNativeModelSuiteTests.swift apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift scripts/test-macos-native-models.sh scripts/verify-macos-native-test-registry.mjs scripts/tests/macos-native-test-registry.test.mjs package.json
git rm apps/macos/Tests/SkillsCopilotTestHarness/NativeModelTestHarness.c
git commit -m "test: make swift entrypoints run the full suite"
```

---

### Task 3: Add a fixture-only headless bundled-sidecar smoke mode

**Files:**
- Create: `scripts/lib/smoke-options.mjs`
- Create: `scripts/tests/smoke-options.test.mjs`
- Modify: `scripts/smoke-macos-app.mjs`

**Interfaces:**
- Produces: `parseSmokeOptions(argv: string[], env: NodeJS.ProcessEnv): SmokeOptions`.
- `SmokeOptions` includes `bundleOnly`, `fixtureData`, `headlessSidecar`, `keepOpen`, `captureWindow`, `checkLogs`, and `allowStaleApp`.
- `headlessSidecar` requires `fixtureData` and rejects `bundleOnly`, `keepOpen`, `captureWindow`, and `checkLogs`.
- Headless execution calls the existing `runFixtureServiceSmoke` and `runFixtureProjectContextSmoke` against the bundled sidecar without process/window management.

- [ ] **Step 1: Write RED option compatibility tests**

```js
import assert from "node:assert/strict";
import test from "node:test";
import { parseSmokeOptions } from "../lib/smoke-options.mjs";

test("headless sidecar requires fixture data", () => {
  assert.throws(() => parseSmokeOptions(["--headless-sidecar"], {}), /requires --fixture-data/);
});

for (const incompatible of ["--bundle-only", "--keep-open", "--capture-window", "--check-logs"]) {
  test(`headless sidecar rejects ${incompatible}`, () => {
    assert.throws(
      () => parseSmokeOptions(["--fixture-data", "--headless-sidecar", incompatible], {}),
      /cannot be combined/,
    );
  });
}

test("accepts fixture-only headless mode", () => {
  const options = parseSmokeOptions(["--fixture-data", "--headless-sidecar"], {});
  assert.equal(options.fixtureData, true);
  assert.equal(options.headlessSidecar, true);
});
```

- [ ] **Step 2: Run and verify RED**

```sh
node --test scripts/tests/smoke-options.test.mjs
```

Expected: FAIL because `scripts/lib/smoke-options.mjs` does not exist.

- [ ] **Step 3: Implement strict option parsing**

Create the helper with a known-flag set and throw on unknown arguments. Return booleans only after enforcing the headless constraints. Import it at the top of `smoke-macos-app.mjs` and replace all direct `process.argv.includes` calls with the parsed object.

- [ ] **Step 4: Add the no-GUI main branch**

Immediately after `verifyBundle()` and `verifyBundleFreshness()`, before any running-app query, add:

```js
if (headlessSidecar) {
  const fixture = createFixtureEnvironment();
  try {
    const env = {
      SKILLS_COPILOT_APP_DATA_DIR: fixture.appData,
      SKILLS_COPILOT_HOME: fixture.home,
    };
    note(`headless fixture data enabled: ${fixture.root}`);
    const status = runFixtureServiceSmoke(env);
    runFixtureProjectContextSmoke(env, fixture, status);
    assertRealOpencodeConfigUntouched(fixture.realOpencodeConfigSnapshot);
    note("headless bundled-sidecar smoke completed");
  } finally {
    rmSync(fixture.root, { force: true, recursive: true });
  }
  return;
}
```

This branch must not call `queryRunningApps`, `terminateExistingApp`, `launchApp`, `captureAppWindow`, or `checkSystemLogs`.

- [ ] **Step 5: Run GREEN option tests and integration smoke**

```sh
node --test scripts/tests/smoke-options.test.mjs
./script/build_and_run.sh --build-only
pnpm smoke:macos-app -- --fixture-data --headless-sidecar
```

Expected: option tests pass; bundle builds without launch; smoke validates fixture mutation/readback flows through the bundled service and leaves no AgentCopilot process.

- [ ] **Step 6: Commit headless smoke**

```sh
git add scripts/lib/smoke-options.mjs scripts/tests/smoke-options.test.mjs scripts/smoke-macos-app.mjs
git commit -m "test: add headless bundled sidecar smoke"
```

---

### Task 4: Make CI build-only, headless, and branch-complete

**Files:**
- Modify: `package.json`
- Modify: `.github/workflows/ci.yml`
- Modify: `script/build_and_run.sh`
- Modify: `scripts/check-macos.mjs`
- Modify: `AGENTS.md`
- Modify: `CONTRIBUTING.md`
- Modify: `docs/runbooks/macos-app-runbook.md`

**Interfaces:**
- `pnpm build:macos` builds/assembles/signs but never launches.
- `pnpm verify:macos-launch` explicitly performs local launch/window verification.
- CI macOS smoke command is `pnpm smoke:macos-app -- --fixture-data --headless-sidecar`.
- `pnpm check:macos` remains the real-local combined GUI gate and calls `--verify` explicitly.

- [ ] **Step 1: Write a RED static CI assertion**

Add to `scripts/tests/smoke-options.test.mjs` a test that reads `.github/workflows/ci.yml` and `package.json`, then asserts the macOS job contains `--headless-sidecar`, does not contain `--bundle-only`, and `build:macos` contains `--build-only` rather than `--verify`.

- [ ] **Step 2: Run and verify RED**

```sh
node --test scripts/tests/smoke-options.test.mjs
```

Expected: FAIL on current build and smoke commands.

- [ ] **Step 3: Separate build from launch in package scripts**

Use these exact script values:

```json
"build": "./script/build_and_run.sh --build-only",
"build:macos": "./script/build_and_run.sh --build-only",
"verify:macos-launch": "./script/build_and_run.sh --verify"
```

Keep architecture-specific build scripts on `--build-only`. Update `build_and_run.sh` help/freshness messages to recommend `pnpm build:macos` for rebuilding and `pnpm verify:macos-launch` only for interactive launch proof.

- [ ] **Step 4: Expand push coverage and replace GUI-dependent CI steps**

Change workflow triggers to:

```yaml
on:
  pull_request:
  push:
```

In the macOS job, replace the conventional build/smoke tail with:

```yaml
- name: Verify native test registry
  run: pnpm verify:macos-native-test-registry

- name: Test SwiftPM package completely
  run: swift test --package-path apps/macos --scratch-path "$RUNNER_TEMP/swift-tests"

- name: Build native macOS app bundle
  run: pnpm build:macos

- name: Smoke bundled Rust sidecar with fixtures
  run: pnpm smoke:macos-app -- --fixture-data --headless-sidecar
```

Keep `pnpm test:macos-native-models` because it validates the standalone executable entrypoint; remove the redundant plain `swift build` step.

- [ ] **Step 5: Keep local GUI verification explicit**

Do not replace the `./script/build_and_run.sh --verify` and fixture window-capture steps in `scripts/check-macos.mjs`. Update docs to distinguish CI headless sidecar evidence from real-local launch/window evidence.

- [ ] **Step 6: Run GREEN static and local headless checks**

```sh
node --test scripts/tests/smoke-options.test.mjs
before_pids="$(pgrep -f '/AgentCopilot.app/Contents/MacOS/AgentCopilot' || true)"
pnpm build:macos
pnpm smoke:macos-app -- --fixture-data --headless-sidecar
after_pids="$(pgrep -f '/AgentCopilot.app/Contents/MacOS/AgentCopilot' || true)"
test "$before_pids" = "$after_pids"
```

Expected: commands exit 0 and the before/after app PID set is identical, including when an unrelated pre-existing local instance exists.

- [ ] **Step 7: Commit CI wiring**

```sh
git add package.json .github/workflows/ci.yml script/build_and_run.sh scripts/check-macos.mjs AGENTS.md CONTRIBUTING.md docs/runbooks/macos-app-runbook.md scripts/tests/smoke-options.test.mjs
git commit -m "ci: use headless bundled sidecar validation"
```

---

### Task 5: Replace hardcoded documentation checks with a repository governance manifest

**Files:**
- Create: `scripts/repository-governance.json`
- Create: `scripts/lib/repository-governance.mjs`
- Create: `scripts/tests/repository-governance.test.mjs`
- Modify: `scripts/verify-doc-governance.mjs`
- Modify: `package.json`

**Interfaces:**
- Produces: `collectMarkdownReferences(markdown, sourcePath): RepositoryReference[]`.
- Produces: `collectDeclaredCreatePaths(markdown, sourcePath): Set<string>` for exact `- Create: \`path\`` entries in implementation plans.
- Produces: `validateReferences({ references, trackedFiles, headingsByFile, declaredCreates? }): string[]`; omitted `declaredCreates` means an empty map.
- Produces: `parseGateMembers(command: string): string[]`.
- Produces: `validateGateMembers(actual: string[], expected: string[]): string[]`.
- Governance script enumerates Markdown with `git ls-files -z -- '*.md'`.
- Markdown links resolve relative to their source; backticked `docs/`, `fixtures/`, `.github/`, `scripts/`, and root Markdown paths resolve from the repository root.

- [ ] **Step 1: Write RED pure-function tests**

Create tests for a missing relative link, a missing backticked repository path, an absent anchor, one missing gate member, one extra gate member, and order drift:

```js
import assert from "node:assert/strict";
import test from "node:test";
import {
  collectDeclaredCreatePaths,
  collectMarkdownReferences,
  validateGateMembers,
  validateReferences,
} from "../lib/repository-governance.mjs";

test("finds markdown links and backticked repository paths", () => {
  const refs = collectMarkdownReferences(
    "Read [runbook](runbooks/app.md#smoke) and `docs/missing.md`.",
    "docs/index.md",
  );
  assert.deepEqual(refs.map((ref) => ref.target), ["docs/runbooks/app.md#smoke", "docs/missing.md"]);
});

test("reports missing files and anchors", () => {
  const errors = validateReferences({
    references: [
      { source: "docs/index.md", target: "docs/missing.md", line: 1 },
      { source: "docs/index.md", target: "docs/runbooks/app.md#missing", line: 1 },
    ],
    trackedFiles: new Set(["docs/index.md", "docs/runbooks/app.md"]),
    headingsByFile: new Map([["docs/runbooks/app.md", new Set(["smoke"])]]),
  });
  assert.equal(errors.length, 2);
});

test("implementation plans allow declared creates but reject undeclared future paths", () => {
  const source = "docs/superpowers/plans/example.md";
  const markdown = [
    "- Create: `scripts/future.mjs`",
    "Use `scripts/future.mjs` and `scripts/typo.mjs`.",
  ].join("\n");
  const declaredCreates = new Map([[source, collectDeclaredCreatePaths(markdown, source)]]);
  const errors = validateReferences({
    references: collectMarkdownReferences(markdown, source),
    trackedFiles: new Set([source]),
    headingsByFile: new Map(),
    declaredCreates,
  });
  assert.deepEqual(errors, ["docs/superpowers/plans/example.md:2 -> scripts/typo.mjs is missing"]);
});

test("gate members require exact order and membership", () => {
  assert.deepEqual(
    validateGateMembers(["verify:a", "verify:c"], ["verify:a", "verify:b"]),
    ["gate members differ: expected verify:a -> verify:b; actual verify:a -> verify:c"],
  );
});
```

- [ ] **Step 2: Run and verify RED**

```sh
node --test scripts/tests/repository-governance.test.mjs
```

Expected: FAIL because the helper module does not exist.

- [ ] **Step 3: Create the governance manifest**

Use this initial manifest:

```json
{
  "schema_version": 1,
  "policy_documents": [
    "README.md",
    "AGENTS.md",
    "CONTRIBUTING.md",
    "docs/ai-agent-workflow.md",
    "docs/plans/roadmap.md",
    "docs/plans/development-tasks.md",
    "docs/runbooks/release-checklist.md",
    "docs/runbooks/distribution-runbook.md",
    "docs/runbooks/macos-app-runbook.md",
    "docs/ui-artifacts/README.md",
    "fixtures/opencode/README.md",
    "fixtures/pi/README.md",
    ".github/pull_request_template.md",
    ".github/ISSUE_TEMPLATE/feature_request.md"
  ],
  "required_text": {
    "README.md": ["## App Features", "GitHub Releases"],
    "AGENTS.md": ["## Safety Boundaries"],
    "docs/plans/roadmap.md": ["## Near-Term Work"],
    "docs/plans/development-tasks.md": ["## Active Task Rules"],
    "docs/runbooks/distribution-runbook.md": ["GitHub tags and GitHub Releases"]
  },
  "forbidden_patterns": [
    "\\bV\\d+\\.\\d+\\b",
    "MVP\\s*(?:/|\\|)\\s*V1",
    "Current (?:Status|State|Baseline)",
    "Completed baseline",
    "Current phase"
  ],
  "forbidden_paths": [
    "CHANGELOG.md",
    "docs/verification"
  ],
  "gate": {
    "script": "verify:gate-parity",
    "members": [
      "verify:service-protocol-drift",
      "verify:module-size",
      "verify:doc-governance",
      "verify:macos-native-test-registry",
      "verify:js-syntax",
      "verify:rust-docs",
      "verify:validation-blockers",
      "verify:screenshot-artifacts"
    ]
  }
}
```

The policy pattern scan is limited to `policy_documents`; every tracked Markdown file participates in link/path validation.

- [ ] **Step 4: Implement reference and gate validation**

The helper must remove fenced code blocks before extracting links or backticked paths, ignore external `http`, `https`, `mailto`, and image data URLs, normalize `.`/`..`, strip Markdown angle brackets, calculate GitHub-style lowercase heading slugs, and report `source:line -> target`. Resolve every Markdown link relative to its source document, including a link whose target is merely `README.md`. For backticked path tokens only, accept repository-looking prefixes plus bare Markdown filenames; resolve the known root tokens `README.md`, `AGENTS.md`, `CONTRIBUTING.md`, and `CLAUDE.md` from repository root and resolve other bare Markdown filenames relative to their source. This prevents command examples and arbitrary prose from being treated as files while keeping sibling-plan references valid.

Implementation plans necessarily name files before they exist. For sources under `docs/superpowers/plans/`, parse exact `- Create: \`repository/path\`` lines outside code fences and allow a missing backticked path only when that same plan declares it. Do not apply this allowance to Markdown links, `Modify`/`Delete`/`Regenerate` entries, ordinary prose, or another plan's declarations. Add the declared-create test from Step 1 so an undeclared typo remains fatal.

Refactor `verify-doc-governance.mjs` to load the manifest, enumerate tracked Markdown, apply policy rules, validate all references, parse `package.json`'s gate command by splitting `&&` terms of the form `pnpm <script>`, and fail on exact-member/order drift.

- [ ] **Step 5: Add deterministic scripts and run GREEN unit tests**

Add:

```json
"test:repository-governance": "node --test scripts/tests/repository-governance.test.mjs"
```

Run:

```sh
pnpm test:repository-governance
```

Expected: pure tests pass; repository verification remains RED only on the governed content corrected in Task 6.

- [ ] **Step 6: Commit the governance engine**

```sh
git add scripts/repository-governance.json scripts/lib/repository-governance.mjs scripts/tests/repository-governance.test.mjs scripts/verify-doc-governance.mjs package.json
git commit -m "test: make repository governance manifest-driven"
```

---

### Task 6: Correct governed documentation and path references

**Files:**
- Modify: `CONTRIBUTING.md`
- Modify: `fixtures/opencode/README.md`
- Modify: `fixtures/pi/README.md`
- Modify: `.github/pull_request_template.md`
- Modify: `.github/ISSUE_TEMPLATE/feature_request.md`
- Modify: `docs/runbooks/macos-app-runbook.md`

**Interfaces:**
- Consumes the policy and reference rules from `scripts/repository-governance.json`.
- Produces no runtime API.

- [ ] **Step 1: Run the repository verifier and capture RED output**

```sh
pnpm verify:doc-governance
```

Expected: FAIL on stale version/scope wording, broken repository paths, and gate claims.

- [ ] **Step 2: Replace contribution status snapshots with durable scope language**

Rewrite the `CONTRIBUTING.md` introduction and checklist around the current architecture and links. Replace `docs/macos-app-runbook.md` with `docs/runbooks/macos-app-runbook.md`. Remove all milestone labels from current-scope/checklist text. Keep release history directed to GitHub tags/Releases.

- [ ] **Step 3: Correct fixture contract paths and current behavior**

Use `docs/adapters/opencode-adapter-spec.md` and `docs/adapters/pi-adapter-spec.md`. Describe opencode's managed `permission.skill` write fixture as current guarded behavior under disposable HOME/project validation. Describe Pi fixtures as current parser/scan and guarded config contract evidence; retain the minimum validated Pi tool version as evidence rather than a future implementation condition.

- [ ] **Step 4: Remove milestone vocabulary from templates**

In the PR template, replace the old scope-boundary checkbox with:

```markdown
- [ ] I kept the current architecture, adapter, security, and write boundaries intact
```

In the feature request template, replace the milestone prompt with:

```markdown
Current / future / requires scoped safety review:
```

- [ ] **Step 5: Make runbook gate claims exact**

List only the eight manifest-declared `verify:gate-parity` members. State that live 10k/native-list benchmarks are separate CI/release commands and that headless sidecar smoke is separate from real-local GUI validation. Task 7 will add the deterministic quality-budget self-test as a ninth member and update this list in the same commit.

- [ ] **Step 6: Run GREEN documentation checks**

```sh
pnpm test:repository-governance
pnpm verify:doc-governance
```

Expected: both commands exit 0 with no missing file, anchor, path-token, policy, or gate-member error.

- [ ] **Step 7: Commit governed content**

```sh
git add CONTRIBUTING.md fixtures/opencode/README.md fixtures/pi/README.md .github/pull_request_template.md .github/ISSUE_TEMPLATE/feature_request.md docs/runbooks/macos-app-runbook.md
git commit -m "docs: align contribution and validation contracts"
```

---

### Task 7: Enforce performance and module-size budgets

**Files:**
- Create: `scripts/performance-budgets.json`
- Create: `scripts/module-size-budgets.json`
- Create: `scripts/lib/quality-budgets.mjs`
- Create: `scripts/tests/quality-budgets.test.mjs`
- Modify: `scripts/benchmark-10k.mjs`
- Modify: `scripts/benchmark-native-list-model.mjs`
- Modify: `scripts/verify-module-size.mjs`
- Modify: `package.json`
- Modify: `.github/workflows/ci.yml`
- Modify: `scripts/repository-governance.json`
- Modify: `docs/runbooks/macos-app-runbook.md`

**Interfaces:**
- Produces: `parseTenKMetrics(output: string): { elapsedMs: number, maxRssMb: number }`.
- Produces: `effectiveMaximum(manifestMaximum: number, override: string | undefined, ci: boolean): number`.
- Produces: `checkPerformanceBudget(metrics, budget): string[]`.
- Produces: `moduleClassFor(relativePath): "production" | "test_fixture_driver"`.
- Produces: `lineBudgetFor(relativePath, manifest): { className: string, maxLines: number, ratcheted: boolean }`.
- Produces: `verifyModuleInventory(files, manifest): string[]`, including missing-ratchet, over-ratchet, and stale-ratchet errors.
- 10k defaults: elapsed `8000ms`, RSS `640MiB`.
- Native list default: p95 `80ms` per scenario.
- Module defaults: production `1200` lines; test/fixture/test-driver `2000` lines.

- [ ] **Step 1: Write RED parser and budget tests**

Create:

```js
import assert from "node:assert/strict";
import test from "node:test";
import {
  checkPerformanceBudget,
  effectiveMaximum,
  moduleClassFor,
  parseTenKMetrics,
  verifyModuleInventory,
} from "../lib/quality-budgets.mjs";

test("parses warmed benchmark runtime metrics", () => {
  const metrics = parseTenKMetrics([
    "skills-copilot-bench scanned=10000 records=10000 elapsed_ms=3460 elapsed_s=3.460",
    "benchmark-runtime: max_rss_mb=404.5",
  ].join("\n"));
  assert.deepEqual(metrics, { elapsedMs: 3460, maxRssMb: 404.5 });
});

test("rejects missing runtime RSS", () => {
  assert.throws(
    () => parseTenKMetrics("skills-copilot-bench scanned=10000 records=10000 elapsed_ms=3460"),
    /missing benchmark-runtime max_rss_mb/,
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

test("CI overrides cannot loosen a manifest maximum", () => {
  assert.throws(() => effectiveMaximum(80, "81", true), /cannot loosen CI budget/);
  assert.equal(effectiveMaximum(80, "70", true), 70);
});

const moduleManifest = {
  defaults: { production: 1200, test_fixture_driver: 2000 },
  legacy_ratchets: {
    "crates/commands/src/lib.rs": 4741,
  },
};

test("classifies production, tests, fixtures, fuzz, and test drivers", () => {
  assert.equal(moduleClassFor("crates/service/src/lib.rs"), "production");
  assert.equal(moduleClassFor("scripts/lib/quality-budgets.mjs"), "production");
  assert.equal(moduleClassFor("crates/service/src/tests/llm_provider.rs"), "test_fixture_driver");
  assert.equal(moduleClassFor("apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift"), "test_fixture_driver");
  assert.equal(moduleClassFor("crates/adapters/fuzz/fuzz_targets/adapter_scan.rs"), "test_fixture_driver");
  assert.equal(moduleClassFor("scripts/smoke-macos-app.mjs"), "test_fixture_driver");
});

test("unknown new production and test files fail at 1201 and 2001", () => {
  assert.deepEqual(
    verifyModuleInventory([
      { path: "crates/commands/src/lib.rs", lines: 4741 },
      { path: "crates/new_feature.rs", lines: 1201 },
      { path: "crates/service/src/tests/new_feature.rs", lines: 2001 },
    ], moduleManifest),
    [
      "crates/new_feature.rs: 1201 lines exceeds production default 1200; new legacy ratchets are forbidden",
      "crates/service/src/tests/new_feature.rs: 2001 lines exceeds test_fixture_driver default 2000; new legacy ratchets are forbidden",
    ],
  );
});

test("legacy ratchets fail both growth and an unrecorded reduction", () => {
  assert.deepEqual(
    verifyModuleInventory([
      { path: "crates/commands/src/lib.rs", lines: 4742 },
    ], moduleManifest),
    ["crates/commands/src/lib.rs: 4742 lines exceeds legacy ratchet 4741"],
  );
  assert.deepEqual(
    verifyModuleInventory([
      { path: "crates/commands/src/lib.rs", lines: 4740 },
    ], moduleManifest),
    ["crates/commands/src/lib.rs: legacy ratchet 4741 must be lowered to 4740 in the same change"],
  );
});
```

- [ ] **Step 2: Run and verify RED**

```sh
node --test scripts/tests/quality-budgets.test.mjs
```

Expected: FAIL because the quality budget helper does not exist.

- [ ] **Step 3: Add exact checked-in budget manifests**

Create:

```json
{
  "schema_version": 1,
  "scan_10k": {
    "max_elapsed_ms": 8000,
    "max_rss_mb": 640
  },
  "native_list": {
    "max_p95_ms": 80
  }
}
```

Create:

```json
{
  "schema_version": 1,
  "defaults": {
    "production": 1200,
    "test_fixture_driver": 2000
  },
  "legacy_ratchets": {
    "crates/commands/src/lib.rs": 4741,
    "crates/commands/src/tests.rs": 4787,
    "apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift": 3315,
    "apps/macos/Sources/SkillsCopilot/Views/SidebarView.swift": 2698,
    "apps/macos/Sources/SkillsCopilot/Views/TaskCockpitPanel.swift": 1882,
    "apps/macos/Sources/SkillsCopilot/Models/ProviderObservability.swift": 1730,
    "apps/macos/Sources/SkillsCopilot/Models/TaskCockpit.swift": 1406,
    "apps/macos/Sources/SkillsCopilot/Support/UIStrings.swift": 1201,
    "apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift": 2105,
    "crates/catalog/src/lib.rs": 1523,
    "crates/commands/src/skill_manager.rs": 1583,
    "crates/scanner/src/lib.rs": 1547,
    "crates/service/src/lib.rs": 1411,
    "crates/service/src/provider.rs": 1361,
    "crates/service/src/service_host.rs": 1329,
    "crates/service/src/service_llm.rs": 1932,
    "crates/service/src/service_local_sessions.rs": 2817,
    "crates/service/src/tests/llm_provider.rs": 2797
  }
}
```

The JSON above is the complete audit of the current checkout: every production file above 1,200 and every test/fixture/test-driver file above 2,000 is listed. Immediately before creating the manifest, after Tasks 1–6 and the service-plan work have landed, rerun the inventory over all scanned `.rs`, `.swift`, and `.mjs` files. Keep this exact path set unless an earlier task split a file below its class default; remove such a path. Set every remaining value to that file's exact implementation-time line count, but never above the value shown above. If earlier work increased a listed file, split or simplify it before recording the manifest. If an additional file now exceeds its default, reduce or split it below the default rather than adding a new ratchet.

`moduleClassFor` must use these deterministic rules, in order:

1. Return `test_fixture_driver` when a case-insensitive path segment is `test`, `tests`, `testing`, `fixture`, `fixtures`, `fuzz`, `fuzz_targets`, `bench`, `benches`, or `benchmarks`.
2. Return `test_fixture_driver` when the filename stem is `test`, `tests`, or ends in `Test`/`Tests` (case-insensitive), including `.test.mjs`.
3. For files directly under `scripts/`, return `test_fixture_driver` when the basename begins `test-`, `verify-`, `smoke-`, `benchmark-`, or `check-`.
4. Otherwise return `production`.

The scan roots remain `crates`, `apps/macos/Sources`, `apps/macos/Tests`, and `scripts`; the covered extensions remain `.rs`, `.swift`, and `.mjs`. `scripts/lib/*.mjs` therefore stays production unless a more specific rule applies.

- [ ] **Step 4: Separate 10k compilation from measured execution**

Refactor `benchmark-10k.mjs` into two phases:

1. Run `cargo test -p skills-copilot-commands benchmark_10k_scan_to_catalog --no-run --message-format=json` without `/usr/bin/time`.
2. Parse every Cargo JSON `compiler-artifact` message for package `skills-copilot-commands`, retain each non-null executable whose target kind is `lib` or `test`, resolve it to an absolute path, and de-duplicate candidates.
3. Run every candidate with `--list`, collect `(executable, full test name)` pairs whose test name ends in `::benchmark_10k_scan_to_catalog`, and require exactly one pair. This avoids choosing the wrong lib/integration test artifact when Cargo emits several executables.
4. Run `/usr/bin/time` around only that resolved executable with the discovered full name plus `--ignored --nocapture --exact`.

The measured command must be equivalent to:

```sh
/usr/bin/time -l /absolute/path/to/skills_copilot_commands-test-binary tests::benchmark_10k_scan_to_catalog --ignored --nocapture --exact
```

On Linux use `/usr/bin/time -v` around the same test binary. Prefix the parsed memory line with `benchmark-runtime:`. Compilation output may be printed before the measurement but must not be passed to `parseTenKMetrics`.

- [ ] **Step 5: Require runtime metrics and enforce thresholds**

Load `performance-budgets.json`, parse `elapsed_ms` from the Rust benchmark line and RSS only from the timed test-binary process, then fail when either metric is absent or above budget. Keep `TEN_K_BENCH_MAX_ELAPSED_MS` and `TEN_K_BENCH_MAX_RSS_MB` as optional tightening overrides; reject higher values when `CI=true`.

Change `benchmark-native-list-model.mjs` to load `native_list.max_p95_ms`. Keep `NATIVE_LIST_BENCH_MAX_P95_MS` only as a tightening override under the same rule.

- [ ] **Step 6: Make module verification manifest-driven**

Load class defaults and exact path ratchets from `module-size-budgets.json`. Classify every scanned file with `moduleClassFor`, then enforce all of these invariants in one pass:

- A path without a ratchet must not exceed its class default; the error must say that new legacy ratchets are forbidden.
- A ratcheted path must exist, must still exceed its class default, and must have an actual line count exactly equal to its manifest value.
- Actual count above a ratchet is growth and fails. Actual count below a ratchet is also a failure that requires lowering the ratchet in the same change; when the count reaches the class default, require removing the ratchet.
- A ratchet for an unscanned extension, missing path, or path at/below its default fails as stale policy.

Print the number of production files, test/fixture/test-driver files, and active legacy ratchets. The tests from Step 1 must prove classification, failures for unknown new files at 1,201/2,001 lines, legacy growth by one line, and legacy reduction without the matching manifest decrease.

- [ ] **Step 7: Wire deterministic and live gates**

Add:

```json
"test:quality-budgets": "node --test scripts/tests/quality-budgets.test.mjs",
"verify:quality-budgets": "pnpm test:quality-budgets"
```

Insert `pnpm verify:quality-budgets` into `verify:gate-parity` immediately after `pnpm verify:module-size`. Add `verify:quality-budgets` at the same position in `repository-governance.json`, retain `verify:macos-native-test-registry` in its Task 5 position, and update the runbook's deterministic member list to the same nine-item order.

In macOS CI, warm/build first, then measure:

```yaml
- name: Build 10k benchmark executable
  run: cargo test -p skills-copilot-commands benchmark_10k_scan_to_catalog --no-run

- name: Enforce 10k runtime and RSS budget
  run: pnpm benchmark:10k

- name: Enforce native list p95 budget
  run: pnpm benchmark:macos-list-model
```

The benchmark script may perform its own `--no-run --message-format=json` lookup after the warm step, but `/usr/bin/time` must wrap only the resolved executable.

- [ ] **Step 8: Run GREEN budget checks**

```sh
pnpm test:quality-budgets
pnpm verify:module-size
pnpm benchmark:10k
pnpm benchmark:macos-list-model
```

Expected: all commands exit 0; 10k output identifies the test executable and reports runtime-only elapsed/RSS; no compilation RSS is presented as benchmark RSS.

- [ ] **Step 9: Commit quality budgets**

```sh
git add scripts/performance-budgets.json scripts/module-size-budgets.json scripts/lib/quality-budgets.mjs scripts/tests/quality-budgets.test.mjs scripts/benchmark-10k.mjs scripts/benchmark-native-list-model.mjs scripts/verify-module-size.mjs scripts/repository-governance.json docs/runbooks/macos-app-runbook.md package.json .github/workflows/ci.yml
git commit -m "test: enforce runtime and module quality budgets"
```

---

### Task 8: Run integrated quality, CI, and governance verification

**Files:**
- Modify only when a command exposes a mismatch in a file owned by Tasks 1–7.

**Interfaces:**
- Consumes all preceding tasks and the completed service consistency Task 3.
- Produces no new runtime API.

- [ ] **Step 1: Run deterministic Node and governance gates**

```sh
pnpm verify:macos-native-test-registry
node --test scripts/tests/macos-native-test-registry.test.mjs
node --test scripts/tests/smoke-options.test.mjs
pnpm test:repository-governance
pnpm test:quality-budgets
pnpm verify:doc-governance
pnpm verify:module-size
pnpm verify:gate-parity
```

Expected: every command exits 0 and the gate member list matches `repository-governance.json` exactly.

- [ ] **Step 2: Run both complete Swift entrypoints**

```sh
swift test --package-path apps/macos --scratch-path /tmp/agent-copilot-swift-final 2>&1 | tee /tmp/agent-copilot-swift-final.log
pnpm verify:macos-native-test-registry -- --log /tmp/agent-copilot-swift-final.log
pnpm test:macos-native-models
```

Expected: both entrypoints report service `2`, main `18`, SkillStore groups `64`, named executions `84`.

- [ ] **Step 3: Build without launch and run headless bundle smoke**

```sh
before_pids="$(pgrep -f '/AgentCopilot.app/Contents/MacOS/AgentCopilot' || true)"
pnpm build:macos
pnpm smoke:macos-app -- --fixture-data --headless-sidecar
after_pids="$(pgrep -f '/AgentCopilot.app/Contents/MacOS/AgentCopilot' || true)"
test "$before_pids" = "$after_pids"
```

Expected: build and fixture RPC smoke pass without changing the app PID set.

- [ ] **Step 4: Run live budgets after warm compilation**

```sh
cargo test -p skills-copilot-commands benchmark_10k_scan_to_catalog --no-run
pnpm benchmark:10k
pnpm benchmark:macos-list-model
```

Expected: runtime-only 10k elapsed is at most 8000ms, runtime-only RSS is at most 640MiB, and every native-list scenario p95 is at most 80ms.

- [ ] **Step 5: Run repository privacy and diff checks**

```sh
pnpm check:privacy
git diff --check
git status --short
git diff --stat
```

Expected: privacy reports clean, diff check has no output, and only plan-owned source/docs/workflow/manifest files are present.

- [ ] **Step 6: Commit an integration-only correction when needed**

Skip this commit if Steps 1–5 require no correction. Otherwise stage only the owning files and run:

```sh
git commit -m "test: align quality and governance gates"
```

---

## Final Acceptance Criteria

- Native settings save sends the loaded document revision; stale saves display `config_conflict` and are never retried automatically.
- Native rollback is disabled until a valid preview is loaded; confirmation sends only that preview token; `stale_preview_token` clears the binding and requires a new preview.
- The C constructor harness and success `_exit` path are gone.
- Both Swift entrypoints execute 2 service suites, 18 main suites, and 64 SkillStore groups and emit the exact `named=84` completion sentinel.
- Adding an unregistered `*Tests` type or duplicating a registration makes the static verifier fail.
- `pnpm build:macos` never launches the app; `pnpm verify:macos-launch` is the explicit local launch command.
- CI triggers on pull requests and every branch push, including `codex/**`.
- CI bundle validation directly executes the embedded sidecar with fixture data and does not use GUI/window/Accessibility paths.
- Every tracked Markdown link and anchor resolves. Every repository-looking backticked path resolves to a tracked file or, only inside an implementation plan, to an exact same-plan `Create` declaration; undeclared future-path typos fail.
- Policy documents contain no stale milestone/status language, and gate documentation matches the manifest-declared command list.
- `verify:gate-parity` has exact manifest membership and does not claim to execute live benchmarks.
- 10k elapsed/RSS budgets measure only the warmed test executable; Cargo compilation is outside `/usr/bin/time`.
- 10k elapsed is at most 8000ms, runtime RSS is at most 640MiB, and native-list p95 is at most 80ms.
- Production modules fail above 1,200 lines and test/fixture/test-driver modules fail above 2,000 lines unless they are in the complete audited legacy set. Every legacy ratchet equals the file's current count, cannot grow, and must be lowered or removed in the same change when the file shrinks.

## Execution Handoff

Execute this plan with `superpowers:subagent-driven-development` and review after every task. Task 1 must immediately follow Task 3 of `2026-07-10-service-consistency-privacy.md`. Use `superpowers:executing-plans` only when one worker must execute sequentially, with checkpoints after Tasks 2, 4, 6, and 7.
