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

enum AIResponseResultSchema: String, Codable, Hashable {
    case copyOnlyMarkdown = "copy_only_markdown"
    case taskReadiness = "task_readiness"
    case sessionDigest = "session_digest"
    case skillChangeReview = "skill_change_review"
    case semanticRerank = "semantic_rerank"
}

struct AIResponseSafetyFlagsWire: Codable, Hashable {
    let copyOnly: Bool
    let writeBackAllowed: Bool
    let commandExecutionAllowed: Bool
    let scriptExecutionAllowed: Bool
    let mutationAllowed: Bool
    let hiddenTaskStateCreated: Bool
    let rawPromptPersisted: Bool
    let rawResponsePersisted: Bool
    let rawTracePersisted: Bool

    enum CodingKeys: String, CodingKey {
        case copyOnly = "copy_only"
        case writeBackAllowed = "write_back_allowed"
        case commandExecutionAllowed = "command_execution_allowed"
        case scriptExecutionAllowed = "script_execution_allowed"
        case mutationAllowed = "mutation_allowed"
        case hiddenTaskStateCreated = "hidden_task_state_created"
        case rawPromptPersisted = "raw_prompt_persisted"
        case rawResponsePersisted = "raw_response_persisted"
        case rawTracePersisted = "raw_trace_persisted"
    }

    var isRequiredCopyOnly: Bool {
        copyOnly
            && !writeBackAllowed
            && !commandExecutionAllowed
            && !scriptExecutionAllowed
            && !mutationAllowed
            && !hiddenTaskStateCreated
            && !rawPromptPersisted
            && !rawResponsePersisted
            && !rawTracePersisted
    }
}

struct AIResponseContractWire: Decodable, Hashable {
    let schemaVersion: Int
    let requestKind: String
    let projectID: String
    let sourceRevision: String
    let resultSchema: AIResponseResultSchema
    let evidence: [EvidenceRef]
    let actions: [ActionDescriptorWire]
    let requiredSafetyFlags: AIResponseSafetyFlagsWire

    enum CodingKeys: String, CodingKey {
        case schemaVersion = "schema_version"
        case requestKind = "request_kind"
        case projectID = "project_id"
        case sourceRevision = "source_revision"
        case resultSchema = "result_schema"
        case evidence
        case actions
        case requiredSafetyFlags = "required_safety_flags"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        schemaVersion = try container.decode(Int.self, forKey: .schemaVersion)
        requestKind = try container.decode(String.self, forKey: .requestKind)
        projectID = try container.decode(String.self, forKey: .projectID)
        sourceRevision = try container.decode(String.self, forKey: .sourceRevision)
        resultSchema = try container.decode(AIResponseResultSchema.self, forKey: .resultSchema)
        evidence = try container.decode([EvidenceRef].self, forKey: .evidence)
        actions = try container.decodeIfPresent([ActionDescriptorWire].self, forKey: .actions) ?? []
        requiredSafetyFlags = try container.decode(
            AIResponseSafetyFlagsWire.self,
            forKey: .requiredSafetyFlags
        )
    }

    @discardableResult
    func validated(requestKind expectedRequestKind: String, projectID expectedProjectID: String?) throws
        -> AIResponseContractWire
    {
        guard schemaVersion == 1,
              requestKind == expectedRequestKind,
              !projectID.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              projectID == expectedProjectID,
              !sourceRevision.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              !evidence.isEmpty,
              requiredSafetyFlags.isRequiredCopyOnly else {
            throw ProductProjectionValidationError.invalid(
                "AI response contract is not bound to the current copy-only project evidence"
            )
        }
        let evidenceIDs = Set(evidence.map(\.id))
        guard evidenceIDs.count == evidence.count else {
            throw ProductProjectionValidationError.invalid(
                "AI response contract contains duplicate evidence"
            )
        }
        for reference in evidence {
            try reference.validated()
        }
        guard Set(actions.map(\.id)).count == actions.count else {
            throw ProductProjectionValidationError.invalid(
                "AI response contract contains duplicate actions"
            )
        }
        for action in actions {
            guard action.projectID == projectID,
                  Set(action.evidenceRefs).isSubset(of: evidenceIDs) else {
                throw ProductProjectionValidationError.invalid(
                    "AI response action is not bound to the contract project and evidence"
                )
            }
        }
        return self
    }
}

