import Foundation

enum LocalSessionContentKind: String, CaseIterable, Identifiable, Hashable {
    case userMessage = "user_message"
    case agentReply = "agent_reply"
    case thinking
    case toolCall = "tool_call"
    case skillCall = "skill_call"

    var id: String { rawValue }

    var title: String {
        switch self {
        case .userMessage:
            return UIStrings.text("localSessionContent.user", "User")
        case .agentReply:
            return UIStrings.text("localSessionContent.agent", "Agent")
        case .thinking:
            return UIStrings.text("localSessionContent.thinking", "Thinking")
        case .toolCall:
            return UIStrings.text("localSessionContent.tool", "Tool")
        case .skillCall:
            return UIStrings.text("localSessionContent.skill", "Skill")
        }
    }

    var systemImage: String {
        switch self {
        case .userMessage:
            return "person"
        case .agentReply:
            return "text.bubble"
        case .thinking:
            return "brain.head.profile"
        case .toolCall:
            return "wrench.and.screwdriver"
        case .skillCall:
            return "square.stack.3d.up"
        }
    }
}

struct LocalSessionPreviewRoot: Decodable, Hashable, Identifiable {
    let root: String
    let status: String
    let candidateCount: Int
    let blocker: String?

    var id: String { root }

    enum CodingKeys: String, CodingKey {
        case root
        case status
        case candidateCount = "candidate_count"
        case candidateCountAlt = "candidateCount"
        case blocker
    }

    init(root: String, status: String, candidateCount: Int, blocker: String? = nil) {
        self.root = root
        self.status = status
        self.candidateCount = candidateCount
        self.blocker = blocker
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        root = try container.decodeIfPresent(String.self, forKey: .root) ?? ""
        status = try container.decodeIfPresent(String.self, forKey: .status) ?? UIStrings.unknown
        candidateCount = try container.decodeIfPresent(Int.self, forKey: .candidateCount)
            ?? container.decodeIfPresent(Int.self, forKey: .candidateCountAlt)
            ?? 0
        blocker = try container.decodeIfPresent(String.self, forKey: .blocker)
    }
}

struct LocalSessionContentItem: Decodable, Hashable, Identifiable {
    let id: String
    let kind: LocalSessionContentKind
    let title: String
    let text: String
    let charCount: Int
    let timestamp: Int64?
    let evidenceRefs: [String]

    enum CodingKeys: String, CodingKey {
        case id
        case kind
        case title
        case text
        case charCount = "char_count"
        case charCountAlt = "charCount"
        case timestamp
        case createdAt = "created_at"
        case createdAtAlt = "createdAt"
        case evidenceRefs = "evidence_refs"
        case evidenceRefsAlt = "evidenceRefs"
        case evidence
    }

    init(
        id: String,
        kind: LocalSessionContentKind,
        title: String,
        text: String,
        charCount: Int? = nil,
        timestamp: Int64? = nil,
        evidenceRefs: [String] = []
    ) {
        self.id = id
        self.kind = kind
        self.title = title
        self.text = text
        self.charCount = charCount ?? text.count
        self.timestamp = timestamp
        self.evidenceRefs = evidenceRefs
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decodeIfPresent(String.self, forKey: .id) ?? UUID().uuidString
        let rawKind = try container.decodeIfPresent(String.self, forKey: .kind) ?? LocalSessionContentKind.agentReply.rawValue
        kind = LocalSessionContentKind(rawValue: rawKind) ?? .agentReply
        title = try container.decodeIfPresent(String.self, forKey: .title) ?? kind.title
        text = try container.decodeIfPresent(String.self, forKey: .text) ?? ""
        charCount = try container.decodeIfPresent(Int.self, forKey: .charCount)
            ?? container.decodeIfPresent(Int.self, forKey: .charCountAlt)
            ?? text.count
        timestamp = try container.decodeFlexibleLocalSessionInt64(keys: [.timestamp, .createdAt, .createdAtAlt])
        evidenceRefs = try container.decodeFlexibleLocalSessionStringArray(keys: [
            .evidenceRefs,
            .evidenceRefsAlt,
            .evidence
        ])
    }
}

struct LocalSessionPreviewRow: Decodable, Hashable, Identifiable {
    let id: String
    let title: String
    let sourceKind: String
    let scope: String
    let agent: String?
    let projectRoot: String?
    let redactedPath: String
    let modifiedAt: String?
    let startedAt: Int64?
    let endedAt: Int64?
    let excerpt: String
    let excerptCharCount: Int
    let userMessageCount: Int
    let totalMessageCount: Int
    let toolCallCount: Int
    let skillCallCount: Int
    let contentHash: String
    let evidenceRefs: [String]
    let contentIncluded: Bool
    let contentItems: [LocalSessionContentItem]

