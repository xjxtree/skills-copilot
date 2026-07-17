import Foundation

extension SkillStore {
    func loadLocalSessionDetailIfNeeded(sessionID: String) async {
        guard let source = activeLocalSessionSnapshotKey,
              let snapshot = localSessionCache.successfulSnapshot(for: source),
              snapshot.result.sessionRows.contains(where: { $0.id == sessionID }) else { return }
        let key = LocalSessionDetailKey(source: source, sessionID: sessionID)
        guard let generation = localSessionCache.beginDetailLoad(for: key) else {
            synchronizeSelectedLocalSessionDetailState()
            return
        }
        synchronizeSelectedLocalSessionDetailState()
        let agent = source.agent == SkillAgentFilter.all.rawValue ? nil : source.agent
        let project = activeProjectContext
        do {
            let result = try await service.previewLocalSessions(
                authorizedRoots: source.authorizedRoots,
                agent: agent,
                scope: .all,
                search: nil,
                project: project,
                sessionID: sessionID,
                includeContentItems: true,
                limit: 1,
                offset: 0,
                sort: .recent,
                direction: .descending
            )
            guard activeLocalSessionSnapshotKey == source else { return }
            guard let detail = result.sessionRows.first(where: { $0.id == sessionID }),
                  detail.contentIncluded else {
                localSessionCache.failDetail(
                    key: key,
                    generation: generation,
                    displayError: "Session detail was unavailable. Retry to load it again."
                )
                if selectedLocalSessionID == sessionID {
                    synchronizeSelectedLocalSessionDetailState()
                }
                return
            }
            let sampledProcessItems = detail.contentItems.filter {
                !matchesFinalSessionMessage($0.kind)
            }
            var accumulator = ListPageAccumulator<LocalSessionContentItem>()
            accumulator.begin(.all)
            var cursor: String?
            var sourceRevision: String?
            repeat {
                try Task.checkCancellation()
                let page = try await service.listLocalSessionMessages(
                    authorizedRoots: source.authorizedRoots,
                    agent: agent,
                    project: project,
                    sessionID: sessionID,
                    limit: 40,
                    cursor: cursor,
                    sourceRevision: sourceRevision
                )
                guard page.sessionID == sessionID else {
                    throw ListPageAccumulatorError.invalidPage
                }
                let previousCursor = cursor
                try accumulator.append(page.listPage)
                guard !page.hasMore || page.nextCursor != previousCursor else {
                    throw ListPageAccumulatorError.invalidPage
                }
                cursor = page.nextCursor
                sourceRevision = page.sourceRevision
                let finalMessages = accumulator.items
                let mergedItems = mergeLocalSessionDetailItems(
                    finalMessages: finalMessages,
                    sampledProcessItems: sampledProcessItems
                )
                let mergedDetail = detail.replacingContentItems(
                    mergedItems,
                    exactFinalMessages: finalMessages
                )
                guard localSessionCache.publishDetailProgress(
                    mergedDetail,
                    completeness: accumulator.state,
                    key: key,
                    generation: generation
                ) else { return }
                if selectedLocalSessionID == sessionID {
                    synchronizeSelectedLocalSessionDetailState()
                }
                guard page.hasMore else { break }
                await Task.yield()
            } while true
        } catch {
            if Task.isCancelled {
                localSessionCache.cancelDetailLoad(key: key)
                if activeLocalSessionSnapshotKey == source,
                   selectedLocalSessionID == sessionID {
                    synchronizeSelectedLocalSessionDetailState()
                }
                return
            }
            localSessionCache.failDetail(
                key: key,
                generation: generation,
                displayError: error.localizedDescription,
                reason: listFailureReason(for: error)
            )
            if activeLocalSessionSnapshotKey == source, selectedLocalSessionID == sessionID {
                synchronizeSelectedLocalSessionDetailState()
            }
        }
    }

    private func matchesFinalSessionMessage(_ kind: LocalSessionContentKind) -> Bool {
        kind == .userMessage || kind == .agentReply
    }

    private func mergeLocalSessionDetailItems(
        finalMessages: [LocalSessionContentItem],
        sampledProcessItems: [LocalSessionContentItem]
    ) -> [LocalSessionContentItem] {
        (finalMessages + sampledProcessItems)
            .enumerated()
            .sorted { left, right in
                switch (left.element.timestamp, right.element.timestamp) {
                case let (leftTime?, rightTime?) where leftTime != rightTime:
                    return leftTime < rightTime
                case (nil, .some):
                    return false
                case (.some, nil):
                    return true
                default:
                    return left.offset < right.offset
                }
            }
            .map(\.element)
    }
}
