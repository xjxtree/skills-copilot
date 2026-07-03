import AppKit
import SwiftUI

struct RoutingAccuracyDashboardPanel: View {
    let dashboard: RoutingAccuracyDashboard?
    let isLoading: Bool
    let onLoad: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.routingAccuracyTitle, systemImage: "chart.xyaxis.line")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.routingAccuracyBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            HStack(spacing: 8) {
                Button {
                    onLoad()
                } label: {
                    Label(UIStrings.routingAccuracyLoadAction, systemImage: "arrow.clockwise")
                }
                .disabled(isLoading)

                if isLoading {
                    Label(UIStrings.llmPreparing, systemImage: "hourglass")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
                Spacer()
            }

            if let dashboard {
                RoutingAccuracyDashboardView(dashboard: dashboard)
            } else {
                Label(UIStrings.routingAccuracyNoDashboard, systemImage: "info.circle")
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

struct RoutingAccuracyDashboardView: View {
    let dashboard: RoutingAccuracyDashboard

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let fallbackReason = dashboard.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            DetailMetricGrid {
                SummaryChip(
                    title: UIStrings.routingAccuracyAccuracyRate,
                    value: dashboard.summary.accuracyRate.map(RoutingAccuracySummary.percentLabel)
                        ?? dashboard.summary.rateLabel(dashboard.summary.hitRate, count: dashboard.summary.hitCount),
                    systemImage: "target"
                )
                SummaryChip(
                    title: UIStrings.routingAccuracyWrongPickRate,
                    value: dashboard.summary.rateLabel(dashboard.summary.wrongPickRate, count: dashboard.summary.wrongPickCount),
                    systemImage: "xmark.octagon"
                )
                SummaryChip(
                    title: UIStrings.routingAccuracyImports,
                    value: RoutingAccuracySummary.countLabel(dashboard.summary.totalImports),
                    systemImage: "doc.text.magnifyingglass"
                )
                SummaryChip(
                    title: UIStrings.routingAccuracyKnownOutcomeRate,
                    value: dashboard.summary.knownOutcomeRate.map(RoutingAccuracySummary.percentLabel) ?? UIStrings.unknown,
                    systemImage: "checklist"
                )
                SummaryChip(
                    title: UIStrings.routingAccuracyRegressions,
                    value: RoutingAccuracySummary.countLabel(dashboard.summary.regressionCount),
                    systemImage: "chart.line.downtrend.xyaxis"
                )
                SummaryChip(
                    title: UIStrings.routingAccuracyAvgConfidence,
                    value: RoutingAccuracySummary.confidenceLabel(dashboard.summary.averageConfidence),
                    systemImage: "gauge.medium"
                )
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.routingAccuracyGeneratedBy, value: dashboard.generatedBy)
                MetadataRow(label: UIStrings.routingAccuracyCatalog, value: dashboard.catalogAvailable ? UIStrings.routingAccuracyAvailable : UIStrings.routingAccuracyUnavailableShort)
                MetadataRow(label: UIStrings.routingAccuracyWindow, value: windowLabel)
                MetadataRow(label: UIStrings.routingAccuracyBenchmarks, value: RoutingAccuracySummary.countLabel(dashboard.summary.totalBenchmarks))
                MetadataRow(label: UIStrings.routingAccuracyBenchmarkMatched, value: RoutingAccuracySummary.countLabel(dashboard.summary.benchmarkMatchedCount))
                MetadataRow(label: UIStrings.routingAccuracyBenchmarkGaps, value: RoutingAccuracySummary.countLabel(dashboard.summary.benchmarkGapCount))
                MetadataRow(label: UIStrings.routingAccuracyMissingBenchmarks, value: RoutingAccuracySummary.countLabel(dashboard.summary.missingBenchmarkCount))
                MetadataRow(label: UIStrings.routingAccuracyGaps, value: RoutingAccuracySummary.countLabel(dashboard.summary.gapCount))
                MetadataRow(label: UIStrings.routingAccuracyBlockers, value: RoutingAccuracySummary.countLabel(dashboard.summary.blockerCount))
                if let promptRequest = dashboard.promptRequest {
                    MetadataRow(label: UIStrings.routingAccuracyPromptRequest, value: promptRequestLabel(promptRequest))
                }
            }

            if !dashboard.summary.summaryText.isEmpty {
                Text(dashboard.summary.summaryText)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            RoutingAccuracyAgentList(agents: dashboard.agents)
            RoutingAccuracyHistoryList(history: dashboard.history)
            RoutingAccuracyGapList(gaps: dashboard.gaps)
            SkillQualityStringList(
                title: UIStrings.routingAccuracyBlockerNotes,
                empty: UIStrings.routingAccuracyNoBlockers,
                values: dashboard.blockerNotes,
                systemImage: "exclamationmark.octagon"
            )
            RoutingAccuracyEvidenceList(evidence: dashboard.recentEvidence)
            RoutingAccuracySafetyList(safety: dashboard.safetyFlags)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 6))
    }

    private var windowLabel: String {
        if let days = dashboard.filters.windowDays {
            return String(format: UIStrings.routingAccuracyDays, days)
        }
        return UIStrings.unknown
    }

    private func promptRequestLabel(_ promptRequest: RoutingAccuracyPromptRequest) -> String {
        let state = promptRequest.enabled ? UIStrings.llmEnabled : UIStrings.llmDisabled
        let copy = promptRequest.copyOnly ? UIStrings.llmPromptCopyOnly : UIStrings.llmSkillAnalysisEnabledUnsafe
        return "\(promptRequest.requestKind) · \(state) · \(copy)"
    }
}

