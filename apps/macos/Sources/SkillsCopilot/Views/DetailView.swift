import AppKit
import SwiftUI

struct DetailView: View {
    @EnvironmentObject private var store: SkillStore
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    let skill: SkillRecord?

    private static let topAnchorID = "skills-copilot.detail.top"

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView {
                VStack(spacing: 0) {
                    Color.clear
                        .frame(height: 0)
                        .accessibilityHidden(true)
                        .id(Self.topAnchorID)

                    VStack(alignment: .leading, spacing: 24) {
                        DetailFeedbackInlineView(
                            errorMessage: store.errorMessage,
                            lastMutationMessage: store.lastMutationMessage
                        )
                        .equatable()

                        if store.selectedSidebarSelection?.isSession == true {
                                AgentSessionDetailPanel()
                            } else if store.selectedSidebarSelection?.isConfig == true {
                                AgentConfigDetailPanel()
                            } else if store.selectedDetailSection == .guidedCleanup {
                                DetailSectionSwitcher(selection: $store.selectedDetailSection)

                                GuidedCleanupFlowPanel(
                                    result: store.guidedCleanupFlowResult,
                                    recordResult: store.guidedCleanupRecordResult,
                                    isPlanning: store.isPlanningGuidedCleanupFlow,
                                    isRecording: store.isRecordingGuidedCleanupStep,
                                    onLoad: {
                                        Task {
                                            await store.planGuidedCleanupFlow()
                                        }
                                    },
                                    onRecord: { step in
                                        Task {
                                            await store.recordGuidedCleanupStep(step)
                                        }
                                    },
                                    onOpenSafeLink: { link, step in
                                        Task {
                                            await store.openGuidedCleanupSafeLink(link, step: step)
                                        }
                                    }
                                )
                            } else if store.selectedSidebarSelection?.isSkill == true, let skill {
                                SkillDetailContentView(
                                    skill: skill,
                                    sessionUsage: sessionUsage(for: skill)
                                )
                        } else {
                            EmptyDetailView(
                                title: emptyDetailTitle,
                                message: emptyDetailMessage,
                                systemImage: emptyDetailSystemImage
                            )
                        }
                    }
                    .padding(.top, 8)
                    .padding(.horizontal, 28)
                    .padding(.bottom, 28)
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            .onChange(of: store.selectedDetailSection) { _ in
                scrollToTop(proxy)
            }
            .onChange(of: store.selectedSidebarSelection) { _ in
                scrollToTop(proxy)
            }
        }
        .navigationTitle("")
        .transaction { transaction in
            if reduceMotion {
                transaction.animation = nil
            }
        }
    }

    private var emptyDetailTitle: String {
        switch store.sidebarContentMode {
        case .sessions:
            UIStrings.noSessionSelected
        case .config:
            UIStrings.noConfigSelected
        case .skills:
            UIStrings.noSkillSelected
        }
    }

    private var emptyDetailMessage: String {
        switch store.sidebarContentMode {
        case .sessions:
            UIStrings.noSessionSelectedMessage
        case .config:
            UIStrings.noConfigSelectedMessage
        case .skills:
            UIStrings.noSkillSelectedMessage
        }
    }

    private var emptyDetailSystemImage: String {
        switch store.sidebarContentMode {
        case .sessions:
            "bubble.left.and.bubble.right"
        case .config:
            "slider.horizontal.3"
        case .skills:
            "sparkle.magnifyingglass"
        }
    }

    private func scrollToTop(_ proxy: ScrollViewProxy) {
        if reduceMotion {
            proxy.scrollTo(Self.topAnchorID, anchor: .top)
        } else {
            withAnimation(.easeInOut(duration: 0.18)) {
                proxy.scrollTo(Self.topAnchorID, anchor: .top)
            }
        }
    }

    private func sessionUsage(for skill: SkillRecord) -> LocalSessionSkillUsageRow? {
        store.localSessionPreviewResult.skillUsageRows.first { row in
            row.skillId == skill.id
                || row.skillName == skill.name
                || row.skillName.caseInsensitiveCompare(skill.name) == .orderedSame
        }
    }
}


private struct DetailFeedbackInlineView: View, Equatable {
    let errorMessage: String?
    let lastMutationMessage: String?

    static func == (lhs: DetailFeedbackInlineView, rhs: DetailFeedbackInlineView) -> Bool {
        lhs.errorMessage == rhs.errorMessage
            && lhs.lastMutationMessage == rhs.lastMutationMessage
    }

