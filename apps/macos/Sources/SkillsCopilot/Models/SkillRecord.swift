import Foundation

struct SkillRecord: Codable, Identifiable, Hashable {
    let id: String
    let agent: String
    let scope: String
    let path: String
    let displayPath: String
    let definitionId: String
    let name: String
    let state: String
    let enabled: Bool
    let publisher: String?
    let packageName: String?
    let packageVersion: String?
    let sourceKind: String?
    let readOnlyReason: String?

    init(
        id: String,
        agent: String,
        scope: String,
        path: String,
        displayPath: String,
        definitionId: String,
        name: String,
        state: String,
        enabled: Bool,
        publisher: String? = nil,
        packageName: String? = nil,
        packageVersion: String? = nil,
        sourceKind: String? = nil,
        readOnlyReason: String? = nil
    ) {
        self.id = id
        self.agent = agent
        self.scope = scope
        self.path = path
        self.displayPath = displayPath
        self.definitionId = definitionId
        self.name = name
        self.state = state
        self.enabled = enabled
        self.publisher = publisher
        self.packageName = packageName
        self.packageVersion = packageVersion
        self.sourceKind = sourceKind
        self.readOnlyReason = readOnlyReason
    }

    enum CodingKeys: String, CodingKey {
        case id
        case agent
        case scope
        case path
        case displayPath = "display_path"
        case definitionId = "definition_id"
        case name
        case state
        case enabled
        case publisher
        case packageName = "package_name"
        case packageVersion = "package_version"
        case sourceKind = "source_kind"
        case readOnlyReason = "read_only_reason"
    }
}

enum SkillProvenanceRootKind: String, Hashable {
    case native
    case compatibility
    case configured
    case external
    case toolGlobal
    case readOnly
    case unknown
}

enum SkillProvenanceScopeKind: String, Hashable {
    case project
    case global
    case external
    case toolGlobal
    case unknown
}

struct SkillProvenance: Hashable {
    let rootKind: SkillProvenanceRootKind
    let scopeKind: SkillProvenanceScopeKind
    let label: String
    let isReadOnly: Bool
    let isCatalogedSkill: Bool
}

struct SkillIdentitySummary: Hashable {
    let title: String
    let definitionId: String
    let identityKey: String
    let sourceKey: String
    let catalogPath: String
    let provenanceLabel: String
    let state: String
    let isCatalogedSkill: Bool
}

enum SkillDedupeReason: String, Hashable {
    case definitionId
    case name
    case catalogPath
    case distinct
}

struct SkillDedupeExplanation: Hashable {
    let reason: SkillDedupeReason
    let summary: String
    let participantSummaries: [String]
}

extension SkillRecord {
    var pluginPackageSummary: String? {
        guard sourceKind == "chatgpt-plugin-cache", let publisher, let packageName else {
            return nil
        }
        let package = "\(publisher)/\(packageName)"
        guard let packageVersion, !packageVersion.isEmpty else { return package }
        return "\(package) \(packageVersion)"
    }

    var provenance: SkillProvenance {
        let rootKind = inferredRootKind
        let scopeKind = inferredScopeKind
        let cataloged = isCatalogedSkillIdentity
        return SkillProvenance(
            rootKind: rootKind,
            scopeKind: scopeKind,
            label: provenanceLabel(rootKind: rootKind, scopeKind: scopeKind, isCatalogedSkill: cataloged),
            isReadOnly: isReadOnlyProvenance,
            isCatalogedSkill: cataloged
        )
    }

    var identitySummary: SkillIdentitySummary {
        let provenance = provenance
        return SkillIdentitySummary(
            title: stableDisplayName,
            definitionId: definitionId,
            identityKey: identityKey,
            sourceKey: sourceKey,
            catalogPath: catalogIdentityPath,
            provenanceLabel: provenance.label,
            state: state,
            isCatalogedSkill: provenance.isCatalogedSkill
        )
    }

    func dedupeExplanation(comparedWith other: SkillRecord) -> SkillDedupeExplanation {
        let reason: SkillDedupeReason
        let summary: String
        if hasMatchingDefinitionId(with: other) {
            reason = .definitionId
            summary = "Same definition ID: \(definitionId.trimmingCharacters(in: .whitespacesAndNewlines).lowercased())"
        } else if hasMatchingName(with: other) {
            reason = .name
            summary = "Same skill name: \(stableDisplayName.trimmingCharacters(in: .whitespacesAndNewlines).lowercased())"
        } else if hasMatchingCatalogPath(with: other) {
            reason = .catalogPath
            summary = "Same catalog path: \(catalogIdentityPath.lowercased())"
        } else {
            reason = .distinct
            summary = "No shared identity signal"
        }
        return SkillDedupeExplanation(
            reason: reason,
            summary: summary,
            participantSummaries: [participantSummary, other.participantSummary].sorted()
        )
    }

    var isCatalogedSkillIdentity: Bool {
        if normalizedAgent == "pi" {
            return pathComponents.last?.caseInsensitiveCompare("SKILL.md") == .orderedSame
        }
        return true
    }

    var catalogIdentityPath: String {
        let components = pathComponents
        guard components.last?.caseInsensitiveCompare("SKILL.md") == .orderedSame else {
            return normalizedPath
        }
        let directoryComponents = components.dropLast()
        return SkillIdentityPath.join(directoryComponents, absolute: normalizedPath.hasPrefix("/"))
    }

