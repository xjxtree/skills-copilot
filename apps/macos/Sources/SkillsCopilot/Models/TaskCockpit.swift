import Foundation

struct TaskCockpitSummaryTextRow: Identifiable, Hashable {
    struct ID: Hashable {
        let value: String
        let occurrence: Int
    }

    let id: ID
    let value: String

    static func rows(for values: [String]) -> [TaskCockpitSummaryTextRow] {
        var occurrences: [String: Int] = [:]
        return values.map { value in
            let occurrence = occurrences[value, default: 0]
            occurrences[value] = occurrence + 1
            return TaskCockpitSummaryTextRow(
                id: ID(value: value, occurrence: occurrence),
                value: value
            )
        }
    }

    static func matchingProcessValues(for result: TaskCockpitResult) -> [String] {
        var values: [String] = []
        if let topRoute = result.routeCandidates.first {
            values.append(contentsOf: topRoute.reasons)
        }
        values.append(contentsOf: result.gapRows.map(\.detail))
        values.append(contentsOf: result.blockerRows.map(\.detail))
        return values
    }
}

struct TaskCockpitDecisionModel {
    let result: TaskCockpitResult

    var keyReasons: [String] {
        var values = attentionRows.flatMap { row -> [String] in
            [
                Self.displayText(row.title),
                Self.displayText(row.detail)
            ].compactMap(\.self)
        }
        values.append(contentsOf: reasons)
        return Self.uniqueMeaningful(values)
    }

    var candidateAlternatives: [String] {
        guard uniqueCandidateRows.count > 1 else { return [] }
        return Array(uniqueCandidateRows.enumerated()).map { index, row in
            candidateAlternativeLine(index: index, row: row)
        }
    }

    var processNotes: [String] {
        TaskCockpitSummaryTextRow.matchingProcessValues(for: result)
            .compactMap(Self.displayText)
    }

    static func displayText(_ value: String) -> String? {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty,
              !isInternalBoundary(trimmed),
              !looksLikeRawStructuredPayload(trimmed)
        else { return nil }

        switch TaskCockpitSignalClassifier.normalizedToken(trimmed) {
        case "permissions.exec-needs-human":
            return UIStrings.taskCockpitReasonExecNeedsHuman
        case "permissions.network-declared":
            return UIStrings.taskCockpitReasonNetworkDeclared
        case "duplicate-name", "cross-agent-analysis":
            return nil
        default:
            return UIStrings.localizedServiceMessage(trimmed)
        }
    }

    private var reasons: [String] {
        var values: [String] = []
        values.append(result.summary.summaryText)
        if let topRoute = result.routeCandidates.first {
            values.append(topRoute.summary)
            values.append(contentsOf: topRoute.reasons)
        }
        if let topSkill = result.skillCandidates.first {
            values.append(topSkill.summary)
            values.append(contentsOf: topSkill.reasons)
        }
        values.append(contentsOf: result.readinessSignals.map(\.detail))
        values.append(contentsOf: result.agentCandidates.map(\.summary))
        values.append(contentsOf: result.agentCandidates.flatMap(\.reasons))
        return Self.uniqueMeaningful(values)
    }

    private var attentionRows: [TaskCockpitContextRow] {
        userBlockerRows + reviewRiskRows + result.gapRows
    }

    private var userBlockerRows: [TaskCockpitContextRow] {
        result.blockerRows.filter { row in
            !Self.isInternalBoundary(row) && !Self.isReviewOnlyRisk(row)
        }
    }

    private var reviewRiskRows: [TaskCockpitContextRow] {
        result.blockerRows.filter { row in
            !Self.isInternalBoundary(row) && Self.isReviewOnlyRisk(row)
        }
    }

    private var uniqueCandidateRows: [TaskCockpitCandidateRow] {
        let rows: [TaskCockpitCandidateRow]
        if !result.skillCandidates.isEmpty {
            rows = result.skillCandidates
        } else if !result.routeCandidates.isEmpty {
            rows = result.routeCandidates
        } else {
            rows = result.agentCandidates
        }
        var seen = Set<String>()
        return rows.filter { row in
            let name = row.skill?.name ?? row.title
            return seen.insert("\(row.agent ?? ""):\(name)".lowercased()).inserted
        }
    }

    private func candidateAlternativeLine(index: Int, row: TaskCockpitCandidateRow) -> String {
        let agent = row.agent.map(DisplayText.agent)
        let name = row.skill?.name ?? row.title
        let score = row.routingScore ?? row.readinessScore ?? row.score
        let scoreText = score.map { " · \(UIStrings.taskCockpitRoutingShort) \($0)" } ?? ""
        if let agent, !agent.isEmpty {
            return "\(index + 1). \(agent) · \(name)\(scoreText)"
        }
        return "\(index + 1). \(name)\(scoreText)"
    }

    private static func uniqueMeaningful(_ values: [String]) -> [String] {
        var seen = Set<String>()
        return values.compactMap { value in
            guard let display = displayText(value),
                  seen.insert(display.lowercased()).inserted
            else { return nil }
            return display
        }
    }

    private static func looksLikeRawStructuredPayload(_ value: String) -> Bool {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.hasPrefix("{")
            || trimmed.hasPrefix("[")
            || trimmed.hasPrefix("```")
            || trimmed.contains("\"agent_candidates\"")
            || trimmed.contains("\"skill_candidates\"")
            || trimmed.contains("\"route_candidates\"")
    }

    private static func isReviewOnlyRisk(_ row: TaskCockpitContextRow) -> Bool {
        TaskCockpitSignalClassifier.classification(for: row) == .reviewOnlyRisk
    }

    private static func isInternalBoundary(_ row: TaskCockpitContextRow) -> Bool {
        TaskCockpitSignalClassifier.classification(for: row) == .internalBoundary
    }

    private static func isInternalBoundary(_ value: String) -> Bool {
        TaskCockpitSignalClassifier.isInternalBoundaryToken(value)
    }
}

struct TaskCockpitOperationState: Hashable {
    enum Phase: String, Hashable {
        case idle
        case preparing
        case completed
        case fallback
        case timedOut
        case cancelled
        case failed
    }

    let phase: Phase
    let taskText: String
    let message: String
    let startedAt: Date?
    let finishedAt: Date?
    let timeoutSeconds: Int

    static let idle = TaskCockpitOperationState(
        phase: .idle,
        taskText: "",
        message: "",
        startedAt: nil,
        finishedAt: nil,
        timeoutSeconds: 0
    )

    var isPreparing: Bool {
        phase == .preparing
    }

    var canCancel: Bool {
        phase == .preparing
    }

    var canRetry: Bool {
        switch phase {
        case .fallback, .timedOut, .cancelled, .failed:
            return !taskText.isEmpty
        case .idle, .preparing, .completed:
            return false
        }
    }

    func elapsedSeconds(now: Date = Date()) -> Int {
        guard let startedAt else { return 0 }
        let end = finishedAt ?? now
        return max(0, Int(end.timeIntervalSince(startedAt).rounded(.down)))
    }

    static func preparing(taskText: String, startedAt: Date = Date(), timeoutSeconds: Int) -> TaskCockpitOperationState {
        TaskCockpitOperationState(
            phase: .preparing,
            taskText: taskText,
            message: UIStrings.taskCockpitPreparingStatus(elapsedSeconds: 0, timeoutSeconds: timeoutSeconds),
            startedAt: startedAt,
            finishedAt: nil,
            timeoutSeconds: timeoutSeconds
        )
    }

    func finished(phase: Phase, message: String, finishedAt: Date = Date()) -> TaskCockpitOperationState {
        TaskCockpitOperationState(
            phase: phase,
            taskText: taskText,
            message: message,
            startedAt: startedAt,
            finishedAt: finishedAt,
            timeoutSeconds: timeoutSeconds
        )
    }
}

enum TaskCockpitAgentScope {

    static func agentScopeSummary(_ agentIDs: [String]) -> String {
        let normalized = normalizedAgentIDs(agentIDs)
        let allAgents = SkillAgentFilter.managementCases.map(\.rawValue)
        if normalized.isEmpty || Set(normalized) == Set(allAgents) {
            return UIStrings.text("taskCockpit.agentScope.all", "All agents")
        }
        return normalized.map(DisplayText.agent).joined(separator: ", ")
    }

    static func normalizedAgentIDs(_ agentIDs: [String]) -> [String] {
        let allowed = Set(SkillAgentFilter.managementCases.map(\.rawValue))
        var seen = Set<String>()
        return agentIDs
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty && allowed.contains($0) }
            .filter { seen.insert($0).inserted }
    }
}

