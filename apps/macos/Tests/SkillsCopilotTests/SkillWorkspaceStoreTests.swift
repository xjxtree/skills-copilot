import Foundation
@testable import SkillsCopilot

@MainActor
struct SkillWorkspaceStoreTests {
    static func run() async throws {
        let suite = SkillWorkspaceStoreTests()
        try await suite.criteriaUseAcceptedCacheWithoutRPC()
        try await suite.paginationPublishesOneTerminalSnapshot()
        try await suite.failurePreservesAcceptedRowsAndSelection()
        try await suite.cancellationAndProjectChangeRejectUnacceptedRows()
    }

    private func criteriaUseAcceptedCacheWithoutRPC() async throws {
        let revision = "sha256:workspace-criteria"
        let client = FakeSkillWorkspaceClient(steps: [
            .result(try makeSkillAggregateResult(
                revision: revision,
                specs: [
                    AggregateSpec(
                        id: "alpha",
                        name: "Alpha",
                        agent: .claudeCode,
                        scope: .agentProject,
                        state: .broken
                    ),
                    AggregateSpec(
                        id: "bravo",
                        name: "Bravo",
                        agent: .claudeCode,
                        scope: .agentGlobal,
                        state: .effective,
                        findingCount: 1
                    ),
                    AggregateSpec(
                        id: "zulu",
                        name: "Zulu",
                        agent: .codex,
                        scope: .agentGlobal,
                        state: .effective
                    ),
                ]
            )),
        ])
        let store = SkillWorkspaceStore(client: client)

        await store.refresh(
            projectID: "project-one",
            expectedProjectContextRevision: "sha256:context-one"
        )

        try expectEqual(
            store.visibleAggregates.map(\.id),
            ["aggregate-alpha", "aggregate-bravo"],
            "Needs Attention must consume typed aggregate state."
        )
        try expectNil(
            store.selectedAggregateID,
            "Loading the Skills route must not manufacture a selected aggregate."
        )
        store.selectAggregate(id: "aggregate-bravo")
        store.configure(
            view: .all,
            agentFilter: .codex,
            searchText: "zulu",
            sortOrder: .issueCount,
            sortDirection: .descending
        )
        try expectEqual(
            store.visibleAggregates.map(\.id),
            ["aggregate-zulu"],
            "Routine criteria must derive from the accepted cache."
        )
        try expectEqual(
            store.selectedAggregateID,
            nil,
            "Criteria changes must clear a selection that is no longer visible."
        )
        try expectEqual(client.requests.count, 1, "Criteria changes must not issue an RPC.")

        await store.load(
            projectID: "project-one",
            expectedProjectContextRevision: "sha256:context-one"
        )
        try expectEqual(client.requests.count, 1, "Exact accepted binding must reuse cache.")
        try expectNil(client.requests.first?.agent, "Workspace reads must fetch all agents.")
    }

    private func paginationPublishesOneTerminalSnapshot() async throws {
        let revision = "sha256:workspace-pages"
        let gate = SkillWorkspaceGate()
        let client = FakeSkillWorkspaceClient(steps: [
            .result(try makeSkillAggregateResult(
                revision: revision,
                specs: [
                    AggregateSpec(
                        id: "alpha",
                        name: "Alpha",
                        agent: .claudeCode,
                        scope: .agentProject,
                        state: .broken
                    ),
                ],
                totalCount: 2,
                hasMore: true,
                nextCursor: "cursor-two"
            )),
            .delayed(
                gate,
                .success(try makeSkillAggregateResult(
                    revision: revision,
                    specs: [
                        AggregateSpec(
                            id: "beta",
                            name: "Beta",
                            agent: .codex,
                            scope: .agentGlobal,
                            state: .effective
                        ),
                    ],
                    totalCount: 2
                ))
            ),
        ])
        let store = SkillWorkspaceStore(client: client)

        let refresh = Task {
            await store.refresh(
                projectID: "project-one",
                expectedProjectContextRevision: "sha256:context-one"
            )
        }
        try await waitUntil("Second aggregate page was not requested.") {
            gate.isWaiting
        }
        try expectEqual(
            store.aggregates.count,
            0,
            "A partial aggregate page must not become accepted UI state."
        )
        try expectNil(
            store.sourceRevision,
            "A source revision must publish only with its terminal snapshot."
        )

        gate.open()
        await refresh.value

        try expectEqual(
            store.aggregates.map(\.id),
            ["aggregate-alpha", "aggregate-beta"],
            "All aggregate pages must publish atomically."
        )
        try expectEqual(store.sourceRevision, revision, "Terminal source revision")
        try expectEqual(store.listCompleteness.isComplete, true, "Terminal completeness")
        try expectEqual(client.requests.count, 2, "Pagination request count")
        try expectEqual(client.requests[1].cursor, "cursor-two", "Continuation cursor")
        try expectEqual(
            client.requests[1].sourceRevision,
            revision,
            "Continuation must bind the first accepted page revision."
        )
    }

