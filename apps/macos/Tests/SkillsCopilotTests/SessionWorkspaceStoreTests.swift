import Foundation
@testable import SkillsCopilot

@MainActor
struct SessionWorkspaceStoreTests {
    static func runRegisteredSuite() async throws {
        try await SessionWorkspaceStoreTests().run()
    }

    func run() async throws {
        try await inventoryPagingOwnsCanonicalSourceAndFiltersLocally()
        try await pendingSelectionLoadsUntilTheExactSessionIsAvailable()
        try await failureCancellationAndSourceSwitchPreserveAcceptedInventory()
        try await timelinePagingBindsRevisionAndPreservesAcceptedMessages()
        try await resumePreviewUsesOnlyServerRecordAndRejectsStalePublication()
    }

    private func inventoryPagingOwnsCanonicalSourceAndFiltersLocally() async throws {
        let fake = FakeSessionWorkspaceService()
        fake.enqueueInventory(.success(inventoryPage(
            rows: [row(id: "one", title: "One", endedAt: 20)],
            total: 2,
            hasMore: true,
            nextCursor: "cursor-1",
            sourceRevision: "sha256:native-1"
        )))
        fake.enqueueInventory(.success(inventoryPage(
            rows: [row(id: "two", title: "Two", endedAt: 10)],
            total: 2,
            hasMore: false,
            nextCursor: nil,
            sourceRevision: "sha256:native-1"
        )))
        let store = SessionWorkspaceStore(service: fake)
        store.configure(
            project: project(),
            snapshotRevision: "sha256:product-1",
            authorizedRoots: ["/sessions-b", "/sessions-a", "/sessions-a"],
            agentFilter: .codex
        )

        await store.refreshInventory()
        try expectEqual(store.rows.map(\.id), ["one"], "First page should publish immediately.")
        try expectEqual(store.sourceRevision, "sha256:native-1", "Native source revision")
        try expectEqual(store.snapshotRevision, "sha256:product-1", "Product snapshot revision")
        try expectNil(
            store.selectedSessionID,
            "Loading a workspace must not manufacture an implicit selection."
        )
        store.selectSession("one")
        try expectEqual(
            store.inventoryCompleteness.loadingPhase,
            .idle,
            "Completed request should return the inventory to idle."
        )

        await store.loadNextInventoryPage()
        try expectEqual(store.rows.map(\.id), ["one", "two"], "Keyset pages should accumulate.")
        try expectEqual(store.inventoryCompleteness.loadedCount, 2, "Accepted row count")
        try expectEqual(store.inventoryCompleteness.isComplete, true, "Terminal exact page")

        store.setCriteria(SessionWorkspaceCriteria(
            scope: .project,
            search: "two",
            sort: .title,
            direction: .ascending
        ))
        try expectEqual(store.rows.map(\.id), ["two"], "Criteria should project accepted rows locally.")
        try expectNil(store.selectedSessionID, "A hidden selection should clear locally.")
        store.selectSession("two")
        try expectEqual(store.selectedSessionID, "two", "Selection should remain explicit.")
        try expectEqual(fake.inventoryRequests.count, 2, "Criteria changes must not issue a read.")

        let first = try required(fake.inventoryRequests.first, "Missing first inventory request.")
        try expectEqual(first.authorizedRoots, ["/sessions-a", "/sessions-b"], "Roots are stable.")
        try expectEqual(first.agent, .codex, "Agent source filter")
        try expectNil(first.cursor, "First keyset cursor")
        try expectNil(first.sourceRevision, "First keyset revision")
        try expectEqual(first.limit, 100, "Inventory page bound")
        let second = try required(fake.inventoryRequests.last, "Missing continuation request.")
        try expectEqual(second.cursor, "cursor-1", "Continuation cursor")
        try expectEqual(second.sourceRevision, "sha256:native-1", "Continuation revision")
    }

