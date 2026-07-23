import Foundation

/// A typed, UI-only handoff into the existing Skill Manager workflows.
///
/// This context may choose presentation state, but it is never an action
/// authorization. Every consequential operation still starts at its existing
/// `skillManager.*` preview method and requires the service-owned confirmation
/// before apply.
enum SkillManagerEntryIntent: String, Hashable {
    case browse
    case add
    case packageDetail = "package-detail"
    case install
    case update
    case remove
    case localCreate = "local-create"
    case importArchive = "import-archive"
}

enum SkillManagerEntryAction: String, Identifiable, Hashable {
    case install
    case update
    case remove
    case deleteSource = "delete-source"

    var id: String { rawValue }
}

struct SkillManagerPackageTarget: Hashable {
    let inventoryItemID: String?
    let name: String
    let instanceIDs: Set<String>
    let scope: SkillManagerScope?

    init(
        inventoryItemID: String? = nil,
        name: String,
        instanceIDs: [String] = [],
        scope: SkillManagerScope? = nil
    ) {
        self.inventoryItemID = Self.trimmed(inventoryItemID)
        self.name = name.trimmingCharacters(in: .whitespacesAndNewlines)
        self.instanceIDs = Set(instanceIDs.compactMap(Self.trimmed))
        self.scope = scope
    }

    init(
        aggregate: SkillAggregateRecord,
        preferredScope: SkillManagerScope? = nil
    ) {
        self.init(
            name: aggregate.canonicalName,
            instanceIDs: aggregate.instanceIDs,
            scope: preferredScope ?? Self.unambiguousScope(for: aggregate.scopes)
        )
    }

    func matches(_ item: SkillManagerInventoryItem) -> Bool {
        matchRank(for: item) != nil
    }

    /// Resolves only a unique best candidate. An exact inventory identity wins
    /// over a catalog instance identity, which wins over the name fallback.
    /// Same-name rows at the same scope intentionally remain unselected.
    func uniqueBestMatch(
        in items: [SkillManagerInventoryItem]
    ) -> SkillManagerInventoryItem? {
        let ranked = items.compactMap { item in
            matchRank(for: item).map { (rank: $0, item: item) }
        }
        guard let bestRank = ranked.map(\.rank).max() else { return nil }
        let best = ranked.filter { $0.rank == bestRank }
        return best.count == 1 ? best[0].item : nil
    }

    private func matchRank(for item: SkillManagerInventoryItem) -> Int? {
        guard scope == nil || item.scope == scope else { return nil }
        if inventoryItemID == item.id { return 3 }
        if let localInstanceID = item.localInstanceID,
           instanceIDs.contains(localInstanceID) {
            return 2
        }
        return !name.isEmpty
            && item.name.caseInsensitiveCompare(name) == .orderedSame
            ? 1
            : nil
    }

    private static func unambiguousScope(
        for scopes: [ProductScope]
    ) -> SkillManagerScope? {
        let managerScopes = Set(scopes.compactMap { scope -> SkillManagerScope? in
            switch scope {
            case .agentProject:
                return .project
            case .agentGlobal, .toolGlobal:
                return .global
            }
        })
        return managerScopes.count == 1 ? managerScopes.first : nil
    }

    private static func trimmed(_ value: String?) -> String? {
        guard let trimmed = value?.trimmingCharacters(in: .whitespacesAndNewlines),
              !trimmed.isEmpty else {
            return nil
        }
        return trimmed
    }
}

struct SkillManagerEntryContext: Hashable {
    static let `default` = SkillManagerEntryContext()

    let intent: SkillManagerEntryIntent
    let target: SkillManagerPackageTarget?
    let scope: SkillManagerScope?
    /// `nil` means derive normal defaults from the selected package.
    /// An explicit empty list means no target agents are selected.
    let agentIDs: [String]?
    let searchQuery: String?
    let suggestedLocalSkillName: String?

    init(
        intent: SkillManagerEntryIntent = .browse,
        target: SkillManagerPackageTarget? = nil,
        scope: SkillManagerScope? = nil,
        agentIDs: [String]? = nil,
        searchQuery: String? = nil,
        suggestedLocalSkillName: String? = nil
    ) {
        self.intent = intent
        self.target = target.map {
            SkillManagerPackageTarget(
                inventoryItemID: $0.inventoryItemID,
                name: $0.name,
                instanceIDs: Array($0.instanceIDs),
                scope: scope ?? $0.scope
            )
        }
        self.scope = scope
        self.agentIDs = agentIDs.map(Self.canonicalAgentIDs)
        self.searchQuery = Self.trimmed(searchQuery)
        self.suggestedLocalSkillName = Self.trimmed(suggestedLocalSkillName)
    }

    static func add(
        query: String? = nil,
        scope: SkillManagerScope = .project,
        agentIDs: [String]? = nil
    ) -> SkillManagerEntryContext {
        SkillManagerEntryContext(
            intent: .add,
            scope: scope,
            agentIDs: agentIDs,
            searchQuery: query
        )
    }

