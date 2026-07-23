import Foundation

enum SkillsWorkspaceEmptyReason: Equatable {
    case noAcceptedSnapshot
    case noAggregates
    case noMatches
}

enum SkillsWorkspaceListContentState: Equatable {
    case loading
    case failed(String)
    case empty(SkillsWorkspaceEmptyReason)
    case ready
}

struct SkillAggregateRowPresentation: Identifiable, Equatable {
    let id: SkillAggregateRecord.ID
    let title: String
    let summary: String
    let logicalSourceLabel: String
    let agentLabels: [String]
    let scopeLabels: [String]
    let instanceCount: Int
    let installedCount: Int
    let enabledCount: Int
    let effectiveCount: Int
    let effectiveness: SkillEffectivenessState
    let effectivenessLabel: String
    let findingCount: Int
    let conflictCount: Int
    let needsAttention: Bool
    let coverageIsComplete: Bool

    init(aggregate: SkillAggregateRecord) {
        id = aggregate.id
        title = aggregate.displayName.isEmpty
            ? aggregate.canonicalName
            : aggregate.displayName
        summary = aggregate.description
        logicalSourceLabel = Self.logicalSourceLabel(for: aggregate.sourceKind)
        agentLabels = aggregate.agents
            .sorted(by: { Self.agentRank($0) < Self.agentRank($1) })
            .map(Self.agentLabel)
        scopeLabels = aggregate.scopes
            .sorted(by: { Self.scopeRank($0) < Self.scopeRank($1) })
            .map(Self.scopeLabel)
        instanceCount = aggregate.instanceIDs.count
        installedCount = aggregate.installedInstanceCount
        enabledCount = aggregate.enabledInstanceCount
        effectiveCount = aggregate.effectiveInstanceCount
        effectiveness = aggregate.primaryEffectiveness
        effectivenessLabel = Self.effectivenessLabel(aggregate.primaryEffectiveness)
        findingCount = aggregate.findingCount
        conflictCount = aggregate.conflictCount
        coverageIsComplete = aggregate.coverage.isComplete
        needsAttention = findingCount > 0
            || conflictCount > 0
            || effectiveness != .effective
            || !coverageIsComplete
    }

    var agentSummary: String {
        agentLabels.isEmpty
            ? UIStrings.text("skills.workspace.agent.toolGlobal", "Tool Global")
            : agentLabels.joined(separator: ", ")
    }

    var scopeSummary: String {
        scopeLabels.joined(separator: ", ")
    }

    var attentionCount: Int {
        findingCount + conflictCount
    }

    var attentionLabel: String {
        if attentionCount > 0 {
            return String(
                format: UIStrings.text(
                    "skills.workspace.row.attentionWithIssues",
                    "%d issues"
                ),
                attentionCount
            )
        }
        return UIStrings.text(
            "skills.workspace.row.needsAttention",
            "Needs Attention"
        )
    }

    var installedLabel: String {
        String(
            format: UIStrings.text(
                "skills.workspace.row.installedCount",
                "Installed %d/%d"
            ),
            installedCount,
            instanceCount
        )
    }

    var enabledLabel: String {
        String(
            format: UIStrings.text(
                "skills.workspace.row.enabledCount",
                "Enabled %d/%d"
            ),
            enabledCount,
            instanceCount
        )
    }

    var effectiveLabel: String {
        String(
            format: UIStrings.text(
                "skills.workspace.row.effectiveCount",
                "Verified Effective %d/%d"
            ),
            effectiveCount,
            instanceCount
        )
    }

    var accessibilitySummary: String {
        [
            title,
            logicalSourceLabel,
            agentSummary,
            scopeSummary,
            installedLabel,
            enabledLabel,
            effectiveLabel,
            effectivenessLabel,
            needsAttention ? attentionLabel : nil,
        ]
        .compactMap { $0 }
        .joined(separator: ", ")
    }

    private static func logicalSourceLabel(for sourceKind: String) -> String {
        let normalized = sourceKind
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
        if normalized.contains("plugin") {
            return UIStrings.text("skills.workspace.source.plugin", "Plugin")
        }
        if normalized.contains("compat") {
            return UIStrings.text(
                "skills.workspace.source.compatibility",
                "Compatibility"
            )
        }
        if normalized.contains("native") {
            return UIStrings.text("skills.workspace.source.native", "Native")
        }
        if normalized.contains("manager")
            || normalized.contains("package")
            || normalized.contains("registry") {
            return UIStrings.text(
                "skills.workspace.source.managedPackage",
                "Managed Package"
            )
        }
        if normalized.contains("local")
            || normalized.contains("project")
            || normalized.contains("user") {
            return UIStrings.text("skills.workspace.source.local", "Local")
        }
        return UIStrings.text(
            "skills.workspace.source.declared",
            "Declared Source"
        )
    }

    private static func agentRank(_ agent: ProductAgentID) -> Int {
        switch agent {
        case .claudeCode: 0
        case .codex: 1
        case .opencode: 2
        case .pi: 3
        case .hermes: 4
        case .openclaw: 5
        case .toolGlobal: 6
        }
    }

