import Foundation

struct LLMPromptField: Decodable, Hashable, Identifiable {
    let name: String
    let label: String
    let reason: String?

    var id: String { "\(name)-\(label)-\(reason ?? "")" }

    enum CodingKeys: String, CodingKey {
        case name
        case field
        case label
        case title
        case reason
    }

    init(name: String, label: String? = nil, reason: String? = nil) {
        self.name = name
        self.label = label ?? name
        self.reason = reason
    }

    init(from decoder: Decoder) throws {
        if let value = try? decoder.singleValueContainer().decode(String.self) {
            name = value
            label = value
            reason = nil
            return
        }

        let container = try decoder.container(keyedBy: CodingKeys.self)
        let decodedName = try container.decodeIfPresent(String.self, forKey: .name)
            ?? container.decodeIfPresent(String.self, forKey: .field)
            ?? UIStrings.unknown
        name = decodedName
        label = try container.decodeIfPresent(String.self, forKey: .label)
            ?? container.decodeIfPresent(String.self, forKey: .title)
            ?? decodedName
        reason = try container.decodeIfPresent(String.self, forKey: .reason)
    }
}

struct LLMPromptRedactionSummary: Decodable, Hashable {
    let status: String
    let summary: String
    let redactedFields: [String]
    let placeholders: [String]
    let warnings: [String]

    enum CodingKeys: String, CodingKey {
        case status
        case summary
        case redactionSummary = "redaction_summary"
        case redactedFields = "redacted_fields"
        case removedFields = "removed_fields"
        case placeholders
        case warnings
    }

    init(
        status: String = "unknown",
        summary: String = "",
        redactedFields: [String] = [],
        placeholders: [String] = [],
        warnings: [String] = []
    ) {
        self.status = status
        self.summary = summary
        self.redactedFields = redactedFields
        self.placeholders = placeholders
        self.warnings = warnings
    }

    init(from decoder: Decoder) throws {
        if let value = try? decoder.singleValueContainer().decode(String.self) {
            status = value.isEmpty ? "unknown" : value
            summary = value
            redactedFields = []
            placeholders = []
            warnings = []
            return
        }

        let container = try decoder.container(keyedBy: CodingKeys.self)
        status = try container.decodeIfPresent(String.self, forKey: .status) ?? "unknown"
        summary = try container.decodeIfPresent(String.self, forKey: .summary)
            ?? container.decodeIfPresent(String.self, forKey: .redactionSummary)
            ?? ""
        redactedFields = try container.decodeIfPresent([String].self, forKey: .redactedFields)
            ?? container.decodeIfPresent([String].self, forKey: .removedFields)
            ?? []
        placeholders = try container.decodeIfPresent([String].self, forKey: .placeholders) ?? []
        warnings = try container.decodeIfPresent([String].self, forKey: .warnings) ?? []
    }
}

struct LLMPromptPreview: Decodable, Identifiable, Hashable {
    let previewID: String
    let action: LLMAction?
    let actionDescriptor: ActionDescriptorWire?
    let preconditions: [ActionPreconditionWire]
    let previewToken: String?
    let analysisKind: String?
    let requestKind: String?
    let scope: String?
    let promptScope: String
    let enabled: Bool
    let disabledReason: String?
    let provider: String?
    let model: String?
    let endpoint: String?
    let destinationHost: String?
    let includedFields: [LLMPromptField]
    let excludedFields: [LLMPromptField]
    let redaction: LLMPromptRedactionSummary
    let estimate: LLMTokenCostEstimate?
    let confirmationRequired: Bool
    let rawPromptPersisted: Bool
    let rawResponsePersisted: Bool
    let draftCopyOnly: Bool
    let promptPreview: String?
    let audit: AIProviderCallAuditMetadata?

    var id: String { previewID }