    var identityKey: String {
        let cleanDefinitionId = definitionId.trimmingCharacters(in: .whitespacesAndNewlines)
        if !cleanDefinitionId.isEmpty {
            return "definition:\(cleanDefinitionId.lowercased())"
        }
        let cleanName = stableDisplayName.trimmingCharacters(in: .whitespacesAndNewlines)
        if !cleanName.isEmpty {
            return "name:\(cleanName.lowercased())"
        }
        return "path:\(catalogIdentityPath.lowercased())"
    }

    var sourceKey: String {
        [
            normalizedAgent,
            scope.trimmingCharacters(in: .whitespacesAndNewlines).lowercased(),
            catalogIdentityPath.lowercased()
        ].joined(separator: "|")
    }

    private var inferredRootKind: SkillProvenanceRootKind {
        if sourceKind == "chatgpt-plugin-cache" || sourceKind == "codex-runtime" {
            return .readOnly
        }
        if normalizedScopeContains("tool") || normalizedPathContains("/tool-global/") || normalizedPathContains("/skill-pool/") {
            return .toolGlobal
        }
        if normalizedScopeContains("external") || normalizedPathContains("/external/") {
            return .external
        }
        switch normalizedAgent {
        case "opencode":
            if normalizedPathContains(".claude/skills/") || normalizedPathContains(".agents/skills/") {
                return .compatibility
            }
            if normalizedPathContains(".opencode/skills/") || normalizedPathContains(".config/opencode/skills/") {
                return .native
            }
            return .configured
        case "claude-code":
            if normalizedPathContains(".claude/skills/") || normalizedPathContains(".claude/commands/") {
                return .native
            }
        case "codex":
            if normalizedPathContains(".agents/skills/") || normalizedPathContains(".codex/skills/") {
                return .native
            }
        case "pi":
            if normalizedPathContains(".agents/skills/") {
                return .compatibility
            }
            if normalizedPathContains(".pi/agent/skills/") || normalizedPathContains(".pi/skills/") {
                return .native
            }
        case "hermes", "openclaw":
            return .readOnly
        default:
            break
        }
        return .unknown
    }

    private var inferredScopeKind: SkillProvenanceScopeKind {
        if normalizedScopeContains("tool") || normalizedPathContains("/tool-global/") || normalizedPathContains("/skill-pool/") {
            return .toolGlobal
        }
        if normalizedScopeContains("external") || normalizedPathContains("/external/") {
            return .external
        }
        if normalizedScopeContains("project") || normalizedPathContains(".opencode/skills/") {
            return .project
        }
        if normalizedScopeContains("global") || normalizedScopeContains("user") || normalizedPathContains(".config/opencode/skills/") {
            return .global
        }
        return .unknown
    }

    private var isReadOnlyProvenance: Bool {
        sourceKind == "chatgpt-plugin-cache" || sourceKind == "codex-runtime" || normalizedAgent == "hermes" || normalizedAgent == "openclaw"
    }

    private var stableDisplayName: String {
        let cleanName = name.trimmingCharacters(in: .whitespacesAndNewlines)
        return cleanName.isEmpty ? definitionId.trimmingCharacters(in: .whitespacesAndNewlines) : cleanName
    }

    private var participantSummary: String {
        "\(stableDisplayName) [\(provenance.label)] \(sourceKey)"
    }

    private var normalizedAgent: String {
        let normalized = agent
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
            .replacingOccurrences(of: "_", with: "-")
            .replacingOccurrences(of: " ", with: "-")
        return normalized == "claude" ? "claude-code" : normalized
    }

    private var normalizedPath: String {
        SkillIdentityPath.normalize(path)
    }

    private var normalizedPathLowercased: String {
        normalizedPath.lowercased()
    }

    private var normalizedDisplayPathLowercased: String {
        SkillIdentityPath.normalize(displayPath).lowercased()
    }

    private var pathComponents: [String] {
        SkillIdentityPath.components(normalizedPath)
    }

    private func normalizedPathContains(_ needle: String) -> Bool {
        normalizedPathLowercased.contains(needle) || normalizedDisplayPathLowercased.contains(needle)
    }

    private func normalizedScopeContains(_ needle: String) -> Bool {
        scope.trimmingCharacters(in: .whitespacesAndNewlines).lowercased().contains(needle)
    }

    private func hasMatchingDefinitionId(with other: SkillRecord) -> Bool {
        let lhs = definitionId.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        let rhs = other.definitionId.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return !lhs.isEmpty && lhs == rhs
    }

    private func hasMatchingName(with other: SkillRecord) -> Bool {
        let lhs = stableDisplayName.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        let rhs = other.stableDisplayName.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return !lhs.isEmpty && lhs == rhs
    }

    private func hasMatchingCatalogPath(with other: SkillRecord) -> Bool {
        catalogIdentityPath.caseInsensitiveCompare(other.catalogIdentityPath) == .orderedSame
    }

