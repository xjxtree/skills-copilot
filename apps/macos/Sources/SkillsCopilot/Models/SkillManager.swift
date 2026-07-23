import Foundation

enum SkillManagerAgent: String, CaseIterable, Identifiable, Hashable {
    case claudeCode = "claude-code"
    case pi
    case opencode
    case codex
    case hermesAgent = "hermes-agent"
    case openclaw

    var id: String { rawValue }

    static let defaultTargets: [SkillManagerAgent] = [
        .claudeCode,
        .pi,
        .opencode,
        .codex,
        .hermesAgent,
        .openclaw
    ]

    var title: String {
        switch self {
        case .claudeCode:
            return UIStrings.claudeCode
        case .pi:
            return UIStrings.pi
        case .opencode:
            return UIStrings.opencode
        case .codex:
            return UIStrings.codex
        case .hermesAgent:
            return UIStrings.hermes
        case .openclaw:
            return UIStrings.openclaw
        }
    }
}

enum SkillManagerScope: String, CaseIterable, Identifiable, Codable, Hashable {
    case project
    case global

    var id: String { rawValue }

    var title: String {
        switch self {
        case .project:
            return UIStrings.text("skillManager.scope.project", "Project")
        case .global:
            return UIStrings.text("skillManager.scope.global", "Global")
        }
    }
}

enum SkillManagerDistribution: String, CaseIterable, Identifiable, Codable, Hashable {
    case symlink
    case copy

    var id: String { rawValue }

    var title: String {
        switch self {
        case .symlink:
            return UIStrings.text("skillManager.distribution.symlink", "Symlink")
        case .copy:
            return UIStrings.text("skillManager.distribution.copy", "Copy")
        }
    }
}

enum SkillManagerWorkflow: String, CaseIterable, Identifiable, Hashable {
    case searchInstall = "search-install"
    case installedUpdates = "installed-updates"

    var id: String { rawValue }

    var title: String {
        switch self {
        case .searchInstall:
            return UIStrings.text("skillManager.workflow.searchInstall", "Search & Install")
        case .installedUpdates:
            return UIStrings.text("skillManager.workflow.installedUpdates", "Installed & Updates")
        }
    }

    var systemImage: String {
        switch self {
        case .searchInstall:
            return "magnifyingglass"
        case .installedUpdates:
            return "list.bullet.rectangle"
        }
    }
}

struct SkillManagerToolRecord: Codable, Identifiable, Hashable {
    let id: String
    let displayName: String
    let status: String
    let executable: String?
    let operations: [String]
    let defaultAgents: [String]
    let notes: [String]

    enum CodingKeys: String, CodingKey {
        case id
        case displayName = "display_name"
        case status
        case executable
        case operations
        case defaultAgents = "default_agents"
        case notes
    }
}

struct SkillManagerEnvPreview: Codable, Hashable {
    let key: String
    let value: String
}

struct SkillManagerCommandPreview: Codable, Hashable {
    let action: ActionDescriptorWire?
    let toolId: String
    let operation: String
    let command: [String]
    let cwd: String
    let env: [SkillManagerEnvPreview]
    let requiresConfirmation: Bool
    let confirmed: Bool
    let networkRequired: Bool
    let networkAllowed: Bool
    let willRun: Bool
    let previewToken: String
    let summary: String
    let risks: [String]
    let source: String?
    let skills: [String]?

    enum CodingKeys: String, CodingKey {
        case action
        case toolId = "tool_id"
        case operation
        case command
        case cwd
        case env
        case requiresConfirmation = "requires_confirmation"
        case confirmed
        case networkRequired = "network_required"
        case networkAllowed = "network_allowed"
        case willRun = "will_run"
        case previewToken = "preview_token"
        case summary
        case risks
        case source
        case skills
    }

