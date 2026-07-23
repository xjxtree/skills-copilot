import SwiftUI

struct SessionWorkspaceDetailView: View {
    let session: LocalSessionPreviewRow?
    let detailState: LocalSessionDetailState?
    let messageCompleteness: ListCompletenessState
    let inventorySourceRevision: String?
    let productSnapshotRevision: String?
    let resumePreview: SessionContinuationRecord?
    let resumeError: String?
    let isLoadingResume: Bool
    let gapNotes: [String]
    let onLoadTimelineMore: () -> Void
    let onLoadTimelineAll: () -> Void
    let onCancelTimelineLoad: () -> Void
    let onPreviewResume: () -> Void
    let onCopyResumeCommand: (String) -> Void

    @State private var selectedLayer: SessionWorkspaceDetailLayer = .summary
    @State private var didCopyCommand = false

    init(
        session: LocalSessionPreviewRow?,
        detailState: LocalSessionDetailState?,
        messageCompleteness: ListCompletenessState,
        inventorySourceRevision: String?,
        productSnapshotRevision: String?,
        resumePreview: SessionContinuationRecord?,
        resumeError: String?,
        isLoadingResume: Bool,
        gapNotes: [String] = [],
        onLoadTimelineMore: @escaping () -> Void,
        onLoadTimelineAll: @escaping () -> Void,
        onCancelTimelineLoad: @escaping () -> Void,
        onPreviewResume: @escaping () -> Void,
        onCopyResumeCommand: @escaping (String) -> Void
    ) {
        self.session = session
        self.detailState = detailState
        self.messageCompleteness = messageCompleteness
        self.inventorySourceRevision = inventorySourceRevision
        self.productSnapshotRevision = productSnapshotRevision
        self.resumePreview = resumePreview
        self.resumeError = resumeError
        self.isLoadingResume = isLoadingResume
        self.gapNotes = gapNotes
        self.onLoadTimelineMore = onLoadTimelineMore
        self.onLoadTimelineAll = onLoadTimelineAll
        self.onCancelTimelineLoad = onCancelTimelineLoad
        self.onPreviewResume = onPreviewResume
        self.onCopyResumeCommand = onCopyResumeCommand
    }

