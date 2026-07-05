import Foundation

struct TaskCockpitAggregation: Decodable, Hashable {
    let status: String?
    let elapsedMS: Int?
    let timeoutMS: Int?
    let timedOut: Bool
    let partial: Bool
    let fallbackUsed: Bool
    let limit: Int?
    let scannedCount: Int?
    let totalCount: Int?
    let completedStages: [String]
    let skippedStages: [String]
    let blockerCodes: [String]
    let notes: [String]

    enum CodingKeys: String, CodingKey {
        case status
        case elapsedMS = "elapsed_ms"
        case elapsedMSAlt = "elapsedMS"
        case timeoutMS = "timeout_ms"
        case timeoutMSAlt = "timeoutMS"
        case timedOut = "timed_out"
        case timedOutAlt = "timedOut"
        case partial
        case fallbackUsed = "fallback_used"
        case fallbackUsedAlt = "fallbackUsed"
        case limit
        case scannedCount = "scanned_count"
        case scannedCountAlt = "scannedCount"
        case totalCount = "total_count"
        case totalCountAlt = "totalCount"
        case completedStages = "completed_stages"
        case completedStagesAlt = "completedStages"
        case skippedStages = "skipped_stages"
        case skippedStagesAlt = "skippedStages"
        case blockerCodes = "blocker_codes"
        case blockerCodesAlt = "blockerCodes"
        case notes
    }

    init(
        status: String? = nil,
        elapsedMS: Int? = nil,
        timeoutMS: Int? = nil,
        timedOut: Bool = false,
        partial: Bool = false,
        fallbackUsed: Bool = false,
        limit: Int? = nil,
        scannedCount: Int? = nil,
        totalCount: Int? = nil,
        completedStages: [String] = [],
        skippedStages: [String] = [],
        blockerCodes: [String] = [],
        notes: [String] = []
    ) {
        self.status = status
        self.elapsedMS = elapsedMS
        self.timeoutMS = timeoutMS
        self.timedOut = timedOut
        self.partial = partial
        self.fallbackUsed = fallbackUsed
        self.limit = limit
        self.scannedCount = scannedCount
        self.totalCount = totalCount
        self.completedStages = completedStages
        self.skippedStages = skippedStages
        self.blockerCodes = blockerCodes
        self.notes = notes
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init(
            status: try container.decodeIfPresent(String.self, forKey: .status),
            elapsedMS: try container.decodeFlexibleAggregationInt(keys: [.elapsedMS, .elapsedMSAlt]),
            timeoutMS: try container.decodeFlexibleAggregationInt(keys: [.timeoutMS, .timeoutMSAlt]),
            timedOut: try container.decodeFlexibleAggregationBool(keys: [.timedOut, .timedOutAlt]) ?? false,
            partial: try container.decodeFlexibleAggregationBool(keys: [.partial]) ?? false,
            fallbackUsed: try container.decodeFlexibleAggregationBool(keys: [.fallbackUsed, .fallbackUsedAlt]) ?? false,
            limit: try container.decodeFlexibleAggregationInt(keys: [.limit]),
            scannedCount: try container.decodeFlexibleAggregationInt(keys: [.scannedCount, .scannedCountAlt]),
            totalCount: try container.decodeFlexibleAggregationInt(keys: [.totalCount, .totalCountAlt]),
            completedStages: try container.decodeFlexibleAggregationStringArray(keys: [.completedStages, .completedStagesAlt]),
            skippedStages: try container.decodeFlexibleAggregationStringArray(keys: [.skippedStages, .skippedStagesAlt]),
            blockerCodes: try container.decodeFlexibleAggregationStringArray(keys: [.blockerCodes, .blockerCodesAlt]),
            notes: try container.decodeFlexibleAggregationStringArray(keys: [.notes])
        )
    }

    func completed(_ stage: TaskCockpitProgressStage) -> Bool {
        completedStages.contains { stage.matchesServiceStageKey($0) }
    }

    func skipped(_ stage: TaskCockpitProgressStage) -> Bool {
        skippedStages.contains { stage.matchesServiceStageKey($0) }
    }
}

enum TaskCockpitProgressStage: String, CaseIterable, Hashable, Identifiable {
    case readiness
    case routing
    case crossAgent
    case actionReview
    case batchChecks
    case provider

    var id: String { rawValue }

