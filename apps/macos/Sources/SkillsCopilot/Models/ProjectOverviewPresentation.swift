import Foundation

struct ProjectOverviewPresentation: Equatable {
    enum State: Equatable {
        case emptyProject
        case emptySnapshot
        case loading
        case ready
        case stale
        case partial
        case blocked
        case error
    }

    struct AttentionRow: Identifiable, Equatable {
        let item: AttentionItem
        let evidence: [EvidenceRef]
        let actions: [ActionDescriptorWire]

        var id: String { item.id }
    }

    let state: State
    let projectName: String?
    let record: ProjectReadinessRecord?
    let agents: [AgentReadinessRecord]
    let attention: [AttentionRow]
    let recentSessions: [SessionContinuationRecord]
    let message: String?
    let isRefreshing: Bool
    let isStale: Bool
    let isPartial: Bool
    let isBlocked: Bool

    init(
        project: ProjectContext?,
        projectContextRevision: String?,
        readinessState: ProjectReadinessCacheState,
        isLoadingProjectContext: Bool,
        projectContextErrorMessage: String?,
        agentFilter: ProductAgentID?
    ) {
        let visibleEntry = readinessState.visibleEntry
        let visibleRecord = visibleEntry?.record
        let contextMatches = visibleEntry.map {
            $0.key.projectID == project?.id
                && $0.key.projectContextRevision == projectContextRevision
        } ?? false
        let stale = readinessState.isStale
            || (visibleEntry != nil && !contextMatches)
            || projectContextErrorMessage != nil
        let partial = visibleRecord.map { !$0.coverage.isComplete } ?? false
        let blocked = visibleRecord?.health == .blocked

        self.projectName = visibleRecord?.projectDisplayName ?? project?.name
        record = visibleRecord
        agents = Self.filteredAgents(visibleRecord?.agents ?? [], by: agentFilter)
        attention = Self.attentionRows(from: visibleRecord)
        recentSessions = Self.filteredSessions(
            visibleRecord?.recentSessions ?? [],
            by: agentFilter
        )
        isRefreshing = readinessState.isRefreshing || isLoadingProjectContext
        isStale = stale
        isPartial = partial
        isBlocked = blocked

        if project == nil {
            if isLoadingProjectContext {
                state = .loading
                message = nil
            } else if let projectContextErrorMessage {
                state = .error
                message = projectContextErrorMessage
            } else {
                state = .emptyProject
                message = nil
            }
            return
        }

        guard let visibleRecord else {
            switch readinessState {
            case .refreshing:
                state = .loading
                message = nil
            case .failed(let failure):
                state = .error
                message = failure
            case .empty, .accepted, .stale:
                if isLoadingProjectContext {
                    state = .loading
                    message = nil
                } else if let projectContextErrorMessage {
                    state = .error
                    message = projectContextErrorMessage
                } else {
                    state = .emptySnapshot
                    message = nil
                }
            }
            return
        }

        if stale {
            state = .stale
            message = readinessState.errorMessage ?? projectContextErrorMessage
        } else if partial {
            state = .partial
            message = Self.incompleteReasonText(visibleRecord.coverage.incompleteReason)
        } else if blocked {
            state = .blocked
            message = visibleRecord.blockingReasons.first?.summary
        } else {
            state = .ready
            message = nil
        }
    }

    var acceptedSnapshotLabel: String? {
        guard let revision = record?.sourceRevision else { return nil }
        let prefix = revision.prefix(12)
        return revision.count > prefix.count ? "\(prefix)…" : String(prefix)
    }

    static func coverageText(_ coverage: SourceCoverage) -> String {
        if let expected = coverage.expectedSources {
            return "\(coverage.inspectedSources) of \(expected) sources inspected"
        }
        return "\(coverage.inspectedSources) sources inspected; expected total unavailable"
    }

    static func incompleteReasonText(_ reason: ListIncompleteReason?) -> String? {
        switch reason {
        case .safetyBudget:
            "Inspection stopped at a safety limit."
        case .sourceChanged:
            "A source changed during inspection."
        case .sourceLimited:
            "One or more sources expose only limited evidence."
        case .unreadableSource:
            "One or more required sources could not be read."
        case .pageFailed:
            "A required evidence page could not be loaded."
        case .unsupportedProtocol:
            "This service cannot enumerate a required source."
        case .staleSource:
            "The available source evidence is stale."
        case .notInspected:
            "A required source has not been inspected."
        case nil:
            nil
        }
    }

    static func unsupportedResumeText(_ reason: ResumeUnsupportedReason?) -> String {
        switch reason {
        case .agentUnsupported:
            "This agent does not expose a verified native continuation command."
        case .sessionUnsupported:
            "This session cannot be continued through a verified native command."
        case .sourceIncomplete:
            "Continuation is unavailable because session evidence is incomplete."
        case .sourceChanged:
            "The session source changed after this snapshot."
        case .missingNativeID:
            "The session has no verified native continuation identifier."
        case .invalidProjectContext:
            "The session does not match the current project context."
        case nil:
            "No verified native continuation command is available."
        }
    }

    private static func filteredAgents(
        _ agents: [AgentReadinessRecord],
        by agentFilter: ProductAgentID?
    ) -> [AgentReadinessRecord] {
        guard let agentFilter else { return agents }
        return agents.filter { $0.agent == agentFilter }
    }

    private static func filteredSessions(
        _ sessions: [SessionContinuationRecord],
        by agentFilter: ProductAgentID?
    ) -> [SessionContinuationRecord] {
        guard let agentFilter else { return sessions }
        return sessions.filter { $0.agent == agentFilter }
    }

    private static func attentionRows(
        from record: ProjectReadinessRecord?
    ) -> [AttentionRow] {
        guard let record else { return [] }
        let evidenceByID = Dictionary(uniqueKeysWithValues: record.evidence.map { ($0.id, $0) })
        let actionsByID = Dictionary(uniqueKeysWithValues: record.actions.map { ($0.id, $0) })
        return record.attention.map { item in
            AttentionRow(
                item: item,
                evidence: item.evidenceRefs.compactMap { evidenceByID[$0] },
                actions: item.actionIDs.compactMap { actionsByID[$0] }
            )
        }
    }
}
