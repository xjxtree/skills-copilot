import Foundation

struct AppSearchIndex {
    let skills: [SkillRecord]
    let sessionSummaries: [LocalSessionPreviewRow]
    let configSnapshots: [ConfigSnapshotRecord]

    func search(query: String, limitPerKind: Int) -> AppSearchResult {
        let query = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else { return .empty() }
        let needle = query.lowercased()
        let limit = max(1, limitPerKind)

        let matchingSkills = skills.filter { SkillListModel.matchesSearchQuery($0, query: query) }
        let summaries = sessionSummaries.map(\.summaryOnly)
        let matchingSessions = summaries.filter {
            [$0.title, $0.excerpt, $0.agent ?? ""].contains { $0.lowercased().contains(needle) }
        }
        let matchingSnapshots = configSnapshots.filter {
            [$0.reason, $0.agent, $0.scope, $0.target].contains { $0.lowercased().contains(needle) }
        }

        let skillItems = matchingSkills.prefix(limit).map { skill in
            let provenance = [skill.publisher, skill.packageName, skill.packageVersion.map { "v\($0)" }, skill.sourceKind]
                .compactMap { $0 }
                .filter { !$0.isEmpty }
            return AppSearchItem(
                id: "skill:\(skill.id)",
                kind: .skill,
                targetID: skill.id,
                title: skill.name,
                subtitle: ([DisplayText.agent(skill.agent), DisplayText.scope(skill.scope)] + provenance)
                    .joined(separator: " · "),
                agent: skill.agent,
                skill: skill
            )
        }
        let sessionItems = matchingSessions.prefix(limit).map { session in
            let time = session.endedAt ?? session.startedAt
            let context = [session.agent.map(DisplayText.agent), session.projectRoot ?? session.scope, time.map(DisplayText.timestamp)]
                .compactMap { $0 }
            return AppSearchItem(
                id: "session:\(session.id)",
                kind: .session,
                targetID: session.id,
                title: session.title,
                subtitle: context.joined(separator: " · "),
                agent: session.agent,
                session: session
            )
        }
        let configItems = matchingSnapshots.prefix(limit).map { snapshot in
            AppSearchItem(
                id: "config_history:\(snapshot.id)",
                kind: .configHistory,
                targetID: snapshot.id,
                title: snapshot.reason,
                subtitle: [
                    DisplayText.agent(snapshot.agent),
                    DisplayText.scope(snapshot.scope),
                    DisplayText.configPathSummary(snapshot.target),
                    DisplayText.timestamp(snapshot.createdAt),
                ].joined(separator: " · "),
                agent: snapshot.agent,
                configSnapshot: snapshot
            )
        }
        let items = Array(skillItems) + Array(sessionItems) + Array(configItems)
        return AppSearchResult(
            generatedBy: "local-summary-index",
            query: query,
            totalMatchedCount: matchingSkills.count + matchingSessions.count + matchingSnapshots.count,
            limitPerKind: limit,
            items: items,
            kindCounts: [
                AppSearchKindCount(kind: .skill, count: matchingSkills.count),
                AppSearchKindCount(kind: .session, count: matchingSessions.count),
                AppSearchKindCount(kind: .configHistory, count: matchingSnapshots.count),
            ]
        )
    }
}
