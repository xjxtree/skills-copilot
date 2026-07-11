import Foundation
@testable import SkillsCopilot

struct LocalSessionPreviewModelTests {
    func run() throws {
        try previewDecodesRedactedRowsAndSafety()
        try previewDecodesCandidateAndContentCompatibility()
        try previewMergesPaginatedRows()
        try previewMergesSkillUsageAcrossPages()
        try unavailableKeepsAuthorizationRequired()
        try filteredEmptyCopyExplainsHiddenLocalSessionCount()
    }

    private func previewDecodesRedactedRowsAndSafety() throws {
        let payload = """
        {
          "generated_by": "local-v2.98",
          "authorized": true,
          "authorization_required": false,
          "roots": [{"root":"$HOME/.codex/sessions","status":"authorized-read-only","candidate_count":1}],
          "count": 1,
          "total_candidate_count": 1,
          "user_message_count": 1,
          "total_message_count": 2,
          "tool_call_count": 1,
          "skill_call_count": 2,
          "skill_usage_rows": [{
            "skill_id": "fixture-skill-id",
            "skill_name": "fixture-skill",
            "agent": "codex",
            "call_count": 2,
            "session_count": 1,
            "latest_modified_at": 1781600000000,
            "evidence_refs": ["session.content_hash:fixturehash"]
          }],
          "session_rows": [{
            "id": "local-session-fixture",
            "title": "fixture",
            "source_kind": "authorized-local-session",
            "agent": "codex",
            "redacted_path": "$HOME/.codex/sessions/fixture.jsonl",
            "started_at": 1781599800000,
            "ended_at": 1781600000000,
            "excerpt": "Used fixture-skill-id with <redacted>.",
            "excerpt_char_count": 38,
            "user_message_count": 1,
            "total_message_count": 2,
            "tool_call_count": 1,
            "skill_call_count": 2,
            "content_hash": "fixturehash",
            "evidence_refs": ["session.path:$HOME/.codex/sessions/fixture.jsonl"],
            "content_items": [
              {
                "id": "session-item-fixture-0",
                "kind": "user_message",
                "title": "User",
                "text": "Run skill:fixture-skill.",
                "char_count": 24,
                "timestamp": 1781599800000,
                "evidence_refs": []
              },
              {
                "id": "session-item-fixture-1",
                "kind": "tool_call",
                "title": "fixture-tool",
                "text": "fixture tool call",
                "char_count": 17,
                "timestamp": 1781600000000,
                "evidence_refs": []
              },
              {
                "id": "session-item-fixture-2",
                "kind": "skill_call",
                "title": "Skill: fixture-skill",
                "text": "fixture-skill (2 calls)",
                "char_count": 23,
                "evidence_refs": ["session.content_hash:fixturehash"]
              }
            ]
          }],
          "gap_notes": [],
          "blocker_notes": [],
          "redaction_summary": {
            "status": "redacted-local-only",
            "redacted_value_count": 2,
            "redacted_fields": ["local paths"],
            "raw_trace_persisted": false,
            "raw_prompt_persisted": false,
            "raw_response_persisted": false,
            "raw_secret_returned": false
          },
          "safety_flags": {
            "read_only": true,
            "app_local_only": true,
            "provider_request_sent": false,
            "write_back_allowed": false,
            "write_actions_available": false,
            "skill_files_mutated": false,
            "agent_config_mutated": false,
            "script_execution_allowed": false,
            "execution_actions_available": false,
            "config_mutation_allowed": false,
            "snapshot_created": false,
            "triage_mutation_allowed": false,
            "credential_accessed": false,
            "raw_secret_returned": false,
            "raw_prompt_persisted": false,
            "raw_response_persisted": false,
            "raw_trace_persisted": false,
            "cloud_sync_performed": false,
            "telemetry_emitted": false
          }
        }
        """

        let result = try JSONDecoder().decode(LocalSessionPreviewResult.self, from: Data(payload.utf8))
        try expectEqual(result.generatedBy, "local-v2.98", "Local session preview generator")
        try expectEqual(result.authorizationRequired, false, "Authorized preview should not require more authorization.")
        try expectEqual(result.skillUsageRows.first?.skillName, "fixture-skill", "Skill usage row should decode.")
        try expectEqual(result.skillUsageRows.first?.callCount, 2, "Skill usage count should decode.")
        try expectEqual(result.sessionRows.first?.agent, "codex", "Agent should decode from preview row.")
        try expectEqual(result.sessionRows.first?.startedAt, 1781599800000, "Session start time should decode.")
        try expectEqual(result.sessionRows.first?.endedAt, 1781600000000, "Session end time should decode.")
        try expectEqual(result.sessionRows.first?.evidenceRefs.count, 1, "Evidence refs should decode.")
        try expectEqual(result.sessionRows.first?.contentItems.count, 3, "Session content items should decode.")
        try expectEqual(result.sessionRows.first?.contentItems.first?.kind, .userMessage, "Session content kind should decode.")
        try expectEqual(result.sessionRows.first?.contentItems.first?.timestamp, 1781599800000, "Message timestamp should decode.")
        try expectEqual(result.sessionRows.first?.contentItems[1].title, "fixture-tool", "Session content title should decode.")
        try expectEqual(result.sessionRows.first?.contentItems[1].timestamp, 1781600000000, "Tool timestamp should decode.")
        try expectEqual(result.sessionRows.first?.contentItems.last?.kind, .skillCall, "Skill call content kind should decode.")
        try expectEqual(result.userMessageCount, 1, "Preview should decode user message count.")
        try expectEqual(result.totalMessageCount, 2, "Preview should decode total message count.")
        try expectEqual(result.toolCallCount, 1, "Preview should decode tool call count.")
        try expectEqual(result.skillCallCount, 2, "Preview should decode skill call count.")
        try expectEqual(result.sessionRows.first?.skillCallCount, 2, "Row should decode skill call count.")
        try expectFalse(result.safetyFlags.providerRequestSent, "Preview should not send provider requests.")
        try expectFalse(result.safetyFlags.writeBackAllowed, "Preview should not enable writes.")
        try expectFalse(result.redactionSummary.rawTracePersisted, "Preview redaction summary should forbid raw trace persistence.")
    }

