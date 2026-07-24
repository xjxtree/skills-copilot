import SwiftUI

struct ProjectOverviewView: View {
    @ObservedObject var appContextStore: AppContextStore
    @ObservedObject var skillStore: SkillStore

    let onOpenAttentionEvidence: (AttentionItem, [EvidenceRef]) -> Void
    let onPreviewAttentionAction: (AttentionItem, ActionDescriptorWire) -> Void
    let onOpenSession: (SessionContinuationRecord) -> Void
    let onPreviewSessionResume: (SessionContinuationRecord) -> Void
    @State private var contextualEvidenceSelection: ContextualEvidenceSelection?

    init(
        appContextStore: AppContextStore,
        skillStore: SkillStore,
        onOpenAttentionEvidence: @escaping (AttentionItem, [EvidenceRef]) -> Void = { _, _ in },
        onPreviewAttentionAction: @escaping (AttentionItem, ActionDescriptorWire) -> Void = { _, _ in },
        onOpenSession: @escaping (SessionContinuationRecord) -> Void = { _ in },
        onPreviewSessionResume: @escaping (SessionContinuationRecord) -> Void = { _ in }
    ) {
        self.appContextStore = appContextStore
        self.skillStore = skillStore
        self.onOpenAttentionEvidence = onOpenAttentionEvidence
        self.onPreviewAttentionAction = onPreviewAttentionAction
        self.onOpenSession = onOpenSession
        self.onPreviewSessionResume = onPreviewSessionResume
    }

    private var presentation: ProjectOverviewPresentation {
        ProjectOverviewPresentation(
            project: appContextStore.activeProject,
            projectContextRevision: appContextStore.projectContextState?.revision,
            readinessState: appContextStore.readinessState,
            isLoadingProjectContext: appContextStore.isLoadingProjectContext,
            projectContextErrorMessage: appContextStore.projectContextErrorMessage,
            agentFilter: appContextStore.agentFilter,
            acceptedAt: appContextStore.visibleProjectReadinessAcceptedAt
        )
    }

