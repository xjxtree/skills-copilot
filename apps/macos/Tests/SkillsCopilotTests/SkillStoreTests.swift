import Foundation
@testable import SkillsCopilot

@MainActor
struct SkillStoreTests {
    private let selectedGroup: Int?
    private let groupCount: Int

    init(selectedGroup: Int? = nil, groupCount: Int = 1) {
        self.selectedGroup = selectedGroup
        self.groupCount = max(groupCount, 1)
    }

    func run() async throws {
        try runCase("defaultNavigationStartsAtSessionsWithoutAgentProfile") {
            try defaultNavigationStartsAtSessionsWithoutAgentProfile()
        }
        try runCase("selectedAgentSessionRefreshKeyFollowsAgentOutsideSessionMode") {
            try selectedAgentSessionRefreshKeyFollowsAgentOutsideSessionMode()
        }
        try await runCase("localSessionSearchNormalizesSelectionAndDetail") {
            try await localSessionSearchNormalizesSelectionAndDetail()
        }
        try await runCase("localSessionRefreshIfNeededSkipsLoadedRequest") {
            try await localSessionRefreshIfNeededSkipsLoadedRequest()
        }
        try await runCase("localSessionScopeChangeFiltersCachedRowsWithoutRefresh") {
            try await localSessionScopeChangeFiltersCachedRowsWithoutRefresh()
        }
        try await runCase("localSessionProjectScopeUsesProjectRootFromAllScopeCache") {
            try await localSessionProjectScopeUsesProjectRootFromAllScopeCache()
        }
        try await runCase("localSessionSortChangesVisibleRowsWithoutRefresh") {
            try await localSessionSortChangesVisibleRowsWithoutRefresh()
        }
        try await runCase("reloadKeepsSelectedSkillWhenItStillExists") {
            try await reloadKeepsSelectedSkillWhenItStillExists()
        }
        try await runCase("reloadFallsBackToFirstSkillWhenSelectionIsMissing") {
            try await reloadFallsBackToFirstSkillWhenSelectionIsMissing()
        }
        try await runCase("emptyCatalogKeepsFriendlyEmptyModel") {
            try await emptyCatalogKeepsFriendlyEmptyModel()
        }
        try await runCase("serviceErrorClearsLoadingAndKeepsReadableError") {
            try await serviceErrorClearsLoadingAndKeepsReadableError()
        }
        try await runCase("reloadUsesStateSnapshotForCollectionRefresh") {
            try await reloadUsesStateSnapshotForCollectionRefresh()
        }
        try await runCase("startupLoadPrewarmsLaunchDataWithoutScanningOrWriting") {
            try await startupLoadPrewarmsLaunchDataWithoutScanningOrWriting()
        }
        try await runCase("stateSnapshotRefreshesDoNotReuseStaleFindingsOrPermissions") {
            try await stateSnapshotRefreshesDoNotReuseStaleFindingsOrPermissions()
        }
        try await runCase("selectedDetailDataIsScopedToCurrentAgentAndSkill") {
            try await selectedDetailDataIsScopedToCurrentAgentAndSkill()
        }
        try await runCase("scanAllUsesGenericCatalogMethod") {
            try await scanAllUsesGenericCatalogMethod()
        }
        try await runCase("searchAndFilterChangesNormalizeSelectionAndDetail") {
            try await searchAndFilterChangesNormalizeSelectionAndDetail()
        }
        try await runCase("agentConfigTimelineFollowsSelectedAgentFilterOnly") {
            try await agentConfigTimelineFollowsSelectedAgentFilterOnly()
        }
        try await runCase("readOnlyAgentConfigLoadsCurrentDocuments") {
            try await readOnlyAgentConfigLoadsCurrentDocuments()
        }
        try await runCase("agentConfigRefreshIfNeededSkipsLoadedRequestAndScopeFiltersLocally") {
            try await agentConfigRefreshIfNeededSkipsLoadedRequestAndScopeFiltersLocally()
        }
        try await runCase("runtimeAnalysisRefreshIfNeededSkipsLoadedRequestsAndForceRefreshes") {
            try await runtimeAnalysisRefreshIfNeededSkipsLoadedRequestsAndForceRefreshes()
        }
        try await runCase("settingsFeedbackClearsAfterFailedConfigSave") {
            try await settingsFeedbackClearsAfterFailedConfigSave()
        }
        try await runCase("previewRollbackShowsDiffWithoutCallingRollback") {
            try await previewRollbackShowsDiffWithoutCallingRollback()
        }
        try await runCase("rollbackSnapshotRequiresVisibleAgentTimelineRecord") {
            try await rollbackSnapshotRequiresVisibleAgentTimelineRecord()
        }
        try await runCase("refreshOperationsIgnoreReentryWhileBusy") {
            try await refreshOperationsIgnoreReentryWhileBusy()
        }
        try await runCase("agentFilterLimitsVisibleSkillsAndSelection") {
            try await agentFilterLimitsVisibleSkillsAndSelection()
        }
        try await runCase("allAgentFilterDoesNotFetchMixedConfigHistory") {
            try await allAgentFilterDoesNotFetchMixedConfigHistory()
        }
        try await runCase("toggleSelectedSkillExposesWritingStateAndRefreshesSelection") {
            try await toggleSelectedSkillExposesWritingStateAndRefreshesSelection()
        }
        try await runCase("writeOperationsIgnoreReentryWhileBusy") {
            try await writeOperationsIgnoreReentryWhileBusy()
        }
        try await runCase("codexToggleAddsRestartRequiredNotice") {
            try await codexToggleAddsRestartRequiredNotice()
        }
        try await runCase("opencodeToggleCallsServiceAndRefreshesSelection") {
            try await opencodeToggleCallsServiceAndRefreshesSelection()
        }
        try await runCase("toolGlobalToggleIsPreviewOnlyAndDoesNotCallService") {
            try await toolGlobalToggleIsPreviewOnlyAndDoesNotCallService()
        }
        try await runCase("batchTogglePreviewFiltersReadOnlyAndNoopSkills") {
            try await batchTogglePreviewFiltersReadOnlyAndNoopSkills()
        }
        try await runCase("batchTogglePreviewHonorsExplicitSelection") {
            try await batchTogglePreviewHonorsExplicitSelection()
        }
        try await runCase("batchToggleApplyUsesBatchServiceAndRefreshes") {
            try await batchToggleApplyUsesBatchServiceAndRefreshes()
        }
        try await runCase("batchToggleApplyRequiresCurrentPreviewConfirmation") {
            try await batchToggleApplyRequiresCurrentPreviewConfirmation()
        }
        try await runCase("localReportExportUsesUserTriggeredServiceContract") {
            try await localReportExportUsesUserTriggeredServiceContract()
        }
        try await runCase("localReportExportCanUseAgentWorkspaceScopeWithoutSelectedSkill") {
            try await localReportExportCanUseAgentWorkspaceScopeWithoutSelectedSkill()
        }
        try await runCase("localReportExportClearsStaleResultWhenScopeChanges") {
            try await localReportExportClearsStaleResultWhenScopeChanges()
        }
        try await runCase("localReportExportUnavailableDoesNotPretendFileWasWritten") {
            try await localReportExportUnavailableDoesNotPretendFileWasWritten()
        }
        try await runCase("reloadLoadsProjectContext") {
            try await reloadLoadsProjectContext()
        }
        try await runCase("setProjectStoresContextAndScans") {
            try await setProjectStoresContextAndScans()
        }
        try await runCase("clearProjectClearsContextAndScans") {
            try await clearProjectClearsContextAndScans()
        }
        try await runCase("projectValidationErrorSkipsScanAndSurfacesMessage") {
            try await projectValidationErrorSkipsScanAndSurfacesMessage()
        }
        try await runCase("reloadFallsBackToDisabledLLMWhenOldServiceDoesNotSupportMethods") {
            try await reloadFallsBackToDisabledLLMWhenOldServiceDoesNotSupportMethods()
        }
        try await runCase("prepareLLMActionStoresEstimateWithoutProviderCall") {
            try await prepareLLMActionStoresEstimateWithoutProviderCall()
        }
        try await runCase("prepareSkillAnalysisUsesReadOnlyPrepareContract") {
            try await prepareSkillAnalysisUsesReadOnlyPrepareContract()
        }
        try await runCase("prepareSkillAnalysisFallsBackWhenMethodUnavailable") {
            try await prepareSkillAnalysisFallsBackWhenMethodUnavailable()
        }
        try await runCase("scoreSkillQualityUsesReadOnlyAnalysisContract") {
            try await scoreSkillQualityUsesReadOnlyAnalysisContract()
        }
        try await runCase("taskReadinessUsesReadOnlyCheckContract") {
            try await taskReadinessUsesReadOnlyCheckContract()
        }
        try await runCase("taskReadinessPromptSendRequiresConfiguredProvider") {
            try await taskReadinessPromptSendRequiresConfiguredProvider()
        }
        try await runCase("routingConfidenceUsesReadOnlyRankContract") {
            try await routingConfidenceUsesReadOnlyRankContract()
        }
        try await runCase("routingConfidencePromptSendRequiresConfiguredProvider") {
            try await routingConfidencePromptSendRequiresConfiguredProvider()
        }
        try await runCase("taskBenchmarkUsesLocalServiceContract") {
            try await taskBenchmarkUsesLocalServiceContract()
        }
        try await runCase("taskBenchmarkFallsBackWhenMethodUnavailable") {
            try await taskBenchmarkFallsBackWhenMethodUnavailable()
        }
        try await runCase("routingRegressionUsesLocalBenchmarkServiceContract") {
            try await routingRegressionUsesLocalBenchmarkServiceContract()
        }
        try await runCase("routingRegressionFallsBackWhenMethodUnavailable") {
            try await routingRegressionFallsBackWhenMethodUnavailable()
        }
        try await runCase("agentTraceImportUsesLocalServiceContract") {
            try await agentTraceImportUsesLocalServiceContract()
        }
        try await runCase("agentTraceImportFallsBackWhenMethodUnavailable") {
            try await agentTraceImportFallsBackWhenMethodUnavailable()
        }
        try await runCase("routingAccuracyDashboardUsesReadOnlyServiceContract") {
            try await routingAccuracyDashboardUsesReadOnlyServiceContract()
        }
        try await runCase("staleDriftDetectionUsesReadOnlyServiceContract") {
            try await staleDriftDetectionUsesReadOnlyServiceContract()
        }
        try await runCase("knowledgeSearchUsesReadOnlyServiceContract") {
            try await knowledgeSearchUsesReadOnlyServiceContract()
        }
        try await runCase("localSkillMapUsesReadOnlyServiceContract") {
            try await localSkillMapUsesReadOnlyServiceContract()
        }
        try await runCase("localSkillMapFallsBackWhenMethodUnavailable") {
            try await localSkillMapFallsBackWhenMethodUnavailable()
        }
        try await runCase("skillLifecycleTimelineUsesReadOnlyServiceContract") {
            try await skillLifecycleTimelineUsesReadOnlyServiceContract()
        }
        try await runCase("skillLifecycleTimelineFallsBackWhenMethodUnavailable") {
            try await skillLifecycleTimelineFallsBackWhenMethodUnavailable()
        }
        try await runCase("providerObservabilityUsesReadOnlyServiceContract") {
            try await providerObservabilityUsesReadOnlyServiceContract()
        }
        try await runCase("providerObservabilityNeedBasedLoadUsesCacheUntilManualRefresh") {
            try await providerObservabilityNeedBasedLoadUsesCacheUntilManualRefresh()
        }
        try await runCase("providerObservabilityFallsBackWhenMethodUnavailable") {
            try await providerObservabilityFallsBackWhenMethodUnavailable()
        }
        try await runCase("taskCockpitUsesReadOnlyServiceContract") {
            try await taskCockpitUsesReadOnlyServiceContract()
        }
        try await runCase("taskCockpitUsesGlobalScopeOutsideSkillDetail") {
            try await taskCockpitUsesGlobalScopeOutsideSkillDetail()
        }
        try await runCase("taskCockpitHistoryPersistsLocally") {
            try await taskCockpitHistoryPersistsLocally()
        }
        try await runCase("taskCockpitPreservesExactUserInputInServiceContract") {
            try await taskCockpitPreservesExactUserInputInServiceContract()
        }
        try await runCase("taskCockpitWhitespaceOnlyInputUsesFallbackTask") {
            try await taskCockpitWhitespaceOnlyInputUsesFallbackTask()
        }
        try await runCase("taskCockpitFallsBackWhenMethodUnavailable") {
            try await taskCockpitFallsBackWhenMethodUnavailable()
        }
        try await runCase("taskCockpitTimeoutShowsRecoveryAndIgnoresStaleResponse") {
            try await taskCockpitTimeoutShowsRecoveryAndIgnoresStaleResponse()
        }
        try await runCase("taskCockpitCancelShowsRecoveryAndAllowsRetry") {
            try await taskCockpitCancelShowsRecoveryAndAllowsRetry()
        }
        try await runCase("similarSkillGroupingUsesReadOnlyServiceContract") {
            try await similarSkillGroupingUsesReadOnlyServiceContract()
        }
        try await runCase("similarSkillGroupingFallsBackWhenMethodUnavailable") {
            try await similarSkillGroupingFallsBackWhenMethodUnavailable()
        }
        try await runCase("capabilityTaxonomyUsesReadOnlyServiceContract") {
            try await capabilityTaxonomyUsesReadOnlyServiceContract()
        }
        try await runCase("capabilityTaxonomyFallsBackWhenMethodUnavailable") {
            try await capabilityTaxonomyFallsBackWhenMethodUnavailable()
        }
        try await runCase("workspaceReadinessUsesReadOnlyServiceContract") {
            try await workspaceReadinessUsesReadOnlyServiceContract()
        }
        try await runCase("workspaceReadinessFallsBackWhenMethodUnavailable") {
            try await workspaceReadinessFallsBackWhenMethodUnavailable()
        }
        try await runCase("remediationPlanUsesReadOnlyServiceContract") {
            try await remediationPlanUsesReadOnlyServiceContract()
        }
        try await runCase("remediationPlanFallsBackWhenMethodUnavailable") {
            try await remediationPlanFallsBackWhenMethodUnavailable()
        }
        try await runCase("remediationPreviewDraftsUsesCopyOnlyServiceContract") {
            try await remediationPreviewDraftsUsesCopyOnlyServiceContract()
        }
        try await runCase("remediationPreviewDraftsFallsBackWhenMethodUnavailable") {
            try await remediationPreviewDraftsFallsBackWhenMethodUnavailable()
        }
        try await runCase("remediationImpactPreviewUsesReadOnlyServiceContract") {
            try await remediationImpactPreviewUsesReadOnlyServiceContract()
        }
        try await runCase("remediationImpactPreviewFallsBackWhenMethodUnavailable") {
            try await remediationImpactPreviewFallsBackWhenMethodUnavailable()
        }
        try await runCase("remediationBatchReviewUsesReadOnlyServiceContract") {
            try await remediationBatchReviewUsesReadOnlyServiceContract()
        }
        try await runCase("remediationBatchReviewFallsBackWhenMethodUnavailable") {
            try await remediationBatchReviewFallsBackWhenMethodUnavailable()
        }
        try await runCase("remediationHistoryUsesLocalServiceContract") {
            try await remediationHistoryUsesLocalServiceContract()
        }
        try await runCase("remediationHistoryFallsBackWhenMethodUnavailable") {
            try await remediationHistoryFallsBackWhenMethodUnavailable()
        }
        try await runCase("guidedCleanupFlowUsesLocalServiceContract") {
            try await guidedCleanupFlowUsesLocalServiceContract()
        }
        try await runCase("guidedCleanupFlowFallsBackWhenMethodUnavailable") {
            try await guidedCleanupFlowFallsBackWhenMethodUnavailable()
        }
        try await runCase("crossAgentReadinessUsesReadOnlyServiceContract") {
            try await crossAgentReadinessUsesReadOnlyServiceContract()
        }
        try await runCase("routingConfidenceClearsStaleSelection") {
            try await routingConfidenceClearsStaleSelection()
        }
        try await runCase("llmPreparePreviewIsScopedToSelectedSkillAndReadOnly") {
            try await llmPreparePreviewIsScopedToSelectedSkillAndReadOnly()
        }
        try await runCase("promptPreviewRequiresConfiguredProviderAndExplicitSend") {
            try await promptPreviewRequiresConfiguredProviderAndExplicitSend()
        }
        try await runCase("skillManagerValidationFeedbackStaysInManagerSurface") {
            try await skillManagerValidationFeedbackStaysInManagerSurface()
        }
        try runCase("skillManagerUsesIndependentInstallRemoveAndLocalInputs") {
            try skillManagerUsesIndependentInstallRemoveAndLocalInputs()
        }
        try await runCase("previewScriptExecutionSafetyStoresBlockedPreviewWithoutExecute") {
            try await previewScriptExecutionSafetyStoresBlockedPreviewWithoutExecute()
        }
    }

    private func runCase(_ name: String, _ body: () throws -> Void) throws {
        guard shouldRunCase(name) else { return }
        fputs("SkillsCopilotTests: SkillStoreTests.\(name) start\n", stderr)
        fflush(stderr)
        try body()
        fputs("SkillsCopilotTests: SkillStoreTests.\(name) ok\n", stderr)
        fflush(stderr)
    }

    private func runCase(_ name: String, _ body: () async throws -> Void) async throws {
        guard shouldRunCase(name) else { return }
        fputs("SkillsCopilotTests: SkillStoreTests.\(name) start\n", stderr)
        fflush(stderr)
        try await body()
        fputs("SkillsCopilotTests: SkillStoreTests.\(name) ok\n", stderr)
        fflush(stderr)
    }

    private func shouldRunCase(_ name: String) -> Bool {
        guard let selectedGroup else { return true }
        return groupIndex(for: name) == selectedGroup
    }

    private func groupIndex(for name: String) -> Int {
        var hash: UInt64 = 1_469_598_103_934_665_603
        for byte in name.utf8 {
            hash ^= UInt64(byte)
            hash = hash &* 1_099_511_628_211
        }
        return Int(hash % UInt64(groupCount))
    }

    private func defaultNavigationStartsAtSessionsWithoutAgentProfile() throws {
        let store = SkillStore(service: ServiceClient())
        try expectNil(store.selectedSidebarSelection, "Agent Copilot should not expose a default Agent Profile detail selection.")
        try expectEqual(store.sidebarContentMode, .skills, "Agent Copilot should start from the Skills primary navigation.")
        try expectEqual(store.selectedDetailSection, .overview, "Default detail section should stay neutral until a session, skill, report, or Preflight is selected.")
    }

    private func selectedAgentSessionRefreshKeyFollowsAgentOutsideSessionMode() throws {
        let store = SkillStore(service: ServiceClient())
        store.sidebarContentMode = .skills
        let claudeKey = store.selectedAgentLocalSessionRefreshKey

        store.agentFilter = .codex

        try expectEqual(store.sidebarContentMode, .skills, "Switching agent should not require the Sessions sidebar to be active.")
        try expectContains(store.selectedAgentLocalSessionRefreshKey, SkillAgentFilter.codex.rawValue, "Selected-agent session refresh key should include the selected agent.")
        try expectFalse(store.selectedAgentLocalSessionRefreshKey == claudeKey, "Switching agent should trigger a new selected-agent session refresh key.")
    }

    private func skillManagerValidationFeedbackStaysInManagerSurface() async throws {
        let store = SkillStore(service: ServiceClient())

        await store.searchSkillManager()

        try expectNil(store.errorMessage, "Skill Manager validation errors should not leak into the global detail banner.")
        try expectEqual(
            store.skillManagerErrorMessage,
            UIStrings.text("skillManager.search.required", "Enter a skill search query."),
            "Skill Manager should keep validation feedback local to the package manager sheet."
        )
    }

    private func skillManagerUsesIndependentInstallRemoveAndLocalInputs() throws {
        let store = SkillStore(service: ServiceClient())

        store.skillManagerSource = "vercel-labs/agent-skills"
        store.skillManagerInstallSkillName = "frontend-design"
        store.skillManagerRemoveSkillName = "legacy-design"
        store.skillManagerLocalSkillName = "local-note"

        try expectEqual(store.skillManagerSkillName, "", "Legacy shared Skill Manager skill name should not retain workflow-specific input.")
        try expectEqual(store.skillManagerInstallSkillName, "frontend-design", "Install skill input should be independent.")
        try expectEqual(store.skillManagerRemoveSkillName, "legacy-design", "Remove skill input should be independent.")
        try expectEqual(store.skillManagerLocalSkillName, "local-note", "Local create input should stay independent.")
    }

