import SwiftUI

struct SessionsWorkspaceListView: View {
    @ObservedObject private var workspace: SessionWorkspaceStore
    @State private var loadAllTask: Task<Void, Never>?
    let onSetCriteria: (SessionWorkspaceCriteria) -> Void
    let onSelectSession: (String?) -> Void
    let onLoadNext: () async -> Void
    let onRefresh: () async -> Void
    let onCancelLoading: () -> Void
    let onAgentFilterChange: (ProductAgentID?) -> Void

    init(
        workspace: SessionWorkspaceStore,
        onSetCriteria: @escaping (SessionWorkspaceCriteria) -> Void,
        onSelectSession: @escaping (String?) -> Void,
        onLoadNext: @escaping () async -> Void,
        onRefresh: @escaping () async -> Void,
        onCancelLoading: @escaping () -> Void,
        onAgentFilterChange: @escaping (ProductAgentID?) -> Void
    ) {
        self.workspace = workspace
        self.onSetCriteria = onSetCriteria
        self.onSelectSession = onSelectSession
        self.onLoadNext = onLoadNext
        self.onRefresh = onRefresh
        self.onCancelLoading = onCancelLoading
        self.onAgentFilterChange = onAgentFilterChange
    }

    var body: some View {
        let presentation = listPresentation
        VStack(alignment: .leading, spacing: 0) {
            header(presentation)
            Divider()
            filterControls
            notices(presentation)

            switch presentation.contentState {
            case .loading:
                centeredState(
                    title: UIStrings.text(
                        "sessions.workspace.loading.title",
                        "Loading sessions"
                    ),
                    message: UIStrings.text(
                        "sessions.workspace.loading.message",
                        "Reading the accepted local session snapshot."
                    ),
                    systemImage: nil,
                    showsProgress: true,
                    actionTitle: nil
                )
            case .failed(let message):
                centeredState(
                    title: UIStrings.text(
                        "sessions.workspace.error.title",
                        "Sessions could not be loaded"
                    ),
                    message: message,
                    systemImage: "exclamationmark.triangle",
                    showsProgress: false,
                    actionTitle: UIStrings.retry
                )
            case .empty(let reason):
                emptyState(reason)
            case .ready:
                List(selection: selectionBinding) {
                    ForEach(presentation.projectGroups) { group in
                        Section(group.title) {
                            ForEach(group.rows) { row in
                                SessionWorkspaceRow(row: row)
                                    .tag(row.id)
                            }
                        }
                    }
                }
                .listStyle(.plain)
                .accessibilityIdentifier("sessions.workspace.rows")
            }

            Divider()
            ListCompletenessFooter(
                state: presentation.completeness,
                onLoadMore: { Task { await onLoadNext() } },
                onLoadAll: beginLoadAll,
                onCancel: cancelLoading,
                accessibilityIdentifierPrefix: "sessions-workspace"
            )
            .accessibilityIdentifier("sessions-workspace.completeness")
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
        }
        .frame(minWidth: 380, idealWidth: 460)
        .accessibilityIdentifier("sessions.workspace.list")
        .onDisappear(perform: cancelLoading)
    }

    private var listPresentation: SessionWorkspaceListPresentation {
        let state = workspace.inventoryState
        let isLoading: Bool
        let isStale: Bool
        let errorMessage: String?
        switch state {
        case .loading:
            isLoading = true
            isStale = false
            errorMessage = nil
        case .refreshing:
            isLoading = true
            isStale = false
            errorMessage = nil
        case .stale(_, let displayError):
            isLoading = false
            isStale = true
            errorMessage = displayError
        case .failed(_, let displayError):
            isLoading = false
            isStale = false
            errorMessage = displayError
        case .empty, .fresh:
            isLoading = false
            isStale = false
            errorMessage = nil
        }
        let accepted = workspace.acceptedSnapshot
        let completeness = displayedCompleteness(
            workspace.inventoryCompleteness
        )
        return SessionWorkspaceListPresentation(
            visibleSessions: workspace.rows,
            loadedSessionCount: accepted?.result.sessionRows.count ?? 0,
            sourceSessionTotal: completeness.totalCount,
            completeness: completeness,
            project: workspace.project,
            scope: workspace.criteria.scope,
            hasAcceptedSnapshot: accepted != nil,
            isLoading: isLoading,
            isStale: isStale,
            errorMessage: errorMessage
        )
    }

