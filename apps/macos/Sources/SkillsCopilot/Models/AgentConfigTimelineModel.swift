import Foundation

struct AgentConfigTimelineModel: Hashable {
    let agentTitle: String
    let isSpecificAgent: Bool
    let items: [AgentConfigTimelineItem]

    var summaryText: String {
        guard isSpecificAgent else {
            return UIStrings.agentConfigTimelineSelectAgent
        }
        if items.isEmpty {
            return UIStrings.agentConfigTimelineEmptySummary(agentTitle)
        }
        return UIStrings.agentConfigTimelineSummary(agentTitle, items.count)
    }

    static func make(
        snapshots: [ConfigSnapshotRecord],
        agentFilter: SkillAgentFilter
    ) -> AgentConfigTimelineModel {
        guard agentFilter != .all else {
            return AgentConfigTimelineModel(
                agentTitle: agentFilter.title,
                isSpecificAgent: false,
                items: []
            )
        }

        let filtered = snapshots
            .filter { $0.agent == agentFilter.rawValue }
            .sorted { lhs, rhs in
                if lhs.createdAt != rhs.createdAt {
                    return lhs.createdAt > rhs.createdAt
                }
                return lhs.id > rhs.id
            }
        return AgentConfigTimelineModel(
            agentTitle: agentFilter.title,
            isSpecificAgent: true,
            items: filtered.map(AgentConfigTimelineItem.init(snapshot:))
        )
    }
}

struct AgentConfigTimelineItem: Identifiable, Hashable {
    let snapshot: ConfigSnapshotRecord
    let timeText: String
    let actionText: String
    let targetSummary: String
    let scopeText: String
    let statusText: String
    let capturedText: String

    var id: String { snapshot.id }

    init(snapshot: ConfigSnapshotRecord) {
        self.snapshot = snapshot
        timeText = DisplayText.timestamp(snapshot.createdAt)
        let trimmedReason = snapshot.reason.trimmingCharacters(in: .whitespacesAndNewlines)
        actionText = trimmedReason.isEmpty ? UIStrings.agentConfigTimelineDefaultAction : trimmedReason
        targetSummary = Self.pathSummary(snapshot.target)
        scopeText = DisplayText.scope(snapshot.scope)
        statusText = UIStrings.agentConfigTimelineStatus
        capturedText = UIStrings.charactersCaptured(snapshot.content.count)
    }

    private static func pathSummary(_ path: String) -> String {
        let trimmed = path.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return UIStrings.unknown }

        return DisplayText.configPathSummary(trimmed)
    }
}
