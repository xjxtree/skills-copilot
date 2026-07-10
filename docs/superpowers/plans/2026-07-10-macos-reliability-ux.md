# macOS Reliability and UX Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the native macOS shell preserve privacy and user edits, keep service and cache state coherent under concurrency, and provide reliable keyboard, accessibility, and compact-window behavior.

**Architecture:** Keep filesystem and protocol behavior behind the existing typed Rust service and `ServiceClient`; extract timing, cache, request-generation, navigation, and privacy decisions into Foundation-only Swift models that the existing headless runner can exercise. Keep SwiftUI/AppKit views thin, reuse the current `NavigationSplitView`, `NativePanelSurface`, `PrivacyPathText`, `ConfigContentRedactor`, localization, and service patterns, and add a fixture-only AX driver for behavior that cannot be proven by model tests.

**Tech Stack:** Swift 5.9, SwiftUI, AppKit, Foundation `Process`/`Pipe`, macOS 13+, Rust workspace service protocol, JSON fixtures, Node.js verification scripts, pnpm.

## Global Constraints

- Support macOS 13 and the existing Swift 5.9 package declared in `apps/macos/Package.swift`.
- Keep product and filesystem logic in Rust workspace crates; the macOS shell must use the typed `ServiceClient` protocol and must not introduce direct agent-config writes.
- Keep `crates/core` no-I/O and do not recreate `ui/`, `src-tauri/`, or Tauri IPC.
- UI filters, scope pickers, sort/search controls, and navigation must derive from startup/manual-refresh cache. Fresh filesystem work is allowed only for startup prewarm, explicit refresh, or consistency-bound config mutation flows.
- Preserve the last successful data while refresh is pending or fails; expose loading, refreshing, stale, and failed states separately.
- Never persist raw Task Preflight text, provider prompt/response/trace material, credentials, or hidden task state.
- Credentials remain Keychain-backed and must not enter SQLite, project files, logs, screenshots, reports, history JSON, or diagnostics.
- A config save or rollback must use compare-and-swap state obtained from the matching read/preview. Missing CAS capability is read-only; there is no unsafe compatibility fallback.
- Skill Manager apply requests must use the exact immutable inputs and preview token that produced the visible confirmation.
- Reuse existing SwiftUI/AppKit components, typography, colors, spacing, icons, materials, and localization. Do not introduce a new visual system.
- Native model tests must remain Foundation-only because `scripts/test-macos-native-models.sh` excludes `Views/**`, `App/**`, and rejects AppKit/SwiftUI imports.
- Smoke and AX validation must use fixture HOME, app-data, project, and agent config roots. It must not read or write real user config.
- Completed UI evidence may capture only the full app window. If the session is locked or AX/window capture is unavailable, report the canonical blocker instead of substituting a smoke screenshot.
- Service behavior changes must update `docs/service-protocol.md`, `fixtures/service-protocol/`, and `scripts/verify-service-protocol-drift.mjs` expectations in the same task.
- Major UI and protocol work must finish with `pnpm check:macos`; every implementation commit and final handoff must pass `pnpm check:privacy`.

---

## File and Responsibility Map

### New Foundation-only production models

- `apps/macos/Sources/SkillsCopilot/Models/RevisionAutosaveCoordinator.swift`: serialized revision-based autosave state machine.
- `apps/macos/Sources/SkillsCopilot/Support/ServiceDiagnosticSanitizer.swift`: bounded path/token-redacted subprocess diagnostics.
- `apps/macos/Sources/SkillsCopilot/Models/ConfigMutationState.swift`: CAS conflict and rollback-confirmation state.
- `apps/macos/Sources/SkillsCopilot/Models/LocalSessionCache.swift`: source-keyed summary snapshots, projections, and bounded detail state.
- `apps/macos/Sources/SkillsCopilot/Models/AppSearchIndex.swift`: cache-backed skill/session/config-history search projection.
- `apps/macos/Sources/SkillsCopilot/Models/SkillManagerRequestState.swift`: immutable request keys, generations, and mutation confirmations.
- `apps/macos/Sources/SkillsCopilot/Models/AppNavigation.swift`: shared primary navigation destinations.
- `apps/macos/Sources/SkillsCopilot/Models/GlobalSearchInteractionModel.swift`: keyboard highlight and submit/dismiss state.
- `apps/macos/Sources/SkillsCopilot/Models/SensitiveTextPresentation.swift`: redacted/revealed display decisions with reset semantics.

### New test files

- `apps/macos/Tests/SkillsCopilotTests/TaskCockpitHistoryStoreTests.swift`
- `apps/macos/Tests/SkillsCopilotTests/RevisionAutosaveCoordinatorTests.swift`
- `apps/macos/Tests/SkillsCopilotTests/ConfigMutationModelTests.swift`
- `apps/macos/Tests/SkillsCopilotTests/LocalSessionCacheTests.swift`
- `apps/macos/Tests/SkillsCopilotTests/SkillManagerRequestGenerationTests.swift`
- `apps/macos/Tests/SkillsCopilotTests/AppNavigationModelTests.swift`
- `apps/macos/Tests/SkillsCopilotTests/GlobalSearchInteractionModelTests.swift`
- `apps/macos/Tests/SkillsCopilotTests/PrivacyPresentationModelTests.swift`
- `apps/macos/Tests/SkillsCopilotAXTestDriver/main.swift`
- `apps/macos/Tests/SkillsCopilotAXTestDriver/AXHarness.swift`
- `apps/macos/Tests/SkillsCopilotAXTestDriver/GlobalSearchAXTests.swift`
- `apps/macos/Tests/SkillsCopilotAXTestDriver/NavigationAndLayoutAXTests.swift`
- `apps/macos/Tests/SkillsCopilotAXTestDriver/PrivacyAXTests.swift`
- `scripts/test-macos-ax.sh`

### Existing integration points

- `apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift`: owns coordinators/caches and publishes UI-safe state.
- `apps/macos/Sources/SkillsCopilot/Stores/TaskCockpitHistoryStore.swift`: locates and deletes history files written by prior versions; it has no save path.
- `apps/macos/Sources/SkillsCopilot/Services/ServiceProcessRunner.swift`: concurrent incremental pipe draining with memory and timeout bounds.
- `apps/macos/Sources/SkillsCopilot/Services/ServiceClient.swift`: CAS request DTOs.
- `apps/macos/Sources/SkillsCopilot/Services/ServiceClientCatalogConfigRPC.swift`: typed CAS/rollback RPC methods.
- `apps/macos/Sources/SkillsCopilot/Views/AgentConfigWorkspacePanel.swift`: config editor and rollback view wiring.
- `apps/macos/Sources/SkillsCopilot/Views/SettingsView.swift`: provider autosave and selected-tab AX wiring.
- `apps/macos/Sources/SkillsCopilot/Views/ContentView.swift`: global search and compact column wiring.
- `apps/macos/Sources/SkillsCopilot/Views/SidebarView.swift`: shared navigation calls.
- `apps/macos/Sources/SkillsCopilot/Views/SkillManagerPanel.swift`: immutable preview/apply wiring.
- `apps/macos/Sources/SkillsCopilot/Views/PrivacyPathView.swift`: labeled reveal controls.
- `apps/macos/Sources/SkillsCopilot/Views/DetailFindingsHistorySection.swift`: privacy-safe snapshot preview.
- `apps/macos/Sources/SkillsCopilot/App/SkillsCopilotApp.swift`: command-menu navigation wiring.
- `apps/macos/Sources/SkillsCopilot/Models/MainWindowModel.swift`: accessibility IDs and compact breakpoints.
- `apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift`: registers every new Foundation-only suite.
- `apps/macos/Tests/SkillsCopilotTests/FakeServiceScript.swift`: deterministic success, conflict, delay, and stale-response fixtures.

## Delivery Sequence

Tasks 1-2 form the persistence/process safety package. Tasks 3-4 form mutation consistency. Tasks 5-6 form cache and async ordering. Tasks 7-8 form navigation, keyboard, AX, compact layout, and privacy presentation. Task 9 is the integrated release gate. Each task ends in a separately reviewable commit and must leave its focused checks green.

---

### Task 1: Keep Task Preflight History Session-only and Purge Legacy Files

**Files:**
- Create: `apps/macos/Tests/SkillsCopilotTests/TaskCockpitHistoryStoreTests.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Stores/TaskCockpitHistoryStore.swift:3-273`
- Modify: `apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift:135-142,399-410,2747-2761`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/TaskCockpitPanel.swift:149-171`
- Modify: `apps/macos/Sources/SkillsCopilot/Support/UIStrings.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Resources/en.lproj/Localizable.strings`
- Modify: `apps/macos/Sources/SkillsCopilot/Resources/zh-Hans.lproj/Localizable.strings`
- Modify: `apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift:147-170,1620-1646`
- Modify: `apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift:110-139`
- Modify: `scripts/verify-native-ui-layout.mjs`

**Interfaces:**
- Consumes: the existing in-memory `TaskCockpitHistoryRecord` model, `TaskCockpitHistoryStore.fileURL` injection seam, `TaskCockpitHistoryStore.maxRecords`, and successful Task Preflight results.
- Produces: `TaskCockpitHistoryPurgeOutcome`, `TaskCockpitHistoryStore.purgeLegacyHistoryIfPresent() throws`, and `SkillStore.clearTaskCockpitHistory()`. It intentionally produces no persistence schema and no save method.

- [ ] **Step 1: Add failing legacy-file purge and session-only tests**

Create and register this suite:

```swift
import Foundation
@testable import SkillsCopilot

struct TaskCockpitHistoryStoreTests {
    func run() throws {
        try missingHistoryFileNeedsNoCleanup()
        try legacyArrayIsDeleted()
        try versionOneEnvelopeIsDeleted()
        try versionTwoEnvelopeIsDeleted()
        try malformedHistoryFileIsDeleted()
        try purgeFailureDoesNotRenameOrCopySensitiveFile()
    }
}
```

Each format fixture must contain `SENSITIVE_SENTINEL_42` in task text, operation state, filters, summary, and result text. After `purgeLegacyHistoryIfPresent()` returns, assert:

```swift
try expectEqual(
    FileManager.default.fileExists(atPath: historyStore.fileURL.path),
    false,
    "Legacy Task Preflight history must be removed."
)
let siblingNames = try FileManager.default.contentsOfDirectory(
    atPath: historyStore.fileURL.deletingLastPathComponent().path
)
try expectFalse(
    siblingNames.contains { $0.contains("task-preflight-history") },
    "Cleanup must not retain a renamed or backup history file."
)
```

Inject a throwing remover into one test and assert the original file remains solely because deletion failed, no sibling backup is created, and the thrown error contains no file contents.

In `SkillStoreTests` replace the current cross-restart persistence expectation with:

- `taskCockpitHistoryStaysInCurrentSessionOnly`
- `newStoreDoesNotRestoreTaskCockpitHistory`
- `successfulTaskCockpitDoesNotCreateHistoryFile`
- `clearTaskCockpitHistoryClearsMemory`
- `legacyHistoryCleanupFailureShowsRedactedMessage`

Run: `pnpm test:macos-native-models`

Expected: FAIL because the current store loads and writes `task-preflight-history.json` and restores task text across store instances.

- [ ] **Step 2: Reduce the history store to deletion-only legacy cleanup**

Replace the persisted record/envelope types with this exact deletion-only API:

```swift
import Foundation

enum TaskCockpitHistoryPurgeOutcome: Equatable {
    case noFile
    case removed
}

struct TaskCockpitHistoryStore {
    static let maxRecords = 12