    init(
        toolId: String,
        operation: String,
        command: [String],
        cwd: String,
        env: [SkillManagerEnvPreview],
        requiresConfirmation: Bool,
        confirmed: Bool,
        networkRequired: Bool,
        networkAllowed: Bool,
        willRun: Bool,
        previewToken: String,
        summary: String,
        risks: [String],
        source: String?,
        skills: [String]?,
        action: ActionDescriptorWire? = nil
    ) {
        self.action = action
        self.toolId = toolId
        self.operation = operation
        self.command = command
        self.cwd = cwd
        self.env = env
        self.requiresConfirmation = requiresConfirmation
        self.confirmed = confirmed
        self.networkRequired = networkRequired
        self.networkAllowed = networkAllowed
        self.willRun = willRun
        self.previewToken = previewToken
        self.summary = summary
        self.risks = risks
        self.source = source
        self.skills = skills
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let action = try container.decodeIfPresent(ActionDescriptorWire.self, forKey: .action)
        let requiresConfirmation = try container.decode(Bool.self, forKey: .requiresConfirmation)
        if requiresConfirmation, action == nil {
            throw DecodingError.dataCorruptedError(
                forKey: .action,
                in: container,
                debugDescription: "A mutating Skill Manager preview requires a typed action."
            )
        }
        self.init(
            toolId: try container.decode(String.self, forKey: .toolId),
            operation: try container.decode(String.self, forKey: .operation),
            command: try container.decode([String].self, forKey: .command),
            cwd: try container.decode(String.self, forKey: .cwd),
            env: try container.decode([SkillManagerEnvPreview].self, forKey: .env),
            requiresConfirmation: requiresConfirmation,
            confirmed: try container.decode(Bool.self, forKey: .confirmed),
            networkRequired: try container.decode(Bool.self, forKey: .networkRequired),
            networkAllowed: try container.decode(Bool.self, forKey: .networkAllowed),
            willRun: try container.decode(Bool.self, forKey: .willRun),
            previewToken: try container.decode(String.self, forKey: .previewToken),
            summary: try container.decode(String.self, forKey: .summary),
            risks: try container.decode([String].self, forKey: .risks),
            source: try container.decodeIfPresent(String.self, forKey: .source),
            skills: try container.decodeIfPresent([String].self, forKey: .skills),
            action: action
        )
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encodeIfPresent(action, forKey: .action)
        try container.encode(toolId, forKey: .toolId)
        try container.encode(operation, forKey: .operation)
        try container.encode(command, forKey: .command)
        try container.encode(cwd, forKey: .cwd)
        try container.encode(env, forKey: .env)
        try container.encode(requiresConfirmation, forKey: .requiresConfirmation)
        try container.encode(confirmed, forKey: .confirmed)
        try container.encode(networkRequired, forKey: .networkRequired)
        try container.encode(networkAllowed, forKey: .networkAllowed)
        try container.encode(willRun, forKey: .willRun)
        try container.encode(previewToken, forKey: .previewToken)
        try container.encode(summary, forKey: .summary)
        try container.encode(risks, forKey: .risks)
        try container.encodeIfPresent(source, forKey: .source)
        try container.encodeIfPresent(skills, forKey: .skills)
    }

    var displayCommand: String {
        command.map(Self.shellDisplay).joined(separator: " ")
    }

    var localizedSummary: String {
        switch operation {
        case "search":
            return UIStrings.text("skillManager.previewSummary.search", summary)
        case "listInstalled":
            return UIStrings.text("skillManager.previewSummary.listInstalled", summary)
        case "install":
            return String(
                format: UIStrings.text(
                    "skillManager.previewSummary.install",
                    "Preview install of %@ for selected targets."
                ),
                source ?? UIStrings.text("skillManager.source", "Source")
            )
        case "remove":
            return String(
                format: UIStrings.text(
                    "skillManager.previewSummary.remove",
                    "Preview removal of %@ from selected targets."
                ),
                skills?.first ?? UIStrings.text("skillManager.skillName", "Skill name")
            )
        case "update":
            return UIStrings.text("skillManager.previewSummary.update", summary)
        case "localCreate":
            return String(
                format: UIStrings.text(
                    "skillManager.previewSummary.localCreate",
                    "Preview local skill template creation for %@."
                ),
                skills?.first ?? UIStrings.text("skillManager.localName", "Local skill name")
            )
        default:
            return summary
        }
    }

    private static func shellDisplay(_ value: String) -> String {
        guard value.rangeOfCharacter(from: .whitespacesAndNewlines) != nil else {
            return value
        }
        return "'\(value.replacingOccurrences(of: "'", with: "'\\''"))'"
    }

