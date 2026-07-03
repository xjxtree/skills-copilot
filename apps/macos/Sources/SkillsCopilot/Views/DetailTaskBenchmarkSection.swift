import AppKit
import SwiftUI

struct TaskBenchmarkPanel: View {
    let skill: SkillRecord
    @Binding var taskText: String
    let currentTaskText: String
    let listResult: TaskBenchmarkListResult
    let evaluation: TaskBenchmarkEvaluationResult?
    let deleteResult: TaskBenchmarkDeleteResult?
    let routingRegressionBaseline: RoutingRegressionBaselineResult?
    let routingRegressionDetection: RoutingRegressionDetectionResult?
    let isLoading: Bool
    let isSaving: Bool
    let isEvaluating: Bool
    let isSavingRoutingBaseline: Bool
    let isDetectingRoutingRegression: Bool
    let isDeleting: (TaskBenchmarkRecord) -> Bool
    let onLoad: () -> Void
    let onSave: () -> Void
    let onEvaluate: () -> Void
    let onSaveRoutingBaseline: () -> Void
    let onDetectRoutingRegression: () -> Void
    let onDelete: (TaskBenchmarkRecord) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.taskBenchmarkTitle, systemImage: "checklist.unchecked")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.taskBenchmarkBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            VStack(alignment: .leading, spacing: 8) {
                TextField(UIStrings.taskBenchmarkTaskPlaceholder, text: $taskText, axis: .vertical)
                    .textFieldStyle(.roundedBorder)
                    .lineLimit(2...4)
                    .labelsHidden()
                HStack(spacing: 8) {
                    Button {
                        onSave()
                    } label: {
                        Label(UIStrings.taskBenchmarkSaveAction, systemImage: "plus.square.on.square")
                    }
                    .disabled(isSaving || currentTaskText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    .help(UIStrings.taskBenchmarkExpectedCurrentSkill(skill.name, DisplayText.agent(skill.agent)))

                    Button {
                        onLoad()
                    } label: {
                        Label(UIStrings.taskBenchmarkLoadAction, systemImage: "arrow.clockwise")
                    }
                    .disabled(isLoading)

                    Button {
                        onEvaluate()
                    } label: {
                        Label(UIStrings.taskBenchmarkEvaluateAction, systemImage: "chart.bar.xaxis")
                    }
                    .disabled(isEvaluating)
                    Spacer()
                }
            }

            Label(UIStrings.taskBenchmarkExpectedCurrentSkill(skill.name, DisplayText.agent(skill.agent)), systemImage: "target")
                .font(.callout)
                .foregroundStyle(.secondary)

            if isLoading || isSaving || isEvaluating {
                Label(UIStrings.llmPreparing, systemImage: "hourglass")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            TaskBenchmarkListView(
                result: listResult,
                deleteResult: deleteResult,
                isDeleting: isDeleting,
                onDelete: onDelete
            )

            if let evaluation {
                TaskBenchmarkEvaluationView(result: evaluation)
            }

            RoutingRegressionPanel(
                baseline: routingRegressionBaseline,
                detection: routingRegressionDetection,
                isSavingBaseline: isSavingRoutingBaseline,
                isDetecting: isDetectingRoutingRegression,
                onSaveBaseline: onSaveRoutingBaseline,
                onDetect: onDetectRoutingRegression
            )

            Label(UIStrings.llmReviewNoActions, systemImage: "nosign")
                .font(.callout)
                .foregroundStyle(.secondary)
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

struct RoutingRegressionPanel: View {
    let baseline: RoutingRegressionBaselineResult?
    let detection: RoutingRegressionDetectionResult?
    let isSavingBaseline: Bool
    let isDetecting: Bool
    let onSaveBaseline: () -> Void
    let onDetect: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.routingRegressionTitle, systemImage: "chart.line.downtrend.xyaxis")
                    .font(.subheadline.bold())
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.routingRegressionBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            HStack(spacing: 8) {
                Button {
                    onSaveBaseline()
                } label: {
                    Label(UIStrings.routingRegressionSaveBaselineAction, systemImage: "tray.and.arrow.down")
                }
                .disabled(isSavingBaseline)

                Button {
                    onDetect()
                } label: {
                    Label(UIStrings.routingRegressionDetectAction, systemImage: "waveform.path.ecg")
                }
                .disabled(isDetecting)
                Spacer()
            }

            if isSavingBaseline || isDetecting {
                Label(UIStrings.llmPreparing, systemImage: "hourglass")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            if let baseline {
                RoutingRegressionBaselineView(result: baseline)
            } else {
                Label(UIStrings.routingRegressionNoBaseline, systemImage: "clock.badge.questionmark")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            if let detection {
                RoutingRegressionDetectionView(result: detection)
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 6))
    }
}

struct RoutingRegressionBaselineView: View {
    let result: RoutingRegressionBaselineResult

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Text(UIStrings.routingRegressionBaselineStatus)
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                Spacer()
                if let baselineID = result.baselineID, !baselineID.isEmpty {
                    Text(baselineID)
                        .font(.caption.monospaced())
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .textSelection(.enabled)
                }
            }

            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.taskBenchmarkEvaluated, value: "\(result.benchmarkCount)")
                if let averageScore = result.averageScore {
                    MetadataRow(label: UIStrings.taskBenchmarkAverageScore, value: "\(averageScore)")
                }
                if let matchedCount = result.matchedCount {
                    MetadataRow(label: UIStrings.taskBenchmarkMatched, value: "\(matchedCount)")
                }
                if let acceptableCount = result.acceptableCount {
                    MetadataRow(label: UIStrings.taskBenchmarkAcceptableMatched, value: "\(acceptableCount)")
                }
                MetadataRow(label: UIStrings.skillQualityProviderNotSent, value: result.safety.providerRequestSent ? UIStrings.llmSkillAnalysisEnabledUnsafe : UIStrings.llmDisabled)
                MetadataRow(label: UIStrings.skillQualityWritesBlocked, value: readOnlyValue(!result.safety.writeBackAllowed && !result.safety.writeActionsAvailable))
                MetadataRow(label: UIStrings.skillQualityScriptsBlocked, value: readOnlyValue(!result.safety.scriptExecutionAllowed && !result.safety.executionActionsAvailable))
                MetadataRow(label: UIStrings.skillQualityMutationsBlocked, value: readOnlyValue(!result.safety.configMutationAllowed && !result.safety.snapshotCreated && !result.safety.triageMutationAllowed))
                MetadataRow(label: UIStrings.skillQualityCredentialsBlocked, value: readOnlyValue(!result.safety.credentialAccessed && !result.safety.rawSecretReturned))
            }

            if !result.summary.isEmpty {
                Text(result.summary)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }
        }
    }

    private func readOnlyValue(_ isBlocked: Bool) -> String {
        isBlocked ? UIStrings.llmSkillAnalysisBlocked : UIStrings.llmSkillAnalysisEnabledUnsafe
    }
}

