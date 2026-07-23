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
    let nextCursor: String?
    let sourceRevision: String?
    let sourceCompleteness: ListSourceCompleteness
    let incompleteReason: ListIncompleteReason?

    init(
        key: LocalSessionSnapshotKey,
        generation: UInt64,
        result: LocalSessionPreviewResult,
        refreshedAt: Date,
        isComplete: Bool,
        nextCursor: String? = nil,
        sourceRevision: String? = nil,
        sourceCompleteness: ListSourceCompleteness = .enumerable,
        incompleteReason: ListIncompleteReason? = nil
    ) {
        self.key = key
        self.generation = generation
        self.result = result
        self.refreshedAt = refreshedAt
        self.isComplete = isComplete
        self.nextCursor = nextCursor
        self.sourceRevision = sourceRevision
        self.sourceCompleteness = sourceCompleteness
        self.incompleteReason = incompleteReason
    }
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
    let currentCWD: String?

    init(
        scope: LocalSessionScopeFilter,
        search: String,
        sort: LocalSessionSortOrder,
        direction: SkillSortDirection,
        projectRoot: String?,
        currentCWD: String? = nil
    ) {
        self.scope = scope
        self.search = search
        self.sort = sort
        self.direction = direction
        self.projectRoot = projectRoot
        self.currentCWD = currentCWD
    }
}

enum LocalSessionRefreshReason: Equatable {
    case startup
    case manual
    case sourceChanged
}

enum LocalSessionSelectionOrigin: Equatable {
    case user
    case navigation
    case criteriaNormalization
}

@MainActor
final class LocalSessionCache {
    static let maximumDetailEntries = 12

    private(set) var summaryStates: [LocalSessionSnapshotKey: LocalSessionLoadState] = [:]
    private(set) var detailStates: [LocalSessionDetailKey: LocalSessionDetailState] = [:]
    private(set) var detailCompleteness: [LocalSessionDetailKey: ListCompletenessState] = [:]

    private var nextGeneration: UInt64 = 0
    private var summaryGenerations: [LocalSessionSnapshotKey: UInt64] = [:]
    private var detailGenerations: [LocalSessionDetailKey: UInt64] = [:]
    private var detailRecency: [LocalSessionDetailKey] = []

    func beginSummaryRefresh(for key: LocalSessionSnapshotKey) -> UInt64 {
        let generation = makeGeneration()
        summaryGenerations[key] = generation
        if let snapshot = successfulSnapshot(for: key) {
            summaryStates[key] = .refreshing(snapshot)
        } else {
            summaryStates[key] = .loading(key: key)
        }
        return generation
    }

    func publishSummary(_ snapshot: LocalSessionSnapshot) {
        guard summaryGenerations[snapshot.key] == snapshot.generation else { return }
        let rows = snapshot.result.sessionRows.map(\.summaryOnly)
        let sanitized = LocalSessionSnapshot(
            key: snapshot.key,
            generation: snapshot.generation,
            result: summaryResult(snapshot.result, rows: rows),
            refreshedAt: snapshot.refreshedAt,
            isComplete: snapshot.isComplete,
            nextCursor: snapshot.nextCursor,
            sourceRevision: snapshot.sourceRevision,
            sourceCompleteness: snapshot.sourceCompleteness,
            incompleteReason: snapshot.incompleteReason
        )
        summaryStates[snapshot.key] = .fresh(sanitized)
    }

    func cancelSummaryLoad(key: LocalSessionSnapshotKey, generation: UInt64) {
        guard summaryGenerations[key] == generation else { return }
        summaryGenerations[key] = makeGeneration()
        if let snapshot = successfulSnapshot(for: key) {
            summaryStates[key] = .fresh(snapshot)
        }
    }

    func failSummary(key: LocalSessionSnapshotKey, generation: UInt64, displayError: String) {
        guard summaryGenerations[key] == generation else { return }
        if let snapshot = successfulSnapshot(for: key) {
            summaryStates[key] = .stale(snapshot, displayError: displayError)
        } else {
            summaryStates[key] = .failed(key: key, displayError: displayError)
        }
    }