    enum CodingKeys: String, CodingKey {
        case previewID = "preview_id"
        case id
        case confirmationID = "confirmation_id"
        case action
        case preconditions
        case previewToken = "preview_token"
        case kind
        case analysisKind = "analysis_kind"
        case requestKind = "request_kind"
        case scope
        case promptScope = "prompt_scope"
        case scopeLabel = "scope_label"
        case enabled
        case allowed
        case disabledReason = "disabled_reason"
        case reason
        case provider
        case providerType = "provider_type"
        case model
        case destinationHost = "destination_host"
        case networkDestination = "network_destination"
        case endpoint
        case host
        case includedFields = "included_fields"
        case excludedFields = "excluded_fields"
        case redaction
        case redactionSummary = "redaction_summary"
        case estimate
        case estimatedInputTokens = "estimated_input_tokens"
        case estimatedOutputTokens = "estimated_output_tokens"
        case estimatedTotalTokens = "estimated_total_tokens"
        case estimatedCostUSD = "estimated_cost_usd"
        case confirmationRequired = "confirmation_required"
        case requiresConfirmation = "requires_confirmation"
        case rawPromptPersisted = "raw_prompt_persisted"
        case promptStored = "prompt_stored"
        case rawResponsePersisted = "raw_response_persisted"
        case responseStored = "response_stored"
        case draftCopyOnly = "draft_copy_only"
        case copyOnly = "copy_only"
        case outputReadOnly = "output_read_only"
        case readOnly = "read_only"
        case promptPreview = "prompt_preview"
        case redactedPromptPreview = "redacted_prompt_preview"
        case redactedPrompt = "redacted_prompt"
        case sanitizedPrompt = "sanitized_prompt"
        case audit
        case auditMetadata = "audit_metadata"
        case metadata
    }

