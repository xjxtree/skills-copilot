import Foundation

struct FilteredSkillListCacheKey: Equatable {
    let dataRevision: Int
    let searchText: String
    let agentFilter: String
    let stateFilter: String
    let scopeFilter: String
    let sortOrder: String
    let sortDirection: String
}

struct FilteredSkillListCache {
    let key: FilteredSkillListCacheKey
    let result: FilteredSkillListResult
}

struct FilteredSkillListResult {
    let skills: [SkillRecord]
    let issueCountsBySkillID: [SkillRecord.ID: Int]

    func issueCount(for skillID: SkillRecord.ID) -> Int {
        issueCountsBySkillID[skillID] ?? 0
    }
}

struct ScopedLocalSessionSummary {
    let rows: [LocalSessionPreviewRow]
    let userMessageCount: Int
    let totalMessageCount: Int
    let toolCallCount: Int
    let skillCallCount: Int
}

struct ScopedLocalSessionSummaryCache {
    let revision: Int
    let summary: ScopedLocalSessionSummary
}

struct AppStartupLoadingState: Equatable {
    let message: String
    let progress: Double

    init(message: String, progress: Double) {
        self.message = message
        self.progress = min(max(progress, 0), 1)
    }
}

@MainActor
final class SkillStore: ObservableObject {
    private static let lastMutationMessageDismissDelayNanoseconds: UInt64 = 3_500_000_000

    @Published private(set) var skills: [SkillRecord] = [] {
        didSet {
            invalidateFilteredSkillListCache()
            invalidateAdoptingAgentSummaryCache()
        }
    }
    @Published private(set) var findings: [RuleFindingRecord] = [] {
        didSet { invalidateFilteredSkillListCache() }
    }
    @Published private(set) var ruleTuning: [RuleTuningRecord] = []
    @Published private(set) var conflicts: [ConflictGroupRecord] = [] {
        didSet { invalidateFilteredSkillListCache() }
    }
    @Published private(set) var cleanupQueue = CleanupQueueResult.emptyFallback()
    @Published private(set) var isLoadingCleanupQueue = false
    @Published private(set) var crossAgentComparisons = CrossAgentComparisonResult.emptyFallback()
    @Published private(set) var isLoadingCrossAgentComparisons = false
    @Published private(set) var localReportExportResult: LocalReportExportResult?
    @Published private(set) var localReportExportHistory: [LocalReportExportHistoryRecord] = []
    @Published private(set) var selectedLocalReportHistoryID: LocalReportExportHistoryRecord.ID?
    @Published private(set) var isExportingLocalReport = false
    @Published private(set) var healthSummary = SkillHealthSummary.empty
    @Published private(set) var agentConfigSnapshots: [ConfigSnapshotRecord] = []
    @Published private(set) var isLoadingAgentConfigSnapshots = false
    @Published private(set) var detailsByID: [SkillRecord.ID: SkillDetailRecord] = [:]
    @Published private(set) var skillEventsByID: [SkillRecord.ID: [SkillEventRecord]] = [:]
    private(set) var adoptingAgentSummaryBySkillID: [SkillRecord.ID: String] = [:]
    @Published private(set) var loadingSkillEventIDs: Set<SkillRecord.ID> = []
    @Published private(set) var status: ServiceStatus?
    @Published private(set) var llmStatus = LLMStatus.disabledFallback()
    @Published private(set) var aiProviderStatus = AIProviderStatus.unavailable()
    @Published private(set) var aiProviderTestResult: AIProviderTestResult?
    @Published private(set) var llmPrepareResults: [LLMAction: LLMPrepareResult] = [:]
    @Published private(set) var preparingLLMActions: Set<LLMAction> = []
    @Published private(set) var skillAnalysisPrepareResults: [String: LLMSkillAnalysisPrepareResult] = [:]
    @Published private(set) var preparingSkillAnalysisKeys: Set<String> = []
    @Published private(set) var skillQualityScores: [SkillRecord.ID: SkillQualityScoreResult] = [:]
    @Published private(set) var scoringSkillQualityIDs: Set<SkillRecord.ID> = []
    @Published private(set) var taskReadinessResult: TaskReadinessResult?
    @Published private(set) var checkingTaskReadinessSkillIDs: Set<SkillRecord.ID> = []
    @Published private(set) var routingConfidenceResult: SkillRoutingConfidenceResult?
    @Published private(set) var rankingRoutingSkillIDs: Set<SkillRecord.ID> = []
    @Published private(set) var crossAgentReadinessResult: CrossAgentReadinessResult?
    @Published private(set) var isComparingCrossAgentReadiness = false
    @Published private(set) var taskBenchmarkList = TaskBenchmarkListResult(benchmarks: [])
    @Published private(set) var taskBenchmarkEvaluation: TaskBenchmarkEvaluationResult?
    @Published private(set) var taskBenchmarkDeleteResult: TaskBenchmarkDeleteResult?
    @Published private(set) var routingRegressionBaseline: RoutingRegressionBaselineResult?
    @Published private(set) var routingRegressionDetection: RoutingRegressionDetectionResult?
    @Published private(set) var routingAccuracyDashboard: RoutingAccuracyDashboard?
    @Published private(set) var staleDriftDetection: StaleDriftDetectionResult?
    @Published private(set) var knowledgeSearchResult: KnowledgeSearchResult?
    @Published private(set) var localSkillMapResult: LocalSkillMapResult?
    @Published private(set) var skillLifecycleTimelineResult: SkillLifecycleTimelineResult?
    @Published private(set) var similarSkillGroupingResult: SimilarSkillGroupingResult?
    @Published private(set) var capabilityTaxonomyResult: CapabilityTaxonomyResult?
    @Published private(set) var workspaceReadinessResult: WorkspaceReadinessResult?
    @Published private(set) var remediationPlanResult: RemediationPlanResult?
    @Published private(set) var remediationPreviewDraftsResult: RemediationPreviewDraftsResult?
    @Published private(set) var remediationImpactPreviewResult: RemediationImpactPreviewResult?
    @Published private(set) var remediationBatchReviewResult: RemediationBatchReviewResult?
    @Published private(set) var remediationHistoryResult: RemediationHistoryResult?
    @Published private(set) var remediationHistoryRecordResult: RemediationHistoryRecordResult?
    @Published private(set) var guidedCleanupFlowResult: GuidedCleanupFlowResult?
    @Published private(set) var guidedCleanupRecordResult: GuidedCleanupRecordStepResult?
    @Published private(set) var traceImportList = AgentTraceImportListResult(imports: [])
    @Published private(set) var traceImportResult: AgentTraceImportResult?
    @Published private(set) var traceImportDeleteResult: AgentTraceImportDeleteResult?
    @Published private(set) var agentSessionSkillReviewList = AgentSessionSkillReviewListResult(reviews: [])
    @Published private(set) var agentSessionSkillReviewResult: AgentSessionSkillReviewResult?
    @Published private(set) var agentSessionSkillReviewDeleteResult: AgentSessionSkillReviewDeleteResult?
    @Published private(set) var localSessionPreviewResult = LocalSessionPreviewResult() {
        didSet { invalidateScopedLocalSessionSummaryCache() }
    }
    @Published private(set) var mcpServerPreviewResult = McpServerPreviewResult()
    @Published private(set) var isLoadingTaskBenchmarks = false
    @Published private(set) var isSavingTaskBenchmark = false
    @Published private(set) var isEvaluatingTaskBenchmarks = false
    @Published private(set) var isSavingRoutingBaseline = false
    @Published private(set) var isDetectingRoutingRegression = false
    @Published private(set) var isLoadingRoutingAccuracyDashboard = false
    @Published private(set) var isDetectingStaleDrift = false
    @Published private(set) var isSearchingKnowledge = false
    @Published private(set) var isBuildingLocalSkillMap = false
    @Published private(set) var isLoadingSkillLifecycleTimeline = false
    @Published private(set) var isGroupingSimilarSkills = false
    @Published private(set) var isBuildingCapabilityTaxonomy = false
    @Published private(set) var isCheckingWorkspaceReadiness = false
    @Published private(set) var isPlanningRemediation = false
    @Published private(set) var isPreviewingRemediationDrafts = false
    @Published private(set) var isPreviewingRemediationImpact = false
    @Published private(set) var isReviewingRemediationBatch = false
    @Published private(set) var isLoadingRemediationHistory = false
    @Published private(set) var isRecordingRemediationHistory = false
    @Published private(set) var isPlanningGuidedCleanupFlow = false
    @Published private(set) var isRecordingGuidedCleanupStep = false
    @Published private(set) var isLoadingTraceImports = false
    @Published private(set) var isImportingTrace = false
    @Published private(set) var isLoadingAgentSessionSkillReviews = false
    @Published private(set) var isReviewingAgentSessionSkillUse = false
    @Published private(set) var isPreviewingLocalSessions = false
    @Published private(set) var isPreviewingMcpServers = false
    @Published private(set) var deletingTaskBenchmarkIDs: Set<String> = []
    @Published private(set) var deletingTraceImportIDs: Set<String> = []
    @Published private(set) var deletingAgentSessionSkillReviewIDs: Set<String> = []
    @Published private(set) var llmPromptPreviews: [String: LLMPromptPreview] = [:]
    @Published private(set) var previewingLLMPromptKeys: Set<String> = []
    @Published private(set) var sendingLLMPromptKeys: Set<String> = []
    @Published private(set) var llmPromptSendResults: [String: LLMPromptSendResult] = [:]
    @Published private(set) var llmPromptRunList = LLMPromptRunListResult.unavailable()
    @Published private(set) var isLoadingLLMPromptRuns = false
    @Published private(set) var providerObservabilityResult: ProviderObservabilityResult?
    @Published private(set) var isLoadingProviderObservability = false
    @Published private(set) var taskCockpitResult: TaskCockpitResult?
    @Published private(set) var taskCockpitHistory: [TaskCockpitHistoryRecord] = []
    @Published private(set) var selectedTaskCockpitHistoryID: TaskCockpitHistoryRecord.ID?
    @Published private(set) var taskCockpitSelectedAgentIDs: Set<String> = [SkillAgentFilter.claudeCode.rawValue]
    @Published private(set) var isBuildingTaskCockpit = false
    @Published private(set) var taskCockpitOperationState = TaskCockpitOperationState.idle
    @Published private(set) var scriptExecutionPreviews: [SkillRecord.ID: ScriptExecutionPreview] = [:]
    @Published private(set) var previewingScriptExecutionSkillIDs: Set<SkillRecord.ID> = []
    @Published private(set) var batchTogglePreview: BatchTogglePreview?
    @Published private(set) var isPreviewingBatchToggle = false
    @Published private(set) var isApplyingBatchToggle = false
    @Published private(set) var skillManagerTools: [SkillManagerToolRecord] = []
    @Published private(set) var skillManagerSearchResult: SkillManagerSearchRecord?
    @Published private(set) var skillManagerInstalled: SkillManagerInstalledListRecord?
    @Published private(set) var skillManagerMutationPreview: SkillManagerMutationRecord?
    @Published private(set) var skillManagerLocalCreatePreview: SkillManagerLocalCreateRecord?
    @Published private(set) var skillManagerLocalDeletePreview: SkillManagerLocalDeleteRecord?
    @Published private(set) var skillManagerErrorMessage: String?
    @Published private(set) var skillManagerMessage: String?
    @Published private(set) var isLoadingSkillManagerTools = false
    @Published private(set) var isSearchingSkillManager = false
    @Published private(set) var isListingSkillManagerInstalled = false
    @Published private(set) var isPreviewingSkillManagerMutation = false
    @Published private(set) var isApplyingSkillManagerMutation = false
    @Published var skillManagerSearchQuery = "" {
        didSet { skillManagerSearchResult = nil }
    }
    @Published var skillManagerOwner = "" {
        didSet { skillManagerSearchResult = nil }
    }
    @Published var skillManagerSource = "" {
        didSet { clearSkillManagerWritePreviews() }
    }
    @Published var skillManagerSkillName = "" {
        didSet { clearSkillManagerWritePreviews() }
    }
    @Published var skillManagerInstallSkillName = "" {
        didSet { clearSkillManagerWritePreviews() }
    }
    @Published var skillManagerRemoveSkillName = "" {
        didSet { clearSkillManagerWritePreviews() }
    }
    @Published var skillManagerLocalSkillName = "" {
        didSet { clearSkillManagerWritePreviews() }
    }
    @Published var skillManagerNetworkAllowed = false {
        didSet { clearSkillManagerWritePreviews() }
    }
    @Published var skillManagerScope: SkillManagerScope = .project {
        didSet { clearSkillManagerWritePreviews() }
    }
    @Published var skillManagerDistribution: SkillManagerDistribution = .symlink {
        didSet { clearSkillManagerWritePreviews() }
    }
    @Published var skillManagerSelectedAgentIDs: Set<String> = Set(SkillManagerAgent.defaultTargets.map(\.rawValue)) {
        didSet { clearSkillManagerWritePreviews() }
    }
    @Published private(set) var projectContextState: ProjectContextState? {
        didSet { invalidateScopedLocalSessionSummaryCache() }
    }
    @Published private(set) var startupLoadingState: AppStartupLoadingState? = AppStartupLoadingState(
        message: UIStrings.startupPreparingLoading,
        progress: 0.02
    )
    @Published private(set) var hasCompletedStartupLoad = false
    @Published private(set) var isRunningStartupLoad = false
    @Published private(set) var isLoading = false
    @Published private(set) var isLoadingDetail = false
    @Published private(set) var isScanning = false
    @Published private(set) var isWriting = false
    @Published private(set) var isProjectUpdating = false
    @Published private(set) var isLoadingSettings = false
    @Published private(set) var isSavingSettings = false
    @Published private(set) var isLoadingAIProvider = false
    @Published private(set) var isSavingAIProvider = false
    @Published private(set) var isTestingAIProvider = false
    @Published private(set) var lastMutationMessage: String? {
        didSet { scheduleLastMutationMessageDismissal() }
    }
    @Published private(set) var refreshStatusMessage = UIStrings.refreshIdle
    @Published private(set) var watcherStatusMessage = UIStrings.refreshWatcherManual
    @Published private(set) var refreshLogEntries: [RefreshLogEntry] = []
    @Published private(set) var lastScanActivity: RefreshActivity?
    @Published private(set) var canRetryLastRefresh = false
    @Published private(set) var claudeSettings: ConfigDocumentRecord?
    @Published private(set) var currentAgentConfigDocuments: [ConfigDocumentRecord] = []
    @Published private(set) var isLoadingAgentConfigDocuments = false
    @Published private(set) var settingsMessage: String?
    @Published private(set) var settingsErrorMessage: String?
    @Published private(set) var aiProviderMessage: String?
    @Published private(set) var aiProviderErrorMessage: String?
    @Published var selectedSidebarSelection: SidebarSelection? {
        didSet {
            guard oldValue != selectedSidebarSelection else { return }
            handleSidebarSelectionChanged()
        }
    }
    @Published var selectedSkillID: SkillRecord.ID? {
        didSet {
            guard oldValue != selectedSkillID else { return }
            clearLocalReportExportState()
            synchronizeSidebarSelectionWithSelectedSkill()
        }
    }
    @Published var selectedDetailSection: DetailSection = .overview
    @Published var sidebarContentMode: SidebarContentMode = .skills {
        didSet {
            guard oldValue != sidebarContentMode else { return }
            handleSidebarContentModeChanged()
        }
    }
    @Published var configScopeFilter: AgentConfigScopeFilter = .all {
        didSet {
            guard oldValue != configScopeFilter else { return }
            normalizeConfigSelection()
        }
    }
    @Published var configSidebarSearchText = "" {
        didSet {
            guard oldValue != configSidebarSearchText else { return }
            guard sidebarContentMode == .config else { return }
            normalizeConfigSelection()
        }
    }
    @Published var searchText = "" {
        didSet {
            guard oldValue != searchText else { return }
            clearLocalReportExportState()
            handleListCriteriaChanged()
        }
    }
    @Published var agentFilter: SkillAgentFilter = .claudeCode {
        didSet {
            guard oldValue != agentFilter else { return }
            clearLocalReportExportState()
            handleListCriteriaChanged()
            routingAccuracyDashboard = nil
            staleDriftDetection = nil
            knowledgeSearchResult = nil
            localSkillMapResult = nil
            skillLifecycleTimelineResult = nil
            clearTaskCockpitTransientState()
            resetTaskCockpitAgentSelectionToSidebarDefault(clearResult: false)
            similarSkillGroupingResult = nil
            capabilityTaxonomyResult = nil
            workspaceReadinessResult = nil
            remediationPlanResult = nil
            remediationPreviewDraftsResult = nil
            remediationImpactPreviewResult = nil
            remediationBatchReviewResult = nil
            remediationHistoryResult = nil
            remediationHistoryRecordResult = nil
            guidedCleanupFlowResult = nil
            guidedCleanupRecordResult = nil
            agentSessionSkillReviewResult = nil
            agentSessionSkillReviewDeleteResult = nil
            agentSessionSkillReviewList = AgentSessionSkillReviewListResult(reviews: [])
            localSessionPreviewResult = LocalSessionPreviewResult()
            loadedLocalSessionPreviewRequestKey = nil
            activeLocalSessionPreviewRequestKey = nil
            selectedLocalSessionID = nil
            mcpServerPreviewResult = McpServerPreviewResult()
            if sidebarContentMode == .config {
                selectedSidebarSelection = .configOverview
            }
            scheduleAgentFilterDependentLoads()
        }
    }
    @Published var stateFilter: SkillStateFilter = .all {
        didSet {
            guard oldValue != stateFilter else { return }
            clearLocalReportExportState()
            handleListCriteriaChanged()
        }
    }
    @Published var skillScopeFilter: SkillScopeFilter = .all {
        didSet {
            guard oldValue != skillScopeFilter else { return }
            clearLocalReportExportState()
            handleListCriteriaChanged()
        }
    }
    @Published var cleanupKindFilter: CleanupQueueKindFilter = .all
    @Published var cleanupPriorityFilter: CleanupQueuePriorityFilter = .all
    @Published var batchToggleAction: BatchToggleAction = .disable {
        didSet { batchTogglePreview = nil }
    }
    @Published private(set) var isBatchToggleSelectionExplicit = false
    @Published private(set) var batchToggleSelectedSkillIDs: Set<SkillRecord.ID> = [] {
        didSet {
            if oldValue != batchToggleSelectedSkillIDs {
                batchTogglePreview = nil
            }
        }
    }
    @Published var localReportFormat: LocalReportFormat = .markdown {
        didSet { clearLocalReportExportState() }
    }
    @Published var sortOrder: SkillSortOrder = .name {
        didSet {
            guard oldValue != sortOrder else { return }
            clearLocalReportExportState()
            handleListCriteriaChanged()
        }
    }
    @Published var sortDirection: SkillSortDirection = .ascending {
        didSet {
            guard oldValue != sortDirection else { return }
            clearLocalReportExportState()
            handleListCriteriaChanged()
        }
    }
    @Published var taskReadinessText = "" {
        didSet {
            if oldValue != taskReadinessText {
                taskReadinessResult = nil
                if normalizedCrossAgentReadinessText.isEmpty {
                    crossAgentReadinessResult = nil
                }
                if normalizedTaskCockpitText.isEmpty {
                    clearTaskCockpitTransientState()
                }
            }
        }
    }
    @Published var routingConfidenceText = "" {
        didSet {
            if oldValue != routingConfidenceText {
                routingConfidenceResult = nil
                if normalizedCrossAgentReadinessText.isEmpty {
                    crossAgentReadinessResult = nil
                }
                if normalizedTaskCockpitText.isEmpty {
                    clearTaskCockpitTransientState()
                }
            }
        }
    }
    @Published var crossAgentReadinessText = "" {
        didSet {
            if oldValue != crossAgentReadinessText {
                crossAgentReadinessResult = nil
                if normalizedTaskCockpitText.isEmpty {
                    clearTaskCockpitTransientState()
                }
            }
        }
    }
    @Published var taskCockpitText = "" {
        didSet {
            if oldValue != taskCockpitText {
                clearTaskCockpitTransientState()
            }
        }
    }
    @Published var knowledgeSearchText = "" {
        didSet {
            if oldValue != knowledgeSearchText {
                knowledgeSearchResult = nil
            }
        }
    }
    @Published var taskBenchmarkText = ""
    @Published var traceImportText = ""
    @Published var traceImportTitle = ""
    @Published var traceImportTask = ""
    @Published var traceImportExpectedSkills = ""
    @Published var agentSessionSkillReviewTranscript = ""
    @Published var agentSessionSkillReviewTask = ""
    @Published var agentSessionSkillReviewExpectedSkills = ""
    @Published var localSessionPreviewRoots = ""
    @Published var localSessionScopeFilter: LocalSessionScopeFilter = .project {
        didSet {
            guard oldValue != localSessionScopeFilter else { return }
            invalidateScopedLocalSessionSummaryCache()
            normalizeSelectedLocalSession()
        }
    }
    @Published var localSessionSortOrder: LocalSessionSortOrder = .recent {
        didSet {
            guard oldValue != localSessionSortOrder else { return }
            normalizeSelectedLocalSession()
        }
    }
    @Published var localSessionSortDirection: SkillSortDirection = .descending {
        didSet {
            guard oldValue != localSessionSortDirection else { return }
            normalizeSelectedLocalSession()
        }
    }
    @Published var localSessionSearchText = "" {
        didSet {
            guard sidebarContentMode == .sessions else { return }
            normalizeSelectedLocalSession()
        }
    }
    @Published var selectedLocalSessionID: LocalSessionPreviewRow.ID?
    @Published var mcpServerPreviewPaths = ""
    @Published var errorMessage: String? {
        didSet { scheduleErrorMessageDismissal() }
    }