    func beginDetailLoad(for key: LocalSessionDetailKey) -> UInt64? {
        if let state = detailStates[key] {
            switch state {
            case .loading:
                touchDetail(key)
                return nil
            case .loaded:
                if detailCompleteness[key]?.isComplete != false {
                    touchDetail(key)
                    return nil
                }
            case .failed:
                break
            }
        }
        let generation = makeGeneration()
        detailGenerations[key] = generation
        let shouldShowInitialLoading: Bool
        switch detailStates[key] {
        case .none, .some(.failed):
            shouldShowInitialLoading = true
        case .some(.loading), .some(.loaded):
            shouldShowInitialLoading = false
        }
        if shouldShowInitialLoading {
            detailStates[key] = .loading(generation: generation)
        }
        let loadedCount = detailCompleteness[key]?.loadedCount ?? 0
        detailCompleteness[key] = ListCompletenessState(
            loadedCount: loadedCount,
            totalCount: detailCompleteness[key]?.totalCount,
            hasMore: true,
            isComplete: false,
            completeness: loadedCount > 0 ? .partial : .unknown,
            incompleteReason: nil,
            loadingPhase: .all,
            canLoadMore: false,
            canLoadAll: false
        )
        touchDetail(key)
        trimDetails()
        return generation
    }

    func publishDetail(
        _ row: LocalSessionPreviewRow,
        key: LocalSessionDetailKey,
        generation: UInt64
    ) -> Bool {
        guard row.id == key.sessionID,
              detailGenerations[key] == generation,
              case .loading(generation) = detailStates[key] else { return false }
        detailStates[key] = .loaded(row)
        detailCompleteness[key] = ListCompletenessState(
            loadedCount: row.contentItems.count,
            totalCount: row.contentItems.count,
            hasMore: false,
            isComplete: true,
            completeness: .complete,
            incompleteReason: nil,
            loadingPhase: .idle,
            canLoadMore: false,
            canLoadAll: false
        )
        touchDetail(key)
        trimDetails()
        return true
    }

    func publishDetailProgress(
        _ row: LocalSessionPreviewRow,
        completeness: ListCompletenessState,
        key: LocalSessionDetailKey,
        generation: UInt64
    ) -> Bool {
        guard row.id == key.sessionID,
              detailGenerations[key] == generation,
              detailStates[key] != nil else { return false }
        detailStates[key] = .loaded(row)
        detailCompleteness[key] = completeness
        touchDetail(key)
        trimDetails()
        return true
    }

    func failDetail(
        key: LocalSessionDetailKey,
        generation: UInt64,
        displayError: String,
        reason: ListIncompleteReason = .pageFailed
    ) {
        guard detailGenerations[key] == generation else { return }
        if case .loaded = detailStates[key] {
            let loadedCount = detailCompleteness[key]?.loadedCount ?? 0
            detailCompleteness[key] = ListCompletenessState(
                loadedCount: loadedCount,
                totalCount: detailCompleteness[key]?.totalCount,
                hasMore: true,
                isComplete: false,
                completeness: .incomplete,
                incompleteReason: reason,
                loadingPhase: .idle,
                canLoadMore: false,
                canLoadAll: true
            )
        } else if case .loading(generation) = detailStates[key] {
            detailStates[key] = .failed(displayError: displayError)
            detailCompleteness.removeValue(forKey: key)
        } else {
            return
        }
        touchDetail(key)
        trimDetails()
    }

    func cancelDetailLoad(key: LocalSessionDetailKey) {
        guard case .loaded = detailStates[key] else { return }
        detailGenerations[key] = makeGeneration()
        let loadedCount = detailCompleteness[key]?.loadedCount ?? 0
        detailCompleteness[key] = ListCompletenessState(
            loadedCount: loadedCount,
            totalCount: detailCompleteness[key]?.totalCount,
            hasMore: true,
            isComplete: false,
            completeness: .partial,
            incompleteReason: .pageFailed,
            loadingPhase: .idle,
            canLoadMore: false,
            canLoadAll: true
        )
    }

    func successfulSnapshot(for key: LocalSessionSnapshotKey) -> LocalSessionSnapshot? {
        switch summaryStates[key] {
        case .fresh(let snapshot), .refreshing(let snapshot), .stale(let snapshot, _):
            return snapshot
        case .empty, .loading, .failed, .none:
            return nil
        }
    }

