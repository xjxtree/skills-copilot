import Foundation
@testable import SkillsCopilot

@MainActor
struct LocalSessionCacheTests {
    func run() throws {
        try eightHundredSummariesRetainNoContentItems()
        try criteriaDoNotChangeSourceKey()
        try projectionFiltersAndSortsSummariesLocally()
        try refreshingKeepsPreviousSummariesVisible()
        try failedRefreshWithDataBecomesStale()
        try failedDetailDoesNotChangeSummaryList()
        try progressiveDetailRetainsAcceptedMessagesOnCancelAndRetry()
        try detailCacheIsBoundedAndSourceScoped()
        try oldSummaryAndDetailGenerationsAreIgnored()
        try cursorMetadataIsTransientAndCancellationRejectsLatePages()
        try globalSearchIndexesSummariesOnly()
        try globalSearchLimitsShortcutsButPreservesExactKindCounts()
        try globalSearchSkillMatchesCanonicalSkillPredicate()
    }

    private let source = LocalSessionSnapshotKey(
        agent: "codex",
        projectRoot: "/project",
        currentCWD: "/project/subdir",
        authorizedRoots: ["/sessions-b", "/sessions-a", "/sessions-a"]
    )

    private func row(
        id: String,
        title: String? = nil,
        scope: String = "all",
        projectRoot: String? = "/project",
        endedAt: Int64? = nil,
        content: Bool = true
    ) -> LocalSessionPreviewRow {
        let item = LocalSessionContentItem(
            id: "item-\(id)",
            kind: .agentReply,
            title: "Agent",
            text: "RAW_DETAIL_\(id)",
            charCount: 12,
            timestamp: endedAt,
            evidenceRefs: []
        )
        return LocalSessionPreviewRow(
            id: id,
            title: title ?? id,
            sourceKind: "authorized-local-session",
            scope: scope,
            agent: "codex",
            projectRoot: projectRoot,
            redactedPath: "$HOME/\(id).jsonl",
            modifiedAt: endedAt.map(String.init),
            startedAt: endedAt,
            endedAt: endedAt,
            excerpt: "Excerpt \(id)",
            excerptCharCount: id.count + 8,
            userMessageCount: 1,
            totalMessageCount: 2,
            toolCallCount: 0,
            skillCallCount: 0,
            contentHash: "hash-\(id)",
            evidenceRefs: [],
            contentIncluded: content,
            contentItems: content ? [item] : []
        )
    }

    private func publish(
        _ rows: [LocalSessionPreviewRow],
        to cache: LocalSessionCache,
        key: LocalSessionSnapshotKey? = nil
    ) {
        let key = key ?? source
        let generation = cache.beginSummaryRefresh(for: key)
        cache.publishSummary(LocalSessionSnapshot(
            key: key,
            generation: generation,
            result: LocalSessionPreviewResult(
                authorized: true,
                sessionRows: rows,
                totalCandidateCount: rows.count,
                totalMatchedCount: rows.count
            ),
            refreshedAt: Date(timeIntervalSince1970: 1),
            isComplete: true
        ))
    }

    private func eightHundredSummariesRetainNoContentItems() throws {
        let cache = LocalSessionCache()
        publish((0..<800).map { row(id: "session-\($0)") }, to: cache)
        let rows = cache.successfulSnapshot(for: source)?.result.sessionRows ?? []
        try expectEqual(rows.count, 800, "Summary cache should retain the complete 800-row source snapshot.")
        try expectFalse(rows.contains { !$0.contentItems.isEmpty || $0.contentIncluded }, "Summary cache must strip every detail item defensively.")
    }

    private func criteriaDoNotChangeSourceKey() throws {
        let reordered = LocalSessionSnapshotKey(
            agent: "codex",
            projectRoot: "/project",
            currentCWD: "/project/subdir",
            authorizedRoots: ["/sessions-a", "/sessions-b"]
        )
        try expectEqual(source, reordered, "Scope/search/sort criteria must not participate in the source cache key.")
    }

    private func projectionFiltersAndSortsSummariesLocally() throws {
        let cache = LocalSessionCache()
        publish([
            row(id: "bravo", title: "Bravo", scope: "project", endedAt: 30),
            row(id: "alpha", title: "Alpha", scope: "project", endedAt: 20),
            row(id: "outside", title: "Alpha outside", scope: "project", projectRoot: "/other", endedAt: 40),
        ], to: cache)
        let rows = cache.projectedRows(for: source, criteria: LocalSessionProjectionCriteria(
            scope: .project,
            search: "a",
            sort: .title,
            direction: .ascending,
            projectRoot: "/project"
        ))
        try expectEqual(rows.map(\.id), ["alpha", "bravo"], "Projection should filter and sort the summary snapshot without replacing its source.")
    }

