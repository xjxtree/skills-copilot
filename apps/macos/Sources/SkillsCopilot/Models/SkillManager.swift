import Foundation

enum SkillManagerArchiveDropModel {
    static func acceptedArchive(in urls: [URL]) -> URL? {
        guard urls.count == 1, let url = urls.first, url.isFileURL else { return nil }
        guard url.pathExtension.caseInsensitiveCompare("zip") == .orderedSame else { return nil }
        return url
    }
}

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
        [
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
            ),
            CompactMetadataRow(label: UIStrings.text("skillManager.token", "Token"), value: previewToken, systemImage: "key", isCopyable: true)
        ]
    }

    var requiresExplicitApplyConfirmation: Bool {
        requiresConfirmation && ["install", "remove", "update"].contains(operation)
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
    let agentTargets: [SkillManagerInventoryAgentTarget]
    let allInstanceIDs: [String]

    init(
        name: String,
        source: String?,
        scope: SkillManagerScope,
        agents: [String],
        origin: Origin,
        localOwnership: LocalOwnership?,
        localInstanceID: String?,
        localPath: String?,
        agentTargets: [SkillManagerInventoryAgentTarget] = [],
        allInstanceIDs: [String] = []
    ) {
        self.name = name
        self.source = source
        self.scope = scope
        self.agents = agents
        self.origin = origin
        self.localOwnership = localOwnership
        self.localInstanceID = localInstanceID
        self.localPath = localPath
        self.agentTargets = agentTargets
        self.allInstanceIDs = allInstanceIDs
    }

    var id: String {
        [scope.rawValue, origin.rawValue, name, localInstanceID ?? source ?? ""].joined(separator: "|")
    }

    var isInstalled: Bool { !agents.isEmpty }

    func instanceIDs(for selectedAgents: [String]) -> [String] {
        let selected = Set(selectedAgents)
        return agentTargets
            .filter { selected.contains($0.agent) }
            .flatMap(\.instanceIDs)
            .sorted()
    }

    func isCompleteRemovalSelection(_ selectedAgents: [String]) -> Bool {
        !agents.isEmpty && Set(selectedAgents) == Set(agents)
    }

    func missingDetachTargets(for selectedAgents: [String]) -> [String] {
        let available = Set(agentTargets.filter { !$0.instanceIDs.isEmpty }.map(\.agent))
        return SkillManagerAgent.defaultTargets
            .map(\.rawValue)
            .filter { selectedAgents.contains($0) && !available.contains($0) }
    }
}

struct SkillManagerInventoryAgentTarget: Hashable {
    let agent: String
    let instanceIDs: [String]
}

enum SkillManagerInventoryBuilder {
    private struct CatalogSource {
        let path: String
        let nameKey: String
        let representative: SkillRecord
        let agents: [String]
        let agentTargets: [SkillManagerInventoryAgentTarget]
        let allInstanceIDs: [String]
    }

