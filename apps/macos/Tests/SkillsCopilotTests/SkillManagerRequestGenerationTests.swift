import Foundation
@testable import SkillsCopilot

@MainActor
struct SkillManagerRequestGenerationTests {
    func run() async throws {
        try canonicalInputsNormalizeIdentity()
        try await newerSearchWinsWhenOlderResponseFinishesLast()
        try await staleSearchErrorDoesNotReplaceNewSuccess()
        try await installedListIsScopedToCapturedAgentsAndScope()
        try await inputChangeInvalidatesMutationPreview()
        try await localCreateInputChangeIgnoresOldPreview()
        try await localDeleteSelectionChangeIgnoresOldPreview()
        try await staleMutationConfirmationCannotApplyCurrentPreview()
        try await staleLocalCreateConfirmationCannotApplyCurrentPreview()
        try await staleLocalDeleteConfirmationCannotApplyCurrentPreview()
        try await sameMutationValuesStillRequireCurrentGeneration()
        try await sameLocalCreateValuesStillRequireCurrentGeneration()
        try await sameLocalDeleteValuesStillRequireCurrentGeneration()
        try await mutationApplyCompletionPreservesNewerAndCrossFamilyConfirmations()
        try await localCreateApplyCompletionPreservesNewerAndCrossFamilyConfirmations()
        try await localDeleteApplyCompletionPreservesNewerAndCrossFamilyConfirmations()
        try await callerCancellationPropagatesToAllRequestFamilies()
        try await hungCancelledRequestDoesNotRetainStore()
        try await sameFamilyGenerationReleasesSupersededCaller()
        try await inputInvalidationReleasesHungRequestAndStore()
        try await mutationApplyConvergesAfterCallerCancellation()
        try await localCreateApplyConvergesAfterCallerCancellation()
        try await localDeleteApplyConvergesAfterCallerCancellation()
        try await applyUsesExactPreviewInputsAndToken()
        try await oldCompletionDoesNotClearCurrentLoadingState()
    }

    private func canonicalInputsNormalizeIdentity() throws {
        let inputs = SkillManagerMutationInputs(
            kind: .install,
            source: "  owner/repo  ",
            skills: [" beta ", "alpha", "alpha", ""],
            agents: ["codex", "claude-code", "codex"],
            scope: .project,
            distribution: .symlink,
            networkAllowed: false
        )

        try expectEqual(inputs.source, "owner/repo", "Mutation source should be trimmed once when captured.")
        try expectEqual(inputs.skills, ["alpha", "beta"], "Mutation skills should be trimmed, unique, and sorted.")
        try expectEqual(inputs.agents, ["claude-code", "codex"], "Mutation agents should be unique and sorted.")
    }

    private func newerSearchWinsWhenOlderResponseFinishesLast() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        await runner.suspend("search:old")
        await runner.suspend("search:new")
        let store = makeStore(runner)

        store.skillManagerSearchQuery = "old"
        let old = Task { await store.searchSkillManager() }
        try await waitForPending("search:old", runner: runner)

        store.skillManagerSearchQuery = "new"
        let new = Task { await store.searchSkillManager() }
        try await waitForPending("search:new", runner: runner)

        await runner.resumeSuccess("search:new")
        await new.value
        await runner.resumeSuccess("search:old")
        await old.value

