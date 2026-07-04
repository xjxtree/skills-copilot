import AppKit
import SwiftUI

struct AnalysisSection: View {
    let skill: SkillRecord
    let llmStatus: LLMStatus
    let qualityScore: (SkillRecord) -> SkillQualityScoreResult?
    let isScoringQuality: (SkillRecord) -> Bool
    let qualityPromptPreview: (SkillRecord) -> LLMPromptPreview?
    let isPreviewingQualityPrompt: (SkillRecord) -> Bool
    let isSendingQualityPrompt: (SkillRecord) -> Bool
    let qualityPromptSendResult: (SkillRecord) -> LLMPromptSendResult?
    let canSendQualityPrompt: (SkillRecord) -> Bool
    @Binding var taskCockpitText: String
    let taskCockpitInput: String
    let taskCockpitResult: TaskCockpitResult?
    let isBuildingTaskCockpit: Bool
    @Binding var taskReadinessText: String
    let taskReadinessResult: (SkillRecord) -> TaskReadinessResult?
    let isCheckingTaskReadiness: (SkillRecord) -> Bool
    let taskReadinessPromptPreview: (SkillRecord) -> LLMPromptPreview?
    let isPreviewingTaskReadinessPrompt: (SkillRecord) -> Bool
    let isSendingTaskReadinessPrompt: (SkillRecord) -> Bool
    let taskReadinessPromptSendResult: (SkillRecord) -> LLMPromptSendResult?
    let canSendTaskReadinessPrompt: (SkillRecord) -> Bool
    @Binding var routingConfidenceText: String
    let routingConfidenceResult: (SkillRecord) -> SkillRoutingConfidenceResult?
    let isRankingRoutingConfidence: (SkillRecord) -> Bool
    let routingConfidencePromptPreview: (SkillRecord) -> LLMPromptPreview?
    let isPreviewingRoutingConfidencePrompt: (SkillRecord) -> Bool
    let isSendingRoutingConfidencePrompt: (SkillRecord) -> Bool
    let routingConfidencePromptSendResult: (SkillRecord) -> LLMPromptSendResult?
    let canSendRoutingConfidencePrompt: (SkillRecord) -> Bool
    @Binding var crossAgentReadinessText: String
    let crossAgentReadinessInput: String
    let crossAgentReadinessResult: CrossAgentReadinessResult?
    let isComparingCrossAgentReadiness: Bool
    let routingAccuracyDashboard: RoutingAccuracyDashboard?
    let isLoadingRoutingAccuracyDashboard: Bool
    let staleDriftDetection: StaleDriftDetectionResult?
    let isDetectingStaleDrift: Bool
    @Binding var knowledgeSearchText: String
    let knowledgeSearchResult: KnowledgeSearchResult?
    let isSearchingKnowledge: Bool
    let localSkillMapResult: LocalSkillMapResult?
    let isBuildingLocalSkillMap: Bool
    let skillLifecycleTimelineResult: SkillLifecycleTimelineResult?
    let isLoadingSkillLifecycleTimeline: Bool
    let providerObservabilityResult: ProviderObservabilityResult?
    let isLoadingProviderObservability: Bool
    let similarSkillGroupingResult: SimilarSkillGroupingResult?
    let isGroupingSimilarSkills: Bool
    let capabilityTaxonomyResult: CapabilityTaxonomyResult?
    let isBuildingCapabilityTaxonomy: Bool
    let workspaceReadinessResult: WorkspaceReadinessResult?
    let isCheckingWorkspaceReadiness: Bool
    let remediationPlanResult: RemediationPlanResult?
    let isPlanningRemediation: Bool
    let remediationPreviewDraftsResult: RemediationPreviewDraftsResult?
    let isPreviewingRemediationDrafts: Bool
    let remediationImpactPreviewResult: RemediationImpactPreviewResult?
    let isPreviewingRemediationImpact: Bool
    let remediationBatchReviewResult: RemediationBatchReviewResult?
    let isReviewingRemediationBatch: Bool
    let remediationHistoryResult: RemediationHistoryResult?
    let remediationHistoryRecordResult: RemediationHistoryRecordResult?
    let isLoadingRemediationHistory: Bool
    let isRecordingRemediationHistory: Bool
    let guidedCleanupFlowResult: GuidedCleanupFlowResult?
    let guidedCleanupRecordResult: GuidedCleanupRecordStepResult?
    let isPlanningGuidedCleanupFlow: Bool
    let isRecordingGuidedCleanupStep: Bool
    let onScoreQuality: () -> Void
    let onPreviewQualityPrompt: () -> Void
    let onSendQualityPrompt: () -> Void
    let onBuildTaskCockpit: () -> Void
    let onCheckTaskReadiness: () -> Void
    let onPreviewTaskReadinessPrompt: () -> Void
    let onSendTaskReadinessPrompt: () -> Void
    let onRankRoutingConfidence: () -> Void
    let onPreviewRoutingConfidencePrompt: () -> Void
    let onSendRoutingConfidencePrompt: () -> Void
    let onCompareCrossAgentReadiness: () -> Void
    let onLoadRoutingAccuracyDashboard: () -> Void
    let onDetectStaleDrift: () -> Void
    let onSearchKnowledge: () -> Void
    let onBuildLocalSkillMap: () -> Void
    let onLoadSkillLifecycleTimeline: () -> Void
    let onLoadProviderObservability: () -> Void
    let onGroupSimilarSkills: () -> Void
    let onBuildCapabilityTaxonomy: () -> Void
    let onCheckWorkspaceReadiness: () -> Void
    let onPlanRemediation: () -> Void
    let onPreviewRemediationDrafts: () -> Void
    let onPreviewRemediationImpact: () -> Void
    let onReviewRemediationBatch: (RemediationBatchReviewOptions) -> Void
    let onLoadRemediationHistory: () -> Void
    let onRecordRemediationHistory: () -> Void
    let onPlanGuidedCleanupFlow: () -> Void
    let onRecordGuidedCleanupStep: (GuidedCleanupFlowStep) -> Void
    @Binding var taskBenchmarkText: String
    let taskBenchmarkInput: String
    let taskBenchmarkList: TaskBenchmarkListResult
    let taskBenchmarkEvaluation: TaskBenchmarkEvaluationResult?
    let taskBenchmarkDeleteResult: TaskBenchmarkDeleteResult?
    let routingRegressionBaseline: RoutingRegressionBaselineResult?
    let routingRegressionDetection: RoutingRegressionDetectionResult?
    let isLoadingTaskBenchmarks: Bool
    let isSavingTaskBenchmark: Bool
    let isEvaluatingTaskBenchmarks: Bool
    let isSavingRoutingBaseline: Bool
    let isDetectingRoutingRegression: Bool
    let isDeletingTaskBenchmark: (TaskBenchmarkRecord) -> Bool
    let onLoadTaskBenchmarks: () -> Void
    let onSaveTaskBenchmark: () -> Void
    let onEvaluateTaskBenchmarks: () -> Void
    let onSaveRoutingBaseline: () -> Void
    let onDetectRoutingRegression: () -> Void
    let onDeleteTaskBenchmark: (TaskBenchmarkRecord) -> Void
    @Binding var traceImportText: String
    @Binding var traceImportTitle: String
    @Binding var traceImportTask: String
    @Binding var traceImportExpectedSkills: String
    let traceImportList: AgentTraceImportListResult
    let traceImportResult: AgentTraceImportResult?
    let traceImportDeleteResult: AgentTraceImportDeleteResult?
    let latestTraceImportRecord: AgentTraceImportRecord?
    let isLoadingTraceImports: Bool
    let isImportingTrace: Bool
    let isDeletingTraceImport: (AgentTraceImportRecord) -> Bool
    @Binding var agentSessionSkillReviewTranscript: String
    @Binding var agentSessionSkillReviewTask: String
    @Binding var agentSessionSkillReviewExpectedSkills: String
    @Binding var localSessionPreviewRoots: String
    let agentSessionSkillReviewList: AgentSessionSkillReviewListResult
    let agentSessionSkillReviewResult: AgentSessionSkillReviewResult?
    let agentSessionSkillReviewDeleteResult: AgentSessionSkillReviewDeleteResult?
    let localSessionPreviewResult: LocalSessionPreviewResult
    let latestAgentSessionSkillReview: AgentSessionSkillReviewRecord?
    let isLoadingAgentSessionSkillReviews: Bool
    let isReviewingAgentSessionSkillUse: Bool
    let isPreviewingLocalSessions: Bool
    let isDeletingAgentSessionSkillReview: (AgentSessionSkillReviewRecord) -> Bool
    let onLoadAgentSessionSkillReviews: () -> Void
    let onReviewAgentSessionSkillUse: () -> Void
    let onPreviewLocalSessions: () -> Void
    let onDeleteAgentSessionSkillReview: (AgentSessionSkillReviewRecord) -> Void
    let onLoadTraceImports: () -> Void
    let onImportTrace: () -> Void
    let onDeleteTraceImport: (AgentTraceImportRecord) -> Void
    let isPreparing: (LLMAction) -> Bool
    let result: (LLMAction) -> LLMPrepareResult?
    let promptPreview: (LLMAction) -> LLMPromptPreview?
    let isPreviewingPrompt: (LLMAction) -> Bool
    let isSendingPrompt: (LLMAction) -> Bool
    let promptSendResult: (LLMAction) -> LLMPromptSendResult?
    let canSendPrompt: (LLMAction) -> Bool
    let skillAnalysisResult: (LLMSkillAnalysisKind, LLMSkillAnalysisRequestScope) -> LLMSkillAnalysisPrepareResult?
    let isPreparingSkillAnalysis: (LLMSkillAnalysisKind, LLMSkillAnalysisRequestScope) -> Bool
    let skillAnalysisPromptPreview: (LLMSkillAnalysisKind, LLMSkillAnalysisRequestScope) -> LLMPromptPreview?
    let isPreviewingSkillAnalysisPrompt: (LLMSkillAnalysisKind, LLMSkillAnalysisRequestScope) -> Bool
    let isSendingSkillAnalysisPrompt: (LLMSkillAnalysisKind, LLMSkillAnalysisRequestScope) -> Bool
    let skillAnalysisPromptSendResult: (LLMSkillAnalysisKind, LLMSkillAnalysisRequestScope) -> LLMPromptSendResult?
    let canSendSkillAnalysisPrompt: (LLMSkillAnalysisKind, LLMSkillAnalysisRequestScope) -> Bool
    let onPrepareSkillAnalysis: (LLMSkillAnalysisKind, LLMSkillAnalysisRequestScope) -> Void
    let onPreviewSkillAnalysisPrompt: (LLMSkillAnalysisKind, LLMSkillAnalysisRequestScope) -> Void
    let onSendSkillAnalysisPrompt: (LLMSkillAnalysisKind, LLMSkillAnalysisRequestScope) -> Void
    let onPrepare: (LLMAction) -> Void
    let onPreviewPrompt: (LLMAction) -> Void
    let onSendPrompt: (LLMAction) -> Void
    let scriptPreview: ScriptExecutionPreview?
    let isPreviewingScript: Bool
    let onPreviewScript: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            VStack(alignment: .leading, spacing: 10) {
                Label(UIStrings.text("analysis.workbench", "Smart Analysis"), systemImage: "sparkles.rectangle.stack")
                    .font(.headline)
                Text(UIStrings.text("analysis.workbench.summary.compact", "Focused single-skill analysis: score local quality and test task fit/routing. Provider-backed explanations remain previewed, confirmed, copy-only, and persisted as local redacted run history."))
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            SkillQualityScorePanel(
                skill: skill,
                result: qualityScore(skill),
                isScoring: isScoringQuality(skill),
                promptPreview: qualityPromptPreview(skill),
                isPreviewingPrompt: isPreviewingQualityPrompt(skill),
                isSendingPrompt: isSendingQualityPrompt(skill),
                promptSendResult: qualityPromptSendResult(skill),
                canSendPrompt: canSendQualityPrompt(skill),
                onScore: onScoreQuality,
                onPreviewPrompt: onPreviewQualityPrompt,
                onSendPrompt: onSendQualityPrompt
            )

            TaskRoutingAssessmentPanel(
                skill: skill,
                readinessText: $taskReadinessText,
                readinessResult: taskReadinessResult(skill),
                isCheckingReadiness: isCheckingTaskReadiness(skill),
                readinessPromptPreview: taskReadinessPromptPreview(skill),
                isPreviewingReadinessPrompt: isPreviewingTaskReadinessPrompt(skill),
                isSendingReadinessPrompt: isSendingTaskReadinessPrompt(skill),
                readinessPromptSendResult: taskReadinessPromptSendResult(skill),
                canSendReadinessPrompt: canSendTaskReadinessPrompt(skill),
                routingText: $routingConfidenceText,
                routingResult: routingConfidenceResult(skill),
                isRankingRouting: isRankingRoutingConfidence(skill),
                routingPromptPreview: routingConfidencePromptPreview(skill),
                isPreviewingRoutingPrompt: isPreviewingRoutingConfidencePrompt(skill),
                isSendingRoutingPrompt: isSendingRoutingConfidencePrompt(skill),
                routingPromptSendResult: routingConfidencePromptSendResult(skill),
                canSendRoutingPrompt: canSendRoutingConfidencePrompt(skill),
                onCheckReadiness: onCheckTaskReadiness,
                onPreviewReadinessPrompt: onPreviewTaskReadinessPrompt,
                onSendReadinessPrompt: onSendTaskReadinessPrompt,
                onRankRouting: onRankRoutingConfidence,
                onPreviewRoutingPrompt: onPreviewRoutingConfidencePrompt,
                onSendRoutingPrompt: onSendRoutingConfidencePrompt
            )

        }
    }

}