    private func provenanceLabel(
        rootKind: SkillProvenanceRootKind,
        scopeKind: SkillProvenanceScopeKind,
        isCatalogedSkill: Bool
    ) -> String {
        if normalizedAgent == "pi" && !isCatalogedSkill {
            return "Pi document (not cataloged)"
        }
        let agentLabel = SkillIdentityText.agentLabel(normalizedAgent)
        let rootLabel: String
        switch rootKind {
        case .native:
            rootLabel = "native"
        case .compatibility:
            rootLabel = "compatibility"
        case .configured:
            rootLabel = "configured"
        case .external:
            rootLabel = "external"
        case .toolGlobal:
            rootLabel = "tool-global"
        case .readOnly:
            rootLabel = "read-only"
        case .unknown:
            rootLabel = "unknown"
        }
        let scopeLabel: String
        switch scopeKind {
        case .project:
            scopeLabel = "project"
        case .global:
            scopeLabel = "global"
        case .external:
            scopeLabel = "external"
        case .toolGlobal:
            scopeLabel = "tool-global"
        case .unknown:
            scopeLabel = "scope"
        }
        if rootKind == .readOnly {
            if sourceKind == "chatgpt-plugin-cache", let pluginPackageSummary {
                return "\(agentLabel) plugin install · \(pluginPackageSummary)"
            }
            if normalizedAgent == "hermes" && scopeKind == .global {
                return "\(agentLabel) home/profile read-only"
            }
            if normalizedAgent == "openclaw" && scopeKind == .project {
                return "\(agentLabel) workspace read-only"
            }
            return "\(agentLabel) read-only \(scopeLabel)"
        }
        if rootKind == .compatibility && normalizedAgent == "opencode" {
            if normalizedPathContains(".claude/skills/") {
                return "opencode official compatibility · .claude/skills"
            }
            if normalizedPathContains(".agents/skills/") {
                return "opencode official compatibility · .agents/skills"
            }
        }
        if rootKind == .toolGlobal || scopeKind == .toolGlobal {
            return "\(agentLabel) tool-global"
        }
        if rootKind == .external || scopeKind == .external {
            if normalizedAgent == "hermes" {
                return "\(agentLabel) explicit external read-only"
            }
            return "\(agentLabel) external"
        }
        return "\(agentLabel) \(rootLabel) \(scopeLabel)"
    }
}

private enum SkillIdentityPath {
    static func normalize(_ path: String) -> String {
        let trimmed = path.trimmingCharacters(in: .whitespacesAndNewlines)
        let slashPath = trimmed.replacingOccurrences(of: "\\", with: "/")
        let absolute = slashPath.hasPrefix("/")
        return join(components(slashPath), absolute: absolute)
    }

    static func components(_ path: String) -> [String] {
        path
            .split(separator: "/", omittingEmptySubsequences: true)
            .map(String.init)
    }

    static func join<S: Sequence>(_ components: S, absolute: Bool) -> String where S.Element == String {
        let joined = components.joined(separator: "/")
        if absolute {
            return joined.isEmpty ? "/" : "/\(joined)"
        }
        return joined
    }
}

private enum SkillIdentityText {
    static func agentLabel(_ agent: String) -> String {
        switch agent {
        case "claude-code":
            return "Claude Code"
        case "codex":
            return "Codex"
        case "opencode":
            return "opencode"
        case "pi":
            return "Pi"
        case "hermes":
            return "Hermes"
        case "openclaw":
            return "OpenClaw"
        default:
            return agent.isEmpty ? "Unknown agent" : agent
        }
    }
}

struct SkillDetailRecord: Codable, Identifiable, Hashable {
    let id: String
    let agent: String
    let scope: String
    let path: String
    let displayPath: String
    let definitionId: String
    let name: String
    let description: String
    let state: String
    let enabled: Bool
    let frontmatterRaw: String
    let body: String
    let permissions: JSONValue
    let fingerprint: String
    let publisher: String?
    let packageName: String?
    let packageVersion: String?
    let sourceKind: String?
    let readOnlyReason: String?

    enum CodingKeys: String, CodingKey {
        case id
        case agent
        case scope
        case path
        case displayPath = "display_path"
        case definitionId = "definition_id"
        case name
        case description
        case state
        case enabled
        case frontmatterRaw = "frontmatter_raw"
        case body
        case permissions
        case fingerprint
        case publisher
        case packageName = "package_name"
        case packageVersion = "package_version"
        case sourceKind = "source_kind"
        case readOnlyReason = "read_only_reason"
    }
}

enum LLMAction: String, Codable, CaseIterable, Identifiable, Hashable {
    case analyze
    case recommend
    case explainConflict = "explain_conflict"
    case draftFrontmatter = "draft_frontmatter"

    var id: String { rawValue }
}

struct LLMStatus: Codable, Hashable {
    let enabled: Bool
    let provider: String?
    let model: String?
    let disabledReason: String?
    let supportedActions: [LLMAction]

    enum CodingKeys: String, CodingKey {
        case enabled
        case provider
        case model
        case disabledReason = "disabled_reason"
        case reason
        case supportedActions = "supported_actions"
    }

    init(
        enabled: Bool,
        provider: String?,
        model: String?,
        disabledReason: String?,
        supportedActions: [LLMAction]
    ) {
        self.enabled = enabled
        self.provider = provider
        self.model = model
        self.disabledReason = disabledReason
        self.supportedActions = supportedActions
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        enabled = try container.decode(Bool.self, forKey: .enabled)
        provider = try container.decodeIfPresent(String.self, forKey: .provider)
        model = try container.decodeIfPresent(String.self, forKey: .model)
        disabledReason = try container.decodeIfPresent(String.self, forKey: .disabledReason)
            ?? container.decodeIfPresent(String.self, forKey: .reason)
        supportedActions = try container.decodeIfPresent([LLMAction].self, forKey: .supportedActions)
            ?? LLMAction.allCases
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(enabled, forKey: .enabled)
        try container.encodeIfPresent(provider, forKey: .provider)
        try container.encodeIfPresent(model, forKey: .model)
        try container.encodeIfPresent(disabledReason, forKey: .disabledReason)
        try container.encode(supportedActions, forKey: .supportedActions)
    }

