@testable import SkillsCopilot

struct AgentConfigTimelineModelTests {
    func run() throws {
        try timelineShowsOnlySelectedAgentSnapshots()
        try allAgentsDoesNotMixRollbackPoints()
        try delayedRollbackPreviewSuccessDoesNotReplaceNewSelection()
        try delayedRollbackPreviewErrorDoesNotReplaceNewSelection()
        try newestRollbackPreviewWinsOutOfOrderCompletion()
        try disappearingSnapshotInvalidatesRollbackPreviewRequest()
        try rollbackFailureReplacesLoadedPreviewWithError()
        try staleRollbackFailureDoesNotReplaceNewSelection()
        try visibleConfigAlwaysAllowsHide()
        try hiddenConfigRejectsRevealWithoutBinding()
        try hiddenConfigRejectsRevealWhileBusy()
        try hiddenConfigAllowsRevealWithValidIdleBinding()
    }

    private func timelineShowsOnlySelectedAgentSnapshots() throws {
        let snapshots = [
            snapshot(id: "old-claude", agent: "claude-code", target: "/tmp/claude/settings.json", reason: "toggle beta", createdAt: 10),
            snapshot(id: "new-codex", agent: "codex", target: "/tmp/project/.codex/config.toml", reason: "disable gamma", createdAt: 30),
            snapshot(id: "old-codex", agent: "codex", target: "/tmp/codex/config.toml", reason: "", createdAt: 20),
            snapshot(id: "older-codex", agent: "codex", target: "/tmp/codex/older.toml", reason: "older", createdAt: 15),
        ]

        let model = AgentConfigTimelineModel.make(snapshots: snapshots, agentFilter: .codex, limit: 2)

        try expectEqual(model.isSpecificAgent, true, "Timeline should be active for a specific agent.")
        try expectEqual(model.agentTitle, UIStrings.codex, "Timeline should use selected agent display name.")
        try expectEqual(model.items.map(\.id), ["new-codex", "old-codex"], "Timeline should sort newest first and keep only the selected agent.")
        try expectEqual(model.hiddenCount, 1, "Timeline should keep older entries out of the compact sidebar.")
        try expectEqual(model.items[0].targetSummary, ".../.codex/config.toml", "Timeline should summarize config target paths.")
        try expectEqual(model.items[1].actionText, UIStrings.agentConfigTimelineDefaultAction, "Empty reasons should fall back to a stable action label.")
        try expectEqual(model.items[0].statusText, UIStrings.agentConfigTimelineStatus, "Timeline rows should expose a visible rollback-point status.")
    }

    private func allAgentsDoesNotMixRollbackPoints() throws {
        let model = AgentConfigTimelineModel.make(
            snapshots: [
                snapshot(id: "claude", agent: "claude-code", target: "/tmp/claude/settings.json", createdAt: 10),
                snapshot(id: "codex", agent: "codex", target: "/tmp/codex/config.toml", createdAt: 20),
            ],
            agentFilter: .all
        )

        try expectEqual(model.isSpecificAgent, false, "All Agents should not expose a mixed config timeline.")
        try expectEqual(model.items.count, 0, "All Agents should not mix rollback points from different agents.")
        try expectEqual(model.summaryText, UIStrings.agentConfigTimelineSelectAgent, "All Agents should ask for one selected agent.")
    }

    private func delayedRollbackPreviewSuccessDoesNotReplaceNewSelection() throws {
        var state = RollbackPreviewPresentationState<String>()
        let request = state.begin(snapshotID: "snapshot-a")

        state.invalidate(selectedSnapshotID: "snapshot-b")
        let published = state.publish(preview: "preview-a", for: request)

        try expectFalse(published, "A delayed success for snapshot A must not publish after selecting snapshot B.")
        try expectNil(state.preview, "Snapshot B must not display snapshot A's delayed preview.")
        try expectNil(state.errorMessage, "Rejecting a delayed success must keep snapshot B's error state clear.")
    }

    private func delayedRollbackPreviewErrorDoesNotReplaceNewSelection() throws {
        var state = RollbackPreviewPresentationState<String>()
        let request = state.begin(snapshotID: "snapshot-a")

        state.invalidate(selectedSnapshotID: "snapshot-b")
        let published = state.publish(errorMessage: "snapshot A failed", for: request)

        try expectFalse(published, "A delayed error for snapshot A must not publish after selecting snapshot B.")
        try expectNil(state.preview, "Rejecting a delayed error must keep snapshot B's preview clear.")
        try expectNil(state.errorMessage, "Snapshot B must not display snapshot A's delayed error.")
    }

    private func newestRollbackPreviewWinsOutOfOrderCompletion() throws {
        var state = RollbackPreviewPresentationState<String>()
        let firstRequest = state.begin(snapshotID: "snapshot-a")
        let secondRequest = state.begin(snapshotID: "snapshot-a")

        let secondPublished = state.publish(preview: "newest-preview", for: secondRequest)
        let firstPublished = state.publish(preview: "stale-preview", for: firstRequest)

        try expectEqual(secondPublished, true, "The newest request should publish for the selected snapshot.")
        try expectFalse(firstPublished, "An older request must not overwrite a newer completed preview.")
        try expectEqual(state.preview, Optional("newest-preview"), "Out-of-order completion should keep only the newest preview.")
        try expectNil(state.errorMessage, "A successful newest preview should keep the error state clear.")
    }

