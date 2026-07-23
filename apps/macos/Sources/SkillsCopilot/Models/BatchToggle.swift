import Foundation

enum BatchToggleAction: String, CaseIterable, Codable, Identifiable, Hashable {
    case enable
    case disable

    var id: String { rawValue }

    var targetEnabled: Bool { self == .enable }

    var title: String {
        switch self {
        case .enable:
            return UIStrings.enable
        case .disable:
            return UIStrings.disable
        }
    }

    var systemImage: String {
        switch self {
        case .enable:
            return "play.circle"
        case .disable:
            return "pause.circle"
        }
    }

    static func from(targetEnabled: Bool) -> BatchToggleAction {
        targetEnabled ? .enable : .disable
    }
}

struct BatchToggleSkillItem: Decodable, Identifiable, Hashable {
    let instanceID: String
    let name: String
    let agent: String
    let scope: String
    let displayPath: String
    let currentEnabled: Bool?
    let targetEnabled: Bool?
    let reason: String?

    var id: String { instanceID }

    init(
        instanceID: String,
        name: String,
        agent: String,
        scope: String,
        displayPath: String,
        currentEnabled: Bool? = nil,
        targetEnabled: Bool? = nil,
        reason: String? = nil
    ) {
        self.instanceID = instanceID
        self.name = name
        self.agent = agent
        self.scope = scope
        self.displayPath = displayPath
        self.currentEnabled = currentEnabled
        self.targetEnabled = targetEnabled
        self.reason = reason
    }

    init(skill: SkillRecord, targetEnabled: Bool, reason: String? = nil) {
        self.init(
            instanceID: skill.id,
            name: skill.name,
            agent: skill.agent,
            scope: skill.scope,
            displayPath: skill.displayPath,
            currentEnabled: skill.enabled,
            targetEnabled: targetEnabled,
            reason: reason
        )
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: FlexibleCodingKey.self)
        instanceID = container.decodeString(for: ["instance_id", "instanceId", "id", "skill_id"]) ?? ""
        name = container.decodeString(for: ["name", "skill_name", "skillName"]) ?? instanceID
        agent = container.decodeString(for: ["agent", "adapter"]) ?? UIStrings.unknown
        scope = container.decodeString(for: ["scope", "skill_scope", "skillScope"]) ?? ""
        displayPath = container.decodeString(for: ["display_path", "displayPath", "path"]) ?? ""
        currentEnabled = container.decodeBool(for: ["current_enabled", "currentEnabled", "enabled"])
        targetEnabled = container.decodeBool(for: ["target_enabled", "targetEnabled", "on"])
        reason = container.decodeString(for: ["reason", "skip_reason", "skipReason", "disabled_reason"])
    }
}

struct BatchToggleSnapshotPlan: Decodable, Hashable {
    let summary: String
    let rollbackSupported: Bool
    let targets: [String]

    static let localUnavailable = BatchToggleSnapshotPlan(
        summary: UIStrings.batchToggleSnapshotPlanUnavailable,
        rollbackSupported: false,
        targets: []
    )

    init(summary: String, rollbackSupported: Bool, targets: [String]) {
        self.summary = summary
        self.rollbackSupported = rollbackSupported
        self.targets = targets
    }

    init(from decoder: Decoder) throws {
        if let text = try? decoder.singleValueContainer().decode(String.self) {
            summary = text
            rollbackSupported = false
            targets = []
            return
        }
        let container = try decoder.container(keyedBy: FlexibleCodingKey.self)
        summary = container.decodeString(for: ["summary", "description", "rollback_plan", "rollbackPlan"])
            ?? UIStrings.batchToggleSnapshotPlanDefault
        rollbackSupported = container.decodeBool(for: ["rollback_supported", "rollbackSupported", "can_rollback"])
            ?? true
        targets = container.decodeStringArray(for: ["targets", "snapshot_targets", "snapshotTargets", "files"])
    }
}

struct BatchTogglePreview: Decodable, Identifiable, Hashable {
    let id: String
    let action: BatchToggleAction
    let actionDescriptor: ActionDescriptorWire?
    let previewToken: String?
    let targetEnabled: Bool
    let selectedCount: Int
    let writableCount: Int
    let skippedCount: Int
    let affectedSkills: [BatchToggleSkillItem]
    let skippedItems: [BatchToggleSkillItem]
    let snapshotPlan: BatchToggleSnapshotPlan
    let applySupported: Bool