    enum CodingKeys: String, CodingKey {
        case id
        case title
        case sourceKind = "source_kind"
        case sourceKindAlt = "sourceKind"
        case scope
        case agent
        case projectRoot = "project_root"
        case projectRootAlt = "projectRoot"
        case redactedPath = "redacted_path"
        case redactedPathAlt = "redactedPath"
        case path
        case modifiedAt = "modified_at"
        case modifiedAtAlt = "modifiedAt"
        case startedAt = "started_at"
        case startedAtAlt = "startedAt"
        case endedAt = "ended_at"
        case endedAtAlt = "endedAt"
        case excerpt
        case redactedExcerpt = "redacted_excerpt"
        case excerptCharCount = "excerpt_char_count"
        case excerptCharCountAlt = "excerptCharCount"
        case userMessageCount = "user_message_count"
        case userMessageCountAlt = "userMessageCount"
        case totalMessageCount = "total_message_count"
        case totalMessageCountAlt = "totalMessageCount"
        case toolCallCount = "tool_call_count"
        case toolCallCountAlt = "toolCallCount"
        case skillCallCount = "skill_call_count"
        case skillCallCountAlt = "skillCallCount"
        case contentHash = "content_hash"
        case contentHashAlt = "contentHash"
        case evidenceRefs = "evidence_refs"
        case evidenceRefsAlt = "evidenceRefs"
        case evidence
        case contentIncluded = "content_included"
        case contentIncludedAlt = "contentIncluded"
        case contentItems = "content_items"
        case contentItemsAlt = "contentItems"
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decodeIfPresent(String.self, forKey: .id)
            ?? container.decodeIfPresent(String.self, forKey: .contentHash)
            ?? container.decodeIfPresent(String.self, forKey: .contentHashAlt)
            ?? UUID().uuidString
        title = try container.decodeIfPresent(String.self, forKey: .title) ?? id
        sourceKind = try container.decodeIfPresent(String.self, forKey: .sourceKind)
            ?? container.decodeIfPresent(String.self, forKey: .sourceKindAlt)
            ?? "authorized-local-session"
        scope = try container.decodeIfPresent(String.self, forKey: .scope) ?? "all"
        agent = try container.decodeIfPresent(String.self, forKey: .agent)
        projectRoot = try container.decodeIfPresent(String.self, forKey: .projectRoot)
            ?? container.decodeIfPresent(String.self, forKey: .projectRootAlt)
        redactedPath = try container.decodeIfPresent(String.self, forKey: .redactedPath)
            ?? container.decodeIfPresent(String.self, forKey: .redactedPathAlt)
            ?? container.decodeIfPresent(String.self, forKey: .path)
            ?? ""
        modifiedAt = try container.decodeFlexibleLocalSessionString(keys: [.modifiedAt, .modifiedAtAlt])
        startedAt = try container.decodeFlexibleLocalSessionInt64(keys: [.startedAt, .startedAtAlt])
        endedAt = try container.decodeFlexibleLocalSessionInt64(keys: [.endedAt, .endedAtAlt])
        excerpt = try container.decodeIfPresent(String.self, forKey: .excerpt)
            ?? container.decodeIfPresent(String.self, forKey: .redactedExcerpt)
            ?? ""
        excerptCharCount = try container.decodeIfPresent(Int.self, forKey: .excerptCharCount)
            ?? container.decodeIfPresent(Int.self, forKey: .excerptCharCountAlt)
            ?? excerpt.count
        userMessageCount = try container.decodeIfPresent(Int.self, forKey: .userMessageCount)
            ?? container.decodeIfPresent(Int.self, forKey: .userMessageCountAlt)
            ?? 0
        totalMessageCount = try container.decodeIfPresent(Int.self, forKey: .totalMessageCount)
            ?? container.decodeIfPresent(Int.self, forKey: .totalMessageCountAlt)
            ?? 0
        toolCallCount = try container.decodeIfPresent(Int.self, forKey: .toolCallCount)
            ?? container.decodeIfPresent(Int.self, forKey: .toolCallCountAlt)
            ?? 0
        skillCallCount = try container.decodeIfPresent(Int.self, forKey: .skillCallCount)
            ?? container.decodeIfPresent(Int.self, forKey: .skillCallCountAlt)
            ?? 0
        contentHash = try container.decodeIfPresent(String.self, forKey: .contentHash)
            ?? container.decodeIfPresent(String.self, forKey: .contentHashAlt)
            ?? ""
        evidenceRefs = try container.decodeFlexibleLocalSessionStringArray(keys: [
            .evidenceRefs,
            .evidenceRefsAlt,
            .evidence
        ])
        contentIncluded = try container.decodeIfPresent(Bool.self, forKey: .contentIncluded)
            ?? container.decodeIfPresent(Bool.self, forKey: .contentIncludedAlt)
            ?? true
        contentItems = try container.decodeIfPresent([LocalSessionContentItem].self, forKey: .contentItems)
            ?? container.decodeIfPresent([LocalSessionContentItem].self, forKey: .contentItemsAlt)
            ?? []
    }

