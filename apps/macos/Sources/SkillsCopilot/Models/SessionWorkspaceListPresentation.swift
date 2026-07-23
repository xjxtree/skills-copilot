import Foundation

enum SessionWorkspaceEmptyReason: Equatable {
    case noProject
    case noAcceptedSnapshot
    case noSessions
    case noMatches
}

enum SessionWorkspaceListContentState: Equatable {
    case loading
    case failed(String)
    case empty(SessionWorkspaceEmptyReason)
    case ready
}

struct SessionWorkspaceRowPresentation: Identifiable, Equatable {
    let id: LocalSessionPreviewRow.ID
    let title: String
    let agentLabel: String
    let projectLabel: String
    let timeLabel: String
    let intentExcerpt: String

    init(
        session: LocalSessionPreviewRow,
        selectedProject: ProjectContext?
    ) {
        id = session.id
        title = Self.nonEmpty(
            session.title,
            fallback: UIStrings.text(
                "sessions.workspace.row.untitled",
                "Untitled session"
            )
        )
        agentLabel = session.agent.map(DisplayText.agent)
            ?? UIStrings.text("sessions.workspace.row.unknownAgent", "Unknown agent")
        projectLabel = Self.projectLabel(
            session.projectRoot,
            selectedProject: selectedProject
        )
        timeLabel = Self.timeLabel(for: session)
        intentExcerpt = Self.compactIntent(session.excerpt)
    }

    var compactSummary: String {
        [agentLabel, projectLabel, timeLabel, intentExcerpt]
            .filter { !$0.isEmpty }
            .joined(separator: " · ")
    }

    var accessibilitySummary: String {
        [title, compactSummary]
            .filter { !$0.isEmpty }
            .joined(separator: ", ")
    }

    fileprivate static func projectLabel(
        _ projectRoot: String?,
        selectedProject: ProjectContext?
    ) -> String {
        guard let projectRoot = projectRoot?
            .trimmingCharacters(in: .whitespacesAndNewlines),
            !projectRoot.isEmpty else {
            return UIStrings.text(
                "sessions.workspace.row.unassignedProject",
                "Unassigned project"
            )
        }
        if projectRoot == "<project-root>"
            || projectRoot.hasPrefix("<project-root>/")
            || normalizedPath(projectRoot) == selectedProject.map({
                normalizedPath($0.rootPath)
            }) {
            return selectedProject.map {
                nonEmpty(
                    $0.name,
                    fallback: UIStrings.text(
                        "sessions.workspace.row.currentProject",
                        "Current project"
                    )
                )
            } ?? UIStrings.text(
                "sessions.workspace.row.currentProject",
                "Current project"
            )
        }

        let component = URL(fileURLWithPath: projectRoot).lastPathComponent
            .trimmingCharacters(in: .whitespacesAndNewlines)
        guard !component.isEmpty,
              !component.hasPrefix("<"),
              component != "." else {
            return UIStrings.text(
                "sessions.workspace.row.otherProject",
                "Other project"
            )
        }
        return component
    }

    private static func timeLabel(for session: LocalSessionPreviewRow) -> String {
        if let timestamp = session.endedAt ?? session.startedAt {
            return DisplayText.timestamp(timestamp)
        }
        guard let modifiedAt = session.modifiedAt?
            .trimmingCharacters(in: .whitespacesAndNewlines),
            !modifiedAt.isEmpty else {
            return UIStrings.text(
                "sessions.workspace.row.timeUnavailable",
                "Time unavailable"
            )
        }
        if let timestamp = Int64(modifiedAt) {
            return DisplayText.timestamp(timestamp)
        }
        return modifiedAt
    }

    private static func compactIntent(_ excerpt: String) -> String {
        let compact = excerpt
            .split(whereSeparator: \.isWhitespace)
            .joined(separator: " ")
        guard !compact.isEmpty else {
            return UIStrings.text(
                "sessions.workspace.row.intentUnavailable",
                "Intent unavailable"
            )
        }
        let maximumLength = 180
        guard compact.count > maximumLength else { return compact }
        return String(compact.prefix(maximumLength - 1)) + "…"
    }