    init(
        previewID: String,
        action: LLMAction?,
        analysisKind: String?,
        requestKind: String?,
        scope: String?,
        promptScope: String,
        enabled: Bool,
        disabledReason: String?,
        provider: String?,
        model: String?,
        destinationHost: String?,
        includedFields: [LLMPromptField],
        excludedFields: [LLMPromptField],
        redaction: LLMPromptRedactionSummary,
        estimate: LLMTokenCostEstimate?,
        confirmationRequired: Bool,
        rawPromptPersisted: Bool,
        rawResponsePersisted: Bool,
        draftCopyOnly: Bool,
        promptPreview: String?,
        audit: AIProviderCallAuditMetadata?,
        actionDescriptor: ActionDescriptorWire? = nil,
        preconditions: [ActionPreconditionWire] = [],
        previewToken: String? = nil,
        endpoint: String? = nil
    ) {
        self.previewID = previewID
        self.action = action
        self.actionDescriptor = actionDescriptor
        self.preconditions = preconditions
        self.previewToken = previewToken
        self.analysisKind = analysisKind
        self.requestKind = requestKind
        self.scope = scope
        self.promptScope = promptScope
        self.enabled = enabled
        self.disabledReason = disabledReason
        self.provider = provider
        self.model = model
        self.endpoint = endpoint
        self.destinationHost = destinationHost
        self.includedFields = includedFields
        self.excludedFields = excludedFields
        self.redaction = redaction
        self.estimate = estimate
        self.confirmationRequired = confirmationRequired
        self.rawPromptPersisted = rawPromptPersisted
        self.rawResponsePersisted = rawResponsePersisted
        self.draftCopyOnly = draftCopyOnly
        self.promptPreview = promptPreview
        self.audit = audit
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        previewID = try container.decodeIfPresent(String.self, forKey: .previewID)
            ?? container.decodeIfPresent(String.self, forKey: .id)
            ?? container.decodeIfPresent(String.self, forKey: .confirmationID)
            ?? ""
        action = try Self.decodeAction(from: container, keys: [.requestKind, .action, .kind])
        actionDescriptor = try? container.decode(ActionDescriptorWire.self, forKey: .action)
        preconditions = try container.decodeIfPresent([ActionPreconditionWire].self, forKey: .preconditions) ?? []
        previewToken = try container.decodeIfPresent(String.self, forKey: .previewToken)
        analysisKind = try Self.decodeFlexibleString(from: container, keys: [.analysisKind])
        requestKind = try container.decodeIfPresent(String.self, forKey: .requestKind)
        scope = try container.decodeIfPresent(String.self, forKey: .scope)
        promptScope = try Self.decodeFlexibleString(from: container, keys: [.promptScope, .scopeLabel])
            ?? scope
            ?? UIStrings.unknown
        enabled = try container.decodeIfPresent(Bool.self, forKey: .enabled)
            ?? container.decodeIfPresent(Bool.self, forKey: .allowed)
            ?? !previewID.isEmpty
        disabledReason = try container.decodeIfPresent(String.self, forKey: .disabledReason)
            ?? container.decodeIfPresent(String.self, forKey: .reason)
        provider = try container.decodeIfPresent(String.self, forKey: .provider)
            ?? container.decodeIfPresent(String.self, forKey: .providerType)
        model = try container.decodeIfPresent(String.self, forKey: .model)
        endpoint = try container.decodeIfPresent(String.self, forKey: .endpoint)
        destinationHost = try container.decodeIfPresent(String.self, forKey: .destinationHost)
            ?? container.decodeIfPresent(String.self, forKey: .networkDestination)
            ?? endpoint
            ?? container.decodeIfPresent(String.self, forKey: .host)
        includedFields = try container.decodeIfPresent([LLMPromptField].self, forKey: .includedFields) ?? []
        excludedFields = try container.decodeIfPresent([LLMPromptField].self, forKey: .excludedFields) ?? []
        redaction = try container.decodeIfPresent(LLMPromptRedactionSummary.self, forKey: .redaction)
            ?? container.decodeIfPresent(LLMPromptRedactionSummary.self, forKey: .redactionSummary)
            ?? LLMPromptRedactionSummary()
        if let nestedEstimate = try container.decodeIfPresent(LLMTokenCostEstimate.self, forKey: .estimate) {
            estimate = nestedEstimate
        } else if
            let input = try container.decodeIfPresent(Int.self, forKey: .estimatedInputTokens),
            let output = try container.decodeIfPresent(Int.self, forKey: .estimatedOutputTokens)
        {
            estimate = LLMTokenCostEstimate(
                inputTokens: input,
                outputTokens: output,
                totalTokens: try container.decodeIfPresent(Int.self, forKey: .estimatedTotalTokens) ?? input + output,
                estimatedCostUSD: try container.decodeIfPresent(Double.self, forKey: .estimatedCostUSD)
            )
        } else {
            estimate = nil
        }
        confirmationRequired = try container.decodeIfPresent(Bool.self, forKey: .confirmationRequired)
            ?? container.decodeIfPresent(Bool.self, forKey: .requiresConfirmation)
            ?? true
        rawPromptPersisted = try container.decodeIfPresent(Bool.self, forKey: .rawPromptPersisted)
            ?? container.decodeIfPresent(Bool.self, forKey: .promptStored)
            ?? false
        rawResponsePersisted = try container.decodeIfPresent(Bool.self, forKey: .rawResponsePersisted)
            ?? container.decodeIfPresent(Bool.self, forKey: .responseStored)
            ?? false
        draftCopyOnly = try container.decodeIfPresent(Bool.self, forKey: .draftCopyOnly)
            ?? container.decodeIfPresent(Bool.self, forKey: .copyOnly)
            ?? container.decodeIfPresent(Bool.self, forKey: .outputReadOnly)
            ?? container.decodeIfPresent(Bool.self, forKey: .readOnly)
            ?? true
        promptPreview = try container.decodeIfPresent(String.self, forKey: .promptPreview)
            ?? container.decodeIfPresent(String.self, forKey: .redactedPromptPreview)
            ?? container.decodeIfPresent(String.self, forKey: .redactedPrompt)
            ?? container.decodeIfPresent(String.self, forKey: .sanitizedPrompt)
        audit = try container.decodeIfPresent(AIProviderCallAuditMetadata.self, forKey: .audit)
            ?? container.decodeIfPresent(AIProviderCallAuditMetadata.self, forKey: .auditMetadata)
            ?? container.decodeIfPresent(AIProviderCallAuditMetadata.self, forKey: .metadata)
    }

    static func unavailable(reason: String) -> LLMPromptPreview {
        LLMPromptPreview(
            previewID: "",
            action: nil,
            analysisKind: nil,
            requestKind: nil,
            scope: nil,
            promptScope: UIStrings.unknown,
            enabled: false,
            disabledReason: reason,
            provider: nil,
            model: nil,
            destinationHost: nil,
            includedFields: [],
            excludedFields: [],
            redaction: LLMPromptRedactionSummary(status: "unavailable", summary: reason),
            estimate: nil,
            confirmationRequired: true,
            rawPromptPersisted: false,
            rawResponsePersisted: false,
            draftCopyOnly: true,
            promptPreview: nil,
            audit: nil
        )
    }

    var actionConfirmation: ActionConfirmationWire? {
        guard let actionDescriptor,
              let previewToken,
              !previewToken.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        else { return nil }
        return ActionConfirmationWire(action: actionDescriptor, previewToken: previewToken)
    }

