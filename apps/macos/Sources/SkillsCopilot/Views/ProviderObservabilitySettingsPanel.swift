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
        VStack(alignment: .leading, spacing: 0) {
            header
                .padding(.horizontal, 20)
                .padding(.top, 20)
                .padding(.bottom, 12)

            Picker(UIStrings.providerObservabilitySettingsMode, selection: $selectedMode) {
                ForEach(ProviderObservabilitySettingsMode.allCases) { mode in
                    Label(mode.title, systemImage: mode.systemImage).tag(mode)
                }
            }
            .pickerStyle(.segmented)
            .frame(maxWidth: 360, alignment: .leading)
            .padding(.horizontal, 20)
            .padding(.bottom, 14)

            Divider()

            ScrollView {
                VStack(alignment: .leading, spacing: 14) {
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
                }
                .padding(20)
            }
            .textSelection(.disabled)
        }
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
            }

            ProviderObservabilitySettingsMetricGrid(metrics: dashboardMetrics)

            if result.isDashboardEmpty {
                ProviderObservabilityEmptyDashboard()
            } else {
                ProviderObservabilitySettingsChartsPanel(result: result)

                if !result.summary.summaryText.isEmpty {
                    Text(result.summary.summaryText)
                        .font(.callout)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }

                ProviderObservabilitySettingsDimensionGroup(result: result)

                ProviderObservabilitySettingsHintGroup(result: result)

                ProviderObservabilitySettingsModelTaskHistoryList(rows: result.modelTaskHistoryRows)
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

    private var dashboardMetrics: [ProviderObservabilitySettingsMetric] {
        [
            ProviderObservabilitySettingsMetric(title: UIStrings.providerObservabilityCalls, value: "\(callCount)", systemImage: "network"),
            ProviderObservabilitySettingsMetric(title: UIStrings.providerObservabilitySuccesses, value: "\(successCount)", systemImage: "checkmark.circle"),
            ProviderObservabilitySettingsMetric(title: UIStrings.providerObservabilityFailures, value: "\(failureCount)", systemImage: "xmark.octagon"),
            ProviderObservabilitySettingsMetric(title: UIStrings.providerObservabilityBlocked, value: "\(blockedCount)", systemImage: "nosign"),
            ProviderObservabilitySettingsMetric(title: UIStrings.providerObservabilityEstimatedTokens, value: "\(estimatedTotalTokens)", systemImage: "sum"),
            ProviderObservabilitySettingsMetric(title: UIStrings.providerObservabilityEstimatedCost, value: costLabel(result.summary.estimatedCostUSD), systemImage: "dollarsign.circle"),
            ProviderObservabilitySettingsMetric(title: UIStrings.providerObservabilityDuration, value: durationLabel(result.summary.totalDurationMS), systemImage: "timer")
        ]
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

private struct ProviderObservabilitySettingsMetric: Identifiable {
    let title: String
    let value: String
    let systemImage: String

    var id: String { "\(title):\(systemImage)" }
}

private struct ProviderObservabilitySettingsMetricGrid: View {
    let metrics: [ProviderObservabilitySettingsMetric]
    private let columnCount = 3

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            ForEach(Array(metricRows.enumerated()), id: \.offset) { _, row in
                HStack(alignment: .top, spacing: 8) {
                    ForEach(row) { metric in
                        ProviderObservabilitySettingsMetricChip(metric: metric)
                    }
                    ForEach(0..<max(0, columnCount - row.count), id: \.self) { _ in
                        Color.clear
                            .frame(maxWidth: .infinity, minHeight: 54)
                    }
                }
            }
        }
    }

    private var metricRows: [[ProviderObservabilitySettingsMetric]] {
        stride(from: 0, to: metrics.count, by: columnCount).map { index in
            Array(metrics[index..<min(index + columnCount, metrics.count)])
        }
    }
}

private struct ProviderObservabilitySettingsMetricChip: View {
    let metric: ProviderObservabilitySettingsMetric