    var body: some View {
        if let error = errorMessage {
            DetailFeedbackToast(
                message: error,
                systemImage: "exclamationmark.triangle.fill",
                color: .red
            )
        } else if let message = lastMutationMessage {
            DetailFeedbackToast(
                message: message,
                systemImage: "checkmark.circle.fill",
                color: .green
            )
        }
    }
}

private struct SkillDetailContentView: View {
    @EnvironmentObject private var store: SkillStore
    let skill: SkillRecord
    let sessionUsage: LocalSessionSkillUsageRow?

    var body: some View {
            let selectedFindingGroups = FindingDisplayModel.issueGroups(
                findings: store.selectedDisplayFindings,
                severityFilter: FindingDisplayModel.allFilterValue,
                ruleFilter: FindingDisplayModel.allFilterValue
            )
            HeaderView(
                skill: skill,
                adoptingAgentSummary: store.adoptingAgentSummary(for: skill),
                sessionUsage: sessionUsage,
                issueCount: selectedFindingGroups.count + store.selectedConflicts.count,
                isWriting: store.isWriting,
                adapterCapability: store.adapterCapabilities.first { $0.agent == skill.agent },
                onSelectSection: { section in
                    store.selectedDetailSection = section
                },
                onToggle: { on in
                    Task { await store.toggleSelectedSkill(on: on) }
                }
            )

        DetailSectionSwitcher(selection: $store.selectedDetailSection)

        switch store.selectedDetailSection {
        case .agentWorkspace, .lineup, .agentProfile, .taskCockpit, .skillManager, .guidedCleanup, .observability:
            EmptyView()
        case .overview:
            VStack(alignment: .leading, spacing: 16) {
                SkillSummaryCard(
                    skill: skill,
                    detail: store.selectedSkillDetail,
                    scriptPreview: store.scriptExecutionPreview(for: skill),
                    isLoading: store.isLoadingDetail
                )

                if DisplayText.isToolGlobal(skill) {
                    ToolGlobalPreviewCard(skill: skill)
                }
            }
        case .cleanup, .findings, .conflicts:
            FindingsSection(
                skill: skill,
                findings: store.selectedDisplayFindings,
                conflicts: store.selectedConflicts,
                selectedSkillID: skill.id,
                currentAgentSkillIDs: Set(store.skills.filter { $0.agent == skill.agent }.map(\.id))
            )
        case .history:
            HistorySection(
                events: store.selectedSkillEvents,
                isLoading: store.isLoadingSelectedSkillEvents
            )
        case .analysis:
            AnalysisSection(
                skill: skill,
                llmStatus: store.llmStatus,
                qualityScore: { skill in store.skillQualityScore(for: skill) },
                isScoringQuality: { skill in store.isScoringSkillQuality(for: skill) },
                qualityPromptPreview: { skill in store.skillQualityPromptPreview(for: skill) },
                isPreviewingQualityPrompt: { skill in store.isPreviewingSkillQualityPrompt(for: skill) },
                isSendingQualityPrompt: { skill in store.isSendingSkillQualityPrompt(for: skill) },
                qualityPromptSendResult: { skill in store.skillQualityPromptSendResult(for: skill) },
                canSendQualityPrompt: { skill in store.canSendSkillQualityPrompt(for: skill) },
                taskCockpitText: $store.taskCockpitText,
                taskCockpitInput: store.selectedTaskCockpitInput,
                taskCockpitResult: store.taskCockpitResult,
                isBuildingTaskCockpit: store.isBuildingTaskCockpit,
                taskReadinessText: $store.taskReadinessText,
                taskReadinessResult: { skill in store.taskReadiness(for: skill) },
                isCheckingTaskReadiness: { skill in store.isCheckingTaskReadiness(for: skill) },
                taskReadinessPromptPreview: { skill in store.taskReadinessPromptPreview(for: skill) },
                isPreviewingTaskReadinessPrompt: { skill in store.isPreviewingTaskReadinessPrompt(for: skill) },
                isSendingTaskReadinessPrompt: { skill in store.isSendingTaskReadinessPrompt(for: skill) },
                taskReadinessPromptSendResult: { skill in store.taskReadinessPromptSendResult(for: skill) },
                canSendTaskReadinessPrompt: { skill in store.canSendTaskReadinessPrompt(for: skill) },
                routingConfidenceText: $store.routingConfidenceText,
                routingConfidenceResult: { skill in store.routingConfidence(for: skill) },
                isRankingRoutingConfidence: { skill in store.isRankingRoutingConfidence(for: skill) },
                routingConfidencePromptPreview: { skill in store.routingConfidencePromptPreview(for: skill) },
                isPreviewingRoutingConfidencePrompt: { skill in store.isPreviewingRoutingConfidencePrompt(for: skill) },
                isSendingRoutingConfidencePrompt: { skill in store.isSendingRoutingConfidencePrompt(for: skill) },
                routingConfidencePromptSendResult: { skill in store.routingConfidencePromptSendResult(for: skill) },
                canSendRoutingConfidencePrompt: { skill in store.canSendRoutingConfidencePrompt(for: skill) },
                crossAgentReadinessText: $store.crossAgentReadinessText,
                crossAgentReadinessInput: store.selectedCrossAgentReadinessInput,
                crossAgentReadinessResult: store.crossAgentReadinessResult,
                isComparingCrossAgentReadiness: store.isComparingCrossAgentReadiness,
                routingAccuracyDashboard: store.routingAccuracyDashboard,
                isLoadingRoutingAccuracyDashboard: store.isLoadingRoutingAccuracyDashboard,
                staleDriftDetection: store.staleDriftDetection,
                isDetectingStaleDrift: store.isDetectingStaleDrift,
                knowledgeSearchText: $store.knowledgeSearchText,
                knowledgeSearchResult: store.knowledgeSearchResult,
                isSearchingKnowledge: store.isSearchingKnowledge,
                localSkillMapResult: store.localSkillMapResult,
                isBuildingLocalSkillMap: store.isBuildingLocalSkillMap,
                skillLifecycleTimelineResult: store.skillLifecycleTimelineResult,
                isLoadingSkillLifecycleTimeline: store.isLoadingSkillLifecycleTimeline,
                providerObservabilityResult: store.providerObservabilityResult,
                isLoadingProviderObservability: store.isLoadingProviderObservability,
                similarSkillGroupingResult: store.similarSkillGroupingResult,
                isGroupingSimilarSkills: store.isGroupingSimilarSkills,
                capabilityTaxonomyResult: store.capabilityTaxonomyResult,
                isBuildingCapabilityTaxonomy: store.isBuildingCapabilityTaxonomy,
                workspaceReadinessResult: store.workspaceReadinessResult,
                isCheckingWorkspaceReadiness: store.isCheckingWorkspaceReadiness,
                remediationPlanResult: store.remediationPlanResult,
                isPlanningRemediation: store.isPlanningRemediation,
                remediationPreviewDraftsResult: store.remediationPreviewDraftsResult,
                isPreviewingRemediationDrafts: store.isPreviewingRemediationDrafts,
                remediationImpactPreviewResult: store.remediationImpactPreviewResult,
                isPreviewingRemediationImpact: store.isPreviewingRemediationImpact,
                remediationBatchReviewResult: store.remediationBatchReviewResult,
                isReviewingRemediationBatch: store.isReviewingRemediationBatch,
                remediationHistoryResult: store.remediationHistoryResult,
                remediationHistoryRecordResult: store.remediationHistoryRecordResult,
                isLoadingRemediationHistory: store.isLoadingRemediationHistory,
                isRecordingRemediationHistory: store.isRecordingRemediationHistory,
                guidedCleanupFlowResult: store.guidedCleanupFlowResult,
                guidedCleanupRecordResult: store.guidedCleanupRecordResult,
                isPlanningGuidedCleanupFlow: store.isPlanningGuidedCleanupFlow,
                isRecordingGuidedCleanupStep: store.isRecordingGuidedCleanupStep,
                onScoreQuality: {
                Task {
                    await store.scoreSelectedSkillQuality()
                }
            },
            onPreviewQualityPrompt: {
                Task {
                    await store.previewPromptForSelectedSkillQuality()
                }
            },
            onSendQualityPrompt: {
                Task {
                    await store.confirmPromptForSelectedSkillQuality()
                }
            },
            onBuildTaskCockpit: {
                Task {
                    await store.buildTaskCockpit()
                }
            },
            onCheckTaskReadiness: {
                Task {
                    await store.checkSelectedTaskReadiness()
                }
            },
            onPreviewTaskReadinessPrompt: {
                Task {
                    await store.previewPromptForSelectedTaskReadiness()
                }
            },
            onSendTaskReadinessPrompt: {
                Task {
                    await store.confirmPromptForSelectedTaskReadiness()
                }
            },
            onRankRoutingConfidence: {
                Task {
                    await store.rankSelectedSkillRoutes()
                }
            },
            onPreviewRoutingConfidencePrompt: {
                Task {
                    await store.previewPromptForSelectedRoutingConfidence()
                }
            },
            onSendRoutingConfidencePrompt: {
                Task {
                    await store.confirmPromptForSelectedRoutingConfidence()
                }
            },
            onCompareCrossAgentReadiness: {
                Task {
                    await store.compareCrossAgentReadiness()
                }
            },
            onLoadRoutingAccuracyDashboard: {
                Task {
                    await store.loadRoutingAccuracyDashboard()
                }
            },
            onDetectStaleDrift: {
                Task {
                    await store.detectStaleDrift()
                }
            },
            onSearchKnowledge: {
                Task {
                    await store.searchKnowledge()
                }
            },
            onBuildLocalSkillMap: {
                Task {
                    await store.buildLocalSkillMap()
                }
            },
            onLoadSkillLifecycleTimeline: {
                Task {
                    await store.loadSkillLifecycleTimeline()
                }
            },
            onLoadProviderObservability: {
                Task {
                    await store.loadProviderObservability()
                }
            },
            onGroupSimilarSkills: {
                Task {
                    await store.groupSimilarSkills()
                }
            },
            onBuildCapabilityTaxonomy: {
                Task {
                    await store.buildCapabilityTaxonomy()
                }
            },
            onCheckWorkspaceReadiness: {
                Task {
                    await store.checkWorkspaceReadiness()
                }
            },
            onPlanRemediation: {
                Task {
                    await store.planRemediation()
                }
            },
            onPreviewRemediationDrafts: {
                Task {
                    await store.previewRemediationDrafts()
                }
            },
            onPreviewRemediationImpact: {
                Task {
                    await store.previewRemediationImpact()
                }
            },
            onReviewRemediationBatch: { options in
                Task {
                    await store.reviewRemediationBatch(options: options)
                }
            },
            onLoadRemediationHistory: {
                Task {
                    await store.loadRemediationHistory()
                }
            },
            onRecordRemediationHistory: {
                Task {
                    await store.recordRemediationHistory()
                }
            },
            onPlanGuidedCleanupFlow: {
                Task {
                    await store.planGuidedCleanupFlow()
                }
            },
            onRecordGuidedCleanupStep: { step in
                Task {
                    await store.recordGuidedCleanupStep(step)
                }
            },
            taskBenchmarkText: $store.taskBenchmarkText,
            taskBenchmarkInput: store.selectedTaskBenchmarkInput,
            taskBenchmarkList: store.taskBenchmarkList,
            taskBenchmarkEvaluation: store.taskBenchmarkEvaluation,
            taskBenchmarkDeleteResult: store.taskBenchmarkDeleteResult,
            routingRegressionBaseline: store.routingRegressionBaseline,
            routingRegressionDetection: store.routingRegressionDetection,
            isLoadingTaskBenchmarks: store.isLoadingTaskBenchmarks,
            isSavingTaskBenchmark: store.isSavingTaskBenchmark,
            isEvaluatingTaskBenchmarks: store.isEvaluatingTaskBenchmarks,
            isSavingRoutingBaseline: store.isSavingRoutingBaseline,
            isDetectingRoutingRegression: store.isDetectingRoutingRegression,
            isDeletingTaskBenchmark: { benchmark in store.isDeletingTaskBenchmark(benchmark) },
            onLoadTaskBenchmarks: {
                Task {
                    await store.loadTaskBenchmarks()
                }
            },
            onSaveTaskBenchmark: {
                Task {
                    await store.saveSelectedTaskBenchmark()
                }
            },
            onEvaluateTaskBenchmarks: {
                Task {
                    await store.evaluateTaskBenchmarks()
                }
            },
            onSaveRoutingBaseline: {
                Task {
                    await store.saveRoutingBaseline()
                }
            },
            onDetectRoutingRegression: {
                Task {
                    await store.detectRoutingRegression()
                }
            },
            onDeleteTaskBenchmark: { benchmark in
                Task {
                    await store.deleteTaskBenchmark(benchmark)
                }
            },
            traceImportText: $store.traceImportText,
            traceImportTitle: $store.traceImportTitle,
            traceImportTask: $store.traceImportTask,
            traceImportExpectedSkills: $store.traceImportExpectedSkills,
            traceImportList: store.traceImportList,
            traceImportResult: store.traceImportResult,
            traceImportDeleteResult: store.traceImportDeleteResult,
            latestTraceImportRecord: store.latestTraceImportRecord,
            isLoadingTraceImports: store.isLoadingTraceImports,
            isImportingTrace: store.isImportingTrace,
            isDeletingTraceImport: { record in store.isDeletingTraceImport(record) },
            agentSessionSkillReviewTranscript: $store.agentSessionSkillReviewTranscript,
            agentSessionSkillReviewTask: $store.agentSessionSkillReviewTask,
            agentSessionSkillReviewExpectedSkills: $store.agentSessionSkillReviewExpectedSkills,
            localSessionPreviewRoots: $store.localSessionPreviewRoots,
            agentSessionSkillReviewList: store.agentSessionSkillReviewList,
            agentSessionSkillReviewResult: store.agentSessionSkillReviewResult,
            agentSessionSkillReviewDeleteResult: store.agentSessionSkillReviewDeleteResult,
            localSessionPreviewResult: store.localSessionPreviewResult,
            latestAgentSessionSkillReview: store.latestAgentSessionSkillReview,
            isLoadingAgentSessionSkillReviews: store.isLoadingAgentSessionSkillReviews,
            isReviewingAgentSessionSkillUse: store.isReviewingAgentSessionSkillUse,
            isPreviewingLocalSessions: store.isPreviewingLocalSessions,
            isDeletingAgentSessionSkillReview: { record in store.isDeletingAgentSessionSkillReview(record) },
            onLoadAgentSessionSkillReviews: {
                Task {
                    await store.loadAgentSessionSkillReviews()
                }
            },
            onReviewAgentSessionSkillUse: {
                Task {
                    await store.reviewAgentSessionSkillUse()
                }
            },
            onPreviewLocalSessions: {
                Task {
                    await store.previewLocalSessions()
                }
            },
            onDeleteAgentSessionSkillReview: { record in
                Task {
                    await store.deleteAgentSessionSkillReview(record)
                }
            },
            onLoadTraceImports: {
                Task {
                    await store.loadTraceImports()
                }
            },
            onImportTrace: {
                Task {
                    await store.importLocalTrace()
                }
            },
            onDeleteTraceImport: { record in
                Task {
                    await store.deleteTraceImport(record)
                }
            },
            isPreparing: { action in store.isPreparingLLMAction(action) },
            result: { action in store.llmPrepareResult(for: action) },
            promptPreview: { action in store.llmPromptPreview(for: action) },
            isPreviewingPrompt: { action in store.isPreviewingLLMPrompt(for: action) },
            isSendingPrompt: { action in store.isSendingLLMPrompt(for: action) },
            promptSendResult: { action in store.llmPromptSendResult(for: action) },
            canSendPrompt: { action in store.canSendLLMPrompt(for: action) },
            skillAnalysisResult: { kind, scope in store.skillAnalysisPrepareResult(kind: kind, scope: scope) },
            isPreparingSkillAnalysis: { kind, scope in store.isPreparingSkillAnalysis(kind: kind, scope: scope) },
            skillAnalysisPromptPreview: { kind, scope in store.skillAnalysisPromptPreview(kind: kind, scope: scope) },
            isPreviewingSkillAnalysisPrompt: { kind, scope in store.isPreviewingSkillAnalysisPrompt(kind: kind, scope: scope) },
            isSendingSkillAnalysisPrompt: { kind, scope in store.isSendingSkillAnalysisPrompt(kind: kind, scope: scope) },
            skillAnalysisPromptSendResult: { kind, scope in store.skillAnalysisPromptSendResult(kind: kind, scope: scope) },
            canSendSkillAnalysisPrompt: { kind, scope in store.canSendSkillAnalysisPrompt(kind: kind, scope: scope) },
            onPrepareSkillAnalysis: { kind, scope in
                Task {
                    switch scope.key {
                    case LLMSkillAnalysisRequestScope.visible.key:
                        await store.prepareVisibleSkillAnalysis(kind: kind)
                    default:
                        await store.prepareSelectedSkillAnalysis(kind: kind)
                    }
                }
            },
            onPreviewSkillAnalysisPrompt: { kind, scope in
                Task {
                    await store.previewPromptForSkillAnalysis(kind: kind, scope: scope)
                }
            },
            onSendSkillAnalysisPrompt: { kind, scope in
                Task {
                    await store.confirmPromptForSkillAnalysis(kind: kind, scope: scope)
                }
            },
            onPrepare: { action in
                Task {
                    switch action {
                    case .analyze:
                        await store.prepareAnalyzeLLM()
                    case .recommend:
                        await store.prepareRecommendLLM()
                    case .explainConflict:
                        await store.prepareExplainConflictLLM()
                    case .draftFrontmatter:
                        await store.prepareDraftFrontmatterLLM()
                    }
                }
            },
            onPreviewPrompt: { action in
                Task {
                    await store.previewPromptForSelectedLLMAction(action)
                }
            },
            onSendPrompt: { action in
                Task {
                    await store.confirmPromptForSelectedLLMAction(action)
                }
            },
            scriptPreview: store.scriptExecutionPreview(for: skill),
            isPreviewingScript: store.isPreviewingScriptExecution(for: skill),
            onPreviewScript: {
                Task {
                    await store.previewScriptExecutionSafety(for: skill)
                }
                }
            )
        case .metadata:
            SkillDetailCard(
                skill: skill,
                detail: store.selectedSkillDetail,
                adapterCapability: store.adapterCapabilities.first { $0.agent == skill.agent },
                isLoading: store.isLoadingDetail
            )
        }
    }
}

