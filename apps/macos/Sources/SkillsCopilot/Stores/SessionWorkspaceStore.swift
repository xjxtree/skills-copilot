import Combine
import Foundation

struct SessionWorkspaceInventoryRequest: Equatable {
    let authorizedRoots: [String]
    let agent: ProductAgentID?
    let project: ProjectContext
    let limit: Int
    let cursor: String?
    let sourceRevision: String?
}

struct SessionWorkspaceResumeRequest: Equatable {
    let authorizedRoots: [String]
    let agent: ProductAgentID
    let project: ProjectContext
    let sessionID: String
    let sourceRevision: String
    let snapshotRevision: String
}

struct SessionWorkspaceMessageRequest: Equatable {
    let authorizedRoots: [String]
    let agent: ProductAgentID?
    let project: ProjectContext
    let sessionID: String
    let limit: Int
    let cursor: String?
    let sourceRevision: String?
}

@MainActor
protocol SessionWorkspaceServicing: AnyObject {
    func readSessionInventory(
        _ request: SessionWorkspaceInventoryRequest
    ) async throws -> LocalSessionPreviewResult

    func readSessionResume(
        _ request: SessionWorkspaceResumeRequest
    ) async throws -> SessionContinuationRecord

    func readSessionMessages(
        _ request: SessionWorkspaceMessageRequest
    ) async throws -> LocalSessionMessagePageResult
}

@MainActor
final class ServiceClientSessionWorkspaceService: SessionWorkspaceServicing {
    private let client: ServiceClient

    init(client: ServiceClient) {
        self.client = client
    }

    func readSessionInventory(
        _ request: SessionWorkspaceInventoryRequest
    ) async throws -> LocalSessionPreviewResult {
        try await client.previewLocalSessions(
            authorizedRoots: request.authorizedRoots,
            agent: request.agent?.rawValue,
            scope: .all,
            search: nil,
            project: request.project,
            sessionID: nil,
            includeContentItems: false,
            limit: request.limit,
            offset: nil,
            pagingMode: "keyset",
            cursor: request.cursor,
            sourceRevision: request.sourceRevision,
            sort: .recent,
            direction: .descending,
            maxFiles: nil
        )
    }

    func readSessionResume(
        _ request: SessionWorkspaceResumeRequest
    ) async throws -> SessionContinuationRecord {
        try await client.previewSessionResume(
            authorizedRoots: request.authorizedRoots,
            agent: request.agent,
            project: request.project,
            sessionID: request.sessionID,
            expectedSourceRevision: request.sourceRevision,
            expectedSnapshotRevision: request.snapshotRevision
        )
    }

    func readSessionMessages(
        _ request: SessionWorkspaceMessageRequest
    ) async throws -> LocalSessionMessagePageResult {
        try await client.listLocalSessionMessages(
            authorizedRoots: request.authorizedRoots,
            agent: request.agent?.rawValue,
            project: request.project,
            sessionID: request.sessionID,
            limit: request.limit,
            cursor: request.cursor,
            sourceRevision: request.sourceRevision
        )
    }
}

struct SessionWorkspaceCriteria: Equatable {
    var scope: LocalSessionScopeFilter = .project
    var search = ""
    var sort: LocalSessionSortOrder = .recent
    var direction: SkillSortDirection = .descending
}

private struct SessionWorkspaceMessageSnapshot {
    var accumulator: ListPageAccumulator<LocalSessionContentItem>
    var displayError: String?

    init() {
        accumulator = ListPageAccumulator()
        displayError = nil
    }
}

@MainActor
final class SessionWorkspaceStore: ObservableObject {
    private static let inventoryPageLimit = 100
    private static let messagePageLimit = 40

    @Published private(set) var project: ProjectContext?
    @Published private(set) var agentFilter: ProductAgentID?
    @Published private(set) var authorizedRoots: [String] = []
    @Published private(set) var snapshotRevision: String?
    @Published private(set) var criteria = SessionWorkspaceCriteria()
    @Published private(set) var inventoryState: LocalSessionLoadState = .empty
    @Published private(set) var inventoryCompleteness = SessionWorkspaceStore.emptyCompleteness
    @Published private(set) var selectedSessionID: String?
    @Published private(set) var selectedSessionDetailState: LocalSessionDetailState?
    @Published private(set) var selectedMessageCompleteness = SessionWorkspaceStore.emptyMessageCompleteness
    @Published private(set) var resumePreview: SessionContinuationRecord?
    @Published private(set) var isPreviewingResume = false
    @Published private(set) var resumeError: String?

