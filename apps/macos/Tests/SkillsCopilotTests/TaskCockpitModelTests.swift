import Foundation
@testable import SkillsCopilot

struct TaskCockpitModelTests {
    func run() throws {
        try decodesRealisticTaskCockpitPayload()
        try decodesAliasesAndStringForms()
        try classifiesFallbackAndPartialDiagnostics()
        try derivesPreparingProgressRowsFromOperationState()
        try derivesCompletedProgressRowsFromResultMetadata()
        try derivesFallbackProgressRowsWithoutUnsafeCapabilities()
        try parsesLooseProviderCandidateJSONWithoutLeakingRawOutput()
        try legacyDuplicateRowsReceiveStableUniqueDisplayIDs()
        try summaryCollectionsExposeEveryStableUniqueRow()
        try duplicateExternalSummaryIDsKeepEveryDisplayOccurrence()
        try signalClassificationHasSingleProductionContract()
    }

    private struct ServiceEnvelope<ResultPayload: Decodable>: Decodable {
        let id: String?
        let ok: Bool
        let result: ResultPayload?
    }

    private func decodesRealisticTaskCockpitPayload() throws {
        let data = Data(
            """
            {
              "id": "cockpit-1",
              "ok": true,
              "result": {
                "generated_by": "local-v2.65",
                "catalog_available": true,
                "filters": {
                  "task": "Prepare local release audit work.",
                  "agent": "claude-code",
                  "selected_skill_id": "beta",
                  "selected_skill_name": "Beta",
                  "selected_skill_agent": "claude-code",
                  "project_root": "<project-root>",
                  "current_cwd": "<project-cwd>",
                  "workspace": "Fixture Project",
                  "limit": "8",
                  "include_provider_observability": true
                },
                "summary": {
                  "task_text": "Prepare local release audit work.",
                  "summary": "Beta is the strongest route, but Codex coverage remains a gap.",
                  "route_candidate_count": 2,
                  "agent_candidate_count": 2,
                  "skill_candidate_count": 2,
                  "readiness_signal_count": 2,
                  "provider_call_count": 3,
                  "gap_count": 1,
                  "blocker_count": 1,
                  "evidence_count": 2,
                  "safety_flag_count": 1,
                  "recommended_agent": "claude-code",
                  "recommended_skill_name": "Beta",
                  "readiness_score": 78,
                  "routing_score": "88"
                },
                "route_candidates": [
                  {
                    "route_id": "route-beta",
                    "rank": 1,
                    "title": "Beta",
                    "agent": "claude-code",
                    "skill": {"instance_id":"beta","skill_name":"Beta","agent":"claude-code","definition_id":"def.beta"},
                    "readiness_score": 78,
                    "routing_score": 88,
                    "band": "High",
                    "status": "ready",
                    "summary": "Best local match for release audit.",
                    "match_reasons": ["Description matches audit work."],
                    "evidence_refs": [{"title":"Routing","detail":"route:beta"}],
                    "safety_flags": ["provider not sent"]
                  },
                  "route:alpha"
                ],
                "agent_candidates": [
                  {"agent_id":"agent-claude","title":"Claude Code","agent":"claude-code","score":82,"reasons":"Selected skill is enabled."}
                ],
                "skill_candidates": [
                  {"skill_id":"beta","name":"Beta","agent":"claude-code","readiness_score":"78","routing_score":"88"}
                ],
                "readiness_signals": [
                  {"id":"readiness-beta","title":"Readiness partial","detail":"Ready for local audit, missing release-note examples.","status":"partial","count":"1"}
                ],
                "provider_observability_context": [
                  {"id":"provider-1","title":"Provider calls observed","detail":"Three redacted call metadata rows.","count":3,"source":"llm.providerObservability"}
                ],
                "gap_rows": [{"title":"Codex coverage gap","detail":"No Codex project route.","severity":"warning","agent":"codex","evidence_refs":["workspace:codex-gap"]}],
                "blocker_rows": [{"title":"No apply path","detail":"Cockpit only recommends review surfaces.","severity":"info"}],
                "evidence_references": [
                  {"title":"Task preflight","detail":"Derived from selected agent, effective skill, provider, and local safety metadata.","source":"llm.confirmPromptAndSend","agent":"claude-code"}
                ],
                "prompt_request": {"enabled":false,"request_kind":"task_cockpit","summary":"No provider request is prepared or sent.","draft_copy_only":true,"redacted":true},
                "safety_flags": {
                  "provider_request_sent": false,
                  "write_back_allowed": false,
                  "write_actions_available": false,
                  "script_execution_allowed": false,
                  "execution_actions_available": false,
                  "config_mutation_allowed": false,
                  "snapshot_created": false,
                  "triage_mutation_allowed": false,
                  "credential_accessed": false,
                  "raw_prompt_persisted": false,
                  "raw_response_persisted": false,
                  "raw_trace_persisted": false,
                  "cloud_sync_enabled": false,
                  "telemetry_enabled": false,
                  "raw_secret_returned": false,
                  "notes": ["provider not sent"]
                }
              }
            }
            """.utf8
        )

        let envelope = try JSONDecoder().decode(ServiceEnvelope<TaskCockpitResult>.self, from: data)
        guard let result = envelope.result else {
            throw NativeModelTestFailure(description: "Task cockpit envelope should include a result.")
        }

        try expectEqual(envelope.ok, true, "Task cockpit envelope should decode ok.")
        try expectEqual(result.generatedBy, "local-v2.65", "Task cockpit should decode generator metadata.")
        try expectEqual(result.filters.taskText, "Prepare local release audit work.", "Task cockpit should decode task filter.")
        try expectEqual(result.filters.limit, 8, "Task cockpit should decode string limits.")
        try expectEqual(result.summary.recommendedAgent, "claude-code", "Task cockpit should decode recommended agent.")
        try expectEqual(result.summary.recommendedSkillName, "Beta", "Task cockpit should decode recommended skill.")
        try expectEqual(result.summary.routingScore, 88, "Task cockpit should decode string routing score.")
        try expectEqual(result.routeCandidates.count, 2, "Task cockpit should decode route candidates and string shorthand.")
        try expectEqual(result.routeCandidates.first?.skill?.name, "Beta", "Task cockpit route should decode skill refs.")
        try expectEqual(result.routeCandidates.first?.evidenceRefs, ["route:beta"], "Task cockpit route evidence should accept objects.")
        try expectEqual(result.routeCandidates[1].title, "route:alpha", "Task cockpit route should accept string shorthand.")
        try expectEqual(result.agentCandidates.first?.reasons, ["Selected skill is enabled."], "Task cockpit agent reasons should accept strings.")
        try expectEqual(result.skillCandidates.first?.routingScore, 88, "Task cockpit skill candidates should decode string scores.")
        try expectEqual(result.readinessSignals.first?.count, 1, "Task cockpit readiness signals should decode string counts.")
        try expectEqual(result.providerObservabilityContext.first?.count, 3, "Task cockpit should decode provider context.")
        try expectEqual(result.gapRows.first?.agent, "codex", "Task cockpit should decode gap rows.")
        try expectEqual(result.blockerRows.first?.title, "No apply path", "Task cockpit should decode blockers.")
        try expectEqual(result.evidenceReferences.first?.source, "llm.confirmPromptAndSend", "Task cockpit should decode evidence references.")
        try expectEqual(result.promptRequest?.requestKind, "task_cockpit", "Task cockpit should decode prompt metadata.")
        try expectFalse(result.safetyFlags.providerRequestSent, "Task cockpit must not send provider requests.")
        try expectFalse(result.safetyFlags.writeBackAllowed, "Task cockpit must not allow write-back.")
        try expectFalse(result.safetyFlags.writeActionsAvailable, "Task cockpit must not expose write actions.")
        try expectFalse(result.safetyFlags.scriptExecutionAllowed, "Task cockpit must not allow script execution.")
        try expectFalse(result.safetyFlags.executionActionsAvailable, "Task cockpit must not expose execution actions.")
        try expectFalse(result.safetyFlags.configMutationAllowed, "Task cockpit must not mutate config.")
        try expectFalse(result.safetyFlags.snapshotCreated, "Task cockpit must not create snapshots.")
        try expectFalse(result.safetyFlags.triageMutationAllowed, "Task cockpit must not mutate triage.")
        try expectFalse(result.safetyFlags.credentialAccessed, "Task cockpit must not access credentials.")
        try expectFalse(result.safetyFlags.rawPromptPersisted, "Task cockpit must not persist raw prompts.")
        try expectFalse(result.safetyFlags.rawResponsePersisted, "Task cockpit must not persist raw responses.")
        try expectFalse(result.safetyFlags.rawTracePersisted, "Task cockpit must not persist raw traces.")
        try expectFalse(result.safetyFlags.cloudSyncEnabled, "Task cockpit must not sync cloud data.")
        try expectFalse(result.safetyFlags.telemetryEnabled, "Task cockpit must not emit telemetry.")
    }

