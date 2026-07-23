import Foundation
@testable import SkillsCopilot

struct ProductReadProjectionModelTests {
    func run() async throws {
        try decodesValidatedCoreProjectionRecords()
        try malformedProjectionRelationshipsFailClosed()
        try await typedRPCsBindProjectDiscoveryAndRevisions()
    }

    private func decodesValidatedCoreProjectionRecords() throws {
        let readiness: ProjectReadinessRecord = try decodeFixtureResult(
            "project.getReadiness.response.json"
        )
        try expectEqual(
            Set(readiness.agents.map(\.agent)),
            ProductAgentID.projectAgents,
            "Readiness must preserve the six project agents."
        )
        try expectEqual(
            readiness.recentSessions.first?.resume.argv,
            ["codex", "resume", "native-thread-fixture"],
            "Readiness must preserve the adapter-native copy-only argv."
        )

        let aggregates: SkillAggregateListResult = try decodeFixtureResult(
            "catalog.listSkillAggregates.response.json"
        )
        try expectEqual(aggregates.page.returnedCount, 1, "Aggregate page count")
        try expectEqual(
            aggregates.aggregates.first?.primaryEffectiveness,
            .effective,
            "Aggregate effectiveness"
        )

        let continuation: SessionContinuationRecord = try decodeFixtureResult(
            "session.previewResume.response.json"
        )
        try expectEqual(continuation.resume.copyOnly, true, "Resume must remain copy-only.")
        try expectEqual(
            continuation.actions.first?.applyMethod,
            nil,
            "Read-only continuation must not expose an apply method."
        )
    }

    private func malformedProjectionRelationshipsFailClosed() throws {
        try expectFixtureMutationRejected(
            "project.getReadiness.response.json",
            as: ProjectReadinessRecord.self
        ) { result in
            var evidence = result["evidence"] as? [[String: Any]] ?? []
            evidence[0]["source_revision"] = "sha256:other"
            result["evidence"] = evidence
        }
        try expectFixtureMutationRejected(
            "catalog.listSkillAggregates.response.json",
            as: SkillAggregateListResult.self
        ) { result in
            var aggregates = result["aggregates"] as? [[String: Any]] ?? []
            aggregates[0]["source_revision"] = "sha256:other"
            result["aggregates"] = aggregates
        }
        try expectFixtureMutationRejected(
            "session.previewResume.response.json",
            as: SessionContinuationRecord.self
        ) { result in
            result["coverage"] = [
                "completeness": "limited",
                "incomplete_reason": "source_limited",
                "inspected_sources": 0,
                "expected_sources": 1,
            ]
        }
        try expectFixtureMutationRejected(
            "project.getReadiness.response.json",
            as: ProjectReadinessRecord.self
        ) { result in
            result["health"] = "future_state"
        }
    }

    private func typedRPCsBindProjectDiscoveryAndRevisions() async throws {
        let runner = ProductReadProjectionRunner()
        let client = ServiceClient(
            processRunner: runner,
            serviceURL: URL(fileURLWithPath: "/tmp/product-read-service")
        )
        let project = ProjectContext(
            id: "project-funnyaccount-system",
            name: "funnyaccount_system",
            rootPath: "<project-root>",
            currentCWD: "<project-root>",
            lastUsedAt: nil,
            isActive: true,
            validationError: nil
        )

        _ = try await client.getProjectReadiness(
            projectID: project.id,
            expectedProjectContextRevision: "sha256:project-context-1",
            sourceRevision: "sha256:project-readiness-1"
        )
        _ = try await client.listSkillAggregates(
            projectID: project.id,
            expectedProjectContextRevision: "sha256:project-context-1",
            agent: .codex,
            limit: 500,
            cursor: nil,
            sourceRevision: "sha256:skill-aggregates-1"
        )
        _ = try await client.previewSessionResume(
            authorizedRoots: [],
            agent: .codex,
            project: project,
            sessionID: "session:codex:fixture",
            expectedSourceRevision: "sha256:session-native-1",
            expectedSnapshotRevision: "sha256:project-readiness-1"
        )

        try expectEqual(
            runner.methods,
            [
                "project.getReadiness",
                "catalog.listSkillAggregates",
                "session.previewResume",
            ],
            "Product read wrappers must use only their typed service methods."
        )
        let readiness = try runner.params(for: "project.getReadiness")
        try expectEqual(
            readiness["expected_project_context_revision"] as? String,
            "sha256:project-context-1",
            "Readiness context revision"
        )
        try expectEqual(
            readiness["source_revision"] as? String,
            "sha256:project-readiness-1",
            "Readiness accepted source revision"
        )
        let aggregates = try runner.params(for: "catalog.listSkillAggregates")
        try expectEqual(aggregates["agent"] as? String, "codex", "Aggregate agent filter")
        try expectEqual(aggregates["limit"] as? Int, 100, "Aggregate limit must be bounded.")
        let resume = try runner.params(for: "session.previewResume")
        try expectEqual(resume["authorized_roots"] as? [String], [], "Resume authorized roots")
        try expectEqual(resume["auto_discover"] as? Bool, true, "Resume discovery policy")
        try expectEqual(resume["agent"] as? String, "codex", "Resume agent")
        try expectEqual(
            resume["expected_source_revision"] as? String,
            "sha256:session-native-1",
            "Resume native revision"
        )
        try expectEqual(
            resume["expected_snapshot_revision"] as? String,
            "sha256:project-readiness-1",
            "Resume projection revision"
        )
        try expectNil(resume["scope"], "Resume resolver must use its fixed all-scope inventory.")
        try expectNil(resume["search"], "Resume resolver must not add a search filter.")
        try expectNil(resume["limit"], "Resume resolver owns its fixed safety bounds.")
    }

