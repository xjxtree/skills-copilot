import SwiftUI

struct ProviderObservabilitySettingsPanel: View {
    @EnvironmentObject private var store: SkillStore
    @State private var selectedMode: ProviderObservabilitySettingsMode = .dashboard
    @State private var statusFilter = ProviderObservabilityLogFilter.allValue
    @State private var providerFilter = ProviderObservabilityLogFilter.allValue
    @State private var modelFilter = ProviderObservabilityLogFilter.allValue
    @State private var destinationFilter = ProviderObservabilityLogFilter.allValue
    @State private var showIssuesOnly = false
    @State private var searchText = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            header

            Picker(UIStrings.providerObservabilitySettingsMode, selection: $selectedMode) {
                ForEach(ProviderObservabilitySettingsMode.allCases) { mode in
                    Label(mode.title, systemImage: mode.systemImage).tag(mode)
                }
            }
            .pickerStyle(.segmented)
            .frame(maxWidth: 360, alignment: .leading)

            if let result = store.providerObservabilityResult {
                switch selectedMode {
                case .dashboard:
                    ProviderObservabilityDashboardSettingsView(result: result)
                case .logs:
                    ProviderObservabilityLogSettingsView(
                        result: result,
                        statusFilter: $statusFilter,
                        providerFilter: $providerFilter,
                        modelFilter: $modelFilter,
                        destinationFilter: $destinationFilter,
                        showIssuesOnly: $showIssuesOnly,
                        searchText: $searchText
                    )
                }
            } else {
                Label(UIStrings.providerObservabilityNoResult, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .padding(12)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .nativePanelSurface()
            }

            Spacer(minLength: 0)
        }
        .padding(4)
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline, spacing: 10) {
                Label(UIStrings.providerObservabilityTitle, systemImage: "waveform.path.ecg.rectangle")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.providerObservabilityBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)

            HStack(spacing: 10) {
                Button {
                    Task { await store.loadProviderObservability() }
                } label: {
                    Label(UIStrings.providerObservabilityAction, systemImage: "arrow.clockwise")
                }
                .disabled(store.isLoadingProviderObservability || store.isRefreshBusy)

                if store.isLoadingProviderObservability {
                    Label(UIStrings.loading, systemImage: "hourglass")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }

                Spacer()
            }
        }
    }
}

private enum ProviderObservabilitySettingsMode: String, CaseIterable, Identifiable {
    case dashboard
    case logs

    var id: String { rawValue }

    var title: String {
        switch self {
        case .dashboard:
            return UIStrings.providerObservabilityDashboard
        case .logs:
            return UIStrings.providerObservabilityLogs
        }
    }

    var systemImage: String {
        switch self {
        case .dashboard:
            return "chart.bar.xaxis"
        case .logs:
            return "list.bullet.rectangle"
        }
    }
}

