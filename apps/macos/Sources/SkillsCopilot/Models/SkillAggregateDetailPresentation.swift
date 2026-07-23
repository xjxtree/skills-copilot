import Foundation

enum SkillAggregatePackageAction: String, CaseIterable, Identifiable {
    case add
    case detail
    case update
    case remove

    var id: String { rawValue }
}

enum SkillAggregateConfigAction: String, CaseIterable, Identifiable {
    case enable
    case disable

    var id: String { rawValue }
}

enum SkillAggregateDetailLayer: String, CaseIterable, Identifiable {
    case answer
    case evidence
    case advanced

    var id: String { rawValue }

    var title: String {
        switch self {
        case .answer:
            UIStrings.text("skillAggregate.detail.answer", "Answer")
        case .evidence:
            UIStrings.text("skillAggregate.detail.evidence", "Evidence")
        case .advanced:
            UIStrings.text("skillAggregate.detail.advanced", "Advanced")
        }
    }

    var systemImage: String {
        switch self {
        case .answer: "checkmark.seal"
        case .evidence: "doc.text.magnifyingglass"
        case .advanced: "wrench.and.screwdriver"
        }
    }
}

struct SkillAggregateDetailPresentation {
    struct StateCount: Identifiable, Hashable {
        let state: SkillEffectivenessState
        let count: Int

        var id: SkillEffectivenessState { state }
    }

    struct Instance: Identifiable, Hashable {
        let record: SkillInstanceEffectivenessRecord
        let agentText: String
        let scopeText: String
        let effectivenessText: String
        let coverageText: String
        let evidenceRefLabels: [String]
        let actionLabels: [String]

        var id: String { record.id }

        var locationText: String {
            [agentText, scopeText].filter { !$0.isEmpty }.joined(separator: " · ")
        }
    }

    struct Evidence: Identifiable, Hashable {
        let reference: EvidenceRef
        let idLabel: String
        let summary: String

        var id: String { reference.id }
    }

    struct TypedAction: Identifiable, Hashable {
        let descriptor: ActionDescriptorWire
        let intentLabel: String
        let targetLabel: String

        var id: String { descriptor.id }
    }

    let aggregate: SkillAggregateRecord
    let displayName: String
    let purpose: String
    let provenanceLabel: String
    let packageLabel: String?
    let stateCounts: [StateCount]
    let instances: [Instance]
    let evidence: [Evidence]
    let typedActions: [TypedAction]
    let effectiveLocations: [String]
    let advancedMetadata: [CompactMetadataRow]