    var compactMetadataRows: [CompactMetadataRow] {
        var rows = [
            CompactMetadataRow(label: "CWD", value: cwd, systemImage: "folder", isCopyable: true),
            CompactMetadataRow(
                label: UIStrings.text("skillManager.confirmed", "Confirmed"),
                value: confirmed ? UIStrings.text("value.yes", "Yes") : UIStrings.text("value.no", "No"),
                systemImage: "checkmark.circle"
            ),
            CompactMetadataRow(
                label: UIStrings.text("skillManager.network", "Network"),
                value: networkAllowed ? UIStrings.text("value.yes", "Yes") : UIStrings.text("value.no", "No"),
                systemImage: "network"
            )
        ]
        if let action {
            rows.append(
                CompactMetadataRow(
                    label: UIStrings.text("skillManager.actionTarget", "Action target"),
                    value: action.target.id,
                    systemImage: "scope"
                )
            )
            rows.append(
                CompactMetadataRow(
                    label: UIStrings.text("skillManager.impact", "Impact"),
                    value: action.impacts.joined(separator: ", "),
                    systemImage: "exclamationmark.shield"
                )
            )
        }
        return rows
    }

    var requiresExplicitApplyConfirmation: Bool {
        requiresConfirmation && ["install", "remove", "update", "localCreate"].contains(operation)
    }
}

struct SkillManagerCommandOutput: Codable, Hashable {
    let status: String
    let exitCode: Int?
    let stdout: String
    let stderr: String

    enum CodingKeys: String, CodingKey {
        case status
        case exitCode = "exit_code"
        case stdout
        case stderr
    }
}

struct SkillManagerSearchParams: Encodable {
    let query: String
    let owner: String?
    let networkAllowed: Bool

    enum CodingKeys: String, CodingKey {
        case query
        case owner
        case networkAllowed = "network_allowed"
    }
}

struct SkillManagerSearchResult: Codable, Identifiable, Hashable {
    let name: String
    let source: String?
    let description: String?
    let raw: JSONValue

    var id: String {
        [source ?? "", name].joined(separator: "|")
    }
}

struct SkillManagerVisibleResults<ID: Hashable>: Equatable {
    private(set) var visibleCount = 20

    func visibleItems<Item>(in items: [Item]) -> [Item] {
        Array(items.prefix(visibleCount))
    }

    mutating func loadMore(totalReturned: Int) {
        visibleCount = min(max(0, totalReturned), visibleCount + 20)
    }

    mutating func loadAll(totalReturned: Int) {
        visibleCount = max(0, totalReturned)
    }

    mutating func reset() {
        visibleCount = 20
    }
}

struct SkillManagerSearchRecord: Codable, Hashable {
    let preview: SkillManagerCommandPreview
    let output: SkillManagerCommandOutput?
    let results: [SkillManagerSearchResult]
    let returnedCount: Int
    let totalCount: Int?
    let hasMore: Bool
    let nextCursor: String?
    let sourceCompleteness: ListSourceCompleteness
    let incompleteReason: ListIncompleteReason?

    enum CodingKeys: String, CodingKey {
        case preview
        case output
        case results
        case returnedCount = "returned_count"
        case totalCount = "total_count"
        case hasMore = "has_more"
        case nextCursor = "next_cursor"
        case sourceCompleteness = "source_completeness"
        case incompleteReason = "incomplete_reason"
    }

    var isBlockedByNetwork: Bool {
        preview.networkRequired && !preview.networkAllowed && output == nil
    }

    var hasValidPageMetadata: Bool {
        returnedCount == results.count
            && totalCount == nil
            && !hasMore
            && nextCursor == nil
            && sourceCompleteness == .unknown
            && incompleteReason == .sourceLimited
    }

    func listStatus(visibleCount: Int) -> ListCompletenessState {
        skillManagerListStatus(
            visibleCount: visibleCount,
            returnedCount: results.count,
            totalCount: totalCount,
            serviceHasMore: hasMore,
            sourceCompleteness: sourceCompleteness,
            incompleteReason: incompleteReason
        )
    }