struct RoutingRegressionDetectionView: View {
    let result: RoutingRegressionDetectionResult

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline, spacing: 14) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("\(result.regressionCount)")
                        .font(.system(size: 30, weight: .semibold, design: .rounded))
                        .monospacedDigit()
                    Text(UIStrings.routingRegressionCount)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                VStack(alignment: .leading, spacing: 4) {
                    Text(UIStrings.routingRegressionDetectionTitle)
                        .font(.caption.bold())
                        .foregroundStyle(.secondary)
                    if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                        Label(fallbackReason, systemImage: "info.circle")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                Spacer()
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.taskBenchmarkEvaluated, value: "\(result.benchmarkCount)")
                MetadataRow(label: UIStrings.routingRegressionImproved, value: "\(result.improvedCount)")
                MetadataRow(label: UIStrings.routingRegressionUnchanged, value: "\(result.unchangedCount)")
                MetadataRow(label: UIStrings.routingRegressionAverageScoreDelta, value: signed(result.averageScoreDelta))
                MetadataRow(label: UIStrings.routingRegressionMatchChanges, value: "\(result.matchStatusChangedCount)")
                MetadataRow(label: UIStrings.routingRegressionTopRouteChanges, value: "\(result.topRouteChangedCount)")
                MetadataRow(label: UIStrings.skillQualityProviderNotSent, value: result.safety.providerRequestSent ? UIStrings.llmSkillAnalysisEnabledUnsafe : UIStrings.llmDisabled)
                MetadataRow(label: UIStrings.skillQualityWritesBlocked, value: readOnlyValue(!result.safety.writeBackAllowed && !result.safety.writeActionsAvailable))
                MetadataRow(label: UIStrings.skillQualityScriptsBlocked, value: readOnlyValue(!result.safety.scriptExecutionAllowed && !result.safety.executionActionsAvailable))
                MetadataRow(label: UIStrings.skillQualityMutationsBlocked, value: readOnlyValue(!result.safety.configMutationAllowed && !result.safety.snapshotCreated && !result.safety.triageMutationAllowed))
                MetadataRow(label: UIStrings.skillQualityCredentialsBlocked, value: readOnlyValue(!result.safety.credentialAccessed && !result.safety.rawSecretReturned))
            }

            RoutingRegressionItemList(items: result.regressions)
            SkillQualityStringList(title: UIStrings.routingRegressionNewBlockers, empty: UIStrings.routingRegressionNoNewBlockers, values: result.newBlockers, systemImage: "exclamationmark.octagon")
            SkillQualityStringList(title: UIStrings.routingRegressionNewGaps, empty: UIStrings.routingRegressionNoNewGaps, values: result.newGaps, systemImage: "puzzlepiece.extension")
            RoutingEvidenceList(evidence: result.evidence)
        }
    }

    private func signed(_ value: Int) -> String {
        value > 0 ? "+\(value)" : "\(value)"
    }

    private func readOnlyValue(_ isBlocked: Bool) -> String {
        isBlocked ? UIStrings.llmSkillAnalysisBlocked : UIStrings.llmSkillAnalysisEnabledUnsafe
    }
}

