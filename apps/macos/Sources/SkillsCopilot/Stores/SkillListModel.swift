import Foundation

enum SidebarContentMode: String, CaseIterable, Identifiable {
    case sessions
    case skills
    case config

    var id: String { rawValue }

    var title: String {
        switch self {
        case .sessions:
            return UIStrings.text("sidebar.mode.sessions", "Sessions")
        case .skills:
            return UIStrings.skills
        case .config:
            return UIStrings.text("sidebar.mode.config", "Config")
        }
    }

    var systemImage: String {
        switch self {
        case .sessions:
            return "bubble.left.and.text.bubble.right"
        case .skills:
            return "square.stack.3d.up"
        case .config:
            return "slider.horizontal.3"
        }
    }
}

enum AgentConfigScopeFilter: String, CaseIterable, Identifiable {
    case all
    case global
    case project
    case other

    var id: String { rawValue }

    var title: String {
        switch self {
        case .all:
            return UIStrings.text("filter.allScopes", "All scopes")
        case .global:
            return UIStrings.text("filter.globalConfig", "Global config")
        case .project:
            return UIStrings.text("filter.projectConfig", "Project config")
        case .other:
            return UIStrings.text("filter.otherConfig", "Other supported")
        }
    }

    func includes(_ snapshot: ConfigSnapshotRecord) -> Bool {
        let scope = snapshot.scope.lowercased()
        switch self {
        case .all:
            return true
        case .global:
            return scope.contains("global")
        case .project:
            return scope.contains("project")
        case .other:
            return !scope.contains("global") && !scope.contains("project")
        }
    }

    func includes(_ document: ConfigDocumentRecord) -> Bool {
        let scope = document.scope.lowercased()
        switch self {
        case .all:
            return true
        case .global:
            return scope.contains("global")
        case .project:
            return scope.contains("project")
        case .other:
            return !scope.contains("global") && !scope.contains("project")
        }
    }
}

enum LocalSessionScopeFilter: String, CaseIterable, Identifiable {
    case project
    case all

    var id: String { rawValue }

    var title: String {
        switch self {
        case .project:
            return UIStrings.text("sidebar.sessions.scope.project", "Project")
        case .all:
            return UIStrings.text("sidebar.sessions.scope.all", "All")
        }
    }
}

enum LocalSessionSortOrder: String, CaseIterable, Identifiable {
    case recent
    case title

    var id: String { rawValue }

    var title: String {
        switch self {
        case .recent:
            return UIStrings.text("sidebar.sessions.sort.recent", "Recent")
        case .title:
            return UIStrings.text("sidebar.sessions.sort.title", "Title")
        }
    }
}

enum SkillStateFilter: String, CaseIterable, Identifiable {
    case all
    case risky
    case enabled
    case disabled
    case broken
    case missing
    case shadowed
    case unknown
    case withFindings
    case withConflicts

    var id: String { rawValue }

    static let sidebarCases: [SkillStateFilter] = [
        .all,
        .enabled,
        .disabled,
        .withFindings,
        .withConflicts,
        .missing,
    ]

    var title: String {
        switch self {
        case .all:
            return UIStrings.text("filter.all", "All")
        case .risky:
            return UIStrings.text("filter.risky", "Risky")
        case .enabled:
            return UIStrings.text("filter.enabled", "Enabled")
        case .disabled:
            return UIStrings.text("filter.disabled", "Disabled")
        case .broken:
            return UIStrings.stateBroken
        case .missing:
            return UIStrings.stateMissing
        case .shadowed:
            return UIStrings.stateShadowed
        case .unknown:
            return UIStrings.stateUnknown
        case .withFindings:
            return UIStrings.findings
        case .withConflicts:
            return UIStrings.text("filter.conflicts", "Conflicts")
        }
    }
}

enum SkillScopeFilter: String, CaseIterable, Identifiable {
    case all
    case project
    case global

    var id: String { rawValue }

    var title: String {
        switch self {
        case .all:
            return UIStrings.text("filter.allScopes", "All scopes")
        case .project:
            return UIStrings.text("filter.projectSkills", "Project skills")
        case .global:
            return UIStrings.text("filter.globalSkills", "Global skills")
        }
    }