    var body: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 18) {
                header
                content
            }
            .padding(24)
            .frame(maxWidth: 1080, alignment: .leading)
            .frame(maxWidth: .infinity, alignment: .topLeading)
        }
        .onAppear {
            skillStore.ensureTaskCockpitAgentSelection()
        }
        .sheet(item: $contextualEvidenceSelection) { selection in
            ContextualEvidenceSheet(selection: selection)
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 5) {
            Text(UIStrings.text("overview.title", "Project Overview"))
                .font(.largeTitle.weight(.semibold))
            Text(
                presentation.projectName
                    ?? UIStrings.text(
                        "overview.subtitle.empty",
                        "Verify a selected project before reviewing capabilities or sessions."
                    )
            )
            .font(.callout)
            .foregroundStyle(.secondary)
        }
    }

    @ViewBuilder
    private var content: some View {
        switch presentation.state {
        case .emptyProject:
            ProjectOverviewPlaceholder(
                title: UIStrings.text("overview.empty.project", "Choose a project"),
                message: UIStrings.text(
                    "overview.empty.project.message",
                    "Project status, task readiness, attention, and recent work are shown in the selected project context."
                ),
                systemImage: "folder.badge.questionmark"
            )
        case .emptySnapshot:
            ProjectOverviewPlaceholder(
                title: UIStrings.text("overview.empty.snapshot", "No readiness snapshot"),
                message: UIStrings.text(
                    "overview.empty.snapshot.message",
                    "Refresh this project to load its deterministic readiness projection."
                ),
                systemImage: "checkmark.seal",
                actionTitle: UIStrings.text("overview.refresh", "Refresh"),
                action: refreshReadiness
            )
        case .loading:
            ProjectOverviewPlaceholder(
                title: UIStrings.text("overview.loading", "Loading project readiness"),
                message: UIStrings.text(
                    "overview.loading.message",
                    "The accepted project context is being inspected."
                ),
                systemImage: "hourglass",
                showsProgress: true
            )
        case .error:
            ProjectOverviewPlaceholder(
                title: UIStrings.text("overview.error", "Project readiness is unavailable"),
                message: presentation.message
                    ?? UIStrings.text(
                        "overview.error.message",
                        "The project snapshot could not be loaded."
                    ),
                systemImage: "exclamationmark.triangle",
                actionTitle: UIStrings.text("overview.retry", "Retry"),
                action: refreshReadiness
            )
        case .ready, .stale, .partial, .blocked:
            overviewSections
        }
    }

    @ViewBuilder
    private var overviewSections: some View {
        if let record = presentation.record {
            ProjectStatusSection(
                presentation: presentation,
                record: record,
                onRefresh: refreshReadiness,
                intelligence: {
                    projectIntelligenceView(record)
                }
            )

            taskReadinessSection

            ProjectAttentionSection(
                rows: presentation.attention,
                onOpenEvidence: onOpenAttentionEvidence,
                onPreviewAction: onPreviewAttentionAction
            )

            ProjectContinueWorkSection(
                sessions: presentation.recentSessions,
                onOpenSession: onOpenSession,
                onPreviewResume: onPreviewSessionResume
            )
        }
    }

    private func projectIntelligenceView(
        _ record: ProjectReadinessRecord
    ) -> some View {
        let key = ContextualIntelligenceStore.projectKey(record.projectID)
        return ContextualIntelligenceView(
                kind: .projectHealth,
                deterministicTitle: UIStrings.text(
                    "overview.intelligence.facts",
                    "Current project readiness snapshot"
                ),
                deterministicFacts: [
                    ContextualIntelligenceFact(
                        label: UIStrings.text("overview.status.health", "Environment"),
                        value: healthTitle(record.health)
                    ),
                    ContextualIntelligenceFact(
                        label: UIStrings.text("overview.status.coverage", "Coverage"),
                        value: ProjectOverviewPresentation.coverageText(record.coverage)
                    ),
                    ContextualIntelligenceFact(
                        label: UIStrings.text("overview.attention.title", "Needs attention"),
                        value: String(record.attention.count)
                    ),
                ],
                flow: skillStore.contextualIntelligenceStore.flow(for: key),
                currentSourceRevision: record.sourceRevision,
                providerGateMessage: taskProviderGateMessage,
                onPreview: {
                    Task {
                        await skillStore.contextualIntelligenceStore
                            .previewProjectHealth(record)
                    }
                },
                onConfirm: {
                    Task {
                        await skillStore.contextualIntelligenceStore
                            .sendProjectHealth(record)
                    }
                },
                onDismissPreview: {
                    skillStore.contextualIntelligenceStore.clear(key)
                },
                onOpenEvidence: {
                    contextualEvidenceSelection = ContextualEvidenceSelection(
                        reference: $0
                    )
                }
            )
    }

    private var taskReadinessSection: some View {
        ProjectOverviewSection(
            title: UIStrings.text("overview.task.title", "Task readiness"),
            subtitle: UIStrings.text(
                "overview.task.subtitle",
                "Describe a task to request optional, evidence-based interpretation of this verified project snapshot."
            ),
            systemImage: "checklist"
        ) {
            TaskCockpitPanel(
                taskText: Binding(
                    get: { skillStore.taskCockpitText },
                    set: { skillStore.taskCockpitText = $0 }
                ),
                currentTaskText: skillStore.selectedTaskCockpitInput,
                agentOptions: skillStore.taskCockpitAgentOptions,
                selectedAgentIDs: skillStore.taskCockpitSelectedAgentIDs,
                promptConfirmation: skillStore.taskCockpitPromptConfirmation,
                isPreviewingPrompt: skillStore.isPreviewingTaskCockpitPrompt,
                result: skillStore.taskCockpitResult,
                failedProviderOutput: skillStore.taskCockpitFailedProviderOutput,
                isBuilding: skillStore.isBuildingTaskCockpit,
                operationState: skillStore.taskCockpitOperationState,
                providerGateMessage: taskProviderGateMessage,
                onToggleAgent: skillStore.toggleTaskCockpitAgentSelection,
                onSelectAllAgents: skillStore.selectAllTaskCockpitAgents,
                onBuild: {
                    Task { await skillStore.buildTaskCockpit() }
                },
                onConfirmPrompt: {
                    Task { await skillStore.confirmTaskCockpitPromptAndBuild() }
                },
                onDismissPrompt: skillStore.clearTaskCockpitPromptConfirmation,
                onCancel: skillStore.cancelTaskCockpitBuild
            )
            taskReadinessEvidenceBinding
        }
    }

    @ViewBuilder
    private var taskReadinessEvidenceBinding: some View {
        let currentRevision = presentation.record?.sourceRevision
        if let pendingRevision = skillStore.taskCockpitPromptConfirmation?
            .preview.responseContract?.sourceRevision,
           pendingRevision != currentRevision {
            Label(
                UIStrings.text(
                    "intelligence.stale.preview",
                    "The evidence changed. Preview the provider request again."
                ),
                systemImage: "clock.arrow.circlepath"
            )
            .font(.callout)
            .foregroundStyle(.orange)
        }
        if let envelope = skillStore.taskCockpitResponseEnvelope,
           let contract = skillStore.taskCockpitResponseContract {
            let stale = envelope.sourceRevision != currentRevision
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Label(
                        UIStrings.text(
                            "intelligence.citations",
                            "Evidence citations"
                        ),
                        systemImage: "link"
                    )
                    .font(.subheadline.bold())
                    if stale {
                        Text(UIStrings.text("intelligence.stale", "Stale"))
                            .font(.caption.bold())
                            .foregroundStyle(.orange)
                    }
                }
                if stale {
                    Text(
                        UIStrings.text(
                            "intelligence.stale.output",
                            "This interpretation belongs to an older evidence revision. It remains visible for comparison but cannot drive any action."
                        )
                    )
                    .font(.callout)
                    .foregroundStyle(.orange)
                }
                ForEach(
                    contract.evidence.filter {
                        Set(envelope.evidenceRefs).contains($0.id)
                    }
                ) { evidence in
                    Button {
                        contextualEvidenceSelection = ContextualEvidenceSelection(
                            reference: evidence
                        )
                    } label: {
                        Label(evidence.summary, systemImage: "arrow.up.right.square")
                            .lineLimit(2)
                    }
                    .buttonStyle(.link)
                }
            }
            .padding(12)
            .nativePanelSurface()
        }
    }

    private var taskProviderGateMessage: String? {
        let status = skillStore.aiProviderStatus
        if !status.serviceAvailable {
            return UIStrings.localizedServiceMessage(
                status.disabledReason ?? UIStrings.aiProviderUnavailable
            )
        }
        if !status.configured || status.activeProfile == nil {
            return UIStrings.text(
                "taskCockpit.providerRequired",
                "Configure an AI provider before reviewing task readiness."
            )
        }
        if !status.enabled {
            return status.disabledReason.map(UIStrings.localizedServiceMessage)
                ?? UIStrings.text(
                    "taskCockpit.providerDisabled",
                    "The configured AI provider is disabled."
                )
        }
        return nil
    }

    private func refreshReadiness() {
        Task {
            await appContextStore.refreshProjectReadiness()
        }
    }
}

