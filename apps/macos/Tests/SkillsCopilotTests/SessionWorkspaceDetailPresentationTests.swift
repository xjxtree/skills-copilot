import Foundation
@testable import SkillsCopilot

struct SessionWorkspaceDetailPresentationTests {
    static func run() throws {
        try preservesRequiredDisclosureOrder()
        try presentsDeterministicSummaryAndBoundedTimeline()
        try presentsExactCopyOnlyResumeCommand()
        try failsClosedForMismatchedAndUnsupportedResume()
        try excludesPhysicalPathsFromEveryDetailLayer()
    }

    private static func preservesRequiredDisclosureOrder() throws {
        try expectEqual(
            SessionWorkspaceDetailLayer.allCases,
            [.summary, .timeline, .evidence],
            "Session detail must preserve Summary, Timeline, Evidence order."
        )
    }

    private static func presentsDeterministicSummaryAndBoundedTimeline() throws {
        let row = fixtureRow()
        let presentation = SessionWorkspaceDetailPresentation(
            session: row.summaryOnly,
            detailState: .loaded(row),
            messageCompleteness: completeMessageState(count: 3),
            inventorySourceRevision: "sha256:inventory",
            productSnapshotRevision: "sha256:product",
            resumePreview: try continuation(state: .supported),
            resumeError: nil,
            isLoadingResume: false,
            gapNotes: ["Final reply extraction is source limited."]
        )

        try expectEqual(presentation.title, "Repair checkout", "Session title")
        try expectEqual(presentation.intent, "Fix the failing checkout", "Typed continuation intent")
        try expectEqual(presentation.agent, "Codex", "Agent identity")
        try expectEqual(presentation.project, "funnyaccount_system", "Logical project identity")
        try expectEqual(
            presentation.timelineItems.map(\.kind),
            [.userMessage, .agentReply],
            "Timeline must default to bounded user and agent rows, excluding tool internals."
        )
        try expectEqual(
            presentation.outcome,
            "Tests now pass.",
            "Summary outcome must use the latest recorded agent reply without inference."
        )
        try expectEqual(presentation.gapNotes.count, 1, "Diagnostic limitation count")
        try expectFalse(presentation.coverage.isEmpty, "Coverage must be visible.")
        try expectEqual(
            presentation.sourceRevision,
            "sha256:native",
            "Matching resume evidence strengthens the inventory source revision."
        )
        try expectEqual(
            presentation.snapshotRevision,
            "sha256:snapshot",
            "Matching resume evidence strengthens the product snapshot revision."
        )
    }

    private static func presentsExactCopyOnlyResumeCommand() throws {
        let state = SessionWorkspaceDetailPresentation.resumeState(
            selectedSessionID: "session-1",
            resumePreview: try continuation(state: .supported),
            resumeError: nil,
            isLoadingResume: false
        )
        guard case .supported(let command) = state else {
            throw NativeModelTestFailure(
                description: "Supported continuation should expose one copy-only command."
            )
        }
        try expectEqual(
            command,
            "codex resume 'native id' 'quote'\"'\"'value'",
            "Resume command must preserve ordered argv with deterministic shell quoting."
        )
    }

    private static func failsClosedForMismatchedAndUnsupportedResume() throws {
        let mismatched = SessionWorkspaceDetailPresentation.resumeState(
            selectedSessionID: "another-session",
            resumePreview: try continuation(state: .supported),
            resumeError: nil,
            isLoadingResume: false
        )
        guard case .failed = mismatched else {
            throw NativeModelTestFailure(
                description: "A preview for another session must fail closed."
            )
        }

        let unsupported = SessionWorkspaceDetailPresentation.resumeState(
            selectedSessionID: "session-1",
            resumePreview: try continuation(
                state: .unsupported,
                unsupportedReason: .missingNativeID
            ),
            resumeError: nil,
            isLoadingResume: false
        )
        guard case .unsupported(let reason) = unsupported else {
            throw NativeModelTestFailure(
                description: "Unsupported continuation must expose a typed reason."
            )
        }
        try expectFalse(reason.isEmpty, "Typed unsupported reason must be user-visible.")
    }

