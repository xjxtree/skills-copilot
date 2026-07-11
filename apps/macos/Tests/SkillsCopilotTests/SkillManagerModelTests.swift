import Foundation
@testable import SkillsCopilot

struct SkillManagerModelTests {
    func run() throws {
        try defaultTargetsMatchSupportedManagerOrder()
        try workflowsSeparatePackageOperations()
        try visibleResultsRevealReturnedRowsInTwentyRowSteps()
        try searchRecordSeparatesNetworkBlockedFromEmptyResults()
        try previewSummaryLocalizesKnownOperations()
        try mutationPreviewDecodesCommandAndAgentTargets()
    }

    private func visibleResultsRevealReturnedRowsInTwentyRowSteps() throws {
        let rows = Array(0..<35).map(String.init)
        var search = SkillManagerVisibleResults<String>()

        try expectEqual(search.visibleItems(in: rows).count, 20, "Initial search page")
        search.loadMore(totalReturned: 35)
        try expectEqual(search.visibleItems(in: rows).count, 35, "Search Load More")
        search.reset()
        search.loadAll(totalReturned: 35)
        try expectEqual(search.visibleItems(in: rows).count, 35, "Search Load All")
    }

    private func defaultTargetsMatchSupportedManagerOrder() throws {
        try expectEqual(
            SkillManagerAgent.defaultTargets.map(\.rawValue),
            [
                "claude-code",
                "pi",
                "opencode",
                "codex",
                "hermes-agent",
                "openclaw"
            ],
            "Skill Manager should default to every app-supported agent in the manager order."
        )
    }

    private func workflowsSeparatePackageOperations() throws {
        try expectEqual(
            SkillManagerWorkflow.allCases.map(\.id),
            ["search-install", "installed-updates", "local-library"],
            "Skill Manager should expose exactly the three planned workflow tabs."
        )
        try expectEqual(
            SkillManagerWorkflow.searchInstall.allowsExternalManagerMutation,
            true,
            "Search & Install should use the external manager gate."
        )
        try expectEqual(
            SkillManagerWorkflow.installedUpdates.allowsExternalManagerMutation,
            true,
            "Installed & Updates should use the external manager gate."
        )
        try expectEqual(
            SkillManagerWorkflow.localLibrary.allowsExternalManagerMutation,
            false,
            "Local Library should remain available as an app-local workflow when the external manager is unavailable."
        )
    }

    private func searchRecordSeparatesNetworkBlockedFromEmptyResults() throws {
        let payload = """
        {
          "preview": {
            "tool_id": "npx-skills",
            "operation": "search",
            "command": ["/usr/local/bin/npx", "skills", "find", "superpower"],
            "cwd": "/tmp/project",
            "env": [
              {"key": "DISABLE_TELEMETRY", "value": "1"},
              {"key": "DO_NOT_TRACK", "value": "1"}
            ],
            "requires_confirmation": false,
            "confirmed": false,
            "network_required": true,
            "network_allowed": false,
            "will_run": false,
            "preview_token": "skill-manager:search",
            "summary": "Search remote skill indexes with npx skills.",
            "risks": ["Search may contact skills.sh."]
          },
          "output": null,
          "results": [],
          "returned_count": 0,
          "total_count": null,
          "has_more": false,
          "source_completeness": "unknown",
          "incomplete_reason": "source_limited"
        }
        """.data(using: .utf8)!

        let search = try JSONDecoder().decode(SkillManagerSearchRecord.self, from: payload)

        try expectEqual(search.isBlockedByNetwork, true, "Network-blocked search should not be presented as an empty result set.")
        try expectEqual(search.totalCount, nil, "Blocked remote search must not invent an enumerable total.")
        try expectEqual(search.sourceCompleteness, .unknown, "Blocked remote search source completeness should remain unknown.")
        try expectEqual(search.incompleteReason, .sourceLimited, "Blocked remote search should explain that the source was not enumerated.")
        try expectEqual(search.hasValidPageMetadata, true, "Matching flattened search metadata should validate.")

        let mismatchedPayload = String(data: payload, encoding: .utf8)!
            .replacingOccurrences(of: "\"returned_count\": 0", with: "\"returned_count\": 1")
            .data(using: .utf8)!
        let mismatched = try JSONDecoder().decode(SkillManagerSearchRecord.self, from: mismatchedPayload)
        try expectEqual(mismatched.hasValidPageMetadata, false, "Returned count must match decoded rows.")
    }