    private static func normalizedPath(_ path: String) -> String {
        URL(fileURLWithPath: path).standardizedFileURL.path
            .trimmingCharacters(in: CharacterSet(charactersIn: "/"))
    }

    private static func nonEmpty(_ value: String, fallback: String) -> String {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? fallback : trimmed
    }
}

enum SessionWorkspaceProjectGroupKind: String, Equatable {
    case selectedProject
    case otherProject
    case unmatched
}

struct SessionWorkspaceProjectGroupPresentation: Identifiable, Equatable {
    let id: String
    let kind: SessionWorkspaceProjectGroupKind
    let title: String
    let rows: [SessionWorkspaceRowPresentation]
}

struct SessionWorkspaceListPresentation: Equatable {
    static let orderedAgents: [ProductAgentID] = [
        .claudeCode,
        .codex,
        .opencode,
        .pi,
        .hermes,
        .openclaw,
    ]

    let projectGroups: [SessionWorkspaceProjectGroupPresentation]
    let loadedSessionCount: Int
    let sourceSessionTotal: Int?
    let completeness: ListCompletenessState
    let contentState: SessionWorkspaceListContentState
    let isRefreshingAcceptedSnapshot: Bool
    let isStale: Bool
    let supportingErrorMessage: String?
    let projectContextLabel: String
    let scope: LocalSessionScopeFilter

    init(
        visibleSessions: [LocalSessionPreviewRow],
        loadedSessionCount: Int,
        sourceSessionTotal: Int?,
        completeness: ListCompletenessState,
        project: ProjectContext?,
        scope: LocalSessionScopeFilter,
        hasAcceptedSnapshot: Bool,
        isLoading: Bool,
        isStale: Bool,
        errorMessage: String?
    ) {
        projectGroups = Self.projectGroups(
            visibleSessions,
            selectedProject: project
        )
        self.loadedSessionCount = loadedSessionCount
        self.sourceSessionTotal = sourceSessionTotal
        self.completeness = completeness
        self.scope = scope
        isRefreshingAcceptedSnapshot = isLoading && hasAcceptedSnapshot
        self.isStale = isStale
        supportingErrorMessage = hasAcceptedSnapshot ? errorMessage : nil
        projectContextLabel = project.map {
            let name = $0.name.trimmingCharacters(in: .whitespacesAndNewlines)
            return name.isEmpty
                ? UIStrings.text(
                    "sessions.workspace.project.selected",
                    "Selected project"
                )
                : name
        } ?? UIStrings.text(
            "sessions.workspace.project.none",
            "No project selected"
        )

        if project == nil {
            contentState = .empty(.noProject)
        } else if !projectGroups.isEmpty {
            contentState = .ready
        } else if isLoading && !hasAcceptedSnapshot {
            contentState = .loading
        } else if !hasAcceptedSnapshot, let errorMessage {
            contentState = .failed(errorMessage)
        } else if !hasAcceptedSnapshot {
            contentState = .empty(.noAcceptedSnapshot)
        } else if loadedSessionCount == 0 {
            contentState = .empty(.noSessions)
        } else {
            contentState = .empty(.noMatches)
        }
    }

    var visibleSessionCount: Int {
        projectGroups.reduce(0) { $0 + $1.rows.count }
    }

    var visibleCountSummary: String {
        String(
            format: UIStrings.text(
                "sessions.workspace.counts.visible",
                "%d sessions"
            ),
            visibleSessionCount
        )
    }

    var sourceCountSummary: String {
        if let sourceSessionTotal {
            return String(
                format: UIStrings.text(
                    "sessions.workspace.counts.sourceTotal",
                    "Source total %d sessions"
                ),
                sourceSessionTotal
            )
        }
        return String(
            format: UIStrings.text(
                "sessions.workspace.counts.sourceLoaded",
                "Loaded %d sessions · source total unknown"
            ),
            loadedSessionCount
        )
    }