    static func disabledFallback(reason: String = UIStrings.llmDisabledFallback) -> LLMStatus {
        LLMStatus(
            enabled: false,
            provider: nil,
            model: nil,
            disabledReason: reason,
            supportedActions: LLMAction.allCases
        )
    }
}

struct LLMTokenCostEstimate: Codable, Hashable {
    let inputTokens: Int
    let outputTokens: Int
    let totalTokens: Int
    let estimatedCostUSD: Double?

    enum CodingKeys: String, CodingKey {
        case inputTokens = "input_tokens"
        case outputTokens = "output_tokens"
        case totalTokens = "total_tokens"
        case estimatedCostUSD = "estimated_cost_usd"
    }
}

struct LLMPrepareResult: Codable, Identifiable, Hashable {
    let action: LLMAction
    let enabled: Bool
    let disabledReason: String?
    let provider: String?
    let model: String?
    let estimate: LLMTokenCostEstimate?
    let confirmationRequired: Bool

    var id: LLMAction { action }

    enum CodingKeys: String, CodingKey {
        case action
        case enabled
        case allowed
        case disabledReason = "disabled_reason"
        case provider
        case model
        case estimate
        case estimatedInputTokens = "estimated_input_tokens"
        case estimatedOutputTokens = "estimated_output_tokens"
        case estimatedTotalTokens = "estimated_total_tokens"
        case estimatedCostUSD = "estimated_cost_usd"
        case confirmationRequired = "confirmation_required"
        case requiresConfirmation = "requires_confirmation"
    }

    init(
        action: LLMAction,
        enabled: Bool,
        disabledReason: String?,
        provider: String?,
        model: String?,
        estimate: LLMTokenCostEstimate?,
        confirmationRequired: Bool
    ) {
        self.action = action
        self.enabled = enabled
        self.disabledReason = disabledReason
        self.provider = provider
        self.model = model
        self.estimate = estimate
        self.confirmationRequired = confirmationRequired
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        action = try container.decode(LLMAction.self, forKey: .action)
        enabled = try container.decodeIfPresent(Bool.self, forKey: .enabled)
            ?? container.decodeIfPresent(Bool.self, forKey: .allowed)
            ?? false
        disabledReason = try container.decodeIfPresent(String.self, forKey: .disabledReason)
        provider = try container.decodeIfPresent(String.self, forKey: .provider)
        model = try container.decodeIfPresent(String.self, forKey: .model)

        if let nestedEstimate = try container.decodeIfPresent(LLMTokenCostEstimate.self, forKey: .estimate) {
            estimate = nestedEstimate
        } else if
            let input = try container.decodeIfPresent(Int.self, forKey: .estimatedInputTokens),
            let output = try container.decodeIfPresent(Int.self, forKey: .estimatedOutputTokens),
            let total = try container.decodeIfPresent(Int.self, forKey: .estimatedTotalTokens)
        {
            estimate = LLMTokenCostEstimate(
                inputTokens: input,
                outputTokens: output,
                totalTokens: total,
                estimatedCostUSD: try container.decodeIfPresent(Double.self, forKey: .estimatedCostUSD)
            )
        } else {
            estimate = nil
        }

        confirmationRequired = try container.decodeIfPresent(Bool.self, forKey: .confirmationRequired)
            ?? container.decodeIfPresent(Bool.self, forKey: .requiresConfirmation)
            ?? true
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(action, forKey: .action)
        try container.encode(enabled, forKey: .enabled)
        try container.encodeIfPresent(disabledReason, forKey: .disabledReason)
        try container.encodeIfPresent(provider, forKey: .provider)
        try container.encodeIfPresent(model, forKey: .model)
        try container.encodeIfPresent(estimate, forKey: .estimate)
        try container.encode(confirmationRequired, forKey: .confirmationRequired)
    }

    static func disabledFallback(action: LLMAction, reason: String = UIStrings.llmDisabledFallback) -> LLMPrepareResult {
        LLMPrepareResult(
            action: action,
            enabled: false,
            disabledReason: reason,
            provider: nil,
            model: nil,
            estimate: nil,
            confirmationRequired: true
        )
    }
}
struct RuleFindingRecord: Codable, Identifiable, Hashable {
    let id: String
    let instanceId: String?
    let definitionId: String?
    let ruleId: String
    let severity: String
    let effectiveSeverity: String?
    let severityOverride: String?
    let message: String
    let suggestion: String?
    let createdAt: Int64
    let suppressed: Bool
    let suppressionReason: String?
    let suppressionNote: String?
    let ruleTuningUpdatedAt: Int64?
    let triageKey: String
    let triageContext: String
    let triageStatus: String
    let triageNote: String?
    let triageUpdatedAt: Int64?