    private func failurePreservesAcceptedRowsAndSelection() async throws {
        let revision = "sha256:workspace-accepted"
        let accepted = try makeSkillAggregateResult(
            revision: revision,
            specs: [
                AggregateSpec(
                    id: "alpha",
                    name: "Alpha",
                    agent: .claudeCode,
                    scope: .agentProject,
                    state: .broken
                ),
                AggregateSpec(
                    id: "beta",
                    name: "Beta",
                    agent: .codex,
                    scope: .agentGlobal,
                    state: .effective
                ),
            ]
        )
        let client = FakeSkillWorkspaceClient(steps: [.result(accepted)])
        let store = SkillWorkspaceStore(client: client)
        store.configure(
            view: .all,
            agentFilter: .all,
            searchText: "",
            sortOrder: .name,
            sortDirection: .ascending
        )
        await store.refresh(
            projectID: "project-one",
            expectedProjectContextRevision: "sha256:context-one"
        )
        store.selectAggregate(id: "aggregate-beta")
        client.append(.failure(ServiceClient.ClientError.service(
            ServiceErrorPayload(code: "source_changed", message: "Source changed.")
        )))

        await store.refresh(
            projectID: "project-one",
            expectedProjectContextRevision: "sha256:context-one"
        )

        try expectEqual(
            store.aggregates.map(\.id),
            ["aggregate-alpha", "aggregate-beta"],
            "Failed refresh must preserve accepted rows."
        )
        try expectEqual(
            store.selectedAggregateID,
            "aggregate-beta",
            "Failed refresh must preserve selection."
        )
        try expectEqual(store.sourceRevision, revision, "Failed refresh accepted revision")
        try expectEqual(store.isStale, true, "Failed refresh must mark accepted rows stale.")
        try expectContains(
            store.errorMessage,
            "source_changed",
            "Typed source-change error must remain visible."
        )
    }

    private func cancellationAndProjectChangeRejectUnacceptedRows() async throws {
        let initial = try makeSkillAggregateResult(
            revision: "sha256:workspace-initial",
            specs: [
                AggregateSpec(
                    id: "accepted",
                    name: "Accepted",
                    agent: .claudeCode,
                    scope: .agentProject,
                    state: .broken
                ),
            ]
        )
        let cancellationGate = SkillWorkspaceGate()
        let projectGate = SkillWorkspaceGate()
        let client = FakeSkillWorkspaceClient(steps: [
            .result(initial),
            .delayed(
                cancellationGate,
                .success(try makeSkillAggregateResult(
                    revision: "sha256:workspace-cancelled",
                    specs: [
                        AggregateSpec(
                            id: "cancelled",
                            name: "Cancelled",
                            agent: .codex,
                            scope: .agentGlobal,
                            state: .broken
                        ),
                    ]
                ))
            ),
            .delayed(
                projectGate,
                .failure(FakeSkillWorkspaceError.expectedFailure)
            ),
        ])
        let store = SkillWorkspaceStore(client: client)
        await store.refresh(
            projectID: "project-one",
            expectedProjectContextRevision: "sha256:context-one"
        )
        store.selectAggregate(id: "aggregate-accepted")
        let cancelledRefresh = Task {
            await store.refresh(
                projectID: "project-one",
                expectedProjectContextRevision: "sha256:context-one"
            )
        }
        try await waitUntil("Cancelable refresh did not reach the client.") {
            cancellationGate.isWaiting
        }
        try expectEqual(
            store.aggregates.map(\.id),
            ["aggregate-accepted"],
            "Refresh in flight must preserve accepted rows."
        )
        store.cancelLoading()
        cancellationGate.open()
        await cancelledRefresh.value
        try expectEqual(
            store.aggregates.map(\.id),
            ["aggregate-accepted"],
            "Cancellation must reject late aggregate rows."
        )
        try expectEqual(
            store.selectedAggregateID,
            "aggregate-accepted",
            "Cancellation must preserve accepted selection."
        )

        let changedProjectRefresh = Task {
            await store.refresh(
                projectID: "project-two",
                expectedProjectContextRevision: "sha256:context-two"
            )
        }
        try await waitUntil("Project-change refresh did not reach the client.") {
            projectGate.isWaiting
        }
        try expectEqual(
            store.aggregates.count,
            0,
            "A different project must not expose the prior project's rows."
        )
        try expectNil(
            store.selectedAggregateID,
            "A different project must not expose the prior project's selection."
        )
        projectGate.open()
        await changedProjectRefresh.value
        try expectEqual(
            store.aggregates.count,
            0,
            "A failed project change must not restore cross-project rows."
        )
        try expectNil(
            store.acceptedProjectID,
            "A failed project change must not claim an accepted project."
        )
    }

