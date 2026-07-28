import Testing
import Foundation
@testable import SkillsCopilot

@Suite("ToolGlobalModelTests")
struct ToolGlobalModelTests {
    @Test("ToolGlobalModelTests")
    func run() throws {
        try toolGlobalScopeDisplaysAsReadOnlyPreview()
        try piNativeSkillsRequireGuardedToggleCapabilityButDoNotDisplayAsReadOnly()
        try piInstallTargetRemainsBlockedEvenIfCapabilityPayloadClaimsSupport()
        try installPreviewRequiresConfirmationWithoutWriteBack()
        try backendInstallPreviewDecodesAsConfirmable()
    }

    private func toolGlobalScopeDisplaysAsReadOnlyPreview() throws {
        let record = toolGlobalSkill()

        try expectEqual(DisplayText.scope(record.scope), UIStrings.text("scope.toolGlobal", "Tool Global"), "Tool-global scope should use the localized display label.")
        try expectEqual(DisplayText.isToolGlobal(record), true, "Tool-global records should be recognized by the display model.")
        try expectEqual(DisplayText.isReadOnlyPreview(record), true, "Tool-global records should display as read-only preview rows.")
        try expectEqual(
            DisplayText.toggleDisabledReason(for: record, isWriting: false),
            UIStrings.toggleUnavailableToolGlobal,
            "Tool-global records should expose the install/copy confirmation disabled reason."
        )
    }

    private func installPreviewRequiresConfirmationWithoutWriteBack() throws {
        let record = toolGlobalSkill()
        let preview = ToolGlobalInstallPreview.localPreview(skill: record, target: .claudeCode)

        try expectEqual(preview.confirmationRequired, true, "Install preview should require confirmation.")
        try expectEqual(preview.writeBackEnabled, false, "Local fallback preview should not enable writes without backend install support.")
        try expectContains(preview.summary, record.name, "Install preview summary should name the skill.")
        try expectContains(preview.confirmationMessage, UIStrings.claudeCode, "Install preview confirmation should name the target agent.")
    }

    private func piNativeSkillsRequireGuardedToggleCapabilityButDoNotDisplayAsReadOnly() throws {
        let record = skill(
            id: "pi-one",
            agent: "pi",
            scope: "agent-global",
            path: "$HOME/.pi/agent/skills/pi-one/SKILL.md",
            definitionId: "pi:one",
            name: "Pi One",
            state: "loaded",
            enabled: true
        )

        try expectEqual(
            DisplayText.isReadOnlyPreview(record),
            false,
            "Pi native skill rows should not display as read-only previews; guarded writes are enforced by service capability checks."
        )
        try expectEqual(
            record.provenance.rootKind,
            .native,
            "Pi native roots should be classified as native provenance, not read-only provenance."
        )
        try expectEqual(
            record.provenance.isReadOnly,
            false,
            "Pi provenance should not be marked read-only when the skill comes from a native Pi root."
        )
        try expectNil(
            DisplayText.catalogToggleDisabledReason(for: record, isWriting: false),
            "Loaded Pi catalog state should not block the guarded toggle when service capability allows it."
        )
        try expectEqual(
            DisplayText.toggleDisabledReason(for: record, isWriting: false),
            UIStrings.piGuardedToggleBoundary,
            "Pi should stay disabled without explicit service config-toggle capability instead of being treated as a read-only adapter."
        )
    }

    private func piInstallTargetRemainsBlockedEvenIfCapabilityPayloadClaimsSupport() throws {
        let payload = """
        [
          {
            "agent": "pi",
            "display_name": "Pi",
            "status": "experimental",
            "scan": {"supported": true, "status": "verified", "reason": null},
            "project_scan": {"supported": true, "status": "verified", "reason": null},
            "config_toggle": {"supported": true, "status": "guarded", "reason": null},
            "config_snapshot": {"supported": true, "status": "guarded", "reason": null},
            "install": {"supported": true, "status": "blocked", "reason": "Pi install remains blocked."},
            "writable": {"supported": true, "status": "guarded", "reason": null},
            "blockers": []
          }
        ]
        """.data(using: .utf8)!
        let capabilities = try JSONDecoder().decode([AdapterCapabilityRecord].self, from: payload)

        try expectEqual(
            ToolInstallTarget.supportedTargets(from: capabilities),
            [],
            "Pi install must not become selectable from adapter capability payloads."
        )
    }

    private func backendInstallPreviewDecodesAsConfirmable() throws {
        let payload = """
        {
          "source_instance_id": "tool-alpha",
          "source_path": "/tmp/app/tool-global/skills/tool-alpha/SKILL.md",
          "target_agent": "codex",
          "target_scope": "agent-global",
          "target_path": "/tmp/home/.agents/skills/tool-alpha/SKILL.md",
          "files": [],
          "risks": ["Only the tool-global SKILL.md source will be copied."],
          "confirmation": {
            "required": true,
            "confirmed": false,
            "fields": ["target_path"],
            "message": "Confirm install to copy this tool-global skill into the selected agent root."
          },
          "wrote": false,
          "snapshot_id": null
        }
        """.data(using: .utf8)!

        let preview = try JSONDecoder().decode(ToolGlobalInstallPreview.self, from: payload)

        try expectEqual(preview.skillID, "tool-alpha", "Backend install preview should map source_instance_id to the UI id.")
        try expectEqual(preview.target, ToolInstallTarget.codex, "Backend install preview should decode the target agent.")
        try expectEqual(preview.targetPath, "/tmp/home/.agents/skills/tool-alpha/SKILL.md", "Backend install preview should expose the target path.")
        try expectEqual(preview.writeBackEnabled, true, "Backend install preview should enable the explicit confirm action.")
        try expectEqual(preview.wrote, false, "Preview should remain non-mutating.")
        try expectContains(preview.risks.joined(separator: "\n"), "SKILL.md", "Backend risks should be visible in the sheet.")
    }

    private func toolGlobalSkill() -> SkillRecord {
        skill(
            id: "tool-alpha",
            agent: "tool-global",
            scope: "tool-global",
            path: "/tmp/skills-copilot/staging/tool-alpha/SKILL.md",
            definitionId: "tool:alpha",
            name: "Tool Alpha",
            state: "loaded",
            enabled: true
        )
    }
}