    private func previewSummaryLocalizesKnownOperations() throws {
        UIStrings.use(.simplifiedChinese)
        defer {
            UIStrings.use(.english)
        }

        let searchPreview = SkillManagerCommandPreview(
            toolId: "npx-skills",
            operation: "search",
            command: ["/usr/local/bin/npx", "skills", "find", "superpower"],
            cwd: "/tmp/project",
            env: [],
            requiresConfirmation: false,
            confirmed: false,
            networkRequired: true,
            networkAllowed: false,
            willRun: false,
            previewToken: "skill-manager:search",
            summary: "Search remote skill indexes with npx skills.",
            risks: [],
            source: nil,
            skills: []
        )
        try expectEqual(
            searchPreview.localizedSummary,
            "通过外部技能管理器搜索远程技能索引。",
            "Search preview summary should use localized UI copy instead of the service English fallback."
        )

        let installPreview = SkillManagerCommandPreview(
            toolId: "npx-skills",
            operation: "install",
            command: ["/usr/local/bin/npx", "skills", "add", "obra/superpowers", "--skill", "brainstorming"],
            cwd: "/tmp/project",
            env: [],
            requiresConfirmation: true,
            confirmed: false,
            networkRequired: true,
            networkAllowed: true,
            willRun: false,
            previewToken: "skill-manager:install",
            summary: "Install obra/superpowers for 1 supported agent target(s).",
            risks: [],
            source: "obra/superpowers",
            skills: ["brainstorming"]
        )
        try expectEqual(
            installPreview.localizedSummary,
            "预览将 obra/superpowers 安装到所选目标。",
            "Install preview summary should preserve the package source while localizing the surrounding copy."
        )

        let removePreview = SkillManagerCommandPreview(
            toolId: "npx-skills",
            operation: "remove",
            command: ["/usr/local/bin/npx", "skills", "remove", "legacy-design"],
            cwd: "/tmp/project",
            env: [],
            requiresConfirmation: true,
            confirmed: false,
            networkRequired: false,
            networkAllowed: true,
            willRun: false,
            previewToken: "skill-manager:remove",
            summary: "Remove legacy-design from 1 supported agent target(s).",
            risks: [],
            source: nil,
            skills: ["legacy-design"]
        )
        try expectEqual(
            removePreview.localizedSummary,
            "预览从所选目标移除 legacy-design。",
            "Remove preview summary should use the skill name from the structured skills field."
        )

        let localCreatePreview = SkillManagerCommandPreview(
            toolId: "npx-skills",
            operation: "localCreate",
            command: ["/usr/local/bin/npx", "skills", "init", "local-note"],
            cwd: "/tmp/project",
            env: [],
            requiresConfirmation: true,
            confirmed: false,
            networkRequired: false,
            networkAllowed: true,
            willRun: false,
            previewToken: "skill-manager:local-create",
            summary: "Create a local skill template named local-note.",
            risks: [],
            source: nil,
            skills: ["local-note"]
        )
        try expectEqual(
            localCreatePreview.localizedSummary,
            "预览创建本地技能模板 local-note。",
            "Local create preview summary should use the skill name from the structured skills field."
        )
    }

    private func mutationPreviewDecodesCommandAndAgentTargets() throws {
        let payload = """
        {
          "preview": {
            "tool_id": "npx-skills",
            "operation": "install",
            "command": [
              "/usr/local/bin/npx",
              "skills",
              "add",
              "vercel-labs/agent-skills",
              "--skill",
              "frontend-design",
              "--agent",
              "claude-code",
              "--agent",
              "pi",
              "--agent",
              "opencode",
              "--agent",
              "codex",
              "--agent",
              "hermes-agent",
              "--agent",
              "openclaw",
              "-y"
            ],
            "cwd": "/tmp/project",
            "env": [
              {"key": "DISABLE_TELEMETRY", "value": "1"},
              {"key": "DO_NOT_TRACK", "value": "1"}
            ],
            "requires_confirmation": true,
            "confirmed": false,
            "network_required": true,
            "network_allowed": false,
            "will_run": false,
            "preview_token": "skill-manager:test",
            "summary": "Install preview",
            "risks": ["External manager writes selected targets."]
          },
          "output": null,
          "applied": false,
          "scanned_count": 0,
          "updated_skills": []
        }
        """.data(using: .utf8)!

        let preview = try JSONDecoder().decode(SkillManagerMutationRecord.self, from: payload)

        try expectEqual(preview.preview.toolId, "npx-skills", "Mutation preview should decode the tool id.")
        try expectEqual(preview.preview.operation, "install", "Mutation preview should decode operation.")
        try expectEqual(preview.applied, false, "Preview payload must remain non-mutating.")
        try expectEqual(
            preview.preview.command.filter { $0 == "--agent" }.count,
            SkillManagerAgent.defaultTargets.count,
            "Install preview should include one --agent flag for every default target."
        )
        try expectEqual(
            preview.preview.command.contains("--copy"),
            false,
            "Symlink distribution should not send --copy."
        )
        try expectEqual(
            preview.preview.env.contains { $0.key == "DISABLE_TELEMETRY" && $0.value == "1" },
            true,
            "Manager preview should expose telemetry-off env."
        )
    }
}
