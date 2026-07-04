import AppKit
import SwiftUI

struct ProviderObservabilityPanel: View {
    let result: ProviderObservabilityResult?
    let isLoading: Bool
    let onLoad: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
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
                .textSelection(.enabled)

            HStack(spacing: 8) {
                Button {
                    onLoad()
                } label: {
                    Label(UIStrings.providerObservabilityAction, systemImage: "arrow.clockwise")
                }
                .disabled(isLoading)
                .help(UIStrings.providerObservabilityBoundary)

                if isLoading {
                    Label(UIStrings.loading, systemImage: "hourglass")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
            }

            if let result {
                ProviderObservabilityResultView(result: result)
            } else {
                Label(UIStrings.providerObservabilityNoResult, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            Label(UIStrings.llmReviewNoActions, systemImage: "nosign")
                .font(.callout)
                .foregroundStyle(.secondary)
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

struct ProviderObservabilityResultView: View {
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

            ProviderObservabilityChartsPanel(result: result)

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.routingAccuracyGeneratedBy, value: result.generatedBy)
                MetadataRow(label: UIStrings.providerObservabilityAppLocalOnly, value: result.appLocalOnly ? UIStrings.llmEnabled : UIStrings.llmSkillAnalysisEnabledUnsafe)
                MetadataRow(label: UIStrings.providerObservabilityMetadataRedacted, value: result.metadataRedacted ? UIStrings.llmEnabled : UIStrings.llmSkillAnalysisEnabledUnsafe)
                MetadataRow(label: UIStrings.providerObservabilityAverageDuration, value: durationLabel(result.summary.averageDurationMS))
                if let windowDays = result.filters.windowDays {
                    MetadataRow(label: UIStrings.routingAccuracyWindow, value: "\(windowDays)d")
                }
                if let limit = result.filters.limit {
                    MetadataRow(label: UIStrings.text("filter.limit", "Limit"), value: "\(limit)")
                }
                if let promptRequest = result.promptRequest {
                    MetadataRow(label: UIStrings.routingAccuracyPromptRequest, value: promptRequestLabel(promptRequest))
                }
            }

            if !result.summary.summaryText.isEmpty {
                Text(result.summary.summaryText)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            ProviderObservabilityDimensionList(title: UIStrings.providerObservabilityProviders, rows: result.providerRows, systemImage: "person.crop.circle.badge.checkmark")
            ProviderObservabilityDimensionList(title: UIStrings.providerObservabilityModels, rows: result.modelRows, systemImage: "cpu")
            ProviderObservabilityDimensionList(title: UIStrings.providerObservabilityDestinations, rows: result.destinationRows, systemImage: "network")
            ProviderObservabilityModelTaskHistoryList(rows: result.modelTaskHistoryRows)
            ProviderObservabilityCallList(rows: result.callRows)
            ProviderObservabilityIssueList(title: UIStrings.providerObservabilityStatusRows, rows: result.statusRows, systemImage: "list.bullet.rectangle")
            ProviderObservabilityIssueList(title: UIStrings.providerObservabilityErrorRows, rows: result.errorRows, systemImage: "exclamationmark.triangle")
            ProviderObservabilityHintList(title: UIStrings.providerObservabilityBudgetHints, rows: result.budgetHints, systemImage: "gauge.with.dots.needle.67percent")
            ProviderObservabilityHintList(title: UIStrings.providerObservabilityUsageHints, rows: result.usageHints, systemImage: "chart.bar.xaxis")
            ProviderObservabilityHintList(title: UIStrings.providerObservabilityRetention, rows: result.retentionRows + result.cleanupRecommendationRows, systemImage: "archivebox")
            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: result.gapNotes, systemImage: "puzzlepiece.extension")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: result.blockerNotes, systemImage: "exclamationmark.octagon")
            ProviderObservabilityEvidenceList(evidence: result.evidenceReferences)
            ProviderObservabilitySafetyList(safety: result.safetyFlags)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
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

    private func promptRequestLabel(_ promptRequest: ProviderObservabilityPromptRequest) -> String {
        let state = promptRequest.enabled ? UIStrings.llmEnabled : UIStrings.llmDisabled
        let copy = promptRequest.copyOnly ? UIStrings.llmPromptCopyOnly : UIStrings.llmSkillAnalysisEnabledUnsafe
        let redaction = promptRequest.redacted ? UIStrings.aiProviderAuditRedaction : UIStrings.llmSkillAnalysisEnabledUnsafe
        return "\(promptRequest.requestKind) · \(state) · \(copy) · \(redaction)"
    }
}

struct ProviderObservabilityChartsPanel: View {
    let result: ProviderObservabilityResult