    var body: some View {
        HStack(alignment: .center, spacing: 10) {
            Image(systemName: metric.systemImage)
                .font(.title3)
                .foregroundStyle(.secondary)
                .frame(width: 24)

            VStack(alignment: .leading, spacing: 1) {
                Text(metric.title)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                Text(metric.value)
                    .font(.callout.bold())
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.horizontal, 10)
        .frame(maxWidth: .infinity, minHeight: 54, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .combine)
        .accessibilityLabel(metric.title)
        .accessibilityValue(metric.value)
    }
}

private struct ProviderObservabilitySettingsChartsPanel: View {
    let result: ProviderObservabilityResult

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Label(UIStrings.providerObservabilityChartsTitle, systemImage: "chart.bar.xaxis")
                    .font(.headline)
                Spacer()
                Text(UIStrings.providerObservabilityChartsMode)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.providerObservabilityChartsSummary)
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)

            VStack(alignment: .leading, spacing: 8) {
                ProviderObservabilitySettingsChartCard(
                    title: UIStrings.providerObservabilityChartStatus,
                    subtitle: UIStrings.providerObservabilityCalls,
                    systemImage: "checklist",
                    rows: statusChartRows
                )
                ProviderObservabilitySettingsChartCard(
                    title: UIStrings.providerObservabilityChartModelTokens,
                    subtitle: UIStrings.providerObservabilityEstimatedTokens,
                    systemImage: "cpu",
                    rows: modelTokenRows
                )
                ProviderObservabilitySettingsChartCard(
                    title: UIStrings.providerObservabilityChartDestinationCost,
                    subtitle: UIStrings.providerObservabilityEstimatedCost,
                    systemImage: "network",
                    rows: destinationCostRows
                )
                ProviderObservabilitySettingsChartCard(
                    title: UIStrings.providerObservabilityChartModelLatency,
                    subtitle: UIStrings.providerObservabilityAverageDuration,
                    systemImage: "timer",
                    rows: modelLatencyRows
                )
                ProviderObservabilitySettingsChartCard(
                    title: UIStrings.providerObservabilityChartModelTaskConfidence,
                    subtitle: UIStrings.providerObservabilityModelTaskHistory,
                    systemImage: "target",
                    rows: modelTaskConfidenceRows
                )
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
    }

    private var statusChartRows: [ProviderObservabilitySettingsChartRow] {
        let success = result.summary.successCount > 0
            ? result.summary.successCount
            : result.callRows.filter { !$0.statusIsProblem }.count
        let failure = result.summary.failureCount > 0
            ? result.summary.failureCount
            : result.callRows.filter(\.statusIsProblem).count
        let blocked = result.summary.blockedCount
        let summaryRows = [
            ProviderObservabilitySettingsChartRow(label: UIStrings.providerObservabilitySuccesses, value: Double(success), valueLabel: "\(success)", detail: UIStrings.providerObservabilityCalls, color: .green),
            ProviderObservabilitySettingsChartRow(label: UIStrings.providerObservabilityFailures, value: Double(failure), valueLabel: "\(failure)", detail: UIStrings.providerObservabilityCalls, color: .red),
            ProviderObservabilitySettingsChartRow(label: UIStrings.providerObservabilityBlocked, value: Double(blocked), valueLabel: "\(blocked)", detail: UIStrings.providerObservabilityCalls, color: .orange),
        ]

        if summaryRows.contains(where: { $0.value > 0 }) {
            return summaryRows
        }

        let groups = Dictionary(grouping: result.callRows, by: \.status)
        return topChartRows(groups.map { status, calls in
            ProviderObservabilitySettingsChartRow(
                label: status,
                value: Double(calls.count),
                valueLabel: "\(calls.count)",
                detail: UIStrings.providerObservabilityCalls,
                color: color(forStatus: status)
            )
        })
    }

    private var modelTokenRows: [ProviderObservabilitySettingsChartRow] {
        let dimensionRows = result.modelRows
            .filter { $0.estimatedTokens > 0 }
            .map { row in
                ProviderObservabilitySettingsChartRow(label: row.label, value: Double(row.estimatedTokens), valueLabel: compactIntLabel(row.estimatedTokens), detail: callsDetail(row.callCount), color: .blue)
            }
        if !dimensionRows.isEmpty {
            return topChartRows(dimensionRows)
        }

        return topChartRows(callAggregates(\.model).map { aggregate in
            ProviderObservabilitySettingsChartRow(label: aggregate.label, value: Double(aggregate.tokenCount), valueLabel: compactIntLabel(aggregate.tokenCount), detail: callsDetail(aggregate.callCount), color: .blue)
        })
    }

    private var destinationCostRows: [ProviderObservabilitySettingsChartRow] {
        let dimensionRows = result.destinationRows.compactMap { row -> ProviderObservabilitySettingsChartRow? in
            guard let cost = row.estimatedCostUSD, cost > 0 else { return nil }
            return ProviderObservabilitySettingsChartRow(label: row.label, value: cost, valueLabel: costLabel(cost), detail: callsDetail(row.callCount), color: .mint)
        }
        if !dimensionRows.isEmpty {
            return topChartRows(dimensionRows)
        }

        return topChartRows(callAggregates(\.destinationHost).compactMap { aggregate in
            guard aggregate.cost > 0 else { return nil }
            return ProviderObservabilitySettingsChartRow(label: aggregate.label, value: aggregate.cost, valueLabel: costLabel(aggregate.cost), detail: callsDetail(aggregate.callCount), color: .mint)
        })
    }

    private var modelLatencyRows: [ProviderObservabilitySettingsChartRow] {
        let dimensionRows = result.modelRows.compactMap { row -> ProviderObservabilitySettingsChartRow? in
            guard let duration = row.averageDurationMS, duration > 0 else { return nil }
            return ProviderObservabilitySettingsChartRow(label: row.label, value: Double(duration), valueLabel: durationLabel(duration), detail: callsDetail(row.callCount), color: .indigo)
        }
        if !dimensionRows.isEmpty {
            return topChartRows(dimensionRows)
        }

        return topChartRows(callAggregates(\.model).compactMap { aggregate in
            guard let duration = aggregate.averageDurationMS else { return nil }
            return ProviderObservabilitySettingsChartRow(label: aggregate.label, value: Double(duration), valueLabel: durationLabel(duration), detail: callsDetail(aggregate.callCount), color: .indigo)
        })
    }

    private var modelTaskConfidenceRows: [ProviderObservabilitySettingsChartRow] {
        topChartRows(result.modelTaskHistoryRows.compactMap { row in
            guard let confidence = row.confidenceScore else { return nil }
            let label = row.model == UIStrings.unknown ? row.title : row.model
            return ProviderObservabilitySettingsChartRow(label: label, value: Double(confidence), valueLabel: "\(confidence)%", detail: row.matchStatus, color: row.statusIsProblem ? .orange : .green)
        })
    }

    private func callAggregates(_ keyPath: KeyPath<ProviderObservabilityCallRow, String>) -> [ProviderObservabilitySettingsCallAggregate] {
        var groups: [String: ProviderObservabilitySettingsCallAggregate] = [:]
        for row in result.callRows {
            let label = row[keyPath: keyPath].isEmpty ? UIStrings.unknown : row[keyPath: keyPath]
            groups[label, default: ProviderObservabilitySettingsCallAggregate(label: label)].add(row)
        }
        return groups.values.sorted { left, right in
            if left.callCount == right.callCount {
                return left.label.localizedCaseInsensitiveCompare(right.label) == .orderedAscending
            }
            return left.callCount > right.callCount
        }
    }

    private func topChartRows(_ rows: [ProviderObservabilitySettingsChartRow]) -> [ProviderObservabilitySettingsChartRow] {
        rows
            .filter { $0.value > 0 }
            .sorted { left, right in
                if left.value == right.value {
                    return left.label.localizedCaseInsensitiveCompare(right.label) == .orderedAscending
                }
                return left.value > right.value
            }
            .prefix(5)
            .map { $0 }
    }

    private func callsDetail(_ count: Int) -> String {
        "\(count) \(UIStrings.providerObservabilityCalls.lowercased())"
    }

    private func compactIntLabel(_ value: Int) -> String {
        if value >= 1_000_000 {
            return "\((Double(value) / 1_000_000.0).providerObservabilitySettingsCompact)M"
        }
        if value >= 1_000 {
            return "\((Double(value) / 1_000.0).providerObservabilitySettingsCompact)k"
        }
        return "\(value)"
    }

    private func color(forStatus status: String) -> Color {
        let value = status.lowercased()
        if value.contains("success") || value.contains("succeed") || value.contains("ok") {
            return .green
        }
        if value.contains("fail") || value.contains("error") || value.contains("timeout") {
            return .red
        }
        if value.contains("block") {
            return .orange
        }
        return .blue
    }
}

