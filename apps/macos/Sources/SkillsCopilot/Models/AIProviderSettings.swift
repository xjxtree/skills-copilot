import Foundation

enum AIProviderKind: String, Codable, CaseIterable, Identifiable, Hashable {
    case openAICompatible = "openai-compatible"
    case claudeCompatible = "claude-compatible"
    case unknown

    var id: String { rawValue }

    init(from decoder: Decoder) throws {
        let value = try decoder.singleValueContainer().decode(String.self)
        self = AIProviderKind.fromService(value)
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        try container.encode(rawValue)
    }

    static func fromService(_ value: String?) -> AIProviderKind {
        switch value?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "openai-compatible", "openai_compatible", "openai", "open-ai-compatible":
            return .openAICompatible
        case "claude-compatible", "claude_compatible", "anthropic", "claude":
            return .claudeCompatible
        default:
            return .unknown
        }
    }

    var title: String {
        switch self {
        case .openAICompatible:
            return UIStrings.aiProviderOpenAICompatible
        case .claudeCompatible:
            return UIStrings.aiProviderClaudeCompatible
        case .unknown:
            return UIStrings.unknown
        }
    }
}

struct AIProviderBudget: Codable, Hashable {
    let singleRequestTokenLimit: Int?
    let monthlyBudgetUSD: Double?
    let perRequestBudgetUSD: Double?
    let spentThisMonthUSD: Double?

    enum CodingKeys: String, CodingKey {
        case singleRequestTokenLimit = "single_request_token_limit"
        case monthlyBudgetUSD = "monthly_budget_usd"
        case perRequestBudgetUSD = "per_request_budget_usd"
        case spentThisMonthUSD = "spent_this_month_usd"
    }

    init(
        singleRequestTokenLimit: Int? = nil,
        monthlyBudgetUSD: Double? = nil,
        perRequestBudgetUSD: Double? = nil,
        spentThisMonthUSD: Double? = nil
    ) {
        self.singleRequestTokenLimit = singleRequestTokenLimit
        self.monthlyBudgetUSD = monthlyBudgetUSD
        self.perRequestBudgetUSD = perRequestBudgetUSD
        self.spentThisMonthUSD = spentThisMonthUSD
    }
}

private struct AIProviderCredentialStatus: Decodable {
    let state: String?
    let reason: String?
    let secretAvailable: Bool

    enum CodingKeys: String, CodingKey {
        case state
        case reason
        case secretAvailable = "secret_available"
    }
}

private struct AIProviderCredentialReference: Decodable {
    let storage: String?

    enum CodingKeys: String, CodingKey {
        case storage
    }
}

struct AIProviderProfile: Decodable, Identifiable, Hashable {
    let id: String
    let name: String
    let kind: AIProviderKind
    let endpoint: String?
    let model: String?
    let apiVersion: String?
    let enabled: Bool
    let configured: Bool
    let hasAPIKey: Bool
    let credentialStorage: String?
    let disabledReason: String?
    let budget: AIProviderBudget?

    enum CodingKeys: String, CodingKey {
        case id
        case name
        case displayName = "display_name"
        case kind
        case providerKind = "provider_kind"
        case providerType = "provider_type"
        case provider
        case endpoint
        case baseURL = "base_url"
        case model
        case apiVersion = "api_version"
        case enabled
        case configured
        case hasAPIKey = "has_api_key"
        case keyConfigured = "key_configured"
        case credentialStatus = "credential_status"
        case credentialReference = "credential_reference"
        case credentialStorage = "credential_storage"
        case disabledReason = "disabled_reason"
        case reason
        case budget
        case singleRequestTokenLimit = "single_request_token_limit"
        case monthlyBudgetUSD = "monthly_budget_usd"
    }

