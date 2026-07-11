import Foundation
@testable import SkillsCopilot

struct ProviderObservabilityModelTests {
    func run() throws {
        try decodesRealisticProviderObservabilityPayload()
        try decodesAliasAndStringForms()
        try normalizesSummaryCallCountAcrossPromptRunsAndCallRows()
        try decodesGroupingRowsIntoDimensionLists()
        try decodesServiceProtocolFixture()
        try decodesProviderActivityPageFixtureAndAliases()
        try emptyServiceFixtureUsesDashboardEmptyState()
    }

    private struct ServiceEnvelope<ResultPayload: Decodable>: Decodable {
        let id: String?
        let ok: Bool
        let result: ResultPayload?
    }

    private func decodesRealisticProviderObservabilityPayload() throws {
        let data = Data(
            """
            {
              "id": "obs-1",
              "ok": true,
              "result": {
                "generated_by": "local-v2.64",
                "app_local_only": true,
                "metadata_redacted": true,
                "filters": {
                  "window_days": "30",
                  "start_at": 1780272000000,
                  "end_at": 1782863999999,
                  "limit": 30,
                  "include_history": true,
                  "include_budget_hints": true,
                  "include_retention_recommendations": true,
                  "include_evidence": true,
                  "aggregation_uses_full_range": true
                },
                "summary": {
                  "call_count": "3",
                  "success_count": 1,
                  "failure_count": 1,
                  "blocked_count": 1,
                  "provider_count": 1,
                  "model_count": 2,
                  "destination_count": 1,
                  "error_count": 1,
                  "estimated_input_tokens": 980,
                  "estimated_output_tokens": 320,
                  "estimated_total_tokens": 1300,
                  "estimated_cost_usd": "0.041",
                  "total_duration_ms": 1800,
                  "average_duration_ms": 600,
                  "budget_hint_count": 1,
                  "retention_recommendation_count": 2,
                  "summary": "Three redacted provider-call metadata rows were reviewed locally."
                },
                "call_rows": [
                  {
                    "id": "call-1",
                    "preview_id": "preview-1",
                    "confirmation_id": "confirm-1",
                    "request_kind": "task_cockpit",
                    "action": "task_cockpit",
                    "provider": "openai-compatible",
                    "model": "gpt-5",
                    "destination_host": "llm.example.com",
                    "status": "succeeded",
                    "duration_ms": 720,
                    "input_tokens": 420,
                    "output_tokens": 120,
                    "total_tokens": 540,
                    "estimated_cost_usd": 0.014,
                    "completed_at": 1781260000000,
                    "draft_copy_only": true,
                    "provider_request_sent": true,
                    "credential_accessed": false,
                    "raw_prompt_persisted": false,
                    "raw_response_persisted": false,
                    "raw_secret_returned": false,
                    "evidence_refs": ["prompt-run:preview-1"],
                    "safety_flags": ["copy-only", "raw prompt not stored"],
                    "detail": "Provider response metadata was stored without raw prompt or response."
                  },
                  {
                    "id": "call-2",
                    "request_kind": "analyze",
                    "provider": "openai-compatible",
                    "model": "gpt-5-mini",
                    "destination_host": "llm.example.com",
                    "status": "failed",
                    "error_code": "timeout",
                    "error_message": "Provider request timed out.",
                    "duration_ms": 1080,
                    "input_tokens": 560,
                    "output_tokens": 0,
                    "total_tokens": 560,
                    "estimated_cost_usd": 0.027,
                    "draft_copy_only": true,
                    "provider_request_sent": true,
                    "credential_accessed": false,
                    "raw_prompt_persisted": false,
                    "raw_response_persisted": false,
                    "raw_secret_returned": false,
                    "evidence": [{"title":"Prompt run","detail":"timeout metadata only","source":"llm.confirmPromptAndSend"}],
                    "safety_flags": "raw response not stored"
                  }
                ],
                "provider_rows": [
                  {
                    "kind": "provider",
                    "label": "OpenAI-compatible",
                    "provider": "openai-compatible",
                    "call_count": 3,
                    "success_count": 1,
                    "failure_count": 1,
                    "blocked_count": 1,
                    "estimated_tokens": 1300,
                    "estimated_cost_usd": 0.041,
                    "average_duration_ms": 600,
                    "status": "partial",
                    "notes": ["One timeout and one blocked local preview."],
                    "evidence_refs": ["provider:openai-compatible"]
                  }
                ],
                "model_rows": [
                  {"kind":"model","label":"gpt-5","model":"gpt-5","call_count":1,"success_count":1,"estimated_tokens":540,"status":"ok"},
                  {"kind":"model","label":"gpt-5-mini","model":"gpt-5-mini","call_count":1,"failure_count":1,"estimated_tokens":560,"status":"warning"}
                ],
                "destination_rows": [
                  {"kind":"destination","label":"llm.example.com","destination_host":"llm.example.com","call_count":2,"status":"partial"}
                ],
                "model_task_history_rows": [
                  {
                    "id": "model-task:fixture",
                    "source": "model-task-matches.json",
                    "source_kind": "manual",
                    "title": "Release audit model fit",
                    "task": "Review local release audit evidence.",
                    "task_kind": "task_cockpit",
                    "agent": "codex",
                    "provider": "openai-compatible",
                    "model": "gpt-5",
                    "destination_host": "llm.example.com",
                    "match_status": "fit",
                    "confidence_score": 88,
                    "status": "fit",
                    "latency_ms": 720,
                    "estimated_total_tokens": 540,
                    "estimated_cost_usd": 0.014,
                    "gap_notes": [],
                    "blocker_notes": [],
                    "outcome_notes": ["The model was recorded as a fit for release audit work."],
                    "evidence_refs": ["prompt-run:preview-1"],
                    "redaction_status": "redacted-local-only",
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
                      "raw_secret_returned": false
                    }
                  }
                ],
                "status_rows": [
                  {"severity":"info","status":"succeeded","title":"Succeeded","detail":"One call completed.","count":1},
                  {"severity":"warning","status":"blocked","title":"Blocked locally","detail":"One preview never sent a provider request.","count":1}
                ],
                "error_rows": [
                  {"severity":"warning","status":"failed","title":"Timeout","detail":"Provider request timed out.","count":1,"provider":"openai-compatible","model":"gpt-5-mini","evidence_refs":["prompt-run:timeout"]}
                ],
                "budget_hints": [
                  {"severity":"info","title":"Monthly budget healthy","detail":"Estimated spend is below the configured budget.","value":"0.041","threshold":"25.00","recommendation":"Keep monitoring prompt-run history."}
                ],
                "usage_hints": [
                  {"severity":"info","title":"Token usage available","detail":"Estimated token totals are derived from redacted metadata.","value":"1300"}
                ],
                "retention_rows": [
                  {"severity":"info","title":"Retain metadata only","detail":"Keep redacted prompt-run metadata; do not retain raw prompts.","recommendation":"Review old metadata periodically."}
                ],
                "cleanup_recommendations": [
                  {"severity":"info","title":"No cleanup required","detail":"No unsafe raw prompt or response payloads were observed."}
                ],
                "gap_notes": ["No raw response bodies are available for observability by design."],
                "blocker_notes": [],
                "evidence_references": [
                  {"title":"Prompt run history","detail":"Read from app-local prompt-runs metadata.","source":"llm.listPromptRuns"}
                ],
                "prompt_request": {
                  "enabled": false,
                  "request_kind": "provider_observability",
                  "summary": "No provider request is prepared or sent by observability.",
                  "draft_copy_only": true,
                  "redacted": true
                },
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
                  "notes": ["observability did not send a provider request"]
                }
              }
            }
            """.utf8
        )

        let envelope = try JSONDecoder().decode(ServiceEnvelope<ProviderObservabilityResult>.self, from: data)
        guard let result = envelope.result else {
            throw NativeModelTestFailure(description: "Provider observability envelope should include a result.")
        }

        try expectEqual(envelope.ok, true, "Provider observability envelope should decode ok.")
        try expectEqual(result.generatedBy, "local-v2.64", "Provider observability should decode generator metadata.")
        try expectEqual(result.appLocalOnly, true, "Provider observability should decode app-local boundary.")
        try expectEqual(result.metadataRedacted, true, "Provider observability should decode redaction boundary.")
        try expectEqual(result.filters.windowDays, 30, "Provider observability should decode string window days.")
        try expectEqual(result.filters.startAt, 1780272000000, "Provider observability should decode start date filters.")
        try expectEqual(result.filters.endAt, 1782863999999, "Provider observability should decode end date filters.")
        try expectEqual(result.filters.aggregationUsesFullRange, true, "Provider observability should decode full-range aggregation metadata.")
        try expectEqual(result.summary.callCount, 3, "Provider observability should decode call count.")
        try expectEqual(result.summary.estimatedTotalTokens, 1300, "Provider observability should decode token total.")
        try expectEqual(result.summary.estimatedCostUSD, 0.041, "Provider observability should decode string cost.")
        try expectEqual(result.callRows.count, 2, "Provider observability should decode call rows.")
        try expectEqual(result.callRows[1].errorCode, "timeout", "Provider observability should decode error codes.")
        try expectEqual(result.callRows[1].evidenceRefs, ["timeout metadata only"], "Provider observability should accept object evidence refs.")
        try expectEqual(result.providerRows.first?.label, "OpenAI-compatible", "Provider observability should decode provider rows.")
        try expectEqual(result.modelRows.count, 2, "Provider observability should decode model rows.")
        try expectEqual(result.destinationRows.first?.destinationHost, "llm.example.com", "Provider observability should decode destination rows.")
        try expectEqual(result.modelTaskHistoryRows.first?.title, "Release audit model fit", "Provider observability should decode model-task history rows.")
        try expectEqual(result.modelTaskHistoryRows.first?.confidenceScore, 88, "Provider observability should decode model-task confidence.")
        try expectFalse(result.modelTaskHistoryRows.first?.safetyFlags.providerRequestSent ?? true, "Model-task history rows must not send provider requests.")
        try expectEqual(result.statusRows.count, 2, "Provider observability should decode status rows.")
        try expectEqual(result.errorRows.first?.title, "Timeout", "Provider observability should decode error rows.")
        try expectEqual(result.budgetHints.first?.title, "Monthly budget healthy", "Provider observability should decode budget hints.")
        try expectEqual(result.usageHints.first?.value, "1300", "Provider observability should decode usage hints.")
        try expectEqual(result.retentionRows.first?.recommendation, "Review old metadata periodically.", "Provider observability should decode retention recommendations.")
        try expectEqual(result.cleanupRecommendationRows.first?.title, "No cleanup required", "Provider observability should decode cleanup recommendations.")
        try expectEqual(result.gapNotes.first, "No raw response bodies are available for observability by design.", "Provider observability should decode gap notes.")
        try expectEqual(result.evidenceReferences.first?.source, "llm.listPromptRuns", "Provider observability should decode evidence sources.")
        try expectEqual(result.promptRequest?.requestKind, "provider_observability", "Provider observability should decode prompt metadata.")
        try expectFalse(result.safetyFlags.providerRequestSent, "Provider observability must not send provider requests.")
        try expectFalse(result.safetyFlags.writeBackAllowed, "Provider observability must not allow write-back.")
        try expectFalse(result.safetyFlags.writeActionsAvailable, "Provider observability must not expose write actions.")
        try expectFalse(result.safetyFlags.scriptExecutionAllowed, "Provider observability must not allow script execution.")
        try expectFalse(result.safetyFlags.executionActionsAvailable, "Provider observability must not expose execution actions.")
        try expectFalse(result.safetyFlags.configMutationAllowed, "Provider observability must not mutate config.")
        try expectFalse(result.safetyFlags.snapshotCreated, "Provider observability must not create snapshots.")
        try expectFalse(result.safetyFlags.triageMutationAllowed, "Provider observability must not mutate triage.")
        try expectFalse(result.safetyFlags.credentialAccessed, "Provider observability must not access credentials.")
        try expectFalse(result.safetyFlags.rawPromptPersisted, "Provider observability must not persist raw prompts.")
        try expectFalse(result.safetyFlags.rawResponsePersisted, "Provider observability must not persist raw responses.")
        try expectFalse(result.safetyFlags.rawTracePersisted, "Provider observability must not persist raw traces.")
        try expectFalse(result.safetyFlags.cloudSyncEnabled, "Provider observability must not sync cloud data.")
        try expectFalse(result.safetyFlags.telemetryEnabled, "Provider observability must not emit telemetry.")
        try expectFalse(result.safetyFlags.rawSecretReturned, "Provider observability must not expose secrets.")
        try expectFalse(result.isDashboardEmpty, "Provider observability with call metadata should not render the empty dashboard state.")
    }

