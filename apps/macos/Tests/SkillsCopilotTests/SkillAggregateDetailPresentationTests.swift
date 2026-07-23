import Foundation
@testable import SkillsCopilot

struct SkillAggregateDetailPresentationTests {
    static func run() throws {
        try presentsAggregateAnswerAndCompleteInstanceEvidence()
        try distinguishesEveryCanonicalEffectivenessState()
        try marksDeterministicIssuesForAttention()
        try neverExposesPhysicalCachePaths()
        try exposesTypedDetailActionVocabulary()
    }

    private static func presentsAggregateAnswerAndCompleteInstanceEvidence() throws {
        let aggregate = try fixtureAggregate()
        let presentation = SkillAggregateDetailPresentation(aggregate: aggregate)

        try expectEqual(presentation.displayName, "Review", "Aggregate display name")
        try expectEqual(
            presentation.instances.map(\.id),
            aggregate.instanceIDs,
            "Detail must retain every aggregate instance."
        )
        try expectEqual(
            presentation.aggregate.installedInstanceCount,
            1,
            "Installed fact count"
        )
        try expectEqual(
            presentation.aggregate.enabledInstanceCount,
            1,
            "Enabled fact count"
        )
        try expectEqual(
            presentation.aggregate.effectiveInstanceCount,
            1,
            "Verified effective count"
        )
        try expectEqual(
            presentation.effectiveLocations.count,
            1,
            "Verified effective locations"
        )
        try expectFalse(presentation.needsAttention, "Healthy aggregate attention state")
        try expectEqual(
            presentation.aggregate.evidence.count,
            1,
            "Detail preserves every evidence reference."
        )
        try expectEqual(
            presentation.coverageText.isEmpty,
            false,
            "Coverage explanation must be visible."
        )
    }

    private static func distinguishesEveryCanonicalEffectivenessState() throws {
        let states: [SkillEffectivenessState] = [
            .effective,
            .disabled,
            .shadowed,
            .installedUnlinked,
            .broken,
            .unavailable,
        ]
        let labels = states.map(SkillAggregateDetailPresentation.effectivenessLabel)

        try expectEqual(
            Set(labels).count,
            states.count,
            "Canonical effectiveness labels must remain distinct."
        )
        try expectFalse(
            labels.contains(where: { $0.trimmingCharacters(in: .whitespaces).isEmpty }),
            "Canonical effectiveness labels must not be empty."
        )
    }

    private static func marksDeterministicIssuesForAttention() throws {
        let aggregate = try fixtureAggregate { aggregate in
            aggregate["finding_count"] = 2
            aggregate["conflict_count"] = 1
        }
        let presentation = SkillAggregateDetailPresentation(aggregate: aggregate)

        try expectFalse(!presentation.needsAttention, "Finding attention state")
        try expectEqual(presentation.aggregate.findingCount, 2, "Finding count")
        try expectEqual(presentation.aggregate.conflictCount, 1, "Conflict count")
    }

    private static func neverExposesPhysicalCachePaths() throws {
        let homePrefix = ["", "Users", "example"].joined(separator: "/")
        let physicalCache = [
            homePrefix,
            ".codex",
            "plugins",
            "cache",
            "vendor",
            "package",
            "skill",
        ].joined(separator: "/")
        let aggregate = try fixtureAggregate { aggregate in
            aggregate["canonical_name"] = physicalCache
            aggregate["display_name"] = physicalCache
            aggregate["description"] = "Loaded from \(physicalCache)"
            aggregate["definition_id"] = physicalCache
            aggregate["source_revision"] = physicalCache
            aggregate["source_kind"] = physicalCache
            aggregate["publisher"] = physicalCache
            aggregate["package_name"] = "safe-package"
            aggregate["read_only_reason"] = "Files at \(physicalCache) are read-only"
            var evidence = aggregate["evidence"] as? [[String: Any]] ?? []
            evidence[0]["id"] = physicalCache
            evidence[0]["source_revision"] = physicalCache
            aggregate["evidence"] = evidence
            var instances = aggregate["instance_effectiveness"] as? [[String: Any]] ?? []
            instances[0]["evidence_refs"] = [physicalCache]
            aggregate["instance_effectiveness"] = instances
        }
        let presentation = SkillAggregateDetailPresentation(aggregate: aggregate)
        let visibleText = (
            [presentation.purpose, presentation.provenanceLabel]
                + presentation.advancedMetadata.flatMap { [$0.label, $0.value] }
                + presentation.evidence.flatMap { [$0.idLabel, $0.summary] }
                + presentation.instances.flatMap {
                    $0.evidenceRefLabels + $0.actionLabels
                }
        ).joined(separator: "\n")

        try expectFalse(
            visibleText.contains("plugins/cache"),
            "Physical plugin cache path must not enter presentation metadata."
        )
        try expectFalse(
            visibleText.contains(homePrefix),
            "Absolute paths must not enter aggregate presentation."
        )
        try expectEqual(
            SkillAggregateDetailPresentation.logicalProvenanceLabel(
                for: "chatgpt-plugin-cache"
            ),
            UIStrings.text(
                "skillAggregate.provenance.plugin",
                "Installed agent plugin"
            ),
            "Plugin source uses a logical product label."
        )
    }

    private static func exposesTypedDetailActionVocabulary() throws {
        try expectEqual(
            SkillAggregatePackageAction.allCases,
            [.add, .detail, .update, .remove],
            "Package ownership action vocabulary"
        )
        try expectEqual(
            SkillAggregateConfigAction.allCases,
            [.enable, .disable],
            "Agent config action vocabulary"
        )
        try expectEqual(
            SkillAggregateDetailLayer.allCases,
            [.answer, .evidence, .advanced],
            "Skill detail disclosure order"
        )
    }

    private static func fixtureAggregate(
        mutate: ((inout [String: Any]) -> Void)? = nil
    ) throws -> SkillAggregateRecord {
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
              let result = envelope["result"] as? [String: Any],
              var aggregate = (result["aggregates"] as? [[String: Any]])?.first else {
            throw NativeModelTestFailure(description: "Invalid aggregate fixture envelope.")
        }
        mutate?(&aggregate)
        return try JSONDecoder().decode(
            SkillAggregateRecord.self,
            from: JSONSerialization.data(withJSONObject: aggregate)
        )
    }
}

#if canImport(XCTest)
import XCTest

final class SkillAggregateDetailPresentationXCTest: XCTestCase {
    func testSkillAggregateDetailPresentation() throws {
        try SkillAggregateDetailPresentationTests.run()
    }
}
#endif
