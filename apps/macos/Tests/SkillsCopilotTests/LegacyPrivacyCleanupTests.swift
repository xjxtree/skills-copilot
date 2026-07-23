import Foundation
@testable import SkillsCopilot

@MainActor
struct LegacyPrivacyCleanupTests {
    func run() async throws {
        try modelDecodesContentFreeInspectionAndConfirmation()
        try await startupInspectionIsReadOnly()
        try await previewCancellationDoesNotApply()
        try await explicitPreviewAndConfirmationApplyCleanup()
    }

    private func modelDecodesContentFreeInspectionAndConfirmation() throws {
        let data = Data(
            """
            {
              "inspection": {
                "generated_by": "local-v2.64",
                "cleanup_required": true,
                "cleanup_source_count": 1,
                "existing_source_count": 1,
                "sources": [{
                  "id": "prompt-runs",
                  "source_file": "prompt-runs.json",
                  "item_type": "regular_file",
                  "state": "legacy_private_content",
                  "cleanup_operation": "sanitize_metadata",
                  "cleanup_required": true,
                  "malformed": false,
                  "generated_residue": false
                }],
                "read_only": true,
                "provider_request_sent": false,
                "raw_content_returned": false,
                "write_performed": false
              },
              "action": {
                "id": "privacy-action",
                "kind": "privacy_cleanup",
                "intent": "clean_legacy_private_content",
                "target": {"kind": "app_data", "id": "legacy-ai-private-content"},
                "impacts": ["app_local_data"],
                "preview_method": "privacy.previewCleanupLegacyContent",
                "apply_method": "privacy.cleanupLegacyContent",
                "source_revision": "sha256:privacy",
                "confirmation_required": true,
                "network": "none",
                "readback": ["private_content"],
                "evidence_refs": ["app-data:legacy-ai-private-content"]
              },
              "preconditions": [{
                "kind": "legacy_private_content",
                "target_id": "prompt-runs",
                "expected_revision": "opaque-revision"
              }],
              "preview_token": "opaque-token",
              "confirmation_required": true
            }
            """.utf8
        )

        let preview = try JSONDecoder().decode(
            LegacyPrivateContentCleanupPreview.self,
            from: data
        )

        try expectEqual(preview.inspection.cleanupSourceCount, 1, "Inspection should decode its bounded source count.")
        try expectEqual(preview.inspection.sources.first?.sourceFile, "prompt-runs.json", "Inspection should expose only the source name.")
        try expectEqual(preview.confirmation?.reference.actionID, "privacy-action", "Preview should construct the exact typed confirmation.")
        let encoded = String(decoding: data, as: UTF8.self)
        try expectFalse(encoded.contains("task_text"), "Privacy wire projection must not include task text.")
        try expectFalse(encoded.contains("draft_output"), "Privacy wire projection must not include provider output.")
    }

    private func startupInspectionIsReadOnly() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "privacy-cleanup")
        let store = SkillStore(service: fake.serviceClient())

        await store.loadAppStartupDataIfNeeded()
        try await waitUntil("Startup should publish the persistent cleanup-required projection.") {
            store.legacyPrivateContentInspection?.cleanupRequired == true
                && !store.isInspectingLegacyPrivateContent
        }

        let calls = fake.calls()
        try expectContains(calls, "\"method\":\"privacy.inspectLegacyContent\"", "Startup should inspect legacy content.")
        try expectFalse(calls.contains("\"method\":\"privacy.previewCleanupLegacyContent\""), "Startup must not create a cleanup preview.")
        try expectFalse(calls.contains("\"method\":\"privacy.cleanupLegacyContent\""), "Startup must not perform cleanup.")
        try expectEqual(store.taskCockpitHistoryCleanupMessage, UIStrings.legacyPrivateContentTaskPreflightWarning, "Task Preflight should surface the unified persistent warning.")
    }

    private func previewCancellationDoesNotApply() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "privacy-cleanup")
        let store = SkillStore(service: fake.serviceClient())

        await store.inspectLegacyPrivateContent()
        await store.previewLegacyPrivateContentCleanup()
        try expectEqual(store.legacyPrivateContentCleanupPreview?.confirmationRequired, true, "Explicit review should publish a confirmable preview.")
        store.cancelLegacyPrivateContentCleanupPreview()

        try expectNil(store.legacyPrivateContentCleanupPreview, "Cancel should discard the one-time preview.")
        try expectFalse(fake.calls().contains("\"method\":\"privacy.cleanupLegacyContent\""), "Cancel must not invoke cleanup.")
        try expectEqual(store.legacyPrivateContentInspection?.cleanupRequired, true, "Cancel should preserve the visible cleanup-required state.")
    }

    private func explicitPreviewAndConfirmationApplyCleanup() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "privacy-cleanup")
        let store = SkillStore(service: fake.serviceClient())

        await store.inspectLegacyPrivateContent()
        await store.previewLegacyPrivateContentCleanup()
        await store.confirmLegacyPrivateContentCleanup()

        try expectEqual(store.legacyPrivateContentInspection?.cleanupRequired, false, "Verified cleanup should publish the clean read-back.")
        try expectNil(store.legacyPrivateContentCleanupPreview, "Consumed cleanup confirmation must not remain reusable.")
        try expectNil(store.legacyPrivateContentCleanupError, "Verified cleanup should clear the persistent error.")
        let calls = fake.calls()
        try expectContains(calls, "\"method\":\"privacy.previewCleanupLegacyContent\"", "Cleanup should require a preview RPC.")
        try expectContains(calls, "\"method\":\"privacy.cleanupLegacyContent\"", "Explicit confirmation should invoke cleanup.")
        try expectContains(calls, "\"confirmed\":true", "Apply should carry an explicit typed confirmation.")
        try expectFalse(calls.contains("task text"), "Cleanup diagnostics must not contain task content.")
        try expectFalse(calls.contains("provider output"), "Cleanup diagnostics must not contain provider output.")
    }

    private func waitUntil(
        _ failureMessage: String,
        attempts: Int = 300,
        condition: @escaping @MainActor () -> Bool
    ) async throws {
        for _ in 0..<attempts {
            if condition() {
                return
            }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        throw NativeModelTestFailure(description: failureMessage)
    }
}
