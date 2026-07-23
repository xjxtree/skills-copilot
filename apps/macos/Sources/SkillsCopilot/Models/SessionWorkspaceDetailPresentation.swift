import Foundation

enum SessionWorkspaceDetailLayer: String, CaseIterable, Identifiable {
    case summary
    case timeline
    case evidence

    var id: String { rawValue }

    var title: String {
        switch self {
        case .summary:
            UIStrings.text("sessions.detail.layer.summary", "Summary")
        case .timeline:
            UIStrings.text("sessions.detail.layer.timeline", "Timeline")
        case .evidence:
            UIStrings.text("sessions.detail.layer.evidence", "Evidence")
        }
    }

    var systemImage: String {
        switch self {
        case .summary: "text.page"
        case .timeline: "clock.arrow.trianglehead.counterclockwise.rotate.90"
        case .evidence: "checklist"
        }
    }
}

enum SessionResumePresentationState: Equatable {
    case notPreviewed
    case loading
    case supported(command: String)
    case unsupported(reason: String)
    case failed(message: String)
}

struct SessionWorkspaceEvidenceRow: Identifiable, Equatable {
    let id: String
    let label: String
    let value: String
}

struct SessionWorkspaceTimelineItem: Identifiable, Equatable {
    let id: String
    let kind: LocalSessionContentKind
    let text: String
    let charCount: Int
    let timestamp: Int64?
    let evidenceRefs: [String]
}

struct SessionWorkspaceDetailPresentation {
    let title: String
    let intent: String
    let agent: String
    let project: String
    let activity: String
    let outcome: String
    let suggestedContinuation: String
    let timelineItems: [SessionWorkspaceTimelineItem]
    let sourceKind: String
    let sourceRevision: String
    let snapshotRevision: String
    let coverage: String
    let gapNotes: [String]
    let evidence: [SessionWorkspaceEvidenceRow]
    let resumeState: SessionResumePresentationState

    init(
        session: LocalSessionPreviewRow,
        detailState: LocalSessionDetailState?,
        messageCompleteness: ListCompletenessState,
        inventorySourceRevision: String?,
        productSnapshotRevision: String?,
        resumePreview: SessionContinuationRecord?,
        resumeError: String?,
        isLoadingResume: Bool,
        gapNotes: [String]
    ) {
        let acceptedRow: LocalSessionPreviewRow
        if case .loaded(let detail) = detailState, detail.id == session.id {
            acceptedRow = detail
        } else {
            acceptedRow = session
        }
        let matchingResume = resumePreview.flatMap { $0.id == session.id ? $0 : nil }

        title = Self.safeDisplayText(session.title)
            ?? UIStrings.text("sessions.detail.untitled", "Untitled session")
        intent = Self.safeDisplayText(matchingResume?.intent)
            ?? Self.safeDisplayText(session.excerpt)
            ?? UIStrings.text(
                "sessions.detail.intentUnavailable",
                "No recorded intent is available in the accepted snapshot."
            )
        agent = matchingResume.map { DisplayText.agent($0.agent.rawValue) }
            ?? Self.safeLogicalIdentity(session.agent).map(DisplayText.agent)
            ?? UIStrings.text("sessions.detail.agentUnknown", "Unknown agent")
        project = Self.logicalProjectLabel(
            projectID: matchingResume?.projectID,
            projectRoot: session.projectRoot
        )
        activity = Self.activityText(session)
        outcome = Self.outcomeText(acceptedRow)
        timelineItems = acceptedRow.contentItems.compactMap { item in
            guard LocalSessionContentKind.defaultDetailKinds.contains(item.kind) else {
                return nil
            }
            return SessionWorkspaceTimelineItem(
                id: item.id,
                kind: item.kind,
                text: item.text,
                charCount: item.charCount,
                timestamp: item.timestamp,
                evidenceRefs: item.evidenceRefs.compactMap(Self.safeLogicalIdentity)
            )
        }
        sourceKind = Self.logicalSourceKind(matchingResume?.sourceKind ?? session.sourceKind)
        sourceRevision = Self.logicalRevision(
            matchingResume?.sourceRevision ?? inventorySourceRevision
        )
        snapshotRevision = Self.logicalRevision(
            matchingResume?.snapshotRevision ?? productSnapshotRevision
        )
        coverage = Self.coverageText(
            resumePreview: matchingResume,
            messageCompleteness: messageCompleteness
        )
        self.gapNotes = gapNotes.map {
            Self.safeDisplayText($0)
                ?? UIStrings.text(
                    "sessions.detail.limitations.redacted",
                    "A source limitation containing private path data was redacted."
                )
        }
        evidence = Self.evidenceRows(
            session: session,
            resumePreview: matchingResume
        )
        resumeState = Self.resumeState(
            selectedSessionID: session.id,
            resumePreview: resumePreview,
            resumeError: resumeError,
            isLoadingResume: isLoadingResume
        )
        suggestedContinuation = Self.continuationText(
            state: resumeState,
            agent: agent
        )
    }