    private let columns = [GridItem(.adaptive(minimum: 245), spacing: 10)]

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

            LazyVGrid(columns: columns, alignment: .leading, spacing: 10) {
                ProviderObservabilityChartCard(
                    title: UIStrings.providerObservabilityChartStatus,
                    subtitle: UIStrings.providerObservabilityCalls,
                    systemImage: "checklist",
                    rows: statusChartRows
                )
                ProviderObservabilityChartCard(
                    title: UIStrings.providerObservabilityChartModelTokens,
                    subtitle: UIStrings.providerObservabilityEstimatedTokens,
                    systemImage: "cpu",
                    rows: modelTokenRows
                )
                ProviderObservabilityChartCard(
                    title: UIStrings.providerObservabilityChartDestinationCost,
                    subtitle: UIStrings.providerObservabilityEstimatedCost,
                    systemImage: "network",
                    rows: destinationCostRows
                )
                ProviderObservabilityChartCard(
                    title: UIStrings.providerObservabilityChartModelLatency,
                    subtitle: UIStrings.providerObservabilityAverageDuration,
                    systemImage: "timer",
                    rows: modelLatencyRows
                )
                ProviderObservabilityChartCard(
                    title: UIStrings.providerObservabilityChartModelTaskConfidence,
                    subtitle: UIStrings.providerObservabilityModelTaskHistory,
                    systemImage: "target",
                    rows: modelTaskConfidenceRows
                )
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
    }

    private var statusChartRows: [ProviderObservabilityChartRow] {
        let success = result.summary.successCount > 0
            ? result.summary.successCount
            : result.callRows.filter { !$0.statusIsProblem }.count
        let failure = result.summary.failureCount > 0
            ? result.summary.failureCount
            : result.callRows.filter(\.statusIsProblem).count
        let blocked = result.summary.blockedCount
        let summaryRows = [
            ProviderObservabilityChartRow(
                label: UIStrings.providerObservabilitySuccesses,
                value: Double(success),
                valueLabel: "\(success)",
                detail: UIStrings.providerObservabilityCalls,
                color: .green
            ),
            ProviderObservabilityChartRow(
                label: UIStrings.providerObservabilityFailures,
                value: Double(failure),
                valueLabel: "\(failure)",
                detail: UIStrings.providerObservabilityCalls,
                color: .red
            ),
            ProviderObservabilityChartRow(
                label: UIStrings.providerObservabilityBlocked,
                value: Double(blocked),
                valueLabel: "\(blocked)",
                detail: UIStrings.providerObservabilityCalls,
                color: .orange
            ),
        ]

        if summaryRows.contains(where: { $0.value > 0 }) {
            return summaryRows
        }

        return chartRowsByStatusFromCalls
    }

    private var chartRowsByStatusFromCalls: [ProviderObservabilityChartRow] {
        let groups = Dictionary(grouping: result.callRows, by: \.status)
        return groups
            .map { status, calls in
                ProviderObservabilityChartRow(
                    label: status,
                    value: Double(calls.count),
                    valueLabel: "\(calls.count)",
                    detail: UIStrings.providerObservabilityCalls,
                    color: color(forStatus: status)
                )
            }
            .sorted { left, right in
                if left.value == right.value {
                    return left.label.localizedCaseInsensitiveCompare(right.label) == .orderedAscending
                }
                return left.value > right.value
            }
            .prefix(5)
            .map { $0 }
    }