    private func unavailableKeepsAuthorizationRequired() throws {
        let result = LocalSessionPreviewResult.unavailable(reason: "missing method")
        try expectEqual(result.authorizationRequired, false, "Unavailable preview should not request manual roots.")
        try expectEqual(result.sessionRows.count, 0, "Unavailable preview should not synthesize rows.")
    }

    private func previewDecodesCandidateAndContentCompatibility() throws {
        let explicit = try decodePreview("""
        {
          "candidate_set_truncated": true,
          "session_rows": [{
            "id": "summary",
            "title": "Summary",
            "content_included": false
          }]
        }
        """)
        try expectFalse(!explicit.candidateSetTruncated, "Explicit candidate truncation should decode.")
        try expectEqual(explicit.sessionRows.first?.contentIncluded, false, "Summary content flag should decode false.")
        try expectEqual(explicit.sessionRows.first?.contentItems, [], "A summary row may omit content_items.")

        let legacy = try decodePreview("""
        {
          "session_rows": [{
            "id": "legacy",
            "title": "Legacy"
          }]
        }
        """)
        try expectFalse(legacy.candidateSetTruncated, "Missing candidate truncation should default false.")
        try expectEqual(legacy.sessionRows.first?.contentIncluded, true, "Missing content flag should preserve legacy detail semantics.")
        try expectEqual(legacy.sessionRows.first?.contentItems, [], "An old response may omit content_items.")
    }

