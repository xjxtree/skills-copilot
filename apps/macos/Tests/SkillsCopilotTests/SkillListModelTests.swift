@testable import SkillsCopilot

struct SkillListModelTests {
    func run() throws {
        try detailWorkbenchSectionsExposeDiagnostics()
        try findingIssueGroupsPreserveRemediationAndImpactCounts()
        try searchMatchesNameDefinitionAndDisplayPathCaseInsensitively()
        try agentFiltersLimitResultsAndGroupsUseStableAdapterOrder()
        try stateFiltersUseEffectiveStatusFindingsAndConflicts()
        try issueIndicatorCountsUseVisibleProblemSemantics()
        try problemAndConflictPresentationStayIndependent()
        try widespreadBaselineFindingsDoNotDrivePerSkillIssuePresentation()
        try declarationBaselineWarningsStayIgnoredForEverySupportedAgent()
        try triagedAndSuppressedFindingsDoNotDriveVisibleIssuePresentation()
        try problemItemsUseCurrentAgentRuntimeSemantics()
        try scopeFiltersSeparateProjectAndGlobalSkills()
        try sortOrdersAreStableForCoreListColumns()
        try sortDirectionCanReverseCoreListColumns()
        try skillProvenanceClassifiesAgentRootsDeterministically()
        try skillIdentitySummaryAndDedupeExplanationAreStable()
        try privacyPathDisplayRedactsAndCollapsesLocalPaths()
        try privacyPathDisplayRedactsEmbeddedEvidencePaths()
    }

    private func detailWorkbenchSectionsExposeDiagnostics() throws {
        try expectEqual(
            DetailSection.visibleCases.map(\.rawValue),
            ["overview", "findings", "conflicts", "history", "metadata"],
            "Skill detail switcher should expose skill issues and same-agent conflicts as independent sections."
        )
        try expectEqual(DetailSection.primaryWorkCases, [], "Sidebar Work surfaces should remain retired; Provider Observability lives in Settings.")
        try expectEqual(DetailSection.findings.title, "Skill Issues", "The single-skill issue tab should name its scope explicitly.")
        try expectEqual(
            DetailSection(rawValue: "conflicts")?.title,
            "Same-Agent Conflicts",
            "Same-agent conflicts should have a distinct detail destination."
        )
        try expectEqual(DetailSection.history.title, "History", "History section title")
        try expectEqual(DetailSection.metadata.title, "Metadata", "Metadata section title")
        try expectEqual(DetailSection.overview.systemImage, "chart.pie", "Overview tab should use a unified icon.")
        try expectEqual(DetailSection.metadata.systemImage, "info.circle", "Metadata tab should use a unified icon.")
    }

    private func findingIssueGroupsPreserveRemediationAndImpactCounts() throws {
        let findings = [
            RuleFindingRecord(
                id: "finding-1",
                instanceId: "alpha",
                definitionId: "def.alpha",
                ruleId: "permissions.exec-needs-human",
                severity: "warning",
                message: "Execution requires a human gate.",
                suggestion: "Require human confirmation before execution.",
                createdAt: 30
            ),
            RuleFindingRecord(
                id: "finding-2",
                instanceId: "beta",
                definitionId: "def.beta",
                ruleId: "permissions.exec-needs-human",
                severity: "warning",
                message: "Execution requires a human gate.",
                suggestion: "Require human confirmation before execution.",
                createdAt: 20
            ),
        ]

        let groups = FindingDisplayModel.issueGroups(
            findings: findings,
            severityFilter: FindingDisplayModel.allFilterValue,
            ruleFilter: FindingDisplayModel.allFilterValue
        )

        try expectEqual(groups.count, 1, "Matching findings should collapse into one issue group.")
        try expectEqual(groups[0].impactedInstanceCount, 2, "Issue groups should retain impacted instance count.")
        try expectEqual(groups[0].entryCount, 2, "Issue groups should retain scan entry count.")
        try expectEqual(groups[0].remediation, "Require human confirmation before execution.", "Issue groups should keep remediation text.")
    }

    private func searchMatchesNameDefinitionAndDisplayPathCaseInsensitively() throws {
        try expectEqual(
            filtered(searchText: "  alpha ").map(\.id),
            ["alpha"],
            "Search should trim whitespace and match names."
        )
        try expectEqual(
            filtered(searchText: "CODEX:GAMMA").map(\.id),
            ["gamma"],
            "Search should match definition IDs case-insensitively."
        )
        try expectEqual(
            filtered(searchText: "open code").map(\.id),
            ["omega"],
            "Search should match opencode agent aliases."
        )
        try expectEqual(
            filtered(searchText: "project/beta").map(\.id),
            ["beta"],
            "Search should match display paths."
        )
    }