    private let service: SessionWorkspaceServicing
    private let cache = LocalSessionCache()
    private var selectionsBySource: [LocalSessionSnapshotKey: String] = [:]
    private var pendingSelectionBySource: [LocalSessionSnapshotKey: String] = [:]
    private var messageSnapshots: [LocalSessionDetailKey: SessionWorkspaceMessageSnapshot] = [:]
    private var inventoryTask: Task<Void, Never>?
    private var messageTask: Task<Void, Never>?
    private var activeMessageKey: LocalSessionDetailKey?
    private var resumeTask: Task<Void, Never>?
    private var inventoryOperation: UInt64 = 0
    private var messageOperation: UInt64 = 0
    private var messageLoadAllOperation: UInt64 = 0
    private var resumeOperation: UInt64 = 0
    private var activeInventoryGeneration: (
        key: LocalSessionSnapshotKey,
        generation: UInt64
    )?

    init(service: SessionWorkspaceServicing) {
        self.service = service
    }

    convenience init(serviceClient: ServiceClient) {
        self.init(service: ServiceClientSessionWorkspaceService(client: serviceClient))
    }

    var rows: [LocalSessionPreviewRow] {
        guard let sourceKey else { return [] }
        return cache.projectedRows(
            for: sourceKey,
            criteria: LocalSessionProjectionCriteria(
                scope: criteria.scope,
                search: criteria.search,
                sort: criteria.sort,
                direction: criteria.direction,
                projectRoot: project?.rootPath,
                currentCWD: project?.currentCWD
            )
        )
    }

    var selectedSession: LocalSessionPreviewRow? {
        guard let selectedSessionID else { return nil }
        return rows.first { $0.id == selectedSessionID }
    }

    var sourceRevision: String? {
        acceptedSnapshot?.sourceRevision
    }

    var acceptedSnapshot: LocalSessionSnapshot? {
        guard let sourceKey else { return nil }
        return cache.successfulSnapshot(for: sourceKey)
    }

    var inventoryBindingID: String {
        guard let sourceKey else { return "unconfigured" }
        return [
            sourceKey.agent,
            sourceKey.projectRoot,
            sourceKey.currentCWD,
            sourceKey.authorizedRoots.joined(separator: "\u{1f}"),
        ].joined(separator: "\u{1e}")
    }

    var selectedSessionDetail: LocalSessionPreviewRow? {
        if case .loaded(let row) = selectedSessionDetailState {
            return row
        }
        return selectedSession
    }

    var selectedTimelineItems: [LocalSessionContentItem] {
        selectedSessionDetail?.contentItems ?? []
    }

    var selectedMessageError: String? {
        guard let selectedDetailKey else { return nil }
        return messageSnapshots[selectedDetailKey]?.displayError
    }

    var inventoryDisplayError: String? {
        switch inventoryState {
        case .stale(_, let displayError), .failed(_, let displayError):
            return displayError
        case .empty, .loading, .fresh, .refreshing:
            return nil
        }
    }

    /// Arguments are exposed exactly as decoded from the server-owned
    /// continuation record. This store never manufactures a fallback command.
    var serverResumeArguments: [String] {
        guard resumePreviewMatchesCurrentSelection else { return [] }
        return resumePreview?.resume.argv ?? []
    }

    func configure(
        project: ProjectContext?,
        snapshotRevision: String?,
        authorizedRoots: [String],
        agentFilter: ProductAgentID?
    ) {
        let previousKey = sourceKey
        let normalizedRoots = Array(
            Set(
                authorizedRoots
                    .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                    .filter { !$0.isEmpty }
            )
        ).sorted()
        let normalizedSnapshotRevision = normalizedRevision(snapshotRevision)

        self.project = project
        self.snapshotRevision = normalizedSnapshotRevision
        self.authorizedRoots = normalizedRoots
        self.agentFilter = normalizedAgent(agentFilter)

        let nextKey = sourceKey
        if previousKey != nextKey {
            cancelInventoryRequest()
            cancelMessageRequest()
            cancelResumeRequest(clearAccepted: true)
            if let nextKey {
                cache.activateSource(nextKey)
                messageSnapshots = messageSnapshots.filter { $0.key.source == nextKey }
            } else {
                messageSnapshots.removeAll()
            }
            synchronizeInventoryState()
            restoreSelectionForCurrentSource()
            synchronizeSelectedMessageState()
        } else {
            invalidateResumePreviewIfNeeded()
        }
    }

