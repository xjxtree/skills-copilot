import Foundation
@testable import SkillsCopilot

struct ScanResultCompatibilityTests {
    func run() throws {
        try legacyScanResultWithoutAdditiveDiagnosticsDecodesAsCompleted()
    }

    private func legacyScanResultWithoutAdditiveDiagnosticsDecodesAsCompleted() throws {
        let payload = #"""
        {
          "scanned_count": 1,
          "accepted_context_revision": "sha256:legacy-project-context",
          "catalog_scan_revision": "sha256:legacy-catalog-scan",
          "readback": {
            "accepted_context_revision": "sha256:legacy-project-context",
            "catalog_scan_revision": "sha256:legacy-catalog-scan",
            "verified": true
          },
          "skills": [
            {
              "id": "legacy-skill",
              "agent": "claude-code",
              "scope": "agent-global",
              "path": "/tmp/legacy/SKILL.md",
              "display_path": "$HOME/.claude/skills/legacy/SKILL.md",
              "definition_id": "legacy-definition",
              "name": "Legacy Skill",
              "state": "loaded",
              "enabled": true
            }
          ],
          "activity": {
            "operation": "catalog.scanAll",
            "status": "completed",
            "started_at": 1,
            "finished_at": 2,
            "scanned_count": 1,
            "skill_count": 1,
            "finding_count": 0,
            "conflict_count": 0,
            "snapshot_count": 0,
            "roots": ["$HOME/.claude/skills"],
            "log_entries": [],
            "recovery_actions": [],
            "agent_summaries": [
              {
                "agent": "claude-code",
                "display_label": "Claude Code",
                "status": "completed",
                "scanned_count": 1,
                "catalog_count": 1,
                "broken_count": 0,
                "roots_considered": ["$HOME/.claude/skills"],
                "roots_scanned": ["$HOME/.claude/skills"],
                "roots_skipped": [],
                "recovery_actions": []
              }
            ]
          }
        }
        """#

        let result = try JSONDecoder().decode(ScanResult.self, from: Data(payload.utf8))
        let activity = try require(result.activity, "Legacy ScanResult should retain refresh activity.")
        let summary = try require(
            activity.agentSummaries?.first,
            "Legacy ScanResult should retain its agent summary."
        )

        try expectEqual(result.scannedCount, 1, "Legacy ScanResult should decode its scan count.")
        try expectEqual(result.skills.count, 1, "Legacy ScanResult should decode its skill collection.")
        try expectEqual(activity.status, "completed", "Legacy scan activity should remain a completed result.")
        try expectEqual(summary.status, "completed", "Legacy agent summary should remain completed.")
        try expectEqual(summary.rootsPartial, [], "Missing additive partial roots should default to an empty list.")
        try expectEqual(summary.scanIssues, [], "Missing additive scan issues should default to an empty list.")

        let encoded = try JSONEncoder().encode(summary)
        let encodedObject = try require(
            JSONSerialization.jsonObject(with: encoded) as? [String: Any],
            "Re-encoded legacy summary should remain a keyed JSON object."
        )
        try expectEqual(
            encodedObject.keys.sorted(),
            [
                "agent", "broken_count", "catalog_count", "display_label", "recovery_actions",
                "roots_considered", "roots_partial", "roots_scanned", "roots_skipped",
                "scan_issues", "scanned_count", "status"
            ].sorted(),
            "Encoding should preserve every existing key and both additive keys."
        )
        try expectEqual(encodedObject["display_label"] as? String, "Claude Code", "Encoding should preserve the existing display_label key.")
        try expectEqual(encodedObject["roots_considered"] as? [String], ["$HOME/.claude/skills"], "Encoding should preserve existing root keys.")
        try expectEqual(encodedObject["roots_partial"] as? [String], [], "Encoding should emit the additive roots_partial key.")
        try expectEqual((encodedObject["scan_issues"] as? [Any])?.count, 0, "Encoding should emit the additive scan_issues key.")
    }

    private func require<T>(_ value: T?, _ message: String) throws -> T {
        guard let value else {
            throw NativeModelTestFailure(description: message)
        }
        return value
    }
}