    init(
        id: String,
        title: String,
        sourceKind: String,
        scope: String,
        agent: String?,
        projectRoot: String?,
        redactedPath: String,
        modifiedAt: String?,
        startedAt: Int64?,
        endedAt: Int64?,
        excerpt: String,
        excerptCharCount: Int,
        userMessageCount: Int,
        totalMessageCount: Int,
        toolCallCount: Int,
        skillCallCount: Int,
        contentHash: String,
        evidenceRefs: [String],
        contentIncluded: Bool,
        contentItems: [LocalSessionContentItem]
    ) {
        self.id = id
        self.title = title
        self.sourceKind = sourceKind
        self.scope = scope
        self.agent = agent
        self.projectRoot = projectRoot
        self.redactedPath = redactedPath
        self.modifiedAt = modifiedAt
        self.startedAt = startedAt
        self.endedAt = endedAt
        self.excerpt = excerpt
        self.excerptCharCount = excerptCharCount
        self.userMessageCount = userMessageCount
        self.totalMessageCount = totalMessageCount
        self.toolCallCount = toolCallCount
        self.skillCallCount = skillCallCount
        self.contentHash = contentHash
        self.evidenceRefs = evidenceRefs
        self.contentIncluded = contentIncluded
        self.contentItems = contentItems
    }

    var summaryOnly: LocalSessionPreviewRow {
        LocalSessionPreviewRow(
            id: id,
            title: title,
            sourceKind: sourceKind,
            scope: scope,
            agent: agent,
            projectRoot: projectRoot,
            redactedPath: redactedPath,
            modifiedAt: modifiedAt,
            startedAt: startedAt,
            endedAt: endedAt,
            excerpt: excerpt,
            excerptCharCount: excerptCharCount,
            userMessageCount: userMessageCount,
            totalMessageCount: totalMessageCount,
            toolCallCount: toolCallCount,
            skillCallCount: skillCallCount,
            contentHash: contentHash,
            evidenceRefs: evidenceRefs,
            contentIncluded: false,
            contentItems: []
        )
    }
}

struct LocalSessionPreviewRedactionSummary: Decodable, Hashable {
    let status: String
    let redactedValueCount: Int
    let redactedFields: [String]
    let rawTracePersisted: Bool
    let rawPromptPersisted: Bool
    let rawResponsePersisted: Bool
    let rawSecretReturned: Bool

    enum CodingKeys: String, CodingKey {
        case status
        case redactedValueCount = "redacted_value_count"
        case redactedValueCountAlt = "redactedValueCount"
        case redactedFields = "redacted_fields"
        case redactedFieldsAlt = "redactedFields"
        case rawTracePersisted = "raw_trace_persisted"
        case rawTracePersistedAlt = "rawTracePersisted"
        case rawPromptPersisted = "raw_prompt_persisted"
        case rawPromptPersistedAlt = "rawPromptPersisted"
        case rawResponsePersisted = "raw_response_persisted"
        case rawResponsePersistedAlt = "rawResponsePersisted"
        case rawSecretReturned = "raw_secret_returned"
        case rawSecretReturnedAlt = "rawSecretReturned"
    }