    func displayResults(visibleCount: Int) -> [OccurrenceIdentifiedItem<SkillManagerSearchResult>] {
        OccurrenceIdentifiedItem.rows(for: Array(results.prefix(max(0, visibleCount))))
    }
}

struct SkillManagerListInstalledParams: Encodable {
    let agents: [String]
    let scope: String?
}

struct SkillManagerInstalledRecord: Codable, Identifiable, Hashable {
    let name: String
    let source: String?
    let sourceKind: String?
    let agents: [String]
    let scope: String?
    let path: String?
    let raw: JSONValue?

    enum CodingKeys: String, CodingKey {
        case name
        case source
        case sourceKind = "source_kind"
        case agents
        case scope
        case path
        case raw
    }

    var id: String {
        [source ?? "", name, scope ?? "", path ?? ""].joined(separator: "|")
    }

    var isLocalSource: Bool {
        sourceKind?.caseInsensitiveCompare("local") == .orderedSame
    }
}

struct SkillManagerInventoryItem: Identifiable, Hashable {
    enum Origin: String, Hashable {
        case manager
        case local
    }

    enum LocalOwnership: String, Hashable {
        case appOwned
        case project
        case global
        case external
    }

    let name: String
    let source: String?
    let scope: SkillManagerScope
    let agents: [String]
    let origin: Origin
    let localOwnership: LocalOwnership?
    let localInstanceID: String?
    let localPath: String?

    var id: String {
        [scope.rawValue, origin.rawValue, name, localInstanceID ?? source ?? ""].joined(separator: "|")
    }

    var isInstalled: Bool { !agents.isEmpty }
}

enum SkillManagerInventoryBuilder {
    private struct CatalogSource {
        let path: String
        let nameKey: String
        let representative: SkillRecord
        let agents: [String]
    }

    static func build(
        installed: [SkillManagerInstalledRecord],
        catalogSkills: [SkillRecord],
        localLibrarySkills: [SkillRecord],
        scope: SkillManagerScope
    ) -> [SkillManagerInventoryItem] {
        let catalogSources = editableCatalogSources(from: catalogSkills, scope: scope)
        let sourcesByName = Dictionary(grouping: catalogSources, by: \.nameKey)
        let libraryByName = Dictionary(grouping: localLibrarySkills, by: { normalizedName($0.name) })
        var consumedSourcePaths = Set<String>()
        var consumedLibraryIDs = Set<String>()
        var installedNameKeys = Set<String>()
        var items = deduplicatedInstalled(installed).map { record in
            let nameKey = normalizedName(record.name)
            installedNameKeys.insert(nameKey)
            let catalogSource = matchingCatalogSource(
                for: record,
                candidates: sourcesByName[nameKey] ?? []
            )
            if let catalogSource {
                consumedSourcePaths.insert(catalogSource.path)
            }
            let localSource = record.isLocalSource ? catalogSource : nil
            let appOwnedSource = record.isLocalSource && localSource == nil
                ? matchingLocalLibrarySource(for: record, candidates: libraryByName[nameKey] ?? [])
                : nil
            if let appOwnedSource {
                consumedLibraryIDs.insert(appOwnedSource.id)
            }
            return SkillManagerInventoryItem(
                name: record.name,
                source: localSource?.path ?? appOwnedSource.map(sourceDirectory) ?? record.source,
                scope: scope,
                agents: canonicalAgentIDs(record.agents),
                origin: record.isLocalSource ? .local : .manager,
                localOwnership: record.isLocalSource
                    ? (localSource != nil ? localOwnership(for: scope) : (appOwnedSource == nil ? .external : .appOwned))
                    : nil,
                localInstanceID: localSource?.representative.id ?? appOwnedSource?.id,
                localPath: localSource?.path ?? appOwnedSource.map(sourceDirectory)
            )
        }

        items.append(contentsOf: catalogSources.compactMap { source in
            guard !consumedSourcePaths.contains(source.path) else { return nil }
            installedNameKeys.insert(source.nameKey)
            return SkillManagerInventoryItem(
                name: source.representative.name,
                source: source.path,
                scope: scope,
                agents: source.agents,
                origin: .local,
                localOwnership: localOwnership(for: scope),
                localInstanceID: source.representative.id,
                localPath: source.path
            )
        })

        let uniqueLibrarySkills = Dictionary(grouping: localLibrarySkills, by: { normalizedName($0.name) })
            .compactMap { nameKey, skills -> SkillRecord? in
                guard !installedNameKeys.contains(nameKey) else { return nil }
                return skills.sorted { $0.id < $1.id }.first { !consumedLibraryIDs.contains($0.id) }
            }
        items.append(contentsOf: uniqueLibrarySkills.map { skill in
            let path = sourceDirectory(for: skill)
            return SkillManagerInventoryItem(
                name: skill.name,
                source: path,
                scope: scope,
                agents: [],
                origin: .local,
                localOwnership: .appOwned,
                localInstanceID: skill.id,
                localPath: path
            )
        })

        return items.sorted {
            let comparison = $0.name.localizedCaseInsensitiveCompare($1.name)
            if comparison != .orderedSame { return comparison == .orderedAscending }
            return $0.id < $1.id
        }
    }