    private func decodesAliasesAndStringForms() throws {
        let json = """
        {
          "generatedBy": "local-v2.65",
          "catalogAvailable": true,
          "summary": "String summary works.",
          "routes": ["Beta"],
          "agents": "claude-code",
          "skills": [{"id":"beta","title":"Beta","agent":"claude-code","score":"80"}],
          "readiness": "partial",
          "provider_rows": "no provider sent",
          "gaps": "No Codex route.",
          "blockers": "No apply path.",
          "evidence": ["task-cockpit:evidence"],
          "promptRequest": {"enabled":false,"requestKind":"task_cockpit","draft_copy_only":true},
          "safety": ["provider not sent"]
        }
        """

        let result = try JSONDecoder().decode(TaskCockpitResult.self, from: Data(json.utf8))
        try expectEqual(result.generatedBy, "local-v2.65", "GeneratedBy alias should decode.")
        try expectEqual(result.summary.summaryText, "String summary works.", "String summary should decode.")
        try expectEqual(result.routeCandidates.first?.title, "Beta", "Routes alias should decode string rows.")
        try expectEqual(result.agentCandidates.first?.title, "claude-code", "Agents alias should decode string row.")
        try expectEqual(result.skillCandidates.first?.score, 80, "Skills alias should decode score strings.")
        try expectEqual(result.readinessSignals.first?.title, "partial", "Readiness alias should decode.")
        try expectEqual(result.providerObservabilityContext.first?.title, "no provider sent", "Provider alias should decode.")
        try expectEqual(result.gapRows.first?.title, "No Codex route.", "Gaps alias should decode.")
        try expectEqual(result.blockerRows.first?.title, "No apply path.", "Blockers alias should decode.")
        try expectEqual(result.evidenceReferences.first?.title, "task-cockpit:evidence", "String evidence should decode.")
        try expectEqual(result.promptRequest?.requestKind, "task_cockpit", "Prompt request camel-case alias should decode.")
        try expectEqual(result.safetyFlags.notes, ["provider not sent"], "Safety string array should decode.")
    }