private struct ProviderObservabilitySettingsChartCard: View {
    let title: String
    let subtitle: String
    let systemImage: String
    let rows: [ProviderObservabilitySettingsChartRow]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Label(title, systemImage: systemImage)
                    .font(.callout.bold())
                    .lineLimit(1)
                Spacer()
                Text(subtitle)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            if rows.isEmpty {
                Text(UIStrings.providerObservabilityChartEmpty)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, minHeight: 28, alignment: .leading)
            } else {
                VStack(alignment: .leading, spacing: 7) {
                    ForEach(rows) { row in
                        ProviderObservabilitySettingsBarRow(row: row, maxValue: maxValue)
                    }
                }
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
    }

    private var maxValue: Double {
        rows.map(\.value).max() ?? 0
    }
}

private struct ProviderObservabilitySettingsBarRow: View {
    let row: ProviderObservabilitySettingsChartRow
    let maxValue: Double
    private let barWidth: CGFloat = 170

    var body: some View {
        HStack(alignment: .center, spacing: 8) {
            Text(row.label)
                .font(.caption.bold())
                .lineLimit(1)
                .truncationMode(.tail)
                .frame(width: 185, alignment: .leading)

            ZStack(alignment: .leading) {
                RoundedRectangle(cornerRadius: 3)
                    .fill(.secondary.opacity(0.12))
                    .frame(width: barWidth, height: 7)
                RoundedRectangle(cornerRadius: 3)
                    .fill(row.color.opacity(0.82))
                    .frame(width: filledWidth, height: 7)
            }

            Text(row.valueLabel)
                .font(.caption2.bold())
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .frame(width: 70, alignment: .leading)

            Text(row.detail)
                .font(.caption2)
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private var filledWidth: CGFloat {
        guard maxValue > 0, row.value > 0 else { return 0 }
        return max(2, barWidth * CGFloat(row.value / maxValue))
    }
}

private struct ProviderObservabilitySettingsChartRow: Identifiable {
    let label: String
    let value: Double
    let valueLabel: String
    let detail: String
    let color: Color

    var id: String { "\(label):\(valueLabel):\(detail)" }
}

private struct ProviderObservabilitySettingsCallAggregate {
    let label: String
    var callCount = 0
    var tokenCount = 0
    var cost = 0.0
    var durationTotalMS = 0
    var durationCount = 0

    var averageDurationMS: Int? {
        guard durationCount > 0 else { return nil }
        return durationTotalMS / durationCount
    }

    mutating func add(_ row: ProviderObservabilityCallRow) {
        callCount += 1
        tokenCount += row.totalTokens
        cost += row.estimatedCostUSD ?? 0
        if let duration = row.durationMS, duration > 0 {
            durationTotalMS += duration
            durationCount += 1
        }
    }
}

private struct ProviderObservabilitySettingsDimensionGroup: View {
    let result: ProviderObservabilityResult

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            ProviderObservabilitySettingsDimensionList(title: UIStrings.providerObservabilityProviders, rows: result.providerRows, systemImage: "person.crop.circle.badge.checkmark")
            ProviderObservabilitySettingsDimensionList(title: UIStrings.providerObservabilityModels, rows: result.modelRows, systemImage: "cpu")
            ProviderObservabilitySettingsDimensionList(title: UIStrings.providerObservabilityDestinations, rows: result.destinationRows, systemImage: "network")
        }
    }
}

