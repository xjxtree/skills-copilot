import Foundation

struct ProviderObservabilityFilters: Decodable, Hashable {
    let windowDays: Int?
    let startAt: Int?
    let endAt: Int?
    let limit: Int?
    let includeHistory: Bool
    let includeBudgetHints: Bool
    let includeRetentionRecommendations: Bool
    let includeEvidence: Bool
    let aggregationUsesFullRange: Bool

    enum CodingKeys: String, CodingKey {
        case windowDays = "window_days"
        case windowDaysAlt = "windowDays"
        case startAt = "start_at"
        case startAtAlt = "startAt"
        case endAt = "end_at"
        case endAtAlt = "endAt"
        case limit
        case includeHistory = "include_history"
        case includeHistoryAlt = "includeHistory"
        case includeBudgetHints = "include_budget_hints"
        case includeBudgetHintsAlt = "includeBudgetHints"
        case includeRetentionRecommendations = "include_retention_recommendations"
        case includeRetentionRecommendationsAlt = "includeRetentionRecommendations"
        case includeEvidence = "include_evidence"
        case includeEvidenceAlt = "includeEvidence"
        case aggregationUsesFullRange = "aggregation_uses_full_range"
        case aggregationUsesFullRangeAlt = "aggregationUsesFullRange"
    }

    init(
        windowDays: Int? = nil,
        startAt: Int? = nil,
        endAt: Int? = nil,
        limit: Int? = nil,
        includeHistory: Bool = true,
        includeBudgetHints: Bool = true,
        includeRetentionRecommendations: Bool = true,
        includeEvidence: Bool = true,
        aggregationUsesFullRange: Bool = true
    ) {
        self.windowDays = windowDays
        self.startAt = startAt
        self.endAt = endAt
        self.limit = limit
        self.includeHistory = includeHistory
        self.includeBudgetHints = includeBudgetHints
        self.includeRetentionRecommendations = includeRetentionRecommendations
        self.includeEvidence = includeEvidence
        self.aggregationUsesFullRange = aggregationUsesFullRange
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        windowDays = try container.decodeFlexibleProviderObservabilityInt(keys: [.windowDays, .windowDaysAlt])
        startAt = try container.decodeFlexibleProviderObservabilityInt(keys: [.startAt, .startAtAlt])
        endAt = try container.decodeFlexibleProviderObservabilityInt(keys: [.endAt, .endAtAlt])
        limit = try container.decodeFlexibleProviderObservabilityInt(keys: [.limit])
        includeHistory = try container.decodeFlexibleProviderObservabilityBool(keys: [.includeHistory, .includeHistoryAlt]) ?? true
        includeBudgetHints = try container.decodeFlexibleProviderObservabilityBool(keys: [.includeBudgetHints, .includeBudgetHintsAlt]) ?? true
        includeRetentionRecommendations = try container.decodeFlexibleProviderObservabilityBool(keys: [.includeRetentionRecommendations, .includeRetentionRecommendationsAlt]) ?? true
        includeEvidence = try container.decodeFlexibleProviderObservabilityBool(keys: [.includeEvidence, .includeEvidenceAlt]) ?? true
        aggregationUsesFullRange = try container.decodeFlexibleProviderObservabilityBool(keys: [.aggregationUsesFullRange, .aggregationUsesFullRangeAlt]) ?? true
    }
}

enum ProviderObservabilityDateRangePreset: String, CaseIterable, Hashable {
    case last7Days
    case last30Days
    case last90Days
    case all
    case custom

    var title: String {
        switch self {
        case .last7Days:
            return UIStrings.providerObservabilityLast7Days
        case .last30Days:
            return UIStrings.providerObservabilityLast30Days
        case .last90Days:
            return UIStrings.providerObservabilityLast90Days
        case .all:
            return UIStrings.providerObservabilityAllTime
        case .custom:
            return UIStrings.providerObservabilityCustomRange
        }
    }

    func resolved(
        customStartDate: Date,
        customEndDate: Date,
        now: Date = Date(),
        calendar: Calendar = .current
    ) -> ProviderObservabilityResolvedDateRange {
        switch self {
        case .last7Days:
            return rolling(days: 7, now: now)
        case .last30Days:
            return rolling(days: 30, now: now)
        case .last90Days:
            return rolling(days: 90, now: now)
        case .all:
            return ProviderObservabilityResolvedDateRange(windowDays: nil, startAt: nil, endAt: nil)
        case .custom:
            let start = calendar.startOfDay(for: customStartDate)
            let endStart = calendar.startOfDay(for: customEndDate)
            let end = calendar
                .date(byAdding: .day, value: 1, to: endStart)?
                .addingTimeInterval(-0.001) ?? customEndDate
            let startMillis = Int(start.timeIntervalSince1970 * 1000)
            let endMillis = Int(end.timeIntervalSince1970 * 1000)
            return ProviderObservabilityResolvedDateRange(
                windowDays: nil,
                startAt: min(startMillis, endMillis),
                endAt: max(startMillis, endMillis)
            )
        }
    }

    private func rolling(days: Int, now: Date) -> ProviderObservabilityResolvedDateRange {
        let endAt = Int(now.timeIntervalSince1970 * 1000)
        let startAt = Int(now.addingTimeInterval(TimeInterval(-days * 86_400)).timeIntervalSince1970 * 1000)
        return ProviderObservabilityResolvedDateRange(windowDays: days, startAt: startAt, endAt: endAt)
    }
}

struct ProviderObservabilityResolvedDateRange: Hashable {
    let windowDays: Int?
    let startAt: Int?
    let endAt: Int?
}

struct ProviderObservabilitySummary: Decodable, Hashable {
    let callCount: Int
    let successCount: Int
    let failureCount: Int
    let blockedCount: Int
    let providerCount: Int
    let modelCount: Int
    let destinationCount: Int
    let errorCount: Int
    let estimatedInputTokens: Int
    let estimatedOutputTokens: Int
    let estimatedTotalTokens: Int
    let estimatedCostUSD: Double?
    let totalDurationMS: Int
    let averageDurationMS: Int?
    let budgetHintCount: Int
    let retentionRecommendationCount: Int
    let summaryText: String

    enum CodingKeys: String, CodingKey {
        case callCount = "call_count"
        case calls
        case totalCalls = "total_calls"
        case totalCallsAlt = "totalCalls"
        case totalPromptRunCount = "total_prompt_run_count"
        case totalCallMetadataCount = "total_call_metadata_count"
        case returnedPromptRunCount = "returned_prompt_run_count"
        case returnedCallRowCount = "returned_call_row_count"
        case successCount = "success_count"
        case successes
        case succeeded
        case succeededCount = "succeeded_count"
        case failureCount = "failure_count"
        case failures
        case failed
        case failedCount = "failed_count"
        case blockedCount = "blocked_count"
        case blocked
        case blockCount = "block_count"
        case providerCount = "provider_count"
        case providerProfileCount = "provider_profile_count"
        case providers
        case modelCount = "model_count"
        case models
        case destinationCount = "destination_count"
        case destinations
        case errorCount = "error_count"
        case errors
        case groupingCount = "grouping_count"
        case observedProviderRequestRowCount = "observed_provider_request_row_count"
        case observedCredentialAccessRowCount = "observed_credential_access_row_count"
        case estimatedInputTokens = "estimated_input_tokens"
        case inputTokens = "input_tokens"
        case estimatedOutputTokens = "estimated_output_tokens"
        case outputTokens = "output_tokens"
        case estimatedTotalTokens = "estimated_total_tokens"
        case totalTokens = "total_tokens"
        case estimatedCostUSD = "estimated_cost_usd"
        case estimatedCostUSDAlt = "estimatedCostUSD"
        case costUSD = "cost_usd"
        case totalDurationMS = "total_duration_ms"
        case totalDurationMSAlt = "totalDurationMS"
        case durationMS = "duration_ms"
        case averageDurationMS = "average_duration_ms"
        case averageDurationMSAlt = "averageDurationMS"
        case avgDurationMS = "avg_duration_ms"
        case budgetHintCount = "budget_hint_count"
        case budgetHints = "budget_hints"
        case budgetUsageHints = "budget_usage_hints"
        case retentionRecommendationCount = "retention_recommendation_count"
        case retentionRows = "retention_rows"
        case retentionRecommendations = "retention_recommendations"
        case cleanupRecommendations = "cleanup_recommendations"
        case summary
        case message
        case text
    }

    init(
        callCount: Int = 0,
        successCount: Int = 0,
        failureCount: Int = 0,
        blockedCount: Int = 0,
        providerCount: Int = 0,
        modelCount: Int = 0,
        destinationCount: Int = 0,
        errorCount: Int = 0,
        estimatedInputTokens: Int = 0,
        estimatedOutputTokens: Int = 0,
        estimatedTotalTokens: Int = 0,
        estimatedCostUSD: Double? = nil,
        totalDurationMS: Int = 0,
        averageDurationMS: Int? = nil,
        budgetHintCount: Int = 0,
        retentionRecommendationCount: Int = 0,
        summaryText: String = ""
    ) {
        self.callCount = callCount
        self.successCount = successCount
        self.failureCount = failureCount
        self.blockedCount = blockedCount
        self.providerCount = providerCount
        self.modelCount = modelCount
        self.destinationCount = destinationCount
        self.errorCount = errorCount
        self.estimatedInputTokens = estimatedInputTokens
        self.estimatedOutputTokens = estimatedOutputTokens
        self.estimatedTotalTokens = estimatedTotalTokens
        self.estimatedCostUSD = estimatedCostUSD
        self.totalDurationMS = totalDurationMS
        self.averageDurationMS = averageDurationMS
        self.budgetHintCount = budgetHintCount
        self.retentionRecommendationCount = retentionRecommendationCount
        self.summaryText = summaryText
    }