    private func decodesAliasAndStringForms() throws {
        let json = """
        {
          "generatedBy": "local-v2.64",
          "local_only": true,
          "redacted": true,
          "summary": "Alias summary works.",
          "calls": ["prompt-run:one"],
          "providers": "openai-compatible",
          "models": [{"name":"gpt-5","calls":"2","estimatedTokens":"900","status":"ok"}],
          "destinations": ["llm.example.com"],
          "statuses": "succeeded",
          "errors": "timeout",
          "budgetHints": "Budget metadata only.",
          "usageHints": [{"title":"Token estimate","value":900}],
          "retention": "Keep metadata, never raw prompts.",
          "cleanup_recommendations": "No unsafe payloads.",
          "gaps": "No raw prompt text by design.",
          "blockers": "None.",
          "evidence": ["prompt-runs:redacted"],
          "promptRequest": {"enabled":false,"kind":"provider_observability","copyOnly":true},
          "safety": ["provider not sent"]
        }
        """

        let result = try JSONDecoder().decode(ProviderObservabilityResult.self, from: Data(json.utf8))

        try expectEqual(result.generatedBy, "local-v2.64", "GeneratedBy alias should decode.")
        try expectEqual(result.appLocalOnly, true, "local_only alias should decode.")
        try expectEqual(result.metadataRedacted, true, "redacted alias should decode.")
        try expectEqual(result.summary.summaryText, "Alias summary works.", "String summary should decode.")
        try expectEqual(result.callRows.first?.id, "call:prompt-run:one", "Call string shorthand should decode.")
        try expectEqual(result.providerRows.first?.label, "openai-compatible", "Provider string shorthand should decode.")
        try expectEqual(result.modelRows.first?.estimatedTokens, 900, "Model row should decode numeric strings.")
        try expectEqual(result.destinationRows.first?.label, "llm.example.com", "Destination string shorthand should decode.")
        try expectEqual(result.statusRows.first?.title, "succeeded", "Status string shorthand should decode.")
        try expectEqual(result.errorRows.first?.title, "timeout", "Error string shorthand should decode.")
        try expectEqual(result.budgetHints.first?.title, "Budget metadata only.", "Budget string shorthand should decode.")
        try expectEqual(result.usageHints.first?.value, "900", "Usage scalar values should decode.")
        try expectEqual(result.retentionRows.first?.title, "Keep metadata, never raw prompts.", "Retention string shorthand should decode.")
        try expectEqual(result.cleanupRecommendationRows.first?.title, "No unsafe payloads.", "Cleanup string shorthand should decode.")
        try expectEqual(result.gapNotes, ["No raw prompt text by design."], "Gap string shorthand should decode.")
        try expectEqual(result.blockerNotes, ["None."], "Blocker string shorthand should decode.")
        try expectEqual(result.evidenceReferences.first?.title, "prompt-runs:redacted", "Evidence string shorthand should decode.")
        try expectEqual(result.promptRequest?.requestKind, "provider_observability", "Prompt request kind alias should decode.")
        try expectFalse(result.safetyFlags.providerRequestSent, "Array safety shorthand should keep provider request false.")
        try expectFalse(result.isDashboardEmpty, "Decoded call and hint rows should make the dashboard non-empty.")
    }