private struct ProjectStatusSection<Intelligence: View>: View {
    let presentation: ProjectOverviewPresentation
    let record: ProjectReadinessRecord
    let onRefresh: () -> Void
    @ViewBuilder let intelligence: () -> Intelligence

    var body: some View {
        ProjectOverviewSection(
            title: UIStrings.text("overview.status.title", "Project status"),
            subtitle: UIStrings.text(
                "overview.status.subtitle",
                "Deterministic source coverage and per-agent environment health."
            ),
            systemImage: "checkmark.seal"
        ) {
            VStack(alignment: .leading, spacing: 12) {
                statusBanners

                HStack(alignment: .top, spacing: 12) {
                    ProjectOverviewFact(
                        title: UIStrings.text("overview.status.health", "Environment"),
                        value: healthTitle(record.health),
                        systemImage: healthSystemImage(record.health),
                        tint: healthColor(record.health)
                    )
                    ProjectOverviewFact(
                        title: UIStrings.text("overview.status.coverage", "Coverage"),
                        value: ProjectOverviewPresentation.coverageText(record.coverage),
                        systemImage: record.coverage.isComplete
                            ? "checkmark.circle"
                            : "exclamationmark.triangle",
                        tint: record.coverage.isComplete ? .green : .orange
                    )
                    ProjectOverviewFact(
                        title: UIStrings.text(
                            "overview.status.lastRefresh",
                            "Last successful refresh"
                        ),
                        value: presentation.acceptedAtLabel
                            ?? UIStrings.text("overview.status.lastRefresh.none", "Unavailable"),
                        systemImage: "clock",
                        tint: .secondary
                    )
                }

                HStack {
                    Text(
                        String(
                            format: UIStrings.text(
                                "overview.status.snapshot.note",
                                "Accepted snapshot: %@"
                            ),
                            presentation.acceptedSnapshotLabel
                                ?? UIStrings.text(
                                    "overview.status.snapshot.none",
                                    "Unavailable"
                                )
                        )
                    )
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    Spacer()
                    if presentation.isRefreshing {
                        ProgressView()
                            .controlSize(.small)
                    }
                    Button {
                        onRefresh()
                    } label: {
                        Label(
                            UIStrings.text("overview.refresh", "Refresh"),
                            systemImage: "arrow.clockwise"
                        )
                    }
                    .disabled(presentation.isRefreshing)
                }

                Divider()

                VStack(alignment: .leading, spacing: 10) {
                    Text(UIStrings.text("overview.status.agents", "Agent health"))
                        .font(.headline)

                    if presentation.agents.isEmpty {
                        Text(
                            UIStrings.text(
                                "overview.status.agents.empty",
                                "No agent readiness row matches the current agent filter."
                            )
                        )
                        .font(.callout)
                        .foregroundStyle(.secondary)
                    } else {
                        ForEach(presentation.agents) { agent in
                            ProjectAgentReadinessRow(agent: agent)
                        }
                    }
                }
                Divider()
                VStack(alignment: .leading, spacing: 4) {
                    Text(
                        UIStrings.text(
                            "overview.intelligence.title",
                            "Project explanation"
                        )
                    )
                    .font(.headline)
                    Text(
                        UIStrings.text(
                            "overview.intelligence.subtitle",
                            "Verified facts stay primary; an optional model can explain and prioritize only the accepted evidence."
                        )
                    )
                    .font(.callout)
                    .foregroundStyle(.secondary)
                }
                intelligence()
            }
        }
    }