struct RoutingAccuracyAgentList: View {
    let agents: [RoutingAccuracyAgentRow]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.routingAccuracyAgents)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if agents.isEmpty {
                Text(UIStrings.routingAccuracyNoAgents)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(agents) { agent in
                        HStack(alignment: .firstTextBaseline, spacing: 12) {
                            Text(DisplayText.agent(agent.agent))
                                .font(.callout.bold())
                                .frame(minWidth: 110, alignment: .leading)
                            Text(agent.hitRateLabel())
                                .font(.caption.monospacedDigit().bold())
                                .frame(width: 58, alignment: .trailing)
                            Text(agent.wrongPickRateLabel())
                                .font(.caption.monospacedDigit())
                                .frame(width: 58, alignment: .trailing)
                            Text(RoutingAccuracySummary.confidenceLabel(agent.averageConfidence))
                                .font(.caption.monospacedDigit())
                                .frame(width: 58, alignment: .trailing)
                            Text("\(agent.totalCount)")
                                .font(.caption.monospacedDigit())
                                .frame(width: 48, alignment: .trailing)
                            Spacer()
                            SafetyPill(
                                label: "\(UIStrings.routingAccuracyBenchmarkGaps) \(agent.benchmarkGapCount)",
                                isBlocked: agent.benchmarkGapCount == 0
                            )
                            SafetyPill(
                                label: "\(UIStrings.routingAccuracyRegressions) \(agent.regressionCount)",
                                isBlocked: agent.regressionCount == 0
                            )
                        }
                        .padding(.vertical, 7)
                        Divider()
                    }
                }
            }
        }
    }
}

struct RoutingAccuracyHistoryList: View {
    let history: [RoutingAccuracyHistoryPoint]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.routingAccuracyHistory)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if history.isEmpty {
                Text(UIStrings.routingAccuracyNoHistory)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 170), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(history.prefix(6)) { point in
                        VStack(alignment: .leading, spacing: 5) {
                            Text(point.label)
                                .font(.caption.bold())
                                .lineLimit(1)
                            MetadataLine(label: UIStrings.routingAccuracyHitRate, value: point.hitRate.map(RoutingAccuracySummary.percentLabel) ?? UIStrings.unknown)
                            MetadataLine(label: UIStrings.routingAccuracyWrongPickRate, value: point.wrongPickRate.map(RoutingAccuracySummary.percentLabel) ?? UIStrings.unknown)
                            MetadataLine(label: UIStrings.routingAccuracyRegressions, value: "\(point.regressionCount)")
                        }
                        .padding(9)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 6))
                    }
                }
            }
        }
    }
}

struct RoutingAccuracyGapList: View {
    let gaps: [RoutingAccuracyGap]

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(UIStrings.routingAccuracyGaps)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if gaps.isEmpty {
                Text(UIStrings.routingAccuracyNoGaps)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(gaps.prefix(6)) { gap in
                    VStack(alignment: .leading, spacing: 2) {
                        HStack(alignment: .firstTextBaseline) {
                            Label(gap.title, systemImage: "puzzlepiece.extension")
                                .font(.callout)
                            Spacer()
                            if let count = gap.count {
                                Text("\(count)")
                                    .font(.caption.monospacedDigit().bold())
                                    .foregroundStyle(.secondary)
                            }
                        }
                        PrivacyEvidenceText(value: gap.detail, font: .caption, lineLimit: nil)
                        if let severity = gap.severity, !severity.isEmpty {
                            Text(severity)
                                .font(.caption2.bold())
                                .foregroundStyle(.secondary)
                        }
                    }
                }
            }
        }
    }
}

struct RoutingAccuracyEvidenceList: View {
    let evidence: [RoutingAccuracyEvidenceRow]

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(UIStrings.routingAccuracyRecentEvidence)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if evidence.isEmpty {
                Text(UIStrings.routingAccuracyNoEvidence)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(evidence.prefix(6)) { item in
                    VStack(alignment: .leading, spacing: 2) {
                        Label(item.title, systemImage: "checklist")
                            .font(.callout)
                        HStack(spacing: 8) {
                            if let agent = item.agent, !agent.isEmpty {
                                Text(DisplayText.agent(agent))
                            }
                            if let outcome = item.outcome, !outcome.isEmpty {
                                Text(outcome)
                            }
                            if let observedAt = item.observedAt {
                                Text(DisplayText.timestamp(observedAt))
                            }
                        }
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        PrivacyEvidenceText(value: item.detail, font: .caption, lineLimit: nil)
                        if let source = item.source, !source.isEmpty {
                            PrivacyEvidenceText(value: source, font: .caption2, lineLimit: 1)
                        }
                        if !item.evidenceRefs.isEmpty {
                            PrivacyEvidenceText(value: item.evidenceRefs.joined(separator: ", "), font: .caption2, lineLimit: 2)
                        }
                    }
                }
            }
        }
    }
}

struct RoutingAccuracySafetyList: View {
    let safety: RoutingAccuracySafety

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.routingAccuracySafetyFlags)
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
            (UIStrings.skillQualityWritesBlocked, safety.writeBackAllowed),
            (UIStrings.skillQualityScriptsBlocked, safety.scriptExecutionAllowed),
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