struct AIResponseEnvelopeWire: Decodable, Hashable {
    let schemaVersion: Int
    let requestKind: String
    let projectID: String
    let sourceRevision: String
    let resultSchema: AIResponseResultSchema
    let evidenceRefs: [String]
    let actionRefs: [String]
    let result: JSONValue
    let safetyFlags: AIResponseSafetyFlagsWire

    enum CodingKeys: String, CodingKey {
        case schemaVersion = "schema_version"
        case requestKind = "request_kind"
        case projectID = "project_id"
        case sourceRevision = "source_revision"
        case resultSchema = "result_schema"
        case evidenceRefs = "evidence_refs"
        case actionRefs = "action_refs"
        case result
        case safetyFlags = "safety_flags"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        schemaVersion = try container.decode(Int.self, forKey: .schemaVersion)
        requestKind = try container.decode(String.self, forKey: .requestKind)
        projectID = try container.decode(String.self, forKey: .projectID)
        sourceRevision = try container.decode(String.self, forKey: .sourceRevision)
        resultSchema = try container.decode(AIResponseResultSchema.self, forKey: .resultSchema)
        evidenceRefs = try container.decode([String].self, forKey: .evidenceRefs)
        actionRefs = try container.decodeIfPresent([String].self, forKey: .actionRefs) ?? []
        result = try container.decode(JSONValue.self, forKey: .result)
        safetyFlags = try container.decode(AIResponseSafetyFlagsWire.self, forKey: .safetyFlags)
    }

    var visibleCopyOnlyText: String? {
        guard case .object(let object) = result else { return nil }
        if let value = object["markdown"],
           case .string(let markdown) = value,
           !markdown.isEmpty {
            return markdown
        }
        if let value = object["summary"],
           case .string(let summary) = value,
           !summary.isEmpty {
            return summary
        }
        if let value = object["summary"],
           case .object(let summary) = value,
           let summaryValue = summary["summary"],
           case .string(let text) = summaryValue,
           !text.isEmpty {
            return text
        }
        return nil
    }

    @discardableResult
    func validated(against contract: AIResponseContractWire) throws -> AIResponseEnvelopeWire {
        guard schemaVersion == contract.schemaVersion,
              requestKind == contract.requestKind,
              projectID == contract.projectID,
              sourceRevision == contract.sourceRevision,
              resultSchema == contract.resultSchema,
              safetyFlags == contract.requiredSafetyFlags,
              safetyFlags.isRequiredCopyOnly,
              !evidenceRefs.isEmpty,
              Set(evidenceRefs).count == evidenceRefs.count,
              Set(actionRefs).count == actionRefs.count,
              Set(evidenceRefs).isSubset(of: Set(contract.evidence.map(\.id))),
              Set(actionRefs).isSubset(of: Set(contract.actions.map(\.id))),
              result.hasValidAIResponseShape(for: resultSchema),
              result.hasValidSemanticRerankReferences(
                  for: resultSchema,
                  evidenceRefs: evidenceRefs
              ) else {
            throw ProductProjectionValidationError.invalid(
                "AI response envelope drifted from its confirmed evidence contract"
            )
        }
        return self
    }
}