    init(from decoder: Decoder) throws {
        if let value = try? decoder.singleValueContainer().decode(String.self) {
            self.init(summaryText: value)
            return
        }

        let container = try decoder.container(keyedBy: CodingKeys.self)
        let input = try container.decodeFlexibleProviderObservabilityInt(keys: [.estimatedInputTokens, .inputTokens]) ?? 0
        let output = try container.decodeFlexibleProviderObservabilityInt(keys: [.estimatedOutputTokens, .outputTokens]) ?? 0
        let total = try container.decodeFlexibleProviderObservabilityInt(keys: [.estimatedTotalTokens, .totalTokens]) ?? input + output
        let successCount = try container.decodeFlexibleProviderObservabilityInt(keys: [.successCount, .successes, .succeeded, .succeededCount]) ?? 0
        let failureCount = try container.decodeFlexibleProviderObservabilityInt(keys: [.failureCount, .failures, .failed, .failedCount]) ?? 0
        let blockedCount = try container.decodeFlexibleProviderObservabilityInt(keys: [.blockedCount, .blocked, .blockCount]) ?? 0
        let explicitCallCount = try container.decodeFlexibleProviderObservabilityInt(keys: [.callCount, .totalCalls, .totalCallsAlt, .calls])
        let returnedPromptRunCount = try container.decodeFlexibleProviderObservabilityInt(keys: [.returnedPromptRunCount])
        let returnedCallRowCount = try container.decodeFlexibleProviderObservabilityInt(keys: [.returnedCallRowCount])
        let totalPromptRunCount = try container.decodeFlexibleProviderObservabilityInt(keys: [.totalPromptRunCount])
        let totalCallMetadataCount = try container.decodeFlexibleProviderObservabilityInt(keys: [.totalCallMetadataCount])
        let returnedCount = [returnedPromptRunCount, returnedCallRowCount].compactMap { $0 }.reduce(0, +)
        let totalObservedCount = [totalPromptRunCount, totalCallMetadataCount].compactMap { $0 }.reduce(0, +)
        let statusCount = successCount + failureCount + blockedCount
        let decodedCallCount = explicitCallCount
            ?? (returnedPromptRunCount != nil || returnedCallRowCount != nil ? returnedCount : nil)
            ?? (totalPromptRunCount != nil || totalCallMetadataCount != nil ? totalObservedCount : nil)
            ?? 0
        self.init(
            callCount: max(decodedCallCount, statusCount),
            successCount: successCount,
            failureCount: failureCount,
            blockedCount: blockedCount,
            providerCount: try container.decodeFlexibleProviderObservabilityInt(keys: [.providerCount, .providerProfileCount, .providers]) ?? 0,
            modelCount: try container.decodeFlexibleProviderObservabilityInt(keys: [.modelCount, .models]) ?? 0,
            destinationCount: try container.decodeFlexibleProviderObservabilityInt(keys: [.destinationCount, .destinations]) ?? 0,
            errorCount: try container.decodeFlexibleProviderObservabilityInt(keys: [.errorCount, .errors, .observedProviderRequestRowCount, .observedCredentialAccessRowCount]) ?? 0,
            estimatedInputTokens: input,
            estimatedOutputTokens: output,
            estimatedTotalTokens: total,
            estimatedCostUSD: try container.decodeFlexibleProviderObservabilityDouble(keys: [.estimatedCostUSD, .estimatedCostUSDAlt, .costUSD]),
            totalDurationMS: try container.decodeFlexibleProviderObservabilityInt(keys: [.totalDurationMS, .totalDurationMSAlt, .durationMS]) ?? 0,
            averageDurationMS: try container.decodeFlexibleProviderObservabilityInt(keys: [.averageDurationMS, .averageDurationMSAlt, .avgDurationMS]),
            budgetHintCount: try container.decodeFlexibleProviderObservabilityInt(keys: [.budgetHintCount, .budgetHints, .budgetUsageHints]) ?? 0,
            retentionRecommendationCount: try container.decodeFlexibleProviderObservabilityInt(keys: [.retentionRecommendationCount, .retentionRows, .retentionRecommendations, .cleanupRecommendations]) ?? 0,
            summaryText: try container.decodeIfPresent(String.self, forKey: .summary)
                ?? container.decodeIfPresent(String.self, forKey: .message)
                ?? container.decodeIfPresent(String.self, forKey: .text)
                ?? ""
        )
    }
}

struct ProviderObservabilityCallRow: Decodable, Identifiable, Hashable {
    let id: String
    let previewID: String?
    let confirmationID: String?
    let requestKind: String
    let action: String
    let provider: String
    let model: String
    let destinationHost: String
    let status: String
    let errorCode: String?
    let errorMessage: String?
    let durationMS: Int?
    let inputTokens: Int
    let outputTokens: Int
    let totalTokens: Int
    let estimatedCostUSD: Double?
    let startedAt: Int?
    let completedAt: Int?
    let copyOnly: Bool
    let providerRequestSent: Bool
    let credentialAccessed: Bool
    let rawPromptPersisted: Bool
    let rawResponsePersisted: Bool
    let rawSecretReturned: Bool
    let evidenceRefs: [String]
    let safetyFlags: [String]
    let detail: String

    enum CodingKeys: String, CodingKey {
        case id
        case callID = "call_id"
        case runID = "run_id"
        case previewID = "preview_id"
        case previewIDAlt = "previewID"
        case confirmationID = "confirmation_id"
        case confirmationIDAlt = "confirmationID"
        case requestKind = "request_kind"
        case requestKindAlt = "requestKind"
        case kind
        case action
        case actionType = "action_type"
        case provider
        case model
        case destinationHost = "destination_host"
        case destinationHostAlt = "destinationHost"
        case host
        case destination
        case status
        case outcome
        case errorCode = "error_code"
        case errorCodeAlt = "errorCode"
        case errorMessage = "error_message"
        case errorMessageAlt = "errorMessage"
        case error
        case durationMS = "duration_ms"
        case durationMSAlt = "durationMS"
        case inputTokens = "input_tokens"
        case inputTokensAlt = "inputTokens"
        case estimatedInputTokens = "estimated_input_tokens"
        case outputTokens = "output_tokens"
        case outputTokensAlt = "outputTokens"
        case estimatedOutputTokens = "estimated_output_tokens"
        case totalTokens = "total_tokens"
        case totalTokensAlt = "totalTokens"
        case estimatedTotalTokens = "estimated_total_tokens"
        case estimatedCostUSD = "estimated_cost_usd"
        case estimatedCostUSDAlt = "estimatedCostUSD"
        case costUSD = "cost_usd"
        case startedAt = "started_at"
        case startedAtAlt = "startedAt"
        case completedAt = "completed_at"
        case completedAtAlt = "completedAt"
        case observedAt = "observed_at"
        case timestamp
        case copyOnly = "copy_only"
        case copyOnlyAlt = "copyOnly"
        case draftCopyOnly = "draft_copy_only"
        case providerRequestSent = "provider_request_sent"
        case providerRequestSentAlt = "providerRequestSent"
        case recordedProviderRequestSent = "recorded_provider_request_sent"
        case credentialAccessed = "credential_accessed"
        case credentialAccessedAlt = "credentialAccessed"
        case recordedCredentialAccessed = "recorded_credential_accessed"
        case rawPromptPersisted = "raw_prompt_persisted"
        case rawPromptPersistedAlt = "rawPromptPersisted"
        case rawResponsePersisted = "raw_response_persisted"
        case rawResponsePersistedAlt = "rawResponsePersisted"
        case rawSecretReturned = "raw_secret_returned"
        case rawSecretReturnedAlt = "rawSecretReturned"
        case evidenceRefs = "evidence_refs"
        case evidenceRefsAlt = "evidenceRefs"
        case evidence
        case safetyFlags = "safety_flags"
        case safetyFlagsAlt = "safetyFlags"
        case detail
        case summary
        case message
        case title
        case redactionStatus = "redaction_status"
    }