    init(aggregate: SkillAggregateRecord) {
        self.aggregate = aggregate
        displayName = Self.safeOptionalDisplayText(aggregate.displayName)
            ?? Self.safeOptionalDisplayText(aggregate.canonicalName)
            ?? UIStrings.text("skillAggregate.detail.unnamed", "Unnamed skill")
        purpose = Self.safeDisplayText(
            aggregate.description,
            fallback: UIStrings.text(
                "skillAggregate.detail.purposeUnavailable",
                "No safe capability description is available."
            )
        )
        provenanceLabel = Self.logicalProvenanceLabel(for: aggregate.sourceKind)
        packageLabel = Self.packageLabel(
            publisher: aggregate.publisher,
            packageName: aggregate.packageName,
            version: aggregate.packageVersion
        )
        stateCounts = aggregate.effectivenessCounts
            .map { StateCount(state: $0.state, count: $0.count) }
            .sorted {
                if $0.state.severityRank == $1.state.severityRank {
                    return $0.state.rawValue < $1.state.rawValue
                }
                return $0.state.severityRank < $1.state.severityRank
            }
        instances = aggregate.instanceEffectiveness.map { record in
            let agentText = record.agent.map {
                DisplayText.agent($0.rawValue)
            } ?? UIStrings.text("skillAggregate.detail.agentShared", "Shared")
            return Instance(
                record: record,
                agentText: agentText,
                scopeText: DisplayText.scope(
                    record.scope.rawValue,
                    agent: record.agent?.rawValue ?? ""
                ),
                effectivenessText: Self.effectivenessLabel(record.state),
                coverageText: Self.coverageLabel(record.coverage),
                evidenceRefLabels: record.evidenceRefs.enumerated().map { index, value in
                    Self.safeReferenceText(
                        value,
                        fallback: String(
                            format: UIStrings.text(
                                "skillAggregate.detail.evidenceReferenceNumber",
                                "Evidence reference %d"
                            ),
                            index + 1
                        )
                    )
                },
                actionLabels: record.actionIDs.enumerated().map { index, value in
                    Self.safeReferenceText(
                        value,
                        fallback: String(
                            format: UIStrings.text(
                                "skillAggregate.detail.actionReferenceNumber",
                                "Action reference %d"
                            ),
                            index + 1
                        )
                    )
                }
            )
        }
        evidence = aggregate.evidence.map { reference in
            Evidence(
                reference: reference,
                idLabel: Self.safeReferenceText(
                    reference.id,
                    fallback: UIStrings.text(
                        "skillAggregate.detail.evidenceReference",
                        "Evidence reference"
                    )
                ),
                summary: Self.safeDisplayText(
                    reference.summary,
                    fallback: UIStrings.text(
                        "skillAggregate.detail.evidenceSummaryUnavailable",
                        "Evidence summary unavailable."
                    )
                )
            )
        }
        typedActions = aggregate.actions.map { descriptor in
            TypedAction(
                descriptor: descriptor,
                intentLabel: Self.safeDisplayText(
                    descriptor.intent.replacingOccurrences(of: "_", with: " ").capitalized,
                    fallback: UIStrings.text(
                        "skillAggregate.detail.typedAction",
                        "Typed action"
                    )
                ),
                targetLabel: Self.safeReferenceText(
                    descriptor.target.id,
                    fallback: UIStrings.text(
                        "skillAggregate.detail.actionTarget",
                        "Logical action target"
                    )
                )
            )
        }
        effectiveLocations = instances
            .filter { $0.record.state == .effective }
            .map(\.locationText)
            .uniquedPreservingOrder()
        advancedMetadata = Self.metadataRows(
            aggregate: aggregate,
            provenanceLabel: provenanceLabel,
            packageLabel: packageLabel
        )
    }

    var needsAttention: Bool {
        aggregate.primaryEffectiveness != .effective
            || aggregate.findingCount > 0
            || aggregate.conflictCount > 0
            || !aggregate.coverage.isComplete
    }

    var attentionTitle: String {
        needsAttention
            ? UIStrings.text("skillAggregate.detail.attentionRequired", "Needs attention")
            : UIStrings.text("skillAggregate.detail.noAttention", "No action required")
    }

    var attentionExplanation: String {
        if !aggregate.coverage.isComplete {
            return UIStrings.text(
                "skillAggregate.detail.incompleteEvidence",
                "Required evidence is incomplete, so this capability cannot be presented as healthy."
            )
        }
        if aggregate.primaryEffectiveness != .effective {
            return String(
                format: UIStrings.text(
                    "skillAggregate.detail.stateAttention",
                    "At least one instance is %@."
                ),
                Self.effectivenessLabel(aggregate.primaryEffectiveness).lowercased()
            )
        }
        if aggregate.findingCount > 0 || aggregate.conflictCount > 0 {
            return UIStrings.text(
                "skillAggregate.detail.reviewIssues",
                "Review the current findings or conflicts before relying on every instance."
            )
        }
        return UIStrings.text(
            "skillAggregate.detail.verified",
            "Every inspected instance is accounted for and no current issue requires action."
        )
    }

    var effectiveLocationText: String {
        effectiveLocations.isEmpty
            ? UIStrings.text(
                "skillAggregate.detail.noEffectiveLocations",
                "No verified effective location"
            )
            : effectiveLocations.joined(separator: ", ")
    }

    var coverageText: String {
        Self.coverageLabel(aggregate.coverage)
    }