struct StaleDriftDetectionPanel: View {
    let result: StaleDriftDetectionResult?
    let isDetecting: Bool
    let onDetect: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.staleDriftTitle, systemImage: "clock.badge.exclamationmark")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.staleDriftBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            HStack(spacing: 8) {
                Button {
                    onDetect()
                } label: {
                    Label(UIStrings.staleDriftDetectAction, systemImage: "waveform.path.ecg.rectangle")
                }
                .disabled(isDetecting)

                if isDetecting {
                    Label(UIStrings.llmPreparing, systemImage: "hourglass")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
                Spacer()
            }

            if let result {
                StaleDriftResultView(result: result)
            } else {
                Label(UIStrings.staleDriftNoResult, systemImage: "info.circle")
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

struct StaleDriftResultView: View {
    let result: StaleDriftDetectionResult

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            DetailMetricGrid {
                SummaryChip(title: UIStrings.staleDriftStale, value: "\(result.summary.staleCount)", systemImage: "clock")
                SummaryChip(title: UIStrings.staleDriftDrift, value: "\(result.summary.driftCount)", systemImage: "arrow.triangle.2.circlepath")
                SummaryChip(title: UIStrings.staleDriftCandidates, value: "\(candidateCount)", systemImage: "rectangle.stack")
                SummaryChip(title: UIStrings.staleDriftAffectedAgents, value: "\(result.summary.affectedAgentCount)", systemImage: "person.3")
                SummaryChip(title: UIStrings.staleDriftReadinessImpact, value: "\(readinessImpactCount)", systemImage: "gauge.medium.badge.exclamationmark")
                SummaryChip(title: UIStrings.staleDriftHighRisk, value: "\(result.summary.highRiskCount)", systemImage: "exclamationmark.triangle")
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.routingAccuracyGeneratedBy, value: result.generatedBy)
                MetadataRow(label: UIStrings.routingAccuracyCatalog, value: result.catalogAvailable ? UIStrings.routingAccuracyAvailable : UIStrings.routingAccuracyUnavailableShort)
                MetadataRow(label: UIStrings.staleDriftCandidates, value: "\(candidateCount)")
                MetadataRow(label: UIStrings.staleDriftReadinessImpact, value: "\(readinessImpactCount)")
                MetadataRow(label: UIStrings.crossAgentReadinessGapsIssues, value: "\(gapIssueCount)")
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

            StaleDriftCandidateList(rows: result.staleDriftRows)
            StaleDriftImpactList(title: UIStrings.staleDriftReadinessImpact, empty: UIStrings.staleDriftNoReadinessImpact, rows: result.readinessImpactRows, systemImage: "gauge.medium.badge.exclamationmark")
            StaleDriftImpactList(title: UIStrings.crossAgentReadinessGapsIssues, empty: UIStrings.crossAgentReadinessNoGapsIssues, rows: result.gapIssueRows, systemImage: "puzzlepiece.extension")
            CrossAgentReadinessEvidenceList(evidence: result.evidenceReferences)
            StaleDriftSafetyList(safety: result.safetyFlags)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 6))
    }

    private var candidateCount: Int {
        result.summary.candidateCount > 0 ? result.summary.candidateCount : result.staleDriftRows.count
    }

    private var readinessImpactCount: Int {
        result.summary.readinessImpactCount > 0 ? result.summary.readinessImpactCount : result.readinessImpactRows.count
    }

    private var gapIssueCount: Int {
        result.summary.gapIssueCount > 0 ? result.summary.gapIssueCount : result.gapIssueRows.count
    }

    private func promptRequestLabel(_ promptRequest: RoutingAccuracyPromptRequest) -> String {
        let state = promptRequest.enabled ? UIStrings.llmEnabled : UIStrings.llmDisabled
        let copy = promptRequest.copyOnly ? UIStrings.llmPromptCopyOnly : UIStrings.llmSkillAnalysisEnabledUnsafe
        return "\(promptRequest.requestKind) · \(state) · \(copy)"
    }
}

struct StaleDriftCandidateList: View {
    let rows: [StaleDriftRow]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.staleDriftCandidates)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if rows.isEmpty {
                Text(UIStrings.staleDriftNoCandidates)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 280), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(rows.prefix(8)) { row in
                        VStack(alignment: .leading, spacing: 8) {
                            HStack(alignment: .firstTextBaseline) {
                                Label(row.title, systemImage: row.kind.localizedCaseInsensitiveContains("drift") ? "arrow.triangle.2.circlepath" : "clock")
                                    .font(.callout.bold())
                                    .lineLimit(1)
                                Spacer()
                                Text(row.kind)
                                    .font(.caption2.bold())
                                    .foregroundStyle(.secondary)
                                if let severity = row.severity, !severity.isEmpty {
                                    Text(severity)
                                        .font(.caption2.bold())
                                        .foregroundStyle(.secondary)
                                }
                            }

                            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                                MetadataRow(label: UIStrings.agent, value: row.agent.map(DisplayText.agent) ?? row.skill?.agent.map(DisplayText.agent) ?? UIStrings.unknown)
                                MetadataRow(label: UIStrings.crossAgentReadinessBestSkill, value: row.skill?.name ?? UIStrings.unknown)
                                MetadataRow(label: UIStrings.scope, value: row.skill?.scope ?? UIStrings.unknown)
                                MetadataRow(label: UIStrings.state, value: row.skill?.state ?? UIStrings.unknown)
                                MetadataRow(label: UIStrings.staleDriftLastSeen, value: row.lastSeen ?? UIStrings.unknown)
                                MetadataRow(label: UIStrings.routingAccuracyAvgConfidence, value: row.confidenceLabel)
                            }

                            PrivacyEvidenceText(value: row.summary, font: .caption, lineLimit: nil)

                            if let current = row.currentSignal, !current.isEmpty {
                                Label(current, systemImage: "waveform.path.ecg")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                            if let expected = row.expectedSignal, !expected.isEmpty {
                                Label(expected, systemImage: "scope")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }

                            RoutingInlineList(title: UIStrings.staleDriftReasons, empty: UIStrings.staleDriftNoReasons, values: row.reasons, systemImage: "text.bubble")
                            RoutingInlineList(title: UIStrings.staleDriftSignals, empty: UIStrings.staleDriftNoSignals, values: row.signals, systemImage: "waveform.path.ecg")
                            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: row.evidenceRefs, systemImage: "checklist")
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }
        }
    }
}

struct StaleDriftImpactList: View {
    let title: String
    let empty: String
    let rows: [StaleDriftImpactRow]
    let systemImage: String

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if rows.isEmpty {
                Text(empty)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(rows.prefix(6)) { row in
                    VStack(alignment: .leading, spacing: 2) {
                        HStack(alignment: .firstTextBaseline) {
                            Label(row.title, systemImage: systemImage)
                                .font(.callout)
                            Spacer()
                            if let severity = row.severity, !severity.isEmpty {
                                Text(severity)
                                    .font(.caption2.bold())
                                    .foregroundStyle(.secondary)
                            }
                        }
                        HStack(spacing: 8) {
                            if let agent = row.agent, !agent.isEmpty {
                                Text(DisplayText.agent(agent))
                            }
                            if let skillName = row.skillName, !skillName.isEmpty {
                                Text(skillName)
                            }
                        }
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        PrivacyEvidenceText(value: row.detail, font: .caption, lineLimit: nil)
                        if !row.evidenceRefs.isEmpty {
                            PrivacyEvidenceText(value: row.evidenceRefs.joined(separator: ", "), font: .caption2, lineLimit: 2)
                        }
                    }
                }
            }
        }
    }
}

struct StaleDriftSafetyList: View {
    let safety: StaleDriftSafety

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.staleDriftSafetyFlags)
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