    enum CodingKeys: String, CodingKey {
        case id
        case instanceId = "instance_id"
        case definitionId = "definition_id"
        case ruleId = "rule_id"
        case severity
        case effectiveSeverity = "effective_severity"
        case severityOverride = "severity_override"
        case message
        case suggestion
        case createdAt = "created_at"
        case suppressed
        case suppressionReason = "suppression_reason"
        case suppressionNote = "suppression_note"
        case ruleTuningUpdatedAt = "rule_tuning_updated_at"
        case triageKey = "triage_key"
        case triageContext = "triage_context"
        case triageStatus = "triage_status"
        case triageNote = "triage_note"
        case triageUpdatedAt = "triage_updated_at"
    }

    init(
        id: String,
        instanceId: String?,
        definitionId: String?,
        ruleId: String,
        severity: String,
        effectiveSeverity: String? = nil,
        severityOverride: String? = nil,
        message: String,
        suggestion: String?,
        createdAt: Int64,
        suppressed: Bool = false,
        suppressionReason: String? = nil,
        suppressionNote: String? = nil,
        ruleTuningUpdatedAt: Int64? = nil,
        triageKey: String? = nil,
        triageContext: String = "",
        triageStatus: String = "open",
        triageNote: String? = nil,
        triageUpdatedAt: Int64? = nil
    ) {
        self.id = id
        self.instanceId = instanceId
        self.definitionId = definitionId
        self.ruleId = ruleId
        self.severity = severity
        self.effectiveSeverity = effectiveSeverity
        self.severityOverride = severityOverride
        self.message = message
        self.suggestion = suggestion
        self.createdAt = createdAt
        self.suppressed = suppressed
        self.suppressionReason = suppressionReason
        self.suppressionNote = suppressionNote
        self.ruleTuningUpdatedAt = ruleTuningUpdatedAt
        self.triageKey = triageKey ?? id
        self.triageContext = triageContext
        self.triageStatus = triageStatus
        self.triageNote = triageNote
        self.triageUpdatedAt = triageUpdatedAt
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        instanceId = try container.decodeIfPresent(String.self, forKey: .instanceId)
        definitionId = try container.decodeIfPresent(String.self, forKey: .definitionId)
        ruleId = try container.decode(String.self, forKey: .ruleId)
        severity = try container.decode(String.self, forKey: .severity)
        effectiveSeverity = try container.decodeIfPresent(String.self, forKey: .effectiveSeverity)
        severityOverride = try container.decodeIfPresent(String.self, forKey: .severityOverride)
        message = try container.decode(String.self, forKey: .message)
        suggestion = try container.decodeIfPresent(String.self, forKey: .suggestion)
        createdAt = try container.decode(Int64.self, forKey: .createdAt)
        suppressed = try container.decodeIfPresent(Bool.self, forKey: .suppressed) ?? false
        suppressionReason = try container.decodeIfPresent(String.self, forKey: .suppressionReason)
        suppressionNote = try container.decodeIfPresent(String.self, forKey: .suppressionNote)
        ruleTuningUpdatedAt = try container.decodeIfPresent(Int64.self, forKey: .ruleTuningUpdatedAt)
        triageKey = try container.decodeIfPresent(String.self, forKey: .triageKey) ?? id
        triageContext = try container.decodeIfPresent(String.self, forKey: .triageContext) ?? ""
        triageStatus = try container.decodeIfPresent(String.self, forKey: .triageStatus) ?? "open"
        triageNote = try container.decodeIfPresent(String.self, forKey: .triageNote)
        triageUpdatedAt = try container.decodeIfPresent(Int64.self, forKey: .triageUpdatedAt)
    }
}

struct ConflictGroupRecord: Codable, Identifiable, Hashable {
    let id: String
    let definitionId: String
    let reason: String
    let winnerId: String?
    let instanceIds: [String]

    enum CodingKeys: String, CodingKey {
        case id
        case definitionId = "definition_id"
        case reason
        case winnerId = "winner_id"
        case instanceIds = "instance_ids"
    }
}

struct SkillHealthSummary: Codable, Hashable {
    let totalCount: Int
    let enabledCount: Int
    let disabledCount: Int
    let brokenCount: Int
    let missingCount: Int
    let malformedCount: Int
    let findingCount: Int
    let conflictCount: Int
    let riskyScriptCount: Int
    let riskyPermissionCount: Int
    let findingsBySeverity: HealthSeverityCounts
    let analysisGroups: HealthAnalysisGroupCounts
    let agentSummaries: [AgentSkillHealthSummary]

    enum CodingKeys: String, CodingKey {
        case totalCount = "total_count"
        case enabledCount = "enabled_count"
        case disabledCount = "disabled_count"
        case brokenCount = "broken_count"
        case missingCount = "missing_count"
        case malformedCount = "malformed_count"
        case findingCount = "finding_count"
        case conflictCount = "conflict_count"
        case riskyScriptCount = "risky_script_count"
        case riskyPermissionCount = "risky_permission_count"
        case findingsBySeverity = "findings_by_severity"
        case analysisGroups = "analysis_groups"
        case agentSummaries = "agent_summaries"
    }

    var riskCount: Int {
        riskyScriptCount + riskyPermissionCount
    }

    static let empty = SkillHealthSummary(
        totalCount: 0,
        enabledCount: 0,
        disabledCount: 0,
        brokenCount: 0,
        missingCount: 0,
        malformedCount: 0,
        findingCount: 0,
        conflictCount: 0,
        riskyScriptCount: 0,
        riskyPermissionCount: 0,
        findingsBySeverity: .empty,
        analysisGroups: .empty,
        agentSummaries: []
    )
}