    private func stateFiltersUseEffectiveStatusFindingsAndConflicts() throws {
        try expectEqual(
            SkillStateFilter.sidebarCases.map(\.rawValue),
            ["all", "enabled", "disabled", "withFindings", "withConflicts"],
            "The sidebar filter should expose Issues and Conflicts as independent user-facing buckets."
        )
        try expectEqual(filtered(stateFilter: .enabled).map(\.id), ["alpha", "gamma", "omega"], "Enabled filter")
        try expectEqual(filtered(stateFilter: .disabled).map(\.id), ["beta"], "Disabled filter")
        try expectEqual(filtered(stateFilter: .broken).map(\.id), ["delta"], "Broken filter")
        try expectEqual(filtered(stateFilter: .missing).map(\.id), ["epsilon"], "Missing filter")
        try expectEqual(filtered(stateFilter: .shadowed).map(\.id), ["zeta"], "Shadowed filter")
        try expectEqual(filtered(stateFilter: .unknown).map(\.id), ["theta"], "Unknown filter")
        try expectEqual(filtered(stateFilter: .withFindings).map(\.id), ["delta", "epsilon", "theta"], "Problem item filter")
        try expectEqual(filtered(stateFilter: .risky).map(\.id), [], "Risky filter")
    }

    private func issueIndicatorCountsUseVisibleProblemSemantics() throws {
        let issueIndex = SkillListModel.issueIndex(
            skills: Self.skills,
            findings: Self.findings,
            conflicts: Self.conflicts
        )
        let counts = Dictionary(uniqueKeysWithValues: Self.skills.map { skill in
            (
                skill.id,
                issueIndex.issueCount(for: skill.id)
            )
        })

        try expectEqual(counts["gamma"], 0, "Rows should ignore built-in declaration-baseline findings without folding in same-agent conflicts.")
        try expectEqual(counts["epsilon"], 1, "Rows should count missing state without folding in same-agent conflicts.")
        try expectEqual(counts["delta"], 1, "Rows should count broken state as a visible issue.")
        try expectEqual(counts["theta"], 1, "Rows should count unknown/root-error state as a visible issue.")
        try expectEqual(counts["beta"], 0, "Rows should ignore cross-agent-only conflicts.")
        try expectEqual(counts["alpha"], 0, "Rows should ignore definition-only findings until they are attached to an instance.")
        try expectEqual(
            SkillListModel.displayFindings(skills: Self.skills, findings: Self.findings).map(\.id),
            [],
            "The shared visible-finding collection should exclude records that cannot navigate to a current skill instance."
        )
        try expectEqual(
            SkillListModel.displayIssueCount(skills: Self.skills, findings: Self.findings, conflicts: Self.conflicts, agentFilter: .all),
            3,
            "The sidebar Issues metric should equal the navigable per-skill issue total, including catalog-state problems and excluding unattached or ignored findings."
        )
        try expectEqual(
            SkillListModel.displayIssueCount(skills: Self.skills, findings: Self.findings, conflicts: Self.conflicts, agentFilter: .claudeCode),
            2,
            "The selected-agent Issues metric should stay scoped to the same visible rows as the Issues filter."
        )
        try expectEqual(
            SkillListModel.displayIssueCount(skills: Self.skills, findings: Self.findings, conflicts: Self.conflicts, agentFilter: .codex),
            1,
            "Missing catalog-state records should contribute to the selected-agent Issues metric even without a finding row."
        )
        try expectEqual(
            Self.skills.map { SkillListModel.issueIndicatorCount(for: $0, skills: Self.skills, findings: Self.findings, conflicts: Self.conflicts) },
            Self.skills.map { issueIndex.issueCount(for: $0.id) },
            "Precomputed issue index should preserve per-row issue semantics while avoiding repeated full-list work."
        )
    }