    func setAgentFilter(_ agent: ProductAgentID?) {
        configure(
            project: project,
            snapshotRevision: snapshotRevision,
            authorizedRoots: authorizedRoots,
            agentFilter: agent
        )
    }

    func setCriteria(_ criteria: SessionWorkspaceCriteria) {
        guard self.criteria != criteria else { return }
        self.criteria = criteria
        cancelResumeRequest(clearAccepted: false)
        normalizeSelection(preserveMissingWhilePaging: false)
        synchronizeSelectedMessageState()
        invalidateResumePreviewIfNeeded()
    }

    func selectSession(_ id: String?) {
        let normalized = id.flatMap { candidate in
            rows.contains(where: { $0.id == candidate }) ? candidate : nil
        }
        guard selectedSessionID != normalized else { return }
        selectedSessionID = normalized
        if let sourceKey {
            if let normalized {
                selectionsBySource[sourceKey] = normalized
                pendingSelectionBySource.removeValue(forKey: sourceKey)
            } else {
                selectionsBySource.removeValue(forKey: sourceKey)
            }
        }
        cancelMessageRequest()
        cancelResumeRequest(clearAccepted: true)
        synchronizeSelectedMessageState()
    }

    func requestSessionSelection(_ id: String) {
        guard let sourceKey else { return }
        if rows.contains(where: { $0.id == id }) {
            selectSession(id)
        } else {
            pendingSelectionBySource[sourceKey] = id
        }
    }

    func loadInventoryIfNeeded() async {
        guard acceptedSnapshot == nil else { return }
        await refreshInventory()
    }

    func loadPendingSelectionIfNeeded() async {
        guard let sourceKey,
              pendingSelectionBySource[sourceKey] != nil else { return }
        if acceptedSnapshot == nil {
            await refreshInventory()
        }
        while !Task.isCancelled,
              self.sourceKey == sourceKey,
              pendingSelectionBySource[sourceKey] != nil,
              let cursor = acceptedSnapshot?.nextCursor {
            await loadNextInventoryPage()
            guard !Task.isCancelled,
                  acceptedSnapshot?.nextCursor != cursor else { return }
            await Task.yield()
        }
    }

    func refreshInventory() async {
        startInventoryRequest(reset: true)
        let task = inventoryTask
        await task?.value
    }

    func loadNextInventoryPage() async {
        guard acceptedSnapshot?.nextCursor != nil else { return }
        startInventoryRequest(reset: false)
        let task = inventoryTask
        await task?.value
    }

    func loadAllInventoryPages() async {
        if acceptedSnapshot == nil {
            await refreshInventory()
        }
        while let cursor = acceptedSnapshot?.nextCursor {
            await loadNextInventoryPage()
            guard !Task.isCancelled,
                  acceptedSnapshot?.nextCursor != cursor else { return }
            await Task.yield()
        }
    }

    func cancelInventoryRequest() {
        inventoryOperation &+= 1
        inventoryTask?.cancel()
        inventoryTask = nil
        if let activeInventoryGeneration {
            cache.cancelSummaryLoad(
                key: activeInventoryGeneration.key,
                generation: activeInventoryGeneration.generation
            )
        }
        activeInventoryGeneration = nil
        synchronizeInventoryState(cancelledWithoutSnapshot: true)
    }

    func loadSelectedSessionTimelineIfNeeded() async {
        guard let key = selectedDetailKey else { return }
        if let snapshot = messageSnapshots[key],
           snapshot.accumulator.state.isComplete {
            synchronizeSelectedMessageState()
            return
        }
        await loadSelectedSessionTimelinePage(reset: messageSnapshots[key] == nil)
    }

    func loadNextSelectedSessionTimelinePage() async {
        await loadSelectedSessionTimelinePage(reset: false)
    }

    func loadAllSelectedSessionTimeline() async {
        guard messageTask == nil else { return }
        messageLoadAllOperation &+= 1
        let loadAllOperation = messageLoadAllOperation
        repeat {
            await loadSelectedSessionTimelinePage(reset: false)
            guard messageLoadAllOperation == loadAllOperation,
                  !Task.isCancelled,
                  selectedMessageCompleteness.canLoadMore else { return }
            await Task.yield()
        } while true
    }