struct SkillQualityScorePanel: View {
    let skill: SkillRecord
    let result: SkillQualityScoreResult?
    let isScoring: Bool
    let promptPreview: LLMPromptPreview?
    let isPreviewingPrompt: Bool
    let isSendingPrompt: Bool
    let promptSendResult: LLMPromptSendResult?
    let canSendPrompt: Bool
    let onScore: () -> Void
    let onPreviewPrompt: () -> Void
    let onSendPrompt: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.skillQualityTitle, systemImage: "gauge.medium")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.skillQualityBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            HStack(spacing: 8) {
                Button {
                    onScore()
                } label: {
                    Label(UIStrings.skillQualityScoreAction, systemImage: "gauge.medium")
                }
                .disabled(isScoring)
                .help(UIStrings.skillQualityBoundary)

                if isScoring {
                    Label(UIStrings.llmPreparing, systemImage: "hourglass")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
            }

            if let result {
                SkillQualityScoreResultView(result: result)
            } else {
                Label(UIStrings.text("quality.empty.prompt", "Run Score Quality to evaluate this skill from local catalog evidence."), systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            PromptPreviewControls(
                preview: promptPreview,
                sendResult: promptSendResult,
                isPreviewing: isPreviewingPrompt,
                isSending: isSendingPrompt,
                canSend: canSendPrompt,
                onPreview: onPreviewPrompt,
                onSend: onSendPrompt
            )
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

struct SkillQualityScoreResultView: View {
    let result: SkillQualityScoreResult

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline, spacing: 14) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("\(result.score)")
                        .font(.system(size: 34, weight: .semibold, design: .rounded))
                        .monospacedDigit()
                    Text(UIStrings.skillQualityScore)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                VStack(alignment: .leading, spacing: 4) {
                    Text(result.displayBand)
                        .font(.subheadline.bold())
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(scoreTint.opacity(0.16), in: Capsule())
                        .foregroundStyle(scoreTint)
                    if !result.summary.isEmpty {
                        Text(result.summary)
                            .font(.callout)
                            .textSelection(.enabled)
                    }
                    if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                        Label(fallbackReason, systemImage: "info.circle")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                Spacer()
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.skillQualityBand, value: result.displayBand)
                MetadataRow(label: UIStrings.skillQualityProviderNotSent, value: result.safety.providerRequestSent ? UIStrings.llmSkillAnalysisEnabledUnsafe : UIStrings.llmDisabled)
                MetadataRow(label: UIStrings.skillQualityWritesBlocked, value: readOnlyValue(!result.safety.writeBackAllowed && !result.safety.writeActionsAvailable))
                MetadataRow(label: UIStrings.skillQualityScriptsBlocked, value: readOnlyValue(!result.safety.scriptExecutionAllowed && !result.safety.executionActionsAvailable))
                MetadataRow(label: UIStrings.skillQualityMutationsBlocked, value: readOnlyValue(!result.safety.configMutationAllowed && !result.safety.snapshotCreated && !result.safety.triageMutationAllowed))
                MetadataRow(label: UIStrings.skillQualityCredentialsBlocked, value: readOnlyValue(!result.safety.credentialAccessed && !result.safety.rawSecretReturned))
            }

            SkillQualityComponentList(components: result.components)
            SkillQualityEvidenceList(evidence: result.evidence)
            SkillQualityStringList(title: UIStrings.skillQualityRiskNotes, empty: UIStrings.skillQualityNoRisks, values: result.riskNotes, systemImage: "exclamationmark.triangle")
            SkillQualityStringList(title: UIStrings.skillQualitySuggestions, empty: UIStrings.skillQualityNoSuggestions, values: result.suggestedImprovements, systemImage: "wand.and.stars")

            Label(UIStrings.llmReviewNoActions, systemImage: "nosign")
                .font(.callout)
                .foregroundStyle(.secondary)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
    }

    private var scoreTint: Color {
        switch result.score {
        case 85...100:
            return .green
        case 65..<85:
            return .blue
        case 45..<65:
            return .orange
        default:
            return .red
        }
    }

    private func readOnlyValue(_ isBlocked: Bool) -> String {
        isBlocked ? UIStrings.llmSkillAnalysisBlocked : UIStrings.llmSkillAnalysisEnabledUnsafe
    }
}

