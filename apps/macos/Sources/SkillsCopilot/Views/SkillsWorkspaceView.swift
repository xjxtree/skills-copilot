import SwiftUI

private struct SkillManagerSheetSelection: Identifiable {
    let id = UUID()
    let entryContext: SkillManagerEntryContext
}

private struct SkillIntelligenceSheetSelection: Identifiable {
    let aggregate: SkillAggregateRecord

    var id: String { aggregate.id }
}

struct SkillsWorkspaceView: View {
    @EnvironmentObject private var store: SkillStore
    @State private var managerSelection: SkillManagerSheetSelection?
    @State private var intelligenceSelection: SkillIntelligenceSheetSelection?

    var body: some View {
        HSplitView {
            SkillsWorkspaceListView(
                workspace: store.skillWorkspaceStore,
                onAdd: openAddFlow,
                onAgentFilterChange: updateAgentFilter,
                onRefresh: refreshWorkspace
            )
            .frame(
                minWidth: 360,
                idealWidth: 440,
                maxWidth: 560,
                maxHeight: .infinity
            )

            detailContent
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .sheet(item: $managerSelection) { selection in
            SkillPackageManagerSheet(entryContext: selection.entryContext)
                .environmentObject(store)
        }
        .sheet(item: $intelligenceSelection) { selection in
            SkillContextualIntelligenceSheet(aggregate: selection.aggregate)
                .environmentObject(store)
        }
        .accessibilityIdentifier("skills.workspace")
    }

    @ViewBuilder
    private var detailContent: some View {
        if let aggregate = store.skillWorkspaceStore.selectedAggregate {
            SkillAggregateDetailView(
                aggregate: aggregate,
                availablePackageActions: availablePackageActions(for: aggregate),
                availableConfigActions: [],
                onPackageAction: { action in
                    openPackageFlow(action, aggregate: aggregate)
                },
                onConfigAction: nil,
                onContextualIntelligence: {
                    intelligenceSelection = SkillIntelligenceSheetSelection(
                        aggregate: aggregate
                    )
                }
            )
            .id(aggregate.id)
        } else {
            SkillsWorkspaceEmptyDetailView(
                isLoading: store.skillWorkspaceStore.isLoading,
                hasVisibleRows: !store.skillWorkspaceStore.visibleAggregates.isEmpty
            )
        }
    }

    private func openAddFlow() {
        managerSelection = SkillManagerSheetSelection(
            entryContext: .add(
                scope: preferredAddScope,
                agentIDs: selectedManagerAgentIDs
            )
        )
    }

    private func openPackageFlow(
        _ action: SkillAggregatePackageAction,
        aggregate: SkillAggregateRecord
    ) {
        if action == .add {
            managerSelection = SkillManagerSheetSelection(
                entryContext: .add(
                    query: aggregate.canonicalName,
                    scope: preferredAddScope,
                    agentIDs: selectedManagerAgentIDs
                )
            )
            return
        }

        guard let match = uniqueInventoryMatch(for: aggregate) else { return }
        let target = SkillManagerPackageTarget(
            inventoryItemID: match.id,
            name: aggregate.canonicalName,
            instanceIDs: aggregate.instanceIDs,
            scope: match.scope
        )
        let context: SkillManagerEntryContext
        switch action {
        case .detail:
            context = .packageDetail(target: target, scope: match.scope)
        case .update:
            context = .update(target: target, scope: match.scope)
        case .remove:
            context = .remove(
                target: target,
                scope: match.scope,
                agentIDs: match.agents
            )
        case .add:
            return
        }
        managerSelection = SkillManagerSheetSelection(entryContext: context)
    }

    private func availablePackageActions(
        for aggregate: SkillAggregateRecord
    ) -> Set<SkillAggregatePackageAction> {
        var result: Set<SkillAggregatePackageAction> = [.add]
        guard let item = uniqueInventoryMatch(for: aggregate) else {
            return result
        }
        result.insert(.detail)
        let managerActions = SkillManagerInventoryActionPolicy.availableActions(for: item)
        if managerActions.contains(.update) {
            result.insert(.update)
        }
        if managerActions.contains(.remove) || managerActions.contains(.deleteSource) {
            result.insert(.remove)
        }
        return result
    }

    private func uniqueInventoryMatch(
        for aggregate: SkillAggregateRecord
    ) -> SkillManagerInventoryItem? {
        SkillManagerPackageTarget(
            aggregate: aggregate,
            preferredScope: preferredPackageScope
        )
        .uniqueBestMatch(in: cachedInventoryItems)
    }

    private var cachedInventoryItems: [SkillManagerInventoryItem] {
        SkillManagerScope.allCases.flatMap { scope in
            SkillManagerInventoryBuilder.build(
                installed: store.skillManagerInstalledByScope[scope]?.installed ?? [],
                catalogSkills: store.skills,
                localLibrarySkills: store.localSkillLibrarySkills,
                scope: scope
            )
        }
    }

    private var preferredPackageScope: SkillManagerScope? {
        switch store.skillWorkspaceStore.view {
        case .project:
            return .project
        case .global:
            return .global
        case .needsAttention, .all:
            return nil
        }
    }

    private var preferredAddScope: SkillManagerScope {
        preferredPackageScope ?? .project
    }

    private var selectedManagerAgentIDs: [String]? {
        guard store.agentFilter != .all,
              let productAgent = ProductAgentID(rawValue: store.agentFilter.rawValue) else {
            return nil
        }
        return SkillManagerEntryContext.managerAgentIDs(for: [productAgent])
    }

    private func updateAgentFilter(_ filter: SkillAgentFilter) {
        store.agentFilter = filter
    }

    private func refreshWorkspace() {
        Task { await store.refreshProductWorkspaces() }
    }
}

private struct SkillsWorkspaceEmptyDetailView: View {
    let isLoading: Bool
    let hasVisibleRows: Bool