    private func waitUntil(
        _ failure: String,
        predicate: @MainActor () -> Bool
    ) async throws {
        for _ in 0..<1_000 {
            if predicate() { return }
            await Task.yield()
        }
        throw NativeModelTestFailure(description: failure)
    }
}

@MainActor
private final class FakeSkillWorkspaceClient: SkillWorkspaceClient {
    struct Request {
        let projectID: String
        let projectContextRevision: String
        let agent: ProductAgentID?
        let limit: Int
        let cursor: String?
        let sourceRevision: String?
    }

    enum Step {
        case result(SkillAggregateListResult)
        case failure(Error)
        case delayed(
            SkillWorkspaceGate,
            Result<SkillAggregateListResult, Error>
        )
    }

    private var steps: [Step]
    private(set) var requests: [Request] = []

    init(steps: [Step]) {
        self.steps = steps
    }

    func append(_ step: Step) {
        steps.append(step)
    }

    func listSkillAggregates(
        projectID: String,
        expectedProjectContextRevision: String,
        agent: ProductAgentID?,
        limit: Int,
        cursor: String?,
        sourceRevision: String?
    ) async throws -> SkillAggregateListResult {
        requests.append(Request(
            projectID: projectID,
            projectContextRevision: expectedProjectContextRevision,
            agent: agent,
            limit: limit,
            cursor: cursor,
            sourceRevision: sourceRevision
        ))
        guard !steps.isEmpty else {
            throw FakeSkillWorkspaceError.unexpectedRequest
        }
        switch steps.removeFirst() {
        case .result(let result):
            return result
        case .failure(let error):
            throw error
        case .delayed(let gate, let result):
            await gate.wait()
            return try result.get()
        }
    }
}

@MainActor
private final class SkillWorkspaceGate {
    private var continuation: CheckedContinuation<Void, Never>?
    private(set) var isWaiting = false

    func wait() async {
        isWaiting = true
        await withCheckedContinuation { continuation in
            self.continuation = continuation
        }
        isWaiting = false
    }

    func open() {
        continuation?.resume()
        continuation = nil
    }
}

private enum FakeSkillWorkspaceError: Error {
    case unexpectedRequest
    case expectedFailure
}

private struct AggregateSpec {
    let id: String
    let name: String
    let agent: ProductAgentID
    let scope: ProductScope
    let state: SkillEffectivenessState
    var findingCount = 0
    var conflictCount = 0
}

private func makeSkillAggregateResult(
    revision: String,
    specs: [AggregateSpec],
    totalCount: Int? = nil,
    hasMore: Bool = false,
    nextCursor: String? = nil
) throws -> SkillAggregateListResult {
    let template = try skillAggregateFixtureResult()
    guard let aggregateTemplate = (template["aggregates"] as? [[String: Any]])?.first else {
        throw NativeModelTestFailure(description: "Missing aggregate fixture template.")
    }
    var result = template
    result["source_revision"] = revision
    result["aggregates"] = specs.map {
        skillAggregateObject(from: aggregateTemplate, spec: $0, revision: revision)
    }
    var page = result["page"] as? [String: Any] ?? [:]
    page["returned_count"] = specs.count
    page["total_count"] = totalCount ?? specs.count
    page["has_more"] = hasMore
    page["source_completeness"] = "enumerable"
    page.removeValue(forKey: "incomplete_reason")
    if let nextCursor {
        page["next_cursor"] = nextCursor
    } else {
        page.removeValue(forKey: "next_cursor")
    }
    result["page"] = page
    let data = try JSONSerialization.data(withJSONObject: result)
    return try JSONDecoder().decode(SkillAggregateListResult.self, from: data)
}

