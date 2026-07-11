import Foundation
@testable import SkillsCopilot

@MainActor
extension SkillStoreTests {
    func appSearchViewAllRoutesCanonicallyWithoutRPC() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()

        let callsBeforeRouting = fake.calls()
        await store.showAllAppSearchResults(kind: .skill, query: "match")
        try expectEqual(store.sidebarContentMode, .skills, "View All Skills should route to the canonical Skills list.")
        try expectEqual(store.searchText, "match", "View All Skills should transfer the query.")
        try expectEqual(store.skillScopeFilter, .all, "View All Skills should include every indexed skill scope.")
        try expectFalse(!(store.selectedSidebarSelection == nil || store.selectedSidebarSelection?.isSkill == true), "View All Skills should normalize selection to the canonical list.")

        await store.showAllAppSearchResults(kind: .session, query: "match")
        try expectEqual(store.sidebarContentMode, .sessions, "View All Sessions should route to the canonical Sessions list.")
        try expectEqual(store.localSessionSearchText, "match", "View All Sessions should transfer the query.")
        try expectEqual(store.localSessionScopeFilter, .all, "View All Sessions should include every indexed session scope.")
        try expectFalse(!(store.selectedSidebarSelection == nil || store.selectedSidebarSelection?.isSession == true), "View All Sessions should normalize selection to the canonical list.")

        await store.showAllAppSearchResults(kind: .configHistory, query: "match")
        try expectEqual(store.sidebarContentMode, .config, "View All Config History should route to the canonical Config list.")
        try expectEqual(store.configSidebarSearchText, "match", "View All Config History should transfer the query.")
        try expectEqual(store.configScopeFilter, .all, "View All Config History should include every indexed config scope.")
        try expectFalse(store.selectedSidebarSelection?.isConfig != true, "View All Config History should normalize selection to the canonical Config list.")

        try expectEqual(fake.calls(), callsBeforeRouting, "View All routing must not issue service RPCs.")

        let allAgentStore = SkillStore(service: fake.serviceClient())
        allAgentStore.agentFilter = .all
        let allAgentCallsBeforeRouting = fake.calls()
        await allAgentStore.showAllAppSearchResults(kind: .configHistory, query: "match")
        let multiAgentSnapshots = [
            ConfigSnapshotRecord(
                id: "snapshot-claude",
                agent: "claude-code",
                scope: "agent-global",
                target: "$HOME/.claude/settings.json",
                content: "{}",
                reason: "Match Claude config",
                createdAt: 20
            ),
            ConfigSnapshotRecord(
                id: "snapshot-codex",
                agent: "codex",
                scope: "agent-project",
                target: "$PROJECT/.codex/config.toml",
                content: "",
                reason: "Match Codex config",
                createdAt: 10
            ),
        ]
        let reachable = AgentConfigSidebarModel.filteredSnapshots(
            multiAgentSnapshots,
            agentFilter: allAgentStore.agentFilter,
            scopeFilter: allAgentStore.configScopeFilter,
            searchText: allAgentStore.configSidebarSearchText
        )
        try expectEqual(reachable.map(\.id), ["snapshot-claude", "snapshot-codex"], "Config History View All under the All agent filter should keep every real-agent snapshot reachable.")
        try expectEqual(fake.calls(), allAgentCallsBeforeRouting, "All-agent Config History View All must remain a zero-RPC route.")
    }

    func allAgentConfigHistoryPageTaskPreservesCachedRowsWithoutRPC() async throws {
        let runner = LocalHistoryPageRunner(delayedThirdMethods: ["snapshot.listAgentConfigPage"])
        defer { runner.release(method: "snapshot.listAgentConfigPage") }
        let store = SkillStore(service: runner.serviceClient())

        await store.loadMoreAgentConfigSnapshots(loadAll: false)
        try expectEqual(store.agentConfigSnapshotCompleteness.loadedCount, 100, "The concrete-agent fixture should publish its first page.")
        try expectEqual(store.agentConfigSnapshotCompleteness.totalCount, 205, "The concrete-agent fixture should expose its source total.")
        try expectEqual(store.agentConfigSnapshotCompleteness.hasMore, true, "The concrete-agent fixture should expose a continuation.")

        let latePageTask = Task { await store.loadMoreAgentConfigSnapshots(loadAll: true) }
        try await waitUntil("The concrete-agent load should suspend on its delayed third page.") {
            runner.syncCallCount(for: "snapshot.listAgentConfigPage") == 3
        }
        try expectEqual(store.agentConfigSnapshots.count, 200, "Two concrete-agent pages should be cached before switching scope.")
        try expectEqual(store.agentConfigSnapshotCompleteness.hasMore, true, "The in-flight concrete-agent accumulator should still carry its old continuation.")

        let codexSnapshot = ConfigSnapshotRecord(
            id: "cached-codex",
            agent: "codex",
            scope: "agent-project",
            target: "$PROJECT/.codex/config.toml",
            content: "",
            reason: "Match Codex config",
            createdAt: 1_000
        )
        await store.selectAppSearchItem(AppSearchItem(
            id: "config:\(codexSnapshot.id)",
            kind: .configHistory,
            targetID: codexSnapshot.id,
            title: codexSnapshot.reason,
            subtitle: codexSnapshot.target,
            agent: codexSnapshot.agent,
            configSnapshot: codexSnapshot
        ))
        try expectEqual(store.agentConfigSnapshots.count, 201, "The real Store should hold a multi-agent cache before the All-agents page lifecycle runs.")

        store.agentFilter = .all
        store.sidebarContentMode = .config
        store.configSidebarSearchText = "match"
        let callsBeforePageTask = runner.syncCallCount(for: "snapshot.listAgentConfigPage")

        await store.loadSelectedAgentConfigDataIfNeeded()

        try expectEqual(store.agentConfigSnapshots.count, 201, "The Config History page task must preserve cached multi-agent rows under All agents.")
        let reachable = AgentConfigSidebarModel.filteredSnapshots(
            store.agentConfigSnapshots,
            agentFilter: store.agentFilter,
            scopeFilter: store.configScopeFilter,
            searchText: ""
        )
        try expectEqual(reachable.count, store.agentConfigSnapshots.count, "Every cached real-agent row should remain reachable after the All-agents page task.")
        let allState = store.agentConfigSnapshotCompleteness
        try expectEqual(allState.loadedCount, store.agentConfigSnapshots.count, "All-agents completeness must describe the published cache rows.")
        try expectNil(allState.totalCount, "All-agents cache state must not reuse the concrete-agent source total.")
        try expectEqual(allState.hasMore, false, "All-agents cache state must not reuse the concrete-agent continuation.")
        try expectEqual(allState.canLoadMore, false, "All-agents cache state must not expose Load More.")
        try expectEqual(allState.canLoadAll, false, "All-agents cache state must not expose Load All.")
        try expectEqual(runner.syncCallCount(for: "snapshot.listAgentConfigPage"), callsBeforePageTask, "The All-agents Config History page task must issue zero service RPCs.")

        await store.loadMoreAgentConfigSnapshots(loadAll: false)
        try expectEqual(runner.syncCallCount(for: "snapshot.listAgentConfigPage"), callsBeforePageTask, "No stale concrete-agent cursor may issue an All-agents continuation RPC.")

        runner.release(method: "snapshot.listAgentConfigPage")
        await latePageTask.value
        try expectEqual(store.agentConfigSnapshots.count, 201, "A late concrete-agent generation must not replace the All-agents cache.")
        try expectEqual(store.agentConfigSnapshotCompleteness, allState, "A late concrete-agent generation must not restore its old pagination metadata.")
    }
}
