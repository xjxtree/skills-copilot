import Foundation

@MainActor
extension SkillStore {
    var supportsConfigConsistencyProtocol: Bool {
        (status?.protocolVersion ?? 0) >= 2
    }

    var supportsConfigActionLifecycle: Bool {
        guard supportsConfigConsistencyProtocol else { return false }
        let methods = Set(status?.supportedMethods ?? [])
        return methods.contains("config.previewSaveClaudeSettings")
            && methods.contains("config.saveClaudeSettings")
            && methods.contains("snapshot.previewRollback")
            && methods.contains("snapshot.rollback")
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

    func skillManagerSourcePath(for localSkill: SkillRecord) -> String {
        let url = URL(fileURLWithPath: localSkill.path)
        if url.lastPathComponent.caseInsensitiveCompare("SKILL.md") == .orderedSame {
            return url.deletingLastPathComponent().path
        }
        return localSkill.path
    }

    func skillManagerApplyMustRetirePreview(_ error: Error) -> Bool {
        guard case let ServiceClient.ClientError.service(payload) = error else { return false }
        return payload.details?.retryAllowed == false
            || [
                "stale_action_reference",
                "unknown_action_reference",
                "action_target_mismatch",
                "no_applicable_action"
            ].contains(payload.code)
    }

    func canonicalSkillManagerAgentIDs(_ agents: [String]) -> [String] {
        Array(
            Set(
                agents
                    .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                    .filter { !$0.isEmpty }
            )
        ).sorted()
    }

    func parsedSkillManagerSkillNames(from rawValue: String) -> [String] {
        rawValue
            .split { character in
                character == "," || character == "\n" || character == ";"
            }
            .map { String($0).trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
    }

    var activeLocalSessionSnapshot: LocalSessionSnapshot? {
        guard let key = activeLocalSessionSnapshotKey else { return nil }
        return localSessionCache.successfulSnapshot(for: key)
    }

    func unknownCatalogCompleteness(loadedCount: Int) -> ListCompletenessState {
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

    func catalogCompleteness(after result: ScanResult) -> ListCompletenessState {
        let currentSkills = SkillListModel.currentSkills(result.skills)
        guard let activity = result.activity else {
            return catalogCompletenessState(
                loadedCount: currentSkills.count,
                isComplete: false,
                incompleteReason: .sourceLimited
            )
        }
        let summaries = activity.agentSummaries ?? []
        let complete = activity.status == "completed"
            && summaries.allSatisfy(\.provesCatalogCompleteness)
        let incompleteSummaries = summaries.filter { !$0.provesCatalogCompleteness }
        let reason = catalogIncompleteReason(for: incompleteSummaries)
        return catalogCompletenessState(
            loadedCount: currentSkills.count,
            isComplete: complete,
            incompleteReason: complete ? nil : reason
        )
    }

    func catalogCompletenessByAgent(after result: ScanResult) -> [String: ListCompletenessState] {
        guard let summaries = result.activity?.agentSummaries else { return [:] }
        let currentSkills = SkillListModel.currentSkills(result.skills)
        return summaries.reduce(into: [String: ListCompletenessState]()) { states, summary in
            states[summary.agent] = catalogCompletenessState(
                loadedCount: currentSkills.filter { $0.agent == summary.agent }.count,
                isComplete: summary.provesCatalogCompleteness,
                incompleteReason: summary.provesCatalogCompleteness
                    ? nil
                    : summary.catalogIncompleteReason
            )
        }
    }

    private func catalogIncompleteReason(
        for summaries: [AgentRefreshSummary]
    ) -> ListIncompleteReason {
        let reasons = summaries.map(\.catalogIncompleteReason)
        if reasons.contains(.safetyBudget) {
            return .safetyBudget
        }
        if reasons.contains(.unreadableSource) {
            return .unreadableSource
        }
        return .sourceLimited
    }

    private func catalogCompletenessState(
        loadedCount: Int,
        isComplete: Bool,
        incompleteReason: ListIncompleteReason?
    ) -> ListCompletenessState {
        ListCompletenessState(
            loadedCount: loadedCount,
            totalCount: isComplete ? loadedCount : nil,
            hasMore: false,
            isComplete: isComplete,
            completeness: isComplete ? .complete : .incomplete,
            incompleteReason: incompleteReason,
            loadingPhase: .idle,
            canLoadMore: false,
            canLoadAll: false
        )
    }

    func listFailureReason(for error: Error) -> ListIncompleteReason {
        if case ServiceClient.ClientError.service(let serviceError) = error,
           serviceError.code == "source_changed" {
            return .sourceChanged
        }
        if case ListPageAccumulatorError.sourceChanged = error {
            return .sourceChanged
        }
        return .pageFailed
    }

    func llmPromptActionKey(action: LLMAction, skillID: SkillRecord.ID) -> String {
        "action:\(skillID):\(action.rawValue)"
    }

    func normalizedTaskCockpitAgentIDs(_ agentIDs: [String]) -> [String] {
        let orderedAgents = SkillAgentFilter.managementCases.map(\.rawValue)
        let selected = Set(agentIDs.map { $0.trimmingCharacters(in: .whitespacesAndNewlines) })
        return orderedAgents.filter { selected.contains($0) }
    }

    func taskCockpitCandidateSkillIDs(for agentIDs: [String]) -> [SkillRecord.ID] {
        let selectedAgents = Set(normalizedTaskCockpitAgentIDs(agentIDs))
        guard !selectedAgents.isEmpty else { return [] }
        return skills
            .filter { skill in
                selectedAgents.contains(skill.agent)
                    && DisplayText.statusKind(skill.state, enabled: skill.enabled) == .enabled
            }
            .map(\.id)
    }

    var normalizedLocalSessionPreviewRoots: [String] {
        localSessionPreviewRoots
            .split(whereSeparator: { $0 == "," || $0 == "\n" || $0 == ";" })
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
    }

    private var normalizedLocalSessionSearchText: String {
        localSessionSearchText.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    func canSendLLMPrompt(_ preview: LLMPromptPreview) -> Bool {
        aiProviderStatus.serviceAvailable
            && aiProviderStatus.configured
            && aiProviderStatus.activeProfile != nil
            && preview.enabled
            && !preview.previewID.isEmpty
            && preview.confirmationRequired
            && preview.actionConfirmation != nil
            && !preview.rawPromptPersisted
            && !preview.rawResponsePersisted
    }

    func canStartScan(allowDuringProjectUpdate: Bool) -> Bool {
        if isLoading || isScanning || isWriting || isSavingSettings || isApplyingBatchToggle {
            return false
        }
        if isProjectUpdating, !allowDuringProjectUpdate {
            return false
        }
        return true
    }

    func localBatchTogglePreview(
        selectedSkills: [SkillRecord],
        reason: String
    ) -> BatchTogglePreview {
        var affected: [BatchToggleSkillItem] = []
        var skipped: [BatchToggleSkillItem] = []
        for skill in selectedSkills {
            if let skipReason = batchToggleSkipReason(for: skill) {
                skipped.append(
                    BatchToggleSkillItem(
                        skill: skill,
                        targetEnabled: batchToggleAction.targetEnabled,
                        reason: skipReason
                    )
                )
            } else if DisplayText.statusKind(skill.state, enabled: skill.enabled)
                == (batchToggleAction.targetEnabled ? .enabled : .disabled) {
                skipped.append(
                    BatchToggleSkillItem(
                        skill: skill,
                        targetEnabled: batchToggleAction.targetEnabled,
                        reason: UIStrings.batchToggleAlreadyInTargetState(
                            batchToggleAction.title.lowercased()
                        )
                    )
                )
            } else {
                affected.append(
                    BatchToggleSkillItem(
                        skill: skill,
                        targetEnabled: batchToggleAction.targetEnabled
                    )
                )
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
            return capability.configToggle.reason
                ?? UIStrings.readOnlyAdapterStatus(capability.displayName)
        }
        if !capability.writable.supported {
            return capability.writable.reason
                ?? UIStrings.batchToggleWritableMissing(capability.displayName)
        }
        return nil
    }
}