struct TaskCockpitAgentOption: Identifiable, Hashable {
    let id: String
    let title: String
    let effectiveSkillCount: Int

    var subtitle: String {
        String(
            format: UIStrings.text("taskCockpit.agentScope.skillCount", "%d active skills"),
            effectiveSkillCount
        )
    }
}

struct TaskCockpitFilters: Decodable, Hashable {
    let taskText: String
    let agent: String?
    let agents: [String]
    let selectedSkillID: String?
    let selectedSkillName: String?
    let selectedSkillAgent: String?
    let projectRoot: String?
    let currentCWD: String?
    let workspace: String?
    let limit: Int?
    let includeProviderObservability: Bool

    enum CodingKeys: String, CodingKey {
        case task
        case taskText = "task_text"
        case taskTextAlt = "taskText"
        case userIntent = "user_intent"
        case agent
        case agents
        case selectedSkillID = "selected_skill_id"
        case selectedSkillIDAlt = "selectedSkillID"
        case selectedSkillName = "selected_skill_name"
        case selectedSkillNameAlt = "selectedSkillName"
        case selectedSkillAgent = "selected_skill_agent"
        case selectedSkillAgentAlt = "selectedSkillAgent"
        case projectRoot = "project_root"
        case projectRootAlt = "projectRoot"
        case currentCWD = "current_cwd"
        case currentCWDAlt = "currentCWD"
        case workspace
        case workspaceID = "workspace_id"
        case limit
        case includeProviderObservability = "include_provider_observability"
        case includeProviderObservabilityAlt = "includeProviderObservability"
    }

    init(
        taskText: String = "",
        agent: String? = nil,
        agents: [String] = [],
        selectedSkillID: String? = nil,
        selectedSkillName: String? = nil,
        selectedSkillAgent: String? = nil,
        projectRoot: String? = nil,
        currentCWD: String? = nil,
        workspace: String? = nil,
        limit: Int? = nil,
        includeProviderObservability: Bool = true
    ) {
        self.taskText = taskText
        self.agent = agent
        self.agents = agents
        self.selectedSkillID = selectedSkillID
        self.selectedSkillName = selectedSkillName
        self.selectedSkillAgent = selectedSkillAgent
        self.projectRoot = projectRoot
        self.currentCWD = currentCWD
        self.workspace = workspace
        self.limit = limit
        self.includeProviderObservability = includeProviderObservability
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        taskText = try container.decodeFlexibleTaskCockpitString(keys: [.task, .taskText, .taskTextAlt, .userIntent]) ?? ""
        agent = try container.decodeFlexibleTaskCockpitString(keys: [.agent])
        agents = try container.decodeFlexibleTaskCockpitStringArray(keys: [.agents, .agent])
        selectedSkillID = try container.decodeFlexibleTaskCockpitString(keys: [.selectedSkillID, .selectedSkillIDAlt])
        selectedSkillName = try container.decodeFlexibleTaskCockpitString(keys: [.selectedSkillName, .selectedSkillNameAlt])
        selectedSkillAgent = try container.decodeFlexibleTaskCockpitString(keys: [.selectedSkillAgent, .selectedSkillAgentAlt])
        projectRoot = try container.decodeFlexibleTaskCockpitString(keys: [.projectRoot, .projectRootAlt])
        currentCWD = try container.decodeFlexibleTaskCockpitString(keys: [.currentCWD, .currentCWDAlt])
        workspace = try container.decodeFlexibleTaskCockpitString(keys: [.workspace, .workspaceID])
        limit = try container.decodeFlexibleTaskCockpitInt(keys: [.limit])
        includeProviderObservability = try container.decodeFlexibleTaskCockpitBool(keys: [.includeProviderObservability, .includeProviderObservabilityAlt]) ?? true
    }
}

struct TaskCockpitSummary: Decodable, Hashable {
    let taskText: String
    let summaryText: String
    let routeCandidateCount: Int
    let agentCandidateCount: Int
    let skillCandidateCount: Int
    let readinessSignalCount: Int
    let providerCallCount: Int
    let gapCount: Int
    let blockerCount: Int
    let evidenceCount: Int
    let safetyFlagCount: Int
    let recommendedAgent: String?
    let recommendedSkillName: String?
    let readinessScore: Int?
    let routingScore: Int?

    enum CodingKeys: String, CodingKey {
        case task
        case taskText = "task_text"
        case taskTextAlt = "taskText"
        case userIntent = "user_intent"
        case summary
        case message
        case text
        case routeCandidateCount = "route_candidate_count"
        case routeCandidateCountAlt = "routeCandidateCount"
        case routeCount = "route_count"
        case routes
        case agentCandidateCount = "agent_candidate_count"
        case agentCandidateCountAlt = "agentCandidateCount"
        case agentCount = "agent_count"
        case agents
        case skillCandidateCount = "skill_candidate_count"
        case skillCandidateCountAlt = "skillCandidateCount"
        case candidateSkillCount = "candidate_skill_count"
        case candidateCount = "candidate_count"
        case skills
        case readinessSignalCount = "readiness_signal_count"
        case readinessSignalCountAlt = "readinessSignalCount"
        case readinessSignals = "readiness_signals"
        case providerCallCount = "provider_call_count"
        case providerCallCountAlt = "providerCallCount"
        case providerObservabilityRowCount = "provider_observability_row_count"
        case providerCalls = "provider_calls"
        case gapCount = "gap_count"
        case gaps
        case blockerCount = "blocker_count"
        case blockers
        case evidenceCount = "evidence_count"
        case evidence
        case evidenceReferences = "evidence_references"
        case safetyFlagCount = "safety_flag_count"
        case safetyFlags = "safety_flags"
        case recommendedAgent = "recommended_agent"
        case recommendedAgentAlt = "recommendedAgent"
        case recommendedSkillName = "recommended_skill_name"
        case recommendedSkillNameAlt = "recommendedSkillName"
        case topSkillName = "top_skill_name"
        case readinessScore = "readiness_score"
        case readinessScoreAlt = "readinessScore"
        case routingScore = "routing_score"
        case routingScoreAlt = "routingScore"
        case confidenceScore = "confidence_score"
    }

    init(
        taskText: String = "",
        summaryText: String = "",
        routeCandidateCount: Int = 0,
        agentCandidateCount: Int = 0,
        skillCandidateCount: Int = 0,
        readinessSignalCount: Int = 0,
        providerCallCount: Int = 0,
        gapCount: Int = 0,
        blockerCount: Int = 0,
        evidenceCount: Int = 0,
        safetyFlagCount: Int = 0,
        recommendedAgent: String? = nil,
        recommendedSkillName: String? = nil,
        readinessScore: Int? = nil,
        routingScore: Int? = nil
    ) {
        self.taskText = taskText
        self.summaryText = summaryText
        self.routeCandidateCount = routeCandidateCount
        self.agentCandidateCount = agentCandidateCount
        self.skillCandidateCount = skillCandidateCount
        self.readinessSignalCount = readinessSignalCount
        self.providerCallCount = providerCallCount
        self.gapCount = gapCount
        self.blockerCount = blockerCount
        self.evidenceCount = evidenceCount
        self.safetyFlagCount = safetyFlagCount
        self.recommendedAgent = recommendedAgent
        self.recommendedSkillName = recommendedSkillName
        self.readinessScore = readinessScore
        self.routingScore = routingScore
    }