struct SimilarSkillGroupingPanel: View {
    let result: SimilarSkillGroupingResult?
    let isGrouping: Bool
    let onGroup: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.similarGroupingTitle, systemImage: "rectangle.3.group.bubble")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.similarGroupingBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            HStack(spacing: 8) {
                Button {
                    onGroup()
                } label: {
                    Label(UIStrings.similarGroupingAction, systemImage: "rectangle.stack.badge.person.crop")
                }
                .disabled(isGrouping)

                if isGrouping {
                    Label(UIStrings.llmPreparing, systemImage: "hourglass")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
            }

            if let result {
                SimilarSkillGroupingResultView(result: result)
            } else {
                Label(UIStrings.similarGroupingNoResult, systemImage: "info.circle")
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

struct SimilarSkillGroupingResultView: View {
    let result: SimilarSkillGroupingResult

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            DetailMetricGrid {
                SummaryChip(title: UIStrings.similarGroupingGroups, value: "\(groupCount)", systemImage: "rectangle.stack")
                SummaryChip(title: UIStrings.similarGroupingMembers, value: "\(memberCount)", systemImage: "person.3")
                SummaryChip(title: UIStrings.similarGroupingDuplicate, value: "\(duplicateCount)", systemImage: "doc.on.doc")
                SummaryChip(title: UIStrings.similarGroupingConfusable, value: "\(confusableCount)", systemImage: "point.3.connected.trianglepath.dotted")
                SummaryChip(title: UIStrings.similarGroupingHighAmbiguity, value: "\(result.summary.highAmbiguityCount)", systemImage: "exclamationmark.triangle")
                SummaryChip(title: UIStrings.similarGroupingRoutingAmbiguity, value: "\(result.summary.routingAmbiguityCount)", systemImage: "arrow.triangle.branch")
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.routingAccuracyGeneratedBy, value: result.generatedBy)
                MetadataRow(label: UIStrings.routingAccuracyCatalog, value: result.catalogAvailable ? UIStrings.routingAccuracyAvailable : UIStrings.routingAccuracyUnavailableShort)
                MetadataRow(label: UIStrings.agent, value: result.filters.agents.isEmpty ? (result.filters.agent.map(DisplayText.agent) ?? UIStrings.text("health.allAgents", "All Agents")) : result.filters.agents.map(DisplayText.agent).joined(separator: ", "))
                if let limit = result.filters.limit {
                    MetadataRow(label: UIStrings.text("filter.limit", "Limit"), value: "\(limit)")
                }
                if let minScore = result.filters.minScore {
                    MetadataRow(label: UIStrings.similarGroupingSimilar, value: RoutingAccuracySummary.confidenceLabel(minScore))
                }
                MetadataRow(label: UIStrings.text("filter.singletons", "Singletons"), value: result.filters.includeSingletons ? UIStrings.stateEnabled : UIStrings.stateDisabled)
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

            SimilarSkillGroupList(groups: result.groups)
            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: result.gapNotes, systemImage: "puzzlepiece.extension")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: result.blockerNotes, systemImage: "exclamationmark.octagon")
            CrossAgentReadinessEvidenceList(evidence: result.evidenceReferences)
            StaleDriftSafetyList(safety: result.safetyFlags)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 6))
    }

    private var groupCount: Int {
        result.summary.groupCount > 0 ? result.summary.groupCount : result.groups.count
    }

    private var memberCount: Int {
        result.summary.memberCount > 0 ? result.summary.memberCount : result.groups.reduce(0) { $0 + $1.members.count }
    }

    private var duplicateCount: Int {
        result.summary.duplicateCount > 0 ? result.summary.duplicateCount : result.groups.filter { $0.typeLabel == UIStrings.similarGroupingDuplicate }.count
    }

    private var confusableCount: Int {
        result.summary.confusableCount > 0 ? result.summary.confusableCount : result.groups.filter { $0.typeLabel == UIStrings.similarGroupingConfusable }.count
    }

    private func promptRequestLabel(_ promptRequest: RoutingAccuracyPromptRequest) -> String {
        let state = promptRequest.enabled ? UIStrings.llmEnabled : UIStrings.llmDisabled
        let copy = promptRequest.copyOnly ? UIStrings.llmPromptCopyOnly : UIStrings.llmSkillAnalysisEnabledUnsafe
        return "\(promptRequest.requestKind) · \(state) · \(copy)"
    }
}

struct SimilarSkillGroupList: View {
    let groups: [SimilarSkillGroup]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.similarGroupingGroups)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if groups.isEmpty {
                Text(UIStrings.similarGroupingNoGroups)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 360), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(groups.prefix(8)) { group in
                        SimilarSkillGroupCard(group: group)
                    }
                }
            }
        }
    }
}

struct SimilarSkillGroupCard: View {
    let group: SimilarSkillGroup

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Label(group.title, systemImage: iconName)
                    .font(.callout.bold())
                    .lineLimit(1)
                Spacer()
                Text(group.displayRank)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
            }

            HStack(spacing: 6) {
                SimilarSkillPill(text: group.typeLabel, systemImage: iconName)
                if let score = group.similarityScore {
                    SimilarSkillPill(text: RoutingAccuracySummary.confidenceLabel(score), systemImage: "gauge.medium")
                }
                if let ambiguityRisk = group.ambiguityRisk, !ambiguityRisk.isEmpty {
                    SimilarSkillPill(text: ambiguityRisk, systemImage: "exclamationmark.triangle")
                }
            }

            if !group.summary.isEmpty {
                PrivacyEvidenceText(value: group.summary, font: .caption, lineLimit: nil)
            }

            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                if let coverage = group.coverageRedundancy, !coverage.isEmpty {
                    MetadataRow(label: UIStrings.similarGroupingCoverageRedundancy, value: coverage)
                }
                if let ambiguity = group.routingAmbiguity, !ambiguity.isEmpty {
                    MetadataRow(label: UIStrings.similarGroupingRoutingAmbiguity, value: ambiguity)
                }
            }

            RoutingInlineList(title: UIStrings.similarGroupingWhyGrouped, empty: UIStrings.routingConfidenceNoReasons, values: group.whyGrouped, systemImage: "text.bubble")
            KnowledgeTokenFlow(title: UIStrings.similarGroupingSharedTerms, values: group.sharedTerms)
            KnowledgeTokenFlow(title: UIStrings.knowledgeTools, values: group.sharedTools)
            KnowledgeTokenFlow(title: UIStrings.knowledgeRules, values: group.sharedRules)
            KnowledgeTokenFlow(title: UIStrings.knowledgeCapabilities, values: group.sharedCapabilities)
            KnowledgeTokenFlow(title: UIStrings.knowledgeRisks, values: group.sharedRisks)
            KnowledgeTokenFlow(title: UIStrings.similarGroupingSourceSignals, values: group.sourceSignals)
            SimilarSkillMemberList(members: group.members)
            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: group.evidenceRefs, systemImage: "checklist")
            RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: group.safetyFlags, systemImage: "checkmark.shield")
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
    }

    private var iconName: String {
        switch group.typeLabel {
        case UIStrings.similarGroupingDuplicate:
            return "doc.on.doc"
        case UIStrings.similarGroupingConfusable:
            return "point.3.connected.trianglepath.dotted"
        default:
            return "rectangle.3.group.bubble"
        }
    }
}