    private func legacyDuplicateRowsReceiveStableUniqueDisplayIDs() throws {
        let json = """
        {
          "route_candidates": [
            {"title":"Same route","agent":"codex"},
            {"title":"Same route","agent":"codex"},
            {"id":"explicit-duplicate","title":"Explicit route"},
            {"id":"explicit-duplicate","title":"Explicit route"}
          ],
          "gap_rows": [
            {"title":"Same gap","detail":"Same detail"},
            {"title":"Same gap","detail":"Same detail"}
          ],
          "blocker_rows": ["Same blocker", "Same blocker"]
        }
        """

        let first = try JSONDecoder().decode(TaskCockpitResult.self, from: Data(json.utf8))
        let second = try JSONDecoder().decode(TaskCockpitResult.self, from: Data(json.utf8))
        try expectEqual(first.routeCandidates.map(\.id), second.routeCandidates.map(\.id), "External candidate logical IDs must remain stable across identical decodes.")
        try expectEqual(first.gapRows.map(\.id), second.gapRows.map(\.id), "External context logical IDs must remain stable across identical decodes.")
        let routeDisplay = OccurrenceIdentifiedItem.rows(for: first.routeCandidates)
        let gapDisplay = OccurrenceIdentifiedItem.rows(for: first.gapRows)
        let blockerDisplay = OccurrenceIdentifiedItem.rows(for: first.blockerRows)
        try expectEqual(Set(routeDisplay.map(\.id)).count, routeDisplay.count, "Legacy route display IDs must be unique.")
        try expectEqual(Set(gapDisplay.map(\.id)).count, gapDisplay.count, "Legacy gap display IDs must be unique.")
        try expectEqual(Set(blockerDisplay.map(\.id)).count, blockerDisplay.count, "Legacy blocker display IDs must be unique.")
    }