    static func effectivenessLabel(_ state: SkillEffectivenessState) -> String {
        switch state {
        case .effective:
            UIStrings.text("skillAggregate.state.effective", "Verified effective")
        case .disabled:
            UIStrings.text("skillAggregate.state.disabled", "Disabled")
        case .shadowed:
            UIStrings.text("skillAggregate.state.shadowed", "Shadowed")
        case .installedUnlinked:
            UIStrings.text(
                "skillAggregate.state.installedUnlinked",
                "Installed but unlinked"
            )
        case .broken:
            UIStrings.text("skillAggregate.state.broken", "Broken")
        case .unavailable:
            UIStrings.text("skillAggregate.state.unavailable", "Unavailable")
        }
    }

    static func logicalProvenanceLabel(for sourceKind: String) -> String {
        let normalized = sourceKind
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
            .replacingOccurrences(of: "_", with: "-")
        if normalized == "chatgpt-plugin-cache"
            || normalized == "codex-plugin"
            || normalized.contains("plugin-manifest") {
            return UIStrings.text(
                "skillAggregate.provenance.plugin",
                "Installed agent plugin"
            )
        }
        if normalized.contains("compat") {
            return UIStrings.text(
                "skillAggregate.provenance.compatibility",
                "Agent compatibility source"
            )
        }
        if normalized.contains("manager") || normalized.contains("package") {
            return UIStrings.text(
                "skillAggregate.provenance.manager",
                "Skill Manager package"
            )
        }
        if normalized.contains("native") {
            return UIStrings.text(
                "skillAggregate.provenance.native",
                "Native agent source"
            )
        }
        if normalized.contains("local") {
            return UIStrings.text(
                "skillAggregate.provenance.local",
                "Local skill"
            )
        }
        return UIStrings.text(
            "skillAggregate.provenance.declared",
            "Agent-declared source"
        )
    }

    static func coverageLabel(_ coverage: SourceCoverage) -> String {
        let counts: String
        if let expected = coverage.expectedSources {
            counts = String(
                format: UIStrings.text(
                    "skillAggregate.coverage.known",
                    "%d of %d sources inspected"
                ),
                coverage.inspectedSources,
                expected
            )
        } else {
            counts = String(
                format: UIStrings.text(
                    "skillAggregate.coverage.unknown",
                    "%d sources inspected; expected total unavailable"
                ),
                coverage.inspectedSources
            )
        }
        guard !coverage.isComplete else { return counts }
        return "\(counts) · \(incompleteReasonLabel(coverage.incompleteReason))"
    }

    private static func incompleteReasonLabel(_ reason: ListIncompleteReason?) -> String {
        switch reason {
        case .safetyBudget:
            UIStrings.text("skillAggregate.coverage.safetyBudget", "safety limit reached")
        case .sourceChanged:
            UIStrings.text("skillAggregate.coverage.sourceChanged", "source changed")
        case .sourceLimited:
            UIStrings.text("skillAggregate.coverage.sourceLimited", "source limited")
        case .unreadableSource:
            UIStrings.text("skillAggregate.coverage.unreadable", "source unreadable")
        case .pageFailed:
            UIStrings.text("skillAggregate.coverage.pageFailed", "page unavailable")
        case .unsupportedProtocol:
            UIStrings.text("skillAggregate.coverage.unsupported", "unsupported inventory")
        case .staleSource:
            UIStrings.text("skillAggregate.coverage.stale", "stale evidence")
        case .notInspected:
            UIStrings.text("skillAggregate.coverage.notInspected", "not inspected")
        case nil:
            UIStrings.text("skillAggregate.coverage.incomplete", "incomplete evidence")
        }
    }

    private static func packageLabel(
        publisher: String?,
        packageName: String?,
        version: String?
    ) -> String? {
        let safePublisher = publisher.flatMap {
            safeOptionalDisplayText($0)
        }
        let safeName = packageName.flatMap {
            safeOptionalDisplayText($0)
        }
        let safeVersion = version.flatMap {
            safeOptionalDisplayText($0)
        }
        let identity = [safePublisher, safeName]
            .compactMap { $0 }
            .joined(separator: " / ")
        guard !identity.isEmpty else { return nil }
        if let safeVersion {
            return "\(identity) \(safeVersion)"
        }
        return identity
    }

