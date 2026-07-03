import Foundation

enum ScriptExecutionAuditStatus: String, Codable, Hashable {
    case unavailable
    case previewOnly = "preview_only"
    case blocked
    case requiresConfirmation = "requires_confirmation"
    case audited
    case unknown

    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        let value = try container.decode(String.self)
        self = ScriptExecutionAuditStatus(rawValue: value) ?? .unknown
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        try container.encode(rawValue)
    }
}

struct ScriptExecutionScope: Codable, Hashable {
    let cwd: String?
    let env: [String: String]
    let network: String?
    let files: [String]

    enum CodingKeys: String, CodingKey {
        case cwd
        case currentCWD = "current_cwd"
        case env
        case network
        case files
    }

    init(cwd: String?, env: [String: String], network: String?, files: [String]) {
        self.cwd = cwd
        self.env = env
        self.network = network
        self.files = files
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        cwd = try container.decodeIfPresent(String.self, forKey: .cwd)
            ?? container.decodeIfPresent(String.self, forKey: .currentCWD)
        env = try container.decodeIfPresent([String: String].self, forKey: .env) ?? [:]
        network = try container.decodeIfPresent(String.self, forKey: .network)
        files = try container.decodeIfPresent([String].self, forKey: .files) ?? []
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encodeIfPresent(cwd, forKey: .cwd)
        try container.encode(env, forKey: .env)
        try container.encodeIfPresent(network, forKey: .network)
        try container.encode(files, forKey: .files)
    }
}

struct ScriptExecutionPreview: Codable, Identifiable, Hashable {
    let skillID: String
    let scriptName: String?
    let commandPreview: [String]
    let scope: ScriptExecutionScope
    let risks: [String]
    let confirmationRequired: Bool
    let executionAllowed: Bool
    let auditStatus: ScriptExecutionAuditStatus
    let auditID: String?
    let summary: String
    let disabledReason: String?

    var id: String { "\(skillID):\(scriptName ?? "default")" }

    var hasOverviewSignal: Bool {
        !risks.isEmpty
            || executionAllowed
            || (!commandPreview.isEmpty && confirmationRequired)
            || auditStatus == .requiresConfirmation
            || auditStatus == .audited
    }

    enum CodingKeys: String, CodingKey {
        case skillID = "skill_id"
        case instanceID = "instance_id"
        case scriptName = "script_name"
        case commandPreview = "command_preview"
        case command
        case scope
        case cwd
        case env
        case network
        case files
        case risks
        case confirmationRequired = "confirmation_required"
        case requiresConfirmation = "requires_confirmation"
        case executionAllowed = "execution_allowed"
        case allowed
        case auditStatus = "audit_status"
        case auditID = "audit_id"
        case summary
        case disabledReason = "disabled_reason"
        case reason
    }

    init(
        skillID: String,
        scriptName: String?,
        commandPreview: [String],
        scope: ScriptExecutionScope,
        risks: [String],
        confirmationRequired: Bool,
        executionAllowed: Bool,
        auditStatus: ScriptExecutionAuditStatus,
        auditID: String?,
        summary: String,
        disabledReason: String?
    ) {
        self.skillID = skillID
        self.scriptName = scriptName
        self.commandPreview = commandPreview
        self.scope = scope
        self.risks = risks
        self.confirmationRequired = confirmationRequired
        self.executionAllowed = executionAllowed
        self.auditStatus = auditStatus
        self.auditID = auditID
        self.summary = summary
        self.disabledReason = disabledReason
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let skillID = try container.decodeIfPresent(String.self, forKey: .skillID)
            ?? container.decode(String.self, forKey: .instanceID)
        let commandPreview = try container.decodeIfPresent([String].self, forKey: .commandPreview)
            ?? container.decodeIfPresent([String].self, forKey: .command)
            ?? []
        let nestedScope = try container.decodeIfPresent(ScriptExecutionScope.self, forKey: .scope)
        let inlineScope = ScriptExecutionScope(
            cwd: try container.decodeIfPresent(String.self, forKey: .cwd),
            env: try container.decodeIfPresent([String: String].self, forKey: .env) ?? [:],
            network: try container.decodeIfPresent(String.self, forKey: .network),
            files: try container.decodeIfPresent([String].self, forKey: .files) ?? []
        )
        let executionAllowed = try container.decodeIfPresent(Bool.self, forKey: .executionAllowed)
            ?? container.decodeIfPresent(Bool.self, forKey: .allowed)
            ?? false
        let auditStatus = try container.decodeIfPresent(ScriptExecutionAuditStatus.self, forKey: .auditStatus)
            ?? (executionAllowed ? .requiresConfirmation : .blocked)

        self.init(
            skillID: skillID,
            scriptName: try container.decodeIfPresent(String.self, forKey: .scriptName),
            commandPreview: commandPreview,
            scope: nestedScope ?? inlineScope,
            risks: try container.decodeIfPresent([String].self, forKey: .risks) ?? [],
            confirmationRequired: try container.decodeIfPresent(Bool.self, forKey: .confirmationRequired)
                ?? container.decodeIfPresent(Bool.self, forKey: .requiresConfirmation)
                ?? true,
            executionAllowed: executionAllowed,
            auditStatus: auditStatus,
            auditID: try container.decodeIfPresent(String.self, forKey: .auditID),
            summary: try container.decodeIfPresent(String.self, forKey: .summary) ?? UIStrings.scriptExecutionPreviewSummary,
            disabledReason: try container.decodeIfPresent(String.self, forKey: .disabledReason)
                ?? container.decodeIfPresent(String.self, forKey: .reason)
        )
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(skillID, forKey: .skillID)
        try container.encodeIfPresent(scriptName, forKey: .scriptName)
        try container.encode(commandPreview, forKey: .commandPreview)
        try container.encode(scope, forKey: .scope)
        try container.encode(risks, forKey: .risks)
        try container.encode(confirmationRequired, forKey: .confirmationRequired)
        try container.encode(executionAllowed, forKey: .executionAllowed)
        try container.encode(auditStatus, forKey: .auditStatus)
        try container.encodeIfPresent(auditID, forKey: .auditID)
        try container.encode(summary, forKey: .summary)
        try container.encodeIfPresent(disabledReason, forKey: .disabledReason)
    }

    static func unavailable(skill: SkillRecord, reason: String = UIStrings.scriptExecutionUnavailable) -> ScriptExecutionPreview {
        ScriptExecutionPreview(
            skillID: skill.id,
            scriptName: nil,
            commandPreview: [],
            scope: ScriptExecutionScope(cwd: nil, env: [:], network: nil, files: []),
            risks: [],
            confirmationRequired: true,
            executionAllowed: false,
            auditStatus: .unavailable,
            auditID: nil,
            summary: UIStrings.scriptExecutionPreviewSummary,
            disabledReason: reason
        )
    }
}
