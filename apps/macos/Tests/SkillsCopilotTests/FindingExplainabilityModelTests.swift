import Testing
@testable import SkillsCopilot

@Suite("FindingExplainabilityModelTests")
struct FindingExplainabilityModelTests {
    @Test("FindingExplainabilityModelTests")
    func run() throws {
        try classifiesKnownRuleNamespaces()
        try trimsWhitespaceAndLowercasesRuleIDs()
        try limitsRiskCategoryToConfiguredRulesAndScriptNamespace()
        try exposesClassificationThroughFindingRecords()
    }

    private func classifiesKnownRuleNamespaces() throws {
        let cases: [(String, FindingRuleSource, FindingRuleCategory)] = [
            ("frontmatter.required-fields", .frontmatter, .metadata),
            ("permissions.network-declared", .permissions, .permissions),
            ("script.no-shebang", .script, .script),
            ("dependency.unknown", .dependency, .dependency),
            ("path.exists", .path, .filesystem),
            ("fingerprint.changed", .fingerprint, .provenance),
            ("name.canonical-case", .name, .identity),
            ("body.too-long", .body, .content),
            ("custom.rule", .custom, .custom),
        ]

        for (ruleId, expectedSource, expectedCategory) in cases {
            try expectEqual(
                FindingRuleSource.classify(ruleId: ruleId),
                expectedSource,
                "Rule source should classify \(ruleId)."
            )
            try expectEqual(
                FindingRuleCategory.classify(ruleId: ruleId),
                expectedCategory,
                "Rule category should classify \(ruleId)."
            )
        }
    }

    private func trimsWhitespaceAndLowercasesRuleIDs() throws {
        try expectEqual(
            FindingRuleSource.classify(ruleId: " PERMISSIONS.Exec-Needs-Human "),
            .permissions,
            "Rule source classification should be stable across whitespace and case."
        )
        try expectEqual(
            FindingRuleCategory.classify(ruleId: "\nScript.Custom-Risk\t"),
            .script,
            "Rule category classification should be stable across whitespace and case."
        )
    }

    private func limitsRiskCategoryToConfiguredRulesAndScriptNamespace() throws {
        let riskyRules = [
            "frontmatter.tools-not-empty",
            "permissions.network-declared",
            "permissions.exec-needs-human",
            "script.no-shebang",
            "script.custom-risk",
            "dependency.unknown",
        ]
        for ruleId in riskyRules {
            try expectEqual(
                FindingExplainabilityModel.isRiskCategoryRuleID(ruleId),
                true,
                "Risk classifier should include \(ruleId)."
            )
        }

        let nonRiskRules = [
            "frontmatter.required-fields",
            "permissions.documentation-only",
            "path.exists",
            "fingerprint.changed",
            "name.canonical-case",
            "body.too-long",
            "custom.rule",
        ]
        for ruleId in nonRiskRules {
            try expectEqual(
                FindingExplainabilityModel.isRiskCategoryRuleID(ruleId),
                false,
                "Risk classifier should not include \(ruleId)."
            )
        }
    }

    private func exposesClassificationThroughFindingRecords() throws {
        let finding = RuleFindingRecord(
            id: "finding",
            instanceId: "instance",
            definitionId: nil,
            ruleId: " script.user-controlled ",
            severity: "warning",
            message: "Script review required.",
            suggestion: nil,
            createdAt: 1
        )

        try expectEqual(finding.ruleSource, .script, "Finding records should expose the classified rule source.")
        try expectEqual(finding.ruleCategory, .script, "Finding records should expose the classified rule category.")
        try expectEqual(finding.isRiskCategoryFinding, true, "Script finding records should be risk-category findings.")
    }
}