    init(from decoder: Decoder) throws {
        if let value = try? decoder.singleValueContainer().decode(String.self) {
            id = "call:\(value)"
            previewID = nil
            confirmationID = nil
            requestKind = value
            action = value
            provider = UIStrings.unknown
            model = UIStrings.unknown
            destinationHost = UIStrings.unknown
            status = UIStrings.unknown
            errorCode = nil
            errorMessage = nil
            durationMS = nil
            inputTokens = 0
            outputTokens = 0
            totalTokens = 0
            estimatedCostUSD = nil
            startedAt = nil
            completedAt = nil
            copyOnly = true
            providerRequestSent = false
            credentialAccessed = false
            rawPromptPersisted = false
            rawResponsePersisted = false
            rawSecretReturned = false
            evidenceRefs = [value]
            safetyFlags = []
            detail = value
            return
        }

        let container = try decoder.container(keyedBy: CodingKeys.self)
        previewID = try container.decodeIfPresent(String.self, forKey: .previewID)
            ?? container.decodeIfPresent(String.self, forKey: .previewIDAlt)
        confirmationID = try container.decodeIfPresent(String.self, forKey: .confirmationID)
            ?? container.decodeIfPresent(String.self, forKey: .confirmationIDAlt)
        requestKind = try container.decodeIfPresent(String.self, forKey: .requestKind)
            ?? container.decodeIfPresent(String.self, forKey: .requestKindAlt)
            ?? container.decodeIfPresent(String.self, forKey: .kind)
            ?? container.decodeIfPresent(String.self, forKey: .action)
            ?? container.decodeIfPresent(String.self, forKey: .actionType)
            ?? UIStrings.unknown
        action = try container.decodeIfPresent(String.self, forKey: .action)
            ?? container.decodeIfPresent(String.self, forKey: .actionType)
            ?? requestKind
        provider = try container.decodeIfPresent(String.self, forKey: .provider) ?? UIStrings.unknown
        model = try container.decodeIfPresent(String.self, forKey: .model) ?? UIStrings.unknown
        destinationHost = try container.decodeIfPresent(String.self, forKey: .destinationHost)
            ?? container.decodeIfPresent(String.self, forKey: .destinationHostAlt)
            ?? container.decodeIfPresent(String.self, forKey: .host)
            ?? container.decodeIfPresent(String.self, forKey: .destination)
            ?? UIStrings.unknown
        status = try container.decodeIfPresent(String.self, forKey: .status)
            ?? container.decodeIfPresent(String.self, forKey: .outcome)
            ?? UIStrings.unknown
        errorCode = try container.decodeIfPresent(String.self, forKey: .errorCode)
            ?? container.decodeIfPresent(String.self, forKey: .errorCodeAlt)
        errorMessage = try container.decodeIfPresent(String.self, forKey: .errorMessage)
            ?? container.decodeIfPresent(String.self, forKey: .errorMessageAlt)
            ?? container.decodeIfPresent(String.self, forKey: .error)
        durationMS = try container.decodeFlexibleProviderObservabilityInt(keys: [.durationMS, .durationMSAlt])
        inputTokens = try container.decodeFlexibleProviderObservabilityInt(keys: [.inputTokens, .inputTokensAlt, .estimatedInputTokens]) ?? 0
        outputTokens = try container.decodeFlexibleProviderObservabilityInt(keys: [.outputTokens, .outputTokensAlt, .estimatedOutputTokens]) ?? 0
        totalTokens = try container.decodeFlexibleProviderObservabilityInt(keys: [.totalTokens, .totalTokensAlt, .estimatedTotalTokens]) ?? inputTokens + outputTokens
        estimatedCostUSD = try container.decodeFlexibleProviderObservabilityDouble(keys: [.estimatedCostUSD, .estimatedCostUSDAlt, .costUSD])
        startedAt = try container.decodeFlexibleProviderObservabilityInt(keys: [.startedAt, .startedAtAlt])
        completedAt = try container.decodeFlexibleProviderObservabilityInt(keys: [.completedAt, .completedAtAlt, .observedAt, .timestamp])
        copyOnly = try container.decodeFlexibleProviderObservabilityBool(keys: [.copyOnly, .copyOnlyAlt, .draftCopyOnly]) ?? true
        providerRequestSent = try container.decodeFlexibleProviderObservabilityBool(keys: [.providerRequestSent, .providerRequestSentAlt, .recordedProviderRequestSent]) ?? false
        credentialAccessed = try container.decodeFlexibleProviderObservabilityBool(keys: [.credentialAccessed, .credentialAccessedAlt, .recordedCredentialAccessed]) ?? false
        rawPromptPersisted = try container.decodeFlexibleProviderObservabilityBool(keys: [.rawPromptPersisted, .rawPromptPersistedAlt]) ?? false
        rawResponsePersisted = try container.decodeFlexibleProviderObservabilityBool(keys: [.rawResponsePersisted, .rawResponsePersistedAlt]) ?? false
        rawSecretReturned = try container.decodeFlexibleProviderObservabilityBool(keys: [.rawSecretReturned, .rawSecretReturnedAlt]) ?? false
        evidenceRefs = try container.decodeFlexibleProviderObservabilityStringArray(keys: [.evidenceRefs, .evidenceRefsAlt, .evidence])
        safetyFlags = try container.decodeFlexibleProviderObservabilityStringArray(keys: [.safetyFlags, .safetyFlagsAlt])
        detail = try container.decodeIfPresent(String.self, forKey: .detail)
            ?? container.decodeIfPresent(String.self, forKey: .summary)
            ?? container.decodeIfPresent(String.self, forKey: .message)
            ?? container.decodeIfPresent(String.self, forKey: .title)
            ?? container.decodeIfPresent(String.self, forKey: .redactionStatus)
            ?? ""

        let decodedID = try container.decodeIfPresent(String.self, forKey: .id)
            ?? container.decodeIfPresent(String.self, forKey: .callID)
            ?? container.decodeIfPresent(String.self, forKey: .runID)
        id = decodedID ?? [
            previewID,
            requestKind,
            provider,
            model,
            destinationHost,
            status,
            completedAt.map(String.init)
        ]
            .compactMap { $0?.providerObservabilityNonEmpty }
            .joined(separator: ":")
    }

    var statusIsProblem: Bool {
        let value = status.lowercased()
        return value.contains("fail") || value.contains("error") || value.contains("block") || value.contains("timeout")
    }
}

struct ProviderObservabilityDimensionRow: Decodable, Identifiable, Hashable {
    let id: String
    let kind: String
    let label: String
    let provider: String?
    let model: String?
    let destinationHost: String?
    let callCount: Int
    let successCount: Int
    let failureCount: Int
    let blockedCount: Int
    let estimatedTokens: Int
    let estimatedCostUSD: Double?
    let averageDurationMS: Int?
    let status: String
    let notes: [String]
    let evidenceRefs: [String]
    let safetyFlags: [String]

    enum CodingKeys: String, CodingKey {
        case id
        case kind
        case type
        case label
        case name
        case title
        case provider
        case model
        case destinationHost = "destination_host"
        case destinationHostAlt = "destinationHost"
        case host
        case callCount = "call_count"
        case calls
        case promptRunCount = "prompt_run_count"
        case callMetadataCount = "call_metadata_count"
        case recordedProviderRequestCount = "recorded_provider_request_count"
        case recordedCredentialAccessCount = "recorded_credential_access_count"
        case successCount = "success_count"
        case successes
        case succeededCount = "succeeded_count"
        case failureCount = "failure_count"
        case failures
        case failedCount = "failed_count"
        case blockedCount = "blocked_count"
        case blocked
        case estimatedTokens = "estimated_tokens"
        case estimatedTokensAlt = "estimatedTokens"
        case estimatedTotalTokens = "estimated_total_tokens"
        case totalTokens = "total_tokens"
        case estimatedCostUSD = "estimated_cost_usd"
        case estimatedCostUSDAlt = "estimatedCostUSD"
        case averageDurationMS = "average_duration_ms"
        case averageDurationMSAlt = "averageDurationMS"
        case status
        case notes
        case note
        case evidenceRefs = "evidence_refs"
        case evidenceRefsAlt = "evidenceRefs"
        case evidence
        case safetyFlags = "safety_flags"
        case safetyFlagsAlt = "safetyFlags"
    }

    init(from decoder: Decoder) throws {
        if let value = try? decoder.singleValueContainer().decode(String.self) {
            id = "dimension:\(value)"
            kind = UIStrings.unknown
            label = value
            provider = nil
            model = nil
            destinationHost = nil
            callCount = 0
            successCount = 0
            failureCount = 0
            blockedCount = 0
            estimatedTokens = 0
            estimatedCostUSD = nil
            averageDurationMS = nil
            status = UIStrings.unknown
            notes = []
            evidenceRefs = [value]
            safetyFlags = []
            return
        }

        let container = try decoder.container(keyedBy: CodingKeys.self)
        kind = try container.decodeIfPresent(String.self, forKey: .kind)
            ?? container.decodeIfPresent(String.self, forKey: .type)
            ?? UIStrings.unknown
        label = try container.decodeIfPresent(String.self, forKey: .label)
            ?? container.decodeIfPresent(String.self, forKey: .name)
            ?? container.decodeIfPresent(String.self, forKey: .title)
            ?? container.decodeIfPresent(String.self, forKey: .provider)
            ?? container.decodeIfPresent(String.self, forKey: .model)
            ?? container.decodeIfPresent(String.self, forKey: .destinationHost)
            ?? UIStrings.unknown
        provider = try container.decodeIfPresent(String.self, forKey: .provider)
        model = try container.decodeIfPresent(String.self, forKey: .model)
        destinationHost = try container.decodeIfPresent(String.self, forKey: .destinationHost)
            ?? container.decodeIfPresent(String.self, forKey: .destinationHostAlt)
            ?? container.decodeIfPresent(String.self, forKey: .host)
        let promptRunCount = try container.decodeFlexibleProviderObservabilityInt(keys: [.promptRunCount]) ?? 0
        let callMetadataCount = try container.decodeFlexibleProviderObservabilityInt(keys: [.callMetadataCount]) ?? 0
        callCount = try container.decodeFlexibleProviderObservabilityInt(keys: [.callCount, .calls]) ?? promptRunCount + callMetadataCount
        successCount = try container.decodeFlexibleProviderObservabilityInt(keys: [.successCount, .successes, .succeededCount]) ?? 0
        failureCount = try container.decodeFlexibleProviderObservabilityInt(keys: [.failureCount, .failures, .failedCount]) ?? 0
        blockedCount = try container.decodeFlexibleProviderObservabilityInt(keys: [.blockedCount, .blocked]) ?? 0
        estimatedTokens = try container.decodeFlexibleProviderObservabilityInt(keys: [.estimatedTokens, .estimatedTokensAlt, .estimatedTotalTokens, .totalTokens]) ?? 0
        estimatedCostUSD = try container.decodeFlexibleProviderObservabilityDouble(keys: [.estimatedCostUSD, .estimatedCostUSDAlt])
        averageDurationMS = try container.decodeFlexibleProviderObservabilityInt(keys: [.averageDurationMS, .averageDurationMSAlt])
        status = try container.decodeIfPresent(String.self, forKey: .status) ?? UIStrings.unknown
        notes = try container.decodeFlexibleProviderObservabilityStringArray(keys: [.notes, .note])
        evidenceRefs = try container.decodeFlexibleProviderObservabilityStringArray(keys: [.evidenceRefs, .evidenceRefsAlt, .evidence])
        safetyFlags = try container.decodeFlexibleProviderObservabilityStringArray(keys: [.safetyFlags, .safetyFlagsAlt])
        id = try container.decodeIfPresent(String.self, forKey: .id) ?? "\(kind):\(label)"
    }

