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

struct SkillListScrollRequest: Equatable {
    let skillID: SkillRecord.ID
    let token = UUID()
}

struct TaskCockpitPromptConfirmation: Identifiable, Hashable {
    let preview: LLMPromptPreview
    let taskText: String
    let agentIDs: [String]
    let instanceIDs: [String]

    var id: String {
        preview.previewID.isEmpty ? taskText : preview.previewID
    }
}

struct ProviderActivityFilterKey: Hashable {
    let provider: String?
    let model: String?
    let action: String?
    let windowDays: Int?
    let startAt: Int?
    let endAt: Int?
}

@MainActor
final class SkillStore: ObservableObject {
    private static let lastMutationMessageDismissDelayNanoseconds: UInt64 = 3_500_000_000
    private static let localSessionPageLimit = 100
    private static let localSessionPrewarmLimit = 800
    private static let globalSearchLimitPerKind = 6
    private static let providerObservabilityRowLimit = 100
    static let providerActivityPageLimit = 50

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
    @Published private(set) var healthSummary = SkillHealthSummary.empty
    @Published private(set) var agentConfigSnapshots: [ConfigSnapshotRecord] = []
    @Published private(set) var isLoadingAgentConfigSnapshots = false
    @Published private(set) var agentConfigSnapshotCompleteness = ListPageAccumulator<ConfigSnapshotRecord>().state
    @Published private(set) var detailsByID: [SkillRecord.ID: SkillDetailRecord] = [:]
    @Published private(set) var skillEventsByID: [SkillRecord.ID: [SkillEventRecord]] = [:]
    @Published private(set) var skillEventCompletenessByID: [SkillRecord.ID: ListCompletenessState] = [:]
    private var skillEventLoadGenerationValue = 0
    private(set) var adoptingAgentSummaryBySkillID: [SkillRecord.ID: String] = [:]
    @Published private(set) var loadingSkillEventIDs: Set<SkillRecord.ID> = []
    @Published private(set) var status: ServiceStatus?
    @Published private(set) var llmStatus = LLMStatus.disabledFallback()
    @Published private(set) var aiProviderStatus = AIProviderStatus.unavailable()
    @Published private(set) var aiProviderTestResult: AIProviderTestResult?
    @Published private(set) var llmPrepareResults: [LLMAction: LLMPrepareResult] = [:]
    @Published private(set) var preparingLLMActions: Set<LLMAction> = []
    @Published private(set) var localSessionPreviewResult = LocalSessionPreviewResult() {
        didSet { invalidateScopedLocalSessionSummaryCache() }
    }
    @Published private(set) var localSessionLoadState: LocalSessionLoadState = .empty
    @Published private(set) var localSessionCompleteness = ListCompletenessState(
        loadedCount: 0,
        totalCount: nil,
        hasMore: false,
        isComplete: false,
        completeness: .unknown,
        incompleteReason: nil,
        loadingPhase: .idle,
        canLoadMore: false,
        canLoadAll: false
    )
    @Published private(set) var selectedLocalSessionDetailState: LocalSessionDetailState?
    @Published private(set) var appSearchResult = AppSearchResult.empty()
    @Published private(set) var skillListScrollRequest: SkillListScrollRequest?
    @Published private(set) var isPreviewingLocalSessions = false
    @Published private(set) var isSearchingApp = false
    @Published private(set) var llmPromptPreviews: [String: LLMPromptPreview] = [:]
    @Published private(set) var previewingLLMPromptKeys: Set<String> = []
    @Published private(set) var sendingLLMPromptKeys: Set<String> = []
    @Published private(set) var llmPromptSendResults: [String: LLMPromptSendResult] = [:]
    @Published private(set) var llmPromptRunList = LLMPromptRunListResult.unavailable()
    @Published private(set) var isLoadingLLMPromptRuns = false
    @Published private(set) var providerObservabilityResult: ProviderObservabilityResult?
    @Published private(set) var isLoadingProviderObservability = false
    @Published var providerActivityRows: [ProviderActivityRow] = []
    @Published var providerActivityCompleteness = ListPageAccumulator<ProviderActivityRow>().state
    @Published var providerActivityErrorMessage: String?
    @Published var providerObservabilityDateRange: ProviderObservabilityDateRangePreset = .last30Days {
        didSet {
            guard oldValue != providerObservabilityDateRange else { return }
            scheduleProviderObservabilityCriteriaRefresh()
        }
    }
    @Published var providerObservabilityCustomStartDate: Date = Calendar.current.date(byAdding: .day, value: -30, to: Date()) ?? Date() {
        didSet {
            guard oldValue != providerObservabilityCustomStartDate else { return }
            guard providerObservabilityDateRange == .custom else { return }
            scheduleProviderObservabilityCriteriaRefresh()
        }
    }
    @Published var providerObservabilityCustomEndDate: Date = Date() {
        didSet {
            guard oldValue != providerObservabilityCustomEndDate else { return }
            guard providerObservabilityDateRange == .custom else { return }
            scheduleProviderObservabilityCriteriaRefresh()
        }
    }
    @Published private(set) var taskCockpitResult: TaskCockpitResult?
    @Published private(set) var taskCockpitHistory: [TaskCockpitHistoryRecord] = []
    @Published private(set) var selectedTaskCockpitHistoryID: TaskCockpitHistoryRecord.ID?
    @Published private(set) var taskCockpitHistoryCleanupMessage: String?
    @Published private(set) var taskCockpitSelectedAgentIDs: Set<String> = [SkillAgentFilter.claudeCode.rawValue]
    @Published private(set) var taskCockpitPromptConfirmation: TaskCockpitPromptConfirmation?
    @Published private(set) var isPreviewingTaskCockpitPrompt = false
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
    @Published private(set) var skillManagerSearchVisibility = SkillManagerVisibleResults<String>()
    @Published private(set) var skillManagerMutationConfirmation: SkillManagerMutationConfirmation?
    @Published private(set) var skillManagerLocalCreateConfirmation: SkillManagerLocalCreateConfirmation?
    @Published private(set) var skillManagerLocalDeleteConfirmation: SkillManagerLocalDeleteConfirmation?
    @Published private(set) var skillManagerErrorMessage: String?
    @Published private(set) var skillManagerMessage: String?
    @Published private(set) var isLoadingSkillManagerTools = false
    @Published private(set) var isSearchingSkillManager = false
    @Published private(set) var isListingSkillManagerInstalled = false
    @Published private(set) var isPreviewingSkillManagerMutation = false
    @Published private(set) var isPreviewingSkillManagerLocalCreate = false
    @Published private(set) var isPreviewingSkillManagerLocalDelete = false
    @Published private(set) var isApplyingSkillManagerMutation = false
    @Published var skillManagerSearchQuery = "" {
        didSet {
            guard oldValue != skillManagerSearchQuery else { return }
            invalidateSkillManagerSearch()
        }
    }
    @Published var skillManagerOwner = "" {
        didSet {
            guard oldValue != skillManagerOwner else { return }
            invalidateSkillManagerSearch()
        }
    }
    @Published var skillManagerSource = "" {
        didSet {
            guard oldValue != skillManagerSource else { return }
            invalidateSkillManagerMutationPreview()
        }
    }
    @Published var skillManagerSkillName = "" {
        didSet {
            guard oldValue != skillManagerSkillName else { return }
            invalidateSkillManagerMutationPreview()
        }
    }
    @Published var skillManagerInstallSkillName = "" {
        didSet {
            guard oldValue != skillManagerInstallSkillName else { return }
            invalidateSkillManagerMutationPreview()
        }
    }
    @Published var skillManagerRemoveSkillName = "" {
        didSet {
            guard oldValue != skillManagerRemoveSkillName else { return }
            invalidateSkillManagerMutationPreview()
        }
    }
    @Published var skillManagerLocalSkillName = "" {
        didSet {
            guard oldValue != skillManagerLocalSkillName else { return }
            invalidateSkillManagerLocalCreatePreview()
        }
    }
    @Published var skillManagerNetworkAllowed = false {
        didSet {
            guard oldValue != skillManagerNetworkAllowed else { return }
            invalidateSkillManagerSearch()
            invalidateSkillManagerMutationPreview()
        }
    }
    @Published var skillManagerScope: SkillManagerScope = .project {
        didSet {
            guard oldValue != skillManagerScope else { return }
            invalidateSkillManagerInstalledList()
            invalidateSkillManagerMutationPreview()
        }
    }
    @Published var skillManagerDistribution: SkillManagerDistribution = .symlink {
        didSet {
            guard oldValue != skillManagerDistribution else { return }
            invalidateSkillManagerMutationPreview()
        }
    }
    @Published var skillManagerSelectedAgentIDs: Set<String> = Set(SkillManagerAgent.defaultTargets.map(\.rawValue)) {
        didSet {
            guard oldValue != skillManagerSelectedAgentIDs else { return }
            invalidateSkillManagerInstalledList()
            invalidateSkillManagerMutationPreview()
        }
    }
    @Published private(set) var projectContextState: ProjectContextState? {
        didSet {
            invalidateScopedLocalSessionSummaryCache()
            activateLocalSessionSourceCache()
        }
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
    @Published private(set) var configAutosavePhase: RevisionAutosavePhase = .idle
    @Published private(set) var providerAutosavePhase: RevisionAutosavePhase = .idle
    @Published private(set) var configAutosaveDraft: String?
    @Published private(set) var providerAutosaveDraft: AIProviderSettingsDraft?
    @Published private(set) var lastMutationMessage: String? {
        didSet { scheduleLastMutationMessageDismissal() }
    }
    @Published private(set) var refreshStatusMessage = UIStrings.refreshIdle
    @Published private(set) var partialScanWarningMessage: String?
    @Published private(set) var watcherStatusMessage = UIStrings.refreshWatcherManual
    @Published private(set) var refreshLogEntries: [RefreshLogEntry] = []
    @Published private(set) var lastScanActivity: RefreshActivity?
    @Published private(set) var catalogListCompleteness = ListCompletenessState(
        loadedCount: 0,
        totalCount: nil,
        hasMore: false,
        isComplete: false,
        completeness: .unknown,
        incompleteReason: nil,
        loadingPhase: .idle,
        canLoadMore: false,
        canLoadAll: false
    )
    @Published private(set) var canRetryLastRefresh = false
    @Published private(set) var claudeSettings: ConfigDocumentRecord?
    @Published private(set) var configMutationState: ConfigMutationState = .idle
    @Published private(set) var rollbackConfirmation: RollbackConfirmation?
    @Published private(set) var currentAgentConfigDocuments: [ConfigDocumentRecord] = []
    @Published private(set) var isLoadingAgentConfigDocuments = false
    @Published private(set) var settingsMessage: String?
    @Published private(set) var settingsErrorMessage: String?
    @Published private(set) var aiProviderMessage: String?
    @Published private(set) var aiProviderErrorMessage: String?
    @Published var selectedSidebarSelection: SidebarSelection? {
        didSet {
            guard oldValue != selectedSidebarSelection else { return }
            clearRollbackConfirmation()
            handleSidebarSelectionChanged()
        }
    }
    @Published var selectedSkillID: SkillRecord.ID? {
        didSet {
            guard oldValue != selectedSkillID else { return }
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
            handleListCriteriaChanged()
        }
    }
    @Published var agentFilter: SkillAgentFilter = .claudeCode {
        didSet {
            guard oldValue != agentFilter else { return }
            handleListCriteriaChanged()
            clearTaskCockpitTransientState()
            resetTaskCockpitAgentSelectionToSidebarDefault(clearResult: false)
            activateLocalSessionSourceCache()
            if sidebarContentMode == .config {
                selectedSidebarSelection = .configOverview
            }
            scheduleAgentFilterDependentLoads()
        }
    }
    @Published var stateFilter: SkillStateFilter = .all {
        didSet {
            guard oldValue != stateFilter else { return }
            handleListCriteriaChanged()
        }
    }
    @Published var skillScopeFilter: SkillScopeFilter = .all {
        didSet {
            guard oldValue != skillScopeFilter else { return }
            handleListCriteriaChanged()
        }
    }
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
    @Published var sortOrder: SkillSortOrder = .name {
        didSet {
            guard oldValue != sortOrder else { return }
            handleListCriteriaChanged()
        }
    }
    @Published var sortDirection: SkillSortDirection = .ascending {
        didSet {
            guard oldValue != sortDirection else { return }
            handleListCriteriaChanged()
        }
    }
    @Published var taskCockpitText = "" {
        didSet {
            if oldValue != taskCockpitText {
                clearTaskCockpitTransientState()
            }
        }
    }
    @Published var localSessionPreviewRoots = "" {
        didSet {
            guard oldValue != localSessionPreviewRoots else { return }
            activateLocalSessionSourceCache()
            guard hasCompletedStartupLoad else { return }
            Task { @MainActor [weak self] in
                await self?.refreshLocalSessionSnapshot(reason: .sourceChanged)
            }
        }
    }
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
            invalidateScopedLocalSessionSummaryCache()
            normalizeSelectedLocalSession()
        }
    }
    @Published var localSessionSortDirection: SkillSortDirection = .descending {
        didSet {
            guard oldValue != localSessionSortDirection else { return }
            invalidateScopedLocalSessionSummaryCache()
            normalizeSelectedLocalSession()
        }
    }
    @Published var localSessionSearchText = "" {
        didSet {
            guard oldValue != localSessionSearchText else { return }
            guard sidebarContentMode == .sessions else { return }
            invalidateScopedLocalSessionSummaryCache()
            normalizeSelectedLocalSession()
        }
    }
    @Published var selectedLocalSessionID: LocalSessionPreviewRow.ID?
    @Published var errorMessage: String? {
        didSet { scheduleErrorMessageDismissal() }
    }

    var supportsConfigConsistencyProtocol: Bool {
        (status?.protocolVersion ?? 0) >= 2
    }

    let service: ServiceClient
    private var lastRefreshAction: RefreshAction = .reload
    private var llmPreparedSkillID: SkillRecord.ID?
    private var agentConfigSnapshotLoadGeneration = 0
    private var agentConfigSnapshotAccumulator = ListPageAccumulator<ConfigSnapshotRecord>()
    private var skillEventAccumulatorsByID: [SkillRecord.ID: ListPageAccumulator<SkillEventRecord>] = [:]
    private var skillEventLoadGenerations: [SkillRecord.ID: Int] = [:]
    private var rollbackPreviewGeneration = 0
    private var agentConfigDocumentLoadGeneration = 0
    private var claudeSettingsLoadGeneration = 0
    private var selectedDetailLoadGeneration = 0
    private var loadedAgentConfigSnapshotRequestKey: String?
    private var activeAgentConfigSnapshotRequestKey: String?
    private var loadedAgentConfigDocumentRequestKey: String?
    private var activeAgentConfigDocumentRequestKey: String?
    private var loadedClaudeSettingsRequestKey: String?
    private var activeClaudeSettingsRequestKey: String?
    private var skillManagerSearchGenerationValue: UInt64 = 0
    private var skillManagerInstalledGenerationValue: UInt64 = 0
    private var skillManagerMutationGenerationValue: UInt64 = 0
    private var skillManagerLocalCreateGenerationValue: UInt64 = 0
    private var skillManagerLocalDeleteGenerationValue: UInt64 = 0
    private var currentSkillManagerSearchGeneration: SkillManagerRequestGeneration?
    private var currentSkillManagerInstalledGeneration: SkillManagerRequestGeneration?
    private var currentSkillManagerMutationGeneration: SkillManagerRequestGeneration?
    private var currentSkillManagerLocalCreateGeneration: SkillManagerRequestGeneration?
    private var currentSkillManagerLocalDeleteGeneration: SkillManagerRequestGeneration?
    private var skillManagerSearchTask: SkillManagerRequestTaskHandle?
    private var skillManagerInstalledTask: SkillManagerRequestTaskHandle?
    private var skillManagerMutationTask: SkillManagerRequestTaskHandle?
    private var skillManagerLocalCreateTask: SkillManagerRequestTaskHandle?
    private var skillManagerLocalDeleteTask: SkillManagerRequestTaskHandle?
    // A confirmed write owns the Store until its result is known; caller cancellation must not
    // interrupt an external mutation after the service RPC has started.
    private var skillManagerApplyTask: Task<Void, Never>?
    private var hasLoadedAIProviderStatus = false
    private var hasLoadedProviderObservability = false
    var providerActivityAccumulators: [ProviderActivityFilterKey: ListPageAccumulator<ProviderActivityRow>] = [:]
    var providerActivityGenerations: [ProviderActivityFilterKey: UInt64] = [:]
    var activeProviderActivityFilterKey: ProviderActivityFilterKey?
    var providerActivityPageTask: Task<ProviderActivityPageResult, Error>?
    var providerActivityPageRequestID: UUID?
    private var taskCockpitOperationID: UUID?
    private var lastMutationMessageDismissTask: Task<Void, Never>?
    private var errorMessageDismissTask: Task<Void, Never>?
    private var agentFilterLoadTask: Task<Void, Never>?
    private var listCriteriaDetailTask: Task<Void, Never>?
    private var localSessionDetailTask: Task<Void, Never>?
    private var localSessionLoadAllTask: Task<Void, Never>?
    private var localSessionLoadAllID: UUID?
    private var appSearchTask: Task<Void, Never>?
    private var providerObservabilityCriteriaTask: Task<Void, Never>?
    private var postRefreshSupplementalLoadTask: Task<Void, Never>?
    private var appSearchQuery = ""
    private var taskCockpitTimeoutTask: Task<Void, Never>?
    private var taskCockpitServiceTask: Task<TaskCockpitResult, Error>?
    private var isSynchronizingSidebarSelection = false
    var filteredSkillListDataRevision = 0
    var filteredSkillListCache: FilteredSkillListCache?
    var isAdoptingAgentSummaryCacheValid = false
    var scopedLocalSessionSummaryRevision = 0
    var scopedLocalSessionSummaryCache: ScopedLocalSessionSummaryCache?
    private let localSessionCache = LocalSessionCache()
    private var activeLocalSessionSnapshotKey: LocalSessionSnapshotKey?
    private var activeLocalSessionRefreshGeneration: UInt64?
    private let taskCockpitTimeoutSeconds: TimeInterval
    private let taskCockpitHistoryStore: TaskCockpitHistoryStore
    private let autosaveDelayNanoseconds: UInt64
    private let autosaveMutationLane = AutosaveMutationLane()
    private var configAutosaveAgentByRevision: [UInt64: String] = [:]
    private var configAutosaveCommittedRevisionByRevision: [UInt64: String] = [:]
    private var latestConfigAutosaveRevision: UInt64?
    private var latestProviderAutosaveRevision: UInt64?
    private lazy var configAutosaveCoordinator = RevisionAutosaveCoordinator<ConfigSaveBinding>(
        delayNanoseconds: autosaveDelayNanoseconds,
        workerWillStart: { [weak self] revision in
            self?.autosaveMutationLane.register(
                AutosaveMutationLaneToken(family: .config, revision: revision)
            )
        },
        save: { [weak self] binding, revision in
            guard let lane = self?.autosaveMutationLane else { return .cancelled }
            let result = await lane.perform(
                token: AutosaveMutationLaneToken(family: .config, revision: revision)
            ) { [weak self] in
                guard let self else { return false }
                let submittedAgent = self.configAutosaveAgentByRevision[revision]
                    ?? SkillAgentFilter.claudeCode.rawValue
                return await self.saveClaudeSettingsInsideMutationLane(
                    binding: binding,
                    submittedAgent: submittedAgent,
                    autosaveRevision: revision
                )
            }
            switch result {
            case .completed(true): return .succeeded
            case .completed(false): return .failed
            case .cancelled: return .cancelled
            }
        },
        phaseChanged: { [weak self] phase in
            self?.configAutosavePhase = phase
        },
        completion: { [weak self] completion in
            self?.handleConfigAutosaveCompletion(completion)
        }
    )
    private lazy var providerAutosaveCoordinator = RevisionAutosaveCoordinator<AIProviderSettingsDraft>(
        delayNanoseconds: autosaveDelayNanoseconds,
        workerWillStart: { [weak self] revision in
            self?.autosaveMutationLane.register(
                AutosaveMutationLaneToken(family: .provider, revision: revision)
            )
        },
        save: { [weak self] draft, revision in
            guard let lane = self?.autosaveMutationLane else { return .cancelled }
            let result = await lane.perform(
                token: AutosaveMutationLaneToken(family: .provider, revision: revision)
            ) { [weak self] in
                guard let self else { return false }
                return await self.saveAIProviderSettingsInsideMutationLane(
                    draft: draft,
                    autosaveRevision: revision
                )
            }
            switch result {
            case .completed(true): return .succeeded
            case .completed(false): return .failed
            case .cancelled: return .cancelled
            }
        },
        phaseChanged: { [weak self] phase in
            self?.providerAutosavePhase = phase
        },
        completion: { [weak self] completion in
            self?.handleProviderAutosaveCompletion(completion)
        }
    )

    init(
        service: ServiceClient,
        taskCockpitTimeoutSeconds: TimeInterval = 300,
        taskCockpitHistoryStore: TaskCockpitHistoryStore = TaskCockpitHistoryStore(),
        autosaveDelayNanoseconds: UInt64 = UIOptimizationPresentation.configEditor.autosaveDelayNanoseconds
    ) {
        self.service = service
        self.taskCockpitTimeoutSeconds = max(0.05, taskCockpitTimeoutSeconds)
        self.taskCockpitHistoryStore = taskCockpitHistoryStore
        self.autosaveDelayNanoseconds = autosaveDelayNanoseconds
        taskCockpitHistory = []
        do {
            _ = try taskCockpitHistoryStore.purgeLegacyHistoryIfPresent()
        } catch {
            taskCockpitHistoryCleanupMessage = UIStrings.taskCockpitHistoryCleanupFailed
        }
    }

    deinit {
        skillManagerSearchTask?.cancel()
        skillManagerInstalledTask?.cancel()
        skillManagerMutationTask?.cancel()
        skillManagerLocalCreateTask?.cancel()
        skillManagerLocalDeleteTask?.cancel()
        localSessionDetailTask?.cancel()
        localSessionLoadAllTask?.cancel()
        providerActivityPageTask?.cancel()
        let lane = autosaveMutationLane
        Task { @MainActor in
            lane.shutdown()
        }
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
        }
    }

    private func scheduleProviderObservabilityCriteriaRefresh() {
        guard hasCompletedStartupLoad else { return }
        providerObservabilityCriteriaTask?.cancel()
        providerObservabilityCriteriaTask = Task { @MainActor [weak self] in
            try? await Task.sleep(nanoseconds: 250_000_000)
            guard let self, !Task.isCancelled else { return }
            await self.loadProviderObservability(force: true, allowDuringRefresh: true)
        }
    }

    private func scheduleStartupSupplementalLoads(
        agentFilter requestedAgentFilter: SkillAgentFilter,
        shouldLoadClaudeSettings: Bool
    ) {
        schedulePostRefreshSupplementalLoads(
            agentFilter: requestedAgentFilter,
            loadLocalSessions: true,
            loadAgentConfigDocuments: true,
            loadClaudeSettings: shouldLoadClaudeSettings,
            forceAgentConfigSnapshots: false,
            forceAIProviderStatus: false,
            forceProviderObservability: false
        )
    }

    private func scheduleReloadSupplementalLoads(agentFilter requestedAgentFilter: SkillAgentFilter) {
        schedulePostRefreshSupplementalLoads(
            agentFilter: requestedAgentFilter,
            loadLocalSessions: false,
            loadAgentConfigDocuments: false,
            loadClaudeSettings: false,
            forceAgentConfigSnapshots: true,
            forceAIProviderStatus: false,
            forceProviderObservability: true
        )
    }

    private func schedulePostRefreshSupplementalLoads(
        agentFilter requestedAgentFilter: SkillAgentFilter,
        loadLocalSessions: Bool,
        loadAgentConfigDocuments: Bool,
        loadClaudeSettings: Bool,
        forceAgentConfigSnapshots: Bool,
        forceAIProviderStatus: Bool,
        forceProviderObservability: Bool
    ) {
        postRefreshSupplementalLoadTask?.cancel()
        postRefreshSupplementalLoadTask = Task { @MainActor [weak self, requestedAgentFilter] in
            guard let self, !Task.isCancelled else { return }
            if forceAIProviderStatus {
                await self.loadAIProviderStatus()
            } else {
                await self.loadAIProviderStatusIfNeeded()
            }
            guard !Task.isCancelled else { return }
            if loadLocalSessions {
                await self.refreshSelectedAgentLocalSessionsIfNeeded()
                guard !Task.isCancelled else { return }
            }
            if forceAgentConfigSnapshots {
                await self.loadAgentConfigSnapshots(agent: requestedAgentFilter.rawValue)
            } else {
                await self.loadAgentConfigSnapshotsIfNeeded(agent: requestedAgentFilter.rawValue)
            }
            guard !Task.isCancelled else { return }
            if loadAgentConfigDocuments {
                await self.loadCurrentAgentConfigDocumentsIfNeeded(agent: requestedAgentFilter.rawValue)
                guard !Task.isCancelled else { return }
            }
            if loadClaudeSettings {
                await self.loadClaudeSettingsIfNeeded()
                guard !Task.isCancelled else { return }
            }
            await self.loadLLMPromptRuns()
            guard !Task.isCancelled else { return }
            await self.loadProviderObservabilityDuringRefresh(force: forceProviderObservability)
        }
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
            skillEventLoadGenerationValue &+= 1
            skillEventLoadGenerations[instanceID] = skillEventLoadGenerationValue
            detailsByID.removeValue(forKey: instanceID)
            skillEventsByID.removeValue(forKey: instanceID)
            skillEventAccumulatorsByID.removeValue(forKey: instanceID)
            skillEventCompletenessByID.removeValue(forKey: instanceID)
            loadingSkillEventIDs.remove(instanceID)
        }
    }

    func pruneDetailCaches(to currentSkillIDs: Set<SkillRecord.ID>) {
        detailsByID = detailsByID.filter { currentSkillIDs.contains($0.key) }
        skillEventsByID = skillEventsByID.filter { currentSkillIDs.contains($0.key) }
        skillEventAccumulatorsByID = skillEventAccumulatorsByID.filter { currentSkillIDs.contains($0.key) }
        skillEventCompletenessByID = skillEventCompletenessByID.filter { currentSkillIDs.contains($0.key) }
        skillEventLoadGenerations = skillEventLoadGenerations.filter { currentSkillIDs.contains($0.key) }
        loadingSkillEventIDs = loadingSkillEventIDs.filter { currentSkillIDs.contains($0) }
    }

    func invalidateScopedLocalSessionSummaryCache() {
        scopedLocalSessionSummaryRevision &+= 1
        scopedLocalSessionSummaryCache = nil
    }

    var selectedLocalSession: LocalSessionPreviewRow? {
        if case .loaded(let detail) = selectedLocalSessionDetailState,
           detail.id == selectedLocalSessionID {
            return detail
        }
        return selectedLocalSessionSummary
    }

    var selectedLocalSessionSummary: LocalSessionPreviewRow? {
        guard let selectedLocalSessionID else { return nil }
        return activeLocalSessionSnapshot?.result.sessionRows.first { $0.id == selectedLocalSessionID }
    }

    var hasActiveLocalSessionSnapshot: Bool {
        activeLocalSessionSnapshot != nil
    }

    var localSessionSummaryDisplayError: String? {
        switch localSessionLoadState {
        case .stale(_, let displayError), .failed(_, let displayError):
            return displayError
        case .empty, .loading, .fresh, .refreshing:
            return nil
        }
    }

    var filteredLocalSessionRows: [LocalSessionPreviewRow] {
        scopedLocalSessionSummary.rows
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
        let projectedRows: [LocalSessionPreviewRow]
        if let key = activeLocalSessionSnapshotKey {
            projectedRows = localSessionCache.projectedRows(
                for: key,
                criteria: LocalSessionProjectionCriteria(
                    scope: localSessionScopeFilter,
                    search: normalizedLocalSessionSearchText,
                    sort: localSessionSortOrder,
                    direction: localSessionSortDirection,
                    projectRoot: activeProjectContext?.rootPath
                )
            )
        } else {
            projectedRows = []
        }
        for row in projectedRows {
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

    func selectLocalSession(
        _ session: LocalSessionPreviewRow,
        origin: LocalSessionSelectionOrigin = .user
    ) {
        selectedLocalSessionID = session.id
        setSidebarSelection(.session(session.id))
        selectedDetailSection = .overview
        synchronizeSelectedLocalSessionDetailState()
        guard origin == .user || origin == .navigation else { return }
        localSessionDetailTask?.cancel()
        localSessionDetailTask = Task { @MainActor [weak self, sessionID = session.id] in
            await self?.loadLocalSessionDetailIfNeeded(sessionID: sessionID)
        }
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

    func selectTaskCockpitHistoryRecord(_ record: TaskCockpitHistoryRecord) {
        taskCockpitText = record.taskText
        setTaskCockpitAgentSelection(record.agentIDs, clearResult: false)
        taskCockpitResult = record.result
        taskCockpitOperationState = record.operationState
        selectedTaskCockpitHistoryID = record.id
    }

    func clearTaskCockpitHistory() {
        taskCockpitHistory = []
        selectedTaskCockpitHistoryID = nil
        do {
            _ = try taskCockpitHistoryStore.purgeLegacyHistoryIfPresent()
            taskCockpitHistoryCleanupMessage = nil
        } catch {
            taskCockpitHistoryCleanupMessage = UIStrings.taskCockpitHistoryCleanupFailed
        }
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

    func loadAppStartupDataIfNeeded() async {
        guard !hasCompletedStartupLoad, !isRunningStartupLoad else { return }
        postRefreshSupplementalLoadTask?.cancel()
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
            try await refreshCollections(includeSupplementalData: false, includeAIProviderStatus: false)

            setStartupLoading(UIStrings.startupAnalysisLoading, progress: 0.40)
            let startupAgentFilter = agentFilter
            let shouldLoadClaudeSettings = startupAgentFilter == .claudeCode
                && status?.supportedMethods.contains("config.readClaudeSettings") == true

            setStartupLoading(UIStrings.startupDetailLoading, progress: 0.90)
            await loadSelectedDetail()

            setStartupLoading(UIStrings.startupReadyLoading, progress: 1.0)
            refreshStatusMessage = UIStrings.refreshReloaded(skills.count, findings.count, sameAgentRuntimeConflictCount)
            appendRefreshLog(level: "info", message: refreshStatusMessage)
            canRetryLastRefresh = false
            scheduleStartupSupplementalLoads(
                agentFilter: startupAgentFilter,
                shouldLoadClaudeSettings: shouldLoadClaudeSettings
            )
        } catch {
            handleRefreshFailure(error, action: .reload)
        }
    }

    private func setStartupLoading(_ message: String, progress: Double) {
        startupLoadingState = AppStartupLoadingState(message: message, progress: progress)
    }

    func reload() async {
        guard !isRefreshBusy else { return }
        clearRollbackConfirmation()
        postRefreshSupplementalLoadTask?.cancel()
        isLoading = true
        errorMessage = nil
        beginRefresh(.reload, message: UIStrings.refreshReloading)
        defer { isLoading = false }

        do {
            try await refreshCollections(includeSupplementalData: false, includeAIProviderStatus: true)
            refreshStatusMessage = UIStrings.refreshReloaded(skills.count, findings.count, sameAgentRuntimeConflictCount)
            appendRefreshLog(level: "info", message: refreshStatusMessage)
            canRetryLastRefresh = false
            await loadSelectedDetail()
            scheduleReloadSupplementalLoads(agentFilter: agentFilter)
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
            lastMutationMessage = UIStrings.scannedSkills(result.scannedCount)
            applyRefreshActivity(result.activity)
            catalogListCompleteness = catalogCompleteness(after: result)
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
            invalidateDetailCaches(for: [skill.id])
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
            invalidateDetailCaches(for: preview.affectedSkills.map(\.instanceID))
            try await refreshCollections()
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
        let trimmedOwner = skillManagerOwner.trimmingCharacters(in: .whitespacesAndNewlines)
        let owner = trimmedOwner.isEmpty ? nil : trimmedOwner
        let networkAllowed = skillManagerNetworkAllowed
        let key = SkillManagerRequestKey.search(
            query: query,
            owner: owner,
            networkAllowed: networkAllowed
        )
        let generation = beginSkillManagerSearch(for: key)
        isSearchingSkillManager = true
        clearSkillManagerFeedback()

        let service = service
        let task = Task { @MainActor [weak self, service] in
            do {
                let result = try await service.searchSkillManager(
                    query: query,
                    owner: owner,
                    networkAllowed: networkAllowed
                )
                guard let self else { return }
                defer { self.finishSkillManagerSearch(generation) }
                guard self.currentSkillManagerSearchGeneration == generation else { return }
                self.skillManagerSearchVisibility.reset()
                self.skillManagerSearchResult = result
            } catch {
                guard let self else { return }
                defer { self.finishSkillManagerSearch(generation) }
                guard self.currentSkillManagerSearchGeneration == generation else { return }
                guard !(error is CancellationError), !Task.isCancelled else { return }
                self.setSkillManagerError(error.localizedDescription)
            }
        }
        let handle = SkillManagerRequestTaskHandle(task: task)
        skillManagerSearchTask = handle
        await handle.wait()
        if Task.isCancelled, currentSkillManagerSearchGeneration == generation {
            invalidateSkillManagerSearch()
        }
    }

    func listSkillManagerInstalled() async {
        let agents = canonicalSkillManagerAgentIDs(selectedSkillManagerAgentIDsForRead())
        let scope = skillManagerScope
        let key = SkillManagerRequestKey.installed(agents: agents, scope: scope)
        let generation = beginSkillManagerInstalledList(for: key)
        clearSkillManagerFeedback()

        let service = service
        let task = Task { @MainActor [weak self, service] in
            do {
                let result = try await service.listSkillManagerInstalled(
                    agents: agents,
                    scope: scope
                )
                guard let self else { return }
                defer { self.finishSkillManagerInstalledList(generation) }
                guard self.currentSkillManagerInstalledGeneration == generation else { return }
                self.skillManagerInstalled = result
            } catch {
                guard let self else { return }
                defer { self.finishSkillManagerInstalledList(generation) }
                guard self.currentSkillManagerInstalledGeneration == generation else { return }
                guard !(error is CancellationError), !Task.isCancelled else { return }
                self.setSkillManagerError(error.localizedDescription)
            }
        }
        let handle = SkillManagerRequestTaskHandle(task: task)
        skillManagerInstalledTask = handle
        await handle.wait()
        if Task.isCancelled, currentSkillManagerInstalledGeneration == generation {
            invalidateSkillManagerInstalledList()
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

    func loadMoreSkillManagerSearchResults() { if let result = skillManagerSearchResult { skillManagerSearchVisibility.loadMore(totalReturned: result.results.count) } }

    func showAllReturnedSkillManagerSearchResults() { if let result = skillManagerSearchResult { skillManagerSearchVisibility.loadAll(totalReturned: result.results.count) } }

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

        let inputs = SkillManagerMutationInputs(
            kind: .install,
            source: source,
            skills: skills,
            agents: agents,
            scope: skillManagerScope,
            distribution: skillManagerDistribution,
            networkAllowed: skillManagerNetworkAllowed
        )
        await previewSkillManagerMutation(inputs: inputs) { [service] in
            try await service.previewSkillManagerInstall(
                source: inputs.source ?? "",
                skills: inputs.skills,
                agents: inputs.agents,
                scope: inputs.scope,
                distribution: inputs.distribution ?? .symlink,
                networkAllowed: inputs.networkAllowed
            )
        }
    }

    func applySkillManagerInstall(confirmation: SkillManagerMutationConfirmation) async {
        guard skillManagerMutationConfirmation == confirmation,
              confirmation.inputs.kind == .install else { return }
        await applySkillManagerMutation(confirmation)
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

        let inputs = SkillManagerMutationInputs(
            kind: .remove,
            source: nil,
            skills: [skill],
            agents: agents,
            scope: skillManagerScope,
            distribution: nil,
            networkAllowed: false
        )
        await previewSkillManagerMutation(inputs: inputs) { [service] in
            try await service.previewSkillManagerRemove(
                skill: inputs.skills.first ?? "",
                agents: inputs.agents,
                scope: inputs.scope
            )
        }
    }

    func applySkillManagerRemove(confirmation: SkillManagerMutationConfirmation) async {
        guard skillManagerMutationConfirmation == confirmation,
              confirmation.inputs.kind == .remove else { return }
        await applySkillManagerMutation(confirmation)
    }

    func previewSkillManagerUpdate(skillName: String? = nil) async {
        if let skillName {
            skillManagerRemoveSkillName = skillName
        }
        guard let agents = selectedSkillManagerAgentIDsForMutation() else { return }

        let inputs = SkillManagerMutationInputs(
            kind: .update,
            source: nil,
            skills: parsedSkillManagerSkillNames(from: skillManagerRemoveSkillName),
            agents: agents,
            scope: skillManagerScope,
            distribution: nil,
            networkAllowed: skillManagerNetworkAllowed
        )
        await previewSkillManagerMutation(inputs: inputs) { [service] in
            try await service.previewSkillManagerUpdate(
                skills: inputs.skills,
                agents: inputs.agents,
                scope: inputs.scope,
                networkAllowed: inputs.networkAllowed
            )
        }
    }

    func applySkillManagerUpdate(confirmation: SkillManagerMutationConfirmation) async {
        guard skillManagerMutationConfirmation == confirmation,
              confirmation.inputs.kind == .update else { return }
        await applySkillManagerMutation(confirmation)
    }

    func previewSkillManagerLocalCreate() async {
        let name = skillManagerLocalSkillName.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !name.isEmpty else {
            setSkillManagerError(UIStrings.text("skillManager.localCreate.required", "Enter a local skill name."))
            return
        }
        let key = SkillManagerRequestKey.localCreate(name: name)
        let generation = beginSkillManagerLocalCreate(for: key)
        clearSkillManagerFeedback()

        let service = service
        let task = Task { @MainActor [weak self, service] in
            do {
                let result = try await service.previewSkillManagerLocalCreate(name: name)
                guard let self else { return }
                defer { self.finishSkillManagerLocalCreate(generation) }
                guard self.currentSkillManagerLocalCreateGeneration == generation else { return }
                self.skillManagerLocalCreateConfirmation = SkillManagerLocalCreateConfirmation(
                    generation: generation,
                    name: name,
                    result: result
                )
            } catch {
                guard let self else { return }
                defer { self.finishSkillManagerLocalCreate(generation) }
                guard self.currentSkillManagerLocalCreateGeneration == generation else { return }
                guard !(error is CancellationError), !Task.isCancelled else { return }
                self.setSkillManagerError(error.localizedDescription)
                self.skillManagerLocalCreateConfirmation = nil
            }
        }
        let handle = SkillManagerRequestTaskHandle(task: task)
        skillManagerLocalCreateTask = handle
        await handle.wait()
        if Task.isCancelled, currentSkillManagerLocalCreateGeneration == generation {
            invalidateSkillManagerLocalCreatePreview()
        }
    }

    func applySkillManagerLocalCreate(confirmation: SkillManagerLocalCreateConfirmation) async {
        guard skillManagerLocalCreateConfirmation == confirmation else { return }
        await runSkillManagerConfirmedWrite { [self] in
            do {
                _ = try await service.applySkillManagerLocalCreate(
                    preview: confirmation.result,
                    name: confirmation.name
                )
                retireSkillManagerLocalCreateConfirmation(confirmation)
                try await refreshCollections()
                skillManagerMessage = UIStrings.text("skillManager.localCreate.applied", "Local skill template created and imported.")
                recordLocalRefresh(message: UIStrings.refreshAfterWrite)
            } catch {
                setSkillManagerError(error.localizedDescription)
            }
        }
    }

    func previewSkillManagerLocalDelete(skill: SkillRecord) async {
        let instanceID = skill.id
        let key = SkillManagerRequestKey.localDelete(instanceID: instanceID)
        let generation = beginSkillManagerLocalDelete(for: key)
        clearSkillManagerFeedback()

        let service = service
        let task = Task { @MainActor [weak self, service] in
            do {
                let result = try await service.previewSkillManagerLocalDelete(instanceID: instanceID)
                guard let self else { return }
                defer { self.finishSkillManagerLocalDelete(generation) }
                guard self.currentSkillManagerLocalDeleteGeneration == generation else { return }
                self.skillManagerLocalDeleteConfirmation = SkillManagerLocalDeleteConfirmation(
                    generation: generation,
                    instanceID: instanceID,
                    result: result
                )
            } catch {
                guard let self else { return }
                defer { self.finishSkillManagerLocalDelete(generation) }
                guard self.currentSkillManagerLocalDeleteGeneration == generation else { return }
                guard !(error is CancellationError), !Task.isCancelled else { return }
                self.setSkillManagerError(error.localizedDescription)
                self.skillManagerLocalDeleteConfirmation = nil
            }
        }
        let handle = SkillManagerRequestTaskHandle(task: task)
        skillManagerLocalDeleteTask = handle
        await handle.wait()
        if Task.isCancelled, currentSkillManagerLocalDeleteGeneration == generation {
            invalidateSkillManagerLocalDeletePreview()
        }
    }

    func applySkillManagerLocalDelete(confirmation: SkillManagerLocalDeleteConfirmation) async {
        guard skillManagerLocalDeleteConfirmation == confirmation else { return }
        guard confirmation.result.physicalDeleteAllowed else {
            setSkillManagerError(confirmation.result.summary)
            return
        }
        await runSkillManagerConfirmedWrite { [self] in
            do {
                _ = try await service.applySkillManagerLocalDelete(instanceID: confirmation.instanceID)
                retireSkillManagerLocalDeleteConfirmation(confirmation)
                try await refreshCollections()
                skillManagerMessage = UIStrings.text("skillManager.localDelete.applied", "Local skill deleted.")
                recordLocalRefresh(message: UIStrings.refreshAfterWrite)
            } catch {
                setSkillManagerError(error.localizedDescription)
            }
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
        invalidateSkillManagerMutationPreview()
        invalidateSkillManagerLocalCreatePreview()
        invalidateSkillManagerLocalDeletePreview()
    }

    private func retireSkillManagerMutationConfirmation(_ confirmation: SkillManagerMutationConfirmation) {
        guard skillManagerMutationConfirmation?.generation == confirmation.generation else { return }
        invalidateSkillManagerMutationPreview()
    }

    private func retireSkillManagerLocalCreateConfirmation(_ confirmation: SkillManagerLocalCreateConfirmation) {
        guard skillManagerLocalCreateConfirmation?.generation == confirmation.generation else { return }
        invalidateSkillManagerLocalCreatePreview()
    }

    private func retireSkillManagerLocalDeleteConfirmation(_ confirmation: SkillManagerLocalDeleteConfirmation) {
        guard skillManagerLocalDeleteConfirmation?.generation == confirmation.generation else { return }
        invalidateSkillManagerLocalDeletePreview()
    }

    private func clearSkillManagerFeedback() {
        skillManagerErrorMessage = nil
        skillManagerMessage = nil
    }

    private func setSkillManagerError(_ message: String) {
        skillManagerErrorMessage = UIStrings.localizedServiceMessage(message)
        skillManagerMessage = nil
    }

    private func previewSkillManagerMutation(
        inputs: SkillManagerMutationInputs,
        operation: @escaping () async throws -> SkillManagerMutationRecord
    ) async {
        let generation = beginSkillManagerMutationPreview(for: .mutation(inputs))
        clearSkillManagerFeedback()

        let task = Task { @MainActor [weak self] in
            do {
                let result = try await operation()
                guard let self else { return }
                defer { self.finishSkillManagerMutationPreview(generation) }
                guard self.currentSkillManagerMutationGeneration == generation else { return }
                self.skillManagerMutationConfirmation = SkillManagerMutationConfirmation(
                    generation: generation,
                    inputs: inputs,
                    result: result
                )
            } catch {
                guard let self else { return }
                defer { self.finishSkillManagerMutationPreview(generation) }
                guard self.currentSkillManagerMutationGeneration == generation else { return }
                guard !(error is CancellationError), !Task.isCancelled else { return }
                self.setSkillManagerError(error.localizedDescription)
                self.skillManagerMutationConfirmation = nil
            }
        }
        let handle = SkillManagerRequestTaskHandle(task: task)
        skillManagerMutationTask = handle
        await handle.wait()
        if Task.isCancelled, currentSkillManagerMutationGeneration == generation {
            invalidateSkillManagerMutationPreview()
        }
    }

    private func runSkillManagerConfirmedWrite(
        _ operation: @escaping @MainActor () async -> Void
    ) async {
        guard !isApplyingSkillManagerMutation else { return }
        isApplyingSkillManagerMutation = true
        isWriting = true
        clearSkillManagerFeedback()

        let task = Task { @MainActor [self] in
            defer {
                isWriting = false
                isApplyingSkillManagerMutation = false
                skillManagerApplyTask = nil
            }
            await operation()
        }
        skillManagerApplyTask = task
        await task.value
    }

    private func applySkillManagerMutation(_ confirmation: SkillManagerMutationConfirmation) async {
        guard skillManagerMutationConfirmation == confirmation else { return }
        await runSkillManagerConfirmedWrite { [self] in
            do {
                let result: SkillManagerMutationRecord
                switch confirmation.inputs.kind {
                case .install:
                    guard let source = confirmation.inputs.source,
                          !source.isEmpty,
                          let distribution = confirmation.inputs.distribution else {
                        setSkillManagerError(UIStrings.text("skillManager.preview.invalid", "The Skill Manager preview is no longer valid."))
                        return
                    }
                    result = try await service.applySkillManagerInstall(
                        preview: confirmation.result,
                        source: source,
                        skills: confirmation.inputs.skills,
                        agents: confirmation.inputs.agents,
                        scope: confirmation.inputs.scope,
                        distribution: distribution,
                        networkAllowed: confirmation.inputs.networkAllowed
                    )
                case .remove:
                    guard let skill = confirmation.inputs.skills.first else {
                        setSkillManagerError(UIStrings.text("skillManager.preview.invalid", "The Skill Manager preview is no longer valid."))
                        return
                    }
                    result = try await service.applySkillManagerRemove(
                        preview: confirmation.result,
                        skill: skill,
                        agents: confirmation.inputs.agents,
                        scope: confirmation.inputs.scope
                    )
                case .update:
                    result = try await service.applySkillManagerUpdate(
                        preview: confirmation.result,
                        skills: confirmation.inputs.skills,
                        agents: confirmation.inputs.agents,
                        scope: confirmation.inputs.scope,
                        networkAllowed: confirmation.inputs.networkAllowed
                    )
                }
                retireSkillManagerMutationConfirmation(confirmation)
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
    }

    private func beginSkillManagerSearch(for key: SkillManagerRequestKey) -> SkillManagerRequestGeneration {
        skillManagerSearchTask?.cancel()
        skillManagerSearchTask = nil
        skillManagerSearchGenerationValue &+= 1
        let generation = SkillManagerRequestGeneration(value: skillManagerSearchGenerationValue, key: key)
        currentSkillManagerSearchGeneration = generation
        isSearchingSkillManager = true
        return generation
    }

    private func finishSkillManagerSearch(_ generation: SkillManagerRequestGeneration) {
        guard currentSkillManagerSearchGeneration == generation else { return }
        skillManagerSearchTask = nil
        isSearchingSkillManager = false
    }

    private func invalidateSkillManagerSearch() {
        skillManagerSearchTask?.cancel()
        skillManagerSearchTask = nil
        skillManagerSearchGenerationValue &+= 1
        currentSkillManagerSearchGeneration = nil
        skillManagerSearchVisibility.reset()
        skillManagerSearchResult = nil
        isSearchingSkillManager = false
    }

    private func beginSkillManagerInstalledList(for key: SkillManagerRequestKey) -> SkillManagerRequestGeneration {
        skillManagerInstalledTask?.cancel()
        skillManagerInstalledTask = nil
        skillManagerInstalledGenerationValue &+= 1
        let generation = SkillManagerRequestGeneration(value: skillManagerInstalledGenerationValue, key: key)
        currentSkillManagerInstalledGeneration = generation
        isListingSkillManagerInstalled = true
        return generation
    }

    private func finishSkillManagerInstalledList(_ generation: SkillManagerRequestGeneration) {
        guard currentSkillManagerInstalledGeneration == generation else { return }
        skillManagerInstalledTask = nil
        isListingSkillManagerInstalled = false
    }

    private func invalidateSkillManagerInstalledList() {
        skillManagerInstalledTask?.cancel()
        skillManagerInstalledTask = nil
        skillManagerInstalledGenerationValue &+= 1
        currentSkillManagerInstalledGeneration = nil
        skillManagerInstalled = nil
        isListingSkillManagerInstalled = false
    }

    private func beginSkillManagerMutationPreview(for key: SkillManagerRequestKey) -> SkillManagerRequestGeneration {
        skillManagerMutationTask?.cancel()
        skillManagerMutationTask = nil
        skillManagerMutationGenerationValue &+= 1
        let generation = SkillManagerRequestGeneration(value: skillManagerMutationGenerationValue, key: key)
        currentSkillManagerMutationGeneration = generation
        skillManagerMutationConfirmation = nil
        isPreviewingSkillManagerMutation = true
        return generation
    }

    private func finishSkillManagerMutationPreview(_ generation: SkillManagerRequestGeneration) {
        guard currentSkillManagerMutationGeneration == generation else { return }
        skillManagerMutationTask = nil
        isPreviewingSkillManagerMutation = false
    }

    private func invalidateSkillManagerMutationPreview() {
        skillManagerMutationTask?.cancel()
        skillManagerMutationTask = nil
        skillManagerMutationGenerationValue &+= 1
        currentSkillManagerMutationGeneration = nil
        skillManagerMutationConfirmation = nil
        isPreviewingSkillManagerMutation = false
    }

    private func beginSkillManagerLocalCreate(for key: SkillManagerRequestKey) -> SkillManagerRequestGeneration {
        skillManagerLocalCreateTask?.cancel()
        skillManagerLocalCreateTask = nil
        skillManagerLocalCreateGenerationValue &+= 1
        let generation = SkillManagerRequestGeneration(value: skillManagerLocalCreateGenerationValue, key: key)
        currentSkillManagerLocalCreateGeneration = generation
        skillManagerLocalCreateConfirmation = nil
        isPreviewingSkillManagerLocalCreate = true
        return generation
    }

    private func finishSkillManagerLocalCreate(_ generation: SkillManagerRequestGeneration) {
        guard currentSkillManagerLocalCreateGeneration == generation else { return }
        skillManagerLocalCreateTask = nil
        isPreviewingSkillManagerLocalCreate = false
    }

    private func invalidateSkillManagerLocalCreatePreview() {
        skillManagerLocalCreateTask?.cancel()
        skillManagerLocalCreateTask = nil
        skillManagerLocalCreateGenerationValue &+= 1
        currentSkillManagerLocalCreateGeneration = nil
        skillManagerLocalCreateConfirmation = nil
        isPreviewingSkillManagerLocalCreate = false
    }

    private func beginSkillManagerLocalDelete(for key: SkillManagerRequestKey) -> SkillManagerRequestGeneration {
        skillManagerLocalDeleteTask?.cancel()
        skillManagerLocalDeleteTask = nil
        skillManagerLocalDeleteGenerationValue &+= 1
        let generation = SkillManagerRequestGeneration(value: skillManagerLocalDeleteGenerationValue, key: key)
        currentSkillManagerLocalDeleteGeneration = generation
        skillManagerLocalDeleteConfirmation = nil
        isPreviewingSkillManagerLocalDelete = true
        return generation
    }

    private func finishSkillManagerLocalDelete(_ generation: SkillManagerRequestGeneration) {
        guard currentSkillManagerLocalDeleteGeneration == generation else { return }
        skillManagerLocalDeleteTask = nil
        isPreviewingSkillManagerLocalDelete = false
    }

    private func invalidateSkillManagerLocalDeletePreview() {
        skillManagerLocalDeleteTask?.cancel()
        skillManagerLocalDeleteTask = nil
        skillManagerLocalDeleteGenerationValue &+= 1
        currentSkillManagerLocalDeleteGeneration = nil
        skillManagerLocalDeleteConfirmation = nil
        isPreviewingSkillManagerLocalDelete = false
    }

    private func selectedSkillManagerAgentIDsForMutation() -> [String]? {
        let agents = canonicalSkillManagerAgentIDs(skillManagerSelectedAgents)
        guard !agents.isEmpty else {
            setSkillManagerError(UIStrings.text("skillManager.agents.required", "Select at least one target agent."))
            return nil
        }
        return agents
    }

    private func selectedSkillManagerAgentIDsForRead() -> [String] {
        let agents = skillManagerSelectedAgents
        let resolved = agents.isEmpty ? SkillManagerAgent.defaultTargets.map(\.rawValue) : agents
        return canonicalSkillManagerAgentIDs(resolved)
    }

    private func canonicalSkillManagerAgentIDs(_ agents: [String]) -> [String] {
        Array(
            Set(
                agents
                    .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                    .filter { !$0.isEmpty }
            )
        ).sorted()
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

    func buildTaskCockpit() async {
        let taskText = selectedTaskCockpitInput
        guard !taskText.isEmpty else {
            taskCockpitResult = .unavailable(taskText: "", reason: UIStrings.taskCockpitTaskRequired)
            taskCockpitPromptConfirmation = nil
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
            taskCockpitPromptConfirmation = nil
            taskCockpitOperationState = TaskCockpitOperationState.idle.finished(
                phase: .failed,
                message: message
            )
            return
        }
        guard !isPreviewingTaskCockpitPrompt, !isBuildingTaskCockpit else { return }
        guard !isRefreshBusy else {
            taskCockpitResult = .unavailable(taskText: taskText, reason: UIStrings.operationUnavailableBusy)
            taskCockpitPromptConfirmation = nil
            taskCockpitOperationState = TaskCockpitOperationState.preparing(
                taskText: taskText,
                timeoutSeconds: roundedTaskCockpitTimeoutSeconds
            ).finished(
                phase: .failed,
                message: UIStrings.operationUnavailableBusy
            )
            return
        }

        taskCockpitResult = nil
        taskCockpitPromptConfirmation = nil
        isPreviewingTaskCockpitPrompt = true
        taskCockpitOperationState = .preparing(
            taskText: taskText,
            timeoutSeconds: roundedTaskCockpitTimeoutSeconds
        )
        defer { isPreviewingTaskCockpitPrompt = false }

        let candidateSkillIDs = taskCockpitCandidateSkillIDs(for: selectedAgents)
        do {
            let preview = try await service.previewPromptForTaskCockpit(
                taskText: taskText,
                agents: selectedAgents,
                instanceIDs: candidateSkillIDs
            )
            guard canSendLLMPrompt(preview) else {
                let reason = UIStrings.localizedServiceMessage(preview.disabledReason ?? UIStrings.llmPromptUnavailable)
                taskCockpitResult = .unavailable(taskText: taskText, reason: reason)
                taskCockpitOperationState = TaskCockpitOperationState.idle.finished(
                    phase: .failed,
                    message: reason
                )
                return
            }
            taskCockpitPromptConfirmation = TaskCockpitPromptConfirmation(
                preview: preview,
                taskText: taskText,
                agentIDs: selectedAgents,
                instanceIDs: candidateSkillIDs
            )
            taskCockpitOperationState = TaskCockpitOperationState.idle.finished(
                phase: .completed,
                message: UIStrings.taskCockpitPromptReady
            )
        } catch {
            let message = UIStrings.localizedServiceMessage(error.localizedDescription)
            taskCockpitResult = .unavailable(taskText: taskText, reason: message)
            taskCockpitOperationState = TaskCockpitOperationState.idle.finished(
                phase: .failed,
                message: UIStrings.taskCockpitFailed(message)
            )
        }
    }

    func confirmTaskCockpitPromptAndBuild() async {
        guard let pending = taskCockpitPromptConfirmation else {
            await buildTaskCockpit()
            return
        }
        guard canSendLLMPrompt(pending.preview) else {
            let reason = aiProviderStatus.configured ? UIStrings.llmPromptPreviewRequired : UIStrings.llmPromptProviderRequired
            taskCockpitResult = .unavailable(taskText: pending.taskText, reason: reason)
            taskCockpitOperationState = TaskCockpitOperationState.idle.finished(
                phase: .failed,
                message: reason
            )
            return
        }
        guard !isBuildingTaskCockpit else { return }
        guard !isRefreshBusy else {
            taskCockpitResult = .unavailable(taskText: pending.taskText, reason: UIStrings.operationUnavailableBusy)
            taskCockpitOperationState = TaskCockpitOperationState.preparing(
                taskText: pending.taskText,
                timeoutSeconds: roundedTaskCockpitTimeoutSeconds
            ).finished(
                phase: .failed,
                message: UIStrings.operationUnavailableBusy
            )
            return
        }

        let taskText = pending.taskText
        let selectedAgents = pending.agentIDs
        let candidateSkillIDs = pending.instanceIDs
        let previewID = pending.preview.previewID
        let operationID = UUID()
        taskCockpitOperationID = operationID
        isBuildingTaskCockpit = true
        taskCockpitPromptConfirmation = nil
        taskCockpitResult = nil
        taskCockpitOperationState = .preparing(
            taskText: taskText,
            timeoutSeconds: roundedTaskCockpitTimeoutSeconds
        )
        scheduleTaskCockpitTimeout(operationID: operationID, taskText: taskText)

        let serviceTask = Task {
            let sendResult = try await service.confirmPromptAndSendForTaskCockpit(
                previewID: previewID,
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

    func clearTaskCockpitPromptConfirmation() {
        guard !isBuildingTaskCockpit else { return }
        taskCockpitPromptConfirmation = nil
        if taskCockpitResult == nil {
            taskCockpitOperationState = .idle
        }
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
        taskCockpitPromptConfirmation = nil
        if publishFallbackResult {
            taskCockpitResult = .unavailable(taskText: taskText, reason: message)
        }
        taskCockpitOperationState = taskCockpitOperationState.finished(
            phase: .cancelled,
            message: message
        )
    }

    func refreshSelectedAgentLocalSessions() async {
        await refreshLocalSessionSnapshot(reason: .manual)
    }

    func refreshSelectedAgentLocalSessionsIfNeeded() async {
        await refreshLocalSessionSnapshot(reason: .sourceChanged)
    }

    func previewLocalSessions() async {
        await refreshLocalSessionSnapshot(reason: .manual)
    }

    func refreshLocalSessionSnapshot(reason: LocalSessionRefreshReason) async {
        let roots = normalizedLocalSessionPreviewRoots
        let key = localSessionSnapshotKey(roots: roots)
        activeLocalSessionSnapshotKey = key
        localSessionCache.activateSource(key)
        if reason != .manual, let snapshot = localSessionCache.successfulSnapshot(for: key) {
            publishLocalSessionSnapshot(snapshot)
            return
        }

        guard reason != .manual || !isRefreshBusy else { return }
        let generation = localSessionCache.beginSummaryRefresh(for: key)
        activeLocalSessionRefreshGeneration = generation
        localSessionLoadState = localSessionCache.summaryStates[key] ?? .loading(key: key)
        let agent = key.agent == SkillAgentFilter.all.rawValue ? nil : key.agent
        let project = activeProjectContext
        isPreviewingLocalSessions = true
        defer {
            if activeLocalSessionSnapshotKey == key,
               activeLocalSessionRefreshGeneration == generation {
                isPreviewingLocalSessions = false
                activeLocalSessionRefreshGeneration = nil
            }
        }

        do {
            var mergedResult: LocalSessionPreviewResult?
            var seenIDs = Set<String>()
            var cursor: String?
            var sourceRevision: String?
            while true {
                let requestedCursor = cursor
                let page = try await service.previewLocalSessions(
                    authorizedRoots: key.authorizedRoots,
                    agent: agent,
                    scope: .all,
                    search: nil,
                    project: project,
                    sessionID: nil,
                    includeContentItems: false,
                    limit: Self.localSessionPageLimit,
                    offset: nil,
                    pagingMode: "keyset",
                    cursor: cursor,
                    sourceRevision: sourceRevision,
                    sort: .recent,
                    direction: .descending,
                    maxFiles: nil
                )
                if page.isUnavailable {
                    throw ServiceClient.ClientError.invalidOutput(
                        page.fallbackReason ?? "local session preview unavailable"
                    )
                }
                guard activeLocalSessionSnapshotKey == key,
                      activeLocalSessionRefreshGeneration == generation else { return }
                if let sourceRevision, page.sourceRevision != sourceRevision {
                    throw ServiceClient.ClientError.service(ServiceErrorPayload(
                        code: "source_changed",
                        message: "local session source changed during prewarm"
                    ))
                }
                let newRows = page.sessionRows.filter { seenIDs.insert($0.id).inserted }
                mergedResult = mergeLocalSessionSummaryPage(
                    accumulated: mergedResult,
                    page: page,
                    newRows: newRows
                )
                guard let mergedResult else {
                    throw ServiceClient.ClientError.invalidOutput("missing local session summary page")
                }
                let snapshot = LocalSessionSnapshot(
                    key: key,
                    generation: generation,
                    result: mergedResult,
                    refreshedAt: Date(),
                    isComplete: !mergedResult.hasMore
                        && mergedResult.sourceCompleteness == .enumerable
                        && mergedResult.incompleteReason == nil
                        && mergedResult.sessionRows.count == mergedResult.totalMatchedCount,
                    nextCursor: mergedResult.nextCursor,
                    sourceRevision: mergedResult.sourceRevision,
                    sourceCompleteness: mergedResult.sourceCompleteness,
                    incompleteReason: mergedResult.incompleteReason
                )
                localSessionCache.publishSummary(snapshot)
                guard let published = localSessionCache.successfulSnapshot(for: key),
                      published.generation == generation,
                      activeLocalSessionSnapshotKey == key,
                      activeLocalSessionRefreshGeneration == generation else { return }
                publishLocalSessionSnapshot(published)
                sourceRevision = page.sourceRevision ?? sourceRevision
                guard page.hasMore,
                      mergedResult.sessionRows.count < Self.localSessionPrewarmLimit else { break }
                guard let nextCursor = page.nextCursor,
                      nextCursor != requestedCursor,
                      sourceRevision != nil else {
                    throw ServiceClient.ClientError.invalidOutput("invalid local session continuation page")
                }
                cursor = nextCursor
            }
        } catch {
            guard activeLocalSessionSnapshotKey == key,
                  activeLocalSessionRefreshGeneration == generation else { return }
            localSessionCache.failSummary(
                key: key,
                generation: generation,
                displayError: error.localizedDescription
            )
            localSessionLoadState = localSessionCache.summaryStates[key]
                ?? .failed(key: key, displayError: error.localizedDescription)
            if let previous = localSessionCache.successfulSnapshot(for: key) {
                localSessionPreviewResult = previous.result
                localSessionCompleteness = localSessionCompletenessAfterFailure(
                    snapshot: previous,
                    error: error
                )
            } else {
                localSessionPreviewResult = .unavailable(reason: error.localizedDescription)
                localSessionCompleteness = localSessionCompletenessAfterFailure(
                    snapshot: nil,
                    error: error
                )
            }
        }
    }

    func loadMoreLocalSessions() async {
        await continueLocalSessionPages(loadAll: false)
    }

    func loadAllLocalSessions() async {
        guard localSessionLoadAllTask == nil else { return }
        let loadID = UUID()
        let task = Task { @MainActor [weak self] in
            guard let self else { return }
            if self.activeLocalSessionSnapshot == nil {
                await self.refreshLocalSessionSnapshot(reason: .manual)
            }
            if self.activeLocalSessionSnapshot?.nextCursor != nil {
                await self.continueLocalSessionPages(loadAll: true)
            }
        }
        localSessionLoadAllID = loadID
        localSessionLoadAllTask = task
        await task.value
        if localSessionLoadAllID == loadID {
            localSessionLoadAllTask = nil
            localSessionLoadAllID = nil
        }
    }

    func cancelLocalSessionLoadAll() {
        localSessionLoadAllTask?.cancel()
        localSessionLoadAllTask = nil
        localSessionLoadAllID = nil
        guard let key = activeLocalSessionSnapshotKey,
              let generation = activeLocalSessionRefreshGeneration else { return }
        localSessionCache.cancelSummaryLoad(key: key, generation: generation)
        activeLocalSessionRefreshGeneration = nil
        isPreviewingLocalSessions = false
        if let snapshot = localSessionCache.successfulSnapshot(for: key) {
            publishLocalSessionSnapshot(snapshot)
        }
    }

    private func continueLocalSessionPages(loadAll: Bool) async {
        guard let key = activeLocalSessionSnapshotKey,
              let initial = localSessionCache.successfulSnapshot(for: key),
              initial.nextCursor != nil,
              initial.sourceRevision != nil,
              !isPreviewingLocalSessions else { return }
        let generation = localSessionCache.beginSummaryRefresh(for: key)
        activeLocalSessionRefreshGeneration = generation
        isPreviewingLocalSessions = true
        localSessionCompleteness = localSessionCompletenessState(
            for: initial,
            phase: loadAll ? .all : .more
        )
        defer {
            if activeLocalSessionSnapshotKey == key,
               activeLocalSessionRefreshGeneration == generation {
                activeLocalSessionRefreshGeneration = nil
                isPreviewingLocalSessions = false
            }
        }

        let agent = key.agent == SkillAgentFilter.all.rawValue ? nil : key.agent
        let project = activeProjectContext
        var snapshot = initial
        do {
            repeat {
                guard !Task.isCancelled,
                      activeLocalSessionSnapshotKey == key,
                      activeLocalSessionRefreshGeneration == generation,
                      let cursor = snapshot.nextCursor,
                      let sourceRevision = snapshot.sourceRevision else { return }
                let page = try await service.previewLocalSessions(
                    authorizedRoots: key.authorizedRoots,
                    agent: agent,
                    scope: .all,
                    search: nil,
                    project: project,
                    sessionID: nil,
                    includeContentItems: false,
                    limit: Self.localSessionPageLimit,
                    offset: nil,
                    pagingMode: "keyset",
                    cursor: cursor,
                    sourceRevision: sourceRevision,
                    sort: .recent,
                    direction: .descending,
                    maxFiles: nil
                )
                guard !Task.isCancelled,
                      activeLocalSessionSnapshotKey == key,
                      activeLocalSessionRefreshGeneration == generation else { return }
                guard page.sourceRevision == sourceRevision else {
                    throw ServiceClient.ClientError.service(ServiceErrorPayload(
                        code: "source_changed",
                        message: "local session source changed during pagination"
                    ))
                }
                let merged = mergeLocalSessionSummaryPage(
                    accumulated: snapshot.result,
                    page: page,
                    newRows: page.sessionRows
                )
                if page.hasMore {
                    guard let nextCursor = page.nextCursor,
                          nextCursor != cursor else {
                        throw ServiceClient.ClientError.invalidOutput("local session page made no cursor progress")
                    }
                }
                if !page.hasMore,
                   page.sourceCompleteness == .enumerable,
                   merged.sessionRows.count != page.totalMatchedCount {
                    throw ServiceClient.ClientError.invalidOutput("terminal local session page count mismatch")
                }
                snapshot = LocalSessionSnapshot(
                    key: key,
                    generation: generation,
                    result: merged,
                    refreshedAt: Date(),
                    isComplete: !page.hasMore
                        && page.sourceCompleteness == .enumerable
                        && page.incompleteReason == nil,
                    nextCursor: page.nextCursor,
                    sourceRevision: page.sourceRevision,
                    sourceCompleteness: page.sourceCompleteness,
                    incompleteReason: page.incompleteReason
                )
                localSessionCache.publishSummary(snapshot)
                guard let published = localSessionCache.successfulSnapshot(for: key),
                      published.generation == generation else { return }
                snapshot = published
                publishLocalSessionSnapshot(published)
                guard !page.hasMore || page.nextCursor != nil else {
                    throw ServiceClient.ClientError.invalidOutput("invalid local session continuation page")
                }
            } while loadAll && snapshot.nextCursor != nil
        } catch {
            localSessionCache.failSummary(
                key: key,
                generation: generation,
                displayError: error.localizedDescription
            )
            guard activeLocalSessionSnapshotKey == key,
                  activeLocalSessionRefreshGeneration == generation else { return }
            localSessionCompleteness = localSessionCompletenessAfterFailure(
                snapshot: localSessionCache.successfulSnapshot(for: key),
                error: error
            )
        }
    }

    private func localSessionSnapshotKey(roots: [String]) -> LocalSessionSnapshotKey {
        let agent = agentFilter == .all ? SkillAgentFilter.all.rawValue : agentFilter.rawValue
        return LocalSessionSnapshotKey(
            agent: agent,
            projectRoot: activeProjectContext?.rootPath,
            currentCWD: activeProjectContext?.currentCWD,
            authorizedRoots: roots
        )
    }

    private var activeLocalSessionSnapshot: LocalSessionSnapshot? {
        guard let key = activeLocalSessionSnapshotKey else { return nil }
        return localSessionCache.successfulSnapshot(for: key)
    }

    private func activateLocalSessionSourceCache() {
        if let activeKey = activeLocalSessionSnapshotKey,
           let generation = activeLocalSessionRefreshGeneration {
            localSessionCache.cancelSummaryLoad(key: activeKey, generation: generation)
        }
        localSessionLoadAllTask?.cancel()
        localSessionLoadAllTask = nil
        localSessionLoadAllID = nil
        let key = localSessionSnapshotKey(roots: normalizedLocalSessionPreviewRoots)
        activeLocalSessionSnapshotKey = key
        activeLocalSessionRefreshGeneration = nil
        isPreviewingLocalSessions = false
        localSessionCache.activateSource(key)
        selectedLocalSessionDetailState = nil
        if let snapshot = localSessionCache.successfulSnapshot(for: key) {
            publishLocalSessionSnapshot(snapshot)
        } else {
            localSessionLoadState = .empty
            localSessionPreviewResult = LocalSessionPreviewResult()
            localSessionCompleteness = ListCompletenessState(
                loadedCount: 0,
                totalCount: nil,
                hasMore: false,
                isComplete: false,
                completeness: .unknown,
                incompleteReason: nil,
                loadingPhase: .idle,
                canLoadMore: false,
                canLoadAll: false
            )
            selectedLocalSessionID = nil
            if selectedSidebarSelection?.isSession == true {
                setSidebarSelection(nil)
            }
        }
    }

    private func publishLocalSessionSnapshot(_ snapshot: LocalSessionSnapshot) {
        guard activeLocalSessionSnapshotKey == snapshot.key else { return }
        localSessionLoadState = localSessionCache.summaryStates[snapshot.key] ?? .fresh(snapshot)
        localSessionPreviewResult = snapshot.result
        localSessionCompleteness = localSessionCompletenessState(for: snapshot, phase: .idle)
        normalizeSelectedLocalSession()
        synchronizeSelectedLocalSessionDetailState()
    }

    private func mergeLocalSessionSummaryPage(
        accumulated: LocalSessionPreviewResult?,
        page: LocalSessionPreviewResult,
        newRows: [LocalSessionPreviewRow]
    ) -> LocalSessionPreviewResult {
        let accumulatedRows = accumulated?.sessionRows ?? []
        var seenIDs = Set(accumulatedRows.map(\.id))
        let novelRows = newRows.filter { seenIDs.insert($0.id).inserted }
        let rows = accumulatedRows + novelRows.map(\.summaryOnly)
        return LocalSessionPreviewResult(
            generatedBy: page.generatedBy,
            authorized: page.authorized || (accumulated?.authorized ?? false),
            authorizationRequired: page.authorizationRequired,
            roots: page.roots.isEmpty ? (accumulated?.roots ?? []) : page.roots,
            sessionRows: rows,
            skillUsageRows: page.skillUsageRows.isEmpty
                ? (accumulated?.skillUsageRows ?? [])
                : page.skillUsageRows,
            count: rows.count,
            totalCandidateCount: max(page.totalCandidateCount, accumulated?.totalCandidateCount ?? 0),
            totalMatchedCount: max(page.totalMatchedCount, rows.count),
            offset: 0,
            limit: page.limit,
            hasMore: page.hasMore,
            nextOffset: page.nextOffset,
            nextCursor: page.nextCursor,
            sourceRevision: page.sourceRevision,
            sourceCompleteness: page.sourceCompleteness,
            incompleteReason: page.incompleteReason,
            candidateSetTruncated: page.candidateSetTruncated
                || (accumulated?.candidateSetTruncated ?? false),
            gapNotes: Array(Set((accumulated?.gapNotes ?? []) + page.gapNotes)).sorted(),
            blockerNotes: Array(Set((accumulated?.blockerNotes ?? []) + page.blockerNotes)).sorted(),
            redactionSummary: page.redactionSummary,
            safetyFlags: page.safetyFlags,
            fallbackReason: page.fallbackReason
        )
    }

    private func localSessionCompletenessState(
        for snapshot: LocalSessionSnapshot,
        phase: ListLoadingPhase
    ) -> ListCompletenessState {
        let loadedCount = snapshot.result.sessionRows.count
        let totalCount = max(snapshot.result.totalMatchedCount, loadedCount)
        let hasMore = snapshot.nextCursor != nil && snapshot.result.hasMore
        let isComplete = snapshot.isComplete
        let completeness: ListCompleteness
        if isComplete {
            completeness = .complete
        } else if snapshot.sourceCompleteness == .limited || snapshot.incompleteReason != nil {
            completeness = .incomplete
        } else if snapshot.sourceCompleteness == .unknown {
            completeness = .unknown
        } else {
            completeness = .partial
        }
        let canContinue = phase == .idle
            && hasMore
            && snapshot.nextCursor != nil
            && (snapshot.incompleteReason == nil || snapshot.incompleteReason == .pageFailed)
        return ListCompletenessState(
            loadedCount: loadedCount,
            totalCount: totalCount,
            hasMore: hasMore,
            isComplete: isComplete,
            completeness: completeness,
            incompleteReason: snapshot.incompleteReason,
            loadingPhase: phase,
            canLoadMore: canContinue,
            canLoadAll: canContinue
        )
    }

    private func localSessionCompletenessAfterFailure(
        snapshot: LocalSessionSnapshot?,
        error: Error
    ) -> ListCompletenessState {
        let reason = listFailureReason(for: error)
        guard let snapshot else {
            return ListCompletenessState(
                loadedCount: 0,
                totalCount: nil,
                hasMore: false,
                isComplete: false,
                completeness: .incomplete,
                incompleteReason: reason,
                loadingPhase: .idle,
                canLoadMore: false,
                canLoadAll: true
            )
        }
        let retryable = reason == .pageFailed && snapshot.nextCursor != nil
        return ListCompletenessState(
            loadedCount: snapshot.result.sessionRows.count,
            totalCount: max(snapshot.result.totalMatchedCount, snapshot.result.sessionRows.count),
            hasMore: retryable,
            isComplete: false,
            completeness: retryable ? .partial : .incomplete,
            incompleteReason: reason,
            loadingPhase: .idle,
            canLoadMore: retryable,
            canLoadAll: retryable
        )
    }

    func loadLocalSessionDetailIfNeeded(sessionID: String) async {
        guard let source = activeLocalSessionSnapshotKey,
              let snapshot = localSessionCache.successfulSnapshot(for: source),
              snapshot.result.sessionRows.contains(where: { $0.id == sessionID }) else { return }
        let key = LocalSessionDetailKey(source: source, sessionID: sessionID)
        guard let generation = localSessionCache.beginDetailLoad(for: key) else {
            synchronizeSelectedLocalSessionDetailState()
            return
        }
        synchronizeSelectedLocalSessionDetailState()
        let agent = source.agent == SkillAgentFilter.all.rawValue ? nil : source.agent
        let project = activeProjectContext
        do {
            let result = try await service.previewLocalSessions(
                authorizedRoots: source.authorizedRoots,
                agent: agent,
                scope: .all,
                search: nil,
                project: project,
                sessionID: sessionID,
                includeContentItems: true,
                limit: 1,
                offset: 0,
                sort: .recent,
                direction: .descending
            )
            guard activeLocalSessionSnapshotKey == source else { return }
            guard let detail = result.sessionRows.first(where: { $0.id == sessionID }),
                  detail.contentIncluded else {
                localSessionCache.failDetail(
                    key: key,
                    generation: generation,
                    displayError: "Session detail was unavailable. Retry to load it again."
                )
                if selectedLocalSessionID == sessionID {
                    synchronizeSelectedLocalSessionDetailState()
                }
                return
            }
            guard localSessionCache.publishDetail(
                detail,
                key: key,
                generation: generation
            ) else { return }
            if selectedLocalSessionID == sessionID {
                synchronizeSelectedLocalSessionDetailState()
            }
        } catch {
            localSessionCache.failDetail(
                key: key,
                generation: generation,
                displayError: error.localizedDescription
            )
            if activeLocalSessionSnapshotKey == source, selectedLocalSessionID == sessionID {
                synchronizeSelectedLocalSessionDetailState()
            }
        }
    }

    private func synchronizeSelectedLocalSessionDetailState() {
        guard let source = activeLocalSessionSnapshotKey,
              let selectedLocalSessionID else {
            selectedLocalSessionDetailState = nil
            return
        }
        selectedLocalSessionDetailState = localSessionCache.detailStates[
            LocalSessionDetailKey(source: source, sessionID: selectedLocalSessionID)
        ]
    }

    func updateAppSearch(query: String) {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        appSearchQuery = trimmed
        appSearchTask?.cancel()
        guard !trimmed.isEmpty else {
            appSearchResult = .empty()
            isSearchingApp = false
            return
        }

        appSearchResult = .empty(query: trimmed)
        isSearchingApp = true
        appSearchTask = Task { @MainActor [weak self, trimmed] in
            try? await Task.sleep(nanoseconds: 220_000_000)
            guard let self, !Task.isCancelled else { return }
            await self.performAppSearch(query: trimmed)
        }
    }

    private func performAppSearch(query: String) async {
        let requestedAgentFilter = agentFilter
        let agent = requestedAgentFilter == .all ? nil : requestedAgentFilter.rawValue
        let indexedSkills = skills.filter { agent == nil || $0.agent == agent }
        let indexedSnapshots = agentConfigSnapshots.filter { agent == nil || $0.agent == agent }
        let summaries = activeLocalSessionSnapshot?.result.sessionRows ?? []
        let result = AppSearchIndex(
            skills: indexedSkills,
            sessionSummaries: summaries,
            configSnapshots: indexedSnapshots
        ).search(query: query, limitPerKind: Self.globalSearchLimitPerKind)
        guard appSearchQuery == query, agentFilter == requestedAgentFilter else { return }
        appSearchResult = result

        if appSearchQuery == query {
            isSearchingApp = false
        }
    }

    func selectAppSearchItem(_ item: AppSearchItem) async {
        switch item.kind {
        case .skill:
            guard let skill = item.skill ?? skills.first(where: { $0.id == item.targetID }) else { return }
            if let filter = agentFilter(for: skill.agent) {
                agentFilter = filter
            }
            sidebarContentMode = .skills
            searchText = ""
            stateFilter = .all
            skillScopeFilter = .all
            selectedDetailSection = .overview
            setSelectedSkillID(skill.id, syncSidebar: false)
            setSidebarSelection(.skill(skill.id))
            skillListScrollRequest = SkillListScrollRequest(skillID: skill.id)

        case .session:
            guard let session = item.session
                ?? localSessionPreviewResult.sessionRows.first(where: { $0.id == item.targetID })
            else { return }
            if agentFilter != .all, let filter = agentFilter(for: session.agent) {
                agentFilter = filter
            }
            localSessionScopeFilter = .all
            localSessionSearchText = ""
            sidebarContentMode = .sessions
            localSessionPreviewResult = localSessionPreviewResult.ensuringSession(session)
            selectLocalSession(session, origin: .navigation)

        case .configHistory:
            guard let snapshot = item.configSnapshot
                ?? agentConfigSnapshots.first(where: { $0.id == item.targetID })
            else { return }
            if let filter = agentFilter(for: snapshot.agent) {
                agentFilter = filter
            }
            sidebarContentMode = .config
            configScopeFilter = .all
            configSidebarSearchText = ""
            ensureConfigSnapshot(snapshot)
            selectConfigSnapshot(snapshot)
        }
    }

    func showAllAppSearchResults(kind: AppSearchItemKind, query: String) async {
        let query = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else { return }

        selectedDetailSection = .overview
        switch kind {
        case .skill:
            stateFilter = .all
            skillScopeFilter = .all
            searchText = query
            sidebarContentMode = .skills
            normalizeSelectionToVisibleSkills()

        case .session:
            localSessionScopeFilter = .all
            sidebarContentMode = .sessions
            localSessionSearchText = query
            normalizeSelectedLocalSession()

        case .configHistory:
            configScopeFilter = .all
            sidebarContentMode = .config
            configSidebarSearchText = query
            setSidebarSelection(.configOverview)
        }
    }

    private func ensureConfigSnapshot(_ snapshot: ConfigSnapshotRecord) {
        guard !agentConfigSnapshots.contains(where: { $0.id == snapshot.id }) else { return }
        agentConfigSnapshots.append(snapshot)
        agentConfigSnapshots.sort { lhs, rhs in
            if lhs.createdAt != rhs.createdAt {
                return lhs.createdAt > rhs.createdAt
            }
            return lhs.id > rhs.id
        }
    }

    private func agentFilter(for agent: String?) -> SkillAgentFilter? {
        guard let agent else { return nil }
        return SkillAgentFilter.managementCases.first { $0.rawValue == agent }
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
        clearRollbackConfirmation()
        let previewGeneration = rollbackPreviewGeneration
        errorMessage = nil
        guard agentConfigSnapshots.contains(where: { $0.id == snapshotID }) else {
            let message = "Snapshot is not in the selected agent config timeline."
            errorMessage = message
            throw ServiceClient.ClientError.invalidOutput(message)
        }
        do {
            let preview = try await service.previewSnapshotRollback(snapshotID: snapshotID)
            guard preview.snapshot.id == snapshotID else {
                let message = "Rollback preview did not match the requested snapshot."
                throw ServiceClient.ClientError.invalidOutput(message)
            }
            if previewGeneration == rollbackPreviewGeneration,
               supportsConfigConsistencyProtocol {
                rollbackConfirmation = RollbackConfirmation(preview: preview)
            }
            return preview
        } catch {
            if previewGeneration == rollbackPreviewGeneration {
                errorMessage = error.localizedDescription
            }
            throw error
        }
    }

    func clearRollbackConfirmation() {
        rollbackPreviewGeneration &+= 1
        rollbackConfirmation = nil
    }

    @discardableResult
    func rollbackSnapshot(confirmation: RollbackConfirmation) async -> Bool {
        guard !isRefreshBusy else {
            errorMessage = UIStrings.operationUnavailableBusy
            return false
        }
        guard supportsConfigConsistencyProtocol else {
            errorMessage = UIStrings.configConsistencyProtocolRequired
            lastMutationMessage = nil
            return false
        }
        guard agentConfigSnapshots.contains(where: { $0.id == confirmation.snapshotID }) else {
            errorMessage = "Snapshot is not in the selected agent config timeline."
            lastMutationMessage = nil
            return false
        }
        guard rollbackConfirmation == confirmation else {
            errorMessage = UIStrings.rollbackPreviewAgain
            lastMutationMessage = nil
            return false
        }
        clearRollbackConfirmation()
        let rollbackFeedbackGeneration = rollbackPreviewGeneration
        isWriting = true
        errorMessage = nil
        lastMutationMessage = nil
        defer { isWriting = false }

        do {
            let scannedCount = try await service.rollbackSnapshot(
                snapshotID: confirmation.snapshotID,
                previewToken: confirmation.previewToken
            )
            detailsByID.removeAll()
            try await refreshCollections()
            lastMutationMessage = UIStrings.rollbackRescanned(scannedCount)
            recordLocalRefresh(message: UIStrings.refreshAfterRollback(scannedCount))
            await loadSelectedDetail()
            return true
        } catch ServiceClient.ClientError.service(let error) where error.code == "stale_preview_token" {
            if rollbackFeedbackGeneration == rollbackPreviewGeneration,
               selectedConfigSnapshot?.id == confirmation.snapshotID {
                errorMessage = UIStrings.rollbackPreviewAgain
            }
            return false
        } catch {
            if rollbackFeedbackGeneration == rollbackPreviewGeneration,
               selectedConfigSnapshot?.id == confirmation.snapshotID {
                errorMessage = error.localizedDescription
            }
            return false
        }
    }

    func loadSelectedAgentConfigDataIfNeeded() async {
        guard agentFilter != .all else { return }
        await loadAgentConfigSnapshotsIfNeeded(agent: agentFilter.rawValue)
        await loadCurrentAgentConfigDocumentsIfNeeded(agent: agentFilter.rawValue)
        if agentFilter == .claudeCode {
            await loadClaudeSettingsIfNeeded()
        }
    }
    func refreshSelectedAgentConfigData() async {
        guard agentFilter != .all else { return }
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
                if activeClaudeSettingsRequestKey == requestKey {
                    activeClaudeSettingsRequestKey = nil
                }
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
                if activeAgentConfigDocumentRequestKey == requestKey {
                    activeAgentConfigDocumentRequestKey = nil
                }
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
        if !isSavingSettings {
            configMutationState = .idle
        }
    }

    @discardableResult
    func submitConfigAutosave(content: String, validationError: String?) -> UInt64 {
        configAutosaveDraft = content
        let submittedAgent = agentFilter.rawValue
        let activeRevision = configAutosaveCoordinator.activeSaveRevision
        if validationError != nil, let activeRevision {
            autosaveMutationLane.cancelQueued(
                AutosaveMutationLaneToken(family: .config, revision: activeRevision)
            )
        }
        configAutosaveAgentByRevision = configAutosaveAgentByRevision.filter {
            $0.key == activeRevision
        }
        let binding = makeClaudeSettingsSaveBinding(content: content)
            ?? ConfigSaveBinding(content: content, expectedRevision: "")
        let bindingError = binding.expectedRevision.isEmpty
            ? (supportsConfigConsistencyProtocol
                ? UIStrings.configRevisionUnavailable
                : UIStrings.configConsistencyProtocolRequired)
            : nil
        if let bindingError, validationError == nil {
            settingsErrorMessage = bindingError
            settingsMessage = nil
            configMutationState = .failed(bindingError)
        }
        let revision = configAutosaveCoordinator.submit(
            binding,
            validationError: validationError ?? bindingError
        )
        latestConfigAutosaveRevision = revision
        if validationError == nil, bindingError == nil {
            configAutosaveAgentByRevision[revision] = submittedAgent
        }
        return revision
    }

    @discardableResult
    func submitProviderAutosave(draft: AIProviderSettingsDraft) -> UInt64 {
        providerAutosaveDraft = draft
        if draft.validationMessage != nil,
           let activeRevision = providerAutosaveCoordinator.activeSaveRevision {
            autosaveMutationLane.cancelQueued(
                AutosaveMutationLaneToken(family: .provider, revision: activeRevision)
            )
        }
        let revision = providerAutosaveCoordinator.submit(
            draft,
            validationError: draft.validationMessage
        )
        latestProviderAutosaveRevision = revision
        return revision
    }

    func cancelPendingConfigAutosave() {
        let activeRevision = configAutosaveCoordinator.activeSaveRevision
        if let activeRevision {
            autosaveMutationLane.cancelQueued(
                AutosaveMutationLaneToken(family: .config, revision: activeRevision)
            )
        }
        configAutosaveCoordinator.cancelPendingDebounce()
        configAutosaveAgentByRevision = configAutosaveAgentByRevision.filter {
            $0.key == activeRevision
        }
        latestConfigAutosaveRevision = nil
        configAutosaveDraft = nil
    }

    func cancelPendingProviderAutosave() {
        if let activeRevision = providerAutosaveCoordinator.activeSaveRevision {
            autosaveMutationLane.cancelQueued(
                AutosaveMutationLaneToken(family: .provider, revision: activeRevision)
            )
        }
        providerAutosaveCoordinator.cancelPendingDebounce()
        latestProviderAutosaveRevision = nil
        providerAutosaveDraft = nil
    }

    func flushPendingAutosaves() async {
        await configAutosaveCoordinator.flush()
        await providerAutosaveCoordinator.flush()
    }

    var configAutosaveHasActiveSave: Bool {
        configAutosaveCoordinator.hasActiveSave
    }

    var providerAutosaveHasActiveSave: Bool {
        providerAutosaveCoordinator.hasActiveSave
    }

    private func handleConfigAutosaveCompletion(
        _ completion: RevisionAutosaveCompletion<ConfigSaveBinding>
    ) {
        configAutosaveAgentByRevision.removeValue(forKey: completion.revision)
        let committedRevision = configAutosaveCommittedRevisionByRevision.removeValue(
            forKey: completion.revision
        )
        if completion.succeeded,
           let committedRevision,
           !committedRevision.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            configAutosaveCoordinator.updatePendingValue { pendingBinding in
                guard pendingBinding.expectedRevision == completion.value.expectedRevision else {
                    return pendingBinding
                }
                return ConfigSaveBinding(
                    content: pendingBinding.content,
                    expectedRevision: committedRevision
                )
            }
        }
        guard completion.revision == latestConfigAutosaveRevision,
              completion.succeeded else { return }
        latestConfigAutosaveRevision = nil
        configAutosaveDraft = nil
    }

    private func handleProviderAutosaveCompletion(
        _ completion: RevisionAutosaveCompletion<AIProviderSettingsDraft>
    ) {
        guard completion.revision == latestProviderAutosaveRevision,
              completion.succeeded else { return }
        latestProviderAutosaveRevision = nil
        providerAutosaveDraft = nil
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

        let range = providerObservabilityDateRange.resolved(
            customStartDate: providerObservabilityCustomStartDate,
            customEndDate: providerObservabilityCustomEndDate
        )
        let activityKey = ProviderActivityFilterKey(
            provider: nil,
            model: nil,
            action: nil,
            windowDays: range.windowDays,
            startAt: range.startAt,
            endAt: range.endAt
        )
        let activityGeneration = beginProviderActivityRefresh(for: activityKey)

        do {
            providerObservabilityResult = try await service.providerObservability(
                windowDays: range.windowDays,
                startAt: range.startAt,
                endAt: range.endAt,
                limit: Self.providerObservabilityRowLimit,
                includeHistory: true,
                includeBudgetHints: false,
                includeRetentionRecommendations: false,
                includeEvidence: false
            )
            hasLoadedProviderObservability = true
            await loadInitialProviderActivity(
                for: activityKey,
                generation: activityGeneration
            )
        } catch {
            providerObservabilityResult = .unavailable(reason: error.localizedDescription)
            hasLoadedProviderObservability = true
            failProviderActivity(
                error,
                for: activityKey,
                generation: activityGeneration
            )
        }
    }

    @discardableResult
    func saveAIProviderSettings(draft: AIProviderSettingsDraft) async -> Bool {
        await autosaveMutationLane.perform { [self] in
            await saveAIProviderSettingsInsideMutationLane(draft: draft, autosaveRevision: nil)
        }
    }

    private func saveAIProviderSettingsInsideMutationLane(
        draft: AIProviderSettingsDraft,
        autosaveRevision: UInt64?
    ) async -> Bool {
        guard !isRefreshBusy else {
            publishProviderSaveFeedback(
                autosaveRevision: autosaveRevision,
                message: nil,
                error: UIStrings.operationUnavailableBusy
            )
            return false
        }
        if let validationMessage = draft.validationMessage {
            publishProviderSaveFeedback(
                autosaveRevision: autosaveRevision,
                message: nil,
                error: validationMessage
            )
            return false
        }

        isSavingAIProvider = true
        publishProviderSaveFeedback(
            autosaveRevision: autosaveRevision,
            message: nil,
            error: nil
        )
        defer { isSavingAIProvider = false }

        do {
            let savedStatus = try await service.saveAIProviderSettings(draft: draft)
            guard savedStatus.serviceAvailable else {
                aiProviderStatus = savedStatus
                hasLoadedAIProviderStatus = true
                publishProviderSaveFeedback(
                    autosaveRevision: autosaveRevision,
                    message: nil,
                    error: UIStrings.aiProviderUnavailable
                )
                return false
            }
            aiProviderStatus = savedStatus
            aiProviderTestResult = aiProviderStatus.lastTest
            hasLoadedAIProviderStatus = true
            publishProviderSaveFeedback(
                autosaveRevision: autosaveRevision,
                message: UIStrings.aiProviderSaved,
                error: nil
            )
            return true
        } catch ServiceClient.ClientError.service(let error) where error.code == "unknown_method" {
            publishProviderSaveFeedback(
                autosaveRevision: autosaveRevision,
                message: nil,
                error: UIStrings.aiProviderUnavailable
            )
            return false
        } catch {
            publishProviderSaveFeedback(
                autosaveRevision: autosaveRevision,
                message: nil,
                error: error.localizedDescription
            )
            return false
        }
    }

    private func publishProviderSaveFeedback(
        autosaveRevision: UInt64?,
        message: String?,
        error: String?
    ) {
        guard autosaveRevision == nil || autosaveRevision == latestProviderAutosaveRevision else {
            return
        }
        aiProviderMessage = message
        aiProviderErrorMessage = error
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

    func makeClaudeSettingsSaveBinding(content: String) -> ConfigSaveBinding? {
        guard supportsConfigConsistencyProtocol,
              let expectedRevision = claudeSettings?.revision,
              !expectedRevision.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return nil
        }
        return ConfigSaveBinding(content: content, expectedRevision: expectedRevision)
    }

    @discardableResult
    func saveClaudeSettings(content: String) async -> Bool {
        let binding = makeClaudeSettingsSaveBinding(content: content)
            ?? ConfigSaveBinding(content: content, expectedRevision: "")
        return await saveClaudeSettings(binding: binding)
    }

    func saveClaudeSettings(binding: ConfigSaveBinding) async -> Bool {
        await saveClaudeSettings(binding: binding, submittedAgent: agentFilter.rawValue)
    }

    private func saveClaudeSettings(
        binding: ConfigSaveBinding,
        submittedAgent: String
    ) async -> Bool {
        await autosaveMutationLane.perform { [self] in
            await saveClaudeSettingsInsideMutationLane(
                binding: binding,
                submittedAgent: submittedAgent,
                autosaveRevision: nil
            )
        }
    }

    private func saveClaudeSettingsInsideMutationLane(
        binding: ConfigSaveBinding,
        submittedAgent: String,
        autosaveRevision: UInt64?
    ) async -> Bool {
        guard !isRefreshBusy else {
            publishConfigSaveFeedback(
                autosaveRevision: autosaveRevision,
                message: nil,
                error: UIStrings.operationUnavailableBusy,
                mutationState: .failed(UIStrings.operationUnavailableBusy)
            )
            return false
        }
        guard supportsConfigConsistencyProtocol else {
            publishConfigSaveFeedback(
                autosaveRevision: autosaveRevision,
                message: nil,
                error: UIStrings.configConsistencyProtocolRequired,
                mutationState: .failed(UIStrings.configConsistencyProtocolRequired)
            )
            return false
        }
        guard let currentRevision = claudeSettings?.revision,
              !currentRevision.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            publishConfigSaveFeedback(
                autosaveRevision: autosaveRevision,
                message: nil,
                error: UIStrings.configRevisionUnavailable,
                mutationState: .failed(UIStrings.configRevisionUnavailable)
            )
            return false
        }
        guard currentRevision == binding.expectedRevision else {
            let conflict = ConfigConflictState(
                attemptedRevision: binding.expectedRevision,
                latestRevision: currentRevision,
                displayMessage: UIStrings.configConflict
            )
            publishConfigSaveFeedback(
                autosaveRevision: autosaveRevision,
                message: nil,
                error: conflict.displayMessage,
                mutationState: .conflict(conflict)
            )
            return false
        }
        isSavingSettings = true
        publishConfigSaveFeedback(
            autosaveRevision: autosaveRevision,
            message: nil,
            error: nil,
            mutationState: .saving
        )
        defer { isSavingSettings = false }

        do {
            let savedSettings = try await service.saveClaudeSettings(
                content: binding.content,
                expectedRevision: binding.expectedRevision
            )
            if let autosaveRevision,
               let committedRevision = savedSettings.revision,
               !committedRevision.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                configAutosaveCommittedRevisionByRevision[autosaveRevision] = committedRevision
            }
            invalidateConfigReadGenerations()
            claudeSettings = savedSettings
            detailsByID.removeAll()
            try await refreshCollections(includeSupplementalData: false)
            await refreshConfigCachesAfterSave(submittedAgent: submittedAgent)
            publishConfigSaveFeedback(
                autosaveRevision: autosaveRevision,
                message: UIStrings.savedSettings,
                error: nil,
                mutationState: .idle
            )
            recordLocalRefresh(message: UIStrings.refreshAfterSettingsSave)
            await loadSelectedDetail()
            return true
        } catch ServiceClient.ClientError.service(let error) where error.code == "config_conflict" {
            let latestDocument = try? await service.readClaudeSettings()
            let conflict = ConfigConflictState(
                attemptedRevision: binding.expectedRevision,
                latestRevision: latestDocument?.revision,
                displayMessage: UIStrings.configConflict
            )
            publishConfigSaveFeedback(
                autosaveRevision: autosaveRevision,
                message: nil,
                error: conflict.displayMessage,
                mutationState: .conflict(conflict)
            )
            if let latestDocument {
                claudeSettings = latestDocument
            }
            return false
        } catch {
            publishConfigSaveFeedback(
                autosaveRevision: autosaveRevision,
                message: nil,
                error: error.localizedDescription,
                mutationState: .failed(error.localizedDescription)
            )
            return false
        }
    }

    private func publishConfigSaveFeedback(
        autosaveRevision: UInt64?,
        message: String?,
        error: String?,
        mutationState: ConfigMutationState
    ) {
        guard autosaveRevision == nil || autosaveRevision == latestConfigAutosaveRevision else {
            return
        }
        settingsMessage = message
        settingsErrorMessage = error
        lastMutationMessage = message
        configMutationState = mutationState
    }

    private func invalidateConfigReadGenerations() {
        claudeSettingsLoadGeneration &+= 1
        agentConfigDocumentLoadGeneration &+= 1
        agentConfigSnapshotLoadGeneration &+= 1
        activeClaudeSettingsRequestKey = nil
        activeAgentConfigDocumentRequestKey = nil
        activeAgentConfigSnapshotRequestKey = nil
        loadedClaudeSettingsRequestKey = nil
        loadedAgentConfigDocumentRequestKey = nil
        loadedAgentConfigSnapshotRequestKey = nil
        isLoadingSettings = false
        isLoadingAgentConfigDocuments = false
        agentConfigSnapshotAccumulator.cancel()
        publishAgentConfigSnapshotPaging()
    }

    private func refreshConfigCachesAfterSave(submittedAgent: String) async {
        await loadClaudeSettings()

        let visibleAgent = normalizedConfigAgent(nil)
        if visibleAgent == submittedAgent {
            await loadCurrentAgentConfigDocuments(agent: submittedAgent)
            await loadAgentConfigSnapshots(agent: submittedAgent)
            return
        }

        _ = try? await service.readAgentConfig(agent: submittedAgent)
        _ = try? await service.listAgentConfigSnapshots(agent: submittedAgent, scope: nil)
        if let visibleAgent {
            await loadCurrentAgentConfigDocuments(agent: visibleAgent)
            await loadAgentConfigSnapshots(agent: visibleAgent)
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

    func loadAgentConfigSnapshotsIfNeeded(agent: String? = nil) async {
        await loadAgentConfigSnapshots(agent: agent, force: false)
    }

    func loadAgentConfigSnapshots(agent: String? = nil) async {
        await loadAgentConfigSnapshots(agent: agent, force: true)
    }

    private func loadAgentConfigSnapshots(agent requestedAgent: String? = nil, force: Bool) async {
        guard let agent = normalizedConfigAgent(requestedAgent) else {
            agentConfigSnapshotLoadGeneration &+= 1
            agentConfigSnapshotAccumulator.cancel()
            agentConfigSnapshotCompleteness = agentConfigSnapshotAccumulator.state
            isLoadingAgentConfigSnapshots = false
            normalizeConfigSelection()
            return
        }

        let requestKey = agentConfigRequestKey(agent: agent)
        if !force {
            if loadedAgentConfigSnapshotRequestKey == requestKey || activeAgentConfigSnapshotRequestKey == requestKey {
                return
            }
        }
        guard activeAgentConfigSnapshotRequestKey != requestKey else { return }

        clearRollbackConfirmation()
        if force || loadedAgentConfigSnapshotRequestKey != requestKey {
            cancelAgentConfigSnapshotLoadAll()
            resetAgentConfigSnapshotPaging(clearRows: true)
        }
        activeAgentConfigSnapshotRequestKey = requestKey
        await loadMoreAgentConfigSnapshots(loadAll: true)
        if activeAgentConfigSnapshotRequestKey == requestKey {
            activeAgentConfigSnapshotRequestKey = nil
        }
        if agentConfigSnapshotCompleteness.isComplete {
            loadedAgentConfigSnapshotRequestKey = requestKey
            normalizeConfigSelection()
        }
    }

    func loadMoreAgentConfigSnapshots(loadAll: Bool) async {
        guard let agent = normalizedConfigAgent(nil), !isLoadingAgentConfigSnapshots else { return }
        agentConfigSnapshotLoadGeneration &+= 1
        let generation = agentConfigSnapshotLoadGeneration
        agentConfigSnapshotAccumulator.begin(
            agentConfigSnapshotAccumulator.items.isEmpty ? .initial : (loadAll ? .all : .more)
        )
        publishAgentConfigSnapshotPaging()
        defer {
            if generation == agentConfigSnapshotLoadGeneration {
                agentConfigSnapshotAccumulator.cancel()
                publishAgentConfigSnapshotPaging()
            }
        }

        while true {
            do {
                let result = try await service.listAgentConfigSnapshotPage(
                    agent: agent,
                    scope: nil,
                    limit: 100,
                    cursor: agentConfigSnapshotAccumulator.nextCursor,
                    sourceRevision: agentConfigSnapshotAccumulator.sourceRevision
                )
                guard generation == agentConfigSnapshotLoadGeneration,
                      !Task.isCancelled,
                      normalizedConfigAgent(nil) == agent else { return }
                try agentConfigSnapshotAccumulator.append(result.page)
                publishAgentConfigSnapshotPaging()
            } catch ServiceClient.ClientError.service(let error)
                where error.code == "unknown_method" && agentConfigSnapshotAccumulator.items.isEmpty {
                do {
                    let records = try await service.listAgentConfigSnapshots(agent: agent, scope: nil)
                    guard generation == agentConfigSnapshotLoadGeneration,
                          !Task.isCancelled,
                          normalizedConfigAgent(nil) == agent else { return }
                    try agentConfigSnapshotAccumulator.append(ListPage(
                        items: records,
                        returnedCount: records.count,
                        totalCount: records.count,
                        hasMore: false,
                        nextCursor: nil,
                        sourceRevision: nil,
                        sourceCompleteness: .enumerable,
                        incompleteReason: nil
                    ))
                    publishAgentConfigSnapshotPaging()
                } catch {
                    failAgentConfigSnapshotPaging(error, generation: generation, agent: agent)
                }
                return
            } catch {
                failAgentConfigSnapshotPaging(error, generation: generation, agent: agent)
                return
            }

            guard loadAll,
                  agentConfigSnapshotAccumulator.state.hasMore,
                  agentConfigSnapshotAccumulator.nextCursor != nil else { return }
            agentConfigSnapshotAccumulator.begin(.all)
            publishAgentConfigSnapshotPaging()
        }
    }

    func cancelAgentConfigSnapshotLoadAll() {
        agentConfigSnapshotLoadGeneration &+= 1
        agentConfigSnapshotAccumulator.cancel()
        publishAgentConfigSnapshotPaging()
    }
    private func resetAgentConfigSnapshotPaging(clearRows: Bool) {
        agentConfigSnapshotAccumulator = ListPageAccumulator()
        if clearRows {
            agentConfigSnapshots = []
        }
        publishAgentConfigSnapshotPaging()
    }

    private func publishAgentConfigSnapshotPaging() {
        agentConfigSnapshots = agentConfigSnapshotAccumulator.items
        agentConfigSnapshotCompleteness = agentConfigSnapshotAccumulator.state
        isLoadingAgentConfigSnapshots = agentConfigSnapshotCompleteness.loadingPhase != .idle
    }

    private func failAgentConfigSnapshotPaging(_ error: Error, generation: Int, agent: String) {
        guard generation == agentConfigSnapshotLoadGeneration,
              normalizedConfigAgent(nil) == agent else { return }
        agentConfigSnapshotAccumulator.fail(reason: listFailureReason(for: error))
        publishAgentConfigSnapshotPaging()
        errorMessage = error.localizedDescription
    }

    private func refreshCollections(
        includeSupplementalData: Bool = true,
        includeAIProviderStatus: Bool = true
    ) async throws {
        let snapshot = try await service.appStateSnapshot()
        let fetchedLLMStatus = try await service.llmStatus()
        let fetchedProjectContextState = try await service.getProjectContext()
        let fetchedRuleTuning = try await service.listRuleTuning()
        let fetchedAIProviderStatus = includeAIProviderStatus ? await fetchAIProviderStatus() : nil
        let fetchedLLMPromptRuns: LLMPromptRunListResult?
        let fetchedAgentConfigSnapshots: [ConfigSnapshotRecord]?
        if includeSupplementalData {
            fetchedLLMPromptRuns = await fetchLLMPromptRuns()
            fetchedAgentConfigSnapshots = try await fetchAgentConfigSnapshots()
        } else {
            fetchedLLMPromptRuns = nil
            fetchedAgentConfigSnapshots = nil
        }

        self.status = snapshot.status
        self.llmStatus = fetchedLLMStatus
        self.projectContextState = fetchedProjectContextState
        self.skills = snapshot.skills
        self.catalogListCompleteness = unknownCatalogCompleteness(loadedCount: snapshot.skills.count)
        self.findings = snapshot.findings
        self.ruleTuning = fetchedRuleTuning
        self.conflicts = snapshot.conflicts
        self.healthSummary = snapshot.health
        if let fetchedAIProviderStatus {
            self.aiProviderStatus = fetchedAIProviderStatus
            self.hasLoadedAIProviderStatus = true
            self.aiProviderTestResult = self.aiProviderStatus.lastTest ?? aiProviderTestResult
        }
        if let fetchedLLMPromptRuns {
            self.llmPromptRunList = fetchedLLMPromptRuns
        }
        if let fetchedAgentConfigSnapshots {
            clearRollbackConfirmation()
            self.agentConfigSnapshotAccumulator = ListPageAccumulator()
            try? self.agentConfigSnapshotAccumulator.append(ListPage(
                items: fetchedAgentConfigSnapshots,
                returnedCount: fetchedAgentConfigSnapshots.count,
                totalCount: fetchedAgentConfigSnapshots.count,
                hasMore: false,
                nextCursor: nil,
                sourceRevision: nil,
                sourceCompleteness: .enumerable,
                incompleteReason: nil
            ))
            self.publishAgentConfigSnapshotPaging()
            if let agent = selectedAgentConfigTimelineAgent {
                loadedAgentConfigSnapshotRequestKey = agentConfigRequestKey(agent: agent)
            } else {
                loadedAgentConfigSnapshotRequestKey = nil
            }
        }
        let currentSkillIDs = Set(snapshot.skills.map(\.id))
        scriptExecutionPreviews = scriptExecutionPreviews.filter { currentSkillIDs.contains($0.key) }
        if fetchedLLMPromptRuns != nil {
            hydratePromptSendResultsFromRuns(currentSkillIDs: currentSkillIDs)
        }
        skillEventsByID = skillEventsByID.filter { currentSkillIDs.contains($0.key) }
        skillEventAccumulatorsByID = skillEventAccumulatorsByID.filter { currentSkillIDs.contains($0.key) }
        skillEventCompletenessByID = skillEventCompletenessByID.filter { currentSkillIDs.contains($0.key) }
        skillEventLoadGenerations = skillEventLoadGenerations.filter { currentSkillIDs.contains($0.key) }
        loadingSkillEventIDs = loadingSkillEventIDs.filter { currentSkillIDs.contains($0) }
        batchTogglePreview = nil
        refreshWatcherMessage(from: self.status)
        normalizeSelectionToVisibleSkills()
    }

    private func unknownCatalogCompleteness(loadedCount: Int) -> ListCompletenessState {
        ListCompletenessState(
            loadedCount: loadedCount,
            totalCount: nil,
            hasMore: false,
            isComplete: false,
            completeness: .unknown,
            incompleteReason: nil,
            loadingPhase: .idle,
            canLoadMore: false,
            canLoadAll: false
        )
    }

    private func catalogCompleteness(after result: ScanResult) -> ListCompletenessState {
        guard let activity = result.activity else {
            return ListCompletenessState(
                loadedCount: result.skills.count,
                totalCount: nil,
                hasMore: false,
                isComplete: false,
                completeness: .incomplete,
                incompleteReason: .sourceLimited,
                loadingPhase: .idle,
                canLoadMore: false,
                canLoadAll: false
            )
        }
        let summaries = activity.agentSummaries ?? []
        let issues = summaries.flatMap(\.scanIssues)
        let complete = activity.status == "completed"
            && summaries.allSatisfy { summary in
                summary.status == "completed"
                    && summary.rootsPartial.isEmpty
                    && summary.rootsSkipped.isEmpty
                    && summary.scanIssues.isEmpty
            }
        if complete {
            return ListCompletenessState(
                loadedCount: result.skills.count,
                totalCount: result.skills.count,
                hasMore: false,
                isComplete: true,
                completeness: .complete,
                incompleteReason: nil,
                loadingPhase: .idle,
                canLoadMore: false,
                canLoadAll: false
            )
        }
        let reason: ListIncompleteReason
        if issues.contains(where: { $0.kind == "budget_exceeded" }) {
            reason = .safetyBudget
        } else if !issues.isEmpty {
            reason = .unreadableSource
        } else {
            reason = .sourceLimited
        }
        return ListCompletenessState(
            loadedCount: result.skills.count,
            totalCount: nil,
            hasMore: false,
            isComplete: false,
            completeness: .incomplete,
            incompleteReason: reason,
            loadingPhase: .idle,
            canLoadMore: false,
            canLoadAll: false
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
        var accumulator = ListPageAccumulator<ConfigSnapshotRecord>()
        do {
            while true {
                let result = try await service.listAgentConfigSnapshotPage(
                    agent: agent,
                    scope: nil,
                    limit: 100,
                    cursor: accumulator.nextCursor,
                    sourceRevision: accumulator.sourceRevision
                )
                try accumulator.append(result.page)
                guard accumulator.state.hasMore, accumulator.nextCursor != nil else {
                    return accumulator.items
                }
                accumulator.begin(.all)
            }
        } catch ServiceClient.ClientError.service(let error) where error.code == "unknown_method" {
            return try await service.listAgentConfigSnapshots(agent: agent, scope: nil)
                .filter { $0.agent == agent }
                .sorted { lhs, rhs in
                    if lhs.createdAt != rhs.createdAt {
                        return lhs.createdAt > rhs.createdAt
                    }
                    return lhs.id > rhs.id
                }
        }
    }

    private func loadSkillEventsIfNeeded(instanceID: SkillRecord.ID, force: Bool = false) async {
        if !force, skillEventsByID[instanceID] != nil {
            if skillEventCompletenessByID[instanceID]?.canLoadAll == true {
                await loadMoreSkillEvents(instanceID: instanceID, loadAll: true)
            }
            return
        }
        if force || skillEventAccumulatorsByID[instanceID] == nil {
            cancelSkillEventLoadAll(instanceID: instanceID)
            skillEventAccumulatorsByID[instanceID] = ListPageAccumulator()
            skillEventsByID[instanceID] = []
            publishSkillEventPaging(instanceID: instanceID)
        }
        await loadMoreSkillEvents(instanceID: instanceID, loadAll: true)
    }

    func loadMoreSkillEvents(instanceID: SkillRecord.ID, loadAll: Bool) async {
        guard !loadingSkillEventIDs.contains(instanceID) else { return }
        var accumulator = skillEventAccumulatorsByID[instanceID] ?? ListPageAccumulator()
        skillEventLoadGenerationValue &+= 1
        let generation = skillEventLoadGenerationValue
        skillEventLoadGenerations[instanceID] = generation
        accumulator.begin(accumulator.items.isEmpty ? .initial : (loadAll ? .all : .more))
        skillEventAccumulatorsByID[instanceID] = accumulator
        loadingSkillEventIDs.insert(instanceID)
        publishSkillEventPaging(instanceID: instanceID)
        defer {
            if skillEventLoadGenerations[instanceID] == generation {
                skillEventAccumulatorsByID[instanceID]?.cancel()
                loadingSkillEventIDs.remove(instanceID)
                publishSkillEventPaging(instanceID: instanceID)
            }
        }

        while true {
            guard let current = skillEventAccumulatorsByID[instanceID] else { return }
            do {
                let result = try await service.listSkillEventPage(
                    instanceID: instanceID,
                    limit: 100,
                    cursor: current.nextCursor,
                    sourceRevision: current.sourceRevision
                )
                guard skillEventLoadGenerations[instanceID] == generation,
                      !Task.isCancelled else { return }
                var accepted = skillEventAccumulatorsByID[instanceID] ?? ListPageAccumulator()
                try accepted.append(result.page)
                skillEventAccumulatorsByID[instanceID] = accepted
                publishSkillEventPaging(instanceID: instanceID)
            } catch ServiceClient.ClientError.service(let error)
                where error.code == "unknown_method" && current.items.isEmpty {
                do {
                    let records = try await service.listSkillEvents(instanceID: instanceID)
                    guard skillEventLoadGenerations[instanceID] == generation,
                          !Task.isCancelled else { return }
                    var accepted = skillEventAccumulatorsByID[instanceID] ?? ListPageAccumulator()
                    try accepted.append(ListPage(
                        items: records,
                        returnedCount: records.count,
                        totalCount: records.count,
                        hasMore: false,
                        nextCursor: nil,
                        sourceRevision: nil,
                        sourceCompleteness: .enumerable,
                        incompleteReason: nil
                    ))
                    skillEventAccumulatorsByID[instanceID] = accepted
                    publishSkillEventPaging(instanceID: instanceID)
                } catch {
                    failSkillEventPaging(error, instanceID: instanceID, generation: generation)
                }
                return
            } catch {
                failSkillEventPaging(error, instanceID: instanceID, generation: generation)
                return
            }

            guard loadAll,
                  skillEventAccumulatorsByID[instanceID]?.state.hasMore == true,
                  skillEventAccumulatorsByID[instanceID]?.nextCursor != nil else { return }
            skillEventAccumulatorsByID[instanceID]?.begin(.all)
            publishSkillEventPaging(instanceID: instanceID)
        }
    }

    func cancelSkillEventLoadAll(instanceID: SkillRecord.ID) {
        skillEventLoadGenerationValue &+= 1
        skillEventLoadGenerations[instanceID] = skillEventLoadGenerationValue
        skillEventAccumulatorsByID[instanceID]?.cancel()
        loadingSkillEventIDs.remove(instanceID)
        publishSkillEventPaging(instanceID: instanceID)
    }

    private func publishSkillEventPaging(instanceID: SkillRecord.ID) {
        guard let accumulator = skillEventAccumulatorsByID[instanceID] else {
            skillEventCompletenessByID.removeValue(forKey: instanceID)
            return
        }
        skillEventsByID[instanceID] = accumulator.items
        skillEventCompletenessByID[instanceID] = accumulator.state
    }

    private func failSkillEventPaging(
        _ error: Error,
        instanceID: SkillRecord.ID,
        generation: Int
    ) {
        guard skillEventLoadGenerations[instanceID] == generation else { return }
        skillEventAccumulatorsByID[instanceID]?.fail(reason: listFailureReason(for: error))
        publishSkillEventPaging(instanceID: instanceID)
        if errorMessage == nil {
            errorMessage = error.localizedDescription
        }
    }

    private func listFailureReason(for error: Error) -> ListIncompleteReason {
        if case ServiceClient.ClientError.service(let serviceError) = error,
           serviceError.code == "source_changed" {
            return .sourceChanged
        }
        if case ListPageAccumulatorError.sourceChanged = error {
            return .sourceChanged
        }
        return .pageFailed
    }

    private func llmPromptActionKey(action: LLMAction, skillID: SkillRecord.ID) -> String {
        "action:\(skillID):\(action.rawValue)"
    }

    private var normalizedTaskCockpitText: String {
        taskCockpitText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private func clearTaskCockpitTransientState() {
        taskCockpitResult = nil
        taskCockpitPromptConfirmation = nil
        isPreviewingTaskCockpitPrompt = false
        selectedTaskCockpitHistoryID = nil
        if isBuildingTaskCockpit {
            cancelTaskCockpitBuild(publishFallbackResult: false)
        } else {
            taskCockpitOperationState = .idle
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

    private var normalizedLocalSessionPreviewRoots: [String] {
        localSessionPreviewRoots
            .split(whereSeparator: { $0 == "," || $0 == "\n" || $0 == ";" })
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
    }

    private var normalizedLocalSessionSearchText: String {
        localSessionSearchText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private func normalizedOptional(_ value: String) -> String? {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
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

    private func runBelongsTo(_ run: LLMPromptRunRecord, skillID: SkillRecord.ID) -> Bool {
        run.instanceID == skillID || run.instanceIDs.contains(skillID)
    }

    private func llmPromptKey(for run: LLMPromptRunRecord) -> String? {
        let skillID = run.instanceID ?? run.instanceIDs.first
        switch run.requestKind {
        case "action":
            guard let skillID, let action = LLMAction(rawValue: run.action) else { return nil }
            return llmPromptActionKey(action: action, skillID: skillID)
        default:
            guard let skillID, let action = LLMAction(rawValue: run.action) else { return nil }
            return llmPromptActionKey(action: action, skillID: skillID)
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
            selectedLocalSessionDetailState = nil
            if selectedSidebarSelection?.isSession == true {
                setSidebarSelection(nil)
                selectedDetailSection = .overview
            }
            return
        }
        if let selectedLocalSessionID, rows.contains(where: { $0.id == selectedLocalSessionID }) {
            synchronizeSelectedLocalSessionDetailState()
            return
        }
        let firstSessionID = rows[0].id
        selectedLocalSessionID = firstSessionID
        synchronizeSelectedLocalSessionDetailState()
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
            if activity.status == "completed-partial" {
                let partialSummary = activity.agentSummaries?.first { summary in
                    summary.status == "completed-partial"
                }
                let issue = partialSummary?.scanIssues.first
                let issueText = issue.map { issue in
                    UIStrings.refreshPartialIssue(
                        kind: issue.kind,
                        path: issue.path,
                        detail: issue.detail
                    )
                } ?? UIStrings.refreshPartialIssueUnavailable
                let recovery = partialSummary?.recoveryActions.first
                    ?? activity.recoveryActions.first
                    ?? UIStrings.refreshPartialRecoveryDefault
                refreshStatusMessage = UIStrings.refreshScanPartial(
                    activity.scannedCount,
                    activity.skillCount,
                    activity.findingCount,
                    sameAgentRuntimeConflictCount,
                    issue: issueText,
                    recovery: recovery
                )
                partialScanWarningMessage = refreshStatusMessage
                // DetailFeedbackInlineView keeps this degraded completion visible
                // as a warning instead of replacing it with a generic success.
                lastMutationMessage = refreshStatusMessage
            } else {
                partialScanWarningMessage = nil
                refreshStatusMessage = UIStrings.refreshScanComplete(
                    activity.scannedCount,
                    activity.skillCount,
                    activity.findingCount,
                    sameAgentRuntimeConflictCount
                )
            }
            refreshLogEntries = activity.logEntries + refreshLogEntries
            trimRefreshLog()
        } else {
            partialScanWarningMessage = nil
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
