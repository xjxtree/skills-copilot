import Foundation
@testable import SkillsCopilot

struct ProjectOverviewPresentationTests {
    static func run() throws {
        try distinguishesEmptyLoadingAndErrorStates()
        try projectsAcceptedAndStaleSnapshots()
        try exposesPartialBlockingWithoutHealthyPresentation()
        try resolvesAttentionAndAgentFilteredSessions()
    }

    private static func distinguishesEmptyLoadingAndErrorStates() throws {
        let empty = presentation(project: nil, readinessState: .empty)
        try expectEqual(empty.state, .emptyProject, "Overview empty-project state")

        let loading = presentation(
            project: nil,
            readinessState: .empty,
            isLoadingProjectContext: true
        )
        try expectEqual(loading.state, .loading, "Overview project loading state")

        let project = projectContext()
        let noSnapshot = presentation(project: project, readinessState: .empty)
        try expectEqual(noSnapshot.state, .emptySnapshot, "Overview empty-snapshot state")

        let failed = presentation(
            project: project,
            readinessState: .failed(message: "readiness failed")
        )
        try expectEqual(failed.state, .error, "Overview readiness error state")
        try expectEqual(failed.message, "readiness failed", "Overview error message")
    }

    private static func projectsAcceptedAndStaleSnapshots() throws {
        let record = try readinessFixture()
        let entry = cacheEntry(record: record)
        let accepted = presentation(
            project: projectContext(),
            readinessState: .accepted(entry),
            agentFilter: .codex
        )

        try expectEqual(accepted.state, .ready, "Accepted overview state")
        try expectEqual(accepted.agents.map(\.agent), [.codex], "Overview agent filter")
        try expectEqual(accepted.recentSessions.map(\.agent), [.codex], "Session agent filter")
        try expectEqual(
            accepted.acceptedSnapshotLabel,
            "sha256:proje…",
            "Overview snapshot revision label"
        )
        try expectFalse(accepted.isPartial, "Complete fixture must not be partial.")
        try expectFalse(accepted.isBlocked, "Healthy fixture must not be blocked.")

        let stale = presentation(
            project: projectContext(),
            readinessState: .stale(entry, message: "refresh failed")
        )
        try expectEqual(stale.state, .stale, "Overview stale state")
        try expectEqual(stale.message, "refresh failed", "Overview stale message")
        try expectEqual(
            stale.record?.sourceRevision,
            record.sourceRevision,
            "Stale overview preserves the accepted snapshot."
        )
    }

    private static func exposesPartialBlockingWithoutHealthyPresentation() throws {
        let record = try mutatedReadinessFixture { result in
            let blocker: [String: Any] = [
                "id": "blocker:codex-coverage",
                "kind": "incomplete_evidence",
                "summary": "Codex evidence is incomplete.",
                "agent": "codex",
                "evidence_refs": ["evidence:project-context"],
                "action_ids": [],
            ]
            result["health"] = "blocked"
            result["coverage"] = [
                "completeness": "limited",
                "incomplete_reason": "source_limited",
                "inspected_sources": 5,
                "expected_sources": 6,
            ]
            result["blocking_reasons"] = [blocker]
            var agents = result["agents"] as? [[String: Any]] ?? []
            guard let codexIndex = agents.firstIndex(where: {
                $0["agent"] as? String == "codex"
            }) else {
                return
            }
            agents[codexIndex]["health"] = "blocked"
            agents[codexIndex]["coverage"] = [
                "completeness": "limited",
                "incomplete_reason": "source_limited",
                "inspected_sources": 0,
                "expected_sources": 1,
            ]
            agents[codexIndex]["blocking_reasons"] = [blocker]
            result["agents"] = agents
        }
        let partial = presentation(
            project: projectContext(),
            readinessState: .accepted(cacheEntry(record: record)),
            agentFilter: .codex
        )

        try expectEqual(partial.state, .partial, "Incomplete overview state")
        try expectFalse(!partial.isPartial, "Incomplete coverage must remain visible.")
        try expectFalse(!partial.isBlocked, "Incomplete evidence must retain blocked health.")
        try expectEqual(
            partial.agents.first?.health,
            .blocked,
            "Incomplete agent must not render healthy."
        )
    }