struct SimilarSkillMemberList: View {
    let members: [SimilarSkillMember]

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(UIStrings.similarGroupingMembers)
                .font(.caption2.bold())
                .foregroundStyle(.secondary)
            if members.isEmpty {
                Text(UIStrings.knowledgeNoRows)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(members.prefix(6)) { member in
                    VStack(alignment: .leading, spacing: 6) {
                        HStack(alignment: .firstTextBaseline) {
                            Label(member.skillName, systemImage: "doc.text")
                                .font(.caption.bold())
                                .lineLimit(1)
                            Spacer()
                            Text(member.agent.map(DisplayText.agent) ?? UIStrings.unknown)
                                .font(.caption2.bold())
                                .foregroundStyle(.secondary)
                        }

                        Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 3) {
                            MetadataRow(label: UIStrings.scope, value: member.scope ?? UIStrings.unknown)
                            MetadataRow(label: UIStrings.state, value: member.statusLabel)
                            if let definitionID = member.definitionID, !definitionID.isEmpty {
                                MetadataRow(label: UIStrings.definition, value: definitionID)
                            }
                            if let quality = qualityLabel(member), !quality.isEmpty {
                                MetadataRow(label: UIStrings.similarGroupingQuality, value: quality)
                            }
                            if let readiness = readinessLabel(member), !readiness.isEmpty {
                                MetadataRow(label: UIStrings.similarGroupingReadiness, value: readiness)
                            }
                            if let staleDrift = member.staleDriftState, !staleDrift.isEmpty {
                                MetadataRow(label: UIStrings.similarGroupingStaleDrift, value: staleDrift)
                            }
                            if let sourceKind = member.sourceKind, !sourceKind.isEmpty {
                                MetadataRow(label: UIStrings.provenanceKind, value: sourceKind)
                            }
                            if let sourceRoot = member.sourceRoot, !sourceRoot.isEmpty {
                                MetadataRow(label: UIStrings.provenanceRoot, value: sourceRoot)
                            }
                        }

                        if let sourcePath = member.sourcePath, !sourcePath.isEmpty {
                            PrivacyPathText(path: sourcePath, font: .caption2, lineLimit: 1)
                        }

                        RoutingInlineList(title: UIStrings.routingConfidenceMatchReasons, empty: UIStrings.routingConfidenceNoReasons, values: member.reasons, systemImage: "text.bubble")
                        RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: member.evidenceRefs, systemImage: "checklist")
                        RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: member.safetyFlags, systemImage: "checkmark.shield")
                    }
                    .padding(8)
                    .background(Color.white, in: RoundedRectangle(cornerRadius: 6))
                }
            }
        }
    }

    private func qualityLabel(_ member: SimilarSkillMember) -> String? {
        scoreBandLabel(score: member.qualityScore, band: member.qualityBand)
    }

    private func readinessLabel(_ member: SimilarSkillMember) -> String? {
        scoreBandLabel(score: member.readinessScore, band: member.readinessBand)
    }

    private func scoreBandLabel(score: Double?, band: String?) -> String? {
        let pieces = [
            score.map(RoutingAccuracySummary.confidenceLabel),
            band
        ].compactMap { value -> String? in
            guard let value, !value.isEmpty else { return nil }
            return value
        }
        return pieces.isEmpty ? nil : pieces.joined(separator: " · ")
    }
}

struct SimilarSkillPill: View {
    let text: String
    let systemImage: String

    var body: some View {
        Label(text, systemImage: systemImage)
            .font(.caption2.bold())
            .padding(.horizontal, 7)
            .padding(.vertical, 3)
            .background(Color.white, in: Capsule())
            .lineLimit(1)
    }
}