    init(
        id: String,
        kind: String,
        label: String,
        provider: String? = nil,
        model: String? = nil,
        destinationHost: String? = nil,
        callCount: Int = 0,
        successCount: Int = 0,
        failureCount: Int = 0,
        blockedCount: Int = 0,
        estimatedTokens: Int = 0,
        estimatedCostUSD: Double? = nil,
        averageDurationMS: Int? = nil,
        status: String = UIStrings.unknown,
        notes: [String] = [],
        evidenceRefs: [String] = [],
        safetyFlags: [String] = []
    ) {
        self.id = id
        self.kind = kind
        self.label = label
        self.provider = provider
        self.model = model
        self.destinationHost = destinationHost
        self.callCount = callCount
        self.successCount = successCount
        self.failureCount = failureCount
        self.blockedCount = blockedCount
        self.estimatedTokens = estimatedTokens
        self.estimatedCostUSD = estimatedCostUSD
        self.averageDurationMS = averageDurationMS
        self.status = status
        self.notes = notes
        self.evidenceRefs = evidenceRefs
        self.safetyFlags = safetyFlags
    }

    var hasActivity: Bool {
        callCount > 0
            || successCount > 0
            || failureCount > 0
            || blockedCount > 0
            || estimatedTokens > 0
            || (estimatedCostUSD ?? 0) > 0
            || (averageDurationMS ?? 0) > 0
    }
}

struct ProviderObservabilityIssueRow: Decodable, Identifiable, Hashable {
    let id: String
    let severity: String
    let status: String
    let title: String
    let detail: String
    let count: Int
    let provider: String?
    let model: String?
    let destinationHost: String?
    let evidenceRefs: [String]

    enum CodingKeys: String, CodingKey {
        case id
        case severity
        case level
        case status
        case outcome
        case title
        case name
        case detail
        case summary
        case message
        case count
        case provider
        case model
        case destinationHost = "destination_host"
        case destinationHostAlt = "destinationHost"
        case host
        case evidenceRefs = "evidence_refs"
        case evidenceRefsAlt = "evidenceRefs"
        case evidence
    }

    init(from decoder: Decoder) throws {
        if let value = try? decoder.singleValueContainer().decode(String.self) {
            id = "issue:\(value)"
            severity = UIStrings.unknown
            status = UIStrings.unknown
            title = value
            detail = value
            count = 0
            provider = nil
            model = nil
            destinationHost = nil
            evidenceRefs = [value]
            return
        }

        let container = try decoder.container(keyedBy: CodingKeys.self)
        severity = try container.decodeIfPresent(String.self, forKey: .severity)
            ?? container.decodeIfPresent(String.self, forKey: .level)
            ?? UIStrings.unknown
        status = try container.decodeIfPresent(String.self, forKey: .status)
            ?? container.decodeIfPresent(String.self, forKey: .outcome)
            ?? UIStrings.unknown
        title = try container.decodeIfPresent(String.self, forKey: .title)
            ?? container.decodeIfPresent(String.self, forKey: .name)
            ?? status
        detail = try container.decodeIfPresent(String.self, forKey: .detail)
            ?? container.decodeIfPresent(String.self, forKey: .summary)
            ?? container.decodeIfPresent(String.self, forKey: .message)
            ?? ""
        count = try container.decodeFlexibleProviderObservabilityInt(keys: [.count]) ?? 0
        provider = try container.decodeIfPresent(String.self, forKey: .provider)
        model = try container.decodeIfPresent(String.self, forKey: .model)
        destinationHost = try container.decodeIfPresent(String.self, forKey: .destinationHost)
            ?? container.decodeIfPresent(String.self, forKey: .destinationHostAlt)
            ?? container.decodeIfPresent(String.self, forKey: .host)
        evidenceRefs = try container.decodeFlexibleProviderObservabilityStringArray(keys: [.evidenceRefs, .evidenceRefsAlt, .evidence])
        id = try container.decodeIfPresent(String.self, forKey: .id) ?? "\(severity):\(status):\(title)"
    }
}

struct ProviderObservabilityModelTaskHistoryRow: Decodable, Identifiable, Hashable {
    let id: String
    let source: String
    let sourceKind: String
    let title: String
    let task: String?
    let taskKind: String
    let agent: String?
    let provider: String
    let model: String
    let destinationHost: String?
    let matchStatus: String
    let confidenceScore: Int?
    let status: String
    let latencyMS: Int?
    let estimatedTotalTokens: Int
    let estimatedCostUSD: Double?
    let gapNotes: [String]
    let blockerNotes: [String]
    let outcomeNotes: [String]
    let evidenceRefs: [String]
    let redactionStatus: String
    let safetyFlags: ProviderObservabilitySafety

    enum CodingKeys: String, CodingKey {
        case id
        case source
        case sourceKind = "source_kind"
        case sourceKindAlt = "sourceKind"
        case title
        case task
        case taskKind = "task_kind"
        case taskKindAlt = "taskKind"
        case agent
        case provider
        case model
        case destinationHost = "destination_host"
        case destinationHostAlt = "destinationHost"
        case matchStatus = "match_status"
        case matchStatusAlt = "matchStatus"
        case confidenceScore = "confidence_score"
        case confidenceScoreAlt = "confidenceScore"
        case status
        case latencyMS = "latency_ms"
        case latencyMSAlt = "latencyMS"
        case estimatedTotalTokens = "estimated_total_tokens"
        case estimatedTotalTokensAlt = "estimatedTotalTokens"
        case estimatedCostUSD = "estimated_cost_usd"
        case estimatedCostUSDAlt = "estimatedCostUSD"
        case gapNotes = "gap_notes"
        case gapNotesAlt = "gapNotes"
        case blockerNotes = "blocker_notes"
        case blockerNotesAlt = "blockerNotes"
        case outcomeNotes = "outcome_notes"
        case outcomeNotesAlt = "outcomeNotes"
        case evidenceRefs = "evidence_refs"
        case evidenceRefsAlt = "evidenceRefs"
        case redactionStatus = "redaction_status"
        case redactionStatusAlt = "redactionStatus"
        case safetyFlags = "safety_flags"
        case safetyFlagsAlt = "safetyFlags"
    }

    init(from decoder: Decoder) throws {
        if let value = try? decoder.singleValueContainer().decode(String.self) {
            id = "model-task:\(value)"
            source = UIStrings.unknown
            sourceKind = UIStrings.unknown
            title = value
            task = nil
            taskKind = UIStrings.unknown
            agent = nil
            provider = UIStrings.unknown
            model = UIStrings.unknown
            destinationHost = nil
            matchStatus = UIStrings.unknown
            confidenceScore = nil
            status = UIStrings.unknown
            latencyMS = nil
            estimatedTotalTokens = 0
            estimatedCostUSD = nil
            gapNotes = []
            blockerNotes = []
            outcomeNotes = []
            evidenceRefs = [value]
            redactionStatus = UIStrings.unknown
            safetyFlags = ProviderObservabilitySafety()
            return
        }

        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decodeIfPresent(String.self, forKey: .id) ?? UUID().uuidString
        source = try container.decodeIfPresent(String.self, forKey: .source) ?? UIStrings.unknown
        sourceKind = try container.decodeIfPresent(String.self, forKey: .sourceKind)
            ?? container.decodeIfPresent(String.self, forKey: .sourceKindAlt)
            ?? UIStrings.unknown
        title = try container.decodeIfPresent(String.self, forKey: .title) ?? id
        task = try container.decodeIfPresent(String.self, forKey: .task)
        taskKind = try container.decodeIfPresent(String.self, forKey: .taskKind)
            ?? container.decodeIfPresent(String.self, forKey: .taskKindAlt)
            ?? UIStrings.unknown
        agent = try container.decodeIfPresent(String.self, forKey: .agent)
        provider = try container.decodeIfPresent(String.self, forKey: .provider) ?? UIStrings.unknown
        model = try container.decodeIfPresent(String.self, forKey: .model) ?? UIStrings.unknown
        destinationHost = try container.decodeIfPresent(String.self, forKey: .destinationHost)
            ?? container.decodeIfPresent(String.self, forKey: .destinationHostAlt)
        matchStatus = try container.decodeIfPresent(String.self, forKey: .matchStatus)
            ?? container.decodeIfPresent(String.self, forKey: .matchStatusAlt)
            ?? UIStrings.unknown
        confidenceScore = try container.decodeFlexibleProviderObservabilityInt(keys: [.confidenceScore, .confidenceScoreAlt])
        status = try container.decodeIfPresent(String.self, forKey: .status) ?? matchStatus
        latencyMS = try container.decodeFlexibleProviderObservabilityInt(keys: [.latencyMS, .latencyMSAlt])
        estimatedTotalTokens = try container.decodeFlexibleProviderObservabilityInt(keys: [.estimatedTotalTokens, .estimatedTotalTokensAlt]) ?? 0
        estimatedCostUSD = try container.decodeFlexibleProviderObservabilityDouble(keys: [.estimatedCostUSD, .estimatedCostUSDAlt])
        gapNotes = try container.decodeFlexibleProviderObservabilityStringArray(keys: [.gapNotes, .gapNotesAlt])
        blockerNotes = try container.decodeFlexibleProviderObservabilityStringArray(keys: [.blockerNotes, .blockerNotesAlt])
        outcomeNotes = try container.decodeFlexibleProviderObservabilityStringArray(keys: [.outcomeNotes, .outcomeNotesAlt])
        evidenceRefs = try container.decodeFlexibleProviderObservabilityStringArray(keys: [.evidenceRefs, .evidenceRefsAlt])
        redactionStatus = try container.decodeIfPresent(String.self, forKey: .redactionStatus)
            ?? container.decodeIfPresent(String.self, forKey: .redactionStatusAlt)
            ?? UIStrings.unknown
        safetyFlags = try container.decodeIfPresent(ProviderObservabilitySafety.self, forKey: .safetyFlags)
            ?? container.decodeIfPresent(ProviderObservabilitySafety.self, forKey: .safetyFlagsAlt)
            ?? ProviderObservabilitySafety()
    }