    init(from decoder: Decoder) throws {
        if let value = try? decoder.singleValueContainer().decode(String.self) {
            self.init(summaryText: value)
            return
        }

        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init(
            taskText: try container.decodeFlexibleTaskCockpitString(keys: [.task, .taskText, .taskTextAlt, .userIntent]) ?? "",
            summaryText: try container.decodeFlexibleTaskCockpitString(keys: [.summary, .message, .text]) ?? "",
            routeCandidateCount: try container.decodeFlexibleTaskCockpitInt(keys: [.routeCandidateCount, .routeCandidateCountAlt, .routeCount, .routes]) ?? 0,
            agentCandidateCount: try container.decodeFlexibleTaskCockpitInt(keys: [.agentCandidateCount, .agentCandidateCountAlt, .agentCount, .agents]) ?? 0,
            skillCandidateCount: try container.decodeFlexibleTaskCockpitInt(keys: [.skillCandidateCount, .skillCandidateCountAlt, .candidateSkillCount, .candidateCount, .skills]) ?? 0,
            readinessSignalCount: try container.decodeFlexibleTaskCockpitInt(keys: [.readinessSignalCount, .readinessSignalCountAlt, .readinessSignals]) ?? 0,
            providerCallCount: try container.decodeFlexibleTaskCockpitInt(keys: [.providerCallCount, .providerCallCountAlt, .providerObservabilityRowCount, .providerCalls]) ?? 0,
            gapCount: try container.decodeFlexibleTaskCockpitInt(keys: [.gapCount, .gaps]) ?? 0,
            blockerCount: try container.decodeFlexibleTaskCockpitInt(keys: [.blockerCount, .blockers]) ?? 0,
            evidenceCount: try container.decodeFlexibleTaskCockpitInt(keys: [.evidenceCount, .evidence, .evidenceReferences]) ?? 0,
            safetyFlagCount: try container.decodeFlexibleTaskCockpitInt(keys: [.safetyFlagCount, .safetyFlags]) ?? 0,
            recommendedAgent: try container.decodeFlexibleTaskCockpitString(keys: [.recommendedAgent, .recommendedAgentAlt]),
            recommendedSkillName: try container.decodeFlexibleTaskCockpitString(keys: [.recommendedSkillName, .recommendedSkillNameAlt, .topSkillName]),
            readinessScore: try container.decodeFlexibleTaskCockpitInt(keys: [.readinessScore, .readinessScoreAlt]),
            routingScore: try container.decodeFlexibleTaskCockpitInt(keys: [.routingScore, .routingScoreAlt, .confidenceScore])
        )
    }
}

struct TaskCockpitCandidateRow: Decodable, Hashable, Identifiable {
    let id: String
    let rank: Int?
    let title: String
    let agent: String?
    let skill: TaskSkillRef?
    let readinessScore: Int?
    let routingScore: Int?
    let score: Int?
    let band: String?
    let status: String?
    let summary: String
    let reasons: [String]
    let evidenceRefs: [String]
    let safetyFlags: [String]

    enum CodingKeys: String, CodingKey {
        case id
        case routeID = "route_id"
        case agentID = "agent_id"
        case skillID = "skill_id"
        case instanceID = "instance_id"
        case rank
        case position
        case title
        case name
        case label
        case task
        case displayName = "display_name"
        case displayNameAlt = "displayName"
        case skillName = "skill_name"
        case skillNameAlt = "skillName"
        case bestSkillName = "best_skill_name"
        case bestSkillNameAlt = "bestSkillName"
        case definitionID = "definition_id"
        case definitionIDAlt = "definitionId"
        case agent
        case skill
        case candidateSkill = "candidate_skill"
        case route
        case readinessScore = "readiness_score"
        case readinessScoreAlt = "readinessScore"
        case routingScore = "routing_score"
        case routingScoreAlt = "routingScore"
        case confidenceScore = "confidence_score"
        case comparisonScore = "comparison_score"
        case comparisonScoreAlt = "comparisonScore"
        case score
        case value
        case band
        case readinessBand = "readiness_band"
        case confidenceBand = "confidence_band"
        case status
        case state
        case enabled
        case scope
        case summary
        case detail
        case rationale
        case reasons
        case reason
        case matchReasons = "match_reasons"
        case blockerNotes = "blocker_notes"
        case gapNotes = "gap_notes"
        case evidenceRefs = "evidence_refs"
        case evidenceRefsAlt = "evidenceRefs"
        case evidence
        case safetyFlags = "safety_flags"
        case safetyFlagsAlt = "safetyFlags"
        case flags
    }

    init(
        id: String,
        rank: Int? = nil,
        title: String,
        agent: String? = nil,
        skill: TaskSkillRef? = nil,
        readinessScore: Int? = nil,
        routingScore: Int? = nil,
        score: Int? = nil,
        band: String? = nil,
        status: String? = nil,
        summary: String = "",
        reasons: [String] = [],
        evidenceRefs: [String] = [],
        safetyFlags: [String] = []
    ) {
        self.id = id
        self.rank = rank
        self.title = title
        self.agent = agent
        self.skill = skill
        self.readinessScore = readinessScore
        self.routingScore = routingScore
        self.score = score
        self.band = band
        self.status = status
        self.summary = summary
        self.reasons = reasons
        self.evidenceRefs = evidenceRefs
        self.safetyFlags = safetyFlags
    }

    init(from decoder: Decoder) throws {
        if let value = try? decoder.singleValueContainer().decode(String.self) {
            self.init(id: value, title: value)
            return
        }

        let container = try decoder.container(keyedBy: CodingKeys.self)
        let topLevelSkillName = try container.decodeFlexibleTaskCockpitString(keys: [.skillName, .skillNameAlt, .bestSkillName, .bestSkillNameAlt])
        let topLevelInstanceID = try container.decodeFlexibleTaskCockpitString(keys: [.instanceID, .skillID])
        let decodedAgent = try container.decodeFlexibleTaskCockpitString(keys: [.agent, .agentID])
        let topLevelDefinitionID = try container.decodeFlexibleTaskCockpitString(keys: [.definitionID, .definitionIDAlt])
        let topLevelSkill = topLevelSkillName.map {
            TaskSkillRef(
                instanceID: topLevelInstanceID,
                name: $0,
                agent: decodedAgent ?? UIStrings.unknown,
                definitionID: topLevelDefinitionID
            )
        }
        let decodedSkill = try container.decodeIfPresent(TaskSkillRef.self, forKey: .skill)
            ?? container.decodeIfPresent(TaskSkillRef.self, forKey: .candidateSkill)
            ?? container.decodeIfPresent(TaskSkillRef.self, forKey: .route)
            ?? topLevelSkill
        let decodedTitle = try container.decodeFlexibleTaskCockpitString(keys: [.title, .name, .label, .task, .skillName, .skillNameAlt, .bestSkillName, .bestSkillNameAlt, .displayName, .displayNameAlt])
            ?? decodedSkill?.name
            ?? UIStrings.unknown
        let rowAgent = decodedAgent ?? decodedSkill?.agent
        self.init(
            id: try container.decodeFlexibleTaskCockpitString(keys: [.id, .routeID, .agentID, .skillID, .instanceID]) ?? "\(decodedAgent ?? "candidate")-\(decodedTitle)",
            rank: try container.decodeFlexibleTaskCockpitInt(keys: [.rank, .position]),
            title: decodedTitle,
            agent: rowAgent,
            skill: decodedSkill,
            readinessScore: try container.decodeFlexibleTaskCockpitInt(keys: [.readinessScore, .readinessScoreAlt]),
            routingScore: try container.decodeFlexibleTaskCockpitInt(keys: [.routingScore, .routingScoreAlt, .confidenceScore]),
            score: try container.decodeFlexibleTaskCockpitInt(keys: [.score, .value, .comparisonScore, .comparisonScoreAlt]),
            band: try container.decodeFlexibleTaskCockpitString(keys: [.band, .readinessBand, .confidenceBand]),
            status: try container.decodeFlexibleTaskCockpitString(keys: [.status, .state, .enabled]),
            summary: try container.decodeFlexibleTaskCockpitString(keys: [.summary, .detail, .rationale, .scope]) ?? "",
            reasons: try container.decodeFlexibleTaskCockpitStringArray(keys: [.reasons, .reason, .matchReasons]),
            evidenceRefs: try container.decodeFlexibleTaskCockpitStringArray(keys: [.evidenceRefs, .evidenceRefsAlt, .evidence]),
            safetyFlags: try container.decodeFlexibleTaskCockpitStringArray(keys: [.safetyFlags, .safetyFlagsAlt, .flags, .blockerNotes, .gapNotes])
        )
    }
}

struct TaskCockpitContextRow: Decodable, Hashable, Identifiable {
    let id: String
    let title: String
    let detail: String
    let status: String?
    let severity: String?
    let source: String?
    let agent: String?
    let count: Int?
    let evidenceRefs: [String]
    let safetyFlags: [String]

    enum CodingKeys: String, CodingKey {
        case id
        case rowID = "row_id"
        case title
        case name
        case label
        case task
        case detail
        case summary
        case message
        case suggestedSafeNextAction = "suggested_safe_next_action"
        case suggestedSafeNextActionAlt = "suggestedSafeNextAction"
        case status
        case outcome
        case severity
        case priority
        case source
        case sourceMethod = "source_method"
        case rowType = "row_type"
        case agent
        case count
        case total
        case rowCount = "row_count"
        case rowCountAlt = "rowCount"
        case evidenceRefs = "evidence_refs"
        case evidenceRefsAlt = "evidenceRefs"
        case evidence
        case safetyFlags = "safety_flags"
        case safetyFlagsAlt = "safetyFlags"
        case flags
    }