    var body: some View {
        VStack(spacing: 14) {
            if isLoading {
                ProgressView()
                    .controlSize(.large)
            } else {
                Image(systemName: hasVisibleRows ? "cursorarrow.click.2" : "square.stack.3d.up")
                    .font(.system(size: 34))
                    .foregroundStyle(.secondary)
            }
            Text(title)
                .font(.title3.bold())
            Text(message)
                .font(.callout)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
                .frame(maxWidth: 420)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding(32)
        .accessibilityIdentifier("skills.workspace.empty-detail")
    }

    private var title: String {
        if isLoading {
            return UIStrings.text(
                "skills.workspace.detail.loading",
                "Loading capability evidence"
            )
        }
        return hasVisibleRows
            ? UIStrings.text("skills.workspace.detail.select", "Select a skill")
            : UIStrings.text("skills.workspace.detail.empty", "No skill to inspect")
    }

    private var message: String {
        if isLoading {
            return UIStrings.text(
                "skills.workspace.detail.loadingMessage",
                "The accepted project snapshot is being assembled."
            )
        }
        return hasVisibleRows
            ? UIStrings.text(
                "skills.workspace.detail.selectMessage",
                "Selection is explicit. Choose an aggregate to inspect its answer, evidence, and advanced details."
            )
            : UIStrings.text(
                "skills.workspace.detail.emptyMessage",
                "Change the current view or filters, or refresh the accepted project snapshot."
            )
    }
}

private struct SkillContextualIntelligenceSheet: View {
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject private var store: SkillStore
    private let presentation: SkillAggregateDetailPresentation
    private let aggregate: SkillAggregateRecord
    @State private var evidenceSelection: ContextualEvidenceSelection?

    init(aggregate: SkillAggregateRecord) {
        self.aggregate = aggregate
        presentation = SkillAggregateDetailPresentation(aggregate: aggregate)
    }

    var body: some View {
        VStack(spacing: 0) {
            HStack(alignment: .top, spacing: 14) {
                Image(systemName: "sparkles")
                    .font(.title2)
                    .foregroundStyle(.secondary)
                VStack(alignment: .leading, spacing: 4) {
                    Text(
                        UIStrings.text(
                            "skillAggregate.intelligence.title",
                            "Contextual intelligence"
                        )
                    )
                    .font(.title2.bold())
                    Text(presentation.displayName)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Button(UIStrings.done) { dismiss() }
                    .keyboardShortcut(.defaultAction)
            }
            .padding(18)

            Divider()

            ScrollView {
                ContextualIntelligenceView(
                    kind: .skillChangeReview,
                    deterministicTitle: presentation.purpose,
                    deterministicFacts: [
                        ContextualIntelligenceFact(
                            label: UIStrings.text(
                                "skillAggregate.intelligence.currentState",
                                "Current verified state"
                            ),
                            value: presentation.attentionExplanation
                        ),
                        ContextualIntelligenceFact(
                            label: UIStrings.text(
                                "skillAggregate.instances.effective",
                                "Effective"
                            ),
                            value: "\(aggregate.effectiveInstanceCount)/\(aggregate.installedInstanceCount)"
                        ),
                        ContextualIntelligenceFact(
                            label: UIStrings.text(
                                "skillAggregate.answer.attention",
                                "Attention"
                            ),
                            value: "\(aggregate.findingCount + aggregate.conflictCount)"
                        ),
                    ],
                    flow: store.contextualIntelligenceStore.flow(for: flowKey),
                    currentSourceRevision: productSourceRevision,
                    providerGateMessage: providerGateMessage,
                    onPreview: {
                        guard let revision = productSourceRevision else { return }
                        Task {
                            await store.contextualIntelligenceStore.previewSkillReview(
                                aggregate: aggregate,
                                productSourceRevision: revision
                            )
                        }
                    },
                    onConfirm: {
                        guard let revision = productSourceRevision else { return }
                        Task {
                            await store.contextualIntelligenceStore.sendSkillReview(
                                aggregate: aggregate,
                                productSourceRevision: revision
                            )
                        }
                    },
                    onDismissPreview: {
                        store.contextualIntelligenceStore.clear(flowKey)
                    },
                    onOpenEvidence: {
                        evidenceSelection = ContextualEvidenceSelection(reference: $0)
                    }
                )
                .padding(18)
            }
        }
        .frame(minWidth: 620, idealWidth: 700, minHeight: 440)
        .sheet(item: $evidenceSelection) { selection in
            ContextualEvidenceSheet(selection: selection)
        }
        .accessibilityIdentifier("skills.workspace.contextual-intelligence")
    }

    private var flowKey: String {
        ContextualIntelligenceStore.skillKey(aggregate.id)
    }

    private var productSourceRevision: String? {
        store.appContextStore.visibleProjectReadiness?.sourceRevision
    }

    private var providerGateMessage: String? {
        guard productSourceRevision != nil else {
            return UIStrings.text(
                "intelligence.snapshotRequired",
                "Refresh the project before requesting interpretation."
            )
        }
        let status = store.aiProviderStatus
        if !status.serviceAvailable {
            return UIStrings.localizedServiceMessage(
                status.disabledReason ?? UIStrings.aiProviderUnavailable
            )
        }
        if !status.configured || status.activeProfile == nil {
            return UIStrings.text(
                "intelligence.providerRequired",
                "Configure an AI provider to request optional interpretation."
            )
        }
        return status.enabled
            ? nil
            : status.disabledReason.map(UIStrings.localizedServiceMessage)
                ?? UIStrings.text(
                    "intelligence.providerDisabled",
                    "The configured AI provider is disabled."
                )
    }
}