    private func summaryCollectionsExposeEveryStableUniqueRow() throws {
        let candidates = (0..<5).map { index in
            TaskCockpitCandidateRow(
                id: "candidate-\(index)",
                title: "Candidate \(index)",
                agent: "codex",
                score: 90 - index,
                summary: "Candidate summary \(index)",
                reasons: ["Candidate reason \(index)"]
            )
        }
        let gaps = (0..<3).map { index in
            TaskCockpitContextRow(
                id: "gap-\(index)",
                title: "Gap \(index)",
                detail: "Gap detail \(index)"
            )
        }
        let blockers = (0..<2).map { index in
            TaskCockpitContextRow(
                id: "blocker-\(index)",
                title: "Blocker \(index)",
                detail: "Blocker detail \(index)"
            )
        }
        let result = TaskCockpitResult(
            summary: TaskCockpitSummary(
                summaryText: "Complete decision summary",
                recommendedAgent: "codex",
                recommendedSkillName: "Candidate 0"
            ),
            routeCandidates: candidates,
            agentCandidates: candidates,
            skillCandidates: candidates,
            gapRows: gaps,
            blockerRows: blockers
        )

        let decision = TaskCockpitDecisionModel(result: result)
        try expectFalse(!decision.keyReasons.contains("Candidate reason 4"), "Production decision reasons must retain the final candidate reason.")
        try expectEqual(decision.candidateAlternatives.count, 5, "Production candidate alternatives must keep every unique candidate.")
        try expectFalse(!decision.candidateAlternatives.last!.contains("Candidate 4"), "Production candidate alternatives must retain the final candidate.")

        let processNotes = decision.processNotes
        try expectEqual(processNotes.count, 6, "Matching-process notes must keep top-route reasons plus every gap and blocker detail.")
        try expectEqual(processNotes.last, "Blocker detail 1", "Matching-process notes must retain the final source row.")

        let values = ["Alpha", "Beta", "Alpha", "Gamma", "Delta"]
        let first = TaskCockpitSummaryTextRow.rows(for: values)
        let second = TaskCockpitSummaryTextRow.rows(for: values)
        try expectEqual(first.count, values.count, "Summary presentation rows must retain every input value.")
        try expectEqual(Set(first.map(\.id)).count, values.count, "Duplicate summary values must receive unique IDs.")
        try expectEqual(first.map(\.id), second.map(\.id), "Summary row IDs must be stable across identical inputs.")
    }

    private func duplicateExternalSummaryIDsKeepEveryDisplayOccurrence() throws {
        let candidates = [
            TaskCockpitCandidateRow(id: "external", title: "First external row"),
            TaskCockpitCandidateRow(id: "external", title: "Second external row")
        ]
        let provider = [
            TaskCockpitContextRow(id: "provider", title: "First provider row"),
            TaskCockpitContextRow(id: "provider", title: "Second provider row")
        ]

        let candidateRows = OccurrenceIdentifiedItem.rows(for: candidates)
        let providerRows = OccurrenceIdentifiedItem.rows(for: provider)
        try expectEqual(candidateRows.map(\.value.title), ["First external row", "Second external row"], "Task Cockpit candidate summaries must retain duplicate external logical IDs.")
        try expectEqual(providerRows.map(\.value.title), ["First provider row", "Second provider row"], "Task Cockpit provider summaries must retain duplicate provider logical IDs.")
        try expectEqual(candidateRows.map(\.id.occurrence), [0, 1], "Candidate display IDs must use logical-ID occurrences.")
        try expectEqual(providerRows.map(\.id.occurrence), [0, 1], "Provider display IDs must use logical-ID occurrences.")
    }