        try expectEqual(
            store.skillManagerSearchResult?.results.first?.name,
            "new",
            "An older search completion must not replace the newest result."
        )
        let calls = await runner.recordedCalls(method: "skillManager.search")
        try expectEqual(
            calls.map(\.owner),
            [nil, nil] as [String?],
            "Search requests should canonicalize a blank owner before constructing their request keys."
        )
    }

    private func staleSearchErrorDoesNotReplaceNewSuccess() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        await runner.suspend("search:stale-error")
        await runner.suspend("search:current")
        let store = makeStore(runner)

        store.skillManagerSearchQuery = "stale-error"
        let stale = Task { await store.searchSkillManager() }
        try await waitForPending("search:stale-error", runner: runner)
        store.skillManagerSearchQuery = "current"
        let current = Task { await store.searchSkillManager() }
        try await waitForPending("search:current", runner: runner)

        await runner.resumeSuccess("search:current")
        await current.value
        await runner.resumeServiceError("search:stale-error")
        await stale.value

        try expectEqual(store.skillManagerSearchResult?.results.first?.name, "current", "Current search success should remain visible.")
        try expectNil(store.skillManagerErrorMessage, "A stale search error must not replace current success feedback.")
    }

    private func installedListIsScopedToCapturedAgentsAndScope() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        await runner.suspend("installed:project:codex")
        await runner.suspend("installed:global:pi")
        let store = makeStore(runner)
        store.skillManagerSelectedAgentIDs = ["codex"]
        store.skillManagerScope = .project

        let old = Task { await store.listSkillManagerInstalled() }
        try await waitForPending("installed:project:codex", runner: runner)
        store.skillManagerSelectedAgentIDs = ["pi"]
        store.skillManagerScope = .global
        let current = Task { await store.listSkillManagerInstalled() }
        try await waitForPending("installed:global:pi", runner: runner)

        await runner.resumeSuccess("installed:global:pi")
        await current.value
        await runner.resumeSuccess("installed:project:codex")
        await old.value

        try expectEqual(store.skillManagerInstalled?.installed.first?.name, "global:pi", "Installed results should match the current captured request.")
        let calls = await runner.recordedCalls(method: "skillManager.listInstalled")
        try expectEqual(calls.map(\.scope), ["project", "global"], "Each installed request should retain its captured scope.")
        try expectEqual(calls.map(\.agents), [["codex"], ["pi"]], "Each installed request should retain its captured agents.")
    }

    private func inputChangeInvalidatesMutationPreview() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        let store = makeStore(runner)
        store.skillManagerSource = "owner/repo"
        store.skillManagerInstallSkillName = "alpha"
        store.skillManagerSelectedAgentIDs = ["codex"]

        await store.previewSkillManagerInstall()
        guard let confirmation = store.skillManagerMutationConfirmation else {
            throw NativeModelTestFailure(description: "Preview should capture canonical mutation inputs.")
        }
        try expectEqual(confirmation.inputs.source, "owner/repo", "Preview should capture canonical mutation inputs.")

        store.skillManagerSource = "other/repo"
        try expectNil(store.skillManagerMutationConfirmation, "Changing mutation input should invalidate confirmation.")
        await store.applySkillManagerInstall(confirmation: confirmation)
        try expectEqual(
            await runner.recordedCalls(method: "skillManager.applyInstall").count,
            0,
            "An invalidated mutation preview must not be applied."
        )
    }

    private func localCreateInputChangeIgnoresOldPreview() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        await runner.suspend("local-create:old-name")
        let store = makeStore(runner)
        store.skillManagerLocalSkillName = "old-name"

        let old = Task { await store.previewSkillManagerLocalCreate() }
        try await waitForPending("local-create:old-name", runner: runner)
        store.skillManagerLocalSkillName = "new-name"
        await runner.resumeSuccess("local-create:old-name")
        await old.value

        try expectNil(store.skillManagerLocalCreateConfirmation, "An old local-create response should be ignored after name changes.")
        try expectEqual(store.isPreviewingSkillManagerLocalCreate, false, "A stale local-create completion should not leave loading active.")
    }

    private func localDeleteSelectionChangeIgnoresOldPreview() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        await runner.suspend("local-delete:local-a")
        await runner.suspend("local-delete:local-b")
        let store = makeStore(runner)

        let old = Task { await store.previewSkillManagerLocalDelete(skill: localSkill(id: "local-a")) }
        try await waitForPending("local-delete:local-a", runner: runner)
        let current = Task { await store.previewSkillManagerLocalDelete(skill: localSkill(id: "local-b")) }
        try await waitForPending("local-delete:local-b", runner: runner)

        await runner.resumeSuccess("local-delete:local-b")
        await current.value
        await runner.resumeSuccess("local-delete:local-a")
        await old.value

        try expectEqual(store.skillManagerLocalDeleteConfirmation?.instanceID, "local-b", "Only the newest local-delete selection should remain confirmable.")
    }

    private func staleMutationConfirmationCannotApplyCurrentPreview() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        let store = makeStore(runner)
        store.skillManagerSelectedAgentIDs = ["codex"]
        store.skillManagerSource = "owner/a"
        store.skillManagerInstallSkillName = "alpha"
        await store.previewSkillManagerInstall()
        guard let capturedA = store.skillManagerMutationConfirmation else {
            throw NativeModelTestFailure(description: "Mutation preview A should be captured.")
        }

        store.skillManagerSource = "owner/b"
        store.skillManagerInstallSkillName = "beta"
        await store.previewSkillManagerInstall()
        guard let currentB = store.skillManagerMutationConfirmation else {
            throw NativeModelTestFailure(description: "Mutation preview B should be current.")
        }

        await store.applySkillManagerInstall(confirmation: capturedA)
        try expectEqual(
            await runner.recordedCalls(method: "skillManager.applyInstall").count,
            0,
            "A stale mutation confirmation must not apply the current preview."
        )
        try expectEqual(store.skillManagerMutationConfirmation, currentB, "A stale mutation apply must leave current preview B intact.")
        try expectNil(store.skillManagerMessage, "A stale mutation apply must not publish success feedback.")

        await store.applySkillManagerInstall(confirmation: currentB)
        guard let apply = await runner.recordedCalls(method: "skillManager.applyInstall").last else {
            throw NativeModelTestFailure(description: "Current mutation preview B should apply.")
        }
        try expectEqual(apply.source, currentB.inputs.source, "Current mutation apply should use B's captured source.")
        try expectEqual(apply.previewToken, currentB.previewToken, "Current mutation apply should use B's exact token.")
    }

    private func staleLocalCreateConfirmationCannotApplyCurrentPreview() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        let store = makeStore(runner)
        store.skillManagerLocalSkillName = "local-a"
        await store.previewSkillManagerLocalCreate()
        guard let capturedA = store.skillManagerLocalCreateConfirmation else {
            throw NativeModelTestFailure(description: "Local-create preview A should be captured.")
        }

        store.skillManagerLocalSkillName = "local-b"
        await store.previewSkillManagerLocalCreate()
        guard let currentB = store.skillManagerLocalCreateConfirmation else {
            throw NativeModelTestFailure(description: "Local-create preview B should be current.")
        }

        await store.applySkillManagerLocalCreate(confirmation: capturedA)
        try expectEqual(
            await runner.recordedCalls(method: "skillManager.applyLocalCreate").count,
            0,
            "A stale local-create confirmation must not apply the current preview."
        )
        try expectEqual(store.skillManagerLocalCreateConfirmation, currentB, "A stale local-create apply must leave current preview B intact.")
        try expectNil(store.skillManagerMessage, "A stale local-create apply must not publish success feedback.")

        await store.applySkillManagerLocalCreate(confirmation: currentB)
        guard let apply = await runner.recordedCalls(method: "skillManager.applyLocalCreate").last else {
            throw NativeModelTestFailure(description: "Current local-create preview B should apply.")
        }
        try expectEqual(apply.name, currentB.name, "Current local-create apply should use B's captured name.")
        try expectEqual(apply.previewToken, currentB.previewToken, "Current local-create apply should use B's exact token.")
    }

    private func staleLocalDeleteConfirmationCannotApplyCurrentPreview() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        let store = makeStore(runner)
        await store.previewSkillManagerLocalDelete(skill: localSkill(id: "local-a"))
        guard let capturedA = store.skillManagerLocalDeleteConfirmation else {
            throw NativeModelTestFailure(description: "Local-delete preview A should be captured.")
        }

        await store.previewSkillManagerLocalDelete(skill: localSkill(id: "local-b"))
        guard let currentB = store.skillManagerLocalDeleteConfirmation else {
            throw NativeModelTestFailure(description: "Local-delete preview B should be current.")
        }

        await store.applySkillManagerLocalDelete(confirmation: capturedA)
        let staleApplyCalls = await runner.recordedCalls(method: "skillManager.deleteLocal").filter { $0.confirmed }
        try expectEqual(staleApplyCalls.count, 0, "A stale local-delete confirmation must not delete the current target.")
        try expectEqual(store.skillManagerLocalDeleteConfirmation, currentB, "A stale local-delete apply must leave current preview B intact.")
        try expectNil(store.skillManagerMessage, "A stale local-delete apply must not publish success feedback.")

        await store.applySkillManagerLocalDelete(confirmation: currentB)
        let currentApplyCalls = await runner.recordedCalls(method: "skillManager.deleteLocal").filter { $0.confirmed }
        try expectEqual(currentApplyCalls.count, 1, "Current local-delete preview B should apply once.")
        try expectEqual(currentApplyCalls.first?.instanceID, currentB.instanceID, "Current local-delete apply should use B's captured instance ID.")
    }

    private func sameMutationValuesStillRequireCurrentGeneration() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        let store = makeStore(runner)
        store.skillManagerSelectedAgentIDs = ["codex"]
        store.skillManagerSource = "owner/repo"
        store.skillManagerInstallSkillName = "alpha"

        await store.previewSkillManagerInstall()
        guard let capturedA = store.skillManagerMutationConfirmation else {
            throw NativeModelTestFailure(description: "Mutation preview A should be captured.")
        }
        await store.previewSkillManagerInstall()
        guard let currentB = store.skillManagerMutationConfirmation else {
            throw NativeModelTestFailure(description: "Mutation preview B should be current.")
        }

        try expectFalse(capturedA == currentB, "Same-value mutation previews from different generations must have distinct identities.")
        await store.applySkillManagerInstall(confirmation: capturedA)
        try expectEqual(
            await runner.recordedCalls(method: "skillManager.applyInstall").count,
            0,
            "Same-value stale mutation confirmation A must not issue an apply RPC."
        )
        try expectEqual(store.skillManagerMutationConfirmation, currentB, "Rejected mutation A must leave current B intact.")

        await store.applySkillManagerInstall(confirmation: currentB)
        try expectEqual(
            await runner.recordedCalls(method: "skillManager.applyInstall").count,
            1,
            "Current mutation confirmation B should remain applicable."
        )
    }

    private func sameLocalCreateValuesStillRequireCurrentGeneration() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        let store = makeStore(runner)
        store.skillManagerLocalSkillName = "local-note"

        await store.previewSkillManagerLocalCreate()
        guard let capturedA = store.skillManagerLocalCreateConfirmation else {
            throw NativeModelTestFailure(description: "Local-create preview A should be captured.")
        }
        await store.previewSkillManagerLocalCreate()
        guard let currentB = store.skillManagerLocalCreateConfirmation else {
            throw NativeModelTestFailure(description: "Local-create preview B should be current.")
        }

        try expectFalse(capturedA == currentB, "Same-value local-create previews from different generations must have distinct identities.")
        await store.applySkillManagerLocalCreate(confirmation: capturedA)
        try expectEqual(
            await runner.recordedCalls(method: "skillManager.applyLocalCreate").count,
            0,
            "Same-value stale local-create confirmation A must not issue an apply RPC."
        )
        try expectEqual(store.skillManagerLocalCreateConfirmation, currentB, "Rejected local-create A must leave current B intact.")

        await store.applySkillManagerLocalCreate(confirmation: currentB)
        try expectEqual(
            await runner.recordedCalls(method: "skillManager.applyLocalCreate").count,
            1,
            "Current local-create confirmation B should remain applicable."
        )
    }

    private func sameLocalDeleteValuesStillRequireCurrentGeneration() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        let store = makeStore(runner)
        let skill = localSkill(id: "local-note")

        await store.previewSkillManagerLocalDelete(skill: skill)
        guard let capturedA = store.skillManagerLocalDeleteConfirmation else {
            throw NativeModelTestFailure(description: "Local-delete preview A should be captured.")
        }
        await store.previewSkillManagerLocalDelete(skill: skill)
        guard let currentB = store.skillManagerLocalDeleteConfirmation else {
            throw NativeModelTestFailure(description: "Local-delete preview B should be current.")
        }

        try expectFalse(capturedA == currentB, "Same-value local-delete previews from different generations must have distinct identities.")
        await store.applySkillManagerLocalDelete(confirmation: capturedA)
        let staleApplyCalls = await runner.recordedCalls(method: "skillManager.deleteLocal").filter(\.confirmed)
        try expectEqual(staleApplyCalls.count, 0, "Same-value stale local-delete confirmation A must not issue an apply RPC.")
        try expectEqual(store.skillManagerLocalDeleteConfirmation, currentB, "Rejected local-delete A must leave current B intact.")

        await store.applySkillManagerLocalDelete(confirmation: currentB)
        let currentApplyCalls = await runner.recordedCalls(method: "skillManager.deleteLocal").filter(\.confirmed)
        try expectEqual(currentApplyCalls.count, 1, "Current local-delete confirmation B should remain applicable.")
    }

    private func mutationApplyCompletionPreservesNewerAndCrossFamilyConfirmations() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        let store = makeStore(runner)
        store.skillManagerSelectedAgentIDs = ["codex"]
        store.skillManagerSource = "owner/a"
        store.skillManagerInstallSkillName = "alpha"
        await store.previewSkillManagerInstall()
        guard let applyA = store.skillManagerMutationConfirmation else {
            throw NativeModelTestFailure(description: "Mutation confirmation A should exist.")
        }

        await runner.suspend("skillManager.applyInstall")
        let applyingA = Task { await store.applySkillManagerInstall(confirmation: applyA) }
        try await waitForPending("skillManager.applyInstall", runner: runner)

        store.skillManagerSource = "owner/b"
        store.skillManagerInstallSkillName = "beta"
        await store.previewSkillManagerInstall()
        guard let mutationB = store.skillManagerMutationConfirmation else {
            throw NativeModelTestFailure(description: "Newer mutation confirmation B should exist during apply A.")
        }
        store.skillManagerLocalSkillName = "local-c"
        await store.previewSkillManagerLocalCreate()
        guard let localCreateC = store.skillManagerLocalCreateConfirmation else {
            throw NativeModelTestFailure(description: "Cross-family local-create confirmation C should exist during apply A.")
        }

        await runner.resumeSuccess("skillManager.applyInstall")
        await applyingA.value
        try expectEqual(store.skillManagerMutationConfirmation, mutationB, "Mutation apply A must not retire newer mutation B.")
        try expectEqual(store.skillManagerLocalCreateConfirmation, localCreateC, "Mutation apply A must not retire cross-family local-create C.")

        await store.applySkillManagerInstall(confirmation: mutationB)
        try expectEqual(
            await runner.recordedCalls(method: "skillManager.applyInstall").count,
            2,
            "Newer mutation B should remain applicable after A completes."
        )
        try expectEqual(store.skillManagerLocalCreateConfirmation, localCreateC, "Applying mutation B must leave local-create C intact.")
        await store.applySkillManagerLocalCreate(confirmation: localCreateC)
        try expectEqual(
            await runner.recordedCalls(method: "skillManager.applyLocalCreate").count,
            1,
            "Cross-family local-create C should remain applicable."
        )
    }

    private func localCreateApplyCompletionPreservesNewerAndCrossFamilyConfirmations() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        let store = makeStore(runner)
        store.skillManagerLocalSkillName = "local-a"
        await store.previewSkillManagerLocalCreate()
        guard let applyA = store.skillManagerLocalCreateConfirmation else {
            throw NativeModelTestFailure(description: "Local-create confirmation A should exist.")
        }

        await runner.suspend("skillManager.applyLocalCreate")
        let applyingA = Task { await store.applySkillManagerLocalCreate(confirmation: applyA) }
        try await waitForPending("skillManager.applyLocalCreate", runner: runner)

        store.skillManagerLocalSkillName = "local-b"
        await store.previewSkillManagerLocalCreate()
        guard let localCreateB = store.skillManagerLocalCreateConfirmation else {
            throw NativeModelTestFailure(description: "Newer local-create confirmation B should exist during apply A.")
        }
        await store.previewSkillManagerLocalDelete(skill: localSkill(id: "local-c"))
        guard let localDeleteC = store.skillManagerLocalDeleteConfirmation else {
            throw NativeModelTestFailure(description: "Cross-family local-delete confirmation C should exist during apply A.")
        }

        await runner.resumeSuccess("skillManager.applyLocalCreate")
        await applyingA.value
        try expectEqual(store.skillManagerLocalCreateConfirmation, localCreateB, "Local-create apply A must not retire newer local-create B.")
        try expectEqual(store.skillManagerLocalDeleteConfirmation, localDeleteC, "Local-create apply A must not retire cross-family local-delete C.")

        await store.applySkillManagerLocalCreate(confirmation: localCreateB)
        try expectEqual(
            await runner.recordedCalls(method: "skillManager.applyLocalCreate").count,
            2,
            "Newer local-create B should remain applicable after A completes."
        )
        try expectEqual(store.skillManagerLocalDeleteConfirmation, localDeleteC, "Applying local-create B must leave local-delete C intact.")
        await store.applySkillManagerLocalDelete(confirmation: localDeleteC)
        let deleteCalls = await runner.recordedCalls(method: "skillManager.deleteLocal").filter(\.confirmed)
        try expectEqual(deleteCalls.count, 1, "Cross-family local-delete C should remain applicable.")
    }

    private func localDeleteApplyCompletionPreservesNewerAndCrossFamilyConfirmations() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        let store = makeStore(runner)
        await store.previewSkillManagerLocalDelete(skill: localSkill(id: "local-a"))
        guard let applyA = store.skillManagerLocalDeleteConfirmation else {
            throw NativeModelTestFailure(description: "Local-delete confirmation A should exist.")
        }

        await runner.suspend("skillManager.deleteLocal")
        let applyingA = Task { await store.applySkillManagerLocalDelete(confirmation: applyA) }
        try await waitForPending("skillManager.deleteLocal", runner: runner)

        await store.previewSkillManagerLocalDelete(skill: localSkill(id: "local-b"))
        guard let localDeleteB = store.skillManagerLocalDeleteConfirmation else {
            throw NativeModelTestFailure(description: "Newer local-delete confirmation B should exist during apply A.")
        }
        store.skillManagerSelectedAgentIDs = ["codex"]
        store.skillManagerSource = "owner/c"
        store.skillManagerInstallSkillName = "gamma"
        await store.previewSkillManagerInstall()
        guard let mutationC = store.skillManagerMutationConfirmation else {
            throw NativeModelTestFailure(description: "Cross-family mutation confirmation C should exist during apply A.")
        }

        await runner.resumeSuccess("skillManager.deleteLocal")
        await applyingA.value
        try expectEqual(store.skillManagerLocalDeleteConfirmation, localDeleteB, "Local-delete apply A must not retire newer local-delete B.")
        try expectEqual(store.skillManagerMutationConfirmation, mutationC, "Local-delete apply A must not retire cross-family mutation C.")

        await store.applySkillManagerLocalDelete(confirmation: localDeleteB)
        let deleteCalls = await runner.recordedCalls(method: "skillManager.deleteLocal").filter(\.confirmed)
        try expectEqual(deleteCalls.count, 2, "Newer local-delete B should remain applicable after A completes.")
        try expectEqual(store.skillManagerMutationConfirmation, mutationC, "Applying local-delete B must leave mutation C intact.")
        await store.applySkillManagerInstall(confirmation: mutationC)
        try expectEqual(
            await runner.recordedCalls(method: "skillManager.applyInstall").count,
            1,
            "Cross-family mutation C should remain applicable."
        )
    }

    private func callerCancellationPropagatesToAllRequestFamilies() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        let labels = [
            "search:cancel-search",
            "installed:project:codex",
            "mutation:install:owner/cancel",
            "local-create:cancel-local",
            "local-delete:cancel-delete"
        ]
        for label in labels {
            await runner.suspendCancellable(label)
        }
        let store = makeStore(runner)
        store.skillManagerSelectedAgentIDs = ["codex"]
        store.skillManagerScope = .project
        store.skillManagerSearchQuery = "cancel-search"
        store.skillManagerSource = "owner/cancel"
        store.skillManagerInstallSkillName = "alpha"
        store.skillManagerLocalSkillName = "cancel-local"

        let search = Task { await store.searchSkillManager() }
        let installed = Task { await store.listSkillManagerInstalled() }
        let mutation = Task { await store.previewSkillManagerInstall() }
        let localCreate = Task { await store.previewSkillManagerLocalCreate() }
        let localDelete = Task { await store.previewSkillManagerLocalDelete(skill: localSkill(id: "cancel-delete")) }
        for label in labels {
            try await waitForPending(label, runner: runner)
        }

        for task in [search, installed, mutation, localCreate, localDelete] {
            task.cancel()
        }
        let cancellationPropagated = await waitForCancellationCount(labels.count, runner: runner)
        if !cancellationPropagated {
            for label in labels {
                await runner.resumeSuccess(label)
            }
        }
        for task in [search, installed, mutation, localCreate, localDelete] {
            await task.value
        }

        try expectEqual(
            await runner.totalCancellationCount(),
            labels.count,
            "Caller cancellation must reach every Skill Manager service request family."
        )
        try expectEqual(store.isSearchingSkillManager, false, "Cancelled search loading should converge.")
        try expectEqual(store.isListingSkillManagerInstalled, false, "Cancelled installed loading should converge.")
        try expectEqual(store.isPreviewingSkillManagerMutation, false, "Cancelled mutation loading should converge.")
        try expectEqual(store.isPreviewingSkillManagerLocalCreate, false, "Cancelled local-create loading should converge.")
        try expectEqual(store.isPreviewingSkillManagerLocalDelete, false, "Cancelled local-delete loading should converge.")
        try expectNil(store.skillManagerErrorMessage, "Expected caller cancellation should not publish an error banner.")
    }

    private func hungCancelledRequestDoesNotRetainStore() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        await runner.suspend("search:hung-cancel")
        var store: SkillStore? = makeStore(runner)
        weak let weakStore = store
        store?.skillManagerSearchQuery = "hung-cancel"
        let completion = SkillManagerRequestCompletionFlag()

        let request = Task { [weak store] in
            await store?.searchSkillManager()
            await completion.markFinished()
        }
        try await waitForPending("search:hung-cancel", runner: runner)
        request.cancel()
        store = nil

        let completedBeforeServiceReturned = await waitForCompletion(completion)
        let releasedBeforeServiceReturned = weakStore == nil
        await runner.resumeSuccess("search:hung-cancel")
        await request.value

        try expectEqual(
            completedBeforeServiceReturned,
            true,
            "Caller cancellation should return even when the service transport ignores cancellation."
        )
        try expectEqual(
            releasedBeforeServiceReturned,
            true,
            "A hung cancelled request must not retain the Store through its inner request closure."
        )
    }

    private func inputInvalidationReleasesHungRequestAndStore() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        await runner.suspend("search:internally-invalidated")
        var store: SkillStore? = makeStore(runner)
        weak let weakStore = store
        store?.skillManagerSearchQuery = "internally-invalidated"
        let completion = SkillManagerRequestCompletionFlag()

        let request = Task { [weak store] in
            await store?.searchSkillManager()
            await completion.markFinished()
        }
        try await waitForPending("search:internally-invalidated", runner: runner)
        store?.skillManagerSearchQuery = "replacement-input"
        store = nil

        let completedBeforeServiceReturned = await waitForCompletion(completion)
        let releasedBeforeServiceReturned = weakStore == nil
        await runner.resumeSuccess("search:internally-invalidated")
        await request.value

        try expectEqual(
            completedBeforeServiceReturned,
            true,
            "Input invalidation should wake a superseded caller even when its transport ignores cancellation."
        )
        try expectEqual(
            releasedBeforeServiceReturned,
            true,
            "Input invalidation should release the Store before the superseded transport returns."
        )
    }

    private func sameFamilyGenerationReleasesSupersededCaller() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        await runner.suspendNext("search:same-family")
        let store = makeStore(runner)
        store.skillManagerSearchQuery = "same-family"
        let firstCompletion = SkillManagerRequestCompletionFlag()

        let first = Task {
            await store.searchSkillManager()
            await firstCompletion.markFinished()
        }
        try await waitForPending("search:same-family", runner: runner)

        let second = Task { await store.searchSkillManager() }
        await second.value
        let firstCompletedBeforeServiceReturned = await waitForCompletion(firstCompletion)
        let currentResultBeforeLateError = store.skillManagerSearchResult?.results.first?.name
        let loadingBeforeLateError = store.isSearchingSkillManager

        await runner.resumeServiceError("search:same-family")
        await first.value

        try expectEqual(
            firstCompletedBeforeServiceReturned,
            true,
            "Beginning generation B should wake generation A's caller when A ignores cancellation."
        )
        try expectEqual(
            currentResultBeforeLateError,
            "same-family",
            "Generation B should publish its own success before generation A is released."
        )
        try expectEqual(
            loadingBeforeLateError,
            false,
            "Generation A must not keep generation B's completed loading state active."
        )
        try expectEqual(
            store.skillManagerSearchResult?.results.first?.name,
            "same-family",
            "Generation A's late service error must not replace generation B's success."
        )
        try expectNil(store.skillManagerErrorMessage, "Generation A's late service error must remain silent.")
        try expectEqual(
            store.isSearchingSkillManager,
            false,
            "Generation A's late completion must not restore or clear generation B's loading state."
        )
    }

    private func mutationApplyConvergesAfterCallerCancellation() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        let store = makeStore(runner)
        store.skillManagerSelectedAgentIDs = ["codex"]
        store.skillManagerSource = "owner/write"
        store.skillManagerInstallSkillName = "alpha"
        await store.previewSkillManagerInstall()
        guard let confirmation = store.skillManagerMutationConfirmation else {
            throw NativeModelTestFailure(description: "Mutation confirmation should exist before apply cancellation test.")
        }
        await runner.suspendCancellable("skillManager.applyInstall")

        let applying = Task { await store.applySkillManagerInstall(confirmation: confirmation) }
        try await waitForPending("skillManager.applyInstall", runner: runner)
        applying.cancel()
        try? await Task.sleep(nanoseconds: 100_000_000)
        let cancellationCount = await runner.totalCancellationCount()
        if cancellationCount == 0 {
            await runner.resumeSuccess("skillManager.applyInstall")
        }
        await applying.value

        try expectEqual(cancellationCount, 0, "Caller cancellation must not abort a started confirmed mutation write.")
        try expectEqual(
            await runner.recordedCalls(method: "skillManager.applyInstall").count,
            1,
            "Confirmed mutation write must run exactly once."
        )
        try expectEqual(store.isApplyingSkillManagerMutation, false, "Mutation apply state should converge after the owned write finishes.")
    }

    private func localCreateApplyConvergesAfterCallerCancellation() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        let store = makeStore(runner)
        store.skillManagerLocalSkillName = "owned-local"
        await store.previewSkillManagerLocalCreate()
        guard let confirmation = store.skillManagerLocalCreateConfirmation else {
            throw NativeModelTestFailure(description: "Local-create confirmation should exist before apply cancellation test.")
        }
        await runner.suspendCancellable("skillManager.applyLocalCreate")

        let applying = Task { await store.applySkillManagerLocalCreate(confirmation: confirmation) }
        try await waitForPending("skillManager.applyLocalCreate", runner: runner)
        applying.cancel()
        try? await Task.sleep(nanoseconds: 100_000_000)
        let cancellationCount = await runner.totalCancellationCount()
        if cancellationCount == 0 {
            await runner.resumeSuccess("skillManager.applyLocalCreate")
        }
        await applying.value

        try expectEqual(cancellationCount, 0, "Caller cancellation must not abort a started confirmed local-create write.")
        try expectEqual(
            await runner.recordedCalls(method: "skillManager.applyLocalCreate").count,
            1,
            "Confirmed local-create write must run exactly once."
        )
        try expectEqual(store.isApplyingSkillManagerMutation, false, "Local-create apply state should converge after the owned write finishes.")
    }

    private func localDeleteApplyConvergesAfterCallerCancellation() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        let store = makeStore(runner)
        await store.previewSkillManagerLocalDelete(skill: localSkill(id: "owned-delete"))
        guard let confirmation = store.skillManagerLocalDeleteConfirmation else {
            throw NativeModelTestFailure(description: "Local-delete confirmation should exist before apply cancellation test.")
        }
        await runner.suspendCancellable("skillManager.deleteLocal")

        let applying = Task { await store.applySkillManagerLocalDelete(confirmation: confirmation) }
        try await waitForPending("skillManager.deleteLocal", runner: runner)
        applying.cancel()
        try? await Task.sleep(nanoseconds: 100_000_000)
        let cancellationCount = await runner.totalCancellationCount()
        if cancellationCount == 0 {
            await runner.resumeSuccess("skillManager.deleteLocal")
        }
        await applying.value

        try expectEqual(cancellationCount, 0, "Caller cancellation must not abort a started confirmed local-delete write.")
        let applyCalls = await runner.recordedCalls(method: "skillManager.deleteLocal").filter(\.confirmed)
        try expectEqual(applyCalls.count, 1, "Confirmed local-delete write must run exactly once.")
        try expectEqual(store.isApplyingSkillManagerMutation, false, "Local-delete apply state should converge after the owned write finishes.")
    }

    private func applyUsesExactPreviewInputsAndToken() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        let store = makeStore(runner)
        store.skillManagerSource = " owner/repo "
        store.skillManagerInstallSkillName = "beta, alpha, alpha"
        store.skillManagerSelectedAgentIDs = ["codex", "claude-code"]
        store.skillManagerScope = .project
        store.skillManagerDistribution = .symlink
        store.skillManagerNetworkAllowed = false

        await store.previewSkillManagerInstall()
        guard let confirmation = store.skillManagerMutationConfirmation else {
            throw NativeModelTestFailure(description: "Install preview should create an immutable confirmation.")
        }
        store.skillManagerSearchQuery = "unrelated live edit"
        store.skillManagerOwner = "unrelated-owner"
        await store.applySkillManagerInstall(confirmation: confirmation)

        guard let apply = await runner.recordedCalls(method: "skillManager.applyInstall").last else {
            throw NativeModelTestFailure(description: "Install apply should reach the service.")
        }
        try expectEqual(apply.source, confirmation.inputs.source, "Apply should reuse the preview source.")
        try expectEqual(apply.skills, confirmation.inputs.skills, "Apply should reuse canonical preview skills.")
        try expectEqual(apply.agents, confirmation.inputs.agents, "Apply should reuse canonical preview agents.")
        try expectEqual(apply.scope, confirmation.inputs.scope.rawValue, "Apply should reuse preview scope.")
        try expectEqual(apply.distribution, nil, "Symlink distribution should preserve the service's omitted wire value.")
        try expectEqual(apply.networkAllowed, confirmation.inputs.networkAllowed, "Apply should reuse preview network posture.")
        try expectEqual(apply.previewToken, confirmation.previewToken, "Apply should reuse the exact preview token.")
        try expectEqual(apply.confirmed, true, "Apply should remain explicitly confirmed.")
    }

    private func oldCompletionDoesNotClearCurrentLoadingState() async throws {
        let runner = SkillManagerGenerationServiceRunner()
        await runner.suspend("search:first")
        await runner.suspend("search:second")
        let store = makeStore(runner)

        store.skillManagerSearchQuery = "first"
        let first = Task { await store.searchSkillManager() }
        try await waitForPending("search:first", runner: runner)
        store.skillManagerSearchQuery = "second"
        let second = Task { await store.searchSkillManager() }
        try await waitForPending("search:second", runner: runner)

        await runner.resumeSuccess("search:first")
        await first.value
        try expectEqual(store.isSearchingSkillManager, true, "An old completion must not clear the current request's loading state.")
        await runner.resumeSuccess("search:second")
        await second.value
        try expectEqual(store.isSearchingSkillManager, false, "The current completion should clear its own loading state.")
    }

    private func makeStore(_ runner: SkillManagerGenerationServiceRunner) -> SkillStore {
        SkillStore(
            service: ServiceClient(
                processRunner: runner,
                serviceURL: URL(fileURLWithPath: "/tmp/fake-skill-manager-service")
            )
        )
    }

    private func localSkill(id: String) -> SkillRecord {
        SkillRecord(
            id: id,
            agent: "tool-global",
            scope: "tool-global",
            path: "/tmp/fixture/\(id)/SKILL.md",
            displayPath: "Tool Pool/\(id)/SKILL.md",
            definitionId: "tool:\(id)",
            name: id,
            state: "loaded",
            enabled: true
        )
    }

    private func waitForPending(
        _ label: String,
        runner: SkillManagerGenerationServiceRunner,
        timeout: TimeInterval = 2
    ) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while !(await runner.hasPending(label)) {
            if Date() > deadline {
                throw NativeModelTestFailure(description: "Timed out waiting for \(label).")
            }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
    }

    private func waitForCancellationCount(
        _ expected: Int,
        runner: SkillManagerGenerationServiceRunner,
        timeout: TimeInterval = 0.5
    ) async -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while await runner.totalCancellationCount() < expected {
            if Date() > deadline {
                return false
            }
            try? await Task.sleep(nanoseconds: 10_000_000)
        }
        return true
    }

    private func waitForCompletion(
        _ completion: SkillManagerRequestCompletionFlag,
        timeout: TimeInterval = 0.5
    ) async -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while !(await completion.isFinished()) {
            if Date() > deadline {
                return false
            }
            try? await Task.sleep(nanoseconds: 10_000_000)
        }
        return true
    }
}