    private func refreshingKeepsPreviousSummariesVisible() throws {
        let cache = LocalSessionCache()
        publish([row(id: "visible")], to: cache)
        _ = cache.beginSummaryRefresh(for: source)
        guard case .refreshing(let snapshot) = cache.summaryStates[source] else {
            throw NativeModelTestFailure(description: "Refreshing should retain the successful source snapshot.")
        }
        try expectEqual(snapshot.result.sessionRows.map(\.id), ["visible"], "Refresh should keep previous summaries visible.")
    }

    private func failedRefreshWithDataBecomesStale() throws {
        let cache = LocalSessionCache()
        publish([row(id: "stale-visible")], to: cache)
        let generation = cache.beginSummaryRefresh(for: source)
        cache.failSummary(key: source, generation: generation, displayError: "refresh failed")
        guard case .stale(let snapshot, let displayError) = cache.summaryStates[source] else {
            throw NativeModelTestFailure(description: "A failed refresh with data should become stale.")
        }
        try expectEqual(snapshot.result.sessionRows.map(\.id), ["stale-visible"], "Stale state should preserve the last successful rows.")
        try expectEqual(displayError, "refresh failed", "Stale state should retain its display error.")
    }

    private func failedDetailDoesNotChangeSummaryList() throws {
        let cache = LocalSessionCache()
        publish([row(id: "summary")], to: cache)
        let key = LocalSessionDetailKey(source: source, sessionID: "summary")
        let generation = try required(cache.beginDetailLoad(for: key), "Detail load should begin.")
        cache.failDetail(key: key, generation: generation, displayError: "detail failed")
        try expectEqual(cache.successfulSnapshot(for: source)?.result.sessionRows.map(\.id), ["summary"], "Detail failure must not mutate summary rows.")
        guard case .failed(let displayError) = cache.detailStates[key] else {
            throw NativeModelTestFailure(description: "Detail failure should remain detail-local.")
        }
        try expectEqual(displayError, "detail failed", "Detail failure should retain its display error.")
    }

    private func progressiveDetailRetainsAcceptedMessagesOnCancelAndRetry() throws {
        let cache = LocalSessionCache()
        publish([row(id: "large")], to: cache)
        let key = LocalSessionDetailKey(source: source, sessionID: "large")
        let generation = try required(cache.beginDetailLoad(for: key), "Progressive detail load should begin.")
        let progress = ListCompletenessState(
            loadedCount: 40,
            totalCount: nil,
            hasMore: true,
            isComplete: false,
            completeness: .partial,
            incompleteReason: nil,
            loadingPhase: .all,
            canLoadMore: false,
            canLoadAll: false
        )
        try expectFalse(!cache.publishDetailProgress(row(id: "large"), completeness: progress, key: key, generation: generation), "A valid progressive page should publish.")
        cache.cancelDetailLoad(key: key)
        guard case .loaded(let retained) = cache.detailStates[key] else {
            throw NativeModelTestFailure(description: "Cancellation should retain accepted detail messages.")
        }
        try expectEqual(retained.id, "large", "Cancellation should retain the selected detail row.")
        try expectEqual(cache.detailCompleteness[key]?.loadedCount, 40, "Cancellation should retain the accepted message count.")
        try expectEqual(cache.detailCompleteness[key]?.canLoadAll, true, "Cancellation should make the incomplete detail retryable.")
        try expectFalse(cache.beginDetailLoad(for: key) == nil, "An incomplete loaded detail should allow retry.")
    }

    private func detailCacheIsBoundedAndSourceScoped() throws {
        let cache = LocalSessionCache()
        for index in 0...LocalSessionCache.maximumDetailEntries {
            let key = LocalSessionDetailKey(source: source, sessionID: "detail-\(index)")
            let generation = try required(cache.beginDetailLoad(for: key), "Detail load should begin.")
            _ = cache.publishDetail(row(id: key.sessionID), key: key, generation: generation)
        }
        try expectEqual(cache.detailStates.count, LocalSessionCache.maximumDetailEntries, "Detail cache should enforce its fixed bound.")
        try expectNil(cache.detailStates[LocalSessionDetailKey(source: source, sessionID: "detail-0")], "Least-recently-used detail should be evicted.")

        let other = LocalSessionSnapshotKey(agent: "claude-code", projectRoot: nil, currentCWD: nil, authorizedRoots: [])
        cache.activateSource(other)
        try expectFalse(cache.detailStates.keys.contains { $0.source == source }, "Activating a source should release details owned by another source.")
    }