    private func problemAndConflictPresentationStayIndependent() throws {
        let skills = [
            skill(id: "finding", agent: "codex", scope: "agent-global", path: "/skills/finding/SKILL.md", definitionId: "def.finding", name: "Finding"),
            skill(id: "missing", agent: "codex", scope: "agent-global", path: "/skills/missing/SKILL.md", definitionId: "def.missing", name: "Missing", state: "missing", enabled: false),
            skill(id: "conflict-a", agent: "codex", scope: "agent-global", path: "/skills/a/SKILL.md", definitionId: "def.conflict", name: "Conflict"),
            skill(id: "conflict-b", agent: "codex", scope: "agent-project", path: "/project/skills/a/SKILL.md", definitionId: "def.conflict", name: "Conflict"),
        ]
        let findings = [
            Self.finding(id: "finding-only", instanceId: "finding", ruleId: "body.too-long"),
            Self.finding(id: "collision-a", instanceId: "conflict-a", ruleId: "name.collision", severity: "info"),
            Self.finding(id: "collision-b", instanceId: "conflict-b", ruleId: "name.collision", severity: "info"),
        ]
        let conflicts = [
            ConflictGroupRecord(
                id: "same-agent-runtime-conflict",
                definitionId: "def.conflict",
                reason: "content-drift",
                winnerId: nil,
                instanceIds: ["conflict-a", "conflict-b"]
            ),
        ]

        let problemSkills = SkillListModel.filteredAndSorted(
            skills: skills,
            findings: findings,
            conflicts: conflicts,
            searchText: "",
            agentFilter: .codex,
            stateFilter: .withFindings,
            sortOrder: .name
        )
        guard let conflictFilter = SkillStateFilter(rawValue: "withConflicts") else {
            throw NativeModelTestFailure(description: "The independent Conflicts filter should be available.")
        }
        let conflictSkills = SkillListModel.filteredAndSorted(
            skills: skills,
            findings: findings,
            conflicts: conflicts,
            searchText: "",
            agentFilter: .codex,
            stateFilter: conflictFilter,
            sortOrder: .name
        )
        let issueIndex = SkillListModel.issueIndex(skills: skills, findings: findings, conflicts: conflicts)

        try expectEqual(
            problemSkills.map(\.id),
            ["finding", "missing"],
            "The Issues filter should contain only single-skill findings and catalog-state problems."
        )
        try expectEqual(issueIndex.issueCount(for: "conflict-a"), 0, "A pure multi-skill conflict should not increment the single-skill issue badge.")
        try expectEqual(issueIndex.issueCount(for: "conflict-b"), 0, "Every member of a pure conflict should keep a zero single-skill issue badge.")
        try expectEqual(issueIndex.conflictCount(for: "conflict-a"), 1, "A conflict member should expose its conflict-group count independently.")
        try expectEqual(issueIndex.conflictCount(for: "conflict-b"), 1, "Every conflict member should expose the same independent group count.")
        try expectEqual(
            conflictSkills.map(\.id),
            ["conflict-a", "conflict-b"],
            "The Conflicts filter should contain every current-agent member of a same-agent runtime conflict."
        )
    }

    private func widespreadBaselineFindingsDoNotDrivePerSkillIssuePresentation() throws {
        let skills = [
            skill(id: "alpha", scope: "agent-global", path: "/skills/alpha/SKILL.md", definitionId: "def.alpha", name: "Alpha"),
            skill(id: "beta", scope: "agent-global", path: "/skills/beta/SKILL.md", definitionId: "def.beta", name: "Beta"),
            skill(id: "gamma", scope: "agent-global", path: "/skills/gamma/SKILL.md", definitionId: "def.gamma", name: "Gamma"),
            skill(id: "delta", scope: "agent-global", path: "/skills/delta/SKILL.md", definitionId: "def.delta", name: "Delta"),
            skill(id: "codex-alpha", agent: "codex", scope: "agent-global", path: "/codex/alpha/SKILL.md", definitionId: "codex.alpha", name: "Codex Alpha"),
            skill(id: "codex-beta", agent: "codex", scope: "agent-global", path: "/codex/beta/SKILL.md", definitionId: "codex.beta", name: "Codex Beta"),
            skill(id: "codex-gamma", agent: "codex", scope: "agent-global", path: "/codex/gamma/SKILL.md", definitionId: "codex.gamma", name: "Codex Gamma"),
            skill(id: "codex-delta", agent: "codex", scope: "agent-global", path: "/codex/delta/SKILL.md", definitionId: "codex.delta", name: "Codex Delta"),
        ]
        let findings = [
            Self.finding(id: "exec-alpha", instanceId: "alpha", ruleId: "permissions.exec-needs-human"),
            Self.finding(id: "exec-beta", instanceId: "beta", ruleId: "permissions.exec-needs-human"),
            Self.finding(id: "exec-gamma", instanceId: "gamma", ruleId: "permissions.exec-needs-human"),
            Self.finding(id: "network-alpha", instanceId: "alpha", ruleId: "permissions.network-declared"),
            Self.finding(id: "network-beta", instanceId: "beta", ruleId: "permissions.network-declared"),
            Self.finding(id: "network-gamma", instanceId: "gamma", ruleId: "permissions.network-declared"),
            Self.finding(id: "body-gamma", instanceId: "gamma", ruleId: "body.too-long"),
            Self.finding(id: "sparse-network", instanceId: "codex-alpha", ruleId: "permissions.network-declared"),
            Self.finding(id: "error-network", instanceId: "codex-beta", ruleId: "permissions.network-declared", severity: "error"),
        ]

        try expectEqual(
            SkillListModel.displayFindings(skills: skills, findings: findings).map(\.id),
            ["body-gamma", "error-network"],
            "Built-in declaration-baseline warnings should be omitted at every coverage level, while specific findings and error-severity diagnostics remain visible."
        )
        try expectEqual(
            SkillListModel.issueIndicatorCount(for: skills[0], skills: skills, findings: findings, conflicts: []),
            0,
            "Rows should not show a per-skill issue count when only widespread baseline findings apply."
        )
        try expectEqual(
            SkillListModel.issueIndicatorCount(for: skills[2], skills: skills, findings: findings, conflicts: []),
            1,
            "Rows should still count specific findings after widespread baseline findings are filtered out."
        )
        try expectEqual(
            SkillListModel.displayIssueCount(skills: skills, findings: findings, conflicts: [], agentFilter: .claudeCode),
            1,
            "Sidebar issue metrics should use the same per-skill presentation semantics as rows and filters."
        )
        try expectEqual(
            SkillListModel.filteredAndSorted(
                skills: skills,
                findings: findings,
                conflicts: [],
                searchText: "",
                agentFilter: .all,
                stateFilter: .withFindings,
                sortOrder: .name
            ).map(\.id),
            ["codex-beta", "gamma"],
            "The Issues filter should navigate only to specific or error-severity findings, not declaration-baseline warnings at any coverage level."
        )
    }

