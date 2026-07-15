import Foundation

@MainActor
extension SkillStore {
    var isRefreshBusy: Bool {
        isLoading
            || isScanning
            || isWriting
            || isProjectUpdating
            || isSavingSettings
            || isSavingAIProvider
            || isTestingAIProvider
            || isApplyingBatchToggle
            // Skill Manager reads/previews use surface-local busy state and may run in the background.
            || isApplyingSkillManagerMutation
            || isBuildingTaskCockpit
            || isLLMPromptBusy
    }

    private var isLLMPromptBusy: Bool {
        !previewingLLMPromptKeys.isEmpty
            || !sendingLLMPromptKeys.isEmpty
    }

    func toggleDisabledReason(for skill: SkillRecord) -> String? {
        if let catalogReason = DisplayText.catalogToggleDisabledReason(for: skill, isWriting: isWriting) {
            return catalogReason
        }
        guard !isWriting else {
            return UIStrings.toggleUnavailableBusy
        }
        guard let capability = adapterCapabilities.first(where: { $0.agent == skill.agent }) else {
            if DisplayText.isReadOnlyAdapter(skill.agent) {
                return UIStrings.toggleUnavailableReadOnlyAdapter(DisplayText.agent(skill.agent))
            }
            return DisplayText.requiresGuardedToggleCapability(skill.agent) ? DisplayText.guardedToggleBoundary(for: skill.agent) : nil
        }
        guard !capability.configToggle.supported else { return nil }
        return capability.configToggle.reason ?? UIStrings.readOnlyAdapterStatus(capability.displayName)
    }

    var selectedSkill: SkillRecord? {
        let visibleSkills = filteredSkills
        if let selectedSkillID {
            return visibleSkills.first { $0.id == selectedSkillID } ?? visibleSkills.first
        }
        return visibleSkills.first
    }

    var localSkillLibrarySkills: [SkillRecord] {
        skills
            .filter(DisplayText.isToolGlobal)
            .sorted {
                if $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedSame {
                    return $0.displayPath < $1.displayPath
                }
                return $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending
            }
    }

    var skillManagerVisibleSearchResults: [OccurrenceIdentifiedItem<SkillManagerSearchResult>] {
        guard let result = skillManagerSearchResult else { return [] }
        return result.displayResults(visibleCount: skillManagerSearchVisibility.visibleCount)
    }

    var skillManagerInstalled: SkillManagerInstalledListRecord? {
        skillManagerInstalledByScope[skillManagerScope]
    }

    var skillManagerVisibleInstalledRecords: [OccurrenceIdentifiedItem<SkillManagerInstalledRecord>] {
        skillManagerInstalled?.displayRecords ?? []
    }

    var skillManagerSearchStatus: ListCompletenessState? {
        guard let result = skillManagerSearchResult else { return nil }
        return result.listStatus(visibleCount: skillManagerVisibleSearchResults.count)
    }

    var skillManagerInstalledStatus: ListCompletenessState? {
        guard let result = skillManagerInstalled else { return nil }
        return result.listStatus(visibleCount: skillManagerVisibleInstalledRecords.count)
    }

    var skillManagerInventoryItems: [SkillManagerInventoryItem] {
        SkillManagerInventoryBuilder.build(
            installed: skillManagerInstalled?.installed ?? [],
            catalogSkills: skills,
            localLibrarySkills: localSkillLibrarySkills,
            scope: skillManagerScope
        )
    }

    var skillManagerSelectedAgents: [String] {
        let selected = skillManagerSelectedAgentIDs
        let ordered = SkillManagerAgent.defaultTargets.map(\.rawValue)
        return ordered.filter { selected.contains($0) }
    }

    var selectedSkillDetail: SkillDetailRecord? {
        guard let id = selectedSkill?.id else { return nil }
        return detailsByID[id]
    }

    func adoptingAgentSummary(for skill: SkillRecord) -> String {
        ensureAdoptingAgentSummaryCache()
        return adoptingAgentSummaryBySkillID[skill.id] ?? DisplayText.agent(skill.agent)
    }

    var enabledCount: Int {
        skills.filter { DisplayText.statusKind($0.state, enabled: $0.enabled) == .enabled }.count
    }

    var activeProjectContext: ProjectContext? {
        projectContextState?.active
    }

    var recentProjectContexts: [ProjectContext] {
        projectContextState?.recent ?? []
    }

    var adapterCapabilities: [AdapterCapabilityRecord] {
        status?.adapterCapabilities ?? []
    }

    var selectedAdapterCapability: AdapterCapabilityRecord? {
        adapterCapabilities.first { $0.agent == agentFilter.rawValue }
    }

    var selectedAgentRefreshSummary: AgentRefreshSummary? {
        lastScanActivity?.agentSummaries?.first { $0.agent == agentFilter.rawValue }
    }

    var selectedAgentHealthSummary: AgentSkillHealthSummary? {
        healthSummary.agentSummaries.first { $0.agent == agentFilter.rawValue }
    }

    var selectedAgentConfigTimelineAgent: String? {
        switch agentFilter {
        case .all:
            return nil
        default:
            return agentFilter.rawValue
        }
    }

    var agentConfigTimeline: AgentConfigTimelineModel {
        AgentConfigTimelineModel.make(snapshots: agentConfigSnapshots, agentFilter: agentFilter)
    }

    var selectedAgentLocalSessionRefreshKey: String {
        [
            agentFilter.rawValue,
            activeProjectContext?.rootPath ?? "",
            activeProjectContext?.currentCWD ?? ""
        ].joined(separator: "|")
    }

