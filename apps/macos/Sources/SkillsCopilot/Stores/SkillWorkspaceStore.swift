import Combine
import Foundation

/// Narrow read seam for the Skills workspace. Production uses the typed
/// `ServiceClient` method; tests can inject a deterministic in-memory client.
@MainActor
protocol SkillWorkspaceClient: AnyObject {
    func listSkillAggregates(
        projectID: String,
        expectedProjectContextRevision: String,
        agent: ProductAgentID?,
        limit: Int,
        cursor: String?,
        sourceRevision: String?
    ) async throws -> SkillAggregateListResult
}

extension ServiceClient: SkillWorkspaceClient {}

enum SkillWorkspaceView: String, CaseIterable, Identifiable {
    case needsAttention = "needs-attention"
    case project
    case global
    case all

    var id: String { rawValue }
}

enum SkillAggregateSortOrder: String, CaseIterable, Identifiable {
    case name
    case issueCount = "issue-count"
    case instanceCount = "instance-count"

    var id: String { rawValue }
}

@MainActor
final class SkillWorkspaceStore: ObservableObject {
    private struct SourceBinding: Equatable {
        let projectID: String
        let projectContextRevision: String
    }

    private struct AcceptedSnapshot {
        let binding: SourceBinding
        let sourceRevision: String
        let coverage: SourceCoverage
        let totalCount: Int?
        let aggregates: [SkillAggregateRecord]
    }

    private static let pageLimit = 100
    private static let maximumPageCount = 100

    @Published private(set) var aggregates: [SkillAggregateRecord] = []
    @Published private(set) var sourceRevision: String?
    @Published private(set) var coverage = SourceCoverage(
        completeness: .unknown,
        incompleteReason: .notInspected,
        inspectedSources: 0,
        expectedSources: nil
    )
    @Published private(set) var totalCount: Int?
    @Published private(set) var selectedAggregateID: SkillAggregateRecord.ID?
    @Published private(set) var isLoading = false
    @Published private(set) var isStale = false
    @Published private(set) var errorMessage: String?
    @Published private(set) var acceptedProjectID: String?
    @Published private(set) var acceptedProjectContextRevision: String?

    @Published private(set) var view: SkillWorkspaceView = .needsAttention {
        didSet {
            guard oldValue != view else { return }
            normalizeSelection()
        }
    }
    @Published private(set) var agentFilter: SkillAgentFilter = .all {
        didSet {
            guard oldValue != agentFilter else { return }
            normalizeSelection()
        }
    }
    @Published private(set) var searchText = "" {
        didSet {
            guard oldValue != searchText else { return }
            normalizeSelection()
        }
    }
    @Published private(set) var sortOrder: SkillAggregateSortOrder = .name {
        didSet {
            guard oldValue != sortOrder else { return }
            normalizeSelection()
        }
    }
    @Published private(set) var sortDirection: SkillSortDirection = .ascending {
        didSet {
            guard oldValue != sortDirection else { return }
            normalizeSelection()
        }
    }

    var visibleAggregates: [SkillAggregateRecord] {
        projectedAggregates()
    }

    var selectedAggregate: SkillAggregateRecord? {
        guard let selectedAggregateID else { return nil }
        return aggregates.first { $0.id == selectedAggregateID }
    }

    var listCompleteness: ListCompletenessState {
        let loadingPhase: ListLoadingPhase = isLoading
            ? (aggregates.isEmpty ? .initial : .all)
            : .idle
        if isStale {
            return ListCompletenessState(
                loadedCount: aggregates.count,
                totalCount: totalCount,
                hasMore: false,
                isComplete: false,
                completeness: .incomplete,
                incompleteReason: .staleSource,
                loadingPhase: loadingPhase,
                canLoadMore: false,
                canLoadAll: false
            )
        }
        let isComplete = coverage.isComplete
            && (totalCount == nil || totalCount == aggregates.count)
        let completeness: ListCompleteness
        switch coverage.completeness {
        case .enumerable:
            completeness = isComplete ? .complete : .partial
        case .limited:
            completeness = .incomplete
        case .unknown:
            completeness = .unknown
        }
        return ListCompletenessState(
            loadedCount: aggregates.count,
            totalCount: totalCount,
            hasMore: false,
            isComplete: isComplete,
            completeness: completeness,
            incompleteReason: coverage.incompleteReason,
            loadingPhase: loadingPhase,
            canLoadMore: false,
            canLoadAll: false
        )
    }

    private let client: any SkillWorkspaceClient
    private var sourceBinding: SourceBinding?
    private var requestGeneration: UInt64 = 0
    private var requestTask: Task<AcceptedSnapshot, Error>?

    init(service: ServiceClient) {
        client = service
    }