    init(
        status: String = "redacted-local-only",
        redactedValueCount: Int = 0,
        redactedFields: [String] = [],
        rawTracePersisted: Bool = false,
        rawPromptPersisted: Bool = false,
        rawResponsePersisted: Bool = false,
        rawSecretReturned: Bool = false
    ) {
        self.status = status
        self.redactedValueCount = redactedValueCount
        self.redactedFields = redactedFields
        self.rawTracePersisted = rawTracePersisted
        self.rawPromptPersisted = rawPromptPersisted
        self.rawResponsePersisted = rawResponsePersisted
        self.rawSecretReturned = rawSecretReturned
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        status = try container.decodeIfPresent(String.self, forKey: .status) ?? "redacted-local-only"
        redactedValueCount = try container.decodeIfPresent(Int.self, forKey: .redactedValueCount)
            ?? container.decodeIfPresent(Int.self, forKey: .redactedValueCountAlt)
            ?? 0
        redactedFields = try container.decodeFlexibleLocalSessionStringArray(keys: [.redactedFields, .redactedFieldsAlt])
        rawTracePersisted = try container.decodeIfPresent(Bool.self, forKey: .rawTracePersisted)
            ?? container.decodeIfPresent(Bool.self, forKey: .rawTracePersistedAlt)
            ?? false
        rawPromptPersisted = try container.decodeIfPresent(Bool.self, forKey: .rawPromptPersisted)
            ?? container.decodeIfPresent(Bool.self, forKey: .rawPromptPersistedAlt)
            ?? false
        rawResponsePersisted = try container.decodeIfPresent(Bool.self, forKey: .rawResponsePersisted)
            ?? container.decodeIfPresent(Bool.self, forKey: .rawResponsePersistedAlt)
            ?? false
        rawSecretReturned = try container.decodeIfPresent(Bool.self, forKey: .rawSecretReturned)
            ?? container.decodeIfPresent(Bool.self, forKey: .rawSecretReturnedAlt)
            ?? false
    }
}

struct LocalSessionSkillUsageRow: Decodable, Hashable, Identifiable {
    let skillId: String
    let skillName: String
    let agent: String
    let callCount: Int
    let sessionCount: Int
    let latestModifiedAt: String?
    let evidenceRefs: [String]

    var id: String { skillId }

    enum CodingKeys: String, CodingKey {
        case skillId = "skill_id"
        case skillIdAlt = "skillId"
        case skillName = "skill_name"
        case skillNameAlt = "skillName"
        case agent
        case callCount = "call_count"
        case callCountAlt = "callCount"
        case sessionCount = "session_count"
        case sessionCountAlt = "sessionCount"
        case latestModifiedAt = "latest_modified_at"
        case latestModifiedAtAlt = "latestModifiedAt"
        case evidenceRefs = "evidence_refs"
        case evidenceRefsAlt = "evidenceRefs"
        case evidence
    }

    init(
        skillId: String,
        skillName: String,
        agent: String,
        callCount: Int,
        sessionCount: Int,
        latestModifiedAt: String? = nil,
        evidenceRefs: [String] = []
    ) {
        self.skillId = skillId
        self.skillName = skillName
        self.agent = agent
        self.callCount = callCount
        self.sessionCount = sessionCount
        self.latestModifiedAt = latestModifiedAt
        self.evidenceRefs = evidenceRefs
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        skillId = try container.decodeIfPresent(String.self, forKey: .skillId)
            ?? container.decodeIfPresent(String.self, forKey: .skillIdAlt)
            ?? UUID().uuidString
        skillName = try container.decodeIfPresent(String.self, forKey: .skillName)
            ?? container.decodeIfPresent(String.self, forKey: .skillNameAlt)
            ?? skillId
        agent = try container.decodeIfPresent(String.self, forKey: .agent) ?? ""
        callCount = try container.decodeIfPresent(Int.self, forKey: .callCount)
            ?? container.decodeIfPresent(Int.self, forKey: .callCountAlt)
            ?? 0
        sessionCount = try container.decodeIfPresent(Int.self, forKey: .sessionCount)
            ?? container.decodeIfPresent(Int.self, forKey: .sessionCountAlt)
            ?? 0
        latestModifiedAt = try container.decodeFlexibleLocalSessionString(keys: [
            .latestModifiedAt,
            .latestModifiedAtAlt
        ])
        evidenceRefs = try container.decodeFlexibleLocalSessionStringArray(keys: [
            .evidenceRefs,
            .evidenceRefsAlt,
            .evidence
        ])
    }
}