    var hasWritableChanges: Bool {
        writableCount > 0 && !affectedSkills.isEmpty
    }

    init(
        id: String,
        action: BatchToggleAction,
        selectedCount: Int,
        affectedSkills: [BatchToggleSkillItem],
        skippedItems: [BatchToggleSkillItem],
        snapshotPlan: BatchToggleSnapshotPlan,
        applySupported: Bool,
        actionDescriptor: ActionDescriptorWire? = nil,
        previewToken: String? = nil
    ) {
        self.id = id
        self.action = action
        self.actionDescriptor = actionDescriptor
        self.previewToken = previewToken
        self.targetEnabled = action.targetEnabled
        self.selectedCount = selectedCount
        self.writableCount = affectedSkills.count
        self.skippedCount = skippedItems.count
        self.affectedSkills = affectedSkills
        self.skippedItems = skippedItems
        self.snapshotPlan = snapshotPlan
        self.applySupported = applySupported
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: FlexibleCodingKey.self)
        let actionKey = FlexibleCodingKey(stringValue: "action")!
        actionDescriptor = try? container.decodeIfPresent(
            ActionDescriptorWire.self,
            forKey: actionKey
        )
        let decodedTarget = container.decodeBool(for: ["target_enabled", "targetEnabled", "on", "enabled"])
        let decodedAction = container.decodeString(for: ["action", "target", "operation"])
            .flatMap { BatchToggleAction(rawValue: $0.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()) }
            ?? decodedTarget.map(BatchToggleAction.from(targetEnabled:))
            ?? .disable
        action = decodedAction
        targetEnabled = decodedTarget ?? decodedAction.targetEnabled
        previewToken = container.decodeString(for: ["preview_token", "previewToken"])
        id = actionDescriptor?.id
            ?? container.decodeString(for: ["preview_id", "previewId", "batch_id", "batchId", "id"])
            ?? "batch-\(decodedAction.rawValue)-preview"
        affectedSkills = container.decodeSkillItems(for: ["affected_items", "affectedItems", "affected_skills", "affectedSkills", "writable_skills", "writableSkills", "changes"])
        skippedItems = container.decodeSkillItems(for: ["skipped_items", "skippedItems", "skipped", "read_only_items", "readOnlyItems", "excluded"])
        selectedCount = container.decodeInt(for: ["requested_count", "requestedCount", "selected_count", "selectedCount", "total_count", "totalCount"])
            ?? affectedSkills.count + skippedItems.count
        writableCount = container.decodeInt(for: ["writable_count", "writableCount", "eligible_count", "eligibleCount", "affected_count", "affectedCount"])
            ?? affectedSkills.count
        skippedCount = container.decodeInt(for: ["skipped_count", "skippedCount", "excluded_count", "excludedCount"])
            ?? skippedItems.count
        let notes = container.decodeStringArray(for: ["snapshot_rollback_notes", "snapshotRollbackNotes", "rollback_notes", "rollbackNotes"])
        snapshotPlan = container.decodeSnapshotPlan(for: ["snapshot_plan", "snapshotPlan", "rollback_plan", "rollbackPlan"])
            ?? BatchToggleSnapshotPlan(
                summary: notes.isEmpty ? UIStrings.batchToggleSnapshotPlanDefault : notes.joined(separator: "\n"),
                rollbackSupported: true,
                targets: []
            )
        applySupported = container.decodeBool(for: ["writes_allowed", "writesAllowed", "apply_supported", "applySupported", "can_apply", "canApply"])
            ?? true
        if applySupported,
           (actionDescriptor == nil
               || previewToken?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty != false) {
            throw DecodingError.dataCorruptedError(
                forKey: actionKey,
                in: container,
                debugDescription: "A writable batch preview requires a typed action and opaque preview token."
            )
        }
    }

    static func local(
        action: BatchToggleAction,
        selectedSkills: [SkillRecord],
        affectedSkills: [BatchToggleSkillItem],
        skippedItems: [BatchToggleSkillItem],
        reason: String
    ) -> BatchTogglePreview {
        BatchTogglePreview(
            id: "local-\(action.rawValue)-preview",
            action: action,
            selectedCount: selectedSkills.count,
            affectedSkills: affectedSkills,
            skippedItems: skippedItems,
            snapshotPlan: BatchToggleSnapshotPlan(summary: reason, rollbackSupported: false, targets: []),
            applySupported: false
        )
    }
}