    func cancelMessageRequest() {
        messageLoadAllOperation &+= 1
        messageOperation &+= 1
        messageTask?.cancel()
        messageTask = nil
        let cancelledKey = activeMessageKey ?? selectedDetailKey
        activeMessageKey = nil
        guard let key = cancelledKey,
              var snapshot = messageSnapshots[key] else {
            synchronizeSelectedMessageState()
            return
        }
        snapshot.accumulator.cancel()
        messageSnapshots[key] = snapshot
        synchronizeSelectedMessageState()
    }

    func previewSelectedSessionResume() async {
        guard let request = makeResumeRequest() else {
            resumeError = SessionWorkspaceStoreError.resumeEvidenceUnavailable.localizedDescription
            return
        }

        cancelResumeRequest(clearAccepted: false)
        resumeOperation &+= 1
        let operation = resumeOperation
        isPreviewingResume = true
        resumeError = nil
        let task = Task { @MainActor [weak self] in
            guard let self else { return }
            await self.performResumeRequest(request, operation: operation)
        }
        resumeTask = task
        await task.value
    }

    func cancelResumeRequest() {
        cancelResumeRequest(clearAccepted: false)
    }

    private var sourceKey: LocalSessionSnapshotKey? {
        guard let project else { return nil }
        return LocalSessionSnapshotKey(
            agent: agentFilter?.rawValue ?? SkillAgentFilter.all.rawValue,
            projectRoot: project.rootPath,
            currentCWD: project.currentCWD ?? project.rootPath,
            authorizedRoots: authorizedRoots
        )
    }

    private var selectedDetailKey: LocalSessionDetailKey? {
        guard let sourceKey, let selectedSessionID else { return nil }
        return LocalSessionDetailKey(source: sourceKey, sessionID: selectedSessionID)
    }

    private func loadSelectedSessionTimelinePage(reset: Bool) async {
        guard messageTask == nil,
              let project,
              let selectedSession,
              let key = selectedDetailKey else { return }

        var snapshot = reset ? SessionWorkspaceMessageSnapshot()
            : messageSnapshots[key] ?? SessionWorkspaceMessageSnapshot()
        if !reset,
           !snapshot.accumulator.items.isEmpty,
           !snapshot.accumulator.state.canLoadMore {
            synchronizeSelectedMessageState()
            return
        }
        snapshot.accumulator.begin(snapshot.accumulator.items.isEmpty ? .initial : .more)
        snapshot.displayError = nil
        messageSnapshots[key] = snapshot
        synchronizeSelectedMessageState()

        messageOperation &+= 1
        let operation = messageOperation
        let request = SessionWorkspaceMessageRequest(
            authorizedRoots: authorizedRoots,
            agent: selectedSession.agent.flatMap(ProductAgentID.init(rawValue:)),
            project: project,
            sessionID: selectedSession.id,
            limit: Self.messagePageLimit,
            cursor: snapshot.accumulator.nextCursor,
            sourceRevision: snapshot.accumulator.sourceRevision
        )
        let task = Task { @MainActor [weak self] in
            guard let self else { return }
            await self.performMessageRequest(
                request,
                key: key,
                operation: operation
            )
        }
        activeMessageKey = key
        messageTask = task
        await task.value
    }

    private func performMessageRequest(
        _ request: SessionWorkspaceMessageRequest,
        key: LocalSessionDetailKey,
        operation: UInt64
    ) async {
        do {
            let page = try await service.readSessionMessages(request)
            try Task.checkCancellation()
            guard messageOperation == operation,
                  selectedDetailKey == key,
                  page.sessionID == request.sessionID else { return }
            guard var snapshot = messageSnapshots[key] else { return }
            try snapshot.accumulator.append(page.listPage)
            snapshot.displayError = nil
            messageSnapshots[key] = snapshot
        } catch {
            guard messageOperation == operation,
                  selectedDetailKey == key,
                  var snapshot = messageSnapshots[key] else { return }
            if Task.isCancelled {
                snapshot.accumulator.cancel()
            } else {
                snapshot.accumulator.fail(reason: messageFailureReason(for: error))
                snapshot.displayError = error.localizedDescription
            }
            messageSnapshots[key] = snapshot
        }
        if messageOperation == operation {
            messageTask = nil
            activeMessageKey = nil
            synchronizeSelectedMessageState()
        }
    }