struct LocalSessionPreviewResult: Decodable, Hashable {
    let generatedBy: String
    let authorized: Bool
    let authorizationRequired: Bool
    let roots: [LocalSessionPreviewRoot]
    let sessionRows: [LocalSessionPreviewRow]
    let skillUsageRows: [LocalSessionSkillUsageRow]
    let count: Int
    let totalCandidateCount: Int
    let totalMatchedCount: Int
    let offset: Int
    let limit: Int
    let hasMore: Bool
    let nextOffset: Int?
    let nextCursor: String?
    let sourceRevision: String?
    let sourceCompleteness: ListSourceCompleteness
    let incompleteReason: ListIncompleteReason?
    let candidateSetTruncated: Bool
    let userMessageCount: Int
    let totalMessageCount: Int
    let toolCallCount: Int
    let skillCallCount: Int
    let gapNotes: [String]
    let blockerNotes: [String]
    let redactionSummary: LocalSessionPreviewRedactionSummary
    let safetyFlags: ReadOnlySafetyFlags
    let fallbackReason: String?

    var isUnavailable: Bool {
        fallbackReason != nil && sessionRows.isEmpty
    }

    enum CodingKeys: String, CodingKey {
        case generatedBy = "generated_by"
        case generatedByAlt = "generatedBy"
        case authorized
        case authorizationRequired = "authorization_required"
        case authorizationRequiredAlt = "authorizationRequired"
        case roots
        case sessionRows = "session_rows"
        case sessionRowsAlt = "sessionRows"
        case rows
        case skillUsageRows = "skill_usage_rows"
        case skillUsageRowsAlt = "skillUsageRows"
        case count
        case totalCandidateCount = "total_candidate_count"
        case totalCandidateCountAlt = "totalCandidateCount"
        case totalMatchedCount = "total_matched_count"
        case totalMatchedCountAlt = "totalMatchedCount"
        case offset
        case limit
        case hasMore = "has_more"
        case hasMoreAlt = "hasMore"
        case nextOffset = "next_offset"
        case nextOffsetAlt = "nextOffset"
        case nextCursor = "next_cursor"
        case nextCursorAlt = "nextCursor"
        case sourceRevision = "source_revision"
        case sourceRevisionAlt = "sourceRevision"
        case sourceCompleteness = "source_completeness"
        case sourceCompletenessAlt = "sourceCompleteness"
        case incompleteReason = "incomplete_reason"
        case incompleteReasonAlt = "incompleteReason"
        case candidateSetTruncated = "candidate_set_truncated"
        case candidateSetTruncatedAlt = "candidateSetTruncated"
        case userMessageCount = "user_message_count"
        case userMessageCountAlt = "userMessageCount"
        case totalMessageCount = "total_message_count"
        case totalMessageCountAlt = "totalMessageCount"
        case toolCallCount = "tool_call_count"
        case toolCallCountAlt = "toolCallCount"
        case skillCallCount = "skill_call_count"
        case skillCallCountAlt = "skillCallCount"
        case gapNotes = "gap_notes"
        case gapNotesAlt = "gapNotes"
        case blockerNotes = "blocker_notes"
        case blockerNotesAlt = "blockerNotes"
        case redactionSummary = "redaction_summary"
        case redactionSummaryAlt = "redactionSummary"
        case safetyFlags = "safety_flags"
        case safety
        case fallbackReason = "fallback_reason"
        case reason
    }