struct RoutingRegressionItemList: View {
    let items: [RoutingRegressionItem]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.routingRegressionItems)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if items.isEmpty {
                Text(UIStrings.routingRegressionNoItems)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 250), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(items) { item in
                        VStack(alignment: .leading, spacing: 7) {
                            HStack(alignment: .firstTextBaseline) {
                                Text(item.taskText.isEmpty ? UIStrings.unknown : item.taskText)
                                    .font(.caption.bold())
                                    .lineLimit(2)
                                    .textSelection(.enabled)
                                Spacer()
                                Text(signed(item.scoreDelta))
                                    .font(.caption.monospacedDigit().bold())
                                    .foregroundStyle(item.scoreDelta < 0 ? Color.red : Color.green)
                            }
                            HStack(spacing: 6) {
                                Text(item.regressionType)
                                    .font(.caption2.bold())
                                    .padding(.horizontal, 6)
                                    .padding(.vertical, 2)
                                    .background(Color.orange.opacity(0.16), in: Capsule())
                                    .foregroundStyle(.orange)
                                if item.topRouteChanged {
                                    Label(UIStrings.routingRegressionTopRouteChanged, systemImage: "arrow.triangle.2.circlepath")
                                        .font(.caption2)
                                        .foregroundStyle(.secondary)
                                }
                            }
                            if !item.previousMatchStatus.isEmpty || !item.currentMatchStatus.isEmpty {
                                Text("\(UIStrings.routingRegressionMatchStatus): \(item.previousMatchStatus.isEmpty ? UIStrings.unknown : item.previousMatchStatus) -> \(item.currentMatchStatus.isEmpty ? UIStrings.unknown : item.currentMatchStatus)")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                            if let previous = item.previousTopRoute ?? item.currentTopRoute {
                                let previousName = item.previousTopRoute.map { "\($0.name) (\(DisplayText.agent($0.agent)))" } ?? UIStrings.unknown
                                let currentName = item.currentTopRoute.map { "\($0.name) (\(DisplayText.agent($0.agent)))" } ?? "\(previous.name) (\(DisplayText.agent(previous.agent)))"
                                Text("\(UIStrings.routingRegressionTopRouteChange): \(previousName) -> \(currentName)")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                    .lineLimit(2)
                            }
                            RoutingInlineList(title: UIStrings.routingRegressionNewBlockers, empty: UIStrings.routingRegressionNoNewBlockers, values: item.newBlockers, systemImage: "exclamationmark.octagon")
                            RoutingInlineList(title: UIStrings.routingRegressionNewGaps, empty: UIStrings.routingRegressionNoNewGaps, values: item.newGaps, systemImage: "puzzlepiece.extension")
                            RoutingInlineList(title: UIStrings.taskBenchmarkSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: item.safetyFlags, systemImage: "lock.shield")
                            RoutingEvidenceList(evidence: item.evidence)
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }
        }
    }

    private func signed(_ value: Int) -> String {
        value > 0 ? "+\(value)" : "\(value)"
    }
}