    @ViewBuilder
    private var statusBanners: some View {
        if presentation.isStale {
            ProjectOverviewBanner(
                message: presentation.message
                    ?? UIStrings.text(
                        "overview.state.stale",
                        "Showing the last accepted snapshot while current evidence is unavailable."
                    ),
                systemImage: "clock.arrow.circlepath",
                tint: .orange
            )
        }
        if presentation.isPartial {
            ProjectOverviewBanner(
                message: ProjectOverviewPresentation.incompleteReasonText(
                    record.coverage.incompleteReason
                ) ?? UIStrings.text(
                    "overview.state.partial",
                    "Required project evidence is incomplete."
                ),
                systemImage: "chart.bar.doc.horizontal",
                tint: .orange
            )
        }
        if presentation.isBlocked {
            ProjectOverviewBanner(
                message: record.blockingReasons.first?.summary
                    ?? UIStrings.text(
                        "overview.state.blocked",
                        "A deterministic blocker prevents a healthy result."
                    ),
                systemImage: "exclamationmark.octagon",
                tint: .red
            )
        }
    }
}

private struct ProjectAgentReadinessRow: View {
    let agent: AgentReadinessRecord

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: healthSystemImage(agent.health))
                .foregroundStyle(healthColor(agent.health))
                .frame(width: 18)

            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(DisplayText.agent(agent.agent.rawValue))
                        .font(.body.weight(.semibold))
                    Spacer()
                    Text(healthTitle(agent.health))
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(healthColor(agent.health))
                }

                Text(ProjectOverviewPresentation.coverageText(agent.coverage))
                    .font(.caption)
                    .foregroundStyle(.secondary)

                Text(
                    String(
                        format: UIStrings.text(
                            "overview.status.agentMetrics",
                            "%d effective skills · %d issues · %d conflicts"
                        ),
                        agent.effectiveSkillCount,
                        agent.issueCount,
                        agent.conflictCount
                    )
                )
                .font(.caption)
                .foregroundStyle(.secondary)

                if let reason = agent.blockingReasons.first?.summary {
                    Text(reason)
                        .font(.caption)
                        .foregroundStyle(.red)
                        .fixedSize(horizontal: false, vertical: true)
                } else if let reason = ProjectOverviewPresentation.incompleteReasonText(
                    agent.coverage.incompleteReason
                ) {
                    Text(reason)
                        .font(.caption)
                        .foregroundStyle(.orange)
                }
            }
        }
        .padding(10)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
    }
}

private struct ProjectAttentionSection: View {
    let rows: [ProjectOverviewPresentation.AttentionRow]
    let onOpenEvidence: (AttentionItem, [EvidenceRef]) -> Void
    let onPreviewAction: (AttentionItem, ActionDescriptorWire) -> Void