    private var modelTokenRows: [ProviderObservabilityChartRow] {
        let dimensionRows = result.modelRows
            .filter { $0.estimatedTokens > 0 }
            .map { row in
                ProviderObservabilityChartRow(
                    label: row.label,
                    value: Double(row.estimatedTokens),
                    valueLabel: compactIntLabel(row.estimatedTokens),
                    detail: callsDetail(row.callCount),
                    color: .blue
                )
            }
        if !dimensionRows.isEmpty {
            return topChartRows(dimensionRows)
        }

        return topChartRows(callAggregates(\.model).map { aggregate in
            ProviderObservabilityChartRow(
                label: aggregate.label,
                value: Double(aggregate.tokenCount),
                valueLabel: compactIntLabel(aggregate.tokenCount),
                detail: callsDetail(aggregate.callCount),
                color: .blue
            )
        })
    }

    private var destinationCostRows: [ProviderObservabilityChartRow] {
        let dimensionRows = result.destinationRows
            .compactMap { row -> ProviderObservabilityChartRow? in
                guard let cost = row.estimatedCostUSD, cost > 0 else { return nil }
                return ProviderObservabilityChartRow(
                    label: row.label,
                    value: cost,
                    valueLabel: costLabel(cost),
                    detail: callsDetail(row.callCount),
                    color: .mint
                )
            }
        if !dimensionRows.isEmpty {
            return topChartRows(dimensionRows)
        }

        return topChartRows(callAggregates(\.destinationHost).compactMap { aggregate in
            guard aggregate.cost > 0 else { return nil }
            return ProviderObservabilityChartRow(
                label: aggregate.label,
                value: aggregate.cost,
                valueLabel: costLabel(aggregate.cost),
                detail: callsDetail(aggregate.callCount),
                color: .mint
            )
        })
    }

    private var modelLatencyRows: [ProviderObservabilityChartRow] {
        let dimensionRows = result.modelRows
            .compactMap { row -> ProviderObservabilityChartRow? in
                guard let duration = row.averageDurationMS, duration > 0 else { return nil }
                return ProviderObservabilityChartRow(
                    label: row.label,
                    value: Double(duration),
                    valueLabel: durationLabel(duration),
                    detail: callsDetail(row.callCount),
                    color: .indigo
                )
            }
        if !dimensionRows.isEmpty {
            return topChartRows(dimensionRows)
        }

        return topChartRows(callAggregates(\.model).compactMap { aggregate in
            guard let duration = aggregate.averageDurationMS else { return nil }
            return ProviderObservabilityChartRow(
                label: aggregate.label,
                value: Double(duration),
                valueLabel: durationLabel(duration),
                detail: callsDetail(aggregate.callCount),
                color: .indigo
            )
        })
    }

    private var modelTaskConfidenceRows: [ProviderObservabilityChartRow] {
        let rows = result.modelTaskHistoryRows.compactMap { row -> ProviderObservabilityChartRow? in
            guard let confidence = row.confidenceScore else { return nil }
            let label = row.model == UIStrings.unknown ? row.title : row.model
            return ProviderObservabilityChartRow(
                label: label,
                value: Double(confidence),
                valueLabel: "\(confidence)%",
                detail: row.matchStatus,
                color: row.statusIsProblem ? .orange : .green
            )
        }
        return topChartRows(rows)
    }

    private func callAggregates(_ keyPath: KeyPath<ProviderObservabilityCallRow, String>) -> [ProviderObservabilityCallAggregate] {
        var groups: [String: ProviderObservabilityCallAggregate] = [:]
        for row in result.callRows {
            let label = row[keyPath: keyPath].isEmpty ? UIStrings.unknown : row[keyPath: keyPath]
            groups[label, default: ProviderObservabilityCallAggregate(label: label)].add(row)
        }
        return groups.values.sorted { left, right in
            if left.callCount == right.callCount {
                return left.label.localizedCaseInsensitiveCompare(right.label) == .orderedAscending
            }
            return left.callCount > right.callCount
        }
    }