    private func triagedAndSuppressedFindingsDoNotDriveVisibleIssuePresentation() throws {
        let skills = [
            skill(id: "alpha", scope: "agent-global", path: "/skills/alpha/SKILL.md", definitionId: "def.alpha", name: "Alpha"),
            skill(id: "beta", scope: "agent-global", path: "/skills/beta/SKILL.md", definitionId: "def.beta", name: "Beta"),
            skill(id: "gamma", scope: "agent-global", path: "/skills/gamma/SKILL.md", definitionId: "def.gamma", name: "Gamma"),
            skill(id: "delta", scope: "agent-global", path: "/skills/delta/SKILL.md", definitionId: "def.delta", name: "Delta"),
            skill(id: "epsilon", scope: "agent-global", path: "/skills/epsilon/SKILL.md", definitionId: "def.epsilon", name: "Epsilon"),
        ]
        let findings = [
            Self.finding(id: "active-alpha", instanceId: "alpha", ruleId: "body.too-long", triageStatus: "open"),
            Self.finding(id: "followup-delta", instanceId: "delta", ruleId: "script.no-shebang", triageStatus: "needs-follow-up"),
            Self.finding(id: "ignored-beta", instanceId: "beta", ruleId: "body.too-long", triageStatus: "ignored"),
            Self.finding(id: "reviewed-gamma", instanceId: "gamma", ruleId: "body.too-long", triageStatus: "reviewed"),
            Self.finding(id: "suppressed-epsilon", instanceId: "epsilon", ruleId: "body.too-long", suppressed: true),
        ]

        try expectEqual(
            SkillListModel.displayFindings(skills: skills, findings: findings).map(\.id),
            ["active-alpha", "followup-delta"],
            "Only active, unsuppressed findings should drive visible issue presentation."
        )
        try expectEqual(
            SkillListModel.filteredAndSorted(
                skills: skills,
                findings: findings,
                conflicts: [],
                searchText: "",
                agentFilter: .all,
                stateFilter: .withFindings,
                sortOrder: .name
            ).map(\.id),
            ["alpha", "delta"],
            "The Issues filter should exclude ignored, reviewed, and suppressed findings."
        )
        try expectEqual(
            SkillListModel.displayIssueCount(skills: skills, findings: findings, conflicts: [], agentFilter: .all),
            2,
            "Sidebar issue metrics should count only active, unsuppressed findings."
        )
        try expectEqual(
            SkillListModel.issueIndicatorCount(for: skills[4], skills: skills, findings: findings, conflicts: []),
            0,
            "Suppressed findings should not show a row issue badge."
        )
    }

    private func declarationBaselineWarningsStayIgnoredForEverySupportedAgent() throws {
        let rules = [
            "frontmatter.tools-not-empty",
            "permissions.network-declared",
            "permissions.exec-needs-human",
        ]
        let skills = SkillAgentFilter.managementCases.enumerated().map { index, agent in
            skill(
                id: "skill-\(agent.rawValue)",
                agent: agent.rawValue,
                scope: "agent-global",
                path: "/\(agent.rawValue)/skills/fixture/SKILL.md",
                definitionId: "def.\(agent.rawValue)",
                name: "Fixture \(index)"
            )
        }
        var findings = skills.enumerated().map { index, skill in
            Self.finding(
                id: "baseline-\(skill.agent)",
                instanceId: skill.id,
                ruleId: rules[index % rules.count]
            )
        }
        findings.append(Self.finding(id: "specific", instanceId: skills[0].id, ruleId: "body.too-long"))

        try expectEqual(
            SkillListModel.displayFindings(skills: skills, findings: findings).map(\.id),
            ["specific"],
            "Every supported agent should apply the same built-in declaration-baseline warning policy."
        )
        for agent in SkillAgentFilter.managementCases {
            let expected = agent == .claudeCode ? 1 : 0
            try expectEqual(
                SkillListModel.displayIssueCount(skills: skills, findings: findings, conflicts: [], agentFilter: agent),
                expected,
                "Filtered issue totals should stay consistent for \(agent.rawValue)."
            )
        }
    }

