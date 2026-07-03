import Foundation

enum FindingTriageStatus: String, CaseIterable, Codable, Identifiable, Equatable, Hashable {
    case open
    case reviewed
    case ignored
    case needsFollowUp = "needs-follow-up"

    var id: String { rawValue }

    static func fromService(_ value: String?) -> FindingTriageStatus {
        let normalized = (value ?? "")
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
            .replacingOccurrences(of: "_", with: "-")
        switch normalized {
        case "reviewed":
            return .reviewed
        case "ignored":
            return .ignored
        case "needs-follow-up", "needsfollowup":
            return .needsFollowUp
        default:
            return .open
        }
    }

    var title: String {
        switch self {
        case .open:
            return UIStrings.findingTriageOpen
        case .reviewed:
            return UIStrings.findingTriageReviewed
        case .ignored:
            return UIStrings.findingTriageIgnored
        case .needsFollowUp:
            return UIStrings.findingTriageNeedsFollowUp
        }
    }

    var systemImage: String {
        switch self {
        case .open:
            return "circle"
        case .reviewed:
            return "checkmark.circle"
        case .ignored:
            return "eye.slash"
        case .needsFollowUp:
            return "flag"
        }
    }
}

enum FindingTriageFilter: String, CaseIterable, Identifiable, Equatable, Hashable {
    case active
    case open
    case needsFollowUp
    case reviewed
    case ignored
    case all

    var id: String { rawValue }

    var title: String {
        switch self {
        case .active:
            return UIStrings.findingTriageFilterActive
        case .open:
            return UIStrings.findingTriageOpen
        case .needsFollowUp:
            return UIStrings.findingTriageNeedsFollowUp
        case .reviewed:
            return UIStrings.findingTriageReviewed
        case .ignored:
            return UIStrings.findingTriageIgnored
        case .all:
            return UIStrings.findingTriageFilterAll
        }
    }

    func includes(_ status: FindingTriageStatus) -> Bool {
        switch self {
        case .active:
            return status == .open || status == .needsFollowUp
        case .open:
            return status == .open
        case .needsFollowUp:
            return status == .needsFollowUp
        case .reviewed:
            return status == .reviewed
        case .ignored:
            return status == .ignored
        case .all:
            return true
        }
    }

    static func availableFilters(for counts: FindingTriageCounts) -> [FindingTriageFilter] {
        var filters: [FindingTriageFilter] = []
        if counts.open > 0 {
            filters.append(.open)
        }
        if counts.needsFollowUp > 0 {
            filters.append(.needsFollowUp)
        }
        if counts.reviewed > 0 {
            filters.append(.reviewed)
        }
        if counts.ignored > 0 {
            filters.append(.ignored)
        }
        if filters.count > 1 {
            filters.append(.all)
        }
        return filters.isEmpty ? [.all] : filters
    }
}

struct FindingTriageRecord: Codable, Identifiable, Hashable {
    let triageKey: String
    let triageContext: String
    let status: String
    let note: String?
    let updatedAt: Int64

    var id: String { triageKey }

    var triageStatus: FindingTriageStatus {
        FindingTriageStatus.fromService(status)
    }

    enum CodingKeys: String, CodingKey {
        case triageKey = "triage_key"
        case triageContext = "triage_context"
        case status
        case note
        case updatedAt = "updated_at"
    }
}

struct FindingTriageCounts: Equatable {
    var open = 0
    var reviewed = 0
    var ignored = 0
    var needsFollowUp = 0

    var total: Int {
        open + reviewed + ignored + needsFollowUp
    }

    func count(for status: FindingTriageStatus) -> Int {
        switch status {
        case .open:
            return open
        case .reviewed:
            return reviewed
        case .ignored:
            return ignored
        case .needsFollowUp:
            return needsFollowUp
        }
    }
}

enum FindingTriageModel {
    static func counts(for statuses: [FindingTriageStatus]) -> FindingTriageCounts {
        statuses.reduce(into: FindingTriageCounts()) { counts, status in
            switch status {
            case .open:
                counts.open += 1
            case .reviewed:
                counts.reviewed += 1
            case .ignored:
                counts.ignored += 1
            case .needsFollowUp:
                counts.needsFollowUp += 1
            }
        }
    }

    static func groupStatus(for statuses: [FindingTriageStatus]) -> FindingTriageStatus {
        let unique = Set(statuses)
        guard unique.count != 1 else {
            return unique.first ?? .open
        }
        if unique.contains(.needsFollowUp) {
            return .needsFollowUp
        }
        if unique.contains(.open) {
            return .open
        }
        if unique.contains(.reviewed) {
            return .reviewed
        }
        return .ignored
    }
}

extension RuleFindingRecord {
    var triageState: FindingTriageStatus {
        FindingTriageStatus.fromService(triageStatus)
    }

    func withTriage(status: FindingTriageStatus, note: String? = nil, updatedAt: Int64? = nil) -> RuleFindingRecord {
        RuleFindingRecord(
            id: id,
            instanceId: instanceId,
            definitionId: definitionId,
            ruleId: ruleId,
            severity: severity,
            effectiveSeverity: effectiveSeverity,
            severityOverride: severityOverride,
            message: message,
            suggestion: suggestion,
            createdAt: createdAt,
            suppressed: suppressed,
            suppressionReason: suppressionReason,
            suppressionNote: suppressionNote,
            ruleTuningUpdatedAt: ruleTuningUpdatedAt,
            triageKey: triageKey,
            triageContext: triageContext,
            triageStatus: status.rawValue,
            triageNote: status == .open ? nil : note,
            triageUpdatedAt: status == .open ? nil : updatedAt
        )
    }
}