    let fileURL: URL
    private let fileExists: (String) -> Bool
    private let removeItem: (URL) throws -> Void

    init(
        fileURL: URL = TaskCockpitHistoryStore.defaultFileURL,
        fileExists: @escaping (String) -> Bool = FileManager.default.fileExists(atPath:),
        removeItem: @escaping (URL) throws -> Void = FileManager.default.removeItem(at:)
    ) {
        self.fileURL = fileURL
        self.fileExists = fileExists
        self.removeItem = removeItem
    }

    func purgeLegacyHistoryIfPresent() throws -> TaskCockpitHistoryPurgeOutcome {
        guard fileExists(fileURL.path) else { return .noFile }
        try removeItem(fileURL)
        return .removed
    }

    static var defaultFileURL: URL {
        appDataURL.appendingPathComponent("task-preflight-history.json", isDirectory: false)
    }
}
```

Keep the existing private `appDataURL` resolution so the code can find files written by prior versions. Remove `load()`, `save(_:)`, every stored envelope/record/result type, directory creation, JSON encoding, and atomic writes. Cleanup deletes any existing file bytes without parsing them, which covers legacy arrays, v1, v2, malformed, and unknown-version files without copying sensitive data.

The default remover is injectable only to make cleanup failure deterministic in Foundation-only tests. It must not log, wrap, or interpolate file contents.

- [ ] **Step 3: Keep complete history in memory and purge at store startup**

Keep:

```swift
@Published private(set) var taskCockpitHistory: [TaskCockpitHistoryRecord] = []
@Published private(set) var selectedTaskCockpitHistoryID: TaskCockpitHistoryRecord.ID?
@Published private(set) var taskCockpitHistoryCleanupMessage: String?
```

During `SkillStore.init`, call `purgeLegacyHistoryIfPresent()` once. Always initialize `taskCockpitHistory` to an empty array; there is no load path. On cleanup failure set `taskCockpitHistoryCleanupMessage = UIStrings.taskCockpitHistoryCleanupFailed`. The localized message must say only that prior local history could not be removed; it must not include a path, error description, task text, or file bytes.

`recordTaskCockpitHistory` continues to insert the complete current-session `TaskCockpitHistoryRecord` and cap it to `TaskCockpitHistoryStore.maxRecords`. Remove the call to `taskCockpitHistoryStore.save`. Completing a Preflight must never create a history file or its parent directory.

Add:

```swift
func clearTaskCockpitHistory() {
    taskCockpitHistory = []
    selectedTaskCockpitHistoryID = nil
    do {
        _ = try taskCockpitHistoryStore.purgeLegacyHistoryIfPresent()
        taskCockpitHistoryCleanupMessage = nil
    } catch {
        taskCockpitHistoryCleanupMessage = UIStrings.taskCockpitHistoryCleanupFailed
    }
}
```

This clear action always clears memory, then retries deletion in case a legacy file survived startup cleanup.

- [ ] **Step 4: Update the existing panel with explicit session-only copy and clear action**

Reuse the current workflow sheet chrome, list rows, banners, confirmation dialog, typography, spacing, and button styles. Change the history explanation to the localized equivalent of:

```text
Completed Preflights stay in memory for this app session. Task text and provider results are not saved to disk and disappear when the app quits.
```

Add a localized “Clear session history” action with the existing destructive-confirmation pattern. Disable it only when the in-memory list is empty and `taskCockpitHistoryCleanupMessage == nil`; when startup cleanup failed, the same action remains enabled so the user can retry deletion. Display `taskCockpitHistoryCleanupMessage` in the existing inline warning style. Do not display the legacy file path or raw error.

Extend `scripts/verify-native-ui-layout.mjs` to require session-only copy, the clear action, destructive confirmation, and a cleanup warning bound to the store’s redacted message.

- [ ] **Step 5: Prove no disk artifact and no cross-restart restore**

The successful Task Preflight integration test must use a temporary `TaskCockpitHistoryStore`, complete a provider-confirmed Preflight, and assert the first store has one full in-memory record while `fileExists(atPath:) == false`. Construct a second `SkillStore` with the same history store and assert its history is empty.

Before constructing the second store, create each old-format file in turn and assert initialization removes it. Read the temporary directory after cleanup and assert no filename beginning with `task-preflight-history` exists and no remaining file contains `SENSITIVE_SENTINEL_42`.

Run: `pnpm test:macos-native-models`

Expected: PASS for deletion of legacy/v1/v2/malformed files, current-session history, zero disk artifacts, zero cross-restart restore, clear behavior, and redacted failure messaging.

Run: `node scripts/verify-native-ui-layout.mjs`

Expected: PASS with session-only copy and the existing visual patterns intact.

- [ ] **Step 6: Commit session-only history and legacy cleanup**

```bash
git add apps/macos/Sources/SkillsCopilot/Stores/TaskCockpitHistoryStore.swift apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift apps/macos/Sources/SkillsCopilot/Views/TaskCockpitPanel.swift apps/macos/Sources/SkillsCopilot/Support/UIStrings.swift apps/macos/Sources/SkillsCopilot/Resources/en.lproj/Localizable.strings apps/macos/Sources/SkillsCopilot/Resources/zh-Hans.lproj/Localizable.strings apps/macos/Tests/SkillsCopilotTests/TaskCockpitHistoryStoreTests.swift apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift scripts/verify-native-ui-layout.mjs
pnpm check:privacy
git commit -m "fix: keep task preflight history session only"
```

---

### Task 2: Incrementally Drain Service Pipes with Hard Memory and Diagnostic Bounds

**Files:**
- Create: `apps/macos/Sources/SkillsCopilot/Support/ServiceDiagnosticSanitizer.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Services/ServiceProcessRunner.swift:8-280`
- Modify: `apps/macos/Sources/SkillsCopilot/Services/ServiceClientTransport.swift:22-40`
- Modify: `apps/macos/Sources/SkillsCopilot/Services/ServiceClient.swift:401-425`
- Modify: `apps/macos/Tests/SkillsCopilotTests/ServiceClientProcessTests.swift:4-330`

**Interfaces:**
- Consumes: existing `ServiceProcessRunning.run(executableURL:input:timeoutNanoseconds:)`, `StdioServiceProcessRunCoordinator`, cancellation escalation, `ConfigContentRedactor`, and `ServiceClient.ClientError`.
- Produces: `StdioServiceProcessRunner.configuredTimeoutNanoseconds(environment:)`, bounded `StdioPipeCollector` outputs, `ServiceDiagnosticSanitizer.displayMessage(_:)`, and `ServiceClient.ClientError.responseTooLarge(maxBytes:)`.

- [ ] **Step 1: Add failing high-volume, overflow, and redaction tests**

Extend `ServiceClientProcessTests.run()` with:

```swift
try await largeStderrBeforeStdoutDoesNotDeadlock()
try await concurrentLargeStdoutAndStderrAreDrained()
try await largeInputAndEarlyOutputDoNotDeadlock()
try await stdoutAboveSixteenMiBReturnsResponseTooLarge()
try await oversizedFailingStderrIsBoundedAndRedacted()
try await malformedStdoutNeverAppearsInDisplayError()
try await cancellationWhileDrainingReapsProcess()
try configuredTimeoutRejectsOverflowAndInvalidValues()
```

Add shell modes with these exact behaviors:

- `stderr-before-stdout` writes 2 MiB stderr, then one valid status JSON response.
- `dual-large` concurrently writes 2 MiB stderr and a valid stdout payload whose total size remains below 16 MiB.
- `large-input-early-output` emits status output before consuming a 2 MiB request body.
- `stdout-too-large` writes 17 MiB stdout and exits 0.
- `stderr-too-large-failure` writes more than 1 MiB stderr, includes the Swift-built strings `"TOKEN" + "=" + "SENSITIVE_SENTINEL_42"` and `"/" + "Users/fixture/private-config.json"` within the retained prefix, then exits 7.
- `malformed-sensitive` writes malformed stdout containing `"TOKEN" + "=" + "SENSITIVE_SENTINEL_42"` and exits 0.

Give successful high-volume cases a five-second timeout. Assert `stdout-too-large` throws exactly `responseTooLarge(maxBytes: 16 * 1_024 * 1_024)`. For the nonzero case, preserve exit status 7 but assert the displayed diagnostic is at most 512 characters and contains neither the sentinel nor the local path. For malformed output, assert the error is stable and contains none of the raw stdout.

For timeout parsing assert:

```swift
try expectEqual(StdioServiceProcessRunner.configuredTimeoutNanoseconds(environment: [:]), 30_000_000_000, "Missing timeout should use 30 seconds.")
try expectEqual(StdioServiceProcessRunner.configuredTimeoutNanoseconds(environment: ["SKILLS_COPILOT_SERVICE_TIMEOUT_MS": "1"]), 50_000_000, "Timeout should clamp to 50 ms.")
try expectEqual(StdioServiceProcessRunner.configuredTimeoutNanoseconds(environment: ["SKILLS_COPILOT_SERVICE_TIMEOUT_MS": "300000"]), 300_000_000_000, "Five minutes should be accepted.")
try expectEqual(StdioServiceProcessRunner.configuredTimeoutNanoseconds(environment: ["SKILLS_COPILOT_SERVICE_TIMEOUT_MS": "300001"]), 30_000_000_000, "Out-of-range timeout should use the default.")
try expectEqual(StdioServiceProcessRunner.configuredTimeoutNanoseconds(environment: ["SKILLS_COPILOT_SERVICE_TIMEOUT_MS": String(UInt64.max)]), 30_000_000_000, "Overflow should use the default.")
```

Run: `pnpm test:macos-native-models`

Expected: FAIL because reads are sequential and unbounded, raw malformed output/stderr enters errors, `responseTooLarge` does not exist, and timeout parsing is not injectable.

- [ ] **Step 2: Extract overflow-safe timeout resolution**

Implement:

```swift
static func configuredTimeoutNanoseconds(
    environment: [String: String] = ProcessInfo.processInfo.environment
) -> UInt64 {
    let defaultMilliseconds: UInt64 = 30_000
    let maximumMilliseconds: UInt64 = 300_000
    guard let raw = environment["SKILLS_COPILOT_SERVICE_TIMEOUT_MS"],
          let parsed = UInt64(raw),
          parsed <= maximumMilliseconds
    else {
        return defaultMilliseconds * 1_000_000
    }
    let milliseconds = max(parsed, 50)
    let product = milliseconds.multipliedReportingOverflow(by: 1_000_000)
    return product.overflow ? defaultMilliseconds * 1_000_000 : product.partialValue
}
```

Values above 300,000 ms and every parse/multiplication failure use 30 seconds; they are not clamped to five minutes. Explicit per-call overrides remain unchanged.

- [ ] **Step 3: Implement concurrent incremental drains that keep reading after truncation**

Add:

```swift
private struct BoundedPipeOutput {
    let data: Data
    let discardedByteCount: Int

    var wasTruncated: Bool { discardedByteCount > 0 }
}

private final class BoundedPipeDrain {
    static let chunkSize = 64 * 1_024

    private let maximumRetainedBytes: Int

    init(maximumRetainedBytes: Int) {
        self.maximumRetainedBytes = maximumRetainedBytes
    }

    func readToEOF(from handle: FileHandle) throws -> BoundedPipeOutput
}