    var statusIsProblem: Bool {
        let value = matchStatus.lowercased()
        return value.contains("mismatch") || value.contains("unknown") || value.contains("fail") || value.contains("error")
    }
}

struct ProviderObservabilityHintRow: Decodable, Identifiable, Hashable {
    let id: String
    let severity: String
    let title: String
    let detail: String
    let value: String?
    let threshold: String?
    let recommendation: String?
    let evidenceRefs: [String]

    enum CodingKeys: String, CodingKey {
        case id
        case severity
        case level
        case title
        case name
        case detail
        case summary
        case message
        case reason
        case value
        case threshold
        case budgetState = "budget_state"
        case sourceFile = "source_file"
        case currentRecordCount = "current_record_count"
        case observedEstimatedTotalTokens = "observed_estimated_total_tokens"
        case observedEstimatedCostUSD = "observed_estimated_cost_usd"
        case monthlyBudgetUSD = "monthly_budget_usd"
        case recommendation
        case action
        case evidenceRefs = "evidence_refs"
        case evidenceRefsAlt = "evidenceRefs"
        case evidence
    }

    init(from decoder: Decoder) throws {
        if let value = try? decoder.singleValueContainer().decode(String.self) {
            id = "hint:\(value)"
            severity = UIStrings.unknown
            title = value
            detail = value
            self.value = nil
            threshold = nil
            recommendation = nil
            evidenceRefs = [value]
            return
        }

        let container = try decoder.container(keyedBy: CodingKeys.self)
        severity = try container.decodeIfPresent(String.self, forKey: .severity)
            ?? container.decodeIfPresent(String.self, forKey: .level)
            ?? UIStrings.unknown
        title = try container.decodeIfPresent(String.self, forKey: .title)
            ?? container.decodeIfPresent(String.self, forKey: .name)
            ?? container.decodeIfPresent(String.self, forKey: .budgetState)
            ?? container.decodeIfPresent(String.self, forKey: .sourceFile)
            ?? UIStrings.unknown
        detail = try container.decodeIfPresent(String.self, forKey: .detail)
            ?? container.decodeIfPresent(String.self, forKey: .summary)
            ?? container.decodeIfPresent(String.self, forKey: .message)
            ?? container.decodeIfPresent(String.self, forKey: .reason)
            ?? container.decodeIfPresent(String.self, forKey: .recommendation)
            ?? ""
        value = try container.decodeProviderObservabilityScalarString(keys: [.value, .observedEstimatedCostUSD, .observedEstimatedTotalTokens, .currentRecordCount])
        threshold = try container.decodeProviderObservabilityScalarString(keys: [.threshold, .monthlyBudgetUSD])
        recommendation = try container.decodeIfPresent(String.self, forKey: .recommendation)
            ?? container.decodeIfPresent(String.self, forKey: .action)
        evidenceRefs = try container.decodeFlexibleProviderObservabilityStringArray(keys: [.evidenceRefs, .evidenceRefsAlt, .evidence])
        id = try container.decodeIfPresent(String.self, forKey: .id) ?? "\(severity):\(title)"
    }

    var hasActivity: Bool {
        guard let value = value?.trimmingCharacters(in: .whitespacesAndNewlines), !value.isEmpty else {
            return false
        }
        let normalized = value
            .replacingOccurrences(of: "$", with: "")
            .replacingOccurrences(of: ",", with: "")
            .trimmingCharacters(in: .whitespacesAndNewlines)
        if let number = Double(normalized) {
            return number > 0
        }
        return true
    }
}

struct ProviderObservabilityEvidenceReference: Decodable, Identifiable, Hashable {
    let id: String
    let title: String
    let detail: String
    let source: String?
    let agent: String?

    enum CodingKeys: String, CodingKey {
        case id
        case title
        case label
        case detail
        case summary
        case ref
        case source
        case sourceType = "source_type"
        case sourceID = "source_id"
        case relatedInstanceID = "related_instance_id"
        case agent
    }

    init(id: String? = nil, title: String, detail: String, source: String? = nil, agent: String? = nil) {
        self.id = id ?? "\(title):\(detail):\(source ?? "")"
        self.title = title
        self.detail = detail
        self.source = source
        self.agent = agent
    }

    init(from decoder: Decoder) throws {
        if let value = try? decoder.singleValueContainer().decode(String.self) {
            id = value
            title = value
            detail = value
            source = nil
            agent = nil
            return
        }

        let container = try decoder.container(keyedBy: CodingKeys.self)
        title = try container.decodeIfPresent(String.self, forKey: .title)
            ?? container.decodeIfPresent(String.self, forKey: .label)
            ?? container.decodeIfPresent(String.self, forKey: .ref)
            ?? container.decodeIfPresent(String.self, forKey: .sourceID)
            ?? UIStrings.unknown
        detail = try container.decodeIfPresent(String.self, forKey: .detail)
            ?? container.decodeIfPresent(String.self, forKey: .summary)
            ?? container.decodeIfPresent(String.self, forKey: .ref)
            ?? container.decodeIfPresent(String.self, forKey: .sourceID)
            ?? container.decodeIfPresent(String.self, forKey: .relatedInstanceID)
            ?? title
        source = try container.decodeIfPresent(String.self, forKey: .source)
            ?? container.decodeIfPresent(String.self, forKey: .sourceType)
        agent = try container.decodeIfPresent(String.self, forKey: .agent)
        id = try container.decodeIfPresent(String.self, forKey: .id) ?? "\(title):\(detail):\(source ?? "")"
    }
}

struct ProviderObservabilityPromptRequest: Decodable, Hashable {
    let enabled: Bool
    let requestKind: String
    let summary: String
    let copyOnly: Bool
    let redacted: Bool

    enum CodingKeys: String, CodingKey {
        case enabled
        case available
        case requestKind = "request_kind"
        case requestKindAlt = "requestKind"
        case kind
        case action
        case method
        case summary
        case detail
        case note
        case copyOnly = "copy_only"
        case copyOnlyAlt = "copyOnly"
        case draftCopyOnly = "draft_copy_only"
        case providerRequestSent = "provider_request_sent"
        case redacted
        case redactionApplied = "redaction_applied"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        enabled = try container.decodeFlexibleProviderObservabilityBool(keys: [.enabled, .available, .providerRequestSent]) ?? false
        requestKind = try container.decodeIfPresent(String.self, forKey: .requestKind)
            ?? container.decodeIfPresent(String.self, forKey: .requestKindAlt)
            ?? container.decodeIfPresent(String.self, forKey: .kind)
            ?? container.decodeIfPresent(String.self, forKey: .action)
            ?? container.decodeIfPresent(String.self, forKey: .method)
            ?? "provider_observability"
        summary = try container.decodeIfPresent(String.self, forKey: .summary)
            ?? container.decodeIfPresent(String.self, forKey: .detail)
            ?? container.decodeIfPresent(String.self, forKey: .note)
            ?? ""
        copyOnly = try container.decodeFlexibleProviderObservabilityBool(keys: [.copyOnly, .copyOnlyAlt, .draftCopyOnly]) ?? true
        redacted = try container.decodeFlexibleProviderObservabilityBool(keys: [.redacted, .redactionApplied]) ?? true
    }
}

struct ProviderObservabilitySafety: Decodable, Hashable {
    let providerRequestSent: Bool
    let writeBackAllowed: Bool
    let writeActionsAvailable: Bool
    let scriptExecutionAllowed: Bool
    let executionActionsAvailable: Bool
    let configMutationAllowed: Bool
    let snapshotCreated: Bool
    let triageMutationAllowed: Bool
    let credentialAccessed: Bool
    let rawPromptPersisted: Bool
    let rawResponsePersisted: Bool
    let rawTracePersisted: Bool
    let cloudSyncEnabled: Bool
    let telemetryEnabled: Bool
    let rawSecretReturned: Bool
    let notes: [String]

    var allReadOnlyFlagsClear: Bool {
        !providerRequestSent
            && !writeBackAllowed
            && !writeActionsAvailable
            && !scriptExecutionAllowed
            && !executionActionsAvailable
            && !configMutationAllowed
            && !snapshotCreated
            && !triageMutationAllowed
            && !credentialAccessed
            && !rawPromptPersisted
            && !rawResponsePersisted
            && !rawTracePersisted
            && !cloudSyncEnabled
            && !telemetryEnabled
            && !rawSecretReturned
    }