    var title: String {
        switch self {
        case .readiness:
            return UIStrings.taskCockpitStageReadiness
        case .routing:
            return UIStrings.taskCockpitStageRouting
        case .crossAgent:
            return UIStrings.taskCockpitStageAgentComparison
        case .actionReview:
            return UIStrings.taskCockpitProgressActionReview
        case .batchChecks:
            return UIStrings.taskCockpitProgressBatchChecks
        case .provider:
            return UIStrings.providerObservabilityTitle
        }
    }

    fileprivate var serviceStageKeys: Set<String> {
        switch self {
        case .readiness:
            return ["readiness", "task-readiness"]
        case .routing:
            return ["routing", "routing-confidence", "skill-routing"]
        case .crossAgent:
            return ["agent-comparison", "agent-candidates"]
        case .actionReview:
            return ["action-review", "task-preflight-action-review"]
        case .batchChecks:
            return ["batch-checks", "batch-review"]
        case .provider:
            return ["provider", "provider-observability", "llm.providerobservability"]
        }
    }

    fileprivate func matchesServiceStageKey(_ rawKey: String) -> Bool {
        serviceStageKeys.contains(TaskCockpitProgressStage.normalizedStageKey(rawKey))
    }

    fileprivate static func normalizedStageKey(_ rawKey: String) -> String {
        rawKey
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
            .replacingOccurrences(of: "_", with: "-")
            .replacingOccurrences(of: " ", with: "-")
    }
}

enum TaskCockpitProgressState: String, Hashable {
    case idle
    case queued
    case active
    case completed
    case empty
    case fallback
    case skipped
    case unavailable
    case timedOut
    case cancelled
    case failed
}

struct TaskCockpitProgressRow: Hashable, Identifiable {
    var id: String { stage.id }

    let stage: TaskCockpitProgressStage
    let title: String
    let state: TaskCockpitProgressState
    let detail: String
    let count: Int
    let score: Int?
    let evidenceCount: Int
    let safetyFlagsClear: Bool
    let diagnostic: String?
}

struct TaskCockpitProgressSnapshot: Hashable {
    static let maximumStageCount = TaskCockpitProgressStage.allCases.count

    let operationPhase: TaskCockpitOperationState.Phase
    let taskText: String
    let elapsedSeconds: Int
    let timeoutSeconds: Int
    let estimatedProgress: Double
    let activeStage: TaskCockpitProgressStage?
    let diagnostic: String?
    let stageRows: [TaskCockpitProgressRow]

    init(
        operationState: TaskCockpitOperationState,
        result: TaskCockpitResult?,
        now: Date = Date()
    ) {
        let elapsed = operationState.elapsedSeconds(now: now)
        let progress = TaskCockpitProgressSnapshot.estimatedProgress(
            operationState: operationState,
            elapsedSeconds: elapsed
        )
        let active = TaskCockpitProgressSnapshot.activeStage(
            operationState: operationState,
            elapsedSeconds: elapsed
        )
        let recoveredDiagnostic = TaskCockpitProgressSnapshot.diagnostic(operationState: operationState, result: result)

        operationPhase = operationState.phase
        taskText = !operationState.taskText.isEmpty ? operationState.taskText : result?.filters.taskText ?? result?.summary.taskText ?? ""
        elapsedSeconds = elapsed
        timeoutSeconds = operationState.timeoutSeconds
        estimatedProgress = progress
        activeStage = active
        diagnostic = recoveredDiagnostic
        stageRows = TaskCockpitProgressStage.allCases.map { stage in
            TaskCockpitProgressSnapshot.row(
                for: stage,
                operationState: operationState,
                result: result,
                activeStage: active,
                diagnostic: recoveredDiagnostic
            )
        }
    }

    var completedStageCount: Int {
        stageRows.filter { row in
            row.state == .completed || row.state == .fallback || row.state == .skipped
        }.count
    }

    func row(for stage: TaskCockpitProgressStage) -> TaskCockpitProgressRow? {
        stageRows.first { $0.stage == stage }
    }