private struct ProviderObservabilityDashboardSettingsView: View {
    let result: ProviderObservabilityResult

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(UIStrings.localizedServiceMessage(fallbackReason), systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            DetailMetricGrid {
                SummaryChip(title: UIStrings.providerObservabilityCalls, value: "\(callCount)", systemImage: "network")
                SummaryChip(title: UIStrings.providerObservabilitySuccesses, value: "\(successCount)", systemImage: "checkmark.circle")
                SummaryChip(title: UIStrings.providerObservabilityFailures, value: "\(failureCount)", systemImage: "xmark.octagon")
                SummaryChip(title: UIStrings.providerObservabilityBlocked, value: "\(blockedCount)", systemImage: "nosign")
                SummaryChip(title: UIStrings.providerObservabilityEstimatedTokens, value: "\(estimatedTotalTokens)", systemImage: "sum")
                SummaryChip(title: UIStrings.providerObservabilityEstimatedCost, value: costLabel(result.summary.estimatedCostUSD), systemImage: "dollarsign.circle")
                SummaryChip(title: UIStrings.providerObservabilityDuration, value: durationLabel(result.summary.totalDurationMS), systemImage: "timer")
            }

            if result.isDashboardEmpty {
                ProviderObservabilityEmptyDashboard()
            } else {
                ProviderObservabilityChartsPanel(result: result)

                if !result.summary.summaryText.isEmpty {
                    Text(result.summary.summaryText)
                        .font(.callout)
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
                }

                LazyVGrid(columns: [GridItem(.adaptive(minimum: 260), spacing: 12)], alignment: .leading, spacing: 12) {
                    ProviderObservabilityDimensionList(title: UIStrings.providerObservabilityProviders, rows: result.providerRows, systemImage: "person.crop.circle.badge.checkmark")
                    ProviderObservabilityDimensionList(title: UIStrings.providerObservabilityModels, rows: result.modelRows, systemImage: "cpu")
                    ProviderObservabilityDimensionList(title: UIStrings.providerObservabilityDestinations, rows: result.destinationRows, systemImage: "network")
                }

                LazyVGrid(columns: [GridItem(.adaptive(minimum: 300), spacing: 12)], alignment: .leading, spacing: 12) {
                    ProviderObservabilityHintList(title: UIStrings.providerObservabilityBudgetHints, rows: result.budgetHints, systemImage: "gauge.with.dots.needle.67percent")
                    ProviderObservabilityHintList(title: UIStrings.providerObservabilityUsageHints, rows: result.usageHints, systemImage: "chart.bar.xaxis")
                    ProviderObservabilityHintList(title: UIStrings.providerObservabilityRetention, rows: result.retentionRows + result.cleanupRecommendationRows, systemImage: "archivebox")
                }

                ProviderObservabilityModelTaskHistoryList(rows: result.modelTaskHistoryRows)
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
    }

    private var callCount: Int {
        result.summary.callCount > 0 ? result.summary.callCount : result.callRows.count
    }

    private var successCount: Int {
        result.summary.successCount > 0 ? result.summary.successCount : result.callRows.filter { !$0.statusIsProblem }.count
    }

    private var failureCount: Int {
        result.summary.failureCount > 0 ? result.summary.failureCount : result.callRows.filter(\.statusIsProblem).count
    }

    private var blockedCount: Int {
        result.summary.blockedCount
    }

    private var estimatedTotalTokens: Int {
        result.summary.estimatedTotalTokens > 0 ? result.summary.estimatedTotalTokens : result.callRows.reduce(0) { $0 + $1.totalTokens }
    }

}

private struct ProviderObservabilityEmptyDashboard: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(
                UIStrings.text("providerObservability.empty.dashboardTitle", "No provider metadata yet"),
                systemImage: "tray"
            )
                .font(.callout.bold())
            Text(UIStrings.text(
                "providerObservability.empty.dashboardSummary",
                "No app-local provider prompt-run or call metadata has been recorded for this dashboard yet."
            ))
            .font(.caption)
            .foregroundStyle(.secondary)
            .fixedSize(horizontal: false, vertical: true)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
    }
}

private struct ProviderObservabilityLogSettingsView: View {
    private static let renderedRowLimit = 40

    let result: ProviderObservabilityResult
    @Binding var statusFilter: String
    @Binding var providerFilter: String
    @Binding var modelFilter: String
    @Binding var destinationFilter: String
    @Binding var showIssuesOnly: Bool
    @Binding var searchText: String

    private var filteredRows: [ProviderObservabilityCallRow] {
        let query = searchText.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return result.callRows.filter { row in
            matches(row.status, filter: statusFilter)
                && matches(row.provider, filter: providerFilter)
                && matches(row.model, filter: modelFilter)
                && matches(row.destinationHost, filter: destinationFilter)
                && (!showIssuesOnly || row.statusIsProblem)
                && (query.isEmpty || searchableText(row).contains(query))
        }
    }

