import Darwin
import Combine
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
        try await runCase("startupPreviewRequestsSummaryRows") {
            try await startupPreviewRequestsSummaryRows()
        }
        try await runCase("selectingSummaryRequestsOnlySelectedDetail") {
            try await selectingSummaryRequestsOnlySelectedDetail()
        }
        try await runCase("sessionCriteriaChangesAndGlobalSearchUseNoRPC") {
            try await sessionCriteriaChangesAndGlobalSearchUseNoRPC()
        }
        try await runCase("failedSummaryAndDetailKeepSummaryStateIsolated") {
            try await failedSummaryAndDetailKeepSummaryStateIsolated()
        }
        try await runCase("localSessionScopeChangeRequestsServerPage") {
            try await localSessionScopeChangeRequestsServerPage()
        }
        try await runCase("localSessionProjectScopeUsesProjectRootFromAllScopeCache") {
            try await localSessionProjectScopeUsesProjectRootFromAllScopeCache()
        }
        try await runCase("localSessionSortChangesRequestServerPage") {
            try await localSessionSortChangesRequestServerPage()
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
        try await runCase("partialScanWarningFollowsCompleteLegacyAndActivitylessLifecycle") {
            try await partialScanWarningFollowsCompleteLegacyAndActivitylessLifecycle()
        }
        try await runCase("searchAndFilterChangesNormalizeSelectionAndDetail") {
            try await searchAndFilterChangesNormalizeSelectionAndDetail()
        }
        try await runCase("appSearchSkillSelectionSynchronizesListAndDetail") {
            try await appSearchSkillSelectionSynchronizesListAndDetail()
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
        try await runCase("settingsFeedbackClearsAfterFailedConfigSave") {
            try await settingsFeedbackClearsAfterFailedConfigSave()
        }
        try await runCase("configAutosaveKeepsEditArrivingDuringSave") {
            try await configAutosaveKeepsEditArrivingDuringSave()
        }
        try await runCase("queuedConfigAutosaveNeverInheritsExternalPostSaveRevision") {
            try await queuedConfigAutosaveNeverInheritsExternalPostSaveRevision()
        }
        try await runCase("olderConfigAutosaveFeedbackDoesNotPublishForNewerDraft") {
            try await olderConfigAutosaveFeedbackDoesNotPublishForNewerDraft()
        }
        try await runCase("providerAutosaveKeepsEditArrivingDuringSave") {
            try await providerAutosaveKeepsEditArrivingDuringSave()
        }
        try await runCase("olderProviderAutosaveFeedbackDoesNotPublishForNewerDraft") {
            try await olderProviderAutosaveFeedbackDoesNotPublishForNewerDraft()
        }
        try await runCase("configAutosaveQueuesRevertDuringActiveSave") {
            try await configAutosaveQueuesRevertDuringActiveSave()
        }
        try await runCase("providerAutosaveQueuesRevertDuringActiveSave") {
            try await providerAutosaveQueuesRevertDuringActiveSave()
        }
        try await runCase("configAndProviderAutosavesShareOneMutationLane") {
            try await configAndProviderAutosavesShareOneMutationLane()
        }
        try await runCase("cancellingConfigAutosaveBeforeWaiterRegistrationMakesZeroRPCs") {
            try await cancellingConfigAutosaveBeforeWaiterRegistrationMakesZeroRPCs()
        }
        try await runCase("cancellingProviderAutosaveBeforeWaiterRegistrationMakesZeroRPCs") {
            try await cancellingProviderAutosaveBeforeWaiterRegistrationMakesZeroRPCs()
        }
        try await runCase("cancellingQueuedProviderAutosaveMakesZeroRPCs") {
            try await cancellingQueuedProviderAutosaveMakesZeroRPCs()
        }
        try await runCase("cancellingQueuedConfigAutosaveMakesZeroRPCs") {
            try await cancellingQueuedConfigAutosaveMakesZeroRPCs()
        }
        try await runCase("invalidConfigDraftCancelsQueuedValidAutosave") {
            try await invalidConfigDraftCancelsQueuedValidAutosave()
        }
        try await runCase("invalidProviderDraftCancelsQueuedValidAutosave") {
            try await invalidProviderDraftCancelsQueuedValidAutosave()
        }
        try await runCase("releasingStoreCancelsQueuedAutosaveWaiter") {
            try await releasingStoreCancelsQueuedAutosaveWaiter()
        }
        try await runCase("configSaveInvalidatesInFlightConfigReaders") {
            try await configSaveInvalidatesInFlightConfigReaders()
        }
        try await runCase("configSaveRefreshesTheSubmittedAgentAfterSelectionChanges") {
            try await configSaveRefreshesTheSubmittedAgentAfterSelectionChanges()
        }
        try await runCase("providerUnknownMutationDoesNotReportSaved") {
            try await providerUnknownMutationDoesNotReportSaved()
        }
        try await runCase("successfulConfigAutosaveRetiresDraftAndAdoptsExternalRefresh") {
            try await successfulConfigAutosaveRetiresDraftAndAdoptsExternalRefresh()
        }
        try await runCase("successfulProviderAutosaveRetiresDraftAndAdoptsExternalRefresh") {
            try await successfulProviderAutosaveRetiresDraftAndAdoptsExternalRefresh()
        }
        try await runCase("failedConfigAutosaveKeepsDraft") {
            try await failedConfigAutosaveKeepsDraft()
        }
        try await runCase("configSaveUsesLoadedRevision") {
            try await configSaveUsesLoadedRevision()
        }
        try await runCase("configSaveNeverFallsBackWithoutRevision") {
            try await configSaveNeverFallsBackWithoutRevision()
        }
        try await runCase("configConflictPreservesDraftAndReloadsRevision") {
            try await configConflictPreservesDraftAndReloadsRevision()
        }
        try await runCase("pendingConfigDraftNeverRebindsToReloadedRevision") {
            try await pendingConfigDraftNeverRebindsToReloadedRevision()
        }
        try await runCase("protocolV1BindingsRemainReadOnlyBeforeAndAfterStatusLoad") {
            try await protocolV1BindingsRemainReadOnlyBeforeAndAfterStatusLoad()
        }
        try await runCase("protocolV2MissingBindingsRemainReadOnly") {
            try await protocolV2MissingBindingsRemainReadOnly()
        }
        try await runCase("previewRollbackShowsDiffWithoutCallingRollback") {
            try await previewRollbackShowsDiffWithoutCallingRollback()
        }
        try await runCase("rollbackUsesImmutablePreviewInputs") {
            try await rollbackUsesImmutablePreviewInputs()
        }
        try await runCase("staleRollbackTokenRequiresAnotherPreview") {
            try await staleRollbackTokenRequiresAnotherPreview()
        }
        try await runCase("inFlightRollbackFailureDoesNotPublishAfterSelectionChanges") {
            try await inFlightRollbackFailureDoesNotPublishAfterSelectionChanges()
        }
        try await runCase("mismatchedInFlightRollbackPreviewDoesNotPublishAfterSelectionChanges") {
            try await mismatchedInFlightRollbackPreviewDoesNotPublishAfterSelectionChanges()
        }
        try await runCase("rollbackConfirmationInvalidatesOnSelectionTimelineAndPreviewChanges") {
            try await rollbackConfirmationInvalidatesOnSelectionTimelineAndPreviewChanges()
        }
        try await runCase("inFlightRollbackPreviewCannotRestoreInvalidatedConfirmation") {
            try await inFlightRollbackPreviewCannotRestoreInvalidatedConfirmation()
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
        try await runCase("providerObservabilityUsesReadOnlyServiceContract") {
            try await providerObservabilityUsesReadOnlyServiceContract()
        }
        try await runCase("providerObservabilityPreloadsAtStartupAndRefreshesWithReload") {
            try await providerObservabilityPreloadsAtStartupAndRefreshesWithReload()
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
        try await runCase("taskCockpitHistoryStaysInCurrentSessionOnly") {
            try await taskCockpitHistoryStaysInCurrentSessionOnly()
        }
        try await runCase("taskCockpitHistoryKeepsNewestTwelveRecords") {
            try await taskCockpitHistoryKeepsNewestTwelveRecords()
        }
        try await runCase("newStoreDoesNotRestoreTaskCockpitHistory") {
            try await newStoreDoesNotRestoreTaskCockpitHistory()
        }
        try await runCase("successfulTaskCockpitDoesNotCreateHistoryFile") {
            try await successfulTaskCockpitDoesNotCreateHistoryFile()
        }
        try await runCase("clearTaskCockpitHistoryClearsMemory") {
            try await clearTaskCockpitHistoryClearsMemory()
        }
        try runCase("legacyHistoryCleanupFailureShowsRedactedMessage") {
            try legacyHistoryCleanupFailureShowsRedactedMessage()
        }
        try runCase("legacyHistoryDirectoryShowsRedactedMessageAndRemainsUntouched") {
            try legacyHistoryDirectoryShowsRedactedMessageAndRemainsUntouched()
        }
        try await runCase("taskCockpitPreservesExactUserInputInServiceContract") {
            try await taskCockpitPreservesExactUserInputInServiceContract()
        }
        try await runCase("taskCockpitWhitespaceOnlyInputRequiresTask") {
            try await taskCockpitWhitespaceOnlyInputRequiresTask()
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
        try expectNil(store.selectedSidebarSelection, "Agent Copilot should not expose a default detail selection.")
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
        let callsBeforeCriteriaChanges = countMethodCalls("session.previewLocalSessions", in: fake.calls())

        store.localSessionSearchText = "develop"

        try expectEqual(store.filteredLocalSessionRows.map(\.id), ["session-develop"], "Session search should narrow visible rows.")
        try expectEqual(store.selectedLocalSessionID, "session-develop", "Search should move selection to the visible session.")
        try expectEqual(store.selectedSidebarSelection, .session("session-develop"), "Detail selection should follow the searched session.")
        try expectEqual(store.selectedLocalSession?.title, "Switch to develop branch", "Detail model should expose the searched session.")
        try expectFalse(fake.calls().contains("\"search\":\"develop\""), "Session search should stay within the summary cache.")

        store.localSessionSearchText = "missing"

        try expectEqual(store.filteredLocalSessionRows.count, 0, "No-match search should show an empty session list.")
        try expectNil(store.selectedLocalSessionID, "No-match search should clear stale session selection.")
        try expectNil(store.selectedSidebarSelection, "No-match search should clear stale session detail.")

        store.localSessionSearchText = ""

        try expectEqual(store.selectedLocalSessionID, "session-alpha", "Clearing search should restore the first visible session.")
        try expectEqual(store.selectedSidebarSelection, .session("session-alpha"), "Clearing search should restore session detail.")
        try expectEqual(countMethodCalls("session.previewLocalSessions", in: fake.calls()), callsBeforeCriteriaChanges, "Search criteria changes should issue no summary or detail reads.")
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

    private func startupPreviewRequestsSummaryRows() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "sessions")

        let store = SkillStore(service: fake.serviceClient())
        await store.refreshSelectedAgentLocalSessionsIfNeeded()

        let calls = fake.calls()
        try expectEqual(countMethodCalls("session.previewLocalSessions", in: calls), 1, "Startup/source prewarm should issue one summary request.")
        try expectContains(calls, #""include_content_items":false"#, "Summary request should explicitly omit content items.")
        try expectFalse(calls.contains(#""session_id""#), "Summary request should not send a session id.")
        try expectFalse(store.localSessionPreviewResult.sessionRows.contains { !$0.contentItems.isEmpty || $0.contentIncluded }, "Store summaries must retain no raw content items.")
    }

    private func selectingSummaryRequestsOnlySelectedDetail() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "sessions")

        let store = SkillStore(service: fake.serviceClient())
        store.sidebarContentMode = .sessions
        await store.refreshSelectedAgentLocalSessionsIfNeeded()
        guard let summary = store.filteredLocalSessionRows.first(where: { $0.id == "session-alpha" }) else {
            throw NativeModelTestFailure(description: "Summary fixture should contain session-alpha.")
        }

        store.selectLocalSession(summary)
        try await waitUntil("Selecting a summary should request exactly one detail.") {
            countMethodCalls("session.previewLocalSessions", in: fake.calls()) == 2
                && store.selectedLocalSessionDetailState != nil
        }
        let callsAfterDetail = fake.calls()
        try expectContains(callsAfterDetail, #""include_content_items":true"#, "Detail request should explicitly include content items.")
        try expectContains(callsAfterDetail, #""session_id":"session-alpha""#, "Detail request should target only the selected stable id.")
        try expectContains(callsAfterDetail, #""limit":1"#, "Detail request should request one row.")

        store.selectLocalSession(summary)
        try? await Task.sleep(nanoseconds: 80_000_000)
        try expectEqual(countMethodCalls("session.previewLocalSessions", in: fake.calls()), 2, "Re-selecting a cached detail should not issue another RPC.")
    }

    private func sessionCriteriaChangesAndGlobalSearchUseNoRPC() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "sessions-mixed")

        let store = SkillStore(service: fake.serviceClient())
        store.sidebarContentMode = .sessions
        await store.refreshSelectedAgentLocalSessionsIfNeeded()
        let initialCalls = countMethodCalls("session.previewLocalSessions", in: fake.calls())
        for index in 0..<20 {
            store.localSessionScopeFilter = index.isMultiple(of: 2) ? .all : .project
            store.localSessionSortOrder = index.isMultiple(of: 3) ? .title : .recent
            store.localSessionSortDirection = index.isMultiple(of: 2) ? .ascending : .descending
            store.localSessionSearchText = index.isMultiple(of: 4) ? "Analyze" : ""
        }
        store.searchText = "no-such-skill"
        try expectEqual(countMethodCalls("session.previewLocalSessions", in: fake.calls()), initialCalls, "Twenty criteria and skill-filter changes should issue no session RPCs.")
        try expectEqual(store.localSessionPreviewResult.sessionRows.count, 3, "Skill criteria should not clear session summaries.")

        store.updateAppSearch(query: "Analyze")
        try await waitUntil("Global search should read the summary index.") {
            store.appSearchResult.items.contains { $0.targetID == "session-alpha" }
        }
        let calls = fake.calls()
        try expectEqual(countMethodCalls("session.previewLocalSessions", in: calls), initialCalls, "Global search should not request session data.")
        try expectFalse(calls.contains("app.search"), "Global search should not call the service search method.")
        try expectFalse(store.appSearchResult.items.contains { !($0.session?.contentItems.isEmpty ?? true) }, "Global search should expose summary-only session records.")
    }

    private func failedSummaryAndDetailKeepSummaryStateIsolated() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "sessions")
        let store = SkillStore(service: fake.serviceClient())
        store.sidebarContentMode = .sessions
        await store.refreshSelectedAgentLocalSessionsIfNeeded()
        let summaryIDs = store.localSessionPreviewResult.sessionRows.map(\.id)

        fake.setScenario("normal")
        await store.previewLocalSessions()
        try expectEqual(store.localSessionPreviewResult.sessionRows.map(\.id), summaryIDs, "Failed explicit summary refresh should preserve stale summaries.")
        guard case .stale = store.localSessionLoadState else {
            throw NativeModelTestFailure(description: "Failed explicit summary refresh should publish stale state.")
        }

        fake.setScenario("sessions-detail-failure")
        guard let summary = store.filteredLocalSessionRows.first else {
            throw NativeModelTestFailure(description: "A stale snapshot should remain selectable.")
        }
        store.selectLocalSession(summary)
        try await waitUntil("Detail failure should stay in the detail state.") {
            if case .failed = store.selectedLocalSessionDetailState { return true }
            return false
        }
        try expectEqual(store.localSessionPreviewResult.sessionRows.map(\.id), summaryIDs, "Detail failure should not mutate the summary list.")
        try expectNil(store.errorMessage, "Detail failure should not publish to the global error banner.")
    }

    private func localSessionScopeChangeRequestsServerPage() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "sessions-mixed")

        let store = SkillStore(service: fake.serviceClient())
        store.sidebarContentMode = .sessions
        await store.previewLocalSessions()

        try expectEqual(store.localSessionPreviewResult.sessionRows.map(\.id), ["session-alpha", "session-develop", "session-global"], "Source snapshot should retain all summary rows.")
        try expectEqual(store.filteredLocalSessionRows.map(\.id), ["session-alpha", "session-develop"], "Project scope should show the server-filtered project rows.")
        try expectEqual(store.scopedLocalSessionUserMessageCount, 2, "Project scope metrics should be derived from project rows only.")
        try expectEqual(store.scopedLocalSessionTotalMessageCount, 4, "Project scope message totals should be derived from project rows only.")
        let callsBeforeScopeChange = countMethodCalls("session.previewLocalSessions", in: fake.calls())
        try expectContains(fake.calls(), "\"scope\":\"all\"", "Summary snapshot should load all scopes once.")

        store.localSessionScopeFilter = .all

        guard let globalSession = store.localSessionPreviewResult.sessionRows.first(where: { $0.id == "session-global" }) else {
            throw NativeModelTestFailure(description: "Fake session fixture should include the global session after all-scope refresh.")
        }

        try expectEqual(store.filteredLocalSessionRows.map(\.id), ["session-alpha", "session-develop", "session-global"], "All scope should request and reveal the global session.")
        try expectEqual(store.scopedLocalSessionUserMessageCount, 4, "All-scope metrics should include global rows.")
        try expectEqual(store.scopedLocalSessionSkillCallCount, 1, "All-scope skill metrics should include global rows.")
        try expectEqual(
            countMethodCalls("session.previewLocalSessions", in: fake.calls()),
            callsBeforeScopeChange,
            "Scope changes should project the cached summary without a service read."
        )

        store.selectLocalSession(globalSession, origin: .criteriaNormalization)
        store.localSessionScopeFilter = .project

        try expectEqual(store.filteredLocalSessionRows.map(\.id), ["session-alpha", "session-develop"], "Returning to project scope should hide global rows from the cached list.")
        try expectEqual(store.selectedLocalSessionID, "session-alpha", "Scope filtering should normalize a hidden global selection to the first visible project session.")
        try expectEqual(store.selectedSidebarSelection, .session("session-alpha"), "Detail selection should follow the locally visible session after scope filtering.")
        try expectEqual(
            countMethodCalls("session.previewLocalSessions", in: fake.calls()),
            callsBeforeScopeChange,
            "Returning to project scope should not request a service page."
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
        try expectContains(fake.calls(), "\"scope\":\"all\"", "Summary preview should load all scopes for local projection.")
        try expectEqual(
            store.localSessionPreviewResult.sessionRows.map(\.id),
            ["session-project-from-all", "session-global"],
            "Source snapshot should retain project and global summaries."
        )
        try expectEqual(
            store.filteredLocalSessionRows.map(\.id),
            ["session-project-from-all"],
            "Project scope should show the service-filtered project rows."
        )
        try expectEqual(store.scopedLocalSessionToolCallCount, 24, "Project scope metrics should include the project-root session.")

        store.localSessionScopeFilter = .all

        try expectEqual(
            store.filteredLocalSessionRows.map(\.id),
            ["session-project-from-all", "session-global"],
            "All scope should request and reveal global rows."
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

    private func localSessionSortChangesRequestServerPage() async throws {
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
            "Session sort controls should request ordered project rows from the service."
        )
        try expectEqual(
            countMethodCalls("session.previewLocalSessions", in: fake.calls()),
            callsBeforeSortChange,
            "Changing local session sort should not trigger a service refresh."
        )
        try expectFalse(fake.calls().contains("\"sort\":\"title\""), "Session sort order should stay local.")
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

    private func appSearchSkillSelectionSynchronizesListAndDetail() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()

        try expectEqual(store.selectedSkillID, "alpha", "Fixture should start on the first skill.")

        let searchItem = try decodeAppSearchItem(
            """
            {
              "id": "skill:beta",
              "kind": "skill",
              "target_id": "beta",
              "title": "Beta",
              "subtitle": "claude-code · agent-project",
              "agent": "claude-code",
              "skill": {
                "id": "beta",
                "agent": "claude-code",
                "scope": "agent-project",
                "path": "/tmp/project/beta/SKILL.md",
                "display_path": "/tmp/project/beta/SKILL.md",
                "definition_id": "def.beta",
                "name": "Beta",
                "state": "loaded",
                "enabled": true
              }
            }
            """
        )

        await store.selectAppSearchItem(searchItem)

        try expectEqual(store.sidebarContentMode, .skills, "Selecting a skill search result should switch to the skill list.")
        try expectEqual(store.selectedSkillID, "beta", "Selecting a skill search result should update the selected skill ID.")
        try expectEqual(store.selectedSidebarSelection, .skill("beta"), "Selecting a skill search result should select the matching sidebar row.")
        try expectEqual(store.selectedSkill?.id, "beta", "Selecting a skill search result should expose matching detail model state.")
        try expectEqual(store.selectedDetailSection, .overview, "Selecting a skill search result should reset the detail to overview.")
        try expectEqual(store.skillListScrollRequest?.skillID, "beta", "Selecting a skill search result should request the matching row be scrolled into view.")
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
        try expectEqual(countMethodCalls("snapshot.listAgentConfig", in: calls), 0, "Reload should not block the core refresh on selected agent config history.")
        try expectContains(calls, "llm.status", "Reload should preserve the separate LLM status behavior.")
        try expectContains(calls, "project.getContext", "Reload should preserve the separate project context behavior.")
        try await waitUntil("Reload should refresh selected agent config history in the background.") {
            countMethodCalls("snapshot.listAgentConfig", in: fake.calls()) == 1
        }
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
        try expectEqual(store.selectedSkillDetail?.id, "alpha", "Startup should prewarm the first selected skill detail.")
        try expectEqual(countOccurrences("app.stateSnapshot", in: calls), 1, "Startup should use the combined state snapshot once.")
        try expectEqual(countMethodCalls("snapshot.listAgentConfig", in: calls), 0, "Startup should not block the progress overlay on selected-agent config history.")
        try expectFalse(calls.contains("session.previewLocalSessions"), "Startup should not block the progress overlay on selected-agent local sessions.")
        try expectFalse(calls.contains("config.readAgentConfig"), "Startup should not block the progress overlay on selected-agent current config documents.")
        try expectContains(calls, "catalog.getSkill", "Startup should prewarm the selected skill detail.")
        try expectEqual(countMethodCalls("llm.listProviderProfiles", in: calls), 0, "Startup should not block the progress overlay on AI provider status.")
        try expectFalse(calls.contains("\"method\":\"catalog.scanAll\""), "Startup should not scan roots automatically.")
        try expectFalse(calls.contains("\"method\":\"config.toggleSkill\""), "Startup should not write agent config.")
        try await waitUntil("Startup should prewarm supplemental launch data in the background.") {
            store.localSessionPreviewResult.sessionRows.count == 2
                && countMethodCalls("snapshot.listAgentConfig", in: fake.calls()) == 1
                && fake.calls().contains("config.readAgentConfig")
                && countMethodCalls("llm.listProviderProfiles", in: fake.calls()) == 1
        }

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
        let runner = CatalogRefreshServiceRunner()

        let store = SkillStore(service: runner.serviceClient())
        await store.scanAll()

        try expectFalse(store.isScanning, "Scan should reset scanning state.")
        try expectNil(store.errorMessage, "Generic scan should not set an error on success.")
        try expectEqual(store.skills.count, 3, "Generic scan should refresh the catalog collections.")
        try expectEqual(store.skills.first { $0.id == "gamma" }?.agent, "codex", "Scan fixtures should exercise a Codex skill record.")
        try expectContains(store.refreshStatusMessage, "completed-partial", "A partial scan must remain visible in the primary refresh status.")
        try expectContains(store.refreshStatusMessage, "<adapter-root>/dangling-link", "The primary partial status should include the first redacted issue path.")
        try expectContains(store.refreshStatusMessage, "Review partial scan diagnostics.", "The primary partial status should include a recovery action.")
        try expectEqual(store.lastMutationMessage, store.refreshStatusMessage, "The visible detail feedback must surface the partial status instead of a generic success toast.")
        try expectEqual(store.partialScanWarningMessage, store.refreshStatusMessage, "Persistent partial feedback must not be coupled to later generic reload status text.")
        try expectEqual(store.lastScanActivity?.agentSummaries?.count, 3, "Scan should retain complete, partial, and skipped adapter diagnostics when the service provides them.")
        try expectEqual(store.lastScanActivity?.agentSummaries?.first { $0.agent == "opencode" }?.rootsSkipped, ["<adapter-root>/missing-opencode"], "Scan diagnostics should decode skipped roots.")
        try expectEqual(store.lastScanActivity?.agentSummaries?.first { $0.agent == "claude-code" }?.status, "completed-partial", "A partial adapter scan must not decode as completed.")
        try expectEqual(store.lastScanActivity?.agentSummaries?.first { $0.agent == "claude-code" }?.rootsPartial, ["<adapter-root>"], "Scan diagnostics should decode partial roots.")
        try expectEqual(store.lastScanActivity?.agentSummaries?.first { $0.agent == "claude-code" }?.scanIssues.first?.kind, "entry_unreadable", "Scan diagnostics should decode typed issue kinds.")
        try expectEqual(store.lastScanActivity?.agentSummaries?.first { $0.agent == "claude-code" }?.scanIssues.first?.path, "<adapter-root>/dangling-link", "Scan issue paths should stay redacted on the client.")
        store.agentFilter = .codex
        try expectEqual(store.selectedAgentRefreshSummary?.rootsScanned, ["$HOME/.agents/skills"], "Selected adapter diagnostics should follow the agent filter.")
        let calls = await runner.calls()
        try expectEqual(countOccurrences("app.stateSnapshot", in: calls), 1, "Scan should refresh collections with one app state snapshot call.")
        try expectEqual(countOccurrences("catalog.listSkills", in: calls), 0, "Scan refresh should not launch a separate skills list sidecar.")
        try expectEqual(countOccurrences("catalog.listFindings", in: calls), 0, "Scan refresh should not launch a separate findings list sidecar.")
        try expectEqual(countOccurrences("catalog.listConflicts", in: calls), 0, "Scan refresh should not launch a separate conflicts list sidecar.")
        try expectEqual(countMethodCalls("snapshot.list", in: calls), 0, "Scan refresh should not launch a global snapshots list sidecar.")
        try expectFalse(countMethodCalls("snapshot.listAgentConfig", in: calls) == 0, "Scan refresh should refresh at least one writable agent config history.")
        let partialWarning = store.partialScanWarningMessage
        await store.reload()
        try expectEqual(store.partialScanWarningMessage, partialWarning, "Reloading cached catalog data must not relabel or discard an unresolved partial-scan warning.")
        try expectFalse(store.refreshStatusMessage == partialWarning, "Generic reload status and persistent partial-scan warning should remain independent.")
    }

    private func partialScanWarningFollowsCompleteLegacyAndActivitylessLifecycle() async throws {
        let runner = CatalogRefreshServiceRunner(scanFixtures: [
            .partial,
            .complete,
            .partial,
            .legacySummary,
            .partial,
            .legacyWithoutActivity
        ])
        let store = SkillStore(service: runner.serviceClient())

        await store.scanAll()
        guard let firstPartialWarning = store.partialScanWarningMessage else {
            throw NativeModelTestFailure(description: "A partial scan should create a persistent warning.")
        }
        try expectEqual(store.lastScanActivity?.status, "completed-partial", "The first scan should retain partial activity state.")

        await store.reload()
        try expectEqual(store.partialScanWarningMessage, firstPartialWarning, "A normal reload should preserve unresolved partial scan state.")

        await store.scanAll()
        try expectNil(store.errorMessage, "A subsequent complete scan should succeed.")
        try expectNil(store.partialScanWarningMessage, "A subsequent complete scan should clear the warning.")
        try expectEqual(store.lastScanActivity?.status, "completed", "The complete scan should replace prior partial activity state.")

        await store.scanAll()
        try expectFalse(store.partialScanWarningMessage == nil, "A later partial scan should recreate the warning.")

        await store.scanAll()
        try expectNil(store.errorMessage, "A legal pre-additive ScanResult should decode without becoming a refresh failure.")
        try expectNil(store.partialScanWarningMessage, "A completed legacy summary should clear the warning.")
        try expectEqual(store.lastScanActivity?.status, "completed", "A completed legacy summary should remain completed.")
        try expectEqual(store.lastScanActivity?.agentSummaries?.first?.rootsPartial, [], "A legacy summary should default missing partial roots to empty.")
        try expectEqual(store.lastScanActivity?.agentSummaries?.first?.scanIssues, [], "A legacy summary should default missing scan issues to empty.")

        await store.scanAll()
        try expectFalse(store.partialScanWarningMessage == nil, "Another partial scan should recreate the warning before the activity-less compatibility case.")

        await store.scanAll()
        try expectNil(store.errorMessage, "A legacy ScanResult without activity should remain a successful scan.")
        try expectNil(store.partialScanWarningMessage, "A legacy ScanResult without activity should exercise the nil-activity clear branch.")
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

        try await waitUntil("Reload should fill the default Claude config timeline in the background.") {
            store.agentConfigSnapshots.map(\.id) == ["snap-claude-new", "snap-claude-old"]
        }
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
        try await waitUntil("Reload should fill the default Claude config timeline in the background.") {
            store.agentConfigSnapshots.map(\.id) == ["snap-claude-new", "snap-claude-old"]
        }

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

    private func settingsFeedbackClearsAfterFailedConfigSave() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        let saved = await store.saveClaudeSettings(
            binding: ConfigSaveBinding(content: "{", expectedRevision: "sha256:not-loaded")
        )

        try expectFalse(saved, "Unsupported fake settings save should fail in this scenario.")
        try expectEqual(
            store.settingsErrorMessage,
            Optional(UIStrings.configRevisionUnavailable),
            "A save attempted without a loaded revision should surface the read-only recovery message."
        )

        store.clearSettingsFeedback()

        try expectNil(store.settingsErrorMessage, "Continuing to edit settings should clear stale config save errors.")
        try expectNil(store.settingsMessage, "Continuing to edit settings should clear stale config save success messages.")
    }

    private func prepareConfigConsistencyContext(_ store: SkillStore) async {
        await store.reload()
        await store.loadClaudeSettings()
    }

    private func configAutosaveKeepsEditArrivingDuringSave() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "autosave-delayed-config")

        let store = SkillStore(service: fake.serviceClient(), autosaveDelayNanoseconds: 0)
        await prepareConfigConsistencyContext(store)
        let revisionA = store.submitConfigAutosave(content: "config-a", validationError: nil)
        try await waitUntil("Config autosave A should reach the service.", timeout: 5) {
            countMethodCalls("config.saveClaudeSettings", in: fake.calls()) == 1
        }

        let revisionB = store.submitConfigAutosave(content: "config-b", validationError: nil)
        try expectEqual(
            store.configAutosavePhase,
            .pendingAfterSave(revision: revisionB),
            "Config edit B should remain pending while A is writing."
        )
        try expectEqual(revisionB, revisionA + 1, "Config autosave revisions should remain ordered.")

        fake.releaseDelayedConfigSave()
        await store.flushPendingAutosaves()

        let calls = fake.calls()
        try expectEqual(
            countMethodCalls("config.saveClaudeSettings", in: calls),
            2,
            "Both config revisions should reach the service exactly once."
        )
        try expectContains(
            calls,
            #""expected_revision":"sha256:autosave-initial""#,
            "Autosave A should use the revision loaded with its original draft."
        )
        try expectContains(
            calls,
            #""expected_revision":"sha256:autosave-a""#,
            "Queued autosave B should advance only to the revision committed by its successful predecessor."
        )
        guard let callA = calls.range(of: "config-a"), let callB = calls.range(of: "config-b") else {
            throw NativeModelTestFailure(description: "Config autosave calls should preserve both draft values.")
        }
        try expectEqual(callA.lowerBound < callB.lowerBound, true, "Config autosave should preserve A then B service order.")
        try expectEqual(store.claudeSettings?.content, "config-b", "The final stored config should be revision B.")
        try expectEqual(store.settingsMessage, Optional(UIStrings.savedSettings), "The exact latest config completion should publish saved feedback.")
        try expectEqual(store.configAutosavePhase, .idle, "Config autosave should settle after both writes.")
    }

    private func queuedConfigAutosaveNeverInheritsExternalPostSaveRevision() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "autosave-external-after-save")

        let store = SkillStore(service: fake.serviceClient(), autosaveDelayNanoseconds: 0)
        await prepareConfigConsistencyContext(store)
        let revisionA = store.submitConfigAutosave(content: "config-a", validationError: nil)
        try await waitUntil("Config autosave A should reach the external-race service.", timeout: 5) {
            countMethodCalls("config.saveClaudeSettings", in: fake.calls()) == 1
        }

        let revisionB = store.submitConfigAutosave(content: "config-b", validationError: nil)
        try expectEqual(revisionB, revisionA + 1, "The queued external-race draft should retain FIFO revision order.")
        fake.releaseDelayedConfigSave()
        await store.flushPendingAutosaves()

        let calls = fake.calls()
        try expectEqual(
            countMethodCalls("config.saveClaudeSettings", in: calls),
            1,
            "External RX observed during A's post-save read must stop B locally before a second write RPC."
        )
        try expectContains(
            calls,
            #""expected_revision":"sha256:autosave-initial""#,
            "A should retain its original R0 authorization."
        )
        try expectFalse(
            calls.contains(#""expected_revision":"sha256:external-after-a""#),
            "Queued B must never inherit mutable external revision RX."
        )
        try expectEqual(store.configAutosaveDraft, "config-b", "Rejected B should remain available as the unsaved draft.")
        guard case let .conflict(conflict) = store.configMutationState else {
            throw NativeModelTestFailure(description: "Queued B should preserve CAS conflict semantics against external RX.")
        }
        try expectEqual(conflict.attemptedRevision, "sha256:autosave-a", "B should remain bound to A's exact returned revision R1.")
        try expectEqual(conflict.latestRevision, Optional("sha256:external-after-a"), "The local CAS boundary should expose external RX without authorizing B with it.")
    }

    private func olderConfigAutosaveFeedbackDoesNotPublishForNewerDraft() async throws {
        for (scenario, expectsSuccess) in [
            ("autosave-delayed-config", true),
            ("autosave-delayed-config-failure", false),
        ] {
            let fake = try FakeServiceScript()
            defer { fake.cleanup() }
            fake.activate(scenario: scenario)

            let store = SkillStore(
                service: fake.serviceClient(),
                autosaveDelayNanoseconds: 500_000_000
            )
            await prepareConfigConsistencyContext(store)
            _ = store.submitConfigAutosave(content: "config-a", validationError: nil)
            try await waitUntil("Config autosave A should reach the feedback service.", timeout: 5) {
                countMethodCalls("config.saveClaudeSettings", in: fake.calls()) == 1
            }

            let revisionB = store.submitConfigAutosave(content: "config-b", validationError: nil)
            fake.releaseDelayedConfigSave()
            try await waitUntil("Config A should finish while B owns visible feedback.", timeout: 5) {
                store.configAutosavePhase == .debouncing(revision: revisionB)
            }

            try expectNil(store.settingsMessage, "Older config A success must not publish a saved banner for B.")
            try expectNil(store.settingsErrorMessage, "Older config A failure must not publish an error for B.")
            if expectsSuccess {
                try expectFalse(store.configMutationState == .idle, "Older config A success must not publish terminal idle state for B.")
            } else {
                guard case .saving = store.configMutationState else {
                    throw NativeModelTestFailure(description: "Older config A failure must leave B's in-progress mutation state untouched.")
                }
            }
            store.cancelPendingConfigAutosave()
        }

        let latestFake = try FakeServiceScript()
        defer { latestFake.cleanup() }
        latestFake.activate(scenario: "autosave-delayed-config-failure")
        let latestStore = SkillStore(service: latestFake.serviceClient(), autosaveDelayNanoseconds: 0)
        await prepareConfigConsistencyContext(latestStore)
        _ = latestStore.submitConfigAutosave(content: "config-a", validationError: nil)
        try await waitUntil("Latest config failure should reach the service.", timeout: 5) {
            countMethodCalls("config.saveClaudeSettings", in: latestFake.calls()) == 1
        }
        latestFake.releaseDelayedConfigSave()
        await latestStore.flushPendingAutosaves()
        try expectContains(
            latestStore.settingsErrorMessage ?? "",
            "config A failed",
            "The exact latest config failure should publish terminal feedback."
        )
    }

    private func providerAutosaveKeepsEditArrivingDuringSave() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "autosave-delayed-provider")

        let store = SkillStore(service: fake.serviceClient(), autosaveDelayNanoseconds: 0)
        await store.loadAIProviderStatus()
        var draftA = AIProviderSettingsDraft(status: .unavailable())
        draftA.endpoint = "https://provider-a.example.com/v1"
        draftA.model = "model-a"
        draftA.apiKey = "A"
        _ = store.submitProviderAutosave(draft: draftA)
        try await waitUntil("Provider autosave A should reach the service.", timeout: 5) {
            countMethodCalls("llm.saveProviderProfile", in: fake.calls()) == 1
        }

        var draftB = draftA
        draftB.endpoint = "https://provider-b.example.com/v1"
        draftB.model = "model-b"
        draftB.apiKey = "B"
        let revisionB = store.submitProviderAutosave(draft: draftB)
        try expectEqual(
            store.providerAutosavePhase,
            .pendingAfterSave(revision: revisionB),
            "Provider edit B should remain pending while A is writing."
        )

        fake.releaseDelayedProviderSaveA()
        try await waitUntil("Provider autosave B should start after A.", timeout: 5) {
            countMethodCalls("llm.saveProviderProfile", in: fake.calls()) == 2
        }
        try expectEqual(
            store.providerAutosaveDraft?.apiKey,
            "B",
            "Completion A must not clear the newer provider API-key draft B."
        )

        fake.releaseDelayedProviderSaveB()
        await store.flushPendingAutosaves()

        let calls = fake.calls()
        try expectFalse(calls.contains(#""api_key":"A""#), "Fake service call evidence must not retain provider API key A.")
        try expectFalse(calls.contains(#""api_key":"B""#), "Fake service call evidence must not retain provider API key B.")
        try expectContains(calls, ConfigContentRedactor.redactedValue, "Fake service call evidence should retain an explicit redaction marker.")
        guard let callA = calls.range(of: "provider-a.example.com"), let callB = calls.range(of: "provider-b.example.com") else {
            throw NativeModelTestFailure(description: "Provider autosave calls should preserve both draft values.")
        }
        try expectEqual(callA.lowerBound < callB.lowerBound, true, "Provider autosave should preserve A then B service order.")
        try expectNil(store.providerAutosaveDraft, "The latest committed provider draft should retire after revision B succeeds.")
        let committedDraft = AIProviderSettingsDraft(status: store.aiProviderStatus)
        try expectEqual(committedDraft.endpoint, draftB.endpoint, "Persisted provider state should settle on revision B.")
        try expectEqual(committedDraft.apiKey, "", "Hydrating committed provider state must not restore its API key.")
        try expectEqual(store.aiProviderMessage, Optional(UIStrings.aiProviderSaved), "The exact latest provider completion should publish saved feedback.")
        try expectEqual(store.providerAutosavePhase, .idle, "Provider autosave should settle after both writes.")
    }

    private func olderProviderAutosaveFeedbackDoesNotPublishForNewerDraft() async throws {
        for scenario in ["autosave-delayed-provider", "autosave-delayed-provider-failure"] {
            let fake = try FakeServiceScript()
            defer { fake.cleanup() }
            fake.activate(scenario: scenario)

            let store = SkillStore(
                service: fake.serviceClient(),
                autosaveDelayNanoseconds: 500_000_000
            )
            await store.loadAIProviderStatus()
            var draftA = AIProviderSettingsDraft(status: store.aiProviderStatus)
            draftA.endpoint = "https://provider-a.example.com/v1"
            draftA.model = "model-a"
            draftA.apiKey = "A"
            _ = store.submitProviderAutosave(draft: draftA)
            try await waitUntil("Provider autosave A should reach the feedback service.", timeout: 5) {
                countMethodCalls("llm.saveProviderProfile", in: fake.calls()) == 1
            }

            var draftB = draftA
            draftB.endpoint = "https://provider-b.example.com/v1"
            draftB.model = "model-b"
            draftB.apiKey = "B"
            let revisionB = store.submitProviderAutosave(draft: draftB)
            fake.releaseDelayedProviderSaveA()
            try await waitUntil("Provider A should finish while B owns visible feedback.", timeout: 5) {
                store.providerAutosavePhase == .debouncing(revision: revisionB)
            }

            try expectNil(store.aiProviderMessage, "Older provider A success must not publish a saved banner for B.")
            try expectNil(store.aiProviderErrorMessage, "Older provider A failure must not publish an error for B.")
            try expectEqual(store.providerAutosaveDraft?.apiKey, "B", "Older provider completion must preserve B's secret draft.")
            store.cancelPendingProviderAutosave()
        }

        let latestFake = try FakeServiceScript()
        defer { latestFake.cleanup() }
        latestFake.activate(scenario: "autosave-delayed-provider-failure")
        let latestStore = SkillStore(service: latestFake.serviceClient(), autosaveDelayNanoseconds: 0)
        await latestStore.loadAIProviderStatus()
        var latestDraft = AIProviderSettingsDraft(status: latestStore.aiProviderStatus)
        latestDraft.endpoint = "https://provider-a.example.com/v1"
        latestDraft.model = "model-a"
        latestDraft.apiKey = "A"
        _ = latestStore.submitProviderAutosave(draft: latestDraft)
        try await waitUntil("Latest provider failure should reach the service.", timeout: 5) {
            countMethodCalls("llm.saveProviderProfile", in: latestFake.calls()) == 1
        }
        latestFake.releaseDelayedProviderSaveA()
        await latestStore.flushPendingAutosaves()
        try expectContains(
            latestStore.aiProviderErrorMessage ?? "",
            "provider A failed",
            "The exact latest provider failure should publish terminal feedback."
        )
    }

    private func configAutosaveQueuesRevertDuringActiveSave() async throws {
        let runner = AutosaveControlServiceRunner()
        let store = SkillStore(service: runner.serviceClient(), autosaveDelayNanoseconds: 0)
        await prepareConfigConsistencyContext(store)

        _ = store.submitConfigAutosave(content: "config-a", validationError: nil)
        try expectEqual(
            store.configAutosaveDraft,
            "config-a",
            "The Store should own the latest config draft while its view is absent."
        )
        try await waitUntil("Config autosave A should start in the continuation runner.") {
            await runner.startedConfigSaveCount == 1
        }

        _ = store.submitConfigAutosave(content: "config-x", validationError: nil)
        try expectEqual(
            store.configAutosaveDraft,
            "config-x",
            "A newer reverted config draft should replace A in Store-owned presentation state."
        )
        await runner.releaseNextConfigSave()
        try await waitUntil("Reverting to X during A should enqueue a second config save.") {
            await runner.startedConfigSaveCount == 2
        }
        try expectEqual(
            store.configAutosaveDraft,
            "config-x",
            "Completion A must not clear the newer reverted config draft."
        )
        await runner.releaseNextConfigSave()
        await store.flushPendingAutosaves()

        try expectEqual(
            await runner.configSaveContents,
            ["config-a", "config-x"],
            "X to A saving to X must persist the final revert as a newer revision."
        )
        try expectEqual(store.claudeSettings?.content, "config-x", "The config cache should settle on the reverted X content.")
    }

    private func providerAutosaveQueuesRevertDuringActiveSave() async throws {
        let runner = AutosaveControlServiceRunner()
        let store = SkillStore(service: runner.serviceClient(), autosaveDelayNanoseconds: 0)
        await store.loadAIProviderStatus()
        let baseline = AIProviderSettingsDraft(status: store.aiProviderStatus)
        var draftA = baseline
        draftA.endpoint = "https://provider-a.example.com/v1"
        draftA.model = "model-a"
        draftA.apiKey = "provider-a-secret"

        _ = store.submitProviderAutosave(draft: draftA)
        try await waitUntil("Provider autosave A should start in the continuation runner.") {
            await runner.startedProviderSaveCount == 1
        }

        _ = store.submitProviderAutosave(draft: baseline)
        await runner.releaseNextProviderSave()
        try await waitUntil("Reverting provider fields to X during A should enqueue a second save.") {
            await runner.startedProviderSaveCount == 2
        }
        try expectEqual(
            store.providerAutosaveDraft?.endpoint,
            baseline.endpoint,
            "Completion A must not overwrite the newer reverted provider draft."
        )

        await runner.releaseNextProviderSave()
        await store.flushPendingAutosaves()
        try expectEqual(
            await runner.providerSaveEndpoints,
            [draftA.endpoint, baseline.endpoint],
            "Provider X to A saving to X must persist the final revert as a newer revision."
        )
        try expectNil(store.providerAutosaveDraft, "The latest committed provider revert should retire its draft.")
        let committedDraft = AIProviderSettingsDraft(status: store.aiProviderStatus)
        try expectEqual(committedDraft.endpoint, baseline.endpoint, "Persisted provider state should settle on X.")
        try expectEqual(committedDraft.apiKey, "", "Hydrating the newest committed revision must not restore its API key.")
    }

    private func configAndProviderAutosavesShareOneMutationLane() async throws {
        let runner = AutosaveControlServiceRunner()
        let store = SkillStore(service: runner.serviceClient(), autosaveDelayNanoseconds: 0)
        await prepareConfigConsistencyContext(store)
        await store.loadAIProviderStatus()

        _ = store.submitConfigAutosave(content: "config-lane-a", validationError: nil)
        try await waitUntil("Config mutation should own the autosave lane first.") {
            await runner.startedConfigSaveCount == 1
        }

        var providerDraft = AIProviderSettingsDraft(status: store.aiProviderStatus)
        providerDraft.endpoint = "https://lane-provider.example.com/v1"
        providerDraft.model = "lane-model"
        providerDraft.apiKey = "lane-secret"
        _ = store.submitProviderAutosave(draft: providerDraft)
        try await Task.sleep(nanoseconds: 30_000_000)
        try expectEqual(
            await runner.startedProviderSaveCount,
            0,
            "Provider mutation B must not start before config mutation A is released."
        )

        await runner.releaseNextConfigSave()
        try await waitUntil("Provider mutation should start after config A leaves the shared lane.") {
            await runner.startedProviderSaveCount == 1
        }
        try expectEqual(
            await runner.maximumConcurrentMutationCount,
            1,
            "Config and provider autosaves must never overlap mutating RPCs."
        )

        await runner.releaseNextProviderSave()
        await store.flushPendingAutosaves()
        try expectEqual(store.configAutosavePhase, .idle, "Config autosave should settle after the shared lane drains.")
        try expectEqual(store.providerAutosavePhase, .idle, "Provider autosave should settle after the shared lane drains.")
    }

    private func cancellingConfigAutosaveBeforeWaiterRegistrationMakesZeroRPCs() async throws {
        let runner = AutosaveControlServiceRunner()
        let store = SkillStore(service: runner.serviceClient(), autosaveDelayNanoseconds: 0)
        await prepareConfigConsistencyContext(store)
        await store.loadAIProviderStatus()

        var providerOwner = AIProviderSettingsDraft(status: store.aiProviderStatus)
        providerOwner.endpoint = "https://provider-pre-registration-owner.example.com/v1"
        providerOwner.model = "owner-model"
        _ = store.submitProviderAutosave(draft: providerOwner)
        try await waitUntil("Provider should own the lane before pre-registration config cancellation.") {
            await runner.startedProviderSaveCount == 1
        }

        var cancellationTriggered = false
        let phaseObserver = store.$configAutosavePhase.sink { phase in
            guard case .saving = phase, !cancellationTriggered else { return }
            cancellationTriggered = true
            store.cancelPendingConfigAutosave()
        }
        _ = store.submitConfigAutosave(
            content: "config-pre-registration-cancelled",
            validationError: nil
        )
        try await waitUntil("Config cancellation should run synchronously from the saving transition.") {
            cancellationTriggered
        }
        phaseObserver.cancel()

        await runner.releaseNextProviderSave()
        try await waitUntil("Pre-registration config cancellation should settle after its owner.", timeout: 5) {
            await runner.startedConfigSaveCount > 0
                || (store.providerAutosavePhase == .idle && store.configAutosavePhase == .idle)
        }
        let unexpectedConfigCalls = await runner.startedConfigSaveCount
        if unexpectedConfigCalls > 0 {
            await runner.releaseNextConfigSave()
        }
        await store.flushPendingAutosaves()

        try expectEqual(unexpectedConfigCalls, 0, "Cancelling config before waiter registration must make zero config RPCs.")
        try expectNil(store.configAutosaveDraft, "Explicit pre-registration config cancellation should retire its draft.")
        try expectEqual(store.configAutosavePhase, .idle, "Pre-registration config cancellation should settle idle.")
    }

    private func cancellingProviderAutosaveBeforeWaiterRegistrationMakesZeroRPCs() async throws {
        let runner = AutosaveControlServiceRunner()
        let store = SkillStore(service: runner.serviceClient(), autosaveDelayNanoseconds: 0)
        await prepareConfigConsistencyContext(store)
        await store.loadAIProviderStatus()

        _ = store.submitConfigAutosave(content: "config-pre-registration-owner", validationError: nil)
        try await waitUntil("Config should own the lane before pre-registration provider cancellation.") {
            await runner.startedConfigSaveCount == 1
        }

        var cancellationTriggered = false
        let phaseObserver = store.$providerAutosavePhase.sink { phase in
            guard case .saving = phase, !cancellationTriggered else { return }
            cancellationTriggered = true
            store.cancelPendingProviderAutosave()
        }
        var providerDraft = AIProviderSettingsDraft(status: store.aiProviderStatus)
        providerDraft.endpoint = "https://provider-pre-registration-cancelled.example.com/v1"
        providerDraft.model = "cancelled-model"
        _ = store.submitProviderAutosave(draft: providerDraft)
        try await waitUntil("Provider cancellation should run synchronously from the saving transition.") {
            cancellationTriggered
        }
        phaseObserver.cancel()

        await runner.releaseNextConfigSave()
        try await waitUntil("Pre-registration provider cancellation should settle after its owner.", timeout: 5) {
            await runner.startedProviderSaveCount > 0
                || (store.configAutosavePhase == .idle && store.providerAutosavePhase == .idle)
        }
        let unexpectedProviderCalls = await runner.startedProviderSaveCount
        if unexpectedProviderCalls > 0 {
            await runner.releaseNextProviderSave()
        }
        await store.flushPendingAutosaves()

        try expectEqual(unexpectedProviderCalls, 0, "Cancelling provider before waiter registration must make zero provider RPCs.")
        try expectNil(store.providerAutosaveDraft, "Explicit pre-registration provider cancellation should retire its draft.")
        try expectEqual(store.providerAutosavePhase, .idle, "Pre-registration provider cancellation should settle idle.")
    }

    private func cancellingQueuedProviderAutosaveMakesZeroRPCs() async throws {
        let runner = AutosaveControlServiceRunner()
        let store = SkillStore(service: runner.serviceClient(), autosaveDelayNanoseconds: 0)
        await prepareConfigConsistencyContext(store)
        await store.loadAIProviderStatus()

        _ = store.submitConfigAutosave(content: "config-owner", validationError: nil)
        try await waitUntil("Config autosave should own the lane before provider cancellation.") {
            await runner.startedConfigSaveCount == 1
        }

        var providerDraft = AIProviderSettingsDraft(status: store.aiProviderStatus)
        providerDraft.endpoint = "https://provider-cancelled.example.com/v1"
        providerDraft.model = "cancelled-model"
        _ = store.submitProviderAutosave(draft: providerDraft)
        try await waitUntil("Provider autosave worker should be waiting for the lane.") {
            store.providerAutosaveHasActiveSave
        }
        try expectEqual(await runner.startedProviderSaveCount, 0, "Queued provider autosave must not start before lane ownership.")

        store.cancelPendingProviderAutosave()
        await runner.releaseNextConfigSave()
        try await waitUntil("Config owner and cancelled provider waiter should settle.", timeout: 5) {
            await runner.startedProviderSaveCount == 1
                || (store.configAutosavePhase == .idle && store.providerAutosavePhase == .idle)
        }

        let unexpectedProviderCalls = await runner.startedProviderSaveCount
        if unexpectedProviderCalls > 0 {
            await runner.releaseNextProviderSave()
            await store.flushPendingAutosaves()
        }
        try expectEqual(unexpectedProviderCalls, 0, "Cancelling a provider waiter before lane ownership must make zero provider RPCs.")
        try expectEqual(store.providerAutosavePhase, .idle, "A cancelled provider waiter should settle idle, not failed.")
        try expectNil(store.aiProviderErrorMessage, "A cancelled provider waiter should not publish a failure.")
    }

    private func cancellingQueuedConfigAutosaveMakesZeroRPCs() async throws {
        let runner = AutosaveControlServiceRunner()
        let store = SkillStore(service: runner.serviceClient(), autosaveDelayNanoseconds: 0)
        await prepareConfigConsistencyContext(store)
        await store.loadAIProviderStatus()

        var providerDraft = AIProviderSettingsDraft(status: store.aiProviderStatus)
        providerDraft.endpoint = "https://provider-owner.example.com/v1"
        providerDraft.model = "owner-model"
        _ = store.submitProviderAutosave(draft: providerDraft)
        try await waitUntil("Provider autosave should own the lane before config cancellation.") {
            await runner.startedProviderSaveCount == 1
        }

        _ = store.submitConfigAutosave(content: "config-cancelled", validationError: nil)
        try await waitUntil("Config autosave worker should be waiting for the lane.") {
            store.configAutosaveHasActiveSave
        }
        try expectEqual(await runner.startedConfigSaveCount, 0, "Queued config autosave must not start before lane ownership.")

        store.cancelPendingConfigAutosave()
        await runner.releaseNextProviderSave()
        try await waitUntil("Provider owner and cancelled config waiter should settle.", timeout: 5) {
            await runner.startedConfigSaveCount == 1
                || (store.providerAutosavePhase == .idle && store.configAutosavePhase == .idle)
        }

        let unexpectedConfigCalls = await runner.startedConfigSaveCount
        if unexpectedConfigCalls > 0 {
            await runner.releaseNextConfigSave()
            await store.flushPendingAutosaves()
        }
        try expectEqual(unexpectedConfigCalls, 0, "Cancelling a config waiter before lane ownership must make zero config RPCs.")
        try expectEqual(store.configAutosavePhase, .idle, "A cancelled config waiter should settle idle, not failed.")
        try expectNil(store.settingsErrorMessage, "A cancelled config waiter should not publish a failure.")
    }

    private func invalidConfigDraftCancelsQueuedValidAutosave() async throws {
        let runner = AutosaveControlServiceRunner()
        let store = SkillStore(service: runner.serviceClient(), autosaveDelayNanoseconds: 0)
        await prepareConfigConsistencyContext(store)
        await store.loadAIProviderStatus()

        var providerOwner = AIProviderSettingsDraft(status: store.aiProviderStatus)
        providerOwner.endpoint = "https://provider-invalid-config-owner.example.com/v1"
        providerOwner.model = "owner-model"
        _ = store.submitProviderAutosave(draft: providerOwner)
        try await waitUntil("Provider should own the lane before config becomes invalid.") {
            await runner.startedProviderSaveCount == 1
        }

        _ = store.submitConfigAutosave(content: "config-valid-a", validationError: nil)
        try await waitUntil("Valid config A should wait for lane ownership.") {
            store.configAutosaveHasActiveSave
        }
        try await Task.sleep(nanoseconds: 30_000_000)
        _ = store.submitConfigAutosave(
            content: "config-invalid-b",
            validationError: "invalid config B"
        )

        await runner.releaseNextProviderSave()
        try await waitUntil("Invalid config B should cancel queued valid A.", timeout: 5) {
            await runner.startedConfigSaveCount > 0
                || (store.providerAutosavePhase == .idle && store.configAutosavePhase == .idle)
        }
        let unexpectedConfigCalls = await runner.startedConfigSaveCount
        if unexpectedConfigCalls > 0 {
            await runner.releaseNextConfigSave()
        }
        await store.flushPendingAutosaves()

        try expectEqual(unexpectedConfigCalls, 0, "Invalid config B must cancel pre-owner valid A with zero config RPCs.")
        try expectEqual(store.configAutosaveDraft, "config-invalid-b", "The current invalid config B should remain available for correction.")
        try expectEqual(store.configAutosavePhase, .idle, "Cancelling pre-owner valid A for invalid B should settle idle.")
    }

    private func invalidProviderDraftCancelsQueuedValidAutosave() async throws {
        let runner = AutosaveControlServiceRunner()
        let store = SkillStore(service: runner.serviceClient(), autosaveDelayNanoseconds: 0)
        await prepareConfigConsistencyContext(store)
        await store.loadAIProviderStatus()

        _ = store.submitConfigAutosave(content: "config-invalid-provider-owner", validationError: nil)
        try await waitUntil("Config should own the lane before provider becomes invalid.") {
            await runner.startedConfigSaveCount == 1
        }

        var validProviderA = AIProviderSettingsDraft(status: store.aiProviderStatus)
        validProviderA.endpoint = "https://provider-valid-a.example.com/v1"
        validProviderA.model = "valid-model-a"
        _ = store.submitProviderAutosave(draft: validProviderA)
        try await waitUntil("Valid provider A should wait for lane ownership.") {
            store.providerAutosaveHasActiveSave
        }
        try await Task.sleep(nanoseconds: 30_000_000)
        var invalidProviderB = validProviderA
        invalidProviderB.endpoint = ""
        _ = store.submitProviderAutosave(draft: invalidProviderB)

        await runner.releaseNextConfigSave()
        try await waitUntil("Invalid provider B should cancel queued valid A.", timeout: 5) {
            await runner.startedProviderSaveCount > 0
                || (store.configAutosavePhase == .idle && store.providerAutosavePhase == .idle)
        }
        let unexpectedProviderCalls = await runner.startedProviderSaveCount
        if unexpectedProviderCalls > 0 {
            await runner.releaseNextProviderSave()
        }
        await store.flushPendingAutosaves()

        try expectEqual(unexpectedProviderCalls, 0, "Invalid provider B must cancel pre-owner valid A with zero provider RPCs.")
        try expectEqual(store.providerAutosaveDraft?.endpoint, "", "The current invalid provider B should remain available for correction.")
        try expectEqual(store.providerAutosavePhase, .idle, "Cancelling pre-owner valid A for invalid B should settle idle.")
    }

    private func releasingStoreCancelsQueuedAutosaveWaiter() async throws {
        let runner = AutosaveControlServiceRunner()
        var store: SkillStore? = SkillStore(service: runner.serviceClient(), autosaveDelayNanoseconds: 0)
        weak let releasedStore = store
        guard store != nil else {
            throw NativeModelTestFailure(description: "Store should exist for release testing.")
        }
        await store?.loadAIProviderStatus()
        if let store {
            await prepareConfigConsistencyContext(store)
        }

        var providerDraft = AIProviderSettingsDraft(status: store?.aiProviderStatus ?? .unavailable())
        providerDraft.endpoint = "https://provider-release-owner.example.com/v1"
        providerDraft.model = "release-owner"
        _ = store?.submitProviderAutosave(draft: providerDraft)
        try await waitUntil("Provider owner should hold the lane before Store release.") {
            await runner.startedProviderSaveCount == 1
        }
        _ = store?.submitConfigAutosave(content: "config-must-not-outlive-store", validationError: nil)
        try await waitUntil("Config worker should queue before Store release.") {
            store?.configAutosaveHasActiveSave == true
        }
        try expectEqual(await runner.startedConfigSaveCount, 0, "Queued config mutation should not own the lane yet.")

        store = nil
        await runner.releaseNextProviderSave()
        try await waitUntil("Store should release or expose the old queued mutation.", timeout: 5) {
            if releasedStore == nil { return true }
            return await runner.startedConfigSaveCount == 1
        }
        let unexpectedConfigCalls = await runner.startedConfigSaveCount
        if unexpectedConfigCalls > 0 {
            await runner.releaseNextConfigSave()
        }
        try await waitUntil("Store should deinitialize after its active owner finishes.", timeout: 5) {
            releasedStore == nil
        }

        try expectEqual(unexpectedConfigCalls, 0, "Store release must cancel a queued autosave before it reaches the service.")
    }

    private func configSaveInvalidatesInFlightConfigReaders() async throws {
        let runner = AutosaveControlServiceRunner(
            suspendMutations: false,
            suspendInitialConfigReads: true
        )
        let store = SkillStore(service: runner.serviceClient(), autosaveDelayNanoseconds: 0)

        await prepareConfigConsistencyContext(store)
        await store.loadCurrentAgentConfigDocuments(agent: SkillAgentFilter.claudeCode.rawValue)
        await store.loadAgentConfigSnapshots(agent: SkillAgentFilter.claudeCode.rawValue)
        await runner.activateConfigReadSuspension()

        let claudeRead = Task { @MainActor in await store.loadClaudeSettings() }
        let documentRead = Task { @MainActor in
            await store.loadCurrentAgentConfigDocuments(agent: SkillAgentFilter.claudeCode.rawValue)
        }
        let snapshotRead = Task { @MainActor in
            await store.loadAgentConfigSnapshots(agent: SkillAgentFilter.claudeCode.rawValue)
        }
        try await waitUntil("All three pre-save config reads should be suspended.") {
            await runner.suspendedInitialConfigReadCount == 3
        }

        let saved = await store.saveClaudeSettings(content: "config-new")
        try expectEqual(saved, true, "The controlled config mutation should succeed.")
        await runner.releaseInitialConfigReads()
        await claudeRead.value
        await documentRead.value
        await snapshotRead.value

        try expectEqual(
            store.claudeSettings?.content,
            "config-new",
            "A stale pre-save Claude read must not overwrite the saved config cache."
        )
        try expectEqual(
            store.currentAgentConfigDocuments.first?.content,
            "config-new",
            "A stale pre-save config document read must not overwrite the post-save document cache."
        )
        try expectEqual(
            store.agentConfigSnapshots.first?.id,
            "snapshot-new",
            "A stale pre-save snapshot read must not overwrite the post-save timeline cache."
        )
    }

    private func configSaveRefreshesTheSubmittedAgentAfterSelectionChanges() async throws {
        let runner = AutosaveControlServiceRunner()
        let store = SkillStore(service: runner.serviceClient(), autosaveDelayNanoseconds: 0)
        await prepareConfigConsistencyContext(store)

        _ = store.submitConfigAutosave(content: "config-captured-agent", validationError: nil)
        try await waitUntil("Captured-agent config save should start.") {
            await runner.startedConfigSaveCount == 1
        }
        store.agentFilter = .codex
        await runner.releaseNextConfigSave()
        await store.flushPendingAutosaves()

        let refreshedAgents = await runner.configRefreshAgentsAfterFirstSave
        try expectEqual(
            refreshedAgents.contains(SkillAgentFilter.claudeCode.rawValue),
            true,
            "Post-save config and snapshot refresh must still target the agent captured at submission time."
        )
    }

    private func providerUnknownMutationDoesNotReportSaved() async throws {
        let runner = AutosaveControlServiceRunner(providerMutationUnavailable: true)
        let store = SkillStore(service: runner.serviceClient())
        var draft = AIProviderSettingsDraft(status: .unavailable())
        draft.endpoint = "https://unavailable-provider.example.com/v1"
        draft.model = "unavailable-model"
        draft.apiKey = "must-not-clear"

        let saved = await store.saveAIProviderSettings(draft: draft)

        try expectFalse(saved, "An unknown mutating provider RPC must fail instead of mapping to a saved unavailable status.")
        try expectNil(store.aiProviderMessage, "An unavailable provider mutation must not publish a saved banner.")
        try expectEqual(store.aiProviderStatus.serviceAvailable, false, "Read-side unavailable compatibility should remain visible.")

        _ = store.submitProviderAutosave(draft: draft)
        await store.flushPendingAutosaves()
        try expectEqual(
            store.providerAutosaveDraft?.apiKey,
            "must-not-clear",
            "A failed provider autosave must preserve the unsaved API key draft."
        )
        guard case .failed = store.providerAutosavePhase else {
            throw NativeModelTestFailure(description: "A failed provider mutation should leave the autosave phase failed.")
        }
    }

    private func successfulConfigAutosaveRetiresDraftAndAdoptsExternalRefresh() async throws {
        let runner = AutosaveControlServiceRunner(suspendMutations: false)
        let store = SkillStore(service: runner.serviceClient(), autosaveDelayNanoseconds: 0)
        await prepareConfigConsistencyContext(store)

        _ = store.submitConfigAutosave(content: "config-saved", validationError: nil)
        await store.flushPendingAutosaves()
        try expectNil(
            store.configAutosaveDraft,
            "The latest successful config completion should retire its Store-owned draft."
        )

        await runner.setExternalConfigContent("config-external")
        await store.loadClaudeSettings()
        let effectiveDraft = store.configAutosaveDraft ?? store.claudeSettings?.content
        try expectEqual(
            effectiveDraft,
            "config-external",
            "Passive config hydration should adopt an external refresh after the last draft commits."
        )
    }

    private func successfulProviderAutosaveRetiresDraftAndAdoptsExternalRefresh() async throws {
        let runner = AutosaveControlServiceRunner(suspendMutations: false)
        let store = SkillStore(service: runner.serviceClient(), autosaveDelayNanoseconds: 0)
        await store.loadAIProviderStatus()
        var draft = AIProviderSettingsDraft(status: store.aiProviderStatus)
        draft.endpoint = "https://provider-saved.example.com/v1"
        draft.model = "model-saved"
        draft.apiKey = "provider-secret"

        _ = store.submitProviderAutosave(draft: draft)
        await store.flushPendingAutosaves()
        try expectNil(
            store.providerAutosaveDraft,
            "The latest successful provider completion should retire its Store-owned draft."
        )
        let committedDraft = AIProviderSettingsDraft(status: store.aiProviderStatus)
        try expectEqual(committedDraft.endpoint, draft.endpoint, "Committed provider hydration should retain the saved endpoint.")
        try expectEqual(committedDraft.apiKey, "", "Committed provider hydration must not retain the submitted API key.")

        await runner.setExternalProvider(
            endpoint: "https://provider-external.example.com/v1",
            model: "model-external"
        )
        await store.loadAIProviderStatus()
        let effectiveDraft = store.providerAutosaveDraft
            ?? AIProviderSettingsDraft(status: store.aiProviderStatus)
        try expectEqual(
            effectiveDraft.endpoint,
            "https://provider-external.example.com/v1",
            "Passive provider hydration should adopt an external status refresh after the last draft commits."
        )
        try expectEqual(effectiveDraft.apiKey, "", "External provider hydration must not restore an API key draft.")
    }

    private func failedConfigAutosaveKeepsDraft() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")
        let store = SkillStore(service: fake.serviceClient(), autosaveDelayNanoseconds: 0)

        _ = store.submitConfigAutosave(content: "config-unsaved", validationError: nil)
        await store.flushPendingAutosaves()

        try expectEqual(
            store.configAutosaveDraft,
            "config-unsaved",
            "A failed config completion must retain the unsaved draft."
        )
        try expectEqual(
            store.configAutosavePhase,
            .idle,
            "A fail-closed autosave without a protocol binding should remain idle because it made no write attempt."
        )
        try expectEqual(
            store.configMutationState,
            .failed(UIStrings.configConsistencyProtocolRequired),
            "A fail-closed autosave should surface the protocol-v2 requirement."
        )
    }

    private func configSaveUsesLoadedRevision() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "config-cas")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        await store.loadClaudeSettings()
        try expectEqual(store.claudeSettings?.revision, Optional("sha256:settings-revision"), "Config load should retain the exact service revision.")
        guard let binding = store.makeClaudeSettingsSaveBinding(content: "{\"theme\":\"dark\"}\n") else {
            throw NativeModelTestFailure(description: "Protocol v2 config load should create an immutable save binding.")
        }

        let saved = await store.saveClaudeSettings(binding: binding)

        try expectEqual(saved, true, "A revision-bound config save should succeed.")
        try expectEqual(countMethodCalls("config.saveClaudeSettings", in: fake.calls()), 1, "Config save should call the mutation RPC once.")
        try expectContains(fake.calls(), #""expected_revision":"sha256:settings-revision""#, "Config save should send the revision paired with the loaded document.")
        try expectEqual(store.claudeSettings?.revision, Optional("sha256:saved-revision"), "Successful save should publish the service's new revision.")
        try expectEqual(store.configMutationState, .idle, "Successful save should leave the config mutation state idle.")
    }

    private func configSaveNeverFallsBackWithoutRevision() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "config-legacy")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        await store.loadClaudeSettings()
        try expectFalse(store.claudeSettings?.supportsCompareAndSwap ?? true, "Legacy config responses should be read-only.")
        try expectNil(store.makeClaudeSettingsSaveBinding(content: "{\"theme\":\"dark\"}\n"), "Missing revision must not create a save binding.")

        let saved = await store.saveClaudeSettings(
            binding: ConfigSaveBinding(content: "{\"theme\":\"dark\"}\n", expectedRevision: "sha256:not-issued")
        )

        try expectEqual(saved, false, "A config document without a revision must not be saved.")
        try expectEqual(countMethodCalls("config.saveClaudeSettings", in: fake.calls()), 0, "Missing revision must make no write RPC call.")
        try expectEqual(store.settingsErrorMessage, Optional(UIStrings.configRevisionUnavailable), "Missing CAS capability should show a localized read-only message.")
        try expectEqual(store.configMutationState, .failed(UIStrings.configRevisionUnavailable), "Missing CAS capability should publish a failed mutation state.")
    }

    private func configConflictPreservesDraftAndReloadsRevision() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "config-conflict")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        await store.loadClaudeSettings()
        let draft = "{\"theme\":\"draft\"}\n"
        guard let binding = store.makeClaudeSettingsSaveBinding(content: draft) else {
            throw NativeModelTestFailure(description: "Protocol v2 conflict fixture should create a save binding.")
        }
        let snapshotsBeforeSave = countMethodCalls("app.stateSnapshot", in: fake.calls())

        let saved = await store.saveClaudeSettings(binding: binding)

        try expectEqual(saved, false, "A stale revision must not be reported as saved.")
        try expectEqual(countMethodCalls("config.saveClaudeSettings", in: fake.calls()), 1, "A config conflict must not retry the write.")
        try expectEqual(countMethodCalls("config.readClaudeSettings", in: fake.calls()), 2, "A config conflict should perform one fresh read for comparison.")
        try expectContains(fake.calls(), #""content":"{\"theme\":\"draft\"}\n""#, "The attempted draft should remain the exact write input.")
        try expectEqual(store.claudeSettings?.content, Optional("{\"theme\":\"external\"}\n"), "Conflict handling should publish the freshly read external document for comparison.")
        try expectEqual(store.claudeSettings?.revision, Optional("sha256:external-revision"), "Conflict reread should publish the latest revision without retrying.")
        try expectEqual(store.settingsErrorMessage, Optional(UIStrings.configConflict), "Config conflict should use localized recovery guidance.")
        try expectNil(store.settingsMessage, "Config conflict must not publish save success.")
        try expectNil(store.lastMutationMessage, "Config conflict must not publish a success mutation message.")
        try expectEqual(countMethodCalls("app.stateSnapshot", in: fake.calls()), snapshotsBeforeSave, "Config conflict must not perform a success-style collections refresh.")

        guard case let .conflict(conflict) = store.configMutationState else {
            throw NativeModelTestFailure(description: "Config conflict should publish conflict state.")
        }
        try expectEqual(conflict.attemptedRevision, "sha256:settings-revision", "Conflict state should retain the attempted revision.")
        try expectEqual(conflict.latestRevision, Optional("sha256:external-revision"), "Conflict state should expose the freshly read revision for comparison.")
        try expectEqual(conflict.displayMessage, UIStrings.configConflict, "Conflict state should use localized recovery guidance.")
    }

    private func pendingConfigDraftNeverRebindsToReloadedRevision() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "config-conflict")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        await store.loadClaudeSettings()
        try expectEqual(store.claudeSettings?.revision, Optional("sha256:settings-revision"), "Initial editor document should use revision A.")

        let draft = "{\"theme\":\"draft-from-a\"}\n"
        guard let pendingSave = store.makeClaudeSettingsSaveBinding(content: draft) else {
            throw NativeModelTestFailure(description: "Protocol v2 editor should capture an immutable pending save binding.")
        }
        try expectEqual(pendingSave.expectedRevision, "sha256:settings-revision", "Pending autosave should capture revision A before debounce.")

        await store.loadClaudeSettings()
        try expectEqual(store.claudeSettings?.content, Optional("{\"theme\":\"external\"}\n"), "Reload should publish external document B.")
        try expectEqual(store.claudeSettings?.revision, Optional("sha256:external-revision"), "Reload should publish revision B.")

        let saved = await store.saveClaudeSettings(binding: pendingSave)

        try expectEqual(saved, false, "The old pending draft must not overwrite reloaded document B.")
        try expectEqual(countMethodCalls("config.saveClaudeSettings", in: fake.calls()), 0, "Revision drift before the debounce fires should be rejected without a write RPC.")
        try expectFalse(fake.calls().contains(#""expected_revision":"sha256:external-revision""#), "Draft A must never be rebound to revision B.")
        try expectEqual(pendingSave.content, draft, "Rejected pending save should retain the exact user draft.")
        try expectEqual(store.claudeSettings?.content, Optional("{\"theme\":\"external\"}\n"), "Rejected pending save must preserve external document B.")
        guard case let .conflict(conflict) = store.configMutationState else {
            throw NativeModelTestFailure(description: "Revision drift should enter explicit conflict state.")
        }
        try expectEqual(conflict.attemptedRevision, "sha256:settings-revision", "Conflict should identify pending revision A.")
        try expectEqual(conflict.latestRevision, Optional("sha256:external-revision"), "Conflict should expose current revision B.")
    }

    private func protocolV1BindingsRemainReadOnlyBeforeAndAfterStatusLoad() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "protocol-v1-bindings")

        let store = SkillStore(service: fake.serviceClient())
        await store.loadClaudeSettings()
        try expectNil(store.status, "Startup config reads may finish before service status is loaded.")
        try expectEqual(store.claudeSettings?.revision, Optional("sha256:malicious-v1-revision"), "Protocol v1 fixture intentionally returns a revision-shaped field.")
        try expectFalse(store.supportsConfigConsistencyProtocol, "Unknown startup protocol must default to read-only.")

        let fabricatedBinding = ConfigSaveBinding(
            content: "{\"theme\":\"unsafe\"}\n",
            expectedRevision: "sha256:malicious-v1-revision"
        )
        try expectEqual(await store.saveClaudeSettings(binding: fabricatedBinding), false, "Status-not-loaded save must remain read-only.")
        try expectEqual(store.settingsErrorMessage, Optional(UIStrings.configConsistencyProtocolRequired), "Unknown protocol should explain the protocol v2 requirement.")

        await store.reload()
        try expectEqual(store.status?.protocolVersion, Optional(1), "Fake service should expose a genuine protocol v1 status.")
        try expectFalse(store.supportsConfigConsistencyProtocol, "Protocol v1 must remain read-only even when revision-shaped fields exist.")
        try expectNil(store.makeClaudeSettingsSaveBinding(content: fabricatedBinding.content), "Protocol v1 must not create a save binding.")
        try expectEqual(await store.saveClaudeSettings(binding: fabricatedBinding), false, "Protocol v1 save must remain read-only after status load.")
        try expectEqual(store.configMutationState, .failed(UIStrings.configConsistencyProtocolRequired), "Protocol v1 save should publish localized read-only state.")
        try expectEqual(countMethodCalls("config.saveClaudeSettings", in: fake.calls()), 0, "Protocol v1 must make no config write RPC.")

        await store.loadAgentConfigSnapshots(agent: "claude-code")
        try await waitUntil("Protocol v1 fixture should expose a snapshot for read-only preview.") {
            !store.agentConfigSnapshots.isEmpty
        }
        guard let snapshot = store.agentConfigSnapshots.first else {
            throw NativeModelTestFailure(description: "Protocol v1 fixture should expose a snapshot for read-only preview.")
        }
        store.selectConfigSnapshot(snapshot)
        let preview = try await store.previewRollback(snapshotID: snapshot.id)
        try expectEqual(preview.previewToken, Optional("sha256:malicious-v1-token"), "Protocol v1 fixture intentionally returns a token-shaped field.")
        try expectNil(store.rollbackConfirmation, "Protocol v1 must not create rollback authorization from token-shaped fields.")
        let fabricatedConfirmation = RollbackConfirmation(
            snapshotID: snapshot.id,
            previewToken: "sha256:malicious-v1-token"
        )
        try expectEqual(await store.rollbackSnapshot(confirmation: fabricatedConfirmation), false, "Protocol v1 rollback must remain read-only.")
        try expectEqual(store.errorMessage, Optional(UIStrings.configConsistencyProtocolRequired), "Protocol v1 rollback should explain the protocol v2 requirement.")
        try expectEqual(countMethodCalls("snapshot.rollback", in: fake.calls()), 0, "Protocol v1 must make no rollback write RPC.")
    }

    private func protocolV2MissingBindingsRemainReadOnly() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "protocol-v2-missing-bindings")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        try expectEqual(store.status?.protocolVersion, Optional(2), "Missing-binding fixture should still use protocol v2.")
        await store.loadClaudeSettings()
        try expectNil(store.claudeSettings?.revision, "Protocol v2 response intentionally omits config revision.")
        try expectNil(store.makeClaudeSettingsSaveBinding(content: "{}\n"), "Protocol v2 without revision must not create a save binding.")
        let fabricatedBinding = ConfigSaveBinding(content: "{}\n", expectedRevision: "sha256:not-issued")
        try expectEqual(await store.saveClaudeSettings(binding: fabricatedBinding), false, "Protocol v2 without revision must remain read-only.")
        try expectEqual(countMethodCalls("config.saveClaudeSettings", in: fake.calls()), 0, "Missing revision must make no config write RPC.")

        await store.loadAgentConfigSnapshots(agent: "claude-code")
        try await waitUntil("Missing-binding fixture should expose a snapshot.") {
            !store.agentConfigSnapshots.isEmpty
        }
        guard let snapshot = store.agentConfigSnapshots.first else {
            throw NativeModelTestFailure(description: "Missing-binding fixture should expose a snapshot.")
        }
        store.selectConfigSnapshot(snapshot)
        let preview = try await store.previewRollback(snapshotID: snapshot.id)
        try expectFalse(preview.rollbackSupported, "Protocol v2 preview missing token/revision must decode as read-only.")
        try expectNil(store.rollbackConfirmation, "Protocol v2 preview missing token/revision must not create confirmation.")
        let fabricatedConfirmation = RollbackConfirmation(snapshotID: snapshot.id, previewToken: "sha256:not-issued")
        try expectEqual(await store.rollbackSnapshot(confirmation: fabricatedConfirmation), false, "Protocol v2 without rollback binding must remain read-only.")
        try expectEqual(countMethodCalls("snapshot.rollback", in: fake.calls()), 0, "Missing rollback binding must make no rollback write RPC.")
    }

    private func rollbackUsesImmutablePreviewInputs() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "config-cas")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        try await waitUntil("Config CAS fixture should expose rollback snapshots.") {
            store.agentConfigSnapshots.count == 2
        }
        guard let snapshot = store.agentConfigSnapshots.first else {
            throw NativeModelTestFailure(description: "Config CAS fixture should expose a rollback snapshot.")
        }
        store.selectConfigSnapshot(snapshot)
        let preview = try await store.previewRollback(snapshotID: snapshot.id)
        guard let confirmation = store.rollbackConfirmation else {
            throw NativeModelTestFailure(description: "A complete rollback preview should create an immutable confirmation.")
        }
        try expectEqual(confirmation.snapshotID, preview.snapshot.id, "Confirmation should capture the previewed snapshot id.")
        try expectEqual(confirmation.previewToken, "sha256:rollback-preview", "Confirmation should capture the opaque preview token.")

        let rolledBack = await store.rollbackSnapshot(confirmation: confirmation)

        try expectEqual(rolledBack, true, "A matching rollback confirmation should succeed.")
        try expectEqual(countMethodCalls("snapshot.rollback", in: fake.calls()), 1, "Rollback should issue one mutation RPC.")
        try expectContains(fake.calls(), #""snapshot_id":"snap-claude-new""#, "Rollback should send the immutable confirmation snapshot id.")
        try expectContains(fake.calls(), #""preview_token":"[REDACTED]""#, "Recorded rollback evidence should redact the opaque preview token.")
        try expectFalse(fake.calls().contains("expected_revision"), "Rollback authorization must never use a bare revision.")
    }

    private func staleRollbackTokenRequiresAnotherPreview() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "rollback-stale")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        try await waitUntil("Stale rollback fixture should expose a snapshot.") {
            !store.agentConfigSnapshots.isEmpty
        }
        guard let snapshot = store.agentConfigSnapshots.first else {
            throw NativeModelTestFailure(description: "Stale rollback fixture should expose a snapshot.")
        }
        store.selectConfigSnapshot(snapshot)
        _ = try await store.previewRollback(snapshotID: snapshot.id)
        guard let confirmation = store.rollbackConfirmation else {
            throw NativeModelTestFailure(description: "Stale rollback fixture should initially create a confirmation.")
        }
        let snapshotRefreshesBeforeRollback = countMethodCalls("app.stateSnapshot", in: fake.calls())

        let rolledBack = await store.rollbackSnapshot(confirmation: confirmation)

        try expectEqual(rolledBack, false, "A stale preview token must not report rollback success.")
        try expectEqual(countMethodCalls("snapshot.rollback", in: fake.calls()), 1, "A stale rollback must not retry automatically.")
        try expectEqual(countMethodCalls("snapshot.previewRollback", in: fake.calls()), 1, "A stale rollback must not auto-preview a second token.")
        try expectEqual(countMethodCalls("app.stateSnapshot", in: fake.calls()), snapshotRefreshesBeforeRollback, "A stale rollback must not perform a success refresh.")
        try expectEqual(store.selectedConfigSnapshot?.id, Optional(snapshot.id), "A stale rollback should preserve the selected snapshot.")
        try expectNil(store.rollbackConfirmation, "A stale rollback should clear its invalid confirmation.")
        try expectEqual(store.errorMessage, Optional(UIStrings.rollbackPreviewAgain), "A stale rollback should ask for another preview with localized guidance.")
        try expectNil(store.lastMutationMessage, "A stale rollback must not publish a success mutation message.")
    }

    private func inFlightRollbackFailureDoesNotPublishAfterSelectionChanges() async throws {
        let variants = [
            (label: "stale_preview_token", scenario: "rollback-stale-blocked", changesSelection: true, reselectSnapshotA: false, expectsCurrentError: false),
            (label: "generic service failure", scenario: "rollback-error-blocked", changesSelection: true, reselectSnapshotA: false, expectsCurrentError: false),
            (label: "generic failure after A-B-A reselection", scenario: "rollback-error-blocked", changesSelection: true, reselectSnapshotA: true, expectsCurrentError: false),
            (label: "generic failure while A remains current", scenario: "rollback-error-blocked", changesSelection: false, reselectSnapshotA: false, expectsCurrentError: true),
        ]
        var leakedErrors: [String] = []

        for variant in variants {
            let errorMessage = try await inFlightRollbackErrorAfterSelectionChange(
                scenario: variant.scenario,
                label: variant.label,
                changesSelection: variant.changesSelection,
                reselectSnapshotA: variant.reselectSnapshotA
            )
            if variant.expectsCurrentError {
                try expectContains(errorMessage, "rollback_failed", "A generic rollback failure should remain visible while snapshot A is still current.")
            } else if let errorMessage {
                leakedErrors.append("\(variant.label): \(errorMessage)")
            }
        }

        try expectEqual(
            leakedErrors,
            [],
            "A stale rollback completion may not publish globally after its selection identity changes."
        )
    }

    private func inFlightRollbackErrorAfterSelectionChange(
        scenario: String,
        label: String,
        changesSelection: Bool,
        reselectSnapshotA: Bool
    ) async throws -> String? {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "config-cas")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        try await waitUntil("The \(label) fixture should expose snapshots A and B.") {
            store.agentConfigSnapshots.count == 2
        }
        let snapshotA = store.agentConfigSnapshots[0]
        let snapshotB = store.agentConfigSnapshots[1]
        store.selectConfigSnapshot(snapshotA)
        _ = try await store.previewRollback(snapshotID: snapshotA.id)
        guard let confirmation = store.rollbackConfirmation else {
            throw NativeModelTestFailure(description: "The \(label) fixture should authorize snapshot A before rollback.")
        }

        fake.setScenario(scenario)
        let rollbackTask = Task { @MainActor in
            await store.rollbackSnapshot(confirmation: confirmation)
        }
        try await waitUntil("The \(label) rollback RPC should reach the controllable service before selection changes.") {
            self.countMethodCalls("snapshot.rollback", in: fake.calls()) == 1
        }

        if changesSelection {
            store.selectConfigSnapshot(snapshotB)
            if reselectSnapshotA {
                store.selectConfigSnapshot(snapshotA)
            }
        }
        fake.releaseBlockedResponse()
        let rolledBack = await rollbackTask.value

        try expectEqual(rolledBack, false, "The \(label) response must not report rollback success.")
        let expectedSelectionID = !changesSelection || reselectSnapshotA ? snapshotA.id : snapshotB.id
        try expectEqual(store.selectedConfigSnapshot?.id, Optional(expectedSelectionID), "The current selection must survive snapshot A's stale completion.")
        try expectNil(store.rollbackConfirmation, "Snapshot A's consumed confirmation must stay cleared after selecting snapshot B.")
        try expectEqual(countMethodCalls("snapshot.rollback", in: fake.calls()), 1, "The \(label) path must issue exactly one rollback RPC.")
        try expectContains(fake.calls(), #""snapshot_id":"snap-claude-new""#, "Rollback must keep snapshot A's immutable id.")
        try expectContains(fake.calls(), #""preview_token":"[REDACTED]""#, "Recorded rollback evidence should redact snapshot A's preview token.")
        try expectFalse(fake.calls().contains("expected_revision"), "Rollback must not substitute a bare revision for its preview token.")
        try expectNil(store.lastMutationMessage, "A failed snapshot A rollback must not publish success feedback.")
        return store.errorMessage
    }

    private func mismatchedInFlightRollbackPreviewDoesNotPublishAfterSelectionChanges() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "config-cas")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        try await waitUntil("The mismatched-preview fixture should expose snapshots A and B.") {
            store.agentConfigSnapshots.count == 2
        }
        let snapshotA = store.agentConfigSnapshots[0]
        let snapshotB = store.agentConfigSnapshots[1]
        store.selectConfigSnapshot(snapshotA)

        var presentation = RollbackPreviewPresentationState<SnapshotRollbackPreviewRecord>()
        let request = presentation.begin(snapshotID: snapshotA.id)
        fake.setScenario("rollback-preview-mismatch-blocked")
        let previewTask: Task<Result<SnapshotRollbackPreviewRecord, Error>, Never> = Task { @MainActor in
            do {
                return .success(try await store.previewRollback(snapshotID: request.snapshotID))
            } catch {
                return .failure(error)
            }
        }
        try await waitUntil("Snapshot A's preview RPC should reach the controllable service before selection changes.") {
            self.countMethodCalls("snapshot.previewRollback", in: fake.calls()) == 1
        }

        store.selectConfigSnapshot(snapshotB)
        presentation.invalidate(selectedSnapshotID: snapshotB.id)
        fake.releaseBlockedResponse()
        let outcome = await previewTask.value

        switch outcome {
        case .success:
            throw NativeModelTestFailure(description: "A preview whose payload names snapshot B must be rejected for snapshot A.")
        case .failure(let error):
            try expectContains(error.localizedDescription, "did not match the requested snapshot", "The service payload mismatch should remain the operation's local error.")
            let published = presentation.publish(errorMessage: error.localizedDescription, for: request)
            try expectFalse(published, "Snapshot A's rejected preview error must not publish into snapshot B's presentation state.")
        }

        try expectEqual(store.selectedConfigSnapshot?.id, Optional(snapshotB.id), "Snapshot B must remain selected after snapshot A's mismatched preview returns.")
        try expectNil(store.errorMessage, "Snapshot A's mismatched preview must not write the Store's global error after selecting snapshot B.")
        try expectNil(store.rollbackConfirmation, "A mismatched stale preview must not create rollback confirmation.")
        try expectEqual(presentation.selectedSnapshotID, Optional(snapshotB.id), "Local preview state must remain bound to snapshot B.")
        try expectNil(presentation.preview, "Snapshot B must not display snapshot A's mismatched preview payload.")
        try expectNil(presentation.errorMessage, "Snapshot B must not display snapshot A's preview mismatch error.")
        try expectNil(presentation.activeRequest, "Snapshot B must not retain snapshot A's preview request.")
        try expectEqual(countMethodCalls("snapshot.previewRollback", in: fake.calls()), 1, "The mismatch path must issue exactly one preview RPC.")
        try expectEqual(countMethodCalls("snapshot.rollback", in: fake.calls()), 0, "A mismatched preview must never issue rollback RPC.")
    }

    private func rollbackConfirmationInvalidatesOnSelectionTimelineAndPreviewChanges() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "config-cas")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        try await waitUntil("Config CAS fixture should expose two snapshots.") {
            store.agentConfigSnapshots.count == 2
        }
        guard store.agentConfigSnapshots.count == 2 else {
            throw NativeModelTestFailure(description: "Config CAS fixture should expose two snapshots.")
        }
        let first = store.agentConfigSnapshots[0]
        let second = store.agentConfigSnapshots[1]
        store.selectConfigSnapshot(first)
        _ = try await store.previewRollback(snapshotID: first.id)
        guard let firstConfirmation = store.rollbackConfirmation else {
            throw NativeModelTestFailure(description: "First preview should create a confirmation.")
        }

        store.selectConfigSnapshot(second)
        try expectNil(store.rollbackConfirmation, "Changing snapshot selection should invalidate the old confirmation.")

        _ = try await store.previewRollback(snapshotID: second.id)
        guard let secondConfirmation = store.rollbackConfirmation else {
            throw NativeModelTestFailure(description: "Second preview should replace the confirmation.")
        }
        try expectEqual(firstConfirmation.previewToken, "sha256:rollback-preview", "Previously captured confirmation should remain immutable.")
        try expectEqual(secondConfirmation.previewToken, "sha256:rollback-preview-2", "Preview replacement should publish only the new token.")

        let rejectedOldConfirmation = await store.rollbackSnapshot(confirmation: firstConfirmation)
        try expectEqual(rejectedOldConfirmation, false, "An invalidated confirmation must be rejected before RPC.")
        try expectEqual(countMethodCalls("snapshot.rollback", in: fake.calls()), 0, "An invalidated confirmation must make no write call.")

        _ = try await store.previewRollback(snapshotID: second.id)
        try expectFalse(store.rollbackConfirmation == nil, "A new preview should restore confirmation.")
        await store.loadAgentConfigSnapshots(agent: "claude-code")
        try expectNil(store.rollbackConfirmation, "Timeline reload should invalidate any prior confirmation.")

        _ = try await store.previewRollback(snapshotID: second.id)
        await store.reload()
        try expectNil(store.rollbackConfirmation, "A full collection reload that replaces the timeline should invalidate confirmation.")

        store.selectConfigSnapshot(second)
        _ = try await store.previewRollback(snapshotID: second.id)
        store.clearRollbackConfirmation()
        try expectNil(store.rollbackConfirmation, "Explicit preview clearing should invalidate confirmation.")
    }

    private func inFlightRollbackPreviewCannotRestoreInvalidatedConfirmation() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "rollback-preview-delay")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        try await waitUntil("Delayed preview fixture should expose two snapshots.") {
            store.agentConfigSnapshots.count == 2
        }
        guard store.agentConfigSnapshots.count == 2 else {
            throw NativeModelTestFailure(description: "Delayed preview fixture should expose two snapshots.")
        }
        let first = store.agentConfigSnapshots[0]
        let second = store.agentConfigSnapshots[1]
        store.selectConfigSnapshot(first)

        let previewTask = Task {
            try await store.previewRollback(snapshotID: first.id)
        }
        try await waitUntil("Delayed preview should reach the fake service before selection changes.") {
            countMethodCalls("snapshot.previewRollback", in: fake.calls()) == 1
        }
        store.selectConfigSnapshot(second)
        _ = try await previewTask.value

        try expectEqual(store.selectedConfigSnapshot?.id, Optional(second.id), "Selection should remain on the newly chosen snapshot.")
        try expectNil(store.rollbackConfirmation, "An invalidated in-flight preview must not restore its old confirmation.")
        try expectEqual(countMethodCalls("snapshot.rollback", in: fake.calls()), 0, "A stale preview response must never issue a rollback RPC.")
    }

    private func rollbackSnapshotRequiresVisibleAgentTimelineRecord() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "timeline")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        try await waitUntil("Reload should fill the default Claude config timeline in the background.") {
            store.agentConfigSnapshots.map(\.id) == ["snap-claude-new", "snap-claude-old"]
        }

        let rejected = await store.rollbackSnapshot(
            confirmation: RollbackConfirmation(snapshotID: "snap-codex", previewToken: "sha256:not-visible")
        )

        try expectEqual(rejected, false, "Rollback should reject snapshots outside the selected timeline.")
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
        let runner = CatalogRefreshServiceRunner()

        let store = SkillStore(service: runner.serviceClient())
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

    private func providerObservabilityUsesReadOnlyServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        try await waitUntil("Reload supplemental reads should settle before the observability contract check.") {
            countMethodCalls("snapshot.listAgentConfig", in: fake.calls()) == 1
                && countMethodCalls("llm.providerObservability", in: fake.calls()) == 1
                && !store.isLoadingProviderObservability
                && store.providerObservabilityResult != nil
        }
        let snapshotCallsBeforeObservability = countOccurrences("snapshot.", in: fake.calls())
        await store.loadProviderObservability()

        let result = store.providerObservabilityResult
        try expectEqual(result?.generatedBy, "local-v2.64", "Provider observability should expose generator metadata.")
        try expectEqual(result?.summary.callCount, 3, "Provider observability should expose call count.")
        try expectEqual(result?.summary.successCount, 1, "Provider observability should expose success count.")
        try expectEqual(result?.summary.failureCount, 1, "Provider observability should expose failure count.")
        try expectEqual(result?.summary.blockedCount, 1, "Provider observability should expose blocked count.")
        try expectEqual(result?.summary.estimatedTotalTokens, 1300, "Provider observability should expose token estimates.")
        try expectEqual(result?.callRows.first?.requestKind, "task_cockpit", "Provider observability should expose recent call rows.")
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
        try expectContains(calls, "\"start_at\":", "Provider observability should pass the applied start date.")
        try expectContains(calls, "\"end_at\":", "Provider observability should pass the applied end date.")
        try expectContains(calls, "\"limit\":100", "Provider observability should pass the bounded evidence row limit.")
        try expectContains(calls, "\"include_history\":true", "Provider observability should request history rows.")
        try expectContains(calls, "\"include_budget_hints\":false", "Provider observability settings should avoid unused budget hint payload.")
        try expectContains(calls, "\"include_retention_recommendations\":false", "Provider observability settings should avoid unused retention payload.")
        try expectContains(calls, "\"include_evidence\":false", "Provider observability settings should avoid unused evidence payload.")
        try expectFalse(calls.contains("llm.previewPrompt"), "Provider observability must not prepare provider prompts.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Provider observability must not send to provider.")
        try expectFalse(calls.contains("llm.recordModelTaskMatch"), "Provider observability UI must not write model-task history.")
        try expectFalse(calls.contains("llm.deleteModelTaskMatch"), "Provider observability UI must not delete model-task history.")
        try expectFalse(calls.contains("config.toggleSkill"), "Provider observability must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Provider observability must not call execution paths.")
        try expectEqual(countOccurrences("snapshot.", in: calls), snapshotCallsBeforeObservability, "Provider observability must not call snapshot paths.")
        try expectFalse(calls.contains("credential"), "Provider observability must not call credential paths.")
    }

    private func providerObservabilityPreloadsAtStartupAndRefreshesWithReload() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        await store.loadAppStartupDataIfNeeded()

        try await waitUntil("Startup should preload provider observability in the background.") {
            countMethodCalls("llm.providerObservability", in: fake.calls()) == 1
                && store.providerObservabilityResult?.summary.callCount == 3
                && !store.isLoadingProviderObservability
        }
        try expectEqual(store.providerObservabilityResult?.summary.callCount, 3, "Startup observability preload should keep the decoded dashboard.")

        await store.loadProviderObservabilityIfNeeded()

        try expectEqual(countMethodCalls("llm.providerObservability", in: fake.calls()), 1, "Need-based observability loading should reuse the startup cache.")

        await store.reload()

        try await waitUntil("Global reload should refresh provider observability in the background.") {
            countMethodCalls("llm.providerObservability", in: fake.calls()) == 2
                && store.providerObservabilityResult?.summary.callCount == 3
                && !store.isLoadingProviderObservability
        }

        await store.loadProviderObservabilityIfNeeded()

        try expectEqual(countMethodCalls("llm.providerObservability", in: fake.calls()), 2, "Need-based loading should reuse the reloaded dashboard.")

        await store.loadProviderObservability()

        try expectEqual(countMethodCalls("llm.providerObservability", in: fake.calls()), 3, "Manual observability refresh should still force a fresh local service request.")
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

    private func previewAndConfirmTaskCockpit(
        _ store: SkillStore,
        _ message: String = "Task cockpit should require explicit provider confirmation."
    ) async throws {
        await store.buildTaskCockpit()
        guard store.taskCockpitPromptConfirmation != nil else {
            throw NativeModelTestFailure(description: "\(message) Preview should be available before provider send.")
        }
        try expectNil(store.taskCockpitResult, "\(message) Previewing should not synthesize a result before confirmation.")
        await store.confirmTaskCockpitPromptAndBuild()
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
        try await waitUntil("Reload supplemental reads should settle before the Task Cockpit contract check.") {
            countMethodCalls("snapshot.listAgentConfig", in: fake.calls()) == 1
        }
        store.selectedSidebarSelection = .skill("beta")
        let snapshotCallsBeforeCockpit = countOccurrences("snapshot.", in: fake.calls())
        await store.buildTaskCockpit()
        guard store.taskCockpitPromptConfirmation != nil else {
            throw NativeModelTestFailure(description: "Task cockpit should prepare a prompt preview before provider send.")
        }
        try expectNil(store.taskCockpitResult, "Task cockpit should not expose a provider result before explicit confirmation.")
        try expectFalse(fake.calls().contains("llm.confirmPromptAndSend"), "Previewing Task Preflight must not send a provider request.")
        await store.confirmTaskCockpitPromptAndBuild()

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

    private func taskCockpitHistoryStaysInCurrentSessionOnly() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let historyStore = makeTemporaryTaskCockpitHistoryStore()
        defer { cleanupTaskCockpitHistoryStore(historyStore) }
        let task = "阿里云 ECS 磁盘负载情况分析"
        let store = SkillStore(service: fake.serviceClient(), taskCockpitHistoryStore: historyStore)
        store.taskCockpitText = task
        await store.reload()
        try await previewAndConfirmTaskCockpit(store)

        try expectEqual(store.taskCockpitHistory.count, 1, "Successful Preflight should add one current-session history record.")
        try expectEqual(store.taskCockpitHistory.first?.displayTask, task, "Session history should preserve the visible task text.")
        try expectEqual(store.taskCockpitHistory.first?.agentIDs, ["claude-code"], "Session history should preserve the full agent scope.")
        try expectEqual(store.taskCockpitHistory.first?.result.summary.recommendedSkillName, "Beta", "Session history should retain the full recommendation result.")
        try expectEqual(store.taskCockpitHistory.first?.operationState.phase, .completed, "Session history should retain the completed operation state.")
        try expectEqual(store.selectedTaskCockpitHistoryID, store.taskCockpitHistory.first?.id, "The latest session record should remain selected.")
    }

    private func taskCockpitHistoryKeepsNewestTwelveRecords() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let historyStore = makeTemporaryTaskCockpitHistoryStore()
        defer { cleanupTaskCockpitHistoryStore(historyStore) }
        let store = SkillStore(service: fake.serviceClient(), taskCockpitHistoryStore: historyStore)
        await store.reload()

        let submittedTasks = (1...13).map { "Session-only Preflight \($0)" }
        for task in submittedTasks {
            store.taskCockpitText = task
            try await previewAndConfirmTaskCockpit(store)
        }

        let expectedTasks = Array(submittedTasks.dropFirst().reversed())
        try expectEqual(
            store.taskCockpitHistory.count,
            12,
            "Session history should retain exactly twelve successful results."
        )
        try expectEqual(
            store.taskCockpitHistory.map(\.displayTask),
            expectedTasks,
            "Session history should remain newest-first after evicting the oldest result."
        )
        try expectFalse(
            store.taskCockpitHistory.contains { $0.displayTask == submittedTasks[0] },
            "The thirteenth result should evict the oldest session record."
        )
        try expectEqual(
            store.selectedTaskCockpitHistoryID,
            store.taskCockpitHistory.first?.id,
            "The newest retained session record should remain selected."
        )
    }

    private func newStoreDoesNotRestoreTaskCockpitHistory() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let historyStore = makeTemporaryTaskCockpitHistoryStore()
        defer { cleanupTaskCockpitHistoryStore(historyStore) }
        let firstStore = SkillStore(service: fake.serviceClient(), taskCockpitHistoryStore: historyStore)
        firstStore.taskCockpitText = "Keep this Preflight in memory only."
        await firstStore.reload()
        try await previewAndConfirmTaskCockpit(firstStore)
        try expectEqual(firstStore.taskCockpitHistory.count, 1, "The first store should retain its completed session result.")
        try expectEqual(firstStore.taskCockpitHistory.first?.displayTask, "Keep this Preflight in memory only.", "The current session should retain the complete task text.")
        try expectEqual(firstStore.taskCockpitHistory.first?.agentIDs, ["claude-code"], "The current session should retain the complete agent scope.")
        try expectEqual(firstStore.taskCockpitHistory.first?.result.summary.recommendedSkillName, "Beta", "The current session should retain the complete provider result.")
        try expectEqual(
            FileManager.default.fileExists(atPath: historyStore.fileURL.path),
            false,
            "The provider-confirmed Preflight must remain in memory without creating a history file."
        )

        let secondStore = SkillStore(service: fake.serviceClient(), taskCockpitHistoryStore: historyStore)
        try expectEqual(secondStore.taskCockpitHistory.count, 0, "A new store must not restore prior Task Preflight history.")

        for fixture in sensitiveLegacyTaskCockpitHistoryFixtures {
            try writeTaskCockpitHistoryFixture(fixture, to: historyStore)
            let restartedStore = SkillStore(service: fake.serviceClient(), taskCockpitHistoryStore: historyStore)
            try expectEqual(restartedStore.taskCockpitHistory.count, 0, "Legacy disk history must never populate a new session.")
            try expectEqual(
                FileManager.default.fileExists(atPath: historyStore.fileURL.path),
                false,
                "Store startup should purge every legacy Task Preflight history format."
            )
        }

        let siblingNames = try FileManager.default.contentsOfDirectory(
            atPath: historyStore.fileURL.deletingLastPathComponent().path
        )
        try expectFalse(
            siblingNames.contains { $0.hasPrefix("task-preflight-history") },
            "Startup cleanup must not leave a history file, rename, or backup."
        )
        for siblingName in siblingNames {
            let siblingURL = historyStore.fileURL.deletingLastPathComponent().appendingPathComponent(siblingName)
            guard let contents = try? String(contentsOf: siblingURL, encoding: .utf8) else { continue }
            try expectFalse(
                contents.contains("SENSITIVE_SENTINEL_42"),
                "Startup cleanup must not copy sensitive legacy history into another file."
            )
        }
    }

    private func successfulTaskCockpitDoesNotCreateHistoryFile() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let historyStore = makeTemporaryTaskCockpitHistoryStore()
        defer { cleanupTaskCockpitHistoryStore(historyStore) }
        let store = SkillStore(service: fake.serviceClient(), taskCockpitHistoryStore: historyStore)
        store.taskCockpitText = "Do not persist this provider-confirmed result."
        await store.reload()
        try await previewAndConfirmTaskCockpit(store)

        try expectEqual(store.taskCockpitHistory.count, 1, "The successful result should remain available in memory.")
        try expectEqual(
            FileManager.default.fileExists(atPath: historyStore.fileURL.path),
            false,
            "A successful Preflight must not create a history file."
        )
        try expectEqual(
            FileManager.default.fileExists(atPath: historyStore.fileURL.deletingLastPathComponent().path),
            false,
            "A successful Preflight must not create a history directory."
        )
    }

    private func clearTaskCockpitHistoryClearsMemory() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let historyStore = makeTemporaryTaskCockpitHistoryStore()
        defer { cleanupTaskCockpitHistoryStore(historyStore) }
        let store = SkillStore(service: fake.serviceClient(), taskCockpitHistoryStore: historyStore)
        store.taskCockpitText = "Clear this in-memory result."
        await store.reload()
        try await previewAndConfirmTaskCockpit(store)
        try writeTaskCockpitHistoryFixture(sensitiveLegacyTaskCockpitHistoryFixtures[0], to: historyStore)

        store.clearTaskCockpitHistory()

        try expectEqual(store.taskCockpitHistory.count, 0, "Clear should remove every in-memory Task Preflight record.")
        try expectNil(store.selectedTaskCockpitHistoryID, "Clear should reset the selected history record.")
        try expectEqual(
            FileManager.default.fileExists(atPath: historyStore.fileURL.path),
            false,
            "Clear should retry deletion when a legacy file survived or reappeared after startup."
        )
        try expectNil(store.taskCockpitHistoryCleanupMessage, "Successful clear cleanup should dismiss the cleanup warning.")
    }

    private func legacyHistoryCleanupFailureShowsRedactedMessage() throws {
        let historyStore = makeTemporaryTaskCockpitHistoryStore()
        defer { cleanupTaskCockpitHistoryStore(historyStore) }
        try writeTaskCockpitHistoryFixture(sensitiveLegacyTaskCockpitHistoryFixtures[0], to: historyStore)
        let failingHistoryStore = TaskCockpitHistoryStore(
            fileURL: historyStore.fileURL,
            unlinkItem: { _ in .failed(errorNumber: EACCES) }
        )

        let store = SkillStore(service: unexpectedServiceClient(), taskCockpitHistoryStore: failingHistoryStore)

        try expectEqual(store.taskCockpitHistory.count, 0, "Cleanup failure must not restore legacy records into memory.")
        try expectEqual(store.taskCockpitHistoryCleanupMessage, UIStrings.taskCockpitHistoryCleanupFailed, "Cleanup failure should show only the localized redacted warning.")
        try expectFalse(store.taskCockpitHistoryCleanupMessage?.contains("SENSITIVE_SENTINEL_42") ?? true, "Cleanup warning must not expose task text or file bytes.")
        try expectFalse(store.taskCockpitHistoryCleanupMessage?.contains(historyStore.fileURL.path) ?? true, "Cleanup warning must not expose the legacy history path.")
        try expectFalse(store.taskCockpitHistoryCleanupMessage?.contains("Cannot remove") ?? true, "Cleanup warning must not expose the raw removal error.")
        try expectEqual(FileManager.default.fileExists(atPath: historyStore.fileURL.path), true, "Failed cleanup should leave the original file for a visible retry.")
    }

    private func legacyHistoryDirectoryShowsRedactedMessageAndRemainsUntouched() throws {
        let historyStore = makeTemporaryTaskCockpitHistoryStore()
        defer { cleanupTaskCockpitHistoryStore(historyStore) }
        try FileManager.default.createDirectory(
            at: historyStore.fileURL,
            withIntermediateDirectories: true
        )
        let nestedSentinelURL = historyStore.fileURL
            .appendingPathComponent("SENSITIVE_SENTINEL_42.txt", isDirectory: false)
        try Data("SENSITIVE_SENTINEL_42 directory contents".utf8).write(to: nestedSentinelURL)

        let store = SkillStore(service: unexpectedServiceClient(), taskCockpitHistoryStore: historyStore)

        try expectEqual(
            store.taskCockpitHistoryCleanupMessage,
            UIStrings.taskCockpitHistoryCleanupFailed,
            "An unexpected directory should show the same generic cleanup warning."
        )
        try expectFalse(
            store.taskCockpitHistoryCleanupMessage?.contains(historyStore.fileURL.path) ?? true,
            "Directory cleanup warnings must not expose the local history path."
        )
        try expectFalse(
            store.taskCockpitHistoryCleanupMessage?.lowercased().contains("directory") ?? true,
            "Directory cleanup warnings must not expose the unexpected item type."
        )
        try expectEqual(
            try String(contentsOf: nestedSentinelURL, encoding: .utf8),
            "SENSITIVE_SENTINEL_42 directory contents",
            "Startup cleanup must not recursively delete or mutate unexpected directory contents."
        )
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
        try await previewAndConfirmTaskCockpit(store)

        try expectEqual(store.selectedTaskCockpitInput, exactTask, "Non-blank cockpit input should preserve the exact user text.")

        let calls = fake.calls()
        try expectContains(calls, "llm.previewPrompt", "Exact-input test should prepare the provider-backed Task Preflight prompt.")
        try expectContains(calls, "llm.confirmPromptAndSend", "Exact-input cockpit flow should send through provider confirmation.")
        try expectContains(calls, "\"task_text\":\"  修复 Task Cockpit 🧪\\n第二行\\t带制表  \"", "Task cockpit should pass Chinese, emoji, multiline text, tabs, and surrounding spaces unchanged.")
        try expectFalse(calls.contains("\"task_text\":\"修复 Task Cockpit 🧪\\n第二行\\t带制表\""), "Task cockpit must not trim non-blank user text before submission.")
        try expectFalse(calls.contains("config.toggleSkill"), "Exact-input cockpit flow must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Exact-input cockpit flow must not call execution paths.")
        try expectFalse(calls.contains("credential"), "Exact-input cockpit flow must not call credential paths.")
    }

    private func taskCockpitWhitespaceOnlyInputRequiresTask() async throws {
        let store = SkillStore(service: unexpectedServiceClient())
        store.taskCockpitText = " \n\t "
        await store.buildTaskCockpit()

        try expectEqual(store.selectedTaskCockpitInput, "", "Whitespace-only cockpit input should not reuse old task fields.")
        try expectEqual(store.taskCockpitResult?.isUnavailable, true, "Whitespace-only cockpit input should produce an unavailable result.")
        try expectEqual(store.taskCockpitResult?.fallbackReason, UIStrings.taskCockpitTaskRequired, "Whitespace-only cockpit input should ask for a task.")
    }

    private func taskCockpitFallsBackWhenMethodUnavailable() async throws {
        let runner = TaskCockpitFallbackServiceRunner()

        let historyStore = makeTemporaryTaskCockpitHistoryStore()
        defer { cleanupTaskCockpitHistoryStore(historyStore) }
        let store = SkillStore(
            service: ServiceClient(
                processRunner: runner,
                serviceURL: URL(fileURLWithPath: "/tmp/task-cockpit-fallback-service")
            ),
            taskCockpitHistoryStore: historyStore
        )
        store.selectedSkillID = "beta"
        store.taskCockpitText = "Route a local audit release note task."
        await store.reload()
        await store.buildTaskCockpit()

        try expectEqual(store.taskCockpitResult?.isUnavailable, true, "Task cockpit should expose unavailable fallback for older services.")
        try expectEqual(store.taskCockpitResult?.fallbackReason, UIStrings.taskCockpitUnavailable, "Unknown method fallback should use the localized unavailable copy.")
        try expectFalse(store.isBuildingTaskCockpit, "Unavailable task cockpit should reset loading state.")
        let calls = await runner.calls()
        try expectContains(calls, "llm.previewPrompt", "Fallback should still prove the intended provider preview method was attempted.")
        try expectContains(calls, "\"task_text\":\"Route a local audit release note task.\"", "Fallback should reuse existing routing task text when cockpit input is blank.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Unavailable cockpit flow must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Unavailable cockpit flow must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Unavailable cockpit flow must not call execution paths.")
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
        await store.buildTaskCockpit()
        let slowBuild = Task {
            await store.confirmTaskCockpitPromptAndBuild()
        }

        try await waitUntil("Slow task cockpit should time out into a visible recovery state.") {
            store.taskCockpitOperationState.phase == .timedOut
        }
        try expectFalse(store.isBuildingTaskCockpit, "Timed-out task cockpit should release the loading state.")
        try expectEqual(store.taskCockpitOperationState.canRetry, true, "Timed-out task cockpit should expose retry.")
        try expectContains(store.taskCockpitResult?.fallbackReason, "did not finish", "Timeout should produce a visible fallback reason.")

        fake.setScenario("prompt-ready")
        try await previewAndConfirmTaskCockpit(store)
        try expectEqual(store.taskCockpitResult?.summary.recommendedSkillName, "Beta", "Retry should load the fresh cockpit result.")
        try expectEqual(store.taskCockpitOperationState.phase, .completed, "Retry success should replace the timeout state.")

        await slowBuild.value
        try expectEqual(store.taskCockpitResult?.summary.recommendedSkillName, "Beta", "Late slow response must not overwrite the retry result.")
        let calls = fake.calls()
        try expectContains(calls, "llm.previewPrompt", "Timeout path should prepare task preflight prompt previews.")
        try expectContains(calls, "llm.confirmPromptAndSend", "Timeout path should use the provider confirmation method.")
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
        await store.buildTaskCockpit()
        let slowBuild = Task {
            await store.confirmTaskCockpitPromptAndBuild()
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
        try await previewAndConfirmTaskCockpit(store)
        try expectEqual(store.taskCockpitResult?.summary.recommendedSkillName, "Beta", "Retry after cancel should load the cockpit result.")
        try expectEqual(store.taskCockpitOperationState.phase, .completed, "Retry success should replace the cancelled state.")

        await slowBuild.value
        try expectEqual(store.taskCockpitResult?.summary.recommendedSkillName, "Beta", "Late cancelled response must not overwrite the retry result.")
        try expectContains(fake.calls(), "llm.confirmPromptAndSend", "Cancel recovery should use the provider confirmation method for the retry.")
        try expectFalse(fake.calls().contains("config.toggleSkill"), "Cancel recovery must not call config write paths.")
        try expectFalse(fake.calls().contains("script.execute"), "Cancel recovery must not call execution paths.")
    }

    private func llmPreparePreviewIsScopedToSelectedSkillAndReadOnly() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "llm-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.prepareAnalyzeLLM()

        let analyze = store.llmPrepareResult(for: .analyze)
        try expectEqual(analyze?.action, .analyze, "Beta analyze preview should be available while beta is selected.")
        try expectEqual(analyze?.enabled, true, "Analyze prepare should be enabled when LLM status is ready.")
        try expectEqual(analyze?.provider, "openai", "Analyze prepare should expose provider.")
        try expectEqual(analyze?.model, "gpt-5", "Analyze prepare should expose model.")
        try expectEqual(analyze?.estimate?.inputTokens, 240, "Analyze prepare should expose input token estimate.")
        try expectEqual(analyze?.estimate?.estimatedCostUSD, 0.0042, "Analyze prepare should expose cost estimate.")
        try expectEqual(analyze?.confirmationRequired, true, "Analyze prepare should require confirmation.")

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

    private func decodeAppSearchItem(_ json: String) throws -> AppSearchItem {
        try JSONDecoder().decode(AppSearchItem.self, from: Data(json.utf8))
    }

    private func waitUntil(
        _ label: String,
        timeout: TimeInterval = 2,
        predicate: @escaping @MainActor () async -> Bool
    ) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while !(await predicate()) {
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
        return TaskCockpitHistoryStore(
            fileURL: directory.appendingPathComponent("task-preflight-history.json", isDirectory: false)
        )
    }

    private func cleanupTaskCockpitHistoryStore(_ store: TaskCockpitHistoryStore) {
        guard ProcessInfo.processInfo.environment["CI"] != "true" else { return }
        try? FileManager.default.removeItem(at: store.fileURL.deletingLastPathComponent())
    }

    private func writeTaskCockpitHistoryFixture(_ fixture: String, to store: TaskCockpitHistoryStore) throws {
        try FileManager.default.createDirectory(
            at: store.fileURL.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try Data(fixture.utf8).write(to: store.fileURL)
    }

    private var sensitiveLegacyTaskCockpitHistoryFixtures: [String] {
        let record = """
        {
          "taskText":"SENSITIVE_SENTINEL_42 task text",
          "operationState":{"message":"SENSITIVE_SENTINEL_42 operation state"},
          "filters":{"taskText":"SENSITIVE_SENTINEL_42 filters"},
          "summary":{"summaryText":"SENSITIVE_SENTINEL_42 summary"},
          "result":{"resultText":"SENSITIVE_SENTINEL_42 result text"}
        }
        """
        return [
            "[\(record)]",
            "{\"version\":1,\"records\":[\(record)]}",
            "{\"version\":2,\"records\":[\(record)]}",
            "taskText=SENSITIVE_SENTINEL_42; operationState=SENSITIVE_SENTINEL_42; filters=SENSITIVE_SENTINEL_42; summary=SENSITIVE_SENTINEL_42; resultText=SENSITIVE_SENTINEL_42",
        ]
    }

    private func unexpectedServiceClient() -> ServiceClient {
        ServiceClient(
            processRunner: UnexpectedServiceProcessRunner(),
            serviceURL: URL(fileURLWithPath: "/dev/null")
        )
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

private struct UnexpectedServiceProcessRunner: ServiceProcessRunning {
    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) async throws -> Data {
        throw NativeModelTestFailure(description: "Whitespace-only task cockpit input should not call the service.")
    }
}

private final class AutosaveControlServiceRunner: ServiceProcessRunning {
    private let state: AutosaveControlServiceState

    init(
        suspendMutations: Bool = true,
        suspendInitialConfigReads: Bool = false,
        providerMutationUnavailable: Bool = false
    ) {
        state = AutosaveControlServiceState(
            suspendMutations: suspendMutations,
            suspendInitialConfigReads: suspendInitialConfigReads,
            providerMutationUnavailable: providerMutationUnavailable
        )
    }

    func serviceClient() -> ServiceClient {
        ServiceClient(
            processRunner: self,
            serviceURL: URL(fileURLWithPath: "/tmp/autosave-control-service")
        )
    }

    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) async throws -> Data {
        _ = executableURL
        _ = timeoutNanoseconds
        return try await state.response(for: input)
    }

    var startedConfigSaveCount: Int {
        get async { await state.startedConfigSaveCount }
    }

    var startedProviderSaveCount: Int {
        get async { await state.startedProviderSaveCount }
    }

    var configSaveContents: [String] {
        get async { await state.configSaveContents }
    }

    var providerSaveEndpoints: [String] {
        get async { await state.providerSaveEndpoints }
    }

    var maximumConcurrentMutationCount: Int {
        get async { await state.maximumConcurrentMutationCount }
    }

    var suspendedInitialConfigReadCount: Int {
        get async { await state.suspendedInitialConfigReadCount }
    }

    var configRefreshAgentsAfterFirstSave: Set<String> {
        get async { await state.configRefreshAgentsAfterFirstSave }
    }

    func releaseNextConfigSave() async {
        await state.releaseNextConfigSave()
    }

    func releaseNextProviderSave() async {
        await state.releaseNextProviderSave()
    }

    func releaseInitialConfigReads() async {
        await state.releaseInitialConfigReads()
    }

    func activateConfigReadSuspension() async {
        await state.activateConfigReadSuspension()
    }

    func setExternalConfigContent(_ content: String) async {
        await state.setExternalConfigContent(content)
    }

    func setExternalProvider(endpoint: String, model: String) async {
        await state.setExternalProvider(endpoint: endpoint, model: model)
    }
}

private actor AutosaveControlServiceState {
    private struct SuspendedConfigRead {
        let continuation: CheckedContinuation<Data, Never>
        let response: Data
    }

    private let suspendMutations: Bool
    private let supportsConfigReadSuspension: Bool
    private let providerMutationUnavailable: Bool
    private var configReadSuspensionActive = false
    private var suspendedConfigReadMethods: Set<String> = []
    private var configContent = "config-x"
    private var providerEndpoint = "https://provider-x.example.com/v1"
    private var providerModel = "model-x"
    private var readCounts: [String: Int] = [:]
    private var initialConfigReads: [SuspendedConfigRead] = []
    private var configSaveContinuations: [CheckedContinuation<Void, Never>] = []
    private var providerSaveContinuations: [CheckedContinuation<Void, Never>] = []
    private var activeMutationCount = 0
    private var events: [String] = []
    private var firstConfigSaveCompletionIndex: Int?
    private(set) var configSaveContents: [String] = []
    private(set) var providerSaveEndpoints: [String] = []
    private(set) var maximumConcurrentMutationCount = 0

    init(
        suspendMutations: Bool,
        suspendInitialConfigReads: Bool,
        providerMutationUnavailable: Bool
    ) {
        self.suspendMutations = suspendMutations
        supportsConfigReadSuspension = suspendInitialConfigReads
        self.providerMutationUnavailable = providerMutationUnavailable
    }

    var startedConfigSaveCount: Int { configSaveContents.count }
    var startedProviderSaveCount: Int { providerSaveEndpoints.count }
    var suspendedInitialConfigReadCount: Int { initialConfigReads.count }

    var configRefreshAgentsAfterFirstSave: Set<String> {
        guard let firstConfigSaveCompletionIndex else { return [] }
        return Set(events.dropFirst(firstConfigSaveCompletionIndex + 1).compactMap { event in
            let parts = event.split(separator: ":", maxSplits: 1).map(String.init)
            guard parts.count == 2,
                  parts[0] == "config.readAgentConfig" || parts[0] == "snapshot.listAgentConfig" else {
                return nil
            }
            return parts[1]
        })
    }

    func response(for input: Data) async throws -> Data {
        guard let request = try JSONSerialization.jsonObject(with: input) as? [String: Any],
              let method = request["method"] as? String else {
            return Self.error(code: "invalid_request", message: "missing method")
        }
        let params = request["params"] as? [String: Any] ?? [:]

        switch method {
        case "config.saveClaudeSettings":
            let content = params["content"] as? String ?? ""
            let expectedRevision = params["expected_revision"] as? String ?? ""
            let currentRevision = "sha256:test-\(configContent)"
            guard expectedRevision == currentRevision else {
                return Self.error(
                    code: "config_conflict",
                    message: "expected \(expectedRevision), current \(currentRevision)"
                )
            }
            configSaveContents.append(content)
            beginMutation(event: "config.save.start")
            if suspendMutations {
                await withCheckedContinuation { continuation in
                    configSaveContinuations.append(continuation)
                }
            }
            configContent = content
            finishMutation(event: "config.save.complete")
            if firstConfigSaveCompletionIndex == nil {
                firstConfigSaveCompletionIndex = events.indices.last
            }
            return Self.ok(configDocument(agent: "claude-code", content: content))

        case "llm.saveProviderProfile":
            if providerMutationUnavailable {
                return Self.error(code: "unknown_method", message: "unknown method: llm.saveProviderProfile")
            }
            let endpoint = params["base_url"] as? String ?? ""
            let model = params["model"] as? String ?? ""
            providerSaveEndpoints.append(endpoint)
            beginMutation(event: "provider.save.start")
            if suspendMutations {
                await withCheckedContinuation { continuation in
                    providerSaveContinuations.append(continuation)
                }
            }
            providerEndpoint = endpoint
            providerModel = model
            finishMutation(event: "provider.save.complete")
            return Self.ok(["profile": NSNull()])

        case "config.readClaudeSettings":
            return await configReadResponse(
                method: method,
                oldResult: configDocument(agent: "claude-code", content: "config-x"),
                currentResult: configDocument(agent: "claude-code", content: configContent)
            )

        case "config.readAgentConfig":
            let agent = params["agent"] as? String ?? "claude-code"
            events.append("config.readAgentConfig:\(agent)")
            return await configReadResponse(
                method: method,
                oldResult: [configDocument(agent: agent, content: "config-x")],
                currentResult: [configDocument(agent: agent, content: configContent)]
            )

        case "snapshot.listAgentConfig":
            let agent = params["agent"] as? String ?? "claude-code"
            events.append("snapshot.listAgentConfig:\(agent)")
            return await configReadResponse(
                method: method,
                oldResult: [snapshot(agent: agent, id: "snapshot-old", content: "config-x")],
                currentResult: [snapshot(agent: agent, id: "snapshot-new", content: configContent)]
            )

        case "llm.listProviderProfiles":
            return Self.ok(providerStatus())
        case "app.stateSnapshot":
            return Self.ok([
                "status": serviceStatus(),
                "skills": [],
                "findings": [],
                "conflicts": [],
                "snapshots": [],
            ])
        case "llm.status":
            return Self.ok([
                "enabled": false,
                "disabled_reason": "disabled",
                "supported_actions": [],
            ])
        case "project.getContext":
            return Self.ok(["active": NSNull(), "recent": []])
        case "rules.listTuning":
            return Self.ok(["records": []])
        case "llm.listPromptRuns":
            return Self.ok(["runs": []])
        case "skill.listEvents":
            return Self.ok([])
        default:
            return Self.error(code: "unknown_method", message: "unknown method: \(method)")
        }
    }

    func releaseNextConfigSave() {
        guard !configSaveContinuations.isEmpty else { return }
        configSaveContinuations.removeFirst().resume()
    }

    func releaseNextProviderSave() {
        guard !providerSaveContinuations.isEmpty else { return }
        providerSaveContinuations.removeFirst().resume()
    }

    func releaseInitialConfigReads() {
        configReadSuspensionActive = false
        let reads = initialConfigReads
        initialConfigReads.removeAll()
        for read in reads {
            read.continuation.resume(returning: read.response)
        }
    }

    func activateConfigReadSuspension() {
        guard supportsConfigReadSuspension else { return }
        suspendedConfigReadMethods.removeAll()
        configReadSuspensionActive = true
    }

    func setExternalConfigContent(_ content: String) {
        configContent = content
    }

    func setExternalProvider(endpoint: String, model: String) {
        providerEndpoint = endpoint
        providerModel = model
    }

    private func configReadResponse(method: String, oldResult: Any, currentResult: Any) async -> Data {
        let count = (readCounts[method] ?? 0) + 1
        readCounts[method] = count
        guard configReadSuspensionActive,
              suspendedConfigReadMethods.insert(method).inserted else {
            return Self.ok(currentResult)
        }
        let oldResponse = Self.ok(oldResult)
        return await withCheckedContinuation { continuation in
            initialConfigReads.append(
                SuspendedConfigRead(continuation: continuation, response: oldResponse)
            )
        }
    }

    private func beginMutation(event: String) {
        activeMutationCount += 1
        maximumConcurrentMutationCount = max(maximumConcurrentMutationCount, activeMutationCount)
        events.append(event)
    }

    private func finishMutation(event: String) {
        activeMutationCount -= 1
        events.append(event)
    }

    private func configDocument(agent: String, content: String) -> [String: Any] {
        [
            "agent": agent,
            "scope": "agent-global",
            "target": "/tmp/home/.\(agent)/config",
            "format": "json",
            "content": content,
            "exists": true,
            "revision": "sha256:test-\(content)",
        ]
    }

    private func snapshot(agent: String, id: String, content: String) -> [String: Any] {
        [
            "id": id,
            "agent": agent,
            "scope": "agent-global",
            "target": "/tmp/home/.\(agent)/config",
            "content": content,
            "reason": "pre-config-edit",
            "created_at": id == "snapshot-new" ? 2 : 1,
        ]
    }

    private func providerStatus() -> [String: Any] {
        [
            "service_available": true,
            "enabled": true,
            "configured": true,
            "active_profile_id": "openai-compatible",
            "credential_storage": "keychain",
            "credential_persistence_allowed": true,
            "profiles": [[
                "id": "openai-compatible",
                "kind": "openai-compatible",
                "endpoint": providerEndpoint,
                "model": providerModel,
                "enabled": true,
                "configured": true,
                "has_api_key": true,
            ]],
        ]
    }

    private func serviceStatus() -> [String: Any] {
        [
            "protocol_version": 2,
            "version": "test",
            "app_data_dir": "/tmp/app-data",
            "catalog_path": "/tmp/app-data/catalog.sqlite",
            "user_home": "/tmp/home",
            "supported_methods": [
                "app.stateSnapshot",
                "config.readClaudeSettings",
                "config.readAgentConfig",
                "config.saveClaudeSettings",
                "snapshot.listAgentConfig",
                "llm.listProviderProfiles",
                "llm.saveProviderProfile",
            ],
        ]
    }

    private static func ok(_ result: Any) -> Data {
        json(["id": "test", "ok": true, "result": result])
    }

    private static func error(code: String, message: String) -> Data {
        json([
            "id": "test",
            "ok": false,
            "result": NSNull(),
            "error": ["code": code, "message": message],
        ])
    }

    private static func json(_ value: Any) -> Data {
        (try? JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])) ?? Data()
    }
}