struct HealthSeverityCounts: Codable, Hashable {
    let errorCount: Int
    let warningCount: Int
    let infoCount: Int

    enum CodingKeys: String, CodingKey {
        case errorCount = "error_count"
        case warningCount = "warning_count"
        case infoCount = "info_count"
    }

    static let empty = HealthSeverityCounts(errorCount: 0, warningCount: 0, infoCount: 0)
}

struct HealthAnalysisGroupCounts: Codable, Hashable {
    let totalCount: Int
    let errorCount: Int
    let warningCount: Int
    let infoCount: Int
    let duplicateNameCount: Int
    let canonicalNameCount: Int
    let pathOverlapCount: Int
    let enabledMismatchCount: Int
    let malformedCount: Int
    let precedenceCount: Int

    enum CodingKeys: String, CodingKey {
        case totalCount = "total_count"
        case errorCount = "error_count"
        case warningCount = "warning_count"
        case infoCount = "info_count"
        case duplicateNameCount = "duplicate_name_count"
        case canonicalNameCount = "canonical_name_count"
        case pathOverlapCount = "path_overlap_count"
        case enabledMismatchCount = "enabled_mismatch_count"
        case malformedCount = "malformed_count"
        case precedenceCount = "precedence_count"
    }

    static let empty = HealthAnalysisGroupCounts(
        totalCount: 0,
        errorCount: 0,
        warningCount: 0,
        infoCount: 0,
        duplicateNameCount: 0,
        canonicalNameCount: 0,
        pathOverlapCount: 0,
        enabledMismatchCount: 0,
        malformedCount: 0,
        precedenceCount: 0
    )
}

struct AgentSkillHealthSummary: Codable, Identifiable, Hashable {
    let agent: String
    let totalCount: Int
    let enabledCount: Int
    let disabledCount: Int
    let brokenCount: Int
    let missingCount: Int
    let malformedCount: Int
    let findingCount: Int
    let conflictCount: Int
    let riskyScriptCount: Int
    let riskyPermissionCount: Int
    let analysisGroupCount: Int

    var id: String { agent }

    enum CodingKeys: String, CodingKey {
        case agent
        case totalCount = "total_count"
        case enabledCount = "enabled_count"
        case disabledCount = "disabled_count"
        case brokenCount = "broken_count"
        case missingCount = "missing_count"
        case malformedCount = "malformed_count"
        case findingCount = "finding_count"
        case conflictCount = "conflict_count"
        case riskyScriptCount = "risky_script_count"
        case riskyPermissionCount = "risky_permission_count"
        case analysisGroupCount = "analysis_group_count"
    }

    var riskCount: Int {
        riskyScriptCount + riskyPermissionCount
    }
}

struct ConfigSnapshotRecord: Codable, Identifiable, Hashable {
    let id: String
    let agent: String
    let scope: String
    let projectRoot: String?
    let target: String
    let content: String
    let reason: String
    let createdAt: Int64

    enum CodingKeys: String, CodingKey {
        case id
        case agent
        case scope
        case projectRoot = "project_root"
        case target
        case content
        case reason
        case createdAt = "created_at"
    }

    init(
        id: String,
        agent: String,
        scope: String,
        projectRoot: String? = nil,
        target: String,
        content: String,
        reason: String,
        createdAt: Int64
    ) {
        self.id = id
        self.agent = agent
        self.scope = scope
        self.projectRoot = projectRoot
        self.target = target
        self.content = content
        self.reason = reason
        self.createdAt = createdAt
    }
}

struct SkillEventRecord: Codable, Identifiable, Hashable {
    let id: Int64
    let instanceId: String
    let kind: String
    let payload: JSONValue
    let occurredAt: Int64

    var isToggleActivity: Bool {
        let normalized = kind.lowercased()
        return normalized.contains("toggle")
            || normalized.contains("enable")
            || normalized.contains("disable")
    }

    enum CodingKeys: String, CodingKey {
        case id
        case instanceId = "instance_id"
        case kind
        case payload
        case occurredAt = "occurred_at"
    }
}

struct ListPageWireResult<Record: Codable & Hashable>: Codable, Hashable {
    let records: [Record]
    let sourceRevision: String
    let returnedCount: Int
    let totalCount: Int?
    let hasMore: Bool
    let nextCursor: String?
    let sourceCompleteness: ListSourceCompleteness
    let incompleteReason: ListIncompleteReason?

    var page: ListPage<Record> {
        ListPage(
            items: records,
            returnedCount: returnedCount,
            totalCount: totalCount,
            hasMore: hasMore,
            nextCursor: nextCursor,
            sourceRevision: sourceRevision,
            sourceCompleteness: sourceCompleteness,
            incompleteReason: incompleteReason
        )
    }