    var body: some View {
        let rows = filteredRows
        let visibleRows = Array(rows.prefix(Self.renderedRowLimit))
        let hiddenRowCount = max(0, rows.count - visibleRows.count)

        VStack(alignment: .leading, spacing: 12) {
            filterBar

            HStack {
                Text(UIStrings.providerObservabilityLogCount(rows.count, total: result.callRows.count))
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                Spacer()
            }

            if rows.isEmpty {
                Text(UIStrings.providerObservabilityNoFilteredCalls)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .padding(12)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
            } else {
                LazyVStack(alignment: .leading, spacing: 8) {
                    ForEach(visibleRows) { row in
                        ProviderObservabilitySettingsCallRow(row: row)
                    }
                    if hiddenRowCount > 0 {
                        Label(
                            UIStrings.providerObservabilityMoreRows(hiddenRowCount),
                            systemImage: "line.3.horizontal.decrease.circle"
                        )
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
    }

    private var filterBar: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass")
                    .foregroundStyle(.secondary)
                TextField(UIStrings.searchPrompt, text: $searchText)
                    .textFieldStyle(.plain)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
            .background(Color.white, in: RoundedRectangle(cornerRadius: 8))

            LazyVGrid(columns: [GridItem(.adaptive(minimum: 150), spacing: 8)], alignment: .leading, spacing: 8) {
                filterPicker(title: UIStrings.providerObservabilityStatusRows, selection: $statusFilter, options: optionValues(result.callRows.map(\.status)))
                filterPicker(title: UIStrings.providerObservabilityProviders, selection: $providerFilter, options: optionValues(result.callRows.map(\.provider)))
                filterPicker(title: UIStrings.providerObservabilityModels, selection: $modelFilter, options: optionValues(result.callRows.map(\.model)))
                filterPicker(title: UIStrings.providerObservabilityDestinations, selection: $destinationFilter, options: optionValues(result.callRows.map(\.destinationHost)))
                Toggle(UIStrings.providerObservabilityIssuesOnly, isOn: $showIssuesOnly)
                    .toggleStyle(.checkbox)
                    .font(.caption)
                    .accessibilityLabel(UIStrings.providerObservabilityIssuesOnly)
            }
        }
    }

    private func filterPicker(title: String, selection: Binding<String>, options: [String]) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(.caption2.bold())
                .foregroundStyle(.secondary)
                .lineLimit(1)

            Picker(title, selection: selection) {
                ForEach(options, id: \.self) { option in
                    Text(option).tag(option)
                }
            }
            .labelsHidden()
            .pickerStyle(.menu)
            .controlSize(.small)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 7)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
    }

    private func optionValues(_ values: [String]) -> [String] {
        let unique = values
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
        return [ProviderObservabilityLogFilter.allValue] + Array(Set(unique)).sorted()
    }

    private func matches(_ value: String, filter: String) -> Bool {
        filter == ProviderObservabilityLogFilter.allValue || value == filter
    }

    private func searchableText(_ row: ProviderObservabilityCallRow) -> String {
        [
            row.id,
            row.requestKind,
            row.action,
            row.provider,
            row.model,
            row.destinationHost,
            row.status,
            row.errorCode,
            row.errorMessage,
            row.detail
        ]
        .compactMap { $0 }
        .joined(separator: " ")
        .lowercased()
    }
}

private enum ProviderObservabilityLogFilter {
    static var allValue: String { UIStrings.text("filter.all", "All") }
}

private struct ProviderObservabilitySettingsCallRow: View {
    let row: ProviderObservabilityCallRow

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Label(callTitle, systemImage: row.statusIsProblem ? "exclamationmark.triangle" : "checkmark.circle")
                    .font(.callout.bold())
                    .foregroundStyle(row.statusIsProblem ? .orange : .primary)
                    .lineLimit(1)
                Spacer()
                Text(row.status)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
            }

            LazyVGrid(columns: [GridItem(.adaptive(minimum: 150), spacing: 8)], alignment: .leading, spacing: 6) {
                MetadataPill(title: UIStrings.llmProvider, value: row.provider)
                MetadataPill(title: UIStrings.llmModel, value: row.model)
                MetadataPill(title: UIStrings.llmPromptDestination, value: row.destinationHost)
                MetadataPill(title: UIStrings.providerObservabilityDuration, value: durationLabel(row.durationMS))
                MetadataPill(title: UIStrings.providerObservabilityEstimatedTokens, value: "\(row.totalTokens)")
                MetadataPill(title: UIStrings.providerObservabilityEstimatedCost, value: costLabel(row.estimatedCostUSD))
            }

            if let error = errorText, !error.isEmpty {
                Label(error, systemImage: "exclamationmark.triangle")
                    .font(.caption)
                    .foregroundStyle(.orange)
                    .textSelection(.enabled)
            }

            if !row.detail.isEmpty {
                PrivacyEvidenceText(value: row.detail, font: .caption, lineLimit: 2)
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
    }

    private var callTitle: String {
        let action = row.requestKind == UIStrings.unknown ? row.action : row.requestKind
        return action.isEmpty ? row.id : action
    }

    private var errorText: String? {
        if let code = row.errorCode, let message = row.errorMessage, !message.isEmpty {
            return "\(code): \(UIStrings.localizedServiceMessage(message))"
        }
        return row.errorMessage.map(UIStrings.localizedServiceMessage) ?? row.errorCode
    }
}

private struct MetadataPill: View {
    let title: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(title)
                .font(.caption2)
                .foregroundStyle(.secondary)
            Text(value)
                .font(.caption.bold())
                .lineLimit(1)
                .textSelection(.enabled)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 6)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 7))
    }
}
