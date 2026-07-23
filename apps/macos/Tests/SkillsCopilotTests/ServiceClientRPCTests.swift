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
            sessionID: nil,
            includeContentItems: false,
            limit: 3
        )
        try expectEqual(sessions.isUnavailable, true, "Session RPC wrapper should map unknown methods to unavailable.")

        _ = try await client.previewLocalSessions(
            authorizedRoots: [],
            agent: "codex",
            scope: .all,
            project: nil,
            sessionID: "session-alpha",
            includeContentItems: true,
            limit: 1
        )

        let observability = try await client.providerObservability()
        try expectEqual(observability.generatedBy, "local-v2.64", "LLM RPC wrapper should decode provider observability.")

        let calls = fake.calls()
        try expectContains(calls, "catalog.listFindings", "Catalog/config wrapper should call the catalog method.")
        try expectContains(calls, "session.previewLocalSessions", "Session wrapper should call the session method.")
        try expectContains(calls, #""auto_discover":true"#, "Session preview should request auto-discovery when no roots are supplied.")
        try expectContains(calls, #""include_content_items":false"#, "Summary RPC should explicitly omit content items.")
        try expectContains(calls, #""include_content_items":true"#, "Detail RPC should explicitly include content items.")
        try expectContains(calls, #""session_id":"session-alpha""#, "Detail RPC should send exactly one stable session id.")
        try expectContains(calls, "llm.providerObservability", "LLM wrapper should call the observability method.")

        try await configConsistencyRequestsUseExactBindings()
        try await localHistoryPageRequestsDecodeAliases()
        try await providerActivityPageRequestUsesExactBindings()
        try await localSessionCursorRequestDecodesCompleteness()
        try await localSessionMessagePageRequestUsesExactBindings()
        try await ruleSuppressionRequestsMatchServiceContract()
        try await nativeScriptPreviewUsesIdentityOnlyAndDecodesBlockedResponse()
        try legacyConfigResponsesFailClosed()
        try unrelatedWritesDoNotGainConfigCASFields()
        try actionReadbackMustMatchTheConfirmedDescriptor()
        try structuredPartialEffectDetailsDecodeForNativeRecovery()
        try await taskCockpitProviderCallsUseFiveMinuteSidecarTimeout()
    }

    private func structuredPartialEffectDetailsDecodeForNativeRecovery() throws {
        let payload = try JSONDecoder().decode(
            ServiceErrorPayload.self,
            from: Data(
                """
                {
                  "code": "partial_effect",
                  "message": "The manager applied a change whose read-back is incomplete.",
                  "details": {
                    "operation": "skillManager.applyUpdate",
                    "state": "applied_unverified",
                    "cleanup_required": true,
                    "retry_allowed": false
                  }
                }
                """.utf8
            )
        )

        try expectEqual(payload.details?.operation, "skillManager.applyUpdate", "Partial-effect operation")
        try expectEqual(payload.details?.state, "applied_unverified", "Partial-effect state")
        try expectEqual(payload.details?.cleanupRequired, true, "Partial-effect cleanup flag")
        try expectEqual(payload.details?.retryAllowed, false, "Partial-effect retry policy")
    }

    private func actionReadbackMustMatchTheConfirmedDescriptor() throws {
        let action = ActionDescriptorWire(
            id: "action:disable-skill:readback",
            kind: "toggle_skill",
            intent: "disable_skill",
            target: ActionTargetWire(
                kind: "skill",
                id: "skill-1",
                agent: "codex",
                scope: "agent-global"
            ),
            projectID: nil,
            impacts: ["agent_config"],
            previewMethod: "batch.previewSkillToggles",
            applyMethod: "batch.applySkillToggles",
            sourceRevision: "sha256:before",
            confirmationRequired: true,
            network: "none",
            readback: ["agent_config", "skill_aggregates"],
            evidenceRefs: ["skill:skill-1"]
        )
        let valid = ActionReadbackWire(
            actionID: action.id,
            sourceRevision: "sha256:after",
            projectID: nil,
            domains: action.readback,
            targetIDs: ["config:codex", "skill-1"],
            observations: [
                ActionReadbackObservationWire(
                    domain: "agent_config",
                    targetID: "config:codex",
                    revision: "sha256:config-after"
                ),
                ActionReadbackObservationWire(
                    domain: "skill_aggregates",
                    targetID: "skill-1",
                    revision: "sha256:catalog-after"
                ),
            ],
            verified: true
        )
        try valid.validated(for: action)

        let mismatched = ActionReadbackWire(
            actionID: "action:disable-skill:other",
            sourceRevision: valid.sourceRevision,
            projectID: nil,
            domains: valid.domains,
            targetIDs: valid.targetIDs,
            observations: valid.observations,
            verified: true
        )
        do {
            try mismatched.validated(for: action)
            throw NativeModelTestFailure(
                description: "A read-back for a different action must fail closed."
            )
        } catch ActionReadbackValidationError.actionMismatch {
            // Expected.
        }

        let duplicateObservation = ActionReadbackWire(
            actionID: valid.actionID,
            sourceRevision: valid.sourceRevision,
            projectID: valid.projectID,
            domains: valid.domains,
            targetIDs: valid.targetIDs,
            observations: valid.observations + [valid.observations[0]],
            verified: true
        )
        do {
            try duplicateObservation.validated(for: action)
            throw NativeModelTestFailure(
                description: "Duplicate read-back observations must fail closed."
            )
        } catch ActionReadbackValidationError.invalidObservation {
            // Expected.
        }
    }

    private func ruleSuppressionRequestsMatchServiceContract() async throws {
        let runner = RecordingServiceProcessRunner()
        let client = ServiceClient(processRunner: runner, serviceURL: URL(fileURLWithPath: "/tmp/fake-service"))

        let setRecord = try await client.setSuppression(
            ruleId: "dependency.unknown",
            scope: .rule,
            findingGroupId: nil,
            note: "Reviewed locally."
        )
        let listedRecords = try await client.listRuleTuning()
        _ = try await client.clearSuppression(
            ruleId: "dependency.unknown",
            scope: .rule,
            findingGroupId: nil
        )

        let setParams = try runner.params(for: "rules.setSuppression")
        try expectEqual(
            setParams["reason"] as? String,
            Optional("Suppressed locally in Agent Copilot after user review."),
            "Suppression requests must include the Rust-required reason."
        )
        try expectNil(setParams["suppressed"], "Suppression requests must not send an unsupported suppressed flag.")
        try expectNil(setParams["finding_group_id"], "Rule-wide suppression must not send an unsupported finding group.")
        try expectNil(setParams["scope"], "Native rule-wide suppression must not overload adapter scope.")
        try expectEqual(setRecord?.suppressed, true, "A suppression reason in the canonical response must decode as suppressed.")
        try expectEqual(
            listedRecords.first?.suppressed,
            true,
            "Suppression read-back must preserve the state encoded by suppression_reason."
        )

        let clearParams = try runner.params(for: "rules.clearSuppression")
        try expectNil(clearParams["finding_group_id"], "Clear suppression must use the Rust rule tuning key.")
        try expectNil(clearParams["scope"], "Clear suppression must not overload adapter scope.")

        let requestCountBeforeUnsupportedScope = runner.requests.count
        do {
            _ = try await client.setSuppression(
                ruleId: "dependency.unknown",
                scope: .findingGroup,
                findingGroupId: "group-a"
            )
            throw NativeModelTestFailure(description: "Finding-group suppression must fail closed.")
        } catch ServiceClient.ClientError.service(let error) {
            try expectEqual(error.code, "unsupported_scope", "Finding-group suppression should report its unsupported scope.")
        }
        try expectEqual(
            runner.requests.count,
            requestCountBeforeUnsupportedScope,
            "Unsupported finding-group suppression must not be silently sent as rule-wide."
        )
    }

    private func nativeScriptPreviewUsesIdentityOnlyAndDecodesBlockedResponse() async throws {
        let runner = RecordingServiceProcessRunner()
        let client = ServiceClient(processRunner: runner, serviceURL: URL(fileURLWithPath: "/tmp/fake-service"))
        let skill = SkillRecord(
            id: "skill-fixture",
            agent: "codex",
            scope: "agent-global",
            path: "/tmp/skill/SKILL.md",
            displayPath: "$HOME/.codex/skills/skill/SKILL.md",
            definitionId: "definition-fixture",
            name: "Fixture",
            state: "loaded",
            enabled: true
        )

        let preview = try await client.previewScriptExecution(skill: skill)
        let params = try runner.params(for: "script.previewExecution")

        try expectEqual(params["instance_id"] as? String, Optional("skill-fixture"), "Script preview stable instance id")
        try expectEqual(params["definition_id"] as? String, Optional("definition-fixture"), "Script preview definition id")
        try expectEqual(params["agent"] as? String, Optional("codex"), "Script preview agent")
        try expectNil(params["command"], "Native script preview must not guess a command.")
        try expectEqual(preview.skillID, "skill-fixture", "Canonical Rust identity must decode.")
        try expectEqual(preview.commandPreview, [], "Blocked identity-only preview must keep command argv empty.")
        try expectEqual(preview.executionAllowed, false, "Identity-only preview must remain blocked.")
        try expectContains(preview.disabledReason, "No verified script command", "Blocked preview must explain why no command is available.")
    }

    private func localHistoryPageRequestsDecodeAliases() async throws {
        let runner = RecordingServiceProcessRunner()
        let client = ServiceClient(processRunner: runner, serviceURL: URL(fileURLWithPath: "/tmp/fake-service"))

        let snapshots = try await client.listAgentConfigSnapshotPage(
            agent: "claude-code",
            scope: nil,
            limit: 100,
            cursor: nil,
            sourceRevision: nil
        )
        let events = try await client.listSkillEventPage(
            instanceID: "skill-1",
            limit: 100,
            cursor: "v1:event-page-1",
            sourceRevision: "sha256:event-revision"
        )

        try expectEqual(snapshots.records.map(\.id), ["snapshot-1"], "Snapshot page should decode records.")
        try expectEqual(snapshots.returnedCount, 1, "Snapshot page should decode snake_case metadata.")
        try expectEqual(snapshots.nextCursor, Optional("v1:snapshot-page-2"), "Snapshot cursor")
        try expectEqual(events.records.map(\.id), [Int64(7)], "Event page should decode records.")
        try expectEqual(events.totalCount, Optional(2), "Event page should decode camelCase metadata aliases.")
        try expectEqual(events.sourceRevision, "sha256:event-revision", "Event source revision alias")

        let camelSnapshot = try JSONDecoder().decode(
            ListPageWireResult<ConfigSnapshotRecord>.self,
            from: Data(#"{"records":[{"id":"snapshot-camel","agent":"claude-code","scope":"agent-global","target":"/tmp/settings.json","content":"{}","reason":"pre-toggle","created_at":1}],"sourceRevision":"sha256:camel-snapshot","returnedCount":1,"totalCount":1,"hasMore":false,"nextCursor":null,"sourceCompleteness":"enumerable","incompleteReason":null}"#.utf8)
        )
        let snakeEvent = try JSONDecoder().decode(
            ListPageWireResult<SkillEventRecord>.self,
            from: Data(#"{"records":[{"id":9,"instance_id":"skill-1","kind":"toggle","payload":{},"occurred_at":1}],"source_revision":"sha256:snake-event","returned_count":1,"total_count":1,"has_more":false,"next_cursor":null,"source_completeness":"enumerable","incomplete_reason":null}"#.utf8)
        )
        try expectEqual(camelSnapshot.records.map(\.id), ["snapshot-camel"], "The shared page wire type should decode camelCase snapshot metadata.")
        try expectEqual(snakeEvent.records.map(\.id), [Int64(9)], "The shared page wire type should decode snake_case event metadata.")
        let snapshotWireKeys = try encodedObjectKeys(camelSnapshot)
        let eventWireKeys = try encodedObjectKeys(snakeEvent)
        try expectEqual(snapshotWireKeys, eventWireKeys, "Snapshot and event pages must encode the same canonical wire metadata aliases.")

        let snapshotParams = try runner.params(for: "snapshot.listAgentConfigPage")
        try expectEqual(snapshotParams["agent"] as? String, Optional("claude-code"), "Snapshot page agent")
        try expectEqual(snapshotParams["limit"] as? Int, Optional(100), "Snapshot page limit")
        try expectNil(snapshotParams["cursor"], "First snapshot page should omit cursor.")
        let eventParams = try runner.params(for: "skill.listEventsPage")
        try expectEqual(eventParams["instance_id"] as? String, Optional("skill-1"), "Event page stable instance id")
        try expectEqual(eventParams["cursor"] as? String, Optional("v1:event-page-1"), "Event continuation cursor")
        try expectEqual(eventParams["source_revision"] as? String, Optional("sha256:event-revision"), "Event source revision")
    }

    private func providerActivityPageRequestUsesExactBindings() async throws {
        let runner = RecordingServiceProcessRunner()
        let client = ServiceClient(processRunner: runner, serviceURL: URL(fileURLWithPath: "/tmp/fake-service"))

        let page = try await client.listProviderActivity(
            provider: "openai-compatible",
            model: "fixture-model",
            action: "analyze",
            windowDays: 30,
            startAt: 100,
            endAt: 200,
            limit: 50,
            cursor: "v1:activity-page-2",
            sourceRevision: "sha256:activity-revision"
        )

        try expectEqual(page.rows.map(\.id), ["activity-1"], "Provider activity RPC should decode rows.")
        try expectEqual(page.totalCount, 130, "Provider activity RPC should decode page totals.")
        try expectEqual(page.nextCursor, "v1:activity-page-3", "Provider activity RPC should decode cursor.")
        let params = try runner.params(for: "llm.listProviderActivity")
        try expectEqual(params["provider"] as? String, Optional("openai-compatible"), "Provider activity provider filter")
        try expectEqual(params["model"] as? String, Optional("fixture-model"), "Provider activity model filter")
        try expectEqual(params["action"] as? String, Optional("analyze"), "Provider activity action filter")
        try expectEqual(params["window_days"] as? Int, Optional(30), "Provider activity window filter")
        try expectEqual(params["start_at"] as? Int, Optional(100), "Provider activity start filter")
        try expectEqual(params["end_at"] as? Int, Optional(200), "Provider activity end filter")
        try expectEqual(params["limit"] as? Int, Optional(50), "Provider activity page limit")
        try expectEqual(params["cursor"] as? String, Optional("v1:activity-page-2"), "Provider activity cursor")
        try expectEqual(params["source_revision"] as? String, Optional("sha256:activity-revision"), "Provider activity source revision")
    }

    private func localSessionCursorRequestDecodesCompleteness() async throws {
        let runner = RecordingServiceProcessRunner()
        let client = ServiceClient(processRunner: runner, serviceURL: URL(fileURLWithPath: "/tmp/fake-service"))
        let page = try await client.previewLocalSessions(
            authorizedRoots: ["/tmp/sessions"],
            agent: "codex",
            scope: .all,
            includeContentItems: false,
            limit: 100,
            cursor: "v1:cursor-100",
            sourceRevision: "sha256:sessions"
        )
        let params = try runner.params(for: "session.previewLocalSessions")
        try expectEqual(params["cursor"] as? String, "v1:cursor-100", "Session cursor request")
        try expectEqual(params["source_revision"] as? String, "sha256:sessions", "Session source revision request")
        try expectEqual(params["paging_mode"] as? String, "keyset", "Session cursor requests should explicitly select keyset paging.")
        try expectNil(params["offset"], "Cursor requests must not send legacy offset.")
        try expectNil(params["max_files"], "Cursor requests must not send legacy max-files.")
        try expectEqual(page.nextCursor, "v1:cursor-200", "Session next cursor")
        try expectEqual(page.sourceCompleteness, .enumerable, "Session source completeness")
        try expectNil(page.incompleteReason, "Enumerable session page should not report incompleteness.")

        let legacyRunner = RecordingServiceProcessRunner()
        let legacyClient = ServiceClient(processRunner: legacyRunner, serviceURL: URL(fileURLWithPath: "/tmp/fake-service"))
        _ = try await legacyClient.previewLocalSessions(
            authorizedRoots: ["/tmp/sessions"],
            agent: "codex",
            scope: .all,
            includeContentItems: false,
            limit: 20
        )
        let legacyParams = try legacyRunner.params(for: "session.previewLocalSessions")
        try expectEqual(legacyParams["offset"] as? Int, 0, "Legacy session wrapper should retain offset zero.")
        try expectEqual(legacyParams["max_files"] as? Int, 800, "Legacy session wrapper should retain the 800-file default.")
        try expectNil(legacyParams["paging_mode"], "Legacy session requests must not opt into keyset paging.")
    }

    private func localSessionMessagePageRequestUsesExactBindings() async throws {
        let runner = RecordingServiceProcessRunner()
        let client = ServiceClient(processRunner: runner, serviceURL: URL(fileURLWithPath: "/tmp/fake-service"))
        let page = try await client.listLocalSessionMessages(
            authorizedRoots: ["/tmp/sessions"],
            agent: "codex",
            project: ProjectContext(
                id: "project-fixture",
                name: "Fixture",
                rootPath: "/tmp/project",
                currentCWD: "/tmp/project/subdir",
                lastUsedAt: nil,
                isActive: true,
                validationError: nil
            ),
            sessionID: "session-large",
            limit: 40,
            cursor: "v1:message-page-2",
            sourceRevision: "sha256:messages"
        )
        let params = try runner.params(for: "session.listLocalSessionMessages")
        try expectEqual(params["session_id"] as? String, "session-large", "Message page stable session id")
        try expectEqual(params["limit"] as? Int, 40, "Message page limit")
        try expectEqual(params["cursor"] as? String, "v1:message-page-2", "Message continuation cursor")
        try expectEqual(params["source_revision"] as? String, "sha256:messages", "Message source revision")
        try expectEqual(params["project_root"] as? String, "/tmp/project", "Message page project root")
        try expectEqual(
            page.contentItems.map(\.kind),
            [LocalSessionContentKind.userMessage, LocalSessionContentKind.agentReply],
            "Message RPC should decode conversation messages."
        )
    }

    private func encodedObjectKeys<T: Encodable>(_ value: T) throws -> Set<String> {
        let data = try JSONEncoder().encode(value)
        guard let object = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw NativeModelTestFailure(description: "Encoded page result should be a JSON object.")
        }
        return Set(object.keys)
    }

    private func configConsistencyRequestsUseExactBindings() async throws {
        let runner = RecordingServiceProcessRunner()
        let client = ServiceClient(processRunner: runner, serviceURL: URL(fileURLWithPath: "/tmp/fake-service"))

        let savePreview = try await client.previewClaudeSettingsSave(
            content: "{\"enabled\":true}\n",
            expectedRevision: "sha256:before"
        )
        let saved = try await client.saveClaudeSettings(
            content: "{\"enabled\":true}\n",
            confirmation: savePreview.confirmation
        )
        let preview = try await client.previewSnapshotRollback(snapshotID: "snap-claude-new")
        let rolledBack = try await client.rollbackSnapshot(
            snapshotID: preview.snapshot.id,
            confirmation: ActionConfirmationWire(
                action: preview.action,
                previewToken: preview.previewToken
            )
        )

        try expectEqual(saved.document.revision, Optional("sha256:after"), "Config save should decode the verified document revision.")
        try expectEqual(preview.currentRevision, "sha256:current", "Rollback preview should decode current_revision.")
        try expectEqual(preview.previewToken, "action-preview:v1:hmac-sha256:rollback-token", "Rollback preview should decode the HMAC preview token.")
        try expectEqual(rolledBack.document.revision, Optional("sha256:restored"), "Rollback should decode the verified restored document.")

        let previewParams = try runner.params(for: "config.previewSaveClaudeSettings")
        try expectEqual(Set(previewParams.keys), Set(["content", "expected_revision"]), "Config preview should bind exact content and revision.")
        let saveParams = try runner.params(for: "config.saveClaudeSettings")
        try expectEqual(Set(saveParams.keys), Set(["content", "confirmation"]), "Config save request should contain content and typed confirmation only.")
        try expectEqual(saveParams["content"] as? String, Optional("{\"enabled\":true}\n"), "Config save should preserve exact content.")
        guard let saveConfirmation = saveParams["confirmation"] as? [String: Any] else {
            throw NativeModelTestFailure(description: "Config save confirmation should be an object.")
        }
        try expectEqual(saveConfirmation["confirmed"] as? Bool, true, "Config save should send an explicit confirmation.")
        try expectEqual(saveConfirmation["preview_token"] as? String, Optional("action-preview:v1:hmac-sha256:save-token"), "Config save should send the opaque HMAC token.")

        let rollbackParams = try runner.params(for: "snapshot.rollback")
        try expectEqual(Set(rollbackParams.keys), Set(["snapshot_id", "confirmation"]), "Rollback request should contain snapshot id and typed confirmation.")
        try expectEqual(rollbackParams["snapshot_id"] as? String, Optional("snap-claude-new"), "Rollback should send the previewed snapshot id.")
        guard let rollbackConfirmation = rollbackParams["confirmation"] as? [String: Any] else {
            throw NativeModelTestFailure(description: "Rollback confirmation should be an object.")
        }
        try expectEqual(rollbackConfirmation["preview_token"] as? String, Optional("action-preview:v1:hmac-sha256:rollback-token"), "Rollback should send the opaque HMAC token.")
        try expectNil(rollbackParams["preview_token"], "A bare preview token must never be sent as rollback authorization.")
    }

    private func legacyConfigResponsesFailClosed() throws {
        let legacyDocument = try JSONDecoder().decode(
            ConfigDocumentRecord.self,
            from: Data(#"{"agent":"claude-code","scope":"agent-global","target":"/tmp/settings.json","format":"json","content":"{}\n","exists":true}"#.utf8)
        )
        try expectNil(legacyDocument.revision, "A legal legacy config response should decode without a revision.")
        try expectFalse(legacyDocument.supportsCompareAndSwap, "A config response without a revision must remain read-only.")

        do {
            _ = try JSONDecoder().decode(
                SnapshotRollbackPreviewRecord.self,
                from: Data(#"{"snapshot":{"id":"snap-legacy","agent":"claude-code","scope":"agent-global","target":"/tmp/settings.json","content":"{}\n","reason":"legacy","created_at":1},"current_content":"{}\n","current_read_error":null,"changed":false,"redacted":false,"rollback_supported":true}"#.utf8)
            )
            throw NativeModelTestFailure(
                description: "A rollback preview missing its typed action and HMAC binding must fail closed."
            )
        } catch is DecodingError {
            // Expected: legacy token-shaped rollback previews cannot authorize a write.
        }
    }

    private func unrelatedWritesDoNotGainConfigCASFields() throws {
        let toggle = try encodedObject(ToggleSkillParams(instanceId: "skill-1", on: true))
        try expectEqual(Set(toggle.keys), Set(["instance_id", "on"]), "Single-skill toggle should keep its existing request contract.")
        try expectNil(toggle["expected_revision"], "Single-skill toggle must not gain config-save CAS fields.")

        let action = ActionDescriptorWire(
            id: "action:disable-skill:fixture",
            kind: "toggle_skill",
            intent: "disable_skill",
            target: ActionTargetWire(
                kind: "skill",
                id: "skill-1",
                agent: "claude-code",
                scope: "agent-global"
            ),
            projectID: nil,
            impacts: ["agent_config"],
            previewMethod: "batch.previewSkillToggles",
            applyMethod: "batch.applySkillToggles",
            sourceRevision: "sha256:source",
            confirmationRequired: true,
            network: "none",
            readback: ["agent_config", "skill_aggregates"],
            evidenceRefs: ["skill:skill-1"]
        )
        let batch = try encodedObject(BatchToggleParams(
            instanceIDs: ["skill-1"],
            targetEnabled: false,
            confirmation: ActionConfirmationWire(
                action: action,
                previewToken: "action-preview:v1:hmac-sha256:fixture"
            )
        ))
        try expectEqual(
            Set(batch.keys),
            Set(["instance_ids", "target_enabled", "confirmation"]),
            "Batch toggle apply should send one typed confirmation envelope."
        )
        try expectNil(batch["preview_token"], "The opaque token must stay nested inside confirmation.")
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

@MainActor
extension SkillStoreTests {
    func localSessionStoreAggregatesSkillUsageAcrossDelayedPages() async throws {
        let runner = DelayedSkillUsageLocalSessionRunner()
        let store = SkillStore(service: ServiceClient(
            processRunner: runner,
            serviceURL: URL(fileURLWithPath: "/tmp/delayed-skill-usage-session-service")
        ))

        let refresh = Task { @MainActor in
            await store.refreshLocalSessionSnapshot(reason: .startup)
        }
        try await waitUntil("The second local-session skill page should be delayed.") {
            await runner.requestCount() == 2
        }
        try expectEqual(
            store.localSessionPreviewResult.skillUsageRows.map(\.skillId),
            ["alpha"],
            "The Store should publish the accepted first-page skill summary while page two is delayed."
        )

        await runner.releaseSecondPage()
        await refresh.value

        let rows = store.localSessionPreviewResult.skillUsageRows
        try expectEqual(rows.map(\.skillId), ["alpha", "beta"], "The Store must retain and sort skills from both accepted pages.")
        try expectEqual(rows.first?.callCount, Optional(4), "Repeated alpha calls must accumulate across Store pages.")
        try expectEqual(rows.first?.sessionCount, Optional(2), "Repeated alpha sessions must accumulate across Store pages.")
        try expectEqual(rows.first?.latestModifiedAt, Optional("300"), "Repeated alpha must retain the latest timestamp.")
        try expectEqual(rows.first?.evidenceRefs, ["session:alpha:1", "session:alpha:2"], "Repeated alpha evidence must be deduplicated in stable order.")
    }

    func localSessionPrewarmMoreAndAllUseCursorPages() async throws {
        let runner = PagedLocalSessionRunner(totalCount: 1_205)
        let store = SkillStore(service: ServiceClient(processRunner: runner, serviceURL: URL(fileURLWithPath: "/tmp/paged-session-service")))
        await store.refreshLocalSessionSnapshot(reason: .startup)
        try expectEqual(store.localSessionPreviewResult.sessionRows.count, 800, "Prewarm boundary")
        try expectEqual(store.localSessionCompleteness.loadedCount, 800, "Prewarm loaded count")
        try expectEqual(store.localSessionCompleteness.hasMore, true, "Prewarm should expose continuation")
        await store.loadMoreLocalSessions()
        try expectEqual(store.localSessionPreviewResult.sessionRows.count, 900, "One more page")
        await store.loadAllLocalSessions()
        try expectEqual(store.localSessionPreviewResult.sessionRows.count, 1_205, "Load all count")
        try expectEqual(store.localSessionCompleteness.completeness, .complete, "Load all completeness")
        try expectEqual(Set(store.localSessionPreviewResult.sessionRows.map(\.id)).count, 1_205, "No duplicates")
        let requests = await runner.recordedRequests()
        try expectFalse(!requests.allSatisfy { $0.limit == 100 }, "Every summary page must request at most 100 rows.")
        try expectFalse(!requests.allSatisfy { $0.pagingMode == "keyset" }, "Every summary page must explicitly opt into keyset paging.")
        try expectFalse(!requests.allSatisfy { $0.offset == nil && $0.maxFiles == nil }, "Keyset pages must omit legacy paging fields.")
        try expectFalse(!requests.dropFirst().allSatisfy { $0.cursor != nil && $0.sourceRevision != nil }, "Every continuation must bind cursor and source revision.")
    }

    func cancelledAndStaleLocalSessionPagesCannotPublish() async throws {
        let runner = PagedLocalSessionRunner(totalCount: 1_205, initiallyReleasedThroughPage: 7)
        let store = SkillStore(service: ServiceClient(processRunner: runner, serviceURL: URL(fileURLWithPath: "/tmp/delayed-session-service")))
        await store.refreshLocalSessionSnapshot(reason: .startup)
        try expectEqual(store.localSessionPreviewResult.sessionRows.count, 800, "Delayed fixture prewarm")
        let loadAll = Task { @MainActor in await store.loadAllLocalSessions() }
        await runner.releaseNextPage()
        try await waitUntil("Load All should accept its first released continuation.") { store.localSessionPreviewResult.sessionRows.count == 900 }
        await runner.releaseNextPage()
        try await waitUntil("Load All should accept its second released continuation.") { store.localSessionPreviewResult.sessionRows.count == 1_000 }
        let acceptedCount = store.localSessionPreviewResult.sessionRows.count
        try await waitUntil("A third Load All page should be in flight.") { await runner.requestCount() >= 11 }
        store.cancelLocalSessionLoadAll()
        await runner.releaseNextPage()
        await loadAll.value
        try? await Task.sleep(nanoseconds: 80_000_000)
        try expectEqual(store.localSessionPreviewResult.sessionRows.count, acceptedCount, "Cancellation must retain accepted rows and reject the old response.")

        let staleRunner = PagedLocalSessionRunner(totalCount: 1_205, initiallyReleasedThroughPage: 7)
        let staleStore = SkillStore(service: ServiceClient(processRunner: staleRunner, serviceURL: URL(fileURLWithPath: "/tmp/stale-session-service")))
        await staleStore.refreshLocalSessionSnapshot(reason: .startup)
        let staleLoad = Task { @MainActor in await staleStore.loadMoreLocalSessions() }
        try await waitUntil("The stale source continuation should be in flight.") { await staleRunner.requestCount() >= 9 }
        staleStore.agentFilter = .codex
        await staleRunner.releaseNextPage()
        await staleLoad.value
        try expectFalse(staleStore.localSessionPreviewResult.sessionRows.contains { $0.id.hasPrefix("claude-code-session-8") }, "A page released after changing agentFilter must never enter the active source.")
    }

    func failedInitialLocalSessionPageRetriesFromNilCursor() async throws {
        let runner = PagedLocalSessionRunner(totalCount: 205, failuresByPage: [0: 1])
        let store = SkillStore(service: ServiceClient(processRunner: runner, serviceURL: URL(fileURLWithPath: "/tmp/first-page-retry-service")))

        await store.refreshLocalSessionSnapshot(reason: .startup)
        try expectEqual(store.localSessionPreviewResult.sessionRows.count, 0, "A failed first page has no accepted rows.")
        try expectEqual(store.localSessionCompleteness.incompleteReason, .pageFailed, "A failed first page should expose pageFailed.")
        try expectEqual(store.localSessionCompleteness.canLoadAll, true, "Load All must offer a real nil-cursor retry.")

        await store.loadAllLocalSessions()
        try expectEqual(store.localSessionPreviewResult.sessionRows.count, 205, "Load All should retry from nil and reach EOF.")
        try expectEqual(store.localSessionCompleteness.completeness, .complete, "The nil-cursor retry should complete.")
        let requests = await runner.recordedRequests()
        try expectNil(requests[0].cursor, "Initial failure starts at nil cursor.")
        try expectNil(requests[1].cursor, "Retry after initial failure must also start at nil cursor.")
    }

    func failedLocalSessionPrewarmRetainsPagesAndRetriesCursor() async throws {
        let runner = PagedLocalSessionRunner(totalCount: 405, failuresByPage: [2: 1])
        let store = SkillStore(service: ServiceClient(processRunner: runner, serviceURL: URL(fileURLWithPath: "/tmp/prewarm-retry-service")))

        await store.refreshLocalSessionSnapshot(reason: .startup)
        try expectEqual(store.localSessionPreviewResult.sessionRows.count, 200, "A failed third prewarm page must retain two accepted pages.")
        try expectEqual(store.localSessionCompleteness.incompleteReason, .pageFailed, "Prewarm failure should be retryable.")
        try expectEqual(store.localSessionCompleteness.canLoadAll, true, "Retained cursor should enable Load All retry.")

        await store.loadAllLocalSessions()
        try expectEqual(store.localSessionPreviewResult.sessionRows.count, 405, "Retry should continue from the accepted cursor.")
        try expectEqual(store.localSessionCompleteness.completeness, .complete, "Cursor retry should reach EOF.")
        let requests = await runner.recordedRequests()
        try expectEqual(requests[3].cursor, "cursor-200", "Retry must resume from the last accepted cursor.")
    }

    func oldLocalSessionGenerationErrorCannotOverwriteReactivatedSource() async throws {
        let runner = ABALocalSessionErrorRunner()
        let store = SkillStore(service: ServiceClient(processRunner: runner, serviceURL: URL(fileURLWithPath: "/tmp/aba-session-service")))
        store.agentFilter = .all

        let old = Task { @MainActor in await store.refreshLocalSessionSnapshot(reason: .manual) }
        try await waitUntil("Old local-session generation should be in flight.") {
            await runner.requestCount() == 1
        }
        store.agentFilter = .codex
        store.agentFilter = .all
        await store.refreshLocalSessionSnapshot(reason: .manual)
        try expectEqual(store.localSessionPreviewResult.sessionRows.map(\.id), ["all-session-current"], "Reactivated source should publish its new generation.")
        try expectEqual(store.localSessionCompleteness.completeness, .complete, "New generation should be complete before the old error.")

        await runner.releaseOldFailure()
        await old.value
        try expectEqual(store.localSessionPreviewResult.sessionRows.map(\.id), ["all-session-current"], "Late old error must not replace the new snapshot.")
        try expectEqual(store.localSessionCompleteness.completeness, .complete, "Late old error must not overwrite completeness.")
        try expectNil(store.localSessionCompleteness.incompleteReason, "Late old error must not publish pageFailed.")
    }

    func localSessionTerminalPageUsesDecreasingExactTotal() async throws {
        let runner = ScriptedLocalSessionPageRunner(pages: [
            ScriptedLocalSessionPage(rowIDs: ["session-a", "session-b"], totalMatchedCount: 5, hasMore: true, nextCursor: "cursor-after-two"),
            ScriptedLocalSessionPage(rowIDs: ["session-c"], totalMatchedCount: 3, hasMore: false, nextCursor: nil),
        ])
        let store = SkillStore(service: ServiceClient(processRunner: runner, serviceURL: URL(fileURLWithPath: "/tmp/decreasing-session-total-service")))

        await store.refreshLocalSessionSnapshot(reason: .startup)

        try expectEqual(store.localSessionPreviewResult.sessionRows.map(\.id), ["session-a", "session-b", "session-c"], "Both pages should merge unique rows.")
        try expectEqual(store.localSessionPreviewResult.totalMatchedCount, 3, "The terminal exact total must replace the earlier candidate upper bound.")
        try expectEqual(store.localSessionCompleteness.loadedCount, 3, "All exact rows should be loaded.")
        try expectEqual(store.localSessionCompleteness.totalCount, Optional(3), "Completeness should expose the terminal exact total.")
        try expectEqual(store.localSessionCompleteness.completeness, .complete, "EOF with loaded equal to total must be complete.")
    }

    func localSessionZeroRowPageContinuesWhenCursorProgresses() async throws {
        let runner = ScriptedLocalSessionPageRunner(pages: [
            ScriptedLocalSessionPage(rowIDs: [], totalMatchedCount: 1, hasMore: true, nextCursor: "cursor-after-empty"),
            ScriptedLocalSessionPage(rowIDs: ["session-valid"], totalMatchedCount: 1, hasMore: false, nextCursor: nil),
        ])
        let store = SkillStore(service: ServiceClient(processRunner: runner, serviceURL: URL(fileURLWithPath: "/tmp/empty-progress-session-service")))

        await store.refreshLocalSessionSnapshot(reason: .startup)

        try expectEqual(store.localSessionPreviewResult.sessionRows.map(\.id), ["session-valid"], "A cursor-advancing empty page must continue to the valid row.")
        try expectEqual(store.localSessionCompleteness.completeness, .complete, "The continuation should reach complete EOF.")
        try expectEqual(await runner.recordedCursors(), [nil, "cursor-after-empty"], "The second request must use the empty page's advanced cursor.")
    }

    func localSessionZeroRowPageRejectsRepeatedCursor() async throws {
        let runner = ScriptedLocalSessionPageRunner(pages: [
            ScriptedLocalSessionPage(rowIDs: ["session-a"], totalMatchedCount: 2, hasMore: true, nextCursor: "cursor-repeat"),
            ScriptedLocalSessionPage(rowIDs: [], totalMatchedCount: 1, hasMore: true, nextCursor: "cursor-repeat"),
        ])
        let store = SkillStore(service: ServiceClient(processRunner: runner, serviceURL: URL(fileURLWithPath: "/tmp/repeated-session-cursor-service")))

        await store.refreshLocalSessionSnapshot(reason: .startup)

        try expectEqual(await runner.requestCount(), 2, "A repeated continuation cursor must stop before a third request.")
        try expectEqual(store.localSessionPreviewResult.sessionRows.map(\.id), ["session-a"], "Rejecting a cursor loop must retain accepted rows.")
        try expectEqual(store.localSessionCompleteness.incompleteReason, .pageFailed, "A repeated cursor must surface a retryable page failure.")
    }
}

actor DelayedSkillUsageLocalSessionRunner: ServiceProcessRunning {
    private var requests = 0
    private var secondPageReleased = false
    private var secondPageWaiter: CheckedContinuation<Void, Never>?

    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) async throws -> Data {
        let request = try JSONSerialization.jsonObject(with: input) as? [String: Any] ?? [:]
        requests += 1
        let pageIndex = requests - 1
        guard pageIndex < 2 else { throw PagedSessionRunnerError.injected }
        if pageIndex == 1, !secondPageReleased {
            await withCheckedContinuation { secondPageWaiter = $0 }
        }

        let hasMore = pageIndex == 0
        let sessionID = pageIndex == 0 ? "session-alpha" : "session-beta"
        let skillRows: [[String: Any]] = pageIndex == 0
            ? [[
                "skill_id": "alpha",
                "skill_name": "Alpha",
                "agent": "codex",
                "call_count": 1,
                "session_count": 1,
                "latest_modified_at": "100",
                "evidence_refs": ["session:alpha:1"],
            ]]
            : [[
                "skill_id": "beta",
                "skill_name": "Beta",
                "agent": "codex",
                "call_count": 2,
                "session_count": 1,
                "latest_modified_at": "200",
                "evidence_refs": ["session:beta:1"],
            ], [
                "skill_id": "alpha",
                "skill_name": "Alpha",
                "agent": "codex",
                "call_count": 3,
                "session_count": 1,
                "latest_modified_at": "300",
                "evidence_refs": ["session:alpha:1", "session:alpha:2"],
            ]]
        let result: [String: Any] = [
            "generated_by": "local-v2.98",
            "authorized": true,
            "count": 1,
            "total_candidate_count": 2,
            "total_matched_count": 2,
            "offset": 0,
            "limit": 100,
            "has_more": hasMore,
            "next_cursor": hasMore ? "cursor-page-two" : NSNull(),
            "source_revision": "sha256:skill-usage-pages",
            "source_completeness": "enumerable",
            "incomplete_reason": NSNull(),
            "candidate_set_truncated": false,
            "session_rows": [[
                "id": sessionID,
                "title": sessionID,
                "source_kind": "authorized-local-session",
                "scope": "all",
                "redacted_path": "$HOME/.sessions/\(sessionID).jsonl",
                "modified_at": pageIndex == 0 ? 300 : 200,
                "excerpt": "Summary \(sessionID)",
                "content_included": false,
                "content_items": [],
            ]],
            "skill_usage_rows": skillRows,
        ]
        return try JSONSerialization.data(withJSONObject: [
            "id": request["id"] ?? "test",
            "ok": true,
            "result": result,
        ])
    }

    func requestCount() -> Int { requests }

    func releaseSecondPage() {
        secondPageReleased = true
        secondPageWaiter?.resume()
        secondPageWaiter = nil
    }
}

struct RecordedPagedSessionRequest: Sendable {
    let limit: Int?
    let cursor: String?
    let sourceRevision: String?
    let pagingMode: String?
    let offset: Int?
    let maxFiles: Int?
}

private enum PagedSessionRunnerError: LocalizedError {
    case injected

    var errorDescription: String? { "injected local-session page failure" }
}

actor PagedLocalSessionRunner: ServiceProcessRunning {
    private let totalCount: Int
    private var releasedThroughPage: Int
    private var waiters: [Int: [CheckedContinuation<Void, Never>]] = [:]
    private var requests: [RecordedPagedSessionRequest] = []
    private var failuresByPage: [Int: Int]

    init(
        totalCount: Int,
        initiallyReleasedThroughPage: Int = .max,
        failuresByPage: [Int: Int] = [:]
    ) {
        self.totalCount = totalCount
        releasedThroughPage = initiallyReleasedThroughPage
        self.failuresByPage = failuresByPage
    }

    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) async throws -> Data {
        let request = try JSONSerialization.jsonObject(with: input) as? [String: Any] ?? [:]
        let params = request["params"] as? [String: Any] ?? [:]
        let cursor = params["cursor"] as? String
        requests.append(RecordedPagedSessionRequest(
            limit: params["limit"] as? Int,
            cursor: cursor,
            sourceRevision: params["source_revision"] as? String,
            pagingMode: params["paging_mode"] as? String,
            offset: params["offset"] as? Int,
            maxFiles: params["max_files"] as? Int
        ))
        let start = cursor.flatMap { Int($0.replacingOccurrences(of: "cursor-", with: "")) } ?? 0
        let pageIndex = start / 100
        if let remaining = failuresByPage[pageIndex], remaining > 0 {
            failuresByPage[pageIndex] = remaining - 1
            throw PagedSessionRunnerError.injected
        }
        if pageIndex > releasedThroughPage {
            await withCheckedContinuation { waiters[pageIndex, default: []].append($0) }
        }
        let agent = (params["agent"] as? String) ?? "all"
        let end = min(start + 100, totalCount)
        let rows: [[String: Any]] = (start..<end).map { index in
            ["id": "\(agent)-session-\(index)", "title": "Session \(index)",
             "source_kind": "authorized-local-session", "scope": "all",
             "redacted_path": "$HOME/.sessions/\(index).jsonl", "modified_at": totalCount - index,
             "excerpt": "Summary \(index)", "content_included": false, "content_items": []]
        }
        let hasMore = end < totalCount
        let result: [String: Any] = [
            "generated_by": "local-v2.98", "authorized": true, "count": rows.count,
            "total_candidate_count": totalCount, "total_matched_count": totalCount,
            "offset": 0, "limit": 100, "has_more": hasMore,
            "next_cursor": hasMore ? "cursor-\(end)" : NSNull(),
            "source_revision": "sha256:\(agent)-sessions", "source_completeness": "enumerable",
            "incomplete_reason": NSNull(), "candidate_set_truncated": false, "session_rows": rows
        ]
        return try JSONSerialization.data(withJSONObject: [
            "id": request["id"] ?? "test", "ok": true, "result": result
        ])
    }

    func releaseNextPage() {
        guard releasedThroughPage < Int.max else { return }
        releasedThroughPage += 1
        for page in waiters.keys.filter({ $0 <= releasedThroughPage }) {
            (waiters.removeValue(forKey: page) ?? []).forEach { $0.resume() }
        }
    }

    func requestCount() -> Int { requests.count }
    func recordedRequests() -> [RecordedPagedSessionRequest] { requests }
}

struct ScriptedLocalSessionPage: Sendable {
    let rowIDs: [String]
    let totalMatchedCount: Int
    let hasMore: Bool
    let nextCursor: String?
}

actor ScriptedLocalSessionPageRunner: ServiceProcessRunning {
    private let pages: [ScriptedLocalSessionPage]
    private var cursors: [String?] = []

    init(pages: [ScriptedLocalSessionPage]) {
        self.pages = pages
    }

    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) async throws -> Data {
        let request = try JSONSerialization.jsonObject(with: input) as? [String: Any] ?? [:]
        let params = request["params"] as? [String: Any] ?? [:]
        cursors.append(params["cursor"] as? String)
        guard pages.indices.contains(cursors.count - 1) else {
            throw PagedSessionRunnerError.injected
        }
        let page = pages[cursors.count - 1]
        let rows = page.rowIDs.enumerated().map { index, id -> [String: Any] in
            [
                "id": id,
                "title": id,
                "source_kind": "authorized-local-session",
                "scope": "all",
                "redacted_path": "$HOME/.sessions/\(id).jsonl",
                "modified_at": 1_000 - index,
                "excerpt": "Summary \(id)",
                "content_included": false,
                "content_items": [],
            ]
        }
        let result: [String: Any] = [
            "generated_by": "local-v2.98",
            "authorized": true,
            "count": rows.count,
            "total_candidate_count": max(page.totalMatchedCount, rows.count),
            "total_matched_count": page.totalMatchedCount,
            "offset": 0,
            "limit": 100,
            "has_more": page.hasMore,
            "next_cursor": page.nextCursor ?? NSNull(),
            "source_revision": "sha256:scripted-sessions",
            "source_completeness": "enumerable",
            "incomplete_reason": NSNull(),
            "candidate_set_truncated": false,
            "session_rows": rows,
        ]
        return try JSONSerialization.data(withJSONObject: [
            "id": request["id"] ?? "test",
            "ok": true,
            "result": result,
        ])
    }

    func requestCount() -> Int { cursors.count }
    func recordedCursors() -> [String?] { cursors }
}

actor ABALocalSessionErrorRunner: ServiceProcessRunning {
    private var requests = 0
    private var oldFailureWaiter: CheckedContinuation<Void, Never>?

    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) async throws -> Data {
        requests += 1
        let requestNumber = requests
        let request = try JSONSerialization.jsonObject(with: input) as? [String: Any] ?? [:]
        let params = request["params"] as? [String: Any] ?? [:]
        if requestNumber == 1 {
            await withCheckedContinuation { oldFailureWaiter = $0 }
            throw PagedSessionRunnerError.injected
        }
        let agent = (params["agent"] as? String) ?? "all"
        let result: [String: Any] = [
            "generated_by": "local-v2.98", "authorized": true, "count": 1,
            "total_candidate_count": 1, "total_matched_count": 1,
            "offset": 0, "limit": 100, "has_more": false,
            "next_cursor": NSNull(), "source_revision": "sha256:\(agent)-sessions",
            "source_completeness": "enumerable", "incomplete_reason": NSNull(),
            "candidate_set_truncated": false,
            "session_rows": [[
                "id": "\(agent)-session-current", "title": "Current Session",
                "source_kind": "authorized-local-session", "scope": "all",
                "redacted_path": "$HOME/.sessions/current.jsonl", "modified_at": 1,
                "excerpt": "Current summary", "content_included": false, "content_items": []
            ]]
        ]
        return try JSONSerialization.data(withJSONObject: [
            "id": request["id"] ?? "test", "ok": true, "result": result
        ])
    }

    func requestCount() -> Int { requests }

    func releaseOldFailure() {
        oldFailureWaiter?.resume()
        oldFailureWaiter = nil
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
        case "config.previewSaveClaudeSettings":
            return Data(Self.configSavePreviewResponse.utf8)
        case "config.saveClaudeSettings":
            return Data(Self.configSaveResponse.utf8)
        case "snapshot.previewRollback":
            return Data(Self.rollbackPreviewResponse.utf8)
        case "snapshot.rollback":
            return Data(Self.rollbackResponse.utf8)
        case "snapshot.listAgentConfigPage":
            return Data(Self.configSnapshotPageResponse.utf8)
        case "skill.listEventsPage":
            return Data(Self.skillEventPageResponse.utf8)
        case "session.previewLocalSessions":
            return Data(Self.localSessionPageResponse.utf8)
        case "session.listLocalSessionMessages":
            return Data(Self.localSessionMessagePageResponse.utf8)
        case "llm.listProviderActivity":
            return Data(Self.providerActivityPageResponse.utf8)
        case "llm.previewPrompt":
            return Data(Self.previewResponse.utf8)
        case "llm.confirmPromptAndSend":
            return Data(Self.sendResponse.utf8)
        case "rules.setSuppression":
            return Data(Self.ruleSuppressionResponse.utf8)
        case "rules.listTuning":
            return Data(Self.ruleTuningListResponse.utf8)
        case "rules.clearSuppression":
            return Data(Self.clearSuppressionResponse.utf8)
        case "script.previewExecution":
            return Data(Self.scriptPreviewResponse.utf8)
        default:
            return Data(Self.unknownMethodResponse.utf8)
        }
    }

    private static let previewResponse = """
    {"id":"test","ok":true,"result":{"preview_id":"prompt-preview-task","request_kind":"task_cockpit","scope":"agents","prompt_scope":"Task Preflight","enabled":true,"provider":"openai-compatible","model":"gpt-test","destination_host":"llm.example.com","included_fields":[],"excluded_fields":[],"redaction":{"status":"redacted","summary":"ok","redacted_fields":[],"placeholders":[]},"confirmation_required":true,"raw_prompt_persisted":false,"raw_response_persisted":false,"draft_copy_only":true,"redacted_prompt_preview":"preview"}}
    """

    private static let ruleSuppressionResponse = """
    {"id":"test","ok":true,"result":{"rule_id":"dependency.unknown","agent":null,"scope":null,"severity_override":null,"suppression_reason":"Suppressed locally in Agent Copilot after user review.","suppression_note":"Reviewed locally.","updated_at":1}}
    """

    private static let ruleTuningListResponse = """
    {"id":"test","ok":true,"result":[{"rule_id":"dependency.unknown","agent":null,"scope":null,"severity_override":null,"suppression_reason":"Suppressed locally in Agent Copilot after user review.","suppression_note":"Reviewed locally.","updated_at":1}]}
    """

    private static let clearSuppressionResponse = """
    {"id":"test","ok":true,"result":true}
    """

    private static let scriptPreviewResponse = """
    {"id":"test","ok":true,"result":{"skill_instance_id":"skill-fixture","initiated_by":"user","initiator_allowed":true,"cwd":{"requested":null,"effective":"/tmp","source":"project"},"env":{"inherit_parent":false,"provided_keys":[],"redacted_keys":[],"value_policy":"values-redacted"},"network":{"requested":"none","allowed":false,"reason":"Network access is not granted because script execution is disabled."},"files":{"requested":[],"read_allowed":false,"write_allowed":false,"allowed_roots":[]},"command_preview":{"argv":[],"display":"","shell":null},"risks":["No verified script command was supplied; Agent Copilot will not infer or execute one."],"confirmation":{"required":true,"confirmed":false,"fields":["command_preview"],"message":"Per-request user confirmation is required before any execution attempt."},"execution_allowed":false,"disabled_reason":"No verified script command was supplied; Agent Copilot will not infer or execute one."}}
    """

    private static let configSavePreviewResponse = """
    {"id":"test","ok":true,"result":{"action":{"id":"action:save_config:test","kind":"save_config","intent":"save_config","target":{"kind":"config","id":"/tmp/settings.json","agent":"claude-code","scope":"agent-global"},"impacts":["agent_config","app_local_data"],"preview_method":"config.previewSaveClaudeSettings","apply_method":"config.saveClaudeSettings","source_revision":"sha256:before","confirmation_required":true,"network":"none","readback":["agent_config","config_snapshots"],"evidence_refs":["config:/tmp/settings.json"]},"preconditions":[{"kind":"agent_config","target_id":"/tmp/settings.json","expected_revision":"sha256:before"}],"preview_token":"action-preview:v1:hmac-sha256:save-token","current":{"agent":"claude-code","scope":"agent-global","target":"/tmp/settings.json","format":"json","content":"{}\\n","exists":true,"revision":"sha256:before"},"candidate_content_digest":"sha256:candidate","current_revision":"sha256:before","changed":true}}
    """

    private static let configSaveResponse = """
    {"id":"test","ok":true,"result":{"action":{"id":"action:save_config:test","kind":"save_config","intent":"save_config","target":{"kind":"config","id":"/tmp/settings.json","agent":"claude-code","scope":"agent-global"},"impacts":["agent_config","app_local_data"],"preview_method":"config.previewSaveClaudeSettings","apply_method":"config.saveClaudeSettings","source_revision":"sha256:before","confirmation_required":true,"network":"none","readback":["agent_config","config_snapshots"],"evidence_refs":["config:/tmp/settings.json"]},"document":{"agent":"claude-code","scope":"agent-global","target":"/tmp/settings.json","format":"json","content":"{\\"enabled\\":true}\\n","exists":true,"revision":"sha256:after"},"snapshot_id":"snapshot-save","readback":{"action_id":"action:save_config:test","source_revision":"sha256:after","domains":["agent_config","config_snapshots"],"target_ids":["/tmp/settings.json","snapshot-save"],"observations":[{"domain":"agent_config","target_id":"/tmp/settings.json","revision":"sha256:after"},{"domain":"config_snapshots","target_id":"snapshot-save","revision":"sha256:snapshot-save"}],"verified":true}}}
    """

    private static let rollbackPreviewResponse = """
    {"id":"test","ok":true,"result":{"action":{"id":"action:rollback_config:test","kind":"rollback_config","intent":"rollback_config","target":{"kind":"config","id":"/tmp/settings.json","agent":"claude-code","scope":"agent-global"},"impacts":["agent_config"],"preview_method":"snapshot.previewRollback","apply_method":"snapshot.rollback","source_revision":"sha256:current","confirmation_required":true,"network":"none","readback":["agent_config"],"evidence_refs":["snapshot:snap-claude-new","config:/tmp/settings.json"]},"preconditions":[{"kind":"catalog_record","target_id":"snap-claude-new","expected_revision":"sha256:snapshot"},{"kind":"agent_config","target_id":"/tmp/settings.json","expected_revision":"sha256:current"}],"preview_token":"action-preview:v1:hmac-sha256:rollback-token","snapshot":{"id":"snap-claude-new","agent":"claude-code","scope":"agent-global","target":"/tmp/settings.json","content":"{}\\n","reason":"test","created_at":1},"snapshot_content_digest":"sha256:snapshot-content","current_content":"{\\"enabled\\":true}\\n","current_read_error":null,"current_revision":"sha256:current","changed":true,"redacted":false,"rollback_supported":true}}
    """

    private static let rollbackResponse = """
    {"id":"test","ok":true,"result":{"action":{"id":"action:rollback_config:test","kind":"rollback_config","intent":"rollback_config","target":{"kind":"config","id":"/tmp/settings.json","agent":"claude-code","scope":"agent-global"},"impacts":["agent_config"],"preview_method":"snapshot.previewRollback","apply_method":"snapshot.rollback","source_revision":"sha256:current","confirmation_required":true,"network":"none","readback":["agent_config"],"evidence_refs":["snapshot:snap-claude-new","config:/tmp/settings.json"]},"snapshot_id":"snap-claude-new","document":{"agent":"claude-code","scope":"agent-global","target":"/tmp/settings.json","format":"json","content":"{}\\n","exists":true,"revision":"sha256:restored"},"readback":{"action_id":"action:rollback_config:test","source_revision":"sha256:restored","domains":["agent_config"],"target_ids":["/tmp/settings.json"],"observations":[{"domain":"agent_config","target_id":"/tmp/settings.json","revision":"sha256:restored"}],"verified":true}}}
    """

    private static let configSnapshotPageResponse = """
    {"id":"test","ok":true,"result":{"records":[{"id":"snapshot-1","agent":"claude-code","scope":"agent-global","target":"/tmp/settings.json","content":"{}\\n","reason":"test","created_at":1}],"source_revision":"sha256:snapshot-revision","returned_count":1,"total_count":2,"has_more":true,"next_cursor":"v1:snapshot-page-2","source_completeness":"enumerable"}}
    """

    private static let skillEventPageResponse = """
    {"id":"test","ok":true,"result":{"records":[{"id":7,"instance_id":"skill-1","kind":"toggle","payload":{},"occurred_at":1}],"sourceRevision":"sha256:event-revision","returnedCount":1,"totalCount":2,"hasMore":true,"nextCursor":"v1:event-page-2","sourceCompleteness":"enumerable","incompleteReason":null}}
    """

    private static let localSessionPageResponse = """
    {"id":"test","ok":true,"result":{"generated_by":"local-v2.98","authorized":true,"count":1,"total_candidate_count":205,"total_matched_count":205,"offset":0,"limit":100,"has_more":true,"next_cursor":"v1:cursor-200","source_revision":"sha256:sessions","source_completeness":"enumerable","candidate_set_truncated":false,"session_rows":[{"id":"session-100","title":"Session 100","source_kind":"authorized-local-session","scope":"all","redacted_path":"$HOME/.sessions/100.jsonl","excerpt":"Summary","content_included":false,"content_items":[]}]}}
    """

    private static let localSessionMessagePageResponse = """
    {"id":"test","ok":true,"result":{"generated_by":"local-v2.99","session_id":"session-large","content_items":[{"id":"user-1","kind":"user_message","title":"User","text":"Set the goal","char_count":12,"evidence_refs":[]},{"id":"agent-1","kind":"agent_reply","title":"Agent","text":"Goal accepted","char_count":13,"evidence_refs":[]}],"returned_count":2,"total_count":2,"has_more":false,"next_cursor":null,"source_revision":"sha256:messages","source_completeness":"enumerable","incomplete_reason":null,"scanned_bytes":1024,"scanned_through_bytes":1024,"snapshot_bytes":1024}}
    """

    private static let providerActivityPageResponse = """
    {"id":"test","ok":true,"result":{"generated_by":"local-v2.64","rows":[{"id":"activity-1","kind":"provider_call","timestamp":42,"title":"analyze","subtitle":"redacted metadata","status":"succeeded","evidence_refs":["provider-call:activity-1"]}],"source_revision":"sha256:activity-revision","returned_count":1,"total_count":130,"has_more":true,"next_cursor":"v1:activity-page-3","source_completeness":"enumerable","incomplete_reason":null,"safety_flags":{"provider_request_sent":false,"raw_prompt_persisted":false,"raw_response_persisted":false,"raw_trace_persisted":false}}}
    """

    private static let sendResponse = """
    {"id":"test","ok":true,"result":{"preview_id":"prompt-preview-task","success":true,"status":"succeeded","message":"Provider response received.","output_text":"{}","draft_copy_only":true,"raw_prompt_persisted":false,"raw_response_persisted":false,"write_back_allowed":false,"script_execution_allowed":false}}
    """

    private static let unknownMethodResponse = """
    {"id":"test","ok":false,"error":{"code":"unknown_method","message":"unknown method"}}
    """
}
