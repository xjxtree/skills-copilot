import Testing
import Foundation
@testable import SkillsCopilot

@Suite("RuleTuningModelTests")
struct RuleTuningModelTests {
    @Test("RuleTuningModelTests")
    func run() throws {
        try decodesFlexibleRuleTuningPayload()
        try groupSuppressionTakesPrecedenceOverRuleState()
        try severityNormalizationFallsBackSafely()
    }

    private func decodesFlexibleRuleTuningPayload() throws {
        let data = Data(
            """
            {
              "rules": [
                {
                  "rule_id": "permissions.network-declared",
                  "base_severity": "warning",
                  "severity_override": "info",
                  "effective_severity": "info",
                  "suppressed": false,
                  "updated_at": 10
                },
                {
                  "rule_id": "script.no-shebang",
                  "scope": "finding_group",
                  "finding_group_id": "warning|script.no-shebang|message",
                  "severity": "error",
                  "is_suppressed": true,
                  "note": "Local exception"
                }
              ]
            }
            """.utf8
        )

        let list = try JSONDecoder().decode(RuleTuningList.self, from: data)

        try expectEqual(list.records.count, 2, "Rule tuning should decode object-wrapped service payloads.")
        try expectEqual(list.records[0].ruleId, "permissions.network-declared", "Rule ID should decode from snake case.")
        try expectEqual(list.records[0].defaultSeverity, "warning", "Base severity should decode as default severity.")
        try expectEqual(list.records[0].severityOverride, "info", "Severity override should decode from service payload.")
        try expectEqual(list.records[1].scope, .findingGroup, "Finding group suppression scope should decode from service spelling.")
        try expectEqual(list.records[1].suppressed, true, "Suppression should decode from is_suppressed fallback.")
        try expectEqual(list.records[1].suppressionReason, "Local exception", "Suppression notes should decode as review context.")
    }

    private func groupSuppressionTakesPrecedenceOverRuleState() throws {
        let records = [
            RuleTuningRecord(
                ruleId: "permissions.network-declared",
                defaultSeverity: "warning",
                severityOverride: "error",
                suppressed: false
            ),
            RuleTuningRecord(
                ruleId: "permissions.network-declared",
                scope: .findingGroup,
                findingGroupId: "group-a",
                effectiveSeverity: "info",
                suppressed: true
            ),
        ]

        try expectEqual(
            RuleTuningModel.effectiveSeverity(
                records: records,
                ruleId: "permissions.network-declared",
                findingGroupId: "group-a",
                fallbackSeverity: "warning"
            ),
            "info",
            "Finding-group tuning should override rule-wide display state for that group."
        )
        try expectEqual(
            RuleTuningModel.isSuppressed(records: records, ruleId: "permissions.network-declared", findingGroupId: "group-a"),
            true,
            "Finding-group suppression should be visible even when the rule-wide record is not suppressed."
        )
        try expectEqual(
            RuleTuningModel.isSuppressed(records: records, ruleId: "permissions.network-declared", findingGroupId: "group-b"),
            false,
            "Other groups should not inherit a group-specific suppression."
        )
    }

    private func severityNormalizationFallsBackSafely() throws {
        let record = RuleTuningRecord(
            ruleId: "body.too-long",
            defaultSeverity: " Warning ",
            severityOverride: " INFO "
        )

        try expectEqual(record.defaultSeverity, "warning", "Default severity should normalize whitespace and case.")
        try expectEqual(record.severityOverride, "info", "Override severity should normalize whitespace and case.")
        try expectEqual(record.effectiveSeverity, "info", "Override severity should drive effective severity when present.")
        try expectEqual(
            RuleTuningModel.effectiveSeverity(records: [], ruleId: "body.too-long", findingGroupId: nil, fallbackSeverity: "Error"),
            "error",
            "Missing service state should fall back to current finding severity."
        )
    }
}