private struct ProviderObservabilitySettingsDimensionList: View {
    let title: String
    let rows: [ProviderObservabilityDimensionRow]
    let systemImage: String

    private var visibleRows: [ProviderObservabilityDimensionRow] {
        Array(rows.prefix(3))
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Text(title)
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                Spacer()
                if !rows.isEmpty {
                    DenseCountBadge(count: rows.count)
                }
            }

            if rows.isEmpty {
                Text(UIStrings.providerObservabilityNoRows)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                VStack(alignment: .leading, spacing: 8) {
                    ForEach(visibleRows) { row in
                        ProviderObservabilitySettingsDimensionRow(row: row, systemImage: systemImage)
                    }
                    if rows.count > visibleRows.count {
                        ProviderObservabilitySettingsMoreRows(count: rows.count - visibleRows.count)
                    }
                }
            }
        }
    }
}

private struct ProviderObservabilitySettingsDimensionRow: View {
    let row: ProviderObservabilityDimensionRow
    let systemImage: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Label(row.label, systemImage: systemImage)
                    .font(.callout.bold())
                    .lineLimit(1)
                Spacer()
                Text(row.status)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
            }

            ProviderObservabilitySettingsMetadataList(rows: [
                CompactMetadataRow(label: UIStrings.providerObservabilityCalls, value: "\(row.callCount)", systemImage: "network"),
                CompactMetadataRow(label: UIStrings.providerObservabilitySuccesses, value: "\(row.successCount)", systemImage: "checkmark.circle"),
                CompactMetadataRow(label: UIStrings.providerObservabilityFailures, value: "\(row.failureCount)", systemImage: "xmark.octagon"),
                CompactMetadataRow(label: UIStrings.providerObservabilityBlocked, value: "\(row.blockedCount)", systemImage: "nosign"),
                CompactMetadataRow(label: UIStrings.providerObservabilityEstimatedTokens, value: "\(row.estimatedTokens)", systemImage: "sum"),
                CompactMetadataRow(label: UIStrings.providerObservabilityEstimatedCost, value: costLabel(row.estimatedCostUSD), systemImage: "dollarsign.circle"),
                CompactMetadataRow(label: UIStrings.providerObservabilityAverageDuration, value: durationLabel(row.averageDurationMS), systemImage: "timer")
            ])

            ProviderObservabilitySettingsInlineNotes(values: row.notes)
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
    }
}