    private func oldSummaryAndDetailGenerationsAreIgnored() throws {
        let cache = LocalSessionCache()
        let oldSummaryGeneration = cache.beginSummaryRefresh(for: source)
        let currentSummaryGeneration = cache.beginSummaryRefresh(for: source)
        cache.publishSummary(LocalSessionSnapshot(
            key: source,
            generation: oldSummaryGeneration,
            result: LocalSessionPreviewResult(sessionRows: [row(id: "old")]),
            refreshedAt: Date(),
            isComplete: true
        ))
        cache.publishSummary(LocalSessionSnapshot(
            key: source,
            generation: currentSummaryGeneration,
            result: LocalSessionPreviewResult(sessionRows: [row(id: "current")]),
            refreshedAt: Date(),
            isComplete: true
        ))
        try expectEqual(cache.successfulSnapshot(for: source)?.result.sessionRows.map(\.id), ["current"], "Late summary generations should be ignored.")

        let detailKey = LocalSessionDetailKey(source: source, sessionID: "detail")
        let oldDetailGeneration = try required(cache.beginDetailLoad(for: detailKey), "Old detail load should begin.")
        let other = LocalSessionSnapshotKey(agent: "pi", projectRoot: nil, currentCWD: nil, authorizedRoots: [])
        cache.activateSource(other)
        cache.activateSource(source)
        let currentDetailGeneration = try required(cache.beginDetailLoad(for: detailKey), "Current detail load should begin.")
        let oldAccepted = cache.publishDetail(
            row(id: "detail", title: "Old detail generation"),
            key: detailKey,
            generation: oldDetailGeneration
        )
        let currentAccepted = cache.publishDetail(
            row(id: "detail", title: "Current detail generation"),
            key: detailKey,
            generation: currentDetailGeneration
        )
        guard case .loaded(let detail) = cache.detailStates[detailKey] else {
            throw NativeModelTestFailure(description: "Current detail generation should load.")
        }
        try expectEqual(detail.id, "detail", "Late detail generations should be ignored.")
        try expectEqual(detail.title, "Current detail generation", "Generation, rather than row-id validation, should reject the late detail.")
        try expectFalse(oldAccepted, "A stale detail generation should report rejection.")
        try expectFalse(!currentAccepted, "The active detail generation should report acceptance.")
    }

    private func globalSearchIndexesSummariesOnly() throws {
        let index = AppSearchIndex(
            skills: [],
            sessionSummaries: [row(id: "session-search", title: "Searchable session")],
            configSnapshots: []
        )
        let result = index.search(query: "Searchable", limitPerKind: 6)
        try expectEqual(result.items.map(\.targetID), ["session-search"], "Global search should index session summary fields.")
        try expectFalse(result.items.contains { !($0.session?.contentItems.isEmpty ?? true) }, "Global search results must contain summary-only sessions.")
        try expectEqual(index.search(query: "RAW_DETAIL", limitPerKind: 6).items.count, 0, "Global search must not inspect detail content items.")
    }

    private func globalSearchLimitsShortcutsButPreservesExactKindCounts() throws {
        let skills = (0..<20).map { index in
            SkillRecord(
                id: "skill-\(index)",
                agent: "codex",
                scope: "agent-project",
                path: "/project/skills/match-\(index)/SKILL.md",
                displayPath: "$PROJECT/skills/match-\(index)/SKILL.md",
                definitionId: "definition.match.\(index)",
                name: "Match skill \(index)",
                state: "loaded",
                enabled: true
            )
        }
        let sessions = (0..<20).map { index in
            row(id: "session-match-\(index)", title: "Match session \(index)")
        }
        let snapshots = (0..<20).map { index in
            ConfigSnapshotRecord(
                id: "snapshot-\(index)",
                agent: "codex",
                scope: "agent-project",
                target: "$PROJECT/config-\(index).json",
                content: "{}",
                reason: "Match config \(index)",
                createdAt: Int64(index)
            )
        }
        let result = AppSearchIndex(
            skills: skills,
            sessionSummaries: sessions,
            configSnapshots: snapshots
        ).search(query: "match", limitPerKind: 6)

        for kind in AppSearchItemKind.allCases {
            try expectEqual(
                result.items.filter { $0.kind == kind }.count,
                6,
                "Global search should expose six shortcuts for \(kind)."
            )
            try expectEqual(
                result.count(for: kind),
                20,
                "Global search should preserve the exact full count for \(kind)."
            )
        }
        try expectEqual(result.totalMatchedCount, 60, "Global search should preserve the exact aggregate match count.")
    }

