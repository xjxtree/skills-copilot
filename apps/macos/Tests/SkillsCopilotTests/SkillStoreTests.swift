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
        try await runCase("taskCockpitHistoryPersistsLocally") {
            try await taskCockpitHistoryPersistsLocally()
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

        store.localSessionSearchText = "develop"
        await store.previewLocalSessions()

        try expectEqual(store.filteredLocalSessionRows.map(\.id), ["session-develop"], "Session search should narrow visible rows.")
        try expectEqual(store.selectedLocalSessionID, "session-develop", "Search should move selection to the visible session.")
        try expectEqual(store.selectedSidebarSelection, .session("session-develop"), "Detail selection should follow the searched session.")
        try expectEqual(store.selectedLocalSession?.title, "Switch to develop branch", "Detail model should expose the searched session.")
        try expectContains(fake.calls(), "\"search\":\"develop\"", "Session search should be sent to the service.")

        store.localSessionSearchText = "missing"
        await store.previewLocalSessions()

        try expectEqual(store.filteredLocalSessionRows.count, 0, "No-match search should show an empty session list.")
        try expectNil(store.selectedLocalSessionID, "No-match search should clear stale session selection.")
        try expectNil(store.selectedSidebarSelection, "No-match search should clear stale session detail.")

        store.localSessionSearchText = ""
        await store.previewLocalSessions()

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

    private func localSessionScopeChangeRequestsServerPage() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "sessions-mixed")

        let store = SkillStore(service: fake.serviceClient())
        store.sidebarContentMode = .sessions
        await store.previewLocalSessions()

        try expectEqual(store.localSessionPreviewResult.sessionRows.map(\.id), ["session-alpha", "session-develop"], "Default project scope should request project rows from the service.")
        try expectEqual(store.filteredLocalSessionRows.map(\.id), ["session-alpha", "session-develop"], "Project scope should show the server-filtered project rows.")
        try expectEqual(store.scopedLocalSessionUserMessageCount, 2, "Project scope metrics should be derived from project rows only.")
        try expectEqual(store.scopedLocalSessionTotalMessageCount, 4, "Project scope message totals should be derived from project rows only.")
        let callsBeforeScopeChange = countMethodCalls("session.previewLocalSessions", in: fake.calls())
        try expectContains(fake.calls(), "\"scope\":\"project\"", "Session preview should send the selected project scope.")

        store.localSessionScopeFilter = .all
        await store.previewLocalSessions()

        guard let globalSession = store.localSessionPreviewResult.sessionRows.first(where: { $0.id == "session-global" }) else {
            throw NativeModelTestFailure(description: "Fake session fixture should include the global session after all-scope refresh.")
        }

        try expectEqual(store.filteredLocalSessionRows.map(\.id), ["session-alpha", "session-develop", "session-global"], "All scope should request and reveal the global session.")
        try expectEqual(store.scopedLocalSessionUserMessageCount, 4, "All-scope metrics should include global rows.")
        try expectEqual(store.scopedLocalSessionSkillCallCount, 1, "All-scope skill metrics should include global rows.")
        try expectEqual(
            countMethodCalls("session.previewLocalSessions", in: fake.calls()),
            callsBeforeScopeChange + 1,
            "Scope changes should fetch a service page for the new scope."
        )

        store.selectLocalSession(globalSession)
        store.localSessionScopeFilter = .project
        await store.previewLocalSessions()

        try expectEqual(store.filteredLocalSessionRows.map(\.id), ["session-alpha", "session-develop"], "Returning to project scope should hide global rows from the cached list.")
        try expectEqual(store.selectedLocalSessionID, "session-alpha", "Scope filtering should normalize a hidden global selection to the first visible project session.")
        try expectEqual(store.selectedSidebarSelection, .session("session-alpha"), "Detail selection should follow the locally visible session after scope filtering.")
        try expectEqual(
            countMethodCalls("session.previewLocalSessions", in: fake.calls()),
            callsBeforeScopeChange + 2,
            "Returning to project scope should request a fresh service page."
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
        try expectContains(fake.calls(), "\"scope\":\"project\"", "Session preview should send the selected project scope.")
        try expectEqual(
            store.localSessionPreviewResult.sessionRows.map(\.id),
            ["session-project-from-all"],
            "Project-scope service page should include the project session."
        )
        try expectEqual(
            store.filteredLocalSessionRows.map(\.id),
            ["session-project-from-all"],
            "Project scope should show the service-filtered project rows."
        )
        try expectEqual(store.scopedLocalSessionToolCallCount, 24, "Project scope metrics should include the project-root session.")

        store.localSessionScopeFilter = .all
        await store.previewLocalSessions()

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
        await store.previewLocalSessions()

        try expectEqual(
            store.filteredLocalSessionRows.map(\.id),
            ["session-develop", "session-alpha"],
            "Session sort controls should request ordered project rows from the service."
        )
        try expectEqual(
            countMethodCalls("session.previewLocalSessions", in: fake.calls()),
            callsBeforeSortChange + 1,
            "Changing local session sort should trigger a service refresh when explicitly previewed."
        )
        try expectContains(fake.calls(), "\"sort\":\"title\"", "Session sort order should be sent to the service.")
        try expectContains(fake.calls(), "\"direction\":\"descending\"", "Session sort direction should be sent to the service.")
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
        let fake = LLMReadyRecordingService()

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        try expectNil(store.errorMessage, "LLM prepare fixture should reload without service errors.")
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
        let calls = fake.calls()
        try expectContains(calls, "llm.prepareAction", "LLM action should use prepare preflight.")
        try expectFalse(calls.contains("llm.complete"), "LLM prepare should not call a provider completion method.")
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

        try expectEqual(countMethodCalls("llm.providerObservability", in: fake.calls()), 1, "Startup should preload provider observability once.")
        try expectEqual(store.providerObservabilityResult?.summary.callCount, 3, "Startup observability preload should keep the decoded dashboard.")

        await store.loadProviderObservabilityIfNeeded()

        try expectEqual(countMethodCalls("llm.providerObservability", in: fake.calls()), 1, "Need-based observability loading should reuse the startup cache.")

        await store.reload()

        try expectEqual(countMethodCalls("llm.providerObservability", in: fake.calls()), 2, "Global reload should refresh provider observability because Settings has no local build button.")

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
        try await previewAndConfirmTaskCockpit(firstStore)

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
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let historyStore = makeTemporaryTaskCockpitHistoryStore()
        defer { cleanupTaskCockpitHistoryStore(historyStore) }
        let store = SkillStore(service: fake.serviceClient(), taskCockpitHistoryStore: historyStore)
        store.selectedSkillID = "beta"
        store.taskCockpitText = " \n\t "
        await store.reload()
        await store.buildTaskCockpit()

        try expectEqual(store.selectedTaskCockpitInput, "", "Whitespace-only cockpit input should not reuse old task fields.")
        try expectEqual(store.taskCockpitResult?.isUnavailable, true, "Whitespace-only cockpit input should produce an unavailable result.")
        try expectEqual(store.taskCockpitResult?.fallbackReason, UIStrings.taskCockpitTaskRequired, "Whitespace-only cockpit input should ask for a task.")

        let calls = fake.calls()
        try expectFalse(calls.contains("llm.previewPrompt"), "Whitespace-only cockpit input must not prepare a provider prompt.")
        try expectFalse(calls.contains("config.toggleSkill"), "Whitespace-only cockpit flow must not call config write paths.")
        try expectFalse(calls.contains("script.execute"), "Whitespace-only cockpit flow must not call execution paths.")
    }

    private func taskCockpitFallsBackWhenMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let historyStore = makeTemporaryTaskCockpitHistoryStore()
        defer { cleanupTaskCockpitHistoryStore(historyStore) }
        let store = SkillStore(service: fake.serviceClient(), taskCockpitHistoryStore: historyStore)
        store.selectedSkillID = "beta"
        store.taskCockpitText = "Route a local audit release note task."
        await store.reload()
        await store.buildTaskCockpit()

        try expectEqual(store.taskCockpitResult?.isUnavailable, true, "Task cockpit should expose unavailable fallback for older services.")
        try expectEqual(store.taskCockpitResult?.fallbackReason, UIStrings.taskCockpitUnavailable, "Unknown method fallback should use the localized unavailable copy.")
        try expectFalse(store.isBuildingTaskCockpit, "Unavailable task cockpit should reset loading state.")
        try expectContains(fake.calls(), "llm.previewPrompt", "Fallback should still prove the intended provider preview method was attempted.")
        try expectContains(fake.calls(), "\"task_text\":\"Route a local audit release note task.\"", "Fallback should reuse existing routing task text when cockpit input is blank.")
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

    private func decodeAppSearchItem(_ json: String) throws -> AppSearchItem {
        try JSONDecoder().decode(AppSearchItem.self, from: Data(json.utf8))
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

private final class LLMReadyRecordingService: ServiceProcessRunning {
    private let lock = NSLock()
    private var recordedInputs: [String] = []

    func serviceClient() -> ServiceClient {
        ServiceClient(processRunner: self, serviceURL: URL(fileURLWithPath: "/usr/bin/true"))
    }

    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) async throws -> Data {
        record(input)
        switch requestMethod(from: input) {
        case "app.stateSnapshot":
            return ok(appStateSnapshotResult)
        case "llm.status":
            return ok(llmStatusResult)
        case "llm.prepareAction":
            let inputText = String(data: input, encoding: .utf8) ?? ""
            if inputText.contains(#""kind":"draft_frontmatter""#) {
                return ok(draftPrepareResult)
            }
            return ok(analyzePrepareResult)
        case "rules.listTuning":
            return ok("[]")
        case "snapshot.listAgentConfig":
            return ok("[]")
        case "catalog.getSkill":
            return ok(betaDetailResult)
        case "skill.listEvents":
            return ok("[]")
        default:
            return unknownMethod(requestMethod(from: input))
        }
    }

    func calls() -> String {
        lock.lock()
        let value = recordedInputs.joined(separator: "\n")
        lock.unlock()
        return value
    }

    private func record(_ input: Data) {
        lock.lock()
        recordedInputs.append(String(data: input, encoding: .utf8) ?? "")
        lock.unlock()
    }

    private func requestMethod(from input: Data) -> String {
        guard
            let object = try? JSONSerialization.jsonObject(with: input) as? [String: Any],
            let method = object["method"] as? String
        else {
            return ""
        }
        return method
    }

    private func ok(_ result: String) -> Data {
        Data(#"{"id":"test","ok":true,"result":\#(result)}"#.utf8)
    }

    private func unknownMethod(_ method: String) -> Data {
        Data(#"{"id":"test","ok":false,"result":null,"error":{"code":"unknown_method","message":"unknown method: \#(method)"}}"#.utf8)
    }

    private var appStateSnapshotResult: String {
        """
        {
          "status": \(serviceStatusResult),
          "skills": [
            {
              "id": "alpha",
              "agent": "claude-code",
              "scope": "agent-global",
              "path": "/tmp/global/alpha/SKILL.md",
              "display_path": "/tmp/global/alpha/SKILL.md",
              "definition_id": "def.alpha",
              "name": "Alpha",
              "state": "loaded",
              "enabled": true
            },
            {
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
          ],
          "findings": [],
          "conflicts": [],
          "snapshots": []
        }
        """
    }

    private let serviceStatusResult = """
    {
      "protocol_version": 1,
      "version": "test",
      "app_data_dir": "/tmp/skills-copilot",
      "catalog_path": "/tmp/skills-copilot/catalog.sqlite",
      "user_home": "/tmp/home",
      "supported_methods": [
        "app.stateSnapshot",
        "llm.status",
        "llm.prepareAction",
        "rules.listTuning",
        "snapshot.listAgentConfig",
        "catalog.getSkill",
        "skill.listEvents"
      ],
      "adapter_capabilities": []
    }
    """

    private let llmStatusResult = """
    {
      "enabled": true,
      "provider": "openai",
      "model": "gpt-5",
      "disabled_reason": null,
      "supported_actions": [
        "analyze",
        "recommend",
        "explain_conflict",
        "draft_frontmatter"
      ]
    }
    """

    private let analyzePrepareResult = """
    {
      "action": "analyze",
      "enabled": true,
      "disabled_reason": null,
      "provider": "openai",
      "model": "gpt-5",
      "estimate": {
        "input_tokens": 240,
        "output_tokens": 120,
        "total_tokens": 360,
        "estimated_cost_usd": 0.0042
      },
      "confirmation_required": true
    }
    """

    private let draftPrepareResult = """
    {
      "action": "draft_frontmatter",
      "enabled": true,
      "disabled_reason": null,
      "provider": "openai",
      "model": "gpt-5",
      "estimate": {
        "input_tokens": 240,
        "output_tokens": 180,
        "total_tokens": 420,
        "estimated_cost_usd": 0.0042
      },
      "confirmation_required": true
    }
    """

    private let betaDetailResult = """
    {
      "id": "beta",
      "agent": "claude-code",
      "scope": "agent-project",
      "path": "/tmp/project/beta/SKILL.md",
      "display_path": "/tmp/project/beta/SKILL.md",
      "definition_id": "def.beta",
      "name": "Beta",
      "description": "Beta skill",
      "state": "loaded",
      "enabled": true,
      "frontmatter_raw": "name: Beta",
      "body": "Beta body",
      "permissions": {
        "marker": "default"
      },
      "fingerprint": "fp-beta"
    }
    """
}
