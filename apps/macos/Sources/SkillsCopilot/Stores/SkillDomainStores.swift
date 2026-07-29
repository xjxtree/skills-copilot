import Combine
import Foundation

@MainActor
final class SessionStore: ObservableObject {
    var onPreviewResultChanged: (() -> Void)?
    var onPreviewRootsChanged: (() -> Void)?
    var onScopeChanged: (() -> Void)?
    var onSortChanged: (() -> Void)?
    var onSearchChanged: (() -> Void)?

    @Published var localSessionPreviewResult = LocalSessionPreviewResult() {
        didSet { onPreviewResultChanged?() }
    }
    @Published var localSessionLoadState: LocalSessionLoadState = .empty
    @Published var localSessionCompleteness = ListCompletenessState(
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
    @Published var selectedLocalSessionDetailState: LocalSessionDetailState?
    @Published var selectedLocalSessionMessageCompleteness = ListCompletenessState(
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
    @Published var isPreviewingLocalSessions = false
    @Published var localSessionPreviewRoots = "" {
        didSet {
            guard oldValue != localSessionPreviewRoots else { return }
            onPreviewRootsChanged?()
        }
    }
    @Published var localSessionScopeFilter: LocalSessionScopeFilter = .project {
        didSet {
            guard oldValue != localSessionScopeFilter else { return }
            onScopeChanged?()
        }
    }
    @Published var localSessionSortOrder: LocalSessionSortOrder = .recent {
        didSet {
            guard oldValue != localSessionSortOrder else { return }
            onSortChanged?()
        }
    }
    @Published var localSessionSortDirection: SkillSortDirection = .descending {
        didSet {
            guard oldValue != localSessionSortDirection else { return }
            onSortChanged?()
        }
    }
    @Published var localSessionSearchText = "" {
        didSet {
            guard oldValue != localSessionSearchText else { return }
            onSearchChanged?()
        }
    }
    @Published var selectedLocalSessionID: LocalSessionPreviewRow.ID?
}

@MainActor
final class ProviderStore: ObservableObject {
    var onObservabilityCriteriaChanged: (() -> Void)?

    @Published var llmStatus = LLMStatus.disabledFallback()
    @Published var aiProviderStatus = AIProviderStatus.unavailable()
    @Published var aiProviderTestResult: AIProviderTestResult?
    @Published var llmPrepareResults: [LLMAction: LLMPrepareResult] = [:]
    @Published var preparingLLMActions: Set<LLMAction> = []
    @Published var llmPromptPreviews: [String: LLMPromptPreview] = [:]
    @Published var previewingLLMPromptKeys: Set<String> = []
    @Published var sendingLLMPromptKeys: Set<String> = []
    @Published var llmPromptSendResults: [String: LLMPromptSendResult] = [:]
    @Published var llmPromptRunList = LLMPromptRunListResult.unavailable()
    @Published var isLoadingLLMPromptRuns = false
    @Published var providerObservabilityResult: ProviderObservabilityResult?
    @Published var isLoadingProviderObservability = false
    @Published var providerObservabilityDateRange: ProviderObservabilityDateRangePreset = .last30Days {
        didSet {
            guard oldValue != providerObservabilityDateRange else { return }
            onObservabilityCriteriaChanged?()
        }
    }
    @Published var providerObservabilityCustomStartDate: Date =
        Calendar.current.date(byAdding: .day, value: -30, to: Date()) ?? Date()
    {
        didSet {
            guard oldValue != providerObservabilityCustomStartDate else { return }
            guard providerObservabilityDateRange == .custom else { return }
            onObservabilityCriteriaChanged?()
        }
    }
    @Published var providerObservabilityCustomEndDate = Date() {
        didSet {
            guard oldValue != providerObservabilityCustomEndDate else { return }
            guard providerObservabilityDateRange == .custom else { return }
            onObservabilityCriteriaChanged?()
        }
    }
    @Published var isLoadingAIProvider = false
    @Published var isSavingAIProvider = false
    @Published var isTestingAIProvider = false
    @Published var providerAutosavePhase: RevisionAutosavePhase = .idle
    @Published var providerAutosaveDraft: AIProviderSettingsDraft?
    @Published var aiProviderMessage: String?
    @Published var aiProviderErrorMessage: String?
}

@MainActor
final class SkillManagerStore: ObservableObject {
    var onSearchCriteriaChanged: (() -> Void)?
    var onMutationCriteriaChanged: (() -> Void)?
    var onLocalCreateCriteriaChanged: (() -> Void)?

    @Published var isSkillManagerPresented = false
    @Published var skillManagerTools: [SkillManagerToolRecord] = []
    @Published var skillManagerSearchResult: SkillManagerSearchRecord?
    @Published var skillManagerInstalledByScope: [SkillManagerScope: SkillManagerInstalledListRecord] = [:]
    @Published var skillManagerSearchVisibility = SkillManagerVisibleResults<String>()
    @Published var skillManagerMutationConfirmation: SkillManagerMutationConfirmation?
    @Published var skillManagerLocalCreateConfirmation: SkillManagerLocalCreateConfirmation?
    @Published var skillManagerLocalDeleteConfirmation: SkillManagerLocalDeleteConfirmation?
    @Published var skillManagerLocalArchiveImportConfirmation: SkillManagerLocalArchiveImportConfirmation?
    @Published var skillManagerLocalArchiveUpdateConfirmation: SkillManagerLocalArchiveUpdateConfirmation?
    @Published var skillManagerErrorMessage: String?
    @Published var skillManagerMessage: String?
    @Published var isLoadingSkillManagerTools = false
    @Published var isSearchingSkillManager = false
    @Published var isListingSkillManagerInstalled = false
    @Published var isPreviewingSkillManagerMutation = false
    @Published var isPreviewingSkillManagerLocalCreate = false
    @Published var isPreviewingSkillManagerLocalDelete = false
    @Published var isPreviewingSkillManagerLocalArchiveImport = false
    @Published var isPreviewingSkillManagerLocalArchiveUpdate = false
    @Published var isApplyingSkillManagerMutation = false
    @Published var skillManagerSearchQuery = "" {
        didSet {
            guard oldValue != skillManagerSearchQuery else { return }
            onSearchCriteriaChanged?()
        }
    }
    @Published var skillManagerOwner = "" {
        didSet {
            guard oldValue != skillManagerOwner else { return }
            onSearchCriteriaChanged?()
        }
    }
    @Published var skillManagerSource = "" {
        didSet {
            guard oldValue != skillManagerSource else { return }
            onMutationCriteriaChanged?()
        }
    }
    @Published var skillManagerSkillName = "" {
        didSet {
            guard oldValue != skillManagerSkillName else { return }
            onMutationCriteriaChanged?()
        }
    }
    @Published var skillManagerInstallSkillName = "" {
        didSet {
            guard oldValue != skillManagerInstallSkillName else { return }
            onMutationCriteriaChanged?()
        }
    }
    @Published var skillManagerRemoveSkillName = "" {
        didSet {
            guard oldValue != skillManagerRemoveSkillName else { return }
            onMutationCriteriaChanged?()
        }
    }
    @Published var skillManagerLocalSkillName = "" {
        didSet {
            guard oldValue != skillManagerLocalSkillName else { return }
            onLocalCreateCriteriaChanged?()
        }
    }
    @Published var skillManagerNetworkAllowed = true {
        didSet {
            guard oldValue != skillManagerNetworkAllowed else { return }
            onSearchCriteriaChanged?()
            onMutationCriteriaChanged?()
        }
    }
    @Published var skillManagerScope: SkillManagerScope = .project {
        didSet {
            guard oldValue != skillManagerScope else { return }
            onMutationCriteriaChanged?()
        }
    }
    @Published var skillManagerDistribution: SkillManagerDistribution = .symlink {
        didSet {
            guard oldValue != skillManagerDistribution else { return }
            onMutationCriteriaChanged?()
        }
    }
    @Published var skillManagerSelectedAgentIDs =
        Set(SkillManagerAgent.defaultTargets.map(\.rawValue))
    {
        didSet {
            guard oldValue != skillManagerSelectedAgentIDs else { return }
            onMutationCriteriaChanged?()
        }
    }
}

extension SkillStore {
    var localSessionPreviewResult: LocalSessionPreviewResult {
        get { sessionStore.localSessionPreviewResult }
        set { sessionStore.localSessionPreviewResult = newValue }
    }
    var localSessionLoadState: LocalSessionLoadState {
        get { sessionStore.localSessionLoadState }
        set { sessionStore.localSessionLoadState = newValue }
    }
    var localSessionCompleteness: ListCompletenessState {
        get { sessionStore.localSessionCompleteness }
        set { sessionStore.localSessionCompleteness = newValue }
    }
    var selectedLocalSessionDetailState: LocalSessionDetailState? {
        get { sessionStore.selectedLocalSessionDetailState }
        set { sessionStore.selectedLocalSessionDetailState = newValue }
    }
    var selectedLocalSessionMessageCompleteness: ListCompletenessState {
        get { sessionStore.selectedLocalSessionMessageCompleteness }
        set { sessionStore.selectedLocalSessionMessageCompleteness = newValue }
    }
    var isPreviewingLocalSessions: Bool {
        get { sessionStore.isPreviewingLocalSessions }
        set { sessionStore.isPreviewingLocalSessions = newValue }
    }
    var localSessionPreviewRoots: String {
        get { sessionStore.localSessionPreviewRoots }
        set { sessionStore.localSessionPreviewRoots = newValue }
    }
    var localSessionScopeFilter: LocalSessionScopeFilter {
        get { sessionStore.localSessionScopeFilter }
        set { sessionStore.localSessionScopeFilter = newValue }
    }
    var localSessionSortOrder: LocalSessionSortOrder {
        get { sessionStore.localSessionSortOrder }
        set { sessionStore.localSessionSortOrder = newValue }
    }
    var localSessionSortDirection: SkillSortDirection {
        get { sessionStore.localSessionSortDirection }
        set { sessionStore.localSessionSortDirection = newValue }
    }
    var localSessionSearchText: String {
        get { sessionStore.localSessionSearchText }
        set { sessionStore.localSessionSearchText = newValue }
    }
    var selectedLocalSessionID: LocalSessionPreviewRow.ID? {
        get { sessionStore.selectedLocalSessionID }
        set { sessionStore.selectedLocalSessionID = newValue }
    }

    var llmStatus: LLMStatus {
        get { providerStore.llmStatus }
        set { providerStore.llmStatus = newValue }
    }
    var aiProviderStatus: AIProviderStatus {
        get { providerStore.aiProviderStatus }
        set { providerStore.aiProviderStatus = newValue }
    }
    var aiProviderTestResult: AIProviderTestResult? {
        get { providerStore.aiProviderTestResult }
        set { providerStore.aiProviderTestResult = newValue }
    }
    var llmPrepareResults: [LLMAction: LLMPrepareResult] {
        get { providerStore.llmPrepareResults }
        set { providerStore.llmPrepareResults = newValue }
    }
    var preparingLLMActions: Set<LLMAction> {
        get { providerStore.preparingLLMActions }
        set { providerStore.preparingLLMActions = newValue }
    }
    var llmPromptPreviews: [String: LLMPromptPreview] {
        get { providerStore.llmPromptPreviews }
        set { providerStore.llmPromptPreviews = newValue }
    }
    var previewingLLMPromptKeys: Set<String> {
        get { providerStore.previewingLLMPromptKeys }
        set { providerStore.previewingLLMPromptKeys = newValue }
    }
    var sendingLLMPromptKeys: Set<String> {
        get { providerStore.sendingLLMPromptKeys }
        set { providerStore.sendingLLMPromptKeys = newValue }
    }
    var llmPromptSendResults: [String: LLMPromptSendResult] {
        get { providerStore.llmPromptSendResults }
        set { providerStore.llmPromptSendResults = newValue }
    }
    var llmPromptRunList: LLMPromptRunListResult {
        get { providerStore.llmPromptRunList }
        set { providerStore.llmPromptRunList = newValue }
    }
    var isLoadingLLMPromptRuns: Bool {
        get { providerStore.isLoadingLLMPromptRuns }
        set { providerStore.isLoadingLLMPromptRuns = newValue }
    }
    var providerObservabilityResult: ProviderObservabilityResult? {
        get { providerStore.providerObservabilityResult }
        set { providerStore.providerObservabilityResult = newValue }
    }
    var isLoadingProviderObservability: Bool {
        get { providerStore.isLoadingProviderObservability }
        set { providerStore.isLoadingProviderObservability = newValue }
    }
    var providerObservabilityDateRange: ProviderObservabilityDateRangePreset {
        get { providerStore.providerObservabilityDateRange }
        set { providerStore.providerObservabilityDateRange = newValue }
    }
    var providerObservabilityCustomStartDate: Date {
        get { providerStore.providerObservabilityCustomStartDate }
        set { providerStore.providerObservabilityCustomStartDate = newValue }
    }
    var providerObservabilityCustomEndDate: Date {
        get { providerStore.providerObservabilityCustomEndDate }
        set { providerStore.providerObservabilityCustomEndDate = newValue }
    }
    var isLoadingAIProvider: Bool {
        get { providerStore.isLoadingAIProvider }
        set { providerStore.isLoadingAIProvider = newValue }
    }
    var isSavingAIProvider: Bool {
        get { providerStore.isSavingAIProvider }
        set { providerStore.isSavingAIProvider = newValue }
    }
    var isTestingAIProvider: Bool {
        get { providerStore.isTestingAIProvider }
        set { providerStore.isTestingAIProvider = newValue }
    }
    var providerAutosavePhase: RevisionAutosavePhase {
        get { providerStore.providerAutosavePhase }
        set { providerStore.providerAutosavePhase = newValue }
    }
    var providerAutosaveDraft: AIProviderSettingsDraft? {
        get { providerStore.providerAutosaveDraft }
        set { providerStore.providerAutosaveDraft = newValue }
    }
    var aiProviderMessage: String? {
        get { providerStore.aiProviderMessage }
        set { providerStore.aiProviderMessage = newValue }
    }
    var aiProviderErrorMessage: String? {
        get { providerStore.aiProviderErrorMessage }
        set { providerStore.aiProviderErrorMessage = newValue }
    }

    var skillManagerTools: [SkillManagerToolRecord] {
        get { skillManagerStore.skillManagerTools }
        set { skillManagerStore.skillManagerTools = newValue }
    }
    var skillManagerSearchResult: SkillManagerSearchRecord? {
        get { skillManagerStore.skillManagerSearchResult }
        set { skillManagerStore.skillManagerSearchResult = newValue }
    }
    var skillManagerInstalledByScope: [SkillManagerScope: SkillManagerInstalledListRecord] {
        get { skillManagerStore.skillManagerInstalledByScope }
        set { skillManagerStore.skillManagerInstalledByScope = newValue }
    }
    var skillManagerSearchVisibility: SkillManagerVisibleResults<String> {
        get { skillManagerStore.skillManagerSearchVisibility }
        set { skillManagerStore.skillManagerSearchVisibility = newValue }
    }
    var skillManagerMutationConfirmation: SkillManagerMutationConfirmation? {
        get { skillManagerStore.skillManagerMutationConfirmation }
        set { skillManagerStore.skillManagerMutationConfirmation = newValue }
    }
    var skillManagerLocalCreateConfirmation: SkillManagerLocalCreateConfirmation? {
        get { skillManagerStore.skillManagerLocalCreateConfirmation }
        set { skillManagerStore.skillManagerLocalCreateConfirmation = newValue }
    }
    var skillManagerLocalDeleteConfirmation: SkillManagerLocalDeleteConfirmation? {
        get { skillManagerStore.skillManagerLocalDeleteConfirmation }
        set { skillManagerStore.skillManagerLocalDeleteConfirmation = newValue }
    }
    var skillManagerLocalArchiveImportConfirmation: SkillManagerLocalArchiveImportConfirmation? {
        get { skillManagerStore.skillManagerLocalArchiveImportConfirmation }
        set { skillManagerStore.skillManagerLocalArchiveImportConfirmation = newValue }
    }
    var skillManagerLocalArchiveUpdateConfirmation: SkillManagerLocalArchiveUpdateConfirmation? {
        get { skillManagerStore.skillManagerLocalArchiveUpdateConfirmation }
        set { skillManagerStore.skillManagerLocalArchiveUpdateConfirmation = newValue }
    }
    var skillManagerErrorMessage: String? {
        get { skillManagerStore.skillManagerErrorMessage }
        set { skillManagerStore.skillManagerErrorMessage = newValue }
    }
    var skillManagerMessage: String? {
        get { skillManagerStore.skillManagerMessage }
        set { skillManagerStore.skillManagerMessage = newValue }
    }
    var isLoadingSkillManagerTools: Bool {
        get { skillManagerStore.isLoadingSkillManagerTools }
        set { skillManagerStore.isLoadingSkillManagerTools = newValue }
    }
    var isSearchingSkillManager: Bool {
        get { skillManagerStore.isSearchingSkillManager }
        set { skillManagerStore.isSearchingSkillManager = newValue }
    }
    var isListingSkillManagerInstalled: Bool {
        get { skillManagerStore.isListingSkillManagerInstalled }
        set { skillManagerStore.isListingSkillManagerInstalled = newValue }
    }
    var isPreviewingSkillManagerMutation: Bool {
        get { skillManagerStore.isPreviewingSkillManagerMutation }
        set { skillManagerStore.isPreviewingSkillManagerMutation = newValue }
    }
    var isPreviewingSkillManagerLocalCreate: Bool {
        get { skillManagerStore.isPreviewingSkillManagerLocalCreate }
        set { skillManagerStore.isPreviewingSkillManagerLocalCreate = newValue }
    }
    var isPreviewingSkillManagerLocalDelete: Bool {
        get { skillManagerStore.isPreviewingSkillManagerLocalDelete }
        set { skillManagerStore.isPreviewingSkillManagerLocalDelete = newValue }
    }
    var isPreviewingSkillManagerLocalArchiveImport: Bool {
        get { skillManagerStore.isPreviewingSkillManagerLocalArchiveImport }
        set { skillManagerStore.isPreviewingSkillManagerLocalArchiveImport = newValue }
    }
    var isPreviewingSkillManagerLocalArchiveUpdate: Bool {
        get { skillManagerStore.isPreviewingSkillManagerLocalArchiveUpdate }
        set { skillManagerStore.isPreviewingSkillManagerLocalArchiveUpdate = newValue }
    }
    var isApplyingSkillManagerMutation: Bool {
        get { skillManagerStore.isApplyingSkillManagerMutation }
        set { skillManagerStore.isApplyingSkillManagerMutation = newValue }
    }
    var skillManagerSearchQuery: String {
        get { skillManagerStore.skillManagerSearchQuery }
        set { skillManagerStore.skillManagerSearchQuery = newValue }
    }
    var skillManagerOwner: String {
        get { skillManagerStore.skillManagerOwner }
        set { skillManagerStore.skillManagerOwner = newValue }
    }
    var skillManagerSource: String {
        get { skillManagerStore.skillManagerSource }
        set { skillManagerStore.skillManagerSource = newValue }
    }
    var skillManagerSkillName: String {
        get { skillManagerStore.skillManagerSkillName }
        set { skillManagerStore.skillManagerSkillName = newValue }
    }
    var skillManagerInstallSkillName: String {
        get { skillManagerStore.skillManagerInstallSkillName }
        set { skillManagerStore.skillManagerInstallSkillName = newValue }
    }
    var skillManagerRemoveSkillName: String {
        get { skillManagerStore.skillManagerRemoveSkillName }
        set { skillManagerStore.skillManagerRemoveSkillName = newValue }
    }
    var skillManagerLocalSkillName: String {
        get { skillManagerStore.skillManagerLocalSkillName }
        set { skillManagerStore.skillManagerLocalSkillName = newValue }
    }
    var skillManagerNetworkAllowed: Bool {
        get { skillManagerStore.skillManagerNetworkAllowed }
        set { skillManagerStore.skillManagerNetworkAllowed = newValue }
    }
    var skillManagerScope: SkillManagerScope {
        get { skillManagerStore.skillManagerScope }
        set { skillManagerStore.skillManagerScope = newValue }
    }
    var skillManagerDistribution: SkillManagerDistribution {
        get { skillManagerStore.skillManagerDistribution }
        set { skillManagerStore.skillManagerDistribution = newValue }
    }
    var skillManagerSelectedAgentIDs: Set<String> {
        get { skillManagerStore.skillManagerSelectedAgentIDs }
        set { skillManagerStore.skillManagerSelectedAgentIDs = newValue }
    }
}