    private func normalizesSummaryCallCountAcrossPromptRunsAndCallRows() throws {
        let serviceSummaryJSON = """
        {
          "returned_prompt_run_count": 9,
          "returned_call_row_count": 15,
          "total_prompt_run_count": 19,
          "total_call_metadata_count": 15,
          "succeeded_count": 19,
          "failed_count": 5
        }
        """
        let serviceSummary = try JSONDecoder().decode(ProviderObservabilitySummary.self, from: Data(serviceSummaryJSON.utf8))

        try expectEqual(serviceSummary.callCount, 24, "Provider observability should count returned prompt runs and call rows together.")
        try expectEqual(serviceSummary.successCount, 19, "Provider observability should keep succeeded status counts.")
        try expectEqual(serviceSummary.failureCount, 5, "Provider observability should keep failed status counts.")

        let legacySummaryJSON = """
        {
          "total_call_metadata_count": 15,
          "succeeded_count": 19,
          "failed_count": 5
        }
        """
        let legacySummary = try JSONDecoder().decode(ProviderObservabilitySummary.self, from: Data(legacySummaryJSON.utf8))

        try expectEqual(legacySummary.callCount, 24, "Provider observability call count should never be below success and failure totals.")
    }

    private func decodesGroupingRowsIntoDimensionLists() throws {
        let json = """
        {
          "generated_by": "local-v2.64",
          "grouping_rows": [
            {
              "id": "group:one",
              "provider": "openai-compatible",
              "model": "gpt-5",
              "destination_host": "llm.example.com",
              "prompt_run_count": 2,
              "call_metadata_count": 1,
              "succeeded_count": 2,
              "failed_count": 1,
              "estimated_total_tokens": 900,
              "estimated_cost_usd": 0.09,
              "evidence_refs": ["group:one"]
            },
            {
              "id": "group:two",
              "provider": "openai-compatible",
              "model": "gpt-5-mini",
              "destination_host": "llm.example.com",
              "prompt_run_count": 1,
              "call_metadata_count": 0,
              "succeeded_count": 1,
              "failed_count": 0,
              "estimated_total_tokens": 300,
              "estimated_cost_usd": 0.03,
              "evidence_refs": ["group:two"]
            }
          ]
        }
        """

        let result = try JSONDecoder().decode(ProviderObservabilityResult.self, from: Data(json.utf8))

        try expectEqual(result.providerRows.count, 1, "Grouping rows should aggregate provider dimensions.")
        try expectEqual(result.providerRows.first?.label, "openai-compatible", "Provider grouping should keep provider label.")
        try expectEqual(result.providerRows.first?.callCount, 4, "Provider grouping should sum prompt-run and call metadata counts.")
        try expectEqual(result.providerRows.first?.successCount, 3, "Provider grouping should sum success counts.")
        try expectEqual(result.providerRows.first?.failureCount, 1, "Provider grouping should sum failure counts.")
        try expectEqual(result.providerRows.first?.estimatedTokens, 1200, "Provider grouping should sum tokens.")
        try expectEqual(result.providerRows.first?.status, "partial", "Provider grouping should flag partial status when failures exist.")
        try expectEqual(result.modelRows.count, 2, "Grouping rows should project model dimensions.")
        try expectEqual(result.destinationRows.count, 1, "Grouping rows should aggregate destination dimensions.")
        try expectEqual(result.destinationRows.first?.evidenceRefs, ["group:one", "group:two"], "Destination grouping should retain evidence refs.")
    }

