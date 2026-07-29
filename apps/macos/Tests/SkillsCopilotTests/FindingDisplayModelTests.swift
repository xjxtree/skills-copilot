import Testing
@testable import SkillsCopilot

@Suite("FindingDisplayModelTests")
struct FindingDisplayModelTests {
    @Test("FindingDisplayModelTests")
    func run() throws {
        try groupsSortBySeverityThenRule()
        try filtersBySeverityAndRule()
        try issueGroupsDeduplicateRepeatedScanEntries()
        try issueGroupsExposeExplanationPayload()
        try ruleClassificationIsDeterministic()
        try riskSubsetClassificationMatchesFindingFilters()
        try remediationUsesSuggestionThenRuleFallback()
        try triageDefaultsToOpenAndUpdatesFromServiceState()
        try triageFilterDistinguishesLocalStates()
        try triageFilterOptionsOnlyExposeMeaningfulStates()
        try permissionSummaryLabelsUnknownAndUndeclaredValues()
        try overviewSignalsSkipPlaceholderPermissionNoise()
    }

    private func groupsSortBySeverityThenRule() throws {
        let groups = FindingDisplayModel.grouped(
            findings: Self.findings,
            severityFilter: FindingDisplayModel.allFilterValue,
            ruleFilter: FindingDisplayModel.allFilterValue
        )

        try expectEqual(groups.map(\.severityKey), ["error", "warning", "info"], "Severity groups should use stable priority order.")
        try expectEqual(
            groups.flatMap { $0.issues.map(\.representative.id) },
            ["error-frontmatter", "warning-frontmatter-new", "warning-path", "info-fingerprint"],
            "Finding issue groups should be sorted within severity groups by rule and recency."
        )
    }

    private func filtersBySeverityAndRule() throws {
        let warningGroups = FindingDisplayModel.grouped(
            findings: Self.findings,
            severityFilter: "warning",
            ruleFilter: FindingDisplayModel.allFilterValue
        )

        try expectEqual(warningGroups.map(\.severityKey), ["warning"], "Severity filter should keep one severity group.")
        try expectEqual(warningGroups.first?.issues.map(\.representative.id), ["warning-frontmatter-new", "warning-path"], "Severity filter should keep matching findings.")

        let ruleGroups = FindingDisplayModel.grouped(
            findings: Self.findings,
            severityFilter: FindingDisplayModel.allFilterValue,
            ruleFilter: "frontmatter.required-fields"
        )

        try expectEqual(ruleGroups.map(\.severityKey), ["error", "warning"], "Rule filter should preserve severity grouping.")
        try expectEqual(ruleGroups.flatMap { $0.issues.map(\.representative.id) }, ["error-frontmatter", "warning-frontmatter-new"], "Rule filter should keep matching rule IDs.")
    }

    private func issueGroupsDeduplicateRepeatedScanEntries() throws {
        let groups = FindingDisplayModel.issueGroups(
            findings: [
                Self.finding(
                    id: "duplicate-older",
                    instanceId: "alpha",
                    ruleId: "permissions.network-declared",
                    severity: "warning",
                    message: "Network permission missing",
                    createdAt: 10,
                    suggestion: "Declare network access."
                ),
                Self.finding(
                    id: "duplicate-newer",
                    instanceId: "beta",
                    ruleId: "permissions.network-declared",
                    severity: "warning",
                    message: "Network permission missing",
                    createdAt: 20,
                    suggestion: "Declare network access."
                ),
                Self.finding(
                    id: "distinct-message",
                    instanceId: "gamma",
                    ruleId: "permissions.network-declared",
                    severity: "warning",
                    message: "Different duplicate context",
                    createdAt: 30,
                    suggestion: "Declare network access."
                ),
            ],
            severityFilter: FindingDisplayModel.allFilterValue,
            ruleFilter: FindingDisplayModel.allFilterValue
        )

        try expectEqual(groups.count, 2, "Repeated scan entries with the same rule, severity, message, and remediation should collapse into one issue group.")
        let duplicateGroup = groups.first { $0.message == "Network permission missing" }
        try expectEqual(duplicateGroup?.entryCount, 2, "Issue groups should keep the raw scan entry count for impact context.")
        try expectEqual(duplicateGroup?.impactedInstanceCount, 2, "Issue groups should count impacted skill instances separately from scan entries.")
        try expectEqual(duplicateGroup?.representative.id, "duplicate-newer", "The newest finding should be the representative entry for a deduped issue group.")
    }