    private let service: ServiceClient
    private var lastRefreshAction: RefreshAction = .reload
    private var llmPreparedSkillID: SkillRecord.ID?
    private var taskReadinessCheckedSkillID: SkillRecord.ID?
    private var routingConfidenceRankedSkillID: SkillRecord.ID?
    private var agentConfigSnapshotLoadGeneration = 0
    private var agentConfigDocumentLoadGeneration = 0
    private var claudeSettingsLoadGeneration = 0
    private var cleanupQueueLoadGeneration = 0
    private var crossAgentComparisonsLoadGeneration = 0
    private var selectedDetailLoadGeneration = 0
    private var loadedAgentConfigSnapshotRequestKey: String?
    private var activeAgentConfigSnapshotRequestKey: String?
    private var loadedAgentConfigDocumentRequestKey: String?
    private var activeAgentConfigDocumentRequestKey: String?
    private var loadedClaudeSettingsRequestKey: String?
    private var activeClaudeSettingsRequestKey: String?
    private var loadedCleanupQueueRequestKey: String?
    private var activeCleanupQueueRequestKey: String?
    private var loadedCrossAgentComparisonsRequestKey: String?
    private var activeCrossAgentComparisonsRequestKey: String?
    private var cleanupQueueCacheByRequestKey: [String: CleanupQueueResult] = [:]
    private var crossAgentComparisonsCacheByRequestKey: [String: CrossAgentComparisonResult] = [:]
    private var localSessionPreviewGeneration = 0
    private var loadedLocalSessionPreviewRequestKey: String?
    private var activeLocalSessionPreviewRequestKey: String?
    private var hasLoadedAIProviderStatus = false
    private var hasLoadedProviderObservability = false
    private var taskCockpitOperationID: UUID?
    private var lastMutationMessageDismissTask: Task<Void, Never>?
    private var errorMessageDismissTask: Task<Void, Never>?
    private var agentFilterLoadTask: Task<Void, Never>?
    private var listCriteriaDetailTask: Task<Void, Never>?
    private var taskCockpitTimeoutTask: Task<Void, Never>?
    private var taskCockpitServiceTask: Task<TaskCockpitResult, Error>?
    private var isSynchronizingSidebarSelection = false
    var filteredSkillListDataRevision = 0
    var filteredSkillListCache: FilteredSkillListCache?
    var isAdoptingAgentSummaryCacheValid = false
    var scopedLocalSessionSummaryRevision = 0
    var scopedLocalSessionSummaryCache: ScopedLocalSessionSummaryCache?
    private let taskCockpitTimeoutSeconds: TimeInterval
    private let taskCockpitHistoryStore: TaskCockpitHistoryStore

    init(
        service: ServiceClient,
        taskCockpitTimeoutSeconds: TimeInterval = 300,
        taskCockpitHistoryStore: TaskCockpitHistoryStore = TaskCockpitHistoryStore()
    ) {
        self.service = service
        self.taskCockpitTimeoutSeconds = max(0.05, taskCockpitTimeoutSeconds)
        self.taskCockpitHistoryStore = taskCockpitHistoryStore
        taskCockpitHistory = taskCockpitHistoryStore.load()
    }

    private func scheduleLastMutationMessageDismissal() {
        lastMutationMessageDismissTask?.cancel()
        guard let message = lastMutationMessage, !message.isEmpty else {
            lastMutationMessageDismissTask = nil
            return
        }

        let delayNanoseconds = Self.lastMutationMessageDismissDelayNanoseconds
        lastMutationMessageDismissTask = Task { [weak self, message, delayNanoseconds] in
            try? await Task.sleep(nanoseconds: delayNanoseconds)
            guard !Task.isCancelled else { return }
            self?.clearLastMutationMessageIfCurrent(message)
        }
    }

    private func clearLastMutationMessageIfCurrent(_ message: String) {
        guard lastMutationMessage == message else { return }
        lastMutationMessage = nil
    }

    private func scheduleErrorMessageDismissal() {
        errorMessageDismissTask?.cancel()
        guard let message = errorMessage, !message.isEmpty else {
            errorMessageDismissTask = nil
            return
        }

        let delayNanoseconds = Self.lastMutationMessageDismissDelayNanoseconds
        errorMessageDismissTask = Task { [weak self, message, delayNanoseconds] in
            try? await Task.sleep(nanoseconds: delayNanoseconds)
            guard !Task.isCancelled else { return }
            self?.clearErrorMessageIfCurrent(message)
        }
    }

    private func clearErrorMessageIfCurrent(_ message: String) {
        guard errorMessage == message else { return }
        errorMessage = nil
    }

    private func scheduleAgentFilterDependentLoads() {
        agentFilterLoadTask?.cancel()
        let requestedAgentFilter = agentFilter
        agentFilterLoadTask = Task { @MainActor [weak self, requestedAgentFilter] in
            guard let self else { return }
            await self.loadAgentConfigSnapshotsIfNeeded()
            guard !Task.isCancelled, self.agentFilter == requestedAgentFilter else { return }
            await self.loadCleanupQueueIfNeeded()
            guard !Task.isCancelled, self.agentFilter == requestedAgentFilter else { return }
            await self.loadCrossAgentComparisonsIfNeeded()
        }
    }

    private func invalidateRuntimeAnalysisCaches() {
        cleanupQueueLoadGeneration &+= 1
        crossAgentComparisonsLoadGeneration &+= 1
        loadedCleanupQueueRequestKey = nil
        activeCleanupQueueRequestKey = nil
        loadedCrossAgentComparisonsRequestKey = nil
        activeCrossAgentComparisonsRequestKey = nil
        cleanupQueueCacheByRequestKey.removeAll()
        crossAgentComparisonsCacheByRequestKey.removeAll()
        isLoadingCleanupQueue = false
        isLoadingCrossAgentComparisons = false
    }

    func invalidateFilteredSkillListCache() {
        filteredSkillListDataRevision &+= 1
        filteredSkillListCache = nil
    }

    func invalidateAdoptingAgentSummaryCache() {
        isAdoptingAgentSummaryCacheValid = false
        adoptingAgentSummaryBySkillID = [:]
    }

    func ensureAdoptingAgentSummaryCache() {
        guard !isAdoptingAgentSummaryCacheValid else { return }
        adoptingAgentSummaryBySkillID = SkillListModel.adoptingAgentSummaryBySkillID(for: skills)
        isAdoptingAgentSummaryCacheValid = true
    }

    func invalidateDetailCaches(for instanceIDs: some Sequence<SkillRecord.ID>) {
        for instanceID in instanceIDs {
            detailsByID.removeValue(forKey: instanceID)
            skillEventsByID.removeValue(forKey: instanceID)
        }
    }

    func pruneDetailCaches(to currentSkillIDs: Set<SkillRecord.ID>) {
        detailsByID = detailsByID.filter { currentSkillIDs.contains($0.key) }
        skillEventsByID = skillEventsByID.filter { currentSkillIDs.contains($0.key) }
    }

    func invalidateScopedLocalSessionSummaryCache() {
        scopedLocalSessionSummaryRevision &+= 1
        scopedLocalSessionSummaryCache = nil
    }

    var selectedLocalSession: LocalSessionPreviewRow? {
        if let selectedLocalSessionID,
           let row = localSessionPreviewResult.sessionRows.first(where: { $0.id == selectedLocalSessionID }) {
            return row
        }
        return nil
    }

    var filteredLocalSessionRows: [LocalSessionPreviewRow] {
        let query = localSessionSearchText.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        let scopedRows = scopedLocalSessionSummary.rows
        guard !query.isEmpty else {
            return sortedLocalSessionRows(scopedRows)
        }
        return sortedLocalSessionRows(scopedRows.filter { row in
            row.title.lowercased().contains(query)
                || row.redactedPath.lowercased().contains(query)
                || (row.projectRoot?.lowercased().contains(query) ?? false)
        })
    }

    private func sortedLocalSessionRows(_ rows: [LocalSessionPreviewRow]) -> [LocalSessionPreviewRow] {
        rows.enumerated().sorted { lhsPair, rhsPair in
            let lhs = lhsPair.element
            let rhs = rhsPair.element
            switch localSessionSortOrder {
            case .recent:
                let result = localSessionSortTimestamp(lhs).compare(localSessionSortTimestamp(rhs))
                if result != .orderedSame {
                    return localSessionSortDirection == .ascending ? result == .orderedAscending : result == .orderedDescending
                }
            case .title:
                let result = lhs.title.localizedStandardCompare(rhs.title)
                if result != .orderedSame {
                    return localSessionSortDirection == .ascending ? result == .orderedAscending : result == .orderedDescending
                }
            }

            return lhsPair.offset < rhsPair.offset
        }
        .map(\.element)
    }

    private func localSessionSortTimestamp(_ row: LocalSessionPreviewRow) -> NSNumber {
        NSNumber(value: row.endedAt ?? row.startedAt ?? 0)
    }

    func configDocumentMatchesSidebarQuery(_ document: ConfigDocumentRecord) -> Bool {
        configSidebarQueryMatches([
            document.agent,
            document.scope,
            document.target,
            document.format,
            document.exists ? UIStrings.existingFile : UIStrings.willCreateFile
        ])
    }

    func configSnapshotMatchesSidebarQuery(_ snapshot: ConfigSnapshotRecord) -> Bool {
        configSidebarQueryMatches([
            snapshot.agent,
            snapshot.scope,
            snapshot.target,
            snapshot.reason,
            DisplayText.timestamp(snapshot.createdAt)
        ])
    }

    var scopedLocalSessionRows: [LocalSessionPreviewRow] {
        scopedLocalSessionSummary.rows
    }

    var scopedLocalSessionUserMessageCount: Int {
        scopedLocalSessionSummary.userMessageCount
    }

    var scopedLocalSessionTotalMessageCount: Int {
        scopedLocalSessionSummary.totalMessageCount
    }

    var scopedLocalSessionToolCallCount: Int {
        scopedLocalSessionSummary.toolCallCount
    }

    var scopedLocalSessionSkillCallCount: Int {
        scopedLocalSessionSummary.skillCallCount
    }

    var scopedLocalSessionSummary: ScopedLocalSessionSummary {
        if let scopedLocalSessionSummaryCache,
           scopedLocalSessionSummaryCache.revision == scopedLocalSessionSummaryRevision {
            return scopedLocalSessionSummaryCache.summary
        }

        var rows: [LocalSessionPreviewRow] = []
        rows.reserveCapacity(localSessionPreviewResult.sessionRows.count)
        var userMessageCount = 0
        var totalMessageCount = 0
        var toolCallCount = 0
        var skillCallCount = 0
        for row in localSessionPreviewResult.sessionRows where localSessionMatchesCurrentScope(row) {
            rows.append(row)
            userMessageCount += row.userMessageCount
            totalMessageCount += row.totalMessageCount
            toolCallCount += row.toolCallCount
            skillCallCount += row.skillCallCount
        }

        let summary = ScopedLocalSessionSummary(
            rows: rows,
            userMessageCount: userMessageCount,
            totalMessageCount: totalMessageCount,
            toolCallCount: toolCallCount,
            skillCallCount: skillCallCount
        )
        scopedLocalSessionSummaryCache = ScopedLocalSessionSummaryCache(
            revision: scopedLocalSessionSummaryRevision,
            summary: summary
        )
        return summary
    }

    private func localSessionMatchesCurrentScope(_ row: LocalSessionPreviewRow) -> Bool {
        switch localSessionScopeFilter {
        case .all:
            return true
        case .project:
            let selectedRoot = activeProjectContext?.rootPath.trimmingCharacters(in: .whitespacesAndNewlines)
            let rowRoot = row.projectRoot?.trimmingCharacters(in: .whitespacesAndNewlines)
            if let rowRoot, !rowRoot.isEmpty {
                if rowRoot == "<project-root>" || rowRoot.hasPrefix("<project-root>/") {
                    return true
                }
                guard let selectedRoot, !selectedRoot.isEmpty else { return true }
                return rowRoot == selectedRoot || rowRoot.hasPrefix(selectedRoot + "/")
            }

            return row.scope.lowercased().contains("project")
        }
    }

    private func configSidebarQueryMatches(_ values: [String]) -> Bool {
        let query = configSidebarSearchText.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard !query.isEmpty else { return true }
        return values.contains { value in
            value.lowercased().contains(query)
        }
    }

    var selectedConfigSnapshot: ConfigSnapshotRecord? {
        guard case let .configSnapshot(id) = selectedSidebarSelection else { return nil }
        return agentConfigSnapshots.first { $0.id == id }
    }

    var selectedConfigDocument: ConfigDocumentRecord? {
        guard case let .configDocument(target) = selectedSidebarSelection else { return nil }
        return currentAgentConfigDocuments.first { $0.target == target }
    }

    var visibleConfigDocuments: [ConfigDocumentRecord] {
        currentAgentConfigDocuments
            .filter { document in
                document.agent == agentFilter.rawValue
                    && configScopeFilter.includes(document)
                    && configDocumentMatchesSidebarQuery(document)
            }
            .sorted { lhs, rhs in
                let lhsProject = lhs.scope.lowercased().contains("project")
                let rhsProject = rhs.scope.lowercased().contains("project")
                if lhsProject != rhsProject {
                    return lhsProject
                }
                return lhs.target.localizedStandardCompare(rhs.target) == .orderedAscending
            }
    }

    func selectLocalSession(_ session: LocalSessionPreviewRow) {
        guard selectedLocalSessionID != session.id || selectedSidebarSelection != .session(session.id) else {
            return
        }
        selectedLocalSessionID = session.id
        setSidebarSelection(.session(session.id))
        selectedDetailSection = .overview
    }

    func selectConfigDocument(_ document: ConfigDocumentRecord) {
        guard selectedSidebarSelection != .configDocument(document.target) else { return }
        selectedSidebarSelection = .configDocument(document.target)
    }

    func enterConfigMode() {
        sidebarContentMode = .config
        selectDefaultConfigDocumentOrOverview()
    }

    func selectConfigSnapshot(_ snapshot: ConfigSnapshotRecord) {
        guard selectedSidebarSelection != .configSnapshot(snapshot.id) else { return }
        selectedSidebarSelection = .configSnapshot(snapshot.id)
    }

    func selectLocalReportHistoryRecord(_ record: LocalReportExportHistoryRecord) {
        selectedLocalReportHistoryID = record.id
        localReportExportResult = record.result
    }

    func selectTaskCockpitHistoryRecord(_ record: TaskCockpitHistoryRecord) {
        taskCockpitText = record.taskText
        setTaskCockpitAgentSelection(record.agentIDs, clearResult: false)
        taskCockpitResult = record.result
        taskCockpitOperationState = record.operationState
        selectedTaskCockpitHistoryID = record.id
    }

    var taskCockpitAgentOptions: [TaskCockpitAgentOption] {
        SkillAgentFilter.managementCases.map { filter in
            TaskCockpitAgentOption(
                id: filter.rawValue,
                title: DisplayText.agent(filter.rawValue),
                enabledSkillCount: skills.filter { skill in
                    skill.agent == filter.rawValue
                        && DisplayText.statusKind(skill.state, enabled: skill.enabled) == .enabled
                }.count
            )
        }
    }

    var taskCockpitSelectedAgents: [String] {
        normalizedTaskCockpitAgentIDs(Array(taskCockpitSelectedAgentIDs))
    }

    func ensureTaskCockpitAgentSelection() {
        let normalized = taskCockpitSelectedAgents
        if normalized.isEmpty {
            resetTaskCockpitAgentSelectionToSidebarDefault(clearResult: false)
        } else if Set(normalized) != taskCockpitSelectedAgentIDs {
            taskCockpitSelectedAgentIDs = Set(normalized)
        }
    }

    func toggleTaskCockpitAgentSelection(_ agentID: String) {
        var next = taskCockpitSelectedAgentIDs
        if next.contains(agentID) {
            next.remove(agentID)
        } else {
            next.insert(agentID)
        }
        setTaskCockpitAgentSelection(Array(next), clearResult: true)
    }

    func selectAllTaskCockpitAgents() {
        setTaskCockpitAgentSelection(SkillAgentFilter.managementCases.map(\.rawValue), clearResult: true)
    }

    func localReportScopeSummary(includeSelectedSkill: Bool) -> String {
        var parts = [agentFilter.title]
        if stateFilter != .all {
            parts.append(stateFilter.title)
        }
        let trimmedSearch = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        if !trimmedSearch.isEmpty {
            parts.append(UIStrings.text("localReport.scope.search", "Search filter active"))
        }
        if includeSelectedSkill, let selectedSkill {
            parts.append(selectedSkill.name)
        }
        return String(
            format: UIStrings.text("localReport.scope.agent", "Exports the current local audit scope: %@."),
            parts.joined(separator: " · ")
        )
    }

    func setFindingTriageStatus(_ status: FindingTriageStatus, for triageKeys: [String]) {
        let keys = Array(Set(triageKeys.filter { !$0.isEmpty })).sorted()
        guard !keys.isEmpty else { return }
        Task {
            await setFindingTriageStatus(status, triageKeys: keys)
        }
    }

    func setRuleSeverityOverride(_ severity: String, for ruleId: String) {
        Task {
            await setRuleSeverityOverride(severity, ruleId: ruleId)
        }
    }

    func clearRuleSeverityOverride(for ruleId: String) {
        Task {
            await clearRuleSeverityOverride(ruleId: ruleId)
        }
    }

    func setRuleSuppression(ruleId: String, findingGroupID: String?, scope: RuleTuningScope) {
        Task {
            await setRuleSuppression(ruleId: ruleId, findingGroupID: findingGroupID, scope: scope)
        }
    }

    func clearRuleSuppression(ruleId: String, findingGroupID: String?, scope: RuleTuningScope) {
        Task {
            await clearRuleSuppression(ruleId: ruleId, findingGroupID: findingGroupID, scope: scope)
        }
    }

    func llmPrepareResult(for action: LLMAction) -> LLMPrepareResult? {
        guard llmPreparedSkillID == selectedSkillID else { return nil }
        return llmPrepareResults[action]
    }

    func isPreparingLLMAction(_ action: LLMAction) -> Bool {
        preparingLLMActions.contains(action)
    }

    func skillAnalysisPrepareResult(kind: LLMSkillAnalysisKind, scope: LLMSkillAnalysisRequestScope) -> LLMSkillAnalysisPrepareResult? {
        skillAnalysisPrepareResults[skillAnalysisKey(kind: kind, scope: scope)]
    }

    func isPreparingSkillAnalysis(kind: LLMSkillAnalysisKind, scope: LLMSkillAnalysisRequestScope) -> Bool {
        preparingSkillAnalysisKeys.contains(skillAnalysisKey(kind: kind, scope: scope))
    }

    func llmPromptPreview(for action: LLMAction) -> LLMPromptPreview? {
        guard let skill = selectedSkill else { return nil }
        return llmPromptPreviews[llmPromptActionKey(action: action, skillID: skill.id)]
    }

    func isPreviewingLLMPrompt(for action: LLMAction) -> Bool {
        guard let skill = selectedSkill else { return false }
        return previewingLLMPromptKeys.contains(llmPromptActionKey(action: action, skillID: skill.id))
    }

    func isSendingLLMPrompt(for action: LLMAction) -> Bool {
        guard let skill = selectedSkill else { return false }
        return sendingLLMPromptKeys.contains(llmPromptActionKey(action: action, skillID: skill.id))
    }

    func llmPromptSendResult(for action: LLMAction) -> LLMPromptSendResult? {
        guard let skill = selectedSkill else { return nil }
        return llmPromptSendResults[llmPromptActionKey(action: action, skillID: skill.id)]
    }

    func canSendLLMPrompt(for action: LLMAction) -> Bool {
        guard let preview = llmPromptPreview(for: action) else { return false }
        return canSendLLMPrompt(preview)
    }

    func skillAnalysisPromptPreview(kind: LLMSkillAnalysisKind, scope: LLMSkillAnalysisRequestScope) -> LLMPromptPreview? {
        let instanceIDs = skillAnalysisInstanceIDs(scope: scope)
        guard !instanceIDs.isEmpty else { return nil }
        return llmPromptPreviews[skillAnalysisPromptKey(kind: kind, scope: scope, instanceIDs: instanceIDs)]
    }

    func isPreviewingSkillAnalysisPrompt(kind: LLMSkillAnalysisKind, scope: LLMSkillAnalysisRequestScope) -> Bool {
        let instanceIDs = skillAnalysisInstanceIDs(scope: scope)
        guard !instanceIDs.isEmpty else { return false }
        return previewingLLMPromptKeys.contains(skillAnalysisPromptKey(kind: kind, scope: scope, instanceIDs: instanceIDs))
    }

    func isSendingSkillAnalysisPrompt(kind: LLMSkillAnalysisKind, scope: LLMSkillAnalysisRequestScope) -> Bool {
        let instanceIDs = skillAnalysisInstanceIDs(scope: scope)
        guard !instanceIDs.isEmpty else { return false }
        return sendingLLMPromptKeys.contains(skillAnalysisPromptKey(kind: kind, scope: scope, instanceIDs: instanceIDs))
    }

    func skillAnalysisPromptSendResult(kind: LLMSkillAnalysisKind, scope: LLMSkillAnalysisRequestScope) -> LLMPromptSendResult? {
        let instanceIDs = skillAnalysisInstanceIDs(scope: scope)
        guard !instanceIDs.isEmpty else { return nil }
        return llmPromptSendResults[skillAnalysisPromptKey(kind: kind, scope: scope, instanceIDs: instanceIDs)]
    }

    func canSendSkillAnalysisPrompt(kind: LLMSkillAnalysisKind, scope: LLMSkillAnalysisRequestScope) -> Bool {
        guard let preview = skillAnalysisPromptPreview(kind: kind, scope: scope) else { return false }
        return canSendLLMPrompt(preview)
    }