    var body: some View {
        ProjectOverviewSection(
            title: UIStrings.text("overview.attention.title", "Needs attention"),
            subtitle: UIStrings.text(
                "overview.attention.subtitle",
                "Deterministic issues ordered by the readiness projection."
            ),
            systemImage: "exclamationmark.triangle"
        ) {
            if rows.isEmpty {
                Label(
                    UIStrings.text(
                        "overview.attention.empty",
                        "No deterministic issue needs attention in this snapshot."
                    ),
                    systemImage: "checkmark.circle"
                )
                .foregroundStyle(.secondary)
            } else {
                VStack(alignment: .leading, spacing: 10) {
                    ForEach(rows) { row in
                        ProjectAttentionRow(
                            row: row,
                            onOpenEvidence: onOpenEvidence,
                            onPreviewAction: onPreviewAction
                        )
                    }
                }
            }
        }
    }
}

private struct ProjectAttentionRow: View {
    let row: ProjectOverviewPresentation.AttentionRow
    let onOpenEvidence: (AttentionItem, [EvidenceRef]) -> Void
    let onPreviewAction: (AttentionItem, ActionDescriptorWire) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Text(row.item.title)
                    .font(.body.weight(.semibold))
                Spacer()
                Text(severityTitle(row.item.severity))
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(severityColor(row.item.severity))
            }
            Text(row.item.summary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)

            if let agent = row.item.agent {
                Label(DisplayText.agent(agent.rawValue), systemImage: "person.crop.circle")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            HStack {
                Button {
                    onOpenEvidence(row.item, row.evidence)
                } label: {
                    Label(
                        UIStrings.text("overview.attention.evidence", "View evidence"),
                        systemImage: "doc.text.magnifyingglass"
                    )
                }

                if row.actions.isEmpty {
                    Text(noSafeActionTitle(row.item.noSafeActionReason))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                } else {
                    ForEach(Array(row.actions.enumerated()), id: \.element.id) { index, action in
                        Button {
                            onPreviewAction(row.item, action)
                        } label: {
                            Label(
                                row.actions.count == 1
                                    ? UIStrings.text(
                                        "overview.attention.preview",
                                        "Preview action"
                                    )
                                    : "\(UIStrings.text("overview.attention.preview", "Preview action")) \(index + 1)",
                                systemImage: "eye"
                            )
                        }
                    }
                }
            }
            .controlSize(.small)
        }
        .padding(10)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
    }
}

private struct ProjectContinueWorkSection: View {
    let sessions: [SessionContinuationRecord]
    let onOpenSession: (SessionContinuationRecord) -> Void
    let onPreviewResume: (SessionContinuationRecord) -> Void

    var body: some View {
        ProjectOverviewSection(
            title: UIStrings.text("overview.continue.title", "Continue work"),
            subtitle: UIStrings.text(
                "overview.continue.subtitle",
                "Recent project sessions with deterministic native continuation capability."
            ),
            systemImage: "bubble.left.and.bubble.right"
        ) {
            if sessions.isEmpty {
                Text(
                    UIStrings.text(
                        "overview.continue.empty",
                        "No recent session is available for this project and agent filter."
                    )
                )
                .font(.callout)
                .foregroundStyle(.secondary)
            } else {
                VStack(alignment: .leading, spacing: 10) {
                    ForEach(sessions) { session in
                        ProjectContinuationRow(
                            session: session,
                            onOpen: onOpenSession,
                            onPreviewResume: onPreviewResume
                        )
                    }
                }
            }
        }
    }
}

private struct ProjectContinuationRow: View {
    let session: SessionContinuationRecord
    let onOpen: (SessionContinuationRecord) -> Void
    let onPreviewResume: (SessionContinuationRecord) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            HStack(alignment: .firstTextBaseline) {
                Text(session.title)
                    .font(.body.weight(.semibold))
                    .lineLimit(1)
                Spacer()
                Text(modifiedDate)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            if let intent = session.intent, !intent.isEmpty {
                Text(intent)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }

            HStack {
                Label(DisplayText.agent(session.agent.rawValue), systemImage: "person.crop.circle")
                Text(ProjectOverviewPresentation.coverageText(session.coverage))
            }
            .font(.caption)
            .foregroundStyle(.secondary)

            HStack {
                Button {
                    onOpen(session)
                } label: {
                    Label(
                        UIStrings.text("overview.continue.open", "Open in Sessions"),
                        systemImage: "arrow.right.circle"
                    )
                }

                if session.resume.state == .supported {
                    Button {
                        onPreviewResume(session)
                    } label: {
                        Label(
                            UIStrings.text(
                                "overview.continue.preview",
                                "Preview continuation"
                            ),
                            systemImage: "doc.on.clipboard"
                        )
                    }
                } else {
                    Text(
                        ProjectOverviewPresentation.unsupportedResumeText(
                            session.resume.unsupportedReason
                        )
                    )
                    .font(.caption)
                    .foregroundStyle(.secondary)
                }
            }
            .controlSize(.small)
        }
        .padding(10)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
    }

    private var modifiedDate: String {
        Date(timeIntervalSince1970: Double(session.modifiedAt) / 1_000)
            .formatted(date: .abbreviated, time: .shortened)
    }
}