    private func failureCancellationAndSourceSwitchPreserveAcceptedInventory() async throws {
        let fake = FakeSessionWorkspaceService()
        fake.enqueueInventory(.success(inventoryPage(
            rows: [row(id: "accepted", title: "Accepted")],
            total: 1,
            hasMore: false,
            sourceRevision: "sha256:accepted"
        )))
        let store = SessionWorkspaceStore(service: fake)
        store.configure(
            project: project(),
            snapshotRevision: "sha256:product-1",
            authorizedRoots: [],
            agentFilter: .codex
        )
        await store.refreshInventory()
        store.selectSession("accepted")

        fake.enqueueInventory(.failure(FakeSessionWorkspaceError.failed))
        await store.refreshInventory()
        try expectEqual(store.rows.map(\.id), ["accepted"], "Failed refresh keeps accepted rows.")
        try expectEqual(store.selectedSessionID, "accepted", "Failed refresh keeps selection.")
        guard case .stale = store.inventoryState else {
            throw NativeModelTestFailure(
                description: "A failed refresh with data should expose stale state."
            )
        }

        fake.suspendNextInventory()
        let cancelledTask = Task { @MainActor in
            await store.refreshInventory()
        }
        try await waitUntil {
            fake.inventoryRequests.count == 3
        }
        store.cancelInventoryRequest()
        fake.resolveInventory(.success(inventoryPage(
            rows: [row(id: "late-cancelled")],
            total: 1,
            hasMore: false,
            sourceRevision: "sha256:late"
        )))
        await cancelledTask.value
        try expectEqual(store.rows.map(\.id), ["accepted"], "Cancelled response must not publish.")

        fake.suspendNextInventory()
        let staleTask = Task { @MainActor in
            await store.refreshInventory()
        }
        try await waitUntil {
            fake.inventoryRequests.count == 4
        }
        let otherProject = project(
            id: "project-other",
            name: "other",
            rootPath: "/other"
        )
        store.configure(
            project: otherProject,
            snapshotRevision: "sha256:product-other",
            authorizedRoots: [],
            agentFilter: .codex
        )
        fake.resolveInventory(.success(inventoryPage(
            rows: [row(id: "wrong-project", projectRoot: "/project")],
            total: 1,
            hasMore: false,
            sourceRevision: "sha256:wrong-project"
        )))
        await staleTask.value
        try expectEqual(store.rows.count, 0, "Old-project response must not enter new source.")

        store.configure(
            project: project(),
            snapshotRevision: "sha256:product-1",
            authorizedRoots: [],
            agentFilter: .codex
        )
        try expectEqual(store.rows.map(\.id), ["accepted"], "Switching back restores source cache.")
        try expectEqual(store.selectedSessionID, "accepted", "Switching back restores selection.")
    }

    private func pendingSelectionLoadsUntilTheExactSessionIsAvailable() async throws {
        let fake = FakeSessionWorkspaceService()
        fake.enqueueInventory(.success(inventoryPage(
            rows: [row(id: "first")],
            total: 2,
            hasMore: true,
            nextCursor: "cursor-1",
            sourceRevision: "sha256:native-1"
        )))
        fake.enqueueInventory(.success(inventoryPage(
            rows: [row(id: "requested")],
            total: 2,
            hasMore: false,
            sourceRevision: "sha256:native-1"
        )))
        let store = SessionWorkspaceStore(service: fake)
        store.configure(
            project: project(),
            snapshotRevision: "sha256:product-1",
            authorizedRoots: [],
            agentFilter: .codex
        )
        store.requestSessionSelection("requested")

        await store.loadInventoryIfNeeded()
        try expectNil(
            store.selectedSessionID,
            "A not-yet-loaded route target must remain pending without selecting another row."
        )
        await store.loadPendingSelectionIfNeeded()

        try expectEqual(
            store.selectedSessionID,
            "requested",
            "Exact navigation should continue bounded inventory pages until its row is accepted."
        )
        try expectEqual(
            fake.inventoryRequests.map(\.cursor),
            [nil, "cursor-1"],
            "Pending navigation must preserve the accepted keyset cursor."
        )
    }

