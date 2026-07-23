import Foundation

struct ReadinessBlocker: Codable, Hashable, Identifiable {
    let id: String
    let kind: AttentionKind
    let summary: String
    let agent: ProductAgentID?
    let evidenceRefs: [String]
    let actionIDs: [String]

    enum CodingKeys: String, CodingKey {
        case id
        case kind
        case summary
        case agent
        case evidenceRefs = "evidence_refs"
        case actionIDs = "action_ids"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        kind = try container.decode(AttentionKind.self, forKey: .kind)
        summary = try container.decode(String.self, forKey: .summary)
        agent = try container.decodeIfPresent(ProductAgentID.self, forKey: .agent)
        evidenceRefs = try container.decode([String].self, forKey: .evidenceRefs)
        actionIDs = try container.decodeIfPresent([String].self, forKey: .actionIDs) ?? []
    }

    @discardableResult
    func validated() throws -> ReadinessBlocker {
        try requireProductText(id, "readiness blocker id is empty")
        try requireSafeProductSummary(summary, "readiness blocker summary is empty")
        try requireUniqueProductIDs(evidenceRefs, allowEmpty: false, label: "blocker evidence")
        try requireUniqueProductIDs(actionIDs, allowEmpty: true, label: "blocker actions")
        return self
    }
}

struct AttentionItem: Codable, Hashable, Identifiable {
    let id: String
    let kind: AttentionKind
    let severity: AttentionSeverity
    let title: String
    let summary: String
    let target: ActionTargetWire
    let agent: ProductAgentID?
    let evidenceRefs: [String]
    let actionIDs: [String]
    let noSafeActionReason: NoSafeActionReason?

    enum CodingKeys: String, CodingKey {
        case id
        case kind
        case severity
        case title
        case summary
        case target
        case agent
        case evidenceRefs = "evidence_refs"
        case actionIDs = "action_ids"
        case noSafeActionReason = "no_safe_action_reason"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        kind = try container.decode(AttentionKind.self, forKey: .kind)
        severity = try container.decode(AttentionSeverity.self, forKey: .severity)
        title = try container.decode(String.self, forKey: .title)
        summary = try container.decode(String.self, forKey: .summary)
        target = try container.decode(ActionTargetWire.self, forKey: .target)
        agent = try container.decodeIfPresent(ProductAgentID.self, forKey: .agent)
        evidenceRefs = try container.decode([String].self, forKey: .evidenceRefs)
        actionIDs = try container.decodeIfPresent([String].self, forKey: .actionIDs) ?? []
        noSafeActionReason = try container.decodeIfPresent(
            NoSafeActionReason.self,
            forKey: .noSafeActionReason
        )
    }

    @discardableResult
    func validated() throws -> AttentionItem {
        try requireProductText(id, "attention id is empty")
        try requireProductText(title, "attention title is empty")
        try requireSafeProductSummary(summary, "attention summary is empty")
        try requireProductText(target.id, "attention target id is empty")
        try requireUniqueProductIDs(evidenceRefs, allowEmpty: false, label: "attention evidence")
        try requireUniqueProductIDs(actionIDs, allowEmpty: true, label: "attention actions")
        guard actionIDs.isEmpty == (noSafeActionReason != nil) else {
            throw ProductProjectionValidationError.invalid(
                "attention action availability is inconsistent"
            )
        }
        return self
    }
}

struct AgentReadinessRecord: Codable, Hashable, Identifiable {
    let agent: ProductAgentID
    let health: EnvironmentHealthState
    let coverage: SourceCoverage
    let effectiveSkillCount: Int
    let issueCount: Int
    let conflictCount: Int
    let evidenceRefs: [String]
    let actionIDs: [String]
    let blockingReasons: [ReadinessBlocker]
    let attentionItemIDs: [String]

    var id: ProductAgentID { agent }