    init(
        id: String,
        title: String,
        detail: String = "",
        status: String? = nil,
        severity: String? = nil,
        source: String? = nil,
        agent: String? = nil,
        count: Int? = nil,
        evidenceRefs: [String] = [],
        safetyFlags: [String] = []
    ) {
        self.id = id
        self.title = title
        self.detail = detail
        self.status = status
        self.severity = severity
        self.source = source
        self.agent = agent
        self.count = count
        self.evidenceRefs = evidenceRefs
        self.safetyFlags = safetyFlags
    }

    init(from decoder: Decoder) throws {
        if let value = try? decoder.singleValueContainer().decode(String.self) {
            self.init(id: value, title: value)
            return
        }

        let container = try decoder.container(keyedBy: CodingKeys.self)
        let decodedTitle = try container.decodeFlexibleTaskCockpitString(keys: [.title, .name, .label, .task]) ?? UIStrings.unknown
        self.init(
            id: try container.decodeFlexibleTaskCockpitString(keys: [.id, .rowID]) ?? decodedTitle,
            title: decodedTitle,
            detail: try container.decodeFlexibleTaskCockpitString(keys: [.detail, .summary, .message, .suggestedSafeNextAction, .suggestedSafeNextActionAlt]) ?? "",
            status: try container.decodeFlexibleTaskCockpitString(keys: [.status, .outcome]),
            severity: try container.decodeFlexibleTaskCockpitString(keys: [.severity, .priority]),
            source: try container.decodeFlexibleTaskCockpitString(keys: [.source, .sourceMethod, .rowType]),
            agent: try container.decodeFlexibleTaskCockpitString(keys: [.agent]),
            count: try container.decodeFlexibleTaskCockpitInt(keys: [.count, .total, .rowCount, .rowCountAlt]),
            evidenceRefs: try container.decodeFlexibleTaskCockpitStringArray(keys: [.evidenceRefs, .evidenceRefsAlt, .evidence]),
            safetyFlags: try container.decodeFlexibleTaskCockpitStringArray(keys: [.safetyFlags, .safetyFlagsAlt, .flags])
        )
    }
}

enum TaskCockpitSignalClassification: Equatable {
    case userFacing
    case reviewOnlyRisk
    case internalBoundary
}

enum TaskCockpitSignalClassifier {
    static func classification(for row: TaskCockpitContextRow) -> TaskCockpitSignalClassification {
        let tokens = signalTokens(for: row)
        if !tokens.isDisjoint(with: internalBoundaryTokens) {
            return .internalBoundary
        }
        if !tokens.isDisjoint(with: reviewOnlyRiskTokens) {
            return .reviewOnlyRisk
        }
        return .userFacing
    }

    static func isInternalBoundaryToken(_ value: String) -> Bool {
        internalBoundaryTokens.contains(normalizedToken(value))
    }

    static func normalizedToken(_ value: String) -> String {
        var normalized = value.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        for separator in ["_", " ", "/", ":", "`"] {
            normalized = normalized.replacingOccurrences(of: separator, with: "-")
        }
        while normalized.contains("--") {
            normalized = normalized.replacingOccurrences(of: "--", with: "-")
        }
        return normalized.trimmingCharacters(in: CharacterSet(charactersIn: "-."))
    }

    private static func signalTokens(for row: TaskCockpitContextRow) -> Set<String> {
        var tokens = Set<String>()
        for value in [row.id, row.status, row.severity, row.source].compactMap(\.self) {
            tokens.formUnion(signalTokenVariants(for: value))
        }
        for value in row.evidenceRefs + row.safetyFlags {
            tokens.formUnion(signalTokenVariants(for: value))
        }
        return tokens
    }

    private static func signalTokenVariants(for value: String) -> Set<String> {
        var tokens = Set<String>()
        tokens.insert(normalizedToken(value))
        for separator in [":", "|", "#"] {
            let parts = value.split(separator: Character(separator), omittingEmptySubsequences: true)
            if parts.count > 1 {
                tokens.insert(normalizedToken(String(parts.last ?? "")))
            }
        }
        return tokens.filter { !$0.isEmpty }
    }

    private static let reviewOnlyRiskTokens: Set<String> = [
        "permissions.exec-needs-human",
        "permissions.network-declared",
        "exec-needs-human",
        "network-declared",
        "requires-confirmation",
        "network-access"
    ]

    private static let internalBoundaryTokens: Set<String> = [
        "no-apply-path",
        "read-only",
        "readonly",
        "read-only-preflight",
        "preview-only",
        "copy-only",
        "provider-not-sent",
        "task-cockpit-combined",
        "cockpit-only",
        "evaluated-top",
        "matched-task-term",
        "description-evidence",
        "top-route-leads",
        "one-visible-route-candidate",
        "no-candidate-level-blockers",
        "no-likely-wrong-pick-risk",
        "skipped-by-filters",
        "provider-observability-skipped",
        "write-action",
        "script-execution",
        "snapshot",
        "telemetry",
        "cross-agent-analysis",
        "duplicate-name",
        "duplicate_name",
        "cross-agent-duplicate",
        "source-overlap",
        "same-name",
        "overlap-signals"
    ]
}

struct TaskCockpitResult: Decodable, Hashable {
    let generatedBy: String
    let catalogAvailable: Bool
    let filters: TaskCockpitFilters
    let summary: TaskCockpitSummary
    let cockpitSections: [TaskCockpitContextRow]
    let taskRows: [TaskCockpitCandidateRow]
    let routeCandidates: [TaskCockpitCandidateRow]
    let agentCandidates: [TaskCockpitCandidateRow]
    let skillCandidates: [TaskCockpitCandidateRow]
    let readinessSignals: [TaskCockpitContextRow]
    let providerObservabilityContext: [TaskCockpitContextRow]
    let gapRows: [TaskCockpitContextRow]
    let blockerRows: [TaskCockpitContextRow]
    let evidenceReferences: [ProviderObservabilityEvidenceReference]
    let promptRequest: ProviderObservabilityPromptRequest?
    let aggregation: TaskCockpitAggregation?
    let safetyFlags: ProviderObservabilitySafety
    let fallbackReason: String?

    var isUnavailable: Bool {
        generatedBy == "unavailable" || fallbackReason != nil && routeCandidates.isEmpty && agentCandidates.isEmpty && skillCandidates.isEmpty
    }