    private func previewMergesPaginatedRows() throws {
        let first = try decodePreview("""
        {
          "generated_by": "local-v2.98",
          "authorized": true,
          "count": 1,
          "total_candidate_count": 3,
          "total_matched_count": 3,
          "offset": 0,
          "limit": 1,
          "has_more": true,
          "next_offset": 1,
          "candidate_set_truncated": true,
          "session_rows": [
            {
              "id": "session-a",
              "title": "A",
              "source_kind": "authorized-local-session",
              "scope": "project",
              "redacted_path": "$HOME/a.jsonl",
              "excerpt": "A",
              "user_message_count": 1,
              "total_message_count": 2,
              "tool_call_count": 0,
              "skill_call_count": 0,
              "content_hash": "a"
            }
          ]
        }
        """)
        let second = try decodePreview("""
        {
          "generated_by": "local-v2.98",
          "authorized": true,
          "count": 2,
          "total_candidate_count": 3,
          "total_matched_count": 3,
          "offset": 1,
          "limit": 2,
          "has_more": false,
          "session_rows": [
            {
              "id": "session-b",
              "title": "B",
              "source_kind": "authorized-local-session",
              "scope": "project",
              "redacted_path": "$HOME/b.jsonl",
              "excerpt": "B",
              "user_message_count": 1,
              "total_message_count": 2,
              "tool_call_count": 0,
              "skill_call_count": 0,
              "content_hash": "b"
            },
            {
              "id": "session-a",
              "title": "A",
              "source_kind": "authorized-local-session",
              "scope": "project",
              "redacted_path": "$HOME/a.jsonl",
              "excerpt": "A duplicate",
              "user_message_count": 1,
              "total_message_count": 2,
              "tool_call_count": 0,
              "skill_call_count": 0,
              "content_hash": "a"
            }
          ]
        }
        """)

        let merged = first.mergingPage(second)
        try expectEqual(merged.sessionRows.map(\.id), ["session-a", "session-b"], "Paginated merge should append unique rows in page order.")
        try expectEqual(merged.totalMatchedCount, 3, "Paginated merge should keep server total matched count.")
        try expectEqual(merged.hasMore, false, "Paginated merge should expose the latest page continuation state.")
        try expectNil(merged.nextOffset, "Final page should clear next offset.")
        try expectFalse(!merged.candidateSetTruncated, "Paginated merge should retain truncation from either page.")
    }

    private func previewMergesSkillUsageAcrossPages() throws {
        let first = LocalSessionPreviewResult(
            skillUsageRows: [
                LocalSessionSkillUsageRow(
                    skillId: "alpha",
                    skillName: "Alpha",
                    agent: "codex",
                    callCount: 2,
                    sessionCount: 1,
                    latestModifiedAt: "100",
                    evidenceRefs: ["alpha:one"]
                )
            ],
            hasMore: true,
            nextCursor: "next",
            sourceRevision: "r1"
        )
        let second = LocalSessionPreviewResult(
            skillUsageRows: [
                LocalSessionSkillUsageRow(
                    skillId: "beta",
                    skillName: "Beta",
                    agent: "codex",
                    callCount: 3,
                    sessionCount: 1,
                    latestModifiedAt: "300",
                    evidenceRefs: ["beta:one"]
                ),
                LocalSessionSkillUsageRow(
                    skillId: "alpha",
                    skillName: "Alpha",
                    agent: "codex",
                    callCount: 4,
                    sessionCount: 2,
                    latestModifiedAt: "200",
                    evidenceRefs: ["alpha:one", "alpha:two"]
                )
            ],
            hasMore: false,
            sourceRevision: "r1"
        )

        let merged = first.mergingPage(second)
        try expectEqual(merged.skillUsageRows.map(\.skillId), ["alpha", "beta"], "Final Show All source must keep skills from every accepted page in deterministic aggregate order.")
        try expectEqual(merged.skillUsageRows.first?.callCount, 6, "Repeated skill calls must aggregate across pages.")
        try expectEqual(merged.skillUsageRows.first?.sessionCount, 3, "Repeated skill session counts must aggregate across pages.")
        try expectEqual(merged.skillUsageRows.first?.latestModifiedAt, "200", "Repeated skills must retain the latest timestamp.")
        try expectEqual(merged.skillUsageRows.first?.evidenceRefs, ["alpha:one", "alpha:two"], "Evidence must merge without duplicates and preserve first-seen order.")
    }

    private func decodePreview(_ payload: String) throws -> LocalSessionPreviewResult {
        try JSONDecoder().decode(LocalSessionPreviewResult.self, from: Data(payload.utf8))
    }

    private func filteredEmptyCopyExplainsHiddenLocalSessionCount() throws {
        let message = UIStrings.localSessionNoMatchesMessage(totalCount: 9)
        try expectFalse(!message.contains("9"), "Filtered-empty session copy should include the loaded local session count.")
        try expectFalse(message.contains("secondary sidebar"), "Session empty copy should not reference an implementation-specific secondary sidebar.")
    }
}