    private func issueGroupsExposeExplanationPayload() throws {
        let groups = FindingDisplayModel.issueGroups(
            findings: [
                Self.finding(
                    id: "risk-alpha",
                    instanceId: "alpha",
                    ruleId: "permissions.network-declared",
                    severity: "warning",
                    message: "Network permission missing",
                    createdAt: 10,
                    suggestion: "Declare network access."
                ),
                Self.finding(
                    id: "risk-beta",
                    instanceId: "beta",
                    ruleId: "permissions.network-declared",
                    severity: "warning",
                    message: "Network permission missing",
                    createdAt: 20,
                    suggestion: "Declare network access."
                ),
            ],
            severityFilter: FindingDisplayModel.allFilterValue,
            ruleFilter: FindingDisplayModel.allFilterValue
        )

        let explanation = groups[0].explanation
        try expectEqual(explanation.ruleId, "permissions.network-declared", "Explanation should expose the rule ID.")
        try expectEqual(explanation.severity, "warning", "Explanation should expose normalized severity.")
        try expectEqual(explanation.trigger, "Network permission missing", "Explanation should expose the trigger message.")
        try expectEqual(explanation.remediation, "Declare network access.", "Explanation should expose remediation text.")
        try expectEqual(explanation.affectedInstanceCount, 2, "Explanation should expose impacted instance count.")
        try expectEqual(explanation.scanEntryCount, 2, "Explanation should expose scan entry count.")
        try expectEqual(explanation.ruleSource, .permissions, "Explanation should expose deterministic rule source.")
        try expectEqual(explanation.ruleCategory, .permissions, "Explanation should expose deterministic rule category.")
        try expectEqual(explanation.isRiskCategoryFinding, true, "Explanation should expose risk-category classification.")
    }

    private func ruleClassificationIsDeterministic() throws {
        try expectEqual(FindingDisplayModel.ruleSource(for: "frontmatter.required-fields"), .frontmatter, "Frontmatter rule source")
        try expectEqual(FindingDisplayModel.ruleCategory(for: "frontmatter.required-fields"), .metadata, "Frontmatter rule category")
        try expectEqual(FindingDisplayModel.ruleSource(for: " permissions.exec-needs-human "), .permissions, "Permission rule source should trim whitespace.")
        try expectEqual(FindingDisplayModel.ruleCategory(for: "script.no-shebang"), .script, "Script rule category")
        try expectEqual(FindingDisplayModel.ruleCategory(for: "path.exists"), .filesystem, "Path rule category")
        try expectEqual(FindingDisplayModel.ruleCategory(for: "fingerprint.changed"), .provenance, "Fingerprint rule category")
        try expectEqual(FindingDisplayModel.ruleCategory(for: "name.canonical-case"), .identity, "Name rule category")
        try expectEqual(FindingDisplayModel.ruleCategory(for: "body.too-long"), .content, "Body rule category")
        try expectEqual(FindingDisplayModel.ruleSource(for: "custom.rule"), .custom, "Unknown rule source should be custom.")
        try expectEqual(FindingDisplayModel.ruleCategory(for: "custom.rule"), .custom, "Unknown rule category should be custom.")
    }

    private func riskSubsetClassificationMatchesFindingFilters() throws {
        let riskRuleIDs = [
            "frontmatter.tools-not-empty",
            "permissions.network-declared",
            "permissions.exec-needs-human",
            "script.no-shebang",
            "script.custom-risk",
            "dependency.unknown",
        ]
        try expectEqual(
            riskRuleIDs.map { FindingDisplayModel.isRiskCategoryRuleID($0) },
            Array(repeating: true, count: riskRuleIDs.count),
            "Risk subset should include the existing risky finding filter rules."
        )

        let nonRiskRuleIDs = [
            "frontmatter.required-fields",
            "path.exists",
            "fingerprint.changed",
            "name.canonical-case",
            "body.too-long",
            "custom.rule",
        ]
        try expectEqual(
            nonRiskRuleIDs.map { FindingDisplayModel.isRiskCategoryRuleID($0) },
            Array(repeating: false, count: nonRiskRuleIDs.count),
            "Risk subset should not classify every finding source as a risk."
        )
    }

