import Foundation

struct ServiceErrorDetailsPayload: Codable, Hashable {
    let operation: String
    let state: String
    let cleanupRequired: Bool
    let retryAllowed: Bool

    enum CodingKeys: String, CodingKey {
        case operation
        case state
        case cleanupRequired = "cleanup_required"
        case retryAllowed = "retry_allowed"
    }
}

struct ServiceErrorPayload: Codable, Error {
    let code: String
    let message: String
    let details: ServiceErrorDetailsPayload?

    init(
        code: String,
        message: String,
        details: ServiceErrorDetailsPayload? = nil
    ) {
        self.code = code
        self.message = message
        self.details = details
    }
}

struct AppVersion: Codable, Hashable {
    let protocolVersion: Int
    let version: String

    enum CodingKeys: String, CodingKey {
        case protocolVersion = "protocol_version"
        case version
    }
}

struct AppStateSnapshot: Codable, Hashable {
    let status: ServiceStatus
    let skills: [SkillRecord]
    let findings: [RuleFindingRecord]
    let conflicts: [ConflictGroupRecord]
    let health: SkillHealthSummary
    let snapshots: [ConfigSnapshotRecord]

    enum CodingKeys: String, CodingKey {
        case status
        case skills
        case findings
        case conflicts
        case health
        case snapshots
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        status = try container.decode(ServiceStatus.self, forKey: .status)
        skills = try container.decode([SkillRecord].self, forKey: .skills)
        findings = try container.decode([RuleFindingRecord].self, forKey: .findings)
        conflicts = try container.decode([ConflictGroupRecord].self, forKey: .conflicts)
        health = try container.decodeIfPresent(SkillHealthSummary.self, forKey: .health) ?? .empty
        snapshots = try container.decodeIfPresent([ConfigSnapshotRecord].self, forKey: .snapshots) ?? []
    }
}

struct EmptyParams: Encodable {}

struct AppSearchParams: Encodable {
    let query: String
    let agent: String?
    let limitPerKind: Int?
    let authorizedRoots: [String]
    let autoDiscover: Bool?
    let projectRoot: String?
    let currentCWD: String?

    enum CodingKeys: String, CodingKey {
        case query
        case agent
        case limitPerKind = "limit_per_kind"
        case authorizedRoots = "authorized_roots"
        case autoDiscover = "auto_discover"
        case projectRoot = "project_root"
        case currentCWD = "current_cwd"
    }
}

struct GetSkillParams: Encodable {
    let instanceId: String

    enum CodingKeys: String, CodingKey {
        case instanceId = "instance_id"
    }
}

struct ToggleSkillParams: Encodable {
    let instanceId: String
    let on: Bool

    enum CodingKeys: String, CodingKey {
        case instanceId = "instance_id"
        case on
    }
}

struct ReadAgentConfigParams: Encodable {
    let agent: String
    let scope: String?
}

struct BatchToggleParams: Encodable {
    let instanceIDs: [String]
    let targetEnabled: Bool
    let confirmation: ActionConfirmationWire?

    enum CodingKeys: String, CodingKey {
        case instanceIDs = "instance_ids"
        case targetEnabled = "target_enabled"
        case confirmation
    }
}

struct ToolInstallPreviewParams: Encodable {
    let instanceId: String
    let targetAgent: String
    let targetScope: String
    let confirmed: Bool
    let actionConfirmation: ActionConfirmationWire?

    enum CodingKeys: String, CodingKey {
        case instanceId = "instance_id"
        case targetAgent = "target_agent"
        case targetScope = "target_scope"
        case confirmed
        case actionConfirmation = "action_confirmation"
    }
}

struct LocalSessionPreviewParams: Encodable {
    let authorizedRoots: [String]
    let autoDiscover: Bool?
    let agent: String?
    let scope: String?
    let search: String?
    let projectRoot: String?
    let currentCWD: String?
    let sessionID: String?
    let includeContentItems: Bool?
    let limit: Int?
    let offset: Int?
    let pagingMode: String?
    let cursor: String?
    let sourceRevision: String?
    let sort: String?
    let direction: String?
    let maxFiles: Int?
    let maxExcerptChars: Int?

