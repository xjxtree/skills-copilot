import Foundation
import Testing
@testable import SkillsCopilot

@Suite("AuthorizedFileWatcherTests", .serialized)
@MainActor
struct AuthorizedFileWatcherTests {
    @Test("Sanitizer accepts only existing non-symlink directories")
    func sanitizerRejectsUnsafeRoots() throws {
        let tree = try TemporaryWatchTree(label: "sanitize")
        defer { tree.cleanup() }
        let accepted = try tree.createDirectory("home/.claude/skills")
        let target = try tree.createDirectory("target")
        let link = tree.root.appendingPathComponent("home/.agents/skills")
        try FileManager.default.createDirectory(
            at: link.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try FileManager.default.createSymbolicLink(
            at: link,
            withDestinationURL: target
        )

        let sanitized = AuthorizedWatchRootSanitizer.sanitizedPaths(
            from: [
                accepted.path,
                accepted.path,
                link.path,
                tree.root.appendingPathComponent("missing").path,
                accepted.appendingPathComponent("../skills").path,
                "/",
                "relative/path",
            ]
        )

        try expectEqual(sanitized, [accepted.path], "Only the bounded regular directory should remain")
    }

    @Test("Watcher events set pending state without exposing paths")
    func watcherEventsSetPendingState() async throws {
        let tree = try TemporaryWatchTree(label: "pending")
        defer { tree.cleanup() }
        let root = try tree.createDirectory("home/.claude/skills")
        let watcher = RecordingFileSystemWatcher()
        let store = SkillStore(
            service: ServiceClient(),
            fileSystemWatcher: watcher
        )

        store.updateAuthorizedFileWatcher(
            with: AuthorizedFileWatchPlan(
                roots: [root.path],
                totalCount: 1,
                truncated: false
            )
        )
        try expectEqual(watcher.startedPaths, [root.path], "Store should pass only sanitized roots")
        try expectFalse(store.hasPendingFileSystemChanges, "Starting a watcher must not dirty the catalog")

        watcher.emit(FileSystemChangeSummary(eventCount: 3, requiresDeepScan: true))
        await Task.yield()

        try expectFalse(
            !store.hasPendingFileSystemChanges,
            "A watcher event should mark cached data stale"
        )
        try expectEqual(
            store.watcherStatusMessage,
            UIStrings.refreshWatcherPendingDeepScan,
            "Dropped or coalesced events should request full reconciliation"
        )
        try expectFalse(
            store.watcherStatusMessage.contains(root.path),
            "Watcher status must not reveal a raw local path"
        )

        store.reconcileAuthorizedFileWatcherAfterDeepScan(
            scanStartedAtGeneration: store.authorizedFileSystemChangeGeneration
        )
        try expectFalse(store.hasPendingFileSystemChanges, "Full refresh reconciliation should clear pending state")
        try expectEqual(watcher.startCount, 1, "Full refresh should preserve the active stream for the same roots")
    }

    @Test("Watcher callbacks queued during reconciliation are not discarded")
    func queuedWatcherCallbacksRemainPendingAfterReconciliation() async throws {
        let tree = try TemporaryWatchTree(label: "queued-scan-race")
        defer { tree.cleanup() }
        let root = try tree.createDirectory("home/.claude/skills")
        let watcher = RecordingFileSystemWatcher()
        let store = SkillStore(
            service: ServiceClient(),
            fileSystemWatcher: watcher
        )
        store.updateAuthorizedFileWatcher(
            with: AuthorizedFileWatchPlan(
                roots: [root.path],
                totalCount: 1,
                truncated: false
            )
        )
        let scanGeneration = store.authorizedFileSystemChangeGeneration

        watcher.emit(FileSystemChangeSummary(eventCount: 1, requiresDeepScan: true))
        store.reconcileAuthorizedFileWatcherAfterDeepScan(
            scanStartedAtGeneration: scanGeneration
        )
        await Task.yield()

        try expectEqual(
            watcher.startCount,
            1,
            "Keeping the stream alive must let an already-queued callback retain its session."
        )
        try expectEqual(
            store.hasPendingFileSystemChanges,
            true,
            "An event delivered after scan reconciliation must remain pending for the next Refresh."
        )
    }

    @Test("Watcher events that arrive during a deep scan remain pending")
    func watcherEventsDuringDeepScanRemainPending() async throws {
        let tree = try TemporaryWatchTree(label: "scan-race")
        defer { tree.cleanup() }
        let root = try tree.createDirectory("home/.claude/skills")
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "scan-slow")
        let watcher = RecordingFileSystemWatcher()
        let store = SkillStore(
            service: fake.serviceClient(),
            fileSystemWatcher: watcher
        )
        store.updateAuthorizedFileWatcher(
            with: AuthorizedFileWatchPlan(
                roots: [root.path],
                totalCount: 1,
                truncated: false
            )
        )

        let scan = Task { await store.scanAll() }
        try await waitForCondition("Full refresh should reach the service before the watcher event.") {
            fake.calls().contains("\"method\":\"catalog.scanAll\"")
        }
        watcher.emit(FileSystemChangeSummary(eventCount: 1, requiresDeepScan: true))
        await Task.yield()
        await scan.value

        try expectEqual(
            store.hasPendingFileSystemChanges,
            true,
            "An event newer than the scan start must survive reconciliation."
        )
        try expectEqual(
            store.watcherStatusMessage,
            UIStrings.refreshWatcherPendingDeepScan,
            "The next Refresh should still require a complete scan."
        )
    }