    static func resumeState(
        selectedSessionID: String,
        resumePreview: SessionContinuationRecord?,
        resumeError: String?,
        isLoadingResume: Bool
    ) -> SessionResumePresentationState {
        if isLoadingResume {
            return .loading
        }
        if let resumeError = safeDisplayText(resumeError) {
            return .failed(message: resumeError)
        }
        guard let resumePreview else {
            return .notPreviewed
        }
        guard resumePreview.id == selectedSessionID else {
            return .failed(
                message: UIStrings.text(
                    "sessions.detail.resumeMismatch",
                    "The continuation preview does not match the selected session."
                )
            )
        }
        switch resumePreview.resume.state {
        case .supported:
            guard let command = SessionResumeCommandPresentation(session: resumePreview)?.command else {
                return .failed(
                    message: UIStrings.text(
                        "sessions.detail.resumeInvalid",
                        "The service did not return a valid copy-only command."
                    )
                )
            }
            return .supported(command: command)
        case .unsupported:
            return .unsupported(
                reason: unsupportedReasonText(resumePreview.resume.unsupportedReason)
            )
        }
    }

    static func unsupportedReasonText(_ reason: ResumeUnsupportedReason?) -> String {
        switch reason {
        case .agentUnsupported:
            UIStrings.text(
                "sessions.detail.resumeUnsupported.agent",
                "This agent does not expose a verified native resume command."
            )
        case .sessionUnsupported:
            UIStrings.text(
                "sessions.detail.resumeUnsupported.session",
                "This session format cannot be resumed with a verified native command."
            )
        case .sourceIncomplete:
            UIStrings.text(
                "sessions.detail.resumeUnsupported.incomplete",
                "Source evidence is incomplete, so continuation is unavailable."
            )
        case .sourceChanged:
            UIStrings.text(
                "sessions.detail.resumeUnsupported.changed",
                "The session source changed after discovery. Refresh before previewing again."
            )
        case .missingNativeID:
            UIStrings.text(
                "sessions.detail.resumeUnsupported.nativeID",
                "The source does not contain the native session identifier required to resume."
            )
        case .invalidProjectContext:
            UIStrings.text(
                "sessions.detail.resumeUnsupported.project",
                "The selected project does not match the verified session context."
            )
        case .none:
            UIStrings.text(
                "sessions.detail.resumeUnsupported.unknown",
                "A verified native resume command is unavailable for this session."
            )
        }
    }

    private static func continuationText(
        state: SessionResumePresentationState,
        agent: String
    ) -> String {
        switch state {
        case .supported:
            return String(
                format: UIStrings.text(
                    "sessions.detail.suggestion.supported",
                    "Copy the verified %@ command and run it yourself in a terminal."
                ),
                agent
            )
        case .unsupported(let reason):
            return reason
        case .loading:
            return UIStrings.text(
                "sessions.detail.suggestion.loading",
                "Checking deterministic adapter-native continuation support."
            )
        case .failed(let message):
            return message
        case .notPreviewed:
            return UIStrings.text(
                "sessions.detail.suggestion.preview",
                "Preview the adapter-native continuation command before deciding how to continue."
            )
        }
    }

    private static func activityText(_ session: LocalSessionPreviewRow) -> String {
        switch (session.startedAt, session.endedAt) {
        case let (started?, ended?) where started != ended:
            return "\(DisplayText.timestamp(started)) – \(DisplayText.timestamp(ended))"
        case let (started?, _):
            return DisplayText.timestamp(started)
        case let (_, ended?):
            return DisplayText.timestamp(ended)
        default:
            return safeDisplayText(session.modifiedAt)
                ?? UIStrings.text("sessions.detail.activityUnknown", "Activity time unavailable")
        }
    }