struct TaskBenchmarkListView: View {
    let result: TaskBenchmarkListResult
    let deleteResult: TaskBenchmarkDeleteResult?
    let isDeleting: (TaskBenchmarkRecord) -> Bool
    let onDelete: (TaskBenchmarkRecord) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Text(UIStrings.taskBenchmarkListTitle)
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                Spacer()
                Text("\(result.benchmarks.count)")
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(.secondary)
            }

            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            if result.benchmarks.isEmpty {
                Text(UIStrings.taskBenchmarkNoBenchmarks)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 230), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(result.benchmarks) { benchmark in
                        VStack(alignment: .leading, spacing: 7) {
                            HStack(alignment: .firstTextBaseline) {
                                Text(benchmark.taskText.isEmpty ? UIStrings.unknown : benchmark.taskText)
                                    .font(.caption.bold())
                                    .lineLimit(2)
                                    .textSelection(.enabled)
                                Spacer()
                                Button {
                                    onDelete(benchmark)
                                } label: {
                                    Image(systemName: "trash")
                                }
                                .buttonStyle(.borderless)
                                .disabled(isDeleting(benchmark))
                                .help(UIStrings.taskBenchmarkDeleteAction)
                            }
                            if let expected = benchmark.expectedSkill {
                                Text("\(UIStrings.taskBenchmarkExpected): \(expected.name) (\(DisplayText.agent(expected.agent)))")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                    .lineLimit(1)
                                    .truncationMode(.middle)
                            }
                            if !benchmark.acceptableSkills.isEmpty {
                                Text("\(UIStrings.taskBenchmarkAcceptable): \(benchmark.acceptableSkills.map(\.name).joined(separator: ", "))")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                    .lineLimit(2)
                            }
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }

            if let deleteResult, let reason = deleteResult.fallbackReason, !reason.isEmpty {
                Label(reason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
        }
    }
}

struct TaskBenchmarkEvaluationView: View {
    let result: TaskBenchmarkEvaluationResult

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline, spacing: 14) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("\(result.averageScore)")
                        .font(.system(size: 34, weight: .semibold, design: .rounded))
                        .monospacedDigit()
                    Text(UIStrings.taskBenchmarkAverageScore)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                VStack(alignment: .leading, spacing: 4) {
                    Text(UIStrings.taskBenchmarkEvaluationTitle)
                        .font(.subheadline.bold())
                    if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                        Label(fallbackReason, systemImage: "info.circle")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                Spacer()
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.taskBenchmarkEvaluated, value: "\(result.evaluatedCount)")
                MetadataRow(label: UIStrings.taskBenchmarkMatched, value: "\(result.matchedCount)")
                MetadataRow(label: UIStrings.taskBenchmarkAcceptableMatched, value: "\(result.acceptableCount)")
                MetadataRow(label: UIStrings.skillQualityProviderNotSent, value: result.safety.providerRequestSent ? UIStrings.llmSkillAnalysisEnabledUnsafe : UIStrings.llmDisabled)
                MetadataRow(label: UIStrings.skillQualityWritesBlocked, value: readOnlyValue(!result.safety.writeBackAllowed && !result.safety.writeActionsAvailable))
                MetadataRow(label: UIStrings.skillQualityScriptsBlocked, value: readOnlyValue(!result.safety.scriptExecutionAllowed && !result.safety.executionActionsAvailable))
                MetadataRow(label: UIStrings.skillQualityMutationsBlocked, value: readOnlyValue(!result.safety.configMutationAllowed && !result.safety.snapshotCreated && !result.safety.triageMutationAllowed))
                MetadataRow(label: UIStrings.skillQualityCredentialsBlocked, value: readOnlyValue(!result.safety.credentialAccessed && !result.safety.rawSecretReturned))
            }

            TaskBenchmarkEvaluationList(evaluations: result.evaluations)
            SkillQualityStringList(title: UIStrings.taskBenchmarkBlockers, empty: UIStrings.taskBenchmarkNoBlockers, values: result.blockers, systemImage: "exclamationmark.octagon")
            SkillQualityStringList(title: UIStrings.taskBenchmarkGaps, empty: UIStrings.taskBenchmarkNoGaps, values: result.gaps, systemImage: "puzzlepiece.extension")
            RoutingEvidenceList(evidence: result.evidence)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 6))
    }

    private func readOnlyValue(_ isBlocked: Bool) -> String {
        isBlocked ? UIStrings.llmSkillAnalysisBlocked : UIStrings.llmSkillAnalysisEnabledUnsafe
    }
}

