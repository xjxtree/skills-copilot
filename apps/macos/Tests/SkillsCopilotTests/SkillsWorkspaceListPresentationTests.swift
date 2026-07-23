import Foundation
@testable import SkillsCopilot

struct SkillsWorkspaceListPresentationTests {
    static func run() throws {
        try defaultViewsFollowProductOrder()
        try aggregateAndInstanceCountsStayDistinct()
        try effectivenessAndAttentionRemainIndependent()
        try logicalSourceNeverCopiesPhysicalIdentity()
        try loadingEmptyErrorAndStaleStatesRemainDistinct()
    }

    private static func defaultViewsFollowProductOrder() throws {
        try expectEqual(
            SkillsWorkspaceListPresentation.orderedViews,
            [.needsAttention, .project, .global, .all],
            "Skills workspace default view order"
        )
    }

    private static func aggregateAndInstanceCountsStayDistinct() throws {
        let aggregate = try aggregateFixture(
            sourceKind: "codex-native",
            sourceIdentity: "codex-native-global",
            includeDisabledProjectInstance: true
        )
        let presentation = makePresentation(
            visibleAggregates: [aggregate],
            loadedAggregateCount: 1,
            sourceAggregateTotal: 4
        )

        try expectEqual(
            presentation.visibleAggregateCount,
            1,
            "One aggregate definition must remain one visible row."
        )
        try expectEqual(
            presentation.visibleInstanceCount,
            2,
            "Aggregate instance evidence must have its own total."
        )
        try expectContains(
            presentation.visibleCountSummary,
            "1 aggregates",
            "Count summary must label aggregate totals."
        )
        try expectContains(
            presentation.visibleCountSummary,
            "2 instances",
            "Count summary must label instance totals."
        )
        try expectContains(
            presentation.sourceCountSummary,
            "4 aggregates",
            "Source total must remain distinct from visible aggregates."
        )
    }

    private static func effectivenessAndAttentionRemainIndependent() throws {
        let aggregate = try aggregateFixture(
            sourceKind: "agent-compatibility",
            sourceIdentity: "claude-compatible-global",
            includeDisabledProjectInstance: true
        )
        let row = SkillAggregateRowPresentation(aggregate: aggregate)

        try expectEqual(row.instanceCount, 2, "Instance count")
        try expectEqual(row.installedCount, 2, "Installed count")
        try expectEqual(row.enabledCount, 1, "Enabled count")
        try expectEqual(row.effectiveCount, 1, "Verified effective count")
        try expectEqual(
            row.effectiveness,
            .disabled,
            "Primary effectiveness must not be inferred from package presence."
        )
        try expectEqual(
            row.needsAttention,
            true,
            "A non-effective aggregate must require attention without a finding."
        )
        try expectEqual(
            row.agentLabels.count,
            2,
            "All aggregate agents must remain visible."
        )
        try expectEqual(
            row.scopeLabels.count,
            2,
            "All aggregate scopes must remain visible."
        )
    }

    private static func logicalSourceNeverCopiesPhysicalIdentity() throws {
        let sourceIdentity = "codex-plugin-cache-openai-curated"
        let aggregate = try aggregateFixture(
            sourceKind: "codex-plugin",
            sourceIdentity: sourceIdentity,
            includeDisabledProjectInstance: false
        )
        let row = SkillAggregateRowPresentation(aggregate: aggregate)
        let visibleText = [
            row.title,
            row.summary,
            row.logicalSourceLabel,
            row.agentSummary,
            row.scopeSummary,
            row.accessibilitySummary,
        ].joined(separator: " ")

        try expectEqual(
            row.logicalSourceLabel,
            UIStrings.text("skills.workspace.source.plugin", "Plugin"),
            "Plugin provenance must use a logical source label."
        )
        try expectFalse(
            visibleText.contains(sourceIdentity),
            "A row must not expose its physical source identity."
        )
        try expectFalse(
            visibleText.localizedCaseInsensitiveContains("plugin/cache"),
            "A row must not render a raw plugin cache path."
        )
    }

    private static func loadingEmptyErrorAndStaleStatesRemainDistinct() throws {
        let loading = makePresentation(
            hasAcceptedSnapshot: false,
            isLoading: true
        )
        try expectEqual(loading.contentState, .loading, "Initial loading state")

        let failed = makePresentation(
            hasAcceptedSnapshot: false,
            errorMessage: "fixture failure"
        )
        try expectEqual(
            failed.contentState,
            .failed("fixture failure"),
            "Initial failure state"
        )

        let unavailable = makePresentation(hasAcceptedSnapshot: false)
        try expectEqual(
            unavailable.contentState,
            .empty(.noAcceptedSnapshot),
            "No accepted snapshot state"
        )

        let empty = makePresentation(hasAcceptedSnapshot: true)
        try expectEqual(
            empty.contentState,
            .empty(.noAggregates),
            "Accepted empty snapshot state"
        )

        let aggregate = try aggregateFixture(
            sourceKind: "codex-native",
            sourceIdentity: "codex-native-global",
            includeDisabledProjectInstance: false
        )
        let noMatches = makePresentation(
            visibleAggregates: [],
            loadedAggregateCount: 1,
            sourceAggregateTotal: 1,
            hasAcceptedSnapshot: true
        )
        try expectEqual(
            noMatches.contentState,
            .empty(.noMatches),
            "Filtered empty state"
        )

        let stale = makePresentation(
            visibleAggregates: [aggregate],
            loadedAggregateCount: 1,
            sourceAggregateTotal: 1,
            hasAcceptedSnapshot: true,
            isStale: true,
            errorMessage: "source changed"
        )
        try expectEqual(stale.contentState, .ready, "Stale rows remain readable.")
        try expectEqual(stale.isStale, true, "Stale state")
        try expectEqual(
            stale.supportingErrorMessage,
            "source changed",
            "Accepted rows retain supporting refresh errors."
        )
    }