    private func header(
        _ presentation: SessionWorkspaceListPresentation
    ) -> some View {
        HStack(alignment: .center, spacing: 12) {
            VStack(alignment: .leading, spacing: 3) {
                Text(UIStrings.text("sessions.workspace.title", "Sessions"))
                    .font(.title2.bold())
                Text(
                    "\(presentation.projectContextLabel) · \(presentation.visibleCountSummary)"
                )
                .font(.caption)
                .foregroundStyle(.secondary)
                Text(presentation.sourceCountSummary)
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }
            Spacer(minLength: 8)
            if presentation.isRefreshingAcceptedSnapshot {
                ProgressView()
                    .controlSize(.small)
                    .help(
                        UIStrings.text(
                            "sessions.workspace.refreshing",
                            "Refreshing accepted session evidence"
                        )
                    )
            }
            Button {
                Task { await onRefresh() }
            } label: {
                Label(UIStrings.reload, systemImage: "arrow.clockwise")
            }
            .disabled(presentation.completeness.loadingPhase != .idle)
            .keyboardShortcut("r", modifiers: [.command, .shift])
            .accessibilityIdentifier("sessions.workspace.refresh")
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
    }

    private var filterControls: some View {
        VStack(alignment: .leading, spacing: 10) {
            Picker(
                UIStrings.scope,
                selection: scopeBinding
            ) {
                ForEach(LocalSessionScopeFilter.allCases) { scope in
                    Text(scope.workspacePresentationTitle).tag(scope)
                }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .accessibilityIdentifier("sessions.workspace.scope")

            HStack(spacing: 8) {
                TextField(
                    UIStrings.text(
                        "sessions.workspace.search.placeholder",
                        "Search sessions"
                    ),
                    text: searchBinding
                )
                .textFieldStyle(.roundedBorder)
                .accessibilityIdentifier("sessions.workspace.search")

                Picker(UIStrings.agent, selection: agentBinding) {
                    Text(UIStrings.text("filter.all", "All"))
                        .tag(ProductAgentID?.none)
                    ForEach(SessionWorkspaceListPresentation.orderedAgents, id: \.self) {
                        agent in
                        Text(DisplayText.agent(agent.rawValue))
                            .tag(ProductAgentID?.some(agent))
                    }
                }
                .frame(width: 130)
                .accessibilityIdentifier("sessions.workspace.agent")

                Picker(UIStrings.sort, selection: sortBinding) {
                    ForEach(LocalSessionSortOrder.allCases) { sort in
                        Text(sort.workspacePresentationTitle).tag(sort)
                    }
                }
                .frame(width: 110)
                .accessibilityIdentifier("sessions.workspace.sort")

                Button {
                    var criteria = workspace.criteria
                    criteria.direction = criteria.direction == .ascending
                        ? .descending
                        : .ascending
                    onSetCriteria(criteria)
                } label: {
                    Image(
                        systemName: workspace.criteria.direction == .ascending
                            ? "arrow.up"
                            : "arrow.down"
                    )
                }
                .accessibilityIdentifier("sessions.workspace.sort-direction")
                .accessibilityLabel(
                    UIStrings.text("sort.direction", "Direction")
                )
                .accessibilityValue(workspace.criteria.direction.title)
                .help(workspace.criteria.direction.title)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
    }

    @ViewBuilder
    private func notices(
        _ presentation: SessionWorkspaceListPresentation
    ) -> some View {
        if presentation.isStale {
            SessionWorkspaceNotice(
                title: UIStrings.text(
                    "sessions.workspace.stale.title",
                    "Showing the last accepted snapshot"
                ),
                message: UIStrings.text(
                    "sessions.workspace.stale.message",
                    "Current session evidence could not be refreshed. The accepted rows remain visible with incomplete status."
                ),
                systemImage: "clock.badge.exclamationmark",
                onRetry: { Task { await onRefresh() } }
            )
        } else if let errorMessage = presentation.supportingErrorMessage {
            SessionWorkspaceNotice(
                title: UIStrings.text(
                    "sessions.workspace.refreshError.title",
                    "Refresh did not complete"
                ),
                message: errorMessage,
                systemImage: "exclamationmark.triangle",
                onRetry: { Task { await onRefresh() } }
            )
        }
    }

    @ViewBuilder
    private func emptyState(_ reason: SessionWorkspaceEmptyReason) -> some View {
        switch reason {
        case .noProject:
            centeredState(
                title: UIStrings.text(
                    "sessions.workspace.empty.noProject.title",
                    "Select a project"
                ),
                message: UIStrings.text(
                    "sessions.workspace.empty.noProject.message",
                    "Sessions are interpreted from the selected project context."
                ),
                systemImage: "folder.badge.questionmark",
                showsProgress: false,
                actionTitle: nil
            )
        case .noAcceptedSnapshot:
            centeredState(
                title: UIStrings.text(
                    "sessions.workspace.empty.noSnapshot.title",
                    "No accepted session snapshot"
                ),
                message: UIStrings.text(
                    "sessions.workspace.empty.noSnapshot.message",
                    "Refresh to inspect supported local session sources."
                ),
                systemImage: "bubble.left.and.text.bubble.right",
                showsProgress: false,
                actionTitle: UIStrings.reload
            )
        case .noSessions:
            centeredState(
                title: UIStrings.text(
                    "sessions.workspace.empty.noSessions.title",
                    "No local sessions"
                ),
                message: UIStrings.text(
                    "sessions.workspace.empty.noSessions.message",
                    "The accepted source snapshot contains no supported sessions."
                ),
                systemImage: "bubble.left.and.text.bubble.right",
                showsProgress: false,
                actionTitle: UIStrings.reload
            )
        case .noMatches:
            centeredState(
                title: UIStrings.text(
                    "sessions.workspace.empty.noMatches.title",
                    "No matching sessions"
                ),
                message: UIStrings.text(
                    "sessions.workspace.empty.noMatches.message",
                    "Change the project scope, agent, search, or sort controls. The accepted rows remain cached."
                ),
                systemImage: "line.3.horizontal.decrease.circle",
                showsProgress: false,
                actionTitle: nil
            )
        }
    }

    private func centeredState(
        title: String,
        message: String,
        systemImage: String?,
        showsProgress: Bool,
        actionTitle: String?
    ) -> some View {
        SessionWorkspaceCenteredState(
            title: title,
            message: message,
            systemImage: systemImage,
            showsProgress: showsProgress,
            actionTitle: actionTitle,
            action: actionTitle == nil ? nil : { Task { await onRefresh() } }
        )
    }

    private var scopeBinding: Binding<LocalSessionScopeFilter> {
        Binding(
            get: { workspace.criteria.scope },
            set: { scope in
                var criteria = workspace.criteria
                criteria.scope = scope
                onSetCriteria(criteria)
            }
        )
    }

    private var searchBinding: Binding<String> {
        Binding(
            get: { workspace.criteria.search },
            set: { search in
                var criteria = workspace.criteria
                criteria.search = search
                onSetCriteria(criteria)
            }
        )
    }

    private var sortBinding: Binding<LocalSessionSortOrder> {
        Binding(
            get: { workspace.criteria.sort },
            set: { sort in
                var criteria = workspace.criteria
                criteria.sort = sort
                onSetCriteria(criteria)
            }
        )
    }

    private var agentBinding: Binding<ProductAgentID?> {
        Binding(
            get: { workspace.agentFilter },
            set: onAgentFilterChange
        )
    }

    private var selectionBinding: Binding<String?> {
        Binding(
            get: { workspace.selectedSessionID },
            set: onSelectSession
        )
    }

    @MainActor
    private func loadAllAvailablePages() async {
        while !Task.isCancelled, workspace.inventoryCompleteness.canLoadMore {
            let cursor = workspace.acceptedSnapshot?.nextCursor
            await onLoadNext()
            guard !Task.isCancelled,
                  workspace.acceptedSnapshot?.nextCursor != cursor else {
                break
            }
        }
    }

    private func beginLoadAll() {
        guard loadAllTask == nil else { return }
        loadAllTask = Task { @MainActor in
            await loadAllAvailablePages()
            loadAllTask = nil
        }
    }

    private func cancelLoading() {
        loadAllTask?.cancel()
        loadAllTask = nil
        onCancelLoading()
    }

    private func displayedCompleteness(
        _ state: ListCompletenessState
    ) -> ListCompletenessState {
        guard loadAllTask != nil else { return state }
        return ListCompletenessState(
            loadedCount: state.loadedCount,
            totalCount: state.totalCount,
            hasMore: state.hasMore,
            isComplete: state.isComplete,
            completeness: state.completeness,
            incompleteReason: state.incompleteReason,
            loadingPhase: .all,
            canLoadMore: false,
            canLoadAll: false
        )
    }
}

private struct SessionWorkspaceRow: View {
    let row: SessionWorkspaceRowPresentation

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            Text(row.title)
                .font(.headline)
                .lineLimit(1)
            Text(row.compactSummary)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(2)
        }
        .padding(.vertical, 7)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(row.accessibilitySummary)
    }
}

private struct SessionWorkspaceNotice: View {
    let title: String
    let message: String
    let systemImage: String
    let onRetry: () -> Void

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: systemImage)
                .foregroundStyle(.orange)
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.subheadline.bold())
                Text(message)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }
            Spacer(minLength: 8)
            Button(UIStrings.retry, action: onRetry)
                .controlSize(.small)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 9)
        .background(Color.orange.opacity(0.07))
    }
}

private struct SessionWorkspaceCenteredState: View {
    let title: String
    let message: String
    let systemImage: String?
    let showsProgress: Bool
    let actionTitle: String?
    let action: (() -> Void)?

    var body: some View {
        VStack(spacing: 10) {
            if showsProgress {
                ProgressView()
                    .controlSize(.large)
            } else if let systemImage {
                Image(systemName: systemImage)
                    .font(.system(size: 32))
                    .foregroundStyle(.secondary)
            }
            Text(title)
                .font(.headline)
            Text(message)
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 360)
            if let actionTitle, let action {
                Button(actionTitle, action: action)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(28)
    }
}