    var body: some View {
        Group {
            if let session {
                detail(for: session)
            } else {
                VStack(spacing: 10) {
                    Image(systemName: "bubble.left.and.text.bubble.right")
                        .font(.largeTitle)
                        .foregroundStyle(.secondary)
                    Text(UIStrings.text("sessions.detail.empty.title", "Select a session"))
                        .font(.headline)
                    Text(
                        UIStrings.text(
                            "sessions.detail.empty.message",
                            "Choose a local conversation to inspect its summary, bounded timeline, and source evidence."
                        )
                    )
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
                }
                .padding(24)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .frame(minWidth: 520, minHeight: 460)
        .onChange(of: session?.id) { _ in
            selectedLayer = .summary
            didCopyCommand = false
        }
    }

    private func detail(for session: LocalSessionPreviewRow) -> some View {
        let presentation = SessionWorkspaceDetailPresentation(
            session: session,
            detailState: detailState,
            messageCompleteness: messageCompleteness,
            inventorySourceRevision: inventorySourceRevision,
            productSnapshotRevision: productSnapshotRevision,
            resumePreview: resumePreview,
            resumeError: resumeError,
            isLoadingResume: isLoadingResume,
            gapNotes: gapNotes
        )
        return VStack(spacing: 0) {
            header(presentation)
            Divider()
            layerPicker
            Divider()
            ScrollView {
                Group {
                    switch selectedLayer {
                    case .summary:
                        summaryLayer(presentation)
                    case .timeline:
                        timelineLayer(presentation)
                    case .evidence:
                        evidenceLayer(presentation)
                    }
                }
                .padding(18)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
    }

    private func header(_ presentation: SessionWorkspaceDetailPresentation) -> some View {
        HStack(alignment: .top, spacing: 14) {
            Image(systemName: "bubble.left.and.text.bubble.right")
                .font(.title2)
                .foregroundStyle(.secondary)
                .frame(width: 30)
            VStack(alignment: .leading, spacing: 4) {
                Text(presentation.title)
                    .font(.title2.bold())
                    .lineLimit(2)
                Text("\(presentation.agent) · \(presentation.project)")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            ListCompletenessBadge(state: messageCompleteness)
        }
        .padding(18)
    }

    private var layerPicker: some View {
        Picker(
            UIStrings.text("sessions.detail.layer.label", "Session detail layer"),
            selection: $selectedLayer
        ) {
            ForEach(SessionWorkspaceDetailLayer.allCases) { layer in
                Label(layer.title, systemImage: layer.systemImage)
                    .tag(layer)
            }
        }
        .pickerStyle(.segmented)
        .labelsHidden()
        .padding(.horizontal, 18)
        .padding(.vertical, 10)
    }

    private func summaryLayer(
        _ presentation: SessionWorkspaceDetailPresentation
    ) -> some View {
        VStack(alignment: .leading, spacing: 16) {
            layerHeading(
                UIStrings.text("sessions.detail.summary.title", "Summary"),
                subtitle: UIStrings.text(
                    "sessions.detail.summary.subtitle",
                    "Recorded intent, participants, activity, outcome evidence, and a safe continuation."
                )
            )

            VStack(alignment: .leading, spacing: 8) {
                Text(UIStrings.text("sessions.detail.intent", "Intent"))
                    .font(.headline)
                Text(presentation.intent)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .sessionDetailCard()

            DetailMetricGrid(maxColumns: 3, minColumnWidth: 145) {
                SummaryChip(
                    title: UIStrings.text("sessions.detail.agent", "Agent"),
                    value: presentation.agent,
                    systemImage: "cpu"
                )
                SummaryChip(
                    title: UIStrings.text("sessions.detail.project", "Project"),
                    value: presentation.project,
                    systemImage: "folder"
                )
                SummaryChip(
                    title: UIStrings.text("sessions.detail.activity", "Activity"),
                    value: presentation.activity,
                    systemImage: "clock"
                )
            }

            VStack(alignment: .leading, spacing: 8) {
                Text(UIStrings.text("sessions.detail.outcome", "Latest recorded outcome"))
                    .font(.headline)
                Text(presentation.outcome)
                    .foregroundStyle(.secondary)
                    .lineLimit(8)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .sessionDetailCard()

            continuationCard(presentation)
        }
    }

    private func timelineLayer(
        _ presentation: SessionWorkspaceDetailPresentation
    ) -> some View {
        SessionWorkspaceTimelineLayer(
            presentation: presentation,
            detailState: detailState,
            messageCompleteness: messageCompleteness,
            onLoadMore: onLoadTimelineMore,
            onLoadAll: onLoadTimelineAll,
            onCancel: onCancelTimelineLoad
        )
    }

    private func evidenceLayer(
        _ presentation: SessionWorkspaceDetailPresentation
    ) -> some View {
        VStack(alignment: .leading, spacing: 16) {
            layerHeading(
                UIStrings.text("sessions.detail.evidence.title", "Evidence"),
                subtitle: UIStrings.text(
                    "sessions.detail.evidence.subtitle",
                    "Logical source identity, revision, coverage, project match, and limitations."
                )
            )

            CompactMetadataGrid(rows: [
                CompactMetadataRow(
                    label: UIStrings.text("sessions.detail.sourceKind", "Source kind"),
                    value: presentation.sourceKind
                ),
                CompactMetadataRow(
                    label: UIStrings.text("sessions.detail.sourceRevision", "Source revision"),
                    value: presentation.sourceRevision
                ),
                CompactMetadataRow(
                    label: UIStrings.text("sessions.detail.snapshotRevision", "Snapshot revision"),
                    value: presentation.snapshotRevision
                ),
                CompactMetadataRow(
                    label: UIStrings.text("sessions.detail.coverage", "Coverage"),
                    value: presentation.coverage
                ),
                CompactMetadataRow(
                    label: UIStrings.text("sessions.detail.projectMatch", "Project match"),
                    value: presentation.project
                ),
            ])
            .padding(12)
            .nativePanelSurface()

            if !presentation.gapNotes.isEmpty {
                VStack(alignment: .leading, spacing: 8) {
                    Text(UIStrings.text("sessions.detail.limitations", "Diagnostic limitations"))
                        .font(.headline)
                    ForEach(Array(presentation.gapNotes.enumerated()), id: \.offset) { _, note in
                        Label(note, systemImage: "exclamationmark.triangle")
                            .foregroundStyle(.secondary)
                    }
                }
                .sessionDetailCard()
            }

            VStack(alignment: .leading, spacing: 8) {
                Text(UIStrings.text("sessions.detail.evidenceRefs", "Evidence references"))
                    .font(.headline)
                if presentation.evidence.isEmpty {
                    Text(
                        UIStrings.text(
                            "sessions.detail.evidenceRefs.empty",
                            "No logical evidence references were returned for this session."
                        )
                    )
                    .foregroundStyle(.secondary)
                } else {
                    ForEach(presentation.evidence) { reference in
                        VStack(alignment: .leading, spacing: 3) {
                            Text(reference.label)
                                .font(.caption.monospaced().bold())
                            Text(reference.value)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        .padding(.vertical, 3)
                    }
                }
            }
            .sessionDetailCard()

            Label(
                UIStrings.text(
                    "sessions.detail.logicalOnly",
                    "Evidence uses logical adapter references. Physical session paths are never shown."
                ),
                systemImage: "lock.shield"
            )
            .font(.callout)
            .foregroundStyle(.secondary)
        }
    }

    private func continuationCard(
        _ presentation: SessionWorkspaceDetailPresentation
    ) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Label(
                UIStrings.text("sessions.detail.continue", "Continue work"),
                systemImage: "arrow.right.circle"
            )
            .font(.headline)
            Text(presentation.suggestedContinuation)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)

            switch presentation.resumeState {
            case .notPreviewed:
                Button(action: onPreviewResume) {
                    Label(
                        UIStrings.text(
                            "sessions.detail.resume.preview",
                            "Preview native resume command"
                        ),
                        systemImage: "doc.text.magnifyingglass"
                    )
                }
            case .loading:
                HStack(spacing: 8) {
                    ProgressView().controlSize(.small)
                    Text(
                        UIStrings.text(
                            "sessions.detail.resume.loading",
                            "Checking native continuation support..."
                        )
                    )
                    .foregroundStyle(.secondary)
                }
            case .supported(let command):
                Text(command)
                    .font(.callout.monospaced())
                    .textSelection(.enabled)
                    .padding(10)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
                Button {
                    onCopyResumeCommand(command)
                    didCopyCommand = true
                } label: {
                    Label(
                        didCopyCommand
                            ? UIStrings.text("action.copied", "Copied")
                            : UIStrings.text("sessions.detail.resume.copy", "Copy command"),
                        systemImage: didCopyCommand ? "checkmark" : "doc.on.doc"
                    )
                }
                .accessibilityIdentifier("session-workspace.resume.copy")
            case .unsupported(let reason):
                Label(reason, systemImage: "nosign")
                    .foregroundStyle(.secondary)
            case .failed(let message):
                Label(message, systemImage: "exclamationmark.triangle")
                    .foregroundStyle(.orange)
                Button(
                    UIStrings.text("sessions.detail.resume.retry", "Preview again"),
                    action: onPreviewResume
                )
            }

            Text(
                UIStrings.text(
                    "sessions.detail.resume.copyOnly",
                    "Copy-only: Agent Copilot never launches a terminal, runs this command, or translates the session to another agent."
                )
            )
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        .sessionDetailCard()
    }

    private func layerHeading(_ title: String, subtitle: String) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(title)
                .font(.title3.bold())
            Text(subtitle)
                .font(.callout)
                .foregroundStyle(.secondary)
        }
    }
}

private struct SessionWorkspaceTimelineLayer: View {
    let presentation: SessionWorkspaceDetailPresentation
    let detailState: LocalSessionDetailState?
    let messageCompleteness: ListCompletenessState
    let onLoadMore: () -> Void
    let onLoadAll: () -> Void
    let onCancel: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            VStack(alignment: .leading, spacing: 3) {
                Text(UIStrings.text("sessions.detail.timeline.title", "Timeline"))
                    .font(.title3.bold())
                Text(
                    UIStrings.text(
                        "sessions.detail.timeline.subtitle",
                        "Bounded user messages and agent replies from one fixed source snapshot."
                    )
                )
                .font(.callout)
                .foregroundStyle(.secondary)
            }

            switch detailState {
            case .loading:
                HStack(spacing: 8) {
                    ProgressView().controlSize(.small)
                    Text(
                        UIStrings.text(
                            "sessions.detail.timeline.loading",
                            "Loading timeline rows from the accepted source revision."
                        )
                    )
                    .foregroundStyle(.secondary)
                }
            case .failed(let displayError):
                VStack(alignment: .leading, spacing: 8) {
                    Label(
                        UIStrings.text(
                            "sessions.detail.timeline.failed",
                            "Timeline could not be loaded"
                        ),
                        systemImage: "exclamationmark.triangle"
                    )
                    .font(.headline)
                    Text(displayError)
                        .foregroundStyle(.secondary)
                    Button(
                        UIStrings.text("sessions.detail.timeline.retry", "Retry timeline"),
                        action: onLoadAll
                    )
                }
                .sessionDetailCard()
            default:
                if presentation.timelineItems.isEmpty {
                    Text(
                        UIStrings.text(
                            "sessions.detail.timeline.empty",
                            "No user messages or agent replies are available in the loaded timeline window."
                        )
                    )
                    .foregroundStyle(.secondary)
                    .sessionDetailCard()
                } else {
                    LazyVStack(alignment: .leading, spacing: 8) {
                        ForEach(presentation.timelineItems) { item in
                            SessionTimelineRow(item: item)
                        }
                    }
                }
            }

            ListCompletenessFooter(
                state: messageCompleteness,
                onLoadMore: onLoadMore,
                onLoadAll: onLoadAll,
                onCancel: onCancel,
                accessibilityIdentifierPrefix: "session-workspace-messages"
            )
            .accessibilityIdentifier("session-workspace-messages.completeness")
        }
    }
}

private struct SessionTimelineRow: View {
    let item: SessionWorkspaceTimelineItem

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Label(
                    item.kind.title,
                    systemImage: item.kind == .userMessage ? "person" : "text.bubble"
                )
                .font(.caption.bold())
                if let timestamp = item.timestamp {
                    Text(DisplayText.timestamp(timestamp))
                        .font(.caption2.monospacedDigit())
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Text(UIStrings.localSessionContentCharacters(item.charCount))
                    .font(.caption2.monospacedDigit())
                    .foregroundStyle(.secondary)
            }
            Text(item.text)
                .textSelection(.enabled)
                .fixedSize(horizontal: false, vertical: true)
            if !item.evidenceRefs.isEmpty {
                RoutingInlineList(
                    title: UIStrings.taskCockpitEvidence,
                    empty: UIStrings.taskCockpitNoEvidence,
                    values: item.evidenceRefs,
                    systemImage: "checklist"
                )
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
    }
}

private struct SessionWorkspaceDetailCardModifier: ViewModifier {
    func body(content: Content) -> some View {
        content
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .nativePanelSurface()
    }
}

private extension View {
    func sessionDetailCard() -> some View {
        modifier(SessionWorkspaceDetailCardModifier())
    }
}