    init(
        id: String,
        name: String,
        kind: AIProviderKind,
        endpoint: String?,
        model: String?,
        apiVersion: String?,
        enabled: Bool,
        configured: Bool,
        hasAPIKey: Bool,
        credentialStorage: String?,
        disabledReason: String?,
        budget: AIProviderBudget?
    ) {
        self.id = id
        self.name = name
        self.kind = kind
        self.endpoint = endpoint
        self.model = model
        self.apiVersion = apiVersion
        self.enabled = enabled
        self.configured = configured
        self.hasAPIKey = hasAPIKey
        self.credentialStorage = credentialStorage
        self.disabledReason = disabledReason
        self.budget = budget
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let decodedKind = try container.decodeIfPresent(AIProviderKind.self, forKey: .kind)
            ?? container.decodeIfPresent(AIProviderKind.self, forKey: .providerKind)
            ?? container.decodeIfPresent(AIProviderKind.self, forKey: .providerType)
            ?? AIProviderKind.fromService(try container.decodeIfPresent(String.self, forKey: .provider))
        kind = decodedKind
        id = try container.decodeIfPresent(String.self, forKey: .id) ?? decodedKind.rawValue
        name = try container.decodeIfPresent(String.self, forKey: .name)
            ?? container.decodeIfPresent(String.self, forKey: .displayName)
            ?? decodedKind.title
        endpoint = try container.decodeIfPresent(String.self, forKey: .endpoint)
            ?? container.decodeIfPresent(String.self, forKey: .baseURL)
        model = try container.decodeIfPresent(String.self, forKey: .model)
        apiVersion = try container.decodeIfPresent(String.self, forKey: .apiVersion)
        enabled = try container.decodeIfPresent(Bool.self, forKey: .enabled) ?? false
        let credentialStatus = try container.decodeIfPresent(AIProviderCredentialStatus.self, forKey: .credentialStatus)
        hasAPIKey = try container.decodeIfPresent(Bool.self, forKey: .hasAPIKey)
            ?? container.decodeIfPresent(Bool.self, forKey: .keyConfigured)
            ?? credentialStatus?.secretAvailable
            ?? false
        configured = try container.decodeIfPresent(Bool.self, forKey: .configured)
            ?? (endpoint?.isEmpty == false && model?.isEmpty == false && hasAPIKey)
        let credentialReference = try container.decodeIfPresent(AIProviderCredentialReference.self, forKey: .credentialReference)
        credentialStorage = try container.decodeIfPresent(String.self, forKey: .credentialStorage)
            ?? credentialReference?.storage
        disabledReason = try container.decodeIfPresent(String.self, forKey: .disabledReason)
            ?? container.decodeIfPresent(String.self, forKey: .reason)
            ?? credentialStatus?.reason
        if let decodedBudget = try container.decodeIfPresent(AIProviderBudget.self, forKey: .budget) {
            budget = decodedBudget
        } else {
            budget = AIProviderBudget(
                singleRequestTokenLimit: try container.decodeIfPresent(Int.self, forKey: .singleRequestTokenLimit),
                monthlyBudgetUSD: try container.decodeIfPresent(Double.self, forKey: .monthlyBudgetUSD)
            )
        }
    }
}

struct AIProviderSaveResult: Decodable, Hashable {
    let profile: AIProviderProfile?
    let outcome: AIProviderActionOutcome?
    let readback: ActionReadbackRecordWire?
}

struct AIProviderActionOutcome: Decodable, Hashable {
    let state: String
    let effect: String
    let remoteEffect: String
    let localEffect: String
    let credentialEffect: String
    let recovery: String?

    enum CodingKeys: String, CodingKey {
        case state
        case effect
        case remoteEffect = "remote_effect"
        case localEffect = "local_effect"
        case credentialEffect = "credential_effect"
        case recovery
    }

    var isVerified: Bool { state.lowercased() == "verified" }
    var isPartial: Bool { state.lowercased() == "partial" }
    var userFacingFailure: String {
        if let recovery {
            let trimmed = recovery.trimmingCharacters(in: .whitespacesAndNewlines)
            if !trimmed.isEmpty {
                return trimmed
            }
        }
        return "Provider action outcome is \(effect); reload provider settings before creating a new preview."
    }
}

struct AIProviderActionPreview: Decodable, Hashable {
    let action: ActionDescriptorWire
    let preconditions: [ActionPreconditionWire]
    let previewToken: String
    let operation: String
    let profileID: String
    let providerType: String
    let destinationHost: String
    let model: String
    let expectedRevision: String
    let credentialChange: Bool
    let rawSecretReturned: Bool

    enum CodingKeys: String, CodingKey {
        case action
        case preconditions
        case previewToken = "preview_token"
        case operation
        case profileID = "profile_id"
        case providerType = "provider_type"
        case destinationHost = "destination_host"
        case model
        case expectedRevision = "expected_revision"
        case credentialChange = "credential_change"
        case rawSecretReturned = "raw_secret_returned"
    }

    var confirmation: ActionConfirmationWire {
        ActionConfirmationWire(action: action, previewToken: previewToken)
    }
}

enum AIProviderPendingAction: String, Hashable {
    case save
    case delete
    case test
}

struct AIProviderDeleteResult: Decodable, Hashable {
    let deletedProfileID: String
    let profileDeleted: Bool
    let credentialDeleted: Bool
    let rawSecretReturned: Bool
    let outcome: AIProviderActionOutcome?
    let readback: ActionReadbackRecordWire?