    private func expectFixtureMutationRejected<Result: Decodable>(
        _ name: String,
        as type: Result.Type,
        mutate: (inout [String: Any]) -> Void
    ) throws {
        var envelope = try fixtureJSONObject(name)
        guard var result = envelope["result"] as? [String: Any] else {
            throw NativeModelTestFailure(description: "Missing fixture result for \(name).")
        }
        mutate(&result)
        envelope["result"] = result
        let data = try JSONSerialization.data(withJSONObject: envelope)
        do {
            _ = try JSONDecoder().decode(ServiceEnvelope<Result>.self, from: data)
            throw NativeModelTestFailure(
                description: "Malformed \(name) should fail closed."
            )
        } catch is NativeModelTestFailure {
            throw NativeModelTestFailure(
                description: "Malformed \(name) unexpectedly decoded."
            )
        } catch {
            // Expected.
        }
    }
}

private final class ProductReadProjectionRunner: ServiceProcessRunning {
    private(set) var methods: [String] = []
    private var requests: [[String: Any]] = []

    func params(for method: String) throws -> [String: Any] {
        guard let request = requests.first(where: { $0["method"] as? String == method }),
              let params = request["params"] as? [String: Any] else {
            throw NativeModelTestFailure(description: "Missing product read request: \(method)")
        }
        return params
    }

    func run(
        executableURL: URL,
        input: Data,
        timeoutNanoseconds: UInt64?
    ) async throws -> Data {
        guard let request = try JSONSerialization.jsonObject(with: input) as? [String: Any],
              let method = request["method"] as? String else {
            throw NativeModelTestFailure(description: "Invalid product read request.")
        }
        requests.append(request)
        methods.append(method)
        switch method {
        case "project.getReadiness":
            return try fixtureData("project.getReadiness.response.json")
        case "catalog.listSkillAggregates":
            return try fixtureData("catalog.listSkillAggregates.response.json")
        case "session.previewResume":
            return try fixtureData("session.previewResume.response.json")
        default:
            return Data(
                #"{"id":"test","ok":false,"error":{"code":"unknown_method","message":"unknown method"}}"#.utf8
            )
        }
    }
}

private func decodeFixtureResult<Result: Decodable>(_ name: String) throws -> Result {
    let envelope = try JSONDecoder().decode(
        ServiceEnvelope<Result>.self,
        from: fixtureData(name)
    )
    guard envelope.ok, let result = envelope.result else {
        throw NativeModelTestFailure(description: "Fixture failed to decode: \(name)")
    }
    return result
}

private func fixtureJSONObject(_ name: String) throws -> [String: Any] {
    guard let value = try JSONSerialization.jsonObject(
        with: fixtureData(name)
    ) as? [String: Any] else {
        throw NativeModelTestFailure(description: "Fixture is not an object: \(name)")
    }
    return value
}

private func fixtureData(_ name: String) throws -> Data {
    var repositoryRoot = URL(fileURLWithPath: #filePath)
    for _ in 0..<5 {
        repositoryRoot.deleteLastPathComponent()
    }
    return try Data(
        contentsOf: repositoryRoot
            .appendingPathComponent("fixtures/service-protocol")
            .appendingPathComponent(name)
    )
}