    enum CodingKeys: String, CodingKey {
        case authorizedRoots = "authorized_roots"
        case autoDiscover = "auto_discover"
        case agent
        case scope
        case search
        case projectRoot = "project_root"
        case currentCWD = "current_cwd"
        case sessionID = "session_id"
        case includeContentItems = "include_content_items"
        case limit
        case offset
        case pagingMode = "paging_mode"
        case cursor
        case sourceRevision = "source_revision"
        case sort
        case direction
        case maxFiles = "max_files"
        case maxExcerptChars = "max_excerpt_chars"
    }
}

struct LocalSessionMessagePageParams: Encodable {
    let authorizedRoots: [String]
    let autoDiscover: Bool?
    let agent: String?
    let projectRoot: String?
    let currentCWD: String?
    let sessionID: String
    let limit: Int?
    let cursor: String?
    let sourceRevision: String?

    enum CodingKeys: String, CodingKey {
        case authorizedRoots = "authorized_roots"
        case autoDiscover = "auto_discover"
        case agent
        case projectRoot = "project_root"
        case currentCWD = "current_cwd"
        case sessionID = "session_id"
        case limit
        case cursor
        case sourceRevision = "source_revision"
    }
}

struct PrepareLLMActionParams: Encodable {
    let action: LLMAction
    let instanceId: String
    let definitionId: String
    let agent: String

    enum CodingKeys: String, CodingKey {
        case action = "kind"
        case instanceId = "instance_id"
        case definitionId = "definition_id"
        case agent
    }
}

struct PreviewLLMPromptParams: Encodable {
    let action: String
    let requestKind: String
    let scope: String?
    let instanceIDs: [String]?
    let instanceId: String?
    let definitionId: String?
    let agent: String?
    let agents: [String]?
    let taskText: String?
    let userIntent: String?
    let candidateInstanceIDs: [String]?
    let appLanguage: String = UIStrings.currentLanguage.rawValue

    enum CodingKeys: String, CodingKey {
        case action
        case requestKind = "request_kind"
        case scope
        case instanceIDs = "instance_ids"
        case instanceId = "instance_id"
        case definitionId = "definition_id"
        case agent
        case agents
        case taskText = "task_text"
        case userIntent = "user_intent"
        case candidateInstanceIDs = "candidate_instance_ids"
        case appLanguage = "app_language"
    }
}

struct ConfirmLLMPromptParams: Encodable {
    let actionConfirmation: ActionConfirmationWire
    let request: PreviewLLMPromptParams
    let timeoutMS: Int

    enum CodingKeys: String, CodingKey {
        case actionConfirmation = "action_confirmation"
        case request
        case timeoutMS = "timeout_ms"
    }
}

struct ListLLMPromptRunsParams: Encodable {
    let instanceId: String?
    let action: String?
    let requestKind: String?
    let limit: Int?

    enum CodingKeys: String, CodingKey {
        case instanceId = "instance_id"
        case action
        case requestKind = "request_kind"
        case limit
    }
}

struct ProviderObservabilityParams: Encodable {
    let windowDays: Int?
    let startAt: Int?
    let endAt: Int?
    let limit: Int
    let includeHistory: Bool
    let includeBudgetHints: Bool
    let includeRetentionRecommendations: Bool
    let includeEvidence: Bool
    let appLanguage: String = UIStrings.currentLanguage.rawValue

    enum CodingKeys: String, CodingKey {
        case windowDays = "window_days"
        case startAt = "start_at"
        case endAt = "end_at"
        case limit
        case includeHistory = "include_history"
        case includeBudgetHints = "include_budget_hints"
        case includeRetentionRecommendations = "include_retention_recommendations"
        case includeEvidence = "include_evidence"
        case appLanguage = "app_language"
    }
}

struct ListProviderActivityParams: Encodable {
    let provider: String?
    let model: String?
    let action: String?
    let windowDays: Int?
    let startAt: Int?
    let endAt: Int?
    let limit: Int
    let cursor: String?
    let sourceRevision: String?

    enum CodingKeys: String, CodingKey {
        case provider
        case model
        case action
        case windowDays = "window_days"
        case startAt = "start_at"
        case endAt = "end_at"
        case limit
        case cursor
        case sourceRevision = "source_revision"
    }
}

struct ScriptExecutionParams: Encodable {
    let instanceId: String
    let definitionId: String
    let agent: String