    init(
        generatedBy: String = "local-v2.98",
        authorized: Bool = false,
        authorizationRequired: Bool = false,
        roots: [LocalSessionPreviewRoot] = [],
        sessionRows: [LocalSessionPreviewRow] = [],
        skillUsageRows: [LocalSessionSkillUsageRow] = [],
        count: Int? = nil,
        totalCandidateCount: Int = 0,
        totalMatchedCount: Int? = nil,
        offset: Int = 0,
        limit: Int? = nil,
        hasMore: Bool = false,
        nextOffset: Int? = nil,
        nextCursor: String? = nil,
        sourceRevision: String? = nil,
        sourceCompleteness: ListSourceCompleteness = .enumerable,
        incompleteReason: ListIncompleteReason? = nil,
        candidateSetTruncated: Bool = false,
        userMessageCount: Int? = nil,
        totalMessageCount: Int? = nil,
        toolCallCount: Int? = nil,
        skillCallCount: Int? = nil,
        gapNotes: [String] = [],
        blockerNotes: [String] = [],
        redactionSummary: LocalSessionPreviewRedactionSummary = LocalSessionPreviewRedactionSummary(),
        safetyFlags: ReadOnlySafetyFlags = ReadOnlySafetyFlags(),
        fallbackReason: String? = nil
    ) {
        self.generatedBy = generatedBy
        self.authorized = authorized
        self.authorizationRequired = authorizationRequired
        self.roots = roots
        self.sessionRows = sessionRows
        self.skillUsageRows = skillUsageRows
        self.count = count ?? sessionRows.count
        self.totalCandidateCount = totalCandidateCount
        self.totalMatchedCount = totalMatchedCount ?? sessionRows.count
        self.offset = max(0, offset)
        self.limit = limit ?? sessionRows.count
        self.hasMore = hasMore
        self.nextOffset = nextOffset
        self.nextCursor = nextCursor
        self.sourceRevision = sourceRevision
        self.sourceCompleteness = sourceCompleteness
        self.incompleteReason = incompleteReason
        self.candidateSetTruncated = candidateSetTruncated
        self.userMessageCount = userMessageCount ?? sessionRows.reduce(0) { $0 + $1.userMessageCount }
        self.totalMessageCount = totalMessageCount ?? sessionRows.reduce(0) { $0 + $1.totalMessageCount }
        self.toolCallCount = toolCallCount ?? sessionRows.reduce(0) { $0 + $1.toolCallCount }
        self.skillCallCount = skillCallCount ?? sessionRows.reduce(0) { $0 + $1.skillCallCount }
        self.gapNotes = gapNotes
        self.blockerNotes = blockerNotes
        self.redactionSummary = redactionSummary
        self.safetyFlags = safetyFlags
        self.fallbackReason = fallbackReason
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let rows = try container.decodeIfPresent([LocalSessionPreviewRow].self, forKey: .sessionRows)
            ?? container.decodeIfPresent([LocalSessionPreviewRow].self, forKey: .sessionRowsAlt)
            ?? container.decodeIfPresent([LocalSessionPreviewRow].self, forKey: .rows)
            ?? []
        let skillUsageRows = try container.decodeIfPresent([LocalSessionSkillUsageRow].self, forKey: .skillUsageRows)
            ?? container.decodeIfPresent([LocalSessionSkillUsageRow].self, forKey: .skillUsageRowsAlt)
            ?? []
        self.init(
            generatedBy: try container.decodeIfPresent(String.self, forKey: .generatedBy)
                ?? container.decodeIfPresent(String.self, forKey: .generatedByAlt)
                ?? "local-v2.98",
            authorized: try container.decodeIfPresent(Bool.self, forKey: .authorized) ?? !rows.isEmpty,
            authorizationRequired: try container.decodeIfPresent(Bool.self, forKey: .authorizationRequired)
                ?? container.decodeIfPresent(Bool.self, forKey: .authorizationRequiredAlt)
                ?? false,
            roots: try container.decodeIfPresent([LocalSessionPreviewRoot].self, forKey: .roots) ?? [],
            sessionRows: rows,
            skillUsageRows: skillUsageRows,
            count: try container.decodeIfPresent(Int.self, forKey: .count),
            totalCandidateCount: try container.decodeIfPresent(Int.self, forKey: .totalCandidateCount)
                ?? container.decodeIfPresent(Int.self, forKey: .totalCandidateCountAlt)
                ?? rows.count,
            totalMatchedCount: try container.decodeIfPresent(Int.self, forKey: .totalMatchedCount)
                ?? container.decodeIfPresent(Int.self, forKey: .totalMatchedCountAlt),
            offset: try container.decodeIfPresent(Int.self, forKey: .offset) ?? 0,
            limit: try container.decodeIfPresent(Int.self, forKey: .limit),
            hasMore: try container.decodeIfPresent(Bool.self, forKey: .hasMore)
                ?? container.decodeIfPresent(Bool.self, forKey: .hasMoreAlt)
                ?? false,
            nextOffset: try container.decodeIfPresent(Int.self, forKey: .nextOffset)
                ?? container.decodeIfPresent(Int.self, forKey: .nextOffsetAlt),
            nextCursor: try container.decodeIfPresent(String.self, forKey: .nextCursor)
                ?? container.decodeIfPresent(String.self, forKey: .nextCursorAlt),
            sourceRevision: try container.decodeIfPresent(String.self, forKey: .sourceRevision)
                ?? container.decodeIfPresent(String.self, forKey: .sourceRevisionAlt),
            sourceCompleteness: try container.decodeIfPresent(ListSourceCompleteness.self, forKey: .sourceCompleteness)
                ?? container.decodeIfPresent(ListSourceCompleteness.self, forKey: .sourceCompletenessAlt)
                ?? .enumerable,
            incompleteReason: try container.decodeIfPresent(ListIncompleteReason.self, forKey: .incompleteReason)
                ?? container.decodeIfPresent(ListIncompleteReason.self, forKey: .incompleteReasonAlt),
            candidateSetTruncated: try container.decodeIfPresent(Bool.self, forKey: .candidateSetTruncated)
                ?? container.decodeIfPresent(Bool.self, forKey: .candidateSetTruncatedAlt)
                ?? false,
            userMessageCount: try container.decodeIfPresent(Int.self, forKey: .userMessageCount)
                ?? container.decodeIfPresent(Int.self, forKey: .userMessageCountAlt),
            totalMessageCount: try container.decodeIfPresent(Int.self, forKey: .totalMessageCount)
                ?? container.decodeIfPresent(Int.self, forKey: .totalMessageCountAlt),
            toolCallCount: try container.decodeIfPresent(Int.self, forKey: .toolCallCount)
                ?? container.decodeIfPresent(Int.self, forKey: .toolCallCountAlt),
            skillCallCount: try container.decodeIfPresent(Int.self, forKey: .skillCallCount)
                ?? container.decodeIfPresent(Int.self, forKey: .skillCallCountAlt),
            gapNotes: try container.decodeFlexibleLocalSessionStringArray(keys: [.gapNotes, .gapNotesAlt]),
            blockerNotes: try container.decodeFlexibleLocalSessionStringArray(keys: [.blockerNotes, .blockerNotesAlt]),
            redactionSummary: try container.decodeIfPresent(LocalSessionPreviewRedactionSummary.self, forKey: .redactionSummary)
                ?? container.decodeIfPresent(LocalSessionPreviewRedactionSummary.self, forKey: .redactionSummaryAlt)
                ?? LocalSessionPreviewRedactionSummary(),
            safetyFlags: try container.decodeIfPresent(ReadOnlySafetyFlags.self, forKey: .safetyFlags)
                ?? container.decodeIfPresent(ReadOnlySafetyFlags.self, forKey: .safety)
                ?? ReadOnlySafetyFlags(),
            fallbackReason: try container.decodeIfPresent(String.self, forKey: .fallbackReason)
                ?? container.decodeIfPresent(String.self, forKey: .reason)
        )
    }

