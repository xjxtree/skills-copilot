import Foundation

enum AppSearchItemKind: String, CaseIterable, Decodable, Hashable {
    case skill
    case session
    case configHistory = "config_history"

    var title: String {
        switch self {
        case .skill:
            return UIStrings.skills
        case .session:
            return UIStrings.text("sidebar.mode.sessions", "Sessions")
        case .configHistory:
            return UIStrings.agentConfigHistory
        }
    }

    var systemImage: String {
        switch self {
        case .skill:
            return "square.stack.3d.up"
        case .session:
            return "bubble.left.and.text.bubble.right"
        case .configHistory:
            return "clock.arrow.circlepath"
        }
    }
}

struct AppSearchResult: Decodable, Hashable {
    let generatedBy: String
    let query: String
    let count: Int
    let totalMatchedCount: Int
    let limitPerKind: Int
    let items: [AppSearchItem]
    let readOnly: Bool
    let providerRequestSent: Bool
    let skillFilesMutated: Bool
    let agentConfigMutated: Bool
    let rawPromptPersisted: Bool
    let rawResponsePersisted: Bool
    let fallbackReason: String?

    enum CodingKeys: String, CodingKey {
        case generatedBy = "generated_by"
        case generatedByAlt = "generatedBy"
        case query
        case count
        case totalMatchedCount = "total_matched_count"
        case totalMatchedCountAlt = "totalMatchedCount"
        case limitPerKind = "limit_per_kind"
        case limitPerKindAlt = "limitPerKind"
        case items
        case readOnly = "read_only"
        case readOnlyAlt = "readOnly"
        case providerRequestSent = "provider_request_sent"
        case providerRequestSentAlt = "providerRequestSent"
        case skillFilesMutated = "skill_files_mutated"
        case skillFilesMutatedAlt = "skillFilesMutated"
        case agentConfigMutated = "agent_config_mutated"
        case agentConfigMutatedAlt = "agentConfigMutated"
        case rawPromptPersisted = "raw_prompt_persisted"
        case rawPromptPersistedAlt = "rawPromptPersisted"
        case rawResponsePersisted = "raw_response_persisted"
        case rawResponsePersistedAlt = "rawResponsePersisted"
        case fallbackReason = "fallback_reason"
        case reason
    }

    init(
        generatedBy: String = "local-v2.99",
        query: String,
        count: Int? = nil,
        totalMatchedCount: Int? = nil,
        limitPerKind: Int = 0,
        items: [AppSearchItem] = [],
        readOnly: Bool = true,
        providerRequestSent: Bool = false,
        skillFilesMutated: Bool = false,
        agentConfigMutated: Bool = false,
        rawPromptPersisted: Bool = false,
        rawResponsePersisted: Bool = false,
        fallbackReason: String? = nil
    ) {
        self.generatedBy = generatedBy
        self.query = query
        self.count = count ?? items.count
        self.totalMatchedCount = totalMatchedCount ?? items.count
        self.limitPerKind = limitPerKind
        self.items = items
        self.readOnly = readOnly
        self.providerRequestSent = providerRequestSent
        self.skillFilesMutated = skillFilesMutated
        self.agentConfigMutated = agentConfigMutated
        self.rawPromptPersisted = rawPromptPersisted
        self.rawResponsePersisted = rawResponsePersisted
        self.fallbackReason = fallbackReason
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let items = try container.decodeIfPresent([AppSearchItem].self, forKey: .items) ?? []
        self.init(
            generatedBy: try container.decodeIfPresent(String.self, forKey: .generatedBy)
                ?? container.decodeIfPresent(String.self, forKey: .generatedByAlt)
                ?? "local-v2.99",
            query: try container.decodeIfPresent(String.self, forKey: .query) ?? "",
            count: try container.decodeIfPresent(Int.self, forKey: .count),
            totalMatchedCount: try container.decodeIfPresent(Int.self, forKey: .totalMatchedCount)
                ?? container.decodeIfPresent(Int.self, forKey: .totalMatchedCountAlt),
            limitPerKind: try container.decodeIfPresent(Int.self, forKey: .limitPerKind)
                ?? container.decodeIfPresent(Int.self, forKey: .limitPerKindAlt)
                ?? items.count,
            items: items,
            readOnly: try container.decodeIfPresent(Bool.self, forKey: .readOnly)
                ?? container.decodeIfPresent(Bool.self, forKey: .readOnlyAlt)
                ?? true,
            providerRequestSent: try container.decodeIfPresent(Bool.self, forKey: .providerRequestSent)
                ?? container.decodeIfPresent(Bool.self, forKey: .providerRequestSentAlt)
                ?? false,
            skillFilesMutated: try container.decodeIfPresent(Bool.self, forKey: .skillFilesMutated)
                ?? container.decodeIfPresent(Bool.self, forKey: .skillFilesMutatedAlt)
                ?? false,
            agentConfigMutated: try container.decodeIfPresent(Bool.self, forKey: .agentConfigMutated)
                ?? container.decodeIfPresent(Bool.self, forKey: .agentConfigMutatedAlt)
                ?? false,
            rawPromptPersisted: try container.decodeIfPresent(Bool.self, forKey: .rawPromptPersisted)
                ?? container.decodeIfPresent(Bool.self, forKey: .rawPromptPersistedAlt)
                ?? false,
            rawResponsePersisted: try container.decodeIfPresent(Bool.self, forKey: .rawResponsePersisted)
                ?? container.decodeIfPresent(Bool.self, forKey: .rawResponsePersistedAlt)
                ?? false,
            fallbackReason: try container.decodeIfPresent(String.self, forKey: .fallbackReason)
                ?? container.decodeIfPresent(String.self, forKey: .reason)
        )
    }

    static func empty(query: String = "") -> AppSearchResult {
        AppSearchResult(query: query)
    }

    static func unavailable(query: String, reason: String) -> AppSearchResult {
        AppSearchResult(query: query, fallbackReason: reason)
    }
}

struct AppSearchItem: Decodable, Identifiable, Hashable {
    let id: String
    let kind: AppSearchItemKind
    let targetID: String
    let title: String
    let subtitle: String
    let agent: String?
    let skill: SkillRecord?
    let session: LocalSessionPreviewRow?
    let configSnapshot: ConfigSnapshotRecord?

    enum CodingKeys: String, CodingKey {
        case id
        case kind
        case targetID = "target_id"
        case targetIDAlt = "targetId"
        case title
        case subtitle
        case agent
        case skill
        case session
        case configSnapshot = "config_snapshot"
        case configSnapshotAlt = "configSnapshot"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(String.self, forKey: .id)
        kind = try container.decode(AppSearchItemKind.self, forKey: .kind)
        targetID = try container.decodeIfPresent(String.self, forKey: .targetID)
            ?? container.decodeIfPresent(String.self, forKey: .targetIDAlt)
            ?? id
        title = try container.decodeIfPresent(String.self, forKey: .title) ?? targetID
        subtitle = try container.decodeIfPresent(String.self, forKey: .subtitle) ?? ""
        agent = try container.decodeIfPresent(String.self, forKey: .agent)
        skill = try container.decodeIfPresent(SkillRecord.self, forKey: .skill)
        session = try container.decodeIfPresent(LocalSessionPreviewRow.self, forKey: .session)
        configSnapshot = try container.decodeIfPresent(ConfigSnapshotRecord.self, forKey: .configSnapshot)
            ?? container.decodeIfPresent(ConfigSnapshotRecord.self, forKey: .configSnapshotAlt)
    }
}