private struct StdioOutputs {
    let stdout: BoundedPipeOutput
    let stderr: BoundedPipeOutput
}
```

`readToEOF` repeatedly calls `handle.read(upToCount: Self.chunkSize)` until nil/empty EOF. Append only the bytes that fit within `maximumRetainedBytes`, increment `discardedByteCount` for every excess byte, and continue reading/discarding until EOF so the child never blocks on pipe backpressure.

`StdioPipeCollector` starts two utility-queue drains before stdin is written:

- stdout limit: `16 * 1_024 * 1_024` bytes.
- stderr limit: `1 * 1_024 * 1_024` bytes.

Each block records data/error under a lock and always leaves a dispatch group. Cancellation closes readers so both drains unblock. In `StdioServiceProcessInvocation.run()` use this order: launch, start drains, write/close stdin, wait for exit, wait for both drains, check cancellation, check stdout bound, map status, cleanup.

If stdout `wasTruncated`, throw `ServiceClient.ClientError.responseTooLarge(maxBytes: 16 * 1_024 * 1_024)` without decoding or exposing bytes. A truncated stderr does not change a successful response; the drain has already prevented backpressure.

- [ ] **Step 4: Sanitize retained stderr and remove raw malformed stdout from errors**

Create a Foundation-only sanitizer:

```swift
import Foundation

enum ServiceDiagnosticSanitizer {
    static let maximumDisplayCharacters = 512

    static func displayMessage(_ raw: String) -> String
}
```

Add this exact `ServiceClient.ClientError` case:

```swift
case responseTooLarge(maxBytes: Int)
```

Its localized description is the stable localized equivalent of “The service response exceeded the 16 MiB limit.” It may interpolate only `maxBytes`; it must not include stdout, stderr, a decode error payload, or an excerpt.

`displayMessage` must:

1. Apply `ConfigContentRedactor.redactedForDisplay`.
2. Replace case-insensitive `API_KEY`, `TOKEN`, `SECRET`, and `PASSWORD` values when immediately followed by `"="` with `<redacted>`.
3. Replace `sk-` tokens of 20 or more token characters with `<redacted-token>`.
4. Replace path runs built from `"/" + "Users/" + "[^\\s]+"`, `"/" + "home/" + "[^\\s]+"`, `"/" + "private/var/" + "[^\\s]+"`, and `"/" + "var/folders/" + "[^\\s]+"` with `<redacted-path>`.
5. Collapse whitespace and return at most 512 characters.
6. Return a localized stable fallback when all content is removed.

On a nonzero exit, convert only the retained stderr prefix through this sanitizer before constructing `processFailed(status, diagnostic)`. `ClientError.localizedDescription` may include that sanitized diagnostic and status, never raw bytes.

In `ServiceClientTransport`, JSON decode failure maps to a stable localized `invalidOutput` message containing only response byte count and decode category. It must not interpolate stdout, `DecodingError` context containing payload text, stderr, or a response excerpt.

- [ ] **Step 5: Prove memory bounds, no backpressure, cancellation, and no diagnostic leak**

Run: `pnpm test:macos-native-models`

Expected: PASS for the three successful high-volume cases, 17 MiB response rejection, >1 MiB failing stderr redaction, malformed-output redaction, timeout bounds, cancellation, force-kill, empty/truncated response classification, and exit status preservation.

Run: `swift test --package-path apps/macos`

Expected: PASS with no continuation misuse, unbounded data retention, or leaked child process.

- [ ] **Step 6: Commit bounded process transport**

```bash
git add apps/macos/Sources/SkillsCopilot/Support/ServiceDiagnosticSanitizer.swift apps/macos/Sources/SkillsCopilot/Services/ServiceProcessRunner.swift apps/macos/Sources/SkillsCopilot/Services/ServiceClientTransport.swift apps/macos/Sources/SkillsCopilot/Services/ServiceClient.swift apps/macos/Tests/SkillsCopilotTests/ServiceClientProcessTests.swift
pnpm check:privacy
git commit -m "fix: bound and sanitize service process output"
```

---

### Task 3: Replace View-owned Debounce Tasks with Revision Autosave

**Files:**
- Create: `apps/macos/Sources/SkillsCopilot/Models/RevisionAutosaveCoordinator.swift`
- Create: `apps/macos/Tests/SkillsCopilotTests/RevisionAutosaveCoordinatorTests.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift:208-227,2435-2463,2502-2526`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/AgentConfigWorkspacePanel.swift:82-109,146-151,236-300`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/SettingsView.swift:53-72,126-155,547-585`
- Modify: `apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift:78-86,1020-1035`
- Modify: `apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift:110-139`
- Modify: `scripts/verify-native-ui-layout.mjs`

**Interfaces:**
- Consumes: the existing `SkillStore.saveClaudeSettings(content:)` during this task, `SkillStore.saveAIProviderSettings(draft:)`, `AIProviderSettingsDraft.validationMessage`, and the current 900 ms autosave delay from `UIOptimizationPresentation.configEditor.autosaveDelayNanoseconds`. Task 4 replaces the config save closure with its CAS signature without changing the coordinator API.
- Produces: generic `RevisionAutosaveCoordinator<Value>`, `RevisionAutosavePhase`, `RevisionAutosaveCompletion`, `SkillStore.submitConfigAutosave(content:validationError:)`, and `SkillStore.submitProviderAutosave(draft:)`.

- [ ] **Step 1: Write deterministic failing revision-order tests**

Create and register `RevisionAutosaveCoordinatorTests`:

```swift
import Foundation
@testable import SkillsCopilot

@MainActor
struct RevisionAutosaveCoordinatorTests {
    func run() async throws {
        try await rapidEditsSaveOnlyLatestRevision()
        try await editDuringSaveRunsAfterCurrentSave()
        try await failedSaveDoesNotDropPendingRevision()
        try await invalidRevisionWaitsUntilInputIsValid()
        try await cancellingDebounceDoesNotCancelActiveSave()
        try await completionIdentifiesExactlyCommittedRevision()
    }
}
```

Use a test `ControlledAutosaveClock` whose `sleep` stores continuations and a test `SaveRecorder<Value>` that can suspend and resume each save. In `editDuringSaveRunsAfterCurrentSave`, submit `"A"`, release its debounce, wait until save A starts, submit `"B"`, complete A, release B’s debounce, and assert recorded values are exactly `["A", "B"]`. In `rapidEditsSaveOnlyLatestRevision`, submit A/B/C before releasing the debounce and assert `["C"]`.

Run: `pnpm test:macos-native-models`

Expected: FAIL because `RevisionAutosaveCoordinator` and its testable clock/save injection do not exist.

- [ ] **Step 2: Implement the Foundation-only autosave state machine**

Create these exact types:

```swift
import Foundation

enum RevisionAutosavePhase: Equatable {
    case idle
    case debouncing(revision: UInt64)
    case saving(revision: UInt64)
    case pendingAfterSave(revision: UInt64)
    case failed(revision: UInt64, message: String)
}

struct RevisionAutosaveCompletion<Value> {
    let revision: UInt64
    let value: Value
    let succeeded: Bool
}

@MainActor
final class RevisionAutosaveCoordinator<Value: Equatable> {
    typealias Sleep = @Sendable (UInt64) async throws -> Void
    typealias Save = @MainActor (Value, UInt64) async -> Bool
    typealias Completion = @MainActor (RevisionAutosaveCompletion<Value>) -> Void
    typealias PhaseChanged = @MainActor (RevisionAutosavePhase) -> Void

    private let delayNanoseconds: UInt64
    private let sleep: Sleep
    private let save: Save
    private let completion: Completion
    private let phaseChanged: PhaseChanged
    private var nextRevision: UInt64 = 0
    private var pending: (revision: UInt64, value: Value)?
    private var debounceTask: Task<Void, Never>?
    private var workerTask: Task<Void, Never>?
    private(set) var phase: RevisionAutosavePhase = .idle

    init(
        delayNanoseconds: UInt64,
        sleep: @escaping Sleep = { try await Task.sleep(nanoseconds: $0) },
        save: @escaping Save,
        phaseChanged: @escaping PhaseChanged,
        completion: @escaping Completion
    )

    @discardableResult
    func submit(_ value: Value, validationError: String?) -> UInt64

    func cancelPendingDebounce()
    func flush() async
}
```

Implement `submit` with these invariants:

1. Increment `nextRevision` using `&+= 1`. When `validationError == nil`, replace `pending` with the new revision/value; otherwise clear any older pending value so a superseded valid draft cannot save after the current draft becomes invalid.
2. Cancel only `debounceTask`; never cancel `workerTask` after `save` has begun.
3. If a worker is active, publish `.pendingAfterSave(revision:)` and let that worker start the newest pending revision after its current save finishes.
4. If no worker is active, create one debounce task for the newest revision.
5. Immediately before saving, verify that the revision is still pending and remove it from `pending`.
6. Change phase only through a private `setPhase(_:)` that stores the value and calls `phaseChanged`; this keeps `SkillStore` published phase synchronized without importing SwiftUI/Combine into the coordinator.
7. After save completes, call `completion` with the exact revision/value. If a newer pending value exists, debounce and save it; otherwise publish `.idle` on success or `.failed` for the completed revision.
8. `cancelPendingDebounce()` removes an unsaved pending revision and cancels only the debounce task.
9. `flush()` cancels the debounce delay, runs the newest valid pending value, and waits until the worker completes.

- [ ] **Step 3: Move config and provider autosave ownership into `SkillStore`**

Add store-owned lazy coordinators and published phases:

```swift
@Published private(set) var configAutosavePhase: RevisionAutosavePhase = .idle
@Published private(set) var providerAutosavePhase: RevisionAutosavePhase = .idle