    @Test("Project transitions stop stale roots before validation or scan recovery")
    func projectTransitionsStopStaleWatcherRoots() async throws {
        let tree = try TemporaryWatchTree(label: "project-transition")
        defer { tree.cleanup() }
        let root = try tree.createDirectory("old-project/.claude/skills")
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "project-validation-error")
        let watcher = RecordingFileSystemWatcher()
        let store = SkillStore(
            service: fake.serviceClient(),
            fileSystemWatcher: watcher
        )
        store.updateAuthorizedFileWatcher(
            with: AuthorizedFileWatchPlan(
                roots: [root.path],
                totalCount: 1,
                truncated: false
            )
        )
        let stopCountBeforeTransition = watcher.stopCount

        await store.setProject(
            rootPath: "/tmp/missing",
            currentCWD: "/tmp/missing",
            name: "Missing Project"
        )

        try expectEqual(
            watcher.stopCount,
            stopCountBeforeTransition + 1,
            "A committed project transition must stop the old watcher immediately."
        )
        try expectEqual(store.activeAuthorizedFileWatchRoots, [], "Old project roots must not remain active after transition.")
        try expectEqual(store.authorizedFileWatchPlan, .empty, "A failed follow-up scan must not retain the old watch plan.")
        try expectEqual(store.hasPendingFileSystemChanges, false, "Old-project invalidations must not leak into the new context.")