    private static func outcomeText(_ session: LocalSessionPreviewRow) -> String {
        if let lastReply = session.contentItems.last(where: { $0.kind == .agentReply }),
           let reply = safeDisplayText(lastReply.text) {
            return reply
        }
        return String(
            format: UIStrings.text(
                "sessions.detail.outcome.counts",
                "%d messages are recorded; inspect Timeline for the available outcome evidence."
            ),
            session.totalMessageCount
        )
    }

    private static func logicalProjectLabel(
        projectID: String?,
        projectRoot: String?
    ) -> String {
        if let projectID = safeLogicalIdentity(projectID) {
            return projectID
        }
        if let projectRoot {
            let label = URL(fileURLWithPath: projectRoot).lastPathComponent
            if let safe = safeLogicalIdentity(label) {
                return safe
            }
        }
        return UIStrings.text("sessions.detail.projectUnknown", "Unmatched project")
    }

    private static func logicalSourceKind(_ value: String?) -> String {
        safeLogicalIdentity(value)
            ?? UIStrings.text("sessions.detail.sourceUnknown", "Authorized local session source")
    }

    private static func logicalRevision(_ value: String?) -> String {
        safeLogicalIdentity(value)
            ?? UIStrings.text("sessions.detail.revisionUnknown", "Unavailable")
    }

    private static func coverageText(
        resumePreview: SessionContinuationRecord?,
        messageCompleteness: ListCompletenessState
    ) -> String {
        if let coverage = resumePreview?.coverage {
            let expected = coverage.expectedSources.map(String.init)
                ?? UIStrings.text("sessions.detail.coverage.unknown", "unknown")
            return String(
                format: UIStrings.text(
                    "sessions.detail.coverage.sources",
                    "%d of %@ sources inspected · %@"
                ),
                coverage.inspectedSources,
                expected,
                coverage.isComplete
                    ? UIStrings.text("sessions.detail.coverage.complete", "complete")
                    : UIStrings.text("sessions.detail.coverage.incomplete", "incomplete")
            )
        }
        let total = messageCompleteness.totalCount.map(String.init)
            ?? UIStrings.text("sessions.detail.coverage.unknown", "unknown")
        return String(
            format: UIStrings.text(
                "sessions.detail.coverage.messages",
                "%d of %@ timeline rows loaded · %@"
            ),
            messageCompleteness.loadedCount,
            total,
            messageCompleteness.isComplete
                ? UIStrings.text("sessions.detail.coverage.complete", "complete")
                : UIStrings.text("sessions.detail.coverage.incomplete", "incomplete")
        )
    }

    private static func evidenceRows(
        session: LocalSessionPreviewRow,
        resumePreview: SessionContinuationRecord?
    ) -> [SessionWorkspaceEvidenceRow] {
        var rows: [SessionWorkspaceEvidenceRow] = []
        if let resumePreview {
            rows = resumePreview.evidence.compactMap { reference in
                guard let id = safeLogicalIdentity(reference.id),
                      let summary = safeDisplayText(reference.summary) else {
                    return nil
                }
                return SessionWorkspaceEvidenceRow(
                    id: id,
                    label: id,
                    value: summary
                )
            }
        }
        var known = Set(rows.map(\.id))
        rows.append(contentsOf: session.evidenceRefs.compactMap { reference in
            guard let safe = safeLogicalIdentity(reference),
                  known.insert(safe).inserted else {
                return nil
            }
            return SessionWorkspaceEvidenceRow(
                id: safe,
                label: safe,
                value: UIStrings.text(
                    "sessions.detail.evidence.localFact",
                    "Local session fact from the accepted source snapshot."
                )
            )
        })
        return rows
    }

    private static func safeDisplayText(_ value: String?) -> String? {
        guard let value else { return nil }
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty,
              !containsPhysicalPath(trimmed) else {
            return nil
        }
        return trimmed
    }

    private static func safeLogicalIdentity(_ value: String?) -> String? {
        guard let safe = safeDisplayText(value),
              !safe.contains("/"),
              !safe.contains("\\") else {
            return nil
        }
        return safe
    }

    private static func containsPhysicalPath(_ value: String) -> Bool {
        if value.contains("file://") || value.contains("\\") {
            return true
        }
        return value.split(whereSeparator: \.isWhitespace).contains { word in
            let token = word.trimmingCharacters(
                in: CharacterSet(charactersIn: "()[]{}\"',;")
            )
            return token.count > 1 && token.hasPrefix("/")
        }
    }
}
