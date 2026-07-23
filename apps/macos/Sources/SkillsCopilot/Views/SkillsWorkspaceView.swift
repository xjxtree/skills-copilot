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
    private let presentation: SkillAggregateDetailPresentation

    init(aggregate: SkillAggregateRecord) {
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
                VStack(alignment: .leading, spacing: 16) {
                    reviewCard(
                        title: UIStrings.text(
                            "skillAggregate.intelligence.localAnswer",
                            "Evidence-bound context"
                        ),
                        text: presentation.purpose
                    )
                    reviewCard(
                        title: UIStrings.text(
                            "skillAggregate.intelligence.currentState",
                            "Current verified state"
                        ),
                        text: presentation.attentionExplanation
                    )
                    reviewCard(
                        title: UIStrings.text(
                            "skillAggregate.intelligence.providerBoundary",
                            "Provider boundary"
                        ),
                        text: UIStrings.text(
                            "skillAggregate.intelligence.providerBoundaryMessage",
                            "No model request has been made. A future provider flow must preview the prompt, redact sensitive content, show the destination, and require explicit confirmation."
                        )
                    )
                }
                .padding(18)
            }
        }
        .frame(minWidth: 620, idealWidth: 700, minHeight: 440)
        .accessibilityIdentifier("skills.workspace.contextual-intelligence")
    }

    private func reviewCard(title: String, text: String) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.headline)
            Text(text)
                .fixedSize(horizontal: false, vertical: true)
                .textSelection(.enabled)
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}