func submitConfigAutosave(content: String, validationError: String?)
func submitProviderAutosave(draft: AIProviderSettingsDraft)
func flushPendingAutosaves() async
```

The config save closure calls the existing `saveClaudeSettings(content:)` and reports failure without discarding a newer pending draft. Task 4 changes only that closure to fetch the loaded revision and call `saveClaudeSettings(content:expectedRevision:)`. The provider save closure calls `saveAIProviderSettings(draft:)`. Its completion clears a provider API-key draft only when the completion revision still equals the most recently submitted provider revision.

Remove `configAutosaveTask` and `providerAutosaveTask` from both views. Their `onChange` handlers call the store submission methods and never gate submission on `isSavingSettings` or `isSavingAIProvider`. Keep existing validation banners and visual components; derive pending/saving text from the published phase.

- [ ] **Step 4: Add store integration tests for edits arriving during a write**

Add a delayed config-save and delayed provider-save scenario to `FakeServiceScript.swift`. In `SkillStoreTests`, add:

```swift
try await runCase("configAutosaveKeepsEditArrivingDuringSave") {
    try await configAutosaveKeepsEditArrivingDuringSave()
}
try await runCase("providerAutosaveKeepsEditArrivingDuringSave") {
    try await providerAutosaveKeepsEditArrivingDuringSave()
}
```

Each case must start save A, submit B while A is blocked, unblock A, then assert two service calls in order and final stored/draft state B. The provider case must prove completion A does not clear B’s API-key draft.

Run: `pnpm test:macos-native-models`

Expected: PASS for all coordinator and store ordering cases.

Run: `node scripts/verify-native-ui-layout.mjs`

Expected: PASS with checks that both views call store autosave APIs and contain no view-owned autosave `Task` property.

- [ ] **Step 5: Commit revision autosave**

```bash
git add apps/macos/Sources/SkillsCopilot/Models/RevisionAutosaveCoordinator.swift apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift apps/macos/Sources/SkillsCopilot/Views/AgentConfigWorkspacePanel.swift apps/macos/Sources/SkillsCopilot/Views/SettingsView.swift apps/macos/Tests/SkillsCopilotTests/RevisionAutosaveCoordinatorTests.swift apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift apps/macos/Tests/SkillsCopilotTests/FakeServiceScript.swift apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift scripts/verify-native-ui-layout.mjs
pnpm check:privacy
git commit -m "fix: serialize macOS autosave revisions"
```

---

### Task 4: Add Config Compare-and-Swap and Rollback Preview Tokens

**Files:**
- Create: `apps/macos/Sources/SkillsCopilot/Models/ConfigMutationState.swift`
- Create: `apps/macos/Tests/SkillsCopilotTests/ConfigMutationModelTests.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Models/SkillRecord.swift:1026-1063`
- Modify: `apps/macos/Sources/SkillsCopilot/Services/ServiceClient.swift:350-352`
- Modify: `apps/macos/Sources/SkillsCopilot/Services/ServiceClientCatalogConfigRPC.swift:250-280`
- Modify: `apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift:2199-2236,2254-2345,2502-2526`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/AgentConfigWorkspacePanel.swift:374-501`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/DetailFindingsHistorySection.swift:430-484`
- Modify: `apps/macos/Tests/SkillsCopilotTests/ServiceClientRPCTests.swift:4-98`
- Modify: `apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift:821-841,1020-1052`
- Modify: `apps/macos/Tests/SkillsCopilotTests/FakeServiceScript.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift:110-139`
- Modify: `fixtures/service-protocol/config.readClaudeSettings.response.json`
- Modify: `fixtures/service-protocol/config.readAgentConfig.response.json`
- Modify: `fixtures/service-protocol/config.saveClaudeSettings.request.json`
- Modify: `fixtures/service-protocol/config.saveClaudeSettings.response.json`
- Modify: `fixtures/service-protocol/snapshot.previewRollback.response.json`
- Modify: `fixtures/service-protocol/snapshot.rollback.request.json`
- Modify: `docs/service-protocol.md:100-115`

**Interfaces:**
- Consumes: service-side `revision` values and `config_conflict` / `stale_preview_token` errors supplied by the matching Rust implementation task; existing `ServiceClient.ClientError.service`, snapshot IDs, and config document loading.
- Produces: `ConfigDocumentRecord.revision: String?`, `SnapshotRollbackPreviewRecord.previewToken: String?`, `SnapshotRollbackPreviewRecord.currentRevision: String?`, `ServiceClient.saveClaudeSettings(content:expectedRevision:)`, `ServiceClient.rollbackSnapshot(snapshotID:previewToken:expectedRevision:)`, `ConfigMutationState`, and `RollbackConfirmation`.

- [ ] **Step 1: Add failing DTO, request-body, and conflict-state tests**

Extend `ServiceClientRPCTests` with a recording runner that returns revision-bearing fixtures and assert the exact JSON request bodies contain:

```json
{"content":"{\"enabled\":true}\n","expected_revision":"sha256:before"}
```

and:

```json
{"snapshot_id":"snap-claude-new","preview_token":"rollback:token-1","expected_revision":"sha256:current"}
```

Create and register `ConfigMutationModelTests` with:

```swift
struct ConfigMutationModelTests {
    func run() throws {
        try missingRevisionIsReadOnly()
        try conflictKeepsDraftAndLatestDocument()
        try rollbackConfirmationRequiresTokenAndRevision()
        try changingSnapshotInvalidatesConfirmation()
    }
}
```

Add store cases `configSaveUsesLoadedRevision`, `configConflictPreservesDraftAndReloadsRevision`, `configSaveNeverFallsBackWithoutRevision`, `rollbackRequiresFreshPreviewToken`, `rollbackUsesImmutablePreviewInputs`, and `staleRollbackTokenDoesNotPublishSuccess`.

Run: `pnpm test:macos-native-models`

Expected: FAIL because the Swift DTOs and service methods do not expose revisions or rollback tokens.

- [ ] **Step 2: Extend typed Swift records while preserving read-only compatibility**

Add optional decode fields so older services remain readable but non-writable:

```swift
struct ConfigDocumentRecord: Codable, Hashable {
    let agent: String
    let scope: String
    let target: String
    let format: String
    let content: String
    let exists: Bool
    let revision: String?

    var supportsCompareAndSwap: Bool {
        guard let revision else { return false }
        return !revision.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }
}

struct SnapshotRollbackPreviewRecord: Codable, Identifiable, Hashable {
    let snapshot: ConfigSnapshotRecord
    let currentContent: String
    let currentReadError: String?
    let changed: Bool
    let redacted: Bool
    let rollbackSupported: Bool
    let previewToken: String?
    let currentRevision: String?
}
```

`rollbackSupported` is true only when the service flag is true and both new fields are nonempty. Do not synthesize tokens or revisions from snapshot IDs or contents.

- [ ] **Step 3: Add exact CAS request types and RPC methods**

Replace the current params with:

```swift
struct SaveClaudeSettingsParams: Encodable {
    let content: String
    let expectedRevision: String

    enum CodingKeys: String, CodingKey {
        case content
        case expectedRevision = "expected_revision"
    }
}

struct SnapshotRollbackParams: Encodable {
    let snapshotId: String
    let previewToken: String
    let expectedRevision: String

    enum CodingKeys: String, CodingKey {
        case snapshotId = "snapshot_id"
        case previewToken = "preview_token"
        case expectedRevision = "expected_revision"
    }
}
```

Expose only these write signatures:

```swift
func saveClaudeSettings(content: String, expectedRevision: String) async throws -> ConfigDocumentRecord
func rollbackSnapshot(snapshotID: String, previewToken: String, expectedRevision: String) async throws -> Int
```

Keep `previewSnapshotRollback(snapshotID:)` read-only. Remove the ID-only rollback method so call sites cannot bypass the token.

- [ ] **Step 4: Model conflict and immutable rollback confirmation state**

Create:

```swift
import Foundation

struct ConfigConflictState: Equatable {
    let attemptedRevision: String
    let latestRevision: String?
    let displayMessage: String
}

enum ConfigMutationState: Equatable {
    case idle
    case saving
    case conflict(ConfigConflictState)
    case failed(String)
}

struct RollbackConfirmation: Identifiable, Hashable {
    let preview: SnapshotRollbackPreviewRecord

    var id: String { preview.id }

    var canApply: Bool {
        preview.rollbackSupported
            && !(preview.previewToken ?? "").isEmpty
            && !(preview.currentRevision ?? "").isEmpty
    }
}
```

`SkillStore.saveClaudeSettings` requires the revision from the exact loaded document. On `config_conflict`, preserve the draft in the autosave coordinator, set `.conflict`, perform a fresh read, and do not call `refreshCollections()` or report success. Missing revision sets a localized read-only error and makes no write call.

`SkillStore.rollbackSnapshot` changes to:

```swift
func rollbackSnapshot(confirmation: RollbackConfirmation) async
```

It sends only values captured in `confirmation.preview`. On `stale_preview_token`, retain the selected snapshot, clear the confirmation, show a localized “Preview again” message, and make no success refresh. Any snapshot selection change clears the confirmation.

- [ ] **Step 5: Wire views to preview-first confirmation without changing visual language**

In both snapshot surfaces, the destructive button first awaits `previewRollback(snapshotID:)`. Present the existing confirmation dialog only after a `RollbackConfirmation` with `canApply == true` exists. The confirmation message uses `AgentConfigDisplay.pathSummary`; it does not expose a full target path or config content. Apply calls `store.rollbackSnapshot(confirmation:)`.

Keep the existing `NativePanelSurface`, buttons, sheet sizing, and diff layout. Task 8 adds default redaction; this task only enforces write-token correctness.

- [ ] **Step 6: Update protocol fixtures and drift documentation**

Add `revision` to read/save responses, `expected_revision` to the save request, `preview_token` and `current_revision` to preview response, and token/revision to rollback request. Document `config_conflict` and `stale_preview_token` and state that clients without CAS fields are read-only.

Run: `node scripts/verify-service-protocol-drift.mjs`

Expected: PASS with the revised fixtures and unchanged method count.

Run: `pnpm test:macos-native-models`

Expected: PASS for RPC encoding, conflict preservation, missing-capability read-only behavior, and token-bound rollback.

- [ ] **Step 7: Commit Swift CAS and rollback-token support**

```bash
git add apps/macos/Sources/SkillsCopilot/Models/ConfigMutationState.swift apps/macos/Sources/SkillsCopilot/Models/SkillRecord.swift apps/macos/Sources/SkillsCopilot/Services/ServiceClient.swift apps/macos/Sources/SkillsCopilot/Services/ServiceClientCatalogConfigRPC.swift apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift apps/macos/Sources/SkillsCopilot/Views/AgentConfigWorkspacePanel.swift apps/macos/Sources/SkillsCopilot/Views/DetailFindingsHistorySection.swift apps/macos/Tests/SkillsCopilotTests/ConfigMutationModelTests.swift apps/macos/Tests/SkillsCopilotTests/ServiceClientRPCTests.swift apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift apps/macos/Tests/SkillsCopilotTests/FakeServiceScript.swift apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift fixtures/service-protocol/config.readClaudeSettings.response.json fixtures/service-protocol/config.readAgentConfig.response.json fixtures/service-protocol/config.saveClaudeSettings.request.json fixtures/service-protocol/config.saveClaudeSettings.response.json fixtures/service-protocol/snapshot.previewRollback.response.json fixtures/service-protocol/snapshot.rollback.request.json docs/service-protocol.md
pnpm check:privacy
git commit -m "feat: require config revisions and rollback tokens"
```

---

### Task 5: Cache Session Summaries and Load Bounded Details by Stable ID

**Files:**
- Create: `apps/macos/Sources/SkillsCopilot/Models/LocalSessionCache.swift`
- Create: `apps/macos/Sources/SkillsCopilot/Models/AppSearchIndex.swift`
- Create: `apps/macos/Tests/SkillsCopilotTests/LocalSessionCacheTests.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Models/LocalSessionPreview.swift:138-249,399-625`
- Modify: `apps/macos/Sources/SkillsCopilot/Models/AppSearch.swift:149-188`
- Modify: `apps/macos/Sources/SkillsCopilot/Services/ServiceClient.swift:121-145`
- Modify: `apps/macos/Sources/SkillsCopilot/Services/ServiceClientSessionRPC.swift:3-36`
- Modify: `apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift:27-38,99-106,266-283,327-358,376-390,463-470,580-714,1960-2150,2849-2858,2967-3009`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/SidebarView.swift:251-260,1140-1290`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/ContentView.swift:934-1025`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/AgentSessionDetailPanel.swift:1-360`
- Modify: `apps/macos/Tests/SkillsCopilotTests/LocalSessionPreviewModelTests.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/ServiceClientRPCTests.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift:18-35,266-455`
- Modify: `apps/macos/Tests/SkillsCopilotTests/FakeServiceScript.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift:110-139`
- Modify: `fixtures/service-protocol/session.previewLocalSessions.request.json`
- Modify: `fixtures/service-protocol/session.previewLocalSessions.response.json`
- Modify: `docs/service-protocol.md:166-177`

**Interfaces:**
- Consumes: the matching Rust summary/detail extension of `session.previewLocalSessions`, stable session IDs, `LocalSessionPreviewResult`, `LocalSessionPreviewRow`, source/sort/filter models, and startup/manual-refresh boundaries.
- Produces: optional request fields `session_id: String?` and `include_content_items: Bool?`, `LocalSessionSnapshotKey`, summary-only `LocalSessionSnapshot`, `LocalSessionLoadState`, `LocalSessionDetailKey`, `LocalSessionDetailState`, `LocalSessionSelectionOrigin`, bounded `LocalSessionCache`, `AppSearchIndex.search(query:limitPerKind:)`, `SkillStore.refreshLocalSessionSnapshot(reason:)`, and `SkillStore.loadLocalSessionDetailIfNeeded(sessionID:)`.