    func projectedRows(
        for key: LocalSessionSnapshotKey,
        criteria: LocalSessionProjectionCriteria
    ) -> [LocalSessionPreviewRow] {
        guard let snapshot = successfulSnapshot(for: key) else { return [] }
        let query = criteria.search.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        let requestedProjects = Set(
            [criteria.projectRoot, criteria.currentCWD]
                .compactMap(normalizedProjectPath)
        )
        let rows = snapshot.result.sessionRows.filter { row in
            let rowProject = normalizedProjectPath(row.projectRoot)
            let scopeMatches = criteria.scope == .all || {
                if row.projectRoot == "<project-root>" {
                    return true
                }
                if let rowProject, !requestedProjects.isEmpty {
                    return requestedProjects.contains(rowProject)
                }
                return row.scope.lowercased().contains("project")
            }()
            guard scopeMatches else { return false }
            guard !query.isEmpty else { return true }
            return [
                row.title,
                row.excerpt,
                row.agent ?? "",
                row.projectRoot ?? "",
                row.sourceKind,
            ]
                .contains { $0.lowercased().contains(query) }
        }
        return rows.enumerated().sorted { leftPair, rightPair in
            let left = leftPair.element
            let right = rightPair.element
            let ordered: ComparisonResult
            switch criteria.sort {
            case .recent:
                let leftTime = left.endedAt ?? left.startedAt ?? Int64(left.modifiedAt ?? "") ?? 0
                let rightTime = right.endedAt ?? right.startedAt ?? Int64(right.modifiedAt ?? "") ?? 0
                if leftTime == rightTime {
                    return leftPair.offset < rightPair.offset
                } else {
                    ordered = leftTime < rightTime ? .orderedAscending : .orderedDescending
                }
            case .title:
                ordered = left.title.localizedCaseInsensitiveCompare(right.title)
            }
            if ordered == .orderedSame { return leftPair.offset < rightPair.offset }
            return criteria.direction == .ascending
                ? ordered == .orderedAscending
                : ordered == .orderedDescending
        }.map(\.element)
    }

    func activateSource(_ key: LocalSessionSnapshotKey) {
        let obsolete = detailStates.keys.filter { $0.source != key }
        for detailKey in obsolete {
            detailStates.removeValue(forKey: detailKey)
            detailGenerations.removeValue(forKey: detailKey)
            detailCompleteness.removeValue(forKey: detailKey)
        }
        detailRecency.removeAll { $0.source != key }
    }

    private func makeGeneration() -> UInt64 {
        nextGeneration &+= 1
        return nextGeneration
    }

    private func touchDetail(_ key: LocalSessionDetailKey) {
        detailRecency.removeAll { $0 == key }
        detailRecency.append(key)
    }

    private func trimDetails() {
        while detailStates.count > Self.maximumDetailEntries, let oldest = detailRecency.first {
            detailRecency.removeFirst()
            detailStates.removeValue(forKey: oldest)
            detailGenerations.removeValue(forKey: oldest)
            detailCompleteness.removeValue(forKey: oldest)
        }
    }

    private func normalizedProjectPath(_ path: String?) -> String? {
        guard let path = path?.trimmingCharacters(in: .whitespacesAndNewlines), !path.isEmpty else {
            return nil
        }
        let standardized = (path as NSString).standardizingPath
        return standardized == "/"
            ? standardized
            : standardized.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
    }

    private func summaryResult(
        _ result: LocalSessionPreviewResult,
        rows: [LocalSessionPreviewRow]
    ) -> LocalSessionPreviewResult {
        LocalSessionPreviewResult(
            generatedBy: result.generatedBy,
            authorized: result.authorized,
            authorizationRequired: result.authorizationRequired,
            roots: result.roots,
            sessionRows: rows,
            skillUsageRows: result.skillUsageRows,
            count: rows.count,
            totalCandidateCount: result.totalCandidateCount,
            totalMatchedCount: result.totalMatchedCount,
            offset: result.offset,
            limit: result.limit,
            hasMore: result.hasMore,
            nextOffset: result.nextOffset,
            nextCursor: result.nextCursor,
            sourceRevision: result.sourceRevision,
            sourceCompleteness: result.sourceCompleteness,
            incompleteReason: result.incompleteReason,
            candidateSetTruncated: result.candidateSetTruncated,
            userMessageCount: result.userMessageCount,
            totalMessageCount: result.totalMessageCount,
            toolCallCount: result.toolCallCount,
            skillCallCount: result.skillCallCount,
            gapNotes: result.gapNotes,
            blockerNotes: result.blockerNotes,
            redactionSummary: result.redactionSummary,
            safetyFlags: result.safetyFlags,
            fallbackReason: result.fallbackReason
        )
    }
}
