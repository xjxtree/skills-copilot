import SwiftUI

struct ListCompletenessBadge: View {
    let state: ListCompletenessState

    var body: some View {
        Label(statusLabel, systemImage: statusImage)
            .font(.caption.bold())
            .foregroundStyle(statusColor)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(statusColor.opacity(0.1), in: Capsule())
            .accessibilityIdentifier("list-completeness.badge")
            .accessibilityLabel(UIStrings.listCompletenessStatus)
            .accessibilityValue(summary)
            .help(summary)
    }

    var statusLabel: String {
        switch state.completeness {
        case .complete:
            return UIStrings.listCompletenessComplete
        case .partial:
            return UIStrings.listCompletenessPartial
        case .incomplete:
            return UIStrings.listCompletenessIncomplete
        case .unknown:
            return UIStrings.listCompletenessUnknown
        }
    }

    private var statusImage: String {
        switch state.completeness {
        case .complete:
            return "checkmark.circle.fill"
        case .partial:
            return "circle.lefthalf.filled"
        case .incomplete:
            return "exclamationmark.triangle.fill"
        case .unknown:
            return "questionmark.circle"
        }
    }

    private var statusColor: Color {
        switch state.completeness {
        case .complete:
            return .green
        case .partial:
            return .accentColor
        case .incomplete:
            return .orange
        case .unknown:
            return .secondary
        }
    }

    private var summary: String {
        UIStrings.listCompletenessSummary(
            loadedCount: state.loadedCount,
            totalCount: state.totalCount,
            status: statusLabel,
            isLoading: state.loadingPhase != .idle
        )
    }
}

struct ListCompletenessFooter: View {
    let state: ListCompletenessState
    let onLoadMore: () -> Void
    let onLoadAll: () -> Void
    let onCancel: () -> Void
    var accessibilityIdentifierPrefix = "list-completeness"

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 10) {
                ListCompletenessBadge(state: state)
                Text(visibleSummary)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                Spacer(minLength: 8)
                ListPagingActions(
                    state: state,
                    onLoadMore: onLoadMore,
                    onLoadAll: onLoadAll,
                    onCancel: onCancel,
                    accessibilityIdentifierPrefix: accessibilityIdentifierPrefix
                )
            }
            if let reason = state.incompleteReason {
                Text(UIStrings.listIncompleteReason(reason.rawValue))
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private var visibleSummary: String {
        UIStrings.listCompletenessSummary(
            loadedCount: state.loadedCount,
            totalCount: state.totalCount,
            status: ListCompletenessBadge(state: state).statusLabel,
            isLoading: state.loadingPhase != .idle
        )
    }
}

struct ListPagingActions: View {
    let state: ListCompletenessState
    let onLoadMore: () -> Void
    let onLoadAll: () -> Void
    let onCancel: () -> Void
    var accessibilityIdentifierPrefix = "list-completeness"

    var body: some View {
        HStack(spacing: 8) {
            if state.loadingPhase == .all {
                cancelButton
            } else {
                if state.canLoadMore {
                    loadMoreButton
                }
                if state.canLoadAll {
                    loadAllButton
                }
            }
        }
        .controlSize(.small)
    }

    private var loadMoreButton: some View {
        Button(UIStrings.listCompletenessLoadMore, action: onLoadMore)
            .accessibilityIdentifier(loadMoreAccessibilityIdentifier)
            .accessibilityLabel(UIStrings.listCompletenessLoadMore)
            .help(UIStrings.listCompletenessLoadMoreHelp)
    }

    private var loadAllButton: some View {
        Button(UIStrings.listCompletenessLoadAll, action: onLoadAll)
            .accessibilityIdentifier(loadAllAccessibilityIdentifier)
            .accessibilityLabel(UIStrings.listCompletenessLoadAll)
            .help(UIStrings.listCompletenessLoadAllHelp)
    }

    private var cancelButton: some View {
        Button(UIStrings.listCompletenessCancelLoadingAll, action: onCancel)
            .accessibilityIdentifier(cancelAccessibilityIdentifier)
            .accessibilityLabel(UIStrings.listCompletenessCancelLoadingAll)
            .help(UIStrings.listCompletenessCancelHelp)
    }

    private var loadMoreAccessibilityIdentifier: String {
        accessibilityIdentifierPrefix == "list-completeness"
            ? "list-completeness.load-more"
            : "\(accessibilityIdentifierPrefix).load-more"
    }

    private var loadAllAccessibilityIdentifier: String {
        accessibilityIdentifierPrefix == "list-completeness"
            ? "list-completeness.load-all"
            : "\(accessibilityIdentifierPrefix).load-all"
    }

    private var cancelAccessibilityIdentifier: String {
        accessibilityIdentifierPrefix == "list-completeness"
            ? "list-completeness.cancel"
            : "\(accessibilityIdentifierPrefix).cancel"
    }
}

struct ExpandableSummaryList<Item: Identifiable, RowContent: View>: View {
    let items: [Item]
    let visibleLimit: Int
    let spacing: CGFloat
    let rowContent: (Item) -> RowContent
    @State private var isExpanded = false

    init(
        _ items: [Item],
        visibleLimit: Int,
        spacing: CGFloat = 4,
        @ViewBuilder rowContent: @escaping (Item) -> RowContent
    ) {
        self.items = items
        self.visibleLimit = max(0, visibleLimit)
        self.spacing = spacing
        self.rowContent = rowContent
    }

    var body: some View {
        VStack(alignment: .leading, spacing: spacing) {
            ForEach(visibleItems) { item in
                rowContent(item)
            }
            if !isExpanded && items.count > visibleLimit {
                Button(UIStrings.listCompletenessShowAll(items.count)) {
                    isExpanded = true
                }
                .buttonStyle(.link)
                .accessibilityIdentifier("list-completeness.show-all")
                .accessibilityLabel(UIStrings.listCompletenessShowAll(items.count))
                .accessibilityValue(UIStrings.listCompletenessSummaryValue(
                    visibleCount: visibleLimit,
                    totalCount: items.count
                ))
                .help(UIStrings.listCompletenessShowAllHelp(items.count))
            }
        }
    }

    private var visibleItems: [Item] {
        isExpanded ? items : Array(items.prefix(visibleLimit))
    }
}