private struct ProviderObservabilitySettingsHintGroup: View {
    let result: ProviderObservabilityResult

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            ProviderObservabilitySettingsHintList(title: UIStrings.providerObservabilityBudgetHints, rows: result.budgetHints, systemImage: "gauge.with.dots.needle.67percent")
            ProviderObservabilitySettingsHintList(title: UIStrings.providerObservabilityUsageHints, rows: result.usageHints, systemImage: "chart.bar.xaxis")
            ProviderObservabilitySettingsHintList(title: UIStrings.providerObservabilityRetention, rows: result.retentionRows + result.cleanupRecommendationRows, systemImage: "archivebox")
        }
    }
}

private struct ProviderObservabilitySettingsHintList: View {
    let title: String
    let rows: [ProviderObservabilityHintRow]
    let systemImage: String

    private var visibleRows: [ProviderObservabilityHintRow] {
        Array(rows.prefix(3))
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Text(title)
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                Spacer()
                if !rows.isEmpty {
                    DenseCountBadge(count: rows.count)
                }
            }

            if rows.isEmpty {
                Text(UIStrings.providerObservabilityNoRows)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                VStack(alignment: .leading, spacing: 8) {
                    ForEach(visibleRows) { row in
                        ProviderObservabilitySettingsHintRow(row: row, systemImage: systemImage)
                    }
                    if rows.count > visibleRows.count {
                        ProviderObservabilitySettingsMoreRows(count: rows.count - visibleRows.count)
                    }
                }
            }
        }
    }
}

private struct ProviderObservabilitySettingsHintRow: View {
    let row: ProviderObservabilityHintRow
    let systemImage: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Label(row.title, systemImage: systemImage)
                    .font(.callout.bold())
                    .lineLimit(1)
                Spacer()
                Text(row.severity)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
            }

            ProviderObservabilitySettingsMetadataList(rows: metadataRows)

            if !row.detail.isEmpty {
                ProviderObservabilitySettingsEvidenceText(value: row.detail, lineLimit: 2)
            }
            if let recommendation = row.recommendation, !recommendation.isEmpty {
                Label(recommendation, systemImage: "arrow.right.circle")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
    }

    private var metadataRows: [CompactMetadataRow] {
        var rows: [CompactMetadataRow] = []
        if let value = row.value, !value.isEmpty {
            rows.append(CompactMetadataRow(label: UIStrings.text("value", "Value"), value: value, systemImage: "number"))
        }
        if let threshold = row.threshold, !threshold.isEmpty {
            rows.append(CompactMetadataRow(label: UIStrings.providerObservabilityThreshold, value: threshold, systemImage: "slider.horizontal.3"))
        }
        return rows
    }
}