        watcher.emitFromStart(
            0,
            FileSystemChangeSummary(eventCount: 1, requiresDeepScan: true)
        )
        await Task.yield()
        try expectEqual(
            store.hasPendingFileSystemChanges,
            false,
            "A callback already queued by the stopped watcher must be rejected after the project transition."
        )
    }

    @Test("FSEvents starts for a sanitized authorized directory")
    func fseventsStartsForAuthorizedDirectory() throws {
        let tree = try TemporaryWatchTree(label: "fsevents")
        defer { tree.cleanup() }
        let root = try tree.createDirectory("home/.claude/skills")
        let watcher = FSEventsFileSystemWatcher()
        defer { watcher.stop() }

        let started = watcher.start(
            paths: [root.path],
            eventFilter: AuthorizedFileWatchEventFilter(
                plan: AuthorizedFileWatchPlan(
                    roots: [root.path],
                    totalCount: 1,
                    truncated: false
                ),
                sanitizedRoots: [root.path]
            )
        ) { _ in }

        try expectFalse(!started, "FSEvents should start for an existing non-symlink directory")
    }

    @Test("Watcher ignores runtime noise while retaining skill and config changes")
    func watcherFiltersRuntimeNoise() throws {
        let tree = try TemporaryWatchTree(label: "precise-events")
        defer { tree.cleanup() }
        let codexHome = try tree.createDirectory("home/.codex")
        let skills = try tree.createDirectory("home/.codex/skills")
        let config = codexHome.appendingPathComponent("config.toml")
        let filter = AuthorizedFileWatchEventFilter(
            plan: AuthorizedFileWatchPlan(
                roots: [codexHome.path, skills.path],
                recursiveRoots: [skills.path],
                exactFiles: [config.path],
                totalCount: 2,
                truncated: false
            ),
            sanitizedRoots: [codexHome.path, skills.path]
        )

        try expectFalse(
            filter.matches(codexHome.appendingPathComponent("state_5.sqlite-wal").path),
            "Codex state database writes must not mark skill/config data stale"
        )
        try expectFalse(
            filter.matches(codexHome.appendingPathComponent("logs/session.log").path),
            "Codex logs must not mark skill/config data stale"
        )
        try expectEqual(
            filter.matches(config.path),
            true,
            "The exact Codex config file must invalidate cached data"
        )
        try expectEqual(
            filter.matches(skills.appendingPathComponent("demo/SKILL.md").path),
            true,
            "Files inside an authorized recursive skill root must invalidate cached data"
        )
        try expectEqual(
            filter.matches(codexHome.path, requiresDeepScan: true),
            true,
            "Dropped events at a watched ancestor must conservatively invalidate cached data"
        )
    }

    @Test("Primary Refresh always performs a complete adapter reconciliation")
    func primaryRefreshAlwaysDeepScans() async throws {
        let tree = try TemporaryWatchTree(label: "routing")
        defer { tree.cleanup() }
        let root = try tree.createDirectory("home/.claude/skills")

        let cleanRunner = CatalogRefreshServiceRunner(scanFixtures: [.complete])
        let cleanWatcher = RecordingFileSystemWatcher()
        let cleanStore = SkillStore(
            service: cleanRunner.serviceClient(),
            fileSystemWatcher: cleanWatcher
        )
        cleanStore.updateAuthorizedFileWatcher(
            with: AuthorizedFileWatchPlan(
                roots: [root.path],
                totalCount: 1,
                truncated: false
            )
        )
        await cleanStore.refresh()
        try expectFalse(
            !(await cleanRunner.calls()).contains("\"method\":\"catalog.scanAll\""),
            "Refresh without invalidation should still perform a complete adapter reconciliation"
        )

        let changedRunner = CatalogRefreshServiceRunner(scanFixtures: [.complete])
        let changedWatcher = RecordingFileSystemWatcher()
        let changedStore = SkillStore(
            service: changedRunner.serviceClient(),
            fileSystemWatcher: changedWatcher
        )
        changedStore.updateAuthorizedFileWatcher(
            with: AuthorizedFileWatchPlan(
                roots: [root.path],
                totalCount: 1,
                truncated: false
            )
        )
        changedWatcher.emit(FileSystemChangeSummary(eventCount: 1, requiresDeepScan: false))
        await Task.yield()
        await changedStore.refresh()

        try expectFalse(
            !(await changedRunner.calls()).contains("\"method\":\"catalog.scanAll\""),
            "Refresh after invalidation should perform the same complete adapter reconciliation"
        )
        try expectFalse(
            changedStore.hasPendingFileSystemChanges,
            "Successful reconciliation should clear pending state"
        )
    }

    private func waitForCondition(
        _ message: String,
        timeout: TimeInterval = 2,
        condition: () -> Bool
    ) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while !condition() {
            if Date() > deadline {
                throw NativeModelTestFailure(description: message)
            }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
    }
}

private final class RecordingFileSystemWatcher: FileSystemWatching {
    private var onChange: (@Sendable (FileSystemChangeSummary) -> Void)?
    private var callbacks: [(@Sendable (FileSystemChangeSummary) -> Void)] = []
    private(set) var startedPaths: [String] = []
    private(set) var startCount = 0
    private(set) var stopCount = 0

    func start(
        paths: [String],
        eventFilter: AuthorizedFileWatchEventFilter,
        onChange: @escaping @Sendable (FileSystemChangeSummary) -> Void
    ) -> Bool {
        startedPaths = paths
        startCount += 1
        self.onChange = onChange
        callbacks.append(onChange)
        return true
    }

    func stop() {
        stopCount += 1
        onChange = nil
    }

    func emit(_ summary: FileSystemChangeSummary) {
        onChange?(summary)
    }

    func emitFromStart(_ index: Int, _ summary: FileSystemChangeSummary) {
        guard callbacks.indices.contains(index) else { return }
        callbacks[index](summary)
    }
}

private struct TemporaryWatchTree {
    let root: URL

    init(label: String) throws {
        let requested = URL(fileURLWithPath: "/private/tmp", isDirectory: true)
            .appendingPathComponent("agent-copilot-watch-\(label)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(
            at: requested,
            withIntermediateDirectories: true
        )
        root = requested
    }

    func createDirectory(_ relativePath: String) throws -> URL {
        let directory = root.appendingPathComponent(relativePath, isDirectory: true)
        try FileManager.default.createDirectory(
            at: directory,
            withIntermediateDirectories: true
        )
        return directory
    }

    func cleanup() {
        try? FileManager.default.removeItem(at: root)
    }
}