    func includes(_ skill: SkillRecord) -> Bool {
        switch self {
        case .all:
            return true
        case .project:
            return skill.provenance.scopeKind == .project
        case .global:
            return skill.provenance.scopeKind == .global || skill.provenance.scopeKind == .toolGlobal
        }
    }
}

enum SkillAgentFilter: String, CaseIterable, Identifiable {
    case all
    case claudeCode = "claude-code"
    case codex
    case opencode
    case pi
    case hermes
    case openclaw

    var id: String { rawValue }

    static let managementCases: [SkillAgentFilter] = [
        .claudeCode,
        .codex,
        .opencode,
        .pi,
        .hermes,
        .openclaw
    ]

    var title: String {
        switch self {
        case .all:
            return UIStrings.text("filter.all", "All")
        case .claudeCode:
            return UIStrings.claudeCode
        case .codex:
            return UIStrings.codex
        case .opencode:
            return UIStrings.opencode
        case .pi:
            return UIStrings.pi
        case .hermes:
            return UIStrings.hermes
        case .openclaw:
            return UIStrings.openclaw
        }
    }

    func includes(_ skill: SkillRecord) -> Bool {
        switch self {
        case .all:
            return true
        case .claudeCode, .codex, .opencode, .pi, .hermes, .openclaw:
            return skill.agent == rawValue
        }
    }
}

enum SkillSortDirection: String, CaseIterable, Identifiable {
    case ascending
    case descending

    var id: String { rawValue }

    var title: String {
        switch self {
        case .ascending:
            return UIStrings.text("sort.ascending", "Ascending")
        case .descending:
            return UIStrings.text("sort.descending", "Descending")
        }
    }
}

enum SkillSortOrder: String, CaseIterable, Identifiable {
    case name
    case scope
    case state
    case path

    var id: String { rawValue }

    var title: String {
        switch self {
        case .name:
            return UIStrings.text("sort.name", "Name")
        case .scope:
            return UIStrings.scope
        case .state:
            return UIStrings.state
        case .path:
            return UIStrings.text("sort.path", "Path")
        }
    }
}

struct SkillAgentGroup: Identifiable, Hashable {
    let agent: String
    let title: String
    let skills: [SkillRecord]

    var id: String { agent }
}

enum SkillListModel {
    static func isDeleted(_ skill: SkillRecord) -> Bool {
        DisplayText.statusKind(skill.state, enabled: skill.enabled) == .missing
    }

    static func currentSkills(_ skills: [SkillRecord]) -> [SkillRecord] {
        skills.filter { !isDeleted($0) }
    }

    static func filteredAndSorted(
        skills: [SkillRecord],
        findings: [RuleFindingRecord],
        conflicts: [ConflictGroupRecord],
        searchText: String,
        agentFilter: SkillAgentFilter,
        stateFilter: SkillStateFilter,
        scopeFilter: SkillScopeFilter = .all,
        sortOrder: SkillSortOrder,
        sortDirection: SkillSortDirection = .ascending,
        issueIndex providedIssueIndex: SkillIssueIndex? = nil
    ) -> [SkillRecord] {
        let query = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        let asciiNeedle = foldedASCIIBytes(query)
        var cachedIssueIndex = providedIssueIndex
        func listIssueIndex() -> SkillIssueIndex {
            if let cachedIssueIndex {
                return cachedIssueIndex
            }
            let index = issueIndex(skills: skills, findings: findings, conflicts: conflicts)
            cachedIssueIndex = index
            return index
        }
        let searched = query.isEmpty
            ? skills
            : skills.filter {
                matchesNormalizedSearchQuery($0, query: query, asciiNeedle: asciiNeedle)
            }
        let filtered = searched.filter { skill in
            guard agentFilter.includes(skill) else {
                return false
            }
            guard scopeFilter.includes(skill) else {
                return false
            }
            let status = DisplayText.statusKind(skill.state, enabled: skill.enabled)
            guard status != .missing || stateFilter == .missing else {
                return false
            }
            switch stateFilter {
            case .all:
                return true
            case .enabled:
                return status == .enabled
            case .disabled:
                return status == .disabled
            case .broken:
                return status == .broken
            case .missing:
                return status == .missing
            case .shadowed:
                return status == .shadowed
            case .unknown:
                return status == .unknown
            case .withFindings:
                let issueIndex = listIssueIndex()
                return issueIndex.findingInstanceIDs.contains(skill.id)
                    || status == .broken
                    || status == .unknown
            case .withConflicts:
                return listIssueIndex().sameAgentConflictInstanceIDs.contains(skill.id)
            case .risky:
                return listIssueIndex().riskyFindingInstanceIDs.contains(skill.id)
            }
        }
        let sorted = sortOrder == .path ? sortByPath(filtered) : filtered.sorted { lhs, rhs in
            switch sortOrder {
            case .name:
                return compare(lhs.name, rhs.name)
            case .scope:
                if lhs.scope != rhs.scope {
                    return compare(lhs.scope, rhs.scope)
                }
                return compare(lhs.name, rhs.name)
            case .state:
                let lhsRank = DisplayText.stateSortRank(lhs.state, enabled: lhs.enabled)
                let rhsRank = DisplayText.stateSortRank(rhs.state, enabled: rhs.enabled)
                if lhsRank != rhsRank {
                    return lhsRank < rhsRank
                }
                let lhsState = DisplayText.state(lhs.state, enabled: lhs.enabled)
                let rhsState = DisplayText.state(rhs.state, enabled: rhs.enabled)
                if lhsState != rhsState { return compare(lhsState, rhsState) }
                return compare(lhs.name, rhs.name)
            case .path:
                return compare(lhs.displayPath, rhs.displayPath)
            }
        }
        switch sortDirection {
        case .ascending:
            return sorted
        case .descending:
            return Array(sorted.reversed())
        }
    }