    private static func decodeAction(
        from container: KeyedDecodingContainer<CodingKeys>,
        keys: [CodingKeys]
    ) throws -> LLMAction? {
        for key in keys {
            if let value = try? container.decode(String.self, forKey: key),
               let action = LLMAction(rawValue: value) {
                return action
            }
        }
        return nil
    }

    private static func decodeFlexibleString(
        from container: KeyedDecodingContainer<CodingKeys>,
        keys: [CodingKeys]
    ) throws -> String? {
        for key in keys {
            if let value = try? container.decodeIfPresent(String.self, forKey: key) {
                return value
            }
            if let values = try? container.decodeIfPresent([String].self, forKey: key) {
                return values.joined(separator: ", ")
            }
        }
        return nil
    }

}

struct LLMPromptSendResult: Decodable, Identifiable, Hashable {
    let previewID: String
    let success: Bool
    let status: String
    let message: String
    let outputText: String?
    let draftCopyOnly: Bool
    let rawPromptPersisted: Bool
    let rawResponsePersisted: Bool
    let writeBackAllowed: Bool
    let scriptExecutionAllowed: Bool
    let audit: AIProviderCallAuditMetadata?
    let readback: ActionReadbackRecordWire?
    let partialOutcome: LLMPromptPartialOutcome?

    var id: String { previewID.isEmpty ? status : previewID }

    enum CodingKeys: String, CodingKey {
        case previewID = "preview_id"
        case confirmationID = "confirmation_id"
        case id
        case success
        case ok
        case status
        case message
        case reason
        case outputText = "output_text"
        case responseText = "response_text"
        case draftOutput = "draft_output"
        case draftText = "draft_text"
        case resultText = "result_text"
        case summaryDraft = "summary_draft"
        case rawPromptPersisted = "raw_prompt_persisted"
        case promptStored = "prompt_stored"
        case rawResponsePersisted = "raw_response_persisted"
        case responseStored = "response_stored"
        case draftCopyOnly = "draft_copy_only"
        case copyOnly = "copy_only"
        case readOnly = "read_only"
        case writeBackAllowed = "write_back_allowed"
        case writeActionsAvailable = "write_actions_available"
        case scriptExecutionAllowed = "script_execution_allowed"
        case executionActionsAvailable = "execution_actions_available"
        case audit
        case auditMetadata = "audit_metadata"
        case metadata
        case readback
        case partialOutcome = "partial_outcome"
    }