private enum CatalogRefreshScanFixture {
    case partial
    case complete
    case legacySummary
    case legacyWithoutActivity
}

private actor CatalogRefreshScanSequence {
    private let fixtures: [CatalogRefreshScanFixture]
    private var index = 0

    init(fixtures: [CatalogRefreshScanFixture]) {
        self.fixtures = fixtures.isEmpty ? [.partial] : fixtures
    }

    func next() -> CatalogRefreshScanFixture {
        let fixture = fixtures[min(index, fixtures.count - 1)]
        index += 1
        return fixture
    }
}

private final class CatalogRefreshServiceRunner: ServiceProcessRunning {
    private let recorder = CatalogRefreshCallRecorder()
    private let scanSequence: CatalogRefreshScanSequence

    init(scanFixtures: [CatalogRefreshScanFixture] = [.partial]) {
        scanSequence = CatalogRefreshScanSequence(fixtures: scanFixtures)
    }

    func serviceClient() -> ServiceClient {
        ServiceClient(
            processRunner: self,
            serviceURL: URL(fileURLWithPath: "/tmp/catalog-refresh-service")
        )
    }

    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) async throws -> Data {
        let rawInput = String(data: input, encoding: .utf8) ?? ""
        await recorder.record(rawInput)

        let object = try JSONSerialization.jsonObject(with: input) as? [String: Any]
        let method = object?["method"] as? String ?? ""
        switch method {
        case "catalog.scanAll":
            return Self.ok(Self.scanResult(for: await scanSequence.next()))
        case "app.stateSnapshot":
            return Self.ok(Self.stateSnapshot)
        case "catalog.getSkill":
            return Self.ok(Self.detail(for: rawInput))
        case "skill.listEvents", "snapshot.listAgentConfig":
            return Self.ok("[]")
        case "project.getContext":
            return Self.ok(Self.projectContext)
        case "llm.status",
             "llm.listProviderProfiles",
             "llm.listPromptRuns",
             "rules.listTuning":
            return Self.unknown(method)
        default:
            return Self.unexpected(method)
        }
    }

    func calls() async -> String {
        await recorder.calls()
    }

    private static func ok(_ result: String) -> Data {
        Data("{\"id\":\"test\",\"ok\":true,\"result\":\(result)}".utf8)
    }

    private static func unknown(_ method: String) -> Data {
        Data("{\"id\":\"test\",\"ok\":false,\"result\":null,\"error\":{\"code\":\"unknown_method\",\"message\":\"unknown method: \(method)\"}}".utf8)
    }

    private static func unexpected(_ method: String) -> Data {
        Data("{\"id\":\"test\",\"ok\":false,\"result\":null,\"error\":{\"code\":\"unexpected_method\",\"message\":\"unexpected method: \(method)\"}}".utf8)
    }

    private static func detail(for rawInput: String) -> String {
        if rawInput.contains("\"instance_id\":\"gamma\"") {
            return detailGamma
        }
        if rawInput.contains("\"instance_id\":\"beta\"") {
            return detailBeta
        }
        return detailAlpha
    }

    private static func scanResult(for fixture: CatalogRefreshScanFixture) -> String {
        switch fixture {
        case .partial:
            partialScanResult
        case .complete:
            completeScanResult
        case .legacySummary:
            legacySummaryScanResult
        case .legacyWithoutActivity:
            legacyActivitylessScanResult
        }
    }

    private static let supportedMethods = """
    ["app.stateSnapshot","catalog.scanAll","catalog.getSkill","skill.listEvents","snapshot.listAgentConfig","project.getContext","llm.status","llm.listProviderProfiles","llm.listPromptRuns","rules.listTuning"]
    """

    private static let status = """
    {"protocol_version":1,"version":"test","app_data_dir":"/tmp/skills-copilot","catalog_path":"/tmp/skills-copilot/catalog.sqlite","user_home":"/tmp/home","supported_methods":\(supportedMethods)}
    """

    private static let skills = """
    [{"id":"alpha","agent":"claude-code","scope":"agent-global","path":"/tmp/global/alpha/SKILL.md","display_path":"/tmp/global/alpha/SKILL.md","definition_id":"def.alpha","name":"Alpha","state":"loaded","enabled":true},{"id":"beta","agent":"claude-code","scope":"agent-project","path":"/tmp/project/beta/SKILL.md","display_path":"/tmp/project/beta/SKILL.md","definition_id":"def.beta","name":"Beta","state":"loaded","enabled":true},{"id":"gamma","agent":"codex","scope":"agent-global","path":"/tmp/codex/skills/gamma/SKILL.md","display_path":"~/.codex/skills/gamma/SKILL.md","definition_id":"codex:gamma","name":"Gamma","state":"loaded","enabled":true}]
    """

    private static let stateSnapshot = """
    {"status":\(status),"skills":\(skills),"findings":[],"conflicts":[],"snapshots":[]}
    """

    private static let partialScanResult = """
    {"scanned_count":3,"skills":\(skills),"activity":{"operation":"catalog.scanAll","status":"completed-partial","started_at":1,"finished_at":2,"scanned_count":3,"skill_count":3,"finding_count":0,"conflict_count":0,"snapshot_count":0,"roots":["$HOME/.claude/skills","$HOME/.agents/skills","<adapter-root>/missing-opencode"],"log_entries":[{"level":"warning","message":"Claude Code discovered 2 skill(s); catalog now has 2 skill(s), 0 broken, across 0 complete root(s), 1 partial root(s), and 0 skipped root(s); first scan issue entry_unreadable at <adapter-root>/dangling-link: A directory entry could not be inspected or resolved."},{"level":"info","message":"Codex discovered 1 skill(s); catalog now has 1 skill(s), 0 broken, across 1 complete root(s), 0 partial root(s), and 0 skipped root(s)."},{"level":"warning","message":"opencode discovered 0 skill(s); catalog now has 0 skill(s), 0 broken, across 0 complete root(s), 0 partial root(s), and 1 skipped root(s); root-error skipped-root path(s): <adapter-root>/missing-opencode."}],"recovery_actions":["Review partial-root diagnostics; unseen rows under partial roots were preserved."],"agent_summaries":[{"agent":"claude-code","display_label":"Claude Code","status":"completed-partial","scanned_count":2,"catalog_count":2,"broken_count":0,"roots_considered":["$HOME/.claude/skills"],"roots_scanned":[],"roots_partial":["<adapter-root>"],"roots_skipped":[],"scan_issues":[{"kind":"entry_unreadable","path":"<adapter-root>/dangling-link","detail":"A directory entry could not be inspected or resolved."}],"recovery_actions":["Review partial scan diagnostics."]},{"agent":"codex","display_label":"Codex","status":"completed","scanned_count":1,"catalog_count":1,"broken_count":0,"roots_considered":["$HOME/.agents/skills"],"roots_scanned":["$HOME/.agents/skills"],"roots_partial":[],"roots_skipped":[],"scan_issues":[],"recovery_actions":[]},{"agent":"opencode","display_label":"opencode","status":"completed-with-skipped-roots","scanned_count":0,"catalog_count":0,"broken_count":0,"roots_considered":["<adapter-root>/missing-opencode"],"roots_scanned":[],"roots_partial":[],"roots_skipped":["<adapter-root>/missing-opencode"],"scan_issues":[{"kind":"root_unavailable","path":"<adapter-root>/missing-opencode","detail":"A declared scan root was unavailable or not a directory."}],"recovery_actions":["Review opencode skipped-root diagnostics, then retry Scan."]}]}}
    """

    private static let completeScanResult = """
    {"scanned_count":3,"skills":\(skills),"activity":{"operation":"catalog.scanAll","status":"completed","started_at":3,"finished_at":4,"scanned_count":3,"skill_count":3,"finding_count":0,"conflict_count":0,"snapshot_count":0,"roots":["$HOME/.claude/skills"],"log_entries":[],"recovery_actions":[],"agent_summaries":[{"agent":"claude-code","display_label":"Claude Code","status":"completed","scanned_count":3,"catalog_count":3,"broken_count":0,"roots_considered":["$HOME/.claude/skills"],"roots_scanned":["$HOME/.claude/skills"],"roots_partial":[],"roots_skipped":[],"scan_issues":[],"recovery_actions":[]}]}}
    """

    private static let legacySummaryScanResult = """
    {"scanned_count":3,"skills":\(skills),"activity":{"operation":"catalog.scanAll","status":"completed","started_at":5,"finished_at":6,"scanned_count":3,"skill_count":3,"finding_count":0,"conflict_count":0,"snapshot_count":0,"roots":["$HOME/.claude/skills"],"log_entries":[],"recovery_actions":[],"agent_summaries":[{"agent":"claude-code","display_label":"Claude Code","status":"completed","scanned_count":3,"catalog_count":3,"broken_count":0,"roots_considered":["$HOME/.claude/skills"],"roots_scanned":["$HOME/.claude/skills"],"roots_skipped":[],"recovery_actions":[]}]}}
    """

    private static let legacyActivitylessScanResult = """
    {"scanned_count":3,"skills":\(skills)}
    """

    private static let projectContext = """
    {"active":null,"recent":[]}
    """

    private static let detailAlpha = """
    {"id":"alpha","agent":"claude-code","scope":"agent-global","path":"/tmp/global/alpha/SKILL.md","display_path":"/tmp/global/alpha/SKILL.md","definition_id":"def.alpha","name":"Alpha","description":"Alpha fixture","state":"loaded","enabled":true,"frontmatter_raw":"","body":"","permissions":{},"fingerprint":"fp-alpha"}
    """

    private static let detailBeta = """
    {"id":"beta","agent":"claude-code","scope":"agent-project","path":"/tmp/project/beta/SKILL.md","display_path":"/tmp/project/beta/SKILL.md","definition_id":"def.beta","name":"Beta","description":"Beta fixture","state":"loaded","enabled":true,"frontmatter_raw":"","body":"","permissions":{},"fingerprint":"fp-beta"}
    """

    private static let detailGamma = """
    {"id":"gamma","agent":"codex","scope":"agent-global","path":"/tmp/codex/skills/gamma/SKILL.md","display_path":"~/.codex/skills/gamma/SKILL.md","definition_id":"codex:gamma","name":"Gamma","description":"Gamma fixture","state":"loaded","enabled":true,"frontmatter_raw":"","body":"","permissions":{},"fingerprint":"fp-gamma"}
    """
}