    private static func metadataRows(
        aggregate: SkillAggregateRecord,
        provenanceLabel: String,
        packageLabel: String?
    ) -> [CompactMetadataRow] {
        let unavailable = UIStrings.text(
            "skillAggregate.metadata.unavailable",
            "Unavailable"
        )
        let definition = safeOptionalDisplayText(aggregate.definitionID) ?? unavailable
        let sourceRevision = safeOptionalDisplayText(aggregate.sourceRevision) ?? unavailable
        var rows = [
            CompactMetadataRow(
                label: UIStrings.text("skillAggregate.metadata.definition", "Definition"),
                value: definition,
                systemImage: "doc.text",
                isCopyable: definition != unavailable
            ),
            CompactMetadataRow(
                label: UIStrings.text("skillAggregate.metadata.provenance", "Provenance"),
                value: provenanceLabel,
                systemImage: "shippingbox",
                isCopyable: false
            ),
            CompactMetadataRow(
                label: UIStrings.text("skillAggregate.metadata.logicalSource", "Logical source"),
                value: aggregate.sourceIdentity,
                systemImage: "link",
                isCopyable: true
            ),
            CompactMetadataRow(
                label: UIStrings.text("skillAggregate.metadata.runtime", "Runtime identity"),
                value: aggregate.runtimeIdentity,
                systemImage: "terminal",
                isCopyable: true
            ),
            CompactMetadataRow(
                label: UIStrings.text("skillAggregate.metadata.revision", "Source revision"),
                value: sourceRevision,
                systemImage: "arrow.triangle.branch",
                isCopyable: sourceRevision != unavailable
            ),
        ]
        if let fingerprint = aggregate.definitionFingerprint.flatMap(safeOptionalDisplayText) {
            rows.append(
                CompactMetadataRow(
                    label: UIStrings.text(
                        "skillAggregate.metadata.fingerprint",
                        "Definition fingerprint"
                    ),
                    value: fingerprint,
                    systemImage: "number",
                    isCopyable: true
                )
            )
        }
        if let packageLabel {
            rows.append(
                CompactMetadataRow(
                    label: UIStrings.text("skillAggregate.metadata.package", "Package"),
                    value: packageLabel,
                    systemImage: "shippingbox",
                    isCopyable: false
                )
            )
        }
        if aggregate.readOnlyReason != nil {
            rows.append(
                CompactMetadataRow(
                    label: UIStrings.text("skillAggregate.metadata.access", "Access"),
                    value: UIStrings.text(
                        "skillAggregate.metadata.readOnly",
                        "Read-only source"
                    ),
                    systemImage: "lock",
                    isCopyable: false
                )
            )
        }
        return rows
    }

    private static func safeOptionalDisplayText(_ value: String) -> String? {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, !containsPhysicalPath(trimmed) else { return nil }
        return trimmed
    }

    private static func safeDisplayText(_ value: String, fallback: String) -> String {
        safeOptionalDisplayText(value) ?? fallback
    }

    private static func safeReferenceText(_ value: String, fallback: String) -> String {
        safeOptionalDisplayText(value) ?? fallback
    }

    private static func containsPhysicalPath(_ value: String) -> Bool {
        let normalized = value.lowercased()
        if normalized.contains("file://")
            || normalized.contains("plugins/cache")
            || normalized.contains(#":\"#)
            || normalized.contains(#"\\"#) {
            return true
        }
        return value.split(whereSeparator: \.isWhitespace).contains { word in
            let token = word.trimmingCharacters(
                in: CharacterSet(charactersIn: "()[]{}\"',;")
            )
            return token.count > 1 && (token.hasPrefix("/") || token.hasPrefix("~/"))
        }
    }
}

private extension Array where Element == String {
    func uniquedPreservingOrder() -> [String] {
        var seen = Set<String>()
        return filter { seen.insert($0).inserted }
    }
}