    private func topChartRows(_ rows: [ProviderObservabilityChartRow]) -> [ProviderObservabilityChartRow] {
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
            return "\((Double(value) / 1_000_000.0).providerObservabilityCompact)M"
        }
        if value >= 1_000 {
            return "\((Double(value) / 1_000.0).providerObservabilityCompact)k"
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

private struct ProviderObservabilityChartCard: View {
    let title: String
    let subtitle: String
    let systemImage: String
    let rows: [ProviderObservabilityChartRow]

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
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
                    .frame(minHeight: 86, alignment: .center)
                    .frame(maxWidth: .infinity)
            } else {
                VStack(alignment: .leading, spacing: 9) {
                    ForEach(rows) { row in
                        ProviderObservabilityBarRow(row: row, maxValue: maxValue)
                    }
                }
                .frame(minHeight: 86, alignment: .top)
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

private struct ProviderObservabilityBarRow: View {
    let row: ProviderObservabilityChartRow
    let maxValue: Double

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(row.label)
                    .font(.caption.bold())
                    .lineLimit(1)
                Spacer(minLength: 6)
                Text(row.valueLabel)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }

            GeometryReader { proxy in
                ZStack(alignment: .leading) {
                    RoundedRectangle(cornerRadius: 3)
                        .fill(.secondary.opacity(0.12))
                    RoundedRectangle(cornerRadius: 3)
                        .fill(row.color.opacity(0.82))
                        .frame(width: barWidth(in: proxy.size.width))
                }
            }
            .frame(height: 7)

            Text(row.detail)
                .font(.caption2)
                .foregroundStyle(.secondary)
                .lineLimit(1)
        }
    }

    private func barWidth(in width: CGFloat) -> CGFloat {
        guard maxValue > 0, row.value > 0 else { return 0 }
        return max(2, width * CGFloat(row.value / maxValue))
    }
}

private struct ProviderObservabilityChartRow: Identifiable {
    let label: String
    let value: Double
    let valueLabel: String
    let detail: String
    let color: Color

    var id: String { "\(label):\(valueLabel):\(detail)" }
}

private struct ProviderObservabilityCallAggregate {
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

private extension Double {
    var providerObservabilityCompact: String {
        if self >= 10 {
            return formatted(.number.precision(.fractionLength(0)))
        }
        return formatted(.number.precision(.fractionLength(1)))
    }
}

struct ProviderObservabilityDimensionList: View {
    let title: String
    let rows: [ProviderObservabilityDimensionRow]
    let systemImage: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if rows.isEmpty {
                Text(UIStrings.providerObservabilityNoRows)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 245), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(rows.prefix(6)) { row in
                        VStack(alignment: .leading, spacing: 8) {
                            HStack(alignment: .firstTextBaseline) {
                                Label(row.label, systemImage: systemImage)
                                    .font(.callout.bold())
                                    .lineLimit(1)
                                Spacer()
                                Text(row.status)
                                    .font(.caption2.bold())
                                    .foregroundStyle(.secondary)
                            }
                            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                                MetadataRow(label: UIStrings.providerObservabilityCalls, value: "\(row.callCount)")
                                MetadataRow(label: UIStrings.providerObservabilitySuccesses, value: "\(row.successCount)")
                                MetadataRow(label: UIStrings.providerObservabilityFailures, value: "\(row.failureCount)")
                                MetadataRow(label: UIStrings.providerObservabilityBlocked, value: "\(row.blockedCount)")
                                MetadataRow(label: UIStrings.providerObservabilityEstimatedTokens, value: "\(row.estimatedTokens)")
                                MetadataRow(label: UIStrings.providerObservabilityEstimatedCost, value: costLabel(row.estimatedCostUSD))
                                MetadataRow(label: UIStrings.providerObservabilityAverageDuration, value: durationLabel(row.averageDurationMS))
                            }
                            RoutingInlineList(title: UIStrings.providerObservabilityNotes, empty: UIStrings.providerObservabilityNoRows, values: row.notes, systemImage: "info.circle")
                            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: row.evidenceRefs, systemImage: "checklist")
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }
        }
    }
}

