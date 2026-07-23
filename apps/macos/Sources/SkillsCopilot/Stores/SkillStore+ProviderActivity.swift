import Foundation

@MainActor
final class ProviderActivityController {
    private static let pageLimit = 50

    private let service: ServiceClient
    private var accumulators: [ProviderActivityFilterKey: ListPageAccumulator<ProviderActivityRow>] = [:]
    private var generations: [ProviderActivityFilterKey: UInt64] = [:]
    private var activeFilterKey: ProviderActivityFilterKey?
    private var pageTask: Task<ProviderActivityPageResult, Error>?
    private var pageRequestID: UUID?
    private var changeHandler: (() -> Void)?

    private(set) var rows: [ProviderActivityRow] = []
    private(set) var completeness = ListPageAccumulator<ProviderActivityRow>().state
    private(set) var errorMessage: String?

    init(service: ServiceClient) {
        self.service = service
    }

    func setChangeHandler(_ handler: @escaping () -> Void) {
        changeHandler = handler
    }

    func loadMore(loadAll: Bool) async {
        guard let key = activeFilterKey,
              pageTask == nil,
              var accumulator = accumulators[key] else { return }
        let state = accumulator.state
        guard loadAll ? state.canLoadAll : state.canLoadMore else { return }
        let generation = generations[key] ?? 0
        accumulator.begin(loadAll ? .all : .more)
        accumulators[key] = accumulator
        publish(for: key)

        repeat {
            let accepted = await requestAndAppendPage(for: key, generation: generation)
            guard accepted,
                  loadAll,
                  generations[key] == generation,
                  accumulators[key]?.state.hasMore == true else {
                break
            }
        } while !Task.isCancelled
    }

    func cancelLoadAll() {
        guard let key = activeFilterKey,
              var accumulator = accumulators[key],
              accumulator.state.loadingPhase == .all else { return }
        generations[key, default: 0] &+= 1
        cancelActiveRequest()
        accumulator.cancel()
        accumulators[key] = accumulator
        publish(for: key)
    }

    func cancelActiveRequest() {
        pageTask?.cancel()
        pageTask = nil
        pageRequestID = nil
    }

    func beginRefresh(for key: ProviderActivityFilterKey) -> UInt64 {
        if let activeKey = activeFilterKey {
            generations[activeKey, default: 0] &+= 1
        }
        cancelActiveRequest()
        activeFilterKey = key
        generations[key, default: 0] &+= 1
        let generation = generations[key] ?? 0
        var accumulator = ListPageAccumulator<ProviderActivityRow>()
        accumulator.begin(.initial)
        accumulators[key] = accumulator
        errorMessage = nil
        publish(for: key)
        return generation
    }

    func loadInitial(for key: ProviderActivityFilterKey, generation: UInt64) async {
        _ = await requestAndAppendPage(for: key, generation: generation)
    }

    func fail(_ error: Error, for key: ProviderActivityFilterKey, generation: UInt64) {
        guard generations[key] == generation,
              activeFilterKey == key,
              var accumulator = accumulators[key] else { return }
        accumulator.fail(reason: Self.incompleteReason(for: error))
        accumulators[key] = accumulator
        errorMessage = error.localizedDescription
        publish(for: key)
    }

    private func requestAndAppendPage(
        for key: ProviderActivityFilterKey,
        generation: UInt64
    ) async -> Bool {
        guard generations[key] == generation,
              let accumulator = accumulators[key] else { return false }
        let requestID = UUID()
        let task = Task {
            try await service.listProviderActivity(
                provider: key.provider,
                model: key.model,
                action: key.action,
                windowDays: key.windowDays,
                startAt: key.startAt,
                endAt: key.endAt,
                limit: Self.pageLimit,
                cursor: accumulator.nextCursor,
                sourceRevision: accumulator.sourceRevision
            )
        }
        pageRequestID = requestID
        pageTask = task

        do {
            let page = try await task.value
            clearPageTask(requestID: requestID)
            guard generations[key] == generation,
                  activeFilterKey == key,
                  var current = accumulators[key] else { return false }
            try current.append(page.page)
            accumulators[key] = current
            errorMessage = nil
            publish(for: key)
            return true
        } catch {
            clearPageTask(requestID: requestID)
            guard generations[key] == generation,
                  activeFilterKey == key else { return false }
            fail(error, for: key, generation: generation)
            return false
        }
    }

    private func clearPageTask(requestID: UUID) {
        guard pageRequestID == requestID else { return }
        pageRequestID = nil
        pageTask = nil
    }

    private func publish(for key: ProviderActivityFilterKey) {
        guard activeFilterKey == key, let accumulator = accumulators[key] else { return }
        rows = accumulator.items
        completeness = accumulator.state
        changeHandler?()
    }

    private static func incompleteReason(for error: Error) -> ListIncompleteReason {
        guard case ServiceClient.ClientError.service(let serviceError) = error else {
            return .pageFailed
        }
        switch serviceError.code {
        case "source_changed":
            return .sourceChanged
        default:
            return .pageFailed
        }
    }
}

@MainActor
extension SkillStore {
    var providerActivityRows: [ProviderActivityRow] {
        providerActivityController.rows
    }

    var providerActivityCompleteness: ListCompletenessState {
        providerActivityController.completeness
    }

    var providerActivityErrorMessage: String? {
        providerActivityController.errorMessage
    }

    func loadMoreProviderActivity(loadAll: Bool) async {
        await providerActivityController.loadMore(loadAll: loadAll)
    }

    func cancelProviderActivityLoadAll() {
        providerActivityController.cancelLoadAll()
    }
}