    private func disappearingSnapshotInvalidatesRollbackPreviewRequest() throws {
        var state = RollbackPreviewPresentationState<String>()
        let request = state.begin(snapshotID: "snapshot-a")

        state.invalidate(selectedSnapshotID: nil)
        let published = state.publish(preview: "preview-after-disappear", for: request)

        try expectFalse(published, "A preview response must not publish after its snapshot view disappears.")
        try expectNil(state.preview, "A disappeared snapshot view must keep preview state clear.")
        try expectNil(state.errorMessage, "A disappeared snapshot view must keep error state clear.")
    }

    private func rollbackFailureReplacesLoadedPreviewWithError() throws {
        var state = RollbackPreviewPresentationState<String>()
        let request = state.begin(snapshotID: "snapshot-a")
        _ = state.publish(preview: "loaded-preview", for: request)

        state.replaceWithError("preview again", selectedSnapshotID: "snapshot-a")

        try expectNil(state.preview, "A failed rollback must clear the preview that authorized it.")
        try expectEqual(state.errorMessage, Optional("preview again"), "A failed rollback should publish its recovery message.")
        try expectNil(state.activeRequest, "A failed rollback must leave no preview request active.")
    }

    private func staleRollbackFailureDoesNotReplaceNewSelection() throws {
        var state = RollbackPreviewPresentationState<String>()
        let request = state.begin(snapshotID: "snapshot-a")
        _ = state.publish(preview: "loaded-preview", for: request)
        state.invalidate(selectedSnapshotID: "snapshot-b")

        state.replaceWithError("snapshot A rollback failed", selectedSnapshotID: "snapshot-a")

        try expectEqual(state.selectedSnapshotID, Optional("snapshot-b"), "A stale rollback failure must not rebind presentation state to snapshot A.")
        try expectNil(state.preview, "Snapshot B should stay clear after snapshot A's rollback failure.")
        try expectNil(state.errorMessage, "Snapshot B must not display snapshot A's rollback failure.")
    }

    private func visibleConfigAlwaysAllowsHide() throws {
        let missingBinding = AgentConfigSensitiveTogglePolicy(
            isSensitiveVisible: true,
            hasLoadedDocument: true,
            hasWritableBinding: false,
            isLoading: false,
            isSaving: false
        )
        let loading = AgentConfigSensitiveTogglePolicy(
            isSensitiveVisible: true,
            hasLoadedDocument: true,
            hasWritableBinding: true,
            isLoading: true,
            isSaving: false
        )
        let saving = AgentConfigSensitiveTogglePolicy(
            isSensitiveVisible: true,
            hasLoadedDocument: true,
            hasWritableBinding: true,
            isLoading: false,
            isSaving: true
        )

        try expectFalse(missingBinding.isDisabled, "Visible config must allow Hide when its binding disappears.")
        try expectFalse(loading.isDisabled, "Visible config must allow Hide while a read is loading.")
        try expectFalse(saving.isDisabled, "Visible config must allow Hide while a save is running.")
    }

    private func hiddenConfigRejectsRevealWithoutBinding() throws {
        let policy = AgentConfigSensitiveTogglePolicy(
            isSensitiveVisible: false,
            hasLoadedDocument: true,
            hasWritableBinding: false,
            isLoading: false,
            isSaving: false
        )

        try expectEqual(policy.isDisabled, true, "Hidden loaded config without a protocol-v2 binding must disable Reveal.")
    }

    private func hiddenConfigRejectsRevealWhileBusy() throws {
        let loading = AgentConfigSensitiveTogglePolicy(
            isSensitiveVisible: false,
            hasLoadedDocument: true,
            hasWritableBinding: true,
            isLoading: true,
            isSaving: false
        )
        let saving = AgentConfigSensitiveTogglePolicy(
            isSensitiveVisible: false,
            hasLoadedDocument: true,
            hasWritableBinding: true,
            isLoading: false,
            isSaving: true
        )

        try expectEqual(loading.isDisabled, true, "Hidden config must disable Reveal while loading.")
        try expectEqual(saving.isDisabled, true, "Hidden config must disable Reveal while saving.")
    }

    private func hiddenConfigAllowsRevealWithValidIdleBinding() throws {
        let policy = AgentConfigSensitiveTogglePolicy(
            isSensitiveVisible: false,
            hasLoadedDocument: true,
            hasWritableBinding: true,
            isLoading: false,
            isSaving: false
        )

        try expectFalse(policy.isDisabled, "Hidden config with a valid protocol-v2 binding should allow Reveal while idle.")
    }

    private func snapshot(
        id: String,
        agent: String,
        target: String,
        reason: String = "config write",
        createdAt: Int64
    ) -> ConfigSnapshotRecord {
        ConfigSnapshotRecord(
            id: id,
            agent: agent,
            scope: "agent-global",
            target: target,
            content: "{}",
            reason: reason,
            createdAt: createdAt
        )
    }
}
