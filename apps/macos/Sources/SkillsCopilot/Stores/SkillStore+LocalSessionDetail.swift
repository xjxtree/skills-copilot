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
                let mergedDetail = detail.replacingContentItems(
                    accumulator.items,
                    countsComplete: !page.hasMore
                )
                guard localSessionCache.publishDetailProgress(
                    mergedDetail,
                    completeness: accumulator.state,
                    key: key,
                    generation: generation
                ) else { return }
                if let reconciled = localSessionCache.successfulSnapshot(for: source),
                   reconciled.result != localSessionPreviewResult {
                    publishReconciledLocalSessionSummary(reconciled)
                } else if selectedLocalSessionID == sessionID {
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

    private func publishReconciledLocalSessionSummary(_ snapshot: LocalSessionSnapshot) {
        guard activeLocalSessionSnapshotKey == snapshot.key else { return }
        localSessionLoadState = localSessionCache.summaryStates[snapshot.key] ?? .fresh(snapshot)
        localSessionPreviewResult = snapshot.result
        synchronizeSelectedLocalSessionDetailState()
    }

}
