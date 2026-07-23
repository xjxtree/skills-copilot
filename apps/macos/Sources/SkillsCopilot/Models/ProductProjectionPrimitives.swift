import Foundation

enum ProductAgentID: String, Codable, CaseIterable, Hashable {
    case toolGlobal = "tool-global"
    case claudeCode = "claude-code"
    case codex
    case pi
    case hermes
    case openclaw
    case opencode

    static let projectAgents: Set<ProductAgentID> = [
        .claudeCode,
        .codex,
        .pi,
        .hermes,
        .openclaw,
        .opencode,
    ]
}

enum ProductScope: String, Codable, Hashable {
    case toolGlobal = "tool-global"
    case agentGlobal = "agent-global"
    case agentProject = "agent-project"
}

enum EnvironmentHealthState: String, Codable, Hashable {
    case healthy
    case review
    case blocked
}

enum SkillEffectivenessState: String, Codable, Hashable {
    case effective
    case disabled
    case shadowed
    case installedUnlinked = "installed_unlinked"
    case broken
    case unavailable

    var severityRank: Int {
        switch self {
        case .broken: 0
        case .unavailable: 1
        case .disabled: 2
        case .shadowed: 3
        case .installedUnlinked: 4
        case .effective: 5
        }
    }
}

enum EvidenceKind: String, Codable, Hashable {
    case projectContext = "project_context"
    case adapterCapability = "adapter_capability"
    case scanCoverage = "scan_coverage"
    case skillDefinition = "skill_definition"
    case skillInstance = "skill_instance"
    case finding
    case conflict
    case session
    case config
    case package
    case actionReadback = "action_readback"
}

enum AttentionKind: String, Codable, Hashable {
    case incompleteEvidence = "incomplete_evidence"
    case staleEvidence = "stale_evidence"
    case sourceUnavailable = "source_unavailable"
    case finding
    case conflict
    case brokenSkill = "broken_skill"
    case skillUnavailable = "skill_unavailable"
}

enum AttentionSeverity: String, Codable, Hashable {
    case critical
    case error
    case warning
    case information
}

enum NoSafeActionReason: String, Codable, Hashable {
    case unsupported
    case readOnlySource = "read_only_source"
    case incompleteEvidence = "incomplete_evidence"
    case noGuardedWritePath = "no_guarded_write_path"
    case manualReviewRequired = "manual_review_required"
}

enum ResumeCapabilityState: String, Codable, Hashable {
    case supported
    case unsupported
}

enum ResumeUnsupportedReason: String, Codable, Hashable {
    case agentUnsupported = "agent_unsupported"
    case sessionUnsupported = "session_unsupported"
    case sourceIncomplete = "source_incomplete"
    case sourceChanged = "source_changed"
    case missingNativeID = "missing_native_id"
    case invalidProjectContext = "invalid_project_context"
}

enum ProductProjectionValidationError: LocalizedError, Equatable {
    case invalid(String)

    var errorDescription: String? {
        switch self {
        case .invalid(let message):
            "The service returned an invalid product projection: \(message)"
        }
    }
}

struct SourceCoverage: Codable, Hashable {
    let completeness: ListSourceCompleteness
    let incompleteReason: ListIncompleteReason?
    let inspectedSources: Int
    let expectedSources: Int?

    enum CodingKeys: String, CodingKey {
        case completeness
        case incompleteReason = "incomplete_reason"
        case inspectedSources = "inspected_sources"
        case expectedSources = "expected_sources"
    }

    var isComplete: Bool {
        completeness == .enumerable
            && incompleteReason == nil
            && expectedSources.map { $0 == inspectedSources } != false
    }

    @discardableResult
    func validated() throws -> SourceCoverage {
        guard inspectedSources >= 0,
              expectedSources.map({ $0 >= inspectedSources }) != false else {
            throw ProductProjectionValidationError.invalid("coverage counts are inconsistent")
        }
        switch completeness {
        case .enumerable:
            guard incompleteReason == nil,
                  expectedSources.map({ $0 == inspectedSources }) != false else {
                throw ProductProjectionValidationError.invalid(
                    "enumerable coverage is incomplete"
                )
            }
        case .limited, .unknown:
            guard incompleteReason != nil else {
                throw ProductProjectionValidationError.invalid(
                    "incomplete coverage omitted its reason"
                )
            }
        }
        return self
    }

    static func merged(_ coverages: [SourceCoverage]) throws -> SourceCoverage {
        guard !coverages.isEmpty else {
            return SourceCoverage(
                completeness: .unknown,
                incompleteReason: .notInspected,
                inspectedSources: 0,
                expectedSources: nil
            )
        }
        for coverage in coverages {
            try coverage.validated()
        }
        let inspectedSources = coverages.reduce(0) { $0 + $1.inspectedSources }
        let expectedSources: Int? = coverages.allSatisfy { $0.expectedSources != nil }
            ? coverages.compactMap(\.expectedSources).reduce(0, +)
            : nil
        if coverages.allSatisfy(\.isComplete) {
            return SourceCoverage(
                completeness: .enumerable,
                incompleteReason: nil,
                inspectedSources: inspectedSources,
                expectedSources: expectedSources
            )
        }
        let completeness: ListSourceCompleteness = coverages.contains {
            $0.completeness == .unknown
        } ? .unknown : .limited
        let reason = coverages
            .compactMap(\.incompleteReason)
            .max(by: { $0.productSeverityRank < $1.productSeverityRank })
            ?? .sourceLimited
        return SourceCoverage(
            completeness: completeness,
            incompleteReason: reason,
            inspectedSources: inspectedSources,
            expectedSources: expectedSources
        )
    }
}