enum SkillProvenanceDisplay {
    static func rootClass(for skill: SkillRecord) -> String {
        switch skill.provenance.rootKind {
        case .toolGlobal:
            return UIStrings.provenanceToolGlobalRoot
        case .native:
            if isNativeOpencodeRoot(skill) {
                return UIStrings.provenanceNativeOpencodeRoot
            }
            return "\(DisplayText.agent(skill.agent)) \(UIStrings.provenanceNativeRoot)"
        case .compatibility:
            if isClaudeCompatibilityRoot(skill) {
                return UIStrings.provenanceClaudeCompatibilityRoot
            }
            if isAgentsCompatibilityRoot(skill) {
                return UIStrings.provenanceAgentsCompatibilityRoot
            }
            return skill.provenance.label
        case .configured:
            return "\(DisplayText.agent(skill.agent)) \(UIStrings.provenanceConfiguredRoot)"
        case .external:
            if skill.agent == "hermes" {
                return UIStrings.provenanceHermesExternalRoot
            }
            return UIStrings.provenanceExternalRoot
        case .readOnly:
            if skill.agent == "hermes" {
                return UIStrings.provenanceHermesHomeProfileRoot
            }
            if skill.agent == "openclaw" {
                if skill.provenance.scopeKind == .project {
                    return UIStrings.provenanceOpenClawWorkspaceRoot
                }
                return UIStrings.provenanceOpenClawReadOnlyRoot
            }
            return "\(DisplayText.agent(skill.agent)) \(UIStrings.provenanceReadOnlyRoot)"
        case .unknown:
            return UIStrings.provenanceUnclassifiedRoot
        }
    }

    static func kind(for skill: SkillRecord) -> String {
        switch skill.provenance.rootKind {
        case .toolGlobal:
            return UIStrings.provenanceToolGlobalKind
        case .native:
            return UIStrings.provenanceNativeKind
        case .compatibility:
            return UIStrings.provenanceCompatibilityKind
        case .configured:
            return UIStrings.provenanceConfiguredKind
        case .external:
            return UIStrings.provenanceExternalKind
        case .readOnly:
            return UIStrings.provenanceReadOnlyKind
        case .unknown:
            return UIStrings.provenanceInferredKind
        }
    }

    private static func isClaudeCompatibilityRoot(_ skill: SkillRecord) -> Bool {
        pathText(for: skill).contains(".claude/skills")
    }

    private static func isAgentsCompatibilityRoot(_ skill: SkillRecord) -> Bool {
        pathText(for: skill).contains(".agents/skills")
    }

    private static func isNativeOpencodeRoot(_ skill: SkillRecord) -> Bool {
        let path = pathText(for: skill)
        return path.contains(".config/opencode/skills") || path.contains(".opencode/skills")
    }

    private static func pathText(for skill: SkillRecord) -> String {
        "\(skill.path)\n\(skill.displayPath)".lowercased()
    }
}
