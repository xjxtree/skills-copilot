import SwiftUI

struct BatchSkillOperationSheet: View {
    @EnvironmentObject private var store: SkillStore
    @Environment(\.dismiss) private var dismiss
    @State private var showAffected = true
    @State private var showSkipped = false
    @State private var pendingApplyPreview: BatchTogglePreview?
    @State private var showApplyConfirmation = false

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            header
            scopeBar
            selectionControls
            skillPicker
            actionBar
            previewContent
        }
        .padding(18)
        .frame(width: 620, alignment: .topLeading)
        .frame(minHeight: 560, alignment: .topLeading)
        .alert(
            pendingApplyPreview.map { UIStrings.batchToggleConfirmTitle(action: $0.action.title, count: $0.writableCount) }
                ?? UIStrings.batchToggleConfirmTitle(action: store.batchToggleAction.title, count: 0),
            isPresented: $showApplyConfirmation,
            presenting: pendingApplyPreview
        ) { preview in
            Button(UIStrings.cancel, role: .cancel) {
                pendingApplyPreview = nil
            }
            Button(UIStrings.batchToggleConfirmApply(action: preview.action.title, count: preview.writableCount), role: .destructive) {
                let previewID = preview.id
                pendingApplyPreview = nil
                Task { await store.applyVisibleBatchTogglePreview(confirmingPreviewID: previewID) }
            }
        } message: { preview in
            let operationSummary = UIStrings.batchToggleConfirmMessage(
                action: preview.action.title.lowercased(),
                affected: preview.writableCount,
                skipped: preview.skippedCount,
                snapshot: preview.snapshotPlan.summary
            )
            let signedDisclosure = preview.actionDescriptor?
                .confirmationSummary
                .disclosureText
            Text(
                [operationSummary, signedDisclosure]
                    .compactMap { $0 }
                    .joined(separator: "\n\n")
            )
        }
    }

    private var header: some View {
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            Label(UIStrings.batchToggleSheetTitle, systemImage: "checklist.checked")
                .font(.title3.bold())
            Spacer()
            Button(UIStrings.done) {
                dismiss()
            }
            .keyboardShortcut(.cancelAction)
        }
    }

    private var scopeBar: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(UIStrings.batchToggleSheetSubtitle)
                .font(.callout)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            Text(UIStrings.batchToggleScopeSummary(
                agent: store.agentFilter.title,
                visible: store.filteredSkills.count,
                selected: store.batchToggleSelectedSkills.count
            ))
            .font(.caption.bold())
            .foregroundStyle(.secondary)
        }
    }

    private var selectionControls: some View {
        HStack(spacing: 10) {
            Picker(UIStrings.batchToggleTarget, selection: $store.batchToggleAction) {
                ForEach(BatchToggleAction.allCases) { action in
                    Label(action.title, systemImage: action.systemImage).tag(action)
                }
            }
            .pickerStyle(.segmented)
            .frame(width: 220)

            Spacer()

            Button(UIStrings.batchToggleSelectAll) {
                store.selectAllVisibleBatchToggleSkills()
            }
            .disabled(store.filteredSkills.isEmpty || store.batchToggleAllVisibleSkillsSelected)

            Button(UIStrings.batchToggleClearSelection) {
                store.clearBatchToggleSelection()
            }
            .disabled(store.batchToggleSelectedSkills.isEmpty)
        }
    }

    private var skillPicker: some View {
        ScrollView {
            LazyVStack(alignment: .leading, spacing: 0) {
                ForEach(store.filteredSkills) { skill in
                    BatchSkillSelectionRow(skill: skill)
                        .environmentObject(store)
                    if skill.id != store.filteredSkills.last?.id {
                        Divider()
                    }
                }
            }
        }
        .frame(minHeight: 190, maxHeight: 240)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
        .overlay {
            if store.filteredSkills.isEmpty {
                Text(UIStrings.noSkillsMatchSearch)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .padding()
            }
        }
    }

    private var actionBar: some View {
        HStack(spacing: 10) {
            Button {
                Task { await store.previewVisibleBatchToggle() }
            } label: {
                Label(UIStrings.preview, systemImage: "eye")
                    .frame(minWidth: 130)
            }
            .keyboardShortcut(.defaultAction)
            .disabled(store.isRefreshBusy || store.isPreviewingBatchToggle || store.batchToggleSelectedSkills.isEmpty)

            Button {
                pendingApplyPreview = store.batchTogglePreview
                showApplyConfirmation = true
            } label: {
                Label(UIStrings.batchToggleApply, systemImage: "checkmark.circle")
                    .frame(minWidth: 130)
            }
            .disabled(!store.canApplyBatchTogglePreview || store.isPreviewingBatchToggle)
            .help(store.batchTogglePreview?.applySupported == false ? UIStrings.batchToggleApplyUnavailable : "")
            .keyboardShortcut("a", modifiers: [.command])

            if store.isPreviewingBatchToggle {
                Label(UIStrings.batchTogglePreviewing, systemImage: "hourglass")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            Spacer()
        }
    }

    @ViewBuilder
    private var previewContent: some View {
        if let preview = store.batchTogglePreview {
            BatchTogglePreviewSummary(
                preview: preview,
                showAffected: $showAffected,
                showSkipped: $showSkipped
            )
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(.blue.opacity(0.07), in: RoundedRectangle(cornerRadius: 10))
        } else if store.batchToggleSelectedSkills.isEmpty {
            Label(UIStrings.batchToggleNoSelection, systemImage: "checkmark.square")
                .font(.caption)
                .foregroundStyle(.secondary)
                .padding(10)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
        } else {
            Text(UIStrings.batchToggleBoundary)
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
                .padding(10)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
        }
    }
}