    static func packageDetail(
        target: SkillManagerPackageTarget,
        scope: SkillManagerScope? = nil
    ) -> SkillManagerEntryContext {
        SkillManagerEntryContext(
            intent: .packageDetail,
            target: target,
            scope: scope
        )
    }

    static func install(
        target: SkillManagerPackageTarget,
        scope: SkillManagerScope? = nil,
        agentIDs: [String]? = nil
    ) -> SkillManagerEntryContext {
        SkillManagerEntryContext(
            intent: .install,
            target: target,
            scope: scope,
            agentIDs: agentIDs
        )
    }

    static func update(
        target: SkillManagerPackageTarget,
        scope: SkillManagerScope? = nil
    ) -> SkillManagerEntryContext {
        SkillManagerEntryContext(
            intent: .update,
            target: target,
            scope: scope
        )
    }

    static func remove(
        target: SkillManagerPackageTarget,
        scope: SkillManagerScope? = nil,
        agentIDs: [String]? = nil
    ) -> SkillManagerEntryContext {
        SkillManagerEntryContext(
            intent: .remove,
            target: target,
            scope: scope,
            agentIDs: agentIDs
        )
    }

    static func localCreate(
        suggestedName: String? = nil
    ) -> SkillManagerEntryContext {
        SkillManagerEntryContext(
            intent: .localCreate,
            suggestedLocalSkillName: suggestedName
        )
    }

    static var importArchive: SkillManagerEntryContext {
        SkillManagerEntryContext(intent: .importArchive)
    }

    static func managerAgentIDs(
        for productAgents: [ProductAgentID]
    ) -> [String] {
        productAgents.compactMap { agent -> String? in
            switch agent {
            case .toolGlobal:
                return nil
            case .claudeCode:
                return SkillManagerAgent.claudeCode.rawValue
            case .codex:
                return SkillManagerAgent.codex.rawValue
            case .pi:
                return SkillManagerAgent.pi.rawValue
            case .hermes:
                return SkillManagerAgent.hermesAgent.rawValue
            case .openclaw:
                return SkillManagerAgent.openclaw.rawValue
            case .opencode:
                return SkillManagerAgent.opencode.rawValue
            }
        }
    }

    var presentation: SkillManagerEntryPresentation {
        SkillManagerEntryPresentation(context: self)
    }

    private static func canonicalAgentIDs(_ values: [String]) -> [String] {
        let supported = Set(SkillManagerAgent.defaultTargets.map(\.rawValue))
        return Array(
            Set(
                values
                    .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                    .filter(supported.contains)
            )
        ).sorted()
    }

    private static func trimmed(_ value: String?) -> String? {
        guard let trimmed = value?.trimmingCharacters(in: .whitespacesAndNewlines),
              !trimmed.isEmpty else {
            return nil
        }
        return trimmed
    }
}

struct SkillManagerEntryPresentation: Hashable {
    let workflow: SkillManagerWorkflow
    let preferredAction: SkillManagerEntryAction?
    let scope: SkillManagerScope
    let agentIDs: Set<String>?
    let inventoryQuery: String
    let searchQuery: String?
    let suggestedLocalSkillName: String?
    let focusedInput: FocusedInput?
    let requestsImportArchive: Bool

    enum FocusedInput: Hashable {
        case search
        case localCreate
    }

    init(context: SkillManagerEntryContext) {
        switch context.intent {
        case .browse, .add, .localCreate, .importArchive:
            workflow = .searchInstall
        case .packageDetail, .install, .update, .remove:
            workflow = .installedUpdates
        }

        switch context.intent {
        case .install:
            preferredAction = .install
        case .update:
            preferredAction = .update
        case .remove:
            preferredAction = .remove
        case .browse, .add, .packageDetail, .localCreate, .importArchive:
            preferredAction = nil
        }

        scope = context.scope ?? context.target?.scope ?? .project
        agentIDs = context.agentIDs.map(Set.init)
        inventoryQuery = context.target?.name ?? ""
        searchQuery = context.searchQuery
        suggestedLocalSkillName = context.suggestedLocalSkillName
        requestsImportArchive = context.intent == .importArchive

        switch context.intent {
        case .add:
            focusedInput = .search
        case .localCreate:
            focusedInput = .localCreate
        case .browse, .packageDetail, .install, .update, .remove, .importArchive:
            focusedInput = nil
        }
    }

    func resolvedAction(
        available: [SkillManagerEntryAction]
    ) -> SkillManagerEntryAction? {
        guard !available.isEmpty else { return nil }
        if let preferredAction, available.contains(preferredAction) {
            return preferredAction
        }
        if preferredAction == .remove, available.contains(.deleteSource) {
            return .deleteSource
        }
        return available.first
    }
}