private extension JSONValue {
    func hasValidAIResponseShape(for schema: AIResponseResultSchema) -> Bool {
        guard case .object(let object) = self, !containsForbiddenAIResponseField else {
            return false
        }
        switch schema {
        case .copyOnlyMarkdown:
            return object["markdown"]?.isNonemptyString == true
        case .taskReadiness:
            return object["summary"]?.isValidTaskReadinessSummary == true
                && object["agent_candidates"]?.isArray == true
                && object["skill_candidates"]?.isArray == true
                && object["readiness_signals"]?.isArray == true
                && object["gap_rows"]?.isArray == true
                && object["blocker_rows"]?.isArray == true
        case .sessionDigest:
            return object["summary"]?.isNonemptyString == true
                && object["suggested_next_prompt"]?.isNonemptyString == true
                && object["evidence_notes"]?.isArray == true
                && object["uncertainties"]?.isArray == true
        case .skillChangeReview:
            return object["summary"]?.isNonemptyString == true
                && object["changes"]?.isArray == true
                && object["risks"]?.isArray == true
                && object["recommendations"]?.isArray == true
        case .semanticRerank:
            return object["summary"]?.isNonemptyString == true
                && object["ranked_evidence_ids"]?.isArray == true
                && object["rationales"]?.isArray == true
                && object["unsupported_claims"]?.isArray == true
        }
    }

    func hasValidSemanticRerankReferences(
        for schema: AIResponseResultSchema,
        evidenceRefs: [String]
    ) -> Bool {
        guard schema == .semanticRerank else { return true }
        guard case .object(let object) = self,
              case .array(let rankedValues) = object["ranked_evidence_ids"],
              !rankedValues.isEmpty,
              case .array(let rationaleValues) = object["rationales"],
              case .array(let unsupportedValues) = object["unsupported_claims"] else {
            return false
        }
        let allowed = Set(evidenceRefs)
        let ranked = rankedValues.compactMap(\.stringValue)
        guard ranked.count == rankedValues.count,
              Set(ranked).count == ranked.count,
              Set(ranked).isSubset(of: allowed),
              unsupportedValues.allSatisfy({ $0.nonemptyStringValue != nil }) else {
            return false
        }
        let rationaleIDs = rationaleValues.compactMap { value -> String? in
            guard case .object(let row) = value,
                  let evidenceID = row["evidence_id"]?.nonemptyStringValue,
                  ranked.contains(evidenceID),
                  row["rationale"]?.nonemptyStringValue != nil else {
                return nil
            }
            return evidenceID
        }
        return rationaleIDs.count == rationaleValues.count
            && Set(rationaleIDs).count == rationaleIDs.count
            && Set(rationaleIDs) == Set(ranked)
    }

    var containsForbiddenAIResponseField: Bool {
        let forbidden = Set([
            "action_confirmation", "apply_method", "argv", "command", "commands",
            "execute", "execution", "mutation", "preview_token", "script", "scripts",
            "tool_call", "tool_calls", "write_back",
        ])
        switch self {
        case .object(let object):
            return object.contains {
                forbidden.contains($0.key.trimmingCharacters(in: .whitespacesAndNewlines).lowercased())
                    || $0.value.containsForbiddenAIResponseField
            }
        case .array(let values):
            return values.contains(where: \.containsForbiddenAIResponseField)
        case .string, .number, .bool, .null:
            return false
        }
    }

    var isNonemptyString: Bool {
        if case .string(let value) = self {
            return !value.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        }
        return false
    }

    var stringValue: String? {
        if case .string(let value) = self { return value }
        return nil
    }

    var nonemptyStringValue: String? {
        stringValue?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
            ? stringValue
            : nil
    }

    var isObject: Bool {
        if case .object = self { return true }
        return false
    }

    var isArray: Bool {
        if case .array = self { return true }
        return false
    }

    var isValidTaskReadinessSummary: Bool {
        guard case .object(let object) = self else { return false }
        return object["summary"]?.isNonemptyString == true
            && object["recommended_agent"]?.isStringOrNull == true
            && object["recommended_skill_name"]?.isStringOrNull == true
            && object["readiness_score"]?.isScore == true
            && object["routing_score"]?.isScore == true
            && object["gap_count"]?.isNonnegativeInteger == true
            && object["blocker_count"]?.isNonnegativeInteger == true
    }

    var isStringOrNull: Bool {
        switch self {
        case .string, .null:
            return true
        case .number, .bool, .object, .array:
            return false
        }
    }

    var isScore: Bool {
        guard case .number(let value) = self else { return false }
        return value >= 0 && value <= 100 && value.rounded() == value
    }