struct SkillQualityComponentList: View {
    let components: [SkillQualityComponent]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.skillQualityComponents)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if components.isEmpty {
                Text(UIStrings.skillQualityNoComponents)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 180), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(components) { component in
                        VStack(alignment: .leading, spacing: 5) {
                            HStack(alignment: .firstTextBaseline) {
                                Text(component.label)
                                    .font(.caption.bold())
                                Spacer()
                                Text(componentScore(component))
                                    .font(.caption.monospacedDigit().bold())
                            }
                            if let status = component.status, !status.isEmpty {
                                Text(status)
                                    .font(.caption2)
                                    .foregroundStyle(.secondary)
                            }
                            if !component.summary.isEmpty {
                                Text(component.summary)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                    .lineLimit(3)
                            }
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }
        }
    }

    private func componentScore(_ component: SkillQualityComponent) -> String {
        if let maxScore = component.maxScore {
            return "\(component.score)/\(maxScore)"
        }
        return "\(component.score)"
    }
}

struct SkillQualityEvidenceList: View {
    let evidence: [SkillQualityEvidenceItem]

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Text(UIStrings.skillQualityEvidence)
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                if !evidence.isEmpty {
                    DenseCountBadge(count: evidence.count)
                }
            }
            if evidence.isEmpty {
                Text(UIStrings.skillQualityNoEvidence)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                DenseDisclosureList(evidence, visibleLimit: 6, spacing: 6) { item in
                    VStack(alignment: .leading, spacing: 2) {
                        Label(item.title, systemImage: "checklist")
                            .font(.callout)
                        PrivacyEvidenceText(value: item.detail, font: .caption, lineLimit: nil)
                        if let source = item.source, !source.isEmpty {
                            PrivacyEvidenceText(value: source, font: .caption2, lineLimit: 1)
                        }
                    }
                }
            }
        }
    }
}