    private func synchronizeSelectedMessageState() {
        guard let selectedSession,
              let key = selectedDetailKey else {
            selectedSessionDetailState = nil
            selectedMessageCompleteness = Self.emptyMessageCompleteness
            return
        }
        guard let snapshot = messageSnapshots[key] else {
            selectedSessionDetailState = nil
            selectedMessageCompleteness = Self.emptyMessageCompleteness
            return
        }
        let items = snapshot.accumulator.items
        selectedMessageCompleteness = snapshot.accumulator.state
        if !items.isEmpty || snapshot.accumulator.state.isComplete {
            selectedSessionDetailState = .loaded(
                selectedSession.replacingContentItems(
                    items,
                    exactFinalMessages: items
                )
            )
        } else if let displayError = snapshot.displayError {
            selectedSessionDetailState = .failed(displayError: displayError)
        } else if snapshot.accumulator.state.loadingPhase != .idle {
            selectedSessionDetailState = .loading(generation: messageOperation)
        } else {
            selectedSessionDetailState = nil
        }
    }

    private func messageFailureReason(for error: Error) -> ListIncompleteReason {
        guard let accumulatorError = error as? ListPageAccumulatorError else {
            return .pageFailed
        }
        switch accumulatorError {
        case .sourceChanged:
            return .sourceChanged
        case .invalidPage:
            return .pageFailed
        }
    }

    private func startInventoryRequest(reset: Bool) {
        guard let project, let sourceKey else { return }
        cancelInventoryRequest()

        let base = reset ? nil : cache.successfulSnapshot(for: sourceKey)
        if !reset, base?.nextCursor == nil {
            return
        }
        let generation = cache.beginSummaryRefresh(for: sourceKey)
        activeInventoryGeneration = (sourceKey, generation)
        inventoryOperation &+= 1
        let operation = inventoryOperation
        let request = SessionWorkspaceInventoryRequest(
            authorizedRoots: sourceKey.authorizedRoots,
            agent: agentFilter,
            project: project,
            limit: Self.inventoryPageLimit,
            cursor: base?.nextCursor,
            sourceRevision: base?.sourceRevision
        )
        let task = Task { @MainActor [weak self] in
            guard let self else { return }
            await self.performInventoryRequest(
                request,
                sourceKey: sourceKey,
                base: base,
                generation: generation,
                operation: operation
            )
        }
        inventoryTask = task
        synchronizeInventoryState()
    }

    private func performInventoryRequest(
        _ request: SessionWorkspaceInventoryRequest,
        sourceKey: LocalSessionSnapshotKey,
        base: LocalSessionSnapshot?,
        generation: UInt64,
        operation: UInt64
    ) async {
        do {
            let page = try await service.readSessionInventory(request)
            try Task.checkCancellation()
            guard inventoryOperation == operation, self.sourceKey == sourceKey else { return }
            try validateInventoryPage(page, request: request, base: base)

            let previousSourceRevision = cache.successfulSnapshot(for: sourceKey)?.sourceRevision
            if let previousSourceRevision,
               page.sourceRevision != previousSourceRevision {
                cancelMessageRequest()
                cancelResumeRequest(clearAccepted: true)
                messageSnapshots = messageSnapshots.filter { $0.key.source != sourceKey }
            }
            let result = base?.result.mergingPage(page) ?? page
            let snapshot = LocalSessionSnapshot(
                key: sourceKey,
                generation: generation,
                result: result,
                refreshedAt: Date(),
                isComplete: inventoryIsComplete(result),
                nextCursor: page.nextCursor,
                sourceRevision: page.sourceRevision,
                sourceCompleteness: page.sourceCompleteness,
                incompleteReason: page.incompleteReason
            )
            cache.publishSummary(snapshot)
            synchronizeInventoryState()
            resolvePendingSelectionIfPossible()
            normalizeSelection(preserveMissingWhilePaging: page.hasMore)
            synchronizeSelectedMessageState()
            invalidateResumePreviewIfNeeded()
        } catch {
            guard inventoryOperation == operation, self.sourceKey == sourceKey else { return }
            if Task.isCancelled {
                cache.cancelSummaryLoad(key: sourceKey, generation: generation)
                synchronizeInventoryState(cancelledWithoutSnapshot: true)
            } else {
                cache.failSummary(
                    key: sourceKey,
                    generation: generation,
                    displayError: error.localizedDescription
                )
                synchronizeInventoryState()
            }
        }
        if inventoryOperation == operation {
            inventoryTask = nil
            activeInventoryGeneration = nil
            synchronizeInventoryState()
        }
    }