    var isNonnegativeInteger: Bool {
        guard case .number(let value) = self else { return false }
        return value >= 0 && value.rounded() == value
    }
}

struct ContextualIntelligenceSection: Identifiable, Hashable {
    let title: String
    let rows: [String]

    var id: String { "\(title):\(rows.joined(separator: "\u{1f}"))" }
}

struct ContextualIntelligenceOutput: Hashable {
    let summary: String
    let sections: [ContextualIntelligenceSection]
    let suggestedNextPrompt: String?
    let rankedEvidenceIDs: [String]
    let unsupportedClaims: [String]

    static func parse(_ envelope: AIResponseEnvelopeWire) -> ContextualIntelligenceOutput? {
        guard case .object(let object) = envelope.result else { return nil }
        let summary: String?
        if envelope.resultSchema == .copyOnlyMarkdown {
            summary = object["markdown"]?.nonemptyStringValue
        } else if case .object(let nested) = object["summary"] {
            summary = nested["summary"]?.nonemptyStringValue
        } else {
            summary = object["summary"]?.nonemptyStringValue
        }
        guard let summary else { return nil }

        var sections: [ContextualIntelligenceSection] = []
        let sectionFields: [(String, String)] = [
            ("changes", UIStrings.text("intelligence.output.changes", "Observed changes")),
            ("risks", UIStrings.text("intelligence.output.risks", "Risks")),
            ("recommendations", UIStrings.text("intelligence.output.recommendations", "Recommendations")),
            ("evidence_notes", UIStrings.text("intelligence.output.evidenceNotes", "Evidence notes")),
            ("uncertainties", UIStrings.text("intelligence.output.uncertainties", "Uncertainty")),
            ("rationales", UIStrings.text("intelligence.output.rationales", "Why this order")),
        ]
        for (field, title) in sectionFields {
            let rows = object[field]?.displayLines ?? []
            if !rows.isEmpty {
                sections.append(ContextualIntelligenceSection(title: title, rows: rows))
            }
        }
        let ranked = object["ranked_evidence_ids"]?.stringArray ?? []
        let unsupported = object["unsupported_claims"]?.displayLines ?? []
        return ContextualIntelligenceOutput(
            summary: summary,
            sections: sections,
            suggestedNextPrompt: object["suggested_next_prompt"]?.nonemptyStringValue,
            rankedEvidenceIDs: ranked,
            unsupportedClaims: unsupported
        )
    }
}

extension JSONValue {
    fileprivate var stringArray: [String] {
        guard case .array(let values) = self else { return [] }
        return values.compactMap(\.nonemptyStringValue)
    }

    fileprivate var displayLines: [String] {
        switch self {
        case .array(let values):
            return values.compactMap(\.displayLine)
        default:
            return []
        }
    }

    fileprivate var displayLine: String? {
        switch self {
        case .string(let value):
            return value.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                ? nil
                : value
        case .object(let object):
            for key in ["rationale", "summary", "detail", "message", "text", "title"] {
                if let value = object[key]?.nonemptyStringValue {
                    return value
                }
            }
            return nil
        case .number, .bool, .array, .null:
            return nil
        }
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
    let responseContract: AIResponseContractWire?
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
        case responseContract = "response_contract"
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
        responseContract: AIResponseContractWire? = nil,
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
        self.responseContract = responseContract
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
        responseContract = try container.decodeIfPresent(
            AIResponseContractWire.self,
            forKey: .responseContract
        )
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
            responseContract: nil,
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
    let responseEnvelope: AIResponseEnvelopeWire?
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
        case responseEnvelope = "response_envelope"
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
        responseEnvelope: AIResponseEnvelopeWire? = nil,
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
        self.responseEnvelope = responseEnvelope
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
        responseEnvelope = try container.decodeIfPresent(
            AIResponseEnvelopeWire.self,
            forKey: .responseEnvelope
        )
        outputText = Self.firstNonEmpty(
            decodedOutputText,
            responseEnvelope?.visibleCopyOnlyText,
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
            responseEnvelope: nil,
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