    private static func editableCatalogSources(
        from skills: [SkillRecord],
        scope: SkillManagerScope
    ) -> [CatalogSource] {
        let eligible = skills.compactMap { skill -> (String, SkillRecord)? in
            guard skill.state != "missing",
                  !DisplayText.isToolGlobal(skill),
                  scopeMatches(skill.scope, scope: scope),
                  agentID(for: skill.agent) != nil,
                  let path = sharedAgentsSourceDirectory(for: skill) else {
                return nil
            }
            return (path, skill)
        }
        return Dictionary(grouping: eligible, by: { $0.0 }).compactMap { path, rows in
            let skills = rows.map(\.1).sorted { $0.id < $1.id }
            guard let representative = skills.first else { return nil }
            return CatalogSource(
                path: path,
                nameKey: normalizedName(representative.name),
                representative: representative,
                agents: canonicalAgentIDs(skills.map(\.agent))
            )
        }
    }

    private static func matchingCatalogSource(
        for record: SkillManagerInstalledRecord,
        candidates: [CatalogSource]
    ) -> CatalogSource? {
        if let pathSuffix = sharedAgentsPathSuffix(record.path ?? record.source),
           let exact = candidates.first(where: { sharedAgentsPathSuffix($0.path) == pathSuffix }) {
            return exact
        }
        if candidates.count == 1 { return candidates[0] }
        return candidates.sorted { $0.path.localizedCaseInsensitiveCompare($1.path) == .orderedAscending }.first
    }

    private static func matchingLocalLibrarySource(
        for record: SkillManagerInstalledRecord,
        candidates: [SkillRecord]
    ) -> SkillRecord? {
        guard !candidates.isEmpty else { return nil }
        let source = record.path ?? record.source
        if let source {
            let normalizedSource = URL(fileURLWithPath: source).standardized.path
            if let exact = candidates.first(where: {
                let candidate = URL(fileURLWithPath: sourceDirectory(for: $0)).standardized.path
                return candidate == normalizedSource || normalizedSource.hasSuffix(candidate)
            }) {
                return exact
            }
        }
        return candidates.count == 1 ? candidates[0] : nil
    }

    private static func deduplicatedInstalled(
        _ records: [SkillManagerInstalledRecord]
    ) -> [SkillManagerInstalledRecord] {
        var order: [String] = []
        var recordsByKey: [String: SkillManagerInstalledRecord] = [:]
        for record in records {
            let key = [
                normalizedName(record.name),
                record.sourceKind?.lowercased() ?? "",
                sharedAgentsPathSuffix(record.path) ?? record.path ?? record.source ?? "",
                record.scope ?? ""
            ].joined(separator: "|")
            guard let existing = recordsByKey[key] else {
                order.append(key)
                recordsByKey[key] = record
                continue
            }
            recordsByKey[key] = SkillManagerInstalledRecord(
                name: existing.name,
                source: existing.source ?? record.source,
                sourceKind: existing.sourceKind ?? record.sourceKind,
                agents: canonicalAgentIDs(existing.agents + record.agents),
                scope: existing.scope ?? record.scope,
                path: existing.path ?? record.path,
                raw: existing.raw ?? record.raw
            )
        }
        return order.compactMap { recordsByKey[$0] }
    }