private actor SkillManagerRequestCompletionFlag {
    private var finished = false

    func markFinished() {
        finished = true
    }

    func isFinished() -> Bool {
        finished
    }
}

private struct RecordedSkillManagerCall: Sendable, Equatable {
    let method: String
    let query: String?
    let owner: String?
    let source: String?
    let skills: [String]
    let agents: [String]
    let scope: String?
    let distribution: String?
    let networkAllowed: Bool
    let confirmed: Bool
    let previewToken: String?
    let name: String?
    let instanceID: String?
}

private actor SkillManagerGenerationServiceRunner: ServiceProcessRunning {
    private struct PendingRequest {
        let call: RecordedSkillManagerCall
        let continuation: CheckedContinuation<Data, Error>
    }

    private var calls: [RecordedSkillManagerCall] = []
    private var suspendedLabels: Set<String> = []
    private var suspendNextLabels: Set<String> = []
    private var cancellationAwareLabels: Set<String> = []
    private var cancellationCounts: [String: Int] = [:]
    private var pending: [String: PendingRequest] = [:]

    func suspend(_ label: String) {
        suspendedLabels.insert(label)
    }

    func suspendNext(_ label: String) {
        suspendedLabels.insert(label)
        suspendNextLabels.insert(label)
    }

    func suspendCancellable(_ label: String) {
        suspendedLabels.insert(label)
        cancellationAwareLabels.insert(label)
    }

    func hasPending(_ label: String) -> Bool {
        pending[label] != nil
    }

    func resumeSuccess(_ label: String) {
        guard let request = pending.removeValue(forKey: label) else { return }
        suspendedLabels.remove(label)
        cancellationAwareLabels.remove(label)
        request.continuation.resume(returning: Self.response(for: request.call))
    }

    func resumeServiceError(_ label: String) {
        guard let request = pending.removeValue(forKey: label) else { return }
        suspendedLabels.remove(label)
        cancellationAwareLabels.remove(label)
        request.continuation.resume(returning: Self.serviceErrorResponse)
    }

    func totalCancellationCount() -> Int {
        cancellationCounts.values.reduce(0, +)
    }

    func recordedCalls(method: String) -> [RecordedSkillManagerCall] {
        calls.filter { $0.method == method }
    }

    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) async throws -> Data {
        let call = try Self.decodeCall(input)
        calls.append(call)
        let label = Self.label(for: call)
        if suspendedLabels.contains(label) {
            if suspendNextLabels.remove(label) != nil {
                suspendedLabels.remove(label)
            }
            if cancellationAwareLabels.contains(label) {
                return try await withTaskCancellationHandler {
                    try await withCheckedThrowingContinuation { continuation in
                        pending[label] = PendingRequest(call: call, continuation: continuation)
                    }
                } onCancel: {
                    Task { await self.cancelPending(label) }
                }
            }
            return try await withCheckedThrowingContinuation { continuation in
                pending[label] = PendingRequest(call: call, continuation: continuation)
            }
        }
        return Self.response(for: call)
    }

    private func cancelPending(_ label: String) {
        guard let request = pending.removeValue(forKey: label) else { return }
        suspendedLabels.remove(label)
        cancellationAwareLabels.remove(label)
        cancellationCounts[label, default: 0] += 1
        request.continuation.resume(throwing: CancellationError())
    }

    private static func decodeCall(_ data: Data) throws -> RecordedSkillManagerCall {
        guard let object = try JSONSerialization.jsonObject(with: data) as? [String: Any],
              let method = object["method"] as? String,
              let params = object["params"] as? [String: Any] else {
            throw NativeModelTestFailure(description: "Skill Manager test runner received malformed JSON.")
        }
        return RecordedSkillManagerCall(
            method: method,
            query: params["query"] as? String,
            owner: params["owner"] as? String,
            source: params["source"] as? String,
            skills: params["skills"] as? [String] ?? ((params["skill"] as? String).map { [$0] } ?? []),
            agents: params["agents"] as? [String] ?? [],
            scope: params["scope"] as? String,
            distribution: params["distribution"] as? String,
            networkAllowed: params["network_allowed"] as? Bool ?? false,
            confirmed: params["confirmed"] as? Bool ?? false,
            previewToken: params["preview_token"] as? String,
            name: params["name"] as? String,
            instanceID: params["instance_id"] as? String
        )
    }

    private static func label(for call: RecordedSkillManagerCall) -> String {
        switch call.method {
        case "skillManager.search":
            return "search:\(call.query ?? "")"
        case "skillManager.listInstalled":
            return "installed:\(call.scope ?? ""):\(call.agents.joined(separator: ","))"
        case "skillManager.previewInstall":
            return "mutation:install:\(call.source ?? "")"
        case "skillManager.previewLocalCreate":
            return "local-create:\(call.name ?? "")"
        case "skillManager.deleteLocal" where !call.confirmed:
            return "local-delete:\(call.instanceID ?? "")"
        default:
            return call.method
        }
    }

    private static func response(for call: RecordedSkillManagerCall) -> Data {
        let result: Any
        switch call.method {
        case "skillManager.search":
            let query = call.query ?? ""
            result = [
                "preview": preview(operation: "search", token: "search:\(query)", source: nil, skills: []),
                "output": NSNull(),
                "results": [["name": query, "source": "fixture/\(query)", "description": "fixture", "raw": [:]]]
            ]
        case "skillManager.listInstalled":
            let name = "\(call.scope ?? "none"):\(call.agents.joined(separator: ","))"
            result = [
                "preview": preview(operation: "listInstalled", token: "installed:\(name)", source: nil, skills: []),
                "output": output,
                "installed": [["name": name, "source": "fixture", "agents": call.agents, "scope": call.scope ?? "", "path": "/tmp/fixture", "raw": [:]]]
            ]
        case "skillManager.previewInstall", "skillManager.applyInstall":
            result = mutationResponse(call: call, operation: "install")
        case "skillManager.previewRemove", "skillManager.applyRemove":
            result = mutationResponse(call: call, operation: "remove")
        case "skillManager.previewUpdate", "skillManager.applyUpdate":
            result = mutationResponse(call: call, operation: "update")
        case "skillManager.previewLocalCreate", "skillManager.applyLocalCreate":
            let name = call.name ?? ""
            result = [
                "preview": preview(operation: "localCreate", token: "local-create:\(name)", source: nil, skills: [name]),
                "output": NSNull(),
                "imported": NSNull(),
                "instance_id": "local:\(name)",
                "source_path": "/tmp/fixture/\(name)",
                "applied": call.confirmed
            ]
        case "skillManager.deleteLocal":
            let instanceID = call.instanceID ?? ""
            result = [
                "instance_id": instanceID,
                "skill_name": instanceID,
                "path": "/tmp/fixture/\(instanceID)",
                "app_owned": true,
                "physical_delete_allowed": true,
                "blocked_by_references": [],
                "confirmed": call.confirmed,
                "deleted": call.confirmed,
                "summary": "fixture local delete"
            ]
        case "app.stateSnapshot":
            result = [
                "status": [
                    "protocol_version": 2,
                    "version": "test",
                    "app_data_dir": "/tmp/app-data",
                    "catalog_path": "/tmp/catalog",
                    "user_home": "/tmp/home",
                    "supported_methods": []
                ],
                "skills": [],
                "findings": [],
                "conflicts": [],
                "snapshots": []
            ]
        default:
            return serviceErrorResponse
        }
        return envelope(result: result)
    }

    private static func mutationResponse(call: RecordedSkillManagerCall, operation: String) -> [String: Any] {
        let token = call.confirmed ? (call.previewToken ?? "missing-token") : "preview:\(operation):\(call.source ?? call.skills.joined(separator: ","))"
        return [
            "preview": preview(operation: operation, token: token, source: call.source, skills: call.skills),
            "output": NSNull(),
            "applied": call.confirmed,
            "scanned_count": 0,
            "updated_skills": []
        ]
    }

    private static func preview(
        operation: String,
        token: String,
        source: String?,
        skills: [String]
    ) -> [String: Any] {
        [
            "tool_id": "npx-skills",
            "operation": operation,
            "command": ["npx", "skills", operation],
            "cwd": "/tmp/project",
            "env": [],
            "requires_confirmation": ["install", "remove", "update", "localCreate"].contains(operation),
            "confirmed": false,
            "network_required": ["search", "install", "update"].contains(operation),
            "network_allowed": true,
            "will_run": false,
            "preview_token": token,
            "summary": "fixture \(operation)",
            "risks": [],
            "source": source ?? NSNull(),
            "skills": skills
        ]
    }

    private static let output: [String: Any] = [
        "status": "ok",
        "exit_code": 0,
        "stdout": "",
        "stderr": ""
    ]

    private static func envelope(result: Any) -> Data {
        try! JSONSerialization.data(withJSONObject: ["id": "test", "ok": true, "result": result])
    }

    private static let serviceErrorResponse = try! JSONSerialization.data(
        withJSONObject: [
            "id": "test",
            "ok": false,
            "error": ["code": "test.error", "message": "stale failure"]
        ]
    )
}