    private static func agentLabel(_ agent: ProductAgentID) -> String {
        switch agent {
        case .claudeCode: UIStrings.claudeCode
        case .codex: UIStrings.codex
        case .opencode: UIStrings.opencode
        case .pi: UIStrings.pi
        case .hermes: UIStrings.hermes
        case .openclaw: UIStrings.openclaw
        case .toolGlobal:
            UIStrings.text("skills.workspace.agent.toolGlobal", "Tool Global")
        }
    }

    private static func scopeRank(_ scope: ProductScope) -> Int {
        switch scope {
        case .agentProject: 0
        case .agentGlobal: 1
        case .toolGlobal: 2
        }
    }

    private static func scopeLabel(_ scope: ProductScope) -> String {
        switch scope {
        case .agentProject:
            UIStrings.text("skills.workspace.scope.project", "Project")
        case .agentGlobal:
            UIStrings.text("skills.workspace.scope.agentGlobal", "Agent Global")
        case .toolGlobal:
            UIStrings.text("skills.workspace.scope.toolGlobal", "Tool Global")
        }
    }

    private static func effectivenessLabel(
        _ effectiveness: SkillEffectivenessState
    ) -> String {
        switch effectiveness {
        case .effective:
            UIStrings.text(
                "skills.workspace.effectiveness.effective",
                "Verified Effective"
            )
        case .disabled:
            UIStrings.text("skills.workspace.effectiveness.disabled", "Disabled")
        case .shadowed:
            UIStrings.text("skills.workspace.effectiveness.shadowed", "Shadowed")
        case .installedUnlinked:
            UIStrings.text(
                "skills.workspace.effectiveness.installedUnlinked",
                "Installed, Not Linked"
            )
        case .broken:
            UIStrings.text("skills.workspace.effectiveness.broken", "Broken")
        case .unavailable:
            UIStrings.text(
                "skills.workspace.effectiveness.unavailable",
                "Unavailable"
            )
        }
    }
}

struct SkillsWorkspaceListPresentation: Equatable {
    static let orderedViews: [SkillWorkspaceView] = [
        .needsAttention,
        .project,
        .global,
        .all,
    ]

    let rows: [SkillAggregateRowPresentation]
    let loadedAggregateCount: Int
    let sourceAggregateTotal: Int?
    let visibleInstanceCount: Int
    let completeness: ListCompletenessState
    let contentState: SkillsWorkspaceListContentState
    let isRefreshingAcceptedSnapshot: Bool
    let isStale: Bool
    let supportingErrorMessage: String?

    init(
        visibleAggregates: [SkillAggregateRecord],
        loadedAggregateCount: Int,
        sourceAggregateTotal: Int?,
        completeness: ListCompletenessState,
        hasAcceptedSnapshot: Bool,
        isLoading: Bool,
        isStale: Bool,
        errorMessage: String?
    ) {
        rows = visibleAggregates.map(SkillAggregateRowPresentation.init)
        self.loadedAggregateCount = loadedAggregateCount
        self.sourceAggregateTotal = sourceAggregateTotal
        visibleInstanceCount = rows.reduce(0) { $0 + $1.instanceCount }
        self.completeness = completeness
        isRefreshingAcceptedSnapshot = isLoading && hasAcceptedSnapshot
        self.isStale = isStale
        supportingErrorMessage = hasAcceptedSnapshot ? errorMessage : nil

        if !rows.isEmpty {
            contentState = .ready
        } else if isLoading && !hasAcceptedSnapshot {
            contentState = .loading
        } else if !hasAcceptedSnapshot, let errorMessage {
            contentState = .failed(errorMessage)
        } else if !hasAcceptedSnapshot {
            contentState = .empty(.noAcceptedSnapshot)
        } else if loadedAggregateCount == 0 {
            contentState = .empty(.noAggregates)
        } else {
            contentState = .empty(.noMatches)
        }
    }

    var visibleAggregateCount: Int {
        rows.count
    }

    var visibleCountSummary: String {
        String(
            format: UIStrings.text(
                "skills.workspace.counts.visible",
                "%d aggregates · %d instances"
            ),
            visibleAggregateCount,
            visibleInstanceCount
        )
    }

    var sourceCountSummary: String {
        if let sourceAggregateTotal {
            return String(
                format: UIStrings.text(
                    "skills.workspace.counts.sourceTotal",
                    "Source total %d aggregates"
                ),
                sourceAggregateTotal
            )
        }
        return String(
            format: UIStrings.text(
                "skills.workspace.counts.sourceLoaded",
                "Loaded %d aggregates · source total unknown"
            ),
            loadedAggregateCount
        )
    }
}

extension SkillWorkspaceView {
    var presentationTitle: String {
        switch self {
        case .needsAttention:
            UIStrings.text(
                "skills.workspace.view.needsAttention",
                "Needs Attention"
            )
        case .project:
            UIStrings.text("skills.workspace.view.project", "Project")
        case .global:
            UIStrings.text("skills.workspace.view.global", "Global")
        case .all:
            UIStrings.text("skills.workspace.view.all", "All")
        }
    }
}

extension SkillAggregateSortOrder {
    var presentationTitle: String {
        switch self {
        case .name:
            UIStrings.text("skills.workspace.sort.name", "Name")
        case .issueCount:
            UIStrings.text("skills.workspace.sort.issues", "Issues")
        case .instanceCount:
            UIStrings.text("skills.workspace.sort.instances", "Instances")
        }
    }
}