    static func unavailable(reason: String = UIStrings.text("localSessionPreview.unavailable", "Local session preview is unavailable.")) -> LocalSessionPreviewResult {
        LocalSessionPreviewResult(generatedBy: "unavailable", fallbackReason: reason)
    }

    func mergingPage(_ page: LocalSessionPreviewResult) -> LocalSessionPreviewResult {
        var rowsByID = Dictionary(uniqueKeysWithValues: sessionRows.map { ($0.id, $0) })
        var mergedRows = sessionRows
        for row in page.sessionRows where rowsByID[row.id] == nil {
            rowsByID[row.id] = row
            mergedRows.append(row)
        }
        return LocalSessionPreviewResult(
            generatedBy: page.generatedBy,
            authorized: page.authorized || authorized,
            authorizationRequired: page.authorizationRequired,
            roots: page.roots.isEmpty ? roots : page.roots,
            sessionRows: mergedRows,
            skillUsageRows: mergingSkillUsageRows(page.skillUsageRows),
            count: mergedRows.count,
            totalCandidateCount: page.totalCandidateCount,
            totalMatchedCount: page.totalMatchedCount,
            offset: 0,
            limit: page.limit,
            hasMore: page.hasMore,
            nextOffset: page.nextOffset,
            nextCursor: page.nextCursor,
            sourceRevision: page.sourceRevision,
            sourceCompleteness: page.sourceCompleteness,
            incompleteReason: page.incompleteReason,
            candidateSetTruncated: candidateSetTruncated || page.candidateSetTruncated,
            gapNotes: page.gapNotes,
            blockerNotes: page.blockerNotes,
            redactionSummary: page.redactionSummary,
            safetyFlags: page.safetyFlags,
            fallbackReason: page.fallbackReason
        )
    }