    var recoveryDiagnosticReason: String? {
        if let fallbackReason, !fallbackReason.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            return fallbackReason
        }
        if !catalogAvailable {
            return UIStrings.taskCockpitCatalogUnavailableDiagnostic
        }
        if hasNoReturnedRows {
            return UIStrings.taskCockpitPartialNoRows
        }
        return nil
    }

    var agentScopeIDs: [String] {
        TaskCockpitAgentScope.normalizedAgentIDs(filters.agents.isEmpty ? filters.agent.map { [$0] } ?? [] : filters.agents)
    }

    private var hasNoReturnedRows: Bool {
        routeCandidates.isEmpty
            && agentCandidates.isEmpty
            && skillCandidates.isEmpty
            && readinessSignals.isEmpty
            && providerObservabilityContext.isEmpty
            && gapRows.isEmpty
            && blockerRows.isEmpty
            && evidenceReferences.isEmpty
    }

    enum CodingKeys: String, CodingKey {
        case generatedBy = "generated_by"
        case generatedByAlt = "generatedBy"
        case catalogAvailable = "catalog_available"
        case catalogAvailableAlt = "catalogAvailable"
        case filters
        case summary
        case taskSummary = "task_summary"
        case cockpitSections = "cockpit_sections"
        case cockpitSectionsAlt = "cockpitSections"
        case sections
        case taskRows = "task_rows"
        case taskRowsAlt = "taskRows"
        case routeCandidates = "route_candidates"
        case routeCandidatesAlt = "routeCandidates"
        case routes
        case candidateRoutes = "candidate_routes"
        case agentCandidates = "agent_candidates"
        case agentCandidatesAlt = "agentCandidates"
        case agentRows = "agent_rows"
        case agentRouteRows = "agent_route_rows"
        case agents
        case skillCandidates = "skill_candidates"
        case skillCandidatesAlt = "skillCandidates"
        case skillCandidateRows = "skill_candidate_rows"
        case candidateSkills = "candidate_skills"
        case skills
        case readinessSignals = "readiness_signals"
        case readinessSignalsAlt = "readinessSignals"
        case readinessRows = "readiness_rows"
        case readiness
        case signals
        case providerObservabilityContext = "provider_observability_context"
        case providerObservabilityContextAlt = "providerObservabilityContext"
        case providerRows = "provider_rows"
        case providerObservabilityRows = "provider_observability_rows"
        case gapRows = "gap_rows"
        case gapNotes = "gap_notes"
        case gaps
        case blockerRows = "blocker_rows"
        case blockerNotes = "blocker_notes"
        case blockers
        case evidenceReferences = "evidence_references"
        case evidenceReferencesAlt = "evidenceReferences"
        case evidence
        case promptRequest = "prompt_request"
        case promptRequestAlt = "promptRequest"
        case aggregation
        case safetyFlags = "safety_flags"
        case safetyFlagsAlt = "safetyFlags"
        case safety
        case fallbackReason = "fallback_reason"
        case reason
    }

    init(
        generatedBy: String = "local-v2.73",
        catalogAvailable: Bool = true,
        filters: TaskCockpitFilters = TaskCockpitFilters(),
        summary: TaskCockpitSummary = TaskCockpitSummary(),
        cockpitSections: [TaskCockpitContextRow] = [],
        taskRows: [TaskCockpitCandidateRow] = [],
        routeCandidates: [TaskCockpitCandidateRow] = [],
        agentCandidates: [TaskCockpitCandidateRow] = [],
        skillCandidates: [TaskCockpitCandidateRow] = [],
        readinessSignals: [TaskCockpitContextRow] = [],
        providerObservabilityContext: [TaskCockpitContextRow] = [],
        gapRows: [TaskCockpitContextRow] = [],
        blockerRows: [TaskCockpitContextRow] = [],
        evidenceReferences: [ProviderObservabilityEvidenceReference] = [],
        promptRequest: ProviderObservabilityPromptRequest? = nil,
        aggregation: TaskCockpitAggregation? = nil,
        safetyFlags: ProviderObservabilitySafety = ProviderObservabilitySafety(),
        fallbackReason: String? = nil
    ) {
        self.generatedBy = generatedBy
        self.catalogAvailable = catalogAvailable
        self.filters = filters
        self.summary = summary
        self.cockpitSections = cockpitSections
        self.taskRows = taskRows
        self.routeCandidates = routeCandidates
        self.agentCandidates = agentCandidates
        self.skillCandidates = skillCandidates
        self.readinessSignals = readinessSignals
        self.providerObservabilityContext = providerObservabilityContext
        self.gapRows = gapRows
        self.blockerRows = blockerRows
        self.evidenceReferences = evidenceReferences
        self.promptRequest = promptRequest
        self.aggregation = aggregation
        self.safetyFlags = safetyFlags
        self.fallbackReason = fallbackReason
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init(
            generatedBy: try container.decodeFlexibleTaskCockpitString(keys: [.generatedBy, .generatedByAlt]) ?? "local-v2.73",
            catalogAvailable: try container.decodeFlexibleTaskCockpitBool(keys: [.catalogAvailable, .catalogAvailableAlt]) ?? true,
            filters: try container.decodeIfPresent(TaskCockpitFilters.self, forKey: .filters) ?? TaskCockpitFilters(),
            summary: try container.decodeIfPresent(TaskCockpitSummary.self, forKey: .summary)
                ?? container.decodeIfPresent(TaskCockpitSummary.self, forKey: .taskSummary)
                ?? TaskCockpitSummary(),
            cockpitSections: try container.decodeFlexibleTaskCockpitContextRows(keys: [.cockpitSections, .cockpitSectionsAlt, .sections]),
            taskRows: try container.decodeFlexibleTaskCockpitRows(keys: [.taskRows, .taskRowsAlt]),
            routeCandidates: try container.decodeFlexibleTaskCockpitRows(keys: [.routeCandidates, .routeCandidatesAlt, .routes, .candidateRoutes]),
            agentCandidates: try container.decodeFlexibleTaskCockpitRows(keys: [.agentCandidates, .agentCandidatesAlt, .agentRows, .agentRouteRows, .agents]),
            skillCandidates: try container.decodeFlexibleTaskCockpitRows(keys: [.skillCandidates, .skillCandidatesAlt, .skillCandidateRows, .candidateSkills, .skills]),
            readinessSignals: try container.decodeFlexibleTaskCockpitContextRows(keys: [.readinessSignals, .readinessSignalsAlt, .readinessRows, .readiness, .signals]),
            providerObservabilityContext: try container.decodeFlexibleTaskCockpitContextRows(keys: [.providerObservabilityContext, .providerObservabilityContextAlt, .providerRows, .providerObservabilityRows]),
            gapRows: try container.decodeFlexibleTaskCockpitContextRows(keys: [.gapRows, .gapNotes, .gaps]),
            blockerRows: try container.decodeFlexibleTaskCockpitContextRows(keys: [.blockerRows, .blockerNotes, .blockers]),
            evidenceReferences: try container.decodeFlexibleTaskCockpitEvidence(keys: [.evidenceReferences, .evidenceReferencesAlt, .evidence]),
            promptRequest: try container.decodeIfPresent(ProviderObservabilityPromptRequest.self, forKey: .promptRequest)
                ?? container.decodeIfPresent(ProviderObservabilityPromptRequest.self, forKey: .promptRequestAlt),
            aggregation: try container.decodeIfPresent(TaskCockpitAggregation.self, forKey: .aggregation),
            safetyFlags: try container.decodeIfPresent(ProviderObservabilitySafety.self, forKey: .safetyFlags)
                ?? container.decodeIfPresent(ProviderObservabilitySafety.self, forKey: .safetyFlagsAlt)
                ?? container.decodeIfPresent(ProviderObservabilitySafety.self, forKey: .safety)
                ?? ProviderObservabilitySafety(),
            fallbackReason: try container.decodeFlexibleTaskCockpitString(keys: [.fallbackReason, .reason])
        )
    }

    static func unavailable(taskText: String = "", reason: String = UIStrings.taskCockpitUnavailable) -> TaskCockpitResult {
        TaskCockpitResult(
            generatedBy: "unavailable",
            catalogAvailable: false,
            filters: TaskCockpitFilters(taskText: taskText),
            summary: TaskCockpitSummary(taskText: taskText, summaryText: reason),
            safetyFlags: ProviderObservabilitySafety(notes: [reason]),
            fallbackReason: reason
        )
    }
}

enum TaskCockpitProviderOutputParser {
    static func result(
        from envelope: AIResponseEnvelopeWire?,
        taskText: String,
        agentIDs: [String]
    ) -> TaskCockpitResult {
        guard let envelope,
              let data = try? JSONEncoder().encode(envelope.result),
              let output = String(data: data, encoding: .utf8) else {
            return result(
                from: Optional<String>.none,
                taskText: taskText,
                agentIDs: agentIDs
            )
        }
        return looseResult(
            from: data,
            taskText: taskText.trimmingCharacters(in: .whitespacesAndNewlines),
            agentIDs: TaskCockpitAgentScope.normalizedAgentIDs(agentIDs)
        ) ?? result(from: output, taskText: taskText, agentIDs: agentIDs)
    }

    static func result(from outputText: String?, taskText: String, agentIDs: [String]) -> TaskCockpitResult {
        let normalizedTask = taskText.trimmingCharacters(in: .whitespacesAndNewlines)
        let normalizedAgents = TaskCockpitAgentScope.normalizedAgentIDs(agentIDs)
        guard let outputText, !outputText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return fallbackResult(
                taskText: normalizedTask,
                agentIDs: normalizedAgents,
                reason: UIStrings.text("taskCockpit.provider.empty", "The provider returned an empty task-readiness response.")
            )
        }

        if let data = extractedJSONData(from: outputText),
           let decoded = try? JSONDecoder().decode(TaskCockpitResult.self, from: data) {
            return normalized(decoded, taskText: normalizedTask, agentIDs: normalizedAgents)
        }
        if let data = extractedJSONData(from: outputText),
           let loose = looseResult(from: data, taskText: normalizedTask, agentIDs: normalizedAgents) {
            return loose
        }