private struct BatchSkillSelectionRow: View {
    @EnvironmentObject private var store: SkillStore
    let skill: SkillRecord

    var body: some View {
        Toggle(isOn: selectionBinding) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Image(systemName: DisplayText.isReadOnlyPreview(skill) ? "lock.fill" : DisplayText.stateSystemImage(skill.state, enabled: skill.enabled))
                    .foregroundStyle(DisplayText.isReadOnlyPreview(skill) ? .secondary : DisplayText.stateColor(skill.state, enabled: skill.enabled))
                    .frame(width: 18)

                VStack(alignment: .leading, spacing: 2) {
                    Text(skill.name)
                        .font(.callout)
                        .lineLimit(1)
                    Text(secondaryText)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
                Spacer(minLength: 8)
            }
        }
        .toggleStyle(.checkbox)
        .padding(.horizontal, 10)
        .padding(.vertical, 7)
    }

    private var selectionBinding: Binding<Bool> {
        Binding {
            store.isBatchToggleSkillSelected(skill)
        } set: { isSelected in
            store.setBatchToggleSkill(skill, selected: isSelected)
        }
    }

    private var secondaryText: String {
        if DisplayText.isToolGlobal(skill) {
            return "\(DisplayText.scope(for: skill)) · \(UIStrings.readOnlyPreview)"
        }
        if skill.agent == "hermes", DisplayText.isReadOnlyPreview(skill) {
            return "\(DisplayText.scope(for: skill)) · \(skill.provenance.label)"
        }
        if DisplayText.isReadOnlyPreview(skill) {
            return "\(DisplayText.scope(for: skill)) · \(UIStrings.readOnly)"
        }
        return "\(DisplayText.scope(for: skill)) · \(DisplayText.state(skill.state, enabled: skill.enabled))"
    }
}

