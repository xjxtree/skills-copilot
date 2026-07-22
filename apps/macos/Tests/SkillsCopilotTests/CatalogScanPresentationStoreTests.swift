@testable import SkillsCopilot

@MainActor
extension SkillStoreTests {
    func catalogScanCompletenessTracksExplicitScan() async throws {
        let runner = CatalogRefreshServiceRunner(scanFixtures: [.completeWithSafeDiagnostics, .budget])
        let store = SkillStore(service: runner.serviceClient())

        await store.loadAppStartupDataIfNeeded()
        try expectEqual(store.catalogListCompleteness.completeness, .unknown, "Catalog-only startup cannot prove scan completeness")
        await store.scanAll()
        try expectEqual(store.catalogListCompleteness.completeness, .complete, "Complete scan should prove catalog completeness")
        try expectNil(store.partialScanWarningMessage, "Claude must not inherit a safe opencode dangling-link diagnostic")
        store.agentFilter = .opencode
        try expectEqual(store.filteredCatalogListCompleteness.completeness, .complete, "A dangling link does not degrade the surrounding opencode root")
        try expectContains(store.partialScanWarningMessage ?? "", "$HOME/.config/opencode/skills/removed", "opencode should retain its own actionable dangling-link warning")
        store.agentFilter = .claudeCode
        await store.scanAll()
        try expectEqual(store.catalogListCompleteness.completeness, .incomplete, "Budgeted scan must be visible as incomplete")
        try expectEqual(store.catalogListCompleteness.incompleteReason, .safetyBudget, "Budget reason")

        let historyRunner = CatalogRefreshServiceRunner(scanFixtures: [.completeWithDeletedHistory])
        let historyStore = SkillStore(service: historyRunner.serviceClient())
        await historyStore.scanAll()
        try expectEqual(historyStore.catalogListCompleteness.loadedCount, 3, "Aggregate completeness count excludes Deleted history")
        try expectEqual(historyStore.catalogListCompleteness.totalCount, 3, "Exact aggregate total counts current skills only")
        historyStore.agentFilter = .codex
        try expectEqual(historyStore.filteredCatalogListCompleteness.loadedCount, 1, "Agent completeness count excludes that agent's Deleted history")
        try expectEqual(historyStore.filteredCatalogListCompleteness.totalCount, 1, "Exact agent total counts current skills only")
    }

    func catalogScanPresentationFollowsSelectedAgent() async throws {
        let runner = CatalogRefreshServiceRunner(scanFixtures: [.partial])
        let store = SkillStore(service: runner.serviceClient())

        await store.scanAll()
        try expectEqual(store.agentFilter, .claudeCode, "Fixture starts on Claude Code")
        try expectEqual(store.filteredCatalogListCompleteness.loadedCount, 2, "Claude catalog count")
        try expectEqual(store.filteredCatalogListCompleteness.completeness, .incomplete, "Claude partial scan")
        try expectEqual(store.filteredCatalogListCompleteness.incompleteReason, .unreadableSource, "Claude traversal failure reason")
        try expectContains(store.partialScanWarningMessage ?? "", "Claude Code", "Claude warning is scoped to Claude")
        try expectContains(store.partialScanWarningMessage ?? "", "entry_unreadable", "Claude warning prioritizes the traversal issue")

        store.agentFilter = .codex
        try expectEqual(store.filteredCatalogListCompleteness.loadedCount, 1, "Codex catalog count")
        try expectEqual(store.filteredCatalogListCompleteness.completeness, .complete, "Codex scan is complete")
        try expectNil(store.partialScanWarningMessage, "Codex must not inherit another agent's warning")

        store.agentFilter = .opencode
        try expectEqual(store.filteredCatalogListCompleteness.loadedCount, 0, "opencode catalog count")
        try expectEqual(store.filteredCatalogListCompleteness.completeness, .incomplete, "Skipped explicit opencode root is incomplete")
        try expectEqual(store.filteredCatalogListCompleteness.incompleteReason, .sourceLimited, "A missing root is unavailable, not unreadable")
        try expectContains(store.partialScanWarningMessage ?? "", "completed-with-skipped-roots", "opencode warning keeps its typed status")
        try expectContains(store.partialScanWarningMessage ?? "", "root_unavailable", "opencode warning identifies the unavailable root")

        store.agentFilter = .all
        try expectEqual(store.filteredCatalogListCompleteness, store.catalogListCompleteness, "All agents use the aggregate scan state")
        try expectEqual(store.filteredCatalogListCompleteness.incompleteReason, .unreadableSource, "Aggregate reason preserves the strongest failure")
    }
}