struct ProviderObservabilityModelTaskHistoryList: View {
    let rows: [ProviderObservabilityModelTaskHistoryRow]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.providerObservabilityModelTaskHistory)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if rows.isEmpty {
                Text(UIStrings.providerObservabilityNoModelTaskHistory)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(rows.prefix(8)) { row in
                    VStack(alignment: .leading, spacing: 8) {
                        HStack(alignment: .firstTextBaseline) {
                            Label(row.title, systemImage: row.statusIsProblem ? "questionmark.diamond" : "checkmark.seal")
                                .font(.callout.bold())
                                .foregroundStyle(row.statusIsProblem ? .orange : .primary)
                            Spacer()
                            Text(row.matchStatus)
                                .font(.caption2.bold())
                                .foregroundStyle(.secondary)
                        }
                        Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                            MetadataRow(label: UIStrings.providerObservabilityTaskKind, value: row.taskKind)
                            MetadataRow(label: UIStrings.providerObservabilitySourceKind, value: row.sourceKind)
                            MetadataRow(label: UIStrings.llmProvider, value: row.provider)
                            MetadataRow(label: UIStrings.llmModel, value: row.model)
                            if let destinationHost = row.destinationHost, !destinationHost.isEmpty {
                                MetadataRow(label: UIStrings.llmPromptDestination, value: destinationHost)
                            }
                            MetadataRow(label: UIStrings.providerObservabilityDuration, value: durationLabel(row.latencyMS))
                            MetadataRow(label: UIStrings.providerObservabilityEstimatedTokens, value: "\(row.estimatedTotalTokens)")
                            MetadataRow(label: UIStrings.providerObservabilityEstimatedCost, value: costLabel(row.estimatedCostUSD))
                            MetadataRow(label: UIStrings.providerObservabilityConfidence, value: confidenceLabel(row.confidenceScore))
                            MetadataRow(label: UIStrings.providerObservabilityRedactionStatus, value: row.redactionStatus)
                        }
                        if let task = row.task, !task.isEmpty {
                            PrivacyEvidenceText(value: task, font: .caption, lineLimit: nil)
                        }
                        RoutingInlineList(title: UIStrings.providerObservabilityNotes, empty: UIStrings.providerObservabilityNoRows, values: row.outcomeNotes, systemImage: "info.circle")
                        RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: row.gapNotes, systemImage: "puzzlepiece.extension")
                        RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: row.blockerNotes, systemImage: "exclamationmark.octagon")
                        RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: row.evidenceRefs, systemImage: "checklist")
                    }
                    .padding(10)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
                }
            }
        }
    }

    private func confidenceLabel(_ value: Int?) -> String {
        guard let value else { return UIStrings.unknown }
        return "\(value)%"
    }
}