    private static func makePresentation(
        visibleAggregates: [SkillAggregateRecord] = [],
        loadedAggregateCount: Int = 0,
        sourceAggregateTotal: Int? = nil,
        hasAcceptedSnapshot: Bool = true,
        isLoading: Bool = false,
        isStale: Bool = false,
        errorMessage: String? = nil
    ) -> SkillsWorkspaceListPresentation {
        SkillsWorkspaceListPresentation(
            visibleAggregates: visibleAggregates,
            loadedAggregateCount: loadedAggregateCount,
            sourceAggregateTotal: sourceAggregateTotal,
            completeness: ListCompletenessState(
                loadedCount: loadedAggregateCount,
                totalCount: sourceAggregateTotal,
                hasMore: false,
                isComplete: hasAcceptedSnapshot,
                completeness: hasAcceptedSnapshot ? .complete : .unknown,
                incompleteReason: hasAcceptedSnapshot ? nil : .unsupportedProtocol,
                loadingPhase: isLoading ? .all : .idle,
                canLoadMore: false,
                canLoadAll: false
            ),
            hasAcceptedSnapshot: hasAcceptedSnapshot,
            isLoading: isLoading,
            isStale: isStale,
            errorMessage: errorMessage
        )
    }

    private static func aggregateFixture(
        sourceKind: String,
        sourceIdentity: String,
        includeDisabledProjectInstance: Bool
    ) throws -> SkillAggregateRecord {
        var result = try aggregateFixtureResult()
        guard var aggregate = (result["aggregates"] as? [[String: Any]])?.first,
              var firstInstance = (aggregate["instance_effectiveness"] as? [[String: Any]])?.first,
              var firstEvidence = (aggregate["evidence"] as? [[String: Any]])?.first else {
            throw NativeModelTestFailure(
                description: "Missing skill aggregate presentation fixture."
            )
        }
        aggregate["source_kind"] = sourceKind
        aggregate["source_identity"] = sourceIdentity
        firstInstance["source_identity"] = sourceIdentity

        if includeDisabledProjectInstance {
            let secondInstanceID = "instance:claude-code:review"
            let secondEvidenceID = "evidence:instance:claude-code:review"
            firstInstance["instance_id"] = secondInstanceID
            firstInstance["agent"] = ProductAgentID.claudeCode.rawValue
            firstInstance["scope"] = ProductScope.agentProject.rawValue
            firstInstance["enabled"] = false
            firstInstance["state"] = SkillEffectivenessState.disabled.rawValue
            firstInstance["evidence_refs"] = [secondEvidenceID]

            firstEvidence["id"] = secondEvidenceID
            firstEvidence["summary"] = "Claude Code compatibility instance"
            firstEvidence["agent"] = ProductAgentID.claudeCode.rawValue
            firstEvidence["target_id"] = secondInstanceID

            let firstInstanceID = "instance:codex:review"
            aggregate["instance_ids"] = [firstInstanceID, secondInstanceID]
            aggregate["agents"] = [
                ProductAgentID.codex.rawValue,
                ProductAgentID.claudeCode.rawValue,
            ]
            aggregate["scopes"] = [
                ProductScope.agentGlobal.rawValue,
                ProductScope.agentProject.rawValue,
            ]
            var originalInstance =
                (aggregate["instance_effectiveness"] as? [[String: Any]])?.first ?? [:]
            originalInstance["source_identity"] = sourceIdentity
            aggregate["instance_effectiveness"] = [originalInstance, firstInstance]
            aggregate["installed_instance_count"] = 2
            aggregate["enabled_instance_count"] = 1
            aggregate["effective_instance_count"] = 1
            aggregate["primary_effectiveness"] =
                SkillEffectivenessState.disabled.rawValue
            aggregate["effectiveness_counts"] = [
                [
                    "state": SkillEffectivenessState.effective.rawValue,
                    "count": 1,
                ],
                [
                    "state": SkillEffectivenessState.disabled.rawValue,
                    "count": 1,
                ],
            ]
            aggregate["evidence"] = [
                (aggregate["evidence"] as? [[String: Any]])?.first ?? [:],
                firstEvidence,
            ]
            aggregate["coverage"] = [
                "completeness": "enumerable",
                "inspected_sources": 2,
                "expected_sources": 2,
            ]
        } else {
            aggregate["instance_effectiveness"] = [
                firstInstance,
            ]
        }

        result["aggregates"] = [aggregate]
        let data = try JSONSerialization.data(withJSONObject: result)
        return try JSONDecoder()
            .decode(SkillAggregateListResult.self, from: data)
            .aggregates[0]
    }

    private static func aggregateFixtureResult() throws -> [String: Any] {
        var repositoryRoot = URL(fileURLWithPath: #filePath)
        for _ in 0..<5 {
            repositoryRoot.deleteLastPathComponent()
        }
        let data = try Data(
            contentsOf: repositoryRoot
                .appendingPathComponent("fixtures/service-protocol")
                .appendingPathComponent("catalog.listSkillAggregates.response.json")
        )
        guard let envelope = try JSONSerialization.jsonObject(with: data)
                as? [String: Any],
              let result = envelope["result"] as? [String: Any] else {
            throw NativeModelTestFailure(
                description: "Invalid aggregate fixture envelope."
            )
        }
        return result
    }
}

#if canImport(XCTest)
import XCTest

final class SkillsWorkspaceListPresentationXCTest: XCTestCase {
    func testSkillsWorkspaceListPresentation() throws {
        try SkillsWorkspaceListPresentationTests.run()
    }
}
#endif