    static func build(
        installed: [SkillManagerInstalledRecord],
        catalogSkills: [SkillRecord],
        localLibrarySkills: [SkillRecord],
        scope: SkillManagerScope
    ) -> [SkillManagerInventoryItem] {
        let eligibleSkills = editableCatalogSkills(from: catalogSkills, scope: scope)
        let eligibleSkillsByName = Dictionary(grouping: eligibleSkills, by: { normalizedName($0.name) })
        let catalogSources = editableCatalogSources(from: eligibleSkills)
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
            let matchingSkills = matchingCatalogSkills(
                for: record,
                source: catalogSource,
                candidates: eligibleSkillsByName[nameKey] ?? []
            )
            let targets = installedAgentTargets(from: matchingSkills)
            let affectedAgents = installedAgentIDs(
                reportedAgents: record.agents,
                matchingSkills: matchingSkills
            )
            return SkillManagerInventoryItem(
                name: record.name,
                source: localSource?.path ?? appOwnedSource.map(sourceDirectory) ?? record.source,
                scope: scope,
                agents: affectedAgents,
                origin: record.isLocalSource ? .local : .manager,
                localOwnership: record.isLocalSource
                    ? (localSource != nil ? localOwnership(for: scope) : (appOwnedSource == nil ? .external : .appOwned))
                    : nil,
                localInstanceID: localSource?.representative.id ?? appOwnedSource?.id,
                localPath: localSource?.path ?? appOwnedSource.map(sourceDirectory),
                agentTargets: targets,
                allInstanceIDs: canonicalInstanceIDs(matchingSkills.map(\.id))
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
                localPath: source.path,
                agentTargets: source.agentTargets,
                allInstanceIDs: source.allInstanceIDs
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
                localPath: path,
                agentTargets: [],
                allInstanceIDs: [skill.id]
            )
        })

        return items.sorted {
            let comparison = $0.name.localizedCaseInsensitiveCompare($1.name)
            if comparison != .orderedSame { return comparison == .orderedAscending }
            return $0.id < $1.id
        }
    }

    private static func editableCatalogSkills(
        from skills: [SkillRecord],
        scope: SkillManagerScope
    ) -> [SkillRecord] {
        skills.filter { skill in
            guard skill.state != "missing",
                  !DisplayText.isToolGlobal(skill),
                  skill.readOnlyReason == nil,
                  scopeMatches(skill.scope, scope: scope),
                  agentID(for: skill.agent) != nil else {
                return false
            }
            return true
        }
    }

    private static func editableCatalogSources(
        from skills: [SkillRecord]
    ) -> [CatalogSource] {
        let eligible = skills.compactMap { skill -> (String, SkillRecord)? in
            guard let path = sharedAgentsSourceDirectory(for: skill) else { return nil }
            return (path, skill)
        }
        return Dictionary(grouping: eligible, by: { $0.0 }).compactMap { path, rows in
            let skills = rows.map(\.1).sorted { $0.id < $1.id }
            guard let representative = skills.first else { return nil }
            return CatalogSource(
                path: path,
                nameKey: normalizedName(representative.name),
                representative: representative,
                agents: installedAgentTargets(from: skills).map(\.agent),
                agentTargets: installedAgentTargets(from: skills),
                allInstanceIDs: canonicalInstanceIDs(skills.map(\.id))
            )
        }
    }

    private static func matchingCatalogSkills(
        for record: SkillManagerInstalledRecord,
        source: CatalogSource?,
        candidates: [SkillRecord]
    ) -> [SkillRecord] {
        guard !candidates.isEmpty else { return [] }
        if let source {
            let sameDefinition = candidates.filter {
                $0.definitionId == source.representative.definitionId
            }
            if !sameDefinition.isEmpty { return sameDefinition }
        }
        if let pathSuffix = sharedAgentsPathSuffix(record.path ?? record.source) {
            let exact = candidates.filter {
                sharedAgentsPathSuffix(sourceDirectory(for: $0)) == pathSuffix
            }
            if !exact.isEmpty { return exact }
        }
        let definitionIDs = Set(candidates.map(\.definitionId))
        return definitionIDs.count == 1 ? candidates : (source.map { [$0.representative] } ?? [])
    }

    private static func installedAgentIDs(
        reportedAgents: [String],
        matchingSkills: [SkillRecord]
    ) -> [String] {
        var installed = Set(canonicalAgentIDs(reportedAgents))
        for skill in matchingSkills {
            if let agent = agentID(for: skill.agent) {
                installed.insert(agent)
            }
        }
        return SkillManagerAgent.defaultTargets.map(\.rawValue).filter(installed.contains)
    }

    private static func installedAgentTargets(
        from skills: [SkillRecord]
    ) -> [SkillManagerInventoryAgentTarget] {
        let grouped = Dictionary(grouping: skills, by: { agentID(for: $0.agent) })
        return SkillManagerAgent.defaultTargets.compactMap { agent in
            let instanceIDs = canonicalInstanceIDs((grouped[agent.rawValue] ?? []).map(\.id))
            guard !instanceIDs.isEmpty else { return nil }
            return SkillManagerInventoryAgentTarget(agent: agent.rawValue, instanceIDs: instanceIDs)
        }
    }

    private static func canonicalInstanceIDs(_ values: [String]) -> [String] {
        Array(Set(values.filter { !$0.isEmpty })).sorted()
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

    enum CodingKeys: String, CodingKey {
        case source
        case skills
        case agents
        case scope
        case distribution
        case networkAllowed = "network_allowed"
        case confirmed
        case previewToken = "preview_token"
    }
}