- [ ] **Step 1: Add failing summary/detail, stale-state, and call-count tests**

Create and register:

```swift
import Foundation
@testable import SkillsCopilot

struct LocalSessionCacheTests {
    func run() throws {
        try eightHundredSummariesRetainNoContentItems()
        try criteriaDoNotChangeSourceKey()
        try projectionFiltersAndSortsSummariesLocally()
        try refreshingKeepsPreviousSummariesVisible()
        try failedRefreshWithDataBecomesStale()
        try failedDetailDoesNotChangeSummaryList()
        try detailCacheIsBoundedAndSourceScoped()
        try oldSummaryAndDetailGenerationsAreIgnored()
    }
}
```

Add RPC tests for exact params:

```json
{
  "session_id": null,
  "include_content_items": false
}
```

for summary pagination, and:

```json
{
  "session_id": "session-alpha",
  "include_content_items": true
}
```

for detail. Also decode an old response that omits `content_items` and both new request-related response hints; assert the row safely has an empty `contentItems` array.

Integration cases must assert:

- A generated set of 800 summary rows has empty `contentItems` in cache.
- Selecting `session-alpha` issues one detail call; selecting it again reuses detail cache.
- Twenty scope/search/sort changes issue zero summary and zero detail calls.
- A detail failure leaves summary IDs, counts, selection, and list stale/fresh state unchanged.
- A failed explicit summary refresh preserves the last successful summary list as stale.
- Skill filter/selection does not clear session summaries or details.
- Global search reads only summary fields and performs no service call.

Run: `pnpm test:macos-native-models`

Expected: FAIL because current summary requests return/cache full `contentItems`, criteria force reads, and no stable-ID detail cache exists.

- [ ] **Step 2: Extend the existing RPC with nullable compatibility fields**

Add to `LocalSessionPreviewParams`:

```swift
let sessionId: String?
let includeContentItems: Bool?

enum CodingKeys: String, CodingKey {
    case authorizedRoots = "authorized_roots"
    case autoDiscover = "auto_discover"
    case agent
    case scope
    case search
    case projectRoot = "project_root"
    case currentCWD = "current_cwd"
    case sessionId = "session_id"
    case includeContentItems = "include_content_items"
    case limit
    case offset
    case sort
    case direction
    case maxFiles = "max_files"
    case maxExcerptChars = "max_excerpt_chars"
}
```

Change the typed RPC signature to:

```swift
func previewLocalSessions(
    authorizedRoots: [String],
    agent: String? = nil,
    scope: LocalSessionScopeFilter = .project,
    search: String? = nil,
    project: ProjectContext? = nil,
    sessionID: String? = nil,
    includeContentItems: Bool? = nil,
    limit: Int = 20,
    offset: Int = 0,
    sort: LocalSessionSortOrder = .recent,
    direction: SkillSortDirection = .descending
) async throws -> LocalSessionPreviewResult
```

A nil `includeContentItems` preserves old callers because the service treats absence as true. Every new summary caller explicitly passes `sessionID: nil, includeContentItems: false`. Every detail caller passes the stable ID and `includeContentItems: true`. Do not add an array field or a new service method.

Keep decoding tolerant: missing `content_items` remains `[]`. Add a complete internal `LocalSessionPreviewRow` initializer accepting every stored property, then add:

```swift
var summaryOnly: LocalSessionPreviewRow {
    LocalSessionPreviewRow(
        id: id,
        title: title,
        sourceKind: sourceKind,
        scope: scope,
        agent: agent,
        projectRoot: projectRoot,
        redactedPath: redactedPath,
        modifiedAt: modifiedAt,
        startedAt: startedAt,
        endedAt: endedAt,
        excerpt: excerpt,
        excerptCharCount: excerptCharCount,
        userMessageCount: userMessageCount,
        totalMessageCount: totalMessageCount,
        toolCallCount: toolCallCount,
        skillCallCount: skillCallCount,
        contentHash: contentHash,
        evidenceRefs: evidenceRefs,
        contentItems: []
    )
}
```

Summary cache ingestion always maps rows through `summaryOnly` even if an older service ignored `include_content_items=false` and returned content.

- [ ] **Step 3: Define summary source state and bounded in-memory detail state**

Create:

```swift
import Foundation

struct LocalSessionSnapshotKey: Hashable {
    let agent: String
    let projectRoot: String
    let currentCWD: String
    let authorizedRoots: [String]

    init(agent: String, projectRoot: String?, currentCWD: String?, authorizedRoots: [String]) {
        self.agent = agent
        self.projectRoot = projectRoot ?? ""
        self.currentCWD = currentCWD ?? ""
        self.authorizedRoots = Array(Set(authorizedRoots)).sorted()
    }
}

struct LocalSessionSnapshot: Equatable {
    let key: LocalSessionSnapshotKey
    let generation: UInt64
    let result: LocalSessionPreviewResult
    let refreshedAt: Date
    let isComplete: Bool
}

enum LocalSessionLoadState: Equatable {
    case empty
    case loading(key: LocalSessionSnapshotKey)
    case fresh(LocalSessionSnapshot)
    case refreshing(LocalSessionSnapshot)
    case stale(LocalSessionSnapshot, displayError: String)
    case failed(key: LocalSessionSnapshotKey, displayError: String)
}

struct LocalSessionDetailKey: Hashable {
    let source: LocalSessionSnapshotKey
    let sessionID: String
}

enum LocalSessionDetailState: Equatable {
    case loading(generation: UInt64)
    case loaded(LocalSessionPreviewRow)
    case failed(displayError: String)
}

struct LocalSessionProjectionCriteria: Equatable {
    let scope: LocalSessionScopeFilter
    let search: String
    let sort: LocalSessionSortOrder
    let direction: SkillSortDirection
    let projectRoot: String?
}

@MainActor
final class LocalSessionCache {
    static let maximumDetailEntries = 12

    private(set) var summaryStates: [LocalSessionSnapshotKey: LocalSessionLoadState] = [:]
    private(set) var detailStates: [LocalSessionDetailKey: LocalSessionDetailState] = [:]

    func beginSummaryRefresh(for key: LocalSessionSnapshotKey) -> UInt64
    func publishSummary(_ snapshot: LocalSessionSnapshot)
    func failSummary(key: LocalSessionSnapshotKey, generation: UInt64, displayError: String)
    func beginDetailLoad(for key: LocalSessionDetailKey) -> UInt64?
    func publishDetail(_ row: LocalSessionPreviewRow, key: LocalSessionDetailKey, generation: UInt64)
    func failDetail(key: LocalSessionDetailKey, generation: UInt64, displayError: String)
    func successfulSnapshot(for key: LocalSessionSnapshotKey) -> LocalSessionSnapshot?
    func projectedRows(for key: LocalSessionSnapshotKey, criteria: LocalSessionProjectionCriteria) -> [LocalSessionPreviewRow]
    func activateSource(_ key: LocalSessionSnapshotKey)
}
```

`publishSummary` force-converts every row to `summaryOnly`. `beginDetailLoad` returns nil when the key is already loaded or loading. Detail generation guards ignore late responses. Maintain least-recently-used order and evict beyond 12 entries. `activateSource` may release every detail entry belonging to a different source key; no summary/detail content is persisted.

Projection filters and sorts the full summary set locally. Recent uses `endedAt ?? startedAt ?? 0`; title uses `localizedCaseInsensitiveCompare`. Project scope compares normalized project roots. Search uses title/excerpt/agent only; it never reads content items.

- [ ] **Step 4: Load summary pages only at startup/manual/source boundaries**

Add:

```swift
enum LocalSessionRefreshReason {
    case startup
    case manual
    case sourceChanged
}

func refreshLocalSessionSnapshot(reason: LocalSessionRefreshReason) async
```

Each summary page passes `sessionID: nil, includeContentItems: false`. Use source-only criteria: all scope, no search, recent descending, limit 800, then continue by offset only if `hasMore` is true and new stable IDs were returned. Accumulate summary-only rows locally and publish atomically after all pages succeed.

Startup prewarm, explicit refresh, and absent changed source may call this method. Scope/search/sort and skill criteria only recompute projection and selection. Refresh failure keeps previous summaries as stale; it never marks a failed key loaded.

- [ ] **Step 5: Load one selected detail once and keep detail failure local**

Add:

```swift
enum LocalSessionSelectionOrigin {
    case user
    case navigation
    case criteriaNormalization
}

func selectLocalSession(_ session: LocalSessionPreviewRow, origin: LocalSessionSelectionOrigin)
func loadLocalSessionDetailIfNeeded(sessionID: String) async
var hasActiveLocalSessionSnapshot: Bool { get }
var selectedLocalSessionSummary: LocalSessionPreviewRow? { get }
var selectedLocalSessionDetailState: LocalSessionDetailState? { get }
```

Look up the selected stable ID in the active summary snapshot before requesting. Call the same RPC with `sessionID: sessionID, includeContentItems: true, limit: 1` and source key fields. Accept only a returned row with the requested ID. Cache it under `LocalSessionDetailKey(source:activeKey, sessionID:)`.

The existing detail panel shows summary metadata immediately, its current progress component while detail loads, content items only from `.loaded`, and a detail-local retry/error surface for `.failed`. A detail failure must not change summary state, rows, aggregate metrics, selection, or global error banner.

Only `.user` and `.navigation` schedule the need-based detail load. `.criteriaNormalization` may preserve a visible selected ID, move selection to a summary row, or clear selection, but it never starts a detail request. Re-selecting a loaded row makes no RPC. Switching source calls `activateSource` and may release old details. Scope/search/sort changes therefore make zero summary/detail reads even when they normalize the selected summary row.

- [ ] **Step 6: Build global search from summaries only**

Add an explicit `AppSearchItem` initializer and create:

```swift
struct AppSearchIndex {
    let skills: [SkillRecord]
    let sessionSummaries: [LocalSessionPreviewRow]
    let configSnapshots: [ConfigSnapshotRecord]

    func search(query: String, limitPerKind: Int) -> AppSearchResult
}
```

Before indexing, assert/map every session row through `summaryOnly`. Match session title/excerpt/agent only. Config subtitles use `DisplayText.configPathSummary`. Query changes perform no RPC and cannot load a session detail.

- [ ] **Step 7: Update protocol fixtures and prove bounded behavior**

Set the canonical summary request fixture to `"session_id": null` and `"include_content_items": false`; omit `content_items` from summary response rows. Document that absent `include_content_items` means true for compatibility, false returns summary-only rows, and true plus one `session_id` returns that session’s detail.

Add delayed summary/detail responses, one detail failure, and old-field responses to `FakeServiceScript.swift`.

Run: `node scripts/verify-service-protocol-drift.mjs`

Expected: PASS with the same service method count and the two nullable fields documented.

Run: `pnpm test:macos-native-models`

Expected: PASS for 800 empty-content summaries, one detail call per stable ID, zero reads during criteria changes, stale summary preservation, detail-local failure, bounded/source-scoped details, old-response compatibility, and summary-only global search.

Run: `node scripts/verify-native-ui-layout.mjs`

Expected: PASS with distinct summary/detail progress and error wiring.

- [ ] **Step 8: Commit summary cache and bounded detail loading**