    private func mergingSkillUsageRows(
        _ pageRows: [LocalSessionSkillUsageRow]
    ) -> [LocalSessionSkillUsageRow] {
        var rowsByID = Dictionary(uniqueKeysWithValues: skillUsageRows.map { ($0.skillId, $0) })
        for row in pageRows {
            guard let current = rowsByID[row.skillId] else {
                rowsByID[row.skillId] = row
                continue
            }
            var seenEvidence = Set<String>()
            let evidenceRefs = (current.evidenceRefs + row.evidenceRefs).filter {
                seenEvidence.insert($0).inserted
            }
            rowsByID[row.skillId] = LocalSessionSkillUsageRow(
                skillId: current.skillId,
                skillName: current.skillName.isEmpty ? row.skillName : current.skillName,
                agent: current.agent.isEmpty ? row.agent : current.agent,
                callCount: current.callCount + row.callCount,
                sessionCount: current.sessionCount + row.sessionCount,
                latestModifiedAt: Self.latestTimestamp(
                    current.latestModifiedAt,
                    row.latestModifiedAt
                ),
                evidenceRefs: evidenceRefs
            )
        }
        return rowsByID.values.sorted { left, right in
            if left.callCount != right.callCount {
                return left.callCount > right.callCount
            }
            if left.sessionCount != right.sessionCount {
                return left.sessionCount > right.sessionCount
            }
            if left.latestModifiedAt != right.latestModifiedAt {
                return Self.timestampIsLater(left.latestModifiedAt, than: right.latestModifiedAt)
            }
            let nameOrder = left.skillName.localizedCaseInsensitiveCompare(right.skillName)
            if nameOrder != .orderedSame {
                return nameOrder == .orderedAscending
            }
            return left.skillId < right.skillId
        }
    }

    private static func latestTimestamp(_ left: String?, _ right: String?) -> String? {
        guard let left else { return right }
        guard let right else { return left }
        return timestampIsLater(left, than: right) ? left : right
    }

    private static func timestampIsLater(_ left: String?, than right: String?) -> Bool {
        guard let left else { return false }
        guard let right else { return true }
        if let leftValue = Int64(left), let rightValue = Int64(right) {
            return leftValue > rightValue
        }
        return left > right
    }

    func ensuringSession(_ session: LocalSessionPreviewRow) -> LocalSessionPreviewResult {
        var rows = sessionRows
        if let index = rows.firstIndex(where: { $0.id == session.id }) {
            rows[index] = session
        } else {
            rows.insert(session, at: 0)
        }
        return LocalSessionPreviewResult(
            generatedBy: generatedBy,
            authorized: true,
            authorizationRequired: authorizationRequired,
            roots: roots,
            sessionRows: rows,
            skillUsageRows: skillUsageRows,
            count: rows.count,
            totalCandidateCount: max(totalCandidateCount, rows.count),
            totalMatchedCount: max(totalMatchedCount, rows.count),
            offset: offset,
            limit: limit,
            hasMore: hasMore,
            nextOffset: nextOffset,
            nextCursor: nextCursor,
            sourceRevision: sourceRevision,
            sourceCompleteness: sourceCompleteness,
            incompleteReason: incompleteReason,
            candidateSetTruncated: candidateSetTruncated,
            gapNotes: gapNotes,
            blockerNotes: blockerNotes,
            redactionSummary: redactionSummary,
            safetyFlags: safetyFlags,
            fallbackReason: fallbackReason
        )
    }
}

private extension KeyedDecodingContainer {
    func decodeFlexibleLocalSessionString(keys: [Key]) throws -> String? {
        for key in keys {
            if let value = try? decodeIfPresent(String.self, forKey: key) {
                return value
            }
            if let value = try? decodeIfPresent(Int.self, forKey: key) {
                return "\(value)"
            }
        }
        return nil
    }

    func decodeFlexibleLocalSessionStringArray(keys: [Key]) throws -> [String] {
        for key in keys {
            if let values = try? decodeIfPresent([String].self, forKey: key) {
                return values
            }
            if let value = try? decodeIfPresent(String.self, forKey: key) {
                return value.isEmpty ? [] : [value]
            }
        }
        return []
    }

    func decodeFlexibleLocalSessionInt64(keys: [Key]) throws -> Int64? {
        for key in keys {
            if let value = try? decodeIfPresent(Int64.self, forKey: key) {
                return value
            }
            if let value = try? decodeIfPresent(Int.self, forKey: key) {
                return Int64(value)
            }
            if let value = try? decodeIfPresent(Double.self, forKey: key), value.isFinite {
                return Int64(value.rounded())
            }
            if let value = try? decodeIfPresent(String.self, forKey: key) {
                let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
                if let intValue = Int64(trimmed) {
                    return intValue
                }
                if let doubleValue = Double(trimmed), doubleValue.isFinite {
                    return Int64(doubleValue.rounded())
                }
            }
        }
        return nil
    }
}