struct SkillQualityStringList: View {
    let title: String
    let empty: String
    let values: [String]
    let systemImage: String

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Text(title)
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                if !values.isEmpty {
                    DenseCountBadge(count: values.count)
                }
            }
            if values.isEmpty {
                Text(empty)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                DenseDisclosureList(values, visibleLimit: 6, spacing: 4) { value in
                    PrivacyEvidenceLabel(value: value, systemImage: systemImage, font: .callout, lineLimit: 3)
                }
            }
        }
    }
}

struct TaskReadinessPanel: View {
    let skill: SkillRecord
    @Binding var taskText: String
    let result: TaskReadinessResult?
    let isChecking: Bool
    let promptPreview: LLMPromptPreview?
    let isPreviewingPrompt: Bool
    let isSendingPrompt: Bool
    let promptSendResult: LLMPromptSendResult?
    let canSendPrompt: Bool
    let onCheck: () -> Void
    let onPreviewPrompt: () -> Void
    let onSendPrompt: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.taskReadinessTitle, systemImage: "checklist.checked")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.taskReadinessBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            HStack(alignment: .firstTextBaseline, spacing: 10) {
                TextField(UIStrings.taskReadinessTaskPlaceholder, text: $taskText, axis: .vertical)
                    .textFieldStyle(.roundedBorder)
                    .lineLimit(2...4)
                    .labelsHidden()

                Button {
                    onCheck()
                } label: {
                    Label(UIStrings.taskReadinessCheckAction, systemImage: "checklist")
                }
                .disabled(isChecking || taskText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                .help(UIStrings.taskReadinessBoundary)
            }

            if isChecking {
                Label(UIStrings.llmPreparing, systemImage: "hourglass")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            if let result {
                TaskReadinessResultView(result: result)
            } else {
                Label(UIStrings.taskReadinessTaskRequired, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            PromptPreviewControls(
                preview: promptPreview,
                sendResult: promptSendResult,
                isPreviewing: isPreviewingPrompt,
                isSending: isSendingPrompt,
                canSend: canSendPrompt,
                onPreview: onPreviewPrompt,
                onSend: onSendPrompt
            )
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

struct TaskRoutingAssessmentPanel: View {
    let skill: SkillRecord
    @Binding var readinessText: String
    let readinessResult: TaskReadinessResult?
    let isCheckingReadiness: Bool
    let readinessPromptPreview: LLMPromptPreview?
    let isPreviewingReadinessPrompt: Bool
    let isSendingReadinessPrompt: Bool
    let readinessPromptSendResult: LLMPromptSendResult?
    let canSendReadinessPrompt: Bool
    @Binding var routingText: String
    let routingResult: SkillRoutingConfidenceResult?
    let isRankingRouting: Bool
    let routingPromptPreview: LLMPromptPreview?
    let isPreviewingRoutingPrompt: Bool
    let isSendingRoutingPrompt: Bool
    let routingPromptSendResult: LLMPromptSendResult?
    let canSendRoutingPrompt: Bool
    let onCheckReadiness: () -> Void
    let onPreviewReadinessPrompt: () -> Void
    let onSendReadinessPrompt: () -> Void
    let onRankRouting: () -> Void
    let onPreviewRoutingPrompt: () -> Void
    let onSendRoutingPrompt: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.text("analysis.taskRouting.title", "Task Fit & Routing"), systemImage: "point.3.connected.trianglepath.dotted")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.text("analysis.taskRouting.boundary", "Use one real task to check whether this skill is ready for the work and whether routing to it is confident. Local scoring stays deterministic; provider output is optional, confirmed, and copy-only."))
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            VStack(alignment: .leading, spacing: 12) {
                Label(UIStrings.taskReadinessTitle, systemImage: "checklist.checked")
                    .font(.subheadline.bold())
                HStack(alignment: .firstTextBaseline, spacing: 10) {
                    TextField(UIStrings.taskReadinessTaskPlaceholder, text: $readinessText, axis: .vertical)
                        .textFieldStyle(.roundedBorder)
                        .lineLimit(2...4)
                        .labelsHidden()

                    Button {
                        onCheckReadiness()
                    } label: {
                        Label(UIStrings.taskReadinessCheckAction, systemImage: "checklist")
                    }
                    .disabled(isCheckingReadiness || readinessText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    .help(UIStrings.taskReadinessBoundary)
                }

                if isCheckingReadiness {
                    Label(UIStrings.llmPreparing, systemImage: "hourglass")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }

                if let readinessResult {
                    TaskReadinessResultView(result: readinessResult)
                } else {
                    Label(UIStrings.taskReadinessTaskRequired, systemImage: "info.circle")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }

                PromptPreviewControls(
                    preview: readinessPromptPreview,
                    sendResult: readinessPromptSendResult,
                    isPreviewing: isPreviewingReadinessPrompt,
                    isSending: isSendingReadinessPrompt,
                    canSend: canSendReadinessPrompt,
                    onPreview: onPreviewReadinessPrompt,
                    onSend: onSendReadinessPrompt
                )
            }

            Divider()

            VStack(alignment: .leading, spacing: 12) {
                Label(UIStrings.routingConfidenceTitle, systemImage: "arrow.up.arrow.down.square")
                    .font(.subheadline.bold())
                HStack(alignment: .firstTextBaseline, spacing: 10) {
                    TextField(UIStrings.routingConfidenceTaskPlaceholder, text: $routingText, axis: .vertical)
                        .textFieldStyle(.roundedBorder)
                        .lineLimit(2...4)
                        .labelsHidden()

                    Button {
                        onRankRouting()
                    } label: {
                        Label(UIStrings.routingConfidenceAction, systemImage: "arrow.up.arrow.down.square")
                    }
                    .disabled(isRankingRouting || routingText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    .help(UIStrings.routingConfidenceBoundary)
                }

                if isRankingRouting {
                    Label(UIStrings.llmPreparing, systemImage: "hourglass")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }

                if let routingResult {
                    RoutingConfidenceResultView(result: routingResult)
                } else {
                    Label(UIStrings.routingConfidenceTaskRequired, systemImage: "info.circle")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }

                PromptPreviewControls(
                    preview: routingPromptPreview,
                    sendResult: routingPromptSendResult,
                    isPreviewing: isPreviewingRoutingPrompt,
                    isSending: isSendingRoutingPrompt,
                    canSend: canSendRoutingPrompt,
                    onPreview: onPreviewRoutingPrompt,
                    onSend: onSendRoutingPrompt
                )
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

struct TaskReadinessResultView: View {
    let result: TaskReadinessResult

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline, spacing: 14) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("\(result.score)")
                        .font(.system(size: 34, weight: .semibold, design: .rounded))
                        .monospacedDigit()
                    Text(UIStrings.taskReadinessScore)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                VStack(alignment: .leading, spacing: 4) {
                    Text(result.band)
                        .font(.subheadline.bold())
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(scoreTint.opacity(0.16), in: Capsule())
                        .foregroundStyle(scoreTint)
                    if !result.summary.isEmpty {
                        Text(result.summary)
                            .font(.callout)
                            .textSelection(.enabled)
                    }
                    if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                        Label(fallbackReason, systemImage: "info.circle")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                Spacer()
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.taskReadinessBand, value: result.band)
                MetadataRow(label: UIStrings.skillQualityProviderNotSent, value: result.safety.providerRequestSent ? UIStrings.llmSkillAnalysisEnabledUnsafe : UIStrings.llmDisabled)
                MetadataRow(label: UIStrings.skillQualityWritesBlocked, value: readOnlyValue(!result.safety.writeBackAllowed && !result.safety.writeActionsAvailable))
                MetadataRow(label: UIStrings.skillQualityScriptsBlocked, value: readOnlyValue(!result.safety.scriptExecutionAllowed && !result.safety.executionActionsAvailable))
                MetadataRow(label: UIStrings.skillQualityMutationsBlocked, value: readOnlyValue(!result.safety.configMutationAllowed && !result.safety.snapshotCreated && !result.safety.triageMutationAllowed))
                MetadataRow(label: UIStrings.skillQualityCredentialsBlocked, value: readOnlyValue(!result.safety.credentialAccessed && !result.safety.rawSecretReturned))
            }

            TaskReadinessCandidateList(candidates: result.candidateSkills)
            SkillQualityStringList(title: UIStrings.taskReadinessGaps, empty: UIStrings.taskReadinessNoGaps, values: result.gaps, systemImage: "puzzlepiece.extension")
            SkillQualityStringList(title: UIStrings.taskReadinessBlockers, empty: UIStrings.taskReadinessNoBlockers, values: result.blockers, systemImage: "exclamationmark.octagon")
            SkillQualityStringList(title: UIStrings.taskReadinessRiskNotes, empty: UIStrings.taskReadinessNoRisks, values: result.riskNotes, systemImage: "exclamationmark.triangle")
            TaskReadinessEvidenceList(evidence: result.evidence)

            Label(UIStrings.llmReviewNoActions, systemImage: "nosign")
                .font(.callout)
                .foregroundStyle(.secondary)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
    }

    private var scoreTint: Color {
        switch result.score {
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

    private func readOnlyValue(_ isBlocked: Bool) -> String {
        isBlocked ? UIStrings.llmSkillAnalysisBlocked : UIStrings.llmSkillAnalysisEnabledUnsafe
    }
}

struct TaskReadinessCandidateList: View {
    let candidates: [TaskReadinessCandidateSkill]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.taskReadinessCandidates)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if candidates.isEmpty {
                Text(UIStrings.taskReadinessNoCandidates)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 190), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(candidates) { candidate in
                        VStack(alignment: .leading, spacing: 5) {
                            HStack(alignment: .firstTextBaseline) {
                                Text(candidate.name)
                                    .font(.caption.bold())
                                    .lineLimit(1)
                                    .truncationMode(.middle)
                                Spacer()
                                Text(candidateScore(candidate))
                                    .font(.caption.monospacedDigit().bold())
                            }
                            Text(DisplayText.agent(candidate.agent))
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                            if let readiness = candidate.readiness, !readiness.isEmpty {
                                Text(readiness)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                            if !candidate.rationale.isEmpty {
                                Text(candidate.rationale)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                    .lineLimit(3)
                            }
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }
        }
    }

    private func candidateScore(_ candidate: TaskReadinessCandidateSkill) -> String {
        candidate.score.map(String.init) ?? UIStrings.unknown
    }
}

struct TaskReadinessEvidenceList: View {
    let evidence: [TaskReadinessEvidenceItem]

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Text(UIStrings.taskReadinessEvidence)
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                if !evidence.isEmpty {
                    DenseCountBadge(count: evidence.count)
                }
            }
            if evidence.isEmpty {
                Text(UIStrings.taskReadinessNoEvidence)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                DenseDisclosureList(evidence, visibleLimit: 6, spacing: 6) { item in
                    VStack(alignment: .leading, spacing: 2) {
                        Label(item.title, systemImage: "checklist")
                            .font(.callout)
                        PrivacyEvidenceText(value: item.detail, font: .caption, lineLimit: nil)
                        if let source = item.source, !source.isEmpty {
                            PrivacyEvidenceText(value: source, font: .caption2, lineLimit: 1)
                        }
                    }
                }
            }
        }
    }
}

struct RoutingConfidencePanel: View {
    let skill: SkillRecord
    @Binding var taskText: String
    let result: SkillRoutingConfidenceResult?
    let isRanking: Bool
    let promptPreview: LLMPromptPreview?
    let isPreviewingPrompt: Bool
    let isSendingPrompt: Bool
    let promptSendResult: LLMPromptSendResult?
    let canSendPrompt: Bool
    let onRank: () -> Void
    let onPreviewPrompt: () -> Void
    let onSendPrompt: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.routingConfidenceTitle, systemImage: "point.3.connected.trianglepath.dotted")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.routingConfidenceBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            HStack(alignment: .firstTextBaseline, spacing: 10) {
                TextField(UIStrings.routingConfidenceTaskPlaceholder, text: $taskText, axis: .vertical)
                    .textFieldStyle(.roundedBorder)
                    .lineLimit(2...4)
                    .labelsHidden()

                Button {
                    onRank()
                } label: {
                    Label(UIStrings.routingConfidenceAction, systemImage: "arrow.up.arrow.down.square")
                }
                .disabled(isRanking || taskText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                .help(UIStrings.routingConfidenceBoundary)
            }

            if isRanking {
                Label(UIStrings.llmPreparing, systemImage: "hourglass")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            if let result {
                RoutingConfidenceResultView(result: result)
            } else {
                Label(UIStrings.routingConfidenceTaskRequired, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            PromptPreviewControls(
                preview: promptPreview,
                sendResult: promptSendResult,
                isPreviewing: isPreviewingPrompt,
                isSending: isSendingPrompt,
                canSend: canSendPrompt,
                onPreview: onPreviewPrompt,
                onSend: onSendPrompt
            )
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

struct RoutingConfidenceResultView: View {
    let result: SkillRoutingConfidenceResult

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline, spacing: 14) {
                VStack(alignment: .leading, spacing: 2) {
                    Text("\(result.score)")
                        .font(.system(size: 34, weight: .semibold, design: .rounded))
                        .monospacedDigit()
                    Text(UIStrings.routingConfidenceScore)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                VStack(alignment: .leading, spacing: 4) {
                    Text(result.band)
                        .font(.subheadline.bold())
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(scoreTint.opacity(0.16), in: Capsule())
                        .foregroundStyle(scoreTint)
                    if !result.summary.isEmpty {
                        Text(result.summary)
                            .font(.callout)
                            .textSelection(.enabled)
                    }
                    if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                        Label(fallbackReason, systemImage: "info.circle")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }
                Spacer()
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.routingConfidenceBand, value: result.band)
                MetadataRow(label: UIStrings.skillQualityProviderNotSent, value: result.safety.providerRequestSent ? UIStrings.llmSkillAnalysisEnabledUnsafe : UIStrings.llmDisabled)
                MetadataRow(label: UIStrings.skillQualityWritesBlocked, value: readOnlyValue(!result.safety.writeBackAllowed && !result.safety.writeActionsAvailable))
                MetadataRow(label: UIStrings.skillQualityScriptsBlocked, value: readOnlyValue(!result.safety.scriptExecutionAllowed && !result.safety.executionActionsAvailable))
                MetadataRow(label: UIStrings.skillQualityMutationsBlocked, value: readOnlyValue(!result.safety.configMutationAllowed && !result.safety.snapshotCreated && !result.safety.triageMutationAllowed))
                MetadataRow(label: UIStrings.skillQualityCredentialsBlocked, value: readOnlyValue(!result.safety.credentialAccessed && !result.safety.rawSecretReturned))
            }

            RoutingRouteList(routes: result.routes)
            SkillQualityStringList(title: UIStrings.routingConfidenceAmbiguity, empty: UIStrings.routingConfidenceNoAmbiguity, values: result.ambiguityWarnings, systemImage: "exclamationmark.triangle")
            SkillQualityStringList(title: UIStrings.routingConfidenceWrongPick, empty: UIStrings.routingConfidenceNoWrongPick, values: result.wrongPickRisks, systemImage: "xmark.octagon")
            RoutingEvidenceList(evidence: result.evidence)

            Label(UIStrings.llmReviewNoActions, systemImage: "nosign")
                .font(.callout)
                .foregroundStyle(.secondary)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
    }

    private var scoreTint: Color {
        switch result.score {
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

    private func readOnlyValue(_ isBlocked: Bool) -> String {
        isBlocked ? UIStrings.llmSkillAnalysisBlocked : UIStrings.llmSkillAnalysisEnabledUnsafe
    }
}

struct RoutingRouteList: View {
    let routes: [SkillRouteCandidate]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.routingConfidenceRoutes)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if routes.isEmpty {
                Text(UIStrings.routingConfidenceNoRoutes)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 220), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(Array(routes.enumerated()), id: \.element.id) { index, route in
                        VStack(alignment: .leading, spacing: 7) {
                            HStack(alignment: .firstTextBaseline) {
                                Text("#\(index + 1) \(route.name)")
                                    .font(.caption.bold())
                                    .lineLimit(1)
                                    .truncationMode(.middle)
                                Spacer()
                                Text("\(route.confidenceScore)")
                                    .font(.caption.monospacedDigit().bold())
                            }
                            HStack(spacing: 6) {
                                Text(DisplayText.agent(route.agent))
                                    .font(.caption2)
                                    .foregroundStyle(.secondary)
                                Text(route.band)
                                    .font(.caption2.bold())
                                    .padding(.horizontal, 6)
                                    .padding(.vertical, 2)
                                    .background(Color.agentCopilotPanelBackground, in: Capsule())
                            }
                            if !route.summary.isEmpty {
                                Text(route.summary)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                    .lineLimit(3)
                                    .textSelection(.enabled)
                            }
                            RoutingInlineList(title: UIStrings.routingConfidenceMatchReasons, empty: UIStrings.routingConfidenceNoReasons, values: route.matchReasons, systemImage: "checkmark.circle")
                            RoutingInlineList(title: UIStrings.routingConfidenceAmbiguity, empty: UIStrings.routingConfidenceNoAmbiguity, values: route.ambiguityWarnings, systemImage: "exclamationmark.triangle")
                            RoutingInlineList(title: UIStrings.routingConfidenceWrongPick, empty: UIStrings.routingConfidenceNoWrongPick, values: route.wrongPickRisks, systemImage: "xmark.octagon")
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

struct RoutingEvidenceList: View {
    let evidence: [TaskReadinessEvidenceItem]

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Text(UIStrings.routingConfidenceEvidence)
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                if !evidence.isEmpty {
                    DenseCountBadge(count: evidence.count)
                }
            }
            if evidence.isEmpty {
                Text(UIStrings.routingConfidenceNoEvidence)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                DenseDisclosureList(evidence, visibleLimit: 6, spacing: 6) { item in
                    VStack(alignment: .leading, spacing: 2) {
                        Label(item.title, systemImage: "checklist")
                            .font(.callout)
                        PrivacyEvidenceText(value: item.detail, font: .caption, lineLimit: nil)
                        if let source = item.source, !source.isEmpty {
                            PrivacyEvidenceText(value: source, font: .caption2, lineLimit: 1)
                        }
                    }
                }
            }
        }
    }
}

struct CrossAgentReadinessPanel: View {
    @Binding var taskText: String
    let currentTaskText: String
    let result: CrossAgentReadinessResult?
    let isComparing: Bool
    let onCompare: () -> Void

    private var effectiveTaskText: String {
        let trimmed = taskText.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? currentTaskText : trimmed
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.crossAgentReadinessTitle, systemImage: "person.3.sequence")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.crossAgentReadinessBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            HStack(alignment: .firstTextBaseline, spacing: 10) {
                TextField(UIStrings.crossAgentReadinessTaskPlaceholder, text: $taskText, axis: .vertical)
                    .textFieldStyle(.roundedBorder)
                    .lineLimit(2...4)
                    .labelsHidden()

                Button {
                    onCompare()
                } label: {
                    Label(UIStrings.crossAgentReadinessCompareAction, systemImage: "arrow.left.arrow.right.square")
                }
                .disabled(isComparing || effectiveTaskText.isEmpty)
                .help(UIStrings.crossAgentReadinessBoundary)
            }

            if isComparing {
                Label(UIStrings.llmPreparing, systemImage: "hourglass")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            if let result {
                CrossAgentReadinessResultView(result: result)
            } else {
                Label(UIStrings.crossAgentReadinessNoResult, systemImage: "info.circle")
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

struct CrossAgentReadinessResultView: View {
    let result: CrossAgentReadinessResult

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            DetailMetricGrid {
                SummaryChip(
                    title: UIStrings.crossAgentReadinessAgents,
                    value: "\(result.summary.agentCount > 0 ? result.summary.agentCount : result.agentRows.count)",
                    systemImage: "person.3"
                )
                SummaryChip(
                    title: UIStrings.crossAgentReadinessCandidateCount,
                    value: "\(result.summary.candidateCount)",
                    systemImage: "rectangle.stack"
                )
                SummaryChip(
                    title: UIStrings.crossAgentReadinessReadinessScore,
                    value: CrossAgentReadinessSummary.scoreLabel(result.summary.averageReadinessScore),
                    systemImage: "gauge.medium"
                )
                SummaryChip(
                    title: UIStrings.crossAgentReadinessRoutingScore,
                    value: CrossAgentReadinessSummary.scoreLabel(result.summary.averageRoutingScore),
                    systemImage: "point.3.connected.trianglepath.dotted"
                )
                SummaryChip(
                    title: UIStrings.taskReadinessGaps,
                    value: "\(result.summary.gapCount)",
                    systemImage: "puzzlepiece.extension"
                )
                SummaryChip(
                    title: UIStrings.taskReadinessBlockers,
                    value: "\(result.summary.blockerCount)",
                    systemImage: "exclamationmark.octagon"
                )
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.routingAccuracyGeneratedBy, value: result.generatedBy)
                MetadataRow(label: UIStrings.routingAccuracyCatalog, value: result.catalogAvailable ? UIStrings.routingAccuracyAvailable : UIStrings.routingAccuracyUnavailableShort)
                MetadataRow(label: UIStrings.taskReadinessTask, value: result.taskText.isEmpty ? UIStrings.unknown : result.taskText)
                MetadataRow(label: UIStrings.crossAgentReadinessCandidateCount, value: "\(result.summary.candidateCount)")
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

            CrossAgentReadinessRecommendationView(recommendation: result.recommendedAgent)
            CrossAgentReadinessAgentList(agents: result.agentRows)
            CrossAgentReadinessGapList(gaps: result.gapIssueRows)
            CrossAgentReadinessEvidenceList(evidence: result.evidenceReferences)
            CrossAgentReadinessSafetyList(safety: result.safetyFlags)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
    }

    private func promptRequestLabel(_ promptRequest: RoutingAccuracyPromptRequest) -> String {
        let state = promptRequest.enabled ? UIStrings.llmEnabled : UIStrings.llmDisabled
        let copy = promptRequest.copyOnly ? UIStrings.llmPromptCopyOnly : UIStrings.llmSkillAnalysisEnabledUnsafe
        return "\(promptRequest.requestKind) · \(state) · \(copy)"
    }
}

struct CrossAgentReadinessRecommendationView: View {
    let recommendation: CrossAgentReadinessRecommendedAgent?

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(UIStrings.crossAgentReadinessRecommendedAgent)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if let recommendation {
                HStack(alignment: .firstTextBaseline, spacing: 10) {
                    Label(recommendation.displayName ?? DisplayText.agent(recommendation.agent), systemImage: "target")
                        .font(.callout.bold())
                    if let comparisonScore = recommendation.comparisonScore {
                        Text("\(comparisonScore)")
                            .font(.caption.monospacedDigit().bold())
                            .foregroundStyle(.secondary)
                    }
                    if let score = recommendation.score {
                        Text("\(score)")
                            .font(.caption.monospacedDigit().bold())
                            .foregroundStyle(.secondary)
                    }
                    if let routingScore = recommendation.routingScore {
                        Text("\(routingScore)")
                            .font(.caption.monospacedDigit())
                            .foregroundStyle(.secondary)
                    }
                    if let band = recommendation.band, !band.isEmpty {
                        Text(band)
                            .font(.caption.bold())
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(Color.agentCopilotPanelBackground, in: Capsule())
                    }
                    if let skill = recommendation.skill {
                        Text(skill.name)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                    Spacer()
                }
                if !recommendation.summary.isEmpty {
                    Text(recommendation.summary)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
                }
            } else {
                Text(UIStrings.crossAgentReadinessNoRecommendation)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
        }
    }
}

struct CrossAgentReadinessAgentList: View {
    let agents: [CrossAgentReadinessAgentRow]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.crossAgentReadinessAgents)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if agents.isEmpty {
                Text(UIStrings.crossAgentReadinessNoAgents)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 260), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(agents) { row in
                        VStack(alignment: .leading, spacing: 8) {
                            HStack(alignment: .firstTextBaseline) {
                                Text(rowTitle(row))
                                    .font(.callout.bold())
                                    .lineLimit(1)
                                Spacer()
                                if let comparisonScore = row.comparisonScore {
                                    Text("\(comparisonScore)")
                                        .font(.caption.monospacedDigit().bold())
                                }
                                Text("\(row.readinessScore)")
                                    .font(.caption.monospacedDigit().bold())
                                Text(row.readinessBand)
                                    .font(.caption2.bold())
                                    .foregroundStyle(.secondary)
                            }

                            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                                MetadataRow(label: UIStrings.crossAgentReadinessComparisonScore, value: row.comparisonScore.map(String.init) ?? UIStrings.unknown)
                                MetadataRow(label: UIStrings.crossAgentReadinessRoutingScore, value: row.routingLabel)
                                MetadataRow(label: UIStrings.crossAgentReadinessBestSkill, value: row.bestCandidateSkill?.name ?? UIStrings.unknown)
                                MetadataRow(label: UIStrings.crossAgentReadinessCandidateCount, value: "\(row.candidateCount)")
                                MetadataRow(label: UIStrings.crossAgentReadinessEnabledState, value: row.enabledState ?? UIStrings.unknown)
                                MetadataRow(label: UIStrings.crossAgentReadinessScopeState, value: row.scopeState ?? UIStrings.unknown)
                                MetadataRow(label: UIStrings.crossAgentReadinessRiskState, value: row.riskState ?? UIStrings.unknown)
                                MetadataRow(label: UIStrings.taskReadinessBlockers, value: "\(row.blockerCount)")
                                MetadataRow(label: UIStrings.taskReadinessGaps, value: "\(row.gapCount)")
                            }

                            if let accuracy = row.accuracyContext, !accuracy.isEmpty {
                                Label(accuracy, systemImage: "target")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                            if let regression = row.regressionContext, !regression.isEmpty {
                                Label(regression, systemImage: "chart.line.downtrend.xyaxis")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                            if let benchmark = row.benchmarkContext, !benchmark.isEmpty {
                                Label(benchmark, systemImage: "checklist")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }

                            RoutingInlineList(
                                title: UIStrings.crossAgentReadinessReasons,
                                empty: UIStrings.crossAgentReadinessNoReasons,
                                values: row.reasons,
                                systemImage: "checkmark.circle"
                            )
                            RoutingInlineList(
                                title: UIStrings.taskReadinessBlockers,
                                empty: UIStrings.taskReadinessNoBlockers,
                                values: row.blockerNotes,
                                systemImage: "exclamationmark.octagon"
                            )
                            RoutingInlineList(
                                title: UIStrings.taskReadinessGaps,
                                empty: UIStrings.taskReadinessNoGaps,
                                values: row.gapNotes,
                                systemImage: "puzzlepiece.extension"
                            )
                            RoutingInlineList(
                                title: UIStrings.crossAgentReadinessEvidence,
                                empty: UIStrings.crossAgentReadinessNoEvidence,
                                values: row.evidenceRefs,
                                systemImage: "checklist"
                            )
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }
        }
    }

    private func rowTitle(_ row: CrossAgentReadinessAgentRow) -> String {
        if let rank = row.rank {
            return "#\(rank) \(row.displayName ?? DisplayText.agent(row.agent))"
        }
        return row.displayName ?? DisplayText.agent(row.agent)
    }
}

struct CrossAgentReadinessGapList: View {
    let gaps: [CrossAgentReadinessGapIssueRow]

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(UIStrings.crossAgentReadinessGapsIssues)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if gaps.isEmpty {
                Text(UIStrings.crossAgentReadinessNoGapsIssues)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(gaps.prefix(6)) { gap in
                    VStack(alignment: .leading, spacing: 2) {
                        HStack(alignment: .firstTextBaseline) {
                            Label(gap.title, systemImage: "puzzlepiece.extension")
                                .font(.callout)
                            Spacer()
                            if let severity = gap.severity, !severity.isEmpty {
                                Text(severity)
                                    .font(.caption2.bold())
                                    .foregroundStyle(.secondary)
                            }
                        }
                        HStack(spacing: 8) {
                            if let agent = gap.agent, !agent.isEmpty {
                                Text(DisplayText.agent(agent))
                            }
                            if let source = gap.source, !source.isEmpty {
                                PrivacyEvidenceText(value: source, font: .caption2, lineLimit: 1)
                            }
                        }
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        PrivacyEvidenceText(value: gap.detail, font: .caption, lineLimit: nil)
                        if !gap.evidenceRefs.isEmpty {
                            PrivacyEvidenceText(value: gap.evidenceRefs.joined(separator: ", "), font: .caption2, lineLimit: 2)
                        }
                    }
                }
            }
        }
    }
}

struct CrossAgentReadinessEvidenceList: View {
    let evidence: [CrossAgentReadinessEvidenceReference]

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Text(UIStrings.crossAgentReadinessEvidence)
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                if !evidence.isEmpty {
                    DenseCountBadge(count: evidence.count)
                }
            }
            if evidence.isEmpty {
                Text(UIStrings.crossAgentReadinessNoEvidence)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                DenseDisclosureList(evidence, visibleLimit: 6, spacing: 6) { item in
                    VStack(alignment: .leading, spacing: 2) {
                        Label(item.title, systemImage: "checklist")
                            .font(.callout)
                        HStack(spacing: 8) {
                            if let agent = item.agent, !agent.isEmpty {
                                Text(DisplayText.agent(agent))
                            }
                            if let source = item.source, !source.isEmpty {
                                PrivacyEvidenceText(value: source, font: .caption2, lineLimit: 1)
                            }
                        }
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        PrivacyEvidenceText(value: item.detail, font: .caption, lineLimit: nil)
                    }
                }
            }
        }
    }
}

struct CrossAgentReadinessSafetyList: View {
    let safety: CrossAgentReadinessSafety

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.crossAgentReadinessSafetyFlags)
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
                DenseDisclosureList(safety.notes, visibleLimit: 4, spacing: 4) { note in
                    PrivacyEvidenceLabel(value: note, systemImage: "info.circle", font: .caption, lineLimit: 2)
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