    private func timelinePagingBindsRevisionAndPreservesAcceptedMessages() async throws {
        let fake = FakeSessionWorkspaceService()
        fake.enqueueInventory(.success(inventoryPage(
            rows: [row(id: "timeline", title: "Timeline")],
            total: 1,
            hasMore: false,
            sourceRevision: "sha256:native-1"
        )))
        let store = SessionWorkspaceStore(service: fake)
        store.configure(
            project: project(),
            snapshotRevision: "sha256:product-1",
            authorizedRoots: [],
            agentFilter: .codex
        )
        await store.refreshInventory()
        store.selectSession("timeline")

        fake.enqueueMessages(.success(messagePage(
            sessionID: "timeline",
            items: [message(id: "message-1", text: "First")],
            total: 2,
            hasMore: true,
            nextCursor: "cursor-1",
            sourceRevision: "sha256:messages-1"
        )))
        await store.loadSelectedSessionTimelineIfNeeded()
        try expectEqual(store.selectedTimelineItems.map(\.id), ["message-1"], "First timeline page")
        try expectEqual(store.selectedMessageCompleteness.canLoadMore, true, "Timeline can continue")

        fake.enqueueMessages(.success(messagePage(
            sessionID: "timeline",
            items: [message(id: "message-2", text: "Second")],
            total: 2,
            hasMore: false,
            sourceRevision: "sha256:messages-1"
        )))
        await store.loadNextSelectedSessionTimelinePage()
        try expectEqual(
            store.selectedTimelineItems.map(\.id),
            ["message-1", "message-2"],
            "Fixed-revision timeline pages should accumulate exactly once."
        )
        try expectEqual(store.selectedMessageCompleteness.isComplete, true, "Terminal exact timeline")
        let requests = fake.messageRequests
        try expectEqual(requests.count, 2, "Two message pages")
        try expectNil(requests[0].cursor, "First message cursor")
        try expectNil(requests[0].sourceRevision, "First message revision is discovered by service.")
        try expectEqual(requests[1].cursor, "cursor-1", "Second message cursor")
        try expectEqual(
            requests[1].sourceRevision,
            "sha256:messages-1",
            "Continuation page is bound to the first accepted message revision."
        )

        store.selectSession(nil)
        store.selectSession("timeline")
        await store.loadSelectedSessionTimelineIfNeeded()
        try expectEqual(fake.messageRequests.count, 2, "Complete cached timeline should not reread.")
        try expectEqual(
            store.selectedTimelineItems.map(\.id),
            ["message-1", "message-2"],
            "Explicit reselection should restore accepted timeline evidence."
        )

        fake.enqueueInventory(.success(inventoryPage(
            rows: [row(id: "timeline", title: "Timeline")],
            total: 1,
            hasMore: false,
            sourceRevision: "sha256:native-2"
        )))
        await store.refreshInventory()
        try expectEqual(store.selectedSessionID, "timeline", "Stable session selection survives refresh.")
        try expectEqual(
            store.selectedTimelineItems,
            [],
            "A changed native inventory revision invalidates cached message evidence."
        )
        try expectEqual(
            store.selectedMessageCompleteness.incompleteReason,
            .notInspected,
            "Changed source requires a fresh bounded timeline read."
        )
    }

    private func resumePreviewUsesOnlyServerRecordAndRejectsStalePublication() async throws {
        let fake = FakeSessionWorkspaceService()
        fake.enqueueInventory(.success(inventoryPage(
            rows: [
                row(id: "session-one", title: "Session One"),
                row(id: "session-two", title: "Session Two"),
            ],
            total: 2,
            hasMore: false,
            sourceRevision: "sha256:native-1"
        )))
        let store = SessionWorkspaceStore(service: fake)
        store.configure(
            project: project(),
            snapshotRevision: "sha256:product-1",
            authorizedRoots: [],
            agentFilter: .codex
        )
        await store.refreshInventory()
        store.selectSession("session-one")
        try expectEqual(
            store.serverResumeArguments,
            [],
            "A selected Codex row alone must never synthesize a command."
        )

        let acceptedRecord = try continuation(
            sessionID: "session-one",
            sourceRevision: "sha256:native-1",
            snapshotRevision: "sha256:product-1",
            argv: ["verified-runtime", "--opaque", "native-session"]
        )
        fake.enqueueResume(.success(acceptedRecord))
        await store.previewSelectedSessionResume()
        try expectEqual(
            store.serverResumeArguments,
            ["verified-runtime", "--opaque", "native-session"],
            "Server argv order and values must remain exact."
        )
        let request = try required(fake.resumeRequests.last, "Missing resume request.")
        try expectEqual(request.sessionID, "session-one", "Resume stable session id")
        try expectEqual(request.sourceRevision, "sha256:native-1", "Resume native revision")
        try expectEqual(request.snapshotRevision, "sha256:product-1", "Resume product revision")

        fake.enqueueResume(.failure(FakeSessionWorkspaceError.failed))
        await store.previewSelectedSessionResume()
        try expectEqual(
            store.resumePreview,
            acceptedRecord,
            "A failed re-preview should preserve the last accepted record."
        )
        try expectFalse(store.resumeError == nil, "Failed re-preview should expose an error.")

        fake.suspendNextResume()
        let criteriaTask = Task { @MainActor in
            await store.previewSelectedSessionResume()
        }
        try await waitUntil {
            fake.resumeRequests.count == 3
        }
        store.setCriteria(SessionWorkspaceCriteria(
            scope: .project,
            search: "session one",
            sort: .recent,
            direction: .descending
        ))
        fake.resolveResume(.success(try continuation(
            sessionID: "session-one",
            sourceRevision: "sha256:native-1",
            snapshotRevision: "sha256:product-1",
            argv: ["late", "criteria"]
        )))
        await criteriaTask.value
        try expectEqual(
            store.resumePreview,
            acceptedRecord,
            "Criteria cancellation keeps accepted preview and rejects late replacement."
        )

        fake.suspendNextResume()
        let revisionTask = Task { @MainActor in
            await store.previewSelectedSessionResume()
        }
        try await waitUntil {
            fake.resumeRequests.count == 4
        }
        store.configure(
            project: project(),
            snapshotRevision: "sha256:product-2",
            authorizedRoots: [],
            agentFilter: .codex
        )
        try expectNil(store.resumePreview, "Snapshot revision change invalidates old preview.")
        fake.resolveResume(.success(try continuation(
            sessionID: "session-one",
            sourceRevision: "sha256:native-1",
            snapshotRevision: "sha256:product-1",
            argv: ["late", "snapshot"]
        )))
        await revisionTask.value
        try expectNil(store.resumePreview, "Late old-snapshot preview must not publish.")

        let currentRecord = try continuation(
            sessionID: "session-one",
            sourceRevision: "sha256:native-1",
            snapshotRevision: "sha256:product-2",
            argv: ["server", "current"]
        )
        fake.enqueueResume(.success(currentRecord))
        await store.previewSelectedSessionResume()
        try expectEqual(store.resumePreview, currentRecord, "Current revision preview should publish.")

        fake.suspendNextResume()
        let selectionTask = Task { @MainActor in
            await store.previewSelectedSessionResume()
        }
        try await waitUntil {
            fake.resumeRequests.count == 6
        }
        store.setCriteria(SessionWorkspaceCriteria())
        store.selectSession("session-two")
        fake.resolveResume(.success(try continuation(
            sessionID: "session-one",
            sourceRevision: "sha256:native-1",
            snapshotRevision: "sha256:product-2",
            argv: ["late", "selection"]
        )))
        await selectionTask.value
        try expectNil(store.resumePreview, "Old-selection preview must not publish.")
        try expectEqual(store.serverResumeArguments, [], "No command is inferred for new selection.")
    }