    private func globalSearchSkillMatchesCanonicalSkillPredicate() throws {
        let skills = [
            SkillRecord(
                id: "scope-only",
                agent: "claude-code",
                scope: "match-scope-only",
                path: "/project/skills/scope/SKILL.md",
                displayPath: "$PROJECT/skills/scope/SKILL.md",
                definitionId: "definition.scope",
                name: "Scope Record",
                state: "loaded",
                enabled: true
            ),
            SkillRecord(
                id: "definition-only",
                agent: "codex",
                scope: "agent-global",
                path: "/project/skills/definition/SKILL.md",
                displayPath: "$PROJECT/skills/definition/SKILL.md",
                definitionId: "match-definition-only",
                name: "Definition Record",
                state: "loaded",
                enabled: true
            ),
            SkillRecord(
                id: "path-only",
                agent: "opencode",
                scope: "agent-global",
                path: "/project/skills/path/SKILL.md",
                displayPath: "$PROJECT/match-path-only/SKILL.md",
                definitionId: "definition.path",
                name: "Path Record",
                state: "loaded",
                enabled: true
            ),
            SkillRecord(
                id: "alias-only",
                agent: "opencode",
                scope: "agent-global",
                path: "/project/skills/alias/SKILL.md",
                displayPath: "$PROJECT/skills/alias/SKILL.md",
                definitionId: "definition.alias",
                name: "Alias Record",
                state: "loaded",
                enabled: true
            ),
        ]

        for query in ["match-scope-only", "match-definition-only", "match-path-only", "open code"] {
            let overlayIDs = Set(AppSearchIndex(
                skills: skills,
                sessionSummaries: [],
                configSnapshots: []
            ).search(query: query, limitPerKind: 6).items.map(\.targetID))
            let canonicalIDs = Set(SkillListModel.filteredAndSorted(
                skills: skills,
                findings: [],
                conflicts: [],
                searchText: query,
                agentFilter: .all,
                stateFilter: .all,
                scopeFilter: .all,
                sortOrder: .name
            ).map(\.id))
            try expectEqual(overlayIDs, canonicalIDs, "Global search and canonical Skills should return the same set for \(query).")
        }
    }

    private func cursorMetadataIsTransientAndCancellationRejectsLatePages() throws {
        let cache = LocalSessionCache()
        let generation = cache.beginSummaryRefresh(for: source)
        let accepted = LocalSessionSnapshot(
            key: source,
            generation: generation,
            result: LocalSessionPreviewResult(
                authorized: true,
                sessionRows: [row(id: "accepted")],
                totalCandidateCount: 2,
                totalMatchedCount: 2,
                hasMore: true,
                nextCursor: "v1:opaque",
                sourceRevision: "sha256:revision"
            ),
            refreshedAt: Date(),
            isComplete: false,
            nextCursor: "v1:opaque",
            sourceRevision: "sha256:revision"
        )
        cache.publishSummary(accepted)
        try expectEqual(cache.successfulSnapshot(for: source)?.nextCursor, "v1:opaque", "The transient snapshot should retain its continuation cursor.")
        cache.cancelSummaryLoad(key: source, generation: generation)
        cache.publishSummary(LocalSessionSnapshot(
            key: source,
            generation: generation,
            result: accepted.result.mergingPage(LocalSessionPreviewResult(sessionRows: [row(id: "late")])),
            refreshedAt: Date(),
            isComplete: true
        ))
        try expectEqual(cache.successfulSnapshot(for: source)?.result.sessionRows.map(\.id), ["accepted"], "Cancellation must invalidate a late page generation while retaining accepted rows.")
    }

    private func required<T>(_ value: T?, _ message: String) throws -> T {
        guard let value else { throw NativeModelTestFailure(description: message) }
        return value
    }
}