    private func validateInventoryPage(
        _ page: LocalSessionPreviewResult,
        request: SessionWorkspaceInventoryRequest,
        base: LocalSessionSnapshot?
    ) throws {
        guard page.count == page.sessionRows.count,
              Set(page.sessionRows.map(\.id)).count == page.sessionRows.count,
              !page.hasMore || (page.nextCursor != nil && page.nextCursor != request.cursor),
              page.hasMore || page.nextCursor == nil else {
            throw SessionWorkspaceStoreError.invalidInventoryPage
        }
        if let expected = request.sourceRevision,
           page.sourceRevision != expected {
            throw SessionWorkspaceStoreError.sourceChanged
        }
        let mergedCount = (base?.result.mergingPage(page) ?? page).sessionRows.count
        if !page.hasMore,
           page.sourceCompleteness == .enumerable,
           page.incompleteReason == nil,
           page.totalMatchedCount != mergedCount {
            throw SessionWorkspaceStoreError.invalidInventoryPage
        }
    }

    private func inventoryIsComplete(_ result: LocalSessionPreviewResult) -> Bool {
        !result.hasMore
            && result.sourceCompleteness == .enumerable
            && result.incompleteReason == nil
            && result.totalMatchedCount == result.sessionRows.count
    }

    private func synchronizeInventoryState(cancelledWithoutSnapshot: Bool = false) {
        guard let sourceKey else {
            inventoryState = .empty
            inventoryCompleteness = Self.emptyCompleteness
            return
        }
        if cancelledWithoutSnapshot, cache.successfulSnapshot(for: sourceKey) == nil {
            inventoryState = .empty
            inventoryCompleteness = Self.emptyCompleteness
            return
        }
        inventoryState = cache.summaryStates[sourceKey] ?? .empty
        guard let snapshot = cache.successfulSnapshot(for: sourceKey) else {
            inventoryCompleteness = inventoryTask == nil
                ? Self.emptyCompleteness
                : ListCompletenessState(
                    loadedCount: 0,
                    totalCount: nil,
                    hasMore: false,
                    isComplete: false,
                    completeness: .unknown,
                    incompleteReason: nil,
                    loadingPhase: .initial,
                    canLoadMore: false,
                    canLoadAll: false
                )
            return
        }
        inventoryCompleteness = ListCompletenessState(
            loadedCount: snapshot.result.sessionRows.count,
            totalCount: snapshot.result.sourceCompleteness == .enumerable
                ? snapshot.result.totalMatchedCount
                : nil,
            hasMore: snapshot.nextCursor != nil,
            isComplete: snapshot.isComplete,
            completeness: snapshot.isComplete
                ? .complete
                : snapshot.sourceCompleteness == .unknown
                    ? .unknown
                    : snapshot.sourceCompleteness == .limited
                        || snapshot.incompleteReason != nil ? .incomplete : .partial,
            incompleteReason: snapshot.incompleteReason,
            loadingPhase: inventoryTask == nil
                ? .idle
                : snapshot.result.sessionRows.isEmpty ? .initial : .more,
            canLoadMore: snapshot.nextCursor != nil && inventoryTask == nil,
            canLoadAll: snapshot.nextCursor != nil && inventoryTask == nil
        )
    }

    private func restoreSelectionForCurrentSource() {
        guard let sourceKey else {
            selectedSessionID = nil
            return
        }
        selectedSessionID = selectionsBySource[sourceKey]
        normalizeSelection(preserveMissingWhilePaging: true)
    }

    private func normalizeSelection(preserveMissingWhilePaging: Bool) {
        let visibleRows = rows
        if let selectedSessionID,
           visibleRows.contains(where: { $0.id == selectedSessionID }) {
            return
        }
        if preserveMissingWhilePaging,
           selectedSessionID != nil,
           acceptedSnapshot?.nextCursor != nil {
            return
        }
        let previous = selectedSessionID
        selectedSessionID = nil
        if let sourceKey {
            selectionsBySource.removeValue(forKey: sourceKey)
        }
        if previous != selectedSessionID {
            cancelMessageRequest()
            cancelResumeRequest(clearAccepted: true)
        }
    }

