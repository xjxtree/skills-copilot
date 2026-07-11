enum ListSourceCompleteness: String, Codable, Hashable {
    case enumerable
    case limited
    case unknown
}

enum ListIncompleteReason: String, Codable, Hashable {
    case safetyBudget = "safety_budget"
    case sourceChanged = "source_changed"
    case sourceLimited = "source_limited"
    case unreadableSource = "unreadable_source"
    case pageFailed = "page_failed"
    case unsupportedProtocol = "unsupported_protocol"
}

enum ListCompleteness: Equatable {
    case complete
    case partial
    case incomplete
    case unknown
}

enum ListLoadingPhase: Equatable {
    case idle
    case initial
    case more
    case all
}

struct ListCompletenessState: Equatable {
    let loadedCount: Int
    let totalCount: Int?
    let hasMore: Bool
    let isComplete: Bool
    let completeness: ListCompleteness
    let incompleteReason: ListIncompleteReason?
    let loadingPhase: ListLoadingPhase
    let canLoadMore: Bool
    let canLoadAll: Bool
}

struct ListPage<Item> {
    let items: [Item]
    let returnedCount: Int
    let totalCount: Int?
    let hasMore: Bool
    let nextCursor: String?
    let sourceRevision: String?
    let sourceCompleteness: ListSourceCompleteness
    let incompleteReason: ListIncompleteReason?
}

enum ListPageAccumulatorError: Error, Equatable {
    case sourceChanged
    case invalidPage
}

struct ListPageAccumulator<Item: Identifiable> where Item.ID: Hashable {
    private(set) var items: [Item] = []
    private(set) var nextCursor: String?
    private(set) var sourceRevision: String?
    private(set) var totalCount: Int?
    private(set) var sourceCompleteness: ListSourceCompleteness = .unknown
    private(set) var incompleteReason: ListIncompleteReason?
    private(set) var loadingPhase: ListLoadingPhase = .idle
    private var seenIDs = Set<Item.ID>()
    private var hasMore = false

    init() {}

    init(cachedItems: [Item]) {
        items = cachedItems
        seenIDs = Set(cachedItems.map(\.id))
    }

    mutating func append(_ page: ListPage<Item>) throws {
        guard page.returnedCount == page.items.count else {
            throw ListPageAccumulatorError.invalidPage
        }
        guard !(page.hasMore
            && page.sourceCompleteness == .enumerable
            && page.nextCursor == nil) else {
            throw ListPageAccumulatorError.invalidPage
        }
        guard page.hasMore || page.nextCursor == nil else {
            throw ListPageAccumulatorError.invalidPage
        }
        if let sourceRevision, let pageRevision = page.sourceRevision,
           sourceRevision != pageRevision {
            throw ListPageAccumulatorError.sourceChanged
        }

        var candidateIDs = seenIDs
        let novelItems = page.items.filter { candidateIDs.insert($0.id).inserted }
        let candidateTotalCount = page.totalCount ?? totalCount
        if !page.hasMore,
           page.sourceCompleteness == .enumerable,
           let candidateTotalCount,
           items.count + novelItems.count != candidateTotalCount {
            throw ListPageAccumulatorError.invalidPage
        }

        seenIDs = candidateIDs
        items.append(contentsOf: novelItems)
        if sourceRevision == nil {
            sourceRevision = page.sourceRevision
        }
        nextCursor = page.nextCursor
        totalCount = candidateTotalCount
        hasMore = page.hasMore
        sourceCompleteness = page.sourceCompleteness
        incompleteReason = page.incompleteReason
        if loadingPhase != .all || !page.hasMore {
            loadingPhase = .idle
        }
    }

    mutating func begin(_ phase: ListLoadingPhase) {
        loadingPhase = phase
    }

    mutating func cancel() {
        loadingPhase = .idle
    }

    mutating func fail(reason: ListIncompleteReason) {
        incompleteReason = reason
        loadingPhase = .idle
        if reason != .pageFailed {
            hasMore = false
            nextCursor = nil
        }
    }

    var state: ListCompletenessState {
        let isComplete = incompleteReason == nil
            && !hasMore
            && sourceCompleteness == .enumerable
            && (totalCount == nil || totalCount == items.count)
        let completeness: ListCompleteness
        if isComplete {
            completeness = .complete
        } else if incompleteReason == .pageFailed,
                  sourceCompleteness == .enumerable,
                  hasMore {
            completeness = .partial
        } else if sourceCompleteness == .limited || incompleteReason != nil {
            completeness = .incomplete
        } else if sourceCompleteness == .unknown {
            completeness = .unknown
        } else {
            completeness = .partial
        }
        let failureAllowsRetry = incompleteReason == nil || incompleteReason == .pageFailed
        let canContinue = hasMore
            && nextCursor != nil
            && loadingPhase == .idle
            && failureAllowsRetry
        let canRetryInitialPage = incompleteReason == .pageFailed
            && items.isEmpty
            && sourceRevision == nil
            && totalCount == nil
            && sourceCompleteness == .unknown
            && loadingPhase == .idle
        return ListCompletenessState(
            loadedCount: items.count,
            totalCount: totalCount,
            hasMore: hasMore,
            isComplete: isComplete,
            completeness: completeness,
            incompleteReason: incompleteReason,
            loadingPhase: loadingPhase,
            canLoadMore: canContinue,
            canLoadAll: canContinue || canRetryInitialPage
        )
    }
}