    static func matchesSearchQuery(_ skill: SkillRecord, query rawQuery: String) -> Bool {
        let query = rawQuery.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else { return true }
        return matchesNormalizedSearchQuery(
            skill,
            query: query,
            asciiNeedle: foldedASCIIBytes(query)
        )
    }

    private static func matchesNormalizedSearchQuery(
        _ skill: SkillRecord,
        query: String,
        asciiNeedle: [UInt8]?
    ) -> Bool {
        containsSearchQuery(skill.name, query: query, asciiNeedle: asciiNeedle)
            || containsSearchQuery(skill.definitionId, query: query, asciiNeedle: asciiNeedle)
            || containsSearchQuery(skill.displayPath, query: query, asciiNeedle: asciiNeedle)
            || containsSearchQuery(agentSearchText(for: skill.agent), query: query, asciiNeedle: asciiNeedle)
            || containsSearchQuery(skill.scope, query: query, asciiNeedle: asciiNeedle)
    }

    private static func containsSearchQuery(
        _ value: String,
        query: String,
        asciiNeedle: [UInt8]?
    ) -> Bool {
        if let asciiNeedle,
           let result = asciiCaseInsensitiveContains(value, foldedNeedle: asciiNeedle) {
            return result
        }
        return value.localizedCaseInsensitiveContains(query)
    }

    private static func foldedASCIIBytes(_ value: String) -> [UInt8]? {
        let bytes = Array(value.utf8)
        guard bytes.allSatisfy({ $0 < 0x80 }) else { return nil }
        return bytes.map(foldASCIIByte)
    }

    private static func asciiCaseInsensitiveContains(
        _ value: String,
        foldedNeedle: [UInt8]
    ) -> Bool? {
        let storageResult: Bool?? = value.utf8.withContiguousStorageIfAvailable { haystack in
            guard haystack.allSatisfy({ $0 < 0x80 }) else { return nil }
            guard foldedNeedle.count <= haystack.count else { return false }
            guard !foldedNeedle.isEmpty else { return true }

            for start in 0...(haystack.count - foldedNeedle.count) {
                var matches = true
                for offset in foldedNeedle.indices
                where foldASCIIByte(haystack[start + offset]) != foldedNeedle[offset] {
                    matches = false
                    break
                }
                if matches {
                    return true
                }
            }
            return false
        }
        if let storageResult {
            return storageResult
        }
        return nil
    }

    private static func foldASCIIByte(_ byte: UInt8) -> UInt8 {
        byte >= 0x41 && byte <= 0x5A ? byte + 0x20 : byte
    }