    enum CodingKeys: String, CodingKey {
        case deletedProfileID = "deleted_profile_id"
        case profileDeleted = "profile_deleted"
        case credentialDeleted = "credential_deleted"
        case rawSecretReturned = "raw_secret_returned"
        case outcome
        case readback
    }
}

struct AIProviderCallAuditMetadata: Decodable, Hashable {
    let auditID: String?
    let status: String
    let provider: String?
    let model: String?
    let endpoint: String?
    let durationMS: Int?
    let redactionApplied: Bool
    let promptStored: Bool
    let responseStored: Bool
    let inputTokens: Int?
    let outputTokens: Int?
    let estimatedCostUSD: Double?
    let errorCode: String?
    let errorMessage: String?

    enum CodingKeys: String, CodingKey {
        case auditID = "audit_id"
        case requestID = "request_id"
        case status
        case provider
        case providerType = "provider_type"
        case model
        case endpoint
        case baseURL = "base_url"
        case destinationHost = "destination_host"
        case durationMS = "duration_ms"
        case redactionApplied = "redaction_applied"
        case redactionStatus = "redaction_status"
        case promptStored = "prompt_stored"
        case rawPromptPersisted = "raw_prompt_persisted"
        case responseStored = "response_stored"
        case rawResponsePersisted = "raw_response_persisted"
        case inputTokens = "input_tokens"
        case estimatedInputTokens = "estimated_input_tokens"
        case outputTokens = "output_tokens"
        case estimatedOutputTokens = "estimated_output_tokens"
        case estimatedCostUSD = "estimated_cost_usd"
        case errorCode = "error_code"
        case errorMessage = "error_message"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        auditID = try container.decodeIfPresent(String.self, forKey: .auditID)
            ?? container.decodeIfPresent(String.self, forKey: .requestID)
        status = try container.decodeIfPresent(String.self, forKey: .status) ?? "unknown"
        provider = try container.decodeIfPresent(String.self, forKey: .provider)
            ?? container.decodeIfPresent(String.self, forKey: .providerType)
        model = try container.decodeIfPresent(String.self, forKey: .model)
        endpoint = try container.decodeIfPresent(String.self, forKey: .endpoint)
            ?? container.decodeIfPresent(String.self, forKey: .baseURL)
            ?? container.decodeIfPresent(String.self, forKey: .destinationHost)
        durationMS = try container.decodeIfPresent(Int.self, forKey: .durationMS)
        if let decodedRedactionApplied = try container.decodeIfPresent(Bool.self, forKey: .redactionApplied) {
            redactionApplied = decodedRedactionApplied
        } else {
            redactionApplied = (try container.decodeIfPresent(String.self, forKey: .redactionStatus)) != nil
        }
        promptStored = try container.decodeIfPresent(Bool.self, forKey: .promptStored)
            ?? container.decodeIfPresent(Bool.self, forKey: .rawPromptPersisted)
            ?? false
        responseStored = try container.decodeIfPresent(Bool.self, forKey: .responseStored)
            ?? container.decodeIfPresent(Bool.self, forKey: .rawResponsePersisted)
            ?? false
        inputTokens = try container.decodeIfPresent(Int.self, forKey: .inputTokens)
            ?? container.decodeIfPresent(Int.self, forKey: .estimatedInputTokens)
        outputTokens = try container.decodeIfPresent(Int.self, forKey: .outputTokens)
            ?? container.decodeIfPresent(Int.self, forKey: .estimatedOutputTokens)
        estimatedCostUSD = try container.decodeIfPresent(Double.self, forKey: .estimatedCostUSD)
        errorCode = try container.decodeIfPresent(String.self, forKey: .errorCode)
        errorMessage = try container.decodeIfPresent(String.self, forKey: .errorMessage)
    }
}

struct AIProviderTestResult: Decodable, Hashable {
    let success: Bool
    let status: String
    let message: String
    let audit: AIProviderCallAuditMetadata?
    let outcome: AIProviderActionOutcome?
    let readback: ActionReadbackRecordWire?

    enum CodingKeys: String, CodingKey {
        case success
        case ok
        case status
        case message
        case reason
        case errorMessage = "error_message"
        case audit
        case auditMetadata = "audit_metadata"
        case metadata
        case outcome
        case readback
    }

