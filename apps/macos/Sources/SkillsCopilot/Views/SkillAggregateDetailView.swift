import SwiftUI

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

struct SkillAggregateDetailView: View {
    let presentation: SkillAggregateDetailPresentation
    let availablePackageActions: Set<SkillAggregatePackageAction>
    let availableConfigActions: Set<SkillAggregateConfigAction>
    let onPackageAction: ((SkillAggregatePackageAction) -> Void)?
    let onConfigAction: ((SkillAggregateConfigAction) -> Void)?
    let onContextualIntelligence: (() -> Void)?
    @State private var selectedLayer: SkillAggregateDetailLayer = .answer

    init(
        aggregate: SkillAggregateRecord,
        availablePackageActions: Set<SkillAggregatePackageAction> = [],
        availableConfigActions: Set<SkillAggregateConfigAction> = [],
        onPackageAction: ((SkillAggregatePackageAction) -> Void)? = nil,
        onConfigAction: ((SkillAggregateConfigAction) -> Void)? = nil,
        onContextualIntelligence: (() -> Void)? = nil
    ) {
        presentation = SkillAggregateDetailPresentation(aggregate: aggregate)
        self.availablePackageActions = availablePackageActions
        self.availableConfigActions = availableConfigActions
        self.onPackageAction = onPackageAction
        self.onConfigAction = onConfigAction
        self.onContextualIntelligence = onContextualIntelligence
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            layerPicker
            Divider()
            ScrollView {
                Group {
                    switch selectedLayer {
                    case .answer:
                        answerLayer
                    case .evidence:
                        evidenceLayer
                    case .advanced:
                        advancedLayer
                    }
                }
                .padding(18)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .frame(minWidth: 520, minHeight: 460)
    }

    private var header: some View {
        HStack(alignment: .top, spacing: 14) {
            Image(systemName: "square.stack.3d.up")
                .font(.title2)
                .foregroundStyle(.secondary)
                .frame(width: 30)

            VStack(alignment: .leading, spacing: 4) {
                Text(presentation.displayName)
                    .font(.title2.bold())
                    .lineLimit(2)
                Text(presentation.provenanceLabel)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            Spacer()

            SkillAggregateStateBadge(
                title: SkillAggregateDetailPresentation.effectivenessLabel(
                    presentation.aggregate.primaryEffectiveness
                ),
                state: presentation.aggregate.primaryEffectiveness
            )
        }
        .padding(18)
    }

    private var layerPicker: some View {
        Picker(
            UIStrings.text("skillAggregate.detail.layer", "Detail layer"),
            selection: $selectedLayer
        ) {
            ForEach(SkillAggregateDetailLayer.allCases) { layer in
                Label(layer.title, systemImage: layer.systemImage)
                    .tag(layer)
            }
        }
        .pickerStyle(.segmented)
        .labelsHidden()
        .padding(.horizontal, 18)
        .padding(.vertical, 10)
    }

    private var answerLayer: some View {
        VStack(alignment: .leading, spacing: 16) {
            layerHeading(
                UIStrings.text("skillAggregate.detail.answer", "Answer"),
                subtitle: UIStrings.text(
                    "skillAggregate.detail.answerSubtitle",
                    "What this capability does, where it is effective, and whether it needs attention."
                )
            )

            VStack(alignment: .leading, spacing: 8) {
                Text(UIStrings.text("skillAggregate.detail.purpose", "Purpose"))
                    .font(.headline)
                Text(presentation.purpose)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .detailCard()

            DetailMetricGrid(maxColumns: 3, minColumnWidth: 145) {
                SummaryChip(
                    title: UIStrings.text("skillAggregate.fact.installed", "Installed"),
                    value: "\(presentation.aggregate.installedInstanceCount)",
                    systemImage: "shippingbox"
                )
                SummaryChip(
                    title: UIStrings.text("skillAggregate.fact.enabled", "Enabled"),
                    value: "\(presentation.aggregate.enabledInstanceCount)",
                    systemImage: "switch.2"
                )
                SummaryChip(
                    title: UIStrings.text(
                        "skillAggregate.fact.effective",
                        "Verified effective"
                    ),
                    value: "\(presentation.aggregate.effectiveInstanceCount)",
                    systemImage: "checkmark.seal"
                )
            }

            VStack(alignment: .leading, spacing: 8) {
                Label(
                    presentation.attentionTitle,
                    systemImage: presentation.needsAttention
                        ? "exclamationmark.triangle"
                        : "checkmark.circle"
                )
                .font(.headline)
                .foregroundStyle(presentation.needsAttention ? .orange : .green)
                Text(presentation.attentionExplanation)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .detailCard()

            VStack(alignment: .leading, spacing: 8) {
                Text(
                    UIStrings.text(
                        "skillAggregate.detail.effectiveLocations",
                        "Effective for"
                    )
                )
                .font(.headline)
                Text(presentation.effectiveLocationText)
                    .foregroundStyle(
                        presentation.effectiveLocations.isEmpty ? .secondary : .primary
                    )
                    .fixedSize(horizontal: false, vertical: true)
            }
            .detailCard()

            contextualIntelligenceSection
            packageActionsSection
            configActionsSection
        }
    }

    private var evidenceLayer: some View {
        VStack(alignment: .leading, spacing: 16) {
            layerHeading(
                UIStrings.text("skillAggregate.detail.evidence", "Evidence"),
                subtitle: UIStrings.text(
                    "skillAggregate.detail.evidenceSubtitle",
                    "Complete instance-level facts from the accepted product snapshot."
                )
            )

            DetailMetricGrid(maxColumns: 4, minColumnWidth: 145) {
                SummaryChip(
                    title: UIStrings.text("skillAggregate.detail.coverage", "Coverage"),
                    value: presentation.coverageText,
                    systemImage: presentation.aggregate.coverage.isComplete
                        ? "checkmark.shield"
                        : "exclamationmark.shield"
                )
                SummaryChip(
                    title: UIStrings.text("skillAggregate.detail.findings", "Findings"),
                    value: "\(presentation.aggregate.findingCount)",
                    systemImage: "exclamationmark.triangle"
                )
                SummaryChip(
                    title: UIStrings.text("skillAggregate.detail.conflicts", "Conflicts"),
                    value: "\(presentation.aggregate.conflictCount)",
                    systemImage: "rectangle.stack.badge.exclamationmark"
                )
                SummaryChip(
                    title: UIStrings.text("skillAggregate.detail.provenance", "Provenance"),
                    value: presentation.provenanceLabel,
                    systemImage: "shippingbox"
                )
            }

            stateSummary
            instanceEvidence
            evidenceReferences
            typedActions
        }
    }

    private var advancedLayer: some View {
        VStack(alignment: .leading, spacing: 16) {
            layerHeading(
                UIStrings.text("skillAggregate.detail.advanced", "Advanced"),
                subtitle: UIStrings.text(
                    "skillAggregate.detail.advancedSubtitle",
                    "Safe logical metadata and expert diagnostics. Physical cache paths are never shown."
                )
            )

            CompactMetadataGrid(rows: presentation.advancedMetadata)
                .padding(12)
                .nativePanelSurface()

            Label(
                UIStrings.text(
                    "skillAggregate.detail.logicalSourcesOnly",
                    "Source labels are logical adapter identities, not physical cache locations."
                ),
                systemImage: "lock.shield"
            )
            .font(.callout)
            .foregroundStyle(.secondary)
        }
    }

    private var stateSummary: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(UIStrings.text("skillAggregate.detail.states", "Instance states"))
                .font(.headline)
            LazyVGrid(
                columns: [GridItem(.adaptive(minimum: 130), spacing: 7)],
                alignment: .leading,
                spacing: 7
            ) {
                ForEach(presentation.stateCounts) { item in
                    SkillAggregateStateBadge(
                        title: "\(SkillAggregateDetailPresentation.effectivenessLabel(item.state)) \(item.count)",
                        state: item.state
                    )
                }
            }
        }
        .detailCard()
    }

    private var instanceEvidence: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text(UIStrings.text("skillAggregate.detail.instances", "Instances"))
                    .font(.headline)
                DenseCountBadge(count: presentation.instances.count)
            }
            ForEach(presentation.instances) { instance in
                SkillAggregateInstanceEvidenceRow(instance: instance)
            }
        }
        .detailCard()
    }

    private var evidenceReferences: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text(
                    UIStrings.text(
                        "skillAggregate.detail.evidenceReferences",
                        "Evidence references"
                    )
                )
                .font(.headline)
                DenseCountBadge(count: presentation.evidence.count)
            }
            if presentation.evidence.isEmpty {
                Text(
                    UIStrings.text(
                        "skillAggregate.detail.noEvidence",
                        "No current evidence reference is available."
                    )
                )
                .foregroundStyle(.secondary)
            } else {
                ForEach(presentation.evidence) { evidence in
                    VStack(alignment: .leading, spacing: 4) {
                        HStack {
                            Text(evidence.reference.kind.rawValue.replacingOccurrences(
                                of: "_",
                                with: " "
                            ).capitalized)
                            .font(.callout.bold())
                            Spacer()
                            if let agent = evidence.reference.agent {
                                Text(DisplayText.agent(agent.rawValue))
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            }
                        }
                        Text(evidence.summary)
                            .font(.callout)
                        PrivacyEvidenceText(
                            value: evidence.idLabel,
                            font: .caption2,
                            lineLimit: 1
                        )
                    }
                    .padding(10)
                    .nativePanelSurface()
                }
            }
        }
        .detailCard()
    }

    private var typedActions: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text(UIStrings.text("skillAggregate.detail.typedActions", "Typed actions"))
                    .font(.headline)
                DenseCountBadge(count: presentation.typedActions.count)
            }
            if presentation.typedActions.isEmpty {
                Text(
                    UIStrings.text(
                        "skillAggregate.detail.noTypedActions",
                        "No service-owned action is available for this snapshot."
                    )
                )
                .foregroundStyle(.secondary)
            } else {
                ForEach(presentation.typedActions) { action in
                    HStack(alignment: .top, spacing: 10) {
                        Image(systemName: "eye")
                            .foregroundStyle(.secondary)
                            .frame(width: 18)
                        VStack(alignment: .leading, spacing: 3) {
                            Text(action.intentLabel)
                            .font(.callout.bold())
                            Text(
                                "\(action.descriptor.target.kind) · \(action.targetLabel) · \(action.descriptor.network) · \(action.descriptor.previewMethod)"
                            )
                            .font(.caption)
                            .foregroundStyle(.secondary)
                        }
                        Spacer()
                    }
                    .padding(10)
                    .nativePanelSurface()
                }
            }
        }
        .detailCard()
    }

    private var contextualIntelligenceSection: some View {
        actionGroup(
            title: UIStrings.text(
                "skillAggregate.actions.intelligence",
                "Contextual intelligence"
            ),
            subtitle: UIStrings.text(
                "skillAggregate.actions.intelligenceNote",
                "Opens the provider-safe review flow. This detail view never calls a model directly."
            )
        ) {
            Button {
                onContextualIntelligence?()
            } label: {
                Label(
                    UIStrings.text(
                        "skillAggregate.actions.explain",
                        "Review with contextual intelligence"
                    ),
                    systemImage: "sparkles"
                )
            }
            .buttonStyle(.borderedProminent)
            .disabled(onContextualIntelligence == nil)
        }
    }

    private var packageActionsSection: some View {
        actionGroup(
            title: UIStrings.text("skillAggregate.actions.package", "Package ownership"),
            subtitle: UIStrings.text(
                "skillAggregate.actions.packageNote",
                "Add, update, and remove use the guarded Skill Manager package flow."
            )
        ) {
            callbackButton(
                UIStrings.text("skillAggregate.actions.add", "Add"),
                systemImage: "plus",
                action: packageCallback(.add)
            )
            callbackButton(
                UIStrings.text("skillAggregate.actions.packageDetail", "Package details"),
                systemImage: "shippingbox",
                action: packageCallback(.detail)
            )
            callbackButton(
                UIStrings.text("skillAggregate.actions.update", "Update"),
                systemImage: "arrow.triangle.2.circlepath",
                action: packageCallback(.update)
            )
            callbackButton(
                UIStrings.text("skillAggregate.actions.remove", "Remove"),
                systemImage: "trash",
                role: .destructive,
                action: packageCallback(.remove)
            )
        }
    }

    private var configActionsSection: some View {
        actionGroup(
            title: UIStrings.text(
                "skillAggregate.actions.config",
                "Agent configuration"
            ),
            subtitle: UIStrings.text(
                "skillAggregate.actions.configNote",
                "Enablement changes agent configuration; it does not install, update, or remove a package."
            )
        ) {
            callbackButton(
                UIStrings.text("skillAggregate.actions.enable", "Enable"),
                systemImage: "checkmark.circle",
                action: configCallback(.enable)
            )
            callbackButton(
                UIStrings.text("skillAggregate.actions.disable", "Disable"),
                systemImage: "nosign",
                action: configCallback(.disable)
            )
        }
    }

    private func layerHeading(_ title: String, subtitle: String) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(.title3.bold())
            Text(subtitle)
                .font(.callout)
                .foregroundStyle(.secondary)
        }
    }

    private func actionGroup<Content: View>(
        title: String,
        subtitle: String,
        @ViewBuilder content: () -> Content
    ) -> some View {
        VStack(alignment: .leading, spacing: 9) {
            Text(title)
                .font(.headline)
            Text(subtitle)
                .font(.caption)
                .foregroundStyle(.secondary)
            LazyVGrid(
                columns: [GridItem(.adaptive(minimum: 140), spacing: 8)],
                alignment: .leading,
                spacing: 8
            ) {
                content()
            }
        }
        .detailCard()
    }

    private func callbackButton(
        _ title: String,
        systemImage: String,
        role: ButtonRole? = nil,
        action: (() -> Void)?
    ) -> some View {
        Button(role: role) {
            action?()
        } label: {
            Label(title, systemImage: systemImage)
        }
        .buttonStyle(.bordered)
        .disabled(action == nil)
    }

    private func packageCallback(
        _ action: SkillAggregatePackageAction
    ) -> (() -> Void)? {
        guard availablePackageActions.contains(action),
              let onPackageAction else {
            return nil
        }
        return { onPackageAction(action) }
    }

    private func configCallback(
        _ action: SkillAggregateConfigAction
    ) -> (() -> Void)? {
        guard availableConfigActions.contains(action),
              let onConfigAction else {
            return nil
        }
        return { onConfigAction(action) }
    }
}