private actor CatalogRefreshCallRecorder {
    private var recordedCalls: [String] = []

    func record(_ rawInput: String) {
        recordedCalls.append(rawInput)
    }

    func calls() -> String {
        recordedCalls.joined(separator: "\n")
    }
}

private final class TaskCockpitFallbackServiceRunner: ServiceProcessRunning {
    private let recorder = TaskCockpitFallbackCallRecorder()

    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) async throws -> Data {
        let rawInput = String(data: input, encoding: .utf8) ?? ""
        await recorder.record(rawInput)

        let object = try JSONSerialization.jsonObject(with: input) as? [String: Any]
        let method = object?["method"] as? String ?? ""
        switch method {
        case "app.stateSnapshot":
            return Data(Self.stateSnapshotResponse.utf8)
        case "llm.previewPrompt":
            return Data(Self.unknownPreviewPromptResponse.utf8)
        default:
            return Data(Self.unexpectedMethodResponse(method).utf8)
        }
    }

    func calls() async -> String {
        await recorder.calls()
    }

    private static let stateSnapshotResponse = """
    {"id":"test","ok":true,"result":{"status":{"protocol_version":1,"version":"test","app_data_dir":"/tmp/skills-copilot","catalog_path":"/tmp/skills-copilot/catalog.sqlite","user_home":"/tmp/home","supported_methods":["app.stateSnapshot","llm.previewPrompt"]},"skills":[{"id":"alpha","agent":"claude-code","scope":"agent-global","path":"/tmp/global/alpha/SKILL.md","display_path":"/tmp/global/alpha/SKILL.md","definition_id":"def.alpha","name":"Alpha","state":"loaded","enabled":true},{"id":"beta","agent":"claude-code","scope":"agent-project","path":"/tmp/project/beta/SKILL.md","display_path":"/tmp/project/beta/SKILL.md","definition_id":"def.beta","name":"Beta","state":"loaded","enabled":true}],"findings":[],"conflicts":[]}}
    """

    private static let unknownPreviewPromptResponse = """
    {"id":"test","ok":false,"result":null,"error":{"code":"unknown_method","message":"unknown method: llm.previewPrompt"}}
    """

    private static func unexpectedMethodResponse(_ method: String) -> String {
        """
        {"id":"test","ok":false,"result":null,"error":{"code":"unexpected_method","message":"unexpected method: \(method)"}}
        """
    }
}

private actor TaskCockpitFallbackCallRecorder {
    private var recordedCalls: [String] = []

    func record(_ rawInput: String) {
        recordedCalls.append(rawInput)
    }

    func calls() -> String {
        recordedCalls.joined(separator: "\n")
    }
}