    private func remediationUsesSuggestionThenRuleFallback() throws {
        let suggested = Self.finding(
            id: "suggested",
            ruleId: "permissions.network-declared",
            severity: "warning",
            createdAt: 50,
            suggestion: "Declare network access in frontmatter."
        )
        try expectEqual(
            FindingDisplayModel.remediationText(for: suggested),
            "Declare network access in frontmatter.",
            "Finding-specific suggestions should remain authoritative."
        )

        let fallback = Self.finding(
            id: "fallback",
            ruleId: "permissions.network-declared",
            severity: "warning",
            createdAt: 60
        )
        try expectEqual(
            FindingDisplayModel.remediationText(for: fallback),
            UIStrings.remediationNetworkDeclared,
            "Known V2.8 rule IDs should get actionable remediation fallback text."
        )

        let generic = Self.finding(
            id: "generic",
            ruleId: "custom.rule",
            severity: "info",
            createdAt: 70
        )
        try expectEqual(
            FindingDisplayModel.remediationText(for: generic),
            UIStrings.findingRemediationFallback("custom.rule"),
            "Unknown rule IDs should still produce a concrete remediation prompt."
        )
    }

    private func triageDefaultsToOpenAndUpdatesFromServiceState() throws {
        let finding = Self.finding(
            id: "triage",
            ruleId: "permissions.network-declared",
            severity: "warning",
            message: "Network permission missing",
            createdAt: 80,
            suggestion: "Declare network access."
        )

        try expectEqual(
            finding.triageState,
            .open,
            "Findings without service triage should default to Open."
        )

        let reviewed = finding.withTriage(status: .reviewed, note: "Checked", updatedAt: 1)
        try expectEqual(
            reviewed.triageState,
            .reviewed,
            "Service-backed triage status should drive the displayed state."
        )
        try expectEqual(reviewed.triageKey, finding.triageKey, "Triage updates should preserve the service triage key.")
        try expectEqual(reviewed.triageNote, "Checked", "Triage notes should roundtrip when provided by the service.")

        let reopened = reviewed.withTriage(status: .open)
        try expectEqual(
            reopened.triageState,
            .open,
            "Clearing service triage should display the issue as Open again."
        )
        try expectEqual(reopened.triageNote, nil, "Reopened findings should not keep stale triage notes.")
    }

    private func triageFilterDistinguishesLocalStates() throws {
        try expectEqual(FindingTriageFilter.active.includes(.open), true, "Active triage should include Open findings.")
        try expectEqual(FindingTriageFilter.active.includes(.needsFollowUp), true, "Active triage should include follow-up findings.")
        try expectEqual(FindingTriageFilter.active.includes(.reviewed), false, "Active triage should hide reviewed findings.")
        try expectEqual(FindingTriageFilter.active.includes(.ignored), false, "Active triage should hide ignored findings.")
        try expectEqual(FindingTriageFilter.reviewed.includes(.reviewed), true, "Reviewed filter should show reviewed findings.")
        try expectEqual(FindingTriageFilter.ignored.includes(.ignored), true, "Ignored filter should show ignored findings.")
        try expectEqual(FindingTriageFilter.all.includes(.ignored), true, "All triage should include ignored findings.")
    }

    private func triageFilterOptionsOnlyExposeMeaningfulStates() throws {
        try expectEqual(
            FindingTriageFilter.availableFilters(for: FindingTriageCounts(open: 2)),
            [.open],
            "Only open issues should not expose duplicate active/all status filters."
        )
        try expectEqual(
            FindingTriageFilter.availableFilters(for: FindingTriageCounts(open: 2, reviewed: 1)),
            [.open, .reviewed, .all],
            "Status filters should include all only when multiple real states exist."
        )
        try expectEqual(
            FindingTriageFilter.availableFilters(for: FindingTriageCounts(open: 2, needsFollowUp: 1)),
            [.open, .needsFollowUp, .all],
            "Open plus follow-up should expose concrete states and an all option without the duplicate active filter."
        )
    }