    private static func sharedAgentsSourceDirectory(for skill: SkillRecord) -> String? {
        let path = URL(fileURLWithPath: skill.path).standardized.path
        let components = URL(fileURLWithPath: path).pathComponents
        guard components.last?.caseInsensitiveCompare("SKILL.md") == .orderedSame else { return nil }
        for index in components.indices where components[index] == ".agents" {
            guard components.indices.contains(index + 2),
                  components[index + 1] == "skills",
                  index + 2 < components.index(before: components.endIndex) else {
                continue
            }
            return URL(fileURLWithPath: path).deletingLastPathComponent().path
        }
        return nil
    }

    private static func sharedAgentsPathSuffix(_ path: String?) -> String? {
        guard let path,
              let range = path.range(of: "/.agents/skills/", options: [.caseInsensitive]) else {
            return nil
        }
        let suffix = String(path[range.upperBound...])
            .trimmingCharacters(in: CharacterSet(charactersIn: "/"))
            .lowercased()
        return suffix.hasSuffix("/skill.md")
            ? String(suffix.dropLast("/skill.md".count))
            : suffix
    }

    private static func sourceDirectory(for skill: SkillRecord) -> String {
        let url = URL(fileURLWithPath: skill.path)
        return url.lastPathComponent.caseInsensitiveCompare("SKILL.md") == .orderedSame
            ? url.deletingLastPathComponent().path
            : skill.path
    }

    private static func scopeMatches(_ value: String, scope: SkillManagerScope) -> Bool {
        let normalized = value.lowercased()
        switch scope {
        case .project: return normalized.contains("project")
        case .global: return normalized.contains("global") || normalized.contains("user")
        }
    }

    private static func localOwnership(for scope: SkillManagerScope) -> SkillManagerInventoryItem.LocalOwnership {
        scope == .project ? .project : .global
    }

    private static func normalizedName(_ name: String) -> String {
        name.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    }

    private static func canonicalAgentIDs(_ agents: [String]) -> [String] {
        let selected = Set(agents.compactMap(agentID))
        return SkillManagerAgent.defaultTargets.map(\.rawValue).filter(selected.contains)
    }

    private static func agentID(for agent: String) -> String? {
        switch agent.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "claude", "claude code", "claude-code": return SkillManagerAgent.claudeCode.rawValue
        case "codex": return SkillManagerAgent.codex.rawValue
        case "opencode", "open code", "open-code": return SkillManagerAgent.opencode.rawValue
        case "pi": return SkillManagerAgent.pi.rawValue
        case "hermes", "hermes agent", "hermes-agent": return SkillManagerAgent.hermesAgent.rawValue
        case "openclaw", "open claw", "open-claw": return SkillManagerAgent.openclaw.rawValue
        default: return nil
        }
    }
}

struct SkillManagerInstalledListRecord: Codable, Hashable {
    let preview: SkillManagerCommandPreview
    let output: SkillManagerCommandOutput
    let installed: [SkillManagerInstalledRecord]
    let returnedCount: Int
    let totalCount: Int?
    let hasMore: Bool
    let nextCursor: String?
    let sourceCompleteness: ListSourceCompleteness
    let incompleteReason: ListIncompleteReason?

    enum CodingKeys: String, CodingKey {
        case preview
        case output
        case installed
        case returnedCount = "returned_count"
        case totalCount = "total_count"
        case hasMore = "has_more"
        case nextCursor = "next_cursor"
        case sourceCompleteness = "source_completeness"
        case incompleteReason = "incomplete_reason"
    }

    var hasValidPageMetadata: Bool {
        returnedCount == installed.count
            && totalCount == installed.count
            && !hasMore
            && nextCursor == nil
            && sourceCompleteness == .enumerable
            && incompleteReason == nil
    }

    func listStatus(visibleCount: Int) -> ListCompletenessState {
        skillManagerListStatus(
            visibleCount: visibleCount,
            returnedCount: installed.count,
            totalCount: totalCount,
            serviceHasMore: hasMore,
            sourceCompleteness: sourceCompleteness,
            incompleteReason: incompleteReason
        )
    }


