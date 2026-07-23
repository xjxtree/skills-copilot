@testable import SkillsCopilot

@MainActor
extension SkillStoreTests {
    func removeRecentProjectUpdatesCacheWithoutScanning() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "project-set")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        await store.removeRecentProject(id: "project-1")

        try expectEqual(
            store.activeProjectContext?.name,
            "Fixture Project",
            "Removing a recent row should preserve the active project."
        )
        try expectEqual(
            store.recentProjectContexts.map(\.id),
            ["project-2"],
            "The removed recent project should disappear from cached picker state."
        )
        try expectContains(
            fake.calls(),
            "project.removeRecentContext",
            "Recent project removal should use the typed service method."
        )
        try expectFalse(
            fake.calls().contains("catalog.scanAll"),
            "Removing a recent project should not scan skills."
        )
    }

    func clearRecentProjectsUpdatesCacheWithoutScanning() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "project-set")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        await store.previewClearRecentProjects()
        await store.confirmProjectContextPendingAction()

        try expectEqual(
            store.activeProjectContext?.name,
            "Fixture Project",
            "Clearing recent projects should preserve the active project."
        )
        try expectEqual(
            store.recentProjectContexts.count,
            0,
            "Clearing recent projects should empty cached picker state."
        )
        try expectContains(
            fake.calls(),
            "project.clearRecentContexts",
            "Recent project clearing should use the typed service method."
        )
        try expectFalse(
            fake.calls().contains("catalog.scanAll"),
            "Clearing recent projects should not scan skills."
        )
    }
}