private struct ProviderObservabilitySettingsMetadataList: View {
    let rows: [CompactMetadataRow]
    var labelWidth: CGFloat = 128

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                    HStack(spacing: 5) {
                        if let systemImage = row.systemImage {
                            Image(systemName: systemImage)
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                                .frame(width: 13)
                        }
                        Text(row.label)
                            .font(.caption2.bold())
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                    }
                    .frame(width: labelWidth, alignment: .leading)

                    Text(row.value)
                        .font(.system(.caption, design: .monospaced))
                        .foregroundStyle(.primary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .help(row.value)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
                .frame(minHeight: 20, alignment: .center)
            }
        }
        .accessibilityElement(children: .contain)
    }
}

private struct ProviderObservabilitySettingsInlineNotes: View {
    let values: [String]

    var body: some View {
        if let note = values
            .map({ $0.trimmingCharacters(in: .whitespacesAndNewlines) })
            .first(where: { !$0.isEmpty }) {
            Label(note, systemImage: "info.circle")
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(2)
        }
    }
}

private struct ProviderObservabilitySettingsMoreRows: View {
    let count: Int

    var body: some View {
        Label(
            UIStrings.providerObservabilityMoreRows(count),
            systemImage: "line.3.horizontal.decrease.circle"
        )
        .font(.caption)
        .foregroundStyle(.secondary)
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
    }
}

private struct ProviderObservabilitySettingsEvidenceText: View {
    let value: String
    var font: Font = .caption
    var lineLimit: Int? = 2

    var body: some View {
        Text(displayValue)
            .font(font)
            .foregroundStyle(.secondary)
            .lineLimit(lineLimit)
            .truncationMode(.middle)
            .fixedSize(horizontal: false, vertical: lineLimit == nil)
            .help(displayValue)
    }

    private var displayValue: String {
        if DisplayText.isLikelyPath(value) {
            return DisplayText.privacyPath(value, privacyModeEnabled: true)
        }
        return value
    }
}

private struct ProviderObservabilitySettingsModelTaskHistoryList: View {
    let rows: [ProviderObservabilityModelTaskHistoryRow]

    private var visibleRows: ArraySlice<ProviderObservabilityModelTaskHistoryRow> {
        rows.prefix(UIOptimizationPresentation.settings.providerObservabilityDashboardHistoryLimit)
    }

    private var hiddenRowCount: Int {
        max(0, rows.count - visibleRows.count)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Text(UIStrings.providerObservabilityModelTaskHistory)
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                Spacer()
                if !rows.isEmpty {
                    DenseCountBadge(count: rows.count)
                }
            }

            if rows.isEmpty {
                Text(UIStrings.providerObservabilityNoModelTaskHistory)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                VStack(alignment: .leading, spacing: 8) {
                    ForEach(visibleRows) { row in
                        ProviderObservabilitySettingsHistoryRow(row: row)
                    }

                    if hiddenRowCount > 0 {
                        ProviderObservabilitySettingsMoreRows(count: hiddenRowCount)
                    }
                }
            }
        }
    }
}

private struct ProviderObservabilitySettingsHistoryRow: View {
    let row: ProviderObservabilityModelTaskHistoryRow

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Label(row.title, systemImage: row.statusIsProblem ? "questionmark.diamond" : "checkmark.seal")
                    .font(.callout.bold())
                    .foregroundStyle(row.statusIsProblem ? .orange : .primary)
                    .lineLimit(1)
                Spacer()
                Text(row.matchStatus)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            ProviderObservabilitySettingsMetadataList(rows: metadataRows, labelWidth: 112)

            if let note = primaryNote {
                Text(note)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }

            if let task = row.task, !task.isEmpty {
                ProviderObservabilitySettingsEvidenceText(value: task, font: .caption, lineLimit: 2)
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
    }

