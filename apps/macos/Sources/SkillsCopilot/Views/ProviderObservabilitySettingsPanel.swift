import SwiftUI

struct ProviderObservabilitySettingsPanel: View {
    @EnvironmentObject private var store: SkillStore

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header
                .padding(.horizontal, 20)
                .padding(.top, 20)
                .padding(.bottom, 14)

            Divider()

            ScrollView {
                VStack(alignment: .leading, spacing: 14) {
                    ProviderObservabilityDateRangeControls(
                        selectedRange: $store.providerObservabilityDateRange,
                        customStartDate: $store.providerObservabilityCustomStartDate,
                        customEndDate: $store.providerObservabilityCustomEndDate,
                        isLoading: store.isLoadingProviderObservability
                    ) {
                        Task { await store.loadProviderObservability() }
                    }
                    content
                }
                .padding(20)
            }
            .textSelection(.disabled)
        }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline, spacing: 10) {
                Label(UIStrings.providerObservabilityTitle, systemImage: "waveform.path.ecg.rectangle")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.providerObservabilityBoundary)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(2)
        }
    }

    @ViewBuilder
    private var content: some View {
        if let result = store.providerObservabilityResult {
            ProviderObservabilityDashboardSettingsView(result: result)
        } else {
            ProviderObservabilityLoadingCard(isLoading: store.isLoadingProviderObservability)
        }
    }
}

private struct ProviderObservabilityDashboardSettingsView: View {
    @EnvironmentObject private var store: SkillStore
    let result: ProviderObservabilityResult

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            if result.isUnavailable {
                ProviderObservabilityStatusCard(
                    title: UIStrings.providerObservabilityUnavailable,
                    message: result.fallbackReason.map(UIStrings.localizedServiceMessage),
                    systemImage: "exclamationmark.triangle"
                )
            } else {
                ProviderObservabilitySummaryStrip(metrics: summaryMetrics)

                if result.isDashboardEmpty {
                    ProviderObservabilityEmptyDashboard()
                } else {
                    ProviderObservabilitySettingsChartsPanel(result: result)
                }

                ProviderActivitySettingsSection(
                    rows: store.providerActivityRows,
                    completeness: store.providerActivityCompleteness,
                    errorMessage: store.providerActivityErrorMessage,
                    loadMore: {
                        Task { await store.loadMoreProviderActivity(loadAll: false) }
                    },
                    loadAll: {
                        Task { await store.loadMoreProviderActivity(loadAll: true) }
                    },
                    cancel: store.cancelProviderActivityLoadAll
                )
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private var callCount: Int {
        result.summary.callCount > 0 ? result.summary.callCount : result.callRows.count
    }

    private var successCount: Int {
        result.summary.successCount > 0 ? result.summary.successCount : result.callRows.filter { !$0.statusIsProblem }.count
    }

    private var estimatedTotalTokens: Int {
        result.summary.estimatedTotalTokens > 0 ? result.summary.estimatedTotalTokens : result.callRows.reduce(0) { $0 + $1.totalTokens }
    }

    private var averageDurationMS: Int? {
        if let value = result.summary.averageDurationMS, value > 0 {
            return value
        }
        let durations = result.callRows.compactMap(\.durationMS).filter { $0 > 0 }
        guard !durations.isEmpty else { return nil }
        return durations.reduce(0, +) / durations.count
    }

    private var successRateLabel: String {
        guard callCount > 0 else { return UIStrings.unknown }
        let rate = min(Double(successCount) / Double(callCount), 1.0) * 100.0
        return "\(rate.formatted(.number.precision(.fractionLength(0))))%"
    }

    private var summaryMetrics: [ProviderObservabilitySettingsMetric] {
        Array([
            ProviderObservabilitySettingsMetric(title: UIStrings.providerObservabilityCalls, value: compactIntLabel(callCount), systemImage: "network"),
            ProviderObservabilitySettingsMetric(title: UIStrings.providerObservabilitySuccessRate, value: successRateLabel, systemImage: "checkmark.circle"),
            ProviderObservabilitySettingsMetric(title: UIStrings.providerObservabilityAverageDuration, value: durationLabel(averageDurationMS), systemImage: "timer"),
            ProviderObservabilitySettingsMetric(title: UIStrings.providerObservabilityEstimatedTokens, value: compactIntLabel(estimatedTotalTokens), systemImage: "sum"),
            ProviderObservabilitySettingsMetric(title: UIStrings.providerObservabilityEstimatedCost, value: costLabel(result.summary.estimatedCostUSD), systemImage: "dollarsign.circle")
        ].prefix(UIOptimizationPresentation.settings.providerObservabilitySummaryMetricCount))
    }
}

private struct ProviderObservabilityLoadingCard: View {
    let isLoading: Bool