    init(client: any SkillWorkspaceClient) {
        self.client = client
    }

    deinit {
        requestTask?.cancel()
    }

    /// Startup/prewarm load. An already accepted snapshot for the exact
    /// project-context revision is reused without another RPC.
    func load(
        projectID: String,
        expectedProjectContextRevision: String
    ) async {
        let binding = SourceBinding(
            projectID: projectID,
            projectContextRevision: expectedProjectContextRevision
        )
        guard sourceBinding != binding || sourceRevision == nil else { return }
        await refresh(
            projectID: projectID,
            expectedProjectContextRevision: expectedProjectContextRevision
        )
    }

    /// Explicitly refreshes only the aggregate skill domain. Rows are assembled
    /// off to the side and published atomically after one terminal revision.
    func refresh(
        projectID: String,
        expectedProjectContextRevision: String
    ) async {
        let binding = SourceBinding(
            projectID: projectID,
            projectContextRevision: expectedProjectContextRevision
        )
        beginRequest(for: binding)
        let generation = requestGeneration
        let client = self.client
        let task = Task<AcceptedSnapshot, Error> {
            try await Self.fetchAllPages(client: client, binding: binding)
        }
        requestTask = task

        do {
            let snapshot = try await task.value
            guard generation == requestGeneration,
                  sourceBinding == binding,
                  !Task.isCancelled else {
                return
            }
            accept(snapshot)
            requestTask = nil
            isLoading = false
        } catch {
            guard generation == requestGeneration, sourceBinding == binding else {
                return
            }
            requestTask = nil
            isLoading = false
            guard !Self.isCancellation(error) else { return }
            isStale = sourceRevision != nil
            errorMessage = error.localizedDescription
        }
    }

    func cancelLoading() {
        requestGeneration &+= 1
        requestTask?.cancel()
        requestTask = nil
        isLoading = false
    }

    /// Removes project-bound aggregate state when there is no active project.
    /// This is a local cache transition and never invokes the service.
    func clearProject() {
        cancelLoading()
        sourceBinding = nil
        clearAcceptedSnapshot()
        errorMessage = nil
    }

    /// Applies routine view criteria against the accepted snapshot. This never
    /// invokes the service or changes the accepted source revision.
    func configure(
        view: SkillWorkspaceView,
        agentFilter: SkillAgentFilter,
        searchText: String,
        sortOrder: SkillAggregateSortOrder,
        sortDirection: SkillSortDirection
    ) {
        self.view = view
        self.agentFilter = agentFilter
        self.searchText = searchText
        self.sortOrder = sortOrder
        self.sortDirection = sortDirection
        normalizeSelection()
    }

    func selectAggregate(id: SkillAggregateRecord.ID?) {
        guard let id else {
            selectedAggregateID = nil
            return
        }
        guard visibleAggregates.contains(where: { $0.id == id }) else { return }
        selectedAggregateID = id
    }

    private func beginRequest(for binding: SourceBinding) {
        requestTask?.cancel()
        requestGeneration &+= 1
        if sourceBinding != binding {
            sourceBinding = binding
            clearAcceptedSnapshot()
        }
        isLoading = true
        errorMessage = nil
    }

    private func clearAcceptedSnapshot() {
        aggregates = []
        sourceRevision = nil
        coverage = SourceCoverage(
            completeness: .unknown,
            incompleteReason: .notInspected,
            inspectedSources: 0,
            expectedSources: nil
        )
        totalCount = nil
        selectedAggregateID = nil
        acceptedProjectID = nil
        acceptedProjectContextRevision = nil
        isStale = false
    }

    private func accept(_ snapshot: AcceptedSnapshot) {
        let preferredSelection = selectedAggregateID
        aggregates = snapshot.aggregates
        sourceRevision = snapshot.sourceRevision
        coverage = snapshot.coverage
        totalCount = snapshot.totalCount
        acceptedProjectID = snapshot.binding.projectID
        acceptedProjectContextRevision = snapshot.binding.projectContextRevision
        isStale = false
        errorMessage = nil
        normalizeSelection(preferred: preferredSelection)
    }

    private func normalizeSelection(preferred: SkillAggregateRecord.ID? = nil) {
        let visible = projectedAggregates()
        let preferred = preferred ?? selectedAggregateID
        if let preferred, visible.contains(where: { $0.id == preferred }) {
            selectedAggregateID = preferred
        } else {
            selectedAggregateID = nil
        }
    }

    private func projectedAggregates() -> [SkillAggregateRecord] {
        let query = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        let filtered = aggregates.filter { aggregate in
            matchesView(aggregate)
                && matchesAgent(aggregate)
                && matchesSearch(aggregate, query: query)
        }
        return filtered.sorted(by: compareAggregates)
    }

