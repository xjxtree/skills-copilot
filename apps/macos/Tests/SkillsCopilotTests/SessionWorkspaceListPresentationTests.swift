import Foundation
@testable import SkillsCopilot

struct SessionWorkspaceListPresentationTests {
    static func run() throws {
        try projectScopeIsTheDefault()
        try rowsExposeCompactSafeContext()
        try allScopeGroupsTheSelectedProjectFirst()
        try loadingEmptyFailureAndStaleStatesRemainDistinct()
        try completenessCountsRemainSourceBound()
    }

    private static func projectScopeIsTheDefault() throws {
        try expectEqual(
            SessionWorkspaceCriteria().scope,
            .project,
            "Sessions workspace default scope"
        )
        try expectEqual(
            SessionWorkspaceListPresentation.orderedAgents,
            [.claudeCode, .codex, .opencode, .pi, .hermes, .openclaw],
            "Agent filters must use the supported product order."
        )
    }

    private static func rowsExposeCompactSafeContext() throws {
        let session = makeSession(
            id: "session-safe",
            title: "Repair checkout flow",
            agent: ProductAgentID.codex.rawValue,
            projectRoot: "/workspace/private/customer-system",
            endedAt: 1_720_000_000_000,
            excerpt: "  Diagnose\n the checkout timeout and verify the fix.  "
        )
        let row = SessionWorkspaceRowPresentation(
            session: session,
            selectedProject: project
        )

        try expectEqual(row.title, "Repair checkout flow", "Session title")
        try expectEqual(row.agentLabel, UIStrings.codex, "Agent label")
        try expectEqual(row.projectLabel, "customer-system", "Safe project label")
        try expectContains(
            row.compactSummary,
            "Diagnose the checkout timeout",
            "Intent excerpt must be compact."
        )
        try expectFalse(
            row.accessibilitySummary.contains("/workspace/private"),
            "Session rows must not disclose raw project paths."
        )
    }

    private static func allScopeGroupsTheSelectedProjectFirst() throws {
        let otherOne = makeSession(
            id: "other-one",
            title: "Other one",
            agent: ProductAgentID.claudeCode.rawValue,
            projectRoot: "/work/other",
            endedAt: 300,
            excerpt: "Other intent"
        )
        let current = makeSession(
            id: "current",
            title: "Current",
            agent: ProductAgentID.codex.rawValue,
            projectRoot: project.rootPath,
            endedAt: 200,
            excerpt: "Current intent"
        )
        let otherTwo = makeSession(
            id: "other-two",
            title: "Other two",
            agent: ProductAgentID.pi.rawValue,
            projectRoot: "/work/other",
            endedAt: 100,
            excerpt: "Other follow-up"
        )
        let presentation = makePresentation(
            sessions: [otherOne, current, otherTwo],
            scope: .all
        )

        try expectEqual(
            presentation.projectGroups.map(\.kind),
            [.selectedProject, .otherProject],
            "All scope must group the selected project first with typed groups."
        )
        try expectEqual(
            presentation.projectGroups.map { $0.rows.map(\.id) },
            [["current"], ["other-one", "other-two"]],
            "Each project group must preserve the accepted row order."
        )
        try expectEqual(
            presentation.projectGroups.first?.title,
            project.name,
            "Current logical project label"
        )
        try expectFalse(
            presentation.projectGroups.map(\.title).joined().contains("/work/"),
            "Project group titles must not reveal full source paths."
        )

        let unmatched = makePresentation(
            sessions: [makeSession(
                id: "unmatched",
                title: "Unmatched",
                agent: ProductAgentID.hermes.rawValue,
                projectRoot: nil,
                endedAt: 50,
                excerpt: "Unmatched intent"
            )],
            scope: .all
        )
        try expectEqual(
            unmatched.projectGroups.first?.kind,
            .unmatched,
            "A missing project identity must use the typed unmatched group."
        )
    }