private struct BatchTogglePreviewSummary: View {
    let preview: BatchTogglePreview
    @Binding var showAffected: Bool
    @Binding var showSkipped: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 9) {
            HStack(spacing: 6) {
                BatchToggleCountPill(title: UIStrings.batchToggleSelected, value: preview.selectedCount)
                BatchToggleCountPill(title: UIStrings.batchToggleWritable, value: preview.writableCount)
                BatchToggleCountPill(title: UIStrings.batchToggleSkipped, value: preview.skippedCount)
            }

            Label(UIStrings.batchToggleActionTarget(preview.action.title), systemImage: preview.action.systemImage)
                .font(.caption.bold())
                .foregroundStyle(.primary)

            VStack(alignment: .leading, spacing: 4) {
                Label(UIStrings.batchToggleSnapshotPlan, systemImage: "clock.arrow.circlepath")
                    .font(.caption.bold())
                Text(preview.snapshotPlan.summary)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(3)
                if !preview.snapshotPlan.targets.isEmpty {
                    ExpandableSummaryList(
                        snapshotTargetRows,
                        visibleLimit: 2,
                        spacing: 2,
                        accessibilityIdentifier: "batch-toggle-snapshot-targets.show-all"
                    ) { row in
                        Text(row.value)
                            .font(.caption2)
                            .foregroundStyle(.tertiary)
                            .lineLimit(1)
                    }
                }
            }
            .padding(8)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))

            DisclosureGroup(isExpanded: $showAffected) {
                BatchToggleItemList(
                    items: preview.affectedSkills,
                    emptyMessage: UIStrings.batchToggleNoAffectedSkills,
                    showAllAccessibilityIdentifier: "batch-toggle-items.show-all"
                )
            } label: {
                Text(UIStrings.batchToggleAffectedSkills(preview.affectedSkills.count))
                    .font(.caption.bold())
            }

            DisclosureGroup(isExpanded: $showSkipped) {
                BatchToggleItemList(
                    items: preview.skippedItems,
                    emptyMessage: UIStrings.batchToggleNoSkippedSkills,
                    showAllAccessibilityIdentifier: "batch-toggle-skipped-items.show-all"
                )
            } label: {
                Text(UIStrings.batchToggleSkippedSkills(preview.skippedItems.count))
                    .font(.caption.bold())
            }

            if !preview.applySupported {
                Label(UIStrings.batchToggleApplyUnavailable, systemImage: "lock.fill")
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
            }
        }
    }

    private var snapshotTargetRows: [BatchToggleSnapshotTargetRow] {
        preview.snapshotPlan.targets.enumerated().map { index, value in
            BatchToggleSnapshotTargetRow(id: index, value: value)
        }
    }
}

private struct BatchToggleSnapshotTargetRow: Identifiable {
    let id: Int
    let value: String
}

private struct BatchToggleCountPill: View {
    let title: String
    let value: Int

    var body: some View {
        VStack(alignment: .leading, spacing: 1) {
            Text(title)
                .foregroundStyle(.secondary)
            Text("\(value)")
                .fontWeight(.bold)
                .monospacedDigit()
        }
        .font(.caption2)
        .padding(.horizontal, 8)
        .padding(.vertical, 5)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
    }
}

private struct BatchToggleItemList: View {
    let items: [BatchToggleSkillItem]
    let emptyMessage: String
    let showAllAccessibilityIdentifier: String

    var body: some View {
        if items.isEmpty {
            Text(emptyMessage)
                .font(.caption2)
                .foregroundStyle(.secondary)
                .padding(.vertical, 3)
        } else {
            ExpandableSummaryList(
                items,
                visibleLimit: 8,
                spacing: 5,
                accessibilityIdentifier: showAllAccessibilityIdentifier
            ) { item in
                VStack(alignment: .leading, spacing: 1) {
                    Text(item.name)
                        .font(.caption.bold())
                        .lineLimit(1)
                    Text(itemSubtitle(item))
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        .lineLimit(2)
                }
                .padding(.vertical, 2)
            }
        }
    }

    private func itemSubtitle(_ item: BatchToggleSkillItem) -> String {
        let base = [DisplayText.agent(item.agent), DisplayText.scope(item.scope, agent: item.agent)].filter { !$0.isEmpty }.joined(separator: " · ")
        guard let reason = item.reason, !reason.isEmpty else { return base }
        return base.isEmpty ? reason : "\(base) · \(reason)"
    }
}