private func skillAggregateObject(
    from template: [String: Any],
    spec: AggregateSpec,
    revision: String
) -> [String: Any] {
    let aggregateID = "aggregate-\(spec.id)"
    let instanceID = "instance-\(spec.id)"
    let evidenceID = "evidence-\(spec.id)"
    let sourceIdentity = "source-\(spec.id)"
    let runtimeIdentity = "runtime-\(spec.id)"
    let isInstalled = spec.state != .unavailable
    let isEnabled = isInstalled && spec.state != .disabled
    let isEffective = spec.state == .effective
    var aggregate = template
    aggregate["id"] = aggregateID
    aggregate["definition_id"] = "definition-\(spec.id)"
    aggregate["definition_fingerprint"] = "sha256:\(spec.id)"
    aggregate["canonical_name"] = spec.id
    aggregate["display_name"] = spec.name
    aggregate["description"] = "\(spec.name) aggregate"
    aggregate["publisher"] = "Fixture Publisher"
    aggregate["package_name"] = "fixture-\(spec.id)"
    aggregate["package_version"] = "1.0.0"
    aggregate["source_identity"] = sourceIdentity
    aggregate["runtime_identity"] = runtimeIdentity
    aggregate["instance_ids"] = [instanceID]
    aggregate["agents"] = [spec.agent.rawValue]
    aggregate["scopes"] = [spec.scope.rawValue]
    aggregate["installed_instance_count"] = isInstalled ? 1 : 0
    aggregate["enabled_instance_count"] = isEnabled ? 1 : 0
    aggregate["effective_instance_count"] = isEffective ? 1 : 0
    aggregate["primary_effectiveness"] = spec.state.rawValue
    aggregate["effectiveness_counts"] = [[
        "state": spec.state.rawValue,
        "count": 1,
    ]]
    aggregate["instance_effectiveness"] = [[
        "instance_id": instanceID,
        "agent": spec.agent.rawValue,
        "scope": spec.scope.rawValue,
        "source_identity": sourceIdentity,
        "runtime_identity": runtimeIdentity,
        "installed": isInstalled,
        "linked": isInstalled,
        "enabled": isEnabled,
        "precedence_proven": true,
        "state": spec.state.rawValue,
        "coverage": [
            "completeness": "enumerable",
            "inspected_sources": 1,
            "expected_sources": 1,
        ],
        "evidence_refs": [evidenceID],
    ]]
    aggregate["finding_count"] = spec.findingCount
    aggregate["conflict_count"] = spec.conflictCount
    aggregate["source_revision"] = revision
    aggregate["evidence"] = [[
        "id": evidenceID,
        "kind": "skill_instance",
        "source_revision": revision,
        "summary": "\(spec.name) evidence",
        "agent": spec.agent.rawValue,
        "target_id": instanceID,
    ]]
    aggregate.removeValue(forKey: "actions")
    return aggregate
}

private func skillAggregateFixtureResult() throws -> [String: Any] {
    var repositoryRoot = URL(fileURLWithPath: #filePath)
    for _ in 0..<5 {
        repositoryRoot.deleteLastPathComponent()
    }
    let data = try Data(
        contentsOf: repositoryRoot
            .appendingPathComponent("fixtures/service-protocol")
            .appendingPathComponent("catalog.listSkillAggregates.response.json")
    )
    guard let envelope = try JSONSerialization.jsonObject(with: data) as? [String: Any],
          let result = envelope["result"] as? [String: Any] else {
        throw NativeModelTestFailure(description: "Invalid aggregate fixture envelope.")
    }
    return result
}

#if canImport(XCTest)
import XCTest

final class SkillWorkspaceStoreXCTest: XCTestCase {
    func testSkillWorkspaceStoreFoundation() async throws {
        try await SkillWorkspaceStoreTests.run()
    }
}
#endif