    private func project(
        id: String = "project-test",
        name: String = "project",
        rootPath: String = "/project"
    ) -> ProjectContext {
        ProjectContext(
            id: id,
            name: name,
            rootPath: rootPath,
            currentCWD: rootPath,
            lastUsedAt: nil,
            isActive: true,
            validationError: nil
        )
    }

    private func row(
        id: String,
        title: String? = nil,
        projectRoot: String = "/project",
        endedAt: Int64 = 10
    ) -> LocalSessionPreviewRow {
        LocalSessionPreviewRow(
            id: id,
            title: title ?? id,
            sourceKind: "codex-state-index",
            scope: "all",
            agent: "codex",
            projectRoot: projectRoot,
            redactedPath: "<session>",
            modifiedAt: String(endedAt),
            startedAt: endedAt,
            endedAt: endedAt,
            excerpt: title ?? id,
            excerptCharCount: (title ?? id).count,
            userMessageCount: 1,
            totalMessageCount: 2,
            toolCallCount: 0,
            skillCallCount: 0,
            contentHash: "sha256:\(id)",
            evidenceRefs: [],
            contentIncluded: false,
            contentItems: []
        )
    }

    private func inventoryPage(
        rows: [LocalSessionPreviewRow],
        total: Int,
        hasMore: Bool,
        nextCursor: String? = nil,
        sourceRevision: String
    ) -> LocalSessionPreviewResult {
        LocalSessionPreviewResult(
            generatedBy: "test",
            authorized: true,
            sessionRows: rows,
            totalCandidateCount: total,
            totalMatchedCount: total,
            limit: 100,
            hasMore: hasMore,
            nextCursor: nextCursor,
            sourceRevision: sourceRevision,
            sourceCompleteness: .enumerable
        )
    }

    private func continuation(
        sessionID: String,
        sourceRevision: String,
        snapshotRevision: String,
        argv: [String]
    ) throws -> SessionContinuationRecord {
        let payload: [String: Any] = [
            "id": sessionID,
            "agent": "codex",
            "project_id": "project-test",
            "title": sessionID,
            "modified_at": 10,
            "source_kind": "codex-state-index",
            "source_revision": sourceRevision,
            "snapshot_revision": snapshotRevision,
            "coverage": [
                "completeness": "enumerable",
                "inspected_sources": 1,
                "expected_sources": 1,
            ],
            "resume": [
                "state": "supported",
                "argv": argv,
                "copy_only": true,
            ],
            "evidence": [],
            "actions": [],
        ]
        return try JSONDecoder().decode(
            SessionContinuationRecord.self,
            from: JSONSerialization.data(withJSONObject: payload)
        )
    }