private struct ProjectOverviewSection<Content: View>: View {
    let title: String
    let subtitle: String
    let systemImage: String
    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .top, spacing: 10) {
                Image(systemName: systemImage)
                    .font(.title3.weight(.semibold))
                    .foregroundStyle(.secondary)
                    .frame(width: 22)
                VStack(alignment: .leading, spacing: 2) {
                    Text(title)
                        .font(.title3.weight(.semibold))
                    Text(subtitle)
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
            }
            content
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

private struct ProjectOverviewPlaceholder: View {
    let title: String
    let message: String
    let systemImage: String
    var actionTitle: String?
    var action: (() -> Void)?
    var showsProgress = false

    var body: some View {
        VStack(spacing: 12) {
            if showsProgress {
                ProgressView()
                    .controlSize(.large)
            } else {
                Image(systemName: systemImage)
                    .font(.system(size: 36))
                    .foregroundStyle(.secondary)
            }
            Text(title)
                .font(.title3.weight(.semibold))
            Text(message)
                .font(.callout)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 480)
            if let actionTitle, let action {
                Button(actionTitle, action: action)
                    .buttonStyle(.borderedProminent)
            }
        }
        .padding(28)
        .frame(maxWidth: .infinity, minHeight: 260)
        .nativePanelSurface()
    }
}

private struct ProjectOverviewFact: View {
    let title: String
    let value: String
    let systemImage: String
    let tint: Color

    var body: some View {
        HStack(alignment: .top, spacing: 8) {
            Image(systemName: systemImage)
                .foregroundStyle(tint)
                .frame(width: 18)
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.secondary)
                Text(value)
                    .font(.callout)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
    }
}

private struct ProjectOverviewBanner: View {
    let message: String
    let systemImage: String
    let tint: Color

    var body: some View {
        Label(message, systemImage: systemImage)
            .font(.callout)
            .foregroundStyle(tint)
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(tint.opacity(0.08), in: RoundedRectangle(cornerRadius: 8))
    }
}

private func healthTitle(_ health: EnvironmentHealthState) -> String {
    switch health {
    case .healthy:
        UIStrings.text("overview.health.healthy", "Healthy")
    case .review:
        UIStrings.text("overview.health.review", "Review")
    case .blocked:
        UIStrings.text("overview.health.blocked", "Blocked")
    }
}

private func healthSystemImage(_ health: EnvironmentHealthState) -> String {
    switch health {
    case .healthy: "checkmark.circle.fill"
    case .review: "exclamationmark.triangle.fill"
    case .blocked: "exclamationmark.octagon.fill"
    }
}

private func healthColor(_ health: EnvironmentHealthState) -> Color {
    switch health {
    case .healthy: .green
    case .review: .orange
    case .blocked: .red
    }
}

private func severityTitle(_ severity: AttentionSeverity) -> String {
    switch severity {
    case .critical: UIStrings.text("severity.critical", "Critical")
    case .error: UIStrings.text("severity.error", "Error")
    case .warning: UIStrings.text("severity.warning", "Warning")
    case .information: UIStrings.text("severity.information", "Information")
    }
}

private func severityColor(_ severity: AttentionSeverity) -> Color {
    switch severity {
    case .critical, .error: .red
    case .warning: .orange
    case .information: .secondary
    }
}

private func noSafeActionTitle(_ reason: NoSafeActionReason?) -> String {
    switch reason {
    case .unsupported:
        UIStrings.text("overview.action.unsupported", "No supported action")
    case .readOnlySource:
        UIStrings.text("overview.action.readOnly", "Read-only source")
    case .incompleteEvidence:
        UIStrings.text("overview.action.incomplete", "More evidence required")
    case .noGuardedWritePath:
        UIStrings.text("overview.action.noGuardedPath", "No guarded action path")
    case .manualReviewRequired:
        UIStrings.text("overview.action.manual", "Manual review required")
    case nil:
        UIStrings.text("overview.action.none", "No safe action")
    }
}