    enum CodingKeys: String, CodingKey {
        case instanceId = "instance_id"
        case definitionId = "definition_id"
        case agent
    }
}

struct SnapshotParams: Encodable {
    let snapshotId: String

    enum CodingKeys: String, CodingKey {
        case snapshotId = "snapshot_id"
    }
}

struct ListAgentConfigSnapshotsParams: Encodable {
    let agent: String
    let scope: String?
}

struct ListAgentConfigPageParams: Encodable {
    let agent: String
    let scope: String?
    let limit: Int
    let cursor: String?
    let sourceRevision: String?

    enum CodingKeys: String, CodingKey {
        case agent
        case scope
        case limit
        case cursor
        case sourceRevision = "source_revision"
    }
}

struct ListSkillEventsParams: Encodable {
    let instanceId: String
    let limit: Int?

    enum CodingKeys: String, CodingKey {
        case instanceId = "instance_id"
        case limit
    }
}

struct ListSkillEventsPageParams: Encodable {
    let instanceId: String
    let limit: Int
    let cursor: String?
    let sourceRevision: String?

    enum CodingKeys: String, CodingKey {
        case instanceId = "instance_id"
        case limit
        case cursor
        case sourceRevision = "source_revision"
    }
}

struct SetFindingTriageParams: Encodable {
    let triageKey: String
    let status: String
    let note: String?

    enum CodingKeys: String, CodingKey {
        case triageKey = "triage_key"
        case status
        case note
    }
}

struct ClearFindingTriageParams: Encodable {
    let triageKey: String

    enum CodingKeys: String, CodingKey {
        case triageKey = "triage_key"
    }
}

struct SetRuleSeverityOverrideParams: Encodable {
    let ruleId: String
    let severity: String

    enum CodingKeys: String, CodingKey {
        case ruleId = "rule_id"
        case severity
    }
}

struct ClearRuleSeverityOverrideParams: Encodable {
    let ruleId: String

    enum CodingKeys: String, CodingKey {
        case ruleId = "rule_id"
    }
}

struct SetRuleSuppressionParams: Encodable {
    let ruleId: String
    let reason: String
    let note: String?

    enum CodingKeys: String, CodingKey {
        case ruleId = "rule_id"
        case reason
        case note
    }
}

struct ClearRuleSuppressionParams: Encodable {
    let ruleId: String

    enum CodingKeys: String, CodingKey {
        case ruleId = "rule_id"
    }
}

struct PreviewSaveClaudeSettingsParams: Encodable {
    let content: String
    let expectedRevision: String

    enum CodingKeys: String, CodingKey {
        case content
        case expectedRevision = "expected_revision"
    }
}

struct SaveClaudeSettingsParams: Encodable {
    let content: String
    let confirmation: ActionConfirmationWire
}

struct RollbackSnapshotParams: Encodable {
    let snapshotId: String
    let confirmation: ActionConfirmationWire

    enum CodingKeys: String, CodingKey {
        case snapshotId = "snapshot_id"
        case confirmation
    }
}

struct SaveAIProviderProfileParams: Encodable {
    let id: String
    let displayName: String
    let providerType: String
    let baseURL: String
    let model: String
    let enabled: Bool
    let apiVersion: String?
    let apiKey: String?
    let singleRequestTokenLimit: Int?
    let monthlyBudgetUSD: Double?
    let actionConfirmation: ActionConfirmationWire?

    enum CodingKeys: String, CodingKey {
        case id
        case displayName = "display_name"
        case providerType = "provider_type"
        case baseURL = "base_url"
        case model
        case enabled
        case apiVersion = "api_version"
        case apiKey = "api_key"
        case singleRequestTokenLimit = "single_request_token_limit"
        case monthlyBudgetUSD = "monthly_budget_usd"
        case actionConfirmation = "action_confirmation"
    }
}

struct DeleteAIProviderProfileParams: Encodable {
    let profileID: String
    let deleteCredential: Bool
    let actionConfirmation: ActionConfirmationWire?

    enum CodingKeys: String, CodingKey {
        case profileID = "profile_id"
        case deleteCredential = "delete_credential"
        case actionConfirmation = "action_confirmation"
    }
}