struct SkillManagerRemoveParams: Encodable {
    let skill: String
    let agents: [String]
    let instanceIDs: [String]
    let scope: String?
    let fullUninstall: Bool
    let confirmed: Bool
    let previewToken: String?

    enum CodingKeys: String, CodingKey {
        case skill
        case agents
        case instanceIDs = "instance_ids"
        case scope
        case fullUninstall = "full_uninstall"
        case confirmed
        case previewToken = "preview_token"
    }
}

struct SkillManagerUpdateParams: Encodable {
    let skills: [String]
    let agents: [String]
    let scope: String?
    let networkAllowed: Bool
    let confirmed: Bool
    let previewToken: String?

    enum CodingKeys: String, CodingKey {
        case skills
        case agents
        case scope
        case networkAllowed = "network_allowed"
        case confirmed
        case previewToken = "preview_token"
    }
}

struct SkillManagerLocalCreateParams: Encodable {
    let name: String
    let confirmed: Bool
    let previewToken: String?

    enum CodingKeys: String, CodingKey {
        case name
        case confirmed
        case previewToken = "preview_token"
    }
}

struct SkillManagerDeleteLocalParams: Encodable {
    let instanceId: String
    let confirmed: Bool

    enum CodingKeys: String, CodingKey {
        case instanceId = "instance_id"
        case confirmed
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
    let removalPlan: SkillManagerRemovalPlan?

    enum CodingKeys: String, CodingKey {
        case preview
        case output
        case applied
        case scannedCount = "scanned_count"
        case updatedSkills = "updated_skills"
        case removalPlan = "removal_plan"
    }
}

struct SkillManagerRemovalPlan: Codable, Hashable {
    let mode: String
    let fullUninstall: Bool
    let selectedAgents: [String]
    let instanceIDs: [String]
    let sourcePreserved: Bool
    let actions: [SkillManagerRemovalAction]
    let verification: String

    enum CodingKeys: String, CodingKey {
        case mode
        case fullUninstall = "full_uninstall"
        case selectedAgents = "selected_agents"
        case instanceIDs = "instance_ids"
        case sourcePreserved = "source_preserved"
        case actions
        case verification
    }

    var localizedRisk: String {
        fullUninstall
            ? UIStrings.text(
                "skillManager.remove.risk.complete",
                "Complete uninstall removes the canonical source and every target recognized by the external manager, including agents outside this app."
            )
            : UIStrings.text(
                "skillManager.remove.risk.partial",
                "Only exact selected symlinks or copied skill directories are removed; shared files and Agent enablement configuration stay unchanged."
            )
    }

    var localizedVerification: String {
        fullUninstall
            ? UIStrings.text(
                "skillManager.remove.verification.complete",
                "After refresh, the skill must no longer appear for any supported agent or in the external-manager inventory."
            )
            : UIStrings.text(
                "skillManager.remove.verification.partial",
                "After refresh, selected physical targets must be absent while the shared source and unselected Agent targets remain present."
            )
    }
}

struct SkillManagerRemovalAction: Codable, Hashable {
    let instanceID: String
    let agent: String
    let scope: String
    let strategy: String
    let target: String
    let summary: String

    enum CodingKeys: String, CodingKey {
        case instanceID = "instance_id"
        case agent
        case scope
        case strategy
        case target
        case summary
    }
}

struct SkillManagerLocalCreateRecord: Codable, Hashable {
    let preview: SkillManagerCommandPreview
    let output: SkillManagerCommandOutput?
    let imported: SkillRecord?
    let instanceId: String?
    let sourcePath: String
    let applied: Bool

    enum CodingKeys: String, CodingKey {
        case preview
        case output
        case imported
        case instanceId = "instance_id"
        case sourcePath = "source_path"
        case applied
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
    let instanceId: String
    let skillName: String
    let path: String
    let appOwned: Bool
    let physicalDeleteAllowed: Bool
    let blockedByReferences: [SkillManagerReferenceRecord]
    let confirmed: Bool
    let deleted: Bool
    let summary: String

    enum CodingKeys: String, CodingKey {
        case instanceId = "instance_id"
        case skillName = "skill_name"
        case path
        case appOwned = "app_owned"
        case physicalDeleteAllowed = "physical_delete_allowed"
        case blockedByReferences = "blocked_by_references"
        case confirmed
        case deleted
        case summary
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