struct ProviderObservabilityCallList: View {
    let rows: [ProviderObservabilityCallRow]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.providerObservabilityRecentCalls)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if rows.isEmpty {
                Text(UIStrings.providerObservabilityNoCalls)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(rows.prefix(8)) { row in
                    VStack(alignment: .leading, spacing: 8) {
                        HStack(alignment: .firstTextBaseline) {
                            Label(callTitle(row), systemImage: row.statusIsProblem ? "exclamationmark.triangle" : "checkmark.circle")
                                .font(.callout.bold())
                                .foregroundStyle(row.statusIsProblem ? .orange : .primary)
                            Spacer()
                            Text(row.status)
                                .font(.caption2.bold())
                                .foregroundStyle(.secondary)
                        }
                        Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                            MetadataRow(label: UIStrings.llmProvider, value: row.provider)
                            MetadataRow(label: UIStrings.llmModel, value: row.model)
                            MetadataRow(label: UIStrings.llmPromptDestination, value: row.destinationHost)
                            MetadataRow(label: UIStrings.providerObservabilityDuration, value: durationLabel(row.durationMS))
                            MetadataRow(label: UIStrings.providerObservabilityEstimatedTokens, value: UIStrings.llmTokenSummary(input: row.inputTokens, output: row.outputTokens, total: row.totalTokens))
                            MetadataRow(label: UIStrings.providerObservabilityEstimatedCost, value: costLabel(row.estimatedCostUSD))
                            MetadataRow(label: UIStrings.llmPromptCopyOnly, value: row.copyOnly ? UIStrings.llmEnabled : UIStrings.llmSkillAnalysisEnabledUnsafe)
                            MetadataRow(label: UIStrings.skillQualityProviderNotSent, value: row.providerRequestSent ? UIStrings.llmSkillAnalysisEnabledUnsafe : UIStrings.llmDisabled)
                        }
                        if let error = errorText(row), !error.isEmpty {
                            Label(error, systemImage: "exclamationmark.triangle")
                                .font(.caption)
                                .foregroundStyle(.orange)
                                .textSelection(.enabled)
                        }
                        if !row.detail.isEmpty {
                            PrivacyEvidenceText(value: row.detail, font: .caption, lineLimit: nil)
                        }
                        RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: row.evidenceRefs, systemImage: "checklist")
                        RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: row.safetyFlags, systemImage: "checkmark.shield")
                    }
                    .padding(10)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
                }
            }
        }
    }

    private func callTitle(_ row: ProviderObservabilityCallRow) -> String {
        let action = row.requestKind == UIStrings.unknown ? row.action : row.requestKind
        return action.isEmpty ? row.id : action
    }

    private func errorText(_ row: ProviderObservabilityCallRow) -> String? {
        if let code = row.errorCode, let message = row.errorMessage, !message.isEmpty {
            return "\(code): \(UIStrings.localizedServiceMessage(message))"
        }
        return row.errorMessage.map(UIStrings.localizedServiceMessage) ?? row.errorCode
    }
}

struct ProviderObservabilityIssueList: View {
    let title: String
    let rows: [ProviderObservabilityIssueRow]
    let systemImage: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if rows.isEmpty {
                Text(UIStrings.providerObservabilityNoRows)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(rows.prefix(8)) { row in
                    VStack(alignment: .leading, spacing: 6) {
                        HStack(alignment: .firstTextBaseline) {
                            Label(row.title, systemImage: systemImage)
                                .font(.callout.bold())
                            Spacer()
                            Text(row.severity)
                                .font(.caption2.bold())
                                .foregroundStyle(.secondary)
                        }
                        Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                            MetadataRow(label: UIStrings.aiProviderTestResult, value: row.status)
                            MetadataRow(label: UIStrings.providerObservabilityCalls, value: "\(row.count)")
                            if let provider = row.provider, !provider.isEmpty {
                                MetadataRow(label: UIStrings.llmProvider, value: provider)
                            }
                            if let model = row.model, !model.isEmpty {
                                MetadataRow(label: UIStrings.llmModel, value: model)
                            }
                            if let destinationHost = row.destinationHost, !destinationHost.isEmpty {
                                MetadataRow(label: UIStrings.llmPromptDestination, value: destinationHost)
                            }
                        }
                        if !row.detail.isEmpty {
                            PrivacyEvidenceText(value: row.detail, font: .caption, lineLimit: nil)
                        }
                        RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: row.evidenceRefs, systemImage: "checklist")
                    }
                    .padding(10)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
                }
            }
        }
    }
}

struct ProviderObservabilityHintList: View {
    let title: String
    let rows: [ProviderObservabilityHintRow]
    let systemImage: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if rows.isEmpty {
                Text(UIStrings.providerObservabilityNoRows)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(rows.prefix(8)) { row in
                    VStack(alignment: .leading, spacing: 6) {
                        HStack(alignment: .firstTextBaseline) {
                            Label(row.title, systemImage: systemImage)
                                .font(.callout.bold())
                            Spacer()
                            Text(row.severity)
                                .font(.caption2.bold())
                                .foregroundStyle(.secondary)
                        }
                        Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                            if let value = row.value, !value.isEmpty {
                                MetadataRow(label: UIStrings.text("value", "Value"), value: value)
                            }
                            if let threshold = row.threshold, !threshold.isEmpty {
                                MetadataRow(label: UIStrings.providerObservabilityThreshold, value: threshold)
                            }
                        }
                        if !row.detail.isEmpty {
                            PrivacyEvidenceText(value: row.detail, font: .caption, lineLimit: nil)
                        }
                        if let recommendation = row.recommendation, !recommendation.isEmpty {
                            Label(recommendation, systemImage: "arrow.right.circle")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                                .textSelection(.enabled)
                        }
                        RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: row.evidenceRefs, systemImage: "checklist")
                    }
                    .padding(10)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
                }
            }
        }
    }
}

