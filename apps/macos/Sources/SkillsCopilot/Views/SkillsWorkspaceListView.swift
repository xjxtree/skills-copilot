import SwiftUI

struct SkillsWorkspaceListView: View {
    @ObservedObject private var workspace: SkillWorkspaceStore
    let onAdd: () -> Void
    let onAgentFilterChange: (SkillAgentFilter) -> Void
    let onRefresh: () -> Void

    init(
        workspace: SkillWorkspaceStore,
        onAdd: @escaping () -> Void,
        onAgentFilterChange: @escaping (SkillAgentFilter) -> Void,
        onRefresh: @escaping () -> Void
    ) {
        self.workspace = workspace
        self.onAdd = onAdd
        self.onAgentFilterChange = onAgentFilterChange
        self.onRefresh = onRefresh
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            workspaceHeader
            Divider()
            filterControls
            statusNotices
            workspaceContent
            Divider()
            completenessFooter
        }
        .frame(minWidth: 360, idealWidth: 440)
        .accessibilityIdentifier("skills.workspace.list")
    }

    private var presentation: SkillsWorkspaceListPresentation {
        SkillsWorkspaceListPresentation(
            visibleAggregates: workspace.visibleAggregates,
            loadedAggregateCount: workspace.aggregates.count,
            sourceAggregateTotal: workspace.totalCount,
            completeness: workspace.listCompleteness,
            hasAcceptedSnapshot: workspace.sourceRevision != nil,
            isLoading: workspace.isLoading,
            isStale: workspace.isStale,
            errorMessage: workspace.errorMessage
        )
    }

    private var workspaceHeader: some View {
        HStack(alignment: .center, spacing: 12) {
            VStack(alignment: .leading, spacing: 3) {
                Text(UIStrings.text("skills.workspace.title", "Skills"))
                    .font(.title2.bold())
                Text(presentation.visibleCountSummary)
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
                            "skills.workspace.refreshing",
                            "Refreshing accepted skill evidence"
                        )
                    )
            }
            Button(action: onAdd) {
                Label(
                    UIStrings.text("skills.workspace.add", "Add"),
                    systemImage: "plus"
                )
            }
            .keyboardShortcut("n", modifiers: [.command, .shift])
            .accessibilityIdentifier("skills.workspace.add")
            .help(
                UIStrings.text(
                    "skills.workspace.add.help",
                    "Open the integrated Skill Package Manager"
                )
            )
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
    }

    private var filterControls: some View {
        VStack(alignment: .leading, spacing: 10) {
            Picker(
                UIStrings.text("skills.workspace.view.label", "Skill View"),
                selection: workspaceViewBinding
            ) {
                ForEach(SkillsWorkspaceListPresentation.orderedViews) { view in
                    Text(view.presentationTitle).tag(view)
                }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .accessibilityIdentifier("skills.workspace.view")

            HStack(spacing: 8) {
                TextField(
                    UIStrings.text(
                        "skills.workspace.search.placeholder",
                        "Search skills"
                    ),
                    text: searchBinding
                )
                .textFieldStyle(.roundedBorder)
                .accessibilityIdentifier("skills.workspace.search")

                Picker(UIStrings.agent, selection: agentBinding) {
                    Text(UIStrings.text("filter.all", "All"))
                        .tag(SkillAgentFilter.all)
                    ForEach(SkillAgentFilter.managementCases) { agent in
                        Text(agent.title).tag(agent)
                    }
                }
                .frame(width: 130)
                .accessibilityIdentifier("skills.workspace.agent")

                Picker(UIStrings.sort, selection: sortOrderBinding) {
                    ForEach(SkillAggregateSortOrder.allCases) { order in
                        Text(order.presentationTitle).tag(order)
                    }
                }
                .frame(width: 120)
                .accessibilityIdentifier("skills.workspace.sort")

                Button {
                    sortDirectionBinding.wrappedValue = workspace.sortDirection == .ascending
                        ? .descending
                        : .ascending
                } label: {
                    Image(
                        systemName: workspace.sortDirection == .ascending
                            ? "arrow.up"
                            : "arrow.down"
                    )
                }
                .accessibilityIdentifier("skills.workspace.sort-direction")
                .accessibilityLabel(workspace.sortDirection.title)
                .help(workspace.sortDirection.title)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
    }

    @ViewBuilder
    private var statusNotices: some View {
        if presentation.isStale {
            SkillsWorkspaceNotice(
                title: UIStrings.text(
                    "skills.workspace.stale.title",
                    "Showing the last accepted snapshot"
                ),
                message: UIStrings.text(
                    "skills.workspace.stale.message",
                    "Current evidence could not be refreshed. Counts are marked incomplete until refresh succeeds."
                ),
                systemImage: "clock.badge.exclamationmark",
                color: .orange,
                onRetry: onRefresh
            )
        } else if let errorMessage = presentation.supportingErrorMessage {
            SkillsWorkspaceNotice(
                title: UIStrings.text(
                    "skills.workspace.refreshError.title",
                    "Refresh did not complete"
                ),
                message: errorMessage,
                systemImage: "exclamationmark.triangle",
                color: .orange,
                onRetry: onRefresh
            )
        }
    }

    @ViewBuilder
    private var workspaceContent: some View {
        switch presentation.contentState {
        case .loading:
            SkillsWorkspaceCenteredState(
                title: UIStrings.text(
                    "skills.workspace.loading.title",
                    "Loading skill aggregates"
                ),
                message: UIStrings.text(
                    "skills.workspace.loading.message",
                    "Reading the accepted project capability snapshot."
                ),
                systemImage: nil,
                showsProgress: true,
                actionTitle: nil,
                action: nil
            )
        case .failed(let message):
            SkillsWorkspaceCenteredState(
                title: UIStrings.text(
                    "skills.workspace.error.title",
                    "Skills could not be loaded"
                ),
                message: message,
                systemImage: "exclamationmark.triangle",
                showsProgress: false,
                actionTitle: UIStrings.retry,
                action: onRefresh
            )
        case .empty(let reason):
            emptyState(reason)
        case .ready:
            aggregateList
        }
    }

    private var aggregateList: some View {
        List(selection: selectionBinding) {
            ForEach(presentation.rows) { row in
                SkillAggregateWorkspaceRow(row: row)
                    .tag(row.id)
            }
        }
        .listStyle(.plain)
        .accessibilityIdentifier("skills.workspace.aggregates")
    }

    @ViewBuilder
    private func emptyState(_ reason: SkillsWorkspaceEmptyReason) -> some View {
        switch reason {
        case .noAcceptedSnapshot:
            SkillsWorkspaceCenteredState(
                title: UIStrings.text(
                    "skills.workspace.empty.noSnapshot.title",
                    "No accepted skill snapshot"
                ),
                message: UIStrings.text(
                    "skills.workspace.empty.noSnapshot.message",
                    "Choose a project and refresh to inspect aggregate capabilities."
                ),
                systemImage: "square.stack.3d.up.slash",
                showsProgress: false,
                actionTitle: UIStrings.reload,
                action: onRefresh
            )
        case .noAggregates:
            SkillsWorkspaceCenteredState(
                title: UIStrings.text(
                    "skills.workspace.empty.noAggregates.title",
                    "No skill aggregates"
                ),
                message: UIStrings.text(
                    "skills.workspace.empty.noAggregates.message",
                    "The accepted snapshot contains no skill definitions. Check source completeness before adding a package."
                ),
                systemImage: "square.stack.3d.up.slash",
                showsProgress: false,
                actionTitle: UIStrings.text("skills.workspace.add", "Add"),
                action: onAdd
            )
        case .noMatches:
            SkillsWorkspaceCenteredState(
                title: UIStrings.text(
                    "skills.workspace.empty.noMatches.title",
                    "No matching skill aggregates"
                ),
                message: UIStrings.text(
                    "skills.workspace.empty.noMatches.message",
                    "Change the view, agent, or search filters. The accepted source snapshot remains available."
                ),
                systemImage: "line.3.horizontal.decrease.circle",
                showsProgress: false,
                actionTitle: nil,
                action: nil
            )
        }
    }

    private var completenessFooter: some View {
        ListCompletenessFooter(
            state: presentation.completeness,
            onLoadMore: {},
            onLoadAll: {},
            onCancel: workspace.cancelLoading,
            accessibilityIdentifierPrefix: "skills-workspace"
        )
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
    }

    private var workspaceViewBinding: Binding<SkillWorkspaceView> {
        Binding(
            get: { workspace.view },
            set: { view in
                configure(view: view)
            }
        )
    }

    private var agentBinding: Binding<SkillAgentFilter> {
        Binding(
            get: { workspace.agentFilter },
            set: { agent in
                onAgentFilterChange(agent)
            }
        )
    }

    private var searchBinding: Binding<String> {
        Binding(
            get: { workspace.searchText },
            set: { searchText in
                configure(searchText: searchText)
            }
        )
    }

    private var sortOrderBinding: Binding<SkillAggregateSortOrder> {
        Binding(
            get: { workspace.sortOrder },
            set: { order in
                configure(sortOrder: order)
            }
        )
    }

    private var sortDirectionBinding: Binding<SkillSortDirection> {
        Binding(
            get: { workspace.sortDirection },
            set: { direction in
                configure(sortDirection: direction)
            }
        )
    }

    private func configure(
        view: SkillWorkspaceView? = nil,
        agentFilter: SkillAgentFilter? = nil,
        searchText: String? = nil,
        sortOrder: SkillAggregateSortOrder? = nil,
        sortDirection: SkillSortDirection? = nil
    ) {
        workspace.configure(
            view: view ?? workspace.view,
            agentFilter: agentFilter ?? workspace.agentFilter,
            searchText: searchText ?? workspace.searchText,
            sortOrder: sortOrder ?? workspace.sortOrder,
            sortDirection: sortDirection ?? workspace.sortDirection
        )
    }

    private var selectionBinding: Binding<SkillAggregateRecord.ID?> {
        Binding(
            get: { workspace.selectedAggregateID },
            set: { workspace.selectAggregate(id: $0) }
        )
    }
}

private struct SkillAggregateWorkspaceRow: View {
    let row: SkillAggregateRowPresentation

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(row.title)
                    .font(.headline)
                    .lineLimit(1)
                Text(row.logicalSourceLabel)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                Spacer(minLength: 6)
                effectivenessBadge
            }

            if !row.summary.isEmpty {
                Text(row.summary)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }

            HStack(spacing: 6) {
                SkillsWorkspaceMetricBadge(
                    title: row.installedLabel,
                    color: .secondary
                )
                SkillsWorkspaceMetricBadge(
                    title: row.enabledLabel,
                    color: .blue
                )
                SkillsWorkspaceMetricBadge(
                    title: row.effectiveLabel,
                    color: .green
                )
            }

            HStack(spacing: 8) {
                Label(row.scopeSummary, systemImage: "folder")
                Label(row.agentSummary, systemImage: "cpu")
                if row.needsAttention {
                    Label(
                        row.attentionLabel,
                        systemImage: "exclamationmark.triangle.fill"
                    )
                    .foregroundStyle(.orange)
                }
                if !row.coverageIsComplete {
                    Label(
                        UIStrings.listCompletenessIncomplete,
                        systemImage: "circle.lefthalf.filled"
                    )
                    .foregroundStyle(.orange)
                }
            }
            .font(.caption2)
            .foregroundStyle(.secondary)
            .lineLimit(1)
        }
        .padding(.vertical, 7)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(row.accessibilitySummary)
    }

    private var effectivenessBadge: some View {
        Text(row.effectivenessLabel)
            .font(.caption.bold())
            .foregroundStyle(effectivenessColor)
            .padding(.horizontal, 7)
            .padding(.vertical, 3)
            .background(effectivenessColor.opacity(0.1), in: Capsule())
    }

    private var effectivenessColor: Color {
        switch row.effectiveness {
        case .effective: .green
        case .disabled: .secondary
        case .shadowed: .orange
        case .installedUnlinked: .blue
        case .broken: .red
        case .unavailable: .orange
        }
    }
}

private struct SkillsWorkspaceMetricBadge: View {
    let title: String
    let color: Color

    var body: some View {
        Text(title)
            .font(.caption2)
            .foregroundStyle(color)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(color.opacity(0.08), in: Capsule())
    }
}

private struct SkillsWorkspaceNotice: View {
    let title: String
    let message: String
    let systemImage: String
    let color: Color
    let onRetry: () -> Void

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: systemImage)
                .foregroundStyle(color)
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
        .background(color.opacity(0.07))
    }
}

private struct SkillsWorkspaceCenteredState: View {
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