    private static func excludesPhysicalPathsFromEveryDetailLayer() throws {
        let home = ["", "Users", "example"].joined(separator: "/")
        let physical = "\(home)/.codex/sessions/session-1.jsonl"
        let row = LocalSessionPreviewRow(
            id: "session-1",
            title: physical,
            sourceKind: physical,
            scope: "project",
            agent: "codex",
            projectRoot: "\(home)/git/funnyaccount_system",
            redactedPath: physical,
            modifiedAt: nil,
            startedAt: nil,
            endedAt: nil,
            excerpt: "Loaded from \(physical)",
            excerptCharCount: physical.count,
            userMessageCount: 0,
            totalMessageCount: 1,
            toolCallCount: 0,
            skillCallCount: 0,
            contentHash: physical,
            evidenceRefs: [physical, "session:logical"],
            contentIncluded: true,
            contentItems: [
                LocalSessionContentItem(
                    id: "reply",
                    kind: .agentReply,
                    title: "Agent",
                    text: "Saved at \(physical)"
                ),
            ]
        )
        let presentation = SessionWorkspaceDetailPresentation(
            session: row,
            detailState: .loaded(row),
            messageCompleteness: completeMessageState(count: 1),
            inventorySourceRevision: physical,
            productSnapshotRevision: physical,
            resumePreview: nil,
            resumeError: nil,
            isLoadingResume: false,
            gapNotes: ["Could not read \(physical)", "Logical limitation"]
        )
        let visible = [
            presentation.title,
            presentation.intent,
            presentation.project,
            presentation.outcome,
            presentation.sourceKind,
            presentation.sourceRevision,
            presentation.snapshotRevision,
        ] + presentation.gapNotes
            + presentation.evidence.flatMap { [$0.label, $0.value] }

        try expectFalse(
            visible.joined(separator: "\n").contains(home),
            "Session detail must never expose an absolute physical path."
        )
        try expectEqual(
            presentation.project,
            "funnyaccount_system",
            "Project display may retain only the logical directory label."
        )
        try expectEqual(
            presentation.evidence.map(\.id),
            ["session:logical"],
            "Physical evidence references must be removed."
        )

        let explicitRevisions = SessionWorkspaceDetailPresentation(
            session: fixtureRow(),
            detailState: nil,
            messageCompleteness: completeMessageState(count: 0),
            inventorySourceRevision: "sha256:inventory",
            productSnapshotRevision: "sha256:product",
            resumePreview: nil,
            resumeError: nil,
            isLoadingResume: false,
            gapNotes: []
        )
        try expectEqual(
            explicitRevisions.sourceRevision,
            "sha256:inventory",
            "Inventory revision must remain visible before resume preview."
        )
        try expectEqual(
            explicitRevisions.snapshotRevision,
            "sha256:product",
            "Product snapshot revision must remain visible before resume preview."
        )
    }

    private static func fixtureRow() -> LocalSessionPreviewRow {
        LocalSessionPreviewRow(
            id: "session-1",
            title: "Repair checkout",
            sourceKind: "codex-native-session",
            scope: "project",
            agent: "codex",
            projectRoot: "/tmp/home/git/funnyaccount_system",
            redactedPath: "$HOME/.codex/sessions/session-1.jsonl",
            modifiedAt: "2026-07-24T10:00:00Z",
            startedAt: 1_721_800_000,
            endedAt: 1_721_803_600,
            excerpt: "Repair the checkout",
            excerptCharCount: 19,
            userMessageCount: 1,
            totalMessageCount: 2,
            toolCallCount: 1,
            skillCallCount: 0,
            contentHash: "sha256:native",
            evidenceRefs: ["session:session-1"],
            contentIncluded: true,
            contentItems: [
                LocalSessionContentItem(
                    id: "user-1",
                    kind: .userMessage,
                    title: "User",
                    text: "Fix checkout.",
                    timestamp: 1_721_800_000,
                    evidenceRefs: ["session-message:user-1"]
                ),
                LocalSessionContentItem(
                    id: "tool-1",
                    kind: .toolCall,
                    title: "Tool",
                    text: "cargo test",
                    timestamp: 1_721_802_000
                ),
                LocalSessionContentItem(
                    id: "agent-1",
                    kind: .agentReply,
                    title: "Agent",
                    text: "Tests now pass.",
                    timestamp: 1_721_803_600,
                    evidenceRefs: ["session-message:agent-1"]
                ),
            ]
        )
    }

    private static func completeMessageState(count: Int) -> ListCompletenessState {
        ListCompletenessState(
            loadedCount: count,
            totalCount: count,
            hasMore: false,
            isComplete: true,
            completeness: .complete,
            incompleteReason: nil,
            loadingPhase: .idle,
            canLoadMore: false,
            canLoadAll: false
        )
    }

    private static func continuation(
        state: ResumeCapabilityState,
        unsupportedReason: ResumeUnsupportedReason? = nil
    ) throws -> SessionContinuationRecord {
        let resume: String
        let coverage: String
        if state == .supported {
            resume = """
            {"state":"supported","argv":["codex","resume","native id","quote'value"],"unsupported_reason":null,"copy_only":true}
            """
            coverage = """
            {"completeness":"enumerable","incomplete_reason":null,"inspected_sources":1,"expected_sources":1}
            """
        } else {
            let reason = unsupportedReason?.rawValue ?? ResumeUnsupportedReason.sessionUnsupported.rawValue
            resume = """
            {"state":"unsupported","argv":[],"unsupported_reason":"\(reason)","copy_only":true}
            """
            coverage = """
            {"completeness":"limited","incomplete_reason":"source_limited","inspected_sources":1,"expected_sources":2}
            """
        }
        let json = """
        {
          "id":"session-1",
          "agent":"codex",
          "project_id":"funnyaccount_system",
          "title":"Repair checkout",
          "intent":"Fix the failing checkout",
          "started_at":1721800000,
          "ended_at":1721803600,
          "modified_at":1721803600,
          "source_kind":"codex-native-session",
          "source_revision":"sha256:native",
          "snapshot_revision":"sha256:snapshot",
          "coverage":\(coverage),
          "resume":\(resume),
          "evidence":[{
            "id":"session:session-1",
            "kind":"session",
            "source_revision":"sha256:native",
            "summary":"Native session identity and project match",
            "agent":"codex",
            "target_id":"session-1"
          }],
          "actions":[]
        }
        """
        return try JSONDecoder().decode(
            SessionContinuationRecord.self,
            from: Data(json.utf8)
        )
    }
}

#if canImport(XCTest)
import XCTest

final class SessionWorkspaceDetailPresentationXCTest: XCTestCase {
    func testSessionWorkspaceDetailPresentation() throws {
        try SessionWorkspaceDetailPresentationTests.run()
    }
}
#endif
