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
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        let snapshots = [
            ConfigSnapshotRecord(
                id: "cached-claude",
                agent: "claude-code",
                scope: "agent-global",
                target: "$HOME/.claude/settings.json",
                content: "{}",
                reason: "Match Claude config",
                createdAt: 20
            ),
            ConfigSnapshotRecord(
                id: "cached-codex",
                agent: "codex",
                scope: "agent-project",
                target: "$PROJECT/.codex/config.toml",
                content: "",
                reason: "Match Codex config",
                createdAt: 10
            ),
        ]
        for snapshot in snapshots {
            await store.selectAppSearchItem(AppSearchItem(
                id: "config:\(snapshot.id)",
                kind: .configHistory,
                targetID: snapshot.id,
                title: snapshot.reason,
                subtitle: snapshot.target,
                agent: snapshot.agent,
                configSnapshot: snapshot
            ))
        }
        try expectEqual(
            Set(store.agentConfigSnapshots.map(\.id)),
            Set(snapshots.map(\.id)),
            "The real Store should hold both cached agent snapshots before the All-agents page lifecycle runs."
        )

        store.agentFilter = .all
        store.sidebarContentMode = .config
        store.configSidebarSearchText = "match"
        let callsBeforePageTask = fake.calls()

        await store.loadSelectedAgentConfigDataIfNeeded()

        try expectEqual(
            Set(store.agentConfigSnapshots.map(\.id)),
            Set(snapshots.map(\.id)),
            "The Config History page task must preserve cached multi-agent rows under All agents."
        )
        let reachable = AgentConfigSidebarModel.filteredSnapshots(
            store.agentConfigSnapshots,
            agentFilter: store.agentFilter,
            scopeFilter: store.configScopeFilter,
            searchText: store.configSidebarSearchText
        )
        try expectEqual(
            reachable.map(\.id),
            ["cached-claude", "cached-codex"],
            "Both cached real-agent rows should remain reachable after the All-agents page task."
        )
        try expectEqual(fake.calls(), callsBeforePageTask, "The All-agents Config History page task must issue zero service RPCs.")
    }
}