    private static func sortByPath(_ skills: [SkillRecord]) -> [SkillRecord] {
        skills
            .map { skill in
                (skill: skill, asciiKey: foldedASCIIBytes(skill.displayPath))
            }
            .sorted { lhs, rhs in
                if let lhsKey = lhs.asciiKey,
                   let rhsKey = rhs.asciiKey,
                   let result = asciiCaseInsensitiveSortDecision(lhsKey, rhsKey) {
                    return result
                }
                return compare(lhs.skill.displayPath, rhs.skill.displayPath)
            }
            .map { $0.skill }
    }

    private static func asciiCaseInsensitiveSortDecision(
        _ lhs: [UInt8],
        _ rhs: [UInt8]
    ) -> Bool? {
        for offset in 0..<min(lhs.count, rhs.count) {
            guard lhs[offset] != rhs[offset] else { continue }
            guard let lhsClass = asciiSortClass(lhs[offset]),
                  lhsClass == asciiSortClass(rhs[offset]) else {
                return nil
            }
            return lhs[offset] < rhs[offset]
        }
        return lhs.count < rhs.count
    }

    private static func asciiSortClass(_ byte: UInt8) -> UInt8? {
        if byte >= 0x61, byte <= 0x7A {
            return 0
        }
        if byte >= 0x30, byte <= 0x39 {
            return 1
        }
        return nil
    }

    struct SkillIssueIndex {
        let displayFindings: [RuleFindingRecord]
        let findingInstanceIDs: Set<SkillRecord.ID>
        let riskyFindingInstanceIDs: Set<SkillRecord.ID>
        let sameAgentConflictInstanceIDs: Set<SkillRecord.ID>
        let issueCountsBySkillID: [SkillRecord.ID: Int]
        let conflictCountsBySkillID: [SkillRecord.ID: Int]

        func issueCount(for skillID: SkillRecord.ID) -> Int {
            issueCountsBySkillID[skillID] ?? 0
        }

        func conflictCount(for skillID: SkillRecord.ID) -> Int {
            conflictCountsBySkillID[skillID] ?? 0
        }
    }

    static func issueIndex(
        skills: [SkillRecord],
        findings: [RuleFindingRecord],
        conflicts: [ConflictGroupRecord]
    ) -> SkillIssueIndex {
        let displayFindings = displayFindings(skills: skills, findings: findings)
        let sameAgentConflictGroups = sameAgentConflictGroups(skills: skills, conflicts: conflicts)
        var findingInstanceIDs = Set<SkillRecord.ID>()
        var riskyFindingInstanceIDs = Set<SkillRecord.ID>()
        var sameAgentConflictInstanceIDs = Set<SkillRecord.ID>()
        var findingCountsBySkillID: [SkillRecord.ID: Int] = [:]
        var conflictCountsBySkillID: [SkillRecord.ID: Int] = [:]

        for finding in displayFindings {
            guard let instanceID = finding.instanceId else { continue }
            findingInstanceIDs.insert(instanceID)
            findingCountsBySkillID[instanceID, default: 0] += 1
            if isRiskFinding(finding) {
                riskyFindingInstanceIDs.insert(instanceID)
            }
        }

        for group in sameAgentConflictGroups {
            for instanceID in group {
                sameAgentConflictInstanceIDs.insert(instanceID)
                conflictCountsBySkillID[instanceID, default: 0] += 1
            }
        }

        var issueCountsBySkillID: [SkillRecord.ID: Int] = [:]
        issueCountsBySkillID.reserveCapacity(skills.count)
        for skill in skills {
            let statusIssueCount = catalogStatusIssueKind(for: skill) == nil ? 0 : 1
            let count = (findingCountsBySkillID[skill.id] ?? 0)
                + statusIssueCount
            if count > 0 {
                issueCountsBySkillID[skill.id] = count
            }
        }

        return SkillIssueIndex(
            displayFindings: displayFindings,
            findingInstanceIDs: findingInstanceIDs,
            riskyFindingInstanceIDs: riskyFindingInstanceIDs,
            sameAgentConflictInstanceIDs: sameAgentConflictInstanceIDs,
            issueCountsBySkillID: issueCountsBySkillID,
            conflictCountsBySkillID: conflictCountsBySkillID
        )
    }

    static func catalogStatusIssueKind(for skill: SkillRecord) -> SkillStatusKind? {
        let status = DisplayText.statusKind(skill.state, enabled: skill.enabled)
        switch status {
        case .broken, .missing, .unknown:
            return status
        case .enabled, .disabled, .shadowed:
            return nil
        }
    }