    private func message(
        id: String,
        text: String
    ) -> LocalSessionContentItem {
        LocalSessionContentItem(
            id: id,
            kind: .userMessage,
            title: "User",
            text: text,
            timestamp: 10
        )
    }

    private func messagePage(
        sessionID: String,
        items: [LocalSessionContentItem],
        total: Int,
        hasMore: Bool,
        nextCursor: String? = nil,
        sourceRevision: String
    ) -> LocalSessionMessagePageResult {
        LocalSessionMessagePageResult(
            generatedBy: "test",
            sessionID: sessionID,
            contentItems: items,
            returnedCount: items.count,
            totalCount: total,
            hasMore: hasMore,
            nextCursor: nextCursor,
            sourceRevision: sourceRevision,
            sourceCompleteness: .enumerable,
            incompleteReason: nil,
            scannedBytes: 0,
            scannedThroughBytes: 0,
            snapshotBytes: 0
        )
    }

    private func waitUntil(
        _ predicate: @MainActor () -> Bool
    ) async throws {
        for _ in 0..<200 {
            if predicate() { return }
            await Task.yield()
        }
        throw NativeModelTestFailure(description: "Timed out waiting for fake request.")
    }

    private func required<Value>(_ value: Value?, _ message: String) throws -> Value {
        guard let value else {
            throw NativeModelTestFailure(description: message)
        }
        return value
    }
}

private enum FakeSessionWorkspaceError: LocalizedError {
    case failed

    var errorDescription: String? {
        "fake request failed"
    }
}

@MainActor
private final class FakeSessionWorkspaceService: SessionWorkspaceServicing {
    private(set) var inventoryRequests: [SessionWorkspaceInventoryRequest] = []
    private(set) var messageRequests: [SessionWorkspaceMessageRequest] = []
    private(set) var resumeRequests: [SessionWorkspaceResumeRequest] = []
    private var inventoryResults: [Result<LocalSessionPreviewResult, Error>] = []
    private var messageResults: [Result<LocalSessionMessagePageResult, Error>] = []
    private var resumeResults: [Result<SessionContinuationRecord, Error>] = []
    private var shouldSuspendInventory = false
    private var shouldSuspendResume = false
    private var inventoryContinuation: CheckedContinuation<LocalSessionPreviewResult, Error>?
    private var resumeContinuation: CheckedContinuation<SessionContinuationRecord, Error>?

    func enqueueInventory(_ result: Result<LocalSessionPreviewResult, Error>) {
        inventoryResults.append(result)
    }

    func enqueueResume(_ result: Result<SessionContinuationRecord, Error>) {
        resumeResults.append(result)
    }

    func enqueueMessages(_ result: Result<LocalSessionMessagePageResult, Error>) {
        messageResults.append(result)
    }

    func suspendNextInventory() {
        shouldSuspendInventory = true
    }

    func suspendNextResume() {
        shouldSuspendResume = true
    }

    func resolveInventory(_ result: Result<LocalSessionPreviewResult, Error>) {
        let continuation = inventoryContinuation
        inventoryContinuation = nil
        continuation?.resume(with: result)
    }

    func resolveResume(_ result: Result<SessionContinuationRecord, Error>) {
        let continuation = resumeContinuation
        resumeContinuation = nil
        continuation?.resume(with: result)
    }

    func readSessionInventory(
        _ request: SessionWorkspaceInventoryRequest
    ) async throws -> LocalSessionPreviewResult {
        inventoryRequests.append(request)
        if shouldSuspendInventory {
            shouldSuspendInventory = false
            return try await withCheckedThrowingContinuation { continuation in
                inventoryContinuation = continuation
            }
        }
        guard !inventoryResults.isEmpty else {
            throw NativeModelTestFailure(description: "Missing fake inventory result.")
        }
        return try inventoryResults.removeFirst().get()
    }

    func readSessionResume(
        _ request: SessionWorkspaceResumeRequest
    ) async throws -> SessionContinuationRecord {
        resumeRequests.append(request)
        if shouldSuspendResume {
            shouldSuspendResume = false
            return try await withCheckedThrowingContinuation { continuation in
                resumeContinuation = continuation
            }
        }
        guard !resumeResults.isEmpty else {
            throw NativeModelTestFailure(description: "Missing fake resume result.")
        }
        return try resumeResults.removeFirst().get()
    }

    func readSessionMessages(
        _ request: SessionWorkspaceMessageRequest
    ) async throws -> LocalSessionMessagePageResult {
        messageRequests.append(request)
        guard !messageResults.isEmpty else {
            throw NativeModelTestFailure(description: "Missing fake message result.")
        }
        return try messageResults.removeFirst().get()
    }
}
