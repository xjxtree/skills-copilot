import Testing
import Foundation
@testable import SkillsCopilot

@Suite("ScriptExecutionModelTests")
struct ScriptExecutionModelTests {
    @Test("ScriptExecutionModelTests")
    func run() throws {
        try previewDecodesServicePayload()
        try previewDecodesInlineScopePayload()
    }

    private func previewDecodesServicePayload() throws {
        let data = Data(
            """
            {
              "instance_id": "beta",
              "script_name": "setup",
              "command_preview": ["bash", "scripts/setup.sh"],
              "scope": {
                "current_cwd": "/tmp/project",
                "env": {"SKILLS_SAFE_MODE": "1"},
                "network": "none",
                "files": ["/tmp/project/scripts/setup.sh"]
              },
              "risks": ["Writes are blocked by default."],
              "requires_confirmation": true,
              "execution_allowed": false,
              "audit_status": "blocked",
              "audit_id": "audit-1",
              "summary": "Blocked until confirmed.",
              "reason": "Native UI is preview-only."
            }
            """.utf8
        )

        let preview = try JSONDecoder().decode(ScriptExecutionPreview.self, from: data)

        try expectEqual(preview.skillID, "beta", "Script preview should decode instance ID alias.")
        try expectEqual(preview.scriptName, "setup", "Script preview should decode script name.")
        try expectEqual(preview.commandPreview, ["bash", "scripts/setup.sh"], "Script preview should decode command preview.")
        try expectEqual(preview.scope.cwd, "/tmp/project", "Script preview should decode current CWD alias.")
        try expectEqual(preview.scope.env["SKILLS_SAFE_MODE"], "1", "Script preview should decode env.")
        try expectEqual(preview.scope.network, "none", "Script preview should decode network scope.")
        try expectEqual(preview.scope.files, ["/tmp/project/scripts/setup.sh"], "Script preview should decode file scope.")
        try expectEqual(preview.risks, ["Writes are blocked by default."], "Script preview should decode risks.")
        try expectEqual(preview.confirmationRequired, true, "Script preview should require confirmation.")
        try expectEqual(preview.executionAllowed, false, "Script preview should decode blocked execution.")
        try expectEqual(preview.auditStatus, .blocked, "Script preview should decode audit status.")
        try expectEqual(preview.auditID, "audit-1", "Script preview should decode audit ID.")
        try expectEqual(preview.disabledReason, "Native UI is preview-only.", "Script preview should decode reason alias.")
    }

    private func previewDecodesInlineScopePayload() throws {
        let data = Data(
            """
            {
              "skill_id": "alpha",
              "command": ["python3", "-m", "tool"],
              "cwd": "/tmp/work",
              "env": {},
              "network": "read-only",
              "files": [],
              "confirmation_required": true,
              "allowed": true
            }
            """.utf8
        )

        let preview = try JSONDecoder().decode(ScriptExecutionPreview.self, from: data)

        try expectEqual(preview.skillID, "alpha", "Inline payload should decode skill ID.")
        try expectEqual(preview.commandPreview, ["python3", "-m", "tool"], "Inline payload should decode command alias.")
        try expectEqual(preview.scope.cwd, "/tmp/work", "Inline payload should decode CWD.")
        try expectEqual(preview.scope.network, "read-only", "Inline payload should decode network.")
        try expectEqual(preview.auditStatus, .requiresConfirmation, "Allowed payload should default to requires-confirmation audit status.")
        try expectEqual(preview.confirmationRequired, true, "Inline payload should preserve confirmation requirement.")
    }
}