    var body: some View {
        HStack(spacing: 10) {
            if isLoading {
                ProgressView()
                    .controlSize(.small)
            } else {
                Image(systemName: "clock")
                    .foregroundStyle(.secondary)
            }

            Text(isLoading ? UIStrings.loading : UIStrings.providerObservabilityNoResult)
                .font(.callout)
                .foregroundStyle(.secondary)

            Spacer()
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

private struct ProviderObservabilityDateRangeControls: View {
    @Binding var selectedRange: ProviderObservabilityDateRangePreset
    @Binding var customStartDate: Date
    @Binding var customEndDate: Date
    let isLoading: Bool
    let refresh: () -> Void

    var body: some View {
        HStack(alignment: .center, spacing: 10) {
            Label(UIStrings.providerObservabilityDateRange, systemImage: "calendar")
                .font(.caption.bold())
                .foregroundStyle(.secondary)
                .frame(width: 86, alignment: .leading)

            Picker(UIStrings.providerObservabilityDateRange, selection: $selectedRange) {
                ForEach(ProviderObservabilityDateRangePreset.allCases, id: \.self) { preset in
                    Text(preset.title).tag(preset)
                }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .frame(maxWidth: 420)

            if selectedRange == .custom {
                DatePicker(
                    UIStrings.providerObservabilityStartDate,
                    selection: $customStartDate,
                    displayedComponents: .date
                )
                .datePickerStyle(.compact)
                .labelsHidden()
                .accessibilityLabel(UIStrings.providerObservabilityStartDate)

                DatePicker(
                    UIStrings.providerObservabilityEndDate,
                    selection: $customEndDate,
                    displayedComponents: .date
                )
                .datePickerStyle(.compact)
                .labelsHidden()
                .accessibilityLabel(UIStrings.providerObservabilityEndDate)
            }

            Spacer(minLength: 8)

            Button(action: refresh) {
                if isLoading {
                    ProgressView()
                        .controlSize(.small)
                        .frame(width: 16, height: 16)
                } else {
                    Image(systemName: "arrow.clockwise")
                }
            }
            .buttonStyle(.borderless)
            .help(UIStrings.providerObservabilityRefresh)
            .disabled(isLoading)
            .accessibilityLabel(UIStrings.providerObservabilityRefresh)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .frame(maxWidth: .infinity, minHeight: 44, alignment: .leading)
        .nativePanelSurface()
    }
}

private struct ProviderObservabilityStatusCard: View {
    let title: String
    let message: String?
    let systemImage: String

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Label(title, systemImage: systemImage)
                .font(.callout.bold())
                .foregroundStyle(.secondary)
            if let message, !message.isEmpty {
                Text(message)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

private struct ProviderObservabilityEmptyDashboard: View {
    var body: some View {
        ProviderObservabilityStatusCard(
            title: UIStrings.text("providerObservability.empty.dashboardTitle", "No provider metadata yet"),
            message: UIStrings.text(
                "providerObservability.empty.dashboardSummary",
                "No app-local provider prompt-run or call metadata has been recorded for this dashboard yet."
            ),
            systemImage: "tray"
        )
    }
}

private struct ProviderObservabilitySettingsMetric: Identifiable {
    let title: String
    let value: String
    let systemImage: String

    var id: String { "\(title):\(systemImage)" }
}

private struct ProviderObservabilitySummaryStrip: View {
    let metrics: [ProviderObservabilitySettingsMetric]

    var body: some View {
        HStack(alignment: .top, spacing: 8) {
            ForEach(metrics) { metric in
                ProviderObservabilitySettingsMetricChip(metric: metric)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct ProviderObservabilitySettingsMetricChip: View {
    let metric: ProviderObservabilitySettingsMetric

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Image(systemName: metric.systemImage)
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                    .frame(width: 14)
                Text(metric.title)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .minimumScaleFactor(0.85)
            }

            Text(metric.value)
                .font(.callout.bold())
                .lineLimit(1)
                .minimumScaleFactor(0.8)
                .truncationMode(.middle)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 9)
        .frame(maxWidth: .infinity, minHeight: 58, alignment: .leading)
        .nativePanelSurface()
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
                Label(UIStrings.providerObservabilityTopFiveSummary, systemImage: "chart.bar.xaxis")
                    .font(.headline)
                Spacer()
                Text(UIStrings.providerObservabilityChartsMode)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
            }

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
                title: UIStrings.providerObservabilityChartModelLatency,
                subtitle: UIStrings.providerObservabilityAverageDuration,
                systemImage: "timer",
                rows: modelLatencyRows
            )
            ProviderObservabilitySettingsChartCard(
                title: UIStrings.providerObservabilityChartDestinationCost,
                subtitle: UIStrings.providerObservabilityCalls,
                systemImage: "network",
                rows: destinationRows
            )
            ProviderObservabilitySettingsChartCard(
                title: UIStrings.providerObservabilityChartModelTaskConfidence,
                subtitle: UIStrings.providerObservabilityConfidence,
                systemImage: "target",
                rows: modelTaskConfidenceRows
            )
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
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
            ProviderObservabilitySettingsChartRow(label: UIStrings.providerObservabilitySuccesses, value: Double(success), valueLabel: compactIntLabel(success), detail: UIStrings.providerObservabilityCalls, color: .green),
            ProviderObservabilitySettingsChartRow(label: UIStrings.providerObservabilityFailures, value: Double(failure), valueLabel: compactIntLabel(failure), detail: UIStrings.providerObservabilityCalls, color: .red),
            ProviderObservabilitySettingsChartRow(label: UIStrings.providerObservabilityBlocked, value: Double(blocked), valueLabel: compactIntLabel(blocked), detail: UIStrings.providerObservabilityCalls, color: .orange),
        ]

        if summaryRows.contains(where: { $0.value > 0 }) {
            return topChartRows(summaryRows)
        }

        let groups = Dictionary(grouping: result.callRows, by: \.status)
        return topChartRows(groups.map { status, calls in
            ProviderObservabilitySettingsChartRow(
                label: status,
                value: Double(calls.count),
                valueLabel: compactIntLabel(calls.count),
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

    private var destinationRows: [ProviderObservabilitySettingsChartRow] {
        let costRows = result.destinationRows.compactMap { row -> ProviderObservabilitySettingsChartRow? in
            guard let cost = row.estimatedCostUSD, cost > 0 else { return nil }
            return ProviderObservabilitySettingsChartRow(label: row.label, value: cost, valueLabel: costLabel(cost), detail: callsDetail(row.callCount), color: .mint)
        }
        if !costRows.isEmpty {
            return topChartRows(costRows)
        }

        let dimensionRows = result.destinationRows
            .filter { $0.callCount > 0 }
            .map { row in
                ProviderObservabilitySettingsChartRow(label: row.label, value: Double(row.callCount), valueLabel: compactIntLabel(row.callCount), detail: UIStrings.providerObservabilityCalls, color: .mint)
            }
        if !dimensionRows.isEmpty {
            return topChartRows(dimensionRows)
        }

        return topChartRows(callAggregates(\.destinationHost).map { aggregate in
            ProviderObservabilitySettingsChartRow(label: aggregate.label, value: Double(aggregate.callCount), valueLabel: compactIntLabel(aggregate.callCount), detail: UIStrings.providerObservabilityCalls, color: .mint)
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
            .prefix(UIOptimizationPresentation.settings.providerObservabilityChartRowLimit)
            .map { $0 }
    }

    private func callsDetail(_ count: Int) -> String {
        "\(compactIntLabel(count)) \(UIStrings.providerObservabilityCalls.lowercased())"
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

private struct ProviderActivitySettingsSection: View {
    let rows: [ProviderActivityRow]
    let completeness: ListCompletenessState
    let errorMessage: String?
    let loadMore: () -> Void
    let loadAll: () -> Void
    let cancel: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Label(UIStrings.providerActivityTitle, systemImage: "list.bullet.rectangle")
                    .font(.headline)
                Spacer()
                Text(UIStrings.providerActivityRedactedDetail)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
            }

            if rows.isEmpty {
                Text(UIStrings.providerActivityEmpty)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(rows) { row in
                        ProviderActivitySettingsRow(row: row)
                        if row.id != rows.last?.id {
                            Divider()
                        }
                    }
                }
            }

            if let errorMessage, !errorMessage.isEmpty {
                Text(UIStrings.localizedServiceMessage(errorMessage))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }

            ListCompletenessFooter(
                state: completeness,
                onLoadMore: loadMore,
                onLoadAll: loadAll,
                onCancel: cancel,
                accessibilityIdentifierPrefix: "provider-activity"
            )
            .accessibilityIdentifier("provider-activity.completeness")
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
        .accessibilityIdentifier("provider-activity.list")
    }
}

private struct ProviderActivitySettingsRow: View {
    let row: ProviderActivityRow

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: row.kind == "provider_call" ? "network" : "text.bubble")
                .foregroundStyle(.secondary)
                .frame(width: 18)
            VStack(alignment: .leading, spacing: 3) {
                Text(row.title)
                    .font(.callout.weight(.medium))
                    .lineLimit(2)
                Text(row.subtitle)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
            }
            Spacer(minLength: 8)
            VStack(alignment: .trailing, spacing: 3) {
                Text(row.status)
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                Text(activityTimestamp(row.timestamp))
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
            }
        }
        .padding(.vertical, 8)
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier("provider-activity.row.\(row.id)")
    }

    private func activityTimestamp(_ milliseconds: Int) -> String {
        Date(timeIntervalSince1970: Double(milliseconds) / 1_000)
            .formatted(date: .abbreviated, time: .shortened)
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
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
    }

    private var maxValue: Double {
        rows.map(\.value).max() ?? 0
    }
}

private struct ProviderObservabilitySettingsBarRow: View {
    let row: ProviderObservabilitySettingsChartRow
    let maxValue: Double
    private let barWidth: CGFloat = 160

    var body: some View {
        HStack(alignment: .center, spacing: 8) {
            Text(row.label)
                .font(.caption.bold())
                .lineLimit(1)
                .truncationMode(.tail)
                .frame(width: 180, alignment: .leading)

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
                .frame(width: 64, alignment: .leading)

            Text(row.detail)
                .font(.caption2)
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(minHeight: 22)
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

private func compactIntLabel(_ value: Int) -> String {
    if value >= 1_000_000 {
        return "\((Double(value) / 1_000_000.0).providerObservabilitySettingsCompact)M"
    }
    if value >= 1_000 {
        return "\((Double(value) / 1_000.0).providerObservabilitySettingsCompact)k"
    }
    return "\(value)"
}

private func durationLabel(_ durationMS: Int?) -> String {
    guard let durationMS, durationMS > 0 else { return UIStrings.unknown }
    if durationMS >= 1_000 {
        let seconds = Double(durationMS) / 1_000.0
        return "\(seconds.formatted(.number.precision(.fractionLength(1))))s"
    }
    return "\(durationMS) ms"
}

private func costLabel(_ cost: Double?) -> String {
    guard let cost else { return UIStrings.unknown }
    return UIStrings.llmEstimatedCost(cost)
}

private extension Double {
    var providerObservabilitySettingsCompact: String {
        if self >= 10 {
            return formatted(.number.precision(.fractionLength(0)))
        }
        return formatted(.number.precision(.fractionLength(1)))
    }
}
