import Foundation

struct TaskCockpitHistoryStore {
    static let maxRecords = 12

    let fileURL: URL

    init(fileURL: URL = TaskCockpitHistoryStore.defaultFileURL) {
        self.fileURL = fileURL
    }

    func load() -> [TaskCockpitHistoryRecord] {
        guard let data = try? Data(contentsOf: fileURL) else { return [] }
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        if let envelope = try? decoder.decode(StoredTaskCockpitHistoryEnvelope.self, from: data) {
            return envelope.records.prefix(Self.maxRecords).map { $0.record() }
        }
        if let records = try? decoder.decode([StoredTaskCockpitHistoryRecord].self, from: data) {
            return records.prefix(Self.maxRecords).map { $0.record() }
        }
        return []
    }

    func save(_ records: [TaskCockpitHistoryRecord]) {
        let bounded = records.prefix(Self.maxRecords).map(StoredTaskCockpitHistoryRecord.init)
        let envelope = StoredTaskCockpitHistoryEnvelope(version: 1, records: bounded)
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        guard let data = try? encoder.encode(envelope) else { return }
        let directory = fileURL.deletingLastPathComponent()
        do {
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            try data.write(to: fileURL, options: [.atomic])
        } catch {
            return
        }
    }

    static var defaultFileURL: URL {
        appDataURL.appendingPathComponent("task-preflight-history.json", isDirectory: false)
    }

    private static var appDataURL: URL {
        let environment = ProcessInfo.processInfo.environment
        if let override = environment["SKILLS_COPILOT_APP_DATA_DIR"], !override.isEmpty {
            return URL(fileURLWithPath: override, isDirectory: true).standardizedFileURL
        }
        return FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library", isDirectory: true)
            .appendingPathComponent("Application Support", isDirectory: true)
            .appendingPathComponent("dev.agent-copilot.native", isDirectory: true)
            .standardizedFileURL
    }
}

private struct StoredTaskCockpitHistoryEnvelope: Codable {
    let version: Int
    let records: [StoredTaskCockpitHistoryRecord]
}

private struct StoredTaskCockpitHistoryRecord: Codable {
    let id: UUID
    let createdAt: Date
    let taskText: String
    let agentIDs: [String]?
    let result: StoredTaskCockpitResult
    let operationState: StoredTaskCockpitOperationState

    init(_ record: TaskCockpitHistoryRecord) {
        id = record.id
        createdAt = record.createdAt
        taskText = record.taskText
        agentIDs = record.agentIDs
        result = StoredTaskCockpitResult(record.result)
        operationState = StoredTaskCockpitOperationState(record.operationState)
    }

    func record() -> TaskCockpitHistoryRecord {
        TaskCockpitHistoryRecord(
            id: id,
            taskText: taskText,
            agentIDs: agentIDs ?? result.result().agentScopeIDs,
            result: result.result(),
            operationState: operationState.state(fallbackTaskText: taskText),
            createdAt: createdAt
        )
    }
}

private struct StoredTaskCockpitOperationState: Codable {
    let phase: String
    let taskText: String
    let message: String
    let startedAt: Date?
    let finishedAt: Date?
    let timeoutSeconds: Int

    init(_ state: TaskCockpitOperationState) {
        phase = state.phase.rawValue
        taskText = state.taskText
        message = state.message
        startedAt = state.startedAt
        finishedAt = state.finishedAt
        timeoutSeconds = state.timeoutSeconds
    }

    func state(fallbackTaskText: String) -> TaskCockpitOperationState {
        TaskCockpitOperationState(
            phase: TaskCockpitOperationState.Phase(rawValue: phase) ?? .completed,
            taskText: taskText.isEmpty ? fallbackTaskText : taskText,
            message: message,
            startedAt: startedAt,
            finishedAt: finishedAt,
            timeoutSeconds: timeoutSeconds
        )
    }
}

private struct StoredTaskCockpitResult: Codable {
    let generatedBy: String
    let catalogAvailable: Bool
    let filters: StoredTaskCockpitFilters
    let summary: StoredTaskCockpitSummary
    let cockpitSections: [StoredTaskCockpitContextRow]
    let taskRows: [StoredTaskCockpitCandidateRow]
    let routeCandidates: [StoredTaskCockpitCandidateRow]
    let agentCandidates: [StoredTaskCockpitCandidateRow]
    let skillCandidates: [StoredTaskCockpitCandidateRow]
    let readinessSignals: [StoredTaskCockpitContextRow]
    let providerObservabilityContext: [StoredTaskCockpitContextRow]
    let gapRows: [StoredTaskCockpitContextRow]
    let blockerRows: [StoredTaskCockpitContextRow]
    let fallbackReason: String?

    init(_ result: TaskCockpitResult) {
        generatedBy = result.generatedBy
        catalogAvailable = result.catalogAvailable
        filters = StoredTaskCockpitFilters(result.filters)
        summary = StoredTaskCockpitSummary(result.summary)
        cockpitSections = result.cockpitSections.prefix(8).map(StoredTaskCockpitContextRow.init)
        taskRows = result.taskRows.prefix(5).map(StoredTaskCockpitCandidateRow.init)
        routeCandidates = result.routeCandidates.prefix(5).map(StoredTaskCockpitCandidateRow.init)
        agentCandidates = result.agentCandidates.prefix(5).map(StoredTaskCockpitCandidateRow.init)
        skillCandidates = result.skillCandidates.prefix(5).map(StoredTaskCockpitCandidateRow.init)
        readinessSignals = result.readinessSignals.prefix(8).map(StoredTaskCockpitContextRow.init)
        providerObservabilityContext = result.providerObservabilityContext.prefix(4).map(StoredTaskCockpitContextRow.init)
        gapRows = result.gapRows.prefix(8).map(StoredTaskCockpitContextRow.init)
        blockerRows = result.blockerRows.prefix(8).map(StoredTaskCockpitContextRow.init)
        fallbackReason = result.fallbackReason
    }