    private func permissionSummaryLabelsUnknownAndUndeclaredValues() throws {
        let undeclared = PermissionDisplayModel.summary(for: .object([:]))
        try expectEqual(
            undeclared.rows,
            [PermissionSummaryRow(label: UIStrings.permissions, value: UIStrings.permissionUndeclared)],
            "Empty permission payloads should be labeled as undeclared or unknown."
        )
        try expectEqual(undeclared.rawText, "{}", "Empty permission payload raw text should remain visible.")

        let summary = PermissionDisplayModel.summary(
            for: .object([
                "exec": .bool(true),
                "files": .array([]),
                "network": .string("ambient"),
                "requires_human": .bool(false),
                "tools": .array([.string("Read")]),
            ])
        )

        try expectEqual(
            summary.rows.map(\.value),
            [
                "Read",
                UIStrings.permissionNoneDeclared,
                UIStrings.permissionUnknownValue("ambient"),
                UIStrings.permissionRequested,
                UIStrings.permissionNotDeclaredRequired,
            ],
            "Permission summaries should preserve declarations without implying safe or unsafe status."
        )
        try expectEqual(
            summary.rawText,
            "{\"exec\": true, \"files\": [], \"network\": \"ambient\", \"requires_human\": false, \"tools\": [\"Read\"]}",
            "Permission raw text should use stable sorted keys for review."
        )
    }

    private func overviewSignalsSkipPlaceholderPermissionNoise() throws {
        try expectFalse(
            PermissionDisplayModel.hasOverviewSignal(for: .null),
            "Null permission payloads should not create overview risk noise."
        )
        try expectFalse(
            PermissionDisplayModel.hasOverviewSignal(for: .object([:])),
            "Empty permission payloads should stay out of the overview risk panel."
        )
        try expectFalse(
            PermissionDisplayModel.hasOverviewSignal(
                for: .object([
                    "exec": .bool(false),
                    "files": .array([]),
                    "network": .string("none"),
                    "requires_human": .bool(false),
                    "tools": .array([]),
                ])
            ),
            "Explicitly empty permissions should not read as overview risk."
        )
        try expectEqual(
            PermissionDisplayModel.hasOverviewSignal(for: .object(["network": .string("full")])),
            true,
            "Declared network access should remain visible in the overview risk panel."
        )

        let unavailablePreview = ScriptExecutionPreview(
            skillID: "alpha",
            scriptName: nil,
            commandPreview: [],
            scope: ScriptExecutionScope(cwd: nil, env: [:], network: nil, files: []),
            risks: [],
            confirmationRequired: true,
            executionAllowed: false,
            auditStatus: .unavailable,
            auditID: nil,
            summary: "Preview",
            disabledReason: "Unavailable"
        )
        try expectFalse(unavailablePreview.hasOverviewSignal, "Unavailable script previews should not create overview risk noise.")

        let commandPreview = ScriptExecutionPreview(
            skillID: "alpha",
            scriptName: "setup",
            commandPreview: ["./setup.sh"],
            scope: ScriptExecutionScope(cwd: nil, env: [:], network: nil, files: []),
            risks: [],
            confirmationRequired: true,
            executionAllowed: false,
            auditStatus: .requiresConfirmation,
            auditID: nil,
            summary: "Preview",
            disabledReason: nil
        )
        try expectEqual(commandPreview.hasOverviewSignal, true, "Concrete script previews should remain visible in the overview risk panel.")
    }

    private static let findings: [RuleFindingRecord] = [
        finding(
            id: "warning-path",
            ruleId: "path.exists",
            severity: "warning",
            createdAt: 20
        ),
        finding(
            id: "info-fingerprint",
            ruleId: "fingerprint.changed",
            severity: "info",
            createdAt: 30
        ),
        finding(
            id: "warning-frontmatter-new",
            ruleId: "frontmatter.required-fields",
            severity: "warning",
            createdAt: 40
        ),
        finding(
            id: "error-frontmatter",
            ruleId: "frontmatter.required-fields",
            severity: "error",
            createdAt: 10
        ),
    ]

    private static func finding(
        id: String,
        instanceId: String = "alpha",
        ruleId: String,
        severity: String,
        message: String? = nil,
        createdAt: Int64,
        suggestion: String? = nil
    ) -> RuleFindingRecord {
        RuleFindingRecord(
            id: id,
            instanceId: instanceId,
            definitionId: nil,
            ruleId: ruleId,
            severity: severity,
            message: message ?? "Finding \(id)",
            suggestion: suggestion,
            createdAt: createdAt
        )
    }
}