    enum CodingKeys: String, CodingKey {
        case providerRequestSent = "provider_request_sent"
        case providerRequestSentAlt = "providerRequestSent"
        case writeBackAllowed = "write_back_allowed"
        case writeBackAllowedAlt = "writeBackAllowed"
        case writeActionsAvailable = "write_actions_available"
        case writeActionsAvailableAlt = "writeActionsAvailable"
        case scriptExecutionAllowed = "script_execution_allowed"
        case scriptExecutionAllowedAlt = "scriptExecutionAllowed"
        case executionActionsAvailable = "execution_actions_available"
        case executionActionsAvailableAlt = "executionActionsAvailable"
        case configMutationAllowed = "config_mutation_allowed"
        case configMutationAllowedAlt = "configMutationAllowed"
        case snapshotCreated = "snapshot_created"
        case snapshotCreatedAlt = "snapshotCreated"
        case triageMutationAllowed = "triage_mutation_allowed"
        case triageMutationAllowedAlt = "triageMutationAllowed"
        case credentialAccessed = "credential_accessed"
        case credentialAccessedAlt = "credentialAccessed"
        case rawPromptPersisted = "raw_prompt_persisted"
        case rawPromptPersistedAlt = "rawPromptPersisted"
        case rawResponsePersisted = "raw_response_persisted"
        case rawResponsePersistedAlt = "rawResponsePersisted"
        case rawTracePersisted = "raw_trace_persisted"
        case rawTracePersistedAlt = "rawTracePersisted"
        case cloudSyncEnabled = "cloud_sync_enabled"
        case cloudSyncEnabledAlt = "cloudSyncEnabled"
        case cloudSyncPerformed = "cloud_sync_performed"
        case cloudSyncPerformedAlt = "cloudSyncPerformed"
        case telemetryEnabled = "telemetry_enabled"
        case telemetryEnabledAlt = "telemetryEnabled"
        case telemetryEmitted = "telemetry_emitted"
        case telemetryEmittedAlt = "telemetryEmitted"
        case rawSecretReturned = "raw_secret_returned"
        case rawSecretReturnedAlt = "rawSecretReturned"
        case notes
        case safetyFlags = "safety_flags"
    }

    init(
        providerRequestSent: Bool = false,
        writeBackAllowed: Bool = false,
        writeActionsAvailable: Bool = false,
        scriptExecutionAllowed: Bool = false,
        executionActionsAvailable: Bool = false,
        configMutationAllowed: Bool = false,
        snapshotCreated: Bool = false,
        triageMutationAllowed: Bool = false,
        credentialAccessed: Bool = false,
        rawPromptPersisted: Bool = false,
        rawResponsePersisted: Bool = false,
        rawTracePersisted: Bool = false,
        cloudSyncEnabled: Bool = false,
        telemetryEnabled: Bool = false,
        rawSecretReturned: Bool = false,
        notes: [String] = []
    ) {
        self.providerRequestSent = providerRequestSent
        self.writeBackAllowed = writeBackAllowed
        self.writeActionsAvailable = writeActionsAvailable
        self.scriptExecutionAllowed = scriptExecutionAllowed
        self.executionActionsAvailable = executionActionsAvailable
        self.configMutationAllowed = configMutationAllowed
        self.snapshotCreated = snapshotCreated
        self.triageMutationAllowed = triageMutationAllowed
        self.credentialAccessed = credentialAccessed
        self.rawPromptPersisted = rawPromptPersisted
        self.rawResponsePersisted = rawResponsePersisted
        self.rawTracePersisted = rawTracePersisted
        self.cloudSyncEnabled = cloudSyncEnabled
        self.telemetryEnabled = telemetryEnabled
        self.rawSecretReturned = rawSecretReturned
        self.notes = notes
    }

    init(from decoder: Decoder) throws {
        if let values = try? decoder.singleValueContainer().decode([String].self) {
            self.init(notes: values)
            return
        }

        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init(
            providerRequestSent: try container.decodeFlexibleProviderObservabilityBool(keys: [.providerRequestSent, .providerRequestSentAlt]) ?? false,
            writeBackAllowed: try container.decodeFlexibleProviderObservabilityBool(keys: [.writeBackAllowed, .writeBackAllowedAlt]) ?? false,
            writeActionsAvailable: try container.decodeFlexibleProviderObservabilityBool(keys: [.writeActionsAvailable, .writeActionsAvailableAlt]) ?? false,
            scriptExecutionAllowed: try container.decodeFlexibleProviderObservabilityBool(keys: [.scriptExecutionAllowed, .scriptExecutionAllowedAlt]) ?? false,
            executionActionsAvailable: try container.decodeFlexibleProviderObservabilityBool(keys: [.executionActionsAvailable, .executionActionsAvailableAlt]) ?? false,
            configMutationAllowed: try container.decodeFlexibleProviderObservabilityBool(keys: [.configMutationAllowed, .configMutationAllowedAlt]) ?? false,
            snapshotCreated: try container.decodeFlexibleProviderObservabilityBool(keys: [.snapshotCreated, .snapshotCreatedAlt]) ?? false,
            triageMutationAllowed: try container.decodeFlexibleProviderObservabilityBool(keys: [.triageMutationAllowed, .triageMutationAllowedAlt]) ?? false,
            credentialAccessed: try container.decodeFlexibleProviderObservabilityBool(keys: [.credentialAccessed, .credentialAccessedAlt]) ?? false,
            rawPromptPersisted: try container.decodeFlexibleProviderObservabilityBool(keys: [.rawPromptPersisted, .rawPromptPersistedAlt]) ?? false,
            rawResponsePersisted: try container.decodeFlexibleProviderObservabilityBool(keys: [.rawResponsePersisted, .rawResponsePersistedAlt]) ?? false,
            rawTracePersisted: try container.decodeFlexibleProviderObservabilityBool(keys: [.rawTracePersisted, .rawTracePersistedAlt]) ?? false,
            cloudSyncEnabled: try container.decodeFlexibleProviderObservabilityBool(keys: [.cloudSyncEnabled, .cloudSyncEnabledAlt, .cloudSyncPerformed, .cloudSyncPerformedAlt]) ?? false,
            telemetryEnabled: try container.decodeFlexibleProviderObservabilityBool(keys: [.telemetryEnabled, .telemetryEnabledAlt, .telemetryEmitted, .telemetryEmittedAlt]) ?? false,
            rawSecretReturned: try container.decodeFlexibleProviderObservabilityBool(keys: [.rawSecretReturned, .rawSecretReturnedAlt]) ?? false,
            notes: try container.decodeFlexibleProviderObservabilityStringArray(keys: [.notes, .safetyFlags])
        )
    }
}

struct ProviderObservabilityResult: Decodable, Hashable {
    let generatedBy: String
    let appLocalOnly: Bool
    let metadataRedacted: Bool
    let filters: ProviderObservabilityFilters
    let summary: ProviderObservabilitySummary
    let callRows: [ProviderObservabilityCallRow]
    let providerRows: [ProviderObservabilityDimensionRow]
    let modelRows: [ProviderObservabilityDimensionRow]
    let destinationRows: [ProviderObservabilityDimensionRow]
    let modelTaskHistoryRows: [ProviderObservabilityModelTaskHistoryRow]
    let statusRows: [ProviderObservabilityIssueRow]
    let errorRows: [ProviderObservabilityIssueRow]
    let budgetHints: [ProviderObservabilityHintRow]
    let usageHints: [ProviderObservabilityHintRow]
    let retentionRows: [ProviderObservabilityHintRow]
    let cleanupRecommendationRows: [ProviderObservabilityHintRow]
    let gapNotes: [String]
    let blockerNotes: [String]
    let evidenceReferences: [ProviderObservabilityEvidenceReference]
    let promptRequest: ProviderObservabilityPromptRequest?
    let safetyFlags: ProviderObservabilitySafety
    let fallbackReason: String?

    var isUnavailable: Bool {
        generatedBy == "unavailable" || fallbackReason != nil && callRows.isEmpty && providerRows.isEmpty && modelRows.isEmpty
    }

    var isDashboardEmpty: Bool {
        !hasProviderMetadata
    }

    private var hasProviderMetadata: Bool {
        summary.callCount > 0
            || summary.successCount > 0
            || summary.failureCount > 0
            || summary.blockedCount > 0
            || summary.errorCount > 0
            || summary.estimatedInputTokens > 0
            || summary.estimatedOutputTokens > 0
            || summary.estimatedTotalTokens > 0
            || (summary.estimatedCostUSD ?? 0) > 0
            || summary.totalDurationMS > 0
            || !callRows.isEmpty
            || providerRows.contains(where: \.hasActivity)
            || modelRows.contains(where: \.hasActivity)
            || destinationRows.contains(where: \.hasActivity)
            || !modelTaskHistoryRows.isEmpty
            || statusRows.contains { $0.count > 0 }
            || errorRows.contains { $0.count > 0 }
            || budgetHints.contains(where: \.hasActivity)
            || usageHints.contains(where: \.hasActivity)
            || retentionRows.contains(where: \.hasActivity)
            || cleanupRecommendationRows.contains(where: \.hasActivity)
    }

