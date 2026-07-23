import Foundation
@testable import SkillsCopilot

struct ToolGlobalModelTests {
    func run() throws {
        try toolGlobalScopeDisplaysAsReadOnlyPreview()
        try piNativeSkillsRequireGuardedToggleCapabilityButDoNotDisplayAsReadOnly()
        try piInstallTargetRemainsBlockedEvenIfCapabilityPayloadClaimsSupport()
        try installTargetCapabilitiesFailClosedAndDeduplicate()
        try verifiedDirectInstallTargetsCoverAllSupportedAgents()
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

    private func verifiedDirectInstallTargetsCoverAllSupportedAgents() throws {
        let agents = ["claude-code", "codex", "opencode", "pi", "hermes", "openclaw"]
        let payload = try JSONSerialization.data(withJSONObject: agents.map { agent in
            [
                "agent": agent,
                "display_name": agent,
                "status": "verified",
                "scan": ["supported": true, "status": "verified", "reason": NSNull()],
                "project_scan": ["supported": true, "status": "verified", "reason": NSNull()],
                "config_toggle": ["supported": false, "status": "blocked", "reason": NSNull()],
                "config_snapshot": ["supported": false, "status": "blocked", "reason": NSNull()],
                "install": ["supported": true, "status": "verified", "reason": NSNull()],
                "writable": ["supported": true, "status": "verified", "reason": NSNull()],
                "blockers": [],
            ] as [String: Any]
        })
        let capabilities = try JSONDecoder().decode([AdapterCapabilityRecord].self, from: payload)

        try expectEqual(
            ToolInstallTarget.supportedTargets(from: capabilities).map(\.rawValue),
            agents,
            "Guarded local-library install targets should include every verified supported agent."
        )
    }

    private func installTargetCapabilitiesFailClosedAndDeduplicate() throws {
        try expectEqual(
            ToolInstallTarget.supportedTargets(from: []),
            [],
            "Missing capability evidence must not expose speculative install targets."
        )

        let payload = try JSONSerialization.data(withJSONObject: [
            installCapability(agent: "codex", supported: true, status: "verified"),
            installCapability(agent: "codex", supported: true, status: "verified-native"),
            installCapability(agent: "claude-code", supported: true, status: "blocked"),
            installCapability(agent: "opencode", supported: true, status: "guarded"),
            installCapability(agent: "hermes", supported: false, status: "verified"),
        ])
        let capabilities = try JSONDecoder().decode([AdapterCapabilityRecord].self, from: payload)
        try expectEqual(
            ToolInstallTarget.supportedTargets(from: capabilities),
            [.codex],
            "Partial capability evidence should expose only one stable entry per verified target."
        )
    }

    private func installCapability(
        agent: String,
        supported: Bool,
        status: String
    ) -> [String: Any] {
        [
            "agent": agent,
            "display_name": agent,
            "status": status,
            "scan": ["supported": true, "status": "verified", "reason": NSNull()],
            "project_scan": ["supported": true, "status": "verified", "reason": NSNull()],
            "config_toggle": ["supported": false, "status": "blocked", "reason": NSNull()],
            "config_snapshot": ["supported": false, "status": "blocked", "reason": NSNull()],
            "install": ["supported": supported, "status": status, "reason": NSNull()],
            "writable": ["supported": supported, "status": status, "reason": NSNull()],
            "blockers": [],
        ]
    }

    private func backendInstallPreviewDecodesAsConfirmable() throws {
        let payload = """
        {
          "action": {
            "id": "action:install-skill:fixture",
            "kind": "install_skill",
            "intent": "install_skill",
            "target": {
              "kind": "skill",
              "id": "tool-alpha",
              "agent": "codex",
              "scope": "agent-global"
            },
            "impacts": ["skill_files", "app_local_data"],
            "preview_method": "skill.install",
            "apply_method": "skill.install",
            "source_revision": "sha256:source",
            "confirmation_required": true,
            "network": "none",
            "readback": ["skill_files", "catalog_skills"],
            "evidence_refs": ["skill:tool-alpha"]
          },
          "preview_token": "action-preview:v1:hmac-sha256:fixture",
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