    private func problemItemsUseCurrentAgentRuntimeSemantics() throws {
        try expectEqual(
            filtered(agentFilter: .claudeCode, stateFilter: .withFindings).map(\.id),
            ["delta", "theta"],
            "Problem items should include broken/unknown Claude Code records but not cross-agent duplicate/source-overlap groups."
        )
        try expectEqual(
            filtered(agentFilter: .codex, stateFilter: .withFindings).map(\.id),
            ["epsilon"],
            "Problem items should include missing records but exclude conflict-only and declaration-baseline findings."
        )
        try expectEqual(
            filtered(agentFilter: .all, stateFilter: .withFindings).map(\.id),
            ["delta", "epsilon", "theta"],
            "The all-agent Problem Items filter should include actionable findings and broken/missing/unknown states without conflict-only or declaration-baseline records."
        )
        try expectEqual(
            SkillListModel.sameAgentConflictGroupCount(skills: Self.skills, conflicts: Self.conflicts),
            1,
            "Presentation conflict count should exclude cross-agent duplicate/source-overlap groups."
        )
    }

    private func scopeFiltersSeparateProjectAndGlobalSkills() throws {
        try expectEqual(filtered(scopeFilter: .project).map(\.id), ["beta"], "Project scope filter")
        try expectEqual(filtered(scopeFilter: .global).map(\.id), ["alpha", "delta", "epsilon", "gamma", "omega", "theta", "zeta"], "Global scope filter")
        try expectEqual(
            filtered(agentFilter: .codex, scopeFilter: .global).map(\.id),
            ["epsilon", "gamma"],
            "Scope filter should compose with the selected agent."
        )
    }

    private func agentFiltersLimitResultsAndGroupsUseStableAdapterOrder() throws {
        try expectEqual(filtered(agentFilter: .all).map(\.id), ["alpha", "beta", "delta", "epsilon", "gamma", "omega", "theta", "zeta"], "All agent filter")
        try expectEqual(filtered(agentFilter: .claudeCode).map(\.id), ["alpha", "beta", "delta", "theta", "zeta"], "Claude Code agent filter")
        try expectEqual(filtered(agentFilter: .codex).map(\.id), ["epsilon", "gamma"], "Codex agent filter")
        try expectEqual(filtered(agentFilter: .opencode).map(\.id), ["omega"], "opencode agent filter")

        let groups = SkillListModel.groupedByAgent(filtered(agentFilter: .all))
        try expectEqual(groups.map(\.title), [UIStrings.claudeCode, UIStrings.codex, UIStrings.opencode], "Agent groups should use display names.")
        try expectEqual(groups.map { $0.skills.map(\.id) }, [["alpha", "beta", "delta", "theta", "zeta"], ["epsilon", "gamma"], ["omega"]], "Agent groups should preserve sorted rows.")
    }

    private func sortOrdersAreStableForCoreListColumns() throws {
        try expectEqual(filtered(sortOrder: .name).map(\.id), ["alpha", "beta", "delta", "epsilon", "gamma", "omega", "theta", "zeta"], "Name sort")
        try expectEqual(filtered(sortOrder: .scope).map(\.id), ["alpha", "delta", "epsilon", "gamma", "omega", "theta", "zeta", "beta"], "Scope sort")
        try expectEqual(filtered(sortOrder: .state).map(\.id), ["delta", "epsilon", "beta", "alpha", "gamma", "omega", "zeta", "theta"], "State sort")
        try expectEqual(filtered(sortOrder: .path).map(\.id), ["epsilon", "gamma", "alpha", "zeta", "omega", "beta", "delta", "theta"], "Path sort")
    }

    private func sortDirectionCanReverseCoreListColumns() throws {
        try expectEqual(
            filtered(sortOrder: .name, sortDirection: .descending).map(\.id),
            ["zeta", "theta", "omega", "gamma", "epsilon", "delta", "beta", "alpha"],
            "Name descending sort"
        )
    }

