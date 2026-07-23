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
        try runCase("workspaceRoutesDoNotManufactureSkillSelection") {
            try workspaceRoutesDoNotManufactureSkillSelection()
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
        try await runCase("localSessionPrewarmMoreAndAllUseCursorPages") {
            try await localSessionPrewarmMoreAndAllUseCursorPages()
        }
        try await runCase("localSessionStoreAggregatesSkillUsageAcrossDelayedPages") {
            try await localSessionStoreAggregatesSkillUsageAcrossDelayedPages()
        }
        try await runCase("cancelledAndStaleLocalSessionPagesCannotPublish") {
            try await cancelledAndStaleLocalSessionPagesCannotPublish()
        }
        try await runCase("failedInitialLocalSessionPageRetriesFromNilCursor") {
            try await failedInitialLocalSessionPageRetriesFromNilCursor()
        }
        try await runCase("failedLocalSessionPrewarmRetainsPagesAndRetriesCursor") {
            try await failedLocalSessionPrewarmRetainsPagesAndRetriesCursor()
        }
        try await runCase("oldLocalSessionGenerationErrorCannotOverwriteReactivatedSource") {
            try await oldLocalSessionGenerationErrorCannotOverwriteReactivatedSource()
        }
        try await runCase("localSessionTerminalPageUsesDecreasingExactTotal") {
            try await localSessionTerminalPageUsesDecreasingExactTotal()
        }
        try await runCase("localSessionZeroRowPageContinuesWhenCursorProgresses") {
            try await localSessionZeroRowPageContinuesWhenCursorProgresses()
        }
        try await runCase("localSessionZeroRowPageRejectsRepeatedCursor") {
            try await localSessionZeroRowPageRejectsRepeatedCursor()
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
        try await runCase("catalogScanPresentationFollowsSelectedAgent") {
            try await catalogScanPresentationFollowsSelectedAgent()
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
        try await runCase("appSearchViewAllRoutesCanonicallyWithoutRPC") {
            try await appSearchViewAllRoutesCanonicallyWithoutRPC()
        }
        try await runCase("agentConfigTimelineFollowsSelectedAgentFilterOnly") {
            try await agentConfigTimelineFollowsSelectedAgentFilterOnly()
        }
        try await runCase("localHistoriesAutoLoadEveryPage") {
            try await localHistoriesAutoLoadEveryPage()
        }
        try await runCase("localHistoryFailureRetryPreservesRows") {
            try await localHistoryFailureRetryPreservesRows()
        }
        try await runCase("initialEventPageFailureRetriesFromNilCursor") {
            try await initialEventPageFailureRetriesFromNilCursor()
        }
        try await runCase("selectedCachedPartialEventHistoryRetriesOnRevisit") {
            try await selectedCachedPartialEventHistoryRetriesOnRevisit()
        }
        try await runCase("invalidatedEventGenerationCannotABAIntoReplacement") {
            try await invalidatedEventGenerationCannotABAIntoReplacement()
        }
        try await runCase("cancellingLocalHistoryLoadAllPreservesAcceptedRows") {
            try await cancellingLocalHistoryLoadAllPreservesAcceptedRows()
        }
        try await runCase("catalogScanCompletenessTracksExplicitScan") {
            try await catalogScanCompletenessTracksExplicitScan()
        }
        try await runCase("readOnlyAgentConfigLoadsCurrentDocuments") {
            try await readOnlyAgentConfigLoadsCurrentDocuments()
        }
        try await runCase("explicitConfigSaveRequiresPreviewAndVerifiedApply") {
            try await explicitConfigSaveRequiresPreviewAndVerifiedApply()
        }
        try await runCase("configSaveDraftChangeInvalidatesDelayedPreviewAndApply") {
            try await configSaveDraftChangeInvalidatesDelayedPreviewAndApply()
        }
        try await runCase("staleConfigSaveDoesNotRetryAndReloadsCurrentDocument") {
            try await staleConfigSaveDoesNotRetryAndReloadsCurrentDocument()
        }
        try await runCase("configSaveRejectsIncompleteReadback") {
            try await configSaveRejectsIncompleteReadback()
        }
        try await runCase("snapshotRefreshFailureDoesNotReclassifyVerifiedConfigSave") {
            try await snapshotRefreshFailureDoesNotReclassifyVerifiedConfigSave()
        }
        try await runCase("agentConfigRefreshIfNeededSkipsLoadedRequestAndScopeFiltersLocally") {
            try await agentConfigRefreshIfNeededSkipsLoadedRequestAndScopeFiltersLocally()
        }
        try await runCase("providerMutationsRequireExplicitPreviewAndConfirmation") {
            try await providerMutationsRequireExplicitPreviewAndConfirmation()
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
        try await runCase("allAgentConfigHistoryPageTaskPreservesCachedRowsWithoutRPC") {
            try await allAgentConfigHistoryPageTaskPreservesCachedRowsWithoutRPC()
        }
        try await runCase("allAgentConfigHistoryInvalidatesActiveSameAgentRequestKey") {
            try await allAgentConfigHistoryInvalidatesActiveSameAgentRequestKey()
        }
        try await runCase("allAgentConfigHistoryInvalidatesCompletedSameAgentRequestKey") {
            try await allAgentConfigHistoryInvalidatesCompletedSameAgentRequestKey()
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
        try await runCase("removeRecentProjectUpdatesCacheWithoutScanning") {
            try await removeRecentProjectUpdatesCacheWithoutScanning()
        }
        try await runCase("clearRecentProjectsUpdatesCacheWithoutScanning") {
            try await clearRecentProjectsUpdatesCacheWithoutScanning()
        }
        try await runCase("projectValidationErrorSkipsScanAndSurfacesMessage") {
            try await projectValidationErrorSkipsScanAndSurfacesMessage()
        }
        try await runCase("reloadSurfacesUnsupportedLLMStatusMethod") {
            try await reloadSurfacesUnsupportedLLMStatusMethod()
        }
        try await runCase("providerObservabilityUsesReadOnlyServiceContract") {
            try await providerObservabilityUsesReadOnlyServiceContract()
        }
        try await runCase("providerObservabilityPreloadsAtStartupAndRefreshesWithReload") {
            try await providerObservabilityPreloadsAtStartupAndRefreshesWithReload()
        }
        try await runCase("providerObservabilitySurfacesMethodUnavailable") {
            try await providerObservabilitySurfacesMethodUnavailable()
        }
        try await runCase("providerActivityAccumulatesAllPagesWithoutChangingSummary") {
            try await providerActivityAccumulatesAllPagesWithoutChangingSummary()
        }
        try await runCase("providerActivityNotifiesAfterEachAcceptedPage") {
            try await providerActivityNotifiesAfterEachAcceptedPage()
        }
        try await runCase("providerActivityCancellationAndStaleGenerationPreserveAcceptedRows") {
            try await providerActivityCancellationAndStaleGenerationPreserveAcceptedRows()
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
        try await runCase("successfulTaskCockpitKeepsSessionHistoryInMemory") {
            try await successfulTaskCockpitKeepsSessionHistoryInMemory()
        }
        try await runCase("clearTaskCockpitHistoryClearsMemory") {
            try await clearTaskCockpitHistoryClearsMemory()
        }
        try await runCase("taskCockpitPreservesExactUserInputInServiceContract") {
            try await taskCockpitPreservesExactUserInputInServiceContract()
        }
        try await runCase("taskCockpitWhitespaceOnlyInputRequiresTask") {
            try await taskCockpitWhitespaceOnlyInputRequiresTask()
        }
        try await runCase("taskCockpitSurfacesMethodUnavailable") {
            try await taskCockpitSurfacesMethodUnavailable()
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
        try runCase("skillManagerReturnedAndLocalCollectionsHaveNoLegacyDisplayCaps") {
            try skillManagerReturnedAndLocalCollectionsHaveNoLegacyDisplayCaps()
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
        try expectEqual(store.appRoute, .overview, "The project-first route should start at Overview.")
        try expectEqual(store.sidebarContentMode, .skills, "Agent Copilot should start from the Skills primary navigation.")
        try expectEqual(store.selectedDetailSection, .overview, "Default detail section should stay neutral until a session, skill, report, or Preflight is selected.")
    }

    private func workspaceRoutesDoNotManufactureSkillSelection() throws {
        let store = SkillStore(service: ServiceClient())
        for route in AppRoute.allCases {
            store.selectAppRoute(route)
            try expectEqual(store.appRoute, route, "Composition-root route selection")
            try expectNil(
                store.selectedSkillID,
                "Selecting \(route.rawValue) must not manufacture a selected skill."
            )
            try expectFalse(
                store.selectedSidebarSelection?.isSkill == true,
                "Selecting \(route.rawValue) must not manufacture a skill detail."
            )
        }
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

    private func skillManagerReturnedAndLocalCollectionsHaveNoLegacyDisplayCaps() throws {
        var installed = SkillManagerVisibleResults<String>()
        let installedIDs = Array(0..<27).map(String.init)
        installed.loadAll(totalReturned: installedIDs.count)
        try expectEqual(installed.visibleItems(in: installedIDs).count, 27, "Installed results should not retain the old 12-row display cap.")

        let localLibraryIDs = Array(0..<31).map(String.init)
        try expectEqual(localLibraryIDs.count, 31, "The complete local library should not retain the old 12-row display cap.")
        let status = emptySkillManagerSearchRecord().listStatus(visibleCount: 0)
        try expectEqual(status.loadedCount, 0, "Empty or blocked search should show zero loaded rows.")
        try expectEqual(status.totalCount, nil, "Empty or blocked remote search total should remain unknown.")
        try expectEqual(status.incompleteReason, .sourceLimited, "Empty or blocked search should expose its typed limitation.")
        try expectEqual(status.canLoadMore, false, "Empty returned search must not offer Load More.")
        try expectEqual(status.canLoadAll, false, "Empty returned search must not offer Load All.")
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
        try await waitUntil("Selecting a summary should load exactly one bounded detail.") {
            guard countMethodCalls("session.previewLocalSessions", in: fake.calls()) == 2,
                  case .loaded(let detail) = store.selectedLocalSessionDetailState else { return false }
            return detail.id == "session-alpha"
                && detail.contentIncluded
                && detail.contentItems.map(\.text) == ["Bounded alpha detail"]
        }
        let callsAfterDetail = fake.calls()
        try expectContains(callsAfterDetail, #""include_content_items":true"#, "Detail request should explicitly include content items.")
        try expectContains(callsAfterDetail, #""session_id":"session-alpha""#, "Detail request should target only the selected stable id.")
        try expectContains(callsAfterDetail, #""limit":1"#, "Detail request should request one row.")

        store.selectLocalSession(summary)
        try? await Task.sleep(nanoseconds: 80_000_000)
        try expectEqual(countMethodCalls("session.previewLocalSessions", in: fake.calls()), 2, "Re-selecting a cached detail should not issue another RPC.")

        try await staleDetailGenerationCannotMutatePublishedState()
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

        for scenario in [
            "sessions-detail-empty",
            "sessions-detail-wrong-id",
            "sessions-detail-unavailable",
            "sessions-detail-summary-only"
        ] {
            try await nonconformingDetailResponseFailsLocallyAndRetries(scenario: scenario)
        }
    }

    private func nonconformingDetailResponseFailsLocallyAndRetries(scenario: String) async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: scenario)
        let store = SkillStore(service: fake.serviceClient())
        store.sidebarContentMode = .sessions
        await store.refreshSelectedAgentLocalSessionsIfNeeded()
        guard let summary = store.filteredLocalSessionRows.first(where: { $0.id == "session-alpha" }) else {
            throw NativeModelTestFailure(description: "Malformed-response fixture should contain session-alpha.")
        }
        let summaryRows = store.localSessionPreviewResult.sessionRows
        let summaryCounts = (
            store.scopedLocalSessionUserMessageCount,
            store.scopedLocalSessionTotalMessageCount,
            store.scopedLocalSessionToolCallCount,
            store.scopedLocalSessionSkillCallCount
        )

        store.selectLocalSession(summary)
        try await waitUntil("\(scenario) should become a retryable detail-local failure.") {
            if case .failed = store.selectedLocalSessionDetailState { return true }
            return false
        }
        try expectEqual(store.selectedLocalSessionID, summary.id, "\(scenario) should preserve selection.")
        try expectEqual(store.localSessionPreviewResult.sessionRows, summaryRows, "\(scenario) should preserve summary rows.")
        try expectEqual(store.scopedLocalSessionUserMessageCount, summaryCounts.0, "\(scenario) should preserve user counts.")
        try expectEqual(store.scopedLocalSessionTotalMessageCount, summaryCounts.1, "\(scenario) should preserve message counts.")
        try expectEqual(store.scopedLocalSessionToolCallCount, summaryCounts.2, "\(scenario) should preserve tool counts.")
        try expectEqual(store.scopedLocalSessionSkillCallCount, summaryCounts.3, "\(scenario) should preserve skill counts.")
        try expectNil(store.errorMessage, "\(scenario) should not publish a global error.")

        store.selectLocalSession(summary)
        try await waitUntil("\(scenario) should issue a new RPC and load bounded detail on retry.") {
            guard countMethodCalls("session.previewLocalSessions", in: fake.calls()) == 3,
                  case .loaded(let detail) = store.selectedLocalSessionDetailState else { return false }
            return detail.id == summary.id
                && detail.contentIncluded
                && detail.contentItems.map(\.text) == ["Bounded alpha detail"]
        }
    }

    private func staleDetailGenerationCannotMutatePublishedState() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "sessions-delayed-detail")
        let store = SkillStore(service: fake.serviceClient())
        store.sidebarContentMode = .sessions
        await store.refreshSelectedAgentLocalSessionsIfNeeded()
        guard let summary = store.filteredLocalSessionRows.first(where: { $0.id == "session-alpha" }) else {
            throw NativeModelTestFailure(description: "Delayed-detail fixture should contain session-alpha.")
        }
        let summaryResult = store.localSessionPreviewResult

        store.selectLocalSession(summary)
        try await waitUntil("The old detail generation should be in flight.") {
            countMethodCalls("session.previewLocalSessions", in: fake.calls()) == 2
                && store.selectedLocalSessionDetailState != nil
        }

        fake.setScenario("sessions-new-detail")
        store.agentFilter = .codex
        store.agentFilter = .claudeCode
        guard let restoredSummary = store.filteredLocalSessionRows.first(where: { $0.id == summary.id }) else {
            throw NativeModelTestFailure(description: "Returning to source A should restore its cached summary.")
        }
        store.selectLocalSession(restoredSummary)
        try await waitUntil("The new detail generation should win before the old response is released.") {
            guard countMethodCalls("session.previewLocalSessions", in: fake.calls()) == 3,
                  case .loaded(let detail) = store.selectedLocalSessionDetailState else { return false }
            return detail.contentItems.map(\.text) == ["FRESH ALPHA DETAIL"]
        }
        try expectEqual(store.localSessionPreviewResult, summaryResult, "Accepted raw detail should remain solely in the bounded detail cache.")

        fake.releaseBlockedResponse()
        try await waitUntil("The old detail response should complete after release.") {
            fake.delayedDetailResponseCompleted()
        }
        try? await Task.sleep(nanoseconds: 80_000_000)
        guard case .loaded(let acceptedDetail) = store.selectedLocalSessionDetailState else {
            throw NativeModelTestFailure(description: "The current detail generation should remain loaded.")
        }
        try expectEqual(acceptedDetail.contentItems.map(\.text), ["FRESH ALPHA DETAIL"], "Late old detail must not replace the accepted cache entry.")
        try expectEqual(store.localSessionPreviewResult, summaryResult, "Late old detail must not mutate published summary state.")
        try expectFalse(store.localSessionPreviewResult.sessionRows.contains { row in
            row.contentItems.contains { $0.text == "Bounded alpha detail" }
        }, "Late old raw content must not be retained in the published preview.")
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
        try expectEqual(countMethodCalls("snapshot.listAgentConfigPage", in: calls), 0, "Reload should not block the core refresh on selected agent config history.")
        try expectContains(calls, "llm.status", "Reload should preserve the separate LLM status behavior.")
        try expectContains(calls, "project.getContext", "Reload should preserve the separate project context behavior.")
        try await waitUntil("Reload should refresh selected agent config history in the background.") {
            countMethodCalls("snapshot.listAgentConfigPage", in: fake.calls()) == 1
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
        try expectEqual(countMethodCalls("snapshot.listAgentConfigPage", in: calls), 0, "Startup should not block the progress overlay on selected-agent config history.")
        try expectFalse(calls.contains("session.previewLocalSessions"), "Startup should not block the progress overlay on selected-agent local sessions.")
        try expectFalse(calls.contains("config.readAgentConfig"), "Startup should not block the progress overlay on selected-agent current config documents.")
        try expectContains(calls, "catalog.getSkill", "Startup should prewarm the selected skill detail.")
        try expectEqual(countMethodCalls("llm.listProviderProfiles", in: calls), 0, "Startup should not block the progress overlay on AI provider status.")
        try expectFalse(calls.contains("\"method\":\"catalog.scanAll\""), "Startup should not scan roots automatically.")
        try expectFalse(calls.contains("\"method\":\"skillManager.listInstalled\""), "Startup should not invoke external manager inventory.")
        try expectFalse(calls.contains("\"method\":\"skillManager.search\""), "Startup should not invoke external manager search or network.")
        try expectFalse(calls.contains("\"method\":\"config.toggleSkill\""), "Startup should not write agent config.")
        try await waitUntil("Startup should prewarm supplemental launch data in the background.") {
            store.localSessionPreviewResult.sessionRows.count == 2
                && countMethodCalls("snapshot.listAgentConfigPage", in: fake.calls()) == 1
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
        try await previewAndConfirmSelectedSkillToggle(store, on: false)

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
        try expectContains(store.refreshStatusMessage, "1 visible issues", "Scan feedback should report the same filtered issue total as the navigable skill UI, not the raw Catalog finding count.")
        try expectContains(store.refreshStatusMessage, "<adapter-root>/dangling-link", "The primary partial status should include the first redacted issue path.")
        try expectContains(store.refreshStatusMessage, "Review partial scan diagnostics.", "The primary partial status should include a recovery action.")
        try expectContains(store.lastMutationMessage ?? "", "Scanned 3 skills", "The mutation toast should stay generic so unrelated agent filters do not inherit a degraded-agent warning.")
        try expectEqual(store.partialScanWarningMessage, store.refreshStatusMessage, "Persistent partial feedback must not be coupled to later generic reload status text.")
        try expectEqual(store.lastScanActivity?.agentSummaries?.count, 3, "Scan should retain complete, partial, and skipped adapter diagnostics when the service provides them.")
        try expectEqual(store.lastScanActivity?.agentSummaries?.first { $0.agent == "opencode" }?.rootsSkipped, ["<adapter-root>/missing-opencode"], "Scan diagnostics should decode skipped roots.")
        try expectEqual(store.lastScanActivity?.agentSummaries?.first { $0.agent == "claude-code" }?.status, "completed-partial", "A partial adapter scan must not decode as completed.")
        try expectEqual(store.lastScanActivity?.agentSummaries?.first { $0.agent == "claude-code" }?.rootsPartial, ["<adapter-root>"], "Scan diagnostics should decode partial roots.")
        let claudeIssues = store.lastScanActivity?.agentSummaries?.first { $0.agent == "claude-code" }?.scanIssues ?? []
        try expectEqual(claudeIssues.first?.kind, "root_unavailable", "Scan diagnostics should preserve service issue order.")
        try expectEqual(claudeIssues.first { $0.kind == "entry_unreadable" }?.path, "<adapter-root>/dangling-link", "Partial feedback should be able to select the issue that actually degraded traversal.")
        store.agentFilter = .codex
        try expectEqual(store.selectedAgentRefreshSummary?.rootsScanned, ["$HOME/.agents/skills"], "Selected adapter diagnostics should follow the agent filter.")
        try expectNil(store.partialScanWarningMessage, "A completed Codex scan must not inherit Claude or opencode warnings.")
        let calls = await runner.calls()
        try expectEqual(countOccurrences("app.stateSnapshot", in: calls), 1, "Scan should refresh collections with one app state snapshot call.")
        try expectEqual(countOccurrences("catalog.listSkills", in: calls), 0, "Scan refresh should not launch a separate skills list sidecar.")
        try expectEqual(countOccurrences("catalog.listFindings", in: calls), 0, "Scan refresh should not launch a separate findings list sidecar.")
        try expectEqual(countOccurrences("catalog.listConflicts", in: calls), 0, "Scan refresh should not launch a separate conflicts list sidecar.")
        try expectEqual(countMethodCalls("snapshot.list", in: calls), 0, "Scan refresh should not launch a global snapshots list sidecar.")
        try expectContains(
            calls,
            "\"explicit_refresh\":true",
            "Catalog scan requests must carry explicit refresh authorization."
        )
        try expectEqual(
            countMethodCalls("snapshot.listAgentConfigPage", in: calls),
            0,
            "Catalog scan read-back should not launch unrelated agent config history requests."
        )
        await store.reload()
        try expectNil(store.partialScanWarningMessage, "Reloading cached data must keep the selected completed agent free of unrelated warnings.")
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
        await store.previewClearProject()
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

        let callsAfterReload = countMethodCalls("snapshot.listAgentConfigPage", in: fake.calls())
        store.selectedSkillID = "alpha"
        await store.loadSelectedDetail()
        try expectEqual(store.agentConfigSnapshots.map(\.id), ["snap-claude-new", "snap-claude-old"], "Changing skill selection within an agent must not turn config snapshots into per-skill history.")
        try expectEqual(countMethodCalls("snapshot.listAgentConfigPage", in: fake.calls()), callsAfterReload, "Skill detail changes should not reload agent config history.")

        store.agentFilter = .codex
        try await waitUntil("Codex filter should load only Codex config snapshots.") {
            store.agentConfigSnapshots.map(\.id) == ["snap-codex"]
        }
        try expectEqual(store.selectedAgentConfigTimelineAgent, "codex", "Codex timeline should use the selected agent filter.")
        try expectEqual(Set(store.agentConfigSnapshots.map(\.agent)), Set(["codex"]), "Codex timeline should not include Claude snapshots.")
        try expectContains(fake.calls(), "\"agent\":\"codex\"", "Timeline fetch should request the selected Codex agent.")

        store.agentFilter = .all
        await store.loadSelectedAgentConfigDataIfNeeded()
        try expectNil(store.selectedAgentConfigTimelineAgent, "All filter has no single selected agent timeline.")
        try expectEqual(store.agentConfigSnapshots.map(\.id), ["snap-codex"], "All filter should preserve the current cached timeline without fetching or merging agent histories.")
    }

    private func localHistoriesAutoLoadEveryPage() async throws {
        let runner = LocalHistoryPageRunner()
        let store = SkillStore(service: runner.serviceClient())

        await store.loadMoreAgentConfigSnapshots(loadAll: true)
        await store.loadMoreSkillEvents(instanceID: "skill-1", loadAll: true)

        try expectEqual(store.agentConfigSnapshots.count, 205, "Config history should auto-load every local page")
        try expectEqual(store.agentConfigSnapshotCompleteness.loadedCount, 205, "Loaded count")
        try expectEqual(store.agentConfigSnapshotCompleteness.completeness, .complete, "Config completeness")
        try expectEqual(store.agentConfigTimeline.items.count, 205, "Timeline must not cap at five")
        let events = store.skillEventsByID["skill-1"] ?? []
        try expectEqual(events.count, 201, "Skill events should auto-load every local page.")
        try expectEqual(Set(events.map(\.id)).count, 201, "Every event stable ID should be retained exactly once.")
        try expectEqual(store.skillEventCompletenessByID["skill-1"]?.completeness, .complete, "Event completeness")
        try expectEqual(runner.cursors(for: "snapshot.listAgentConfigPage"), [nil, "config-100", "config-200"], "Config pages must be serial keyset continuations.")
        try expectEqual(runner.cursors(for: "skill.listEventsPage"), [nil, "event-100", "event-200"], "Event pages must be serial keyset continuations.")
    }

    private func localHistoryFailureRetryPreservesRows() async throws {
        let runner = LocalHistoryPageRunner(failSecondMethods: ["skill.listEventsPage"])
        let store = SkillStore(service: runner.serviceClient())

        await store.loadMoreSkillEvents(instanceID: "skill-1", loadAll: true)

        try expectEqual(store.skillEventsByID["skill-1"]?.count, 100, "A second-page failure must retain the first accepted page.")
        try expectEqual(store.skillEventCompletenessByID["skill-1"]?.completeness, .incomplete, "A retryable page failure should remain visibly incomplete.")

        await store.loadMoreSkillEvents(instanceID: "skill-1", loadAll: true)

        try expectEqual(store.skillEventsByID["skill-1"]?.count, 201, "Retry should continue from the accepted cursor and finish.")
        try expectEqual(store.skillEventCompletenessByID["skill-1"]?.completeness, .complete, "Retry completeness")
        try expectEqual(runner.cursors(for: "skill.listEventsPage"), [nil, "event-100", "event-100", "event-200"], "Retry must reuse the last accepted cursor.")

        let changedRunner = LocalHistoryPageRunner(sourceChangedSecondMethods: ["snapshot.listAgentConfigPage"])
        let changedStore = SkillStore(service: changedRunner.serviceClient())
        await changedStore.loadMoreAgentConfigSnapshots(loadAll: true)
        try expectEqual(changedStore.agentConfigSnapshots.count, 100, "Source change must retain the accepted first page.")
        try expectEqual(changedStore.agentConfigSnapshotCompleteness.completeness, .incomplete, "Source change should be terminal incomplete.")
        try expectEqual(changedStore.agentConfigSnapshotCompleteness.incompleteReason, .sourceChanged, "Source change reason")
    }

    private func initialEventPageFailureRetriesFromNilCursor() async throws {
        let runner = LocalHistoryPageRunner(failFirstMethods: ["skill.listEventsPage"])
        let store = SkillStore(service: runner.serviceClient())

        await store.loadMoreSkillEvents(instanceID: "skill-1", loadAll: true)

        try expectEqual(store.skillEventsByID["skill-1"]?.count, 0, "An initial page failure should cache a visible empty history.")
        try expectEqual(store.skillEventCompletenessByID["skill-1"]?.completeness, .incomplete, "Initial failure completeness")
        try expectEqual(store.skillEventCompletenessByID["skill-1"]?.incompleteReason, .pageFailed, "Initial failure reason")
        try expectEqual(store.skillEventCompletenessByID["skill-1"]?.canLoadAll, true, "The footer should offer retry-all after an initial failure.")

        await store.loadMoreSkillEvents(instanceID: "skill-1", loadAll: true)

        let events = store.skillEventsByID["skill-1"] ?? []
        try expectEqual(events.count, 201, "Retry from the nil cursor should enumerate every event.")
        try expectEqual(Set(events.map(\.id)).count, 201, "Initial retry must not duplicate stable IDs.")
        try expectEqual(store.skillEventCompletenessByID["skill-1"]?.completeness, .complete, "Initial retry completeness")
        try expectEqual(runner.cursors(for: "skill.listEventsPage"), [nil, nil, "event-100", "event-200"], "Initial retry must restart at the nil cursor before continuing.")
    }

    private func selectedCachedPartialEventHistoryRetriesOnRevisit() async throws {
        let runner = LocalHistoryPageRunner(
            failSecondMethods: ["skill.listEventsPage"],
            includesSelectedSkillFixture: true
        )
        let store = SkillStore(service: runner.serviceClient())

        await store.reload()
        try await waitUntil("Initial selected history should accept the first page before exposing the partial retry state.") {
            store.skillEventsByID["skill-1"]?.count == 100
        }
        try expectEqual(store.skillEventsByID["skill-1"]?.count, 100, "Initial selected history should preserve the first page.")
        try expectEqual(store.selectedSkillEventCompleteness.completeness, .incomplete, "The selected cached failure must be visibly incomplete.")

        await store.loadSelectedDetail()

        try expectEqual(store.skillEventsByID["skill-1"]?.count, 201, "Revisiting a cached partial history should retry its accepted cursor.")
        try expectEqual(store.selectedSkillEventCompleteness.completeness, .complete, "A successful revisit should finish selected event history.")
    }

    private func invalidatedEventGenerationCannotABAIntoReplacement() async throws {
        let runner = EventHistoryABARunner()
        let store = SkillStore(service: runner.serviceClient())

        let oldTask = Task { await store.loadMoreSkillEvents(instanceID: "skill-1", loadAll: true) }
        try await waitUntil("The old event request should be suspended.") {
            runner.syncCallCount == 1
        }
        store.invalidateDetailCaches(for: ["skill-1"])

        await store.loadMoreSkillEvents(instanceID: "skill-1", loadAll: true)
        try expectEqual(store.skillEventsByID["skill-1"]?.map(\.id), [2], "The replacement generation should publish only its current response.")

        runner.releaseOldResponse()
        await oldTask.value

        try expectEqual(store.skillEventsByID["skill-1"]?.map(\.id), [2], "A late pre-invalidation response must never enter the replacement accumulator.")
        try expectEqual(store.skillEventCompletenessByID["skill-1"]?.completeness, .complete, "The stale response must not degrade replacement completeness.")
    }

    private func cancellingLocalHistoryLoadAllPreservesAcceptedRows() async throws {
        let configRunner = LocalHistoryPageRunner(delayedThirdMethods: ["snapshot.listAgentConfigPage"])
        let configStore = SkillStore(service: configRunner.serviceClient())
        let configTask = Task { await configStore.loadMoreAgentConfigSnapshots(loadAll: true) }
        try await waitUntil("Config Load All should reach the delayed third page.") {
            configRunner.syncCallCount(for: "snapshot.listAgentConfigPage") == 3
        }
        configStore.cancelAgentConfigSnapshotLoadAll()
        configRunner.release(method: "snapshot.listAgentConfigPage")
        await configTask.value
        try expectEqual(configStore.agentConfigSnapshots.count, 200, "Config cancellation must retain accepted rows and reject the delayed page.")
        try expectEqual(configStore.agentConfigSnapshotCompleteness.completeness, .partial, "Cancelled config Load All should remain partial.")

        let eventRunner = LocalHistoryPageRunner(delayedThirdMethods: ["skill.listEventsPage"])
        let eventStore = SkillStore(service: eventRunner.serviceClient())
        let eventTask = Task { await eventStore.loadMoreSkillEvents(instanceID: "skill-1", loadAll: true) }
        try await waitUntil("Event Load All should reach the delayed third page.") {
            eventRunner.syncCallCount(for: "skill.listEventsPage") == 3
        }
        eventStore.cancelSkillEventLoadAll(instanceID: "skill-1")
        eventRunner.release(method: "skill.listEventsPage")
        await eventTask.value
        try expectEqual(eventStore.skillEventsByID["skill-1"]?.count, 200, "Event cancellation must retain accepted rows and reject the delayed page.")
        try expectEqual(eventStore.skillEventCompletenessByID["skill-1"]?.completeness, .partial, "Cancelled event Load All should remain partial.")
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

    private func explicitConfigSaveRequiresPreviewAndVerifiedApply() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "config-cas")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        await store.loadClaudeSettings()
        guard let loaded = store.claudeSettings else {
            throw NativeModelTestFailure(description: "Config action fixture should load Claude settings.")
        }
        let candidate = "{\"theme\":\"dark\"}\n"
        guard let confirmation = await store.previewClaudeSettingsSave(content: candidate) else {
            throw NativeModelTestFailure(description: "A valid config preview should create an immutable confirmation.")
        }

        try expectEqual(confirmation.content, candidate, "Confirmation should preserve the exact reviewed candidate content.")
        try expectEqual(confirmation.preview.current, loaded, "Preview should be bound to the exact loaded config document.")
        try expectEqual(countMethodCalls("config.previewSaveClaudeSettings", in: fake.calls()), 1, "Save should issue one read-only preview.")
        try expectEqual(countMethodCalls("config.saveClaudeSettings", in: fake.calls()), 0, "Preview must not apply the config write.")
        try expectEqual(store.configMutationState, .awaitingConfirmation, "A valid preview should wait for explicit confirmation.")
        try expectNil(store.lastMutationMessage, "Preview must not publish saved feedback.")

        let saved = await store.applyClaudeSettingsSave(confirmation)

        try expectEqual(saved, true, "A matching confirmed save with complete read-back should succeed.")
        try expectEqual(countMethodCalls("config.saveClaudeSettings", in: fake.calls()), 1, "Explicit confirmation should issue exactly one apply RPC.")
        try expectEqual(store.claudeSettings?.content, Optional(candidate), "Verified apply should publish the returned current document.")
        try expectContains(store.lastMutationMessage, UIStrings.savedSettings, "Verified apply should publish success feedback.")
        try expectEqual(countMethodCalls("app.stateSnapshot", in: fake.calls()), 1, "A config apply should not trigger an unrelated full-state refresh.")
    }

    private func configSaveDraftChangeInvalidatesDelayedPreviewAndApply() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "config-save-preview-delay")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        await store.loadClaudeSettings()
        let candidate = "{\"theme\":\"dark\"}\n"
        let delayedPreview = Task {
            await store.previewClaudeSettingsSave(content: candidate)
        }
        try await waitUntil("The delayed config preview should reach the service.") {
            countMethodCalls("config.previewSaveClaudeSettings", in: fake.calls()) == 1
        }

        store.invalidateConfigSavePreview()
        let staleConfirmation = await delayedPreview.value

        try expectNil(staleConfirmation, "A preview completed after the draft changed must not re-establish confirmation.")
        try expectEqual(store.configMutationState, .idle, "Invalidating an in-flight preview should restore the editor to an unconfirmed state.")
        try expectEqual(store.isSavingSettings, false, "A stale preview completion must not leave the editor busy.")
        try expectEqual(countMethodCalls("config.saveClaudeSettings", in: fake.calls()), 0, "Draft invalidation must never apply the stale candidate.")

        fake.activate(scenario: "config-cas")
        guard let confirmation = await store.previewClaudeSettingsSave(content: candidate) else {
            throw NativeModelTestFailure(description: "A fresh candidate should still produce a confirmation.")
        }
        store.invalidateConfigSavePreview()
        let applied = await store.applyClaudeSettingsSave(confirmation)

        try expectEqual(applied, false, "An invalidated confirmation must be rejected before the apply RPC.")
        try expectEqual(countMethodCalls("config.saveClaudeSettings", in: fake.calls()), 0, "An invalidated confirmation must not reach the service write method.")
        try expectEqual(store.settingsErrorMessage, UIStrings.configSavePreviewAgain, "The editor should require a new preview for the changed draft.")
    }

    private func staleConfigSaveDoesNotRetryAndReloadsCurrentDocument() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "config-save-stale")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        await store.loadClaudeSettings()
        let candidate = "{\"theme\":\"dark\"}\n"
        guard let confirmation = await store.previewClaudeSettingsSave(content: candidate) else {
            throw NativeModelTestFailure(description: "Stale-save fixture should first produce a valid preview.")
        }

        let saved = await store.applyClaudeSettingsSave(confirmation)

        try expectEqual(saved, false, "A stale action reference must not report config save success.")
        try expectEqual(countMethodCalls("config.saveClaudeSettings", in: fake.calls()), 1, "A stale apply must never be retried automatically.")
        try expectEqual(countMethodCalls("config.previewSaveClaudeSettings", in: fake.calls()), 1, "A stale apply must require the user to request another preview.")
        try expectContains(store.claudeSettings?.content, "external", "Conflict handling should reload the latest config document once.")
        try expectNil(store.lastMutationMessage, "A stale apply must not publish saved feedback.")
        guard case .conflict = store.configMutationState else {
            throw NativeModelTestFailure(description: "A stale apply should leave the config editor in conflict state.")
        }
    }

    private func configSaveRejectsIncompleteReadback() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "config-save-readback-mismatch")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        await store.loadClaudeSettings()
        let candidate = "{\"theme\":\"dark\"}\n"
        guard let confirmation = await store.previewClaudeSettingsSave(content: candidate) else {
            throw NativeModelTestFailure(description: "Read-back mismatch fixture should first produce a valid preview.")
        }

        let saved = await store.applyClaudeSettingsSave(confirmation)

        try expectEqual(saved, false, "Missing snapshot read-back must fail closed in the native client.")
        try expectEqual(countMethodCalls("config.saveClaudeSettings", in: fake.calls()), 1, "Read-back rejection must not retry the write.")
        try expectContains(store.settingsErrorMessage, "does not cover every declared domain", "The user should be told that the declared read-back coverage was incomplete.")
        try expectNil(store.lastMutationMessage, "Incomplete read-back must not publish saved feedback.")
        guard case .failed = store.configMutationState else {
            throw NativeModelTestFailure(description: "Incomplete read-back should leave the config editor in a failed state.")
        }
    }

    private func snapshotRefreshFailureDoesNotReclassifyVerifiedConfigSave() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "config-save-snapshot-refresh-fails")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()
        await store.loadClaudeSettings()
        let candidate = "{\"theme\":\"dark\"}\n"
        guard let confirmation = await store.previewClaudeSettingsSave(content: candidate) else {
            throw NativeModelTestFailure(description: "Refresh-failure fixture should first produce a valid preview.")
        }

        let saved = await store.applyClaudeSettingsSave(confirmation)

        try expectEqual(saved, true, "A later timeline refresh failure must not reclassify an already verified config write.")
        try expectEqual(store.claudeSettings?.content, Optional(candidate), "The verified config document should remain published.")
        try expectContains(store.lastMutationMessage, UIStrings.savedSettings, "Verified write success should remain visible.")
        try expectEqual(countMethodCalls("config.saveClaudeSettings", in: fake.calls()), 1, "Timeline refresh failure must not retry the write.")
        try expectEqual(countMethodCalls("app.stateSnapshot", in: fake.calls()), 1, "Timeline recovery must not broaden into a full-state refresh.")
    }

    private func agentConfigRefreshIfNeededSkipsLoadedRequestAndScopeFiltersLocally() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "agent-config")

        let store = SkillStore(service: fake.serviceClient())
        await store.loadCurrentAgentConfigDocumentsIfNeeded(agent: "claude-code")
        await store.loadAgentConfigSnapshotsIfNeeded(agent: "claude-code")

        let configReadsAfterFirstLoad = countMethodCalls("config.readAgentConfig", in: fake.calls())
        let snapshotReadsAfterFirstLoad = countMethodCalls("snapshot.listAgentConfigPage", in: fake.calls())

        await store.loadCurrentAgentConfigDocumentsIfNeeded(agent: "claude-code")
        await store.loadAgentConfigSnapshotsIfNeeded(agent: "claude-code")

        try expectEqual(
            countMethodCalls("config.readAgentConfig", in: fake.calls()),
            configReadsAfterFirstLoad,
            "Need-based current config load should not reread the same agent/project payload."
        )
        try expectEqual(
            countMethodCalls("snapshot.listAgentConfigPage", in: fake.calls()),
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
            countMethodCalls("snapshot.listAgentConfigPage", in: fake.calls()),
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
            countMethodCalls("snapshot.listAgentConfigPage", in: fake.calls()),
            snapshotReadsAfterFirstLoad + 1,
            "Manual config history refresh should still force a new read."
        )
    }

    private func providerMutationsRequireExplicitPreviewAndConfirmation() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "provider-action-ready")

        let store = SkillStore(service: fake.serviceClient())
        await store.loadAIProviderStatus()
        var draft = AIProviderSettingsDraft(status: store.aiProviderStatus)
        draft.endpoint = "https://provider-action.example.com/v1"
        draft.model = "model-action"
        draft.apiKey = "store-provider-secret"

        await store.previewSaveAIProviderSettings(draft: draft)
        try expectEqual(store.aiProviderPendingAction, .save, "Provider save should stop at a signed preview.")
        try expectEqual(
            countMethodCalls("llm.saveProviderProfile", in: fake.calls()),
            0,
            "Previewing provider save must perform zero writes."
        )
        store.cancelAIProviderActionPreview()
        try expectEqual(
            await store.confirmSaveAIProviderSettings(draft: draft),
            false,
            "A cancelled provider preview must not be confirmable."
        )

        await store.previewSaveAIProviderSettings(draft: draft)
        try expectEqual(
            await store.confirmSaveAIProviderSettings(draft: draft),
            true,
            "Explicit save confirmation should apply the signed provider action."
        )

        await store.previewAIProviderConnectionTest()
        try expectEqual(store.aiProviderPendingAction, .test, "Provider test should stop at a network preview.")
        try expectEqual(
            countMethodCalls("llm.testProviderConnection", in: fake.calls()),
            0,
            "Previewing provider test must send zero network requests."
        )
        let tested = await store.confirmAIProviderConnectionTest()
        try expectEqual(tested?.readback?.verified, true, "Confirmed provider test should publish verified activity read-back.")

        await store.previewDeleteAIProviderSettings()
        try expectEqual(store.aiProviderPendingAction, .delete, "Provider delete should stop at a destructive preview.")
        try expectEqual(
            countMethodCalls("llm.deleteProviderProfile", in: fake.calls()),
            0,
            "Previewing provider delete must perform zero deletes."
        )
        try expectEqual(
            await store.confirmDeleteAIProviderSettings(),
            true,
            "Explicit delete confirmation should apply the signed provider action."
        )
        try expectFalse(fake.calls().contains("store-provider-secret"), "Store-level fake evidence must not retain provider secrets.")
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
        try expectEqual(confirmation.previewToken, "action-preview:v1:hmac-sha256:rollback-preview", "Confirmation should capture the opaque preview token.")

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
            (label: "stale_action_reference", scenario: "rollback-stale-blocked", changesSelection: true, reselectSnapshotA: false, expectsCurrentError: false),
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
        try expectEqual(firstConfirmation.previewToken, "action-preview:v1:hmac-sha256:rollback-preview", "Previously captured confirmation should remain immutable.")
        try expectEqual(secondConfirmation.previewToken, "action-preview:v1:hmac-sha256:rollback-preview-2", "Preview replacement should publish only the new token.")

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
            confirmation: RollbackConfirmation(
                snapshotID: "snap-codex",
                action: ActionDescriptorWire(
                    id: "action:rollback_config:not-visible",
                    kind: "rollback_config",
                    intent: "rollback_config",
                    target: ActionTargetWire(
                        kind: "config",
                        id: "/tmp/codex.toml",
                        agent: "codex",
                        scope: "agent-global"
                    ),
                    projectID: nil,
                    impacts: ["agent_config"],
                    previewMethod: "snapshot.previewRollback",
                    applyMethod: "snapshot.rollback",
                    sourceRevision: "sha256:not-visible",
                    confirmationRequired: true,
                    network: "none",
                    readback: ["agent_config"],
                    evidenceRefs: ["snapshot:snap-codex"]
                ),
                previewToken: "action-preview:v1:hmac-sha256:not-visible"
            )
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

    private func toggleSelectedSkillExposesWritingStateAndRefreshesSelection() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()

        fake.setScenario("toggle-disabled")
        guard let skill = store.selectedSkill else {
            throw NativeModelTestFailure(description: "Toggle fixture should select a skill.")
        }
        await store.prepareSingleSkillTogglePreview(skill: skill, on: false)
        guard let previewID = store.batchTogglePreview?.id else {
            throw NativeModelTestFailure(description: "Single-skill toggle should produce a confirmation preview.")
        }
        let task = Task {
            await store.applyVisibleBatchTogglePreview(confirmingPreviewID: previewID)
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
        try expectEqual(
            store.lastMutationMessage,
            UIStrings.batchToggleApplied(action: BatchToggleAction.disable.title, count: 1),
            "A confirmed single-skill toggle should use the same lifecycle result as a confirmed batch."
        )
    }

    private func writeOperationsIgnoreReentryWhileBusy() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()

        fake.setScenario("toggle-disabled")
        guard let skill = store.selectedSkill else {
            throw NativeModelTestFailure(description: "Toggle fixture should select a skill.")
        }
        await store.prepareSingleSkillTogglePreview(skill: skill, on: false)
        guard let previewID = store.batchTogglePreview?.id else {
            throw NativeModelTestFailure(description: "Single-skill toggle should produce a confirmation preview.")
        }
        let task = Task {
            await store.applyVisibleBatchTogglePreview(confirmingPreviewID: previewID)
        }

        try await waitUntil("Toggle should expose writing state while the service request is in flight.") {
            store.isWriting
        }
        await store.prepareSingleSkillTogglePreview(skill: skill, on: true)
        await task.value

        try expectEqual(countMethodCalls("batch.previewSkillToggles", in: fake.calls()), 1, "Busy write should ignore reentrant preview attempts.")
        try expectEqual(countMethodCalls("batch.applySkillToggles", in: fake.calls()), 1, "Busy write should authorize exactly one apply.")
        try expectEqual(countMethodCalls("config.toggleSkill", in: fake.calls()), 0, "Single-skill UI toggles should use the typed batch lifecycle.")
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
        try await previewAndConfirmSelectedSkillToggle(store, on: false)

        try expectFalse(store.isWriting, "Codex toggle should reset writing state.")
        try expectNil(store.errorMessage, "Codex toggle should not set an error on success.")
        try expectEqual(store.selectedSkillID, "gamma", "Codex toggle refresh should keep the selected skill stable.")
        try expectEqual(store.selectedSkill?.enabled, false, "Codex toggle refresh should expose the updated enabled state.")
        try expectEqual(store.selectedSkillDetail?.enabled, false, "Codex toggle refresh should reload detail for the updated skill.")
        try expectEqual(
            store.lastMutationMessage,
            "\(UIStrings.batchToggleApplied(action: BatchToggleAction.disable.title, count: 1)) \(UIStrings.codexRestartRequired)",
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

        try await previewAndConfirmSelectedSkillToggle(store, on: false)

        try expectFalse(store.isWriting, "opencode toggle should finish writing state.")
        try expectNil(store.errorMessage, "opencode toggle should not surface a read-only error.")
        try expectContains(fake.calls(), "batch.previewSkillToggles", "opencode toggle should preview the typed action.")
        try expectContains(fake.calls(), "batch.applySkillToggles", "opencode toggle should apply the confirmed typed action.")
        try expectFalse(fake.calls().contains("config.toggleSkill"), "opencode UI toggles should not bypass the typed lifecycle.")
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

        try expectFalse(store.isWriting, "Tool-global toggle should not enter writing state.")
        try expectNil(store.errorMessage, "A disabled tool-global row must not start a mutation path.")
        try expectFalse(fake.calls().contains("batch.previewSkillToggles"), "A disabled tool-global row must not preview a toggle.")
        try expectFalse(fake.calls().contains("batch.applySkillToggles"), "A disabled tool-global row must not apply a toggle.")
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
        try expectContains(
            fake.calls(),
            #""instance_ids":["alpha","gamma","beta","pi-one"]"#,
            "Batch apply must re-submit both writable and skipped entries from the reviewed selection."
        )
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
        try expectEqual(store.appRoute, .overview, "Selecting a project should make Project Overview the coherent entry point.")
        try expectNil(store.selectedSidebarSelection, "Selecting a project should not manufacture a skill or session detail.")
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
        await store.previewClearProject()
        await store.confirmProjectContextPendingAction()

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

    private func reloadSurfacesUnsupportedLLMStatusMethod() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "old-service")

        let store = SkillStore(service: fake.serviceClient())
        await store.reload()

        try expectContains(store.errorMessage, "unknown_method", "Reload should fail closed when a required LLM status method is unavailable.")
        try expectFalse(store.isLoading, "A failed reload should still reset its loading state.")
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
            countMethodCalls("snapshot.listAgentConfigPage", in: fake.calls()) == 1
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

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.taskCockpitText = "Prepare local release audit work."
        await store.reload()
        try await waitUntil("Reload supplemental reads should settle before the Task Cockpit contract check.") {
            countMethodCalls("snapshot.listAgentConfigPage", in: fake.calls()) == 1
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
        try expectContains(
            calls,
            "\"task_text\":\"[REDACTED]\"",
            "Task cockpit request diagnostics must redact task text."
        )
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

        let store = SkillStore(service: fake.serviceClient())
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
        try expectContains(previewCall, "\"task_text\":\"[REDACTED]\"", "Captured fake-service evidence must redact the original task text.")
        try expectContains(previewCall, "\"agents\":[\"claude-code\"]", "Global Preflight should use the current sidebar agent as the default scope.")
        try expectContains(previewCall, "\"instance_ids\":[\"alpha\",\"beta\"]", "Global Preflight should include effective skills for the selected agent scope.")
        try expectFalse(previewCall?.contains("\"selected_skill_id\"") ?? false, "Global Preflight should not inherit a retained selected skill id.")
    }

    private func taskCockpitHistoryStaysInCurrentSessionOnly() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let task = "阿里云 ECS 磁盘负载情况分析"
        let store = SkillStore(service: fake.serviceClient())
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

        let store = SkillStore(service: fake.serviceClient())
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

        let firstStore = SkillStore(service: fake.serviceClient())
        firstStore.taskCockpitText = "Keep this Preflight in memory only."
        await firstStore.reload()
        try await previewAndConfirmTaskCockpit(firstStore)
        try expectEqual(firstStore.taskCockpitHistory.count, 1, "The first store should retain its completed session result.")
        try expectEqual(firstStore.taskCockpitHistory.first?.displayTask, "Keep this Preflight in memory only.", "The current session should retain the complete task text.")
        try expectEqual(firstStore.taskCockpitHistory.first?.agentIDs, ["claude-code"], "The current session should retain the complete agent scope.")
        try expectEqual(firstStore.taskCockpitHistory.first?.result.summary.recommendedSkillName, "Beta", "The current session should retain the complete provider result.")

        let secondStore = SkillStore(service: fake.serviceClient())
        try expectEqual(
            secondStore.taskCockpitHistory.count,
            0,
            "A new store must start with fresh in-memory Task Preflight history."
        )
    }

    private func successfulTaskCockpitKeepsSessionHistoryInMemory() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.taskCockpitText = "Do not persist this provider-confirmed result."
        await store.reload()
        try await previewAndConfirmTaskCockpit(store)

        try expectEqual(store.taskCockpitHistory.count, 1, "The successful result should remain available in memory.")
        try expectEqual(
            store.taskCockpitHistory.first?.displayTask,
            "Do not persist this provider-confirmed result.",
            "The in-memory session record should preserve its display task."
        )
    }

    private func clearTaskCockpitHistoryClearsMemory() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient())
        store.taskCockpitText = "Clear this in-memory result."
        await store.reload()
        try await previewAndConfirmTaskCockpit(store)

        store.clearTaskCockpitHistory()

        try expectEqual(store.taskCockpitHistory.count, 0, "Clear should remove every in-memory Task Preflight record.")
        try expectNil(store.selectedTaskCockpitHistoryID, "Clear should reset the selected history record.")
        try expectNil(store.taskCockpitHistoryCleanupMessage, "A normal in-memory clear should not manufacture a cleanup warning.")
    }

    private func taskCockpitPreservesExactUserInputInServiceContract() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let exactTask = "  修复 Task Cockpit 🧪\n第二行\t带制表  "
        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        store.taskCockpitText = exactTask
        await store.reload()
        try await previewAndConfirmTaskCockpit(store)

        try expectEqual(store.selectedTaskCockpitInput, exactTask, "Non-blank cockpit input should preserve the exact user text.")

        let calls = fake.calls()
        try expectContains(calls, "llm.previewPrompt", "Exact-input test should prepare the provider-backed Task Preflight prompt.")
        try expectContains(calls, "llm.confirmPromptAndSend", "Exact-input cockpit flow should send through provider confirmation.")
        try expectContains(calls, "\"task_text\":\"[REDACTED]\"", "Captured fake-service evidence must redact exact task input.")
        try expectFalse(calls.contains(exactTask), "Captured fake-service evidence must not retain the raw task input.")
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

    private func taskCockpitSurfacesMethodUnavailable() async throws {
        let runner = TaskCockpitFallbackServiceRunner()

        let store = SkillStore(
            service: ServiceClient(
                processRunner: runner,
                serviceURL: URL(fileURLWithPath: "/tmp/task-cockpit-fallback-service")
            )
        )
        store.selectedSkillID = "beta"
        store.taskCockpitText = "Route a local audit release note task."
        await store.reload()
        await store.buildTaskCockpit()

        try expectEqual(store.taskCockpitResult?.isUnavailable, true, "Task cockpit should expose an unavailable result when the required method is absent.")
        try expectContains(store.taskCockpitResult?.fallbackReason, "unknown_method", "A missing required method must remain visible instead of silently falling back.")
        try expectFalse(store.isBuildingTaskCockpit, "Unavailable task cockpit should reset loading state.")
        let calls = await runner.calls()
        try expectContains(calls, "llm.previewPrompt", "The failed request should still prove the intended provider preview method was attempted.")
        try expectContains(calls, "\"task_text\":\"Route a local audit release note task.\"", "The failed request should reuse existing routing task text when cockpit input is blank.")
        try expectFalse(calls.contains("llm.confirmPromptAndSend"), "Unavailable cockpit flow must not send to provider.")
        try expectFalse(calls.contains("config.toggleSkill"), "Unavailable cockpit flow must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Unavailable cockpit flow must not call execution paths.")
    }

    private func taskCockpitTimeoutShowsRecoveryAndIgnoresStaleResponse() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let store = SkillStore(service: fake.serviceClient(), taskCockpitTimeoutSeconds: 0.5)
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

        let store = SkillStore(service: fake.serviceClient(), taskCockpitTimeoutSeconds: 1)
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
        try expectContains(calls, "\"action_id\":\"prompt-preview-beta\"", "Confirm should bind the previewed action reference.")
        try expectContains(calls, "\"preview_token\":\"[REDACTED]\"", "Captured confirmation evidence must redact the opaque action token.")
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

    func waitUntil(
        _ label: String,
        timeout: TimeInterval = 5,
        predicate: @escaping @MainActor () async -> Bool
    ) async throws {
        let deadline = ProcessInfo.processInfo.systemUptime + timeout
        while !(await predicate()) {
            if ProcessInfo.processInfo.systemUptime >= deadline {
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

    private func previewAndConfirmSelectedSkillToggle(
        _ store: SkillStore,
        on: Bool
    ) async throws {
        guard let skill = store.selectedSkill else {
            throw NativeModelTestFailure(description: "Single-skill toggle requires a selected skill.")
        }
        await store.prepareSingleSkillTogglePreview(skill: skill, on: on)
        guard let previewID = store.batchTogglePreview?.id else {
            throw NativeModelTestFailure(description: "Single-skill toggle must produce a preview before apply.")
        }
        await store.applyVisibleBatchTogglePreview(confirmingPreviewID: previewID)
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

private final class ExplicitProviderMutationControlServiceRunner: ServiceProcessRunning {
    private let state: ExplicitProviderMutationControlServiceState

    init(
        suspendMutations: Bool = true,
        providerMutationUnavailable: Bool = false
    ) {
        state = ExplicitProviderMutationControlServiceState(
            suspendMutations: suspendMutations,
            providerMutationUnavailable: providerMutationUnavailable
        )
    }

    func serviceClient() -> ServiceClient {
        ServiceClient(
            processRunner: self,
            serviceURL: URL(fileURLWithPath: "/tmp/explicit-provider-mutation-control-service")
        )
    }

    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) async throws -> Data {
        _ = executableURL
        _ = timeoutNanoseconds
        return try await state.response(for: input)
    }

    var startedProviderSaveCount: Int {
        get async { await state.startedProviderSaveCount }
    }

    var providerSaveEndpoints: [String] {
        get async { await state.providerSaveEndpoints }
    }

    func releaseNextProviderSave() async {
        await state.releaseNextProviderSave()
    }

    func setExternalProvider(endpoint: String, model: String) async {
        await state.setExternalProvider(endpoint: endpoint, model: model)
    }
}

private actor ExplicitProviderMutationControlServiceState {
    private let suspendMutations: Bool
    private let providerMutationUnavailable: Bool
    private var providerEndpoint = "https://provider-x.example.com/v1"
    private var providerModel = "model-x"
    private var providerSaveContinuations: [CheckedContinuation<Void, Never>] = []
    private(set) var providerSaveEndpoints: [String] = []

    init(
        suspendMutations: Bool,
        providerMutationUnavailable: Bool
    ) {
        self.suspendMutations = suspendMutations
        self.providerMutationUnavailable = providerMutationUnavailable
    }

    var startedProviderSaveCount: Int { providerSaveEndpoints.count }

    func response(for input: Data) async throws -> Data {
        guard let request = try JSONSerialization.jsonObject(with: input) as? [String: Any],
              let method = request["method"] as? String else {
            return Self.error(code: "invalid_request", message: "missing method")
        }
        let params = request["params"] as? [String: Any] ?? [:]

        switch method {
        case "llm.saveProviderProfile":
            if providerMutationUnavailable {
                return Self.error(code: "unknown_method", message: "unknown method: llm.saveProviderProfile")
            }
            let endpoint = params["base_url"] as? String ?? ""
            let model = params["model"] as? String ?? ""
            providerSaveEndpoints.append(endpoint)
            if suspendMutations {
                await withCheckedContinuation { continuation in
                    providerSaveContinuations.append(continuation)
                }
            }
            providerEndpoint = endpoint
            providerModel = model
            return Self.ok(["profile": NSNull()])

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
        case "skill.listEventsPage", "snapshot.listAgentConfigPage":
            return Self.ok([
                "records": [],
                "source_revision": "sha256:empty-history",
                "returned_count": 0,
                "total_count": 0,
                "has_more": false,
                "next_cursor": NSNull(),
                "source_completeness": "enumerable",
                "incomplete_reason": NSNull(),
            ])
        default:
            return Self.error(code: "unknown_method", message: "unknown method: \(method)")
        }
    }

    func releaseNextProviderSave() {
        guard !providerSaveContinuations.isEmpty else { return }
        providerSaveContinuations.removeFirst().resume()
    }

    func setExternalProvider(endpoint: String, model: String) {
        providerEndpoint = endpoint
        providerModel = model
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
                "llm.listProviderProfiles",
                "llm.saveProviderProfile",
                "skill.listEventsPage",
                "snapshot.listAgentConfigPage",
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

final class LocalHistoryPageRunner: ServiceProcessRunning, @unchecked Sendable {
    private let lock = NSLock()
    private let failFirstMethods: Set<String>
    private let failSecondMethods: Set<String>
    private let sourceChangedSecondMethods: Set<String>
    private let delayedThirdMethods: Set<String>
    private let includesSelectedSkillFixture: Bool
    private var failedMethods = Set<String>()
    private var recordedCursors: [String: [String?]] = [:]
    private var callCounts: [String: Int] = [:]
    private var releaseContinuations: [String: [CheckedContinuation<Void, Never>]] = [:]
    private var releasedMethods = Set<String>()

    init(
        failFirstMethods: Set<String> = [],
        failSecondMethods: Set<String> = [],
        sourceChangedSecondMethods: Set<String> = [],
        delayedThirdMethods: Set<String> = [],
        includesSelectedSkillFixture: Bool = false
    ) {
        self.failFirstMethods = failFirstMethods
        self.failSecondMethods = failSecondMethods
        self.sourceChangedSecondMethods = sourceChangedSecondMethods
        self.delayedThirdMethods = delayedThirdMethods
        self.includesSelectedSkillFixture = includesSelectedSkillFixture
    }

    func serviceClient() -> ServiceClient {
        ServiceClient(
            processRunner: self,
            serviceURL: URL(fileURLWithPath: "/tmp/local-history-page-service")
        )
    }

    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) async throws -> Data {
        guard let request = try JSONSerialization.jsonObject(with: input) as? [String: Any],
              let method = request["method"] as? String else {
            return Self.error(code: "invalid_request", message: "invalid test request")
        }
        if includesSelectedSkillFixture {
            switch method {
            case "app.stateSnapshot":
                return Self.selectedSkillStateSnapshot()
            case "catalog.getSkill":
                return Self.selectedSkillDetail()
            case "llm.status":
                return Self.response(result: [
                    "enabled": false,
                    "disabled_reason": "disabled",
                    "supported_actions": [],
                ])
            case "project.getContext":
                return Self.response(result: [
                    "active": NSNull(),
                    "recent": [],
                    "revision": "sha256:project-context-empty",
                ])
            case "rules.listTuning":
                return Self.response(result: ["records": []])
            default:
                break
            }
        }
        let params = request["params"] as? [String: Any] ?? [:]
        let cursor = params["cursor"] as? String
        let offset = cursor.flatMap { Int($0.split(separator: "-").last ?? "") } ?? 0
        let behavior = record(method: method, cursor: cursor, offset: offset)
        if let errorCode = behavior.errorCode {
            return Self.error(code: errorCode, message: "synthetic second-page failure")
        }
        if behavior.delay {
            await waitForRelease(method: method)
        }
        switch method {
        case "snapshot.listAgentConfigPage":
            return Self.configPage(offset: offset)
        case "skill.listEventsPage":
            return Self.eventPage(offset: offset)
        default:
            return Self.error(code: "unknown_method", message: "unknown method: \(method)")
        }
    }

    func cursors(for method: String) -> [String?] {
        lock.lock()
        let values = recordedCursors[method] ?? []
        lock.unlock()
        return values
    }

    func syncCallCount(for method: String) -> Int {
        lock.lock()
        let count = callCounts[method] ?? 0
        lock.unlock()
        return count
    }

    func release(method: String) {
        lock.lock()
        let continuations = releaseContinuations.removeValue(forKey: method) ?? []
        if continuations.isEmpty {
            releasedMethods.insert(method)
        }
        lock.unlock()
        continuations.forEach { $0.resume() }
    }

    private func record(method: String, cursor: String?, offset: Int) -> (errorCode: String?, delay: Bool) {
        lock.lock()
        defer { lock.unlock() }
        recordedCursors[method, default: []].append(cursor)
        callCounts[method, default: 0] += 1
        let initialFailure = offset == 0
            && failFirstMethods.contains(method)
            && failedMethods.insert("initial:\(method)").inserted
        let pageFailure = offset == 100
            && failSecondMethods.contains(method)
            && failedMethods.insert(method).inserted
        let sourceChanged = offset == 100 && sourceChangedSecondMethods.contains(method)
        let delay = offset == 200 && delayedThirdMethods.contains(method)
        return (
            sourceChanged ? "source_changed" : ((initialFailure || pageFailure) ? "page_failed" : nil),
            delay
        )
    }

    private func waitForRelease(method: String) async {
        await withCheckedContinuation { continuation in
            lock.lock()
            let wasReleased = releasedMethods.remove(method) != nil
            if !wasReleased {
                releaseContinuations[method, default: []].append(continuation)
            }
            lock.unlock()
            if wasReleased {
                continuation.resume()
            }
        }
    }

    private static func configPage(offset: Int) -> Data {
        let total = 205
        let end = min(offset + 100, total)
        let records: [[String: Any]] = (offset..<end).map { index in
            [
                "id": "config-\(index)",
                "agent": "claude-code",
                "scope": "agent-global",
                "target": "/tmp/home/.claude/settings.json",
                "content": "{}\n",
                "reason": "pre-toggle",
                "created_at": total - index,
            ]
        }
        return page(
            records: records,
            sourceRevision: "sha256:config-revision",
            total: total,
            nextCursor: end < total ? "config-\(end)" : nil
        )
    }

    private static func eventPage(offset: Int) -> Data {
        let total = 201
        let end = min(offset + 100, total)
        let records: [[String: Any]] = (offset..<end).map { index in
            [
                "id": total - index,
                "instance_id": "skill-1",
                "kind": "toggle",
                "payload": ["index": index],
                "occurred_at": total - index,
            ]
        }
        return page(
            records: records,
            sourceRevision: "sha256:event-revision",
            total: total,
            nextCursor: end < total ? "event-\(end)" : nil
        )
    }

    private static func page(
        records: [[String: Any]],
        sourceRevision: String,
        total: Int,
        nextCursor: String?
    ) -> Data {
        var result: [String: Any] = [
            "records": records,
            "source_revision": sourceRevision,
            "returned_count": records.count,
            "total_count": total,
            "has_more": nextCursor != nil,
            "source_completeness": "enumerable",
        ]
        if let nextCursor {
            result["next_cursor"] = nextCursor
        }
        return response(result: result)
    }

    private static func error(code: String, message: String) -> Data {
        let object: [String: Any] = [
            "id": "test",
            "ok": false,
            "error": ["code": code, "message": message],
        ]
        return (try? JSONSerialization.data(withJSONObject: object, options: [.sortedKeys])) ?? Data()
    }

    private static func response(result: [String: Any]) -> Data {
        let object: [String: Any] = ["id": "test", "ok": true, "result": result]
        return (try? JSONSerialization.data(withJSONObject: object, options: [.sortedKeys])) ?? Data()
    }

    private static func selectedSkillStateSnapshot() -> Data {
        response(result: [
            "status": [
                "protocol_version": 2,
                "version": "test",
                "app_data_dir": "/tmp/app-data",
                "catalog_path": "/tmp/app-data/catalog.sqlite",
                "user_home": "/tmp/home",
                "supported_methods": [
                    "app.stateSnapshot",
                    "catalog.getSkill",
                    "llm.status",
                    "project.getContext",
                    "rules.listTuning",
                    "skill.listEventsPage",
                ],
            ],
            "skills": [[
                "id": "skill-1",
                "agent": "claude-code",
                "scope": "agent-global",
                "path": "/tmp/skill-1/SKILL.md",
                "display_path": "/tmp/skill-1/SKILL.md",
                "definition_id": "fixture:skill-1",
                "name": "Skill One",
                "state": "loaded",
                "enabled": true,
            ]],
            "findings": [],
            "conflicts": [],
            "snapshots": [],
        ])
    }

    private static func selectedSkillDetail() -> Data {
        response(result: [
            "id": "skill-1",
            "agent": "claude-code",
            "scope": "agent-global",
            "path": "/tmp/skill-1/SKILL.md",
            "display_path": "/tmp/skill-1/SKILL.md",
            "definition_id": "fixture:skill-1",
            "name": "Skill One",
            "description": "Fixture",
            "state": "loaded",
            "enabled": true,
            "frontmatter_raw": "",
            "body": "",
            "permissions": [:],
            "fingerprint": "fixture",
        ])
    }
}

private final class EventHistoryABARunner: ServiceProcessRunning, @unchecked Sendable {
    private let lock = NSLock()
    private var calls = 0
    private var oldContinuation: CheckedContinuation<Void, Never>?

    var syncCallCount: Int {
        lock.lock()
        defer { lock.unlock() }
        return calls
    }

    func serviceClient() -> ServiceClient {
        ServiceClient(processRunner: self, serviceURL: URL(fileURLWithPath: "/tmp/event-history-aba-service"))
    }

    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) async throws -> Data {
        let call = nextCall()
        if call == 1 {
            await withCheckedContinuation { continuation in
                lock.lock()
                oldContinuation = continuation
                lock.unlock()
            }
        }
        return Self.eventPage(id: call == 1 ? 1 : 2)
    }

    private func nextCall() -> Int {
        lock.lock()
        defer { lock.unlock() }
        calls += 1
        return calls
    }

    func releaseOldResponse() {
        lock.lock()
        let continuation = oldContinuation
        oldContinuation = nil
        lock.unlock()
        continuation?.resume()
    }

    private static func eventPage(id: Int64) -> Data {
        let object: [String: Any] = [
            "id": "test",
            "ok": true,
            "result": [
                "records": [[
                    "id": id,
                    "instance_id": "skill-1",
                    "kind": "toggle",
                    "payload": [:],
                    "occurred_at": id,
                ]],
                "source_revision": "sha256:generation-\(id)",
                "returned_count": 1,
                "total_count": 1,
                "has_more": false,
                "source_completeness": "enumerable",
            ],
        ]
        return (try? JSONSerialization.data(withJSONObject: object, options: [.sortedKeys])) ?? Data()
    }
}

enum CatalogRefreshScanFixture {
    case partial
    case complete, completeWithSafeDiagnostics, completeWithDeletedHistory
    case budget
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

final class CatalogRefreshServiceRunner: ServiceProcessRunning {
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
        case "skill.listEventsPage", "snapshot.listAgentConfigPage":
            return Self.ok(Self.emptyHistoryPage)
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
        let raw = switch fixture {
        case .partial:
            partialScanResult
        case .complete:
            completeScanResult
        case .completeWithSafeDiagnostics: completeWithSafeDiagnosticsScanResult
        case .completeWithDeletedHistory: completeWithDeletedHistoryScanResult
        case .budget:
            budgetScanResult
        case .legacySummary:
            legacySummaryScanResult
        case .legacyWithoutActivity:
            legacyActivitylessScanResult
        }
        return raw.replacingOccurrences(
            of: "{\"scanned_count\":",
            with: "{\"accepted_context_revision\":\"sha256:project-context\",\"catalog_scan_revision\":\"sha256:catalog-scan\",\"readback\":{\"accepted_context_revision\":\"sha256:project-context\",\"catalog_scan_revision\":\"sha256:catalog-scan\",\"verified\":true},\"scanned_count\":"
        )
    }

    private static let supportedMethods = """
    ["app.stateSnapshot","catalog.scanAll","catalog.getSkill","skill.listEventsPage","snapshot.listAgentConfigPage","project.getContext","llm.status","llm.listProviderProfiles","llm.listPromptRuns","rules.listTuning"]
    """

    private static let emptyHistoryPage = """
    {"records":[],"source_revision":"sha256:empty-history","returned_count":0,"total_count":0,"has_more":false,"next_cursor":null,"source_completeness":"enumerable","incomplete_reason":null}
    """

    private static let status = """
    {"protocol_version":1,"version":"test","app_data_dir":"/tmp/skills-copilot","catalog_path":"/tmp/skills-copilot/catalog.sqlite","user_home":"/tmp/home","supported_methods":\(supportedMethods)}
    """

    private static let skills = """
    [{"id":"alpha","agent":"claude-code","scope":"agent-global","path":"/tmp/global/alpha/SKILL.md","display_path":"/tmp/global/alpha/SKILL.md","definition_id":"def.alpha","name":"Alpha","state":"loaded","enabled":true},{"id":"beta","agent":"claude-code","scope":"agent-project","path":"/tmp/project/beta/SKILL.md","display_path":"/tmp/project/beta/SKILL.md","definition_id":"def.beta","name":"Beta","state":"loaded","enabled":true},{"id":"gamma","agent":"codex","scope":"agent-global","path":"/tmp/codex/skills/gamma/SKILL.md","display_path":"~/.codex/skills/gamma/SKILL.md","definition_id":"codex:gamma","name":"Gamma","state":"loaded","enabled":true}]
    """

    private static let findings = #"[{"id":"baseline-warning","instance_id":"alpha","definition_id":"def.alpha","rule_id":"permissions.network-declared","severity":"warning","message":"baseline warning","suggestion":"declare network","created_at":1},{"id":"visible-finding","instance_id":"beta","definition_id":"def.beta","rule_id":"body.too-long","severity":"warning","message":"visible issue","suggestion":"shorten body","created_at":1}]"#

    private static let stateSnapshot = "{\"status\":\(status),\"skills\":\(skills),\"findings\":\(findings),\"conflicts\":[],\"snapshots\":[]}"

    private static let partialScanResult = """
    {"scanned_count":3,"skills":\(skills),"activity":{"operation":"catalog.scanAll","status":"completed-partial","started_at":1,"finished_at":2,"scanned_count":3,"skill_count":3,"finding_count":0,"conflict_count":0,"snapshot_count":0,"roots":["$HOME/.claude/skills","$HOME/.agents/skills","<adapter-root>/missing-opencode"],"log_entries":[{"level":"warning","message":"Claude Code discovered 2 skill(s); catalog now has 2 skill(s), 0 broken, across 0 complete root(s), 1 partial root(s), and 0 skipped root(s); first scan issue entry_unreadable at <adapter-root>/dangling-link: A directory entry could not be inspected or resolved."},{"level":"info","message":"Codex discovered 1 skill(s); catalog now has 1 skill(s), 0 broken, across 1 complete root(s), 0 partial root(s), and 0 skipped root(s)."},{"level":"warning","message":"opencode discovered 0 skill(s); catalog now has 0 skill(s), 0 broken, across 0 complete root(s), 0 partial root(s), and 1 skipped root(s); root-error skipped-root path(s): <adapter-root>/missing-opencode."}],"recovery_actions":["Review partial-root diagnostics; unseen rows under partial roots were preserved."],"agent_summaries":[{"agent":"claude-code","display_label":"Claude Code","status":"completed-partial","scanned_count":2,"catalog_count":2,"broken_count":0,"roots_considered":["$HOME/.claude/skills"],"roots_scanned":[],"roots_partial":["<adapter-root>"],"roots_skipped":[],"scan_issues":[{"kind":"entry_unreadable","path":"<adapter-root>/dangling-link","detail":"A directory entry could not be inspected or resolved."}],"recovery_actions":["Review partial scan diagnostics."]},{"agent":"codex","display_label":"Codex","status":"completed","scanned_count":1,"catalog_count":1,"broken_count":0,"roots_considered":["$HOME/.agents/skills"],"roots_scanned":["$HOME/.agents/skills"],"roots_partial":[],"roots_skipped":[],"scan_issues":[],"recovery_actions":[]},{"agent":"opencode","display_label":"opencode","status":"completed-with-skipped-roots","scanned_count":0,"catalog_count":0,"broken_count":0,"roots_considered":["<adapter-root>/missing-opencode"],"roots_scanned":[],"roots_partial":[],"roots_skipped":["<adapter-root>/missing-opencode"],"scan_issues":[{"kind":"root_unavailable","path":"<adapter-root>/missing-opencode","detail":"A declared scan root was unavailable or not a directory."}],"recovery_actions":["Review opencode skipped-root diagnostics, then retry Scan."]}]}}
    """.replacingOccurrences(
        of: "\"scan_issues\":[{\"kind\":\"entry_unreadable\"", with: "\"scan_issues\":[{\"kind\":\"root_unavailable\",\"path\":\"<adapter-root>/missing-optional\",\"detail\":\"A declared scan root was unavailable or not a directory.\"},{\"kind\":\"entry_unreadable\""
    ).replacingOccurrences(of: "\"finding_count\":0", with: "\"finding_count\":2")

    private static let completeScanResult = """
    {"scanned_count":3,"skills":\(skills),"activity":{"operation":"catalog.scanAll","status":"completed","started_at":3,"finished_at":4,"scanned_count":3,"skill_count":3,"finding_count":0,"conflict_count":0,"snapshot_count":0,"roots":["$HOME/.claude/skills"],"log_entries":[],"recovery_actions":[],"agent_summaries":[{"agent":"claude-code","display_label":"Claude Code","status":"completed","scanned_count":3,"catalog_count":3,"broken_count":0,"roots_considered":["$HOME/.claude/skills"],"roots_scanned":["$HOME/.claude/skills"],"roots_partial":[],"roots_skipped":[],"scan_issues":[],"recovery_actions":[]}]}}
    """

    private static let completeWithSafeDiagnosticsScanResult = """
    {"scanned_count":3,"skills":\(skills),"activity":{"operation":"catalog.scanAll","status":"completed","started_at":3,"finished_at":4,"scanned_count":3,"skill_count":3,"finding_count":0,"conflict_count":0,"snapshot_count":0,"roots":["$HOME/.agents/skills","$HOME/.hermes/skills"],"log_entries":[],"recovery_actions":[],"agent_summaries":[{"agent":"opencode","display_label":"opencode","status":"completed","scanned_count":3,"catalog_count":3,"broken_count":0,"roots_considered":["$HOME/.agents/skills"],"roots_scanned":["$HOME/.agents/skills"],"roots_partial":[],"roots_skipped":[],"scan_issues":[{"kind":"root_outside_allowlist","path":"$HOME/.agents/skills/external","detail":"A resolved path was outside the explicit same-scope adapter roots."},{"kind":"dangling_symlink","path":"$HOME/.config/opencode/skills/removed","detail":"An Agent skill link points to an unavailable source; the link was skipped and the rest of the root was reconciled."}],"recovery_actions":[]},{"agent":"hermes","display_label":"Hermes","status":"completed-no-roots-scanned","scanned_count":0,"catalog_count":0,"broken_count":0,"roots_considered":["$HOME/.hermes/skills"],"roots_scanned":[],"roots_partial":[],"roots_skipped":[],"scan_issues":[],"recovery_actions":[]}]}}
    """

    private static let completeWithDeletedHistoryScanResult = """
    {"scanned_count":3,"skills":[{"id":"alpha","agent":"claude-code","scope":"agent-global","path":"/tmp/global/alpha/SKILL.md","display_path":"/tmp/global/alpha/SKILL.md","definition_id":"def.alpha","name":"Alpha","state":"loaded","enabled":true},{"id":"beta","agent":"claude-code","scope":"agent-project","path":"/tmp/project/beta/SKILL.md","display_path":"/tmp/project/beta/SKILL.md","definition_id":"def.beta","name":"Beta","state":"loaded","enabled":true},{"id":"gamma","agent":"codex","scope":"agent-global","path":"/tmp/codex/skills/gamma/SKILL.md","display_path":"~/.codex/skills/gamma/SKILL.md","definition_id":"codex:gamma","name":"Gamma","state":"loaded","enabled":true},{"id":"legacy-runtime","agent":"codex","scope":"agent-global","path":"/tmp/codex/.agent-copilot-runtime/legacy/SKILL.md","display_path":"Codex Runtime/legacy","definition_id":"legacy","name":"Legacy","state":"missing","enabled":false}],"activity":{"operation":"catalog.scanAll","status":"completed","started_at":3,"finished_at":4,"scanned_count":3,"skill_count":4,"finding_count":0,"conflict_count":0,"snapshot_count":0,"roots":["$HOME/.claude/skills","$HOME/.codex/skills"],"log_entries":[],"recovery_actions":[],"agent_summaries":[{"agent":"claude-code","display_label":"Claude Code","status":"completed","scanned_count":2,"catalog_count":2,"broken_count":0,"roots_considered":["$HOME/.claude/skills"],"roots_scanned":["$HOME/.claude/skills"],"roots_partial":[],"roots_skipped":[],"scan_issues":[],"recovery_actions":[]},{"agent":"codex","display_label":"Codex","status":"completed","scanned_count":1,"catalog_count":2,"broken_count":0,"roots_considered":["$HOME/.codex/skills"],"roots_scanned":["$HOME/.codex/skills"],"roots_partial":[],"roots_skipped":[],"scan_issues":[],"recovery_actions":[]}]}}
    """

    private static let budgetScanResult = """
    {"scanned_count":2,"skills":\(skills),"activity":{"operation":"catalog.scanAll","status":"completed-partial","started_at":7,"finished_at":8,"scanned_count":2,"skill_count":3,"finding_count":0,"conflict_count":0,"snapshot_count":0,"roots":["$HOME/.claude/skills"],"log_entries":[],"recovery_actions":["Retry Scan after reducing the selected roots."],"agent_summaries":[{"agent":"claude-code","display_label":"Claude Code","status":"completed-partial","scanned_count":2,"catalog_count":3,"broken_count":0,"roots_considered":["$HOME/.claude/skills"],"roots_scanned":[],"roots_partial":["$HOME/.claude/skills"],"roots_skipped":[],"scan_issues":[{"kind":"budget_exceeded","path":"$HOME/.claude/skills","detail":"The scan stopped after reaching a configured work budget."}],"recovery_actions":["Retry Scan after reducing the selected roots."]}]}}
    """

    private static let legacySummaryScanResult = """
    {"scanned_count":3,"skills":\(skills),"activity":{"operation":"catalog.scanAll","status":"completed","started_at":5,"finished_at":6,"scanned_count":3,"skill_count":3,"finding_count":0,"conflict_count":0,"snapshot_count":0,"roots":["$HOME/.claude/skills"],"log_entries":[],"recovery_actions":[],"agent_summaries":[{"agent":"claude-code","display_label":"Claude Code","status":"completed","scanned_count":3,"catalog_count":3,"broken_count":0,"roots_considered":["$HOME/.claude/skills"],"roots_scanned":["$HOME/.claude/skills"],"roots_skipped":[],"recovery_actions":[]}]}}
    """

    private static let legacyActivitylessScanResult = """
    {"scanned_count":3,"skills":\(skills)}
    """

    private static let projectContext = """
    {"revision":"sha256:project-context","active":null,"recent":[]}
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