    var selectedAgentConfigRefreshKey: String {
        [
            agentFilter.rawValue,
            activeProjectContext?.rootPath ?? "",
            activeProjectContext?.currentCWD ?? ""
        ].joined(separator: "|")
    }

    var projectValidationMessage: String? {
        guard let message = activeProjectContext?.validationError, !message.isEmpty else {
            return nil
        }
        return message
    }

    var filteredSkillListResult: FilteredSkillListResult {
        let normalizedSearchText = searchText.trimmingCharacters(in: .whitespacesAndNewlines)
        let cacheKey = FilteredSkillListCacheKey(
            dataRevision: filteredSkillListDataRevision,
            searchText: normalizedSearchText,
            agentFilter: agentFilter.rawValue,
            stateFilter: stateFilter.rawValue,
            scopeFilter: skillScopeFilter.rawValue,
            sortOrder: sortOrder.rawValue,
            sortDirection: sortDirection.rawValue
        )

        if let filteredSkillListCache, filteredSkillListCache.key == cacheKey {
            return filteredSkillListCache.result
        }

        let issueIndex = SkillListModel.issueIndex(
            skills: skills,
            findings: findings,
            conflicts: conflicts
        )
        let visibleSkills = SkillListModel.filteredAndSorted(
            skills: skills,
            findings: findings,
            conflicts: conflicts,
            searchText: normalizedSearchText,
            agentFilter: agentFilter,
            stateFilter: stateFilter,
            scopeFilter: skillScopeFilter,
            sortOrder: sortOrder,
            sortDirection: sortDirection,
            issueIndex: issueIndex
        )
        let result = FilteredSkillListResult(
            skills: visibleSkills,
            issueCountsBySkillID: issueIndex.issueCountsBySkillID,
            conflictCountsBySkillID: issueIndex.conflictCountsBySkillID
        )
        filteredSkillListCache = FilteredSkillListCache(key: cacheKey, result: result)
        return result
    }

    var filteredSkills: [SkillRecord] {
        filteredSkillListResult.skills
    }

    func issueIndicatorCount(for skill: SkillRecord) -> Int {
        filteredSkillListResult.issueCount(for: skill.id)
    }

    func conflictIndicatorCount(for skill: SkillRecord) -> Int {
        filteredSkillListResult.conflictCount(for: skill.id)
    }

    var filteredSkillGroups: [SkillAgentGroup] {
        SkillListModel.groupedByAgent(filteredSkills)
    }

    var batchToggleSelectedSkills: [SkillRecord] {
        guard isBatchToggleSelectionExplicit else { return filteredSkills }
        return filteredSkills.filter { batchToggleSelectedSkillIDs.contains($0.id) }
    }

    var batchToggleAllVisibleSkillsSelected: Bool {
        !filteredSkills.isEmpty && batchToggleSelectedSkills.count == filteredSkills.count
    }

    func isBatchToggleSkillSelected(_ skill: SkillRecord) -> Bool {
        guard isBatchToggleSelectionExplicit else { return true }
        return batchToggleSelectedSkillIDs.contains(skill.id)
    }

    var canApplyBatchTogglePreview: Bool {
        guard let preview = batchTogglePreview else { return false }
        return !isRefreshBusy && preview.applySupported && preview.hasWritableChanges
    }

    var selectedFindings: [RuleFindingRecord] {
        guard let skill = selectedSkill else { return [] }
        return findings.filter { finding in
            finding.instanceId == skill.id
        }
    }

    var selectedDisplayFindings: [RuleFindingRecord] {
        guard let skill = selectedSkill else { return [] }
        return SkillListModel.displayFindings(skills: skills, findings: findings)
            .filter { finding in
                finding.instanceId == skill.id
            }
    }

    func ruleTuningRecord(ruleId: String, findingGroupID: String? = nil) -> RuleTuningRecord? {
        RuleTuningModel.record(in: ruleTuning, ruleId: ruleId, findingGroupId: findingGroupID)
    }

    var selectedConflicts: [ConflictGroupRecord] {
        guard let skill = selectedSkill else { return [] }
        let sameAgentSkillIDs = Set(skills.filter { $0.agent == skill.agent }.map(\.id))
        return conflicts.filter { conflict in
            conflict.instanceIds.contains(skill.id)
                && conflict.instanceIds.filter { sameAgentSkillIDs.contains($0) }.count > 1
        }
    }

    var sameAgentRuntimeConflictCount: Int {
        SkillListModel.sameAgentConflictGroupCount(skills: skills, conflicts: conflicts)
    }

    var visibleIssueCount: Int {
        SkillListModel.displayIssueCount(
            skills: skills,
            findings: findings,
            conflicts: conflicts,
            agentFilter: .all
        )
    }

    var selectedSkillEvents: [SkillEventRecord] {
        guard let id = selectedSkill?.id else { return [] }
        return (skillEventsByID[id] ?? []).filter(\.isToggleActivity)
    }

    var isLoadingSelectedSkillEvents: Bool {
        guard let id = selectedSkill?.id else { return false }
        return loadingSkillEventIDs.contains(id)
    }

    var selectedSkillEventCompleteness: ListCompletenessState {
        guard let id = selectedSkill?.id else {
            return ListPageAccumulator<SkillEventRecord>().state
        }
        return skillEventCompletenessByID[id] ?? ListPageAccumulator<SkillEventRecord>().state
    }
}
