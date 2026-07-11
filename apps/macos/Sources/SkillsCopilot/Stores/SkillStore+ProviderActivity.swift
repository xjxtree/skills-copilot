import Foundation

@MainActor
extension SkillStore {
    func loadMoreProviderActivity(loadAll: Bool) async {
        guard let key = activeProviderActivityFilterKey,
              providerActivityPageTask == nil,
              var accumulator = providerActivityAccumulators[key] else { return }
        let state = accumulator.state
        guard loadAll ? state.canLoadAll : state.canLoadMore else { return }
        let generation = providerActivityGenerations[key] ?? 0
        accumulator.begin(loadAll ? .all : .more)
        providerActivityAccumulators[key] = accumulator
        publishProviderActivity(for: key)

        repeat {
            let accepted = await requestAndAppendProviderActivityPage(
                for: key,
                generation: generation
            )
            guard accepted,
                  loadAll,
                  providerActivityGenerations[key] == generation,
                  providerActivityAccumulators[key]?.state.hasMore == true else {
                break
            }
        } while !Task.isCancelled
    }

    func cancelProviderActivityLoadAll() {
        guard let key = activeProviderActivityFilterKey,
              var accumulator = providerActivityAccumulators[key],
              accumulator.state.loadingPhase == .all else { return }
        providerActivityGenerations[key, default: 0] &+= 1
        providerActivityPageTask?.cancel()
        providerActivityPageTask = nil
        providerActivityPageRequestID = nil
        accumulator.cancel()
        providerActivityAccumulators[key] = accumulator
        publishProviderActivity(for: key)
    }

    func beginProviderActivityRefresh(for key: ProviderActivityFilterKey) -> UInt64 {
        if let activeKey = activeProviderActivityFilterKey {
            providerActivityGenerations[activeKey, default: 0] &+= 1
        }
        providerActivityPageTask?.cancel()
        providerActivityPageTask = nil
        providerActivityPageRequestID = nil
        activeProviderActivityFilterKey = key
        providerActivityGenerations[key, default: 0] &+= 1
        let generation = providerActivityGenerations[key] ?? 0
        var accumulator = ListPageAccumulator<ProviderActivityRow>()
        accumulator.begin(.initial)
        providerActivityAccumulators[key] = accumulator
        providerActivityErrorMessage = nil
        publishProviderActivity(for: key)
        return generation
    }

    func loadInitialProviderActivity(
        for key: ProviderActivityFilterKey,
        generation: UInt64
    ) async {
        _ = await requestAndAppendProviderActivityPage(
            for: key,
            generation: generation
        )
    }

    private func requestAndAppendProviderActivityPage(
        for key: ProviderActivityFilterKey,
        generation: UInt64
    ) async -> Bool {
        guard providerActivityGenerations[key] == generation,
              let accumulator = providerActivityAccumulators[key] else { return false }
        let requestID = UUID()
        let service = service
        let task = Task {
            try await service.listProviderActivity(
                provider: key.provider,
                model: key.model,
                action: key.action,
                windowDays: key.windowDays,
                startAt: key.startAt,
                endAt: key.endAt,
                limit: Self.providerActivityPageLimit,
                cursor: accumulator.nextCursor,
                sourceRevision: accumulator.sourceRevision
            )
        }
        providerActivityPageRequestID = requestID
        providerActivityPageTask = task

        do {
            let page = try await task.value
            clearProviderActivityPageTask(requestID: requestID)
            guard providerActivityGenerations[key] == generation,
                  activeProviderActivityFilterKey == key,
                  var current = providerActivityAccumulators[key] else { return false }
            try current.append(page.page)
            providerActivityAccumulators[key] = current
            providerActivityErrorMessage = nil
            publishProviderActivity(for: key)
            return true
        } catch {
            clearProviderActivityPageTask(requestID: requestID)
            guard providerActivityGenerations[key] == generation,
                  activeProviderActivityFilterKey == key else { return false }
            failProviderActivity(error, for: key, generation: generation)
            return false
        }
    }

    private func clearProviderActivityPageTask(requestID: UUID) {
        guard providerActivityPageRequestID == requestID else { return }
        providerActivityPageRequestID = nil
        providerActivityPageTask = nil
    }

    func failProviderActivity(
        _ error: Error,
        for key: ProviderActivityFilterKey,
        generation: UInt64
    ) {
        guard providerActivityGenerations[key] == generation,
              activeProviderActivityFilterKey == key,
              var accumulator = providerActivityAccumulators[key] else { return }
        let reason: ListIncompleteReason
        if case ServiceClient.ClientError.service(let serviceError) = error {
            switch serviceError.code {
            case "source_changed":
                reason = .sourceChanged
            case "unknown_method":
                reason = .unsupportedProtocol
            default:
                reason = .pageFailed
            }
        } else {
            reason = .pageFailed
        }
        accumulator.fail(reason: reason)
        providerActivityAccumulators[key] = accumulator
        providerActivityErrorMessage = error.localizedDescription
        publishProviderActivity(for: key)
    }

    private func publishProviderActivity(for key: ProviderActivityFilterKey) {
        guard activeProviderActivityFilterKey == key,
              let accumulator = providerActivityAccumulators[key] else { return }
        providerActivityRows = accumulator.items
        providerActivityCompleteness = accumulator.state
    }
}