    init(
        success: Bool,
        status: String,
        message: String,
        audit: AIProviderCallAuditMetadata?,
        outcome: AIProviderActionOutcome? = nil,
        readback: ActionReadbackRecordWire? = nil
    ) {
        self.success = success
        self.status = status
        self.message = message
        self.audit = audit
        self.outcome = outcome
        self.readback = readback
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let decodedStatus = try container.decodeIfPresent(String.self, forKey: .status) ?? "unknown"
        status = decodedStatus
        success = try container.decodeIfPresent(Bool.self, forKey: .success)
            ?? container.decodeIfPresent(Bool.self, forKey: .ok)
            ?? ["ok", "success", "passed", "succeeded"].contains(decodedStatus.lowercased())
        message = try container.decodeIfPresent(String.self, forKey: .message)
            ?? container.decodeIfPresent(String.self, forKey: .reason)
            ?? container.decodeIfPresent(String.self, forKey: .errorMessage)
            ?? (success ? UIStrings.aiProviderTestSucceeded : UIStrings.aiProviderTestFailed)
        audit = try container.decodeIfPresent(AIProviderCallAuditMetadata.self, forKey: .audit)
            ?? container.decodeIfPresent(AIProviderCallAuditMetadata.self, forKey: .auditMetadata)
            ?? container.decodeIfPresent(AIProviderCallAuditMetadata.self, forKey: .metadata)
        outcome = try container.decodeIfPresent(AIProviderActionOutcome.self, forKey: .outcome)
        readback = try container.decodeIfPresent(ActionReadbackRecordWire.self, forKey: .readback)
    }

    static func unavailable(reason: String = UIStrings.aiProviderUnavailable) -> AIProviderTestResult {
        AIProviderTestResult(
            success: false,
            status: "unavailable",
            message: reason,
            audit: nil,
            readback: nil
        )
    }
}

struct AIProviderStatus: Decodable, Hashable {
    let serviceAvailable: Bool
    let enabled: Bool
    let configured: Bool
    let disabledReason: String?
    let activeProfileID: String?
    let profiles: [AIProviderProfile]
    let budget: AIProviderBudget
    let credentialStorage: String?
    let credentialPersistenceAllowed: Bool
    let lastTest: AIProviderTestResult?

    enum CodingKeys: String, CodingKey {
        case serviceAvailable = "service_available"
        case enabled
        case configured
        case disabledReason = "disabled_reason"
        case reason
        case activeProfileID = "active_profile_id"
        case defaultProfileID = "default_profile_id"
        case activeProfile = "active_profile"
        case profiles
        case providerProfiles = "provider_profiles"
        case budget
        case singleRequestTokenLimit = "single_request_token_limit"
        case monthlyBudgetUSD = "monthly_budget_usd"
        case perRequestBudgetUSD = "per_request_budget_usd"
        case spentThisMonthUSD = "spent_this_month_usd"
        case credentialStorage = "credential_storage"
        case credentialPersistenceAllowed = "credential_persistence_allowed"
        case lastTest = "last_test"
        case testResult = "test_result"
    }

    init(
        serviceAvailable: Bool,
        enabled: Bool,
        configured: Bool,
        disabledReason: String?,
        activeProfileID: String?,
        profiles: [AIProviderProfile],
        budget: AIProviderBudget,
        credentialStorage: String?,
        credentialPersistenceAllowed: Bool,
        lastTest: AIProviderTestResult?
    ) {
        self.serviceAvailable = serviceAvailable
        self.enabled = enabled
        self.configured = configured
        self.disabledReason = disabledReason
        self.activeProfileID = activeProfileID
        self.profiles = profiles
        self.budget = budget
        self.credentialStorage = credentialStorage
        self.credentialPersistenceAllowed = credentialPersistenceAllowed
        self.lastTest = lastTest
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        serviceAvailable = try container.decodeIfPresent(Bool.self, forKey: .serviceAvailable) ?? true
        disabledReason = try container.decodeIfPresent(String.self, forKey: .disabledReason)
            ?? container.decodeIfPresent(String.self, forKey: .reason)
        let decodedActiveProfileID = try container.decodeIfPresent(String.self, forKey: .activeProfileID)
            ?? container.decodeIfPresent(String.self, forKey: .defaultProfileID)
            ?? container.decodeIfPresent(String.self, forKey: .activeProfile)
        activeProfileID = decodedActiveProfileID
        let decodedProfiles = try container.decodeIfPresent([AIProviderProfile].self, forKey: .profiles)
            ?? container.decodeIfPresent([AIProviderProfile].self, forKey: .providerProfiles)
            ?? []
        profiles = decodedProfiles
        let selectedProfile = decodedActiveProfileID.flatMap { id in decodedProfiles.first { $0.id == id } } ?? decodedProfiles.first
        enabled = try container.decodeIfPresent(Bool.self, forKey: .enabled)
            ?? selectedProfile?.enabled
            ?? false
        configured = try container.decodeIfPresent(Bool.self, forKey: .configured)
            ?? selectedProfile?.configured
            ?? false
        if let decodedBudget = try container.decodeIfPresent(AIProviderBudget.self, forKey: .budget) {
            budget = decodedBudget
        } else {
            budget = AIProviderBudget(
                singleRequestTokenLimit: try container.decodeIfPresent(Int.self, forKey: .singleRequestTokenLimit),
                monthlyBudgetUSD: try container.decodeIfPresent(Double.self, forKey: .monthlyBudgetUSD),
                perRequestBudgetUSD: try container.decodeIfPresent(Double.self, forKey: .perRequestBudgetUSD),
                spentThisMonthUSD: try container.decodeIfPresent(Double.self, forKey: .spentThisMonthUSD)
            )
        }
        credentialStorage = try container.decodeIfPresent(String.self, forKey: .credentialStorage)
        credentialPersistenceAllowed = try container.decodeIfPresent(Bool.self, forKey: .credentialPersistenceAllowed) ?? false
        lastTest = try container.decodeIfPresent(AIProviderTestResult.self, forKey: .lastTest)
            ?? container.decodeIfPresent(AIProviderTestResult.self, forKey: .testResult)
    }