    private var metadataRows: [CompactMetadataRow] {
        var rows = [
            CompactMetadataRow(label: UIStrings.providerObservabilityTaskKind, value: row.taskKind, systemImage: "tag"),
            CompactMetadataRow(label: UIStrings.llmProvider, value: row.provider, systemImage: "network"),
            CompactMetadataRow(label: UIStrings.llmModel, value: row.model, systemImage: "cpu"),
            CompactMetadataRow(label: UIStrings.providerObservabilityDuration, value: durationLabel(row.latencyMS), systemImage: "timer"),
            CompactMetadataRow(label: UIStrings.providerObservabilityEstimatedTokens, value: "\(row.estimatedTotalTokens)", systemImage: "sum"),
            CompactMetadataRow(label: UIStrings.providerObservabilityEstimatedCost, value: costLabel(row.estimatedCostUSD), systemImage: "dollarsign.circle"),
            CompactMetadataRow(label: UIStrings.providerObservabilityConfidence, value: confidenceLabel, systemImage: "target"),
            CompactMetadataRow(label: UIStrings.providerObservabilityRedactionStatus, value: row.redactionStatus, systemImage: "eye.slash")
        ]
        if let destinationHost = row.destinationHost, !destinationHost.isEmpty {
            rows.insert(
                CompactMetadataRow(label: UIStrings.llmPromptDestination, value: destinationHost, systemImage: "point.3.connected.trianglepath.dotted"),
                at: 3
            )
        }
        return rows
    }

    private var confidenceLabel: String {
        guard let confidence = row.confidenceScore else { return UIStrings.unknown }
        return "\(confidence)%"
    }

    private var primaryNote: String? {
        (row.outcomeNotes + row.gapNotes + row.blockerNotes)
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .first { !$0.isEmpty }
    }
}

private struct ProviderObservabilityLogSettingsView: View {
    private static let renderedRowLimit = UIOptimizationPresentation.settings.providerObservabilityLogRowLimit

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
                VStack(alignment: .leading, spacing: 8) {
                    ForEach(visibleRows) { row in
                        ProviderObservabilitySettingsCallRow(row: row)
                    }
                    if hiddenRowCount > 0 {
                        ProviderObservabilitySettingsMoreRows(count: hiddenRowCount)
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

            VStack(alignment: .leading, spacing: 8) {
                HStack(alignment: .top, spacing: 8) {
                    filterPicker(title: UIStrings.providerObservabilityStatusRows, selection: $statusFilter, options: optionValues(result.callRows.map(\.status)))
                    filterPicker(title: UIStrings.providerObservabilityProviders, selection: $providerFilter, options: optionValues(result.callRows.map(\.provider)))
                }
                HStack(alignment: .top, spacing: 8) {
                    filterPicker(title: UIStrings.providerObservabilityModels, selection: $modelFilter, options: optionValues(result.callRows.map(\.model)))
                    filterPicker(title: UIStrings.providerObservabilityDestinations, selection: $destinationFilter, options: optionValues(result.callRows.map(\.destinationHost)))
                    Toggle(UIStrings.providerObservabilityIssuesOnly, isOn: $showIssuesOnly)
                        .toggleStyle(.checkbox)
                        .font(.caption)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 7)
                        .frame(maxWidth: .infinity, minHeight: 48, alignment: .leading)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
                        .accessibilityLabel(UIStrings.providerObservabilityIssuesOnly)
                }
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

            ProviderObservabilitySettingsMetadataList(rows: metadataRows)

            if let error = errorText, !error.isEmpty {
                Label(error, systemImage: "exclamationmark.triangle")
                    .font(.caption)
                    .foregroundStyle(.orange)
                    .lineLimit(2)
            }

            if !row.detail.isEmpty {
                ProviderObservabilitySettingsEvidenceText(value: row.detail, font: .caption, lineLimit: 2)
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

    private var metadataRows: [CompactMetadataRow] {
        [
            CompactMetadataRow(label: UIStrings.llmProvider, value: row.provider, systemImage: "network"),
            CompactMetadataRow(label: UIStrings.llmModel, value: row.model, systemImage: "cpu"),
            CompactMetadataRow(label: UIStrings.llmPromptDestination, value: row.destinationHost, systemImage: "point.3.connected.trianglepath.dotted"),
            CompactMetadataRow(label: UIStrings.providerObservabilityDuration, value: durationLabel(row.durationMS), systemImage: "timer"),
            CompactMetadataRow(label: UIStrings.providerObservabilityEstimatedTokens, value: "\(row.totalTokens)", systemImage: "sum"),
            CompactMetadataRow(label: UIStrings.providerObservabilityEstimatedCost, value: costLabel(row.estimatedCostUSD), systemImage: "dollarsign.circle")
        ]
    }
}

private extension Double {
    var providerObservabilitySettingsCompact: String {
        if self >= 10 {
            return formatted(.number.precision(.fractionLength(0)))
        }
        return formatted(.number.precision(.fractionLength(1)))
    }
}
