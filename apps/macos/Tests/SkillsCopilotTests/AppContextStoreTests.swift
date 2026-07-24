import Foundation
@testable import SkillsCopilot

@MainActor
struct AppContextStoreTests {
    static func run() async throws {
        try routesRestoreWithoutDetailSelection()
        try await prewarmUsesTypedProjectReads()
        try await failureAndCancellationPreserveAcceptedReadiness()
        try await lateResponsesCannotCrossProjectRevisions()
    }

    private static func routesRestoreWithoutDetailSelection() throws {
        for route in AppRoute.allCases {
            let restored = try AppRoute.restored(from: route.encodedForRestoration())
            try expectEqual(restored, route, "App route restoration")
        }
        try expectEqual(
            SidebarSelection.skill("skill-1").appRoute,
            .skills,
            "Skill detail route"
        )
        try expectEqual(
            SidebarSelection.session("session-1").appRoute,
            .sessions,
            "Session detail route"
        )
        try expectEqual(
            SidebarSelection.configOverview.appRoute,
            .advanced,
            "Config migration route"
        )
    }

    private static func prewarmUsesTypedProjectReads() async throws {
        let runner = AppContextServiceRunner()
        runner.enqueueContext(projectState(id: "project-a", revision: "context-a"))
        runner.enqueueReadiness(readinessEnvelope(
            projectID: "project-a",
            sourceRevision: "source-a"
        ))
        let store = AppContextStore(
            service: runner.serviceClient(),
            restoredRoute: .sessions,
            initialAgentFilter: .codex
        )

        await store.prewarm()

        try expectEqual(store.route, .sessions, "Prewarm should preserve the restored route.")
        try expectEqual(store.agentFilter, .codex, "Prewarm should preserve the agent filter.")
        try expectEqual(
            store.visibleProjectReadiness?.sourceRevision,
            "source-a",
            "Prewarm readiness"
        )
        try expectFalse(
            store.visibleProjectReadinessAcceptedAt == nil,
            "Accepted readiness should expose its local successful-refresh time."
        )
        try expectFalse(
            !store.hasCurrentProjectReadiness,
            "Prewarm readiness should match the accepted context revision."
        )
        try expectEqual(
            runner.methods,
            ["project.getContext", "project.getReadiness"],
            "Prewarm should perform only typed project reads."
        )
        let params = try runner.params(for: "project.getReadiness")
        try expectEqual(params["project_id"] as? String, "project-a", "Readiness project binding")
        try expectEqual(
            params["expected_project_context_revision"] as? String,
            "context-a",
            "Readiness context revision binding"
        )

        store.selectRoute(.advanced)
        _ = store.selectAgent(.pi)
        try expectEqual(
            runner.methods.count,
            2,
            "Route and filter changes must not cause service reads."
        )
    }

    private static func failureAndCancellationPreserveAcceptedReadiness() async throws {
        let runner = AppContextServiceRunner()
        let store = AppContextStore(service: runner.serviceClient())
        store.acceptProjectContextState(projectState(id: "project-a", revision: "context-a"))
        runner.enqueueReadiness(readinessEnvelope(
            projectID: "project-a",
            sourceRevision: "source-accepted"
        ))
        await store.refreshProjectReadiness()

        runner.enqueueFailure(AppContextTestError.expectedFailure)
        await store.refreshProjectReadiness()
        try expectEqual(
            store.visibleProjectReadiness?.sourceRevision,
            "source-accepted",
            "Failed refresh must preserve accepted readiness."
        )
        try expectFalse(
            !store.readinessState.isStale,
            "A failed refresh with accepted data should be stale."
        )

        runner.enqueueReadiness(
            readinessEnvelope(projectID: "project-a", sourceRevision: "source-late"),
            delayNanoseconds: 100_000_000
        )
        let readinessRequestCount = runner.requestCount(for: "project.getReadiness")
        let refresh = Task { @MainActor in
            await store.refreshProjectReadiness()
        }
        try await runner.waitForRequest(
            "project.getReadiness",
            count: readinessRequestCount + 1
        )
        store.cancelProjectReadinessRefresh()
        await refresh.value

        try expectEqual(
            store.visibleProjectReadiness?.sourceRevision,
            "source-accepted",
            "Cancellation must not replace accepted readiness."
        )
        try expectFalse(
            !store.hasCurrentProjectReadiness,
            "Cancellation should restore the exact accepted context snapshot."
        )
    }

    private static func lateResponsesCannotCrossProjectRevisions() async throws {
        let runner = AppContextServiceRunner()
        let store = AppContextStore(service: runner.serviceClient())
        store.acceptProjectContextState(projectState(id: "project-a", revision: "context-a"))
        runner.enqueueReadiness(
            readinessEnvelope(projectID: "project-a", sourceRevision: "source-a-late"),
            delayNanoseconds: 100_000_000
        )
        runner.enqueueReadiness(readinessEnvelope(
            projectID: "project-b",
            sourceRevision: "source-b"
        ))

        let staleRefresh = Task { @MainActor in
            await store.refreshProjectReadiness()
        }
        try await runner.waitForRequest("project.getReadiness", count: 1)
        store.acceptProjectContextState(projectState(id: "project-b", revision: "context-b"))
        await store.refreshProjectReadiness()
        await staleRefresh.value

        try expectEqual(
            store.activeProject?.id,
            "project-b",
            "Project context should remain on the current accepted project."
        )
        try expectEqual(
            store.visibleProjectReadiness?.sourceRevision,
            "source-b",
            "A late response from another project must not publish."
        )
        try expectFalse(
            !store.hasCurrentProjectReadiness,
            "Published readiness must match the current project revision."
        )
    }