    private func signalClassificationHasSingleProductionContract() throws {
        let review = TaskCockpitContextRow(
            id: "fixture",
            title: "Review",
            safetyFlags: ["permissions.exec-needs-human"]
        )
        let internalBoundary = TaskCockpitContextRow(
            id: "provider-observability-skipped",
            title: "Internal boundary"
        )
        let userFacing = TaskCockpitContextRow(
            id: "user-facing-gap",
            title: "User-facing gap",
            source: "task.preflight"
        )

        try expectEqual(TaskCockpitSignalClassifier.classification(for: review), .reviewOnlyRisk, "Review-only risk tokens must use the shared production classifier.")
        try expectEqual(TaskCockpitSignalClassifier.classification(for: internalBoundary), .internalBoundary, "Internal-boundary tokens must use the shared production classifier.")
        try expectEqual(TaskCockpitSignalClassifier.classification(for: userFacing), .userFacing, "Unclassified source rows must remain user-facing.")
        try expectEqual(TaskCockpitSignalClassifier.normalizedToken(" Permissions_Exec Needs Human "), "permissions-exec-needs-human", "Token normalization must remain one production contract.")
    }

    private func classifiesFallbackAndPartialDiagnostics() throws {
        let fallbackJSON = """
        {
          "generated_by": "local-v2.65",
          "catalog_available": true,
          "summary": {"task_text": "Review release audit", "summary": "Fallback metadata only."},
          "fallback_reason": "Readiness subcall timed out; showing local fallback metadata.",
          "safety_flags": {"provider_request_sent": false, "write_back_allowed": false}
        }
        """
        let fallback = try JSONDecoder().decode(TaskCockpitResult.self, from: Data(fallbackJSON.utf8))
        try expectEqual(fallback.recoveryDiagnosticReason, "Readiness subcall timed out; showing local fallback metadata.", "Fallback reason should drive the recovery diagnostic.")

        let partialJSON = """
        {
          "generated_by": "local-v2.65",
          "catalog_available": false,
          "summary": {"task_text": "Review release audit", "summary": "Catalog unavailable."},
          "safety_flags": {"provider_request_sent": false, "write_back_allowed": false}
        }
        """
        let partial = try JSONDecoder().decode(TaskCockpitResult.self, from: Data(partialJSON.utf8))
        try expectEqual(partial.recoveryDiagnosticReason, UIStrings.taskCockpitCatalogUnavailableDiagnostic, "Catalog-unavailable payloads should surface a diagnostic even without fallback_reason.")
    }

    private func derivesPreparingProgressRowsFromOperationState() throws {
        let startedAt = Date(timeIntervalSinceReferenceDate: 1_000)
        let now = Date(timeIntervalSinceReferenceDate: 1_004)
        let state = TaskCockpitOperationState.preparing(
            taskText: "Build a local skill routing cockpit.",
            startedAt: startedAt,
            timeoutSeconds: 14
        )
        let snapshot = TaskCockpitProgressSnapshot(operationState: state, result: nil, now: now)

        try expectEqual(snapshot.stageRows.count, TaskCockpitProgressSnapshot.maximumStageCount, "Progress rows should stay bounded to the fixed cockpit stages.")
        try expectEqual(
            snapshot.stageRows.map(\.stage),
            [.readiness, .routing, .crossAgent, .actionReview, .batchChecks, .provider],
            "Progress rows should preserve the stage order expected by the cockpit UI."
        )
        try expectEqual(snapshot.activeStage, .routing, "Preparing progress should derive an active stage from elapsed bounded timeout.")
        try expectEqual(snapshot.row(for: .routing)?.state, .active, "The derived active stage should be marked active.")
        try expectEqual(snapshot.row(for: .readiness)?.state, .queued, "Non-active loading stages should stay queued rather than pretending to be complete.")
        try expectEqual(Int(snapshot.estimatedProgress * 100), 28, "Preparing progress should be clamped and derived from elapsed timeout.")
        try expectEqual(snapshot.taskText, "Build a local skill routing cockpit.", "Progress snapshot should keep the exact operation task text.")
    }

