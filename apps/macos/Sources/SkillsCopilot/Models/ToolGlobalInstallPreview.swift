import Foundation

enum ToolInstallTarget: String, Codable, CaseIterable, Identifiable, Hashable {
    case claudeCode = "claude-code"
    case codex
    case opencode
    case pi
    case hermes
    case openclaw

    var id: String { rawValue }

    var title: String {
        DisplayText.agent(rawValue)
    }

    static func supportedTargets(from capabilities: [AdapterCapabilityRecord]) -> [ToolInstallTarget] {
        guard !capabilities.isEmpty else {
            return allCases
        }
        return capabilities
            .filter {
                $0.install.supported
                    && $0.install.status.lowercased().hasPrefix("verified")
            }
            .compactMap { ToolInstallTarget(rawValue: $0.agent) }
    }
}

struct ToolGlobalInstallPreview: Codable, Identifiable, Hashable {
    let skillID: String
    let action: ActionDescriptorWire?
    let previewToken: String?
    let skillName: String
    let sourcePath: String
    let target: ToolInstallTarget
    let targetPath: String?
    let confirmationRequired: Bool
    let writeBackEnabled: Bool
    let wrote: Bool
    let summary: String
    let confirmationMessage: String
    let risks: [String]
    let snapshotID: String?
    let readback: ActionReadbackWire?

    var id: String { "\(skillID):\(target.rawValue)" }

    enum CodingKeys: String, CodingKey {
        case skillID = "skill_id"
        case action
        case previewToken = "preview_token"
        case sourceInstanceID = "source_instance_id"
        case skillName = "skill_name"
        case sourcePath = "source_path"
        case target
        case targetAgent = "target_agent"
        case targetPath = "target_path"
        case confirmationRequired = "confirmation_required"
        case writeBackEnabled = "write_back_enabled"
        case confirmation
        case wrote
        case summary
        case confirmationMessage = "confirmation_message"
        case risks
        case snapshotID = "snapshot_id"
        case readback
    }

    enum ConfirmationKeys: String, CodingKey {
        case required
        case confirmed
        case message
    }

    init(
        skillID: String,
        action: ActionDescriptorWire? = nil,
        previewToken: String? = nil,
        skillName: String,
        sourcePath: String,
        target: ToolInstallTarget,
        targetPath: String?,
        confirmationRequired: Bool,
        writeBackEnabled: Bool,
        wrote: Bool,
        summary: String,
        confirmationMessage: String,
        risks: [String],
        snapshotID: String?,
        readback: ActionReadbackWire? = nil
    ) {
        self.skillID = skillID
        self.action = action
        self.previewToken = previewToken
        self.skillName = skillName
        self.sourcePath = sourcePath
        self.target = target
        self.targetPath = targetPath
        self.confirmationRequired = confirmationRequired
        self.writeBackEnabled = writeBackEnabled
        self.wrote = wrote
        self.summary = summary
        self.confirmationMessage = confirmationMessage
        self.risks = risks
        self.snapshotID = snapshotID
        self.readback = readback
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let skillID = try container.decodeIfPresent(String.self, forKey: .skillID)
            ?? container.decode(String.self, forKey: .sourceInstanceID)
        let action = try container.decodeIfPresent(ActionDescriptorWire.self, forKey: .action)
        let previewToken = try container.decodeIfPresent(String.self, forKey: .previewToken)
        let target = try container.decodeIfPresent(ToolInstallTarget.self, forKey: .target)
            ?? container.decode(ToolInstallTarget.self, forKey: .targetAgent)
        let sourcePath = try container.decode(String.self, forKey: .sourcePath)
        let skillName = try container.decodeIfPresent(String.self, forKey: .skillName)
            ?? URL(fileURLWithPath: sourcePath).deletingLastPathComponent().lastPathComponent
            .nonEmptyFallback(skillID)
        let targetPath = try container.decodeIfPresent(String.self, forKey: .targetPath)
        let wrote = try container.decodeIfPresent(Bool.self, forKey: .wrote) ?? false
        let risks = try container.decodeIfPresent([String].self, forKey: .risks) ?? []
        let snapshotID = try container.decodeIfPresent(String.self, forKey: .snapshotID)
        let readback = try container.decodeIfPresent(ActionReadbackWire.self, forKey: .readback)

        let confirmation = try container.decodeIfPresent(ConfirmationPayload.self, forKey: .confirmation)
        let confirmationRequired = try container.decodeIfPresent(Bool.self, forKey: .confirmationRequired)
            ?? confirmation?.required
            ?? true
        let legacyWriteBack = try container.decodeIfPresent(Bool.self, forKey: .writeBackEnabled)
        let writeBackEnabled = legacyWriteBack ?? (confirmationRequired && !wrote && confirmation != nil)
        if writeBackEnabled,
           (action == nil
               || previewToken?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty != false) {
            throw DecodingError.dataCorruptedError(
                forKey: .action,
                in: container,
                debugDescription: "A writable install preview requires a typed action and opaque preview token."
            )
        }
        let summary = try container.decodeIfPresent(String.self, forKey: .summary)
            ?? UIStrings.toolGlobalInstallPreviewSummary(skillName, target.title)
        let confirmationMessage = try container.decodeIfPresent(String.self, forKey: .confirmationMessage)
            ?? confirmation?.message
            ?? UIStrings.toolGlobalInstallConfirmation(skillName, target.title)

        self.init(
            skillID: skillID,
            action: action,
            previewToken: previewToken,
            skillName: skillName,
            sourcePath: sourcePath,
            target: target,
            targetPath: targetPath,
            confirmationRequired: confirmationRequired,
            writeBackEnabled: writeBackEnabled,
            wrote: wrote,
            summary: summary,
            confirmationMessage: confirmationMessage,
            risks: risks,
            snapshotID: snapshotID,
            readback: readback
        )
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(skillID, forKey: .skillID)
        try container.encodeIfPresent(action, forKey: .action)
        try container.encodeIfPresent(previewToken, forKey: .previewToken)
        try container.encode(skillName, forKey: .skillName)
        try container.encode(sourcePath, forKey: .sourcePath)
        try container.encode(target, forKey: .target)
        try container.encodeIfPresent(targetPath, forKey: .targetPath)
        try container.encode(confirmationRequired, forKey: .confirmationRequired)
        try container.encode(writeBackEnabled, forKey: .writeBackEnabled)
        try container.encode(wrote, forKey: .wrote)
        try container.encode(summary, forKey: .summary)
        try container.encode(confirmationMessage, forKey: .confirmationMessage)
        try container.encode(risks, forKey: .risks)
        try container.encodeIfPresent(snapshotID, forKey: .snapshotID)
        try container.encodeIfPresent(readback, forKey: .readback)
    }

}

private struct ConfirmationPayload: Codable, Hashable {
    let required: Bool
    let confirmed: Bool
    let message: String
}

private extension String {
    func nonEmptyFallback(_ fallback: String) -> String {
        isEmpty ? fallback : self
    }
}