    func result() -> TaskCockpitResult {
        TaskCockpitResult(
            generatedBy: generatedBy,
            catalogAvailable: catalogAvailable,
            filters: filters.filters(),
            summary: summary.summary(),
            cockpitSections: cockpitSections.map { $0.row() },
            taskRows: taskRows.map { $0.row() },
            routeCandidates: routeCandidates.map { $0.row() },
            agentCandidates: agentCandidates.map { $0.row() },
            skillCandidates: skillCandidates.map { $0.row() },
            readinessSignals: readinessSignals.map { $0.row() },
            providerObservabilityContext: providerObservabilityContext.map { $0.row() },
            gapRows: gapRows.map { $0.row() },
            blockerRows: blockerRows.map { $0.row() },
            safetyFlags: ProviderObservabilitySafety(),
            fallbackReason: fallbackReason
        )
    }
}

private struct StoredTaskCockpitFilters: Codable {
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

    init(_ filters: TaskCockpitFilters) {
        taskText = filters.taskText
        agent = filters.agent
        agents = filters.agents
        selectedSkillID = filters.selectedSkillID
        selectedSkillName = filters.selectedSkillName
        selectedSkillAgent = filters.selectedSkillAgent
        projectRoot = filters.projectRoot
        currentCWD = filters.currentCWD
        workspace = filters.workspace
        limit = filters.limit
        includeProviderObservability = filters.includeProviderObservability
    }

    func filters() -> TaskCockpitFilters {
        TaskCockpitFilters(
            taskText: taskText,
            agent: agent,
            agents: agents,
            selectedSkillID: selectedSkillID,
            selectedSkillName: selectedSkillName,
            selectedSkillAgent: selectedSkillAgent,
            projectRoot: projectRoot,
            currentCWD: currentCWD,
            workspace: workspace,
            limit: limit,
            includeProviderObservability: includeProviderObservability
        )
    }
}

private struct StoredTaskCockpitSummary: Codable {
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

    init(_ summary: TaskCockpitSummary) {
        taskText = summary.taskText
        summaryText = summary.summaryText
        routeCandidateCount = summary.routeCandidateCount
        agentCandidateCount = summary.agentCandidateCount
        skillCandidateCount = summary.skillCandidateCount
        readinessSignalCount = summary.readinessSignalCount
        providerCallCount = summary.providerCallCount
        gapCount = summary.gapCount
        blockerCount = summary.blockerCount
        evidenceCount = summary.evidenceCount
        safetyFlagCount = summary.safetyFlagCount
        recommendedAgent = summary.recommendedAgent
        recommendedSkillName = summary.recommendedSkillName
        readinessScore = summary.readinessScore
        routingScore = summary.routingScore
    }

    func summary() -> TaskCockpitSummary {
        TaskCockpitSummary(
            taskText: taskText,
            summaryText: summaryText,
            routeCandidateCount: routeCandidateCount,
            agentCandidateCount: agentCandidateCount,
            skillCandidateCount: skillCandidateCount,
            readinessSignalCount: readinessSignalCount,
            providerCallCount: providerCallCount,
            gapCount: gapCount,
            blockerCount: blockerCount,
            evidenceCount: evidenceCount,
            safetyFlagCount: safetyFlagCount,
            recommendedAgent: recommendedAgent,
            recommendedSkillName: recommendedSkillName,
            readinessScore: readinessScore,
            routingScore: routingScore
        )
    }
}

private struct StoredTaskCockpitCandidateRow: Codable {
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

    init(_ row: TaskCockpitCandidateRow) {
        id = row.id
        rank = row.rank
        title = row.title
        agent = row.agent
        skill = row.skill
        readinessScore = row.readinessScore
        routingScore = row.routingScore
        score = row.score
        band = row.band
        status = row.status
        summary = row.summary
        reasons = Array(row.reasons.prefix(6))
        evidenceRefs = Array(row.evidenceRefs.prefix(6))
        safetyFlags = Array(row.safetyFlags.prefix(6))
    }

    func row() -> TaskCockpitCandidateRow {
        TaskCockpitCandidateRow(
            id: id,
            rank: rank,
            title: title,
            agent: agent,
            skill: skill,
            readinessScore: readinessScore,
            routingScore: routingScore,
            score: score,
            band: band,
            status: status,
            summary: summary,
            reasons: reasons,
            evidenceRefs: evidenceRefs,
            safetyFlags: safetyFlags
        )
    }
}

private struct StoredTaskCockpitContextRow: Codable {
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

    init(_ row: TaskCockpitContextRow) {
        id = row.id
        title = row.title
        detail = row.detail
        status = row.status
        severity = row.severity
        source = row.source
        agent = row.agent
        count = row.count
        evidenceRefs = Array(row.evidenceRefs.prefix(6))
        safetyFlags = Array(row.safetyFlags.prefix(6))
    }

    func row() -> TaskCockpitContextRow {
        TaskCockpitContextRow(
            id: id,
            title: title,
            detail: detail,
            status: status,
            severity: severity,
            source: source,
            agent: agent,
            count: count,
            evidenceRefs: evidenceRefs,
            safetyFlags: safetyFlags
        )
    }
}