```bash
git add apps/macos/Sources/SkillsCopilot/Models/LocalSessionCache.swift apps/macos/Sources/SkillsCopilot/Models/AppSearchIndex.swift apps/macos/Sources/SkillsCopilot/Models/LocalSessionPreview.swift apps/macos/Sources/SkillsCopilot/Models/AppSearch.swift apps/macos/Sources/SkillsCopilot/Services/ServiceClient.swift apps/macos/Sources/SkillsCopilot/Services/ServiceClientSessionRPC.swift apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift apps/macos/Sources/SkillsCopilot/Views/SidebarView.swift apps/macos/Sources/SkillsCopilot/Views/ContentView.swift apps/macos/Sources/SkillsCopilot/Views/AgentSessionDetailPanel.swift apps/macos/Tests/SkillsCopilotTests/LocalSessionCacheTests.swift apps/macos/Tests/SkillsCopilotTests/LocalSessionPreviewModelTests.swift apps/macos/Tests/SkillsCopilotTests/ServiceClientRPCTests.swift apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift apps/macos/Tests/SkillsCopilotTests/FakeServiceScript.swift apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift fixtures/service-protocol/session.previewLocalSessions.request.json fixtures/service-protocol/session.previewLocalSessions.response.json docs/service-protocol.md
pnpm check:privacy
git commit -m "fix: cache session summaries and bound detail loading"
```

---

### Task 6: Guard Skill Manager Results and Apply with Immutable Request Generations

**Files:**
- Create: `apps/macos/Sources/SkillsCopilot/Models/SkillManagerRequestState.swift`
- Create: `apps/macos/Tests/SkillsCopilotTests/SkillManagerRequestGenerationTests.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift:148-193,1178-1439,1487-1545`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/SkillManagerPanel.swift:135-200,281-315,424-502`
- Modify: `apps/macos/Tests/SkillsCopilotTests/SkillManagerModelTests.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift:177-182,239-264`
- Modify: `apps/macos/Tests/SkillsCopilotTests/FakeServiceScript.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift:110-139`

**Interfaces:**
- Consumes: `SkillManagerScope`, `SkillManagerDistribution`, `SkillManagerMutationRecord`, `SkillManagerLocalCreateRecord`, `SkillManagerLocalDeleteRecord`, existing preview tokens, and typed `ServiceClientSkillManagerRPC` methods.
- Produces: `SkillManagerRequestKey`, `SkillManagerRequestGeneration`, `SkillManagerMutationInputs`, `SkillManagerMutationConfirmation`, and generation-guarded store request tasks.

- [ ] **Step 1: Add failing out-of-order result and immutable-apply tests**

Create and register:

```swift
import Foundation
@testable import SkillsCopilot

@MainActor
struct SkillManagerRequestGenerationTests {
    func run() async throws {
        try await newerSearchWinsWhenOlderResponseFinishesLast()
        try await staleSearchErrorDoesNotReplaceNewSuccess()
        try await installedListIsScopedToCapturedAgentsAndScope()
        try await inputChangeInvalidatesMutationPreview()
        try await localCreateInputChangeIgnoresOldPreview()
        try await localDeleteSelectionChangeIgnoresOldPreview()
        try await applyUsesExactPreviewInputsAndToken()
        try await oldCompletionDoesNotClearCurrentLoadingState()
    }
}
```

Use a `ServiceProcessRunning` test actor that records decoded method/params and suspends responses by request label. Make search A ignore cancellation, complete search B first, then A. Assert only B is visible. For mutation, preview source `owner/repo`, skills `["alpha"]`, agents `["codex"]`, scope project, distribution symlink, and network false; mutate all live UI inputs before apply and assert either confirmation is cleared or apply sends the original captured values, never a mixture.

Run: `pnpm test:macos-native-models`

Expected: FAIL because current responses assign without generation checks and apply rereads mutable store fields.

- [ ] **Step 2: Define canonical immutable request keys and confirmation values**

Create:

```swift
import Foundation

enum SkillManagerRequestKey: Hashable {
    case search(query: String, owner: String?, networkAllowed: Bool)
    case installed(agents: [String], scope: SkillManagerScope)
    case mutation(SkillManagerMutationInputs)
    case localCreate(name: String)
    case localDelete(instanceID: String)
}

struct SkillManagerMutationInputs: Hashable {
    enum Kind: String, Hashable {
        case install
        case remove
        case update
    }

    let kind: Kind
    let source: String?
    let skills: [String]
    let agents: [String]
    let scope: SkillManagerScope
    let distribution: SkillManagerDistribution?
    let networkAllowed: Bool

    init(
        kind: Kind,
        source: String?,
        skills: [String],
        agents: [String],
        scope: SkillManagerScope,
        distribution: SkillManagerDistribution?,
        networkAllowed: Bool
    ) {
        self.kind = kind
        self.source = source?.trimmingCharacters(in: .whitespacesAndNewlines)
        self.skills = Array(Set(skills.map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }.filter { !$0.isEmpty })).sorted()
        self.agents = Array(Set(agents)).sorted()
        self.scope = scope
        self.distribution = distribution
        self.networkAllowed = networkAllowed
    }
}

struct SkillManagerMutationConfirmation: Hashable {
    let inputs: SkillManagerMutationInputs
    let result: SkillManagerMutationRecord

    var previewToken: String { result.preview.previewToken }
}

struct SkillManagerRequestGeneration: Equatable {
    let value: UInt64
    let key: SkillManagerRequestKey
}
```

Canonicalize owner empty string to nil and always sort/deduplicate agents and skills before constructing a key.

- [ ] **Step 3: Give each request family a cancellable task and current generation**

Add separate generation counters and tasks for search, installed list, mutation preview, local create, and local delete. Every request captures `SkillManagerRequestGeneration`. Input setters cancel the relevant task, increment its generation, clear its visible result/confirmation, and leave unrelated workflows intact.

After every await, require both generation value and key to equal the current generation before assigning a result, error, or loading flag. A stale completion performs no state mutation even when its underlying process ignored cancellation.

Replace `skillManagerMutationPreview` with:

```swift
@Published private(set) var skillManagerMutationConfirmation: SkillManagerMutationConfirmation?
```

Apply install/remove/update switches on `confirmation.inputs.kind`, sends the exact stored inputs and `confirmation.previewToken`, and never reads current text fields. Any input change clears confirmation, which disables Apply.

- [ ] **Step 4: Wire the existing panel to immutable confirmation state**

Keep the current segmented workflows, target summary, `NativePanelSurface`, command preview, and explicit confirmation presentation. Display the captured confirmation inputs, not current editor fields, beside the command preview. Disable only the controls whose request family is active; search and installed-list work remain independent.

Add an accessibility value to Apply that reports whether the current inputs still match a preview. Task 8 supplies stable identifiers and labels.

- [ ] **Step 5: Prove stale responses and payload identity**

Add fake service scenarios that return search and preview requests in reverse order and one stale error. Assert stale data never reappears, loading state belongs only to the current request, and the recorded apply JSON equals the preview key plus preview token exactly.

Run: `pnpm test:macos-native-models`

Expected: PASS for all generation cases plus existing Skill Manager model/validation and preview-token tests.

Run: `node scripts/verify-native-ui-layout.mjs`

Expected: PASS with the panel reading `skillManagerMutationConfirmation` and applying the captured inputs.

- [ ] **Step 6: Commit Skill Manager generation guards**

```bash
git add apps/macos/Sources/SkillsCopilot/Models/SkillManagerRequestState.swift apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift apps/macos/Sources/SkillsCopilot/Views/SkillManagerPanel.swift apps/macos/Tests/SkillsCopilotTests/SkillManagerRequestGenerationTests.swift apps/macos/Tests/SkillsCopilotTests/SkillManagerModelTests.swift apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift apps/macos/Tests/SkillsCopilotTests/FakeServiceScript.swift apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift
pnpm check:privacy
git commit -m "fix: scope skill manager results to request generations"
```

---

### Task 7: Unify Navigation and Implement Deterministic Global-search Keyboard State

**Files:**
- Create: `apps/macos/Sources/SkillsCopilot/Models/AppNavigation.swift`
- Create: `apps/macos/Sources/SkillsCopilot/Models/GlobalSearchInteractionModel.swift`
- Create: `apps/macos/Tests/SkillsCopilotTests/AppNavigationModelTests.swift`
- Create: `apps/macos/Tests/SkillsCopilotTests/GlobalSearchInteractionModelTests.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift:228-246,2080-2165,3039-3120`
- Modify: `apps/macos/Sources/SkillsCopilot/App/SkillsCopilotApp.swift:67-95`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/SidebarView.swift:251-276`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/ContentView.swift:730-915,934-1025`
- Modify: `apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift:15-35,220-237,487-550`
- Modify: `apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift:110-139`
- Modify: `scripts/verify-native-ui-layout.mjs`

**Interfaces:**
- Consumes: Task 5 cache-backed session state and `AppSearchResult.items`, existing `SidebarContentMode`, `SidebarSelection`, `DetailSection`, and AppKit `NSTextFieldDelegate` command selectors.
- Produces: `AppNavigationDestination`, `SkillStore.navigate(to:) async`, `GlobalSearchCommand`, `GlobalSearchAction`, and `GlobalSearchInteractionModel`.

- [ ] **Step 1: Add failing navigation parity and keyboard-state tests**

Create and register `AppNavigationModelTests`:

```swift
import Foundation
@testable import SkillsCopilot

@MainActor
struct AppNavigationModelTests {
    func run() async throws {
        try await overviewNavigationAlwaysEntersSkillsMode()
        try await findingsNavigationAlwaysEntersSkillsMode()
        try await sessionsNavigationSelectsCachedFirstRow()
        try await sessionsNavigationFetchesOnlyWhenCacheIsAbsent()
        try await configNavigationUsesExistingDefaultSelection()
    }
}
```

Create and register `GlobalSearchInteractionModelTests`:

```swift
struct GlobalSearchInteractionModelTests {
    func run() throws {
        try downSelectsFirstAndStopsAtLast()
        try upStopsAtFirst()
        try returnSelectsHighlightedOrFirstResult()
        try escapeDismissesAndKeepsQuery()
        try resultRefreshPreservesStableHighlightedID()
        try removedHighlightedIDFallsBackToFirst()
        try focusInsideResultsKeepsOverlayVisible()
    }
}
```

Add store parity tests that begin in config or sessions mode, call the same navigation API used by command-menu and sidebar wiring, and assert identical mode/selection/section. Assert warm session navigation makes zero service calls and empty-cache navigation makes exactly one need-based source refresh.

Run: `pnpm test:macos-native-models`

Expected: FAIL because commands and sidebar use separate mutation sequences and no keyboard interaction model exists.

- [ ] **Step 2: Define one primary navigation contract**

Create:

```swift
enum AppNavigationDestination: Equatable {
    case sessions
    case skills(section: DetailSection)
    case config
}
```

Add this store method:

```swift
@MainActor
func navigate(to destination: AppNavigationDestination) async {
    switch destination {
    case .sessions:
        sidebarContentMode = .sessions
        if !hasActiveLocalSessionSnapshot {
            await refreshLocalSessionSnapshot(reason: .sourceChanged)
        }
        if let session = selectedLocalSession ?? filteredLocalSessionRows.first {
            selectLocalSession(session, origin: .navigation)
        } else {
            selectedSidebarSelection = nil
            selectedDetailSection = .overview
        }
    case .skills(let section):
        sidebarContentMode = .skills
        selectedDetailSection = section
        selectedSidebarSelection = selectedSkillID.map(SidebarSelection.skill)
    case .config:
        enterConfigMode()
    }
}
```

Both command-menu buttons and sidebar navigation buttons wrap only `Task { await store.navigate(to: destination) }`. Remove their duplicated mode/selection logic. Cmd+1 maps to `.sessions`, Cmd+2 to `.skills(section: .overview)`, and Cmd+3 to `.skills(section: .findings)`.

- [ ] **Step 3: Implement pure global-search command state**

Create:

```swift
import Foundation