private struct SkillAggregateInstanceEvidenceRow: View {
    let instance: SkillAggregateDetailPresentation.Instance

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            HStack {
                Text(instance.locationText)
                    .font(.callout.bold())
                Spacer()
                SkillAggregateStateBadge(
                    title: instance.effectivenessText,
                    state: instance.record.state
                )
            }

            HStack(spacing: 12) {
                Label(
                    instance.record.installed
                        ? UIStrings.text("skillAggregate.fact.installed", "Installed")
                        : UIStrings.text("skillAggregate.fact.notInstalled", "Not installed"),
                    systemImage: instance.record.installed ? "shippingbox.fill" : "shippingbox"
                )
                Label(
                    instance.record.enabled
                        ? UIStrings.text("skillAggregate.fact.enabled", "Enabled")
                        : UIStrings.text("skillAggregate.fact.notEnabled", "Not enabled"),
                    systemImage: instance.record.enabled ? "checkmark.circle.fill" : "circle"
                )
                Label(
                    instance.record.precedenceProven
                        ? UIStrings.text(
                            "skillAggregate.fact.precedenceProven",
                            "Precedence verified"
                        )
                        : UIStrings.text(
                            "skillAggregate.fact.precedenceUnknown",
                            "Precedence unavailable"
                        ),
                    systemImage: instance.record.precedenceProven
                        ? "arrow.up.circle.fill"
                        : "questionmark.circle"
                )
            }
            .font(.caption)
            .foregroundStyle(.secondary)