    private static func loadingEmptyFailureAndStaleStatesRemainDistinct() throws {
        let noProject = makePresentation(
            project: nil,
            hasAcceptedSnapshot: false
        )
        try expectEqual(
            noProject.contentState,
            .empty(.noProject),
            "Missing project state"
        )

        let loading = makePresentation(
            hasAcceptedSnapshot: false,
            isLoading: true
        )
        try expectEqual(loading.contentState, .loading, "Initial loading state")

        let failed = makePresentation(
            hasAcceptedSnapshot: false,
            errorMessage: "fixture failure"
        )
        try expectEqual(
            failed.contentState,
            .failed("fixture failure"),
            "Initial failure state"
        )

        let noSnapshot = makePresentation(hasAcceptedSnapshot: false)
        try expectEqual(
            noSnapshot.contentState,
            .empty(.noAcceptedSnapshot),
            "No snapshot state"
        )

        let noSessions = makePresentation()
        try expectEqual(
            noSessions.contentState,
            .empty(.noSessions),
            "Accepted empty state"
        )

        let noMatches = makePresentation(loadedSessionCount: 2)
        try expectEqual(
            noMatches.contentState,
            .empty(.noMatches),
            "Filtered empty state"
        )

        let stale = makePresentation(
            sessions: [makeSession(
                id: "accepted",
                title: "Accepted",
                agent: ProductAgentID.codex.rawValue,
                projectRoot: project.rootPath,
                endedAt: 100,
                excerpt: "Accepted intent"
            )],
            loadedSessionCount: 1,
            sourceSessionTotal: 1,
            isStale: true,
            errorMessage: "source changed"
        )
        try expectEqual(stale.contentState, .ready, "Stale rows remain visible.")
        try expectEqual(stale.isStale, true, "Stale flag")
        try expectEqual(
            stale.supportingErrorMessage,
            "source changed",
            "Stale state keeps its supporting error."
        )
    }

    private static func completenessCountsRemainSourceBound() throws {
        let session = makeSession(
            id: "visible",
            title: "Visible",
            agent: ProductAgentID.codex.rawValue,
            projectRoot: project.rootPath,
            endedAt: 100,
            excerpt: "Visible intent"
        )
        let presentation = makePresentation(
            sessions: [session],
            loadedSessionCount: 100,
            sourceSessionTotal: 240
        )

        try expectEqual(presentation.visibleSessionCount, 1, "Visible count")
        try expectEqual(presentation.loadedSessionCount, 100, "Loaded count")
        try expectEqual(presentation.sourceSessionTotal, 240, "Source total")
        try expectContains(
            presentation.sourceCountSummary,
            "240 sessions",
            "Source count disclosure"
        )
    }

    private static var project: ProjectContext {
        ProjectContext(
            id: "project-current",
            name: "funnyaccount_system",
            rootPath: "/work/funnyaccount_system",
            currentCWD: "/work/funnyaccount_system",
            lastUsedAt: nil,
            isActive: true,
            validationError: nil
        )
    }

    private static func makePresentation(
        sessions: [LocalSessionPreviewRow] = [],
        loadedSessionCount: Int = 0,
        sourceSessionTotal: Int? = nil,
        project: ProjectContext? = project,
        scope: LocalSessionScopeFilter = .project,
        hasAcceptedSnapshot: Bool = true,
        isLoading: Bool = false,
        isStale: Bool = false,
        errorMessage: String? = nil
    ) -> SessionWorkspaceListPresentation {
        let isComplete = hasAcceptedSnapshot
            && (sourceSessionTotal == nil
                || sourceSessionTotal == loadedSessionCount)
        return SessionWorkspaceListPresentation(
            visibleSessions: sessions,
            loadedSessionCount: loadedSessionCount,
            sourceSessionTotal: sourceSessionTotal,
            completeness: ListCompletenessState(
                loadedCount: loadedSessionCount,
                totalCount: sourceSessionTotal,
                hasMore: sourceSessionTotal.map { $0 > loadedSessionCount } ?? false,
                isComplete: isComplete,
                completeness: !hasAcceptedSnapshot
                    ? .unknown
                    : isComplete ? .complete : .partial,
                incompleteReason: hasAcceptedSnapshot ? nil : .notInspected,
                loadingPhase: isLoading ? .initial : .idle,
                canLoadMore: false,
                canLoadAll: false
            ),
            project: project,
            scope: scope,
            hasAcceptedSnapshot: hasAcceptedSnapshot,
            isLoading: isLoading,
            isStale: isStale,
            errorMessage: errorMessage
        )
    }

    private static func makeSession(
        id: String,
        title: String,
        agent: String?,
        projectRoot: String?,
        endedAt: Int64?,
        excerpt: String
    ) -> LocalSessionPreviewRow {
        LocalSessionPreviewRow(
            id: id,
            title: title,
            sourceKind: "authorized-local-session",
            scope: "project",
            agent: agent,
            projectRoot: projectRoot,
            redactedPath: "$HOME/.agent/session.jsonl",
            modifiedAt: nil,
            startedAt: nil,
            endedAt: endedAt,
            excerpt: excerpt,
            excerptCharCount: excerpt.count,
            userMessageCount: 1,
            totalMessageCount: 2,
            toolCallCount: 0,
            skillCallCount: 0,
            contentHash: "hash-\(id)",
            evidenceRefs: ["evidence:\(id)"],
            contentIncluded: false,
            contentItems: []
        )
    }
}

#if canImport(XCTest)
import XCTest

final class SessionWorkspaceListPresentationFocusedCases: XCTestCase {
    func testPresentationContract() throws {
        try SessionWorkspaceListPresentationTests.run()
    }
}
#endif