        return fallbackResult(
            taskText: normalizedTask,
            agentIDs: normalizedAgents,
            reason: UIStrings.taskCockpitProviderUnparsed
        )
    }

    private static func normalized(_ result: TaskCockpitResult, taskText: String, agentIDs: [String]) -> TaskCockpitResult {
        let inferredTopSkill = result.skillCandidates.first ?? result.routeCandidates.first
        let inferredAgent = inferredTopSkill?.agent ?? result.agentCandidates.first?.agent
        let filters = TaskCockpitFilters(
            taskText: result.filters.taskText.isEmpty ? taskText : result.filters.taskText,
            agent: result.filters.agent,
            agents: result.filters.agents.isEmpty ? agentIDs : result.filters.agents,
            selectedSkillID: result.filters.selectedSkillID,
            selectedSkillName: result.filters.selectedSkillName,
            selectedSkillAgent: result.filters.selectedSkillAgent,
            projectRoot: result.filters.projectRoot,
            currentCWD: result.filters.currentCWD,
            workspace: result.filters.workspace,
            limit: result.filters.limit,
            includeProviderObservability: result.filters.includeProviderObservability
        )
        let summary = TaskCockpitSummary(
            taskText: result.summary.taskText.isEmpty ? taskText : result.summary.taskText,
            summaryText: result.summary.summaryText,
            routeCandidateCount: max(result.summary.routeCandidateCount, result.routeCandidates.count),
            agentCandidateCount: max(result.summary.agentCandidateCount, result.agentCandidates.count),
            skillCandidateCount: max(result.summary.skillCandidateCount, result.skillCandidates.count),
            readinessSignalCount: max(result.summary.readinessSignalCount, result.readinessSignals.count),
            providerCallCount: result.summary.providerCallCount,
            gapCount: max(result.summary.gapCount, result.gapRows.count),
            blockerCount: max(result.summary.blockerCount, result.blockerRows.count),
            evidenceCount: max(result.summary.evidenceCount, result.evidenceReferences.count),
            safetyFlagCount: result.summary.safetyFlagCount,
            recommendedAgent: result.summary.recommendedAgent ?? inferredAgent,
            recommendedSkillName: result.summary.recommendedSkillName
                ?? inferredTopSkill?.skill?.name
                ?? (inferredTopSkill?.agent == nil ? inferredTopSkill?.title : nil),
            readinessScore: result.summary.readinessScore ?? inferredTopSkill?.readinessScore,
            routingScore: result.summary.routingScore ?? inferredTopSkill?.routingScore ?? inferredTopSkill?.score
        )
        return TaskCockpitResult(
            generatedBy: result.generatedBy.isEmpty ? "provider-task-cockpit" : result.generatedBy,
            catalogAvailable: result.catalogAvailable,
            filters: filters,
            summary: summary,
            cockpitSections: result.cockpitSections,
            taskRows: result.taskRows,
            routeCandidates: result.routeCandidates,
            agentCandidates: result.agentCandidates,
            skillCandidates: result.skillCandidates,
            readinessSignals: result.readinessSignals,
            providerObservabilityContext: result.providerObservabilityContext,
            gapRows: result.gapRows,
            blockerRows: result.blockerRows,
            evidenceReferences: result.evidenceReferences,
            promptRequest: result.promptRequest,
            aggregation: result.aggregation,
            safetyFlags: result.safetyFlags,
            fallbackReason: result.fallbackReason
        )
    }

    private static func fallbackResult(taskText: String, agentIDs: [String], reason: String) -> TaskCockpitResult {
        TaskCockpitResult(
            generatedBy: "provider-task-cockpit",
            catalogAvailable: true,
            filters: TaskCockpitFilters(taskText: taskText, agents: agentIDs),
            summary: TaskCockpitSummary(
                taskText: taskText,
                summaryText: reason,
                readinessScore: 0,
                routingScore: 0
            ),
            readinessSignals: [
                TaskCockpitContextRow(
                    id: "provider-output",
                    title: UIStrings.text("taskCockpit.provider.output", "Provider output"),
                    detail: reason,
                    status: "review",
                    source: "llm.confirmPromptAndSend"
                )
            ],
            safetyFlags: ProviderObservabilitySafety(providerRequestSent: true),
            fallbackReason: nil
        )
    }

    private static func looseResult(from data: Data, taskText: String, agentIDs: [String]) -> TaskCockpitResult? {
        guard let raw = try? JSONSerialization.jsonObject(with: data),
              let object = raw as? [String: Any]
        else { return nil }
        let payload = dictionaryValue(object, keys: ["result"]) ?? object
        let summaryObject = dictionaryValue(payload, keys: ["summary", "task_summary", "taskSummary"])
        let routeCandidates = looseCandidateRows(
            firstValue(payload, keys: ["route_candidates", "routeCandidates", "routes", "candidate_routes", "candidateRoutes"]),
            kind: .route
        )
        let agentCandidates = looseCandidateRows(
            firstValue(payload, keys: ["agent_candidates", "agentCandidates", "agent_rows", "agentRows", "agent_route_rows", "agentRouteRows", "agents"]),
            kind: .agent
        )
        let skillCandidates = looseCandidateRows(
            firstValue(payload, keys: ["skill_candidates", "skillCandidates", "skill_candidate_rows", "skillCandidateRows", "candidate_skills", "candidateSkills", "skills"]),
            kind: .skill
        )
        let readinessRows = looseContextRows(
            firstValue(payload, keys: ["readiness_signals", "readinessSignals", "readiness_rows", "readinessRows", "readiness", "signals"]),
            fallbackIDPrefix: "signal"
        )
        let gapRows = looseContextRows(
            firstValue(payload, keys: ["gap_rows", "gapRows", "gap_notes", "gapNotes", "gaps"]),
            fallbackIDPrefix: "gap"
        )
        let blockerRows = looseContextRows(
            firstValue(payload, keys: ["blocker_rows", "blockerRows", "blocker_notes", "blockerNotes", "blockers"]),
            fallbackIDPrefix: "blocker"
        )

        let hasCandidateOrContext = !routeCandidates.isEmpty
            || !agentCandidates.isEmpty
            || !skillCandidates.isEmpty
            || !readinessRows.isEmpty
            || !gapRows.isEmpty
            || !blockerRows.isEmpty
        guard hasCandidateOrContext || summaryObject != nil else { return nil }

        let topSkill = skillCandidates.first ?? routeCandidates.first
        let topAgent = topSkill?.agent ?? agentCandidates.first?.agent
        let summaryText = stringValue(summaryObject, keys: ["summary", "message", "text"])
            ?? stringValue(payload, keys: ["summary", "message", "text", "reason"])
            ?? UIStrings.taskCockpitProviderPartialSummary
        let summary = TaskCockpitSummary(
            taskText: stringValue(summaryObject, keys: ["task_text", "taskText", "task", "user_intent", "userIntent"]) ?? taskText,
            summaryText: sanitizedProviderSummary(summaryText) ?? UIStrings.taskCockpitProviderPartialSummary,
            routeCandidateCount: intValue(summaryObject, keys: ["route_candidate_count", "routeCandidateCount", "route_count", "routeCount"]) ?? routeCandidates.count,
            agentCandidateCount: intValue(summaryObject, keys: ["agent_candidate_count", "agentCandidateCount", "agent_count", "agentCount"]) ?? agentCandidates.count,
            skillCandidateCount: intValue(summaryObject, keys: ["skill_candidate_count", "skillCandidateCount", "candidate_skill_count", "candidateSkillCount", "candidate_count", "candidateCount"]) ?? skillCandidates.count,
            readinessSignalCount: intValue(summaryObject, keys: ["readiness_signal_count", "readinessSignalCount"]) ?? readinessRows.count,
            gapCount: intValue(summaryObject, keys: ["gap_count", "gapCount"]) ?? gapRows.count,
            blockerCount: intValue(summaryObject, keys: ["blocker_count", "blockerCount"]) ?? blockerRows.count,
            recommendedAgent: normalizedAgentID(
                stringValue(summaryObject, keys: ["recommended_agent", "recommendedAgent", "top_agent", "topAgent"])
            ) ?? topAgent,
            recommendedSkillName: stringValue(summaryObject, keys: ["recommended_skill_name", "recommendedSkillName", "top_skill_name", "topSkillName"])
                ?? topSkill?.skill?.name
                ?? (topSkill?.agent == nil ? topSkill?.title : nil),
            readinessScore: intValue(summaryObject, keys: ["readiness_score", "readinessScore"]) ?? topSkill?.readinessScore,
            routingScore: intValue(summaryObject, keys: ["routing_score", "routingScore", "confidence_score", "confidenceScore"]) ?? topSkill?.routingScore ?? topSkill?.score
        )

        return TaskCockpitResult(
            generatedBy: stringValue(payload, keys: ["generated_by", "generatedBy"]) ?? "provider-task-cockpit",
            catalogAvailable: boolValue(payload, keys: ["catalog_available", "catalogAvailable"]) ?? true,
            filters: TaskCockpitFilters(taskText: taskText, agents: agentIDs),
            summary: summary,
            routeCandidates: routeCandidates,
            agentCandidates: agentCandidates,
            skillCandidates: skillCandidates,
            readinessSignals: readinessRows,
            gapRows: gapRows,
            blockerRows: blockerRows,
            safetyFlags: ProviderObservabilitySafety(providerRequestSent: true)
        )
    }

    private enum LooseCandidateKind {
        case route
        case agent
        case skill

        var fallbackIDPrefix: String {
            switch self {
            case .route: return "route"
            case .agent: return "agent"
            case .skill: return "skill"
            }
        }
    }

    private static func looseCandidateRows(_ value: Any?, kind: LooseCandidateKind) -> [TaskCockpitCandidateRow] {
        if let array = value as? [Any] {
            return array.enumerated().compactMap { index, item in
                looseCandidateRow(item, kind: kind, index: index)
            }
        }
        if let value {
            return looseCandidateRow(value, kind: kind, index: 0).map { [$0] } ?? []
        }
        return []
    }

    private static func looseCandidateRow(_ value: Any, kind: LooseCandidateKind, index: Int) -> TaskCockpitCandidateRow? {
        if let text = sanitizedProviderSummary(stringValue(value)), !text.isEmpty {
            return TaskCockpitCandidateRow(id: "\(kind.fallbackIDPrefix):\(index)", rank: index + 1, title: text)
        }
        guard let object = value as? [String: Any] else { return nil }
        let nestedSkillObject = dictionaryValue(object, keys: ["skill", "candidate_skill", "candidateSkill", "route"])
        let nestedSkillName = stringValue(nestedSkillObject, keys: ["name", "skill_name", "skillName", "title"])
        let nestedSkillID = stringValue(nestedSkillObject, keys: ["instance_id", "instanceId", "skill_id", "skillId", "id"])
        let nestedDefinitionID = stringValue(nestedSkillObject, keys: ["definition_id", "definitionId"])
        let agent = normalizedAgentID(stringValue(object, keys: ["agent", "agent_id", "agentId"]))
            ?? normalizedAgentID(stringValue(nestedSkillObject, keys: ["agent", "agent_id", "agentId"]))
        let topSkillName = stringValue(object, keys: ["skill_name", "skillName", "best_skill_name", "bestSkillName"])
        let inferredSkillName = nestedSkillName ?? topSkillName ?? (kind == .skill || kind == .route ? stringValue(object, keys: ["title", "name", "label"]) : nil)
        let skillID = nestedSkillID ?? stringValue(object, keys: ["instance_id", "instanceId", "skill_id", "skillId"])
        let skill = inferredSkillName.map {
            TaskSkillRef(
                instanceID: skillID,
                name: $0,
                agent: agent ?? UIStrings.unknown,
                definitionID: nestedDefinitionID ?? stringValue(object, keys: ["definition_id", "definitionId"])
            )
        }
        let title = stringValue(object, keys: ["title", "name", "label", "display_name", "displayName", "agent_name", "agentName"])
            ?? inferredSkillName
            ?? agent.map(DisplayText.agent)
            ?? UIStrings.unknown
        let id = stringValue(object, keys: ["id", "route_id", "routeId", "agent_id", "agentId", "skill_id", "skillId", "instance_id", "instanceId"])
            ?? "\(kind.fallbackIDPrefix):\(index)"

        return TaskCockpitCandidateRow(
            id: id,
            rank: intValue(object, keys: ["rank", "position"]) ?? index + 1,
            title: title,
            agent: agent ?? skill?.agent,
            skill: skill,
            readinessScore: intValue(object, keys: ["readiness_score", "readinessScore"]),
            routingScore: intValue(object, keys: ["routing_score", "routingScore", "confidence_score", "confidenceScore"]),
            score: intValue(object, keys: ["score", "value", "comparison_score", "comparisonScore"]),
            band: stringValue(object, keys: ["band", "readiness_band", "readinessBand", "confidence_band", "confidenceBand"]),
            status: stringValue(object, keys: ["status", "state", "enabled"]),
            summary: sanitizedProviderSummary(stringValue(object, keys: ["summary", "detail", "rationale", "reason", "scope"]) ?? "") ?? "",
            reasons: stringArrayValue(object, keys: ["reasons", "reason", "match_reasons", "matchReasons", "why"]),
            evidenceRefs: stringArrayValue(object, keys: ["evidence_refs", "evidenceRefs", "evidence"]),
            safetyFlags: stringArrayValue(object, keys: ["safety_flags", "safetyFlags", "flags", "blocker_notes", "blockerNotes", "gap_notes", "gapNotes"])
        )
    }

    private static func looseContextRows(_ value: Any?, fallbackIDPrefix: String) -> [TaskCockpitContextRow] {
        if let array = value as? [Any] {
            return array.enumerated().compactMap { index, item in
                looseContextRow(item, fallbackIDPrefix: fallbackIDPrefix, index: index)
            }
        }
        if let value {
            return looseContextRow(value, fallbackIDPrefix: fallbackIDPrefix, index: 0).map { [$0] } ?? []
        }
        return []
    }

    private static func looseContextRow(_ value: Any, fallbackIDPrefix: String, index: Int) -> TaskCockpitContextRow? {
        if let text = sanitizedProviderSummary(stringValue(value)), !text.isEmpty {
            return TaskCockpitContextRow(id: "\(fallbackIDPrefix):\(index)", title: text)
        }
        guard let object = value as? [String: Any] else { return nil }
        let title = sanitizedProviderSummary(stringValue(object, keys: ["title", "name", "label", "task", "summary", "message", "reason"]) ?? "") ?? UIStrings.unknown
        let detail = sanitizedProviderSummary(stringValue(object, keys: ["detail", "description", "suggested_safe_next_action", "suggestedSafeNextAction"]) ?? "") ?? ""
        return TaskCockpitContextRow(
            id: stringValue(object, keys: ["id", "row_id", "rowId"]) ?? "\(fallbackIDPrefix):\(index)",
            title: title,
            detail: detail,
            status: stringValue(object, keys: ["status", "outcome"]),
            severity: stringValue(object, keys: ["severity", "priority"]),
            source: stringValue(object, keys: ["source", "source_method", "sourceMethod", "row_type", "rowType"]),
            agent: normalizedAgentID(stringValue(object, keys: ["agent", "agent_id", "agentId"])),
            count: intValue(object, keys: ["count", "total", "row_count", "rowCount"]),
            evidenceRefs: stringArrayValue(object, keys: ["evidence_refs", "evidenceRefs", "evidence"]),
            safetyFlags: stringArrayValue(object, keys: ["safety_flags", "safetyFlags", "flags"])
        )
    }

    private static func firstValue(_ object: [String: Any], keys: [String]) -> Any? {
        for key in keys {
            if let value = object[key], !(value is NSNull) {
                return value
            }
        }
        return nil
    }

    private static func dictionaryValue(_ object: [String: Any]?, keys: [String]) -> [String: Any]? {
        guard let object else { return nil }
        for key in keys {
            if let value = object[key] as? [String: Any] {
                return value
            }
        }
        return nil
    }

    private static func stringValue(_ object: [String: Any]?, keys: [String]) -> String? {
        guard let object else { return nil }
        for key in keys {
            if let value = stringValue(object[key]) {
                return value
            }
        }
        return nil
    }

    private static func stringValue(_ value: Any?) -> String? {
        switch value {
        case let value as String:
            let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
            return trimmed.isEmpty ? nil : trimmed
        case let value as NSNumber:
            return value.stringValue
        case let value as Bool:
            return value ? UIStrings.stateEnabled : UIStrings.stateDisabled
        default:
            return nil
        }
    }

    private static func intValue(_ object: [String: Any]?, keys: [String]) -> Int? {
        guard let object else { return nil }
        for key in keys {
            if let value = intValue(object[key]) {
                return value
            }
        }
        return nil
    }

    private static func intValue(_ value: Any?) -> Int? {
        switch value {
        case let value as Int:
            return value
        case let value as Double:
            return Int(value.rounded())
        case let value as NSNumber:
            return value.intValue
        case let value as String:
            let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
            if let intValue = Int(trimmed) { return intValue }
            if let doubleValue = Double(trimmed) { return Int(doubleValue.rounded()) }
            return nil
        default:
            return nil
        }
    }

    private static func boolValue(_ object: [String: Any]?, keys: [String]) -> Bool? {
        guard let object else { return nil }
        for key in keys {
            if let value = boolValue(object[key]) {
                return value
            }
        }
        return nil
    }

    private static func boolValue(_ value: Any?) -> Bool? {
        switch value {
        case let value as Bool:
            return value
        case let value as NSNumber:
            return value.boolValue
        case let value as String:
            switch value.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
            case "true", "yes", "1", "enabled", "available":
                return true
            case "false", "no", "0", "disabled", "unavailable":
                return false
            default:
                return nil
            }
        default:
            return nil
        }
    }

    private static func stringArrayValue(_ object: [String: Any], keys: [String]) -> [String] {
        for key in keys {
            if let values = stringArrayValue(object[key]) {
                return values
            }
        }
        return []
    }

    private static func stringArrayValue(_ value: Any?) -> [String]? {
        if let values = value as? [Any] {
            return values.compactMap { item -> String? in
                if let text = sanitizedProviderSummary(stringValue(item)) {
                    return text
                }
                if let object = item as? [String: Any] {
                    return sanitizedProviderSummary(
                        stringValue(object, keys: ["detail", "summary", "message", "title", "name", "source", "id"]) ?? ""
                    )
                }
                return nil
            }
        }
        if let text = sanitizedProviderSummary(stringValue(value) ?? "") {
            return [text]
        }
        return nil
    }

    private static func normalizedAgentID(_ value: String?) -> String? {
        guard let value, !value.isEmpty else { return nil }
        if value.hasPrefix("agent:") {
            return String(value.dropFirst("agent:".count))
        }
        return value
    }

    private static func sanitizedProviderSummary(_ value: String?) -> String? {
        guard let value else { return nil }
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, !looksLikeRawStructuredPayload(trimmed) else { return nil }
        return UIStrings.localizedServiceMessage(trimmed)
    }

    private static func looksLikeRawStructuredPayload(_ value: String) -> Bool {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.hasPrefix("{")
            || trimmed.hasPrefix("[")
            || trimmed.hasPrefix("```")
            || trimmed.contains("\"agent_candidates\"")
            || trimmed.contains("\"skill_candidates\"")
            || trimmed.contains("\"route_candidates\"")
    }

    private static func extractedJSONData(from text: String) -> Data? {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        if let fenced = fencedJSONBody(from: trimmed) {
            return fenced.data(using: .utf8)
        }
        guard let first = trimmed.firstIndex(of: "{"),
              let last = trimmed.lastIndex(of: "}"),
              first <= last
        else {
            return nil
        }
        return String(trimmed[first...last]).data(using: .utf8)
    }

    private static func fencedJSONBody(from text: String) -> String? {
        guard let startFence = text.range(of: "```") else { return nil }
        let afterFence = text[startFence.upperBound...]
        guard let endFence = afterFence.range(of: "```") else { return nil }
        var body = String(afterFence[..<endFence.lowerBound])
        if body.lowercased().hasPrefix("json") {
            body = String(body.dropFirst(4))
        }
        return body.trimmingCharacters(in: .whitespacesAndNewlines)
    }
}