            Text(instance.coverageText)
                .font(.caption)
                .foregroundStyle(.secondary)

            if !instance.evidenceRefLabels.isEmpty {
                PrivacyEvidenceText(
                    value: instance.evidenceRefLabels.joined(separator: ", "),
                    font: .caption2,
                    lineLimit: 2
                )
            }
            if !instance.actionLabels.isEmpty {
                Text(
                    String(
                        format: UIStrings.text(
                            "skillAggregate.detail.instanceActions",
                            "Actions: %@"
                        ),
                        instance.actionLabels.joined(separator: ", ")
                    )
                )
                .font(.caption2)
                .foregroundStyle(.secondary)
            }
        }
        .padding(10)
        .nativePanelSurface()
    }
}

private struct SkillAggregateStateBadge: View {
    let title: String
    let state: SkillEffectivenessState

    var body: some View {
        Text(title)
            .font(.caption.bold())
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .foregroundStyle(color)
            .background(color.opacity(0.12), in: Capsule())
    }

    private var color: Color {
        switch state {
        case .effective: .green
        case .disabled: .secondary
        case .shadowed, .installedUnlinked: .orange
        case .broken: .red
        case .unavailable: .purple
        }
    }
}

private struct SkillAggregateDetailCardModifier: ViewModifier {
    func body(content: Content) -> some View {
        content
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .nativePanelSurface()
    }
}

private extension View {
    func detailCard() -> some View {
        modifier(SkillAggregateDetailCardModifier())
    }
}