    init(
        previewID: String,
        success: Bool,
        status: String,
        message: String,
        outputText: String?,
        draftCopyOnly: Bool,
        rawPromptPersisted: Bool,
        rawResponsePersisted: Bool,
        writeBackAllowed: Bool,
        scriptExecutionAllowed: Bool,
        audit: AIProviderCallAuditMetadata?,
        readback: ActionReadbackRecordWire? = nil,
        partialOutcome: LLMPromptPartialOutcome? = nil
    ) {
        self.previewID = previewID
        self.success = success
        self.status = status
        self.message = message
        self.outputText = outputText
        self.draftCopyOnly = draftCopyOnly
        self.rawPromptPersisted = rawPromptPersisted
        self.rawResponsePersisted = rawResponsePersisted
        self.writeBackAllowed = writeBackAllowed
        self.scriptExecutionAllowed = scriptExecutionAllowed
        self.audit = audit
        self.readback = readback
        self.partialOutcome = partialOutcome
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        previewID = try container.decodeIfPresent(String.self, forKey: .previewID)
            ?? container.decodeIfPresent(String.self, forKey: .confirmationID)
            ?? container.decodeIfPresent(String.self, forKey: .id)
            ?? ""
        let decodedStatus = try container.decodeIfPresent(String.self, forKey: .status) ?? "unknown"
        status = decodedStatus
        success = try container.decodeIfPresent(Bool.self, forKey: .success)
            ?? container.decodeIfPresent(Bool.self, forKey: .ok)
            ?? ["ok", "success", "succeeded", "completed"].contains(decodedStatus.lowercased())
        let decodedAudit = try container.decodeIfPresent(AIProviderCallAuditMetadata.self, forKey: .audit)
            ?? container.decodeIfPresent(AIProviderCallAuditMetadata.self, forKey: .auditMetadata)
            ?? container.decodeIfPresent(AIProviderCallAuditMetadata.self, forKey: .metadata)
        message = try container.decodeIfPresent(String.self, forKey: .message)
            ?? container.decodeIfPresent(String.self, forKey: .reason)
            ?? LLMPromptSendResult.messageFromAudit(decodedAudit, success: success)
            ?? (success ? UIStrings.llmPromptSendSucceeded : UIStrings.llmPromptSendFailed)
        let decodedOutputText = try container.decodeIfPresent(String.self, forKey: .outputText)
        let decodedResponseText = try container.decodeIfPresent(String.self, forKey: .responseText)
        let decodedDraftOutput = try container.decodeIfPresent(String.self, forKey: .draftOutput)
        let decodedDraftText = try container.decodeIfPresent(String.self, forKey: .draftText)
        let decodedResultText = try container.decodeIfPresent(String.self, forKey: .resultText)
        let decodedSummaryDraft = try container.decodeIfPresent(String.self, forKey: .summaryDraft)
        outputText = Self.firstNonEmpty(
            decodedOutputText,
            decodedResponseText,
            decodedDraftOutput,
            decodedDraftText,
            decodedResultText,
            decodedSummaryDraft
        )
        draftCopyOnly = try container.decodeIfPresent(Bool.self, forKey: .draftCopyOnly)
            ?? container.decodeIfPresent(Bool.self, forKey: .copyOnly)
            ?? container.decodeIfPresent(Bool.self, forKey: .readOnly)
            ?? true
        rawPromptPersisted = try container.decodeIfPresent(Bool.self, forKey: .rawPromptPersisted)
            ?? container.decodeIfPresent(Bool.self, forKey: .promptStored)
            ?? false
        rawResponsePersisted = try container.decodeIfPresent(Bool.self, forKey: .rawResponsePersisted)
            ?? container.decodeIfPresent(Bool.self, forKey: .responseStored)
            ?? false
        writeBackAllowed = try container.decodeIfPresent(Bool.self, forKey: .writeBackAllowed)
            ?? container.decodeIfPresent(Bool.self, forKey: .writeActionsAvailable)
            ?? false
        scriptExecutionAllowed = try container.decodeIfPresent(Bool.self, forKey: .scriptExecutionAllowed)
            ?? container.decodeIfPresent(Bool.self, forKey: .executionActionsAvailable)
            ?? false
        audit = decodedAudit
        readback = try container.decodeIfPresent(ActionReadbackRecordWire.self, forKey: .readback)
        partialOutcome = try container.decodeIfPresent(LLMPromptPartialOutcome.self, forKey: .partialOutcome)
    }

    private static func firstNonEmpty(_ values: String?...) -> String? {
        for value in values {
            if let value, !value.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                return value
            }
        }
        return nil
    }

    static func unavailable(previewID: String = "", reason: String) -> LLMPromptSendResult {
        LLMPromptSendResult(
            previewID: previewID,
            success: false,
            status: "unavailable",
            message: reason,
            outputText: nil,
            draftCopyOnly: true,
            rawPromptPersisted: false,
            rawResponsePersisted: false,
            writeBackAllowed: false,
            scriptExecutionAllowed: false,
            audit: nil,
            readback: nil,
            partialOutcome: nil
        )
    }

    private static func messageFromAudit(_ audit: AIProviderCallAuditMetadata?, success: Bool) -> String? {
        guard !success, let audit else { return nil }
        if let errorCode = audit.errorCode, let errorMessage = audit.errorMessage, !errorMessage.isEmpty {
            return "\(errorCode): \(errorMessage)"
        }
        if let errorMessage = audit.errorMessage, !errorMessage.isEmpty {
            return errorMessage
        }
        if let errorCode = audit.errorCode, !errorCode.isEmpty {
            return errorCode
        }
        return nil
    }
}

struct LLMPromptPartialOutcome: Decodable, Hashable {
    let remoteEffect: String
    let localRecord: String
    let recovery: String

    enum CodingKeys: String, CodingKey {
        case remoteEffect = "remote_effect"
        case localRecord = "local_record"
        case recovery
    }
}

struct LLMPromptRunListResult: Decodable, Hashable {
    let generatedBy: String
    let count: Int
    let totalCount: Int
    let returnedCount: Int
    let limit: Int?
    let truncated: Bool
    let runs: [LLMPromptRunRecord]
    let appLocalOnly: Bool
    let providerRequestSent: Bool
    let rawPromptPersisted: Bool
    let rawResponsePersisted: Bool
    let rawSecretReturned: Bool