struct BatchToggleApplyResult: Decodable, Hashable {
    let updatedCount: Int
    let skippedCount: Int
    let snapshotIDs: [String]

    init(updatedCount: Int, skippedCount: Int, snapshotIDs: [String] = []) {
        self.updatedCount = updatedCount
        self.skippedCount = skippedCount
        self.snapshotIDs = snapshotIDs
    }

    init(from decoder: Decoder) throws {
        if let records = try? decoder.singleValueContainer().decode([SkillRecord].self) {
            updatedCount = records.count
            skippedCount = 0
            snapshotIDs = []
            return
        }
        let container = try decoder.container(keyedBy: FlexibleCodingKey.self)
        updatedCount = container.decodeInt(for: ["updated_count", "updatedCount", "applied_count", "appliedCount", "writable_count", "writableCount"]) ?? 0
        skippedCount = container.decodeInt(for: ["skipped_count", "skippedCount"]) ?? 0
        snapshotIDs = container.decodeStringArray(for: ["snapshot_ids", "snapshotIds", "snapshots"])
    }
}

struct FlexibleCodingKey: CodingKey {
    let stringValue: String
    let intValue: Int?

    init?(stringValue: String) {
        self.stringValue = stringValue
        intValue = nil
    }

    init?(intValue: Int) {
        self.intValue = intValue
        stringValue = "\(intValue)"
    }
}

extension KeyedDecodingContainer where Key == FlexibleCodingKey {
    func decodeString(for aliases: [String]) -> String? {
        for alias in aliases {
            guard let key = FlexibleCodingKey(stringValue: alias) else { continue }
            if let value = try? decodeIfPresent(String.self, forKey: key) {
                return value
            }
        }
        return nil
    }

    func decodeBool(for aliases: [String]) -> Bool? {
        for alias in aliases {
            guard let key = FlexibleCodingKey(stringValue: alias) else { continue }
            if let value = try? decodeIfPresent(Bool.self, forKey: key) {
                return value
            }
            if let string = try? decodeIfPresent(String.self, forKey: key) {
                switch string.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
                case "true", "yes", "1":
                    return true
                case "false", "no", "0":
                    return false
                default:
                    break
                }
            }
        }
        return nil
    }

    func decodeInt(for aliases: [String]) -> Int? {
        for alias in aliases {
            guard let key = FlexibleCodingKey(stringValue: alias) else { continue }
            if let value = try? decodeIfPresent(Int.self, forKey: key) {
                return value
            }
            if let string = try? decodeIfPresent(String.self, forKey: key), let value = Int(string) {
                return value
            }
        }
        return nil
    }

    func decodeStringArray(for aliases: [String]) -> [String] {
        for alias in aliases {
            guard let key = FlexibleCodingKey(stringValue: alias) else { continue }
            if let values = try? decodeIfPresent([String].self, forKey: key) {
                return values
            }
            if let value = try? decodeIfPresent(String.self, forKey: key), !value.isEmpty {
                return [value]
            }
        }
        return []
    }

    func decodeSkillItems(for aliases: [String]) -> [BatchToggleSkillItem] {
        for alias in aliases {
            guard let key = FlexibleCodingKey(stringValue: alias) else { continue }
            if let values = try? decodeIfPresent([BatchToggleSkillItem].self, forKey: key) {
                return values
            }
        }
        return []
    }

    func decodeSnapshotPlan(for aliases: [String]) -> BatchToggleSnapshotPlan? {
        for alias in aliases {
            guard let key = FlexibleCodingKey(stringValue: alias) else { continue }
            if let value = try? decodeIfPresent(BatchToggleSnapshotPlan.self, forKey: key) {
                return value
            }
        }
        return nil
    }
}