    private enum CodingKeys: String, CodingKey {
        case records
        case sourceRevisionSnake = "source_revision"
        case sourceRevisionCamel = "sourceRevision"
        case returnedCountSnake = "returned_count"
        case returnedCountCamel = "returnedCount"
        case totalCountSnake = "total_count"
        case totalCountCamel = "totalCount"
        case hasMoreSnake = "has_more"
        case hasMoreCamel = "hasMore"
        case nextCursorSnake = "next_cursor"
        case nextCursorCamel = "nextCursor"
        case sourceCompletenessSnake = "source_completeness"
        case sourceCompletenessCamel = "sourceCompleteness"
        case incompleteReasonSnake = "incomplete_reason"
        case incompleteReasonCamel = "incompleteReason"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        records = try container.decode([Record].self, forKey: .records)
        sourceRevision = try Self.decode(String.self, from: container, snake: .sourceRevisionSnake, camel: .sourceRevisionCamel)
        returnedCount = try Self.decode(Int.self, from: container, snake: .returnedCountSnake, camel: .returnedCountCamel)
        totalCount = try Self.decodeOptional(Int.self, from: container, snake: .totalCountSnake, camel: .totalCountCamel)
        hasMore = try Self.decode(Bool.self, from: container, snake: .hasMoreSnake, camel: .hasMoreCamel)
        nextCursor = try Self.decodeOptional(String.self, from: container, snake: .nextCursorSnake, camel: .nextCursorCamel)
        sourceCompleteness = try Self.decode(ListSourceCompleteness.self, from: container, snake: .sourceCompletenessSnake, camel: .sourceCompletenessCamel)
        incompleteReason = try Self.decodeOptional(ListIncompleteReason.self, from: container, snake: .incompleteReasonSnake, camel: .incompleteReasonCamel)
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(records, forKey: .records)
        try container.encode(sourceRevision, forKey: .sourceRevisionSnake)
        try container.encode(returnedCount, forKey: .returnedCountSnake)
        try container.encodeIfPresent(totalCount, forKey: .totalCountSnake)
        try container.encode(hasMore, forKey: .hasMoreSnake)
        try container.encodeIfPresent(nextCursor, forKey: .nextCursorSnake)
        try container.encode(sourceCompleteness, forKey: .sourceCompletenessSnake)
        try container.encodeIfPresent(incompleteReason, forKey: .incompleteReasonSnake)
    }

    private static func decode<T: Decodable>(
        _ type: T.Type,
        from container: KeyedDecodingContainer<CodingKeys>,
        snake: CodingKeys,
        camel: CodingKeys
    ) throws -> T {
        if let value = try container.decodeIfPresent(T.self, forKey: snake) { return value }
        return try container.decode(T.self, forKey: camel)
    }

    private static func decodeOptional<T: Decodable>(
        _ type: T.Type,
        from container: KeyedDecodingContainer<CodingKeys>,
        snake: CodingKeys,
        camel: CodingKeys
    ) throws -> T? {
        if container.contains(snake) { return try container.decodeIfPresent(T.self, forKey: snake) }
        return try container.decodeIfPresent(T.self, forKey: camel)
    }
}

typealias ConfigSnapshotPageResult = ListPageWireResult<ConfigSnapshotRecord>
typealias SkillEventPageResult = ListPageWireResult<SkillEventRecord>

struct AdapterCapabilityRecord: Codable, Identifiable, Hashable {
    let agent: String
    let displayName: String
    let status: String
    let scan: AdapterFeatureCapability
    let projectScan: AdapterFeatureCapability
    let configToggle: AdapterFeatureCapability
    let configSnapshot: AdapterFeatureCapability
    let install: AdapterFeatureCapability
    let writable: AdapterFeatureCapability
    let blockers: [String]

    var id: String { agent }

    enum CodingKeys: String, CodingKey {
        case agent
        case displayName = "display_name"
        case status
        case scan
        case projectScan = "project_scan"
        case configToggle = "config_toggle"
        case configSnapshot = "config_snapshot"
        case install
        case writable
        case blockers
    }
}

struct AdapterFeatureCapability: Codable, Hashable {
    let supported: Bool
    let status: String
    let reason: String?
}

struct ScanResult: Codable, Hashable {
    let scannedCount: Int
    let skills: [SkillRecord]
    let activity: RefreshActivity?

    enum CodingKeys: String, CodingKey {
        case scannedCount = "scanned_count"
        case skills
        case activity
    }
}

struct RefreshActivity: Codable, Hashable {
    let operation: String
    let status: String
    let startedAt: Int64
    let finishedAt: Int64
    let scannedCount: Int
    let skillCount: Int
    let findingCount: Int
    let conflictCount: Int
    let snapshotCount: Int
    let roots: [String]
    let logEntries: [RefreshLogEntry]
    let recoveryActions: [String]
    let agentSummaries: [AgentRefreshSummary]?

    enum CodingKeys: String, CodingKey {
        case operation
        case status
        case startedAt = "started_at"
        case finishedAt = "finished_at"
        case scannedCount = "scanned_count"
        case skillCount = "skill_count"
        case findingCount = "finding_count"
        case conflictCount = "conflict_count"
        case snapshotCount = "snapshot_count"
        case roots
        case logEntries = "log_entries"
        case recoveryActions = "recovery_actions"
        case agentSummaries = "agent_summaries"
    }
}

struct RefreshLogEntry: Codable, Hashable, Identifiable {
    let level: String
    let message: String

    var id: String { "\(level):\(message)" }
}

struct AgentRefreshScanIssue: Codable, Hashable, Identifiable {
    let kind: String
    let path: String
    let detail: String

    var id: String { "\(kind):\(path):\(detail)" }
}