private extension KeyedDecodingContainer {
    func decodeFlexibleTaskCockpitString(keys: [Key]) throws -> String? {
        for key in keys {
            if let value = try? decodeIfPresent(String.self, forKey: key) {
                return value
            }
            if let value = try? decodeIfPresent(Int.self, forKey: key) {
                return "\(value)"
            }
            if let value = try? decodeIfPresent(Double.self, forKey: key) {
                return "\(value)"
            }
            if let value = try? decodeIfPresent(Bool.self, forKey: key) {
                return value ? UIStrings.stateEnabled : UIStrings.stateDisabled
            }
        }
        return nil
    }

    func decodeFlexibleTaskCockpitInt(keys: [Key]) throws -> Int? {
        for key in keys {
            if let value = try? decodeIfPresent(Int.self, forKey: key) {
                return value
            }
            if let value = try? decodeIfPresent(Double.self, forKey: key) {
                return Int(value.rounded())
            }
            if let value = try? decodeIfPresent(String.self, forKey: key) {
                let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
                if let intValue = Int(trimmed) {
                    return intValue
                }
                if let doubleValue = Double(trimmed) {
                    return Int(doubleValue.rounded())
                }
            }
            if let values = try? decodeIfPresent([String].self, forKey: key) {
                return values.count
            }
            if let values = try? decodeIfPresent([TaskCockpitCandidateRow].self, forKey: key) {
                return values.count
            }
            if let values = try? decodeIfPresent([TaskCockpitContextRow].self, forKey: key) {
                return values.count
            }
            if let values = try? decodeIfPresent([ProviderObservabilityEvidenceReference].self, forKey: key) {
                return values.count
            }
        }
        return nil
    }