    private func skillProvenanceClassifiesAgentRootsDeterministically() throws {
        let opencodeProject = Self.identityRecord(
            agent: "opencode",
            scope: "agent-project",
            path: "/repo/.opencode/skills/foo/SKILL.md"
        )
        let opencodeGlobal = Self.identityRecord(
            agent: "opencode",
            scope: "agent-global",
            path: "$HOME/.config/opencode/skills/foo/SKILL.md"
        )
        let opencodeClaudeCompatibility = Self.identityRecord(
            agent: "opencode",
            scope: "agent-project",
            path: "/repo/.claude/skills/foo/SKILL.md"
        )
        let opencodeAgentsCompatibility = Self.identityRecord(
            agent: "opencode",
            scope: "agent-project",
            path: "/repo/.agents/skills/foo/SKILL.md"
        )
        let opencodeConfigured = Self.identityRecord(
            agent: "opencode",
            scope: "agent-global",
            path: "/fixture/custom-opencode-skills/foo/SKILL.md"
        )
        let codexAgentsNative = Self.identityRecord(
            agent: "codex",
            scope: "agent-project",
            path: "/repo/.agents/skills/foo/SKILL.md"
        )
        let claudeAgentsCompatibility = Self.identityRecord(
            agent: "claude-code",
            scope: "agent-project",
            path: "/repo/.agents/skills/foo/SKILL.md"
        )
        let claudeGlobalDisplayAgent = Self.identityRecord(
            agent: "Claude Code",
            scope: "Agent Global",
            path: "~/.claude/skills/foo/SKILL.md"
        )
        let claudeDisplayPathOnly = Self.identityRecord(
            agent: "Claude Code",
            scope: "Agent Global",
            path: "stable-instance-id",
            displayPath: "$HOME/.claude/skills/foo/SKILL.md"
        )
        let piDirectorySkill = Self.identityRecord(
            agent: "pi",
            scope: "agent-global",
            path: "$HOME/.pi/skills/foo/SKILL.md"
        )
        let piDirectDocument = Self.identityRecord(
            agent: "pi",
            scope: "agent-global",
            path: "$HOME/.pi/skills/foo.md"
        )
        let hermesSkill = Self.identityRecord(
            agent: "hermes",
            scope: "agent-global",
            path: "$HOME/.hermes/skills/foo/SKILL.md"
        )
        let hermesExternalSkill = Self.identityRecord(
            agent: "hermes",
            scope: "agent-external",
            path: "/mnt/shared/hermes-skills/foo/SKILL.md"
        )
        let openClawSkill = Self.identityRecord(
            agent: "openclaw",
            scope: "agent-project",
            path: "/repo/skills/foo/SKILL.md"
        )
        let codexPluginSkill = SkillRecord(
            id: "codex-plugin",
            agent: "codex",
            scope: "agent-global",
            path: "/home/.codex/plugins/cache/openai-bundled/browser/1.10.0/skills/control/SKILL.md",
            displayPath: "$HOME/.codex/plugins/cache/openai-bundled/browser/1.10.0/skills/control/SKILL.md",
            definitionId: "browser-control",
            name: "Browser Control",
            state: "loaded",
            enabled: true,
            publisher: "openai-bundled",
            packageName: "browser",
            packageVersion: "1.10.0",
            sourceKind: "chatgpt-plugin-cache",
            readOnlyReason: "Managed by the ChatGPT plugin cache"
        )

        try expectEqual(opencodeProject.provenance.rootKind, .native, "opencode project .opencode roots should be native.")
        try expectEqual(opencodeProject.provenance.scopeKind, .project, "opencode project .opencode roots should remain project scoped.")
        try expectEqual(opencodeProject.provenance.label, "opencode native project", "opencode project native label")
        try expectEqual(opencodeGlobal.provenance.rootKind, .native, "opencode ~/.config/opencode roots should be native.")
        try expectEqual(opencodeGlobal.provenance.scopeKind, .global, "opencode ~/.config/opencode roots should be global scoped.")
        try expectEqual(opencodeGlobal.provenance.label, "opencode native global", "opencode global native label")
        try expectEqual(opencodeClaudeCompatibility.provenance.rootKind, .compatibility, "opencode .claude roots should be compatibility roots.")
        try expectEqual(opencodeAgentsCompatibility.provenance.rootKind, .compatibility, "opencode .agents roots should be compatibility roots.")
        try expectEqual(opencodeConfigured.provenance.rootKind, .configured, "opencode skills.paths rows should be configured roots.")
        try expectEqual(opencodeConfigured.provenance.label, "opencode configured global", "opencode configured root label")
        try expectEqual(codexAgentsNative.provenance.rootKind, .native, "Codex .agents roots should be native roots.")
        try expectEqual(claudeAgentsCompatibility.provenance.rootKind, .unknown, "Claude Code should not treat .agents roots as native Claude roots.")
        try expectEqual(claudeGlobalDisplayAgent.provenance.rootKind, .native, "Claude Code display agent and tilde .claude roots should be native.")
        try expectEqual(claudeGlobalDisplayAgent.provenance.label, "Claude Code native global", "Claude Code display agent label")
        try expectEqual(claudeDisplayPathOnly.provenance.rootKind, .native, "Display path should classify provenance when path is a stable record ID.")
        try expectEqual(piDirectorySkill.isCatalogedSkillIdentity, true, "Pi directory SKILL.md records should remain cataloged skills.")
        try expectEqual(piDirectorySkill.catalogIdentityPath, "$HOME/.pi/skills/foo", "Pi directory SKILL.md identity should use its containing directory.")
        try expectEqual(piDirectDocument.isCatalogedSkillIdentity, false, "Pi direct .md files should not be treated as cataloged skills.")
        try expectEqual(piDirectDocument.provenance.label, "Pi document (not cataloged)", "Pi direct .md label")
        try expectEqual(hermesSkill.provenance.label, "Hermes home/profile read-only", "Hermes home/profile roots should be explicit.")
        try expectEqual(hermesExternalSkill.provenance.rootKind, .external, "Hermes external dirs should be modeled as external roots.")
        try expectEqual(hermesExternalSkill.provenance.scopeKind, .external, "Hermes external dirs should not be treated as project scope.")
        try expectEqual(hermesExternalSkill.provenance.label, "Hermes explicit external read-only", "Hermes external dirs should retain read-only provenance.")
        try expectEqual(openClawSkill.provenance.label, "OpenClaw workspace read-only", "OpenClaw should present project rows as workspace read-only provenance.")
        try expectEqual(codexPluginSkill.provenance.rootKind, .readOnly, "ChatGPT plugin cache skills must be classified read-only.")
        try expectEqual(codexPluginSkill.provenance.label, "Codex ChatGPT plugin · openai-bundled/browser 1.10.0", "Plugin provenance should identify its package.")
        try expectEqual(codexPluginSkill.readOnlyReason, "Managed by the ChatGPT plugin cache", "Plugin cache ownership should remain visible.")
        try expectEqual(DisplayText.scope(for: openClawSkill), UIStrings.openClawWorkspaceScope, "OpenClaw project rows should display as workspace scope.")
    }