    private static func resolvesAttentionAndAgentFilteredSessions() throws {
        let record = try mutatedReadinessFixture { result in
            result["actions"] = [[
                "id": "action:review-project-evidence",
                "kind": "review_evidence",
                "intent": "review_evidence",
                "target": [
                    "kind": "project",
                    "id": "project-funnyaccount-system",
                ],
                "project_id": "project-funnyaccount-system",
                "impacts": ["read_only"],
                "preview_method": "project.getReadiness",
                "source_revision": "sha256:project-readiness-1",
                "confirmation_required": false,
                "network": "none",
                "readback": ["project_context"],
                "evidence_refs": ["evidence:project-context"],
            ]]
            result["attention"] = [[
                "id": "attention:review-project-evidence",
                "kind": "finding",
                "severity": "warning",
                "title": "Review project evidence",
                "summary": "One project fact needs review.",
                "target": [
                    "kind": "project",
                    "id": "project-funnyaccount-system",
                ],
                "evidence_refs": ["evidence:project-context"],
                "action_ids": ["action:review-project-evidence"],
            ]]
        }
        let all = presentation(
            project: projectContext(),
            readinessState: .accepted(cacheEntry(record: record))
        )
        try expectEqual(all.attention.count, 1, "Overview attention rows")
        try expectEqual(all.attention.first?.evidence.count, 1, "Resolved attention evidence")
        try expectEqual(all.attention.first?.actions.count, 1, "Resolved attention actions")

        let claude = presentation(
            project: projectContext(),
            readinessState: .accepted(cacheEntry(record: record)),
            agentFilter: .claudeCode
        )
        try expectEqual(claude.recentSessions.count, 0, "Agent-filtered recent sessions")
    }

    private static func presentation(
        project: ProjectContext?,
        readinessState: ProjectReadinessCacheState,
        isLoadingProjectContext: Bool = false,
        projectContextErrorMessage: String? = nil,
        agentFilter: ProductAgentID? = nil
    ) -> ProjectOverviewPresentation {
        ProjectOverviewPresentation(
            project: project,
            projectContextRevision: "sha256:project-context-1",
            readinessState: readinessState,
            isLoadingProjectContext: isLoadingProjectContext,
            projectContextErrorMessage: projectContextErrorMessage,
            agentFilter: agentFilter
        )
    }

    private static func cacheEntry(
        record: ProjectReadinessRecord
    ) -> ProjectReadinessCacheEntry {
        ProjectReadinessCacheEntry(
            key: ProjectReadinessCacheKey(
                projectID: record.projectID,
                projectContextRevision: "sha256:project-context-1"
            ),
            record: record
        )
    }

    private static func projectContext() -> ProjectContext {
        ProjectContext(
            id: "project-funnyaccount-system",
            name: "funnyaccount_system",
            rootPath: "<project-root>",
            currentCWD: "<project-root>",
            lastUsedAt: nil,
            isActive: true,
            validationError: nil
        )
    }

    private static func readinessFixture() throws -> ProjectReadinessRecord {
        let envelope = try JSONDecoder().decode(
            ServiceEnvelope<ProjectReadinessRecord>.self,
            from: fixtureData()
        )
        guard let result = envelope.result else {
            throw NativeModelTestFailure(description: "Missing readiness fixture result.")
        }
        return result
    }

    private static func mutatedReadinessFixture(
        _ mutate: (inout [String: Any]) -> Void
    ) throws -> ProjectReadinessRecord {
        guard var envelope = try JSONSerialization.jsonObject(
            with: fixtureData()
        ) as? [String: Any],
        var result = envelope["result"] as? [String: Any] else {
            throw NativeModelTestFailure(description: "Invalid readiness fixture.")
        }
        mutate(&result)
        envelope["result"] = result
        let data = try JSONSerialization.data(withJSONObject: envelope)
        let decoded = try JSONDecoder().decode(
            ServiceEnvelope<ProjectReadinessRecord>.self,
            from: data
        )
        guard let record = decoded.result else {
            throw NativeModelTestFailure(description: "Missing mutated readiness result.")
        }
        return record
    }

    private static func fixtureData() throws -> Data {
        var repositoryRoot = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 {
            repositoryRoot.deleteLastPathComponent()
        }
        return try Data(
            contentsOf: repositoryRoot
                .appendingPathComponent("fixtures/service-protocol")
                .appendingPathComponent("project.getReadiness.response.json")
        )
    }
}