    private func decodesServiceProtocolFixture() throws {
        let fixtureURL = try repositoryRoot()
            .appendingPathComponent("fixtures/service-protocol/llm.providerObservability.response.json")
        let data = try Data(contentsOf: fixtureURL)
        let envelope = try JSONDecoder().decode(ServiceEnvelope<ProviderObservabilityResult>.self, from: data)
        guard let result = envelope.result else {
            throw NativeModelTestFailure(description: "Provider observability fixture should include a result.")
        }

        try expectEqual(envelope.ok, true, "Provider observability fixture envelope should decode ok.")
        try expectEqual(result.generatedBy, "local-v2.64", "Provider observability should decode service generator metadata.")
        try expectEqual(result.summary.callCount, 0, "Provider observability should decode empty service call counts.")
        try expectEqual(result.summary.estimatedTotalTokens, 0, "Provider observability should decode service token totals.")
        try expectEqual(result.statusRows.count, 3, "Provider observability should decode service status rows.")
        try expectEqual(result.retentionRows.count, 2, "Provider observability should decode service retention recommendations.")
        try expectEqual(result.retentionRows.first?.title, "prompt-runs.json", "Provider observability should title retention rows from source_file.")
        try expectEqual(result.retentionRows.first?.value, "0", "Provider observability should expose retention record counts.")
        try expectEqual(result.retentionRows.first?.recommendation, "No prompt run metadata cleanup is needed.", "Provider observability should decode retention recommendation text.")
        try expectEqual(result.evidenceReferences.count, 3, "Provider observability should decode service evidence references.")
        try expectEqual(result.promptRequest?.requestKind, "provider_observability", "Provider observability should decode service prompt metadata.")
        try expectEqual(result.promptRequest?.copyOnly, true, "Provider observability prompt metadata should remain copy-only.")
        try expectContains(result.promptRequest?.summary, "never sends provider traffic", "Provider observability should surface provider-not-sent prompt metadata.")
        try expectFalse(result.safetyFlags.providerRequestSent, "Provider observability fixture must keep provider request false.")
        try expectFalse(result.safetyFlags.writeBackAllowed, "Provider observability fixture must keep writes blocked.")
        try expectFalse(result.safetyFlags.scriptExecutionAllowed, "Provider observability fixture must keep scripts blocked.")
        try expectFalse(result.safetyFlags.credentialAccessed, "Provider observability fixture must not access credentials.")
        try expectFalse(result.safetyFlags.rawPromptPersisted, "Provider observability fixture must not persist raw prompts.")
        try expectFalse(result.safetyFlags.rawResponsePersisted, "Provider observability fixture must not persist raw responses.")
        try expectFalse(result.safetyFlags.cloudSyncEnabled, "Provider observability fixture must keep cloud sync blocked.")
        try expectFalse(result.safetyFlags.telemetryEnabled, "Provider observability fixture must keep telemetry blocked.")
    }