struct TestAIProviderConnectionParams: Encodable {
    let profileID: String
    let timeoutMS: Int
    let actionConfirmation: ActionConfirmationWire?

    enum CodingKeys: String, CodingKey {
        case profileID = "profile_id"
        case timeoutMS = "timeout_ms"
        case actionConfirmation = "action_confirmation"
    }
}

struct ProjectContextParams: Encodable {
    let rootPath: String
    let currentCWD: String?
    let name: String?

    enum CodingKeys: String, CodingKey {
        case rootPath = "root_path"
        case currentCWD = "current_cwd"
        case name
    }
}

struct ProjectContextIDParams: Encodable {
    let id: String
}

final class ServiceClient {
    enum ClientError: LocalizedError {
        case missingBinary
        case invalidOutput(String)
        case actionOutcome(String)
        case service(ServiceErrorPayload)
        case processFailed(Int32, String)
        case processTimedOut
        case responseTooLarge(maxBytes: Int)

        var errorDescription: String? {
            switch self {
            case .missingBinary:
                return "skills-copilot-service was not found in the app bundle."
            case .invalidOutput(let output):
                return "\(UIStrings.text("service.error.invalidOutput", "Invalid service output:")) \(output)"
            case .actionOutcome(let message):
                return message
            case .service(let error):
                return "\(error.code): \(error.message)"
            case .processFailed(let status, let stderr):
                return "Service exited with \(status): \(stderr)"
            case .processTimedOut:
                return UIStrings.text(
                    "service.error.sidecarTimedOut",
                    "Service call timed out before the sidecar returned a complete response."
                )
            case .responseTooLarge(let maxBytes):
                let mebibytes = maxBytes / (1_024 * 1_024)
                return String(
                    format: UIStrings.text(
                        "service.error.responseTooLarge",
                        "The service response exceeded the %d MiB limit."
                    ),
                    mebibytes
                )
            }
        }
    }

    let processRunner: ServiceProcessRunning
    let serviceURLOverride: URL?

    init(
        processRunner: ServiceProcessRunning = StdioServiceProcessRunner(),
        serviceURL: URL? = nil
    ) {
        self.processRunner = processRunner
        serviceURLOverride = serviceURL ?? Self.configuredServiceURLFromEnvironment()
    }

    private static func configuredServiceURLFromEnvironment() -> URL? {
        #if DEBUG
        if let override = ProcessInfo.processInfo.environment["SKILLS_COPILOT_SERVICE_PATH"],
           !override.isEmpty {
            let overrideURL = URL(fileURLWithPath: override)
            if FileManager.default.isExecutableFile(atPath: overrideURL.path) {
                return overrideURL
            }
        }
        #endif
        return nil
    }

    func status() async throws -> ServiceStatus {
        try await call(method: "service.status", params: EmptyParams())
    }

    func listAdapterCapabilities() async throws -> [AdapterCapabilityRecord] {
        try await call(method: "adapter.listCapabilities", params: EmptyParams())
    }

    func appVersion() async throws -> AppVersion {
        try await call(method: "app.version", params: EmptyParams())
    }

    func appStateSnapshot() async throws -> AppStateSnapshot {
        try await call(method: "app.stateSnapshot", params: EmptyParams())
    }

    func searchApp(
        query: String,
        agent: String? = nil,
        limitPerKind: Int? = nil,
        authorizedRoots: [String] = [],
        autoDiscover: Bool? = nil,
        project: ProjectContext? = nil
    ) async throws -> AppSearchResult {
        let normalizedQuery = query.trimmingCharacters(in: .whitespacesAndNewlines)
        let params = AppSearchParams(
            query: normalizedQuery,
            agent: agent,
            limitPerKind: limitPerKind,
            authorizedRoots: authorizedRoots,
            autoDiscover: autoDiscover,
            projectRoot: project?.rootPath,
            currentCWD: project?.currentCWD
        )
        do {
            return try await call(method: "app.search", params: params)
        } catch ClientError.service(let error) where error.code == "unknown_method" {
            return .unavailable(query: normalizedQuery, reason: UIStrings.text("globalSearch.unavailable", "Global search is unavailable in this service build."))
        }
    }

    func listSkills() async throws -> [SkillRecord] {
        try await call(method: "catalog.listSkills", params: EmptyParams())
    }


}