    enum CodingKeys: String, CodingKey {
        case generatedBy = "generated_by"
        case count
        case totalCount = "total_count"
        case totalCountAlt = "totalCount"
        case returnedCount = "returned_count"
        case returnedCountAlt = "returnedCount"
        case limit
        case truncated
        case runs
        case appLocalOnly = "app_local_only"
        case providerRequestSent = "provider_request_sent"
        case rawPromptPersisted = "raw_prompt_persisted"
        case rawResponsePersisted = "raw_response_persisted"
        case rawSecretReturned = "raw_secret_returned"
    }

    init(
        generatedBy: String,
        count: Int,
        totalCount: Int? = nil,
        returnedCount: Int? = nil,
        limit: Int? = nil,
        truncated: Bool? = nil,
        runs: [LLMPromptRunRecord],
        appLocalOnly: Bool,
        providerRequestSent: Bool,
        rawPromptPersisted: Bool,
        rawResponsePersisted: Bool,
        rawSecretReturned: Bool
    ) {
        self.generatedBy = generatedBy
        self.count = count
        self.totalCount = totalCount ?? max(count, runs.count)
        self.returnedCount = returnedCount ?? runs.count
        self.limit = limit
        self.truncated = truncated ?? ((returnedCount ?? runs.count) < (totalCount ?? max(count, runs.count)))
        self.runs = runs
        self.appLocalOnly = appLocalOnly
        self.providerRequestSent = providerRequestSent
        self.rawPromptPersisted = rawPromptPersisted
        self.rawResponsePersisted = rawResponsePersisted
        self.rawSecretReturned = rawSecretReturned
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        runs = try container.decodeIfPresent([LLMPromptRunRecord].self, forKey: .runs) ?? []
        generatedBy = try container.decodeIfPresent(String.self, forKey: .generatedBy) ?? "local-v2.61"
        count = try container.decodeIfPresent(Int.self, forKey: .count) ?? runs.count
        totalCount = try container.decodeIfPresent(Int.self, forKey: .totalCount)
            ?? container.decodeIfPresent(Int.self, forKey: .totalCountAlt)
            ?? max(count, runs.count)
        returnedCount = try container.decodeIfPresent(Int.self, forKey: .returnedCount)
            ?? container.decodeIfPresent(Int.self, forKey: .returnedCountAlt)
            ?? runs.count
        limit = try container.decodeIfPresent(Int.self, forKey: .limit)
        truncated = try container.decodeIfPresent(Bool.self, forKey: .truncated)
            ?? (returnedCount < totalCount)
        appLocalOnly = try container.decodeIfPresent(Bool.self, forKey: .appLocalOnly) ?? true
        providerRequestSent = try container.decodeIfPresent(Bool.self, forKey: .providerRequestSent) ?? false
        rawPromptPersisted = try container.decodeIfPresent(Bool.self, forKey: .rawPromptPersisted) ?? false
        rawResponsePersisted = try container.decodeIfPresent(Bool.self, forKey: .rawResponsePersisted) ?? false
        rawSecretReturned = try container.decodeIfPresent(Bool.self, forKey: .rawSecretReturned) ?? false
    }

    static func unavailable() -> LLMPromptRunListResult {
        LLMPromptRunListResult(
            generatedBy: "unavailable",
            count: 0,
            totalCount: 0,
            returnedCount: 0,
            limit: nil,
            truncated: false,
            runs: [],
            appLocalOnly: true,
            providerRequestSent: false,
            rawPromptPersisted: false,
            rawResponsePersisted: false,
            rawSecretReturned: false
        )
    }
}

struct LLMPromptRunRecord: Decodable, Identifiable, Hashable {
    let id: String
    let previewID: String
    let confirmationID: String
    let action: String
    let requestKind: String
    let analysisKind: String?
    let scope: String?
    let instanceID: String?
    let instanceIDs: [String]
    let task: String?
    let profileID: String
    let provider: String
    let model: String
    let destinationHost: String
    let status: String
    let errorCode: String?
    let errorMessage: String?
    let durationMS: Int
    let draftOutput: String?
    let draftRequiresUserCopy: Bool
    let providerRequestSent: Bool
    let credentialAccessed: Bool
    let rawPromptPersisted: Bool
    let rawResponsePersisted: Bool
    let rawSecretReturned: Bool
    let completedAt: Int?