    enum CodingKeys: String, CodingKey {
        case generatedBy = "generated_by"
        case generatedByAlt = "generatedBy"
        case appLocalOnly = "app_local_only"
        case appLocalOnlyAlt = "appLocalOnly"
        case localOnly = "local_only"
        case metadataRedacted = "metadata_redacted"
        case metadataRedactedAlt = "metadataRedacted"
        case redacted
        case filters
        case summary
        case callRows = "call_rows"
        case callRowsAlt = "callRows"
        case calls
        case historyRows = "history_rows"
        case historyRowsAlt = "historyRows"
        case recentCalls = "recent_calls"
        case records
        case rows
        case groupingRows = "grouping_rows"
        case groupingRowsAlt = "groupingRows"
        case providerRows = "provider_rows"
        case providerRowsAlt = "providerRows"
        case providers
        case modelRows = "model_rows"
        case modelRowsAlt = "modelRows"
        case models
        case destinationRows = "destination_rows"
        case destinationRowsAlt = "destinationRows"
        case destinations
        case modelTaskHistoryRows = "model_task_history_rows"
        case modelTaskHistoryRowsAlt = "modelTaskHistoryRows"
        case statusRows = "status_rows"
        case statusRowsAlt = "statusRows"
        case statuses
        case errorRows = "error_rows"
        case errorRowsAlt = "errorRows"
        case errors
        case budgetHints = "budget_hints"
        case budgetHintsAlt = "budgetHints"
        case budgetUsageHints = "budget_usage_hints"
        case budgetUsageHintsAlt = "budgetUsageHints"
        case usageHints = "usage_hints"
        case usageHintsAlt = "usageHints"
        case retentionRows = "retention_rows"
        case retentionRowsAlt = "retentionRows"
        case retentionRecommendations = "retention_recommendations"
        case retentionRecommendationsAlt = "retentionRecommendations"
        case retention
        case cleanupRecommendationRows = "cleanup_recommendation_rows"
        case cleanupRecommendations = "cleanup_recommendations"
        case recommendations
        case gapNotes = "gap_notes"
        case gapNotesAlt = "gapNotes"
        case gaps
        case blockerNotes = "blocker_notes"
        case blockerNotesAlt = "blockerNotes"
        case blockers
        case evidenceReferences = "evidence_references"
        case evidenceReferencesAlt = "evidenceReferences"
        case evidence
        case promptRequest = "prompt_request"
        case promptRequestAlt = "promptRequest"
        case promptMetadata = "prompt_metadata"
        case promptMetadataAlt = "promptMetadata"
        case safetyFlags = "safety_flags"
        case safetyFlagsAlt = "safetyFlags"
        case safety
        case fallbackReason = "fallback_reason"
        case fallbackReasonAlt = "fallbackReason"
        case reason
    }

    init(
        generatedBy: String = "local-v2.64",
        appLocalOnly: Bool = true,
        metadataRedacted: Bool = true,
        filters: ProviderObservabilityFilters = ProviderObservabilityFilters(),
        summary: ProviderObservabilitySummary = ProviderObservabilitySummary(),
        callRows: [ProviderObservabilityCallRow] = [],
        providerRows: [ProviderObservabilityDimensionRow] = [],
        modelRows: [ProviderObservabilityDimensionRow] = [],
        destinationRows: [ProviderObservabilityDimensionRow] = [],
        modelTaskHistoryRows: [ProviderObservabilityModelTaskHistoryRow] = [],
        statusRows: [ProviderObservabilityIssueRow] = [],
        errorRows: [ProviderObservabilityIssueRow] = [],
        budgetHints: [ProviderObservabilityHintRow] = [],
        usageHints: [ProviderObservabilityHintRow] = [],
        retentionRows: [ProviderObservabilityHintRow] = [],
        cleanupRecommendationRows: [ProviderObservabilityHintRow] = [],
        gapNotes: [String] = [],
        blockerNotes: [String] = [],
        evidenceReferences: [ProviderObservabilityEvidenceReference] = [],
        promptRequest: ProviderObservabilityPromptRequest? = nil,
        safetyFlags: ProviderObservabilitySafety = ProviderObservabilitySafety(),
        fallbackReason: String? = nil
    ) {
        self.generatedBy = generatedBy
        self.appLocalOnly = appLocalOnly
        self.metadataRedacted = metadataRedacted
        self.filters = filters
        self.summary = summary
        self.callRows = callRows
        self.providerRows = providerRows
        self.modelRows = modelRows
        self.destinationRows = destinationRows
        self.modelTaskHistoryRows = modelTaskHistoryRows
        self.statusRows = statusRows
        self.errorRows = errorRows
        self.budgetHints = budgetHints
        self.usageHints = usageHints
        self.retentionRows = retentionRows
        self.cleanupRecommendationRows = cleanupRecommendationRows
        self.gapNotes = gapNotes
        self.blockerNotes = blockerNotes
        self.evidenceReferences = evidenceReferences
        self.promptRequest = promptRequest
        self.safetyFlags = safetyFlags
        self.fallbackReason = fallbackReason
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let decodedCalls = try container.decodeProviderObservabilityRows(
            type: ProviderObservabilityCallRow.self,
            keys: [.callRows, .callRowsAlt, .calls, .historyRows, .historyRowsAlt, .recentCalls, .records, .rows]
        )
        let decodedProviderRows = try container.decodeProviderObservabilityRows(
            type: ProviderObservabilityDimensionRow.self,
            keys: [.providerRows, .providerRowsAlt, .providers]
        )
        let decodedGroupingRows = try container.decodeProviderObservabilityRows(
            type: ProviderObservabilityDimensionRow.self,
            keys: [.groupingRows, .groupingRowsAlt]
        )
        let decodedModelRows = try container.decodeProviderObservabilityRows(
            type: ProviderObservabilityDimensionRow.self,
            keys: [.modelRows, .modelRowsAlt, .models]
        )
        let decodedDestinationRows = try container.decodeProviderObservabilityRows(
            type: ProviderObservabilityDimensionRow.self,
            keys: [.destinationRows, .destinationRowsAlt, .destinations]
        )
        let decodedModelTaskHistoryRows = try container.decodeProviderObservabilityRows(
            type: ProviderObservabilityModelTaskHistoryRow.self,
            keys: [.modelTaskHistoryRows, .modelTaskHistoryRowsAlt]
        )
        let decodedStatusRows = try container.decodeProviderObservabilityRows(
            type: ProviderObservabilityIssueRow.self,
            keys: [.statusRows, .statusRowsAlt, .statuses]
        )
        let decodedErrorRows = try container.decodeProviderObservabilityRows(
            type: ProviderObservabilityIssueRow.self,
            keys: [.errorRows, .errorRowsAlt, .errors]
        )
        let decodedBudgetHints = try container.decodeProviderObservabilityRows(
            type: ProviderObservabilityHintRow.self,
            keys: [.budgetHints, .budgetHintsAlt, .budgetUsageHints, .budgetUsageHintsAlt]
        )
        let decodedUsageHints = try container.decodeProviderObservabilityRows(
            type: ProviderObservabilityHintRow.self,
            keys: [.usageHints, .usageHintsAlt]
        )
        let decodedRetentionRows = try container.decodeProviderObservabilityRows(
            type: ProviderObservabilityHintRow.self,
            keys: [.retentionRows, .retentionRowsAlt, .retentionRecommendations, .retentionRecommendationsAlt, .retention]
        )
        let decodedCleanupRows = try container.decodeProviderObservabilityRows(
            type: ProviderObservabilityHintRow.self,
            keys: [.cleanupRecommendationRows, .cleanupRecommendations, .recommendations]
        )
        generatedBy = try container.decodeIfPresent(String.self, forKey: .generatedBy)
            ?? container.decodeIfPresent(String.self, forKey: .generatedByAlt)
            ?? "local-v2.64"
        appLocalOnly = try container.decodeFlexibleProviderObservabilityBool(keys: [.appLocalOnly, .appLocalOnlyAlt, .localOnly]) ?? true
        metadataRedacted = try container.decodeFlexibleProviderObservabilityBool(keys: [.metadataRedacted, .metadataRedactedAlt, .redacted]) ?? true
        filters = try container.decodeIfPresent(ProviderObservabilityFilters.self, forKey: .filters) ?? ProviderObservabilityFilters()
        let normalizedProviderRows = decodedProviderRows.isEmpty
            ? Self.aggregateGroupingRows(decodedGroupingRows, kind: "provider")
            : decodedProviderRows
        let normalizedModelRows = decodedModelRows.isEmpty
            ? Self.aggregateGroupingRows(decodedGroupingRows, kind: "model")
            : decodedModelRows
        let normalizedDestinationRows = decodedDestinationRows.isEmpty
            ? Self.aggregateGroupingRows(decodedGroupingRows, kind: "destination")
            : decodedDestinationRows
        summary = try container.decodeIfPresent(ProviderObservabilitySummary.self, forKey: .summary)
            ?? ProviderObservabilitySummary(
                callCount: decodedCalls.count,
                successCount: decodedCalls.filter { !$0.statusIsProblem }.count,
                failureCount: decodedCalls.filter(\.statusIsProblem).count,
                providerCount: normalizedProviderRows.count,
                modelCount: normalizedModelRows.count,
                destinationCount: normalizedDestinationRows.count,
                errorCount: decodedErrorRows.count,
                estimatedInputTokens: decodedCalls.reduce(0) { $0 + $1.inputTokens },
                estimatedOutputTokens: decodedCalls.reduce(0) { $0 + $1.outputTokens },
                estimatedTotalTokens: decodedCalls.reduce(0) { $0 + $1.totalTokens },
                estimatedCostUSD: decodedCalls.compactMap(\.estimatedCostUSD).reduce(0, +),
                totalDurationMS: decodedCalls.compactMap(\.durationMS).reduce(0, +),
                budgetHintCount: decodedBudgetHints.count,
                retentionRecommendationCount: decodedRetentionRows.count + decodedCleanupRows.count
            )
        callRows = decodedCalls
        providerRows = normalizedProviderRows
        modelRows = normalizedModelRows
        destinationRows = normalizedDestinationRows
        modelTaskHistoryRows = decodedModelTaskHistoryRows
        statusRows = decodedStatusRows
        errorRows = decodedErrorRows
        budgetHints = decodedBudgetHints
        usageHints = decodedUsageHints
        retentionRows = decodedRetentionRows
        cleanupRecommendationRows = decodedCleanupRows
        gapNotes = try container.decodeFlexibleProviderObservabilityStringArray(keys: [.gapNotes, .gapNotesAlt, .gaps])
        blockerNotes = try container.decodeFlexibleProviderObservabilityStringArray(keys: [.blockerNotes, .blockerNotesAlt, .blockers])
        evidenceReferences = try container.decodeProviderObservabilityRows(
            type: ProviderObservabilityEvidenceReference.self,
            keys: [.evidenceReferences, .evidenceReferencesAlt, .evidence]
        )
        promptRequest = try container.decodeIfPresent(ProviderObservabilityPromptRequest.self, forKey: .promptRequest)
            ?? container.decodeIfPresent(ProviderObservabilityPromptRequest.self, forKey: .promptRequestAlt)
            ?? container.decodeIfPresent(ProviderObservabilityPromptRequest.self, forKey: .promptMetadata)
            ?? container.decodeIfPresent(ProviderObservabilityPromptRequest.self, forKey: .promptMetadataAlt)
        safetyFlags = try container.decodeIfPresent(ProviderObservabilitySafety.self, forKey: .safetyFlags)
            ?? container.decodeIfPresent(ProviderObservabilitySafety.self, forKey: .safetyFlagsAlt)
            ?? container.decodeIfPresent(ProviderObservabilitySafety.self, forKey: .safety)
            ?? ProviderObservabilitySafety()
        fallbackReason = try container.decodeIfPresent(String.self, forKey: .fallbackReason)
            ?? container.decodeIfPresent(String.self, forKey: .fallbackReasonAlt)
            ?? container.decodeIfPresent(String.self, forKey: .reason)
    }