    var activeProfile: AIProviderProfile? {
        guard let activeProfileID else { return profiles.first }
        return profiles.first { $0.id == activeProfileID } ?? profiles.first
    }

    static func unavailable(reason: String = UIStrings.aiProviderUnavailable) -> AIProviderStatus {
        AIProviderStatus(
            serviceAvailable: false,
            enabled: false,
            configured: false,
            disabledReason: reason,
            activeProfileID: nil,
            profiles: [],
            budget: AIProviderBudget(),
            credentialStorage: "none",
            credentialPersistenceAllowed: false,
            lastTest: nil
        )
    }
}

struct AIProviderSettingsDraft: Equatable {
    var kind: AIProviderKind
    var endpoint: String
    var model: String
    var apiVersion: String
    var apiKey: String
    var monthlyBudgetUSD: String
    var singleRequestTokenLimit: String

    init(status: AIProviderStatus) {
        let profile = status.activeProfile
        kind = profile?.kind == .unknown ? .openAICompatible : (profile?.kind ?? .openAICompatible)
        endpoint = profile?.endpoint ?? ""
        model = profile?.model ?? ""
        apiVersion = profile?.apiVersion ?? ""
        apiKey = ""
        monthlyBudgetUSD = AIProviderSettingsDraft.string(from: profile?.budget?.monthlyBudgetUSD ?? status.budget.monthlyBudgetUSD)
        singleRequestTokenLimit = AIProviderSettingsDraft.string(from: profile?.budget?.singleRequestTokenLimit ?? status.budget.singleRequestTokenLimit)
    }

    var trimmedEndpoint: String {
        endpoint.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    var trimmedModel: String {
        model.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    var trimmedAPIVersion: String? {
        let value = apiVersion.trimmingCharacters(in: .whitespacesAndNewlines)
        return value.isEmpty ? nil : value
    }

    var trimmedAPIKey: String? {
        let value = apiKey.trimmingCharacters(in: .whitespacesAndNewlines)
        return value.isEmpty ? nil : value
    }

    var parsedMonthlyBudgetUSD: Double? {
        Double(monthlyBudgetUSD.trimmingCharacters(in: .whitespacesAndNewlines))
    }

    var parsedSingleRequestTokenLimit: Int? {
        Int(singleRequestTokenLimit.trimmingCharacters(in: .whitespacesAndNewlines))
    }

    var validationMessage: String? {
        if trimmedEndpoint.isEmpty {
            return UIStrings.aiProviderEndpointRequired
        }
        if URL(string: trimmedEndpoint)?.scheme == nil {
            return UIStrings.aiProviderEndpointInvalid
        }
        if trimmedModel.isEmpty {
            return UIStrings.aiProviderModelRequired
        }
        if !monthlyBudgetUSD.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty, parsedMonthlyBudgetUSD == nil {
            return UIStrings.aiProviderBudgetInvalid
        }
        if !singleRequestTokenLimit.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty, parsedSingleRequestTokenLimit == nil {
            return UIStrings.aiProviderTokenLimitInvalid
        }
        return nil
    }

    private static func string(from value: Double?) -> String {
        guard let value else { return "" }
        return value == floor(value) ? String(Int(value)) : String(value)
    }

    private static func string(from value: Int?) -> String {
        guard let value else { return "" }
        return String(value)
    }
}