enum GlobalSearchCommand: Equatable {
    case moveUp
    case moveDown
    case submit
    case cancel
    case focusChanged(isInsideSearchExperience: Bool)
}

enum GlobalSearchAction: Equatable {
    case none
    case select(resultID: String)
    case dismiss
}

struct GlobalSearchInteractionModel: Equatable {
    private(set) var query = ""
    private(set) var resultIDs: [String] = []
    private(set) var highlightedResultID: String?
    private(set) var isPresented = false

    mutating func updateQuery(_ query: String)
    mutating func updateResults(_ ids: [String])
    mutating func handle(_ command: GlobalSearchCommand) -> GlobalSearchAction
}
```

Use stable IDs, not indices, as stored selection. `moveDown` selects the first result from nil and stops at the last. `moveUp` selects the first from nil and stops at the first. `submit` selects the highlighted ID or first ID. `cancel` clears highlight and returns `.dismiss` while leaving query unchanged. Focus leaving both field and overlay dismisses; focus moving from field into a result does not.

- [ ] **Step 4: Wire AppKit commands and visible selection to the model**

Pass closures for move up, move down, submit, and cancel into `WindowChromeSearchTextField.Coordinator`. Map exactly:

```swift
switch commandSelector {
case #selector(NSResponder.moveUp(_:)):
    onMoveUp()
case #selector(NSResponder.moveDown(_:)):
    onMoveDown()
case #selector(NSResponder.insertNewline(_:)):
    onSubmit()
case #selector(NSResponder.cancelOperation(_:)):
    onCancel()
default:
    return false
}
return true
```

Restore `focusRingType = .default`. Bind each result button’s selected presentation to `highlightedResultID == result.id` using the existing accent and rounded-row vocabulary. Scroll the highlighted row into view with `ScrollViewReader`. Return invokes `store.selectAppSearchItem` for the chosen stable ID.

- [ ] **Step 5: Prove menu/sidebar parity and keyboard behavior**

Run: `pnpm test:macos-native-models`

Expected: PASS for navigation and interaction models plus store cache-call counts.

Run: `node scripts/verify-native-ui-layout.mjs`

Expected: PASS with both navigation entry points calling `navigate(to:)`, all four AppKit command selectors mapped, default focus ring enabled, and highlighted-row presentation present.

- [ ] **Step 6: Commit navigation and keyboard behavior**

```bash
git add apps/macos/Sources/SkillsCopilot/Models/AppNavigation.swift apps/macos/Sources/SkillsCopilot/Models/GlobalSearchInteractionModel.swift apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift apps/macos/Sources/SkillsCopilot/App/SkillsCopilotApp.swift apps/macos/Sources/SkillsCopilot/Views/SidebarView.swift apps/macos/Sources/SkillsCopilot/Views/ContentView.swift apps/macos/Tests/SkillsCopilotTests/AppNavigationModelTests.swift apps/macos/Tests/SkillsCopilotTests/GlobalSearchInteractionModelTests.swift apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift scripts/verify-native-ui-layout.mjs
pnpm check:privacy
git commit -m "fix: unify macOS navigation and search keyboard flow"
```

---

### Task 8: Enforce Privacy-safe Presentation, AX Semantics, and a 960-point Compact Layout

**Files:**
- Create: `apps/macos/Sources/SkillsCopilot/Models/SensitiveTextPresentation.swift`
- Create: `apps/macos/Tests/SkillsCopilotTests/PrivacyPresentationModelTests.swift`
- Create: `apps/macos/Tests/SkillsCopilotAXTestDriver/main.swift`
- Create: `apps/macos/Tests/SkillsCopilotAXTestDriver/AXHarness.swift`
- Create: `apps/macos/Tests/SkillsCopilotAXTestDriver/GlobalSearchAXTests.swift`
- Create: `apps/macos/Tests/SkillsCopilotAXTestDriver/NavigationAndLayoutAXTests.swift`
- Create: `apps/macos/Tests/SkillsCopilotAXTestDriver/PrivacyAXTests.swift`
- Create: `scripts/test-macos-ax.sh`
- Modify: `apps/macos/Package.swift:1-25`
- Modify: `package.json`
- Modify: `apps/macos/Sources/SkillsCopilot/Models/MainWindowModel.swift:1-33`
- Modify: `apps/macos/Sources/SkillsCopilot/App/SkillsCopilotApp.swift:37-44`
- Modify: `apps/macos/Sources/SkillsCopilot/App/MainWindowCoordinator.swift:4-47`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/ContentView.swift:1-1025`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/PrivacyPathView.swift:18-51,98-130`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/DetailFindingsHistorySection.swift:487-550`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/AgentConfigWorkspacePanel.swift:374-501`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/AgentSessionDetailPanel.swift:300-345`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/SettingsView.swift:608-647`
- Modify: `apps/macos/Sources/SkillsCopilot/Support/UIStrings.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Resources/en.lproj/Localizable.strings`
- Modify: `apps/macos/Sources/SkillsCopilot/Resources/zh-Hans.lproj/Localizable.strings`
- Modify: `apps/macos/Tests/SkillsCopilotTests/MainWindowModelTests.swift:3-50`
- Modify: `apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift:110-139`
- Modify: `scripts/verify-native-ui-layout.mjs`
- Modify: `scripts/check-macos.mjs`
- Modify: `docs/runbooks/macos-app-runbook.md`
- Modify: `docs/ui-delivery-standards.md`

**Interfaces:**
- Consumes: existing `ConfigContentRedactor`, `DisplayText.privacyPath`, Task 2 `ServiceDiagnosticSanitizer`, screenshot privacy `@AppStorage`, Task 7 `GlobalSearchInteractionModel`, current `NavigationSplitView`, `NativePanelSurface`, and fixture-data smoke launch environment.
- Produces: `SensitiveTextKind`, `SensitiveTextPresentation`, `SensitiveRevealState`, `MainWindowLayoutMode`, stable `AppAccessibilityID` values, and the `SkillsCopilotAXTestDriver` executable.

- [ ] **Step 1: Add failing privacy, layout, and AX-ID model tests**

Create and register:

```swift
import Foundation
@testable import SkillsCopilot

struct PrivacyPresentationModelTests {
    func run() throws {
        try configAndSnapshotContentAreRedactedByDefault()
        try explicitRevealShowsOnlyCurrentItem()
        try changingIdentityResetsReveal()
        try privacyPathNeverLeaksIntoAccessibilityLabel()
        try serviceDiagnosticsAreRedactedAndBounded()
    }
}
```

Extend `MainWindowModelTests` to assert:

```swift
try expectEqual(MainWindowModel.minimumWidth, 960, "Main window should support compact workspaces.")
try expectEqual(MainWindowModel.layoutMode(forWidth: 960), .compact, "960 points should use compact layout.")
try expectEqual(MainWindowModel.layoutMode(forWidth: 1199), .compact, "1199 points should use compact layout.")
try expectEqual(MainWindowModel.layoutMode(forWidth: 1200), .regular, "1200 points should use regular layout.")
```

Assert every new accessibility identifier is nonempty and unique. Use `SENSITIVE_SENTINEL_42` in config, snapshot, path, stdout, and stderr inputs; default display and AX label must not contain it. Diagnostic output must be at most 512 characters.

Run: `pnpm test:macos-native-models`

Expected: FAIL because snapshot panes render raw content, compact mode does not exist, the required AX semantics are absent, and minimum width is 1349. Task 2 diagnostic assertions remain green.

- [ ] **Step 2: Define one privacy presentation model and resettable reveal state**

Create:

```swift
import Foundation

enum SensitiveTextKind: Equatable {
    case path
    case config
    case snapshot
    case diagnostic
}

struct SensitiveTextPresentation: Equatable {
    let identity: String
    let kind: SensitiveTextKind
    let redactedText: String
    let revealedText: String
    let accessibilityLabel: String
}

struct SensitiveRevealState: Equatable {
    private(set) var revealedIdentity: String?

    mutating func toggle(identity: String) {
        revealedIdentity = revealedIdentity == identity ? nil : identity
    }

    mutating func reset() {
        revealedIdentity = nil
    }

    mutating func resetIfIdentityChanged(to identity: String) {
        if revealedIdentity != identity {
            revealedIdentity = nil
        }
    }

    func isRevealed(identity: String) -> Bool {
        revealedIdentity == identity
    }
}
```

Construct config/snapshot redaction with `ConfigContentRedactor.redactedForDisplay`. Construct path redaction with `DisplayText.privacyPath(rawValue, privacyModeEnabled: true, revealFull: false)`. Accessibility labels always use `redactedText`, even while visual content is explicitly revealed. Reuse Task 2 `ServiceDiagnosticSanitizer` for diagnostic presentation and keep its 512-character contract unchanged.

- [ ] **Step 3: Route snapshot, path, search, and error views through the privacy model**

`SnapshotPreviewSheet`, `SnapshotTextPane`, and `AgentConfigSnapshotDetailPanel` start redacted. Add one labeled reveal control per pane using existing button styles and localized “Reveal sensitive content” / “Hide sensitive content” labels. Closing a sheet or changing snapshot ID resets `SensitiveRevealState`.

`PrivacyPathText` icon-only buttons receive explicit `.accessibilityLabel` and `.accessibilityValue`; `AgentSessionDetailPanel` copy/reveal buttons do the same. Settings sidebar buttons add `.accessibilityAddTraits(.isSelected)` only when selected.

Keep global-search config subtitles privacy-safe through Task 5’s `DisplayText.configPathSummary`. Do not expose a full path in a tooltip, help string, accessibility label, fallback reason, or confirmation message. `PrivacyPresentationModelTests` must also call Task 2 `ServiceDiagnosticSanitizer` with a token/path sentinel and assert its output remains redacted and at most 512 characters.

- [ ] **Step 4: Add compact layout modes without replacing existing components**

Add to `MainWindowModel.swift`:

```swift
enum MainWindowLayoutMode: Equatable {
    case compact
    case regular
}

enum MainWindowModel {
    static let minimumWidth = 960
    static let minimumHeight = 600
    static let regularLayoutMinimumWidth = 1200