    private func emptyServiceFixtureUsesDashboardEmptyState() throws {
        let fixtureURL = try repositoryRoot()
            .appendingPathComponent("fixtures/service-protocol/llm.providerObservability.response.json")
        let data = try Data(contentsOf: fixtureURL)
        let envelope = try JSONDecoder().decode(ServiceEnvelope<ProviderObservabilityResult>.self, from: data)
        guard let result = envelope.result else {
            throw NativeModelTestFailure(description: "Provider observability fixture should include a result.")
        }

        try expectEqual(result.isDashboardEmpty, true, "Zero-count explanatory status rows should render one empty dashboard card.")
    }

    private func decodesProviderActivityPageFixtureAndAliases() throws {
        let fixtureURL = try repositoryRoot()
            .appendingPathComponent("fixtures/service-protocol/llm.listProviderActivity.response.json")
        let data = try Data(contentsOf: fixtureURL)
        let envelope = try JSONDecoder().decode(ServiceEnvelope<ProviderActivityPageResult>.self, from: data)
        guard let page = envelope.result else {
            throw NativeModelTestFailure(description: "Provider activity fixture should include a result.")
        }

        try expectEqual(page.rows.count, 2, "Provider activity fixture should decode both activity kinds.")
        try expectEqual(page.rows.map(\.kind), ["provider_call", "prompt_run"], "Provider activity should preserve unified row kinds.")
        try expectEqual(page.rows.first?.title, "analyze", "Provider activity should decode redacted titles.")
        try expectEqual(page.sourceRevision, "sha256:fixture-provider-activity", "Provider activity should decode source revision.")
        try expectEqual(page.returnedCount, 2, "Provider activity should decode returned count.")
        try expectEqual(page.totalCount, 2, "Provider activity should decode total count.")
        try expectFalse(page.hasMore, "Provider activity fixture should be terminal.")
        try expectEqual(page.sourceCompleteness, .enumerable, "Provider activity should decode source completeness.")
        try expectFalse(page.safetyFlags.providerRequestSent, "Listing activity must not send provider requests.")
        try expectFalse(page.safetyFlags.rawPromptPersisted, "Listing activity must not persist raw prompts.")
        try expectFalse(page.safetyFlags.rawResponsePersisted, "Listing activity must not persist raw responses.")
        try expectFalse(page.safetyFlags.rawTracePersisted, "Listing activity must not persist raw traces.")

        let camelJSON = """
        {
          "generatedBy": "local-v2.64",
          "rows": [{
            "id": "activity-camel",
            "kind": "prompt_run",
            "timestamp": 42,
            "title": "Camel activity",
            "subtitle": "redacted metadata",
            "status": "succeeded",
            "evidenceRefs": ["prompt-run:camel"]
          }],
          "sourceRevision": "sha256:camel",
          "returnedCount": 1,
          "totalCount": 2,
          "hasMore": true,
          "nextCursor": "v1:camel",
          "sourceCompleteness": "enumerable",
          "incompleteReason": null,
          "safetyFlags": {
            "providerRequestSent": false,
            "rawPromptPersisted": false,
            "rawResponsePersisted": false,
            "rawTracePersisted": false
          }
        }
        """
        let camel = try JSONDecoder().decode(ProviderActivityPageResult.self, from: Data(camelJSON.utf8))
        try expectEqual(camel.rows.first?.evidenceRefs, ["prompt-run:camel"], "Provider activity row should decode camelCase evidence aliases.")
        try expectEqual(camel.nextCursor, "v1:camel", "Provider activity should decode camelCase cursor aliases.")
        try expectEqual(camel.totalCount, 2, "Provider activity should decode camelCase total aliases.")
    }

    private func repositoryRoot() throws -> URL {
        var url = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        for _ in 0..<6 {
            if FileManager.default.fileExists(atPath: url.appendingPathComponent("fixtures/service-protocol").path) {
                return url
            }
            url.deleteLastPathComponent()
        }
        throw NativeModelTestFailure(description: "Unable to locate repository root from \(FileManager.default.currentDirectoryPath).")
    }
}