    private static func row(
        for stage: TaskCockpitProgressStage,
        operationState: TaskCockpitOperationState,
        result: TaskCockpitResult?,
        activeStage: TaskCockpitProgressStage?,
        diagnostic: String?
    ) -> TaskCockpitProgressRow {
        let facts = StageFacts(stage: stage, result: result)
        let state = rowState(
            stage: stage,
            facts: facts,
            operationState: operationState,
            result: result,
            activeStage: activeStage
        )
        return TaskCockpitProgressRow(
            stage: stage,
            title: stage.title,
            state: state,
            detail: detail(state: state, facts: facts, operationState: operationState, diagnostic: diagnostic),
            count: facts.count,
            score: facts.score,
            evidenceCount: facts.evidenceCount,
            safetyFlagsClear: result?.safetyFlags.allReadOnlyFlagsClear ?? true,
            diagnostic: diagnostic
        )
    }

    private static func rowState(
        stage: TaskCockpitProgressStage,
        facts: StageFacts,
        operationState: TaskCockpitOperationState,
        result: TaskCockpitResult?,
        activeStage: TaskCockpitProgressStage?
    ) -> TaskCockpitProgressState {
        if operationState.phase == .preparing {
            return stage == activeStage ? .active : .queued
        }

        switch operationState.phase {
        case .timedOut:
            return .timedOut
        case .cancelled:
            return .cancelled
        case .failed:
            return .failed
        case .idle, .completed, .fallback, .preparing:
            break
        }

        guard let result else {
            return operationState.phase == .idle ? .idle : .empty
        }

        if result.aggregation?.skipped(stage) == true {
            return .skipped
        }

        let hasStageEvidence = facts.hasRows || facts.hasScore || result.aggregation?.completed(stage) == true
        let isRecovered = operationState.phase == .fallback
            || result.recoveryDiagnosticReason != nil
            || result.aggregation?.partial == true
            || result.aggregation?.fallbackUsed == true
            || result.aggregation?.timedOut == true

        if isRecovered {
            return hasStageEvidence ? .fallback : .unavailable
        }

        if result.isUnavailable {
            return hasStageEvidence ? .fallback : .unavailable
        }

        if hasStageEvidence {
            return .completed
        }

        return .empty
    }

    private static func detail(
        state: TaskCockpitProgressState,
        facts: StageFacts,
        operationState: TaskCockpitOperationState,
        diagnostic: String?
    ) -> String {
        if operationState.phase == .preparing, state == .active {
            return operationState.message
        }
        if [.failed, .timedOut, .cancelled].contains(state), !operationState.message.isEmpty {
            return operationState.message
        }
        if let detail = facts.detail, !detail.isEmpty {
            return detail
        }
        if let diagnostic, [.fallback, .unavailable].contains(state) {
            return diagnostic
        }
        if state == .empty {
            return UIStrings.taskCockpitNoRows
        }
        return ""
    }

    private static func activeStage(
        operationState: TaskCockpitOperationState,
        elapsedSeconds: Int
    ) -> TaskCockpitProgressStage? {
        guard operationState.phase == .preparing else { return nil }
        let stages = TaskCockpitProgressStage.allCases
        guard !stages.isEmpty else { return nil }
        let timeoutSeconds = max(1, operationState.timeoutSeconds)
        let progress = min(max(Double(elapsedSeconds) / Double(timeoutSeconds), 0), 0.999_999)
        let index = min(stages.count - 1, Int(progress * Double(stages.count)))
        return stages[index]
    }

    private static func estimatedProgress(
        operationState: TaskCockpitOperationState,
        elapsedSeconds: Int
    ) -> Double {
        guard operationState.phase == .preparing else {
            return operationState.phase == .idle ? 0 : 1
        }
        let timeoutSeconds = max(1, operationState.timeoutSeconds)
        return min(max(Double(elapsedSeconds) / Double(timeoutSeconds), 0), 1)
    }

    private static func diagnostic(
        operationState: TaskCockpitOperationState,
        result: TaskCockpitResult?
    ) -> String? {
        if let resultDiagnostic = result?.recoveryDiagnosticReason {
            return resultDiagnostic
        }
        guard [.failed, .timedOut, .cancelled, .fallback].contains(operationState.phase), !operationState.message.isEmpty else {
            return nil
        }
        return operationState.message
    }
}

private struct StageFacts {
    let count: Int
    let score: Int?
    let evidenceCount: Int
    let detail: String?

    var hasRows: Bool { count > 0 || evidenceCount > 0 }
    var hasScore: Bool { score != nil }