    func skillQualityScore(for skill: SkillRecord) -> SkillQualityScoreResult? {
        skillQualityScores[skill.id]
    }

    func isScoringSkillQuality(for skill: SkillRecord) -> Bool {
        scoringSkillQualityIDs.contains(skill.id)
    }

    func skillQualityPromptPreview(for skill: SkillRecord) -> LLMPromptPreview? {
        llmPromptPreviews[skillQualityPromptKey(skillID: skill.id)]
    }

    func isPreviewingSkillQualityPrompt(for skill: SkillRecord) -> Bool {
        previewingLLMPromptKeys.contains(skillQualityPromptKey(skillID: skill.id))
    }

    func isSendingSkillQualityPrompt(for skill: SkillRecord) -> Bool {
        sendingLLMPromptKeys.contains(skillQualityPromptKey(skillID: skill.id))
    }

    func skillQualityPromptSendResult(for skill: SkillRecord) -> LLMPromptSendResult? {
        llmPromptSendResults[skillQualityPromptKey(skillID: skill.id)]
    }

    func canSendSkillQualityPrompt(for skill: SkillRecord) -> Bool {
        guard let preview = skillQualityPromptPreview(for: skill) else { return false }
        return canSendLLMPrompt(preview)
    }

    func taskReadiness(for skill: SkillRecord) -> TaskReadinessResult? {
        guard taskReadinessCheckedSkillID == skill.id else { return nil }
        return taskReadinessResult
    }

    func isCheckingTaskReadiness(for skill: SkillRecord) -> Bool {
        checkingTaskReadinessSkillIDs.contains(skill.id)
    }

    func taskReadinessPromptPreview(for skill: SkillRecord) -> LLMPromptPreview? {
        let taskText = normalizedTaskReadinessText
        guard !taskText.isEmpty else { return nil }
        return llmPromptPreviews[taskReadinessPromptKey(skillID: skill.id, taskText: taskText)]
    }

    func isPreviewingTaskReadinessPrompt(for skill: SkillRecord) -> Bool {
        let taskText = normalizedTaskReadinessText
        guard !taskText.isEmpty else { return false }
        return previewingLLMPromptKeys.contains(taskReadinessPromptKey(skillID: skill.id, taskText: taskText))
    }

    func isSendingTaskReadinessPrompt(for skill: SkillRecord) -> Bool {
        let taskText = normalizedTaskReadinessText
        guard !taskText.isEmpty else { return false }
        return sendingLLMPromptKeys.contains(taskReadinessPromptKey(skillID: skill.id, taskText: taskText))
    }

    func taskReadinessPromptSendResult(for skill: SkillRecord) -> LLMPromptSendResult? {
        let taskText = normalizedTaskReadinessText
        guard !taskText.isEmpty else {
            return latestPromptRun(for: skill, requestKind: "task_readiness")?.sendResult
        }
        return llmPromptSendResults[taskReadinessPromptKey(skillID: skill.id, taskText: taskText)]
    }

    func canSendTaskReadinessPrompt(for skill: SkillRecord) -> Bool {
        guard let preview = taskReadinessPromptPreview(for: skill) else { return false }
        return canSendLLMPrompt(preview)
    }

    func routingConfidence(for skill: SkillRecord) -> SkillRoutingConfidenceResult? {
        guard routingConfidenceRankedSkillID == skill.id else { return nil }
        return routingConfidenceResult
    }

    func isRankingRoutingConfidence(for skill: SkillRecord) -> Bool {
        rankingRoutingSkillIDs.contains(skill.id)
    }

    func routingConfidencePromptPreview(for skill: SkillRecord) -> LLMPromptPreview? {
        let taskText = normalizedRoutingConfidenceText
        guard !taskText.isEmpty else { return nil }
        return llmPromptPreviews[routingConfidencePromptKey(skillID: skill.id, taskText: taskText)]
    }

    func isPreviewingRoutingConfidencePrompt(for skill: SkillRecord) -> Bool {
        let taskText = normalizedRoutingConfidenceText
        guard !taskText.isEmpty else { return false }
        return previewingLLMPromptKeys.contains(routingConfidencePromptKey(skillID: skill.id, taskText: taskText))
    }

    func isSendingRoutingConfidencePrompt(for skill: SkillRecord) -> Bool {
        let taskText = normalizedRoutingConfidenceText
        guard !taskText.isEmpty else { return false }
        return sendingLLMPromptKeys.contains(routingConfidencePromptKey(skillID: skill.id, taskText: taskText))
    }

    func routingConfidencePromptSendResult(for skill: SkillRecord) -> LLMPromptSendResult? {
        let taskText = normalizedRoutingConfidenceText
        guard !taskText.isEmpty else {
            return latestPromptRun(for: skill, requestKind: "routing_confidence")?.sendResult
        }
        return llmPromptSendResults[routingConfidencePromptKey(skillID: skill.id, taskText: taskText)]
    }

    func canSendRoutingConfidencePrompt(for skill: SkillRecord) -> Bool {
        guard let preview = routingConfidencePromptPreview(for: skill) else { return false }
        return canSendLLMPrompt(preview)
    }

    func loadAppStartupDataIfNeeded() async {
        guard !hasCompletedStartupLoad, !isRunningStartupLoad else { return }
        isRunningStartupLoad = true
        isLoading = true
        errorMessage = nil
        beginRefresh(.reload, message: UIStrings.startupCatalogLoading)
        setStartupLoading(UIStrings.startupPreparingLoading, progress: 0.04)
        defer {
            startupLoadingState = nil
            hasCompletedStartupLoad = true
            isRunningStartupLoad = false
            isLoading = false
        }

        do {
            setStartupLoading(UIStrings.startupCatalogLoading, progress: 0.16)
            try await refreshCollections()

            setStartupLoading(UIStrings.startupAnalysisLoading, progress: 0.40)
            let startupAgentFilter = agentFilter
            let shouldLoadClaudeSettings = startupAgentFilter == .claudeCode
                && status?.supportedMethods.contains("config.readClaudeSettings") == true
            async let cleanupQueueLoad: Void = loadCleanupQueueIfNeeded()
            async let crossAgentComparisonsLoad: Void = loadCrossAgentComparisonsIfNeeded()
            async let localSessionsLoad: Void = refreshSelectedAgentLocalSessionsIfNeeded()
            async let currentConfigDocumentsLoad: Void = loadCurrentAgentConfigDocumentsIfNeeded(agent: startupAgentFilter.rawValue)
            async let providerObservabilityLoad: Void = loadProviderObservabilityDuringRefresh(force: false)
            if shouldLoadClaudeSettings {
                await loadClaudeSettingsIfNeeded()
            }
            _ = await (
                cleanupQueueLoad,
                crossAgentComparisonsLoad,
                localSessionsLoad,
                currentConfigDocumentsLoad,
                providerObservabilityLoad
            )

            setStartupLoading(UIStrings.startupDetailLoading, progress: 0.90)
            await loadSelectedDetail()

            setStartupLoading(UIStrings.startupReadyLoading, progress: 1.0)
            refreshStatusMessage = UIStrings.refreshReloaded(skills.count, findings.count, sameAgentRuntimeConflictCount)
            appendRefreshLog(level: "info", message: refreshStatusMessage)
            canRetryLastRefresh = false
        } catch {
            handleRefreshFailure(error, action: .reload)
        }
    }

    private func setStartupLoading(_ message: String, progress: Double) {
        startupLoadingState = AppStartupLoadingState(message: message, progress: progress)
    }

    func reload() async {
        guard !isRefreshBusy else { return }
        isLoading = true
        errorMessage = nil
        beginRefresh(.reload, message: UIStrings.refreshReloading)
        defer { isLoading = false }

        do {
            try await refreshCollections()
            async let cleanupQueueLoad: Void = loadCleanupQueue()
            async let crossAgentComparisonsLoad: Void = loadCrossAgentComparisons()
            async let providerObservabilityLoad: Void = loadProviderObservabilityDuringRefresh(force: true)
            _ = await (cleanupQueueLoad, crossAgentComparisonsLoad, providerObservabilityLoad)
            refreshStatusMessage = UIStrings.refreshReloaded(skills.count, findings.count, sameAgentRuntimeConflictCount)
            appendRefreshLog(level: "info", message: refreshStatusMessage)
            canRetryLastRefresh = false
            await loadSelectedDetail()
        } catch {
            handleRefreshFailure(error, action: .reload)
        }
    }

    func scanAll() async {
        await scanAll(allowDuringProjectUpdate: false)
    }

    private func scanAll(allowDuringProjectUpdate: Bool) async {
        guard canStartScan(allowDuringProjectUpdate: allowDuringProjectUpdate) else { return }
        isScanning = true
        errorMessage = nil
        lastMutationMessage = nil
        beginRefresh(.scan, message: UIStrings.refreshScanning)
        defer { isScanning = false }

        do {
            let result = try await service.scanAll()
            pruneDetailCaches(to: Set(result.skills.map(\.id)))
            if let selectedSkillID {
                invalidateDetailCaches(for: [selectedSkillID])
            }
            try await refreshCollections()
            await loadCleanupQueue()
            await loadCrossAgentComparisons()
            lastMutationMessage = UIStrings.scannedSkills(result.scannedCount)
            applyRefreshActivity(result.activity)
            await loadSelectedDetail()
        } catch {
            handleRefreshFailure(error, action: .scan)
        }
    }