    private func matchesView(_ aggregate: SkillAggregateRecord) -> Bool {
        switch view {
        case .needsAttention:
            return aggregate.findingCount > 0
                || aggregate.conflictCount > 0
                || aggregate.primaryEffectiveness != .effective
                || !aggregate.coverage.isComplete
        case .project:
            return aggregate.scopes.contains(.agentProject)
        case .global:
            return aggregate.scopes.contains(.agentGlobal)
                || aggregate.scopes.contains(.toolGlobal)
        case .all:
            return true
        }
    }

    private func matchesAgent(_ aggregate: SkillAggregateRecord) -> Bool {
        switch agentFilter {
        case .all:
            return true
        default:
            return aggregate.agents.contains {
                $0.rawValue == agentFilter.rawValue
            }
        }
    }

    private func matchesSearch(_ aggregate: SkillAggregateRecord, query: String) -> Bool {
        guard !query.isEmpty else { return true }
        return [
            aggregate.displayName,
            aggregate.canonicalName,
            aggregate.description,
            aggregate.publisher,
            aggregate.packageName,
            aggregate.packageVersion,
        ]
        .compactMap { $0 }
        .contains { $0.localizedCaseInsensitiveContains(query) }
    }

    private func compareAggregates(
        _ left: SkillAggregateRecord,
        _ right: SkillAggregateRecord
    ) -> Bool {
        let order: ComparisonResult
        switch sortOrder {
        case .name:
            order = left.displayName.localizedCaseInsensitiveCompare(right.displayName)
        case .issueCount:
            order = compare(
                left.findingCount + left.conflictCount,
                right.findingCount + right.conflictCount
            )
        case .instanceCount:
            order = compare(left.instanceIDs.count, right.instanceIDs.count)
        }
        if order == .orderedSame {
            let nameOrder = left.displayName.localizedCaseInsensitiveCompare(right.displayName)
            if nameOrder == .orderedSame {
                return sortDirection == .ascending
                    ? left.id < right.id
                    : left.id > right.id
            }
            return sortDirection == .ascending
                ? nameOrder == .orderedAscending
                : nameOrder == .orderedDescending
        }
        return sortDirection == .ascending
            ? order == .orderedAscending
            : order == .orderedDescending
    }

    private func compare(_ left: Int, _ right: Int) -> ComparisonResult {
        if left == right { return .orderedSame }
        return left < right ? .orderedAscending : .orderedDescending
    }

    private static func fetchAllPages(
        client: any SkillWorkspaceClient,
        binding: SourceBinding
    ) async throws -> AcceptedSnapshot {
        var accumulator = ListPageAccumulator<SkillAggregateRecord>()
        accumulator.begin(.all)
        var cursor: String?
        var sourceRevision: String?
        var acceptedCoverage: SourceCoverage?
        var seenCursors = Set<String>()

        for _ in 0..<maximumPageCount {
            try Task.checkCancellation()
            let result = try await client.listSkillAggregates(
                projectID: binding.projectID,
                expectedProjectContextRevision: binding.projectContextRevision,
                agent: nil,
                limit: pageLimit,
                cursor: cursor,
                sourceRevision: sourceRevision
            )
            try Task.checkCancellation()
            if let sourceRevision, result.sourceRevision != sourceRevision {
                throw ListPageAccumulatorError.sourceChanged
            }
            if let acceptedCoverage, result.coverage != acceptedCoverage {
                throw ListPageAccumulatorError.sourceChanged
            }
            acceptedCoverage = acceptedCoverage ?? result.coverage
            try accumulator.append(ListPage(
                items: result.aggregates,
                returnedCount: result.page.returnedCount,
                totalCount: result.page.totalCount,
                hasMore: result.page.hasMore,
                nextCursor: result.page.nextCursor,
                sourceRevision: result.sourceRevision,
                sourceCompleteness: result.page.sourceCompleteness,
                incompleteReason: result.page.incompleteReason
            ))
            sourceRevision = result.sourceRevision
            guard result.page.hasMore else {
                guard let acceptedCoverage else {
                    throw ListPageAccumulatorError.invalidPage
                }
                return AcceptedSnapshot(
                    binding: binding,
                    sourceRevision: result.sourceRevision,
                    coverage: acceptedCoverage,
                    totalCount: accumulator.totalCount,
                    aggregates: accumulator.items
                )
            }
            guard let nextCursor = result.page.nextCursor,
                  nextCursor != cursor,
                  seenCursors.insert(nextCursor).inserted else {
                throw ListPageAccumulatorError.invalidPage
            }
            cursor = nextCursor
        }
        throw ListPageAccumulatorError.invalidPage
    }

    private static func isCancellation(_ error: Error) -> Bool {
        error is CancellationError
    }
}