struct TaskBenchmarkEvaluationList: View {
    let evaluations: [TaskBenchmarkEvaluationItem]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.taskBenchmarkPerBenchmark)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if evaluations.isEmpty {
                Text(UIStrings.taskBenchmarkNoEvaluations)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 240), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(evaluations) { item in
                        VStack(alignment: .leading, spacing: 7) {
                            HStack(alignment: .firstTextBaseline) {
                                Text(item.taskText.isEmpty ? UIStrings.unknown : item.taskText)
                                    .font(.caption.bold())
                                    .lineLimit(2)
                                    .textSelection(.enabled)
                                Spacer()
                                Text("\(item.score)")
                                    .font(.caption.monospacedDigit().bold())
                            }
                            HStack(spacing: 6) {
                                Text(item.matchStatus)
                                    .font(.caption2.bold())
                                    .padding(.horizontal, 6)
                                    .padding(.vertical, 2)
                                    .background(scoreTint(item.score).opacity(0.16), in: Capsule())
                                    .foregroundStyle(scoreTint(item.score))
                                Text(item.band)
                                    .font(.caption2)
                                    .foregroundStyle(.secondary)
                            }
                            if let topRoute = item.topRoute {
                                Text("\(UIStrings.taskBenchmarkTopRoute): \(topRoute.name) (\(DisplayText.agent(topRoute.agent)))")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                    .lineLimit(1)
                                    .truncationMode(.middle)
                            }
                            HStack(spacing: 8) {
                                Label(item.expectedCovered ? UIStrings.taskBenchmarkExpectedCovered : UIStrings.taskBenchmarkExpectedMissed, systemImage: item.expectedCovered ? "checkmark.circle" : "xmark.circle")
                                Label(item.acceptableCovered ? UIStrings.taskBenchmarkAcceptableCovered : UIStrings.taskBenchmarkAcceptableMissed, systemImage: item.acceptableCovered ? "checkmark.circle" : "xmark.circle")
                            }
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            RoutingInlineList(title: UIStrings.taskBenchmarkBlockers, empty: UIStrings.taskBenchmarkNoBlockers, values: item.blockers, systemImage: "exclamationmark.octagon")
                            RoutingInlineList(title: UIStrings.taskBenchmarkGaps, empty: UIStrings.taskBenchmarkNoGaps, values: item.gaps, systemImage: "puzzlepiece.extension")
                            RoutingInlineList(title: UIStrings.taskBenchmarkSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: item.safetyFlags, systemImage: "lock.shield")
                            RoutingEvidenceList(evidence: item.evidence)
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }
        }
    }

    private func scoreTint(_ score: Int) -> Color {
        switch score {
        case 85...100:
            return .green
        case 65..<85:
            return .blue
        case 40..<65:
            return .orange
        default:
            return .red
        }
    }
}