    private func localSessionSearchNormalizesSelectionAndDetail() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "sessions")

        let store = SkillStore(service: fake.serviceClient())
        store.sidebarContentMode = .sessions
        await store.previewLocalSessions()

        try expectEqual(store.filteredLocalSessionRows.map(\.id), ["session-alpha", "session-develop"], "Session preview should load the fake rows.")
        try expectEqual(store.selectedLocalSessionID, "session-alpha", "Initial session selection should use the first visible row.")
        try expectEqual(store.selectedSidebarSelection, .session("session-alpha"), "Initial detail selection should point at the first session.")

        store.localSessionSearchText = "develop"

        try expectEqual(store.filteredLocalSessionRows.map(\.id), ["session-develop"], "Session search should narrow visible rows.")
        try expectEqual(store.selectedLocalSessionID, "session-develop", "Search should move selection to the visible session.")
        try expectEqual(store.selectedSidebarSelection, .session("session-develop"), "Detail selection should follow the searched session.")
        try expectEqual(store.selectedLocalSession?.title, "Switch to develop branch", "Detail model should expose the searched session.")

        store.localSessionSearchText = "missing"

        try expectEqual(store.filteredLocalSessionRows.count, 0, "No-match search should show an empty session list.")
        try expectNil(store.selectedLocalSessionID, "No-match search should clear stale session selection.")
        try expectNil(store.selectedSidebarSelection, "No-match search should clear stale session detail.")

        store.localSessionSearchText = ""

        try expectEqual(store.selectedLocalSessionID, "session-alpha", "Clearing search should restore the first visible session.")
        try expectEqual(store.selectedSidebarSelection, .session("session-alpha"), "Clearing search should restore session detail.")
    }

    private func localSessionRefreshIfNeededSkipsLoadedRequest() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "sessions")

        let store = SkillStore(service: fake.serviceClient())
        await store.refreshSelectedAgentLocalSessionsIfNeeded()

        try expectEqual(
            countMethodCalls("session.previewLocalSessions", in: fake.calls()),
            1,
            "Initial need-based session refresh should load local sessions."
        )

        await store.refreshSelectedAgentLocalSessionsIfNeeded()

        try expectEqual(
            countMethodCalls("session.previewLocalSessions", in: fake.calls()),
            1,
            "Need-based session refresh should not rescan the same agent/project payload."
        )

        await store.previewLocalSessions()

        try expectEqual(
            countMethodCalls("session.previewLocalSessions", in: fake.calls()),
            2,
            "Manual session refresh should still force a new preview request."
        )
    }

    private func localSessionScopeChangeFiltersCachedRowsWithoutRefresh() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "sessions-mixed")

        let store = SkillStore(service: fake.serviceClient())
        store.sidebarContentMode = .sessions
        await store.previewLocalSessions()
        guard let globalSession = store.localSessionPreviewResult.sessionRows.first(where: { $0.id == "session-global" }) else {
            throw NativeModelTestFailure(description: "Fake session fixture should include the global session.")
        }

        try expectEqual(store.localSessionPreviewResult.sessionRows.map(\.id), ["session-alpha", "session-develop", "session-global"], "Session preview should keep the all-scope startup cache.")
        try expectEqual(store.filteredLocalSessionRows.map(\.id), ["session-alpha", "session-develop"], "Default project scope should locally filter the all-scope cache.")
        try expectEqual(store.scopedLocalSessionUserMessageCount, 2, "Project scope metrics should be derived from project rows only.")
        try expectEqual(store.scopedLocalSessionTotalMessageCount, 4, "Project scope message totals should be derived from project rows only.")
        let callsBeforeScopeChange = countMethodCalls("session.previewLocalSessions", in: fake.calls())
        try expectContains(fake.calls(), "\"scope\":\"all\"", "Session preview should request the all-scope cache for local filtering.")

        store.localSessionScopeFilter = .all

        try expectEqual(store.filteredLocalSessionRows.map(\.id), ["session-alpha", "session-develop", "session-global"], "All scope should reveal the cached global session without rescanning.")
        try expectEqual(store.scopedLocalSessionUserMessageCount, 4, "All-scope metrics should include global rows.")
        try expectEqual(store.scopedLocalSessionSkillCallCount, 1, "All-scope skill metrics should include global rows.")
        try expectEqual(
            countMethodCalls("session.previewLocalSessions", in: fake.calls()),
            callsBeforeScopeChange,
            "Scope changes should be local filters and must not trigger a background session preview."
        )

        store.selectLocalSession(globalSession)
        store.localSessionScopeFilter = .project

        try expectEqual(store.filteredLocalSessionRows.map(\.id), ["session-alpha", "session-develop"], "Returning to project scope should hide global rows from the cached list.")
        try expectEqual(store.selectedLocalSessionID, "session-alpha", "Scope filtering should normalize a hidden global selection to the first visible project session.")
        try expectEqual(store.selectedSidebarSelection, .session("session-alpha"), "Detail selection should follow the locally visible session after scope filtering.")
        try expectEqual(
            countMethodCalls("session.previewLocalSessions", in: fake.calls()),
            callsBeforeScopeChange,
            "Returning to project scope should still avoid a service rescan."
        )
    }

    private func localSessionProjectScopeUsesProjectRootFromAllScopeCache() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "sessions-all-scope-project-root")

        let store = SkillStore(service: fake.serviceClient())
        store.setProjectContextForTesting(Self.fixtureProjectContextState)
        store.sidebarContentMode = .sessions

        await store.previewLocalSessions()

        try expectEqual(store.activeProjectContext?.rootPath, "/tmp/project", "Fixture should expose an active project context.")
        try expectContains(fake.calls(), "\"scope\":\"all\"", "Session preview should keep requesting the all-scope cache.")
        try expectEqual(
            store.localSessionPreviewResult.sessionRows.map(\.id),
            ["session-project-from-all", "session-global"],
            "All-scope cache should retain both project and global sessions."
        )
        try expectEqual(
            store.filteredLocalSessionRows.map(\.id),
            ["session-project-from-all"],
            "Project scope should include cached all-scope rows when project root metadata matches."
        )
        try expectEqual(store.scopedLocalSessionToolCallCount, 24, "Project scope metrics should include the project-root session.")

        store.localSessionScopeFilter = .all

        try expectEqual(
            store.filteredLocalSessionRows.map(\.id),
            ["session-project-from-all", "session-global"],
            "All scope should still reveal global rows from the same cache."
        )
    }

    private static let fixtureProjectContext = ProjectContext(
        id: "project-1",
        name: "Fixture Project",
        rootPath: "/tmp/project",
        currentCWD: "/tmp/project",
        lastUsedAt: "2026-06-08T00:00:00Z",
        isActive: true,
        validationError: nil
    )

    private static let fixtureProjectContextState = ProjectContextState(
        active: fixtureProjectContext,
        recent: [fixtureProjectContext]
    )

    private func localSessionSortChangesVisibleRowsWithoutRefresh() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "sessions-mixed")

        let store = SkillStore(service: fake.serviceClient())
        store.sidebarContentMode = .sessions
        await store.previewLocalSessions()
        let callsBeforeSortChange = countMethodCalls("session.previewLocalSessions", in: fake.calls())

        store.localSessionSortOrder = .title
        store.localSessionSortDirection = .descending

        try expectEqual(
            store.filteredLocalSessionRows.map(\.id),
            ["session-develop", "session-alpha"],
            "Session sort controls should reorder the already loaded project rows locally."
        )
        try expectEqual(
            countMethodCalls("session.previewLocalSessions", in: fake.calls()),
            callsBeforeSortChange,
            "Changing local session sort should not trigger a service rescan."
        )
    }

    private func reloadKeepsSelectedSkillWhenItStillExists() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "batch-mixed")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()

        try expectEqual(store.selectedSkillID, "beta", "Reload should keep an existing selected skill ID.")
        try expectEqual(store.selectedSkill?.id, "beta", "Reload should keep the selected skill model stable.")
        try expectEqual(store.selectedSkillDetail?.id, "beta", "Reload should load detail for the stable selection.")
        try expectFalse(store.isLoading, "Reload should reset loading state.")
        try expectNil(store.errorMessage, "Reload should not set an error on success.")
    }

    private func reloadFallsBackToFirstSkillWhenSelectionIsMissing() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "batch-mixed")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "missing"
        await store.reload()

        try expectEqual(store.selectedSkillID, "alpha", "Reload should select the first skill when the previous selection disappears.")
        try expectEqual(store.selectedSkill?.id, "alpha", "Fallback selection should expose the first skill model.")
        try expectEqual(store.selectedSkillDetail?.id, "alpha", "Fallback selection should load matching detail.")
    }

    private func emptyCatalogKeepsFriendlyEmptyModel() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "empty")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "missing"
        await store.reload()

        try expectEqual(store.skills.count, 0, "Empty catalog should expose no skills.")
        try expectEqual(store.filteredSkills.count, 0, "Empty catalog should expose no filtered skills.")
        try expectEqual(store.enabledCount, 0, "Empty catalog should expose zero enabled skills.")
        try expectNil(store.selectedSkillID, "Empty catalog should clear a stale selection.")
        try expectNil(store.selectedSkill, "Empty catalog should not synthesize a selected skill.")
        try expectNil(store.selectedSkillDetail, "Empty catalog should not synthesize detail.")
        try expectNil(store.errorMessage, "Empty catalog should not be treated as an error.")
        try expectFalse(store.isLoading, "Empty catalog reload should reset loading state.")
    }

    private func serviceErrorClearsLoadingAndKeepsReadableError() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "error")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()

        try expectFalse(store.isLoading, "Failed reload should reset loading state.")
        try expectContains(store.errorMessage, "test.error: boom", "Failed reload should surface the service error.")
        try expectEqual(store.skills.count, 0, "Failed reload should not invent skills.")
        try expectNil(store.selectedSkillID, "Failed reload should not invent selection.")
    }

    private func reloadUsesStateSnapshotForCollectionRefresh() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()

        let calls = fake.calls()
        try expectEqual(countOccurrences("app.stateSnapshot", in: calls), 1, "Reload should refresh status and collections with one app state snapshot call.")
        try expectEqual(countOccurrences("service.status", in: calls), 0, "Reload collection refresh should not launch a separate status sidecar.")
        try expectEqual(countOccurrences("catalog.listSkills", in: calls), 0, "Reload collection refresh should not launch a separate skills list sidecar.")
        try expectEqual(countOccurrences("catalog.listFindings", in: calls), 0, "Reload collection refresh should not launch a separate findings list sidecar.")
        try expectEqual(countOccurrences("catalog.listConflicts", in: calls), 0, "Reload collection refresh should not launch a separate conflicts list sidecar.")
        try expectEqual(countMethodCalls("snapshot.list", in: calls), 0, "Reload collection refresh should not launch a global snapshots list sidecar.")
        try expectEqual(countMethodCalls("snapshot.listAgentConfig", in: calls), 1, "Reload should refresh the selected agent config history.")
        try expectContains(calls, "llm.status", "Reload should preserve the separate LLM status behavior.")
        try expectContains(calls, "project.getContext", "Reload should preserve the separate project context behavior.")
    }

    private func startupLoadPrewarmsLaunchDataWithoutScanningOrWriting() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "sessions")

        let store = SkillStore(service: fake.serviceClient())
        try expectFalse(store.hasCompletedStartupLoad, "Startup should begin behind the progress overlay.")
        try expectFalse(store.startupLoadingState == nil, "Startup should expose an initial progress state.")

        await store.loadAppStartupDataIfNeeded()

        let calls = fake.calls()
        try expectFalse(!store.hasCompletedStartupLoad, "Startup should mark the initial prewarm as complete.")
        try expectNil(store.startupLoadingState, "Startup should clear the progress overlay when prewarm completes.")
        try expectFalse(store.isLoading, "Startup should reset the global loading state.")
        try expectNil(store.errorMessage, "Startup should not surface a background error on success.")
        try expectEqual(store.localSessionPreviewResult.sessionRows.count, 2, "Startup should prewarm the selected agent session summary.")
        try expectEqual(store.selectedSkillDetail?.id, "alpha", "Startup should prewarm the first selected skill detail.")
        try expectEqual(countOccurrences("app.stateSnapshot", in: calls), 1, "Startup should use the combined state snapshot once.")
        try expectEqual(countMethodCalls("snapshot.listAgentConfig", in: calls), 1, "Startup should prewarm selected-agent config history.")
        try expectContains(calls, "session.previewLocalSessions", "Startup should prewarm selected-agent local sessions.")
        try expectContains(calls, "config.readAgentConfig", "Startup should prewarm selected-agent current config documents.")
        try expectContains(calls, "catalog.getSkill", "Startup should prewarm the selected skill detail.")
        try expectEqual(countMethodCalls("llm.listProviderProfiles", in: calls), 1, "Startup should prewarm the AI provider status once.")
        try expectFalse(calls.contains("\"method\":\"catalog.scanAll\""), "Startup should not scan roots automatically.")
        try expectFalse(calls.contains("\"method\":\"config.toggleSkill\""), "Startup should not write agent config.")

        await store.loadAIProviderStatusIfNeeded()

        try expectEqual(countMethodCalls("llm.listProviderProfiles", in: fake.calls()), 1, "Need-based provider status loading should reuse startup cache.")

        await store.loadAIProviderStatus()

        try expectEqual(countMethodCalls("llm.listProviderProfiles", in: fake.calls()), 2, "Manual provider status refresh should force a fresh service request.")

        await store.loadAppStartupDataIfNeeded()

        try expectEqual(countOccurrences("app.stateSnapshot", in: fake.calls()), 1, "Startup prewarm should be idempotent after completion.")
    }

    private func stateSnapshotRefreshesDoNotReuseStaleFindingsOrPermissions() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "stale-before")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()

        try expectEqual(store.selectedFindings.map(\.id), ["finding-stale-before"], "Initial reload should expose the stale-before finding fixture.")
        try expectEqual(permissionMarker(store.selectedSkillDetail), "before", "Initial reload should load the before permissions fixture.")

        store.searchText = "alpha"
        try await waitUntil("Search filter should move selection away from beta.") {
            store.selectedSkillID == "alpha"
        }
        try expectEqual(store.selectedFindings.map(\.id), [], "Filter changes should not keep findings from the previous selected skill.")

        store.searchText = ""
        store.selectedSkillID = "beta"
        fake.setScenario("stale-after-scan")
        await store.scanAll()

        try expectEqual(store.findings.map(\.id), ["finding-fresh-scan", "finding-fresh-codex"], "Scan refresh should replace stale findings from the prior snapshot.")
        try expectEqual(store.selectedFindings.map(\.id), ["finding-fresh-scan"], "Scan refresh should expose findings for the current selection only.")
        try expectEqual(permissionMarker(store.selectedSkillDetail), "scan", "Scan refresh should reload selected detail permissions.")

        store.agentFilter = .codex
        try await waitUntil("Agent filter should move selection to the Codex fixture.") {
            store.selectedSkillID == "gamma" && store.selectedSkillDetail?.id == "gamma"
        }
        try expectEqual(store.selectedFindings.map(\.id), ["finding-fresh-codex"], "Agent filter should not show findings from a previously selected adapter.")
        try expectEqual(permissionMarker(store.selectedSkillDetail), "codex-scan", "Agent filter should load detail permissions for the newly selected adapter.")

        fake.setScenario("stale-after-project")
        await store.setProject(rootPath: "/tmp/project", currentCWD: "/tmp/project", name: "Fixture Project")

        try expectEqual(store.findings.map(\.id), ["finding-project"], "Project context scan should replace findings from the previous adapter state.")
        try expectEqual(store.selectedFindings.map(\.id), ["finding-project"], "Project context scan should expose the fresh selected finding.")
        try expectEqual(permissionMarker(store.selectedSkillDetail), "project", "Project context scan should reload selected detail permissions.")

        store.agentFilter = .all
        store.selectedSkillID = "beta"
        fake.setScenario("stale-after-toggle")
        await store.toggleSelectedSkill(on: false)

        try expectEqual(store.findings.map(\.id), ["finding-toggle"], "Adapter state changes should replace stale findings.")
        try expectEqual(store.selectedFindings.map(\.id), ["finding-toggle"], "Adapter state changes should keep selected findings fresh.")
        try expectEqual(permissionMarker(store.selectedSkillDetail), "toggle", "Adapter state changes should reload selected detail permissions.")
    }

    private func selectedDetailDataIsScopedToCurrentAgentAndSkill() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "detail-scope")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()

        try expectEqual(store.selectedSkill?.id, "beta", "Fixture should select the Claude beta skill.")
        try expectEqual(store.selectedFindings.map(\.id), ["finding-beta-instance"], "Selected findings must use the selected instance, not a shared definition or another agent.")
        try expectEqual(store.selectedConflicts.map(\.id), ["conflict-beta-alpha"], "Selected conflicts should include only same-agent runtime conflicts for the selected skill.")
        try expectEqual(store.selectedSkillEvents.map(\.id), [1001], "Selected history should show only toggle activity for the current skill.")

        store.agentFilter = .codex
        try await waitUntil("Agent filter should move detail selection to the Codex skill and load its events.") {
            store.selectedSkillID == "gamma" && store.selectedSkillEvents.map(\.id) == [2001]
        }

        try expectEqual(store.selectedSkill?.agent, "codex", "Selection should now be scoped to the Codex agent.")
        try expectEqual(store.selectedFindings.map(\.id), ["finding-gamma-instance"], "Changing agents must not keep Claude findings on the detail page.")
        try expectEqual(store.selectedConflicts.map(\.id), [], "Cross-agent duplicate/source overlap must not appear as a detail conflict.")
        try expectContains(fake.calls(), "\"instance_id\":\"beta\"", "Skill event fetch should request the selected beta instance.")
        try expectContains(fake.calls(), "\"instance_id\":\"gamma\"", "Skill event fetch should request the selected gamma instance after agent change.")
    }

    private func scanAllUsesGenericCatalogMethod() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        await store.scanAll()

        try expectFalse(store.isScanning, "Scan should reset scanning state.")
        try expectNil(store.errorMessage, "Generic scan should not set an error on success.")
        try expectEqual(store.skills.count, 3, "Generic scan should refresh the catalog collections.")
        try expectEqual(store.skills.first { $0.id == "gamma" }?.agent, "codex", "Scan fixtures should exercise a Codex skill record.")
        try expectEqual(store.lastMutationMessage, UIStrings.scannedSkills(3), "Generic scan should expose adapter-neutral copy.")
        try expectEqual(store.refreshStatusMessage, UIStrings.refreshScanComplete(3, 3, 0, 0), "Generic scan should use refresh activity counts.")
        try expectEqual(store.lastScanActivity?.agentSummaries?.count, 2, "Scan should retain per-agent adapter diagnostics when the service provides them.")
        try expectEqual(store.lastScanActivity?.agentSummaries?.first { $0.agent == "claude-code" }?.rootsSkipped, ["/tmp/missing-claude"], "Scan diagnostics should decode skipped roots.")
        store.agentFilter = .codex
        try expectEqual(store.selectedAgentRefreshSummary?.rootsScanned, ["/tmp/codex"], "Selected adapter diagnostics should follow the agent filter.")
        try expectEqual(countOccurrences("app.stateSnapshot", in: fake.calls()), 1, "Scan should refresh collections with one app state snapshot call.")
        try expectEqual(countOccurrences("catalog.listSkills", in: fake.calls()), 0, "Scan refresh should not launch a separate skills list sidecar.")
        try expectEqual(countOccurrences("catalog.listFindings", in: fake.calls()), 0, "Scan refresh should not launch a separate findings list sidecar.")
        try expectEqual(countOccurrences("catalog.listConflicts", in: fake.calls()), 0, "Scan refresh should not launch a separate conflicts list sidecar.")
        try expectEqual(countMethodCalls("snapshot.list", in: fake.calls()), 0, "Scan refresh should not launch a global snapshots list sidecar.")
        try expectFalse(countMethodCalls("snapshot.listAgentConfig", in: fake.calls()) == 0, "Scan refresh should refresh at least one writable agent config history.")
    }

    private func searchAndFilterChangesNormalizeSelectionAndDetail() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "toggle-disabled")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "gamma"
        await store.reload()

        store.searchText = "beta"
        try await waitUntil("Search should move selection to the visible matching skill.") {
            store.selectedSkillID == "beta" && store.selectedSkillDetail?.id == "beta"
        }

        store.searchText = ""
        try await waitUntil("Clearing search should keep the normalized visible selection and matching detail.") {
            store.selectedSkillID == "beta" && store.selectedSkillDetail?.id == "beta"
        }

        store.stateFilter = .enabled
        try await waitUntil("State filter should move selection to a visible enabled skill and load matching detail.") {
            store.selectedSkillID == "alpha" && store.selectedSkillDetail?.id == "alpha"
        }
    }

    private func refreshOperationsIgnoreReentryWhileBusy() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "scan-slow")

        let store = SkillStore(service: fake.serviceClient())
        let task = Task {
            await store.scanAll()
        }

        try await waitUntil("Scan should expose scanning state while the service request is in flight.") {
            store.isScanning
        }

        await store.scanAll()
        await store.reload()
        await store.setProject(rootPath: "/tmp/project", currentCWD: "/tmp/project", name: "Fixture Project")
        await store.clearProject()
        await task.value

        try expectEqual(countOccurrences("catalog.scanAll", in: fake.calls()), 1, "Busy scan should ignore nested scan/reload/project update attempts.")
        try expectFalse(fake.calls().contains("project.setContext"), "Busy scan should guard project set reentry.")
        try expectFalse(fake.calls().contains("project.clearContext"), "Busy scan should guard project clear reentry.")
    }

    private func agentConfigTimelineFollowsSelectedAgentFilterOnly() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "timeline")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()

        try expectEqual(store.selectedAgentConfigTimelineAgent, "claude-code", "Default timeline should use the selected agent filter.")
        try expectEqual(store.agentConfigSnapshots.map(\.id), ["snap-claude-new", "snap-claude-old"], "Claude filter should show only Claude config snapshots.")
        try expectEqual(Set(store.agentConfigSnapshots.map(\.agent)), Set(["claude-code"]), "Claude timeline should not include other agents.")

        let callsAfterReload = countMethodCalls("snapshot.listAgentConfig", in: fake.calls())
        store.selectedSkillID = "alpha"
        await store.loadSelectedDetail()
        try expectEqual(store.agentConfigSnapshots.map(\.id), ["snap-claude-new", "snap-claude-old"], "Changing skill selection within an agent must not turn config snapshots into per-skill history.")
        try expectEqual(countMethodCalls("snapshot.listAgentConfig", in: fake.calls()), callsAfterReload, "Skill detail changes should not reload agent config history.")

        store.agentFilter = .codex
        try await waitUntil("Codex filter should load only Codex config snapshots.") {
            store.agentConfigSnapshots.map(\.id) == ["snap-codex"]
        }
        try expectEqual(store.selectedAgentConfigTimelineAgent, "codex", "Codex timeline should use the selected agent filter.")
        try expectEqual(Set(store.agentConfigSnapshots.map(\.agent)), Set(["codex"]), "Codex timeline should not include Claude snapshots.")
        try expectContains(fake.calls(), "\"agent\":\"codex\"", "Timeline fetch should request the selected Codex agent.")

        store.agentFilter = .all
        try await waitUntil("All filter should not merge every agent config timeline.") {
            store.agentConfigSnapshots.isEmpty
        }
        try expectNil(store.selectedAgentConfigTimelineAgent, "All filter has no single selected agent timeline.")
    }

    private func previewRollbackShowsDiffWithoutCallingRollback() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "timeline")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()

        let preview = try await store.previewRollback(snapshotID: "snap-claude-new")

        try expectEqual(preview.snapshot.id, "snap-claude-new", "Preview should return the selected snapshot.")
        try expectEqual(preview.snapshot.agent, "claude-code", "Preview should keep the snapshot agent.")
        try expectContains(preview.currentContent, "skillOverrides", "Preview diff should include current config content.")
        try expectEqual(preview.changed, true, "Preview should report that current config differs from the snapshot.")
        try expectEqual(preview.rollbackSupported, true, "Preview should expose rollback support without performing it.")
        try expectContains(fake.calls(), "snapshot.previewRollback", "Preview should call only the preview method.")
        try expectEqual(countMethodCalls("snapshot.rollback", in: fake.calls()), 0, "Preview must not call rollback or write automatically.")
    }

    private func readOnlyAgentConfigLoadsCurrentDocuments() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "agent-config")

        let prewarmedStore = SkillStore(service: fake.serviceClient())
        await prewarmedStore.loadCurrentAgentConfigDocuments(agent: "claude-code")
        guard let prewarmedProjectClaudeDocument = prewarmedStore.currentAgentConfigDocuments.first(where: { $0.scope == "agent-project" }) else {
            throw NativeModelTestFailure(description: "Prewarmed Claude config preview should include the selected project's current settings file.")
        }
        try expectNil(
            prewarmedStore.selectedConfigDocument,
            "Prewarming config documents outside config mode should not steal the visible selection."
        )
        prewarmedStore.sidebarContentMode = .config
        try expectEqual(
            prewarmedStore.selectedConfigDocument?.target,
            Optional(prewarmedProjectClaudeDocument.target),
            "Entering config mode after startup prewarm should select the first visible current config document."
        )

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()

        store.sidebarContentMode = .config
        await store.loadCurrentAgentConfigDocuments(agent: "claude-code")
        try expectEqual(store.currentAgentConfigDocuments.count, 2, "Claude config preview should include global and project documents.")
        try expectEqual(
            Set(store.currentAgentConfigDocuments.map(\.scope)),
            Set(["agent-global", "agent-project"]),
            "Claude config preview should keep project config documents visible alongside global config."
        )
        guard let projectClaudeDocument = store.currentAgentConfigDocuments.first(where: { $0.scope == "agent-project" }) else {
            throw NativeModelTestFailure(description: "Claude config preview should include the selected project's current settings file.")
        }
        try expectEqual(
            store.selectedConfigDocument?.target,
            Optional(projectClaudeDocument.target),
            "Config mode should default to the first visible current config document."
        )
        try expectEqual(
            AgentConfigScopeFilter.project.includes(projectClaudeDocument),
            true,
            "Project scope filter should include current project config documents."
        )
        store.selectConfigDocument(projectClaudeDocument)
        try expectEqual(
            store.selectedConfigDocument?.target,
            Optional(projectClaudeDocument.target),
            "Selecting a current config row should make that exact document available to the detail view."
        )
        store.configScopeFilter = .global
        guard let globalClaudeDocument = store.currentAgentConfigDocuments.first(where: { $0.scope == "agent-global" }) else {
            throw NativeModelTestFailure(description: "Claude config preview should include the agent-global settings file.")
        }
        try expectEqual(
            store.selectedConfigDocument?.target,
            Optional(globalClaudeDocument.target),
            "Changing to a scope filter that hides the current config document should select the first visible config document."
        )
        store.configScopeFilter = .all
        store.selectConfigDocument(globalClaudeDocument)
        store.sidebarContentMode = .skills
        store.enterConfigMode()
        try expectEqual(
            store.selectedConfigDocument?.target,
            Optional(projectClaudeDocument.target),
            "Entering config mode should reset stale config detail selection to the first visible current config document."
        )

        store.agentFilter = .codex
        await store.loadCurrentAgentConfigDocuments(agent: "codex")
        try expectEqual(store.currentAgentConfigDocuments.count, 2, "Codex config preview should include user and project documents.")
        try expectEqual(
            Set(store.currentAgentConfigDocuments.map(\.scope)),
            Set(["agent-global", "agent-project"]),
            "Codex config preview should keep project diagnostics visible alongside user config."
        )
        guard let projectCodexDocument = store.currentAgentConfigDocuments.first(where: { $0.scope == "agent-project" }) else {
            throw NativeModelTestFailure(description: "Codex config preview should include the selected project's .codex/config.toml.")
        }
        try expectContains(projectCodexDocument.target, ".codex/config.toml", "Codex project document should use the project config path.")
        try expectEqual(
            AgentConfigScopeFilter.project.includes(projectCodexDocument),
            true,
            "Project scope filter should include Codex project config documents."
        )

        store.agentFilter = .opencode
        await store.loadCurrentAgentConfigDocuments(agent: "opencode")
        try expectEqual(store.currentAgentConfigDocuments.count, 2, "opencode config preview should include global and project documents.")
        try expectEqual(
            Set(store.currentAgentConfigDocuments.map(\.scope)),
            Set(["agent-global", "agent-project"]),
            "opencode config preview should keep document scopes."
        )

        store.agentFilter = .pi
        await store.loadCurrentAgentConfigDocuments(agent: "pi")

        try expectEqual(store.currentAgentConfigDocuments.count, 2, "Pi config preview should include global and project documents.")
        try expectEqual(Set(store.currentAgentConfigDocuments.map(\.scope)), Set(["agent-global", "agent-project"]), "Pi config preview should keep document scopes.")
        try expectContains(store.currentAgentConfigDocuments.first?.content, "alibabacloud-agentbay-aio-skills", "Pi config preview should expose current disabled skill config content.")

        store.agentFilter = .hermes
        await store.loadCurrentAgentConfigDocuments(agent: "hermes")
        try expectEqual(store.currentAgentConfigDocuments.count, 1, "Hermes config preview should expose its global config document.")
        try expectEqual(store.currentAgentConfigDocuments.first?.scope, "agent-global", "Hermes should not invent a project config document.")
        try expectEqual(
            AgentConfigScopeFilter.project.includes(store.currentAgentConfigDocuments[0]),
            false,
            "Project scope filter should not include global-only Hermes config documents."
        )

        store.agentFilter = .openclaw
        await store.loadCurrentAgentConfigDocuments(agent: "openclaw")
        try expectEqual(store.currentAgentConfigDocuments.count, 1, "OpenClaw config preview should expose its global config document.")
        try expectEqual(store.currentAgentConfigDocuments.first?.scope, "agent-global", "OpenClaw config writes are global-only.")

        try expectContains(fake.calls(), "config.readAgentConfig", "Config preview should call the read-only agent config method.")
        try expectEqual(countMethodCalls("config.toggleSkill", in: fake.calls()), 0, "Config preview must not call toggle or write paths.")
    }

    private func agentConfigRefreshIfNeededSkipsLoadedRequestAndScopeFiltersLocally() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "agent-config")

        let store = SkillStore(service: fake.serviceClient())
        await store.loadCurrentAgentConfigDocumentsIfNeeded(agent: "claude-code")
        await store.loadAgentConfigSnapshotsIfNeeded(agent: "claude-code")

        let configReadsAfterFirstLoad = countMethodCalls("config.readAgentConfig", in: fake.calls())
        let snapshotReadsAfterFirstLoad = countMethodCalls("snapshot.listAgentConfig", in: fake.calls())

        await store.loadCurrentAgentConfigDocumentsIfNeeded(agent: "claude-code")
        await store.loadAgentConfigSnapshotsIfNeeded(agent: "claude-code")

        try expectEqual(
            countMethodCalls("config.readAgentConfig", in: fake.calls()),
            configReadsAfterFirstLoad,
            "Need-based current config load should not reread the same agent/project payload."
        )
        try expectEqual(
            countMethodCalls("snapshot.listAgentConfig", in: fake.calls()),
            snapshotReadsAfterFirstLoad,
            "Need-based config history load should not reread the same agent/project payload."
        )

        store.configScopeFilter = .project

        try expectEqual(
            countMethodCalls("config.readAgentConfig", in: fake.calls()),
            configReadsAfterFirstLoad,
            "Config scope changes should filter cached current documents locally."
        )
        try expectEqual(
            countMethodCalls("snapshot.listAgentConfig", in: fake.calls()),
            snapshotReadsAfterFirstLoad,
            "Config scope changes should filter cached history locally."
        )

        await store.loadCurrentAgentConfigDocuments(agent: "claude-code")
        await store.loadAgentConfigSnapshots(agent: "claude-code")

        try expectEqual(
            countMethodCalls("config.readAgentConfig", in: fake.calls()),
            configReadsAfterFirstLoad + 1,
            "Manual current config refresh should still force a new read."
        )
        try expectEqual(
            countMethodCalls("snapshot.listAgentConfig", in: fake.calls()),
            snapshotReadsAfterFirstLoad + 1,
            "Manual config history refresh should still force a new read."
        )
    }

    private func runtimeAnalysisRefreshIfNeededSkipsLoadedRequestsAndForceRefreshes() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        await store.loadCleanupQueueIfNeeded()
        await store.loadCrossAgentComparisonsIfNeeded()

        let cleanupReadsAfterFirstLoad = countMethodCalls("cleanup.listQueue", in: fake.calls())
        let comparisonReadsAfterFirstLoad = countMethodCalls("comparison.listCrossAgent", in: fake.calls())

        try expectEqual(cleanupReadsAfterFirstLoad, 1, "Need-based cleanup load should read once for the current request key.")
        try expectEqual(comparisonReadsAfterFirstLoad, 1, "Need-based cross-agent load should read once for the current request key.")

        await store.loadCleanupQueueIfNeeded()
        await store.loadCrossAgentComparisonsIfNeeded()

        try expectEqual(
            countMethodCalls("cleanup.listQueue", in: fake.calls()),
            cleanupReadsAfterFirstLoad,
            "Need-based cleanup load should reuse the loaded request key instead of rereading."
        )
        try expectEqual(
            countMethodCalls("comparison.listCrossAgent", in: fake.calls()),
            comparisonReadsAfterFirstLoad,
            "Need-based cross-agent load should reuse the loaded request key instead of rereading."
        )

        await store.loadCleanupQueue()
        await store.loadCrossAgentComparisons()

        try expectEqual(
            countMethodCalls("cleanup.listQueue", in: fake.calls()),
            cleanupReadsAfterFirstLoad + 1,
            "Manual cleanup refresh should still force a new service read."
        )
        try expectEqual(
            countMethodCalls("comparison.listCrossAgent", in: fake.calls()),
            comparisonReadsAfterFirstLoad + 1,
            "Manual cross-agent refresh should still force a new service read."
        )
    }

    private func settingsFeedbackClearsAfterFailedConfigSave() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        let saved = await store.saveClaudeSettings(content: "{")

        try expectFalse(saved, "Unsupported fake settings save should fail in this scenario.")
        try expectContains(store.settingsErrorMessage, "test.unknown", "Failed config save should surface a settings error.")

        store.clearSettingsFeedback()

        try expectNil(store.settingsErrorMessage, "Continuing to edit settings should clear stale config save errors.")
        try expectNil(store.settingsMessage, "Continuing to edit settings should clear stale config save success messages.")
    }

    private func rollbackSnapshotRequiresVisibleAgentTimelineRecord() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "timeline")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()

        await store.rollbackSnapshot(snapshotID: "snap-codex")

        try expectContains(store.errorMessage, "selected agent config timeline", "Rollback should reject snapshots outside the selected agent timeline.")
        try expectEqual(countMethodCalls("snapshot.rollback", in: fake.calls()), 0, "Rollback guard should not call the write API for hidden agent snapshots.")
    }

    private func agentFilterLimitsVisibleSkillsAndSelection() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()

        store.agentFilter = .codex

        try expectEqual(store.filteredSkills.map(\.id), ["gamma"], "Codex filter should expose only Codex skills.")
        try expectEqual(store.filteredSkillGroups.map(\.title), [UIStrings.codex], "Codex filter should group under the Codex display name.")
        try expectEqual(store.selectedSkillID, "gamma", "Agent filter should move selection to a visible skill.")
        try expectEqual(store.selectedSkill?.agent, "codex", "Selected skill should respect the active agent filter.")
    }

    private func allAgentFilterDoesNotFetchMixedConfigHistory() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.agentFilter = .all
        await store.reload()

        try expectEqual(store.agentConfigSnapshots.count, 0, "All Agents should not expose a mixed agent-config history.")
    }

    private func toggleSelectedSkillExposesWritingStateAndRefreshesSelection() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()

        fake.setScenario("toggle-disabled")
        let task = Task {
            await store.toggleSelectedSkill(on: false)
        }

        try await waitUntil("Toggle should expose writing state while the service request is in flight.") {
            store.isWriting
        }
        await task.value

        try expectFalse(store.isWriting, "Toggle should reset writing state.")
        try expectNil(store.errorMessage, "Toggle should not set an error on success.")
        try expectEqual(store.selectedSkillID, "beta", "Toggle refresh should keep the selected skill stable.")
        try expectEqual(store.selectedSkill?.enabled, false, "Toggle refresh should expose the updated enabled state.")
        try expectEqual(store.selectedSkillDetail?.enabled, false, "Toggle refresh should reload detail for the updated skill.")
        try expectEqual(store.lastMutationMessage, UIStrings.toggledSkill(on: false, name: "Beta"), "Toggle should expose a success message.")
    }

    private func writeOperationsIgnoreReentryWhileBusy() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()

        fake.setScenario("toggle-disabled")
        let task = Task {
            await store.toggleSelectedSkill(on: false)
        }

        try await waitUntil("Toggle should expose writing state while the service request is in flight.") {
            store.isWriting
        }
        await store.toggleSelectedSkill(on: true)
        await task.value

        try expectEqual(countOccurrences("config.toggleSkill", in: fake.calls()), 1, "Busy write should ignore reentrant write attempts.")
    }

    private func codexToggleAddsRestartRequiredNotice() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.agentFilter = .codex
        store.selectedSkillID = "gamma"
        await store.reload()

        fake.setScenario("toggle-codex-disabled")
        await store.toggleSelectedSkill(on: false)

        try expectFalse(store.isWriting, "Codex toggle should reset writing state.")
        try expectNil(store.errorMessage, "Codex toggle should not set an error on success.")
        try expectEqual(store.selectedSkillID, "gamma", "Codex toggle refresh should keep the selected skill stable.")
        try expectEqual(store.selectedSkill?.enabled, false, "Codex toggle refresh should expose the updated enabled state.")
        try expectEqual(store.selectedSkillDetail?.enabled, false, "Codex toggle refresh should reload detail for the updated skill.")
        try expectEqual(
            store.lastMutationMessage,
            UIStrings.toggledSkill(on: false, name: "Gamma", agent: "codex"),
            "Codex toggle should add the restart-required note."
        )
        try expectContains(store.lastMutationMessage, UIStrings.codexRestartRequired, "Codex toggle should mention restart.")
    }

    private func opencodeToggleCallsServiceAndRefreshesSelection() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "opencode")

        let store = SkillStore(service: fake.serviceClient())
        store.agentFilter = .opencode
        store.selectedSkillID = "omega"
        await store.reload()

        guard let selectedSkill = store.selectedSkill else {
            throw NativeModelTestFailure(description: "Fixture should select an opencode skill.")
        }
        try expectEqual(store.selectedSkill?.agent, "opencode", "Fixture should select an opencode skill.")
        try expectNil(
            DisplayText.toggleDisabledReason(for: selectedSkill, isWriting: false),
            "opencode toggle should be available in V2.12."
        )

        await store.toggleSelectedSkill(on: false)

        try expectFalse(store.isWriting, "opencode toggle should finish writing state.")
        try expectNil(store.errorMessage, "opencode toggle should not surface a read-only error.")
        try expectContains(fake.calls(), "config.toggleSkill", "opencode toggle should call the write API.")
        try expectEqual(store.selectedSkill?.enabled, false, "opencode toggle refresh should expose the updated enabled state.")
        try expectEqual(store.selectedSkillDetail?.enabled, false, "opencode toggle refresh should reload detail for the updated skill.")
    }

    private func toolGlobalToggleIsPreviewOnlyAndDoesNotCallService() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "tool-global")

        let store = SkillStore(service: fake.serviceClient())
        store.agentFilter = .all
        store.selectedSkillID = "tool-alpha"
        await store.reload()

        let expectedReason = UIStrings.toggleUnavailableToolGlobal
        guard let selectedSkill = store.selectedSkill else {
            throw NativeModelTestFailure(description: "Fixture should select a tool-global skill.")
        }
        try expectEqual(store.selectedSkill?.scope, "tool-global", "Fixture should select a tool-global skill.")
        try expectEqual(
            DisplayText.toggleDisabledReason(for: selectedSkill, isWriting: false),
            expectedReason,
            "Tool-global toggle should explain the read-only preview and confirmed install path."
        )

        await store.toggleSelectedSkill(on: false)

        try expectFalse(store.isWriting, "Tool-global toggle should not enter writing state.")
        try expectEqual(store.errorMessage, expectedReason, "Tool-global toggle should surface the disabled reason.")
        try expectFalse(fake.calls().contains("config.toggleSkill"), "Tool-global toggle should not call the write API.")
    }

    private func batchTogglePreviewFiltersReadOnlyAndNoopSkills() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "batch-mixed")

        let store = SkillStore(service: fake.serviceClient())
        store.agentFilter = .all
        store.batchToggleAction = .disable
        await store.reload()
        await store.previewVisibleBatchToggle()

        guard let preview = store.batchTogglePreview else {
            throw NativeModelTestFailure(description: "Batch preview should be stored.")
        }
        try expectEqual(preview.action, .disable, "Batch preview should preserve the selected action.")
        try expectEqual(preview.selectedCount, 4, "Batch preview should include the visible filtered skills.")
        try expectEqual(preview.writableCount, 2, "Batch preview should include only writable affected skills.")
        try expectEqual(preview.skippedCount, 2, "Batch preview should skip read-only and no-op skills.")
        try expectEqual(preview.affectedSkills.map(\.instanceID), ["alpha", "gamma"], "Batch preview should affect writable enabled skills.")
        try expectEqual(preview.skippedItems.map(\.instanceID), ["beta", "pi-one"], "Batch preview should report skipped skills.")
        try expectContains(preview.skippedItems.first { $0.instanceID == "pi-one" }?.reason, "read-only", "Read-only agent skip reason should be visible.")
        try expectContains(fake.calls(), "batch.previewSkillToggles", "Batch preview should use the service preview method.")
        try expectFalse(fake.calls().contains("batch.applySkillToggles"), "Preview must not apply.")
        try expectFalse(fake.calls().contains("config.toggleSkill"), "Batch preview must not use the single-toggle write path.")
    }

    private func batchTogglePreviewHonorsExplicitSelection() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "batch-mixed")

        let store = SkillStore(service: fake.serviceClient())
        store.agentFilter = .all
        store.batchToggleAction = .disable
        await store.reload()

        guard let alpha = store.filteredSkills.first(where: { $0.id == "alpha" }) else {
            throw NativeModelTestFailure(description: "Batch fixture should include alpha.")
        }

        store.clearBatchToggleSelection()
        try expectEqual(store.batchToggleSelectedSkills.count, 0, "Clearing batch selection should not fall back to all visible skills.")

        store.setBatchToggleSkill(alpha, selected: true)
        await store.previewVisibleBatchToggle()

        guard let preview = store.batchTogglePreview else {
            throw NativeModelTestFailure(description: "Explicit batch selection should produce a preview.")
        }
        try expectEqual(store.batchToggleSelectedSkills.map(\.id), ["alpha"], "Store selection should keep only explicitly selected skills.")
        try expectContains(fake.calls(), #""instance_ids":["alpha"]"#, "Batch preview request should send only explicitly selected skill IDs.")
        try expectEqual(preview.id, "batch-preview-1", "Fixture preview should still be decoded after explicit selection.")

        store.selectAllVisibleBatchToggleSkills()
        try expectEqual(store.batchToggleSelectedSkills.count, store.filteredSkills.count, "Select All should restore the full visible skill set.")
    }

    private func batchToggleApplyUsesBatchServiceAndRefreshes() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "batch-mixed")

        let store = SkillStore(service: fake.serviceClient())
        store.agentFilter = .all
        store.batchToggleAction = .disable
        await store.reload()
        await store.previewVisibleBatchToggle()
        await store.applyVisibleBatchTogglePreview()

        try expectNil(store.batchTogglePreview, "Batch preview should clear after apply and refresh.")
        try expectContains(store.lastMutationMessage, "Disable batch applied", "Batch apply should surface an explicit success message.")
        try expectContains(fake.calls(), "batch.applySkillToggles", "Batch apply should use the service batch apply method.")
        try expectFalse(fake.calls().contains("config.toggleSkill"), "Batch apply must not silently loop over single-toggle writes.")
        try expectEqual(store.skills.first { $0.id == "alpha" }?.enabled, false, "Batch apply refresh should pick up changed alpha state.")
        try expectEqual(store.skills.first { $0.id == "gamma" }?.enabled, false, "Batch apply refresh should pick up changed gamma state.")
        try expectEqual(store.skills.first { $0.id == "pi-one" }?.enabled, true, "Batch apply should not mutate read-only Pi skills.")
    }

    private func batchToggleApplyRequiresCurrentPreviewConfirmation() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "batch-mixed")

        let store = SkillStore(service: fake.serviceClient())
        store.agentFilter = .all
        store.batchToggleAction = .disable
        await store.reload()
        await store.previewVisibleBatchToggle()

        guard store.batchTogglePreview != nil else {
            throw NativeModelTestFailure(description: "Batch preview should be stored before confirmation.")
        }

        await store.applyVisibleBatchTogglePreview(confirmingPreviewID: "stale-preview-token")

        guard store.batchTogglePreview != nil else {
            throw NativeModelTestFailure(description: "Stale confirmation must not clear the active preview.")
        }
        try expectContains(store.errorMessage, "Preview again", "Stale confirmation should explain that a fresh preview is required.")
        try expectFalse(fake.calls().contains("batch.applySkillToggles"), "Stale confirmation must not call the batch apply service.")
    }

    private func localReportExportUsesUserTriggeredServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "report-export")

        let store = SkillStore(service: fake.serviceClient())
        store.agentFilter = .all
        store.localReportFormat = .json
        await store.reload()
        await store.exportLocalReport()

        guard let result = store.localReportExportResult else {
            throw NativeModelTestFailure(description: "Local report export should store the service result.")
        }
        try expectFalse(result.isUnavailable, "Successful export should not use the unavailable fallback.")
        try expectEqual(result.format, .json, "Export result should preserve the selected JSON format.")
        try expectEqual(result.filename, "report.json", "Export should expose the local filename from the exported files list.")
        try expectEqual(result.path, "<app-data-dir>/report-exports/local-report-test/report.json", "Export should expose the redacted local path.")
        try expectEqual(result.redacted, true, "Local report export should surface the redaction flag.")
        try expectEqual(result.sections.map(\.name), ["cleanup_item_count", "comparison_group_count", "finding_count", "open_finding_count", "skill_count", "triage_count"], "Export should derive section counts from the service summary object.")
        try expectContains(result.summary, "redacted", "Export summary should be user-visible and redaction-explicit.")
        try expectContains(store.lastMutationMessage, "report.json", "Successful export should show the filename.")
        try expectContains(fake.calls(), "report.exportLocal", "Export must use the V2.35 report.exportLocal service method.")
        try expectContains(fake.calls(), #""formats":["json"]"#, "Export must send the selected format using the service formats array.")
        try expectFalse(fake.calls().contains("llm.prepare"), "Export must not trigger AI provider preparation.")
        try expectFalse(fake.calls().contains("script.previewExecution"), "Export must not trigger script execution preview.")
        try expectFalse(fake.calls().contains("config.toggleSkill"), "Export must not mutate agent config.")
    }

    private func localReportExportCanUseAgentWorkspaceScopeWithoutSelectedSkill() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "report-export")

        let store = SkillStore(service: fake.serviceClient())
        store.agentFilter = .claudeCode
        store.localReportFormat = .json
        await store.reload()

        guard store.selectedSkill != nil else {
            throw NativeModelTestFailure(description: "Fixture reload should select a visible skill.")
        }

        await store.exportLocalReport(includeSelectedSkill: false)

        guard let reportCall = fake.calls()
            .split(separator: "\n")
            .last(where: { $0.contains(#""method":"report.exportLocal""#) }) else {
            throw NativeModelTestFailure(description: "Agent workspace export should call report.exportLocal.")
        }
        try expectContains(String(reportCall), #""agent":"claude-code""#, "Agent workspace export should retain the current agent scope.")
        try expectFalse(reportCall.contains(#""instance_id""#), "Agent workspace export should not silently narrow to the previously selected skill.")
    }

    private func localReportExportClearsStaleResultWhenScopeChanges() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "report-export")

        let store = SkillStore(service: fake.serviceClient())
        store.agentFilter = .claudeCode
        await store.reload()
        await store.exportLocalReport(includeSelectedSkill: false)

        try expectEqual(store.localReportExportResult?.filename, "report.json", "Export should store the current report before scope changes.")
        try expectContains(store.lastMutationMessage, "report.json", "Export should show the current report filename before scope changes.")

        store.agentFilter = .codex

        try expectNil(store.localReportExportResult, "Changing agent scope should hide the stale report export result.")
        try expectNil(store.lastMutationMessage, "Changing report scope should clear the stale report success message.")
    }

    private func localReportExportUnavailableDoesNotPretendFileWasWritten() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.localReportFormat = .markdown
        await store.reload()
        await store.exportLocalReport()

        guard let result = store.localReportExportResult else {
            throw NativeModelTestFailure(description: "Unavailable export should store a visible fallback result.")
        }
        try expectEqual(result.isUnavailable, true, "Unknown report method should expose an unavailable result.")
        try expectNil(result.path, "Unavailable export must not fake a local file path.")
        try expectNil(store.lastMutationMessage, "Unavailable export must not show a success mutation message.")
        try expectContains(result.summary, "unavailable", "Unavailable result should explain that no file was written.")
        try expectContains(fake.calls(), "report.exportLocal", "Unavailable path should still attempt the explicit user-triggered export method.")
    }

    private func reloadLoadsProjectContext() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "project-set")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()

        try expectEqual(store.activeProjectContext?.name, "Fixture Project", "Reload should load active project context.")
        try expectEqual(store.activeProjectContext?.rootPath, "/tmp/project", "Reload should expose the active project root.")
        try expectEqual(store.recentProjectContexts.count, 2, "Reload should expose recent project contexts.")
        try expectNil(store.projectValidationMessage, "Valid context should not expose a validation error.")
    }

    private func setProjectStoresContextAndScans() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "project-set")

        let store = SkillStore(service: fake.serviceClient())
        await store.setProject(rootPath: "/tmp/project", currentCWD: "/tmp/project", name: "Fixture Project")

        try expectFalse(store.isProjectUpdating, "Project set should reset updating state.")
        try expectNil(store.errorMessage, "Project set should not set an error on success.")
        try expectEqual(store.activeProjectContext?.name, "Fixture Project", "Project set should store returned active context.")
        try expectEqual(store.skills.count, 3, "Project set should scan and refresh catalog collections.")
        try expectContains(fake.calls(), "project.setContext", "Project set should call the service project method.")
        try expectContains(fake.calls(), "catalog.scanAll", "Project set should scan after a valid context is selected.")
        try expectEqual(store.lastMutationMessage, UIStrings.projectSelectedAndScanned("Fixture Project"), "Project set should expose a context refresh message.")
    }

    private func clearProjectClearsContextAndScans() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "project-clear")

        let store = SkillStore(service: fake.serviceClient())
        await store.clearProject()

        try expectFalse(store.isProjectUpdating, "Project clear should reset updating state.")
        try expectNil(store.errorMessage, "Project clear should not set an error on success.")
        try expectNil(store.activeProjectContext, "Project clear should remove the active context.")
        try expectEqual(store.skills.count, 3, "Project clear should scan and refresh catalog collections.")
        try expectContains(fake.calls(), "project.clearContext", "Project clear should call the service project method.")
        try expectContains(fake.calls(), "catalog.scanAll", "Project clear should scan after clearing context.")
        try expectEqual(store.lastMutationMessage, UIStrings.projectClearedAndScanned, "Project clear should expose a context refresh message.")
    }

    private func projectValidationErrorSkipsScanAndSurfacesMessage() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "project-validation-error")

        let store = SkillStore(service: fake.serviceClient())
        await store.setProject(rootPath: "/tmp/missing", currentCWD: "/tmp/missing", name: "Missing Project")

        try expectFalse(store.isProjectUpdating, "Invalid project set should reset updating state.")
        try expectEqual(store.activeProjectContext?.name, "Missing Project", "Invalid project set should keep returned context for user repair.")
        try expectEqual(store.projectValidationMessage, "Project root does not exist.", "Invalid project set should expose validation details.")
        try expectContains(store.errorMessage, "Project validation failed", "Invalid project set should surface a readable error.")
        try expectFalse(fake.calls().contains("catalog.scanAll"), "Invalid project set should not scan.")
    }

    private func reloadFallsBackToDisabledLLMWhenOldServiceDoesNotSupportMethods() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "old-service")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()

        try expectNil(store.errorMessage, "Old service LLM fallback should not fail reload.")
        try expectFalse(store.llmStatus.enabled, "Old service LLM fallback should be disabled.")
        try expectEqual(store.llmStatus.disabledReason, UIStrings.llmDisabledFallback, "Old service LLM fallback should expose a stable reason.")
        try expectContains(fake.calls(), "llm.status", "Reload should ask the service for LLM status.")
    }

    private func prepareLLMActionStoresEstimateWithoutProviderCall() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "llm-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.prepareAnalyzeLLM()
        await store.prepareDraftFrontmatterLLM()

        let analyze = store.llmPrepareResult(for: .analyze)
        try expectEqual(analyze?.enabled, true, "Analyze prepare should be enabled when LLM status is ready.")
        try expectEqual(analyze?.provider, "openai", "Analyze prepare should expose provider.")
        try expectEqual(analyze?.model, "gpt-5", "Analyze prepare should expose model.")
        try expectEqual(analyze?.estimate?.inputTokens, 240, "Analyze prepare should expose input token estimate.")
        try expectEqual(analyze?.estimate?.estimatedCostUSD, 0.0042, "Analyze prepare should expose cost estimate.")
        try expectEqual(analyze?.confirmationRequired, true, "Analyze prepare should require confirmation.")

        let draft = store.llmPrepareResult(for: .draftFrontmatter)
        try expectEqual(draft?.action, .draftFrontmatter, "Draft prepare should be stored under the draft action.")
        try expectEqual(draft?.confirmationRequired, true, "Draft prepare should require confirmation.")
        try expectContains(fake.calls(), "llm.prepareAction", "LLM action should use prepare preflight.")
        try expectFalse(fake.calls().contains("llm.complete"), "LLM prepare should not call a provider completion method.")
    }


    private func prepareSkillAnalysisUsesReadOnlyPrepareContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "llm-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.prepareSelectedSkillAnalysis(kind: .risk)
        await store.prepareVisibleSkillAnalysis(kind: .overview)

        let selected = store.skillAnalysisPrepareResult(kind: .risk, scope: .selected)
        try expectEqual(selected?.enabled, false, "Skill analysis prepare should stay disabled by default.")
        try expectEqual(selected?.analysisKind, .risk, "Selected skill analysis should preserve requested kind.")
        try expectEqual(selected?.selectedSkillCount, 1, "Selected skill analysis should include one skill.")
        try expectEqual(selected?.includedSkills.map(\.name), ["Beta"], "Selected skill analysis should expose included skill names.")
        try expectFalse(selected?.safety.writeBackEnabled ?? true, "Skill analysis prepare must not enable write-back.")
        try expectFalse(selected?.safety.scriptExecutionEnabled ?? true, "Skill analysis prepare must not enable script execution.")
        try expectFalse(selected?.safety.credentialStorageEnabled ?? true, "Skill analysis prepare must not enable credential storage.")
        try expectEqual(selected?.safety.confirmationRequired, true, "Skill analysis prepare should require confirmation.")

        let visible = store.skillAnalysisPrepareResult(kind: .overview, scope: .visible)
        try expectEqual(visible?.selectedSkillCount, 2, "Visible skill analysis should include current visible filtered skills.")
        let calls = fake.calls()
        try expectContains(calls, "llm.prepareSkillAnalysis", "Skill analysis should use the V2.30 prepare method.")
        try expectContains(calls, "\"analysis_kind\":\"risk\"", "Skill analysis should send requested risk kind.")
        try expectContains(calls, "\"instance_ids\":[\"beta\"]", "Selected skill analysis should send selected instance IDs only.")
        try expectFalse(calls.contains("llm.complete"), "Skill analysis prepare must not call provider completion.")
        try expectFalse(calls.contains("config.toggleSkill"), "Skill analysis prepare must not call write paths.")
        try expectFalse(calls.contains("script.execute"), "Skill analysis prepare must not call execution paths.")
    }

    private func prepareSkillAnalysisFallsBackWhenMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "old-service")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.prepareSelectedSkillAnalysis(kind: .cleanup)

        let result = store.skillAnalysisPrepareResult(kind: .cleanup, scope: .selected)
        try expectEqual(result?.enabled, false, "Unavailable skill analysis should be disabled.")
        try expectEqual(result?.disabledReason, UIStrings.llmSkillAnalysisUnavailable, "Unavailable skill analysis should expose quiet fallback reason.")
        try expectContains(result?.summaryDraft, "Disabled fallback preview only", "Unavailable skill analysis should provide read-only preview copy.")
    }

    private func scoreSkillQualityUsesReadOnlyAnalysisContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.scoreSelectedSkillQuality()

        guard let skill = store.selectedSkill else {
            throw NativeModelTestFailure(description: "Fixture should select beta for quality scoring.")
        }
        let result = store.skillQualityScore(for: skill)
        try expectEqual(result?.score, 82, "Quality score should store the service score for the selected skill.")
        try expectEqual(result?.displayBand, "Good", "Quality score should expose the band/grade.")
        try expectEqual(result?.components.map(\.key), ["metadata", "permissions"], "Quality score should expose component keys.")
        try expectFalse(result?.safety.providerRequestSent ?? true, "Local quality score must not send a provider request.")
        try expectFalse(result?.safety.writeBackAllowed ?? true, "Quality score must not allow write-back.")
        try expectFalse(result?.safety.scriptExecutionAllowed ?? true, "Quality score must not allow script execution.")
        try expectFalse(result?.safety.configMutationAllowed ?? true, "Quality score must not allow config mutation.")
        try expectFalse(result?.safety.credentialAccessed ?? true, "Quality score must not access credentials.")

        await store.previewPromptForSelectedSkillQuality()
        let preview = store.skillQualityPromptPreview(for: skill)
        try expectEqual(preview?.previewID, "quality-preview-beta", "Quality prompt preview should be stored for the selected skill.")
        try expectEqual(store.canSendSkillQualityPrompt(for: skill), true, "Configured provider and current quality preview should allow explicit send.")

        await store.confirmPromptForSelectedSkillQuality()
        let sendResult = store.skillQualityPromptSendResult(for: skill)
        try expectEqual(sendResult?.success, true, "Confirmed quality prompt should store provider result.")
        try expectFalse(sendResult?.writeBackAllowed ?? true, "Quality prompt output must not enable write-back.")
        try expectFalse(sendResult?.scriptExecutionAllowed ?? true, "Quality prompt output must not enable script execution.")

        let calls = fake.calls()
        try expectContains(calls, "analysis.scoreSkillQuality", "Quality score should use the V2.43 analysis method.")
        try expectContains(calls, "\"request_kind\":\"quality_score\"", "Quality prompt preview should identify the quality_score prompt action.")
        try expectContains(calls, "\"preview_id\":\"quality-preview-beta\"", "Quality confirm should send the quality preview id.")
        try expectContains(calls, "llm.confirmPromptAndSend", "Quality provider path should require explicit confirmation.")
        try expectFalse(calls.contains("config.toggleSkill"), "Quality scoring must not call write paths.")
        try expectFalse(calls.contains("script.execute"), "Quality scoring must not call execution paths.")
    }

    private func taskReadinessUsesReadOnlyCheckContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.taskReadinessText = "Audit local skills for a release note."
        await store.reload()
        await store.checkSelectedTaskReadiness()

        guard let skill = store.selectedSkill else {
            throw NativeModelTestFailure(description: "Fixture should select beta for task readiness.")
        }
        let result = store.taskReadiness(for: skill)
        try expectEqual(result?.score, 74, "Task readiness should store the service score for the selected task.")
        try expectEqual(result?.band, "Partial", "Task readiness should expose the readiness band.")
        try expectEqual(result?.candidateSkills.map(\.name), ["Beta"], "Task readiness should expose candidate skills.")
        try expectEqual(result?.gaps, ["No release-note-specific examples."], "Task readiness should expose missing capabilities.")
        try expectFalse(result?.safety.providerRequestSent ?? true, "Local readiness check must not send a provider request.")
        try expectFalse(result?.safety.writeBackAllowed ?? true, "Readiness check must not allow write-back.")
        try expectFalse(result?.safety.scriptExecutionAllowed ?? true, "Readiness check must not allow script execution.")
        try expectFalse(result?.safety.configMutationAllowed ?? true, "Readiness check must not allow config mutation.")
        try expectFalse(result?.safety.credentialAccessed ?? true, "Readiness check must not access credentials.")

        await store.previewPromptForSelectedTaskReadiness()
        let preview = store.taskReadinessPromptPreview(for: skill)
        try expectEqual(preview?.previewID, "readiness-preview-beta", "Task readiness prompt preview should be scoped to the selected task.")
        try expectEqual(store.canSendTaskReadinessPrompt(for: skill), true, "Configured provider and current readiness preview should allow explicit send.")

        await store.confirmPromptForSelectedTaskReadiness()
        let sendResult = store.taskReadinessPromptSendResult(for: skill)
        try expectEqual(sendResult?.success, true, "Confirmed task readiness prompt should store provider result.")
        try expectFalse(sendResult?.writeBackAllowed ?? true, "Task readiness prompt output must not enable write-back.")
        try expectFalse(sendResult?.scriptExecutionAllowed ?? true, "Task readiness prompt output must not enable script execution.")

        let calls = fake.calls()
        try expectContains(calls, "task.checkReadiness", "Task readiness should use the V2.44 task.checkReadiness method.")
        try expectContains(calls, "\"task\":\"Audit local skills for a release note.\"", "Task readiness should send task text.")
        try expectContains(calls, "\"candidate_instance_ids\":[\"beta\"]", "Task readiness should constrain candidate filters to the selected skill.")
        try expectContains(calls, "\"request_kind\":\"task_readiness\"", "Task readiness prompt preview should identify the task_readiness request kind.")
        try expectContains(calls, "\"preview_id\":\"readiness-preview-beta\"", "Task readiness confirm should send the readiness preview id.")
        try expectContains(calls, "llm.confirmPromptAndSend", "Configured task readiness provider path should require explicit confirmation.")
        try expectFalse(calls.contains("config.toggleSkill"), "Task readiness must not call write paths.")
        try expectFalse(calls.contains("script.execute"), "Task readiness must not call execution paths.")
    }

    private func taskReadinessPromptSendRequiresConfiguredProvider() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "llm-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.taskReadinessText = "Audit local skills for a release note."
        await store.reload()
        await store.checkSelectedTaskReadiness()

        guard let skill = store.selectedSkill else {
            throw NativeModelTestFailure(description: "Fixture should select beta for task readiness gating.")
        }

        await store.previewPromptForSelectedTaskReadiness()
        try expectEqual(store.canSendTaskReadinessPrompt(for: skill), false, "Unconfigured provider should keep Confirm & Send disabled.")

        await store.confirmPromptForSelectedTaskReadiness()
        let sendResult = store.taskReadinessPromptSendResult(for: skill)
        try expectEqual(sendResult?.success, false, "Blocked task readiness send should store an unavailable result, not throw.")
        try expectContains(sendResult?.message, "Configure and save", "Blocked task readiness send should explain provider setup.")

        let calls = fake.calls()
        try expectContains(calls, "task.checkReadiness", "No-provider path should still allow local readiness check.")
        try expectContains(calls, "llm.previewPrompt", "No-provider path may ask for a blocked preview.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "No-provider path must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "No-provider readiness path must not call write paths.")
        try expectFalse(calls.contains("script.execute"), "No-provider readiness path must not call execution paths.")
    }

    private func routingConfidenceUsesReadOnlyRankContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.routingConfidenceText = "Route a local audit release note task."
        await store.reload()
        await store.rankSelectedSkillRoutes()

        guard let skill = store.selectedSkill else {
            throw NativeModelTestFailure(description: "Fixture should select beta for routing confidence.")
        }
        let result = store.routingConfidence(for: skill)
        try expectEqual(result?.score, 88, "Routing confidence should store the service score for the selected task.")
        try expectEqual(result?.band, "High", "Routing confidence should expose the confidence band.")
        try expectEqual(result?.routes.map(\.name), ["Beta", "Alpha"], "Routing confidence should expose ordered candidate routes.")
        try expectEqual(result?.routes.first?.matchReasons, ["Description matches local audit.", "Selected skill is enabled."], "Routing confidence should expose match reasons.")
        try expectEqual(result?.ambiguityWarnings, ["Alpha has overlapping audit wording."], "Routing confidence should expose ambiguity warnings.")
        try expectEqual(result?.wrongPickRisks, ["Choosing Alpha may miss project-scoped evidence."], "Routing confidence should expose likely wrong-pick risks.")
        try expectFalse(result?.safety.providerRequestSent ?? true, "Local route ranking must not send a provider request.")
        try expectFalse(result?.safety.writeBackAllowed ?? true, "Route ranking must not allow write-back.")
        try expectFalse(result?.safety.scriptExecutionAllowed ?? true, "Route ranking must not allow script execution.")
        try expectFalse(result?.safety.configMutationAllowed ?? true, "Route ranking must not allow config mutation.")
        try expectFalse(result?.safety.credentialAccessed ?? true, "Route ranking must not access credentials.")
        let localRankingCalls = fake.calls()
        try expectContains(localRankingCalls, "task.rankSkillRoutes", "Routing confidence should use the V2.45 task.rankSkillRoutes method.")
        try expectContains(localRankingCalls, "\"task\":\"Route a local audit release note task.\"", "Routing confidence should send canonical task text.")
        try expectContains(localRankingCalls, "\"agent\":\"claude-code\"", "Routing confidence should send selected skill agent.")
        try expectContains(localRankingCalls, "\"candidate_instance_ids\":[\"beta\"]", "Routing confidence should constrain candidate filters to the selected skill.")
        try expectContains(localRankingCalls, "\"limit\":6", "Routing confidence should send a route limit.")
        try expectFalse(localRankingCalls.contains("\"task_text\":\"Route a local audit release note task.\""), "Local route ranking must not send duplicate task_text aliases.")
        try expectFalse(localRankingCalls.contains("\"user_intent\":\"Route a local audit release note task.\""), "Local route ranking must not send duplicate user_intent aliases.")

        await store.previewPromptForSelectedRoutingConfidence()
        let preview = store.routingConfidencePromptPreview(for: skill)
        try expectEqual(preview?.previewID, "routing-preview-beta", "Routing prompt preview should be scoped to the selected task.")
        try expectEqual(store.canSendRoutingConfidencePrompt(for: skill), true, "Configured provider and current routing preview should allow explicit send.")

        await store.confirmPromptForSelectedRoutingConfidence()
        let sendResult = store.routingConfidencePromptSendResult(for: skill)
        try expectEqual(sendResult?.success, true, "Confirmed routing prompt should store provider result.")
        try expectFalse(sendResult?.writeBackAllowed ?? true, "Routing prompt output must not enable write-back.")
        try expectFalse(sendResult?.scriptExecutionAllowed ?? true, "Routing prompt output must not enable script execution.")

        let calls = fake.calls()
        try expectContains(calls, "\"request_kind\":\"routing_confidence\"", "Routing prompt preview should identify the routing_confidence request kind.")
        try expectContains(calls, "\"task_text\":\"Route a local audit release note task.\"", "Routing prompt preview/confirm should include the selected task text.")
        try expectContains(calls, "\"user_intent\":\"Route a local audit release note task.\"", "Routing prompt preview/confirm should include the user intent required by the service.")
        try expectFalse(countOccurrences("\"request_kind\":\"routing_confidence\"", in: calls) < 2, "Routing preview and confirmation should both carry the routing request kind.")
        try expectFalse(countOccurrences("\"user_intent\":\"Route a local audit release note task.\"", in: calls) < 2, "Routing preview and confirmation should both carry user intent.")
        try expectContains(calls, "\"preview_id\":\"routing-preview-beta\"", "Routing confirm should send the routing preview id.")
        try expectContains(calls, "llm.confirmPromptAndSend", "Configured routing provider path should require explicit confirmation.")
        try expectFalse(calls.contains("config.toggleSkill"), "Routing confidence must not call write paths.")
        try expectFalse(calls.contains("script.execute"), "Routing confidence must not call execution paths.")
    }

    private func routingConfidencePromptSendRequiresConfiguredProvider() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "llm-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.routingConfidenceText = "Route a local audit release note task."
        await store.reload()
        await store.rankSelectedSkillRoutes()

        guard let skill = store.selectedSkill else {
            throw NativeModelTestFailure(description: "Fixture should select beta for routing confidence gating.")
        }

        await store.previewPromptForSelectedRoutingConfidence()
        try expectEqual(store.canSendRoutingConfidencePrompt(for: skill), false, "Unconfigured provider should keep routing Confirm & Send disabled.")

        await store.confirmPromptForSelectedRoutingConfidence()
        let sendResult = store.routingConfidencePromptSendResult(for: skill)
        try expectEqual(sendResult?.success, false, "Blocked routing send should store an unavailable result, not throw.")
        try expectContains(sendResult?.message, "Configure and save", "Blocked routing send should explain provider setup.")

        let calls = fake.calls()
        try expectContains(calls, "task.rankSkillRoutes", "No-provider path should still allow local route ranking.")
        try expectContains(calls, "llm.previewPrompt", "No-provider path may ask for a blocked routing preview.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "No-provider routing path must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "No-provider routing path must not call write paths.")
        try expectFalse(calls.contains("script.execute"), "No-provider routing path must not call execution paths.")
    }

    private func taskBenchmarkUsesLocalServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.routingConfidenceText = "Route a local audit release note task."
        await store.reload()
        await store.loadTaskBenchmarks()
        await store.saveSelectedTaskBenchmark()
        await store.evaluateTaskBenchmarks()

        try expectEqual(store.taskBenchmarkList.benchmarks.first?.id, "bench-2", "Saved benchmark should be inserted at the front of the local list.")
        try expectEqual(store.taskBenchmarkList.benchmarks.first?.expectedSkill?.name, "Beta", "Saved benchmark should retain selected expected skill context.")
        try expectEqual(store.taskBenchmarkEvaluation?.evaluatedCount, 2, "Benchmark evaluation should expose evaluated count.")
        try expectEqual(store.taskBenchmarkEvaluation?.matchedCount, 1, "Benchmark evaluation should expose expected matches.")
        try expectEqual(store.taskBenchmarkEvaluation?.acceptableCount, 2, "Benchmark evaluation should expose acceptable matches.")
        try expectEqual(store.taskBenchmarkEvaluation?.evaluations.first?.topRoute?.name, "Beta", "Benchmark evaluation should expose top route.")
        try expectFalse(store.taskBenchmarkEvaluation?.safety.providerRequestSent ?? true, "Local benchmark evaluation must not send a provider request.")
        try expectFalse(store.taskBenchmarkEvaluation?.safety.writeBackAllowed ?? true, "Benchmark evaluation must not allow write-back.")
        try expectFalse(store.taskBenchmarkEvaluation?.safety.scriptExecutionAllowed ?? true, "Benchmark evaluation must not allow script execution.")
        try expectFalse(store.taskBenchmarkEvaluation?.safety.configMutationAllowed ?? true, "Benchmark evaluation must not mutate config.")
        try expectFalse(store.taskBenchmarkEvaluation?.safety.credentialAccessed ?? true, "Benchmark evaluation must not access credentials.")

        let calls = fake.calls()
        try expectContains(calls, "task.listBenchmarks", "Benchmark UI should list benchmarks through the V2.46 task list method.")
        try expectContains(calls, "task.saveBenchmark", "Benchmark UI should save benchmarks through the V2.46 task save method.")
        try expectContains(calls, "task.evaluateBenchmarks", "Benchmark UI should evaluate benchmarks through the V2.46 task evaluation method.")
        try expectContains(calls, "\"task\":\"Route a local audit release note task.\"", "Benchmark save should use the current routing/readiness task text.")
        try expectContains(calls, "\"expected_skill_refs\":[\"beta\",\"def.beta\"]", "Benchmark save should carry selected expected skill references.")
        try expectContains(calls, "\"expected_skill_names\":[\"Beta\"]", "Benchmark save should carry selected expected skill name.")
        try expectContains(calls, "\"acceptable_agents\":[\"claude-code\"]", "Benchmark save should carry acceptable agent context.")
        try expectContains(calls, "\"acceptable_scopes\":[\"agent-project\"]", "Benchmark save should carry acceptable scope context.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Local benchmark evaluation must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Local benchmark evaluation must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Benchmark flow must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Benchmark flow must not call execution paths.")
    }

    private func taskBenchmarkFallsBackWhenMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "old-service")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.taskBenchmarkText = "Route a local audit release note task."
        await store.reload()
        await store.loadTaskBenchmarks()
        await store.saveSelectedTaskBenchmark()
        await store.evaluateTaskBenchmarks()

        try expectEqual(store.taskBenchmarkList.isUnavailable, true, "Unknown benchmark methods should expose an unavailable benchmark list.")
        try expectEqual(store.taskBenchmarkEvaluation?.isUnavailable, true, "Unknown benchmark evaluation should expose an unavailable result.")
        try expectContains(store.taskBenchmarkList.fallbackReason, "unavailable", "Unavailable benchmark list should expose quiet fallback reason.")

        let calls = fake.calls()
        try expectContains(calls, "task.listBenchmarks", "Fallback test should still attempt the V2.46 list method.")
        try expectContains(calls, "task.saveBenchmark", "Fallback test should still attempt the V2.46 save method.")
        try expectContains(calls, "task.evaluateBenchmarks", "Fallback test should still attempt the V2.46 evaluate method.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Unavailable benchmark flow must not fall back to provider prompt preview.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Unavailable benchmark flow must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Unavailable benchmark flow must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Unavailable benchmark flow must not call execution paths.")
    }

    private func routingRegressionUsesLocalBenchmarkServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.loadTaskBenchmarks()
        let snapshotCallsBeforeRegression = countOccurrences("snapshot.", in: fake.calls())
        await store.saveRoutingBaseline()
        await store.detectRoutingRegression()

        try expectEqual(store.routingRegressionBaseline?.baselineID, "baseline-1", "Routing regression baseline should expose saved baseline id.")
        try expectEqual(store.routingRegressionBaseline?.benchmarkCount, 1, "Routing baseline should use loaded benchmark scope.")
        try expectEqual(store.routingRegressionDetection?.regressionCount, 1, "Routing regression detection should expose regression count.")
        try expectEqual(store.routingRegressionDetection?.regressions.first?.currentTopRoute?.name, "Alpha", "Routing regression should expose changed top route.")
        try expectEqual(store.routingRegressionDetection?.regressions.first?.scoreDelta, -16, "Routing regression should expose score delta.")
        try expectFalse(store.routingRegressionDetection?.safety.providerRequestSent ?? true, "Local regression detection must not send a provider request.")
        try expectFalse(store.routingRegressionDetection?.safety.writeBackAllowed ?? true, "Regression detection must not allow write-back.")
        try expectFalse(store.routingRegressionDetection?.safety.scriptExecutionAllowed ?? true, "Regression detection must not allow script execution.")
        try expectFalse(store.routingRegressionDetection?.safety.configMutationAllowed ?? true, "Regression detection must not mutate config.")
        try expectFalse(store.routingRegressionDetection?.safety.snapshotCreated ?? true, "Regression detection must not create snapshots.")
        try expectFalse(store.routingRegressionDetection?.safety.credentialAccessed ?? true, "Regression detection must not access credentials.")

        let calls = fake.calls()
        try expectContains(calls, "task.listBenchmarks", "Regression flow may load the app-local benchmark set.")
        try expectContains(calls, "task.saveRoutingBaseline", "Regression flow should save baseline through the V2.47 baseline method.")
        try expectContains(calls, "task.detectRoutingRegression", "Regression flow should detect regressions through the V2.47 detection method.")
        try expectContains(calls, "\"benchmark_ids\":[\"bench-1\"]", "Regression flow should pass loaded benchmark ids to local regression methods.")
        try expectFalse(calls.contains("task.evaluateBenchmarks"), "Regression flow should not need benchmark evaluation service calls unless the user asks.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Local regression detection must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Local regression detection must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Regression flow must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Regression flow must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforeRegression, "Regression flow must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Regression flow must not call credential paths.")
    }

    private func routingRegressionFallsBackWhenMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "old-service")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        let snapshotCallsBeforeRegression = countOccurrences("snapshot.", in: fake.calls())
        await store.saveRoutingBaseline()
        await store.detectRoutingRegression()

        try expectEqual(store.routingRegressionBaseline?.isUnavailable, true, "Unknown baseline method should expose unavailable baseline status.")
        try expectEqual(store.routingRegressionDetection?.isUnavailable, true, "Unknown detection method should expose unavailable detection status.")
        try expectContains(store.routingRegressionDetection?.fallbackReason, "unavailable", "Unavailable regression detection should expose quiet fallback reason.")

        let calls = fake.calls()
        try expectContains(calls, "task.saveRoutingBaseline", "Fallback test should still attempt the V2.47 baseline method.")
        try expectContains(calls, "task.detectRoutingRegression", "Fallback test should still attempt the V2.47 detection method.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Unavailable regression flow must not fall back to provider prompt preview.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Unavailable regression flow must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Unavailable regression flow must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Unavailable regression flow must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforeRegression, "Unavailable regression flow must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Unavailable regression flow must not call credential paths.")
    }

    private func agentTraceImportUsesLocalServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.traceImportTitle = "Local trace"
        store.traceImportTask = "Route a local audit release note task."
        store.traceImportExpectedSkills = "Beta, Alpha"
        store.traceImportText = "raw local transcript should be sent once"
        await store.reload()
        await store.loadTraceImports()
        await store.importLocalTrace()

        try expectEqual(store.traceImportList.imports.first?.id, "trace-2", "Imported trace should be inserted at the front of the trace list.")
        try expectEqual(store.traceImportResult?.record?.outcome, "wrong_pick", "Trace import should expose imported outcome.")
        try expectEqual(store.latestTraceImportRecord?.redactedExcerpt, "User asked for <project-root> release notes. Assistant selected Alpha.", "Trace result should expose redacted excerpt metadata.")
        try expectEqual(store.traceImportText, "", "Successful import should clear raw pasted trace text from UI state.")
        try expectFalse(store.traceImportResult?.record?.safety.providerRequestSent ?? true, "Local trace import must not send a provider request.")
        try expectFalse(store.traceImportResult?.record?.safety.writeBackAllowed ?? true, "Trace import must not allow write-back.")
        try expectFalse(store.traceImportResult?.record?.safety.scriptExecutionAllowed ?? true, "Trace import must not allow script execution.")
        try expectFalse(store.traceImportResult?.record?.safety.configMutationAllowed ?? true, "Trace import must not mutate config.")
        try expectFalse(store.traceImportResult?.record?.safety.credentialAccessed ?? true, "Trace import must not access credentials.")

        guard let listedRecord = store.traceImportList.imports.last else {
            throw NativeModelTestFailure(description: "Trace list should include the pre-existing trace.")
        }
        await store.deleteTraceImport(listedRecord)
        try expectEqual(store.traceImportDeleteResult?.deleted, true, "Trace delete should expose deletion success.")

        let calls = fake.calls()
        try expectContains(calls, "trace.listImports", "Trace UI should list imports through the V2.48 list method.")
        try expectContains(calls, "trace.importLocal", "Trace UI should import local traces through the V2.48 import method.")
        try expectContains(calls, "trace.deleteImport", "Trace UI should delete imports through the V2.48 delete method.")
        try expectContains(calls, "\"trace_text\":\"raw local transcript should be sent once\"", "Trace import should send the pasted trace text to the local service.")
        try expectContains(calls, "\"title\":\"Local trace\"", "Trace import should send optional title metadata.")
        try expectContains(calls, "\"task\":\"Route a local audit release note task.\"", "Trace import should send optional task metadata.")
        try expectContains(calls, "\"expected_skill_names\":[\"Beta\",\"Alpha\"]", "Trace import should send expected skill names.")
        try expectContains(calls, "\"candidate_instance_ids\":[\"beta\"]", "Trace import should include selected skill context.")
        try expectContains(calls, "\"agent\":\"claude-code\"", "Trace import should include selected agent context.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Local trace import must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Local trace import must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Trace import flow must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Trace import flow must not call execution paths.")
    }

    private func agentTraceImportFallsBackWhenMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "old-service")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.traceImportText = "raw local transcript"
        await store.reload()
        await store.loadTraceImports()
        await store.importLocalTrace()

        try expectEqual(store.traceImportList.isUnavailable, true, "Unknown trace list method should expose unavailable trace list.")
        try expectEqual(store.traceImportResult?.isUnavailable, true, "Unknown trace import method should expose unavailable trace result.")
        try expectContains(store.traceImportResult?.fallbackReason, "unavailable", "Unavailable trace import should expose quiet fallback reason.")
        try expectEqual(store.traceImportText, "raw local transcript", "Failed import should keep raw pasted input for user correction.")

        let calls = fake.calls()
        try expectContains(calls, "trace.listImports", "Fallback test should still attempt the V2.48 list method.")
        try expectContains(calls, "trace.importLocal", "Fallback test should still attempt the V2.48 import method.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Unavailable trace flow must not fall back to provider prompt preview.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Unavailable trace flow must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Unavailable trace flow must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Unavailable trace flow must not call execution paths.")
    }

    private func routingAccuracyDashboardUsesReadOnlyServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        let snapshotCallsBeforeDashboard = countOccurrences("snapshot.", in: fake.calls())
        await store.loadRoutingAccuracyDashboard()

        let dashboard = store.routingAccuracyDashboard
        try expectEqual(dashboard?.generatedBy, "local-v2.49", "Routing accuracy dashboard should expose generator metadata.")
        try expectEqual(dashboard?.summary.hitCount, 7, "Routing accuracy dashboard should expose hit count.")
        try expectEqual(dashboard?.summary.wrongPickCount, 1, "Routing accuracy dashboard should expose wrong-pick count.")
        try expectEqual(dashboard?.agents.first?.agent, "claude-code", "Routing accuracy dashboard should expose per-agent rows.")
        try expectEqual(dashboard?.gaps.first?.title, "Missing trace coverage", "Routing accuracy dashboard should expose gaps.")
        try expectFalse(dashboard?.safetyFlags.providerRequestSent ?? true, "Routing accuracy must not send provider requests.")
        try expectFalse(dashboard?.safetyFlags.writeBackAllowed ?? true, "Routing accuracy must not allow write-back.")
        try expectFalse(dashboard?.safetyFlags.scriptExecutionAllowed ?? true, "Routing accuracy must not allow script execution.")
        try expectFalse(dashboard?.safetyFlags.configMutationAllowed ?? true, "Routing accuracy must not mutate config.")
        try expectFalse(dashboard?.safetyFlags.snapshotCreated ?? true, "Routing accuracy must not create snapshots.")
        try expectFalse(dashboard?.safetyFlags.triageMutationAllowed ?? true, "Routing accuracy must not mutate triage.")
        try expectFalse(dashboard?.safetyFlags.credentialAccessed ?? true, "Routing accuracy must not access credentials.")
        try expectFalse(dashboard?.safetyFlags.rawPromptPersisted ?? true, "Routing accuracy must not persist raw prompts.")
        try expectFalse(dashboard?.safetyFlags.rawResponsePersisted ?? true, "Routing accuracy must not persist raw responses.")
        try expectFalse(dashboard?.safetyFlags.rawTracePersisted ?? true, "Routing accuracy must not persist raw traces.")
        try expectFalse(dashboard?.safetyFlags.cloudSyncEnabled ?? true, "Routing accuracy must not sync cloud data.")
        try expectFalse(dashboard?.safetyFlags.telemetryEnabled ?? true, "Routing accuracy must not emit telemetry.")
        try expectFalse(store.isLoadingRoutingAccuracyDashboard, "Routing accuracy load should reset loading state.")

        let calls = fake.calls()
        try expectContains(calls, "routing.accuracyDashboard", "Routing accuracy UI should call the V2.49 dashboard method.")
        try expectContains(calls, "\"agent\":\"claude-code\"", "Routing accuracy should pass the current agent filter.")
        try expectContains(calls, "\"window_days\":30", "Routing accuracy should pass the dashboard window.")
        try expectContains(calls, "\"limit\":20", "Routing accuracy should pass the dashboard limit.")
        try expectContains(calls, "\"include_history\":true", "Routing accuracy should request history explicitly.")
        try expectContains(calls, "\"include_recent_evidence\":true", "Routing accuracy should request recent evidence explicitly.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Routing accuracy dashboard must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Routing accuracy dashboard must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Routing accuracy dashboard must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Routing accuracy dashboard must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforeDashboard, "Routing accuracy dashboard must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Routing accuracy dashboard must not call credential paths.")
    }

    private func staleDriftDetectionUsesReadOnlyServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        let snapshotCallsBeforeDetect = countOccurrences("snapshot.", in: fake.calls())
        await store.detectStaleDrift()

        let result = store.staleDriftDetection
        try expectEqual(result?.generatedBy, "local-v2.51", "Stale drift detection should expose generator metadata.")
        try expectEqual(result?.summary.staleCount, 2, "Stale drift detection should expose stale count.")
        try expectEqual(result?.summary.driftCount, 1, "Stale drift detection should expose drift count.")
        try expectEqual(result?.staleDriftRows.first?.title, "Beta appears stale", "Stale drift detection should expose candidate rows.")
        try expectEqual(result?.staleDriftRows.first?.skill?.name, "Beta", "Stale drift detection should expose row skill refs.")
        try expectEqual(result?.readinessImpactRows.first?.title, "Readiness lowered", "Stale drift detection should expose readiness impact rows.")
        try expectEqual(result?.gapIssueRows.first?.title, "Missing fresh evidence", "Stale drift detection should expose gap rows.")
        try expectEqual(result?.evidenceReferences.first?.title, "Catalog freshness", "Stale drift detection should expose evidence references.")
        try expectFalse(result?.safetyFlags.providerRequestSent ?? true, "Stale drift detection must not send provider requests.")
        try expectFalse(result?.safetyFlags.writeBackAllowed ?? true, "Stale drift detection must not allow write-back.")
        try expectFalse(result?.safetyFlags.writeActionsAvailable ?? true, "Stale drift detection must not expose write actions.")
        try expectFalse(result?.safetyFlags.scriptExecutionAllowed ?? true, "Stale drift detection must not allow script execution.")
        try expectFalse(result?.safetyFlags.executionActionsAvailable ?? true, "Stale drift detection must not expose execution actions.")
        try expectFalse(result?.safetyFlags.configMutationAllowed ?? true, "Stale drift detection must not mutate config.")
        try expectFalse(result?.safetyFlags.snapshotCreated ?? true, "Stale drift detection must not create snapshots.")
        try expectFalse(result?.safetyFlags.triageMutationAllowed ?? true, "Stale drift detection must not mutate triage.")
        try expectFalse(result?.safetyFlags.credentialAccessed ?? true, "Stale drift detection must not access credentials.")
        try expectFalse(result?.safetyFlags.rawPromptPersisted ?? true, "Stale drift detection must not persist raw prompts.")
        try expectFalse(result?.safetyFlags.rawResponsePersisted ?? true, "Stale drift detection must not persist raw responses.")
        try expectFalse(result?.safetyFlags.rawTracePersisted ?? true, "Stale drift detection must not persist raw traces.")
        try expectFalse(result?.safetyFlags.cloudSyncEnabled ?? true, "Stale drift detection must not sync cloud data.")
        try expectFalse(result?.safetyFlags.telemetryEnabled ?? true, "Stale drift detection must not emit telemetry.")
        try expectFalse(store.isDetectingStaleDrift, "Stale drift detection should reset loading state.")

        let calls = fake.calls()
        try expectContains(calls, "analysis.detectStaleDrift", "Stale drift UI should call the V2.51 detection method.")
        try expectContains(calls, "\"agent\":\"claude-code\"", "Stale drift detection should pass the current agent filter.")
        try expectContains(calls, "\"limit\":40", "Stale drift detection should pass the detection limit.")
        try expectContains(calls, "\"include_readiness_impact\":true", "Stale drift detection should request readiness impact explicitly.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Stale drift detection must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Stale drift detection must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Stale drift detection must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Stale drift detection must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforeDetect, "Stale drift detection must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Stale drift detection must not call credential paths.")
    }

    private func knowledgeSearchUsesReadOnlyServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.knowledgeSearchText = "release audit"
        await store.reload()
        let snapshotCallsBeforeSearch = countOccurrences("snapshot.", in: fake.calls())
        await store.searchKnowledge()

        let result = store.knowledgeSearchResult
        try expectEqual(result?.generatedBy, "local-v2.52", "Knowledge search should expose generator metadata.")
        try expectEqual(result?.summary.resultCount, 1, "Knowledge search should expose result count.")
        try expectEqual(result?.knowledgeRows.first?.skillName, "Beta", "Knowledge search should expose matched skill rows.")
        try expectEqual(result?.knowledgeRows.first?.tools, ["rg"], "Knowledge search should expose tool metadata.")
        try expectEqual(result?.facetRows.first?.value, "claude-code", "Knowledge search should expose facet rows.")
        try expectEqual(result?.gapNotes.first, "No fresh trace confirms the release audit route.", "Knowledge search should expose gap notes.")
        try expectEqual(result?.evidenceReferences.first?.title, "Knowledge index", "Knowledge search should expose evidence references.")
        try expectFalse(result?.safetyFlags.providerRequestSent ?? true, "Knowledge search must not send provider requests.")
        try expectFalse(result?.safetyFlags.writeBackAllowed ?? true, "Knowledge search must not allow write-back.")
        try expectFalse(result?.safetyFlags.writeActionsAvailable ?? true, "Knowledge search must not expose write actions.")
        try expectFalse(result?.safetyFlags.scriptExecutionAllowed ?? true, "Knowledge search must not allow script execution.")
        try expectFalse(result?.safetyFlags.executionActionsAvailable ?? true, "Knowledge search must not expose execution actions.")
        try expectFalse(result?.safetyFlags.configMutationAllowed ?? true, "Knowledge search must not mutate config.")
        try expectFalse(result?.safetyFlags.snapshotCreated ?? true, "Knowledge search must not create snapshots.")
        try expectFalse(result?.safetyFlags.triageMutationAllowed ?? true, "Knowledge search must not mutate triage.")
        try expectFalse(result?.safetyFlags.credentialAccessed ?? true, "Knowledge search must not access credentials.")
        try expectFalse(result?.safetyFlags.rawPromptPersisted ?? true, "Knowledge search must not persist raw prompts.")
        try expectFalse(result?.safetyFlags.rawResponsePersisted ?? true, "Knowledge search must not persist raw responses.")
        try expectFalse(result?.safetyFlags.rawTracePersisted ?? true, "Knowledge search must not persist raw traces.")
        try expectFalse(result?.safetyFlags.cloudSyncEnabled ?? true, "Knowledge search must not sync cloud data.")
        try expectFalse(result?.safetyFlags.telemetryEnabled ?? true, "Knowledge search must not emit telemetry.")
        try expectFalse(store.isSearchingKnowledge, "Knowledge search should reset loading state.")

        let calls = fake.calls()
        try expectContains(calls, "knowledge.search", "Knowledge UI should call the V2.52 search method.")
        try expectContains(calls, "\"query\":\"release audit\"", "Knowledge search should pass the user query.")
        try expectContains(calls, "\"agent\":\"claude-code\"", "Knowledge search should pass the current agent filter.")
        try expectContains(calls, "\"limit\":20", "Knowledge search should pass the search limit.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Knowledge search must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Knowledge search must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Knowledge search must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Knowledge search must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforeSearch, "Knowledge search must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Knowledge search must not call credential paths.")
    }

    private func localSkillMapUsesReadOnlyServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        let snapshotCallsBeforeMap = countOccurrences("snapshot.", in: fake.calls())
        await store.buildLocalSkillMap()

        let result = store.localSkillMapResult
        try expectEqual(result?.generatedBy, "local-v2.63", "Local skill map should expose generator metadata.")
        try expectEqual(result?.summary.nodeCount, 2, "Local skill map should expose node count.")
        try expectEqual(result?.summary.edgeCount, 1, "Local skill map should expose edge count.")
        try expectEqual(result?.summary.clusterCount, 1, "Local skill map should expose cluster count.")
        try expectEqual(result?.selectedSkill?.skillName, "Beta", "Local skill map should expose selected skill context.")
        try expectEqual(result?.nodes.first?.label, "Beta", "Local skill map should expose key nodes.")
        try expectEqual(result?.edges.first?.relation, "similar-purpose", "Local skill map should expose edge relations.")
        try expectEqual(result?.clusters.first?.title, "Release audit", "Local skill map should expose clusters.")
        try expectEqual(result?.gapRows.first?.detail, "No Codex project route.", "Local skill map should expose gap rows.")
        try expectEqual(result?.evidenceReferences.first?.title, "Local skill map", "Local skill map should expose evidence references.")
        try expectFalse(result?.safetyFlags.providerRequestSent ?? true, "Local skill map must not send provider requests.")
        try expectFalse(result?.safetyFlags.writeBackAllowed ?? true, "Local skill map must not allow write-back.")
        try expectFalse(result?.safetyFlags.writeActionsAvailable ?? true, "Local skill map must not expose write actions.")
        try expectFalse(result?.safetyFlags.scriptExecutionAllowed ?? true, "Local skill map must not allow script execution.")
        try expectFalse(result?.safetyFlags.executionActionsAvailable ?? true, "Local skill map must not expose execution actions.")
        try expectFalse(result?.safetyFlags.configMutationAllowed ?? true, "Local skill map must not mutate config.")
        try expectFalse(result?.safetyFlags.snapshotCreated ?? true, "Local skill map must not create snapshots.")
        try expectFalse(result?.safetyFlags.triageMutationAllowed ?? true, "Local skill map must not mutate triage.")
        try expectFalse(result?.safetyFlags.credentialAccessed ?? true, "Local skill map must not access credentials.")
        try expectFalse(result?.safetyFlags.rawPromptPersisted ?? true, "Local skill map must not persist raw prompts.")
        try expectFalse(result?.safetyFlags.rawResponsePersisted ?? true, "Local skill map must not persist raw responses.")
        try expectFalse(result?.safetyFlags.rawTracePersisted ?? true, "Local skill map must not persist raw traces.")
        try expectFalse(result?.safetyFlags.cloudSyncEnabled ?? true, "Local skill map must not sync cloud data.")
        try expectFalse(result?.safetyFlags.telemetryEnabled ?? true, "Local skill map must not emit telemetry.")
        try expectFalse(store.isBuildingLocalSkillMap, "Local skill map should reset loading state.")

        let calls = fake.calls()
        try expectContains(calls, "knowledge.buildLocalSkillMap", "Local skill map should call the V2.63 map method.")
        try expectContains(calls, "\"agent\":\"claude-code\"", "Local skill map should pass the current agent filter.")
        try expectContains(calls, "\"selected_skill_id\":\"beta\"", "Local skill map should pass selected skill id.")
        try expectContains(calls, "\"selected_skill_name\":\"Beta\"", "Local skill map should pass selected skill name.")
        try expectContains(calls, "\"selected_skill_agent\":\"claude-code\"", "Local skill map should pass selected skill agent.")
        try expectContains(calls, "\"project_root\":\"\\/tmp\\/project\"", "Local skill map should pass active project root.")
        try expectContains(calls, "\"current_cwd\":\"\\/tmp\\/project\"", "Local skill map should pass active project cwd.")
        try expectContains(calls, "\"workspace\":\"Fixture Project\"", "Local skill map should pass active workspace name.")
        try expectContains(calls, "\"limit\":30", "Local skill map should pass map limit.")
        try expectContains(calls, "\"include_edges\":true", "Local skill map should request edges.")
        try expectContains(calls, "\"include_clusters\":true", "Local skill map should request clusters.")
        try expectContains(calls, "\"include_evidence\":true", "Local skill map should request evidence rows.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Local skill map must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Local skill map must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Local skill map must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Local skill map must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforeMap, "Local skill map must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Local skill map must not call credential paths.")
    }

    private func localSkillMapFallsBackWhenMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.buildLocalSkillMap()

        try expectEqual(store.localSkillMapResult?.isUnavailable, true, "Local skill map should expose unavailable fallback for older services.")
        try expectEqual(store.localSkillMapResult?.fallbackReason, UIStrings.localSkillMapUnavailable, "Unknown method fallback should use the localized unavailable copy.")
        try expectFalse(store.isBuildingLocalSkillMap, "Unavailable local skill map should reset loading state.")
        try expectContains(fake.calls(), "knowledge.buildLocalSkillMap", "Fallback should still prove the intended V2.63 method was attempted.")
    }

    private func skillLifecycleTimelineUsesReadOnlyServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        let snapshotCallsBeforeTimeline = countOccurrences("snapshot.", in: fake.calls())
        await store.loadSkillLifecycleTimeline()

        let result = store.skillLifecycleTimelineResult
        try expectEqual(result?.generatedBy, "local-v2.66", "Skill lifecycle timeline should expose generator metadata.")
        try expectEqual(result?.summary.eventCount, 3, "Skill lifecycle timeline should expose event count.")
        try expectEqual(result?.summary.skillCount, 1, "Skill lifecycle timeline should expose skill aggregate count.")
        try expectEqual(result?.summary.agentCount, 1, "Skill lifecycle timeline should expose agent aggregate count.")
        try expectEqual(result?.timelineRows.first?.title, "Beta loaded from project root", "Skill lifecycle timeline should expose timeline rows.")
        try expectEqual(result?.timelineRows.first?.eventType, "scan.detected", "Skill lifecycle timeline should expose event type.")
        try expectEqual(result?.timelineRows.first?.lifecycleStage, "discovered", "Skill lifecycle timeline should expose lifecycle stage.")
        try expectEqual(result?.skillRows.first?.skillName, "Beta", "Skill lifecycle timeline should expose skill aggregate rows.")
        try expectEqual(result?.skillRows.first?.count, 3, "Skill lifecycle timeline should expose aggregate counts.")
        try expectEqual(result?.agentRows.first?.agent, "claude-code", "Skill lifecycle timeline should expose agent aggregate rows.")
        try expectEqual(result?.gapNotes.first, "No Codex lifecycle evidence for this selected skill.", "Skill lifecycle timeline should expose gap notes.")
        try expectEqual(result?.blockerNotes.first, "Lifecycle timeline is read-only and does not create snapshots.", "Skill lifecycle timeline should expose blocker notes.")
        try expectEqual(result?.evidenceReferences.first?.source, "skill.lifecycleTimeline", "Skill lifecycle timeline should expose evidence references.")
        try expectEqual(result?.promptRequest?.requestKind, "skill_lifecycle_timeline", "Skill lifecycle timeline should expose prompt metadata.")
        try expectFalse(result?.safetyFlags.providerRequestSent ?? true, "Skill lifecycle timeline must not send provider requests.")
        try expectFalse(result?.safetyFlags.writeBackAllowed ?? true, "Skill lifecycle timeline must not allow write-back.")
        try expectFalse(result?.safetyFlags.writeActionsAvailable ?? true, "Skill lifecycle timeline must not expose write actions.")
        try expectFalse(result?.safetyFlags.scriptExecutionAllowed ?? true, "Skill lifecycle timeline must not allow script execution.")
        try expectFalse(result?.safetyFlags.executionActionsAvailable ?? true, "Skill lifecycle timeline must not expose execution actions.")
        try expectFalse(result?.safetyFlags.configMutationAllowed ?? true, "Skill lifecycle timeline must not mutate config.")
        try expectFalse(result?.safetyFlags.snapshotCreated ?? true, "Skill lifecycle timeline must not create snapshots.")
        try expectFalse(result?.safetyFlags.triageMutationAllowed ?? true, "Skill lifecycle timeline must not mutate triage.")
        try expectFalse(result?.safetyFlags.credentialAccessed ?? true, "Skill lifecycle timeline must not access credentials.")
        try expectFalse(result?.safetyFlags.rawPromptPersisted ?? true, "Skill lifecycle timeline must not persist raw prompts.")
        try expectFalse(result?.safetyFlags.rawResponsePersisted ?? true, "Skill lifecycle timeline must not persist raw responses.")
        try expectFalse(result?.safetyFlags.rawTracePersisted ?? true, "Skill lifecycle timeline must not persist raw traces.")
        try expectFalse(result?.safetyFlags.cloudSyncEnabled ?? true, "Skill lifecycle timeline must not sync cloud data.")
        try expectFalse(result?.safetyFlags.telemetryEnabled ?? true, "Skill lifecycle timeline must not emit telemetry.")
        try expectFalse(store.isLoadingSkillLifecycleTimeline, "Skill lifecycle timeline should reset loading state.")

        let calls = fake.calls()
        try expectContains(calls, "skill.lifecycleTimeline", "Skill lifecycle timeline should call the V2.66 lifecycle method.")
        try expectContains(calls, "\"agent\":\"claude-code\"", "Skill lifecycle timeline should pass the current agent filter.")
        try expectContains(calls, "\"selected_skill_id\":\"beta\"", "Skill lifecycle timeline should pass selected skill id.")
        try expectContains(calls, "\"selected_skill_name\":\"Beta\"", "Skill lifecycle timeline should pass selected skill name.")
        try expectContains(calls, "\"selected_skill_agent\":\"claude-code\"", "Skill lifecycle timeline should pass selected skill agent.")
        try expectContains(calls, "\"candidate_instance_ids\":[\"beta\"]", "Skill lifecycle timeline should include selected skill candidate context.")
        try expectContains(calls, "\"project_root\":\"\\/tmp\\/project\"", "Skill lifecycle timeline should pass active project root.")
        try expectContains(calls, "\"current_cwd\":\"\\/tmp\\/project\"", "Skill lifecycle timeline should pass active project cwd.")
        try expectContains(calls, "\"workspace\":\"Fixture Project\"", "Skill lifecycle timeline should pass active workspace name.")
        try expectContains(calls, "\"limit\":20", "Skill lifecycle timeline should pass timeline limit.")
        try expectContains(calls, "\"include_skill_rows\":true", "Skill lifecycle timeline should request skill aggregates.")
        try expectContains(calls, "\"include_agent_rows\":true", "Skill lifecycle timeline should request agent aggregates.")
        try expectContains(calls, "\"include_evidence\":true", "Skill lifecycle timeline should request evidence rows.")
        try expectContains(calls, "\"include_safety_flags\":true", "Skill lifecycle timeline should request safety flags.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Skill lifecycle timeline must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Skill lifecycle timeline must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Skill lifecycle timeline must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Skill lifecycle timeline must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforeTimeline, "Skill lifecycle timeline must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Skill lifecycle timeline must not call credential paths.")
    }

    private func skillLifecycleTimelineFallsBackWhenMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.loadSkillLifecycleTimeline()

        try expectEqual(store.skillLifecycleTimelineResult?.isUnavailable, true, "Skill lifecycle timeline should expose unavailable fallback for older services.")
        try expectEqual(store.skillLifecycleTimelineResult?.fallbackReason, UIStrings.skillLifecycleTimelineUnavailable, "Unknown method fallback should use the localized unavailable copy.")
        try expectFalse(store.isLoadingSkillLifecycleTimeline, "Unavailable lifecycle timeline should reset loading state.")
        try expectContains(fake.calls(), "skill.lifecycleTimeline", "Fallback should still prove the intended V2.66 method was attempted.")
        try expectFalse(fake.calls().contains("llm.previewPrompt"), "Unavailable lifecycle timeline must not fall back to provider prompt preview.")
        try expectFalse(fake.calls().contains("llm.confirmPromptAndSend"), "Unavailable lifecycle timeline must not send to provider.")
        try expectFalse(fake.calls().contains("config.toggleSkill"), "Unavailable lifecycle timeline must not call config write paths.")
        try expectFalse(fake.calls().contains("script.execute"), "Unavailable lifecycle timeline must not call execution paths.")
    }

    private func providerObservabilityUsesReadOnlyServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        let snapshotCallsBeforeObservability = countOccurrences("snapshot.", in: fake.calls())
        await store.loadProviderObservability()

        let result = store.providerObservabilityResult
        try expectEqual(result?.generatedBy, "local-v2.64", "Provider observability should expose generator metadata.")
        try expectEqual(result?.summary.callCount, 3, "Provider observability should expose call count.")
        try expectEqual(result?.summary.successCount, 1, "Provider observability should expose success count.")
        try expectEqual(result?.summary.failureCount, 1, "Provider observability should expose failure count.")
        try expectEqual(result?.summary.blockedCount, 1, "Provider observability should expose blocked count.")
        try expectEqual(result?.summary.estimatedTotalTokens, 1300, "Provider observability should expose token estimates.")
        try expectEqual(result?.callRows.first?.requestKind, "task_readiness", "Provider observability should expose recent call rows.")
        try expectEqual(result?.providerRows.first?.label, "OpenAI-compatible", "Provider observability should expose provider rows.")
        try expectEqual(result?.modelRows.first?.label, "gpt-5", "Provider observability should expose model rows.")
        try expectEqual(result?.destinationRows.first?.destinationHost, "llm.example.com", "Provider observability should expose destination rows.")
        try expectEqual(result?.modelTaskHistoryRows.first?.title, "Release audit model fit", "Provider observability should expose model-task history rows.")
        try expectEqual(result?.errorRows.first?.title, "Timeout", "Provider observability should expose error rows.")
        try expectEqual(result?.budgetHints.first?.title, "Monthly budget healthy", "Provider observability should expose budget hints.")
        try expectEqual(result?.retentionRows.first?.title, "Retain metadata only", "Provider observability should expose retention rows.")
        try expectEqual(result?.cleanupRecommendationRows.first?.title, "No cleanup required", "Provider observability should expose cleanup rows.")
        try expectEqual(result?.evidenceReferences.first?.title, "Prompt run history", "Provider observability should expose evidence references.")
        try expectEqual(result?.promptRequest?.requestKind, "provider_observability", "Provider observability should expose prompt metadata.")
        try expectFalse(result?.safetyFlags.providerRequestSent ?? true, "Provider observability must not send provider requests.")
        try expectFalse(result?.safetyFlags.writeBackAllowed ?? true, "Provider observability must not allow write-back.")
        try expectFalse(result?.safetyFlags.writeActionsAvailable ?? true, "Provider observability must not expose write actions.")
        try expectFalse(result?.safetyFlags.scriptExecutionAllowed ?? true, "Provider observability must not allow script execution.")
        try expectFalse(result?.safetyFlags.executionActionsAvailable ?? true, "Provider observability must not expose execution actions.")
        try expectFalse(result?.safetyFlags.configMutationAllowed ?? true, "Provider observability must not mutate config.")
        try expectFalse(result?.safetyFlags.snapshotCreated ?? true, "Provider observability must not create snapshots.")
        try expectFalse(result?.safetyFlags.triageMutationAllowed ?? true, "Provider observability must not mutate triage.")
        try expectFalse(result?.safetyFlags.credentialAccessed ?? true, "Provider observability must not access credentials.")
        try expectFalse(result?.safetyFlags.rawPromptPersisted ?? true, "Provider observability must not persist raw prompts.")
        try expectFalse(result?.safetyFlags.rawResponsePersisted ?? true, "Provider observability must not persist raw responses.")
        try expectFalse(result?.safetyFlags.rawTracePersisted ?? true, "Provider observability must not persist raw traces.")
        try expectFalse(result?.safetyFlags.cloudSyncEnabled ?? true, "Provider observability must not sync cloud data.")
        try expectFalse(result?.safetyFlags.telemetryEnabled ?? true, "Provider observability must not emit telemetry.")
        try expectFalse(result?.safetyFlags.rawSecretReturned ?? true, "Provider observability must not expose raw secrets.")
        try expectFalse(store.isLoadingProviderObservability, "Provider observability should reset loading state.")

        let calls = fake.calls()
        try expectContains(calls, "llm.providerObservability", "Provider observability should call the V2.64 observability method.")
        try expectContains(calls, "\"window_days\":30", "Provider observability should pass the dashboard window.")
        try expectContains(calls, "\"limit\":30", "Provider observability should pass the dashboard limit.")
        try expectContains(calls, "\"include_history\":true", "Provider observability should request history rows.")
        try expectContains(calls, "\"include_budget_hints\":true", "Provider observability should request budget hints.")
        try expectContains(calls, "\"include_retention_recommendations\":true", "Provider observability should request retention recommendations.")
        try expectContains(calls, "\"include_evidence\":true", "Provider observability should request evidence rows.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Provider observability must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Provider observability must not send to provider.")
        try expectFalse(calls.contains("llm.recordModelTaskMatch"), "Provider observability UI must not write model-task history.")
        try expectFalse(calls.contains("llm.deleteModelTaskMatch"), "Provider observability UI must not delete model-task history.")
        try expectFalse(calls.contains("config.toggleSkill"), "Provider observability must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Provider observability must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforeObservability, "Provider observability must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Provider observability must not call credential paths.")
    }

    private func providerObservabilityNeedBasedLoadUsesCacheUntilManualRefresh() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()

        try expectEqual(countMethodCalls("llm.providerObservability", in: fake.calls()), 0, "Startup/reload should not build provider observability automatically.")

        await store.loadProviderObservabilityIfNeeded()

        try expectEqual(countMethodCalls("llm.providerObservability", in: fake.calls()), 1, "Initial need-based observability load should call the local service.")
        try expectEqual(store.providerObservabilityResult?.summary.callCount, 3, "Need-based observability load should keep the decoded dashboard.")

        await store.loadProviderObservabilityIfNeeded()

        try expectEqual(countMethodCalls("llm.providerObservability", in: fake.calls()), 1, "Need-based observability loading should reuse the cached dashboard.")

        await store.loadProviderObservability()

        try expectEqual(countMethodCalls("llm.providerObservability", in: fake.calls()), 2, "Manual observability refresh should force a fresh local service request.")
    }

    private func providerObservabilityFallsBackWhenMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.loadProviderObservability()

        try expectEqual(store.providerObservabilityResult?.isUnavailable, true, "Provider observability should expose unavailable fallback for older services.")
        try expectEqual(store.providerObservabilityResult?.fallbackReason, UIStrings.providerObservabilityUnavailable, "Unknown method fallback should use the localized unavailable copy.")
        try expectFalse(store.isLoadingProviderObservability, "Unavailable provider observability should reset loading state.")
        try expectContains(fake.calls(), "llm.providerObservability", "Fallback should still prove the intended V2.64 method was attempted.")
    }

    private func taskCockpitUsesReadOnlyServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let historyStore = makeTemporaryTaskCockpitHistoryStore()
        defer { cleanupTaskCockpitHistoryStore(historyStore) }
        let store = SkillStore(service: fake.serviceClient(), taskCockpitHistoryStore: historyStore)
        store.selectedSkillID = "beta"
        store.taskCockpitText = "Prepare local release audit work."
        await store.reload()
        store.selectedSidebarSelection = .skill("beta")
        let snapshotCallsBeforeCockpit = countOccurrences("snapshot.", in: fake.calls())
        await store.buildTaskCockpit()

        let result = store.taskCockpitResult
        try expectEqual(result?.generatedBy, "provider-task-cockpit", "Task cockpit should expose provider-backed generator metadata.")
        try expectEqual(result?.summary.recommendedAgent, "claude-code", "Task cockpit should expose recommended agent.")
        try expectEqual(result?.summary.recommendedSkillName, "Beta", "Task cockpit should expose recommended skill.")
        try expectEqual(result?.filters.agents, ["claude-code"], "Task cockpit should preserve the selected agent scope.")
        try expectEqual(result?.agentCandidates.first?.agent, "claude-code", "Task cockpit should expose agent candidates.")
        try expectEqual(result?.skillCandidates.first?.skill?.name, "Beta", "Task cockpit should expose skill candidates.")
        try expectEqual(result?.skillCandidates.count, 2, "Task cockpit should expose the top provider-ranked skill candidates.")
        try expectEqual(result?.readinessSignals.first?.title, "Provider readiness", "Task cockpit should expose concise provider process signals.")
        try expectEqual(result?.gapRows.first?.title, "Codex coverage not selected", "Task cockpit should expose gaps.")
        try expectEqual(result?.blockerRows.count, 0, "Task cockpit should expose provider blockers.")
        try expectEqual(result?.safetyFlags.providerRequestSent, true, "Provider-backed task cockpit should mark the provider request as sent.")
        try expectFalse(result?.safetyFlags.writeBackAllowed ?? true, "Task cockpit must not allow write-back.")
        try expectFalse(result?.safetyFlags.writeActionsAvailable ?? true, "Task cockpit must not expose write actions.")
        try expectFalse(result?.safetyFlags.scriptExecutionAllowed ?? true, "Task cockpit must not allow script execution.")
        try expectFalse(result?.safetyFlags.executionActionsAvailable ?? true, "Task cockpit must not expose execution actions.")
        try expectFalse(result?.safetyFlags.configMutationAllowed ?? true, "Task cockpit must not mutate config.")
        try expectFalse(result?.safetyFlags.snapshotCreated ?? true, "Task cockpit must not create snapshots.")
        try expectFalse(result?.safetyFlags.triageMutationAllowed ?? true, "Task cockpit must not mutate triage.")
        try expectFalse(result?.safetyFlags.credentialAccessed ?? true, "Task cockpit must not access credentials.")
        try expectFalse(result?.safetyFlags.rawPromptPersisted ?? true, "Task cockpit must not persist raw prompts.")
        try expectFalse(result?.safetyFlags.rawResponsePersisted ?? true, "Task cockpit must not persist raw responses.")
        try expectFalse(result?.safetyFlags.rawTracePersisted ?? true, "Task cockpit must not persist raw traces.")
        try expectFalse(result?.safetyFlags.cloudSyncEnabled ?? true, "Task cockpit must not sync cloud data.")
        try expectFalse(result?.safetyFlags.telemetryEnabled ?? true, "Task cockpit must not emit telemetry.")
        try expectFalse(store.isBuildingTaskCockpit, "Task cockpit should reset loading state.")
        try expectEqual(store.taskCockpitOperationState.timeoutSeconds, 300, "Task cockpit should use a five minute UI timeout by default.")

        let calls = fake.calls()
        try expectContains(calls, "llm.previewPrompt", "Task cockpit should prepare a provider prompt preview.")
        try expectContains(calls, "llm.confirmPromptAndSend", "Task cockpit should send through the confirmation-gated provider path.")
        try expectContains(calls, "\"timeout_ms\":300000", "Task cockpit provider send should use a five minute request timeout.")
        try expectContains(calls, "\"request_kind\":\"task_cockpit\"", "Task cockpit should use the task_cockpit prompt action.")
        try expectContains(calls, "\"task_text\":\"Prepare local release audit work.\"", "Task cockpit should send task text.")
        try expectContains(calls, "\"agents\":[\"claude-code\"]", "Task cockpit should pass the selected agent scope.")
        try expectContains(calls, "\"instance_ids\":[\"alpha\",\"beta\"]", "Task cockpit should include effective skill candidates for selected agents.")
        try expectFalse(calls.contains("task.buildCockpit"), "New Task cockpit flow must not use local string-matching cockpit RPC.")
        try expectFalse(calls.contains("config.toggleSkill"), "Task cockpit must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Task cockpit must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforeCockpit, "Task cockpit must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Task cockpit must not call credential paths.")
    }

    private func taskCockpitUsesGlobalScopeOutsideSkillDetail() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let historyStore = makeTemporaryTaskCockpitHistoryStore()
        defer { cleanupTaskCockpitHistoryStore(historyStore) }
        let store = SkillStore(service: fake.serviceClient(), taskCockpitHistoryStore: historyStore)
        store.agentFilter = .claudeCode
        store.taskCockpitText = "查看阿里云 ALB 报警历史"
        await store.reload()
        try expectEqual(store.selectedSidebarSelection?.isSkill, true, "Fixture should cover the default selected skill sidebar state.")
        await store.buildTaskCockpit()

        let calls = fake.calls()
        let previewCall = calls
            .split(separator: "\n")
            .map(String.init)
            .last { $0.contains("\"method\":\"llm.previewPrompt\"") && $0.contains("\"request_kind\":\"task_cockpit\"") }
        try expectContains(previewCall, "llm.previewPrompt", "Global Preflight should prepare the provider-backed task_cockpit prompt.")
        try expectContains(previewCall, "\"task_text\":\"查看阿里云 ALB 报警历史\"", "Global Preflight should send the original Chinese task text.")
        try expectContains(previewCall, "\"agents\":[\"claude-code\"]", "Global Preflight should use the current sidebar agent as the default scope.")
        try expectContains(previewCall, "\"instance_ids\":[\"alpha\",\"beta\"]", "Global Preflight should include effective skills for the selected agent scope.")
        try expectFalse(previewCall?.contains("\"selected_skill_id\"") ?? false, "Global Preflight should not inherit a retained selected skill id.")
    }

    private func taskCockpitHistoryPersistsLocally() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let historyStore = makeTemporaryTaskCockpitHistoryStore()
        defer { cleanupTaskCockpitHistoryStore(historyStore) }
        let task = "阿里云 ECS 磁盘负载情况分析"
        let firstStore = SkillStore(service: fake.serviceClient(), taskCockpitHistoryStore: historyStore)
        firstStore.taskCockpitText = task
        await firstStore.reload()
        await firstStore.buildTaskCockpit()

        try expectEqual(firstStore.taskCockpitHistory.count, 1, "Successful Preflight should add one local history record.")
        try expectEqual(firstStore.taskCockpitHistory.first?.displayTask, task, "History should preserve the visible task text.")
        try expectEqual(FileManager.default.fileExists(atPath: historyStore.fileURL.path), true, "Preflight history should be written to the local app data file.")

        let persisted = try String(contentsOf: historyStore.fileURL, encoding: .utf8)
        try expectContains(persisted, task, "Persisted history should include the task for later recall.")
        try expectFalse(persisted.contains("prompt_request"), "Persisted history must not keep provider prompt request metadata.")
        try expectFalse(persisted.contains("task_cockpit"), "Persisted history must not keep raw prompt request kind metadata.")

        let secondStore = SkillStore(service: fake.serviceClient(), taskCockpitHistoryStore: historyStore)
        try expectEqual(secondStore.taskCockpitHistory.count, 1, "A new store should load persisted Preflight history.")
        try expectEqual(secondStore.taskCockpitHistory.first?.displayTask, task, "Reloaded history should show the original task.")
        try expectEqual(secondStore.taskCockpitHistory.first?.result.summary.recommendedSkillName, "Beta", "Reloaded history should retain the recommendation summary.")
    }

    private func taskCockpitPreservesExactUserInputInServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let exactTask = "  修复 Task Cockpit 🧪\n第二行\t带制表  "
        let historyStore = makeTemporaryTaskCockpitHistoryStore()
        defer { cleanupTaskCockpitHistoryStore(historyStore) }
        let store = SkillStore(service: fake.serviceClient(), taskCockpitHistoryStore: historyStore)
        store.selectedSkillID = "beta"
        store.taskCockpitText = exactTask
        await store.reload()
        await store.buildTaskCockpit()

        try expectEqual(store.selectedTaskCockpitInput, exactTask, "Non-blank cockpit input should preserve the exact user text.")

        let calls = fake.calls()
        try expectContains(calls, "llm.previewPrompt", "Exact-input test should prepare the provider-backed Task Preflight prompt.")
        try expectContains(calls, "llm.confirmPromptAndSend", "Exact-input cockpit flow should send through provider confirmation.")
        try expectContains(calls, "\"task_text\":\"  修复 Task Cockpit 🧪\\n第二行\\t带制表  \"", "Task cockpit should pass Chinese, emoji, multiline text, tabs, and surrounding spaces unchanged.")
        try expectFalse(calls.contains("\"task_text\":\"修复 Task Cockpit 🧪\\n第二行\\t带制表\""), "Task cockpit must not trim non-blank user text before submission.")
        try expectFalse(calls.contains("task.buildCockpit"), "Exact-input cockpit flow must not use local string matching.")
        try expectFalse(calls.contains("config.toggleSkill"), "Exact-input cockpit flow must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Exact-input cockpit flow must not call execution paths.")
        try expectFalse(calls.contains("credential"), "Exact-input cockpit flow must not call credential paths.")
    }

    private func taskCockpitWhitespaceOnlyInputUsesFallbackTask() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let historyStore = makeTemporaryTaskCockpitHistoryStore()
        defer { cleanupTaskCockpitHistoryStore(historyStore) }
        let store = SkillStore(service: fake.serviceClient(), taskCockpitHistoryStore: historyStore)
        store.selectedSkillID = "beta"
        store.routingConfidenceText = "Route a local audit release note task."
        store.taskCockpitText = " \n\t "
        await store.reload()
        await store.buildTaskCockpit()

        try expectEqual(store.selectedTaskCockpitInput, "Route a local audit release note task.", "Whitespace-only cockpit input should reuse the existing fallback task.")

        let calls = fake.calls()
        try expectContains(calls, "llm.previewPrompt", "Whitespace fallback test should prepare the provider-backed Task Preflight prompt.")
        try expectContains(calls, "\"task_text\":\"Route a local audit release note task.\"", "Whitespace-only cockpit input should submit the fallback task.")
        try expectFalse(calls.contains("\"task_text\":\" \\n\\t \""), "Whitespace-only cockpit input must not be sent as the task.")
        try expectFalse(calls.contains("task.buildCockpit"), "Whitespace fallback cockpit flow must not use local string matching.")
        try expectFalse(calls.contains("config.toggleSkill"), "Whitespace fallback cockpit flow must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Whitespace fallback cockpit flow must not call execution paths.")
    }

    private func taskCockpitFallsBackWhenMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let historyStore = makeTemporaryTaskCockpitHistoryStore()
        defer { cleanupTaskCockpitHistoryStore(historyStore) }
        let store = SkillStore(service: fake.serviceClient(), taskCockpitHistoryStore: historyStore)
        store.selectedSkillID = "beta"
        store.routingConfidenceText = "Route a local audit release note task."
        await store.reload()
        await store.buildTaskCockpit()

        try expectEqual(store.taskCockpitResult?.isUnavailable, true, "Task cockpit should expose unavailable fallback for older services.")
        try expectEqual(store.taskCockpitResult?.fallbackReason, UIStrings.taskCockpitUnavailable, "Unknown method fallback should use the localized unavailable copy.")
        try expectFalse(store.isBuildingTaskCockpit, "Unavailable task cockpit should reset loading state.")
        try expectContains(fake.calls(), "llm.previewPrompt", "Fallback should still prove the intended provider preview method was attempted.")
        try expectContains(fake.calls(), "\"task_text\":\"Route a local audit release note task.\"", "Fallback should reuse existing routing task text when cockpit input is blank.")
        try expectFalse(fake.calls().contains("task.buildCockpit"), "Unavailable cockpit flow must not fall back to local string matching.")
        try expectFalse(fake.calls().contains("llm.confirmPromptAndSend"), "Unavailable cockpit flow must not send to provider.")
        try expectFalse(fake.calls().contains("config.toggleSkill"), "Unavailable cockpit flow must not call config write paths.")
        try expectFalse(fake.calls().contains("script.execute"), "Unavailable cockpit flow must not call execution paths.")
    }

    private func taskCockpitTimeoutShowsRecoveryAndIgnoresStaleResponse() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let historyStore = makeTemporaryTaskCockpitHistoryStore()
        defer { cleanupTaskCockpitHistoryStore(historyStore) }
        let store = SkillStore(service: fake.serviceClient(), taskCockpitTimeoutSeconds: 0.5, taskCockpitHistoryStore: historyStore)
        store.selectedSkillID = "beta"
        store.taskCockpitText = "Prepare local release audit work."
        await store.reload()

        fake.setScenario("slow-task-cockpit")
        let slowBuild = Task {
            await store.buildTaskCockpit()
        }

        try await waitUntil("Slow task cockpit should time out into a visible recovery state.") {
            store.taskCockpitOperationState.phase == .timedOut
        }
        try expectFalse(store.isBuildingTaskCockpit, "Timed-out task cockpit should release the loading state.")
        try expectEqual(store.taskCockpitOperationState.canRetry, true, "Timed-out task cockpit should expose retry.")
        try expectContains(store.taskCockpitResult?.fallbackReason, "did not finish", "Timeout should produce a visible fallback reason.")

        fake.setScenario("prompt-ready")
        await store.buildTaskCockpit()
        try expectEqual(store.taskCockpitResult?.summary.recommendedSkillName, "Beta", "Retry should load the fresh cockpit result.")
        try expectEqual(store.taskCockpitOperationState.phase, .completed, "Retry success should replace the timeout state.")

        await slowBuild.value
        try expectEqual(store.taskCockpitResult?.summary.recommendedSkillName, "Beta", "Late slow response must not overwrite the retry result.")
        let calls = fake.calls()
        try expectContains(calls, "llm.previewPrompt", "Timeout path should prepare task preflight prompt previews.")
        try expectContains(calls, "llm.confirmPromptAndSend", "Timeout path should use the provider confirmation method.")
        try expectFalse(calls.contains("task.buildCockpit"), "Timeout recovery must not use local string matching.")
        try expectFalse(calls.contains("config.toggleSkill"), "Timeout recovery must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Timeout recovery must not call execution paths.")
    }

    private func taskCockpitCancelShowsRecoveryAndAllowsRetry() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let historyStore = makeTemporaryTaskCockpitHistoryStore()
        defer { cleanupTaskCockpitHistoryStore(historyStore) }
        let store = SkillStore(service: fake.serviceClient(), taskCockpitTimeoutSeconds: 1, taskCockpitHistoryStore: historyStore)
        store.selectedSkillID = "beta"
        store.taskCockpitText = "Prepare local release audit work."
        await store.reload()

        fake.setScenario("slow-task-cockpit")
        let slowBuild = Task {
            await store.buildTaskCockpit()
        }

        try await waitUntil("Slow task cockpit should enter preparing state.") {
            store.taskCockpitOperationState.phase == .preparing
        }
        store.cancelTaskCockpitBuild()

        try expectFalse(store.isBuildingTaskCockpit, "Cancelled task cockpit should release the loading state.")
        try expectEqual(store.taskCockpitOperationState.phase, .cancelled, "Cancel should expose a visible cancelled state.")
        try expectEqual(store.taskCockpitOperationState.canRetry, true, "Cancelled task cockpit should expose retry.")
        try expectEqual(store.taskCockpitResult?.fallbackReason, UIStrings.taskCockpitCancelled, "Cancel should produce localized recovery metadata.")

        fake.setScenario("prompt-ready")
        await store.buildTaskCockpit()
        try expectEqual(store.taskCockpitResult?.summary.recommendedSkillName, "Beta", "Retry after cancel should load the cockpit result.")
        try expectEqual(store.taskCockpitOperationState.phase, .completed, "Retry success should replace the cancelled state.")

        await slowBuild.value
        try expectEqual(store.taskCockpitResult?.summary.recommendedSkillName, "Beta", "Late cancelled response must not overwrite the retry result.")
        try expectContains(fake.calls(), "llm.confirmPromptAndSend", "Cancel recovery should use the provider confirmation method for the retry.")
        try expectFalse(fake.calls().contains("task.buildCockpit"), "Cancel recovery must not use local string matching.")
        try expectFalse(fake.calls().contains("config.toggleSkill"), "Cancel recovery must not call config write paths.")
        try expectFalse(fake.calls().contains("script.execute"), "Cancel recovery must not call execution paths.")
    }

    private func similarSkillGroupingUsesReadOnlyServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        let snapshotCallsBeforeGrouping = countOccurrences("snapshot.", in: fake.calls())
        await store.groupSimilarSkills()

        let result = store.similarSkillGroupingResult
        try expectEqual(result?.generatedBy, "local-v2.53", "Similar grouping should expose generator metadata.")
        try expectEqual(result?.summary.groupCount, 1, "Similar grouping should expose group count.")
        try expectEqual(result?.groups.first?.title, "Audit release skills", "Similar grouping should expose group title.")
        try expectEqual(result?.groups.first?.typeLabel, UIStrings.similarGroupingDuplicate, "Similar grouping should expose duplicate type.")
        try expectEqual(result?.groups.first?.members.first?.skillName, "Beta", "Similar grouping should expose member rows.")
        try expectEqual(result?.groups.first?.sharedTools, ["rg"], "Similar grouping should expose shared tools.")
        try expectEqual(result?.gapNotes.first, "No benchmark separates the two skills.", "Similar grouping should expose gap notes.")
        try expectEqual(result?.evidenceReferences.first?.title, "Similar grouping", "Similar grouping should expose evidence references.")
        try expectFalse(result?.safetyFlags.providerRequestSent ?? true, "Similar grouping must not send provider requests.")
        try expectFalse(result?.safetyFlags.writeBackAllowed ?? true, "Similar grouping must not allow write-back.")
        try expectFalse(result?.safetyFlags.writeActionsAvailable ?? true, "Similar grouping must not expose write actions.")
        try expectFalse(result?.safetyFlags.scriptExecutionAllowed ?? true, "Similar grouping must not allow script execution.")
        try expectFalse(result?.safetyFlags.executionActionsAvailable ?? true, "Similar grouping must not expose execution actions.")
        try expectFalse(result?.safetyFlags.configMutationAllowed ?? true, "Similar grouping must not mutate config.")
        try expectFalse(result?.safetyFlags.snapshotCreated ?? true, "Similar grouping must not create snapshots.")
        try expectFalse(result?.safetyFlags.triageMutationAllowed ?? true, "Similar grouping must not mutate triage.")
        try expectFalse(result?.safetyFlags.credentialAccessed ?? true, "Similar grouping must not access credentials.")
        try expectFalse(result?.safetyFlags.rawPromptPersisted ?? true, "Similar grouping must not persist raw prompts.")
        try expectFalse(result?.safetyFlags.rawResponsePersisted ?? true, "Similar grouping must not persist raw responses.")
        try expectFalse(result?.safetyFlags.rawTracePersisted ?? true, "Similar grouping must not persist raw traces.")
        try expectFalse(result?.safetyFlags.cloudSyncEnabled ?? true, "Similar grouping must not sync cloud data.")
        try expectFalse(result?.safetyFlags.telemetryEnabled ?? true, "Similar grouping must not emit telemetry.")
        try expectFalse(store.isGroupingSimilarSkills, "Similar grouping should reset loading state.")

        let calls = fake.calls()
        try expectContains(calls, "knowledge.groupSimilarSkills", "Similar grouping should call the V2.53 grouping method.")
        try expectContains(calls, "\"agent\":\"claude-code\"", "Similar grouping should pass the current agent filter.")
        try expectContains(calls, "\"limit\":20", "Similar grouping should pass the group limit.")
        try expectContains(calls, "\"min_score\":0.62", "Similar grouping should pass the score threshold.")
        try expectContains(calls, "\"include_singletons\":false", "Similar grouping should omit singleton groups by default.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Similar grouping must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Similar grouping must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Similar grouping must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Similar grouping must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforeGrouping, "Similar grouping must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Similar grouping must not call credential paths.")
    }

    private func similarSkillGroupingFallsBackWhenMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.groupSimilarSkills()

        try expectEqual(store.similarSkillGroupingResult?.isUnavailable, true, "Similar grouping should expose unavailable fallback for older services.")
        try expectEqual(store.similarSkillGroupingResult?.fallbackReason, UIStrings.similarGroupingUnavailable, "Unknown method fallback should use the localized unavailable copy.")
        try expectFalse(store.isGroupingSimilarSkills, "Unavailable similar grouping should reset loading state.")
        try expectContains(fake.calls(), "knowledge.groupSimilarSkills", "Fallback should still prove the intended V2.53 method was attempted.")
    }

    private func capabilityTaxonomyUsesReadOnlyServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        let snapshotCallsBeforeBuild = countOccurrences("snapshot.", in: fake.calls())
        await store.buildCapabilityTaxonomy()

        let result = store.capabilityTaxonomyResult
        try expectEqual(result?.generatedBy, "local-v2.54", "Capability taxonomy should expose generator metadata.")
        try expectEqual(result?.summary.domainCount, 1, "Capability taxonomy should expose domain count.")
        try expectEqual(result?.summary.capabilityCount, 2, "Capability taxonomy should expose capability count.")
        try expectEqual(result?.coverageByAgent.first?.agent, "claude-code", "Capability taxonomy should expose agent coverage.")
        try expectEqual(result?.domains.first?.name, "Audit workflows", "Capability taxonomy should expose domain title.")
        try expectEqual(result?.domains.first?.capabilities.first?.name, "Release audit", "Capability taxonomy should expose capability rows.")
        try expectEqual(result?.domains.first?.capabilities.first?.representativeSkills.first?.skillName, "Beta", "Capability taxonomy should expose representative skills.")
        try expectEqual(result?.gapNotes.first, "Codex has no equivalent audit capability.", "Capability taxonomy should expose gap notes.")
        try expectEqual(result?.evidenceReferences.first?.title, "Capability taxonomy", "Capability taxonomy should expose evidence references.")
        try expectFalse(result?.safetyFlags.providerRequestSent ?? true, "Capability taxonomy must not send provider requests.")
        try expectFalse(result?.safetyFlags.writeBackAllowed ?? true, "Capability taxonomy must not allow write-back.")
        try expectFalse(result?.safetyFlags.writeActionsAvailable ?? true, "Capability taxonomy must not expose write actions.")
        try expectFalse(result?.safetyFlags.scriptExecutionAllowed ?? true, "Capability taxonomy must not allow script execution.")
        try expectFalse(result?.safetyFlags.executionActionsAvailable ?? true, "Capability taxonomy must not expose execution actions.")
        try expectFalse(result?.safetyFlags.configMutationAllowed ?? true, "Capability taxonomy must not mutate config.")
        try expectFalse(result?.safetyFlags.snapshotCreated ?? true, "Capability taxonomy must not create snapshots.")
        try expectFalse(result?.safetyFlags.triageMutationAllowed ?? true, "Capability taxonomy must not mutate triage.")
        try expectFalse(result?.safetyFlags.credentialAccessed ?? true, "Capability taxonomy must not access credentials.")
        try expectFalse(result?.safetyFlags.rawPromptPersisted ?? true, "Capability taxonomy must not persist raw prompts.")
        try expectFalse(result?.safetyFlags.rawResponsePersisted ?? true, "Capability taxonomy must not persist raw responses.")
        try expectFalse(result?.safetyFlags.rawTracePersisted ?? true, "Capability taxonomy must not persist raw traces.")
        try expectFalse(result?.safetyFlags.cloudSyncEnabled ?? true, "Capability taxonomy must not sync cloud data.")
        try expectFalse(result?.safetyFlags.telemetryEnabled ?? true, "Capability taxonomy must not emit telemetry.")
        try expectFalse(store.isBuildingCapabilityTaxonomy, "Capability taxonomy should reset loading state.")

        let calls = fake.calls()
        try expectContains(calls, "knowledge.buildCapabilityTaxonomy", "Capability taxonomy should call the V2.54 taxonomy method.")
        try expectContains(calls, "\"agent\":\"claude-code\"", "Capability taxonomy should pass the current agent filter.")
        try expectContains(calls, "\"limit\":20", "Capability taxonomy should pass the domain limit.")
        try expectContains(calls, "\"include_single_skill_domains\":true", "Capability taxonomy should request single-skill domain coverage.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Capability taxonomy must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Capability taxonomy must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Capability taxonomy must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Capability taxonomy must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforeBuild, "Capability taxonomy must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Capability taxonomy must not call credential paths.")
    }

    private func capabilityTaxonomyFallsBackWhenMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.buildCapabilityTaxonomy()

        try expectEqual(store.capabilityTaxonomyResult?.isUnavailable, true, "Capability taxonomy should expose unavailable fallback for older services.")
        try expectEqual(store.capabilityTaxonomyResult?.fallbackReason, UIStrings.capabilityTaxonomyUnavailable, "Unknown method fallback should use the localized unavailable copy.")
        try expectFalse(store.isBuildingCapabilityTaxonomy, "Unavailable capability taxonomy should reset loading state.")
        try expectContains(fake.calls(), "knowledge.buildCapabilityTaxonomy", "Fallback should still prove the intended V2.54 method was attempted.")
    }

    private func workspaceReadinessUsesReadOnlyServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.routingConfidenceText = "Prepare local release audit work."
        await store.reload()
        let snapshotCallsBeforeCheck = countOccurrences("snapshot.", in: fake.calls())
        await store.checkWorkspaceReadiness()

        let result = store.workspaceReadinessResult
        try expectEqual(result?.generatedBy, "local-v2.55", "Workspace readiness should expose generator metadata.")
        try expectEqual(result?.summary.overallState, "partial", "Workspace readiness should expose overall state.")
        try expectEqual(result?.summary.readinessScore, 78, "Workspace readiness should expose readiness score.")
        try expectEqual(result?.checklistRows.first?.title, "Release audit skill enabled", "Workspace readiness should expose checklist rows.")
        try expectEqual(result?.checklistRows.first?.matchedSkills.first?.skillName, "Beta", "Workspace readiness should expose matched skills.")
        try expectEqual(result?.agentRows.first?.agent, "claude-code", "Workspace readiness should expose per-agent rows.")
        try expectEqual(result?.capabilityRows.first?.capability, "Release audit", "Workspace readiness should expose capability rows.")
        try expectEqual(result?.gapNotes.first, "Codex lacks a project-scoped release audit skill.", "Workspace readiness should expose gap notes.")
        try expectEqual(result?.evidenceReferences.first?.title, "Workspace readiness", "Workspace readiness should expose evidence references.")
        try expectFalse(result?.safetyFlags.providerRequestSent ?? true, "Workspace readiness must not send provider requests.")
        try expectFalse(result?.safetyFlags.writeBackAllowed ?? true, "Workspace readiness must not allow write-back.")
        try expectFalse(result?.safetyFlags.writeActionsAvailable ?? true, "Workspace readiness must not expose write actions.")
        try expectFalse(result?.safetyFlags.scriptExecutionAllowed ?? true, "Workspace readiness must not allow script execution.")
        try expectFalse(result?.safetyFlags.executionActionsAvailable ?? true, "Workspace readiness must not expose execution actions.")
        try expectFalse(result?.safetyFlags.configMutationAllowed ?? true, "Workspace readiness must not mutate config.")
        try expectFalse(result?.safetyFlags.snapshotCreated ?? true, "Workspace readiness must not create snapshots.")
        try expectFalse(result?.safetyFlags.triageMutationAllowed ?? true, "Workspace readiness must not mutate triage.")
        try expectFalse(result?.safetyFlags.credentialAccessed ?? true, "Workspace readiness must not access credentials.")
        try expectFalse(result?.safetyFlags.rawPromptPersisted ?? true, "Workspace readiness must not persist raw prompts.")
        try expectFalse(result?.safetyFlags.rawResponsePersisted ?? true, "Workspace readiness must not persist raw responses.")
        try expectFalse(result?.safetyFlags.rawTracePersisted ?? true, "Workspace readiness must not persist raw traces.")
        try expectFalse(result?.safetyFlags.cloudSyncEnabled ?? true, "Workspace readiness must not sync cloud data.")
        try expectFalse(result?.safetyFlags.telemetryEnabled ?? true, "Workspace readiness must not emit telemetry.")
        try expectFalse(store.isCheckingWorkspaceReadiness, "Workspace readiness should reset loading state.")

        let calls = fake.calls()
        try expectContains(calls, "workspace.checkReadiness", "Workspace readiness should call the V2.55 workspace method.")
        try expectContains(calls, "\"task\":\"Prepare local release audit work.\"", "Workspace readiness should pass current task context when present.")
        try expectContains(calls, "\"agent\":\"claude-code\"", "Workspace readiness should pass the current agent filter.")
        try expectContains(calls, "\"limit\":40", "Workspace readiness should pass the check limit.")
        try expectContains(calls, "\"include_checklist\":true", "Workspace readiness should request checklist rows.")
        try expectContains(calls, "\"include_capabilities\":true", "Workspace readiness should request capability rows.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Workspace readiness must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Workspace readiness must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Workspace readiness must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Workspace readiness must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforeCheck, "Workspace readiness must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Workspace readiness must not call credential paths.")
    }

    private func workspaceReadinessFallsBackWhenMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.checkWorkspaceReadiness()

        try expectEqual(store.workspaceReadinessResult?.isUnavailable, true, "Workspace readiness should expose unavailable fallback for older services.")
        try expectEqual(store.workspaceReadinessResult?.fallbackReason, UIStrings.workspaceReadinessUnavailable, "Unknown method fallback should use the localized unavailable copy.")
        try expectFalse(store.isCheckingWorkspaceReadiness, "Unavailable workspace readiness should reset loading state.")
        try expectContains(fake.calls(), "workspace.checkReadiness", "Fallback should still prove the intended V2.55 method was attempted.")
    }

    private func remediationPlanUsesReadOnlyServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.routingConfidenceText = "Prepare local release audit work."
        await store.reload()
        let snapshotCallsBeforePlan = countOccurrences("snapshot.", in: fake.calls())
        await store.planRemediation()

        let result = store.remediationPlanResult
        try expectEqual(result?.generatedBy, "local-v2.56", "Remediation plan should expose generator metadata.")
        try expectEqual(result?.summary.totalCount, 2, "Remediation plan should expose total item count.")
        try expectEqual(result?.summary.highCount, 1, "Remediation plan should expose high-priority count.")
        try expectEqual(result?.priorityRows.first?.title, "High priority", "Remediation plan should expose priority rows.")
        try expectEqual(result?.items.first?.title, "Add Codex release audit coverage", "Remediation plan should expose plan items.")
        try expectEqual(result?.items.first?.skill?.skillName, "Beta", "Remediation plan should expose referenced skill evidence.")
        try expectEqual(result?.items.first?.guidanceOnly, true, "Remediation items must stay guidance-only.")
        try expectEqual(result?.gapNotes.first, "Codex lacks a project-scoped release audit skill.", "Remediation plan should expose gap notes.")
        try expectEqual(result?.blockerNotes.first, "No automatic write/apply path is exposed.", "Remediation plan should expose blocker notes.")
        try expectEqual(result?.evidenceReferences.first?.title, "Remediation planner", "Remediation plan should expose evidence references.")
        try expectFalse(result?.safetyFlags.providerRequestSent ?? true, "Remediation planning must not send provider requests.")
        try expectFalse(result?.safetyFlags.writeBackAllowed ?? true, "Remediation planning must not allow write-back.")
        try expectFalse(result?.safetyFlags.writeActionsAvailable ?? true, "Remediation planning must not expose write actions.")
        try expectFalse(result?.safetyFlags.scriptExecutionAllowed ?? true, "Remediation planning must not allow script execution.")
        try expectFalse(result?.safetyFlags.executionActionsAvailable ?? true, "Remediation planning must not expose execution actions.")
        try expectFalse(result?.safetyFlags.configMutationAllowed ?? true, "Remediation planning must not mutate config.")
        try expectFalse(result?.safetyFlags.snapshotCreated ?? true, "Remediation planning must not create snapshots.")
        try expectFalse(result?.safetyFlags.triageMutationAllowed ?? true, "Remediation planning must not mutate triage.")
        try expectFalse(result?.safetyFlags.credentialAccessed ?? true, "Remediation planning must not access credentials.")
        try expectFalse(result?.safetyFlags.rawPromptPersisted ?? true, "Remediation planning must not persist raw prompts.")
        try expectFalse(result?.safetyFlags.rawResponsePersisted ?? true, "Remediation planning must not persist raw responses.")
        try expectFalse(result?.safetyFlags.rawTracePersisted ?? true, "Remediation planning must not persist raw traces.")
        try expectFalse(result?.safetyFlags.cloudSyncEnabled ?? true, "Remediation planning must not sync cloud data.")
        try expectFalse(result?.safetyFlags.telemetryEnabled ?? true, "Remediation planning must not emit telemetry.")
        try expectFalse(store.isPlanningRemediation, "Remediation planner should reset loading state.")

        let calls = fake.calls()
        try expectContains(calls, "remediation.plan", "Remediation planner should call the V2.56 remediation method.")
        try expectContains(calls, "\"task\":\"Prepare local release audit work.\"", "Remediation planner should pass current task context when present.")
        try expectContains(calls, "\"agent\":\"claude-code\"", "Remediation planner should pass the current agent filter.")
        try expectContains(calls, "\"limit\":20", "Remediation planner should pass the plan limit.")
        try expectContains(calls, "\"include_guidance_only\":true", "Remediation planner should request guidance-only output.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Remediation planning must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Remediation planning must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Remediation planning must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Remediation planning must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforePlan, "Remediation planning must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Remediation planning must not call credential paths.")
    }

    private func remediationPlanFallsBackWhenMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.planRemediation()

        try expectEqual(store.remediationPlanResult?.isUnavailable, true, "Remediation planner should expose unavailable fallback for older services.")
        try expectEqual(store.remediationPlanResult?.fallbackReason, UIStrings.remediationPlanUnavailable, "Unknown method fallback should use the localized unavailable copy.")
        try expectFalse(store.isPlanningRemediation, "Unavailable remediation planner should reset loading state.")
        try expectContains(fake.calls(), "remediation.plan", "Fallback should still prove the intended V2.56 method was attempted.")
    }

    private func remediationPreviewDraftsUsesCopyOnlyServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.routingConfidenceText = "Prepare local release audit work."
        await store.reload()
        let snapshotCallsBeforePreview = countOccurrences("snapshot.", in: fake.calls())
        await store.previewRemediationDrafts()

        let result = store.remediationPreviewDraftsResult
        try expectEqual(result?.generatedBy, "local-v2.57", "Fix preview drafts should expose generator metadata.")
        try expectEqual(result?.summary.totalCount, 2, "Fix preview drafts should expose total draft count.")
        try expectEqual(result?.summary.frontmatterCount, 1, "Fix preview drafts should expose frontmatter count.")
        try expectEqual(result?.summary.permissionsCount, 1, "Fix preview drafts should expose permissions count.")
        try expectEqual(result?.draftItems.first?.title, "Declare network permission", "Fix preview drafts should expose draft items.")
        try expectEqual(result?.draftItems.first?.draftType, "frontmatter", "Fix preview drafts should expose draft type.")
        try expectEqual(result?.draftItems.first?.affectedSkill?.skillName, "Beta", "Fix preview drafts should expose affected skill evidence.")
        try expectEqual(result?.draftItems.first?.proposedText, "permissions:\n  network: true", "Fix preview drafts should expose proposed copy text.")
        try expectEqual(result?.draftItems.first?.copyLabel, "Copy YAML", "Fix preview drafts should expose copy label.")
        try expectEqual(result?.gapNotes.first, "No dependency draft needed.", "Fix preview drafts should expose gap notes.")
        try expectEqual(result?.blockerNotes.first, "No automatic write/apply path is exposed.", "Fix preview drafts should expose blocker notes.")
        try expectEqual(result?.evidenceReferences.first?.title, "Fix preview", "Fix preview drafts should expose evidence references.")
        try expectEqual(result?.promptRequest?.requestKind, "remediation_preview_drafts", "Fix preview prompt metadata should use V2.57 request kind.")
        try expectFalse(result?.safetyFlags.providerRequestSent ?? true, "Fix preview drafts must not send provider requests.")
        try expectFalse(result?.safetyFlags.writeBackAllowed ?? true, "Fix preview drafts must not allow write-back.")
        try expectFalse(result?.safetyFlags.writeActionsAvailable ?? true, "Fix preview drafts must not expose write actions.")
        try expectFalse(result?.safetyFlags.scriptExecutionAllowed ?? true, "Fix preview drafts must not allow script execution.")
        try expectFalse(result?.safetyFlags.executionActionsAvailable ?? true, "Fix preview drafts must not expose execution actions.")
        try expectFalse(result?.safetyFlags.configMutationAllowed ?? true, "Fix preview drafts must not mutate config.")
        try expectFalse(result?.safetyFlags.snapshotCreated ?? true, "Fix preview drafts must not create snapshots.")
        try expectFalse(result?.safetyFlags.triageMutationAllowed ?? true, "Fix preview drafts must not mutate triage.")
        try expectFalse(result?.safetyFlags.credentialAccessed ?? true, "Fix preview drafts must not access credentials.")
        try expectFalse(result?.safetyFlags.rawPromptPersisted ?? true, "Fix preview drafts must not persist raw prompts.")
        try expectFalse(result?.safetyFlags.rawResponsePersisted ?? true, "Fix preview drafts must not persist raw responses.")
        try expectFalse(result?.safetyFlags.rawTracePersisted ?? true, "Fix preview drafts must not persist raw traces.")
        try expectFalse(result?.safetyFlags.cloudSyncEnabled ?? true, "Fix preview drafts must not sync cloud data.")
        try expectFalse(result?.safetyFlags.telemetryEnabled ?? true, "Fix preview drafts must not emit telemetry.")
        try expectFalse(store.isPreviewingRemediationDrafts, "Fix preview drafts should reset loading state.")

        let calls = fake.calls()
        try expectContains(calls, "remediation.previewDrafts", "Fix preview UI should call the V2.57 drafts method.")
        try expectContains(calls, "\"task\":\"Prepare local release audit work.\"", "Fix preview drafts should pass current task context when present.")
        try expectContains(calls, "\"agent\":\"claude-code\"", "Fix preview drafts should pass the current agent filter.")
        try expectContains(calls, "\"limit\":20", "Fix preview drafts should pass the draft limit.")
        try expectContains(calls, "\"draft_types\":[\"frontmatter\",\"description\",\"permissions\",\"dependency\",\"policy\"]", "Fix preview drafts should request the supported draft kinds.")
        try expectContains(calls, "\"include_blocked\":true", "Fix preview drafts should keep blocked/manual-review drafts visible.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Fix preview drafts must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Fix preview drafts must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Fix preview drafts must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Fix preview drafts must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforePreview, "Fix preview drafts must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Fix preview drafts must not call credential paths.")
    }

    private func remediationPreviewDraftsFallsBackWhenMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.previewRemediationDrafts()

        try expectEqual(store.remediationPreviewDraftsResult?.isUnavailable, true, "Fix preview drafts should expose unavailable fallback for older services.")
        try expectEqual(store.remediationPreviewDraftsResult?.fallbackReason, UIStrings.fixPreviewUnavailable, "Unknown method fallback should use the localized unavailable copy.")
        try expectFalse(store.isPreviewingRemediationDrafts, "Unavailable fix preview drafts should reset loading state.")
        try expectContains(fake.calls(), "remediation.previewDrafts", "Fallback should still prove the intended V2.57 method was attempted.")
    }

    private func remediationImpactPreviewUsesReadOnlyServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.routingConfidenceText = "Prepare local release audit work."
        await store.reload()
        let snapshotCallsBeforePreview = countOccurrences("snapshot.", in: fake.calls())
        await store.previewRemediationImpact()

        let result = store.remediationImpactPreviewResult
        try expectEqual(result?.generatedBy, "local-v2.58", "Impact preview should expose generator metadata.")
        try expectEqual(result?.summary.totalCount, 6, "Impact preview should expose total impact count.")
        try expectEqual(result?.summary.taskImpactCount, 1, "Impact preview should expose task impact count.")
        try expectEqual(result?.impactRows.first?.title, "Overall readiness improves", "Impact preview should expose general impact rows.")
        try expectEqual(result?.taskImpactRows.first?.delta, "+12 readiness", "Impact preview should expose task deltas.")
        try expectEqual(result?.agentImpactRows.first?.agent, "claude-code", "Impact preview should expose agent rows.")
        try expectEqual(result?.skillImpactRows.first?.skill?.skillName, "Beta", "Impact preview should expose affected skill evidence.")
        try expectEqual(result?.riskDeltaRows.first?.title, "Network declaration risk drops", "Impact preview should expose risk deltas.")
        try expectEqual(result?.snapshotRollbackRows.first?.title, "No snapshot is created", "Impact preview should expose snapshot/rollback plan rows.")
        try expectEqual(result?.gapNotes.first, "Codex still lacks project-scoped coverage.", "Impact preview should expose gap notes.")
        try expectEqual(result?.blockerNotes.first, "No apply/write path is exposed.", "Impact preview should expose blocker notes.")
        try expectEqual(result?.evidenceReferences.first?.title, "Impact preview", "Impact preview should expose evidence references.")
        try expectEqual(result?.promptRequest?.requestKind, "remediation_preview_impact", "Impact preview prompt metadata should use V2.58 request kind.")
        try expectFalse(result?.safetyFlags.providerRequestSent ?? true, "Impact preview must not send provider requests.")
        try expectFalse(result?.safetyFlags.writeBackAllowed ?? true, "Impact preview must not allow write-back.")
        try expectFalse(result?.safetyFlags.writeActionsAvailable ?? true, "Impact preview must not expose write actions.")
        try expectFalse(result?.safetyFlags.scriptExecutionAllowed ?? true, "Impact preview must not allow script execution.")
        try expectFalse(result?.safetyFlags.executionActionsAvailable ?? true, "Impact preview must not expose execution actions.")
        try expectFalse(result?.safetyFlags.configMutationAllowed ?? true, "Impact preview must not mutate config.")
        try expectFalse(result?.safetyFlags.snapshotCreated ?? true, "Impact preview must not create snapshots.")
        try expectFalse(result?.safetyFlags.triageMutationAllowed ?? true, "Impact preview must not mutate triage.")
        try expectFalse(result?.safetyFlags.credentialAccessed ?? true, "Impact preview must not access credentials.")
        try expectFalse(result?.safetyFlags.rawPromptPersisted ?? true, "Impact preview must not persist raw prompts.")
        try expectFalse(result?.safetyFlags.rawResponsePersisted ?? true, "Impact preview must not persist raw responses.")
        try expectFalse(result?.safetyFlags.rawTracePersisted ?? true, "Impact preview must not persist raw traces.")
        try expectFalse(result?.safetyFlags.cloudSyncEnabled ?? true, "Impact preview must not sync cloud data.")
        try expectFalse(result?.safetyFlags.telemetryEnabled ?? true, "Impact preview must not emit telemetry.")
        try expectFalse(store.isPreviewingRemediationImpact, "Impact preview should reset loading state.")

        let calls = fake.calls()
        try expectContains(calls, "remediation.previewImpact", "Impact preview UI should call the V2.58 impact method.")
        try expectContains(calls, "\"task\":\"Prepare local release audit work.\"", "Impact preview should pass current task context when present.")
        try expectContains(calls, "\"agent\":\"claude-code\"", "Impact preview should pass the current agent filter.")
        try expectContains(calls, "\"selected_skill_id\":\"beta\"", "Impact preview should pass selected skill context.")
        try expectContains(calls, "\"selected_skill_name\":\"Beta\"", "Impact preview should pass selected skill name.")
        try expectContains(calls, "\"action\":\"review\"", "Impact preview should default to review action.")
        try expectContains(calls, "\"limit\":20", "Impact preview should pass the impact limit.")
        try expectContains(calls, "\"include_task_impacts\":true", "Impact preview should include task impact rows.")
        try expectContains(calls, "\"include_agent_impacts\":true", "Impact preview should include agent impact rows.")
        try expectContains(calls, "\"include_skill_impacts\":true", "Impact preview should include skill impact rows.")
        try expectContains(calls, "\"include_risk_deltas\":true", "Impact preview should include risk deltas.")
        try expectContains(calls, "\"include_snapshot_rollback\":true", "Impact preview should include snapshot/rollback plan rows.")
        try expectContains(calls, "\"include_blocked\":true", "Impact preview should keep blockers visible.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Impact preview must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Impact preview must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Impact preview must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Impact preview must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforePreview, "Impact preview must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Impact preview must not call credential paths.")
    }

    private func remediationImpactPreviewFallsBackWhenMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.previewRemediationImpact()

        try expectEqual(store.remediationImpactPreviewResult?.isUnavailable, true, "Impact preview should expose unavailable fallback for older services.")
        try expectEqual(store.remediationImpactPreviewResult?.fallbackReason, UIStrings.impactPreviewUnavailable, "Unknown method fallback should use the localized unavailable copy.")
        try expectFalse(store.isPreviewingRemediationImpact, "Unavailable impact preview should reset loading state.")
        try expectContains(fake.calls(), "remediation.previewImpact", "Fallback should still prove the intended V2.58 method was attempted.")
    }

    private func remediationBatchReviewUsesReadOnlyServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.routingConfidenceText = "Prepare local release audit work."
        await store.reload()
        let snapshotCallsBeforeReview = countOccurrences("snapshot.", in: fake.calls())
        await store.reviewRemediationBatch(
            options: RemediationBatchReviewOptions(
                includeTask: true,
                includeRisk: true,
                includeRule: true,
                includeAgent: true,
                includeWorkspace: true,
                includeBlocked: true
            )
        )

        let result = store.remediationBatchReviewResult
        try expectEqual(result?.generatedBy, "local-v2.59", "Batch review should expose generator metadata.")
        try expectEqual(result?.summary.totalCount, 3, "Batch review should expose total review item count.")
        try expectEqual(result?.summary.groupCount, 2, "Batch review should expose group count.")
        try expectEqual(result?.groups.first?.title, "Risk and rule review", "Batch review should expose review groups.")
        try expectEqual(result?.groups.first?.items.first?.ruleID, "permissions.network-declared", "Batch review should expose nested rule review items.")
        try expectEqual(result?.groups.first?.items.first?.skill?.skillName, "Beta", "Batch review should expose affected skill evidence.")
        try expectEqual(result?.items.first?.reviewArea, "Workspace Readiness", "Batch review should expose safe review area labels.")
        try expectEqual(result?.safeNextStepLabels.first, "Open Remediation Planner", "Batch review should expose top-level safe next steps.")
        try expectEqual(result?.gapNotes.first, "Codex lacks project-scoped release audit coverage.", "Batch review should expose gap notes.")
        try expectEqual(result?.blockerNotes.first, "No batch apply path is available from review.", "Batch review should expose blocker notes.")
        try expectEqual(result?.evidenceReferences.first?.title, "Batch review", "Batch review should expose evidence references.")
        try expectEqual(result?.promptRequest?.requestKind, "remediation_batch_review", "Batch review prompt metadata should use V2.59 request kind.")
        try expectFalse(result?.safetyFlags.providerRequestSent ?? true, "Batch review must not send provider requests.")
        try expectFalse(result?.safetyFlags.writeBackAllowed ?? true, "Batch review must not allow write-back.")
        try expectFalse(result?.safetyFlags.writeActionsAvailable ?? true, "Batch review must not expose write actions.")
        try expectFalse(result?.safetyFlags.scriptExecutionAllowed ?? true, "Batch review must not allow script execution.")
        try expectFalse(result?.safetyFlags.executionActionsAvailable ?? true, "Batch review must not expose execution actions.")
        try expectFalse(result?.safetyFlags.configMutationAllowed ?? true, "Batch review must not mutate config.")
        try expectFalse(result?.safetyFlags.snapshotCreated ?? true, "Batch review must not create snapshots.")
        try expectFalse(result?.safetyFlags.triageMutationAllowed ?? true, "Batch review must not mutate triage.")
        try expectFalse(result?.safetyFlags.credentialAccessed ?? true, "Batch review must not access credentials.")
        try expectFalse(result?.safetyFlags.rawPromptPersisted ?? true, "Batch review must not persist raw prompts.")
        try expectFalse(result?.safetyFlags.rawResponsePersisted ?? true, "Batch review must not persist raw responses.")
        try expectFalse(result?.safetyFlags.rawTracePersisted ?? true, "Batch review must not persist raw traces.")
        try expectFalse(result?.safetyFlags.cloudSyncEnabled ?? true, "Batch review must not sync cloud data.")
        try expectFalse(result?.safetyFlags.telemetryEnabled ?? true, "Batch review must not emit telemetry.")
        try expectFalse(store.isReviewingRemediationBatch, "Batch review should reset loading state.")

        let calls = fake.calls()
        try expectContains(calls, "remediation.batchReview", "Batch review UI should call the V2.59 batch review method.")
        try expectContains(calls, "\"task\":\"Prepare local release audit work.\"", "Batch review should pass current task context when present.")
        try expectContains(calls, "\"agent\":\"claude-code\"", "Batch review should pass the current agent filter.")
        try expectContains(calls, "\"selected_skill_id\":\"beta\"", "Batch review should pass selected skill context.")
        try expectContains(calls, "\"selected_skill_name\":\"Beta\"", "Batch review should pass selected skill name.")
        try expectContains(calls, "\"limit\":30", "Batch review should pass the review limit.")
        try expectContains(calls, "\"review_dimensions\":[\"task\",\"risk\",\"rule\",\"agent\",\"workspace\"]", "Batch review should pass selected review dimensions.")
        try expectContains(calls, "\"include_task\":true", "Batch review should include task rows.")
        try expectContains(calls, "\"include_risk\":true", "Batch review should include risk rows.")
        try expectContains(calls, "\"include_rule\":true", "Batch review should include rule rows.")
        try expectContains(calls, "\"include_agent\":true", "Batch review should include agent rows.")
        try expectContains(calls, "\"include_workspace\":true", "Batch review should include workspace rows.")
        try expectContains(calls, "\"include_blocked\":true", "Batch review should keep blockers visible.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Batch review must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Batch review must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Batch review must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Batch review must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforeReview, "Batch review must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Batch review must not call credential paths.")
    }

    private func remediationBatchReviewFallsBackWhenMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.reviewRemediationBatch()

        try expectEqual(store.remediationBatchReviewResult?.isUnavailable, true, "Batch review should expose unavailable fallback for older services.")
        try expectEqual(store.remediationBatchReviewResult?.fallbackReason, UIStrings.remediationBatchReviewUnavailable, "Unknown method fallback should use the localized unavailable copy.")
        try expectFalse(store.isReviewingRemediationBatch, "Unavailable batch review should reset loading state.")
        try expectContains(fake.calls(), "remediation.batchReview", "Fallback should still prove the intended V2.59 method was attempted.")
    }

    private func remediationHistoryUsesLocalServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.routingConfidenceText = "Prepare local release audit work."
        await store.reload()
        await store.reviewRemediationBatch()
        let snapshotCallsBeforeHistory = countOccurrences("snapshot.", in: fake.calls())
        await store.loadRemediationHistory()
        await store.recordRemediationHistory()

        let history = store.remediationHistoryResult
        try expectEqual(history?.generatedBy, "local-v2.60", "Remediation history should expose generator metadata.")
        try expectEqual(history?.summary.totalCount, 2, "Remediation history should expose total record count.")
        try expectEqual(history?.summary.recurrenceCount, 1, "Remediation history should expose recurrence count.")
        try expectEqual(history?.summary.reopenedCount, 1, "Remediation history should expose reopened count.")
        try expectEqual(history?.summary.readinessImprovementCount, 1, "Remediation history should expose readiness improvement count.")
        try expectEqual(history?.records.first?.title, "Network permission reviewed", "Remediation history should expose record rows.")
        try expectEqual(history?.records.first?.skill?.skillName, "Beta", "Remediation history should expose affected skill evidence.")
        try expectEqual(history?.records.first?.sourceMethod, "remediation.previewDrafts", "Remediation history should expose source method context.")
        try expectEqual(history?.evidenceReferences.first?.title, "History", "Remediation history should expose evidence references.")
        try expectEqual(history?.promptRequest?.requestKind, "remediation_history", "Remediation history prompt metadata should use V2.60 request kind.")
        try expectFalse(history?.safetyFlags.providerRequestSent ?? true, "Remediation history list must not send provider requests.")
        try expectFalse(history?.safetyFlags.writeBackAllowed ?? true, "Remediation history list must not allow write-back.")
        try expectFalse(history?.safetyFlags.writeActionsAvailable ?? true, "Remediation history list must not expose write actions.")
        try expectFalse(history?.safetyFlags.scriptExecutionAllowed ?? true, "Remediation history list must not allow script execution.")
        try expectFalse(history?.safetyFlags.executionActionsAvailable ?? true, "Remediation history list must not expose execution actions.")
        try expectFalse(history?.safetyFlags.configMutationAllowed ?? true, "Remediation history list must not mutate config.")
        try expectFalse(history?.safetyFlags.snapshotCreated ?? true, "Remediation history list must not create snapshots.")
        try expectFalse(history?.safetyFlags.triageMutationAllowed ?? true, "Remediation history list must not mutate triage.")
        try expectFalse(history?.safetyFlags.credentialAccessed ?? true, "Remediation history list must not access credentials.")
        try expectFalse(history?.safetyFlags.rawPromptPersisted ?? true, "Remediation history list must not persist raw prompts.")
        try expectFalse(history?.safetyFlags.rawResponsePersisted ?? true, "Remediation history list must not persist raw responses.")
        try expectFalse(history?.safetyFlags.rawTracePersisted ?? true, "Remediation history list must not persist raw traces.")
        try expectFalse(history?.safetyFlags.cloudSyncEnabled ?? true, "Remediation history list must not sync cloud data.")
        try expectFalse(history?.safetyFlags.telemetryEnabled ?? true, "Remediation history list must not emit telemetry.")

        let record = store.remediationHistoryRecordResult
        try expectEqual(record?.recorded, true, "Record history should report local audit persistence.")
        try expectEqual(record?.record?.sourceMethod, "analysis.remediationHistory.ui", "Record history should identify the native audit source.")
        try expectFalse(record?.safetyFlags.providerRequestSent ?? true, "Record history must not send provider requests.")
        try expectFalse(record?.safetyFlags.writeActionsAvailable ?? true, "Record history must not expose write actions.")
        try expectFalse(record?.safetyFlags.snapshotCreated ?? true, "Record history must not create snapshots.")
        try expectFalse(record?.safetyFlags.triageMutationAllowed ?? true, "Record history must not mutate triage.")
        try expectFalse(store.isLoadingRemediationHistory, "Remediation history should reset loading state.")
        try expectFalse(store.isRecordingRemediationHistory, "Record history should reset loading state.")

        let calls = fake.calls()
        try expectContains(calls, "remediation.listHistory", "History UI should call the V2.60 list method.")
        try expectContains(calls, "remediation.recordHistory", "History UI should call the V2.60 record method.")
        try expectContains(calls, "\"task\":\"Prepare local release audit work.\"", "History should pass current task context when present.")
        try expectContains(calls, "\"agent\":\"claude-code\"", "History should pass the current agent filter.")
        try expectContains(calls, "\"selected_skill_id\":\"beta\"", "History should pass selected skill context.")
        try expectContains(calls, "\"limit\":30", "History list should pass the history limit.")
        try expectContains(calls, "\"decision\":\"reviewed\"", "Record history should store an audit decision label.")
        try expectContains(calls, "\"status\":\"recorded\"", "Record history should store an audit status label.")
        try expectContains(calls, "\"source_method\":\"analysis.remediationHistory.ui\"", "Record history should identify the native UI source.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Remediation history must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Remediation history must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Remediation history must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Remediation history must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforeHistory, "Remediation history must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Remediation history must not call credential paths.")
    }

    private func remediationHistoryFallsBackWhenMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.loadRemediationHistory()
        await store.recordRemediationHistory()

        try expectEqual(store.remediationHistoryResult?.isUnavailable, true, "Remediation history should expose unavailable fallback for older services.")
        try expectEqual(store.remediationHistoryResult?.fallbackReason, UIStrings.remediationHistoryUnavailable, "Unknown list method fallback should use localized unavailable copy.")
        try expectEqual(store.remediationHistoryRecordResult?.isUnavailable, true, "Record history should expose unavailable fallback for older services.")
        try expectEqual(store.remediationHistoryRecordResult?.fallbackReason, UIStrings.remediationHistoryRecordUnavailable, "Unknown record method fallback should use localized unavailable copy.")
        try expectFalse(store.isLoadingRemediationHistory, "Unavailable history list should reset loading state.")
        try expectFalse(store.isRecordingRemediationHistory, "Unavailable record history should reset loading state.")
        try expectContains(fake.calls(), "remediation.listHistory", "Fallback should still prove the intended V2.60 list method was attempted.")
        try expectContains(fake.calls(), "remediation.recordHistory", "Fallback should still prove the intended V2.60 record method was attempted.")
    }

    private func guidedCleanupFlowUsesLocalServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.routingConfidenceText = "Prepare local release audit work."
        await store.reload()
        let snapshotCallsBeforeGuided = countOccurrences("snapshot.", in: fake.calls())
        await store.planGuidedCleanupFlow()
        await store.recordGuidedCleanupStep()

        let result = store.guidedCleanupFlowResult
        try expectEqual(result?.generatedBy, "local-v2.67", "Guided cleanup should expose generator metadata.")
        try expectEqual(result?.summary.stepCount, 2, "Guided cleanup should expose step count.")
        try expectEqual(result?.summary.issueGroupCount, 1, "Guided cleanup should expose issue group count.")
        try expectEqual(result?.summary.safeActionCount, 2, "Guided cleanup should expose safe action count.")
        try expectEqual(result?.flowSteps.first?.id, "step-review-permission", "Guided cleanup should expose flow steps.")
        try expectEqual(result?.flowSteps.first?.recommended, true, "Guided cleanup should expose recommended steps.")
        try expectEqual(result?.flowSteps.first?.skill?.skillName, "Beta", "Guided cleanup should expose affected skill evidence.")
        try expectEqual(result?.flowSteps.first?.safeEntryMethod, "remediation.previewDrafts", "Guided cleanup should expose safe entry methods.")
        try expectEqual(result?.flowSteps.first?.safeActionDeepLink.canApply, false, "Guided cleanup step safe links must not apply changes.")
        try expectEqual(result?.issueGroups.first?.title, "Permission clarity", "Guided cleanup should expose issue groups.")
        try expectEqual(result?.safeNextActions.first?.canApplyFix, false, "Guided cleanup safe actions should stay non-applying.")
        try expectEqual(result?.safeNextActions.first?.entryMethod, "remediation.previewDrafts", "Guided cleanup safe actions should expose existing entry methods.")
        try expectEqual(result?.safeNextActions.first?.copyOnly, true, "Guided cleanup safe actions should preserve copy-only semantics.")
        try expectEqual(result?.safeNextActions.first?.deepLink.trigger, "previewRemediationDrafts", "Guided cleanup safe actions should expose safe link triggers.")
        try expectEqual(result?.safeNextActions.first?.deepLink.canApply, false, "Guided cleanup safe action deep links must not apply changes.")
        try expectEqual(result?.recordedSteps.first?.sourceMethod, "cleanup.recordGuidedStep", "Guided cleanup should expose existing recorded metadata.")
        try expectEqual(result?.evidenceReferences.first?.source, "cleanup.planGuidedFlow", "Guided cleanup should expose evidence references.")
        try expectEqual(result?.promptRequest?.requestKind, "guided_cleanup_flow", "Guided cleanup should expose prompt metadata.")
        try expectFalse(result?.safetyFlags.providerRequestSent ?? true, "Guided cleanup planning must not send provider requests.")
        try expectFalse(result?.safetyFlags.writeBackAllowed ?? true, "Guided cleanup planning must not allow write-back.")
        try expectFalse(result?.safetyFlags.writeActionsAvailable ?? true, "Guided cleanup planning must not expose write actions.")
        try expectFalse(result?.safetyFlags.scriptExecutionAllowed ?? true, "Guided cleanup planning must not allow script execution.")
        try expectFalse(result?.safetyFlags.executionActionsAvailable ?? true, "Guided cleanup planning must not expose execution actions.")
        try expectFalse(result?.safetyFlags.configMutationAllowed ?? true, "Guided cleanup planning must not mutate config.")
        try expectFalse(result?.safetyFlags.snapshotCreated ?? true, "Guided cleanup planning must not create snapshots.")
        try expectFalse(result?.safetyFlags.triageMutationAllowed ?? true, "Guided cleanup planning must not mutate triage.")
        try expectFalse(result?.safetyFlags.credentialAccessed ?? true, "Guided cleanup planning must not access credentials.")
        try expectFalse(result?.safetyFlags.rawPromptPersisted ?? true, "Guided cleanup planning must not persist raw prompts.")
        try expectFalse(result?.safetyFlags.rawResponsePersisted ?? true, "Guided cleanup planning must not persist raw responses.")
        try expectFalse(result?.safetyFlags.rawTracePersisted ?? true, "Guided cleanup planning must not persist raw traces.")
        try expectFalse(result?.safetyFlags.cloudSyncEnabled ?? true, "Guided cleanup planning must not sync cloud data.")
        try expectFalse(result?.safetyFlags.telemetryEnabled ?? true, "Guided cleanup planning must not emit telemetry.")

        let record = store.guidedCleanupRecordResult
        try expectEqual(record?.recorded, true, "Guided cleanup record should report local metadata persistence.")
        try expectEqual(record?.record?.sourceMethod, "analysis.guidedCleanupFlow.ui", "Guided cleanup record should identify the native UI source.")
        try expectEqual(record?.record?.appLocalOnly, true, "Guided cleanup record should remain app-local.")
        try expectEqual(record?.metadataRedacted, true, "Guided cleanup record should be marked redacted.")
        try expectFalse(record?.safetyFlags.providerRequestSent ?? true, "Guided cleanup record must not send provider requests.")
        try expectFalse(record?.safetyFlags.writeActionsAvailable ?? true, "Guided cleanup record must not expose write actions.")
        try expectFalse(record?.safetyFlags.snapshotCreated ?? true, "Guided cleanup record must not create snapshots.")
        try expectFalse(record?.safetyFlags.triageMutationAllowed ?? true, "Guided cleanup record must not mutate triage.")
        try expectFalse(store.isPlanningGuidedCleanupFlow, "Guided cleanup planning should reset loading state.")
        try expectFalse(store.isRecordingGuidedCleanupStep, "Guided cleanup recording should reset loading state.")

        if let link = result?.safeNextActions.first?.deepLink {
            await store.openGuidedCleanupSafeLink(link)
        } else {
            throw NativeModelTestFailure(description: "Guided cleanup should include a safe action deep link.")
        }
        try expectEqual(store.selectedDetailSection, .analysis, "Opening a guided cleanup safe link should navigate to the existing Analysis surface.")
        try expectEqual(store.remediationPreviewDraftsResult?.generatedBy, "local-v2.57", "Opening a copy-only draft link should invoke the existing preview method.")

        let calls = fake.calls()
        try expectContains(calls, "cleanup.planGuidedFlow", "Guided cleanup UI should call the V2.67 plan method.")
        try expectContains(calls, "cleanup.recordGuidedStep", "Guided cleanup UI should call the V2.67 record method.")
        try expectContains(calls, "remediation.previewDrafts", "Guided cleanup safe links should call existing read-only preview methods.")
        try expectContains(calls, "\"task\":\"Prepare local release audit work.\"", "Guided cleanup should pass current task context when present.")
        try expectContains(calls, "\"agent\":\"claude-code\"", "Guided cleanup should pass the current agent filter.")
        try expectContains(calls, "\"selected_skill_id\":\"beta\"", "Guided cleanup should pass selected skill context.")
        try expectContains(calls, "\"selected_skill_name\":\"Beta\"", "Guided cleanup should pass selected skill name.")
        try expectContains(calls, "\"limit\":12", "Guided cleanup should pass the guided flow limit.")
        try expectContains(calls, "\"include_issue_groups\":true", "Guided cleanup should request issue groups.")
        try expectContains(calls, "\"include_safe_next_actions\":true", "Guided cleanup should request safe next actions.")
        try expectContains(calls, "\"include_recorded_steps\":true", "Guided cleanup should request app-local recorded steps.")
        try expectContains(calls, "\"app_language\":\"en\"", "Guided cleanup should pass app language for localized service rows.")
        try expectContains(calls, "\"step_id\":\"step-review-permission\"", "Guided cleanup record should pass selected/recommended step id.")
        try expectContains(calls, "\"source_method\":\"analysis.guidedCleanupFlow.ui\"", "Guided cleanup record should identify the native source.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Guided cleanup must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Guided cleanup must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Guided cleanup must not call config write paths.")
        try expectFalse(calls.contains("batch.applySkillToggles"), "Guided cleanup must not call batch apply.")
        try expectFalse(calls.contains("script.execute"), "Guided cleanup must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforeGuided, "Guided cleanup must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Guided cleanup must not call credential paths.")
    }

    private func guidedCleanupFlowFallsBackWhenMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.planGuidedCleanupFlow()
        await store.recordGuidedCleanupStep()

        try expectEqual(store.guidedCleanupFlowResult?.isUnavailable, true, "Guided cleanup should expose unavailable fallback for older services.")
        try expectEqual(store.guidedCleanupFlowResult?.fallbackReason, UIStrings.guidedCleanupFlowUnavailable, "Unknown plan method fallback should use localized unavailable copy.")
        try expectEqual(store.guidedCleanupRecordResult?.isUnavailable, true, "Guided cleanup record should expose unavailable fallback when no step exists.")
        try expectEqual(store.guidedCleanupRecordResult?.fallbackReason, UIStrings.guidedCleanupFlowNoSteps, "Missing step fallback should use no-steps copy before attempting record.")
        try expectFalse(store.isPlanningGuidedCleanupFlow, "Unavailable guided cleanup planning should reset loading state.")
        try expectFalse(store.isRecordingGuidedCleanupStep, "Unavailable guided cleanup recording should reset loading state.")
        try expectContains(fake.calls(), "cleanup.planGuidedFlow", "Fallback should still prove the intended V2.67 plan method was attempted.")
        try expectFalse(fake.calls().contains("cleanup.recordGuidedStep"), "Record fallback without a loaded step must not call service record.")
        try expectFalse(fake.calls().contains("llm.confirmPromptAndSend"), "Unavailable guided cleanup must not send to provider.")
        try expectFalse(fake.calls().contains("config.toggleSkill"), "Unavailable guided cleanup must not call config write paths.")
        try expectFalse(fake.calls().contains("script.execute"), "Unavailable guided cleanup must not call execution paths.")
    }

    private func crossAgentReadinessUsesReadOnlyServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.routingConfidenceText = "Route a local audit release note task."
        await store.reload()
        let snapshotCallsBeforeCompare = countOccurrences("snapshot.", in: fake.calls())
        await store.compareCrossAgentReadiness()

        let result = store.crossAgentReadinessResult
        try expectEqual(result?.generatedBy, "local-v2.50", "Cross-agent readiness should expose generator metadata.")
        try expectEqual(result?.recommendedAgent?.agent, "claude-code", "Cross-agent readiness should expose recommended agent.")
        try expectEqual(result?.recommendedAgent?.comparisonScore, 93, "Cross-agent readiness should expose recommendation comparison score.")
        try expectEqual(result?.recommendedAgent?.skill?.name, "Beta", "Cross-agent readiness should expose recommended skill.")
        try expectEqual(result?.agentRows.map(\.agent), ["claude-code", "codex"], "Cross-agent readiness should expose per-agent rows.")
        try expectEqual(result?.agentRows.first?.rank, 1, "Cross-agent readiness should expose agent rank.")
        try expectEqual(result?.agentRows.first?.comparisonScore, 93, "Cross-agent readiness should expose comparison score.")
        try expectEqual(result?.agentRows.first?.readinessScore, 88, "Cross-agent readiness should expose readiness score.")
        try expectEqual(result?.agentRows.first?.routingScore, 91, "Cross-agent readiness should expose routing score.")
        try expectEqual(result?.agentRows.first?.bestCandidateSkill?.definitionID, "def.beta", "Cross-agent readiness should expose best candidate definition id.")
        try expectEqual(result?.agentRows.first?.enabledState, "Enabled", "Cross-agent readiness should expose nested enabled state.")
        try expectEqual(result?.agentRows.first?.scopeState, "agent-project", "Cross-agent readiness should expose nested scope state.")
        try expectEqual(result?.agentRows.first?.riskState, "low", "Cross-agent readiness should expose nested risk state.")
        try expectEqual(result?.agentRows.first?.accuracyContext, "7 of 8 known traces hit expected route.", "Cross-agent readiness should expose routing accuracy context.")
        try expectEqual(result?.agentRows.first?.benchmarkContext, "Benchmark expected route is covered.", "Cross-agent readiness should expose benchmark context.")
        try expectEqual(result?.gapIssueRows.first?.title, "Missing Codex benchmark", "Cross-agent readiness should expose gap rows.")
        try expectEqual(result?.evidenceReferences.first?.title, "Benchmark", "Cross-agent readiness should expose evidence references.")
        try expectFalse(result?.safetyFlags.providerRequestSent ?? true, "Cross-agent readiness must not send provider requests.")
        try expectFalse(result?.safetyFlags.writeBackAllowed ?? true, "Cross-agent readiness must not allow write-back.")
        try expectFalse(result?.safetyFlags.writeActionsAvailable ?? true, "Cross-agent readiness must not expose write actions.")
        try expectFalse(result?.safetyFlags.scriptExecutionAllowed ?? true, "Cross-agent readiness must not allow script execution.")
        try expectFalse(result?.safetyFlags.executionActionsAvailable ?? true, "Cross-agent readiness must not expose execution actions.")
        try expectFalse(result?.safetyFlags.configMutationAllowed ?? true, "Cross-agent readiness must not mutate config.")
        try expectFalse(result?.safetyFlags.snapshotCreated ?? true, "Cross-agent readiness must not create snapshots.")
        try expectFalse(result?.safetyFlags.triageMutationAllowed ?? true, "Cross-agent readiness must not mutate triage.")
        try expectFalse(result?.safetyFlags.credentialAccessed ?? true, "Cross-agent readiness must not access credentials.")
        try expectFalse(result?.safetyFlags.rawPromptPersisted ?? true, "Cross-agent readiness must not persist raw prompts.")
        try expectFalse(result?.safetyFlags.rawResponsePersisted ?? true, "Cross-agent readiness must not persist raw responses.")
        try expectFalse(result?.safetyFlags.rawTracePersisted ?? true, "Cross-agent readiness must not persist raw traces.")
        try expectFalse(result?.safetyFlags.cloudSyncEnabled ?? true, "Cross-agent readiness must not sync cloud data.")
        try expectFalse(result?.safetyFlags.telemetryEnabled ?? true, "Cross-agent readiness must not emit telemetry.")
        try expectFalse(store.isComparingCrossAgentReadiness, "Cross-agent readiness compare should reset loading state.")

        let calls = fake.calls()
        try expectContains(calls, "task.compareAgentReadiness", "Cross-agent readiness should call the V2.50 compare method.")
        try expectContains(calls, "\"task\":\"Route a local audit release note task.\"", "Cross-agent readiness should send canonical task text.")
        try expectContains(calls, "\"limit_per_agent\":3", "Cross-agent readiness should send the per-agent limit.")
        try expectContains(calls, "\"include_routing_accuracy\":true", "Cross-agent readiness should request accuracy context explicitly.")
        try expectContains(calls, "\"include_benchmarks\":true", "Cross-agent readiness should request benchmark context explicitly.")
        try expectFalse(calls.contains("\"task_text\":\"Route a local audit release note task.\""), "Local compare must not send duplicate task_text aliases.")
        try expectFalse(calls.contains("\"user_intent\":\"Route a local audit release note task.\""), "Local compare must not send duplicate user_intent aliases.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Cross-agent readiness must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Cross-agent readiness must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Cross-agent readiness must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Cross-agent readiness must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforeCompare, "Cross-agent readiness must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Cross-agent readiness must not call credential paths.")
    }

    private func routingConfidenceClearsStaleSelection() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.routingConfidenceText = "Route a local audit release note task."
        await store.reload()
        await store.rankSelectedSkillRoutes()

        guard let beta = store.selectedSkill else {
            throw NativeModelTestFailure(description: "Fixture should select beta before stale routing cleanup.")
        }
        guard store.routingConfidence(for: beta) != nil else {
            throw NativeModelTestFailure(description: "Routing result should exist for beta before selection changes.")
        }

        store.agentFilter = .codex
        try await waitUntil("Agent filter should move selection away from beta for route cleanup.") {
            store.selectedSkillID == "gamma"
        }

        guard let gamma = store.selectedSkill else {
            throw NativeModelTestFailure(description: "Fixture should select gamma after agent filter changes.")
        }
        try expectNil(store.routingConfidence(for: gamma), "Routing confidence prepared for beta must not be reused after selecting another agent skill.")
    }

    private func llmPreparePreviewIsScopedToSelectedSkillAndReadOnly() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "llm-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.prepareAnalyzeLLM()

        try expectEqual(store.llmPrepareResult(for: .analyze)?.action, .analyze, "Beta analyze preview should be available while beta is selected.")

        store.agentFilter = .codex
        try await waitUntil("Agent filter should move selection away from beta.") {
            store.selectedSkillID == "gamma"
        }

        try expectNil(store.llmPrepareResult(for: .analyze), "LLM preview prepared for beta must not be reused after selecting another agent skill.")
        await store.prepareAnalyzeLLM()

        let calls = fake.calls()
        try expectContains(calls, "\"instance_id\":\"beta\"", "LLM prepare should send the beta instance context.")
        try expectContains(calls, "\"agent\":\"claude-code\"", "LLM prepare should send beta's agent context.")
        try expectContains(calls, "\"instance_id\":\"gamma\"", "LLM prepare should send the gamma instance context after selection changes.")
        try expectContains(calls, "\"agent\":\"codex\"", "LLM prepare should send gamma's agent context.")
        try expectFalse(calls.contains("llm.complete"), "LLM prepare must not call a provider completion method.")
        try expectFalse(calls.contains("config.toggleSkill"), "LLM analysis preview must not call write paths.")
        try expectFalse(calls.contains("script.execute"), "LLM analysis preview must not call execution paths.")
    }

    private func promptPreviewRequiresConfiguredProviderAndExplicitSend() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.prepareAnalyzeLLM()
        await store.previewPromptForSelectedLLMAction(.analyze)

        let preview = store.llmPromptPreview(for: .analyze)
        try expectEqual(preview?.previewID, "prompt-preview-beta", "Prompt preview should be stored for the selected skill/action.")
        try expectEqual(preview?.destinationHost, "llm.example.com", "Prompt preview should expose network destination.")
        try expectEqual(preview?.includedFields.map(\.name), ["skill.name", "findings.summary"], "Prompt preview should expose included fields.")
        try expectEqual(store.canSendLLMPrompt(for: .analyze), true, "Configured provider and current preview should allow explicit send.")

        await store.confirmPromptForSelectedLLMAction(.analyze)
        let sendResult = store.llmPromptSendResult(for: .analyze)
        try expectEqual(sendResult?.success, true, "Confirmed prompt should store provider result.")
        try expectEqual(sendResult?.outputText, "Read-only analysis for Beta.", "Provider output should be retained as copy-only text.")
        try expectFalse(sendResult?.writeBackAllowed ?? true, "Provider result must not enable write-back.")
        try expectFalse(sendResult?.scriptExecutionAllowed ?? true, "Provider result must not enable script execution.")

        let calls = fake.calls()
        try expectContains(calls, "llm.previewPrompt", "Prompt preview should use the V2.42 preview method.")
        try expectContains(calls, "\"preview_id\":\"prompt-preview-beta\"", "Confirm should send the preview id.")
        try expectContains(calls, "llm.confirmPromptAndSend", "Explicit send should use the V2.42 confirmation method.")
        try expectFalse(calls.contains("config.toggleSkill"), "Prompt confirmation must not call write paths.")
        try expectFalse(calls.contains("script.execute"), "Prompt confirmation must not call execution paths.")
    }

    private func previewScriptExecutionSafetyStoresBlockedPreviewWithoutExecute() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "script-preview")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        guard let skill = store.selectedSkill else {
            throw NativeModelTestFailure(description: "Fixture should select beta for script preview.")
        }

        await store.previewScriptExecutionSafety(for: skill)

        let preview = store.scriptExecutionPreview(for: skill)
        try expectEqual(preview?.skillID, "beta", "Script preview should be stored by skill ID.")
        try expectEqual(preview?.commandPreview, ["bash", "scripts/setup.sh"], "Script preview should expose command preview.")
        try expectEqual(preview?.scope.cwd, "/tmp/project", "Script preview should expose CWD.")
        try expectEqual(preview?.scope.env["SKILLS_SAFE_MODE"], "1", "Script preview should expose env.")
        try expectEqual(preview?.scope.network, "none", "Script preview should expose network scope.")
        try expectEqual(preview?.scope.files, ["/tmp/project/scripts/setup.sh"], "Script preview should expose file scope.")
        try expectEqual(preview?.executionAllowed, false, "Script preview should keep execution blocked.")
        try expectEqual(preview?.confirmationRequired, true, "Script preview should require confirmation.")
        try expectEqual(preview?.auditStatus, .blocked, "Script preview should expose audit status.")
        try expectContains(fake.calls(), "script.previewExecution", "Script safety card should use preview preflight.")
        try expectFalse(fake.calls().contains("script.execute"), "Script safety card must not call an execution method.")
    }

    private func waitUntil(_ label: String, timeout: TimeInterval = 2, predicate: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while !predicate() {
            if Date() > deadline {
                throw NativeModelTestFailure(description: label)
            }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
    }

    private func countOccurrences(_ needle: String, in haystack: String) -> Int {
        haystack.components(separatedBy: needle).count - 1
    }

    private func countMethodCalls(_ method: String, in calls: String) -> Int {
        countOccurrences("\"method\":\"\(method)\"", in: calls)
    }

    private func makeTemporaryTaskCockpitHistoryStore() -> TaskCockpitHistoryStore {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("skills-copilot-task-preflight-history-\(UUID().uuidString)", isDirectory: true)
        return TaskCockpitHistoryStore(fileURL: directory.appendingPathComponent("history.json"))
    }

    private func cleanupTaskCockpitHistoryStore(_ store: TaskCockpitHistoryStore) {
        try? FileManager.default.removeItem(at: store.fileURL.deletingLastPathComponent())
    }

    private func permissionMarker(_ detail: SkillDetailRecord?) -> String? {
        guard
            case .object(let permissions)? = detail?.permissions,
            case .string(let marker)? = permissions["marker"]
        else {
            return nil
        }
        return marker
    }
}