    static func displayFindings(
        skills: [SkillRecord],
        findings: [RuleFindingRecord]
    ) -> [RuleFindingRecord] {
        let currentSkillIDs = Set(skills.map(\.id))
        return findings.filter { finding in
            guard let instanceID = finding.instanceId,
                  currentSkillIDs.contains(instanceID) else {
                return false
            }
            guard isVisibleFinding(finding) else {
                return false
            }
            guard !conflictOnlyRuleIDs.contains(normalizedRuleID(finding.ruleId)) else {
                return false
            }
            if isBaselineRuleFinding(finding) {
                return false
            }
            return true
        }
    }

    static func displayIssueCount(
        skills: [SkillRecord],
        findings: [RuleFindingRecord],
        conflicts: [ConflictGroupRecord],
        agentFilter: SkillAgentFilter
    ) -> Int {
        let issueIndex = issueIndex(skills: skills, findings: findings, conflicts: conflicts)
        return skills
            .filter { agentFilter.includes($0) && !isDeleted($0) }
            .reduce(0) { count, skill in
                count + issueIndex.issueCount(for: skill.id)
            }
    }

    static func groupedByAgent(_ skills: [SkillRecord]) -> [SkillAgentGroup] {
        let grouped = Dictionary(grouping: skills, by: \.agent)
        return grouped
            .map { agent, skills in
                SkillAgentGroup(agent: agent, title: DisplayText.agent(agent), skills: skills)
            }
            .sorted { lhs, rhs in
                let lhsRank = agentRank(lhs.agent)
                let rhsRank = agentRank(rhs.agent)
                if lhsRank != rhsRank {
                    return lhsRank < rhsRank
                }
                return compare(lhs.title, rhs.title)
            }
    }

    static func issueIndicatorCount(
        for skill: SkillRecord,
        skills: [SkillRecord],
        findings: [RuleFindingRecord],
        conflicts: [ConflictGroupRecord]
    ) -> Int {
        issueIndex(skills: skills, findings: findings, conflicts: conflicts)
            .issueCount(for: skill.id)
    }

    static func adoptingAgentSummaryBySkillID(for skills: [SkillRecord]) -> [SkillRecord.ID: String] {
        var agentsByIdentity: [String: Set<String>] = [:]
        var identityKeysBySkillID: [SkillRecord.ID: Set<String>] = [:]

        for skill in skills {
            let keys = identityKeys(for: skill)
            identityKeysBySkillID[skill.id] = keys

            for key in keys {
                agentsByIdentity[key, default: []].insert(DisplayText.agent(skill.agent))
            }
        }

        return skills.reduce(into: [SkillRecord.ID: String]()) { partialResult, skill in
            let keys = identityKeysBySkillID[skill.id] ?? []
            let agents = keys
                .flatMap { agentsByIdentity[$0] ?? [] }
                .reduce(into: Set<String>()) { partialAgents, agent in
                    partialAgents.insert(agent)
                }
                .sorted { lhs, rhs in
                    lhs.localizedStandardCompare(rhs) == .orderedAscending
                }
            let displayAgents = agents.isEmpty ? [DisplayText.agent(skill.agent)] : agents
            partialResult[skill.id] = displayAgents.joined(separator: ", ")
        }
    }

    private static func compare(_ lhs: String, _ rhs: String) -> Bool {
        lhs.localizedCaseInsensitiveCompare(rhs) == .orderedAscending
    }

    private static func agentRank(_ agent: String) -> Int {
        switch agent {
        case SkillAgentFilter.claudeCode.rawValue:
            return 0
        case SkillAgentFilter.codex.rawValue:
            return 1
        case SkillAgentFilter.opencode.rawValue:
            return 2
        case SkillAgentFilter.pi.rawValue:
            return 3
        case SkillAgentFilter.hermes.rawValue:
            return 4
        case SkillAgentFilter.openclaw.rawValue:
            return 5
        default:
            return 6
        }
    }