    private func derivesCompletedProgressRowsFromResultMetadata() throws {
        let result = TaskCockpitResult(
            generatedBy: "local-v2.76",
            catalogAvailable: true,
            filters: TaskCockpitFilters(taskText: "Review local release readiness."),
            summary: TaskCockpitSummary(
                taskText: "Review local release readiness.",
                routeCandidateCount: 1,
                agentCandidateCount: 1,
                skillCandidateCount: 1,
                readinessSignalCount: 1,
                providerCallCount: 1,
                recommendedAgent: "codex",
                recommendedSkillName: "Fixture",
                readinessScore: 82,
                routingScore: 76
            ),
            cockpitSections: [
                TaskCockpitContextRow(id: "action-review", title: "Action review", detail: "Open read-only checks.", source: "task.preflight.actionReview", evidenceRefs: ["action-review:evidence"])
            ],
            routeCandidates: [
                TaskCockpitCandidateRow(id: "route", title: "Route", routingScore: 76, summary: "Routing is medium.", evidenceRefs: ["routing:evidence"])
            ],
            agentCandidates: [
                TaskCockpitCandidateRow(id: "agent", title: "Codex", agent: "codex", score: 80, summary: "Agent row is available.", evidenceRefs: ["agent:evidence"])
            ],
            skillCandidates: [
                TaskCockpitCandidateRow(id: "skill", title: "Fixture", agent: "codex", score: 78, evidenceRefs: ["skill:evidence"])
            ],
            readinessSignals: [
                TaskCockpitContextRow(id: "readiness", title: "Readiness", detail: "Readiness is mostly ready.", evidenceRefs: ["readiness:evidence"])
            ],
            providerObservabilityContext: [
                TaskCockpitContextRow(id: "provider", title: "Provider", detail: "Provider metadata only.", evidenceRefs: ["provider:evidence"])
            ],
            aggregation: TaskCockpitAggregation(
                status: "complete",
                elapsedMS: 12,
                timeoutMS: 1_500,
                completedStages: [
                    "task-readiness",
                    "routing",
                    "agent-comparison",
                    "action-review",
                    "batch-review",
                    "provider-observability"
                ]
            )
        )
        let operationState = TaskCockpitOperationState.preparing(
            taskText: "Review local release readiness.",
            startedAt: Date(timeIntervalSinceReferenceDate: 2_000),
            timeoutSeconds: 15
        ).finished(
            phase: .completed,
            message: UIStrings.taskCockpitLoaded,
            finishedAt: Date(timeIntervalSinceReferenceDate: 2_001)
        )

        let snapshot = TaskCockpitProgressSnapshot(operationState: operationState, result: result)
        try expectEqual(snapshot.completedStageCount, TaskCockpitProgressSnapshot.maximumStageCount, "Completed aggregation metadata should mark every known cockpit stage complete.")
        try expectEqual(snapshot.row(for: .readiness)?.state, .completed, "Readiness rows should be complete when result evidence exists.")
        try expectEqual(snapshot.row(for: .readiness)?.score, 82, "Readiness stage should surface the readiness score.")
        try expectEqual(snapshot.row(for: .routing)?.count, 1, "Routing stage should infer a bounded route count from route or skill candidates.")
        try expectEqual(snapshot.row(for: .routing)?.score, 76, "Routing stage should surface the routing score.")
        try expectEqual(snapshot.row(for: .batchChecks)?.state, .completed, "Batch review should be derived from aggregation metadata even without a separate service row.")
        try expectEqual(snapshot.row(for: .batchChecks)?.count, 0, "Batch review should not invent rows when only stage metadata is present.")
        try expectEqual(snapshot.row(for: .provider)?.safetyFlagsClear, true, "Progress rows should preserve the read-only safety boundary.")
    }