    private struct ProjectGroupAccumulator {
        let id: String
        let kind: SessionWorkspaceProjectGroupKind
        let title: String
        var sessions: [LocalSessionPreviewRow]
    }

    private static func projectGroups(
        _ sessions: [LocalSessionPreviewRow],
        selectedProject: ProjectContext?
    ) -> [SessionWorkspaceProjectGroupPresentation] {
        var accumulators: [String: ProjectGroupAccumulator] = [:]
        for session in sessions {
            let identity = projectGroupIdentity(
                session,
                selectedProject: selectedProject
            )
            if accumulators[identity.id] == nil {
                accumulators[identity.id] = ProjectGroupAccumulator(
                    id: identity.id,
                    kind: identity.kind,
                    title: identity.title,
                    sessions: []
                )
            }
            accumulators[identity.id]?.sessions.append(session)
        }
        return accumulators.values.sorted { left, right in
            let leftRank = projectGroupRank(left.kind)
            let rightRank = projectGroupRank(right.kind)
            if leftRank != rightRank {
                return leftRank < rightRank
            }
            let titleOrder = left.title.localizedCaseInsensitiveCompare(
                right.title
            )
            if titleOrder != .orderedSame {
                return titleOrder == .orderedAscending
            }
            return left.id < right.id
        }.map { group in
            SessionWorkspaceProjectGroupPresentation(
                id: group.id,
                kind: group.kind,
                title: group.title,
                rows: group.sessions.map {
                    SessionWorkspaceRowPresentation(
                        session: $0,
                        selectedProject: selectedProject
                    )
                }
            )
        }
    }

    private static func projectGroupIdentity(
        _ session: LocalSessionPreviewRow,
        selectedProject: ProjectContext?
    ) -> (
        id: String,
        kind: SessionWorkspaceProjectGroupKind,
        title: String
    ) {
        let root = session.projectRoot?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if root == "<project-root>"
            || root.hasPrefix("<project-root>/")
            || selectedProject.map({
                URL(fileURLWithPath: root).standardizedFileURL.path
                    == URL(fileURLWithPath: $0.rootPath).standardizedFileURL.path
            }) == true {
            return (
                id: selectedProject.map { "selected:\($0.id)" } ?? "selected",
                kind: .selectedProject,
                title: SessionWorkspaceRowPresentation.projectLabel(
                    session.projectRoot,
                    selectedProject: selectedProject
                )
            )
        }
        guard !root.isEmpty else {
            return (
                id: "unmatched",
                kind: .unmatched,
                title: UIStrings.text(
                    "sessions.workspace.group.unmatched",
                    "Unknown or unmatched project"
                )
            )
        }
        return (
            id: "other:\(stableProjectIdentity(root))",
            kind: .otherProject,
            title: SessionWorkspaceRowPresentation.projectLabel(
                session.projectRoot,
                selectedProject: selectedProject
            )
        )
    }

    private static func stableProjectIdentity(_ value: String) -> String {
        var hash: UInt64 = 14_695_981_039_346_656_037
        for byte in value.utf8 {
            hash ^= UInt64(byte)
            hash &*= 1_099_511_628_211
        }
        return String(hash, radix: 16)
    }

    private static func projectGroupRank(
        _ kind: SessionWorkspaceProjectGroupKind
    ) -> Int {
        switch kind {
        case .selectedProject: 0
        case .otherProject: 1
        case .unmatched: 2
        }
    }
}

extension LocalSessionScopeFilter {
    var workspacePresentationTitle: String {
        switch self {
        case .project:
            UIStrings.text("sessions.workspace.scope.project", "Project")
        case .all:
            UIStrings.text("sessions.workspace.scope.all", "All")
        }
    }
}

extension LocalSessionSortOrder {
    var workspacePresentationTitle: String {
        switch self {
        case .recent:
            UIStrings.text("sessions.workspace.sort.recent", "Recent")
        case .title:
            UIStrings.text("sessions.workspace.sort.title", "Title")
        }
    }
}
