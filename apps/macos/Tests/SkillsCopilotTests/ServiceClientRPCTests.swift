import Foundation
@testable import SkillsCopilot

struct ServiceClientRPCTests {
    func run() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "prompt-ready")

        let client = fake.serviceClient()

        let findings = try await client.listFindings()
        try expectEqual(findings.count, 0, "Catalog/config RPC wrapper should decode listFindings.")

        let sessions = try await client.previewLocalSessions(
            authorizedRoots: [],
            agent: "codex",
            scope: .all,
            search: "release",
            project: nil,
            limit: 3
        )
        try expectEqual(sessions.isUnavailable, true, "Session RPC wrapper should map unknown methods to unavailable.")

        let observability = try await client.providerObservability()
        try expectEqual(observability.generatedBy, "local-v2.64", "LLM RPC wrapper should decode provider observability.")

        let calls = fake.calls()
        try expectContains(calls, "catalog.listFindings", "Catalog/config wrapper should call the catalog method.")
        try expectContains(calls, "session.previewLocalSessions", "Session wrapper should call the session method.")
        try expectContains(calls, #""auto_discover":true"#, "Session preview should request auto-discovery when no roots are supplied.")
        try expectContains(calls, "llm.providerObservability", "LLM wrapper should call the observability method.")

        try await configConsistencyRequestsUseExactBindings()
        try legacyConfigResponsesAreReadOnly()
        try unrelatedWritesDoNotGainConfigCASFields()
        try await taskCockpitProviderCallsUseFiveMinuteSidecarTimeout()
    }

    private func configConsistencyRequestsUseExactBindings() async throws {
        let runner = RecordingServiceProcessRunner()
        let client = ServiceClient(processRunner: runner, serviceURL: URL(fileURLWithPath: "/tmp/fake-service"))

        let saved = try await client.saveClaudeSettings(
            content: "{\"enabled\":true}\n",
            expectedRevision: "sha256:before"
        )
        let preview = try await client.previewSnapshotRollback(snapshotID: "snap-claude-new")
        let rolledBack = try await client.rollbackSnapshot(
            snapshotID: preview.snapshot.id,
            previewToken: preview.previewToken ?? ""
        )

        try expectEqual(saved.revision, Optional("sha256:after"), "Config save should decode the returned revision.")
        try expectEqual(preview.currentRevision, Optional("sha256:current"), "Rollback preview should decode current_revision.")
        try expectEqual(preview.previewToken, Optional("rollback:token-1"), "Rollback preview should decode preview_token.")
        try expectEqual(rolledBack, 3, "Rollback should decode the scanned count.")

        let saveParams = try runner.params(for: "config.saveClaudeSettings")
        try expectEqual(Set(saveParams.keys), Set(["content", "expected_revision"]), "Config save request should contain only content and expected_revision.")
        try expectEqual(saveParams["content"] as? String, Optional("{\"enabled\":true}\n"), "Config save should preserve exact content.")
        try expectEqual(saveParams["expected_revision"] as? String, Optional("sha256:before"), "Config save should send the loaded revision.")

        let rollbackParams = try runner.params(for: "snapshot.rollback")
        try expectEqual(Set(rollbackParams.keys), Set(["snapshot_id", "preview_token"]), "Rollback request should contain only snapshot_id and preview_token.")
        try expectEqual(rollbackParams["snapshot_id"] as? String, Optional("snap-claude-new"), "Rollback should send the previewed snapshot id.")
        try expectEqual(rollbackParams["preview_token"] as? String, Optional("rollback:token-1"), "Rollback should send the opaque preview token.")
        try expectNil(rollbackParams["expected_revision"], "A bare revision must never be sent as rollback authorization.")
    }

    private func legacyConfigResponsesAreReadOnly() throws {
        let legacyDocument = try JSONDecoder().decode(
            ConfigDocumentRecord.self,
            from: Data(#"{"agent":"claude-code","scope":"agent-global","target":"/tmp/settings.json","format":"json","content":"{}\n","exists":true}"#.utf8)
        )
        try expectNil(legacyDocument.revision, "A legal legacy config response should decode without a revision.")
        try expectFalse(legacyDocument.supportsCompareAndSwap, "A config response without a revision must remain read-only.")

        let legacyPreview = try JSONDecoder().decode(
            SnapshotRollbackPreviewRecord.self,
            from: Data(#"{"snapshot":{"id":"snap-legacy","agent":"claude-code","scope":"agent-global","target":"/tmp/settings.json","content":"{}\n","reason":"legacy","created_at":1},"current_content":"{}\n","current_read_error":null,"changed":false,"redacted":false,"rollback_supported":true}"#.utf8)
        )
        try expectNil(legacyPreview.previewToken, "A legal legacy rollback preview should decode without a token.")
        try expectNil(legacyPreview.currentRevision, "A legal legacy rollback preview should decode without a current revision.")
        try expectFalse(legacyPreview.rollbackSupported, "A preview missing protocol-v2 bindings must remain read-only.")
    }

    private func unrelatedWritesDoNotGainConfigCASFields() throws {
        let toggle = try encodedObject(ToggleSkillParams(instanceId: "skill-1", on: true))
        try expectEqual(Set(toggle.keys), Set(["instance_id", "on"]), "Single-skill toggle should keep its existing request contract.")
        try expectNil(toggle["expected_revision"], "Single-skill toggle must not gain config-save CAS fields.")

        let batch = try encodedObject(BatchToggleParams(
            instanceIDs: ["skill-1"],
            targetEnabled: false,
            action: "apply",
            previewToken: "batch:preview",
            confirmed: true
        ))
        try expectEqual(
            Set(batch.keys),
            Set(["instance_ids", "target_enabled", "action", "preview_token", "confirmed"]),
            "Batch toggle should keep its existing preview-token request contract."
        )
        try expectNil(batch["expected_revision"], "Batch toggle must not gain config-save CAS fields.")
        try expectNil(batch["current_revision"], "Batch toggle must not send rollback revision fields.")
    }

    private func encodedObject<Value: Encodable>(_ value: Value) throws -> [String: Any] {
        guard let object = try JSONSerialization.jsonObject(with: JSONEncoder().encode(value)) as? [String: Any] else {
            throw NativeModelTestFailure(description: "Expected encoded params to be a JSON object.")
        }
        return object
    }

    private func taskCockpitProviderCallsUseFiveMinuteSidecarTimeout() async throws {
        let runner = RecordingServiceProcessRunner()
        let client = ServiceClient(processRunner: runner, serviceURL: URL(fileURLWithPath: "/tmp/fake-service"))

        _ = try await client.previewPromptForTaskCockpit(
            taskText: "查看下阿里云 ALB 指标与错误情况",
            agents: ["claude-code", "codex"],
            instanceIDs: ["alb-skill"]
        )
        _ = try await client.confirmPromptAndSendForTaskCockpit(
            previewID: "prompt-preview-task",
            taskText: "查看下阿里云 ALB 指标与错误情况",
            agents: ["claude-code", "codex"],
            instanceIDs: ["alb-skill"]
        )

        try expectEqual(
            runner.timeoutMilliseconds,
            [300_000, 300_000],
            "Task Preflight provider preview and send should use the five-minute sidecar timeout."
        )
        try expectEqual(
            runner.methods,
            ["llm.previewPrompt", "llm.confirmPromptAndSend"],
            "Task Preflight should use the provider preview and confirmation methods."
        )
    }
}

private final class RecordingServiceProcessRunner: ServiceProcessRunning {
    private(set) var methods: [String] = []
    private(set) var timeoutMilliseconds: [Int?] = []
    private(set) var requests: [[String: Any]] = []

    func params(for method: String) throws -> [String: Any] {
        guard let request = requests.first(where: { $0["method"] as? String == method }),
              let params = request["params"] as? [String: Any] else {
            throw NativeModelTestFailure(description: "Missing recorded request for \(method).")
        }
        return params
    }

    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) async throws -> Data {
        timeoutMilliseconds.append(timeoutNanoseconds.map { Int($0 / 1_000_000) })

        let object = try JSONSerialization.jsonObject(with: input) as? [String: Any]
        if let object {
            requests.append(object)
        }
        let method = object?["method"] as? String ?? ""
        methods.append(method)

        switch method {
        case "config.saveClaudeSettings":
            return Data(Self.configSaveResponse.utf8)
        case "snapshot.previewRollback":
            return Data(Self.rollbackPreviewResponse.utf8)
        case "snapshot.rollback":
            return Data(Self.rollbackResponse.utf8)
        case "llm.previewPrompt":
            return Data(Self.previewResponse.utf8)
        case "llm.confirmPromptAndSend":
            return Data(Self.sendResponse.utf8)
        default:
            return Data(Self.unknownMethodResponse.utf8)
        }
    }

    private static let previewResponse = """
    {"id":"test","ok":true,"result":{"preview_id":"prompt-preview-task","request_kind":"task_cockpit","scope":"agents","prompt_scope":"Task Preflight","enabled":true,"provider":"openai-compatible","model":"gpt-test","destination_host":"llm.example.com","included_fields":[],"excluded_fields":[],"redaction":{"status":"redacted","summary":"ok","redacted_fields":[],"placeholders":[]},"confirmation_required":true,"raw_prompt_persisted":false,"raw_response_persisted":false,"draft_copy_only":true,"redacted_prompt_preview":"preview"}}
    """

    private static let configSaveResponse = """
    {"id":"test","ok":true,"result":{"agent":"claude-code","scope":"agent-global","target":"/tmp/settings.json","format":"json","content":"{\\"enabled\\":true}\\n","exists":true,"revision":"sha256:after"}}
    """

    private static let rollbackPreviewResponse = """
    {"id":"test","ok":true,"result":{"snapshot":{"id":"snap-claude-new","agent":"claude-code","scope":"agent-global","target":"/tmp/settings.json","content":"{}\\n","reason":"test","created_at":1},"current_content":"{\\"enabled\\":true}\\n","current_read_error":null,"current_revision":"sha256:current","preview_token":"rollback:token-1","changed":true,"redacted":false,"rollback_supported":true}}
    """

    private static let rollbackResponse = """
    {"id":"test","ok":true,"result":3}
    """

    private static let sendResponse = """
    {"id":"test","ok":true,"result":{"preview_id":"prompt-preview-task","success":true,"status":"succeeded","message":"Provider response received.","output_text":"{}","draft_copy_only":true,"raw_prompt_persisted":false,"raw_response_persisted":false,"write_back_allowed":false,"script_execution_allowed":false}}
    """

    private static let unknownMethodResponse = """
    {"id":"test","ok":false,"error":{"code":"unknown_method","message":"unknown method"}}
    """
}
