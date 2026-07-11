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
    }
}