    func decodeFlexibleTaskCockpitBool(keys: [Key]) throws -> Bool? {
        for key in keys {
            if let value = try? decodeIfPresent(Bool.self, forKey: key) {
                return value
            }
            if let value = try? decodeIfPresent(String.self, forKey: key) {
                switch value.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
                case "true", "yes", "1", "enabled", "available":
                    return true
                case "false", "no", "0", "disabled", "unavailable":
                    return false
                default:
                    continue
                }
            }
            if let value = try? decodeIfPresent(Int.self, forKey: key) {
                return value != 0
            }
        }
        return nil
    }

    func decodeFlexibleTaskCockpitStringArray(keys: [Key]) throws -> [String] {
        for key in keys {
            if let values = try? decodeIfPresent([String].self, forKey: key) {
                return values
            }
            if let value = try? decodeIfPresent(String.self, forKey: key) {
                return value.isEmpty ? [] : [value]
            }
            if let values = try? decodeIfPresent([ProviderObservabilityEvidenceReference].self, forKey: key) {
                return values.map { item in
                    if !item.detail.isEmpty { return item.detail }
                    if let source = item.source, !source.isEmpty { return source }
                    return item.title
                }
            }
            if let values = try? decodeIfPresent([TaskCockpitContextRow].self, forKey: key) {
                return values.map(\.title)
            }
        }
        return []
    }

    func decodeFlexibleTaskCockpitRows(keys: [Key]) throws -> [TaskCockpitCandidateRow] {
        for key in keys {
            if let values = try? decodeIfPresent([TaskCockpitCandidateRow].self, forKey: key) {
                return values
            }
            if let value = try? decodeIfPresent(TaskCockpitCandidateRow.self, forKey: key) {
                return [value]
            }
            if let values = try? decodeIfPresent([String].self, forKey: key) {
                return values.map { TaskCockpitCandidateRow(id: $0, title: $0) }
            }
            if let value = try? decodeIfPresent(String.self, forKey: key), !value.isEmpty {
                return [TaskCockpitCandidateRow(id: value, title: value)]
            }
        }
        return []
    }

    func decodeFlexibleTaskCockpitContextRows(keys: [Key]) throws -> [TaskCockpitContextRow] {
        for key in keys {
            if let values = try? decodeIfPresent([TaskCockpitContextRow].self, forKey: key) {
                return values
            }
            if let value = try? decodeIfPresent(TaskCockpitContextRow.self, forKey: key) {
                return [value]
            }
            if let values = try? decodeIfPresent([String].self, forKey: key) {
                return values.map { TaskCockpitContextRow(id: $0, title: $0) }
            }
            if let value = try? decodeIfPresent(String.self, forKey: key), !value.isEmpty {
                return [TaskCockpitContextRow(id: value, title: value)]
            }
        }
        return []
    }

    func decodeFlexibleTaskCockpitEvidence(keys: [Key]) throws -> [ProviderObservabilityEvidenceReference] {
        for key in keys {
            if let values = try? decodeIfPresent([ProviderObservabilityEvidenceReference].self, forKey: key) {
                return values
            }
            if let values = try? decodeIfPresent([String].self, forKey: key) {
                return values.map { ProviderObservabilityEvidenceReference(title: $0, detail: $0, source: nil) }
            }
            if let value = try? decodeIfPresent(String.self, forKey: key), !value.isEmpty {
                return [ProviderObservabilityEvidenceReference(title: value, detail: value, source: nil)]
            }
        }
        return []
    }
}
