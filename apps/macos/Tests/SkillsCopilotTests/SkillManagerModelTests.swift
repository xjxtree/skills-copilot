import Foundation
@testable import SkillsCopilot

struct SkillManagerModelTests {
    func run() throws {
        try defaultTargetsMatchSupportedManagerOrder()
        try workflowsSeparatePackageOperations()
        try visibleResultsRevealReturnedRowsInTwentyRowSteps()
        try searchRecordSeparatesNetworkBlockedFromEmptyResults()
        try methodSpecificPageMetadataRejectsCrossMethodSemantics()
        try installedSourceOwnershipDecodesIndependentlyFromInventoryDiscovery()
        try localArchiveImportPreviewDecodes()
        try managerInventoryConsumesItsMatchingCatalogSource()
        try inventoryExcludesReadOnlyPluginCacheSources()
        try installedLocalSourceWinsOverSameNameAppLibraryEntry()
        try installedAppLibrarySourceRetainsCleanupIdentity()
        try nestedSharedLocalSourceSupportsZipUpdate()
        try externalLocalSourceHasNoArchiveUpdateTarget()
        try duplicateInstalledRowsMergeAgentLinks()
        try previewSummaryLocalizesKnownOperations()
        try mutationPreviewDecodesCommandAndAgentTargets()
        try duplicateSearchAndInstalledIDsKeepEveryDisplayOccurrence()
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

    private func duplicateSearchAndInstalledIDsKeepEveryDisplayOccurrence() throws {
        let searchPayload = """
        {
          "preview": {"tool_id":"npx-skills","operation":"search","command":[],"cwd":"<project-root>","env":[],"requires_confirmation":false,"confirmed":false,"network_required":true,"network_allowed":true,"will_run":true,"preview_token":"fixture","summary":"fixture","risks":[]},
          "output": null,
          "results": [
            {"name":"same","source":"owner/repo","raw":{}},
            {"name":"same","source":"owner/repo","raw":{"variant":2}}
          ],
          "returned_count":2,"total_count":null,"has_more":false,
          "source_completeness":"unknown","incomplete_reason":"source_limited"
        }
        """
        let search = try JSONDecoder().decode(
            SkillManagerSearchRecord.self,
            from: Data(searchPayload.utf8)
        )
        let searchRows = search.displayResults(visibleCount: 2)
        try expectEqual(searchRows.count, 2, "Duplicate manager search IDs must retain every returned row.")
        try expectEqual(Set(searchRows.map(\.id)).count, 2, "Duplicate manager search IDs must receive occurrence-disambiguated display IDs.")
        try expectEqual(searchRows.map(\.id.occurrence), [0, 1], "Manager search occurrences must be stable within the logical ID.")

        let installedPayload = """
        {
          "preview": {"tool_id":"npx-skills","operation":"listInstalled","command":[],"cwd":"<project-root>","env":[],"requires_confirmation":false,"confirmed":false,"network_required":false,"network_allowed":true,"will_run":true,"preview_token":"fixture","summary":"fixture","risks":[]},
          "output":{"status":"completed","exit_code":0,"stdout":"","stderr":""},
          "installed": [
            {"name":"same","source":"owner/repo","agents":["codex"],"scope":"project","path":"<project-root>/same","raw":{}},
            {"name":"same","source":"owner/repo","agents":["codex"],"scope":"project","path":"<project-root>/same","raw":{"variant":2}}
          ],
          "returned_count":2,"total_count":2,"has_more":false,
          "source_completeness":"enumerable"
        }
        """
        let installed = try JSONDecoder().decode(
            SkillManagerInstalledListRecord.self,
            from: Data(installedPayload.utf8)
        )
        let installedRows = installed.displayRecords
        try expectEqual(installedRows.count, 2, "Duplicate installed IDs must retain every returned row.")
        try expectEqual(Set(installedRows.map(\.id)).count, 2, "Duplicate installed IDs must receive occurrence-disambiguated display IDs.")
        try expectEqual(installedRows.map(\.id.occurrence), [0, 1], "Installed occurrences must be stable within the logical ID.")
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
            ["search-install", "installed-updates"],
            "Skill Manager should expose search and a unified installed/local inventory."
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

        let invalidSearchPayloads = try [
            mutatingPayload(payload) { $0.removeValue(forKey: "incomplete_reason") },
            mutatingPayload(payload) { $0["incomplete_reason"] = "page_failed" },
            mutatingPayload(payload) { $0["source_completeness"] = "enumerable" },
            mutatingPayload(payload) { $0["total_count"] = 0 },
            mutatingPayload(payload) { $0["has_more"] = true }
        ]
        for (index, invalidPayload) in invalidSearchPayloads.enumerated() {
            let invalid = try JSONDecoder().decode(
                SkillManagerSearchRecord.self,
                from: invalidPayload
            )
            try expectEqual(invalid.hasValidPageMetadata, false, "Search invalid metadata case \(index) must be rejected.")
        }
    }

    private func methodSpecificPageMetadataRejectsCrossMethodSemantics() throws {
        let payload = """
        {
          "preview": {
            "tool_id": "npx-skills",
            "operation": "listInstalled",
            "command": ["/usr/local/bin/npx", "skills", "list", "--json"],
            "cwd": "/tmp/project",
            "env": [],
            "requires_confirmation": false,
            "confirmed": false,
            "network_required": false,
            "network_allowed": true,
            "will_run": false,
            "preview_token": "skill-manager:installed",
            "summary": "List installed skills.",
            "risks": []
          },
          "output": {"status":"completed","exit_code":0,"stdout":"","stderr":""},
          "installed": [
            {"name":"alpha","source":"owner/repo","agents":["codex"],"scope":"project","path":"/tmp/alpha","raw":{}}
          ],
          "returned_count": 1,
          "total_count": 1,
          "has_more": false,
          "source_completeness": "enumerable"
        }
        """
        let valid = try JSONDecoder().decode(
            SkillManagerInstalledListRecord.self,
            from: payload.data(using: .utf8)!
        )
        try expectEqual(valid.hasValidPageMetadata, true, "Installed exact metadata should validate.")

        let installedData = payload.data(using: .utf8)!
        let invalidInstalledPayloads = try [
            mutatingPayload(installedData) { $0["incomplete_reason"] = "source_limited" },
            mutatingPayload(installedData) { $0["source_completeness"] = "unknown" },
            mutatingPayload(installedData) { $0["total_count"] = 2 },
            mutatingPayload(installedData) {
                $0["has_more"] = true
                $0["next_cursor"] = "unexpected"
            }
        ]
        for (index, invalidPayload) in invalidInstalledPayloads.enumerated() {
            let invalid = try JSONDecoder().decode(
                SkillManagerInstalledListRecord.self,
                from: invalidPayload
            )
            try expectEqual(invalid.hasValidPageMetadata, false, "Installed invalid metadata case \(index) must be rejected.")
        }
    }

    private func installedSourceOwnershipDecodesIndependentlyFromInventoryDiscovery() throws {
        let payload = """
        {
          "name": "bug-fix",
          "source": "<project-root>/.agents/skills/bug-fix",
          "source_kind": "local",
          "agents": ["Claude Code", "Codex", "OpenCode"],
          "scope": "project",
          "path": "<project-root>/.agents/skills/bug-fix",
          "raw": {}
        }
        """.data(using: .utf8)!

        let installed = try JSONDecoder().decode(SkillManagerInstalledRecord.self, from: payload)

        try expectEqual(installed.isLocalSource, true, "An unlocked discovered skill should decode as a local source.")
        try expectEqual(installed.name, "bug-fix", "Local ownership must preserve the discovered skill identity.")
        try expectEqual(installed.source, "<project-root>/.agents/skills/bug-fix", "Local source paths should remain redacted but useful.")
        try expectEqual(installed.path, "<project-root>/.agents/skills/bug-fix", "Installed records should retain a dedicated source path for identity matching.")
    }

    private func localArchiveImportPreviewDecodes() throws {
        let payload = """
        {
          "skill_name": "local-review",
          "archive_path": "<user-home>/Downloads/local-review.zip",
          "archive_sha256": "sha256:fixture",
          "file_count": 2,
          "uncompressed_bytes": 512,
          "preview_token": "skill-manager-local-archive-import:fixture",
          "confirmed": false,
          "applied": false,
          "summary": "Validated local ZIP.",
          "imported_skill": null,
          "instance_id": null
        }
        """.data(using: .utf8)!

        let preview = try JSONDecoder().decode(SkillManagerLocalArchiveImportRecord.self, from: payload)

        try expectEqual(preview.skillName, "local-review", "Local ZIP preview should expose the contained skill name.")
        try expectEqual(preview.fileCount, 2, "Local ZIP preview should expose bounded archive metadata.")
        try expectEqual(preview.applied, false, "Previewing a local ZIP must remain non-mutating.")
        try expectNil(preview.instanceID, "A preview should not invent an imported catalog instance.")
    }

    private func managerInventoryConsumesItsMatchingCatalogSource() throws {
        let items = SkillManagerInventoryBuilder.build(
            installed: [installedRecord(
                name: "managed-skill",
                source: "owner/repository",
                sourceKind: "manager",
                agents: ["Codex"],
                path: "$HOME/.agents/skills/managed-skill"
            )],
            catalogSkills: [catalogSkill(
                id: "managed-catalog",
                name: "managed-skill",
                path: "/home/test/.agents/skills/managed-skill/SKILL.md"
            )],
            localLibrarySkills: [],
            scope: .global
        )

        try expectEqual(items.count, 1, "A manager row and its scanned canonical source must collapse into one inventory item.")
        try expectEqual(items[0].origin, .manager, "Lock-proven ownership must remain manager-owned after catalog association.")
        try expectNil(items[0].localInstanceID, "A manager-owned item must not expose local ZIP replacement.")
    }

    private func inventoryExcludesReadOnlyPluginCacheSources() throws {
        let items = SkillManagerInventoryBuilder.build(
            installed: [],
            catalogSkills: [catalogSkill(
                id: "plugin-cache",
                name: "cached-plugin-skill",
                path: "/home/test/.codex/plugins/cache/example/skills/cached-plugin-skill/SKILL.md"
            )],
            localLibrarySkills: [],
            scope: .global
        )

        try expectEqual(items.isEmpty, true, "Read-only plugin caches must not appear as editable local packages.")
    }

    private func installedLocalSourceWinsOverSameNameAppLibraryEntry() throws {
        let installedPath = "/home/test/.agents/skills/local-review"
        let items = SkillManagerInventoryBuilder.build(
            installed: [installedRecord(
                name: "local-review",
                source: installedPath,
                sourceKind: "local",
                agents: ["Claude Code", "Codex"],
                path: installedPath
            )],
            catalogSkills: [catalogSkill(
                id: "installed-local",
                name: "local-review",
                path: "\(installedPath)/SKILL.md"
            )],
            localLibrarySkills: [catalogSkill(
                id: "app-library-copy",
                agent: "tool-global",
                scope: "tool-global",
                name: "local-review",
                path: "/app-data/tool-global/skills/local-review/SKILL.md"
            )],
            scope: .global
        )

        try expectEqual(items.count, 1, "An installed local package and its same-name library copy should present one actionable row.")
        try expectEqual(items[0].localInstanceID, "installed-local", "ZIP update must target the active shared install source.")
        try expectEqual(items[0].localPath, installedPath, "The active shared source path should drive local update details.")
    }

    private func installedAppLibrarySourceRetainsCleanupIdentity() throws {
        let libraryPath = "/app-data/tool-global/skills/local-review/SKILL.md"
        let items = SkillManagerInventoryBuilder.build(
            installed: [installedRecord(
                name: "local-review",
                source: "/app-data/tool-global/skills/local-review",
                sourceKind: "local",
                agents: ["Claude Code"],
                path: "/project/.claude/skills/local-review"
            )],
            catalogSkills: [],
            localLibrarySkills: [catalogSkill(
                id: "app-library-source",
                agent: "tool-global",
                scope: "tool-global",
                name: "local-review",
                path: libraryPath
            )],
            scope: .project
        )

        try expectEqual(items.count, 1, "An installed app-library source should remain one inventory row.")
        try expectEqual(items[0].localOwnership, .appOwned, "Full uninstall must recognize app-owned source cleanup.")
        try expectEqual(items[0].localInstanceID, "app-library-source", "Full uninstall must retain the guarded local source identity.")
        try expectEqual(items[0].agents, ["claude-code"], "The linked Agent target must remain explicit in preview state.")
    }

    private func nestedSharedLocalSourceSupportsZipUpdate() throws {
        let installedPath = "/home/test/.agents/skills/minimax-skills/minimax-docx"
        let items = SkillManagerInventoryBuilder.build(
            installed: [installedRecord(
                name: "minimax-docx",
                source: installedPath,
                sourceKind: "local",
                agents: ["OpenCode"],
                path: installedPath
            )],
            catalogSkills: [catalogSkill(
                id: "nested-local",
                agent: "opencode",
                name: "minimax-docx",
                path: "\(installedPath)/SKILL.md"
            )],
            localLibrarySkills: [],
            scope: .global
        )

        try expectEqual(items.count, 1, "A nested shared skill should remain one inventory source.")
        try expectEqual(items[0].localOwnership, .global, "Nested .agents skills should be recognized as guarded global sources.")
        try expectEqual(items[0].localInstanceID, "nested-local", "Nested local updates must retain an exact catalog target.")
        try expectEqual(items[0].localPath, installedPath, "Nested local updates must use the containing skill directory.")
    }

    private func externalLocalSourceHasNoArchiveUpdateTarget() throws {
        let items = SkillManagerInventoryBuilder.build(
            installed: [installedRecord(
                name: "external-local",
                source: "/home/test/.config/opencode/skills/external-local",
                sourceKind: "local",
                agents: ["OpenCode"],
                path: "/home/test/.config/opencode/skills/external-local"
            )],
            catalogSkills: [catalogSkill(
                id: "external-catalog",
                agent: "opencode",
                name: "external-local",
                path: "/home/test/.config/opencode/skills/external-local/SKILL.md"
            )],
            localLibrarySkills: [],
            scope: .global
        )

        try expectEqual(items.count, 1, "External local sources should remain visible.")
        try expectEqual(items[0].localOwnership, .external, "Sources outside guarded .agents roots must be labeled external.")
        try expectNil(items[0].localInstanceID, "External local sources must not expose a ZIP replacement target.")
        try expectNil(items[0].localPath, "External local paths must not be passed to the guarded replacement flow.")
    }

    private func duplicateInstalledRowsMergeAgentLinks() throws {
        let first = installedRecord(
            name: "shared-skill",
            source: "owner/repository",
            sourceKind: "manager",
            agents: ["Codex"],
            path: "$HOME/.agents/skills/shared-skill"
        )
        let second = installedRecord(
            name: "shared-skill",
            source: "owner/repository",
            sourceKind: "manager",
            agents: ["Claude Code"],
            path: "$HOME/.agents/skills/shared-skill"
        )

        let items = SkillManagerInventoryBuilder.build(
            installed: [first, second],
            catalogSkills: [],
            localLibrarySkills: [],
            scope: .global
        )

        try expectEqual(items.count, 1, "Duplicate CLI rows for one source should collapse into one inventory item.")
        try expectEqual(items[0].agents, ["claude-code", "codex"], "Collapsed rows should preserve the union of supported agent links.")
    }

    private func installedRecord(
        name: String,
        source: String,
        sourceKind: String,
        agents: [String],
        path: String
    ) -> SkillManagerInstalledRecord {
        SkillManagerInstalledRecord(
            name: name,
            source: source,
            sourceKind: sourceKind,
            agents: agents,
            scope: "global",
            path: path,
            raw: nil
        )
    }

    private func catalogSkill(
        id: String,
        agent: String = "codex",
        scope: String = "agent-global",
        name: String,
        path: String
    ) -> SkillRecord {
        SkillRecord(
            id: id,
            agent: agent,
            scope: scope,
            path: path,
            displayPath: path,
            definitionId: name,
            name: name,
            state: "loaded",
            enabled: true
        )
    }

    private func mutatingPayload(
        _ data: Data,
        _ mutation: (inout [String: Any]) -> Void
    ) throws -> Data {
        guard var object = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw NativeModelTestFailure(description: "Expected a JSON object fixture.")
        }
        mutation(&object)
        return try JSONSerialization.data(withJSONObject: object)
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
