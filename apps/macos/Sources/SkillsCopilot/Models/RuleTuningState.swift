import Foundation

enum RuleTuningScope: String, Codable, CaseIterable, Identifiable, Hashable {
    case rule
    case findingGroup = "finding-group"

    var id: String { rawValue }

    static func fromService(_ value: String?) -> RuleTuningScope {
        let normalized = (value ?? "")
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
            .replacingOccurrences(of: "_", with: "-")
        switch normalized {
        case "finding-group", "findinggroup", "group":
            return .findingGroup
        default:
            return .rule
        }
    }
}

struct RuleTuningRecord: Decodable, Identifiable, Hashable {
    let ruleId: String
    let scope: RuleTuningScope
    let findingGroupId: String?
    let defaultSeverity: String?
    let severityOverride: String?
    let effectiveSeverity: String
    let suppressed: Bool
    let suppressionReason: String?
    let updatedAt: Int64?

    var id: String {
        [scope.rawValue, ruleId, findingGroupId ?? ""].joined(separator: ":")
    }

    enum CodingKeys: String, CodingKey {
        case id
        case ruleId = "rule_id"
        case ruleID = "ruleID"
        case scope
        case findingGroupId = "finding_group_id"
        case findingGroupID = "findingGroupID"
        case groupId = "group_id"
        case groupID = "groupID"
        case defaultSeverity = "default_severity"
        case baseSeverity = "base_severity"
        case severity
        case severityOverride = "severity_override"
        case overrideSeverity = "override_severity"
        case effectiveSeverity = "effective_severity"
        case suppressed
        case isSuppressed = "is_suppressed"
        case suppressionReason = "suppression_reason"
        case note
        case updatedAt = "updated_at"
    }

    init(
        ruleId: String,
        scope: RuleTuningScope = .rule,
        findingGroupId: String? = nil,
        defaultSeverity: String? = nil,
        severityOverride: String? = nil,
        effectiveSeverity: String? = nil,
        suppressed: Bool = false,
        suppressionReason: String? = nil,
        updatedAt: Int64? = nil
    ) {
        self.ruleId = ruleId
        self.scope = scope
        self.findingGroupId = findingGroupId
        self.defaultSeverity = RuleTuningModel.normalizedSeverity(defaultSeverity)
        self.severityOverride = RuleTuningModel.normalizedSeverity(severityOverride)
        self.effectiveSeverity = RuleTuningModel.normalizedSeverity(effectiveSeverity)
            ?? RuleTuningModel.normalizedSeverity(severityOverride)
            ?? RuleTuningModel.normalizedSeverity(defaultSeverity)
            ?? "unknown"
        self.suppressed = suppressed
        self.suppressionReason = suppressionReason
        self.updatedAt = updatedAt
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let decodedRuleId = try container.decodeIfPresent(String.self, forKey: .ruleId)
            ?? container.decodeIfPresent(String.self, forKey: .ruleID)
            ?? container.decodeIfPresent(String.self, forKey: .id)
            ?? ""
        let decodedScope = RuleTuningScope.fromService(try container.decodeIfPresent(String.self, forKey: .scope))
        let decodedGroupId = try container.decodeIfPresent(String.self, forKey: .findingGroupId)
            ?? container.decodeIfPresent(String.self, forKey: .findingGroupID)
            ?? container.decodeIfPresent(String.self, forKey: .groupId)
            ?? container.decodeIfPresent(String.self, forKey: .groupID)
        let decodedDefaultSeverity = try container.decodeIfPresent(String.self, forKey: .defaultSeverity)
            ?? container.decodeIfPresent(String.self, forKey: .baseSeverity)
            ?? container.decodeIfPresent(String.self, forKey: .severity)
        let decodedOverride = try container.decodeIfPresent(String.self, forKey: .severityOverride)
            ?? container.decodeIfPresent(String.self, forKey: .overrideSeverity)
        let decodedEffective = try container.decodeIfPresent(String.self, forKey: .effectiveSeverity)
        let decodedReason = try container.decodeIfPresent(String.self, forKey: .suppressionReason)
            ?? container.decodeIfPresent(String.self, forKey: .note)
        let decodedSuppressed = try container.decodeIfPresent(Bool.self, forKey: .suppressed)
            ?? container.decodeIfPresent(Bool.self, forKey: .isSuppressed)
            ?? (decodedReason != nil)
        let decodedUpdatedAt = try container.decodeIfPresent(Int64.self, forKey: .updatedAt)

        self.init(
            ruleId: decodedRuleId,
            scope: decodedScope,
            findingGroupId: decodedGroupId,
            defaultSeverity: decodedDefaultSeverity,
            severityOverride: decodedOverride,
            effectiveSeverity: decodedEffective,
            suppressed: decodedSuppressed,
            suppressionReason: decodedReason,
            updatedAt: decodedUpdatedAt
        )
    }
}

struct RuleTuningList: Decodable, Hashable {
    let records: [RuleTuningRecord]

    enum CodingKeys: String, CodingKey {
        case rules
        case records
        case items
    }

    init(records: [RuleTuningRecord]) {
        self.records = records
    }

    init(from decoder: Decoder) throws {
        if let array = try? [RuleTuningRecord](from: decoder) {
            records = array
            return
        }

        let container = try decoder.container(keyedBy: CodingKeys.self)
        records = try container.decodeIfPresent([RuleTuningRecord].self, forKey: .rules)
            ?? container.decodeIfPresent([RuleTuningRecord].self, forKey: .records)
            ?? container.decodeIfPresent([RuleTuningRecord].self, forKey: .items)
            ?? []
    }
}

enum RuleTuningModel {
    static let overrideSeverities = ["critical", "error", "warning", "info"]

    static func normalizedSeverity(_ severity: String?) -> String? {
        guard let severity else { return nil }
        let normalized = severity
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
        return normalized.isEmpty ? nil : normalized
    }

    static func record(
        in records: [RuleTuningRecord],
        ruleId: String,
        findingGroupId: String? = nil
    ) -> RuleTuningRecord? {
        let normalizedRuleId = normalizedRule(ruleId)
        let normalizedGroupId = findingGroupId?.trimmingCharacters(in: .whitespacesAndNewlines)

        if let normalizedGroupId, !normalizedGroupId.isEmpty {
            return records.first { record in
                normalizedRule(record.ruleId) == normalizedRuleId
                    && record.scope == .findingGroup
                    && record.findingGroupId == normalizedGroupId
            }
        }

        return records.first { record in
            normalizedRule(record.ruleId) == normalizedRuleId && record.scope == .rule
        }
    }

    static func effectiveSeverity(
        records: [RuleTuningRecord],
        ruleId: String,
        findingGroupId: String?,
        fallbackSeverity: String
    ) -> String {
        record(in: records, ruleId: ruleId, findingGroupId: findingGroupId)?.effectiveSeverity
            ?? record(in: records, ruleId: ruleId)?.effectiveSeverity
            ?? normalizedSeverity(fallbackSeverity)
            ?? "unknown"
    }

    static func isSuppressed(
        records: [RuleTuningRecord],
        ruleId: String,
        findingGroupId: String?
    ) -> Bool {
        if record(in: records, ruleId: ruleId, findingGroupId: findingGroupId)?.suppressed == true {
            return true
        }
        return record(in: records, ruleId: ruleId)?.suppressed == true
    }

    private static func normalizedRule(_ ruleId: String) -> String {
        ruleId.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    }
}