    private static func agentSearchText(for agent: String) -> String {
        var aliases = [agent, DisplayText.agent(agent)]
        switch agent {
        case SkillAgentFilter.claudeCode.rawValue:
            aliases.append("claude")
        case SkillAgentFilter.opencode.rawValue:
            aliases.append("open code")
        case SkillAgentFilter.pi.rawValue:
            aliases.append("pi coding agent")
        case SkillAgentFilter.openclaw.rawValue:
            aliases.append("open claw")
        default:
            break
        }
        return aliases.joined(separator: " ")
    }

    private static func identityKeys(for skill: SkillRecord) -> Set<String> {
        var keys = Set<String>()
        let definition = normalizedIdentityValue(skill.definitionId)
        if !definition.isEmpty {
            keys.insert("definition:\(definition)")
        }
        let name = normalizedIdentityValue(skill.name)
        if !name.isEmpty {
            keys.insert("name:\(name)")
        }
        return keys
    }

    private static func normalizedIdentityValue(_ value: String) -> String {
        value.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    }

    private static func isRiskFinding(_ finding: RuleFindingRecord) -> Bool {
        finding.isRiskCategoryFinding
    }

    private static func isVisibleFinding(_ finding: RuleFindingRecord) -> Bool {
        !finding.suppressed && FindingTriageFilter.active.includes(finding.triageState)
    }

    private static let conflictOnlyRuleIDs: Set<String> = ["name.collision"]
    private static let baselineRuleIDs: Set<String> = [
        "frontmatter.tools-not-empty",
        "permissions.network-declared",
        "permissions.exec-needs-human",
    ]

    private static func isBaselineRuleFinding(_ finding: RuleFindingRecord) -> Bool {
        baselineRuleIDs.contains(normalizedRuleID(finding.ruleId))
            && isBaselineSeverity(finding.effectiveSeverity ?? finding.severity)
    }

    private static func normalizedRuleID(_ ruleID: String) -> String {
        ruleID.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    }

    private static func isBaselineSeverity(_ severity: String) -> Bool {
        switch severity.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "critical", "error":
            return false
        default:
            return true
        }
    }

    static func sameAgentConflictGroupCount(
        skills: [SkillRecord],
        conflicts: [ConflictGroupRecord]
    ) -> Int {
        sameAgentConflictGroups(skills: skills, conflicts: conflicts).count
    }

    static func sameAgentConflictInstanceIDs(
        skills: [SkillRecord],
        conflicts: [ConflictGroupRecord]
    ) -> Set<String> {
        sameAgentConflictGroups(skills: skills, conflicts: conflicts)
            .reduce(into: Set<String>()) { ids, groupIDs in
                ids.formUnion(groupIDs)
            }
    }

    private static func sameAgentConflictGroups(
        skills: [SkillRecord],
        conflicts: [ConflictGroupRecord]
    ) -> [[String]] {
        let agentBySkillID = Dictionary(uniqueKeysWithValues: skills.map { ($0.id, $0.agent) })
        var groups: [[String]] = []
        for conflict in conflicts {
            let groupedIDs = Dictionary(grouping: conflict.instanceIds) { instanceID in
                agentBySkillID[instanceID]
            }
            for (agent, instanceIDs) in groupedIDs where agent != nil && instanceIDs.count > 1 {
                groups.append(instanceIDs)
            }
        }
        return groups
    }
}

enum AgentConfigSidebarModel {
    static func filteredSnapshots(
        _ snapshots: [ConfigSnapshotRecord],
        agentFilter: SkillAgentFilter,
        scopeFilter: AgentConfigScopeFilter,
        searchText: String
    ) -> [ConfigSnapshotRecord] {
        let query = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        return snapshots
            .filter { snapshot in
                (agentFilter == .all || snapshot.agent == agentFilter.rawValue)
                    && scopeFilter.includes(snapshot)
                    && matches(snapshot, query: query)
            }
            .sorted { lhs, rhs in
                if lhs.createdAt != rhs.createdAt {
                    return lhs.createdAt > rhs.createdAt
                }
                return lhs.id > rhs.id
            }
    }

    private static func matches(_ snapshot: ConfigSnapshotRecord, query: String) -> Bool {
        guard !query.isEmpty else { return true }
        return [
            snapshot.agent,
            snapshot.scope,
            snapshot.target,
            snapshot.reason,
            DisplayText.timestamp(snapshot.createdAt),
        ].contains { value in
            value.localizedCaseInsensitiveContains(query)
        }
    }
}