struct AgentRefreshSummary: Codable, Hashable, Identifiable {
    let agent: String
    let displayLabel: String
    let status: String
    let scannedCount: Int
    let catalogCount: Int
    let brokenCount: Int
    let rootsConsidered: [String]
    let rootsScanned: [String]
    let rootsPartial: [String]
    let rootsSkipped: [String]
    let scanIssues: [AgentRefreshScanIssue]
    let recoveryActions: [String]

    var id: String { agent }

    var primaryPartialIssue: AgentRefreshScanIssue? {
        let partialKinds: Set<String> = [
            "directory_unreadable", "entry_unreadable", "file_unreadable",
            "file_too_large", "budget_exceeded",
        ]
        return scanIssues.first { partialKinds.contains($0.kind) } ?? scanIssues.first
    }

    var primaryCatalogIncompleteIssue: AgentRefreshScanIssue? {
        if !rootsPartial.isEmpty {
            return primaryPartialIssue
        }
        if !rootsSkipped.isEmpty {
            let skippedKinds: Set<String> = ["root_unavailable", "root_outside_allowlist"]
            return scanIssues.first { skippedKinds.contains($0.kind) } ?? scanIssues.first
        }
        return scanIssues.first
    }

    var provesCatalogCompleteness: Bool {
        rootsPartial.isEmpty
            && rootsSkipped.isEmpty
            && ["completed", "completed-no-roots-scanned"].contains(status)
    }

    var catalogIncompleteReason: ListIncompleteReason {
        if scanIssues.contains(where: { $0.kind == "budget_exceeded" }) {
            return .safetyBudget
        }
        let unreadableKinds: Set<String> = [
            "directory_unreadable", "entry_unreadable", "file_unreadable",
        ]
        if !rootsPartial.isEmpty,
           scanIssues.contains(where: { unreadableKinds.contains($0.kind) }) {
            return .unreadableSource
        }
        return .sourceLimited
    }

    enum CodingKeys: String, CodingKey {
        case agent
        case displayLabel = "display_label"
        case status
        case scannedCount = "scanned_count"
        case catalogCount = "catalog_count"
        case brokenCount = "broken_count"
        case rootsConsidered = "roots_considered"
        case rootsScanned = "roots_scanned"
        case rootsPartial = "roots_partial"
        case rootsSkipped = "roots_skipped"
        case scanIssues = "scan_issues"
        case recoveryActions = "recovery_actions"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        agent = try container.decode(String.self, forKey: .agent)
        displayLabel = try container.decode(String.self, forKey: .displayLabel)
        status = try container.decode(String.self, forKey: .status)
        scannedCount = try container.decode(Int.self, forKey: .scannedCount)
        catalogCount = try container.decode(Int.self, forKey: .catalogCount)
        brokenCount = try container.decode(Int.self, forKey: .brokenCount)
        rootsConsidered = try container.decode([String].self, forKey: .rootsConsidered)
        rootsScanned = try container.decode([String].self, forKey: .rootsScanned)
        rootsPartial = try container.decodeIfPresent([String].self, forKey: .rootsPartial) ?? []
        rootsSkipped = try container.decode([String].self, forKey: .rootsSkipped)
        scanIssues = try container.decodeIfPresent([AgentRefreshScanIssue].self, forKey: .scanIssues) ?? []
        recoveryActions = try container.decode([String].self, forKey: .recoveryActions)
    }
}

struct ConfigDocumentRecord: Codable, Hashable {
    let agent: String
    let scope: String
    let target: String
    let format: String
    let content: String
    let exists: Bool
    let revision: String?

    var supportsCompareAndSwap: Bool {
        revision?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
    }
}

struct ServiceStatus: Codable, Hashable {
    let protocolVersion: Int
    let version: String
    let appDataDir: String
    let catalogPath: String
    let userHome: String
    let supportedMethods: [String]
    let refresh: RefreshStatus?
    let adapterCapabilities: [AdapterCapabilityRecord]?

    enum CodingKeys: String, CodingKey {
        case protocolVersion = "protocol_version"
        case version
        case appDataDir = "app_data_dir"
        case catalogPath = "catalog_path"
        case userHome = "user_home"
        case supportedMethods = "supported_methods"
        case refresh
        case adapterCapabilities = "adapter_capabilities"
    }
}

struct RefreshStatus: Codable, Hashable {
    let scanProgress: String
    let watcherState: String
    let watcherDetail: String
    let recoveryActions: [String]

    enum CodingKeys: String, CodingKey {
        case scanProgress = "scan_progress"
        case watcherState = "watcher_state"
        case watcherDetail = "watcher_detail"
        case recoveryActions = "recovery_actions"
    }
}

enum JSONValue: Codable, Hashable {
    case string(String)
    case number(Double)
    case bool(Bool)
    case object([String: JSONValue])
    case array([JSONValue])
    case null

    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            self = .null
        } else if let value = try? container.decode(Bool.self) {
            self = .bool(value)
        } else if let value = try? container.decode(Double.self) {
            self = .number(value)
        } else if let value = try? container.decode(String.self) {
            self = .string(value)
        } else if let value = try? container.decode([JSONValue].self) {
            self = .array(value)
        } else {
            self = .object(try container.decode([String: JSONValue].self))
        }
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .string(let value):
            try container.encode(value)
        case .number(let value):
            try container.encode(value)
        case .bool(let value):
            try container.encode(value)
        case .object(let value):
            try container.encode(value)
        case .array(let value):
            try container.encode(value)
        case .null:
            try container.encodeNil()
        }
    }
}