    private static func projectState(id: String, revision: String) -> ProjectContextState {
        ProjectContextState(
            revision: revision,
            active: ProjectContext(
                id: id,
                name: id,
                rootPath: "<project-root>",
                currentCWD: "<project-root>",
                lastUsedAt: nil,
                isActive: true,
                validationError: nil
            ),
            recent: []
        )
    }

    private static func readinessEnvelope(
        projectID: String,
        sourceRevision: String
    ) -> Data {
        AppContextServiceRunner.data([
            "id": "test",
            "ok": true,
            "result": [
                "project_id": projectID,
                "project_display_name": projectID,
                "source_revision": sourceRevision,
                "health": "healthy",
                "coverage": [
                    "completeness": "enumerable",
                    "inspected_sources": 0,
                    "expected_sources": 0,
                ],
                "agents": [],
                "blocking_reasons": [],
                "attention": [],
                "evidence": [],
                "actions": [],
                "recent_sessions": [],
            ],
        ])
    }
}

private enum AppContextTestError: LocalizedError {
    case expectedFailure

    var errorDescription: String? { "expected refresh failure" }
}

private final class AppContextServiceRunner: ServiceProcessRunning {
    private struct Response {
        let result: Result<Data, Error>
        let delayNanoseconds: UInt64
    }

    private let lock = NSLock()
    private var contextResponses: [Response] = []
    private var readinessResponses: [Response] = []
    private var requests: [[String: Any]] = []

    var methods: [String] {
        lock.withLock {
            requests.compactMap { $0["method"] as? String }
        }
    }

    func requestCount(for method: String) -> Int {
        lock.withLock {
            requests.count { $0["method"] as? String == method }
        }
    }

    func waitForRequest(_ method: String, count: Int) async throws {
        for _ in 0 ..< 1_000 {
            if requestCount(for: method) >= count {
                return
            }
            await Task.yield()
        }
        throw NativeModelTestFailure(
            description: "Timed out waiting for AppContextStore request \(method)."
        )
    }

    func serviceClient() -> ServiceClient {
        ServiceClient(
            processRunner: self,
            serviceURL: URL(fileURLWithPath: "/test/product-context-service")
        )
    }

    func enqueueContext(_ state: ProjectContextState, delayNanoseconds: UInt64 = 0) {
        let result = Self.data([
            "id": "test",
            "ok": true,
            "result": Self.projectContextJSONObject(state),
        ])
        lock.withLock {
            contextResponses.append(Response(
                result: .success(result),
                delayNanoseconds: delayNanoseconds
            ))
        }
    }

    func enqueueReadiness(_ data: Data, delayNanoseconds: UInt64 = 0) {
        lock.withLock {
            readinessResponses.append(Response(
                result: .success(data),
                delayNanoseconds: delayNanoseconds
            ))
        }
    }

    func enqueueFailure(_ error: Error, delayNanoseconds: UInt64 = 0) {
        lock.withLock {
            readinessResponses.append(Response(
                result: .failure(error),
                delayNanoseconds: delayNanoseconds
            ))
        }
    }

    func params(for method: String) throws -> [String: Any] {
        try lock.withLock {
            guard let request = requests.first(where: {
                $0["method"] as? String == method
            }), let params = request["params"] as? [String: Any] else {
                throw NativeModelTestFailure(
                    description: "Missing AppContextStore request for \(method)."
                )
            }
            return params
        }
    }

    func run(
        executableURL: URL,
        input: Data,
        timeoutNanoseconds: UInt64?
    ) async throws -> Data {
        guard let request = try JSONSerialization.jsonObject(with: input) as? [String: Any],
              let method = request["method"] as? String else {
            throw NativeModelTestFailure(description: "Invalid AppContextStore request.")
        }
        let response: Response = try lock.withLock {
            requests.append(request)
            switch method {
            case "project.getContext":
                guard !contextResponses.isEmpty else {
                    throw NativeModelTestFailure(description: "Missing context response.")
                }
                return contextResponses.removeFirst()
            case "project.getReadiness":
                guard !readinessResponses.isEmpty else {
                    throw NativeModelTestFailure(description: "Missing readiness response.")
                }
                return readinessResponses.removeFirst()
            default:
                throw NativeModelTestFailure(
                    description: "Unexpected AppContextStore method \(method)."
                )
            }
        }
        if response.delayNanoseconds > 0 {
            try? await Task.sleep(nanoseconds: response.delayNanoseconds)
        }
        return try response.result.get()
    }

    static func data(_ object: [String: Any]) -> Data {
        try! JSONSerialization.data(withJSONObject: object)
    }

    private static func projectContextJSONObject(
        _ state: ProjectContextState
    ) -> [String: Any] {
        let active: Any
        if let project = state.active {
            active = [
                "id": project.id,
                "name": project.name,
                "root_path": project.rootPath,
                "current_cwd": project.currentCWD ?? project.rootPath,
                "is_active": project.isActive,
                "validation_error": (project.validationError as Any?) ?? NSNull(),
            ]
        } else {
            active = NSNull()
        }
        return [
            "revision": state.revision,
            "active": active,
            "recent": [],
        ]
    }
}

private extension NSLock {
    func withLock<Result>(_ body: () throws -> Result) rethrows -> Result {
        lock()
        defer { unlock() }
        return try body()
    }
}