struct CapabilityTaxonomyPanel: View {
    let result: CapabilityTaxonomyResult?
    let isBuilding: Bool
    let onBuild: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.capabilityTaxonomyTitle, systemImage: "square.grid.3x3.topleft.filled")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.capabilityTaxonomyBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            HStack(spacing: 8) {
                Button {
                    onBuild()
                } label: {
                    Label(UIStrings.capabilityTaxonomyAction, systemImage: "point.3.connected.trianglepath.dotted")
                }
                .disabled(isBuilding)

                if isBuilding {
                    Label(UIStrings.llmPreparing, systemImage: "hourglass")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
            }

            if let result {
                CapabilityTaxonomyResultView(result: result)
            } else {
                Label(UIStrings.capabilityTaxonomyNoResult, systemImage: "info.circle")
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

struct CapabilityTaxonomyResultView: View {
    let result: CapabilityTaxonomyResult

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            DetailMetricGrid {
                SummaryChip(title: UIStrings.capabilityTaxonomyDomains, value: "\(domainCount)", systemImage: "square.grid.3x3")
                SummaryChip(title: UIStrings.knowledgeCapabilities, value: "\(capabilityCount)", systemImage: "tag")
                SummaryChip(title: UIStrings.similarGroupingMembers, value: "\(skillCount)", systemImage: "doc.text")
                SummaryChip(title: UIStrings.agent, value: "\(agentCount)", systemImage: "person.2")
                SummaryChip(title: UIStrings.knowledgeGapNotes, value: "\(gapCount)", systemImage: "puzzlepiece.extension")
                SummaryChip(title: UIStrings.knowledgeBlockerNotes, value: "\(blockerCount)", systemImage: "exclamationmark.octagon")
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.routingAccuracyGeneratedBy, value: result.generatedBy)
                MetadataRow(label: UIStrings.routingAccuracyCatalog, value: result.catalogAvailable ? UIStrings.routingAccuracyAvailable : UIStrings.routingAccuracyUnavailableShort)
                MetadataRow(label: UIStrings.agent, value: result.filters.agents.isEmpty ? (result.filters.agent.map(DisplayText.agent) ?? UIStrings.text("health.allAgents", "All Agents")) : result.filters.agents.map(DisplayText.agent).joined(separator: ", "))
                if let limit = result.filters.limit {
                    MetadataRow(label: UIStrings.text("filter.limit", "Limit"), value: "\(limit)")
                }
                MetadataRow(label: UIStrings.knowledgeGapNotes, value: result.filters.includeGaps ? UIStrings.stateEnabled : UIStrings.stateDisabled)
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

            CapabilityCoverageList(coverage: result.coverageByAgent)
            CapabilityDomainList(domains: result.domains)
            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: result.gapNotes, systemImage: "puzzlepiece.extension")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: result.blockerNotes, systemImage: "exclamationmark.octagon")
            CrossAgentReadinessEvidenceList(evidence: result.evidenceReferences)
            StaleDriftSafetyList(safety: result.safetyFlags)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 6))
    }

    private var domainCount: Int {
        result.summary.domainCount > 0 ? result.summary.domainCount : result.domains.count
    }

    private var capabilityCount: Int {
        result.summary.capabilityCount > 0 ? result.summary.capabilityCount : result.domains.reduce(0) { $0 + $1.capabilityCount }
    }

    private var skillCount: Int {
        result.summary.skillCount > 0 ? result.summary.skillCount : result.domains.reduce(0) { $0 + $1.skillCount }
    }

    private var agentCount: Int {
        result.summary.agentCount > 0 ? result.summary.agentCount : Set(result.coverageByAgent.map(\.agent)).count
    }

    private var gapCount: Int {
        result.summary.gapCount > 0 ? result.summary.gapCount : result.gapNotes.count + result.domains.reduce(0) { $0 + $1.gapNotes.count }
    }

    private var blockerCount: Int {
        result.summary.blockerCount > 0 ? result.summary.blockerCount : result.blockerNotes.count + result.domains.reduce(0) { $0 + $1.blockerNotes.count }
    }

    private func promptRequestLabel(_ promptRequest: RoutingAccuracyPromptRequest) -> String {
        let state = promptRequest.enabled ? UIStrings.llmEnabled : UIStrings.llmDisabled
        let copy = promptRequest.copyOnly ? UIStrings.llmPromptCopyOnly : UIStrings.llmSkillAnalysisEnabledUnsafe
        return "\(promptRequest.requestKind) · \(state) · \(copy)"
    }
}

struct CapabilityCoverageList: View {
    let coverage: [CapabilityTaxonomyCoverage]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.capabilityTaxonomyAgentCoverage)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if coverage.isEmpty {
                Text(UIStrings.routingAccuracyNoEvidence)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 210), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(coverage.prefix(8)) { row in
                        VStack(alignment: .leading, spacing: 6) {
                            HStack(alignment: .firstTextBaseline) {
                                Text(DisplayText.agent(row.agent))
                                    .font(.caption.bold())
                                Spacer()
                                Text(row.coverageState)
                                    .font(.caption2.bold())
                                    .foregroundStyle(.secondary)
                            }
                            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 3) {
                                MetadataRow(label: UIStrings.knowledgeCapabilities, value: "\(row.capabilityCount)")
                                MetadataRow(label: UIStrings.similarGroupingMembers, value: "\(row.skillCount)")
                            }
                            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: row.notes, systemImage: "puzzlepiece.extension")
                        }
                        .padding(8)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 6))
                    }
                }
            }
        }
    }
}

struct CapabilityDomainList: View {
    let domains: [CapabilityTaxonomyDomain]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.capabilityTaxonomyDomains)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if domains.isEmpty {
                Text(UIStrings.capabilityTaxonomyNoDomains)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 360), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(domains.prefix(8)) { domain in
                        CapabilityDomainCard(domain: domain)
                    }
                }
            }
        }
    }
}

struct CapabilityDomainCard: View {
    let domain: CapabilityTaxonomyDomain

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline) {
                Label(domain.name, systemImage: "square.grid.3x3.topleft.filled")
                    .font(.callout.bold())
                    .lineLimit(1)
                Spacer()
                Text("\(domain.capabilityCount) / \(domain.skillCount)")
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
            }

            if !domain.summary.isEmpty {
                Text(domain.summary)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            CapabilityCoverageList(coverage: domain.coverageByAgent)
            CapabilityList(capabilities: domain.capabilities)
            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: domain.gapNotes, systemImage: "puzzlepiece.extension")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: domain.blockerNotes, systemImage: "exclamationmark.octagon")
            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: domain.evidenceRefs, systemImage: "checklist")
            RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: domain.safetyFlags, systemImage: "checkmark.shield")
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
    }
}

struct CapabilityList: View {
    let capabilities: [CapabilityTaxonomyCapability]

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(UIStrings.knowledgeCapabilities)
                .font(.caption2.bold())
                .foregroundStyle(.secondary)
            if capabilities.isEmpty {
                Text(UIStrings.knowledgeNoRows)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(capabilities.prefix(6)) { capability in
                    VStack(alignment: .leading, spacing: 6) {
                        Label(capability.name, systemImage: "tag")
                            .font(.caption.bold())
                            .lineLimit(1)
                        if !capability.summary.isEmpty {
                            Text(capability.summary)
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                                .textSelection(.enabled)
                        }
                        KnowledgeTokenFlow(title: UIStrings.knowledgeKeywords, values: capability.keywords)
                        KnowledgeTokenFlow(title: UIStrings.knowledgeTools, values: capability.tools)
                        KnowledgeTokenFlow(title: UIStrings.knowledgeRules, values: capability.rules)
                        KnowledgeTokenFlow(title: UIStrings.knowledgeRisks, values: capability.riskTags)
                        CapabilitySkillList(skills: capability.representativeSkills)
                        RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: capability.evidenceRefs, systemImage: "checklist")
                        RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: capability.safetyFlags, systemImage: "checkmark.shield")
                    }
                    .padding(8)
                    .background(Color.white, in: RoundedRectangle(cornerRadius: 6))
                }
            }
        }
    }
}