    var displayRecords: [OccurrenceIdentifiedItem<SkillManagerInstalledRecord>] {
        OccurrenceIdentifiedItem.rows(for: installed)
    }
}

private func skillManagerListStatus(
    visibleCount: Int,
    returnedCount: Int,
    totalCount: Int?,
    serviceHasMore: Bool,
    sourceCompleteness: ListSourceCompleteness,
    incompleteReason: ListIncompleteReason?
) -> ListCompletenessState {
    let hasHiddenReturnedRows = visibleCount < returnedCount
    let isComplete = !hasHiddenReturnedRows
        && !serviceHasMore
        && sourceCompleteness == .enumerable
        && incompleteReason == nil
        && (totalCount == nil || totalCount == returnedCount)
    let completeness: ListCompleteness
    if isComplete {
        completeness = .complete
    } else if sourceCompleteness == .limited || incompleteReason != nil {
        completeness = .incomplete
    } else if sourceCompleteness == .unknown {
        completeness = .unknown
    } else {
        completeness = .partial
    }
    return ListCompletenessState(
        loadedCount: visibleCount,
        totalCount: totalCount,
        hasMore: hasHiddenReturnedRows || serviceHasMore,
        isComplete: isComplete,
        completeness: completeness,
        incompleteReason: incompleteReason,
        loadingPhase: .idle,
        canLoadMore: hasHiddenReturnedRows,
        canLoadAll: hasHiddenReturnedRows
    )
}

struct SkillManagerInstallParams: Encodable {
    let source: String
    let skills: [String]
    let agents: [String]
    let scope: String?
    let distribution: String?
    let networkAllowed: Bool
    let confirmed: Bool
    let previewToken: String?
    let actionReference: ActionReferenceWire?

    enum CodingKeys: String, CodingKey {
        case source
        case skills
        case agents
        case scope
        case distribution
        case networkAllowed = "network_allowed"
        case confirmed
        case previewToken = "preview_token"
        case actionReference = "action_reference"
    }
}

struct SkillManagerRemoveParams: Encodable {
    let skill: String
    let agents: [String]
    let scope: String?
    let confirmed: Bool
    let previewToken: String?
    let actionReference: ActionReferenceWire?

    enum CodingKeys: String, CodingKey {
        case skill
        case agents
        case scope
        case confirmed
        case previewToken = "preview_token"
        case actionReference = "action_reference"
    }
}

struct SkillManagerUpdateParams: Encodable {
    let skills: [String]
    let agents: [String]
    let scope: String?
    let networkAllowed: Bool
    let confirmed: Bool
    let previewToken: String?
    let actionReference: ActionReferenceWire?

    enum CodingKeys: String, CodingKey {
        case skills
        case agents
        case scope
        case networkAllowed = "network_allowed"
        case confirmed
        case previewToken = "preview_token"
        case actionReference = "action_reference"
    }
}

struct SkillManagerLocalCreateParams: Encodable {
    let name: String
    let confirmed: Bool
    let previewToken: String?
    let actionReference: ActionReferenceWire?

    enum CodingKeys: String, CodingKey {
        case name
        case confirmed
        case previewToken = "preview_token"
        case actionReference = "action_reference"
    }
}

struct SkillManagerDeleteLocalParams: Encodable {
    let instanceId: String
    let confirmed: Bool
    let previewToken: String?
    let actionReference: ActionReferenceWire?

    enum CodingKeys: String, CodingKey {
        case instanceId = "instance_id"
        case confirmed
        case previewToken = "preview_token"
        case actionReference = "action_reference"
    }
}

struct SkillManagerLocalArchiveUpdateParams: Encodable {
    let instanceId: String
    let archivePath: String
    let confirmed: Bool
    let previewToken: String?

    enum CodingKeys: String, CodingKey {
        case instanceId = "instance_id"
        case archivePath = "archive_path"
        case confirmed
        case previewToken = "preview_token"
    }
}

struct SkillManagerLocalArchiveImportParams: Encodable {
    let archivePath: String
    let confirmed: Bool
    let previewToken: String?

    enum CodingKeys: String, CodingKey {
        case archivePath = "archive_path"
        case confirmed
        case previewToken = "preview_token"
    }
}