    init(stage: TaskCockpitProgressStage, result: TaskCockpitResult?) {
        guard let result else {
            count = 0
            score = nil
            evidenceCount = 0
            detail = nil
            return
        }

        switch stage {
        case .readiness:
            let rows = result.readinessSignals
            count = max(result.summary.readinessSignalCount, rows.count)
            score = result.summary.readinessScore
            evidenceCount = rows.flatMap(\.evidenceRefs).count
            detail = StageFacts.firstDetail(rows)
        case .routing:
            count = [result.summary.routeCandidateCount, result.routeCandidates.count, result.skillCandidates.count].max() ?? 0
            score = result.summary.routingScore
            evidenceCount = result.routeCandidates.flatMap(\.evidenceRefs).count + result.skillCandidates.flatMap(\.evidenceRefs).count
            detail = StageFacts.firstDetail(result.routeCandidates) ?? StageFacts.firstDetail(result.skillCandidates)
        case .crossAgent:
            count = max(result.summary.agentCandidateCount, result.agentCandidates.count)
            score = result.agentCandidates.compactMap { $0.score ?? $0.readinessScore ?? $0.routingScore }.first
            evidenceCount = result.agentCandidates.flatMap(\.evidenceRefs).count
            detail = StageFacts.firstDetail(result.agentCandidates)
        case .actionReview:
            let rows = result.cockpitSections.filter(StageFacts.isActionReviewContext)
            let completedByAggregation = result.aggregation?.completed(.actionReview) == true
            count = rows.isEmpty && completedByAggregation ? 0 : rows.count
            score = nil
            evidenceCount = rows.flatMap(\.evidenceRefs).count
            detail = StageFacts.firstDetail(rows)
        case .batchChecks:
            let rows = result.cockpitSections.filter(StageFacts.isBatchReviewContext)
            let completedByAggregation = result.aggregation?.completed(.batchChecks) == true
            count = rows.isEmpty && completedByAggregation ? 0 : rows.count
            score = nil
            evidenceCount = rows.flatMap(\.evidenceRefs).count
            detail = StageFacts.firstDetail(rows)
        case .provider:
            let rows = result.providerObservabilityContext
            count = max(result.summary.providerCallCount, rows.count)
            score = nil
            evidenceCount = rows.flatMap(\.evidenceRefs).count
            detail = StageFacts.firstDetail(rows)
        }
    }

    private static func firstDetail(_ rows: [TaskCockpitContextRow]) -> String? {
        rows.lazy.map { row in
            if !row.detail.isEmpty { return row.detail }
            return row.title
        }.first
    }

    private static func firstDetail(_ rows: [TaskCockpitCandidateRow]) -> String? {
        rows.lazy.map { row in
            if !row.summary.isEmpty { return row.summary }
            return row.title
        }.first
    }

    private static func isBatchReviewContext(_ row: TaskCockpitContextRow) -> Bool {
        [row.id, row.title, row.detail, row.source].compactMap { $0 }.contains { value in
            let normalized = TaskCockpitProgressStage.normalizedStageKey(value)
            return normalized.contains("batch-review") || normalized.contains("batchreview")
        }
    }

    private static func isActionReviewContext(_ row: TaskCockpitContextRow) -> Bool {
        [row.id, row.title, row.detail, row.source].compactMap { $0 }.contains { value in
            let normalized = TaskCockpitProgressStage.normalizedStageKey(value)
            return normalized.contains("action-review") || normalized.contains("actionreview")
        }
    }
}

private extension KeyedDecodingContainer where Key == TaskCockpitAggregation.CodingKeys {
    func decodeFlexibleAggregationInt(keys: [Key]) throws -> Int? {
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
        }
        return nil
    }

    func decodeFlexibleAggregationBool(keys: [Key]) throws -> Bool? {
        for key in keys {
            if let value = try? decodeIfPresent(Bool.self, forKey: key) {
                return value
            }
            if let value = try? decodeIfPresent(String.self, forKey: key) {
                switch value.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
                case "true", "yes", "1", "enabled", "available", "complete", "completed":
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

    func decodeFlexibleAggregationStringArray(keys: [Key]) throws -> [String] {
        for key in keys {
            if let values = try? decodeIfPresent([String].self, forKey: key) {
                return values
            }
            if let value = try? decodeIfPresent(String.self, forKey: key) {
                return value.isEmpty ? [] : [value]
            }
        }
        return []
    }
}