struct CapabilitySkillList: View {
    let skills: [CapabilityTaxonomySkill]

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            Text(UIStrings.capabilityTaxonomyRepresentativeSkills)
                .font(.caption2.bold())
                .foregroundStyle(.secondary)
            if skills.isEmpty {
                Text(UIStrings.knowledgeNoRows)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(skills.prefix(5)) { skill in
                    VStack(alignment: .leading, spacing: 4) {
                        HStack(alignment: .firstTextBaseline) {
                            Label(skill.skillName, systemImage: "doc.text")
                                .font(.caption2.bold())
                                .lineLimit(1)
                            Spacer()
                            Text(skill.agent.map(DisplayText.agent) ?? UIStrings.unknown)
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                        }
                        Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 3) {
                            MetadataRow(label: UIStrings.scope, value: skill.scope ?? UIStrings.unknown)
                            MetadataRow(label: UIStrings.state, value: skill.statusLabel)
                            if let qualityScore = skill.qualityScore {
                                MetadataRow(label: UIStrings.similarGroupingQuality, value: RoutingAccuracySummary.confidenceLabel(qualityScore))
                            }
                            if let readinessScore = skill.readinessScore {
                                MetadataRow(label: UIStrings.similarGroupingReadiness, value: RoutingAccuracySummary.confidenceLabel(readinessScore))
                            }
                        }
                        RoutingInlineList(title: UIStrings.routingConfidenceMatchReasons, empty: UIStrings.routingConfidenceNoReasons, values: skill.reasons, systemImage: "text.bubble")
                        RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: skill.evidenceRefs, systemImage: "checklist")
                        RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: skill.safetyFlags, systemImage: "checkmark.shield")
                    }
                    .padding(6)
                    .background(Color.white, in: RoundedRectangle(cornerRadius: 6))
                }
            }
        }
    }
}

struct WorkspaceReadinessPanel: View {
    let result: WorkspaceReadinessResult?
    let isChecking: Bool
    let onCheck: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.workspaceReadinessTitle, systemImage: "checklist.checked")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.workspaceReadinessBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            HStack(spacing: 8) {
                Button {
                    onCheck()
                } label: {
                    Label(UIStrings.workspaceReadinessAction, systemImage: "checkmark.seal")
                }
                .disabled(isChecking)

                if isChecking {
                    Label(UIStrings.llmPreparing, systemImage: "hourglass")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
            }

            if let result {
                WorkspaceReadinessResultView(result: result)
            } else {
                Label(UIStrings.workspaceReadinessNoResult, systemImage: "info.circle")
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

struct WorkspaceReadinessResultView: View {
    let result: WorkspaceReadinessResult

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            DetailMetricGrid {
                SummaryChip(title: UIStrings.workspaceReadinessOverall, value: result.summary.overallState, systemImage: "gauge.medium")
                SummaryChip(title: UIStrings.similarGroupingReadiness, value: scoreLabel, systemImage: "speedometer")
                SummaryChip(title: UIStrings.workspaceReadinessChecklist, value: "\(checklistCount)", systemImage: "checklist")
                SummaryChip(title: UIStrings.workspaceReadinessReady, value: "\(readyCount)", systemImage: "checkmark.circle")
                SummaryChip(title: UIStrings.workspaceReadinessPartial, value: "\(partialCount)", systemImage: "circle.lefthalf.filled")
                SummaryChip(title: UIStrings.workspaceReadinessBlocked, value: "\(blockedCount)", systemImage: "exclamationmark.octagon")
                SummaryChip(title: UIStrings.agent, value: "\(agentCount)", systemImage: "person.2")
                SummaryChip(title: UIStrings.knowledgeCapabilities, value: "\(capabilityCount)", systemImage: "tag")
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.routingAccuracyGeneratedBy, value: result.generatedBy)
                MetadataRow(label: UIStrings.routingAccuracyCatalog, value: result.catalogAvailable ? UIStrings.routingAccuracyAvailable : UIStrings.routingAccuracyUnavailableShort)
                MetadataRow(label: UIStrings.agent, value: agentFilterLabel)
                if let projectRoot = result.filters.projectRoot, !projectRoot.isEmpty {
                    PrivacyPathRow(label: UIStrings.project, path: projectRoot)
                }
                if let workspace = result.filters.workspace, !workspace.isEmpty {
                    MetadataRow(label: UIStrings.workspaceReadinessTitle, value: workspace)
                }
                if let taskText = result.filters.taskText, !taskText.isEmpty {
                    MetadataRow(label: UIStrings.taskBenchmarkTaskPlaceholder, value: taskText)
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

            WorkspaceReadinessChecklistList(rows: result.checklistRows)
            WorkspaceReadinessAgentList(rows: result.agentRows)
            WorkspaceReadinessCapabilityList(rows: result.capabilityRows)
            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: result.gapNotes, systemImage: "puzzlepiece.extension")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: result.blockerNotes, systemImage: "exclamationmark.octagon")
            CrossAgentReadinessEvidenceList(evidence: result.evidenceReferences)
            StaleDriftSafetyList(safety: result.safetyFlags)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 6))
    }

    private var scoreLabel: String {
        guard let score = result.summary.readinessScore else { return UIStrings.unknown }
        return "\(score)"
    }

    private var checklistCount: Int {
        result.summary.checklistCount > 0 ? result.summary.checklistCount : result.checklistRows.count
    }

    private var readyCount: Int {
        result.summary.readyCount > 0 ? result.summary.readyCount : result.checklistRows.filter { $0.status.localizedCaseInsensitiveContains("ready") }.count
    }

    private var partialCount: Int {
        result.summary.partialCount > 0 ? result.summary.partialCount : result.checklistRows.filter { $0.status.localizedCaseInsensitiveContains("partial") }.count
    }

    private var blockedCount: Int {
        result.summary.blockedCount > 0 ? result.summary.blockedCount : result.checklistRows.filter { $0.status.localizedCaseInsensitiveContains("block") }.count
    }

    private var agentCount: Int {
        result.summary.agentCount > 0 ? result.summary.agentCount : result.agentRows.count
    }

    private var capabilityCount: Int {
        result.summary.capabilityCount > 0 ? result.summary.capabilityCount : result.capabilityRows.count
    }

    private var agentFilterLabel: String {
        if !result.filters.agents.isEmpty {
            return result.filters.agents.map(DisplayText.agent).joined(separator: ", ")
        }
        return result.filters.agent.map(DisplayText.agent) ?? UIStrings.text("health.allAgents", "All Agents")
    }

    private func promptRequestLabel(_ promptRequest: RoutingAccuracyPromptRequest) -> String {
        let state = promptRequest.enabled ? UIStrings.llmEnabled : UIStrings.llmDisabled
        let copy = promptRequest.copyOnly ? UIStrings.llmPromptCopyOnly : UIStrings.llmSkillAnalysisEnabledUnsafe
        return "\(promptRequest.requestKind) · \(state) · \(copy)"
    }
}