struct SkillManagerMutationRecord: Codable, Hashable {
    let preview: SkillManagerCommandPreview
    let output: SkillManagerCommandOutput?
    let applied: Bool
    let scannedCount: Int
    let updatedSkills: [SkillRecord]
    let readback: ActionReadbackWire?

    enum CodingKeys: String, CodingKey {
        case preview
        case output
        case applied
        case scannedCount = "scanned_count"
        case updatedSkills = "updated_skills"
        case readback
    }
}

struct SkillManagerLocalCreateRecord: Codable, Hashable {
    let preview: SkillManagerCommandPreview
    let output: SkillManagerCommandOutput?
    let imported: SkillRecord?
    let instanceId: String?
    let sourcePath: String
    let applied: Bool
    let readback: ActionReadbackWire?

    enum CodingKeys: String, CodingKey {
        case preview
        case output
        case imported
        case instanceId = "instance_id"
        case sourcePath = "source_path"
        case applied
        case readback
    }
}

struct SkillManagerLocalArchiveImportRecord: Codable, Hashable {
    let skillName: String
    let archivePath: String
    let archiveSha256: String
    let fileCount: Int
    let uncompressedBytes: UInt64
    let previewToken: String
    let confirmed: Bool
    let applied: Bool
    let summary: String
    let importedSkill: SkillRecord?
    let instanceID: String?

    enum CodingKeys: String, CodingKey {
        case skillName = "skill_name"
        case archivePath = "archive_path"
        case archiveSha256 = "archive_sha256"
        case fileCount = "file_count"
        case uncompressedBytes = "uncompressed_bytes"
        case previewToken = "preview_token"
        case confirmed
        case applied
        case summary
        case importedSkill = "imported_skill"
        case instanceID = "instance_id"
    }
}

struct SkillManagerReferenceRecord: Codable, Identifiable, Hashable {
    let instanceId: String
    let name: String
    let agent: String
    let scope: String
    let path: String

    var id: String { instanceId }

    enum CodingKeys: String, CodingKey {
        case instanceId = "instance_id"
        case name
        case agent
        case scope
        case path
    }
}

struct SkillManagerLocalDeleteRecord: Codable, Hashable {
    let action: ActionDescriptorWire?
    let previewToken: String?
    let instanceId: String
    let skillName: String
    let path: String
    let appOwned: Bool
    let physicalDeleteAllowed: Bool
    let blockedByReferences: [SkillManagerReferenceRecord]
    let confirmed: Bool
    let deleted: Bool
    let summary: String
    let readback: ActionReadbackWire?
    let followUp: SkillManagerCleanupFollowUp?

    enum CodingKeys: String, CodingKey {
        case action
        case previewToken = "preview_token"
        case instanceId = "instance_id"
        case skillName = "skill_name"
        case path
        case appOwned = "app_owned"
        case physicalDeleteAllowed = "physical_delete_allowed"
        case blockedByReferences = "blocked_by_references"
        case confirmed
        case deleted
        case summary
        case readback
        case followUp = "follow_up"
    }
}

struct SkillManagerCleanupFollowUp: Codable, Hashable {
    let kind: String
    let state: String
    let cleanupRequired: Bool
    let message: String

    enum CodingKeys: String, CodingKey {
        case kind
        case state
        case cleanupRequired = "cleanup_required"
        case message
    }
}

struct SkillManagerLocalArchiveUpdateRecord: Codable, Hashable {
    let instanceId: String
    let skillName: String
    let archivePath: String
    let archiveSha256: String
    let fileCount: Int
    let uncompressedBytes: UInt64
    let previewToken: String
    let confirmed: Bool
    let applied: Bool
    let summary: String
    let updatedSkill: SkillRecord?

    enum CodingKeys: String, CodingKey {
        case instanceId = "instance_id"
        case skillName = "skill_name"
        case archivePath = "archive_path"
        case archiveSha256 = "archive_sha256"
        case fileCount = "file_count"
        case uncompressedBytes = "uncompressed_bytes"
        case previewToken = "preview_token"
        case confirmed
        case applied
        case summary
        case updatedSkill = "updated_skill"
    }
}