    static func layoutMode(forWidth width: Double) -> MainWindowLayoutMode {
        width < Double(regularLayoutMinimumWidth) ? .compact : .regular
    }
}
```

Use `GeometryReader` only to select compact versus regular behavior. Keep the existing `NavigationSplitView`; in compact mode allow its sidebar/content columns to collapse and expose the standard sidebar toggle. Do not shrink text below existing minimum scale factors, add horizontal app-level scrolling, or replace the current toolbar/search surfaces. The AppKit coordinator and SwiftUI frame use the same 960×600 minimum.

- [ ] **Step 5: Add stable AX identifiers and roles**

Extend `AppAccessibilityID` with exact stable strings:

```swift
static let globalSearchField = "skills-copilot.global-search.field"
static let globalSearchResults = "skills-copilot.global-search.results"
static let globalSearchResultPrefix = "skills-copilot.global-search.result."
static let navigationSessions = "skills-copilot.navigation.sessions"
static let navigationSkills = "skills-copilot.navigation.skills"
static let navigationConfig = "skills-copilot.navigation.config"
static let settingsTabPrefix = "skills-copilot.settings.tab."
static let privacyRevealPrefix = "skills-copilot.privacy.reveal."
static let snapshotCurrentPane = "skills-copilot.snapshot.current"
static let snapshotStoredPane = "skills-copilot.snapshot.stored"
static let snapshotRollback = "skills-copilot.snapshot.rollback"
```

Result IDs append the stable result ID after replacing non-alphanumeric characters with `-`. Settings tab IDs append `SettingsTab.rawValue`. Privacy reveal IDs append a fixed semantic key such as `source-path` or `session-redacted-path`; snapshot panes append `snapshot-current-` or `snapshot-stored-` followed by the sanitized stable snapshot ID, as in `snapshot-current-snap-claude-new`. They never derive from a path or content and never use Swift’s process-randomized `hashValue`. The global search remains an AX text field, its overlay is an AX group, each result/navigation/settings/reveal control is an AX button, the current result/settings/navigation button exposes the selected trait, and every button exposes a localized label plus a redacted value when it has state.

- [ ] **Step 6: Create a fixture-only AX executable and test script**

Add the first declaration to `products` and the second declaration to `targets` in `apps/macos/Package.swift`:

```swift
.executable(
    name: "SkillsCopilotAXTestDriver",
    targets: ["SkillsCopilotAXTestDriver"]
)

.executableTarget(
    name: "SkillsCopilotAXTestDriver",
    path: "Tests/SkillsCopilotAXTestDriver"
)
```

`AXHarness.swift` exposes:

```swift
import ApplicationServices
import Foundation

struct AXHarness {
    let application: AXUIElement

    static func launchFixtureApp(bundleURL: URL, environment: [String: String]) throws -> AXHarness
    func element(identifier: String, timeout: TimeInterval = 5) throws -> AXUIElement
    func elements(identifierPrefix: String) throws -> [AXUIElement]
    func press(_ keyCode: CGKeyCode, modifiers: CGEventFlags = []) throws
    func setMainWindowSize(width: Double, height: Double) throws
    func assertElementIsInsideMainWindow(_ element: AXUIElement) throws
    func terminate() throws
}
```

`scripts/test-macos-ax.sh` creates temporary fixture HOME/app-data/project directories, launches only the already built fixture-configured app bundle, runs the driver, and cleans up. It checks that the macOS session is interactive and AX is trusted; otherwise it exits with the same canonical blocker language used by the macOS runbook. Add `"test:macos-ax": "./scripts/test-macos-ax.sh"` to `package.json`.

The AX suites must verify:

- Down, Down, Return selects the second global-search result and opens its matching detail.
- Escape closes the result group while the search field remains keyboard-focusable.
- Cmd+1/2/3 and clicking the matching primary-navigation control yield the same selected AX element.
- The current settings tab exposes the selected trait; every icon-only privacy/copy action has a nonempty label.
- At 960×700, primary navigation, global search, list, and current detail controls have frames inside the main window and are hittable.
- Default snapshot/config/search/error AX values do not contain `SENSITIVE_SENTINEL_42`; explicit reveal affects only the current visual pane; selecting a different snapshot restores redaction.

- [ ] **Step 7: Update runbook, static verifier, and macOS gate wiring**

Document `pnpm test:macos-ax` as fixture-only real UI validation and its locked-session blocker. Add it to `scripts/check-macos.mjs` after the fixture smoke bundle is available and before privacy verification. Extend `verify-native-ui-layout.mjs` to check stable IDs, selected traits, labeled icon actions, privacy model use, and the 960-point minimum; keep the real AX driver as the behavioral authority.

Run: `pnpm test:macos-native-models`

Expected: PASS for privacy presentation, diagnostic sanitization, stable IDs, and compact breakpoints.

Run: `node scripts/verify-native-ui-layout.mjs`

Expected: PASS for view wiring and accessibility semantics.

Run: `pnpm test:macos-ax`

Expected: PASS in an interactive fixture-data session; otherwise emit the canonical locked/AX-unavailable blocker and do not claim real UI validation.

- [ ] **Step 8: Commit privacy, AX, and compact layout**

```bash
git add apps/macos/Sources/SkillsCopilot/Models/SensitiveTextPresentation.swift apps/macos/Sources/SkillsCopilot/Models/MainWindowModel.swift apps/macos/Sources/SkillsCopilot/App/SkillsCopilotApp.swift apps/macos/Sources/SkillsCopilot/App/MainWindowCoordinator.swift apps/macos/Sources/SkillsCopilot/Views/ContentView.swift apps/macos/Sources/SkillsCopilot/Views/PrivacyPathView.swift apps/macos/Sources/SkillsCopilot/Views/DetailFindingsHistorySection.swift apps/macos/Sources/SkillsCopilot/Views/AgentConfigWorkspacePanel.swift apps/macos/Sources/SkillsCopilot/Views/AgentSessionDetailPanel.swift apps/macos/Sources/SkillsCopilot/Views/SettingsView.swift apps/macos/Sources/SkillsCopilot/Support/UIStrings.swift apps/macos/Sources/SkillsCopilot/Resources/en.lproj/Localizable.strings apps/macos/Sources/SkillsCopilot/Resources/zh-Hans.lproj/Localizable.strings apps/macos/Tests/SkillsCopilotTests/PrivacyPresentationModelTests.swift apps/macos/Tests/SkillsCopilotTests/MainWindowModelTests.swift apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift apps/macos/Tests/SkillsCopilotAXTestDriver apps/macos/Package.swift scripts/test-macos-ax.sh scripts/verify-native-ui-layout.mjs scripts/check-macos.mjs package.json docs/runbooks/macos-app-runbook.md docs/ui-delivery-standards.md
pnpm check:privacy
git commit -m "fix: harden macOS privacy accessibility and compact layout"
```

---

### Task 9: Run the Integrated Reliability and UX Release Gate

**Files:**
- Verify: `apps/macos/Sources/SkillsCopilot/`
- Verify: `apps/macos/Tests/SkillsCopilotTests/`
- Verify: `apps/macos/Tests/SkillsCopilotAXTestDriver/`
- Verify: `fixtures/service-protocol/`
- Verify: `docs/service-protocol.md`
- Verify: `docs/runbooks/macos-app-runbook.md`
- Verify: `docs/ui-delivery-standards.md`
- Verify: `scripts/check-macos.mjs`
- Verify: `scripts/test-macos-ax.sh`

**Interfaces:**
- Consumes: every interface produced by Tasks 1-8 and the matching Rust service implementation for config revisions, conflict detection, rollback preview tokens, token validation, and server-side session ordering.
- Produces: one evidence bundle consisting of command results, a clean working tree, fixture-only full-window screenshots, and a canonical blocker record when the real interactive macOS session cannot be used.

- [ ] **Step 1: Verify protocol, fixture, layout, and module drift**

Run:

```bash
pnpm verify:service-protocol-drift
pnpm verify:gate-parity
pnpm verify:module-size
pnpm verify:macos-ui-layout
```

Expected: all commands exit 0. Protocol fixtures include revision/token fields, native UI checks recognize the new cache/navigation/privacy wiring, and no Swift source exceeds the repository module-size limit.

- [ ] **Step 2: Run Rust workspace checks for the matching service behavior**

Run:

```bash
cargo test --workspace
cargo clippy --workspace --all-targets --all-features
```

Expected: exit 0. Config CAS rejects a changed revision, rollback rejects a stale or mismatched token, session sort/direction is applied before pagination, and existing adapter safety tests remain green.

- [ ] **Step 3: Run all native Swift model and package tests**

Run:

```bash
pnpm test:macos-native-models
swift test --package-path apps/macos
```

Expected: exit 0. The output names and passes all newly registered suites:

- `TaskCockpitHistoryStoreTests`
- `RevisionAutosaveCoordinatorTests`
- `ConfigMutationModelTests`
- `LocalSessionCacheTests`
- `SkillManagerRequestGenerationTests`
- `AppNavigationModelTests`
- `GlobalSearchInteractionModelTests`
- `PrivacyPresentationModelTests`
- Existing `ServiceClientProcessTests`, `ServiceClientRPCTests`, and grouped `SkillStoreTests`

- [ ] **Step 4: Run fixture-only app, AX, and full macOS gate validation**

Run:

```bash
pnpm smoke:macos-app -- --fixture-data --capture-window
pnpm test:macos-ax
pnpm check:macos
```

Expected in an interactive session: exit 0, only fixture HOME/app-data/config roots are accessed, the capture contains only the full app window, keyboard navigation reaches a non-first search result, Cmd+1/2/3 stay coherent, selected AX traits are present, sensitive fixture text is redacted by default, and 960×700 layout elements remain inside the main window.

Expected when the session is locked or AX/window capture is unavailable: the command emits the canonical blocker from `docs/runbooks/macos-app-runbook.md`. Record that blocker verbatim and do not report real local UI validation as passed.

- [ ] **Step 5: Run privacy and inspect the final implementation diff**

Run:

```bash
pnpm check:privacy
git status --short
git diff --check
git diff --stat HEAD~8..HEAD
```

Expected: privacy exits 0, `git diff --check` reports no whitespace errors, the working tree is clean, and the eight implementation commits contain only the files assigned to Tasks 1-8.

- [ ] **Step 6: Confirm every acceptance contract before handoff**

Use this exact checklist:

- History: startup removes legacy/v1/v2/malformed history without backup, complete results remain in memory only, no run creates the history file, a new store restores nothing, and clear empties memory while retrying cleanup.
- Process: 2 MiB stderr-before-stdout and simultaneous 2 MiB stdout/stderr complete; stdout retention is capped at 16 MiB, stderr at 1 MiB while drains continue to EOF; oversized/malformed output exposes no sentinel; cancel/timeout reaps children; extreme timeout input cannot trap.
- Autosave: edits arriving before debounce coalesce; edits arriving during save run afterward; a newer API-key draft is never cleared by an older completion.
- Config: every save sends the loaded revision; conflicts preserve draft; rollback requires the matching preview token/revision; missing capability is read-only.
- Sessions: startup/manual/source refresh caches summary-only rows with empty content items; criteria and global search make zero reads; selecting one stable ID loads one bounded in-memory detail; detail failure leaves summaries intact; old-field responses decode safely.
- Skill Manager: stale success/error/loading completions are ignored; Apply uses the exact preview inputs and token.
- Navigation/search: sidebar and commands call one navigation method; Up/Down/Return/Escape work with stable IDs and visible focus.
- Accessibility/layout: identifiers are stable and unique, selected traits and icon labels are present, and 960-point compact mode preserves reachable controls.
- Privacy: config/snapshot/path/diagnostic content starts redacted, reveal is item-local and resets, and AX labels remain redacted.
- Evidence: fixture data only, full app window only, privacy gate passed, and any real-session blocker is recorded without substitution.

No additional release commit is created in this gate task. If a gate fails, return to the owning task, add a failing regression case to that task’s exact test file, implement the smallest correction, rerun the owning focused gate, and then rerun Task 9 from Step 1.

---

## Implementation Completion Criteria

The package is complete only when Tasks 1-8 have their focused RED/GREEN evidence and commits, Task 9 passes in full or carries the canonical real-session blocker, the service protocol fixtures and documentation match the Rust service, and `pnpm check:privacy` passes on a clean tree. Static regex verification and screenshots supplement but do not replace native model, process, protocol, keyboard, or AX behavior tests.