    private func resolvePendingSelectionIfPossible() {
        guard let sourceKey,
              let requested = pendingSelectionBySource[sourceKey] else { return }
        if rows.contains(where: { $0.id == requested }) {
            pendingSelectionBySource.removeValue(forKey: sourceKey)
            selectedSessionID = requested
            selectionsBySource[sourceKey] = requested
            cancelMessageRequest()
            cancelResumeRequest(clearAccepted: true)
        } else if acceptedSnapshot?.nextCursor == nil {
            pendingSelectionBySource.removeValue(forKey: sourceKey)
        }
    }

    private func makeResumeRequest() -> SessionWorkspaceResumeRequest? {
        guard let project,
              let selectedSession,
              let sourceRevision = normalizedRevision(sourceRevision),
              let snapshotRevision = normalizedRevision(snapshotRevision),
              let rawAgent = selectedSession.agent,
              let agent = ProductAgentID(rawValue: rawAgent),
              ProductAgentID.projectAgents.contains(agent) else {
            return nil
        }
        return SessionWorkspaceResumeRequest(
            authorizedRoots: authorizedRoots,
            agent: agent,
            project: project,
            sessionID: selectedSession.id,
            sourceRevision: sourceRevision,
            snapshotRevision: snapshotRevision
        )
    }

    private func performResumeRequest(
        _ request: SessionWorkspaceResumeRequest,
        operation: UInt64
    ) async {
        do {
            let record = try await service.readSessionResume(request)
            try Task.checkCancellation()
            guard resumeOperation == operation,
                  makeResumeRequest() == request,
                  resumeRecord(record, matches: request) else { return }
            resumePreview = record
            resumeError = nil
        } catch {
            guard resumeOperation == operation else { return }
            if !Task.isCancelled {
                resumeError = error.localizedDescription
            }
        }
        if resumeOperation == operation {
            resumeTask = nil
            isPreviewingResume = false
        }
    }

    private func resumeRecord(
        _ record: SessionContinuationRecord,
        matches request: SessionWorkspaceResumeRequest
    ) -> Bool {
        record.id == request.sessionID
            && record.agent == request.agent
            && record.projectID == request.project.id
            && record.sourceRevision == request.sourceRevision
            && record.snapshotRevision == request.snapshotRevision
    }

    private var resumePreviewMatchesCurrentSelection: Bool {
        guard let resumePreview, let request = makeResumeRequest() else { return false }
        return resumeRecord(resumePreview, matches: request)
    }

    private func invalidateResumePreviewIfNeeded() {
        guard resumePreview != nil else { return }
        if !resumePreviewMatchesCurrentSelection {
            self.resumePreview = nil
            resumeError = nil
        }
    }

    private func cancelResumeRequest(clearAccepted: Bool) {
        resumeOperation &+= 1
        resumeTask?.cancel()
        resumeTask = nil
        isPreviewingResume = false
        resumeError = nil
        if clearAccepted {
            resumePreview = nil
        }
    }

    private func normalizedRevision(_ revision: String?) -> String? {
        guard let revision = revision?.trimmingCharacters(in: .whitespacesAndNewlines),
              !revision.isEmpty else { return nil }
        return revision
    }

    private func normalizedAgent(_ agent: ProductAgentID?) -> ProductAgentID? {
        guard agent != .toolGlobal else { return nil }
        return agent
    }

    private static let emptyCompleteness = ListCompletenessState(
        loadedCount: 0,
        totalCount: nil,
        hasMore: false,
        isComplete: false,
        completeness: .unknown,
        incompleteReason: .notInspected,
        loadingPhase: .idle,
        canLoadMore: false,
        canLoadAll: false
    )

    private static let emptyMessageCompleteness = ListCompletenessState(
        loadedCount: 0,
        totalCount: nil,
        hasMore: false,
        isComplete: false,
        completeness: .unknown,
        incompleteReason: .notInspected,
        loadingPhase: .idle,
        canLoadMore: false,
        canLoadAll: true
    )
}

private enum SessionWorkspaceStoreError: LocalizedError {
    case invalidInventoryPage
    case sourceChanged
    case resumeEvidenceUnavailable

    var errorDescription: String? {
        switch self {
        case .invalidInventoryPage:
            return "The session service returned an invalid inventory page."
        case .sourceChanged:
            return "The session source changed while loading."
        case .resumeEvidenceUnavailable:
            return "Verified session and revision evidence is required before previewing resume."
        }
    }
}