struct WorkspaceReadinessChecklistList: View {
    let rows: [WorkspaceReadinessChecklistRow]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.workspaceReadinessChecklist)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if rows.isEmpty {
                Text(UIStrings.workspaceReadinessNoChecklist)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 320), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(rows.prefix(8)) { row in
                        VStack(alignment: .leading, spacing: 8) {
                            HStack(alignment: .firstTextBaseline) {
                                Label(row.title, systemImage: "checklist")
                                    .font(.callout.bold())
                                    .lineLimit(1)
                                Spacer()
                                Text(row.status)
                                    .font(.caption2.bold())
                                    .foregroundStyle(.secondary)
                            }
                            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 3) {
                                if let agent = row.agent, !agent.isEmpty {
                                    MetadataRow(label: UIStrings.agent, value: DisplayText.agent(agent))
                                }
                                if let capability = row.capability, !capability.isEmpty {
                                    MetadataRow(label: UIStrings.capabilityTaxonomyCapability, value: capability)
                                }
                                if let severity = row.severity, !severity.isEmpty {
                                    MetadataRow(label: UIStrings.cleanupFilterPriority, value: severity)
                                }
                            }
                            if !row.summary.isEmpty {
                                PrivacyEvidenceText(value: row.summary, font: .caption, lineLimit: nil)
                            }
                            RoutingInlineList(title: UIStrings.workspaceReadinessRequired, empty: UIStrings.knowledgeNoRows, values: row.requiredSkills, systemImage: "target")
                            CapabilitySkillList(skills: row.matchedSkills)
                            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: row.gapNotes, systemImage: "puzzlepiece.extension")
                            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: row.blockerNotes, systemImage: "exclamationmark.octagon")
                            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: row.evidenceRefs, systemImage: "checklist")
                            RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: row.safetyFlags, systemImage: "checkmark.shield")
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }
        }
    }
}

struct WorkspaceReadinessAgentList: View {
    let rows: [WorkspaceReadinessAgentRow]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.workspaceReadinessAgentRows)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if rows.isEmpty {
                Text(UIStrings.workspaceReadinessNoAgentRows)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 240), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(rows.prefix(8)) { row in
                        VStack(alignment: .leading, spacing: 6) {
                            HStack(alignment: .firstTextBaseline) {
                                Text(row.displayName ?? DisplayText.agent(row.agent))
                                    .font(.caption.bold())
                                Spacer()
                                Text(agentScoreLabel(row))
                                    .font(.caption2.bold())
                                    .foregroundStyle(.secondary)
                            }
                            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 3) {
                                MetadataRow(label: UIStrings.workspaceReadinessEnabled, value: "\(row.enabledSkillCount)")
                                MetadataRow(label: UIStrings.workspaceReadinessRequired, value: "\(row.requiredSkillCount)")
                                MetadataRow(label: UIStrings.workspaceReadinessMatched, value: "\(row.matchedSkillCount)")
                                MetadataRow(label: UIStrings.knowledgeGapNotes, value: "\(row.gapCount)")
                                MetadataRow(label: UIStrings.knowledgeBlockerNotes, value: "\(row.blockerCount)")
                            }
                            RoutingInlineList(title: UIStrings.staleDriftReasons, empty: UIStrings.staleDriftNoReasons, values: row.notes, systemImage: "text.bubble")
                            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: row.evidenceRefs, systemImage: "checklist")
                        }
                        .padding(8)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 6))
                    }
                }
            }
        }
    }

    private func agentScoreLabel(_ row: WorkspaceReadinessAgentRow) -> String {
        if let readinessScore = row.readinessScore {
            return "\(readinessScore) · \(row.readinessState)"
        }
        return row.readinessState
    }
}

struct WorkspaceReadinessCapabilityList: View {
    let rows: [WorkspaceReadinessCapabilityRow]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.workspaceReadinessCapabilityRows)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if rows.isEmpty {
                Text(UIStrings.workspaceReadinessNoCapabilityRows)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 340), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(rows.prefix(8)) { row in
                        VStack(alignment: .leading, spacing: 8) {
                            HStack(alignment: .firstTextBaseline) {
                                Label(row.capability, systemImage: "tag")
                                    .font(.callout.bold())
                                    .lineLimit(1)
                                Spacer()
                                Text(capabilityScoreLabel(row))
                                    .font(.caption2.bold())
                                    .foregroundStyle(.secondary)
                            }
                            Text(row.domain)
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                            CapabilityCoverageList(coverage: row.agentCoverage)
                            CapabilitySkillList(skills: row.representativeSkills)
                            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: row.gapNotes, systemImage: "puzzlepiece.extension")
                            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: row.blockerNotes, systemImage: "exclamationmark.octagon")
                            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: row.evidenceRefs, systemImage: "checklist")
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }
        }
    }

    private func capabilityScoreLabel(_ row: WorkspaceReadinessCapabilityRow) -> String {
        if let readinessScore = row.readinessScore {
            return "\(readinessScore) · \(row.readinessState)"
        }
        return row.readinessState
    }
}