    private func derivesFallbackProgressRowsWithoutUnsafeCapabilities() throws {
        let fallbackReason = "Readiness subcall timed out; showing local fallback metadata."
        let result = TaskCockpitResult(
            generatedBy: "local-v2.76",
            catalogAvailable: true,
            filters: TaskCockpitFilters(taskText: "Review local fallback."),
            summary: TaskCockpitSummary(taskText: "Review local fallback.", readinessScore: 60),
            readinessSignals: [
                TaskCockpitContextRow(id: "readiness", title: "Readiness fallback", detail: "Partial readiness metadata.")
            ],
            aggregation: TaskCockpitAggregation(partial: true, fallbackUsed: true, completedStages: ["task-readiness"]),
            fallbackReason: fallbackReason
        )
        let operationState = TaskCockpitOperationState.preparing(
            taskText: "Review local fallback.",
            startedAt: Date(timeIntervalSinceReferenceDate: 3_000),
            timeoutSeconds: 15
        ).finished(
            phase: .fallback,
            message: UIStrings.taskCockpitLoadedWithFallback(fallbackReason),
            finishedAt: Date(timeIntervalSinceReferenceDate: 3_001)
        )

        let snapshot = TaskCockpitProgressSnapshot(operationState: operationState, result: result)
        try expectEqual(snapshot.diagnostic, fallbackReason, "Fallback progress should expose the service recovery diagnostic.")
        try expectEqual(snapshot.row(for: .readiness)?.state, .fallback, "Stages with returned fallback evidence should be marked fallback.")
        try expectEqual(snapshot.row(for: .routing)?.state, .unavailable, "Missing stages in a fallback result should be unavailable, not complete.")
        try expectEqual(snapshot.stageRows.allSatisfy(\.safetyFlagsClear), true, "Fallback progress rows must not introduce provider, write, script, credential, cloud, or telemetry affordances.")
    }

    private func parsesLooseProviderCandidateJSONWithoutLeakingRawOutput() throws {
        let output = """
        {
          "agent_candidates": [
            {
              "agent_id": "claude-code",
              "title": "Claude Code",
              "score": 73,
              "summary": "任务提到 ALB 指标与错误，Claude Code 有可用阿里云技能候选。",
              "reasons": ["任务包含明确的 ALB 产品和指标/错误查询意图。"]
            }
          ],
          "skill_candidates": [
            {
              "skill_id": "alibabacloud-cms-alert-rule-create",
              "name": "alibabacloud-cms-alert-rule-create",
              "agent": "claude-code",
              "score": 61,
              "routing_score": 61,
              "summary": "候选与云监控相关，但仍需确认是否覆盖 ALB 指标读取。"
            }
          ],
          "gap_rows": [
            {
              "title": "缺少查询边界",
              "detail": "建议补充地域、实例或时间范围。"
            }
          ]
        }
        """

        let result = TaskCockpitProviderOutputParser.result(
            from: output,
            taskText: "查看下阿里云 ALB 指标与错误情况",
            agentIDs: ["claude-code", "codex"]
        )

        try expectFalse(result.isUnavailable, "Loose provider candidate JSON should recover a usable preflight result.")
        try expectEqual(result.summary.recommendedAgent, "claude-code", "Loose provider result should infer the recommended agent from candidates.")
        try expectEqual(result.agentCandidates.count, 1, "Loose provider result should recover agent candidates.")
        try expectEqual(result.skillCandidates.first?.title, "alibabacloud-cms-alert-rule-create", "Loose provider result should recover skill candidates.")
        try expectEqual(result.gapRows.first?.title, "缺少查询边界", "Loose provider result should recover gap rows.")
        try expectFalse(
            result.summary.summaryText.contains("agent_candidates"),
            "Loose provider fallback must not leak raw JSON into the user-facing summary."
        )
    }
}