    enum CodingKeys: String, CodingKey {
        case agent
        case health
        case coverage
        case effectiveSkillCount = "effective_skill_count"
        case issueCount = "issue_count"
        case conflictCount = "conflict_count"
        case evidenceRefs = "evidence_refs"
        case actionIDs = "action_ids"
        case blockingReasons = "blocking_reasons"
        case attentionItemIDs = "attention_item_ids"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        agent = try container.decode(ProductAgentID.self, forKey: .agent)
        health = try container.decode(EnvironmentHealthState.self, forKey: .health)
        coverage = try container.decode(SourceCoverage.self, forKey: .coverage)
        effectiveSkillCount = try container.decode(Int.self, forKey: .effectiveSkillCount)
        issueCount = try container.decode(Int.self, forKey: .issueCount)
        conflictCount = try container.decode(Int.self, forKey: .conflictCount)
        evidenceRefs = try container.decodeIfPresent([String].self, forKey: .evidenceRefs) ?? []
        actionIDs = try container.decodeIfPresent([String].self, forKey: .actionIDs) ?? []
        blockingReasons = try container.decodeIfPresent(
            [ReadinessBlocker].self,
            forKey: .blockingReasons
        ) ?? []
        attentionItemIDs = try container.decodeIfPresent(
            [String].self,
            forKey: .attentionItemIDs
        ) ?? []
    }

    @discardableResult
    fileprivate func validated(
        evidenceIDs: Set<String>,
        actionIDs knownActionIDs: Set<String>,
        attentionIDs: Set<String>
    ) throws -> AgentReadinessRecord {
        guard ProductAgentID.projectAgents.contains(agent),
              effectiveSkillCount >= 0,
              issueCount >= 0,
              conflictCount >= 0 else {
            throw ProductProjectionValidationError.invalid("invalid agent readiness row")
        }
        try coverage.validated()
        guard health == .blocked || coverage.isComplete else {
            throw ProductProjectionValidationError.invalid(
                "incomplete agent coverage was not blocked"
            )
        }
        guard health != .healthy || blockingReasons.isEmpty,
              health != .blocked || !blockingReasons.isEmpty else {
            throw ProductProjectionValidationError.invalid(
                "agent health and blockers are inconsistent"
            )
        }
        try requireKnownProductIDs(
            evidenceRefs,
            known: evidenceIDs,
            label: "agent readiness evidence"
        )
        try requireKnownProductIDs(
            actionIDs,
            known: knownActionIDs,
            label: "agent readiness actions"
        )
        try requireKnownProductIDs(
            attentionItemIDs,
            known: attentionIDs,
            label: "agent readiness attention"
        )
        for blocker in blockingReasons {
            try blocker.validated()
            guard blocker.agent == nil || blocker.agent == agent else {
                throw ProductProjectionValidationError.invalid(
                    "agent blocker belongs to another agent"
                )
            }
            try requireKnownProductIDs(
                blocker.evidenceRefs,
                known: evidenceIDs,
                label: "agent blocker evidence"
            )
            try requireKnownProductIDs(
                blocker.actionIDs,
                known: knownActionIDs,
                label: "agent blocker actions"
            )
        }
        return self
    }
}

struct ProjectReadinessRecord: Decodable, Hashable, Identifiable {
    let projectID: String
    let projectDisplayName: String
    let sourceRevision: String
    let health: EnvironmentHealthState
    let coverage: SourceCoverage
    let agents: [AgentReadinessRecord]
    let blockingReasons: [ReadinessBlocker]
    let attention: [AttentionItem]
    let evidence: [EvidenceRef]
    let actions: [ActionDescriptorWire]
    let recentSessions: [SessionContinuationRecord]

    var id: String { projectID }