    private func privacyPathDisplayRedactsAndCollapsesLocalPaths() throws {
        let rawPath = "/" + "Users" + "/alice/example-project/.agents/skills/very-long-skill-name-with-extra-path-segments/SKILL.md"
        let redacted = DisplayText.privacyPath(rawPath, privacyModeEnabled: true)
        try expectFalse(redacted.contains("/" + "Users" + "/alice"), "Screenshot privacy mode should redact local macOS user paths.")
        try expectFalse(!redacted.contains("$HOME"), "Screenshot privacy mode should preserve useful home-root context.")
        try expectFalse(!redacted.contains("/.../"), "Long screenshot-safe paths should be collapsed by default.")

        let revealed = DisplayText.privacyPath(rawPath, privacyModeEnabled: true, revealFull: true)
        try expectEqual(revealed, rawPath, "Explicit reveal should show the original path without mutating the model value.")
    }

    private func privacyPathDisplayRedactsEmbeddedEvidencePaths() throws {
        let evidence = "session:evidence source=/" + "Users" + "/alice/example-project/.agents/skills/review/SKILL.md"
        try expectEqual(DisplayText.isLikelyPath(evidence), true, "Evidence strings with embedded local paths should use privacy rendering.")

        let redacted = DisplayText.privacyPath(evidence, privacyModeEnabled: true)
        try expectFalse(redacted.contains("/" + "Users" + "/alice"), "Embedded evidence paths should redact local macOS user paths.")
        try expectFalse(!redacted.contains("$HOME"), "Embedded evidence paths should preserve useful redacted home context.")

        let tempEvidence = "capture=/" + "private" + "/" + "var" + "/folders/ab/cd/ef/T/completed.png"
        let redactedTemp = DisplayText.privacyPath(tempEvidence, privacyModeEnabled: true)
        try expectFalse(redactedTemp.contains("/" + "private" + "/var/folders"), "Private temp evidence paths should redact as a single temp placeholder.")
        try expectFalse(!redactedTemp.contains("<temp>/T/completed.png"), "Private temp evidence paths should retain useful screenshot filename context.")
    }

    private func skillIdentitySummaryAndDedupeExplanationAreStable() throws {
        let native = Self.identityRecord(
            id: "native",
            agent: "opencode",
            scope: "agent-project",
            path: "/repo//.opencode/skills/Foo/SKILL.md",
            definitionId: "Shared.Skill",
            name: "Foo"
        )
        let compatibility = Self.identityRecord(
            id: "compatibility",
            agent: "opencode",
            scope: "agent-project",
            path: "/repo/.claude/skills/foo/SKILL.md",
            definitionId: "shared.skill",
            name: "Foo"
        )
        let summary = native.identitySummary
        try expectEqual(summary.title, "Foo", "Identity summary should expose a stable display title.")
        try expectEqual(summary.identityKey, "definition:shared.skill", "Identity key should prefer canonical definition ID.")
        try expectEqual(summary.sourceKey, "opencode|agent-project|/repo/.opencode/skills/foo", "Source key should be canonical and deterministic.")
        try expectEqual(summary.catalogPath, "/repo/.opencode/skills/Foo", "Directory SKILL.md identity should use the containing directory.")
        try expectEqual(summary.provenanceLabel, "opencode native project", "Identity summary should carry provenance label.")

        let forward = native.dedupeExplanation(comparedWith: compatibility)
        let reverse = compatibility.dedupeExplanation(comparedWith: native)
        try expectEqual(forward.reason, .definitionId, "Dedupe should prefer definition ID matches.")
        try expectEqual(forward.summary, "Same definition ID: shared.skill", "Dedupe explanation should use canonical definition ID.")
        try expectEqual(forward, reverse, "Pairwise dedupe explanation should not depend on call order.")
    }