struct ProviderObservabilityEvidenceList: View {
    let evidence: [ProviderObservabilityEvidenceReference]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.crossAgentReadinessEvidence)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if evidence.isEmpty {
                Text(UIStrings.crossAgentReadinessNoEvidence)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(evidence.prefix(8)) { item in
                    VStack(alignment: .leading, spacing: 4) {
                        Label(item.title, systemImage: "checklist")
                            .font(.callout)
                        PrivacyEvidenceText(value: item.detail, font: .caption, lineLimit: nil)
                        HStack(spacing: 8) {
                            if let source = item.source, !source.isEmpty {
                                PrivacyEvidenceText(value: source, font: .caption2, lineLimit: 1)
                            }
                            if let agent = item.agent, !agent.isEmpty {
                                Text(DisplayText.agent(agent))
                            }
                        }
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                    }
                }
            }
        }
    }
}

struct ProviderObservabilitySafetyList: View {
    let safety: ProviderObservabilitySafety

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.knowledgeSafetyFlags)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            Label(
                safety.allReadOnlyFlagsClear ? UIStrings.routingAccuracySafetyClear : UIStrings.llmSkillAnalysisEnabledUnsafe,
                systemImage: safety.allReadOnlyFlagsClear ? "checkmark.shield" : "exclamationmark.triangle"
            )
            .font(.callout)
            .foregroundStyle(.secondary)

            LazyVGrid(columns: [GridItem(.adaptive(minimum: 185), spacing: 8)], alignment: .leading, spacing: 8) {
                ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                    SafetyPill(label: row.label, isBlocked: !row.isUnsafe)
                }
            }

            if !safety.notes.isEmpty {
                ForEach(safety.notes.prefix(4), id: \.self) { note in
                    Label(note, systemImage: "info.circle")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
        }
    }

    private var rows: [(label: String, isUnsafe: Bool)] {
        [
            (UIStrings.skillQualityProviderNotSent, safety.providerRequestSent),
            (UIStrings.skillQualityWritesBlocked, safety.writeBackAllowed || safety.writeActionsAvailable),
            (UIStrings.skillQualityScriptsBlocked, safety.scriptExecutionAllowed || safety.executionActionsAvailable),
            (UIStrings.skillQualityMutationsBlocked, safety.configMutationAllowed || safety.snapshotCreated || safety.triageMutationAllowed),
            (UIStrings.skillQualityCredentialsBlocked, safety.credentialAccessed || safety.rawSecretReturned),
            (UIStrings.llmPromptRawPromptStored, safety.rawPromptPersisted),
            (UIStrings.llmPromptRawResponseStored, safety.rawResponsePersisted),
            (UIStrings.routingAccuracyRawTraceStored, safety.rawTracePersisted),
            (UIStrings.routingAccuracyCloudSync, safety.cloudSyncEnabled),
            (UIStrings.routingAccuracyTelemetry, safety.telemetryEnabled)
        ]
    }
}

func durationLabel(_ durationMS: Int?) -> String {
    guard let durationMS, durationMS > 0 else { return UIStrings.unknown }
    if durationMS >= 1_000 {
        let seconds = Double(durationMS) / 1_000.0
        return "\(seconds.formatted(.number.precision(.fractionLength(1))))s"
    }
    return "\(durationMS) ms"
}

func costLabel(_ cost: Double?) -> String {
    guard let cost else { return UIStrings.unknown }
    return UIStrings.llmEstimatedCost(cost)
}