    func setProject(rootPath: String, currentCWD: String? = nil, name: String? = nil) async {
        guard !isRefreshBusy else { return }
        isProjectUpdating = true
        errorMessage = nil
        lastMutationMessage = nil
        defer { isProjectUpdating = false }

        do {
            let resolvedName = name ?? URL(fileURLWithPath: rootPath).lastPathComponent
            let state = try await service.setProjectContext(
                rootPath: rootPath,
                currentCWD: currentCWD ?? rootPath,
                name: resolvedName.isEmpty ? nil : resolvedName
            )
            projectContextState = state
            detailsByID.removeAll()

            if let validationMessage = projectValidationMessage {
                errorMessage = UIStrings.projectValidationFailed(validationMessage)
                refreshStatusMessage = UIStrings.projectScanSkippedValidation
                appendRefreshLog(level: "error", message: refreshStatusMessage)
                return
            }

            await scanAll(allowDuringProjectUpdate: true)
            if errorMessage == nil {
                lastMutationMessage = UIStrings.projectSelectedAndScanned(activeProjectContext?.name ?? resolvedName)
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func clearProject() async {
        guard !isRefreshBusy else { return }
        isProjectUpdating = true
        errorMessage = nil
        lastMutationMessage = nil
        defer { isProjectUpdating = false }

        do {
            projectContextState = try await service.clearProjectContext()
            detailsByID.removeAll()
            await scanAll(allowDuringProjectUpdate: true)
            if errorMessage == nil {
                lastMutationMessage = UIStrings.projectClearedAndScanned
            }
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func retryLastRefresh() async {
        switch lastRefreshAction {
        case .reload:
            await reload()
        case .scan:
            await scanAll()
        }
    }

    func toggleSelectedSkill(on: Bool) async {
        guard !isLoading, !isScanning, !isProjectUpdating, !isSavingSettings else {
            errorMessage = UIStrings.operationUnavailableBusy
            lastMutationMessage = nil
            return
        }
        guard let skill = selectedSkill else { return }
        if let disabledReason = toggleDisabledReason(for: skill) {
            errorMessage = disabledReason
            lastMutationMessage = nil
            return
        }

        isWriting = true
        errorMessage = nil
        lastMutationMessage = nil
        defer { isWriting = false }

        do {
            _ = try await service.toggleSkill(instanceID: skill.id, on: on)
            detailsByID.removeValue(forKey: skill.id)
            skillEventsByID.removeValue(forKey: skill.id)
            try await refreshCollections()
            lastMutationMessage = UIStrings.toggledSkill(on: on, name: skill.name, agent: skill.agent)
            recordLocalRefresh(message: UIStrings.refreshAfterWrite)
            await loadSelectedDetail()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func previewVisibleBatchToggle() async {
        let selectedSkills = batchToggleSelectedSkills
        guard !selectedSkills.isEmpty else {
            batchTogglePreview = nil
            return
        }
        guard !isRefreshBusy else {
            errorMessage = UIStrings.operationUnavailableBusy
            return
        }

        isPreviewingBatchToggle = true
        errorMessage = nil
        lastMutationMessage = nil
        defer { isPreviewingBatchToggle = false }

        do {
            batchTogglePreview = try await service.previewBatchSkillToggles(
                instanceIDs: selectedSkills.map(\.id),
                on: batchToggleAction.targetEnabled
            )
        } catch ServiceClient.ClientError.service(let error) where error.code == "unknown_method" {
            batchTogglePreview = localBatchTogglePreview(selectedSkills: selectedSkills, reason: UIStrings.batchToggleServicePreviewUnavailable)
        } catch {
            errorMessage = error.localizedDescription
            batchTogglePreview = nil
        }
    }

    func resetBatchToggleSelectionToVisibleSkills() {
        isBatchToggleSelectionExplicit = true
        batchToggleSelectedSkillIDs = Set(filteredSkills.map(\.id))
    }

    func selectAllVisibleBatchToggleSkills() {
        resetBatchToggleSelectionToVisibleSkills()
    }

    func clearBatchToggleSelection() {
        isBatchToggleSelectionExplicit = true
        batchToggleSelectedSkillIDs = []
    }

    func setBatchToggleSkill(_ skill: SkillRecord, selected: Bool) {
        isBatchToggleSelectionExplicit = true
        var selection = batchToggleSelectedSkillIDs
        if selected {
            selection.insert(skill.id)
        } else {
            selection.remove(skill.id)
        }
        batchToggleSelectedSkillIDs = selection
    }

    func applyVisibleBatchTogglePreview(confirmingPreviewID: String? = nil) async {
        guard let preview = batchTogglePreview else { return }
        if let confirmingPreviewID, confirmingPreviewID != preview.id {
            errorMessage = UIStrings.batchTogglePreviewChanged
            lastMutationMessage = nil
            return
        }
        guard preview.applySupported else {
            errorMessage = UIStrings.batchToggleApplyUnavailable
            lastMutationMessage = nil
            return
        }
        guard preview.hasWritableChanges else {
            errorMessage = UIStrings.batchToggleNoWritableChanges
            lastMutationMessage = nil
            return
        }
        guard !isLoading, !isScanning, !isProjectUpdating, !isSavingSettings, !isWriting else {
            errorMessage = UIStrings.operationUnavailableBusy
            lastMutationMessage = nil
            return
        }

        isApplyingBatchToggle = true
        isWriting = true
        errorMessage = nil
        lastMutationMessage = nil
        defer {
            isWriting = false
            isApplyingBatchToggle = false
        }

        do {
            let result = try await service.applyBatchSkillToggles(preview: preview)
            for item in preview.affectedSkills {
                detailsByID.removeValue(forKey: item.instanceID)
                skillEventsByID.removeValue(forKey: item.instanceID)
            }
            try await refreshCollections()
            await loadCleanupQueue()
            await loadCrossAgentComparisons()
            lastMutationMessage = UIStrings.batchToggleApplied(
                action: preview.action.title,
                count: result.updatedCount == 0 ? preview.writableCount : result.updatedCount
            )
            recordLocalRefresh(message: UIStrings.refreshAfterWrite)
            batchTogglePreview = nil
            await loadSelectedDetail()
        } catch {
            errorMessage = error.localizedDescription
            lastMutationMessage = nil
        }
    }

    func loadSkillManagerTools() async {
        guard !isLoadingSkillManagerTools else { return }
        isLoadingSkillManagerTools = true
        defer { isLoadingSkillManagerTools = false }

        do {
            skillManagerTools = try await service.listSkillManagerTools()
        } catch {
            setSkillManagerError(error.localizedDescription)
        }
    }

    func searchSkillManager() async {
        let query = skillManagerSearchQuery.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else {
            setSkillManagerError(UIStrings.text("skillManager.search.required", "Enter a skill search query."))
            return
        }
        guard !isSearchingSkillManager else { return }
        isSearchingSkillManager = true
        clearSkillManagerFeedback()
        defer { isSearchingSkillManager = false }

        do {
            skillManagerSearchResult = try await service.searchSkillManager(
                query: query,
                owner: skillManagerOwner,
                networkAllowed: skillManagerNetworkAllowed
            )
        } catch {
            setSkillManagerError(error.localizedDescription)
            skillManagerSearchResult = nil
        }
    }

    func listSkillManagerInstalled() async {
        guard !isListingSkillManagerInstalled else { return }
        isListingSkillManagerInstalled = true
        clearSkillManagerFeedback()
        defer { isListingSkillManagerInstalled = false }

        do {
            skillManagerInstalled = try await service.listSkillManagerInstalled(
                agents: selectedSkillManagerAgentIDsForRead(),
                scope: skillManagerScope
            )
        } catch {
            setSkillManagerError(error.localizedDescription)
            skillManagerInstalled = nil
        }
    }

    func setSkillManagerAgent(_ agentID: String, selected: Bool) {
        var next = skillManagerSelectedAgentIDs
        if selected {
            next.insert(agentID)
        } else {
            next.remove(agentID)
        }
        skillManagerSelectedAgentIDs = next
    }

    func selectAllSkillManagerAgents() {
        skillManagerSelectedAgentIDs = Set(SkillManagerAgent.defaultTargets.map(\.rawValue))
    }

    func clearSkillManagerAgents() {
        skillManagerSelectedAgentIDs = []
    }

    func previewSkillManagerInstall(source: String? = nil, skillName: String? = nil) async {
        if let source {
            skillManagerSource = source
        }
        if let skillName {
            skillManagerInstallSkillName = skillName
        }
        guard let agents = selectedSkillManagerAgentIDsForMutation() else { return }
        let source = skillManagerSource.trimmingCharacters(in: .whitespacesAndNewlines)
        let skills = parsedSkillManagerSkillNames(from: skillManagerInstallSkillName)
        guard !source.isEmpty else {
            setSkillManagerError(UIStrings.text("skillManager.source.required", "Enter a skill source."))
            return
        }
        guard !skills.isEmpty else {
            setSkillManagerError(UIStrings.text("skillManager.skill.required", "Enter at least one skill name."))
            return
        }

        await previewSkillManagerMutation {
            try await service.previewSkillManagerInstall(
                source: source,
                skills: skills,
                agents: agents,
                scope: skillManagerScope,
                distribution: skillManagerDistribution,
                networkAllowed: skillManagerNetworkAllowed
            )
        }
    }

    func applySkillManagerInstall() async {
        guard let preview = skillManagerMutationPreview else { return }
        guard let agents = selectedSkillManagerAgentIDsForMutation() else { return }
        await applySkillManagerMutation {
            try await service.applySkillManagerInstall(
                preview: preview,
                source: skillManagerSource.trimmingCharacters(in: .whitespacesAndNewlines),
                skills: parsedSkillManagerSkillNames(from: skillManagerInstallSkillName),
                agents: agents,
                scope: skillManagerScope,
                distribution: skillManagerDistribution,
                networkAllowed: skillManagerNetworkAllowed
            )
        }
    }

    func previewSkillManagerRemove(skillName: String? = nil) async {
        if let skillName {
            skillManagerRemoveSkillName = skillName
        }
        guard let agents = selectedSkillManagerAgentIDsForMutation() else { return }
        let skill = skillManagerRemoveSkillName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !skill.isEmpty else {
            setSkillManagerError(UIStrings.text("skillManager.skill.required", "Enter at least one skill name."))
            return
        }

        await previewSkillManagerMutation {
            try await service.previewSkillManagerRemove(
                skill: skill,
                agents: agents,
                scope: skillManagerScope
            )
        }
    }

    func applySkillManagerRemove() async {
        guard let preview = skillManagerMutationPreview else { return }
        guard let agents = selectedSkillManagerAgentIDsForMutation() else { return }
        await applySkillManagerMutation {
            try await service.applySkillManagerRemove(
                preview: preview,
                skill: skillManagerRemoveSkillName.trimmingCharacters(in: .whitespacesAndNewlines),
                agents: agents,
                scope: skillManagerScope
            )
        }
    }

    func previewSkillManagerUpdate(skillName: String? = nil) async {
        if let skillName {
            skillManagerRemoveSkillName = skillName
        }
        guard let agents = selectedSkillManagerAgentIDsForMutation() else { return }

        await previewSkillManagerMutation {
            try await service.previewSkillManagerUpdate(
                skills: parsedSkillManagerSkillNames(from: skillManagerRemoveSkillName),
                agents: agents,
                scope: skillManagerScope,
                networkAllowed: skillManagerNetworkAllowed
            )
        }
    }

    func applySkillManagerUpdate() async {
        guard let preview = skillManagerMutationPreview else { return }
        guard let agents = selectedSkillManagerAgentIDsForMutation() else { return }
        await applySkillManagerMutation {
            try await service.applySkillManagerUpdate(
                preview: preview,
                skills: parsedSkillManagerSkillNames(from: skillManagerRemoveSkillName),
                agents: agents,
                scope: skillManagerScope,
                networkAllowed: skillManagerNetworkAllowed
            )
        }
    }

    func previewSkillManagerLocalCreate() async {
        let name = skillManagerLocalSkillName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty else {
            setSkillManagerError(UIStrings.text("skillManager.localCreate.required", "Enter a local skill name."))
            return
        }
        guard !isPreviewingSkillManagerMutation else { return }
        isPreviewingSkillManagerMutation = true
        clearSkillManagerWorkflowPreviews()
        defer { isPreviewingSkillManagerMutation = false }

        do {
            skillManagerLocalCreatePreview = try await service.previewSkillManagerLocalCreate(name: name)
        } catch {
            setSkillManagerError(error.localizedDescription)
            skillManagerLocalCreatePreview = nil
        }
    }

    func applySkillManagerLocalCreate() async {
        guard let preview = skillManagerLocalCreatePreview else { return }
        let name = skillManagerLocalSkillName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty else { return }
        guard !isApplyingSkillManagerMutation else { return }
        isApplyingSkillManagerMutation = true
        isWriting = true
        clearSkillManagerFeedback()
        defer {
            isWriting = false
            isApplyingSkillManagerMutation = false
        }

        do {
            _ = try await service.applySkillManagerLocalCreate(preview: preview, name: name)
            clearSkillManagerWritePreviews()
            try await refreshCollections()
            skillManagerMessage = UIStrings.text("skillManager.localCreate.applied", "Local skill template created and imported.")
            recordLocalRefresh(message: UIStrings.refreshAfterWrite)
        } catch {
            setSkillManagerError(error.localizedDescription)
        }
    }

    func previewSkillManagerLocalDelete(skill: SkillRecord) async {
        guard !isPreviewingSkillManagerMutation else { return }
        isPreviewingSkillManagerMutation = true
        clearSkillManagerWorkflowPreviews()
        defer { isPreviewingSkillManagerMutation = false }

        do {
            skillManagerLocalDeletePreview = try await service.previewSkillManagerLocalDelete(instanceID: skill.id)
        } catch {
            setSkillManagerError(error.localizedDescription)
            skillManagerLocalDeletePreview = nil
        }
    }

    func applySkillManagerLocalDelete() async {
        guard let preview = skillManagerLocalDeletePreview else { return }
        guard preview.physicalDeleteAllowed else {
            setSkillManagerError(preview.summary)
            return
        }
        guard !isApplyingSkillManagerMutation else { return }
        isApplyingSkillManagerMutation = true
        isWriting = true
        clearSkillManagerFeedback()
        defer {
            isWriting = false
            isApplyingSkillManagerMutation = false
        }

        do {
            _ = try await service.applySkillManagerLocalDelete(instanceID: preview.instanceId)
            clearSkillManagerWritePreviews()
            try await refreshCollections()
            skillManagerMessage = UIStrings.text("skillManager.localDelete.applied", "Local skill deleted.")
            recordLocalRefresh(message: UIStrings.refreshAfterWrite)
        } catch {
            setSkillManagerError(error.localizedDescription)
        }
    }

    func exportLocalReport(includeSelectedSkill: Bool = true) async {
        guard !isRefreshBusy else {
            errorMessage = UIStrings.operationUnavailableBusy
            lastMutationMessage = nil
            return
        }

        isExportingLocalReport = true
        errorMessage = nil
        lastMutationMessage = nil
        defer { isExportingLocalReport = false }

        let agent = agentFilter == .all ? nil : agentFilter.rawValue
        let state = stateFilter == .all ? nil : stateFilter.rawValue
        let trimmedSearch = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        let scopeSummary = localReportScopeSummary(includeSelectedSkill: includeSelectedSkill)
        do {
            let result = try await service.exportLocalReport(
                format: localReportFormat,
                agent: agent,
                instanceID: includeSelectedSkill ? selectedSkill?.id : nil,
                stateFilter: state,
                search: trimmedSearch.isEmpty ? nil : trimmedSearch
            )
            localReportExportResult = result
            if result.isUnavailable {
                lastMutationMessage = nil
            } else {
                recordLocalReportExportHistory(result: result, scopeSummary: scopeSummary)
                lastMutationMessage = UIStrings.localReportExported(result.displayName)
            }
        } catch {
            localReportExportResult = .unavailable(reason: UIStrings.localReportUnavailableFallback, format: localReportFormat)
            errorMessage = error.localizedDescription
            lastMutationMessage = nil
        }
    }

    func previewToolInstall(skill: SkillRecord, target: ToolInstallTarget) async -> ToolGlobalInstallPreview? {
        guard !isRefreshBusy else {
            errorMessage = UIStrings.operationUnavailableBusy
            return nil
        }
        errorMessage = nil
        do {
            return try await service.previewToolInstall(skill: skill, target: target)
        } catch {
            errorMessage = error.localizedDescription
            return nil
        }
    }

    func confirmToolInstall(skill: SkillRecord, target: ToolInstallTarget) async -> ToolGlobalInstallPreview? {
        guard !isRefreshBusy else {
            errorMessage = UIStrings.operationUnavailableBusy
            return nil
        }
        isWriting = true
        errorMessage = nil
        lastMutationMessage = nil
        defer { isWriting = false }

        do {
            let result = try await service.confirmToolInstall(skill: skill, target: target)
            invalidateDetailCaches(for: [skill.id])
            try await refreshCollections()
            lastMutationMessage = UIStrings.toolGlobalInstalled(skill.name, target.title)
            recordLocalRefresh(message: UIStrings.refreshAfterWrite)
            await loadSelectedDetail()
            return result
        } catch {
            errorMessage = error.localizedDescription
            return nil
        }
    }

    func skillManagerSourcePath(for localSkill: SkillRecord) -> String {
        let url = URL(fileURLWithPath: localSkill.path)
        if url.lastPathComponent.caseInsensitiveCompare("SKILL.md") == .orderedSame {
            return url.deletingLastPathComponent().path
        }
        return localSkill.path
    }

    func clearSkillManagerWorkflowPreviews() {
        clearSkillManagerWritePreviews()
        clearSkillManagerFeedback()
    }

    private func clearSkillManagerWritePreviews() {
        skillManagerMutationPreview = nil
        skillManagerLocalCreatePreview = nil
        skillManagerLocalDeletePreview = nil
    }

    private func clearSkillManagerFeedback() {
        skillManagerErrorMessage = nil
        skillManagerMessage = nil
    }

    private func setSkillManagerError(_ message: String) {
        skillManagerErrorMessage = UIStrings.localizedServiceMessage(message)
        skillManagerMessage = nil
    }

    private func previewSkillManagerMutation(_ operation: () async throws -> SkillManagerMutationRecord) async {
        guard !isPreviewingSkillManagerMutation else { return }
        isPreviewingSkillManagerMutation = true
        clearSkillManagerWorkflowPreviews()
        defer { isPreviewingSkillManagerMutation = false }

        do {
            skillManagerMutationPreview = try await operation()
        } catch {
            setSkillManagerError(error.localizedDescription)
            skillManagerMutationPreview = nil
        }
    }

    private func applySkillManagerMutation(_ operation: () async throws -> SkillManagerMutationRecord) async {
        guard !isApplyingSkillManagerMutation else { return }
        isApplyingSkillManagerMutation = true
        isWriting = true
        clearSkillManagerFeedback()
        defer {
            isWriting = false
            isApplyingSkillManagerMutation = false
        }

        do {
            let result = try await operation()
            clearSkillManagerWritePreviews()
            invalidateDetailCaches(for: result.updatedSkills.map(\.id))
            try await refreshCollections()
            pruneDetailCaches(to: Set(skills.map(\.id)))
            await listSkillManagerInstalled()
            skillManagerMessage = UIStrings.text("skillManager.apply.applied", "Skill Manager operation applied.")
            recordLocalRefresh(message: UIStrings.refreshAfterWrite)
            await loadSelectedDetail()
        } catch {
            setSkillManagerError(error.localizedDescription)
        }
    }

    private func selectedSkillManagerAgentIDsForMutation() -> [String]? {
        let agents = skillManagerSelectedAgents
        guard !agents.isEmpty else {
            setSkillManagerError(UIStrings.text("skillManager.agents.required", "Select at least one target agent."))
            return nil
        }
        return agents
    }

    private func selectedSkillManagerAgentIDsForRead() -> [String] {
        let agents = skillManagerSelectedAgents
        return agents.isEmpty ? SkillManagerAgent.defaultTargets.map(\.rawValue) : agents
    }

    private func parsedSkillManagerSkillNames(from rawValue: String) -> [String] {
        rawValue
            .split { character in
                character == "," || character == "\n" || character == ";"
            }
            .map { String($0).trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
    }

    private func setFindingTriageStatus(_ status: FindingTriageStatus, triageKeys: [String]) async {
        guard !isRefreshBusy else {
            errorMessage = UIStrings.operationUnavailableBusy
            lastMutationMessage = nil
            return
        }

        isWriting = true
        errorMessage = nil
        lastMutationMessage = nil
        defer { isWriting = false }

        do {
            for triageKey in triageKeys {
                if status == .open {
                    _ = try await service.clearFindingTriage(triageKey: triageKey)
                    applyFindingTriage(status: .open, triageKeys: [triageKey], note: nil, updatedAt: nil)
                } else {
                    let record = try await service.setFindingTriage(triageKey: triageKey, status: status)
                    applyFindingTriage(record)
                }
            }
            lastMutationMessage = status == .open
                ? UIStrings.findingTriageReopened
                : UIStrings.findingTriageUpdated(status.title)
        } catch {
            errorMessage = error.localizedDescription
            lastMutationMessage = nil
        }
    }

    private func applyFindingTriage(_ record: FindingTriageRecord) {
        applyFindingTriage(
            status: record.triageStatus,
            triageKeys: [record.triageKey],
            note: record.note,
            updatedAt: record.updatedAt
        )
    }

    private func applyFindingTriage(status: FindingTriageStatus, triageKeys: [String], note: String?, updatedAt: Int64?) {
        let keys = Set(triageKeys)
        findings = findings.map { finding in
            guard keys.contains(finding.triageKey) else { return finding }
            return finding.withTriage(status: status, note: note, updatedAt: updatedAt)
        }
    }

    private func setRuleSeverityOverride(_ severity: String, ruleId: String) async {
        guard !isRefreshBusy else {
            errorMessage = UIStrings.operationUnavailableBusy
            lastMutationMessage = nil
            return
        }

        isWriting = true
        errorMessage = nil
        lastMutationMessage = nil
        defer { isWriting = false }

        do {
            _ = try await service.setSeverityOverride(ruleId: ruleId, severity: severity)
            ruleTuning = try await service.listRuleTuning()
            lastMutationMessage = UIStrings.ruleTuningSeverityUpdated(FindingDisplayModel.severityTitle(severity))
        } catch {
            errorMessage = error.localizedDescription
            lastMutationMessage = nil
        }
    }

    private func clearRuleSeverityOverride(ruleId: String) async {
        guard !isRefreshBusy else {
            errorMessage = UIStrings.operationUnavailableBusy
            lastMutationMessage = nil
            return
        }

        isWriting = true
        errorMessage = nil
        lastMutationMessage = nil
        defer { isWriting = false }

        do {
            _ = try await service.clearSeverityOverride(ruleId: ruleId)
            ruleTuning = try await service.listRuleTuning()
            lastMutationMessage = UIStrings.ruleTuningSeverityCleared
        } catch {
            errorMessage = error.localizedDescription
            lastMutationMessage = nil
        }
    }

    private func setRuleSuppression(ruleId: String, findingGroupID: String?, scope: RuleTuningScope) async {
        guard !isRefreshBusy else {
            errorMessage = UIStrings.operationUnavailableBusy
            lastMutationMessage = nil
            return
        }

        isWriting = true
        errorMessage = nil
        lastMutationMessage = nil
        defer { isWriting = false }

        do {
            _ = try await service.setSuppression(ruleId: ruleId, scope: scope, findingGroupId: findingGroupID)
            ruleTuning = try await service.listRuleTuning()
            lastMutationMessage = UIStrings.ruleTuningSuppressionUpdated
        } catch {
            errorMessage = error.localizedDescription
            lastMutationMessage = nil
        }
    }

    private func clearRuleSuppression(ruleId: String, findingGroupID: String?, scope: RuleTuningScope) async {
        guard !isRefreshBusy else {
            errorMessage = UIStrings.operationUnavailableBusy
            lastMutationMessage = nil
            return
        }

        isWriting = true
        errorMessage = nil
        lastMutationMessage = nil
        defer { isWriting = false }

        do {
            _ = try await service.clearSuppression(ruleId: ruleId, scope: scope, findingGroupId: findingGroupID)
            ruleTuning = try await service.listRuleTuning()
            lastMutationMessage = UIStrings.ruleTuningSuppressionCleared
        } catch {
            errorMessage = error.localizedDescription
            lastMutationMessage = nil
        }
    }

    func prepareAnalyzeLLM() async {
        await prepareLLMAction(.analyze)
    }

    func prepareRecommendLLM() async {
        await prepareLLMAction(.recommend)
    }

    func prepareExplainConflictLLM() async {
        await prepareLLMAction(.explainConflict)
    }

    func prepareDraftFrontmatterLLM() async {
        await prepareLLMAction(.draftFrontmatter)
    }

    func prepareSelectedSkillAnalysis(kind: LLMSkillAnalysisKind) async {
        guard let skill = selectedSkill else { return }
        await prepareSkillAnalysis(kind: kind, scope: .selected, instanceIDs: [skill.id])
    }

    func prepareVisibleSkillAnalysis(kind: LLMSkillAnalysisKind) async {
        let instanceIDs = filteredSkills.map(\.id)
        guard !instanceIDs.isEmpty else { return }
        await prepareSkillAnalysis(kind: kind, scope: .visible, instanceIDs: instanceIDs)
    }

    func previewPromptForSelectedLLMAction(_ action: LLMAction) async {
        guard let skill = selectedSkill else { return }
        let key = llmPromptActionKey(action: action, skillID: skill.id)
        guard !isRefreshBusy else {
            llmPromptPreviews[key] = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        previewingLLMPromptKeys.insert(key)
        llmPromptSendResults.removeValue(forKey: key)
        defer { previewingLLMPromptKeys.remove(key) }

        do {
            llmPromptPreviews[key] = try await service.previewPromptForLLMAction(action: action, skill: skill)
        } catch {
            llmPromptPreviews[key] = .unavailable(reason: error.localizedDescription)
        }
    }

    func confirmPromptForSelectedLLMAction(_ action: LLMAction) async {
        guard let skill = selectedSkill else { return }
        let key = llmPromptActionKey(action: action, skillID: skill.id)
        await confirmLLMPrompt(key: key) { previewID in
            try await service.confirmPromptAndSendForLLMAction(
                previewID: previewID,
                action: action,
                skill: skill
            )
        }
    }

    func previewPromptForSkillAnalysis(kind: LLMSkillAnalysisKind, scope: LLMSkillAnalysisRequestScope) async {
        let instanceIDs = skillAnalysisInstanceIDs(scope: scope)
        guard !instanceIDs.isEmpty else { return }
        let key = skillAnalysisPromptKey(kind: kind, scope: scope, instanceIDs: instanceIDs)
        guard !isRefreshBusy else {
            llmPromptPreviews[key] = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        previewingLLMPromptKeys.insert(key)
        llmPromptSendResults.removeValue(forKey: key)
        defer { previewingLLMPromptKeys.remove(key) }

        do {
            llmPromptPreviews[key] = try await service.previewPromptForSkillAnalysis(
                instanceIDs: instanceIDs,
                kind: kind,
                scope: scope
            )
        } catch {
            llmPromptPreviews[key] = .unavailable(reason: error.localizedDescription)
        }
    }

    func confirmPromptForSkillAnalysis(kind: LLMSkillAnalysisKind, scope: LLMSkillAnalysisRequestScope) async {
        let instanceIDs = skillAnalysisInstanceIDs(scope: scope)
        guard !instanceIDs.isEmpty else { return }
        let key = skillAnalysisPromptKey(kind: kind, scope: scope, instanceIDs: instanceIDs)
        await confirmLLMPrompt(key: key) { previewID in
            try await service.confirmPromptAndSendForSkillAnalysis(
                previewID: previewID,
                instanceIDs: instanceIDs,
                kind: kind,
                scope: scope
            )
        }
    }

    func scoreSelectedSkillQuality() async {
        guard let skill = selectedSkill else { return }
        await scoreSkillQuality(for: skill)
    }

    func previewPromptForSelectedSkillQuality() async {
        guard let skill = selectedSkill else { return }
        let key = skillQualityPromptKey(skillID: skill.id)
        guard !isRefreshBusy else {
            llmPromptPreviews[key] = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        previewingLLMPromptKeys.insert(key)
        llmPromptSendResults.removeValue(forKey: key)
        defer { previewingLLMPromptKeys.remove(key) }

        do {
            llmPromptPreviews[key] = try await service.previewPromptForSkillQuality(skill: skill)
        } catch {
            llmPromptPreviews[key] = .unavailable(reason: error.localizedDescription)
        }
    }

    func confirmPromptForSelectedSkillQuality() async {
        guard let skill = selectedSkill else { return }
        let key = skillQualityPromptKey(skillID: skill.id)
        await confirmLLMPrompt(key: key) { previewID in
            try await service.confirmPromptAndSendForSkillQuality(previewID: previewID, skill: skill)
        }
    }

    func checkSelectedTaskReadiness() async {
        guard let skill = selectedSkill else { return }
        let taskText = normalizedTaskReadinessText
        guard !taskText.isEmpty else {
            taskReadinessResult = .unavailable(taskText: "", reason: UIStrings.taskReadinessTaskRequired)
            taskReadinessCheckedSkillID = skill.id
            return
        }
        guard !isRefreshBusy else {
            taskReadinessResult = .unavailable(taskText: taskText, reason: UIStrings.operationUnavailableBusy)
            taskReadinessCheckedSkillID = skill.id
            return
        }

        checkingTaskReadinessSkillIDs.insert(skill.id)
        defer { checkingTaskReadinessSkillIDs.remove(skill.id) }

        do {
            taskReadinessResult = try await service.checkTaskReadiness(taskText: taskText, skill: skill)
            taskReadinessCheckedSkillID = skill.id
        } catch {
            taskReadinessResult = .unavailable(taskText: taskText, reason: error.localizedDescription)
            taskReadinessCheckedSkillID = skill.id
        }
    }

    func previewPromptForSelectedTaskReadiness() async {
        guard let skill = selectedSkill else { return }
        let taskText = normalizedTaskReadinessText
        guard !taskText.isEmpty else {
            taskReadinessResult = .unavailable(taskText: "", reason: UIStrings.taskReadinessTaskRequired)
            taskReadinessCheckedSkillID = skill.id
            return
        }
        let key = taskReadinessPromptKey(skillID: skill.id, taskText: taskText)
        guard !isRefreshBusy else {
            llmPromptPreviews[key] = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        previewingLLMPromptKeys.insert(key)
        llmPromptSendResults.removeValue(forKey: key)
        defer { previewingLLMPromptKeys.remove(key) }

        do {
            llmPromptPreviews[key] = try await service.previewPromptForTaskReadiness(taskText: taskText, skill: skill)
        } catch {
            llmPromptPreviews[key] = .unavailable(reason: error.localizedDescription)
        }
    }

    func confirmPromptForSelectedTaskReadiness() async {
        guard let skill = selectedSkill else { return }
        let taskText = normalizedTaskReadinessText
        guard !taskText.isEmpty else { return }
        let key = taskReadinessPromptKey(skillID: skill.id, taskText: taskText)
        await confirmLLMPrompt(key: key) { previewID in
            try await service.confirmPromptAndSendForTaskReadiness(
                previewID: previewID,
                taskText: taskText,
                skill: skill
            )
        }
    }

    func rankSelectedSkillRoutes() async {
        guard let skill = selectedSkill else { return }
        let taskText = normalizedRoutingConfidenceText
        guard !taskText.isEmpty else {
            routingConfidenceResult = .unavailable(taskText: "", reason: UIStrings.routingConfidenceTaskRequired)
            routingConfidenceRankedSkillID = skill.id
            return
        }
        guard !isRefreshBusy else {
            routingConfidenceResult = .unavailable(taskText: taskText, reason: UIStrings.operationUnavailableBusy)
            routingConfidenceRankedSkillID = skill.id
            return
        }

        rankingRoutingSkillIDs.insert(skill.id)
        defer { rankingRoutingSkillIDs.remove(skill.id) }

        do {
            routingConfidenceResult = try await service.rankSkillRoutes(taskText: taskText, skill: skill)
            routingConfidenceRankedSkillID = skill.id
        } catch {
            routingConfidenceResult = .unavailable(taskText: taskText, reason: error.localizedDescription)
            routingConfidenceRankedSkillID = skill.id
        }
    }

    func previewPromptForSelectedRoutingConfidence() async {
        guard let skill = selectedSkill else { return }
        let taskText = normalizedRoutingConfidenceText
        guard !taskText.isEmpty else {
            routingConfidenceResult = .unavailable(taskText: "", reason: UIStrings.routingConfidenceTaskRequired)
            routingConfidenceRankedSkillID = skill.id
            return
        }
        let key = routingConfidencePromptKey(skillID: skill.id, taskText: taskText)
        guard !isRefreshBusy else {
            llmPromptPreviews[key] = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        previewingLLMPromptKeys.insert(key)
        llmPromptSendResults.removeValue(forKey: key)
        defer { previewingLLMPromptKeys.remove(key) }

        do {
            llmPromptPreviews[key] = try await service.previewPromptForRoutingConfidence(taskText: taskText, skill: skill)
        } catch {
            llmPromptPreviews[key] = .unavailable(reason: error.localizedDescription)
        }
    }

    func confirmPromptForSelectedRoutingConfidence() async {
        guard let skill = selectedSkill else { return }
        let taskText = normalizedRoutingConfidenceText
        guard !taskText.isEmpty else { return }
        let key = routingConfidencePromptKey(skillID: skill.id, taskText: taskText)
        await confirmLLMPrompt(key: key) { previewID in
            try await service.confirmPromptAndSendForRoutingConfidence(
                previewID: previewID,
                taskText: taskText,
                skill: skill
            )
        }
    }

    func loadTaskBenchmarks() async {
        guard !isLoadingTaskBenchmarks else { return }
        isLoadingTaskBenchmarks = true
        defer { isLoadingTaskBenchmarks = false }

        do {
            taskBenchmarkList = try await service.listTaskBenchmarks(skill: selectedSkill)
        } catch {
            taskBenchmarkList = .unavailable(reason: error.localizedDescription)
        }
    }

    func saveSelectedTaskBenchmark() async {
        guard let skill = selectedSkill else { return }
        let taskText = selectedTaskBenchmarkInput
        guard !taskText.isEmpty else {
            taskBenchmarkEvaluation = .unavailable(reason: UIStrings.taskBenchmarkTaskRequired)
            return
        }
        guard !isRefreshBusy else {
            taskBenchmarkEvaluation = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isSavingTaskBenchmark = true
        defer { isSavingTaskBenchmark = false }

        do {
            let result = try await service.saveTaskBenchmark(taskText: taskText, skill: skill)
            if let benchmark = result.benchmark {
                upsertTaskBenchmark(benchmark)
                taskBenchmarkDeleteResult = nil
                routingRegressionBaseline = nil
                routingRegressionDetection = nil
            } else if let reason = result.fallbackReason {
                taskBenchmarkList = .unavailable(reason: reason)
            }
        } catch {
            taskBenchmarkList = .unavailable(reason: error.localizedDescription)
        }
    }

    func evaluateTaskBenchmarks() async {
        guard !isRefreshBusy else {
            taskBenchmarkEvaluation = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isEvaluatingTaskBenchmarks = true
        defer { isEvaluatingTaskBenchmarks = false }

        do {
            taskBenchmarkEvaluation = try await service.evaluateTaskBenchmarks(
                skill: selectedSkill,
                benchmarkIDs: taskBenchmarkList.benchmarks.isEmpty ? nil : taskBenchmarkList.benchmarks.map(\.id)
            )
        } catch {
            taskBenchmarkEvaluation = .unavailable(reason: error.localizedDescription)
        }
    }

    func saveRoutingBaseline() async {
        guard !isRefreshBusy else {
            routingRegressionBaseline = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isSavingRoutingBaseline = true
        defer { isSavingRoutingBaseline = false }

        do {
            routingRegressionBaseline = try await service.saveRoutingBaseline(
                skill: selectedSkill,
                benchmarkIDs: taskBenchmarkList.benchmarks.isEmpty ? nil : taskBenchmarkList.benchmarks.map(\.id)
            )
            routingRegressionDetection = nil
        } catch {
            routingRegressionBaseline = .unavailable(reason: error.localizedDescription)
        }
    }

    func detectRoutingRegression() async {
        guard !isRefreshBusy else {
            routingRegressionDetection = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isDetectingRoutingRegression = true
        defer { isDetectingRoutingRegression = false }

        do {
            routingRegressionDetection = try await service.detectRoutingRegression(
                skill: selectedSkill,
                benchmarkIDs: taskBenchmarkList.benchmarks.isEmpty ? nil : taskBenchmarkList.benchmarks.map(\.id)
            )
        } catch {
            routingRegressionDetection = .unavailable(reason: error.localizedDescription)
        }
    }

    func loadRoutingAccuracyDashboard() async {
        guard !isLoadingRoutingAccuracyDashboard else { return }
        guard !isRefreshBusy else {
            routingAccuracyDashboard = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isLoadingRoutingAccuracyDashboard = true
        defer { isLoadingRoutingAccuracyDashboard = false }

        let agent = agentFilter == .all ? nil : agentFilter.rawValue
        do {
            routingAccuracyDashboard = try await service.routingAccuracyDashboard(
                agent: agent,
                windowDays: 30,
                limit: 20,
                includeHistory: true,
                includeRecentEvidence: true
            )
        } catch {
            routingAccuracyDashboard = .unavailable(reason: error.localizedDescription)
        }
    }

    func detectStaleDrift() async {
        guard !isDetectingStaleDrift else { return }
        guard !isRefreshBusy else {
            staleDriftDetection = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isDetectingStaleDrift = true
        defer { isDetectingStaleDrift = false }

        let agent = agentFilter == .all ? nil : agentFilter.rawValue
        do {
            staleDriftDetection = try await service.detectStaleDrift(
                agent: agent,
                limit: 40,
                includeReadinessImpact: true
            )
        } catch {
            staleDriftDetection = .unavailable(reason: error.localizedDescription)
        }
    }

    func searchKnowledge() async {
        let query = normalizedKnowledgeSearchText
        guard !query.isEmpty else {
            knowledgeSearchResult = .unavailable(reason: UIStrings.knowledgeQueryRequired)
            return
        }
        guard !isSearchingKnowledge else { return }
        guard !isRefreshBusy else {
            knowledgeSearchResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isSearchingKnowledge = true
        defer { isSearchingKnowledge = false }

        let agent = agentFilter == .all ? nil : agentFilter.rawValue
        do {
            knowledgeSearchResult = try await service.searchKnowledge(
                query: query,
                agent: agent,
                limit: 20
            )
        } catch {
            knowledgeSearchResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func buildLocalSkillMap() async {
        guard !isBuildingLocalSkillMap else { return }
        guard !isRefreshBusy else {
            localSkillMapResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isBuildingLocalSkillMap = true
        defer { isBuildingLocalSkillMap = false }

        let agent = agentFilter == .all ? nil : agentFilter.rawValue
        do {
            localSkillMapResult = try await service.buildLocalSkillMap(
                agent: agent,
                project: activeProjectContext,
                selectedSkill: selectedSkill,
                limit: 30,
                includeEdges: true,
                includeClusters: true,
                includeEvidence: true
            )
        } catch {
            localSkillMapResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func loadSkillLifecycleTimeline() async {
        guard !isLoadingSkillLifecycleTimeline else { return }
        guard !isRefreshBusy else {
            skillLifecycleTimelineResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isLoadingSkillLifecycleTimeline = true
        defer { isLoadingSkillLifecycleTimeline = false }

        let agent = agentFilter == .all ? nil : agentFilter.rawValue
        do {
            skillLifecycleTimelineResult = try await service.loadSkillLifecycleTimeline(
                agent: agent,
                project: activeProjectContext,
                selectedSkill: selectedSkill,
                limit: 20,
                includeSkillRows: true,
                includeAgentRows: true,
                includeEvidence: true,
                includeSafetyFlags: true
            )
        } catch {
            skillLifecycleTimelineResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func buildTaskCockpit() async {
        let taskText = selectedTaskCockpitInput
        guard !taskText.isEmpty else {
            taskCockpitResult = .unavailable(taskText: "", reason: UIStrings.taskCockpitTaskRequired)
            taskCockpitOperationState = TaskCockpitOperationState.idle.finished(
                phase: .failed,
                message: UIStrings.taskCockpitTaskRequired
            )
            return
        }
        let selectedAgents = taskCockpitSelectedAgents
        guard !selectedAgents.isEmpty else {
            let message = UIStrings.text("taskCockpit.agentScope.required", "Select at least one agent.")
            taskCockpitResult = .unavailable(taskText: taskText, reason: message)
            taskCockpitOperationState = TaskCockpitOperationState.idle.finished(
                phase: .failed,
                message: message
            )
            return
        }
        guard !isBuildingTaskCockpit else { return }
        guard !isRefreshBusy else {
            taskCockpitResult = .unavailable(taskText: taskText, reason: UIStrings.operationUnavailableBusy)
            taskCockpitOperationState = TaskCockpitOperationState.preparing(
                taskText: taskText,
                timeoutSeconds: roundedTaskCockpitTimeoutSeconds
            ).finished(
                phase: .failed,
                message: UIStrings.operationUnavailableBusy
            )
            return
        }

        let operationID = UUID()
        taskCockpitOperationID = operationID
        isBuildingTaskCockpit = true
        taskCockpitOperationState = .preparing(
            taskText: taskText,
            timeoutSeconds: roundedTaskCockpitTimeoutSeconds
        )
        scheduleTaskCockpitTimeout(operationID: operationID, taskText: taskText)

        let candidateSkillIDs = taskCockpitCandidateSkillIDs(for: selectedAgents)
        let serviceTask = Task {
            let preview = try await service.previewPromptForTaskCockpit(
                taskText: taskText,
                agents: selectedAgents,
                instanceIDs: candidateSkillIDs
            )
            guard self.canSendLLMPrompt(preview) else {
                let reason = UIStrings.localizedServiceMessage(preview.disabledReason ?? UIStrings.llmSkillAnalysisUnavailable)
                return TaskCockpitResult.unavailable(taskText: taskText, reason: reason)
            }
            let sendResult = try await service.confirmPromptAndSendForTaskCockpit(
                previewID: preview.previewID,
                taskText: taskText,
                agents: selectedAgents,
                instanceIDs: candidateSkillIDs
            )
            guard sendResult.success else {
                return TaskCockpitResult.unavailable(taskText: taskText, reason: UIStrings.localizedServiceMessage(sendResult.message))
            }
            return TaskCockpitProviderOutputParser.result(
                from: sendResult.outputText,
                taskText: taskText,
                agentIDs: selectedAgents
            )
        }
        taskCockpitServiceTask = serviceTask

        do {
            let result = try await serviceTask.value
            guard isCurrentTaskCockpitOperation(operationID) else { return }
            taskCockpitResult = result
            if let diagnosticReason = result.recoveryDiagnosticReason {
                finishTaskCockpitOperation(
                    operationID,
                    phase: .fallback,
                    message: UIStrings.taskCockpitLoadedWithFallback(diagnosticReason)
                )
            } else {
                finishTaskCockpitOperation(
                    operationID,
                    phase: .completed,
                    message: UIStrings.taskCockpitLoaded
                )
            }
            recordTaskCockpitHistory(result: result, taskText: taskText, agentIDs: selectedAgents)
        } catch {
            guard isCurrentTaskCockpitOperation(operationID) else { return }
            let message = UIStrings.localizedServiceMessage(error.localizedDescription)
            taskCockpitResult = .unavailable(taskText: taskText, reason: message)
            finishTaskCockpitOperation(
                operationID,
                phase: .failed,
                message: UIStrings.taskCockpitFailed(message)
            )
        }
    }

    func cancelTaskCockpitBuild() {
        cancelTaskCockpitBuild(publishFallbackResult: true)
    }

    private func cancelTaskCockpitBuild(publishFallbackResult: Bool) {
        guard taskCockpitOperationID != nil, isBuildingTaskCockpit else { return }
        let taskText = taskCockpitOperationState.taskText
        let message = UIStrings.taskCockpitCancelled
        taskCockpitTimeoutTask?.cancel()
        taskCockpitTimeoutTask = nil
        taskCockpitServiceTask?.cancel()
        taskCockpitServiceTask = nil
        taskCockpitOperationID = nil
        isBuildingTaskCockpit = false
        if publishFallbackResult {
            taskCockpitResult = .unavailable(taskText: taskText, reason: message)
        }
        taskCockpitOperationState = taskCockpitOperationState.finished(
            phase: .cancelled,
            message: message
        )
    }

    func groupSimilarSkills() async {
        guard !isGroupingSimilarSkills else { return }
        guard !isRefreshBusy else {
            similarSkillGroupingResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isGroupingSimilarSkills = true
        defer { isGroupingSimilarSkills = false }

        let agent = agentFilter == .all ? nil : agentFilter.rawValue
        do {
            similarSkillGroupingResult = try await service.groupSimilarSkills(
                agent: agent,
                limit: 20,
                minScore: 0.62,
                includeSingletons: false
            )
        } catch {
            similarSkillGroupingResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func buildCapabilityTaxonomy() async {
        guard !isBuildingCapabilityTaxonomy else { return }
        guard !isRefreshBusy else {
            capabilityTaxonomyResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isBuildingCapabilityTaxonomy = true
        defer { isBuildingCapabilityTaxonomy = false }

        let agent = agentFilter == .all ? nil : agentFilter.rawValue
        do {
            capabilityTaxonomyResult = try await service.buildCapabilityTaxonomy(
                agent: agent,
                limit: 20,
                includeSingleSkillDomains: true
            )
        } catch {
            capabilityTaxonomyResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func checkWorkspaceReadiness() async {
        guard !isCheckingWorkspaceReadiness else { return }
        guard !isRefreshBusy else {
            workspaceReadinessResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isCheckingWorkspaceReadiness = true
        defer { isCheckingWorkspaceReadiness = false }

        let agent = agentFilter == .all ? nil : agentFilter.rawValue
        let taskText = selectedCrossAgentReadinessInput.trimmingCharacters(in: .whitespacesAndNewlines)
        do {
            workspaceReadinessResult = try await service.checkWorkspaceReadiness(
                taskText: taskText.isEmpty ? nil : taskText,
                agent: agent,
                project: activeProjectContext,
                limit: 40,
                includeChecklist: true,
                includeCapabilities: true
            )
        } catch {
            workspaceReadinessResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func planRemediation() async {
        guard !isPlanningRemediation else { return }
        guard !isRefreshBusy else {
            remediationPlanResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isPlanningRemediation = true
        defer { isPlanningRemediation = false }

        let agent = agentFilter == .all ? nil : agentFilter.rawValue
        let taskText = selectedCrossAgentReadinessInput.trimmingCharacters(in: .whitespacesAndNewlines)
        do {
            remediationPlanResult = try await service.planRemediation(
                taskText: taskText.isEmpty ? nil : taskText,
                agent: agent,
                project: activeProjectContext,
                limit: 20,
                includeGuidanceOnly: true
            )
        } catch {
            remediationPlanResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func previewRemediationDrafts() async {
        guard !isPreviewingRemediationDrafts else { return }
        guard !isRefreshBusy else {
            remediationPreviewDraftsResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isPreviewingRemediationDrafts = true
        defer { isPreviewingRemediationDrafts = false }

        let agent = agentFilter == .all ? nil : agentFilter.rawValue
        let taskText = selectedCrossAgentReadinessInput.trimmingCharacters(in: .whitespacesAndNewlines)
        do {
            remediationPreviewDraftsResult = try await service.previewRemediationDrafts(
                taskText: taskText.isEmpty ? nil : taskText,
                agent: agent,
                project: activeProjectContext,
                limit: 20
            )
        } catch {
            remediationPreviewDraftsResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func previewRemediationImpact() async {
        guard !isPreviewingRemediationImpact else { return }
        guard !isRefreshBusy else {
            remediationImpactPreviewResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isPreviewingRemediationImpact = true
        defer { isPreviewingRemediationImpact = false }

        let agent = agentFilter == .all ? nil : agentFilter.rawValue
        let taskText = selectedCrossAgentReadinessInput.trimmingCharacters(in: .whitespacesAndNewlines)
        do {
            remediationImpactPreviewResult = try await service.previewRemediationImpact(
                taskText: taskText.isEmpty ? nil : taskText,
                agent: agent,
                project: activeProjectContext,
                selectedSkill: selectedSkill,
                action: "review",
                limit: 20,
                includeTaskImpacts: true,
                includeAgentImpacts: true,
                includeSkillImpacts: true,
                includeRiskDeltas: true,
                includeSnapshotRollback: true,
                includeBlocked: true
            )
        } catch {
            remediationImpactPreviewResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func reviewRemediationBatch(options: RemediationBatchReviewOptions = RemediationBatchReviewOptions()) async {
        guard !isReviewingRemediationBatch else { return }
        guard !isRefreshBusy else {
            remediationBatchReviewResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isReviewingRemediationBatch = true
        defer { isReviewingRemediationBatch = false }

        let agent = agentFilter == .all ? nil : agentFilter.rawValue
        let taskText = selectedCrossAgentReadinessInput.trimmingCharacters(in: .whitespacesAndNewlines)
        do {
            remediationBatchReviewResult = try await service.batchReviewRemediation(
                taskText: taskText.isEmpty ? nil : taskText,
                agent: agent,
                project: activeProjectContext,
                selectedSkill: selectedSkill,
                limit: 30,
                options: options
            )
        } catch {
            remediationBatchReviewResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func loadRemediationHistory() async {
        guard !isLoadingRemediationHistory else { return }
        guard !isRefreshBusy else {
            remediationHistoryResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isLoadingRemediationHistory = true
        defer { isLoadingRemediationHistory = false }

        let agent = agentFilter == .all ? nil : agentFilter.rawValue
        let taskText = selectedCrossAgentReadinessInput.trimmingCharacters(in: .whitespacesAndNewlines)
        do {
            remediationHistoryResult = try await service.listRemediationHistory(
                taskText: taskText.isEmpty ? nil : taskText,
                agent: agent,
                project: activeProjectContext,
                selectedSkill: selectedSkill,
                limit: 30
            )
        } catch {
            remediationHistoryResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func recordRemediationHistory() async {
        guard !isRecordingRemediationHistory else { return }
        guard !isRefreshBusy else {
            remediationHistoryRecordResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isRecordingRemediationHistory = true
        defer { isRecordingRemediationHistory = false }

        let agent = agentFilter == .all ? nil : agentFilter.rawValue
        let taskText = selectedCrossAgentReadinessInput.trimmingCharacters(in: .whitespacesAndNewlines)
        do {
            remediationHistoryRecordResult = try await service.recordRemediationHistory(
                taskText: taskText.isEmpty ? nil : taskText,
                agent: agent,
                project: activeProjectContext,
                selectedSkill: selectedSkill,
                evidenceRefs: remediationHistoryEvidenceRefs()
            )
        } catch {
            remediationHistoryRecordResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func planGuidedCleanupFlow() async {
        guard !isPlanningGuidedCleanupFlow else { return }
        guard !isRefreshBusy else {
            guidedCleanupFlowResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isPlanningGuidedCleanupFlow = true
        defer { isPlanningGuidedCleanupFlow = false }

        let agent = agentFilter == .all ? nil : agentFilter.rawValue
        let taskText = selectedCrossAgentReadinessInput.trimmingCharacters(in: .whitespacesAndNewlines)
        do {
            guidedCleanupFlowResult = try await service.planGuidedCleanupFlow(
                taskText: taskText.isEmpty ? nil : taskText,
                agent: agent,
                project: activeProjectContext,
                selectedSkill: selectedSkill,
                limit: 12,
                includeIssueGroups: true,
                includeSafeNextActions: true,
                includeRecordedSteps: true,
                includeEvidence: true,
                includeSafetyFlags: true
            )
        } catch {
            guidedCleanupFlowResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func recordGuidedCleanupStep(_ step: GuidedCleanupFlowStep? = nil) async {
        guard !isRecordingGuidedCleanupStep else { return }
        guard !isRefreshBusy else {
            guidedCleanupRecordResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }
        guard let step = step ?? guidedCleanupFlowResult?.recommendedStep else {
            guidedCleanupRecordResult = .unavailable(reason: UIStrings.guidedCleanupFlowNoSteps)
            return
        }

        isRecordingGuidedCleanupStep = true
        defer { isRecordingGuidedCleanupStep = false }

        let agent = agentFilter == .all ? nil : agentFilter.rawValue
        let taskText = selectedCrossAgentReadinessInput.trimmingCharacters(in: .whitespacesAndNewlines)
        do {
            guidedCleanupRecordResult = try await service.recordGuidedCleanupStep(
                taskText: taskText.isEmpty ? nil : taskText,
                agent: agent,
                project: activeProjectContext,
                selectedSkill: selectedSkill,
                step: step,
                evidenceRefs: guidedCleanupEvidenceRefs(for: step)
            )
        } catch {
            guidedCleanupRecordResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func compareCrossAgentReadiness() async {
        let taskText = selectedCrossAgentReadinessInput
        guard !taskText.isEmpty else {
            crossAgentReadinessResult = .unavailable(taskText: "", reason: UIStrings.crossAgentReadinessTaskRequired)
            return
        }
        guard !isRefreshBusy else {
            crossAgentReadinessResult = .unavailable(taskText: taskText, reason: UIStrings.operationUnavailableBusy)
            return
        }

        isComparingCrossAgentReadiness = true
        defer { isComparingCrossAgentReadiness = false }

        do {
            crossAgentReadinessResult = try await service.compareAgentReadiness(
                taskText: taskText,
                agents: nil,
                limitPerAgent: 3,
                includeRoutingAccuracy: true,
                includeBenchmarks: true
            )
        } catch {
            crossAgentReadinessResult = .unavailable(taskText: taskText, reason: error.localizedDescription)
        }
    }

    func loadTraceImports() async {
        guard !isLoadingTraceImports else { return }
        isLoadingTraceImports = true
        defer { isLoadingTraceImports = false }

        do {
            traceImportList = try await service.listTraceImports()
        } catch {
            traceImportList = .unavailable(reason: error.localizedDescription)
        }
    }

    func importLocalTrace() async {
        let traceText = normalizedTraceImportText
        guard !traceText.isEmpty else {
            traceImportResult = .unavailable(reason: UIStrings.traceImportInputRequired)
            return
        }
        guard !isRefreshBusy else {
            traceImportResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isImportingTrace = true
        defer { isImportingTrace = false }

        do {
            let result = try await service.importLocalTrace(
                traceText: traceText,
                title: normalizedOptional(traceImportTitle),
                taskText: normalizedOptional(traceImportTask),
                expectedSkillNames: normalizedTraceExpectedSkillNames,
                skill: selectedSkill
            )
            traceImportResult = result
            if let record = result.record {
                upsertTraceImport(record)
                traceImportDeleteResult = nil
                traceImportText = ""
            } else if let reason = result.fallbackReason {
                traceImportList = .unavailable(reason: reason)
            }
        } catch {
            traceImportResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func deleteTraceImport(_ record: AgentTraceImportRecord) async {
        guard !isRefreshBusy else {
            traceImportDeleteResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        deletingTraceImportIDs.insert(record.id)
        defer { deletingTraceImportIDs.remove(record.id) }

        do {
            let result = try await service.deleteTraceImport(importID: record.id)
            traceImportDeleteResult = result
            guard result.deleted else { return }
            traceImportList = AgentTraceImportListResult(
                imports: traceImportList.imports.filter { $0.id != record.id },
                fallbackReason: traceImportList.fallbackReason
            )
            if traceImportResult?.record?.id == record.id {
                traceImportResult = nil
            }
        } catch {
            traceImportDeleteResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func loadAgentSessionSkillReviews() async {
        guard !isLoadingAgentSessionSkillReviews else { return }
        guard !isRefreshBusy else {
            agentSessionSkillReviewList = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isLoadingAgentSessionSkillReviews = true
        defer { isLoadingAgentSessionSkillReviews = false }

        let agent = agentFilter == .all ? nil : agentFilter.rawValue
        do {
            agentSessionSkillReviewList = try await service.listAgentSessionSkillReviews(
                taskText: normalizedOptional(agentSessionSkillReviewTask),
                agent: agent,
                skill: selectedSkill,
                project: activeProjectContext,
                limit: 20
            )
        } catch {
            agentSessionSkillReviewList = .unavailable(reason: error.localizedDescription)
        }
    }

    func refreshSelectedAgentLocalSessions() async {
        await previewLocalSessions(allowDuringCatalogRefresh: true, force: true)
    }

    func refreshSelectedAgentLocalSessionsIfNeeded() async {
        await previewLocalSessions(allowDuringCatalogRefresh: true, force: false)
    }

    func previewLocalSessions() async {
        await previewLocalSessions(allowDuringCatalogRefresh: false, force: true)
    }

    private func previewLocalSessions(allowDuringCatalogRefresh: Bool, force: Bool) async {
        let roots = normalizedLocalSessionPreviewRoots
        guard allowDuringCatalogRefresh || !isRefreshBusy else {
            localSessionPreviewResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        let requestKey = localSessionPreviewRequestKey(roots: roots)
        if !force {
            if loadedLocalSessionPreviewRequestKey == requestKey || activeLocalSessionPreviewRequestKey == requestKey {
                return
            }
        }

        localSessionPreviewGeneration += 1
        let generation = localSessionPreviewGeneration
        let requestedAgentFilter = agentFilter
        let previousResult = localSessionPreviewResult
        let agent = requestedAgentFilter == .all ? nil : requestedAgentFilter.rawValue
        activeLocalSessionPreviewRequestKey = requestKey
        isPreviewingLocalSessions = true
        defer {
            if generation == localSessionPreviewGeneration {
                isPreviewingLocalSessions = false
                activeLocalSessionPreviewRequestKey = nil
            }
        }

        do {
            let result = try await service.previewLocalSessions(
                authorizedRoots: roots,
                agent: agent,
                scope: .all,
                search: nil,
                project: activeProjectContext,
                limit: 20
            )
            guard generation == localSessionPreviewGeneration, agentFilter == requestedAgentFilter else { return }
            localSessionPreviewResult = result
            loadedLocalSessionPreviewRequestKey = requestKey
            normalizeSelectedLocalSession()
        } catch {
            guard generation == localSessionPreviewGeneration, agentFilter == requestedAgentFilter else { return }
            loadedLocalSessionPreviewRequestKey = requestKey
            if previousResult.sessionRows.isEmpty {
                localSessionPreviewResult = .unavailable(reason: error.localizedDescription)
                selectedLocalSessionID = nil
                if selectedSidebarSelection?.isSession == true {
                    setSidebarSelection(nil)
                    selectedDetailSection = .overview
                }
            }
        }
    }

    private func localSessionPreviewRequestKey(roots: [String]) -> String {
        let agent = agentFilter == .all ? SkillAgentFilter.all.rawValue : agentFilter.rawValue
        let rootKey = roots.joined(separator: "\u{1f}")
        let projectRoot = activeProjectContext?.rootPath ?? ""
        let projectCWD = activeProjectContext?.currentCWD ?? ""
        return [
            agent,
            projectRoot,
            projectCWD,
            rootKey
        ].joined(separator: "\u{1e}")
    }

    func previewMcpServers() async {
        let paths = normalizedMcpServerPreviewPaths
        guard !isRefreshBusy else {
            mcpServerPreviewResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isPreviewingMcpServers = true
        defer { isPreviewingMcpServers = false }

        do {
            mcpServerPreviewResult = try await service.previewMcpServers(
                authorizedConfigPaths: paths,
                limit: 20
            )
        } catch {
            mcpServerPreviewResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func reviewAgentSessionSkillUse() async {
        let transcriptText = normalizedAgentSessionSkillReviewTranscript
        guard !transcriptText.isEmpty else {
            agentSessionSkillReviewResult = .unavailable(reason: UIStrings.agentSessionReviewInputRequired)
            return
        }
        guard !isRefreshBusy else {
            agentSessionSkillReviewResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isReviewingAgentSessionSkillUse = true
        defer { isReviewingAgentSessionSkillUse = false }

        do {
            let result = try await service.reviewAgentSessionSkillUse(
                transcriptText: transcriptText,
                taskText: normalizedOptional(agentSessionSkillReviewTask),
                expectedSkillNames: normalizedAgentSessionExpectedSkillNames,
                skill: selectedSkill,
                project: activeProjectContext
            )
            agentSessionSkillReviewResult = result
            var returnedReviews = result.reviews
            if let review = result.review, !returnedReviews.contains(where: { $0.id == review.id }) {
                returnedReviews.insert(review, at: 0)
            }
            for review in returnedReviews.reversed() {
                upsertAgentSessionSkillReview(review)
            }
            if result.review != nil || !result.reviews.isEmpty {
                agentSessionSkillReviewDeleteResult = nil
                agentSessionSkillReviewTranscript = ""
            } else if let reason = result.fallbackReason {
                agentSessionSkillReviewList = .unavailable(reason: reason)
            }
        } catch {
            agentSessionSkillReviewResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func deleteAgentSessionSkillReview(_ record: AgentSessionSkillReviewRecord) async {
        guard !isRefreshBusy else {
            agentSessionSkillReviewDeleteResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        deletingAgentSessionSkillReviewIDs.insert(record.id)
        defer { deletingAgentSessionSkillReviewIDs.remove(record.id) }

        do {
            let result = try await service.deleteAgentSessionSkillReview(reviewID: record.id)
            agentSessionSkillReviewDeleteResult = result
            guard result.deleted else { return }
            agentSessionSkillReviewList = AgentSessionSkillReviewListResult(
                generatedBy: agentSessionSkillReviewList.generatedBy,
                catalogAvailable: agentSessionSkillReviewList.catalogAvailable,
                filters: agentSessionSkillReviewList.filters,
                summary: agentSessionSkillReviewList.summary,
                reviews: agentSessionSkillReviewList.reviews.filter { $0.id != record.id },
                evidenceReferences: agentSessionSkillReviewList.evidenceReferences,
                safetyFlags: agentSessionSkillReviewList.safetyFlags,
                fallbackReason: agentSessionSkillReviewList.fallbackReason
            )
            if agentSessionSkillReviewResult?.review?.id == record.id {
                agentSessionSkillReviewResult = nil
            }
        } catch {
            agentSessionSkillReviewDeleteResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func deleteTaskBenchmark(_ benchmark: TaskBenchmarkRecord) async {
        guard !isRefreshBusy else {
            taskBenchmarkDeleteResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        deletingTaskBenchmarkIDs.insert(benchmark.id)
        defer { deletingTaskBenchmarkIDs.remove(benchmark.id) }

        do {
            let result = try await service.deleteTaskBenchmark(benchmarkID: benchmark.id)
            taskBenchmarkDeleteResult = result
            guard result.deleted else { return }
            taskBenchmarkList = TaskBenchmarkListResult(
                benchmarks: taskBenchmarkList.benchmarks.filter { $0.id != benchmark.id },
                fallbackReason: taskBenchmarkList.fallbackReason
            )
            taskBenchmarkEvaluation = nil
            routingRegressionBaseline = nil
            routingRegressionDetection = nil
        } catch {
            taskBenchmarkDeleteResult = .unavailable(reason: error.localizedDescription)
        }
    }

    func previewScriptExecutionSafety(for skill: SkillRecord) async {
        guard !isRefreshBusy else {
            scriptExecutionPreviews[skill.id] = .unavailable(skill: skill, reason: UIStrings.operationUnavailableBusy)
            return
        }

        previewingScriptExecutionSkillIDs.insert(skill.id)
        defer { previewingScriptExecutionSkillIDs.remove(skill.id) }

        do {
            scriptExecutionPreviews[skill.id] = try await service.previewScriptExecution(skill: skill)
        } catch {
            scriptExecutionPreviews[skill.id] = .unavailable(skill: skill, reason: error.localizedDescription)
        }
    }

    func previewRollback(snapshotID: String) async throws -> SnapshotRollbackPreviewRecord {
        errorMessage = nil
        guard agentConfigSnapshots.contains(where: { $0.id == snapshotID }) else {
            let message = "Snapshot is not in the selected agent config timeline."
            errorMessage = message
            throw ServiceClient.ClientError.invalidOutput(message)
        }
        do {
            return try await service.previewSnapshotRollback(snapshotID: snapshotID)
        } catch {
            errorMessage = error.localizedDescription
            throw error
        }
    }

    func rollbackSnapshot(snapshotID: String) async {
        guard !isRefreshBusy else { return }
        guard agentConfigSnapshots.contains(where: { $0.id == snapshotID }) else {
            errorMessage = "Snapshot is not in the selected agent config timeline."
            lastMutationMessage = nil
            return
        }
        isWriting = true
        errorMessage = nil
        lastMutationMessage = nil
        defer { isWriting = false }

        do {
            let scannedCount = try await service.rollbackSnapshot(snapshotID: snapshotID)
            detailsByID.removeAll()
            try await refreshCollections()
            lastMutationMessage = UIStrings.rollbackRescanned(scannedCount)
            recordLocalRefresh(message: UIStrings.refreshAfterRollback(scannedCount))
            await loadSelectedDetail()
        } catch {
            errorMessage = error.localizedDescription
        }
    }

    func loadSelectedAgentConfigDataIfNeeded() async {
        await loadAgentConfigSnapshotsIfNeeded(agent: agentFilter.rawValue)
        await loadCurrentAgentConfigDocumentsIfNeeded(agent: agentFilter.rawValue)
        if agentFilter == .claudeCode {
            await loadClaudeSettingsIfNeeded()
        }
    }

    func refreshSelectedAgentConfigData() async {
        await loadAgentConfigSnapshots(agent: agentFilter.rawValue)
        await loadCurrentAgentConfigDocuments(agent: agentFilter.rawValue)
        if agentFilter == .claudeCode {
            await loadClaudeSettings()
        }
    }

    func loadClaudeSettingsIfNeeded() async {
        await loadClaudeSettings(force: false)
    }

    func loadClaudeSettings() async {
        await loadClaudeSettings(force: true)
    }

    private func loadClaudeSettings(force: Bool) async {
        let requestKey = claudeSettingsRequestKey()
        if !force {
            if loadedClaudeSettingsRequestKey == requestKey || activeClaudeSettingsRequestKey == requestKey {
                return
            }
        }
        guard activeClaudeSettingsRequestKey != requestKey else { return }

        claudeSettingsLoadGeneration += 1
        let generation = claudeSettingsLoadGeneration
        activeClaudeSettingsRequestKey = requestKey
        isLoadingSettings = true
        settingsErrorMessage = nil
        defer {
            if generation == claudeSettingsLoadGeneration {
                isLoadingSettings = false
            }
            if activeClaudeSettingsRequestKey == requestKey {
                activeClaudeSettingsRequestKey = nil
            }
        }

        do {
            let settings = try await service.readClaudeSettings()
            guard generation == claudeSettingsLoadGeneration else { return }
            claudeSettings = settings
            loadedClaudeSettingsRequestKey = requestKey
        } catch {
            guard generation == claudeSettingsLoadGeneration else { return }
            settingsErrorMessage = error.localizedDescription
        }
    }

    func loadCurrentAgentConfigDocumentsIfNeeded(agent requestedAgent: String? = nil) async {
        await loadCurrentAgentConfigDocuments(agent: requestedAgent, force: false)
    }

    func loadCurrentAgentConfigDocuments(agent requestedAgent: String? = nil) async {
        await loadCurrentAgentConfigDocuments(agent: requestedAgent, force: true)
    }

    private func loadCurrentAgentConfigDocuments(agent requestedAgent: String? = nil, force: Bool) async {
        guard let agent = normalizedConfigAgent(requestedAgent) else {
            if !currentAgentConfigDocuments.isEmpty {
                currentAgentConfigDocuments = []
                normalizeConfigSelection()
            }
            return
        }

        let requestKey = agentConfigRequestKey(agent: agent)
        if !force {
            if loadedAgentConfigDocumentRequestKey == requestKey || activeAgentConfigDocumentRequestKey == requestKey {
                return
            }
        }
        guard activeAgentConfigDocumentRequestKey != requestKey else { return }

        agentConfigDocumentLoadGeneration += 1
        let generation = agentConfigDocumentLoadGeneration
        activeAgentConfigDocumentRequestKey = requestKey
        isLoadingAgentConfigDocuments = true
        settingsErrorMessage = nil
        defer {
            if generation == agentConfigDocumentLoadGeneration {
                isLoadingAgentConfigDocuments = false
            }
            if activeAgentConfigDocumentRequestKey == requestKey {
                activeAgentConfigDocumentRequestKey = nil
            }
        }

        do {
            let documents = try await service.readAgentConfig(agent: agent)
            guard generation == agentConfigDocumentLoadGeneration, normalizedConfigAgent(nil) == agent else { return }
            currentAgentConfigDocuments = documents
            loadedAgentConfigDocumentRequestKey = requestKey
            normalizeConfigSelection()
        } catch {
            guard generation == agentConfigDocumentLoadGeneration, normalizedConfigAgent(nil) == agent else { return }
            normalizeConfigSelection()
            settingsErrorMessage = error.localizedDescription
        }
    }

    func clearSettingsFeedback() {
        settingsMessage = nil
        settingsErrorMessage = nil
    }

    func loadAIProviderStatusIfNeeded() async {
        guard !hasLoadedAIProviderStatus else { return }
        await loadAIProviderStatus(force: false)
    }

    func loadAIProviderStatus() async {
        await loadAIProviderStatus(force: true)
    }

    private func loadAIProviderStatus(force: Bool) async {
        guard !isLoadingAIProvider else { return }
        guard force || !hasLoadedAIProviderStatus else { return }
        isLoadingAIProvider = true
        aiProviderErrorMessage = nil
        defer { isLoadingAIProvider = false }

        do {
            aiProviderStatus = try await service.aiProviderStatus()
            aiProviderTestResult = aiProviderStatus.lastTest
            hasLoadedAIProviderStatus = true
        } catch {
            aiProviderStatus = .unavailable(reason: error.localizedDescription)
            aiProviderErrorMessage = error.localizedDescription
            hasLoadedAIProviderStatus = true
        }
    }

    func loadLLMPromptRuns() async {
        guard !isLoadingLLMPromptRuns else { return }
        isLoadingLLMPromptRuns = true
        defer { isLoadingLLMPromptRuns = false }

        llmPromptRunList = await fetchLLMPromptRuns()
        hydratePromptSendResultsFromRuns(currentSkillIDs: Set(skills.map(\.id)))
    }

    func loadProviderObservabilityIfNeeded() async {
        guard !hasLoadedProviderObservability else { return }
        await loadProviderObservability(force: false, allowDuringRefresh: false)
    }

    func loadProviderObservability() async {
        await loadProviderObservability(force: true, allowDuringRefresh: false)
    }

    private func loadProviderObservabilityDuringRefresh(force: Bool) async {
        await loadProviderObservability(force: force, allowDuringRefresh: true)
    }

    private func loadProviderObservability(force: Bool, allowDuringRefresh: Bool) async {
        guard !isLoadingProviderObservability else { return }
        guard force || !hasLoadedProviderObservability else { return }
        guard allowDuringRefresh || !isRefreshBusy else {
            providerObservabilityResult = .unavailable(reason: UIStrings.operationUnavailableBusy)
            return
        }

        isLoadingProviderObservability = true
        defer { isLoadingProviderObservability = false }

        do {
            providerObservabilityResult = try await service.providerObservability(
                windowDays: 30,
                limit: 24,
                includeHistory: true,
                includeBudgetHints: false,
                includeRetentionRecommendations: false,
                includeEvidence: false
            )
            hasLoadedProviderObservability = true
        } catch {
            providerObservabilityResult = .unavailable(reason: error.localizedDescription)
            hasLoadedProviderObservability = true
        }
    }

    @discardableResult
    func saveAIProviderSettings(draft: AIProviderSettingsDraft) async -> Bool {
        guard !isRefreshBusy else {
            aiProviderErrorMessage = UIStrings.operationUnavailableBusy
            return false
        }
        if let validationMessage = draft.validationMessage {
            aiProviderErrorMessage = validationMessage
            return false
        }

        isSavingAIProvider = true
        aiProviderErrorMessage = nil
        aiProviderMessage = nil
        defer { isSavingAIProvider = false }

        do {
            aiProviderStatus = try await service.saveAIProviderSettings(draft: draft)
            aiProviderTestResult = aiProviderStatus.lastTest
            hasLoadedAIProviderStatus = true
            aiProviderMessage = UIStrings.aiProviderSaved
            return true
        } catch ServiceClient.ClientError.service(let error) where error.code == "unknown_method" {
            aiProviderErrorMessage = UIStrings.aiProviderUnavailable
            return false
        } catch {
            aiProviderErrorMessage = error.localizedDescription
            return false
        }
    }

    @discardableResult
    func testAIProviderConnection(draft: AIProviderSettingsDraft) async -> AIProviderTestResult? {
        guard !isRefreshBusy else {
            aiProviderErrorMessage = UIStrings.operationUnavailableBusy
            return nil
        }
        if let validationMessage = draft.validationMessage {
            aiProviderErrorMessage = validationMessage
            return nil
        }

        isTestingAIProvider = true
        aiProviderErrorMessage = nil
        aiProviderMessage = nil
        defer { isTestingAIProvider = false }

        do {
            let result = try await service.testAIProviderConnection(draft: draft)
            if let refreshedStatus = try? await service.aiProviderStatus() {
                aiProviderStatus = refreshedStatus
                hasLoadedAIProviderStatus = true
            }
            aiProviderTestResult = result
            aiProviderMessage = result.success ? UIStrings.aiProviderTestSucceeded : nil
            if !result.success {
                aiProviderErrorMessage = result.message
            }
            return result
        } catch {
            let result = AIProviderTestResult.unavailable(reason: error.localizedDescription)
            aiProviderTestResult = result
            aiProviderErrorMessage = result.message
            return result
        }
    }

    @discardableResult
    func saveClaudeSettings(content: String) async -> Bool {
        guard !isRefreshBusy else {
            settingsErrorMessage = UIStrings.operationUnavailableBusy
            return false
        }
        isSavingSettings = true
        settingsErrorMessage = nil
        settingsMessage = nil
        defer { isSavingSettings = false }

        do {
            claudeSettings = try await service.saveClaudeSettings(content: content)
            detailsByID.removeAll()
            try await refreshCollections()
            settingsMessage = UIStrings.savedSettings
            lastMutationMessage = settingsMessage
            recordLocalRefresh(message: UIStrings.refreshAfterSettingsSave)
            await loadSelectedDetail()
            return true
        } catch {
            settingsErrorMessage = error.localizedDescription
            return false
        }
    }

    func loadSelectedDetail() async {
        normalizeSelectionToVisibleSkills()
        guard let id = selectedSkill?.id else { return }
        if detailsByID[id] != nil {
            await loadSkillEventsIfNeeded(instanceID: id)
            return
        }

        selectedDetailLoadGeneration += 1
        let generation = selectedDetailLoadGeneration
        isLoadingDetail = true
        errorMessage = nil
        defer {
            if generation == selectedDetailLoadGeneration {
                isLoadingDetail = false
            }
        }

        do {
            let detail = try await service.getSkill(instanceID: id)
            guard generation == selectedDetailLoadGeneration, selectedSkill?.id == id else { return }
            detailsByID[id] = detail
            await loadSkillEventsIfNeeded(instanceID: id)
        } catch {
            guard generation == selectedDetailLoadGeneration, selectedSkill?.id == id else { return }
            errorMessage = error.localizedDescription
        }
    }

    func loadCleanupQueueIfNeeded() async {
        await loadCleanupQueue(force: false)
    }

    func loadCleanupQueue() async {
        await loadCleanupQueue(force: true)
    }

    private func loadCleanupQueue(force: Bool) async {
        let requestedAgentFilter = agentFilter
        let agent = requestedAgentFilter == .all ? nil : requestedAgentFilter.rawValue
        let requestKey = cleanupQueueRequestKey(agent: agent)
        if !force {
            if let cachedResult = cleanupQueueCacheByRequestKey[requestKey] {
                cleanupQueue = cachedResult
                loadedCleanupQueueRequestKey = requestKey
                return
            }
            if loadedCleanupQueueRequestKey == requestKey || activeCleanupQueueRequestKey == requestKey {
                return
            }
        }
        guard activeCleanupQueueRequestKey != requestKey else { return }

        cleanupQueueLoadGeneration += 1
        let generation = cleanupQueueLoadGeneration
        activeCleanupQueueRequestKey = requestKey
        isLoadingCleanupQueue = true
        defer {
            if generation == cleanupQueueLoadGeneration {
                isLoadingCleanupQueue = false
            }
            if activeCleanupQueueRequestKey == requestKey {
                activeCleanupQueueRequestKey = nil
            }
        }

        do {
            let result = try await service.listCleanupQueue(agent: agent, limit: 100)
            guard generation == cleanupQueueLoadGeneration, agentFilter == requestedAgentFilter else { return }
            cleanupQueue = result
            cleanupQueueCacheByRequestKey[requestKey] = result
            loadedCleanupQueueRequestKey = requestKey
        } catch {
            guard generation == cleanupQueueLoadGeneration, agentFilter == requestedAgentFilter else { return }
            let fallback = CleanupQueueResult.emptyFallback(reason: UIStrings.cleanupUnavailableFallback)
            cleanupQueue = fallback
            cleanupQueueCacheByRequestKey[requestKey] = fallback
            loadedCleanupQueueRequestKey = requestKey
        }
    }

    func loadCrossAgentComparisonsIfNeeded() async {
        await loadCrossAgentComparisons(force: false)
    }

    func loadCrossAgentComparisons() async {
        await loadCrossAgentComparisons(force: true)
    }

    private func loadCrossAgentComparisons(force: Bool) async {
        let requestedAgentFilter = agentFilter
        let requestedSelectedSkillID = selectedSkill?.id
        let agent = requestedAgentFilter == .all ? nil : requestedAgentFilter.rawValue
        let requestKey = crossAgentComparisonsRequestKey(agent: agent, selectedSkillID: requestedSelectedSkillID)
        if !force {
            if let cachedResult = crossAgentComparisonsCacheByRequestKey[requestKey] {
                crossAgentComparisons = cachedResult
                loadedCrossAgentComparisonsRequestKey = requestKey
                return
            }
            if loadedCrossAgentComparisonsRequestKey == requestKey || activeCrossAgentComparisonsRequestKey == requestKey {
                return
            }
        }
        guard activeCrossAgentComparisonsRequestKey != requestKey else { return }

        crossAgentComparisonsLoadGeneration += 1
        let generation = crossAgentComparisonsLoadGeneration
        activeCrossAgentComparisonsRequestKey = requestKey
        isLoadingCrossAgentComparisons = true
        defer {
            if generation == crossAgentComparisonsLoadGeneration {
                isLoadingCrossAgentComparisons = false
            }
            if activeCrossAgentComparisonsRequestKey == requestKey {
                activeCrossAgentComparisonsRequestKey = nil
            }
        }

        do {
            let result = try await service.listCrossAgentComparisons(
                agent: agent,
                instanceID: requestedSelectedSkillID,
                limit: 100
            )
            guard generation == crossAgentComparisonsLoadGeneration,
                  agentFilter == requestedAgentFilter,
                  selectedSkill?.id == requestedSelectedSkillID else { return }
            crossAgentComparisons = result
            crossAgentComparisonsCacheByRequestKey[requestKey] = result
            loadedCrossAgentComparisonsRequestKey = requestKey
        } catch {
            guard generation == crossAgentComparisonsLoadGeneration,
                  agentFilter == requestedAgentFilter,
                  selectedSkill?.id == requestedSelectedSkillID else { return }
            let fallback = CrossAgentComparisonResult.local(
                skills: skills,
                findings: findings,
                capabilities: adapterCapabilities,
                agentFilter: requestedAgentFilter,
                reason: UIStrings.crossAgentComparisonLocalFallback
            )
            crossAgentComparisons = fallback
            crossAgentComparisonsCacheByRequestKey[requestKey] = fallback
            loadedCrossAgentComparisonsRequestKey = requestKey
        }
    }

    func loadAgentConfigSnapshotsIfNeeded(agent: String? = nil) async {
        await loadAgentConfigSnapshots(agent: agent, force: false)
    }

    func loadAgentConfigSnapshots(agent: String? = nil) async {
        await loadAgentConfigSnapshots(agent: agent, force: true)
    }

    private func loadAgentConfigSnapshots(agent requestedAgent: String? = nil, force: Bool) async {
        guard let agent = normalizedConfigAgent(requestedAgent) else {
            if !agentConfigSnapshots.isEmpty {
                agentConfigSnapshots = []
                normalizeConfigSelection()
            }
            return
        }

        let requestKey = agentConfigRequestKey(agent: agent)
        if !force {
            if loadedAgentConfigSnapshotRequestKey == requestKey || activeAgentConfigSnapshotRequestKey == requestKey {
                return
            }
        }
        guard activeAgentConfigSnapshotRequestKey != requestKey else { return }

        agentConfigSnapshotLoadGeneration += 1
        let generation = agentConfigSnapshotLoadGeneration
        activeAgentConfigSnapshotRequestKey = requestKey
        isLoadingAgentConfigSnapshots = true
        defer {
            if generation == agentConfigSnapshotLoadGeneration {
                isLoadingAgentConfigSnapshots = false
            }
            if activeAgentConfigSnapshotRequestKey == requestKey {
                activeAgentConfigSnapshotRequestKey = nil
            }
        }

        do {
            let records = try await fetchAgentConfigSnapshots(agent: agent)
            guard generation == agentConfigSnapshotLoadGeneration, normalizedConfigAgent(nil) == agent else { return }
            agentConfigSnapshots = records
            loadedAgentConfigSnapshotRequestKey = requestKey
            normalizeConfigSelection()
        } catch {
            guard generation == agentConfigSnapshotLoadGeneration, normalizedConfigAgent(nil) == agent else { return }
            errorMessage = error.localizedDescription
        }
    }

    private func refreshCollections() async throws {
        let snapshot = try await service.appStateSnapshot()
        async let llmStatusTask = service.llmStatus()
        async let aiProviderStatusTask = fetchAIProviderStatus()
        async let llmPromptRunsTask = fetchLLMPromptRuns()
        async let projectContextTask = service.getProjectContext()
        async let agentConfigSnapshotsTask = fetchAgentConfigSnapshots()
        async let ruleTuningTask = service.listRuleTuning()
        let (
            fetchedLLMStatus,
            fetchedAIProviderStatus,
            fetchedLLMPromptRuns,
            fetchedProjectContextState,
            fetchedAgentConfigSnapshots,
            fetchedRuleTuning
        ) = try await (
            llmStatusTask,
            aiProviderStatusTask,
            llmPromptRunsTask,
            projectContextTask,
            agentConfigSnapshotsTask,
            ruleTuningTask
        )

        self.status = snapshot.status
        self.llmStatus = fetchedLLMStatus
        self.aiProviderStatus = fetchedAIProviderStatus
        self.hasLoadedAIProviderStatus = true
        self.aiProviderTestResult = self.aiProviderStatus.lastTest ?? aiProviderTestResult
        self.llmPromptRunList = fetchedLLMPromptRuns
        self.projectContextState = fetchedProjectContextState
        self.skills = snapshot.skills
        self.findings = snapshot.findings
        self.ruleTuning = fetchedRuleTuning
        self.conflicts = snapshot.conflicts
        self.healthSummary = snapshot.health
        invalidateRuntimeAnalysisCaches()
        self.agentConfigSnapshots = fetchedAgentConfigSnapshots
        if let agent = selectedAgentConfigTimelineAgent {
            loadedAgentConfigSnapshotRequestKey = agentConfigRequestKey(agent: agent)
        } else {
            loadedAgentConfigSnapshotRequestKey = nil
        }
        let currentSkillIDs = Set(snapshot.skills.map(\.id))
        scriptExecutionPreviews = scriptExecutionPreviews.filter { currentSkillIDs.contains($0.key) }
        hydratePromptSendResultsFromRuns(currentSkillIDs: currentSkillIDs)
        skillQualityScores = skillQualityScores.filter { currentSkillIDs.contains($0.key) }
        scoringSkillQualityIDs = scoringSkillQualityIDs.filter { currentSkillIDs.contains($0) }
        if let checkedSkillID = taskReadinessCheckedSkillID, !currentSkillIDs.contains(checkedSkillID) {
            taskReadinessResult = nil
            taskReadinessCheckedSkillID = nil
        }
        checkingTaskReadinessSkillIDs = checkingTaskReadinessSkillIDs.filter { currentSkillIDs.contains($0) }
        if let rankedSkillID = routingConfidenceRankedSkillID, !currentSkillIDs.contains(rankedSkillID) {
            routingConfidenceResult = nil
            routingConfidenceRankedSkillID = nil
        }
        rankingRoutingSkillIDs = rankingRoutingSkillIDs.filter { currentSkillIDs.contains($0) }
        if crossAgentReadinessResult?.isUnavailable == true {
            crossAgentReadinessResult = nil
        }
        if staleDriftDetection?.isUnavailable == true {
            staleDriftDetection = nil
        }
        if knowledgeSearchResult?.isUnavailable == true {
            knowledgeSearchResult = nil
        }
        if localSkillMapResult?.isUnavailable == true {
            localSkillMapResult = nil
        }
        if skillLifecycleTimelineResult?.isUnavailable == true {
            skillLifecycleTimelineResult = nil
        }
        if capabilityTaxonomyResult?.isUnavailable == true {
            capabilityTaxonomyResult = nil
        }
        if workspaceReadinessResult?.isUnavailable == true {
            workspaceReadinessResult = nil
        }
        if guidedCleanupFlowResult?.isUnavailable == true {
            guidedCleanupFlowResult = nil
        }
        if guidedCleanupRecordResult?.isUnavailable == true {
            guidedCleanupRecordResult = nil
        }
        deletingTaskBenchmarkIDs.removeAll()
        if taskBenchmarkList.isUnavailable {
            taskBenchmarkEvaluation = nil
            routingRegressionBaseline = nil
            routingRegressionDetection = nil
        }
        skillEventsByID = skillEventsByID.filter { currentSkillIDs.contains($0.key) }
        skillAnalysisPrepareResults.removeAll()
        preparingSkillAnalysisKeys.removeAll()
        batchTogglePreview = nil
        refreshWatcherMessage(from: self.status)
        normalizeSelectionToVisibleSkills()
        crossAgentComparisons = CrossAgentComparisonResult.local(
            skills: skills,
            findings: findings,
            capabilities: adapterCapabilities,
            agentFilter: agentFilter,
            reason: UIStrings.crossAgentComparisonLocalFallback
        )
    }

    private func fetchAIProviderStatus() async -> AIProviderStatus {
        do {
            return try await service.aiProviderStatus()
        } catch {
            return .unavailable(reason: error.localizedDescription)
        }
    }

    private func fetchLLMPromptRuns() async -> LLMPromptRunListResult {
        do {
            return try await service.listLLMPromptRuns()
        } catch {
            return .unavailable()
        }
    }

    private func normalizedConfigAgent(_ requestedAgent: String?) -> String? {
        let agent = requestedAgent ?? selectedAgentConfigTimelineAgent
        guard let agent, agent != SkillAgentFilter.all.rawValue else { return nil }
        return agent
    }

    private func agentConfigRequestKey(agent: String) -> String {
        [
            agent,
            activeProjectContext?.rootPath ?? "",
            activeProjectContext?.currentCWD ?? ""
        ].joined(separator: "\u{1e}")
    }

    private func cleanupQueueRequestKey(agent: String?) -> String {
        [
            agent ?? SkillAgentFilter.all.rawValue,
            activeProjectContext?.rootPath ?? "",
            activeProjectContext?.currentCWD ?? ""
        ].joined(separator: "\u{1e}")
    }

    private func crossAgentComparisonsRequestKey(agent: String?, selectedSkillID: SkillRecord.ID?) -> String {
        [
            agent ?? SkillAgentFilter.all.rawValue,
            selectedSkillID ?? "",
            activeProjectContext?.rootPath ?? "",
            activeProjectContext?.currentCWD ?? ""
        ].joined(separator: "\u{1e}")
    }

    private func claudeSettingsRequestKey() -> String {
        [
            SkillAgentFilter.claudeCode.rawValue,
            activeProjectContext?.rootPath ?? "",
            activeProjectContext?.currentCWD ?? ""
        ].joined(separator: "\u{1e}")
    }

    private func fetchAgentConfigSnapshots(agent requestedAgent: String? = nil) async throws -> [ConfigSnapshotRecord] {
        guard let agent = normalizedConfigAgent(requestedAgent) else {
            return []
        }
        let records = try await service.listAgentConfigSnapshots(agent: agent, scope: nil)
        return records
            .filter { $0.agent == agent }
            .sorted { $0.createdAt > $1.createdAt }
    }

    private func loadSkillEventsIfNeeded(instanceID: SkillRecord.ID, force: Bool = false) async {
        if !force, skillEventsByID[instanceID] != nil {
            return
        }
        guard !loadingSkillEventIDs.contains(instanceID) else { return }
        loadingSkillEventIDs.insert(instanceID)
        defer { loadingSkillEventIDs.remove(instanceID) }

        do {
            skillEventsByID[instanceID] = try await service.listSkillEvents(instanceID: instanceID, limit: 12)
        } catch {
            skillEventsByID[instanceID] = []
            if errorMessage == nil {
                errorMessage = error.localizedDescription
            }
        }
    }

    private func prepareSkillAnalysis(kind: LLMSkillAnalysisKind, scope: LLMSkillAnalysisRequestScope, instanceIDs: [String]) async {
        let key = skillAnalysisKey(kind: kind, scope: scope)
        guard !isRefreshBusy else {
            skillAnalysisPrepareResults[key] = .unavailable(kind: kind, reason: UIStrings.operationUnavailableBusy)
            return
        }

        preparingSkillAnalysisKeys.insert(key)
        defer { preparingSkillAnalysisKeys.remove(key) }

        do {
            skillAnalysisPrepareResults[key] = try await service.prepareSkillAnalysis(instanceIDs: instanceIDs, kind: kind)
        } catch {
            skillAnalysisPrepareResults[key] = .unavailable(kind: kind, reason: error.localizedDescription)
        }
    }

    private func scoreSkillQuality(for skill: SkillRecord) async {
        guard !isRefreshBusy else {
            skillQualityScores[skill.id] = .unavailable(skillID: skill.id, reason: UIStrings.operationUnavailableBusy)
            return
        }

        scoringSkillQualityIDs.insert(skill.id)
        defer { scoringSkillQualityIDs.remove(skill.id) }

        do {
            skillQualityScores[skill.id] = try await service.scoreSkillQuality(skill: skill)
        } catch {
            skillQualityScores[skill.id] = .unavailable(skillID: skill.id, reason: error.localizedDescription)
        }
    }

    private func skillAnalysisKey(kind: LLMSkillAnalysisKind, scope: LLMSkillAnalysisRequestScope) -> String {
        "\(scope.key):\(kind.rawValue)"
    }

    private func skillAnalysisInstanceIDs(scope: LLMSkillAnalysisRequestScope) -> [String] {
        switch scope.key {
        case LLMSkillAnalysisRequestScope.visible.key:
            return filteredSkills.map(\.id)
        default:
            return selectedSkill.map { [$0.id] } ?? []
        }
    }

    private func llmPromptActionKey(action: LLMAction, skillID: SkillRecord.ID) -> String {
        "action:\(skillID):\(action.rawValue)"
    }

    private func skillAnalysisPromptKey(
        kind: LLMSkillAnalysisKind,
        scope: LLMSkillAnalysisRequestScope,
        instanceIDs: [String]
    ) -> String {
        "skill-analysis:\(scope.key):\(kind.rawValue):\(instanceIDs.joined(separator: ","))"
    }

    private func skillQualityPromptKey(skillID: SkillRecord.ID) -> String {
        "quality-score:\(skillID)"
    }

    private var normalizedTaskReadinessText: String {
        taskReadinessText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private func taskReadinessPromptKey(skillID: SkillRecord.ID, taskText: String) -> String {
        "task-readiness:\(skillID):\(taskText)"
    }

    private var normalizedRoutingConfidenceText: String {
        routingConfidenceText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private func routingConfidencePromptKey(skillID: SkillRecord.ID, taskText: String) -> String {
        "routing-confidence:\(skillID):\(taskText)"
    }

    private var normalizedCrossAgentReadinessText: String {
        crossAgentReadinessText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var normalizedTaskCockpitText: String {
        taskCockpitText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private func clearLocalReportExportState() {
        let hadLocalReportExportResult = localReportExportResult != nil
        localReportExportResult = nil
        selectedLocalReportHistoryID = nil
        if hadLocalReportExportResult {
            lastMutationMessage = nil
        }
    }

    private func clearTaskCockpitTransientState() {
        taskCockpitResult = nil
        selectedTaskCockpitHistoryID = nil
        if isBuildingTaskCockpit {
            cancelTaskCockpitBuild(publishFallbackResult: false)
        } else {
            taskCockpitOperationState = .idle
        }
    }

    private func recordLocalReportExportHistory(result: LocalReportExportResult, scopeSummary: String) {
        guard !result.isUnavailable else { return }
        let record = LocalReportExportHistoryRecord(result: result, scopeSummary: scopeSummary)
        localReportExportHistory.removeAll { $0.id == record.id }
        localReportExportHistory.insert(record, at: 0)
        selectedLocalReportHistoryID = record.id
        if localReportExportHistory.count > 12 {
            localReportExportHistory.removeLast(localReportExportHistory.count - 12)
        }
    }

    private func recordTaskCockpitHistory(result: TaskCockpitResult, taskText: String, agentIDs: [String]) {
        let normalizedTask = taskText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalizedTask.isEmpty, !result.isUnavailable else { return }
        let record = TaskCockpitHistoryRecord(
            taskText: normalizedTask,
            agentIDs: agentIDs,
            result: result,
            operationState: taskCockpitOperationState
        )
        taskCockpitHistory.insert(record, at: 0)
        selectedTaskCockpitHistoryID = record.id
        if taskCockpitHistory.count > TaskCockpitHistoryStore.maxRecords {
            taskCockpitHistory.removeLast(taskCockpitHistory.count - TaskCockpitHistoryStore.maxRecords)
        }
        taskCockpitHistoryStore.save(taskCockpitHistory)
    }

    private func resetTaskCockpitAgentSelectionToSidebarDefault(clearResult: Bool) {
        let defaultAgents: [String]
        if agentFilter == .all {
            defaultAgents = SkillAgentFilter.managementCases.map(\.rawValue)
        } else {
            defaultAgents = [agentFilter.rawValue]
        }
        setTaskCockpitAgentSelection(defaultAgents, clearResult: clearResult)
    }

    private func setTaskCockpitAgentSelection(_ agentIDs: [String], clearResult: Bool) {
        let normalized = normalizedTaskCockpitAgentIDs(agentIDs)
        taskCockpitSelectedAgentIDs = Set(normalized)
        if clearResult {
            clearTaskCockpitTransientState()
        }
    }

    private func normalizedTaskCockpitAgentIDs(_ agentIDs: [String]) -> [String] {
        let orderedAgents = SkillAgentFilter.managementCases.map(\.rawValue)
        let selected = Set(agentIDs.map { $0.trimmingCharacters(in: .whitespacesAndNewlines) })
        return orderedAgents.filter { selected.contains($0) }
    }

    private func taskCockpitCandidateSkillIDs(for agentIDs: [String]) -> [SkillRecord.ID] {
        let selectedAgents = Set(normalizedTaskCockpitAgentIDs(agentIDs))
        guard !selectedAgents.isEmpty else { return [] }
        return skills
            .filter { skill in
                selectedAgents.contains(skill.agent)
                    && DisplayText.statusKind(skill.state, enabled: skill.enabled) == .enabled
            }
            .map(\.id)
    }

    private var roundedTaskCockpitTimeoutSeconds: Int {
        max(1, Int(taskCockpitTimeoutSeconds.rounded(.up)))
    }

    private func scheduleTaskCockpitTimeout(operationID: UUID, taskText: String) {
        taskCockpitTimeoutTask?.cancel()
        let timeoutSeconds = taskCockpitTimeoutSeconds
        taskCockpitTimeoutTask = Task { [weak self] in
            let nanoseconds = UInt64(max(0, timeoutSeconds) * 1_000_000_000)
            try? await Task.sleep(nanoseconds: nanoseconds)
            guard !Task.isCancelled else { return }
            await MainActor.run {
                self?.timeOutTaskCockpitOperation(operationID: operationID, taskText: taskText)
            }
        }
    }

    private func timeOutTaskCockpitOperation(operationID: UUID, taskText: String) {
        guard isCurrentTaskCockpitOperation(operationID) else { return }
        let timeoutSeconds = roundedTaskCockpitTimeoutSeconds
        let message = UIStrings.taskCockpitTimedOut(timeoutSeconds)
        taskCockpitOperationID = nil
        taskCockpitTimeoutTask = nil
        taskCockpitServiceTask?.cancel()
        taskCockpitServiceTask = nil
        isBuildingTaskCockpit = false
        taskCockpitResult = .unavailable(taskText: taskText, reason: message)
        taskCockpitOperationState = taskCockpitOperationState.finished(
            phase: .timedOut,
            message: message
        )
    }

    private func finishTaskCockpitOperation(_ operationID: UUID, phase: TaskCockpitOperationState.Phase, message: String) {
        guard isCurrentTaskCockpitOperation(operationID) else { return }
        taskCockpitTimeoutTask?.cancel()
        taskCockpitTimeoutTask = nil
        taskCockpitServiceTask = nil
        taskCockpitOperationID = nil
        isBuildingTaskCockpit = false
        taskCockpitOperationState = taskCockpitOperationState.finished(
            phase: phase,
            message: message
        )
    }

    private func isCurrentTaskCockpitOperation(_ operationID: UUID) -> Bool {
        taskCockpitOperationID == operationID && isBuildingTaskCockpit
    }

    private var normalizedKnowledgeSearchText: String {
        knowledgeSearchText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var normalizedTaskBenchmarkText: String {
        taskBenchmarkText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var normalizedTraceImportText: String {
        traceImportText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var normalizedAgentSessionSkillReviewTranscript: String {
        agentSessionSkillReviewTranscript.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var normalizedLocalSessionPreviewRoots: [String] {
        localSessionPreviewRoots
            .split(whereSeparator: { $0 == "," || $0 == "\n" || $0 == ";" })
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
    }

    private var normalizedMcpServerPreviewPaths: [String] {
        mcpServerPreviewPaths
            .split(whereSeparator: { $0 == "," || $0 == "\n" || $0 == ";" })
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
    }

    private var normalizedTraceExpectedSkillNames: [String] {
        traceImportExpectedSkills
            .split(whereSeparator: { $0 == "," || $0 == "\n" || $0 == ";" })
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
    }

    private var normalizedAgentSessionExpectedSkillNames: [String] {
        agentSessionSkillReviewExpectedSkills
            .split(whereSeparator: { $0 == "," || $0 == "\n" || $0 == ";" })
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
    }

    private func normalizedOptional(_ value: String) -> String? {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }

    private func upsertTaskBenchmark(_ benchmark: TaskBenchmarkRecord) {
        var benchmarks = taskBenchmarkList.benchmarks.filter { $0.id != benchmark.id }
        benchmarks.insert(benchmark, at: 0)
        taskBenchmarkList = TaskBenchmarkListResult(benchmarks: benchmarks, fallbackReason: nil)
    }

    private func upsertTraceImport(_ record: AgentTraceImportRecord) {
        var imports = traceImportList.imports.filter { $0.id != record.id }
        imports.insert(record, at: 0)
        traceImportList = AgentTraceImportListResult(imports: imports, fallbackReason: nil)
    }

    private func upsertAgentSessionSkillReview(_ record: AgentSessionSkillReviewRecord) {
        var reviews = agentSessionSkillReviewList.reviews.filter { $0.id != record.id }
        reviews.insert(record, at: 0)
        agentSessionSkillReviewList = AgentSessionSkillReviewListResult(reviews: reviews, fallbackReason: nil)
    }

    private func canSendLLMPrompt(_ preview: LLMPromptPreview) -> Bool {
        aiProviderStatus.serviceAvailable
            && aiProviderStatus.configured
            && aiProviderStatus.activeProfile != nil
            && preview.enabled
            && !preview.previewID.isEmpty
            && preview.confirmationRequired
            && !preview.rawPromptPersisted
            && !preview.rawResponsePersisted
    }

    private func hydratePromptSendResultsFromRuns(currentSkillIDs: Set<SkillRecord.ID>) {
        var hydrated = llmPromptSendResults
        for run in llmPromptRunList.runs {
            guard runMatchesCurrentCatalog(run, currentSkillIDs: currentSkillIDs),
                  let key = llmPromptKey(for: run),
                  !sendingLLMPromptKeys.contains(key),
                  !previewingLLMPromptKeys.contains(key)
            else {
                continue
            }
            if hydrated[key] == nil {
                hydrated[key] = run.sendResult
            }
            hydrateTaskInputIfNeeded(from: run)
        }
        llmPromptSendResults = hydrated
    }

    private func runMatchesCurrentCatalog(_ run: LLMPromptRunRecord, currentSkillIDs: Set<SkillRecord.ID>) -> Bool {
        if currentSkillIDs.isEmpty { return true }
        if let instanceID = run.instanceID, currentSkillIDs.contains(instanceID) {
            return true
        }
        return run.instanceIDs.contains { currentSkillIDs.contains($0) }
    }

    private func hydrateTaskInputIfNeeded(from run: LLMPromptRunRecord) {
        guard let selectedSkill, runBelongsTo(run, skillID: selectedSkill.id) else { return }
        switch run.requestKind {
        case "task_readiness":
            if normalizedTaskReadinessText.isEmpty, let task = run.task, !task.isEmpty {
                taskReadinessText = task
            }
        case "routing_confidence":
            if normalizedRoutingConfidenceText.isEmpty, let task = run.task, !task.isEmpty {
                routingConfidenceText = task
            }
        default:
            break
        }
    }

    private func runBelongsTo(_ run: LLMPromptRunRecord, skillID: SkillRecord.ID) -> Bool {
        run.instanceID == skillID || run.instanceIDs.contains(skillID)
    }

    private func latestPromptRun(for skill: SkillRecord, requestKind: String) -> LLMPromptRunRecord? {
        llmPromptRunList.runs.first { run in
            run.requestKind == requestKind && runBelongsTo(run, skillID: skill.id)
        }
    }

    private func llmPromptKey(for run: LLMPromptRunRecord) -> String? {
        let skillID = run.instanceID ?? run.instanceIDs.first
        switch run.requestKind {
        case "quality_score":
            return skillID.map { skillQualityPromptKey(skillID: $0) }
        case "task_readiness":
            guard let skillID, let task = run.task, !task.isEmpty else { return nil }
            return taskReadinessPromptKey(skillID: skillID, taskText: task)
        case "routing_confidence":
            guard let skillID, let task = run.task, !task.isEmpty else { return nil }
            return routingConfidencePromptKey(skillID: skillID, taskText: task)
        case "skill_analysis":
            guard
                let kindValue = run.analysisKind,
                let kind = LLMSkillAnalysisKind(rawValue: kindValue),
                let scopeValue = run.scope,
                let scope = llmSkillAnalysisScope(for: scopeValue)
            else { return nil }
            let instanceIDs = run.instanceIDs.isEmpty ? skillID.map { [$0] } ?? [] : run.instanceIDs
            guard !instanceIDs.isEmpty else { return nil }
            return skillAnalysisPromptKey(kind: kind, scope: scope, instanceIDs: instanceIDs)
        case "action":
            guard let skillID, let action = LLMAction(rawValue: run.action) else { return nil }
            return llmPromptActionKey(action: action, skillID: skillID)
        default:
            guard let skillID, let action = LLMAction(rawValue: run.action) else { return nil }
            return llmPromptActionKey(action: action, skillID: skillID)
        }
    }

    private func llmSkillAnalysisScope(for value: String) -> LLMSkillAnalysisRequestScope? {
        switch value {
        case LLMSkillAnalysisRequestScope.selected.key:
            return .selected
        case LLMSkillAnalysisRequestScope.visible.key:
            return .visible
        default:
            return nil
        }
    }

    private func confirmLLMPrompt(
        key: String,
        send: (String) async throws -> LLMPromptSendResult
    ) async {
        guard let preview = llmPromptPreviews[key] else { return }
        guard canSendLLMPrompt(preview) else {
            llmPromptSendResults[key] = .unavailable(
                previewID: preview.previewID,
                reason: aiProviderStatus.configured ? UIStrings.llmPromptPreviewRequired : UIStrings.llmPromptProviderRequired
            )
            return
        }
        guard !isRefreshBusy else {
            llmPromptSendResults[key] = .unavailable(previewID: preview.previewID, reason: UIStrings.operationUnavailableBusy)
            return
        }

        sendingLLMPromptKeys.insert(key)
        defer { sendingLLMPromptKeys.remove(key) }

        do {
            llmPromptSendResults[key] = try await send(preview.previewID)
            await loadLLMPromptRuns()
        } catch {
            llmPromptSendResults[key] = .unavailable(previewID: preview.previewID, reason: error.localizedDescription)
        }
    }

    private func prepareLLMAction(_ action: LLMAction) async {
        guard !isRefreshBusy else {
            llmPreparedSkillID = selectedSkillID
            llmPrepareResults[action] = .disabledFallback(action: action, reason: UIStrings.operationUnavailableBusy)
            return
        }
        guard let skill = selectedSkill else { return }
        if llmPreparedSkillID != skill.id {
            llmPrepareResults.removeAll()
            llmPreparedSkillID = skill.id
        }

        preparingLLMActions.insert(action)
        defer { preparingLLMActions.remove(action) }

        do {
            llmPrepareResults[action] = try await service.prepareLLMAction(action: action, skill: skill)
        } catch {
            llmPrepareResults[action] = .disabledFallback(action: action, reason: error.localizedDescription)
        }
    }

    private func handleListCriteriaChanged() {
        let previousID = selectedSkillID
        batchTogglePreview = nil
        pruneBatchToggleSelectionToVisibleSkills()
        normalizeSelectionToVisibleSkills()
        guard previousID != selectedSkillID else { return }
        taskReadinessResult = nil
        taskReadinessCheckedSkillID = nil
        routingConfidenceResult = nil
        routingConfidenceRankedSkillID = nil
        taskBenchmarkEvaluation = nil
        taskBenchmarkDeleteResult = nil
        routingRegressionBaseline = nil
        routingRegressionDetection = nil
        localSkillMapResult = nil
        skillLifecycleTimelineResult = nil
        remediationHistoryResult = nil
        remediationHistoryRecordResult = nil
        guidedCleanupFlowResult = nil
        guidedCleanupRecordResult = nil
        agentSessionSkillReviewResult = nil
        agentSessionSkillReviewDeleteResult = nil
        agentSessionSkillReviewList = AgentSessionSkillReviewListResult(reviews: [])
        localSessionPreviewResult = LocalSessionPreviewResult()
        loadedLocalSessionPreviewRequestKey = nil
        activeLocalSessionPreviewRequestKey = nil
        selectedLocalSessionID = nil
        if selectedSidebarSelection?.isSession == true {
            setSidebarSelection(nil)
            selectedDetailSection = .overview
        }
        mcpServerPreviewResult = McpServerPreviewResult()
        listCriteriaDetailTask?.cancel()
        listCriteriaDetailTask = Task { @MainActor [weak self] in
            guard !Task.isCancelled else { return }
            await self?.loadSelectedDetail()
        }
    }

    private func normalizeSelectedLocalSession() {
        let rows = sidebarContentMode == .sessions ? filteredLocalSessionRows : localSessionPreviewResult.sessionRows
        guard !rows.isEmpty else {
            selectedLocalSessionID = nil
            if selectedSidebarSelection?.isSession == true {
                setSidebarSelection(nil)
                selectedDetailSection = .overview
            }
            return
        }
        if let selectedLocalSessionID, rows.contains(where: { $0.id == selectedLocalSessionID }) {
            return
        }
        let firstSessionID = rows[0].id
        selectedLocalSessionID = firstSessionID
        if sidebarContentMode == .sessions,
           selectedSidebarSelection == nil || selectedSidebarSelection?.isSession == true {
            setSidebarSelection(.session(firstSessionID))
        }
    }

    private func pruneBatchToggleSelectionToVisibleSkills() {
        guard isBatchToggleSelectionExplicit else { return }
        let visibleIDs = Set(filteredSkills.map(\.id))
        let prunedSelection = batchToggleSelectedSkillIDs.intersection(visibleIDs)
        if prunedSelection != batchToggleSelectedSkillIDs {
            batchToggleSelectedSkillIDs = prunedSelection
        }
    }

    private func remediationHistoryEvidenceRefs() -> [String] {
        var refs: [String] = []
        refs.append(contentsOf: remediationBatchReviewResult?.evidenceReferences.map(\.detail) ?? [])
        refs.append(contentsOf: remediationImpactPreviewResult?.evidenceReferences.map(\.detail) ?? [])
        refs.append(contentsOf: remediationPreviewDraftsResult?.evidenceReferences.map(\.detail) ?? [])
        refs.append(contentsOf: remediationPlanResult?.evidenceReferences.map(\.detail) ?? [])
        refs.append(contentsOf: remediationBatchReviewResult?.safeNextStepLabels.map { "safe_next_step:\($0)" } ?? [])
        if let selectedSkill {
            refs.append("selected_skill:\(selectedSkill.id)")
        }
        var seen = Set<String>()
        return refs.compactMap { value in
            let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !trimmed.isEmpty, !seen.contains(trimmed) else { return nil }
            seen.insert(trimmed)
            return trimmed
        }
    }

    private func guidedCleanupEvidenceRefs(for step: GuidedCleanupFlowStep) -> [String] {
        var refs: [String] = []
        refs.append("guided_step:\(step.id)")
        refs.append(contentsOf: step.evidenceRefs)
        refs.append(contentsOf: guidedCleanupFlowResult?.evidenceReferences.map(\.detail) ?? [])
        refs.append(contentsOf: guidedCleanupFlowResult?.safeNextActions.map { "safe_action:\($0.id)" } ?? [])
        if let selectedSkill {
            refs.append("selected_skill:\(selectedSkill.id)")
        }
        var seen = Set<String>()
        return refs.compactMap { value in
            let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !trimmed.isEmpty, !seen.contains(trimmed) else { return nil }
            seen.insert(trimmed)
            return trimmed
        }
    }

    private func normalizeSelectionToVisibleSkills() {
        let visibleSkills = filteredSkills
        if let selectedSkillID, visibleSkills.contains(where: { $0.id == selectedSkillID }) {
            if sidebarContentMode == .skills, selectedSidebarSelection == nil {
                setSidebarSelection(.skill(selectedSkillID))
            }
            return
        }
        let nextSkillID = visibleSkills.first?.id
        setSelectedSkillID(
            nextSkillID,
            syncSidebar: selectedSidebarSelection?.isSkill == true
        )
        if sidebarContentMode == .skills, selectedSidebarSelection == nil, let nextSkillID {
            setSidebarSelection(.skill(nextSkillID))
        }
    }

    private func handleSidebarSelectionChanged() {
        guard !isSynchronizingSidebarSelection else { return }

        guard let selectedSidebarSelection else {
            selectedDetailSection = .overview
            return
        }

        switch selectedSidebarSelection {
        case .work(let section):
            if section.requiresSelectedSkill, selectedSkillID == nil {
                setSelectedSkillID(filteredSkills.first?.id ?? skills.first?.id, syncSidebar: false)
            }
            selectedDetailSection = section
        case .session(let id):
            if selectedLocalSessionID != id {
                selectedLocalSessionID = id
            }
            selectedDetailSection = .overview
        case .skill(let id):
            setSelectedSkillID(id, syncSidebar: false)
            if selectedDetailSection.isAgentWorkspaceSurface {
                selectedDetailSection = .overview
            }
        case .configOverview:
            selectedDetailSection = .overview
        case .configDocument(let target):
            if currentAgentConfigDocuments.contains(where: { $0.target == target }) {
                selectedDetailSection = .overview
            } else {
                setSidebarSelection(.configOverview)
                selectedDetailSection = .overview
            }
        case .configSnapshot(let id):
            if agentConfigSnapshots.contains(where: { $0.id == id }) {
                selectedDetailSection = .overview
            } else {
                setSidebarSelection(.configOverview)
                selectedDetailSection = .overview
            }
        }
    }

    private func synchronizeSidebarSelectionWithSelectedSkill() {
        guard !isSynchronizingSidebarSelection else { return }

        guard sidebarContentMode == .skills else {
            if selectedSidebarSelection?.isSkill == true {
                setSidebarSelection(nil)
            }
            return
        }

        if selectedSidebarSelection?.isSkill == true, let selectedSkillID {
            guard selectedSidebarSelection != .skill(selectedSkillID) else { return }
            setSidebarSelection(.skill(selectedSkillID))
        } else if selectedSidebarSelection?.isSkill == true {
            setSidebarSelection(nil)
            selectedDetailSection = .overview
        }
    }

    private func handleSidebarContentModeChanged() {
        guard !isSynchronizingSidebarSelection else { return }

        switch sidebarContentMode {
        case .sessions:
            normalizeSelectedLocalSession()
        case .skills:
            if selectedSidebarSelection?.isSession == true {
                if let skill = selectedSkill {
                    setSidebarSelection(.skill(skill.id))
                } else {
                    setSidebarSelection(nil)
                    selectedDetailSection = .overview
                }
            } else if selectedSidebarSelection?.isConfig == true {
                if let skill = selectedSkill {
                    setSidebarSelection(.skill(skill.id))
                } else {
                    setSidebarSelection(nil)
                    selectedDetailSection = .overview
                }
            }
        case .config:
            selectDefaultConfigDocumentOrOverview()
        }
    }

    private func normalizeConfigSelection() {
        switch selectedSidebarSelection {
        case .configDocument(let target):
            let visible = visibleConfigDocuments.contains { $0.target == target }
            if !visible {
                selectDefaultConfigDocumentOrOverview()
            }
        case .configSnapshot(let id):
            let visible = agentConfigSnapshots.contains { snapshot in
                snapshot.id == id
                    && (agentFilter == .all || snapshot.agent == agentFilter.rawValue)
                    && configScopeFilter.includes(snapshot)
                    && configSnapshotMatchesSidebarQuery(snapshot)
            }
            if !visible {
                selectDefaultConfigDocumentOrOverview()
            }
        case .configOverview, nil:
            selectDefaultConfigDocumentIfVisible()
        default:
            return
        }
    }

    private func selectDefaultConfigDocumentOrOverview() {
        if !selectDefaultConfigDocumentIfVisible() {
            setSidebarSelection(.configOverview)
            selectedDetailSection = .overview
        }
    }

    @discardableResult
    private func selectDefaultConfigDocumentIfVisible() -> Bool {
        guard sidebarContentMode == .config,
              let firstDocument = visibleConfigDocuments.first
        else {
            return false
        }
        setSidebarSelection(.configDocument(firstDocument.target))
        selectedDetailSection = .overview
        return true
    }

    private func setSelectedSkillID(_ id: SkillRecord.ID?, syncSidebar: Bool) {
        guard selectedSkillID != id else {
            if syncSidebar, sidebarContentMode == .skills, let id, selectedSidebarSelection != .skill(id) {
                setSidebarSelection(.skill(id))
            } else if syncSidebar, id == nil, selectedSidebarSelection?.isSkill == true {
                setSidebarSelection(nil)
            }
            return
        }
        if syncSidebar {
            selectedSkillID = id
        } else {
            isSynchronizingSidebarSelection = true
            selectedSkillID = id
            isSynchronizingSidebarSelection = false
        }
    }

    private func setSidebarSelection(_ selection: SidebarSelection?) {
        guard selectedSidebarSelection != selection else { return }
        isSynchronizingSidebarSelection = true
        selectedSidebarSelection = selection
        isSynchronizingSidebarSelection = false
    }

    private func canStartScan(allowDuringProjectUpdate: Bool) -> Bool {
        if isLoading || isScanning || isWriting || isSavingSettings || isApplyingBatchToggle {
            return false
        }
        if isProjectUpdating, !allowDuringProjectUpdate {
            return false
        }
        return true
    }

    private func localBatchTogglePreview(selectedSkills: [SkillRecord], reason: String) -> BatchTogglePreview {
        var affected: [BatchToggleSkillItem] = []
        var skipped: [BatchToggleSkillItem] = []
        for skill in selectedSkills {
            if let skipReason = batchToggleSkipReason(for: skill) {
                skipped.append(BatchToggleSkillItem(skill: skill, targetEnabled: batchToggleAction.targetEnabled, reason: skipReason))
            } else if DisplayText.statusKind(skill.state, enabled: skill.enabled) == (batchToggleAction.targetEnabled ? .enabled : .disabled) {
                skipped.append(BatchToggleSkillItem(skill: skill, targetEnabled: batchToggleAction.targetEnabled, reason: UIStrings.batchToggleAlreadyInTargetState(batchToggleAction.title.lowercased())))
            } else {
                affected.append(BatchToggleSkillItem(skill: skill, targetEnabled: batchToggleAction.targetEnabled))
            }
        }
        return .local(
            action: batchToggleAction,
            selectedSkills: selectedSkills,
            affectedSkills: affected,
            skippedItems: skipped,
            reason: reason
        )
    }

    private func batchToggleSkipReason(for skill: SkillRecord) -> String? {
        if let catalogReason = DisplayText.catalogToggleDisabledReason(for: skill, isWriting: false) {
            return catalogReason
        }
        guard let capability = adapterCapabilities.first(where: { $0.agent == skill.agent }) else {
            return UIStrings.batchToggleCapabilityMissing(DisplayText.agent(skill.agent))
        }
        if !capability.configToggle.supported {
            return capability.configToggle.reason ?? UIStrings.readOnlyAdapterStatus(capability.displayName)
        }
        if !capability.writable.supported {
            return capability.writable.reason ?? UIStrings.batchToggleWritableMissing(capability.displayName)
        }
        return nil
    }

    private func beginRefresh(_ action: RefreshAction, message: String) {
        lastRefreshAction = action
        canRetryLastRefresh = false
        refreshStatusMessage = message
        appendRefreshLog(level: "info", message: message)
    }

    private func applyRefreshActivity(_ activity: RefreshActivity?) {
        if let activity {
            lastScanActivity = activity
            refreshStatusMessage = UIStrings.refreshScanComplete(
                activity.scannedCount,
                activity.skillCount,
                activity.findingCount,
                sameAgentRuntimeConflictCount
            )
            refreshLogEntries = activity.logEntries + refreshLogEntries
            trimRefreshLog()
        } else {
            refreshStatusMessage = UIStrings.refreshScanComplete(
                skills.count,
                skills.count,
                findings.count,
                sameAgentRuntimeConflictCount
            )
            appendRefreshLog(level: "info", message: refreshStatusMessage)
        }
        canRetryLastRefresh = false
    }

    private func recordLocalRefresh(message: String) {
        refreshStatusMessage = message
        appendRefreshLog(level: "info", message: message)
        canRetryLastRefresh = false
    }

    private func handleRefreshFailure(_ error: Error, action: RefreshAction) {
        let message = UIStrings.refreshFailed(error.localizedDescription)
        errorMessage = message
        refreshStatusMessage = message
        appendRefreshLog(level: "error", message: message)
        lastRefreshAction = action
        canRetryLastRefresh = true
    }

    private func refreshWatcherMessage(from status: ServiceStatus?) {
        guard let refresh = status?.refresh else {
            watcherStatusMessage = UIStrings.refreshWatcherManual
            return
        }
        watcherStatusMessage = refresh.watcherDetail
    }

    private func appendRefreshLog(level: String, message: String) {
        refreshLogEntries.insert(RefreshLogEntry(level: level, message: message), at: 0)
        trimRefreshLog()
    }

    private func trimRefreshLog() {
        if refreshLogEntries.count > 6 {
            refreshLogEntries = Array(refreshLogEntries.prefix(6))
        }
    }

}

#if DEBUG
extension SkillStore {
    func setProjectContextForTesting(_ state: ProjectContextState?) {
        projectContextState = state
    }
}
#endif

private enum RefreshAction {
    case reload
    case scan
}