    private static func identityRecord(
        id: String = "identity",
        agent: String,
        scope: String,
        path: String,
        displayPath: String? = nil,
        definitionId: String = "identity.definition",
        name: String = "Identity",
        state: String = "loaded",
        enabled: Bool = true
    ) -> SkillRecord {
        SkillRecord(
            id: id,
            agent: agent,
            scope: scope,
            path: path,
            displayPath: displayPath ?? path,
            definitionId: definitionId,
            name: name,
            state: state,
            enabled: enabled
        )
    }

    private func filtered(
        searchText: String = "",
        agentFilter: SkillAgentFilter = .all,
        stateFilter: SkillStateFilter = .all,
        scopeFilter: SkillScopeFilter = .all,
        sortOrder: SkillSortOrder = .name,
        sortDirection: SkillSortDirection = .ascending
    ) -> [SkillRecord] {
        SkillListModel.filteredAndSorted(
            skills: Self.skills,
            findings: Self.findings,
            conflicts: Self.conflicts,
            searchText: searchText,
            agentFilter: agentFilter,
            stateFilter: stateFilter,
            scopeFilter: scopeFilter,
            sortOrder: sortOrder,
            sortDirection: sortDirection
        )
    }

    private static let skills: [SkillRecord] = [
        skill(
            id: "beta",
            scope: "agent-project",
            path: "/tmp/project/beta/SKILL.md",
            definitionId: "def.beta",
            name: "Beta",
            state: "loaded",
            enabled: false
        ),
        skill(
            id: "gamma",
            agent: "codex",
            scope: "agent-global",
            path: "/tmp/codex/skills/gamma/SKILL.md",
            definitionId: "codex:gamma",
            name: "Gamma",
            state: "loaded",
            enabled: true
        ),
        skill(
            id: "epsilon",
            agent: "codex",
            scope: "agent-global",
            path: "/tmp/codex/skills/epsilon/SKILL.md",
            definitionId: "codex:epsilon",
            name: "Epsilon",
            state: "missing",
            enabled: false
        ),
        skill(
            id: "alpha",
            scope: "agent-global",
            path: "/tmp/global/alpha/SKILL.md",
            definitionId: "def.alpha",
            name: "Alpha",
            state: "loaded",
            enabled: true
        ),
        skill(
            id: "zeta",
            scope: "agent-global",
            path: "/tmp/global/zeta/SKILL.md",
            definitionId: "def.zeta",
            name: "Zeta",
            state: "shadowed",
            enabled: true
        ),
        skill(
            id: "delta",
            scope: "agent-global",
            path: "/tmp/project/delta/SKILL.md",
            definitionId: "def.delta",
            name: "Delta",
            state: "broken",
            enabled: false
        ),
        skill(
            id: "omega",
            agent: "opencode",
            scope: "agent-global",
            path: "/tmp/opencode/skills/omega/SKILL.md",
            definitionId: "opencode:omega",
            name: "Omega",
            state: "loaded",
            enabled: true
        ),
        skill(
            id: "theta",
            scope: "agent-global",
            path: "/tmp/project/theta/SKILL.md",
            definitionId: "def.theta",
            name: "Theta",
            state: "root-error",
            enabled: false
        ),
    ]

    private static let findings: [RuleFindingRecord] = [
        RuleFindingRecord(
            id: "finding-instance",
            instanceId: "gamma",
            definitionId: nil,
            ruleId: "frontmatter.tools-not-empty",
            severity: "warning",
            message: "Tool permissions need review",
            suggestion: nil,
            createdAt: 0
        ),
        RuleFindingRecord(
            id: "finding-definition",
            instanceId: nil,
            definitionId: "def.alpha",
            ruleId: "fingerprint.changed",
            severity: "info",
            message: "Fingerprint changed",
            suggestion: nil,
            createdAt: 0
        ),
    ]

    private static func finding(
        id: String,
        instanceId: String,
        ruleId: String,
        severity: String = "warning",
        suppressed: Bool = false,
        triageStatus: String = "open"
    ) -> RuleFindingRecord {
        RuleFindingRecord(
            id: id,
            instanceId: instanceId,
            definitionId: nil,
            ruleId: ruleId,
            severity: severity,
            message: "\(ruleId) message",
            suggestion: nil,
            createdAt: 0,
            suppressed: suppressed,
            triageStatus: triageStatus
        )
    }

    private static let conflicts: [ConflictGroupRecord] = [
        ConflictGroupRecord(
            id: "conflict-definition",
            definitionId: "def.beta",
            reason: "name-collision",
            winnerId: "beta",
            instanceIds: ["beta", "gamma"]
        ),
        ConflictGroupRecord(
            id: "conflict-instance",
            definitionId: "def.unmatched",
            reason: "path-collision",
            winnerId: nil,
            instanceIds: ["gamma", "epsilon"]
        ),
    ]
}