    static func unavailable(reason: String = UIStrings.providerObservabilityUnavailable) -> ProviderObservabilityResult {
        ProviderObservabilityResult(
            generatedBy: "unavailable",
            appLocalOnly: true,
            metadataRedacted: true,
            fallbackReason: reason
        )
    }

    private static func aggregateGroupingRows(
        _ rows: [ProviderObservabilityDimensionRow],
        kind: String
    ) -> [ProviderObservabilityDimensionRow] {
        var accumulators: [String: ProviderObservabilityDimensionAccumulator] = [:]
        for row in rows {
            guard let label = groupingLabel(for: row, kind: kind) else {
                continue
            }
            accumulators[label, default: ProviderObservabilityDimensionAccumulator(label: label)]
                .add(row)
        }

        return accumulators.values
            .sorted { left, right in
                left.label.localizedCaseInsensitiveCompare(right.label) == .orderedAscending
            }
            .map { $0.row(kind: kind) }
    }

    private static func groupingLabel(for row: ProviderObservabilityDimensionRow, kind: String) -> String? {
        let value: String?
        switch kind {
        case "provider":
            value = row.provider
        case "model":
            value = row.model
        case "destination":
            value = row.destinationHost
        default:
            value = row.label
        }
        return value?.providerObservabilityNonEmpty
    }
}

private struct ProviderObservabilityDimensionAccumulator {
    let label: String
    var callCount = 0
    var successCount = 0
    var failureCount = 0
    var blockedCount = 0
    var estimatedTokens = 0
    var estimatedCostUSD = 0.0
    var hasEstimatedCost = false
    var notes: [String] = []
    var evidenceRefs: [String] = []
    var safetyFlags: [String] = []

    mutating func add(_ row: ProviderObservabilityDimensionRow) {
        callCount += row.callCount
        successCount += row.successCount
        failureCount += row.failureCount
        blockedCount += row.blockedCount
        estimatedTokens += row.estimatedTokens
        if let cost = row.estimatedCostUSD {
            estimatedCostUSD += cost
            hasEstimatedCost = true
        }
        appendUnique(row.notes, to: &notes)
        appendUnique(row.evidenceRefs, to: &evidenceRefs)
        appendUnique(row.safetyFlags, to: &safetyFlags)
    }

    func row(kind: String) -> ProviderObservabilityDimensionRow {
        ProviderObservabilityDimensionRow(
            id: "\(kind):\(label)",
            kind: kind,
            label: label,
            provider: kind == "provider" ? label : nil,
            model: kind == "model" ? label : nil,
            destinationHost: kind == "destination" ? label : nil,
            callCount: callCount,
            successCount: successCount,
            failureCount: failureCount,
            blockedCount: blockedCount,
            estimatedTokens: estimatedTokens,
            estimatedCostUSD: hasEstimatedCost ? estimatedCostUSD : nil,
            status: resolvedStatus,
            notes: notes,
            evidenceRefs: evidenceRefs,
            safetyFlags: safetyFlags
        )
    }

    private var resolvedStatus: String {
        if failureCount > 0 || blockedCount > 0 {
            return "partial"
        }
        if successCount > 0 || callCount > 0 {
            return "ok"
        }
        return UIStrings.unknown
    }

    private func appendUnique(_ values: [String], to target: inout [String]) {
        for value in values where !target.contains(value) {
            target.append(value)
        }
    }
}

private extension KeyedDecodingContainer {
    func decodeFlexibleProviderObservabilityInt(keys: [Key]) throws -> Int? {
        for key in keys {
            if let value = try? decodeIfPresent(Int.self, forKey: key) {
                return value
            }
            if let value = try? decodeIfPresent(Double.self, forKey: key) {
                return Int(value.rounded())
            }
            if let value = try? decodeIfPresent(String.self, forKey: key),
               let int = Int(value.trimmingCharacters(in: .whitespacesAndNewlines)) {
                return int
            }
            if let values = try? decodeIfPresent([String].self, forKey: key) {
                return values.count
            }
            if let values = try? decodeIfPresent([ProviderObservabilityCallRow].self, forKey: key) {
                return values.count
            }
            if let values = try? decodeIfPresent([ProviderObservabilityDimensionRow].self, forKey: key) {
                return values.count
            }
            if let values = try? decodeIfPresent([ProviderObservabilityIssueRow].self, forKey: key) {
                return values.count
            }
            if let values = try? decodeIfPresent([ProviderObservabilityHintRow].self, forKey: key) {
                return values.count
            }
        }
        return nil
    }

    func decodeFlexibleProviderObservabilityDouble(keys: [Key]) throws -> Double? {
        for key in keys {
            if let value = try? decodeIfPresent(Double.self, forKey: key) {
                return value
            }
            if let value = try? decodeIfPresent(Int.self, forKey: key) {
                return Double(value)
            }
            if let value = try? decodeIfPresent(String.self, forKey: key),
               let double = Double(value.trimmingCharacters(in: .whitespacesAndNewlines)) {
                return double
            }
        }
        return nil
    }

    func decodeFlexibleProviderObservabilityBool(keys: [Key]) throws -> Bool? {
        for key in keys {
            if let value = try? decodeIfPresent(Bool.self, forKey: key) {
                return value
            }
            if let value = try? decodeIfPresent(Int.self, forKey: key) {
                return value != 0
            }
            if let value = try? decodeIfPresent(String.self, forKey: key) {
                switch value.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
                case "true", "yes", "1", "enabled":
                    return true
                case "false", "no", "0", "disabled", "blocked":
                    return false
                default:
                    break
                }
            }
        }
        return nil
    }

    func decodeFlexibleProviderObservabilityStringArray(keys: [Key]) throws -> [String] {
        for key in keys {
            if let values = try? decodeIfPresent([String].self, forKey: key) {
                return values.compactMap(\.providerObservabilityNonEmpty)
            }
            if let value = try? decodeIfPresent(String.self, forKey: key),
               let text = value.providerObservabilityNonEmpty {
                return [text]
            }
            if let values = try? decodeIfPresent([ProviderObservabilityEvidenceReference].self, forKey: key) {
                return values.map { $0.detail.providerObservabilityNonEmpty ?? $0.title }
            }
        }
        return []
    }

    func decodeProviderObservabilityScalarString(keys: [Key]) throws -> String? {
        for key in keys {
            if let value = try? decodeIfPresent(String.self, forKey: key) {
                return value.providerObservabilityNonEmpty
            }
            if let value = try? decodeIfPresent(Int.self, forKey: key) {
                return String(value)
            }
            if let value = try? decodeIfPresent(Double.self, forKey: key) {
                return value.formatted()
            }
            if let value = try? decodeIfPresent(Bool.self, forKey: key) {
                return value ? UIStrings.llmEnabled : UIStrings.llmDisabled
            }
        }
        return nil
    }

    func decodeProviderObservabilityRows<T: Decodable>(type: T.Type, keys: [Key]) throws -> [T] {
        for key in keys {
            if let values = try? decodeIfPresent([T].self, forKey: key) {
                return values
            }
            if let value = try? decodeIfPresent(T.self, forKey: key) {
                return [value]
            }
        }
        return []
    }
}

private extension String {
    var providerObservabilityNonEmpty: String? {
        let trimmed = trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }
}