    enum CodingKeys: String, CodingKey {
        case projectID = "project_id"
        case projectDisplayName = "project_display_name"
        case sourceRevision = "source_revision"
        case health
        case coverage
        case agents
        case blockingReasons = "blocking_reasons"
        case attention
        case evidence
        case actions
        case recentSessions = "recent_sessions"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        projectID = try container.decode(String.self, forKey: .projectID)
        projectDisplayName = try container.decode(String.self, forKey: .projectDisplayName)
        sourceRevision = try container.decode(String.self, forKey: .sourceRevision)
        health = try container.decode(EnvironmentHealthState.self, forKey: .health)
        coverage = try container.decode(SourceCoverage.self, forKey: .coverage)
        agents = try container.decode([AgentReadinessRecord].self, forKey: .agents)
        blockingReasons = try container.decodeIfPresent(
            [ReadinessBlocker].self,
            forKey: .blockingReasons
        ) ?? []
        attention = try container.decodeIfPresent([AttentionItem].self, forKey: .attention) ?? []
        evidence = try container.decodeIfPresent([EvidenceRef].self, forKey: .evidence) ?? []
        actions = try container.decodeIfPresent([ActionDescriptorWire].self, forKey: .actions) ?? []
        recentSessions = try container.decodeIfPresent(
            [SessionContinuationRecord].self,
            forKey: .recentSessions
        ) ?? []
        try validate()
    }

    private func validate() throws {
        try requireSafeProductIdentity(projectID, "project readiness id is empty")
        try requireProductText(projectDisplayName, "project display name is empty")
        try requireProductText(sourceRevision, "project source revision is empty")
        try coverage.validated()
        guard health == .blocked || coverage.isComplete,
              health != .healthy || blockingReasons.isEmpty,
              health != .blocked || !blockingReasons.isEmpty else {
            throw ProductProjectionValidationError.invalid(
                "project health, coverage, and blockers are inconsistent"
            )
        }
        try validateProductProjectionLinks(
            evidence: evidence,
            evidenceRevision: sourceRevision,
            actions: actions,
            actionRevision: sourceRevision
        )
        let evidenceIDs = Set(evidence.map(\.id))
        let actionIDs = Set(actions.map(\.id))
        let attentionIDs = Set(attention.map(\.id))
        let blockerIDs = Set(blockingReasons.map(\.id))
        guard attentionIDs.count == attention.count,
              blockerIDs.count == blockingReasons.count,
              Set(agents.map(\.agent)).count == agents.count else {
            throw ProductProjectionValidationError.invalid(
                "project readiness contains duplicate rows"
            )
        }
        for item in attention {
            try item.validated()
            try requireKnownProductIDs(
                item.evidenceRefs,
                known: evidenceIDs,
                allowEmpty: false,
                label: "attention evidence"
            )
            try requireKnownProductIDs(
                item.actionIDs,
                known: actionIDs,
                label: "attention actions"
            )
        }
        for blocker in blockingReasons {
            try blocker.validated()
            try requireKnownProductIDs(
                blocker.evidenceRefs,
                known: evidenceIDs,
                allowEmpty: false,
                label: "project blocker evidence"
            )
            try requireKnownProductIDs(
                blocker.actionIDs,
                known: actionIDs,
                label: "project blocker actions"
            )
        }
        for agent in agents {
            try agent.validated(
                evidenceIDs: evidenceIDs,
                actionIDs: actionIDs,
                attentionIDs: attentionIDs
            )
            guard agent.blockingReasons.allSatisfy({ blockerIDs.contains($0.id) }) else {
                throw ProductProjectionValidationError.invalid(
                    "agent blocker is missing from the project"
                )
            }
        }
        if !agents.isEmpty {
            guard try SourceCoverage.merged(agents.map(\.coverage)) == coverage else {
                throw ProductProjectionValidationError.invalid(
                    "project coverage does not match its agent rows"
                )
            }
        }
        guard recentSessions.allSatisfy({ $0.snapshotRevision == sourceRevision }) else {
            throw ProductProjectionValidationError.invalid(
                "recent session snapshot differs from project readiness"
            )
        }
    }
}