    enum CodingKeys: String, CodingKey {
        case id
        case previewID = "preview_id"
        case confirmationID = "confirmation_id"
        case action
        case requestKind = "request_kind"
        case analysisKind = "analysis_kind"
        case scope
        case instanceID = "instance_id"
        case instanceIDs = "instance_ids"
        case task
        case profileID = "profile_id"
        case provider
        case model
        case destinationHost = "destination_host"
        case status
        case errorCode = "error_code"
        case errorMessage = "error_message"
        case durationMS = "duration_ms"
        case draftOutput = "draft_output"
        case draftRequiresUserCopy = "draft_requires_user_copy"
        case providerRequestSent = "provider_request_sent"
        case credentialAccessed = "credential_accessed"
        case rawPromptPersisted = "raw_prompt_persisted"
        case rawResponsePersisted = "raw_response_persisted"
        case rawSecretReturned = "raw_secret_returned"
        case completedAt = "completed_at"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decodeIfPresent(String.self, forKey: .id) ?? ""
        previewID = try container.decodeIfPresent(String.self, forKey: .previewID) ?? id
        confirmationID = try container.decodeIfPresent(String.self, forKey: .confirmationID) ?? ""
        action = try container.decodeIfPresent(String.self, forKey: .action) ?? "unknown"
        requestKind = try container.decodeIfPresent(String.self, forKey: .requestKind) ?? action
        analysisKind = try container.decodeIfPresent(String.self, forKey: .analysisKind)
        scope = try container.decodeIfPresent(String.self, forKey: .scope)
        instanceID = try container.decodeIfPresent(String.self, forKey: .instanceID)
        instanceIDs = try container.decodeIfPresent([String].self, forKey: .instanceIDs) ?? []
        // Compatibility keys remain decodable, but prompt-run history is
        // metadata-only and must never hydrate user intent into UI state.
        task = nil
        profileID = try container.decodeIfPresent(String.self, forKey: .profileID) ?? ""
        provider = try container.decodeIfPresent(String.self, forKey: .provider) ?? UIStrings.unknown
        model = try container.decodeIfPresent(String.self, forKey: .model) ?? UIStrings.unknown
        destinationHost = try container.decodeIfPresent(String.self, forKey: .destinationHost) ?? UIStrings.unknown
        status = try container.decodeIfPresent(String.self, forKey: .status) ?? "unknown"
        errorCode = try container.decodeIfPresent(String.self, forKey: .errorCode)
        errorMessage = try container.decodeIfPresent(String.self, forKey: .errorMessage)
        durationMS = try container.decodeIfPresent(Int.self, forKey: .durationMS) ?? 0
        // Provider output is transient on the immediate confirmed response.
        // Historical payloads cannot rehydrate it from prompt-run metadata.
        draftOutput = nil
        draftRequiresUserCopy = false
        providerRequestSent = try container.decodeIfPresent(Bool.self, forKey: .providerRequestSent) ?? false
        credentialAccessed = try container.decodeIfPresent(Bool.self, forKey: .credentialAccessed) ?? false
        rawPromptPersisted = try container.decodeIfPresent(Bool.self, forKey: .rawPromptPersisted) ?? false
        rawResponsePersisted = try container.decodeIfPresent(Bool.self, forKey: .rawResponsePersisted) ?? false
        rawSecretReturned = try container.decodeIfPresent(Bool.self, forKey: .rawSecretReturned) ?? false
        completedAt = try container.decodeIfPresent(Int.self, forKey: .completedAt)
    }

    var sendResult: LLMPromptSendResult {
        let success = ["ok", "success", "succeeded", "completed"].contains(status.lowercased())
        let message: String
        if success {
            message = UIStrings.llmPromptSendSucceeded
        } else if let errorCode, let errorMessage, !errorMessage.isEmpty {
            message = "\(errorCode): \(errorMessage)"
        } else if let errorMessage, !errorMessage.isEmpty {
            message = errorMessage
        } else {
            message = UIStrings.llmPromptSendFailed
        }
        return LLMPromptSendResult(
            previewID: previewID,
            success: success,
            status: status,
            message: message,
            outputText: draftOutput,
            draftCopyOnly: draftRequiresUserCopy,
            rawPromptPersisted: rawPromptPersisted,
            rawResponsePersisted: rawResponsePersisted,
            writeBackAllowed: false,
            scriptExecutionAllowed: false,
            audit: nil
        )
    }
}
