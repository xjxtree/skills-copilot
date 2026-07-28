import AppKit
import Foundation
import SwiftUI

struct TaskPreflightPreviewSheet: View {
    @EnvironmentObject private var store: SkillStore

    var body: some View {
        WorkflowSheetShell(
            title: UIStrings.taskCockpitTitle,
            systemImage: "checklist",
            subtitle: UIStrings.text(
                "taskCockpit.sheet.subtitle",
                "Scope agents, preview the redacted request, then confirm sending."
            ),
            content: {
                WorkflowSheetSplitLayout(
                    secondaryWidth: CGFloat(UIOptimizationPresentation.taskPreflight.historyColumnWidth)
                ) {
                    ScrollView {
                        TaskPreflightEditorPane(historySelectionID: store.selectedTaskCockpitHistoryID)
                            .padding(.trailing, 14)
                    }
                } secondary: {
                    TaskPreflightHistoryPanel(
                        records: store.taskCockpitHistory,
                        selectedID: store.selectedTaskCockpitHistoryID,
                        cleanupMessage: store.taskCockpitHistoryCleanupMessage,
                        onSelect: { record in
                            store.selectTaskCockpitHistoryRecord(record)
                        },
                        onClear: {
                            store.clearTaskCockpitHistory()
                        }
                    )
                }
            }
        )
        .frame(
            minWidth: CGFloat(UIOptimizationPresentation.taskPreflight.sheetMinimumWidth),
            idealWidth: CGFloat(UIOptimizationPresentation.taskPreflight.sheetIdealWidth),
            minHeight: CGFloat(UIOptimizationPresentation.taskPreflight.sheetMinimumHeight),
            alignment: .topLeading
        )
    }
}

private struct TaskPreflightEditorPane: View {
    @EnvironmentObject private var store: SkillStore
    @EnvironmentObject private var providerStore: ProviderStore
    @Environment(\.dismiss) private var dismiss
    @State private var draftTaskText = ""
    @State private var hasLoadedInitialDraft = false
    let historySelectionID: TaskCockpitHistoryRecord.ID?

    var body: some View {
        TaskCockpitPanel(
            taskText: $draftTaskText,
            currentTaskText: effectiveTaskText,
            agentOptions: store.taskCockpitAgentOptions,
            selectedAgentIDs: store.taskCockpitSelectedAgentIDs,
            promptConfirmation: displayedPromptConfirmation,
            isPreviewingPrompt: displayedIsPreviewingPrompt,
            result: displayedResult,
            failedProviderOutput: isDraftSyncedWithStore ? store.taskCockpitFailedProviderOutput : nil,
            isBuilding: displayedIsBuilding,
            operationState: displayedOperationState,
            providerGateMessage: providerGateMessage,
            onToggleAgent: { agentID in
                store.toggleTaskCockpitAgentSelection(agentID)
            },
            onSelectAllAgents: {
                store.selectAllTaskCockpitAgents()
            },
            onBuild: {
                Task {
                    syncDraftToStore()
                    await store.buildTaskCockpit()
                }
            },
            onConfirmPrompt: {
                Task {
                    await store.confirmTaskCockpitPromptAndBuild()
                }
            },
            onDismissPrompt: {
                store.clearTaskCockpitPromptConfirmation()
            },
            onCancel: {
                store.cancelTaskCockpitBuild()
            },
            onOpenRecommendation: { recommendation in
                openRecommendation(recommendation)
            }
        )
        .onAppear {
            guard !hasLoadedInitialDraft else { return }
            store.ensureTaskCockpitAgentSelection()
            draftTaskText = store.taskCockpitText
            hasLoadedInitialDraft = true
        }
        .onDisappear {
            syncDraftToStore()
        }
        .onChange(of: historySelectionID) { _ in
            draftTaskText = store.taskCockpitText
        }
    }

    private var effectiveTaskText: String {
        let trimmedDraft = draftTaskText.trimmingCharacters(in: .whitespacesAndNewlines)
        if !trimmedDraft.isEmpty {
            return draftTaskText
        }
        return store.selectedTaskCockpitInput
    }

    private var isDraftSyncedWithStore: Bool {
        draftTaskText == store.taskCockpitText
    }

    private var displayedResult: TaskCockpitResult? {
        isDraftSyncedWithStore ? store.taskCockpitResult : nil
    }

    private var displayedPromptConfirmation: TaskCockpitPromptConfirmation? {
        isDraftSyncedWithStore ? store.taskCockpitPromptConfirmation : nil
    }

    private var displayedIsPreviewingPrompt: Bool {
        isDraftSyncedWithStore && store.isPreviewingTaskCockpitPrompt
    }

    private var displayedIsBuilding: Bool {
        isDraftSyncedWithStore && store.isBuildingTaskCockpit
    }

    private var displayedOperationState: TaskCockpitOperationState {
        isDraftSyncedWithStore ? store.taskCockpitOperationState : .idle
    }

    private func syncDraftToStore() {
        guard store.taskCockpitText != draftTaskText else { return }
        store.taskCockpitText = draftTaskText
    }

    private var providerGateMessage: String? {
        let status = providerStore.aiProviderStatus
        if !status.serviceAvailable {
            return UIStrings.localizedServiceMessage(status.disabledReason ?? UIStrings.aiProviderUnavailable)
        }
        if !status.configured || status.activeProfile == nil {
            return UIStrings.text("taskCockpit.providerRequired", "Configure an AI provider before generating Task Preflight.")
        }
        if !status.enabled {
            return status.disabledReason.map(UIStrings.localizedServiceMessage)
                ?? UIStrings.text("taskCockpit.providerDisabled", "The configured AI provider is disabled.")
        }
        return nil
    }

    private func openRecommendation(_ recommendation: TaskCockpitRecommendation) {
        if store.navigateToSkill(
            instanceID: recommendation.instanceID,
            name: recommendation.skillName,
            agent: recommendation.agent
        ) {
            dismiss()
            return
        }

        guard let skillName = recommendation.skillName else { return }
        dismiss()
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) {
            store.presentSkillManager(searchQuery: skillName)
        }
    }
}

private struct TaskPreflightHistoryPanel: View {
    @State private var isConfirmingClear = false
    let records: [TaskCockpitHistoryRecord]
    let selectedID: TaskCockpitHistoryRecord.ID?
    let cleanupMessage: String?
    let onSelect: (TaskCockpitHistoryRecord) -> Void
    let onClear: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 8) {
                Label(UIStrings.text("taskCockpit.history.title", "History"), systemImage: "clock.arrow.circlepath")
                    .font(.headline)

                Spacer(minLength: 8)

                Button(role: .destructive) {
                    isConfirmingClear = true
                } label: {
                    Label(UIStrings.taskCockpitHistoryClear, systemImage: "trash")
                }
                .buttonStyle(.bordered)
                .controlSize(.small)
                .disabled(records.isEmpty && cleanupMessage == nil)
            }

            Text(UIStrings.taskCockpitHistorySummary)
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)

            if let cleanupMessage {
                WorkflowSheetInlineBanner(message: cleanupMessage, style: .warning)
            }

            if records.isEmpty {
                EmptyState(
                    title: UIStrings.text("taskCockpit.history.emptyTitle", "No History"),
                    systemImage: "clock.badge.questionmark",
                    message: UIStrings.text("taskCockpit.history.emptyMessage", "Run a Task Preflight to keep a result for this app session.")
                )
            } else {
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 8) {
                        ForEach(records) { record in
                            TaskPreflightHistoryRow(
                                record: record,
                                isSelected: record.id == selectedID,
                                onSelect: {
                                    onSelect(record)
                                }
                            )
                        }
                    }
                }
            }
        }
        .padding(12)
        .frame(maxHeight: .infinity, alignment: .topLeading)
        .nativePanelSurface()
        .confirmationDialog(
            UIStrings.taskCockpitHistoryClearConfirmationTitle,
            isPresented: $isConfirmingClear,
            titleVisibility: .visible
        ) {
            Button(UIStrings.taskCockpitHistoryClear, role: .destructive) {
                onClear()
            }
            Button(UIStrings.cancel, role: .cancel) {
                isConfirmingClear = false
            }
        } message: {
            Text(UIStrings.taskCockpitHistoryClearConfirmationMessage)
        }
    }
}

private struct TaskPreflightHistoryRow: View {
    let record: TaskCockpitHistoryRecord
    let isSelected: Bool
    let onSelect: () -> Void

    private var model: TaskCockpitDecisionPresentationModel {
        TaskCockpitDecisionPresentationModel(result: record.result)
    }

    var body: some View {
        Button(action: onSelect) {
            VStack(alignment: .leading, spacing: 6) {
                HStack(alignment: .firstTextBaseline, spacing: 6) {
                    Image(systemName: model.verdict.systemImage)
                        .foregroundStyle(model.verdict.tint)
                    Text(record.displayTask)
                        .font(.caption.bold())
                        .foregroundStyle(.primary)
                        .lineLimit(2)
                }

                Text(Self.dateFormatter.string(from: record.createdAt))
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)

                Label(record.agentScopeSummary, systemImage: "person.2")
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)

                VStack(alignment: .leading, spacing: 2) {
                    Text(model.verdict.title)
                        .font(.caption2.weight(.semibold))
                        .foregroundStyle(model.verdict.tint)
                        .lineLimit(1)
                    Text(model.recommendationLine)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        .lineLimit(2)
                }
            }
            .padding(8)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(
                isSelected ? Color(nsColor: .selectedContentBackgroundColor).opacity(0.18) : Color.agentCopilotPanelBackground,
                in: RoundedRectangle(cornerRadius: 8)
            )
            .overlay(alignment: .leading) {
                if isSelected {
                    Rectangle()
                        .fill(Color.accentColor)
                        .frame(width: CGFloat(UIOptimizationPresentation.sidebarSelection.accentLineWidth))
                        .clipShape(Capsule())
                        .padding(.vertical, 6)
                }
            }
        }
        .buttonStyle(.plain)
        .accessibilityLabel(record.displayTask)
    }

    private static let dateFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateStyle = .short
        formatter.timeStyle = .short
        return formatter
    }()
}

struct TaskCockpitPanel: View {
    @Binding var taskText: String
    let currentTaskText: String
    let agentOptions: [TaskCockpitAgentOption]
    let selectedAgentIDs: Set<String>
    let promptConfirmation: TaskCockpitPromptConfirmation?
    let isPreviewingPrompt: Bool
    let result: TaskCockpitResult?
    let failedProviderOutput: String?
    let isBuilding: Bool
    let operationState: TaskCockpitOperationState
    let providerGateMessage: String?
    let onToggleAgent: (String) -> Void
    let onSelectAllAgents: () -> Void
    let onBuild: () -> Void
    let onConfirmPrompt: () -> Void
    let onDismissPrompt: () -> Void
    let onCancel: () -> Void
    let onOpenRecommendation: (TaskCockpitRecommendation) -> Void

    private var inputModel: TaskInputModel {
        TaskInputModel(rawText: taskText)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.taskCockpitTitle, systemImage: "checklist")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.taskCockpitBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            TaskCockpitAgentSelector(
                options: agentOptions,
                selectedAgentIDs: selectedAgentIDs,
                onToggle: onToggleAgent,
                onSelectAll: onSelectAllAgents
            )

            if let providerGateMessage {
                WorkflowSheetInlineBanner(message: providerGateMessage, style: .warning)
            }

            VStack(alignment: .trailing, spacing: 8) {
                TaskInputTextEditor(
                    text: $taskText,
                    placeholder: UIStrings.taskCockpitTaskPlaceholder
                )
                .frame(maxWidth: .infinity)

                buildButton
            }

            if let promptConfirmation {
                TaskCockpitPromptPreviewCard(
                    confirmation: promptConfirmation,
                    isSending: isBuilding,
                    onConfirm: onConfirmPrompt,
                    onDismiss: onDismissPrompt
                )
            }

            if isPreviewingPrompt || isBuilding || (result == nil && promptConfirmation == nil) {
                TaskCockpitOperationStatusView(
                    state: operationState,
                    isBuilding: isBuilding,
                    onCancel: onCancel,
                    onRetry: onBuild
                )
            }

            if let result {
                TaskCockpitResultView(
                    taskText: currentTaskText,
                    result: result,
                    operationState: operationState,
                    isBuilding: isBuilding,
                    onOpenRecommendation: onOpenRecommendation
                )
                if let failedProviderOutput {
                    TaskCockpitFailedProviderOutputButton(text: failedProviderOutput)
                }
            } else if !isBuilding {
                Label(UIStrings.taskCockpitNoResult, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier(AppAccessibilityID.taskCockpitPanel)
        .accessibilityLabel(UIStrings.taskCockpitTitle)
    }

    private var actionTitle: String {
        operationState.canRetry ? UIStrings.taskCockpitRetry : UIStrings.taskCockpitAction
    }

    private var actionSystemImage: String {
        operationState.canRetry ? "arrow.clockwise" : "checklist"
    }

    private var buildButton: some View {
        Button {
            onBuild()
        } label: {
            Label(actionTitle, systemImage: actionSystemImage)
                .frame(minWidth: 132)
        }
        .controlSize(.regular)
        .buttonStyle(.borderedProminent)
        .disabled(isPreviewingPrompt || isBuilding || !inputModel.canSubmit || selectedAgentIDs.isEmpty || providerGateMessage != nil)
        .help(providerGateMessage ?? UIStrings.taskCockpitBoundary)
        .accessibilityIdentifier(AppAccessibilityID.taskCockpitBuildButton)
        .accessibilityLabel(actionTitle)
    }
}

private struct TaskCockpitPromptPreviewCard: View {
    let confirmation: TaskCockpitPromptConfirmation
    let isSending: Bool
    let onConfirm: () -> Void
    let onDismiss: () -> Void

    private var preview: LLMPromptPreview {
        confirmation.preview
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Label(UIStrings.taskCockpitPromptPreviewTitle, systemImage: "lock.shield")
                    .font(.callout.bold())
                Spacer()
                Text(UIStrings.text("taskCockpit.promptPreview.confirmationRequired", "Confirmation required"))
                    .font(.caption.bold())
                    .foregroundStyle(.orange)
            }

            Text(UIStrings.taskCockpitPromptPreviewSummary)
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)

            LazyVGrid(columns: [GridItem(.adaptive(minimum: 150), spacing: 8)], alignment: .leading, spacing: 8) {
                promptFact(
                    title: UIStrings.text("llm.prompt.provider", "Provider"),
                    value: preview.provider ?? UIStrings.unknown,
                    systemImage: "network"
                )
                promptFact(
                    title: UIStrings.text("llm.prompt.model", "Model"),
                    value: preview.model ?? UIStrings.unknown,
                    systemImage: "cpu"
                )
                promptFact(
                    title: UIStrings.text("llm.prompt.destination", "Destination"),
                    value: preview.destinationHost ?? UIStrings.unknown,
                    systemImage: "paperplane"
                )
                promptFact(
                    title: UIStrings.text("llm.prompt.tokens", "Tokens"),
                    value: preview.estimate.map { "\($0.totalTokens)" } ?? UIStrings.unknown,
                    systemImage: "sum"
                )
            }

            if !preview.redaction.summary.isEmpty {
                Label(preview.redaction.summary, systemImage: "eye.slash")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }

            if let prompt = preview.promptPreview, !prompt.isEmpty {
                Text(prompt)
                    .font(.system(.caption, design: .monospaced))
                    .textSelection(.enabled)
                    .lineLimit(5)
                    .padding(8)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.agentCopilotWindowBackground.opacity(0.7), in: RoundedRectangle(cornerRadius: 6))
            }

            HStack {
                Button(UIStrings.cancel) {
                    onDismiss()
                }
                .disabled(isSending)

                Spacer()

                Button {
                    onConfirm()
                } label: {
                    Label(UIStrings.taskCockpitPromptConfirmSend, systemImage: "paperplane")
                }
                .buttonStyle(.borderedProminent)
                .disabled(isSending)
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
        .overlay {
            RoundedRectangle(cornerRadius: 8)
                .stroke(Color.orange.opacity(0.35), lineWidth: 1)
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(UIStrings.taskCockpitPromptPreviewTitle)
    }

    private func promptFact(title: String, value: String, systemImage: String) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 6) {
            Image(systemName: systemImage)
                .foregroundStyle(.secondary)
                .frame(width: 16)
            VStack(alignment: .leading, spacing: 1) {
                Text(title)
                    .font(.caption2.weight(.semibold))
                    .foregroundStyle(.secondary)
                Text(value)
                    .font(.caption)
                    .foregroundStyle(.primary)
                    .lineLimit(1)
                    .truncationMode(.middle)
            }
        }
    }
}

private struct TaskCockpitAgentSelector: View {
    let options: [TaskCockpitAgentOption]
    let selectedAgentIDs: Set<String>
    let onToggle: (String) -> Void
    let onSelectAll: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Label(UIStrings.text("taskCockpit.agentScope.title", "Agents"), systemImage: "person.2")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                Spacer()
                Button {
                    onSelectAll()
                } label: {
                    Label(UIStrings.text("taskCockpit.agentScope.selectAll", "Select all"), systemImage: "checkmark.circle")
                }
                .buttonStyle(.borderless)
                .font(.caption)
                .disabled(isAllSelected)
            }

            LazyVGrid(columns: agentGridColumns, alignment: .leading, spacing: 8) {
                ForEach(options) { option in
                    TaskCockpitAgentChip(
                        option: option,
                        isSelected: selectedAgentIDs.contains(option.id),
                        onToggle: {
                            onToggle(option.id)
                        }
                    )
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            if selectedAgentIDs.isEmpty {
                Label(UIStrings.text("taskCockpit.agentScope.required", "Select at least one agent."), systemImage: "exclamationmark.circle")
                    .font(.caption)
                    .foregroundStyle(.orange)
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
    }

    private var isAllSelected: Bool {
        !options.isEmpty && Set(options.map(\.id)).isSubset(of: selectedAgentIDs)
    }

    private var agentGridColumns: [GridItem] {
        Array(
            repeating: GridItem(.flexible(minimum: 150), spacing: 8, alignment: .topLeading),
            count: UIOptimizationPresentation.taskPreflight.agentGridColumnCount
        )
    }
}

private struct TaskCockpitAgentChip: View {
    let option: TaskCockpitAgentOption
    let isSelected: Bool
    let onToggle: () -> Void

    var body: some View {
        Button(action: onToggle) {
            HStack(spacing: 7) {
                Image(systemName: isSelected ? "checkmark.circle.fill" : "circle")
                    .foregroundStyle(isSelected ? Color.accentColor : Color.secondary)
                VStack(alignment: .leading, spacing: 1) {
                    Text(option.title)
                        .font(.caption.bold())
                        .lineLimit(1)
                    Text(option.subtitle)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 7)
            .frame(minHeight: 44, alignment: .leading)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(
                isSelected ? Color(nsColor: .selectedContentBackgroundColor).opacity(0.18) : Color.agentCopilotPanelBackground,
                in: RoundedRectangle(cornerRadius: 8)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 8)
                    .stroke(isSelected ? Color.accentColor.opacity(0.45) : Color.clear, lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
        .help("\(option.title) · \(option.subtitle)")
        .accessibilityLabel(option.title)
        .accessibilityValue(isSelected ? UIStrings.stateEnabled : UIStrings.stateDisabled)
    }
}

private struct TaskCockpitOperationStatusView: View {
    let state: TaskCockpitOperationState
    let isBuilding: Bool
    let onCancel: () -> Void
    let onRetry: () -> Void

    var body: some View {
        if state.phase != .idle && state.phase != .completed {
            TimelineView(.periodic(from: state.startedAt ?? Date(), by: 1)) { context in
                HStack(alignment: .top, spacing: 10) {
                    if state.phase == .preparing {
                        ProgressView()
                            .controlSize(.small)
                    }
                    Label(statusMessage(now: context.date), systemImage: systemImage)
                        .font(.callout)
                        .foregroundStyle(foregroundStyle)
                        .textSelection(.enabled)
                    Spacer(minLength: 8)
                    if state.canCancel && isBuilding {
                        Button {
                            onCancel()
                        } label: {
                            Label(UIStrings.cancel, systemImage: "xmark.circle")
                        }
                        .buttonStyle(.bordered)
                        .controlSize(.small)
                        .accessibilityIdentifier(AppAccessibilityID.taskCockpitCancelButton)
                        .accessibilityLabel(UIStrings.cancel)
                    }
                    if state.canRetry {
                        Button {
                            onRetry()
                        } label: {
                            Label(UIStrings.taskCockpitRetry, systemImage: "arrow.clockwise")
                        }
                        .buttonStyle(.bordered)
                        .controlSize(.small)
                        .accessibilityIdentifier(AppAccessibilityID.taskCockpitRetryButton)
                        .accessibilityLabel(UIStrings.taskCockpitRetry)
                    }
                }
                .padding(10)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
                .accessibilityElement(children: .contain)
                .accessibilityIdentifier(AppAccessibilityID.taskCockpitStatus)
                .accessibilityLabel(statusMessage(now: context.date))
                .overlay(alignment: .bottomLeading) {
                    if state.phase == .preparing, state.timeoutSeconds > 0 {
                        GeometryReader { proxy in
                            Rectangle()
                                .fill(.secondary.opacity(0.35))
                                .frame(width: proxy.size.width * progress(now: context.date), height: 2)
                                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomLeading)
                        }
                        .allowsHitTesting(false)
                    }
                }
            }
        }
    }

    private var systemImage: String {
        switch state.phase {
        case .idle, .completed:
            return "checkmark.circle"
        case .preparing:
            return "hourglass"
        case .fallback:
            return "exclamationmark.triangle"
        case .timedOut:
            return "clock.badge.exclamationmark"
        case .cancelled:
            return "xmark.circle"
        case .failed:
            return "exclamationmark.octagon"
        }
    }

    private var foregroundStyle: AnyShapeStyle {
        switch state.phase {
        case .timedOut, .failed:
            return AnyShapeStyle(.orange)
        case .fallback, .cancelled:
            return AnyShapeStyle(.secondary)
        case .idle, .preparing, .completed:
            return AnyShapeStyle(.secondary)
        }
    }

    private func statusMessage(now: Date) -> String {
        if state.phase == .preparing {
            return UIStrings.taskCockpitPreparingStatus(
                elapsedSeconds: state.elapsedSeconds(now: now),
                timeoutSeconds: state.timeoutSeconds
            )
        }
        if state.elapsedSeconds() > 0 {
            return "\(UIStrings.localizedServiceMessage(state.message)) \(UIStrings.taskCockpitElapsedSeconds(state.elapsedSeconds()))"
        }
        return UIStrings.localizedServiceMessage(state.message)
    }

    private func progress(now: Date) -> CGFloat {
        guard state.timeoutSeconds > 0 else { return 0 }
        return min(1, CGFloat(Double(state.elapsedSeconds(now: now)) / Double(state.timeoutSeconds)))
    }
}

private struct TaskCockpitStageProgressView: View {
    let state: TaskCockpitOperationState
    let isBuilding: Bool
    let result: TaskCockpitResult?

    var body: some View {
        if shouldRender {
            TimelineView(.periodic(from: state.startedAt ?? Date(), by: 1)) { context in
                let snapshot = TaskCockpitProgressSnapshot(
                    operationState: state,
                    result: result,
                    now: context.date
                )
                content(snapshot: snapshot)
            }
        }
    }

    private var shouldRender: Bool {
        isBuilding || result != nil || state.phase != .idle
    }

    private var blockerCount: Int {
        guard let result else { return 0 }
        return max(result.summary.blockerCount, result.blockerRows.count, result.aggregation?.blockerCodes.count ?? 0)
    }

    private func content(snapshot: TaskCockpitProgressSnapshot) -> some View {
        VStack(alignment: .leading, spacing: 9) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Label(UIStrings.taskCockpitProgressTitle, systemImage: "list.bullet.clipboard")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                Spacer(minLength: 8)
                ForEach(indicators(snapshot: snapshot)) { indicator in
                    Label(indicator.title, systemImage: indicator.systemImage)
                        .font(.caption2.bold())
                        .foregroundStyle(indicator.foregroundStyle)
                        .lineLimit(1)
                }
            }

            ProgressView(value: snapshot.estimatedProgress)
                .controlSize(.small)

            LazyVGrid(columns: [GridItem(.adaptive(minimum: 190), spacing: 8)], alignment: .leading, spacing: 8) {
                ForEach(snapshot.stageRows) { row in
                    TaskCockpitStageTile(row: row)
                }
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier(AppAccessibilityID.taskCockpitStageProgress)
        .accessibilityLabel(UIStrings.taskCockpitProgressTitle)
        .accessibilityValue(accessibilitySummary(snapshot: snapshot))
    }

    private func indicators(snapshot: TaskCockpitProgressSnapshot) -> [TaskCockpitStageIndicator] {
        var rows: [TaskCockpitStageIndicator] = []
        if state.startedAt != nil || isBuilding {
            rows.append(
                TaskCockpitStageIndicator(
                    id: "elapsed",
                    title: UIStrings.taskCockpitElapsedSeconds(snapshot.elapsedSeconds),
                    systemImage: "timer",
                    foregroundStyle: AnyShapeStyle(.secondary)
                )
            )
        }
        if hasFallbackIndicator(snapshot: snapshot) {
            rows.append(
                TaskCockpitStageIndicator(
                    id: "fallback",
                    title: UIStrings.taskCockpitProgressFallback,
                    systemImage: "exclamationmark.triangle",
                    foregroundStyle: AnyShapeStyle(.secondary)
                )
            )
        }
        if blockerCount > 0 {
            rows.append(
                TaskCockpitStageIndicator(
                    id: "blocked",
                    title: UIStrings.taskCockpitProgressBlocked(blockerCount),
                    systemImage: "exclamationmark.octagon",
                    foregroundStyle: AnyShapeStyle(.orange)
                )
            )
        }
        if state.phase == .timedOut || result?.aggregation?.timedOut == true {
            rows.append(
                TaskCockpitStageIndicator(
                    id: "timedOut",
                    title: UIStrings.taskCockpitProgressTimedOut,
                    systemImage: "clock.badge.exclamationmark",
                    foregroundStyle: AnyShapeStyle(.orange)
                )
            )
        }
        return rows
    }

    private func hasFallbackIndicator(snapshot: TaskCockpitProgressSnapshot) -> Bool {
        snapshot.stageRows.contains { row in
            row.state == .fallback || row.state == .unavailable
        } || result?.aggregation?.partial == true || result?.aggregation?.fallbackUsed == true
    }

    private func accessibilitySummary(snapshot: TaskCockpitProgressSnapshot) -> String {
        let stageSummary = snapshot.stageRows
            .map { "\($0.title): \(TaskCockpitStageTile.stateTitle($0.state))" }
            .joined(separator: ", ")
        let indicatorSummary = indicators(snapshot: snapshot)
            .map(\.title)
            .joined(separator: ", ")
        guard !indicatorSummary.isEmpty else { return stageSummary }
        return "\(indicatorSummary). \(stageSummary)"
    }
}

private struct TaskCockpitStageIndicator: Identifiable {
    let id: String
    let title: String
    let systemImage: String
    let foregroundStyle: AnyShapeStyle
}

private struct TaskCockpitStageTile: View {
    let row: TaskCockpitProgressRow

    var body: some View {
        HStack(alignment: .center, spacing: 8) {
            Image(systemName: stageSystemImage(row.stage))
                .font(.callout)
                .foregroundStyle(.secondary)
                .frame(width: 18)
            VStack(alignment: .leading, spacing: 4) {
                Text(row.title)
                    .font(.caption.bold())
                    .lineLimit(2)
                Label(Self.stateTitle(row.state), systemImage: stateSystemImage(row.state))
                    .font(.caption2)
                    .foregroundStyle(stateForegroundStyle(row.state))
                    .lineLimit(1)
            }
            Spacer(minLength: 4)
            VStack(alignment: .trailing, spacing: 2) {
                if row.count > 0 {
                    Text("\(row.count)")
                        .font(.caption.monospacedDigit().bold())
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
                if let score = row.score {
                    Text("\(score)")
                        .font(.caption2.monospacedDigit())
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
            }
        }
        .padding(.horizontal, 9)
        .padding(.vertical, 8)
        .frame(minHeight: 58, alignment: .center)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
        .accessibilityElement(children: .combine)
        .accessibilityLabel(row.title)
        .accessibilityValue(accessibilityValue)
    }

    static func stateTitle(_ state: TaskCockpitProgressState) -> String {
        switch state {
        case .idle, .queued:
            return UIStrings.taskCockpitProgressPending
        case .active:
            return UIStrings.taskCockpitProgressChecking
        case .completed:
            return UIStrings.taskCockpitProgressReady
        case .empty:
            return UIStrings.taskCockpitProgressNoRows
        case .fallback:
            return UIStrings.taskCockpitProgressPartial
        case .skipped:
            return UIStrings.taskCockpitProgressSkipped
        case .unavailable:
            return UIStrings.taskCockpitProgressUnavailable
        case .timedOut:
            return UIStrings.taskCockpitProgressTimedOut
        case .cancelled:
            return UIStrings.taskCockpitProgressCancelled
        case .failed:
            return UIStrings.taskCockpitProgressFailed
        }
    }

    private var accessibilityValue: String {
        var parts = [Self.stateTitle(row.state)]
        if row.count > 0 {
            parts.append(UIStrings.taskCockpitProgressRows(row.count))
        }
        if !row.detail.isEmpty {
            parts.append(row.detail)
        }
        return parts.joined(separator: ". ")
    }

    private func stageSystemImage(_ stage: TaskCockpitProgressStage) -> String {
        switch stage {
        case .readiness:
            return "gauge.medium"
        case .routing:
            return "point.3.connected.trianglepath.dotted"
        case .crossAgent:
            return "person.3"
        case .actionReview:
            return "wrench.and.screwdriver"
        case .batchChecks:
            return "checklist"
        case .provider:
            return "network"
        }
    }

    private func stateSystemImage(_ state: TaskCockpitProgressState) -> String {
        switch state {
        case .idle, .queued:
            return "circle"
        case .active:
            return "hourglass"
        case .completed:
            return "checkmark.circle"
        case .empty:
            return "minus.circle"
        case .fallback:
            return "exclamationmark.triangle"
        case .skipped:
            return "forward.circle"
        case .unavailable, .failed:
            return "exclamationmark.octagon"
        case .timedOut:
            return "clock.badge.exclamationmark"
        case .cancelled:
            return "xmark.circle"
        }
    }

    private func stateForegroundStyle(_ state: TaskCockpitProgressState) -> AnyShapeStyle {
        switch state {
        case .active:
            return AnyShapeStyle(.primary)
        case .unavailable, .timedOut, .failed:
            return AnyShapeStyle(.orange)
        case .idle, .queued, .completed, .empty, .fallback, .skipped, .cancelled:
            return AnyShapeStyle(.secondary)
        }
    }
}

private struct TaskCockpitFailedProviderOutputButton: View {
    let text: String
    @State private var didCopy = false

    var body: some View {
        Button {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(text, forType: .string)
            if let window = NSApp.mainWindow {
                NSAccessibility.post(
                    element: window,
                    notification: .announcementRequested,
                    userInfo: [
                        .announcement: UIStrings.text("action.copied", "Copied"),
                        .priority: NSAccessibilityPriorityLevel.high.rawValue,
                    ]
                )
            }
            didCopy = true
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
                didCopy = false
            }
        } label: {
            Label(
                didCopy
                    ? UIStrings.text("action.copied", "Copied")
                    : UIStrings.text("taskCockpit.provider.copyFailedOutput", "Copy untrusted provider response"),
                systemImage: didCopy ? "checkmark" : "doc.on.doc"
            )
        }
        .buttonStyle(.bordered)
        .controlSize(.small)
        .foregroundStyle(didCopy ? .green : .secondary)
        .help(UIStrings.text(
            "taskCockpit.provider.copyFailedOutput.help",
            "Copies the in-memory provider response for diagnosis. It is untrusted and is not persisted by the app."
        ))
        .accessibilityValue(didCopy ? UIStrings.text("action.copied", "Copied") : "")
    }
}

private struct TaskCockpitResultView: View {
    let taskText: String
    let result: TaskCockpitResult
    let operationState: TaskCockpitOperationState
    let isBuilding: Bool
    let onOpenRecommendation: (TaskCockpitRecommendation) -> Void
    @State private var diagnosticsExpanded = false

    private var model: TaskCockpitDecisionPresentationModel {
        TaskCockpitDecisionPresentationModel(result: result)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            TaskCockpitDecisionSummaryCard(
                model: model,
                handoffText: TaskCockpitHandoffModel.text(taskText: taskText, result: result),
                recommendation: TaskCockpitRecommendation.from(result),
                onOpenRecommendation: onOpenRecommendation
            )

            DisclosureGroup(isExpanded: $diagnosticsExpanded) {
                TaskCockpitTechnicalDiagnosticsView(
                    result: result,
                    operationState: operationState,
                    isBuilding: isBuilding
                )
                .padding(.top, 8)
            } label: {
                Label(UIStrings.taskCockpitDiagnosticsTitle, systemImage: "stethoscope")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier(AppAccessibilityID.taskCockpitResult)
        .accessibilityLabel(UIStrings.taskCockpitTitle)
    }
}

private enum TaskCockpitVerdict {
    case ready
    case needsReview
    case blocked
    case unavailable

    var title: String {
        switch self {
        case .ready:
            return UIStrings.taskCockpitVerdictReady
        case .needsReview:
            return UIStrings.taskCockpitVerdictNeedsReview
        case .blocked:
            return UIStrings.taskCockpitVerdictBlocked
        case .unavailable:
            return UIStrings.taskCockpitVerdictUnavailable
        }
    }

    var message: String {
        switch self {
        case .ready:
            return UIStrings.taskCockpitVerdictReadyMessage
        case .needsReview:
            return UIStrings.taskCockpitVerdictNeedsReviewMessage
        case .blocked:
            return UIStrings.taskCockpitVerdictBlockedMessage
        case .unavailable:
            return UIStrings.taskCockpitVerdictUnavailableMessage
        }
    }

    var systemImage: String {
        switch self {
        case .ready:
            return "checkmark.seal"
        case .needsReview:
            return "exclamationmark.triangle"
        case .blocked:
            return "octagon"
        case .unavailable:
            return "questionmark.circle"
        }
    }

    var tint: Color {
        switch self {
        case .ready:
            return .green
        case .needsReview:
            return .orange
        case .blocked:
            return .red
        case .unavailable:
            return .gray
        }
    }
}

private struct TaskCockpitDecisionPresentationModel {
    let result: TaskCockpitResult

    var verdict: TaskCockpitVerdict {
        if result.isUnavailable {
            return .unavailable
        }
        if !hasCandidatePath {
            return .blocked
        }
        if hasAgentOnlyCandidate {
            return .needsReview
        }
        if userBlockerCount > 0
            || gapCount > 0
            || scoreNeedsReview(readinessScore)
            || scoreNeedsReview(routingScore)
            || reviewRiskCount > 0
        {
            return .needsReview
        }
        return .ready
    }

    var hasReliableRecommendation: Bool {
        switch verdict {
        case .ready, .needsReview:
            return hasCandidatePath
        case .blocked, .unavailable:
            return false
        }
    }

    var recommendationLine: String {
        guard hasReliableRecommendation else {
            return UIStrings.taskCockpitNoReliableRecommendation
        }
        if recommendedSkill != UIStrings.unknown {
            return "\(recommendedAgent) · \(recommendedSkill)"
        }
        if recommendedAgent != UIStrings.unknown {
            return UIStrings.taskCockpitAgentOnlyRecommendation(recommendedAgent)
        }
        return UIStrings.taskCockpitNoReliableRecommendation
    }

    var agentScopeSummary: String {
        TaskCockpitHistoryRecord.agentScopeSummary(result.agentScopeIDs)
    }

    var recommendedAgent: String {
        let raw = result.summary.recommendedAgent ?? topRoute?.agent ?? topSkill?.agent ?? topAgent?.agent
        return raw.map(DisplayText.agent) ?? UIStrings.unknown
    }

    var recommendedSkill: String {
        result.summary.recommendedSkillName
            ?? topRoute?.skill?.name
            ?? topSkill?.skill?.name
            ?? topSkill?.title
            ?? topRoute?.title
            ?? UIStrings.unknown
    }

    var readinessScore: Int? {
        result.summary.readinessScore ?? topRoute?.readinessScore ?? topSkill?.readinessScore
    }

    var routingScore: Int? {
        result.summary.routingScore ?? topRoute?.routingScore ?? topSkill?.routingScore ?? topRoute?.score
    }

    var routeCount: Int {
        max(result.summary.routeCandidateCount, result.routeCandidates.count)
    }

    var skillCount: Int {
        max(result.summary.skillCandidateCount, result.skillCandidates.count)
    }

    var gapCount: Int {
        result.gapRows.isEmpty ? result.summary.gapCount : result.gapRows.count
    }

    var userBlockerCount: Int {
        if result.blockerRows.isEmpty {
            return result.summary.blockerCount
        }
        return userBlockerRows.count
    }

    var reviewRiskCount: Int {
        reviewRiskRows.count
    }

    var showsPartialNotice: Bool {
        result.recoveryDiagnosticReason != nil && !result.isUnavailable
    }

    var keyReasons: [String] {
        TaskCockpitDecisionModel(result: result).keyReasons
    }

    var candidateAlternatives: [String] {
        TaskCockpitDecisionModel(result: result).candidateAlternatives
    }

    var nextStep: String {
        switch verdict {
        case .ready:
            return UIStrings.taskCockpitNextStepReady
        case .needsReview:
            return UIStrings.taskCockpitNextStepNeedsReview
        case .blocked:
            return UIStrings.taskCockpitNextStepBlocked
        case .unavailable:
            return UIStrings.taskCockpitNextStepUnavailable
        }
    }

    private var topRoute: TaskCockpitCandidateRow? {
        result.routeCandidates.first
    }

    private var topSkill: TaskCockpitCandidateRow? {
        result.skillCandidates.first
    }

    private var topAgent: TaskCockpitCandidateRow? {
        result.agentCandidates.first
    }

    private var hasCandidatePath: Bool {
        recommendedAgent != UIStrings.unknown
            || recommendedSkill != UIStrings.unknown
            || topRoute != nil
            || topSkill != nil
            || topAgent != nil
    }

    private var hasAgentOnlyCandidate: Bool {
        topAgent != nil
            && topRoute == nil
            && topSkill == nil
            && recommendedSkill == UIStrings.unknown
    }

    private var hasRouteAmbiguity: Bool {
        guard uniqueCandidateRows.count > 1 else { return false }
        let candidateScores = uniqueCandidateRows.prefix(2).compactMap { row in
            row.routingScore ?? row.readinessScore ?? row.score
        }
        if candidateScores.count == 2 {
            return abs(candidateScores[0] - candidateScores[1]) <= 10
        }
        return result.summary.routeCandidateCount > 1
            || result.summary.skillCandidateCount > 1
            || result.summary.agentCandidateCount > 1
            || result.routeCandidates.count > 1
            || result.skillCandidates.count > 1
    }

    private var uniqueCandidateRows: [TaskCockpitCandidateRow] {
        let rows: [TaskCockpitCandidateRow]
        if !result.skillCandidates.isEmpty {
            rows = result.skillCandidates
        } else if !result.routeCandidates.isEmpty {
            rows = result.routeCandidates
        } else {
            rows = result.agentCandidates
        }
        var seen = Set<String>()
        var unique: [TaskCockpitCandidateRow] = []
        for row in rows {
            let name = row.skill?.name ?? row.title
            let key = "\(row.agent ?? ""):\(name)".lowercased()
            guard seen.insert(key).inserted else { continue }
            unique.append(row)
        }
        return unique
    }

    private var userBlockerRows: [TaskCockpitContextRow] {
        result.blockerRows.filter { row in
            !Self.isInternalBoundary(row)
                && !Self.isReviewOnlyRisk(row)
        }
    }

    private var reviewRiskRows: [TaskCockpitContextRow] {
        result.blockerRows.filter { row in
            !Self.isInternalBoundary(row)
                && Self.isReviewOnlyRisk(row)
        }
    }

    private func scoreNeedsReview(_ score: Int?) -> Bool {
        guard let score else { return false }
        return score < 70
    }

    private static func isReviewOnlyRisk(_ row: TaskCockpitContextRow) -> Bool {
        TaskCockpitSignalClassifier.classification(for: row) == .reviewOnlyRisk
    }

    private static func isInternalBoundary(_ row: TaskCockpitContextRow) -> Bool {
        TaskCockpitSignalClassifier.classification(for: row) == .internalBoundary
    }
}

private struct TaskCockpitScorePill: View {
    let label: String
    let score: Int?

    var body: some View {
        VStack(alignment: .trailing, spacing: 1) {
            Text(label)
                .font(.caption2.bold())
                .foregroundStyle(.secondary)
            Text(score.map(String.init) ?? UIStrings.unknown)
                .font(.callout.monospacedDigit().bold())
        }
        .frame(minWidth: 58, alignment: .trailing)
    }
}

private struct TaskCockpitDecisionSummaryCard: View {
    let model: TaskCockpitDecisionPresentationModel
    let handoffText: String
    let recommendation: TaskCockpitRecommendation?
    let onOpenRecommendation: (TaskCockpitRecommendation) -> Void
    @State private var didCopyHandoff = false

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .top, spacing: 12) {
                Image(systemName: model.verdict.systemImage)
                    .font(.title3)
                    .foregroundStyle(model.verdict.tint)
                    .frame(width: 26, alignment: .center)

                VStack(alignment: .leading, spacing: 5) {
                    Text(model.verdict.title)
                        .font(.headline)
                    Text(model.verdict.message)
                        .font(.callout)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                .frame(maxWidth: .infinity, alignment: .leading)

                HStack(spacing: 8) {
                    TaskCockpitScorePill(label: UIStrings.taskCockpitReadinessShort, score: model.readinessScore)
                    TaskCockpitScorePill(label: UIStrings.taskCockpitRoutingShort, score: model.routingScore)
                }
            }

            if model.showsPartialNotice {
                Label(UIStrings.taskCockpitPartialNotice, systemImage: "info.circle")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            if model.hasReliableRecommendation {
                Label(model.recommendationLine, systemImage: "arrow.triangle.branch")
                    .font(.callout.weight(.semibold))
                    .foregroundStyle(.primary)
                    .lineLimit(2)
            } else {
                Label(UIStrings.taskCockpitNoReliableRecommendation, systemImage: "hand.raised")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            Label(model.agentScopeSummary, systemImage: "person.2")
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(1)

            if !model.keyReasons.isEmpty {
                VStack(alignment: .leading, spacing: 7) {
                    Label(UIStrings.taskCockpitReasonsTitle, systemImage: "text.bubble")
                        .font(.callout.bold())

                    ExpandableSummaryList(
                        TaskCockpitSummaryTextRow.rows(for: model.keyReasons),
                        visibleLimit: 2,
                        spacing: 7,
                        accessibilityIdentifier: "task-cockpit-decision-reasons.show-all"
                    ) { reason in
                        PrivacyEvidenceLabel(value: reason.value, systemImage: reasonSystemImage, font: .callout, lineLimit: 2)
                    }
                }
            }

            if !model.candidateAlternatives.isEmpty {
                VStack(alignment: .leading, spacing: 6) {
                    Label(UIStrings.taskCockpitCandidateAlternativesTitle, systemImage: "list.bullet")
                        .font(.callout.bold())

                    ExpandableSummaryList(
                        TaskCockpitSummaryTextRow.rows(for: model.candidateAlternatives),
                        visibleLimit: 3,
                        spacing: 6,
                        accessibilityIdentifier: "task-cockpit-candidate-alternatives.show-all"
                    ) { candidate in
                        PrivacyEvidenceLabel(value: candidate.value, systemImage: "chevron.right.circle", font: .callout, lineLimit: 1)
                    }
                }
            }

            Label(model.nextStep, systemImage: "arrow.forward.circle")
                .font(.callout)
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, alignment: .leading)

            HStack(spacing: 8) {
                Button {
                    copyHandoff()
                } label: {
                    Label(
                        didCopyHandoff
                            ? UIStrings.text("action.copied", "Copied")
                            : UIStrings.text("taskCockpit.action.copyHandoff", "Copy Handoff"),
                        systemImage: didCopyHandoff ? "checkmark" : "doc.on.doc"
                    )
                }
                .buttonStyle(.bordered)
                .controlSize(.small)
                .accessibilityHint(UIStrings.text(
                    "taskCockpit.action.copyHandoff.hint",
                    "Copy the task, recommended route, scores, and review notes for use in your agent."
                ))

                if let recommendation {
                    Button {
                        onOpenRecommendation(recommendation)
                    } label: {
                        Label(
                            UIStrings.text("taskCockpit.action.useRecommendation", "Use Recommendation"),
                            systemImage: "arrow.right.circle.fill"
                        )
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.small)
                    .accessibilityHint(UIStrings.text(
                        "taskCockpit.action.useRecommendation.hint",
                        "Open the recommended local skill, or prepare a Skill Manager search if it is not installed."
                    ))
                }
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(model.verdict.tint.opacity(0.10), in: RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(model.verdict.title)
        .accessibilityValue(model.verdict.message)
    }

    private func copyHandoff() {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(handoffText, forType: .string)
        didCopyHandoff = true
        if let window = NSApp.mainWindow {
            NSAccessibility.post(
                element: window,
                notification: .announcementRequested,
                userInfo: [
                    .announcement: UIStrings.text("action.copied", "Copied"),
                    .priority: NSAccessibilityPriorityLevel.high.rawValue,
                ]
            )
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
            didCopyHandoff = false
        }
    }

    private var reasonSystemImage: String {
        switch model.verdict {
        case .ready:
            return "checkmark.circle"
        case .needsReview:
            return "exclamationmark.triangle"
        case .blocked:
            return "exclamationmark.circle"
        case .unavailable:
            return "questionmark.circle"
        }
    }

}

private struct TaskCockpitTechnicalDiagnosticsView: View {
    let result: TaskCockpitResult
    let operationState: TaskCockpitOperationState
    let isBuilding: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(UIStrings.taskCockpitDiagnosticsSummary)
                .font(.callout)
                .foregroundStyle(.secondary)

            TaskCockpitStageProgressView(
                state: operationState,
                isBuilding: isBuilding,
                result: result
            )

            if let fallbackReason = result.fallbackReason,
               let displayFallbackReason = TaskCockpitDecisionModel.displayText(fallbackReason) {
                PrivacyEvidenceLabel(value: displayFallbackReason, systemImage: "info.circle", font: .callout, lineLimit: 2)
            }

            TaskCockpitMatchingProcessView(
                result: result,
                routeCount: routeCount,
                skillCount: skillCount,
                agentCount: agentCount
            )

            if !result.taskRows.isEmpty {
                TaskCockpitCandidateList(
                    title: UIStrings.taskCockpitTasks,
                    empty: UIStrings.taskCockpitNoRows,
                    rows: result.taskRows,
                    systemImage: "checklist",
                    accessibilityIdentifier: "task-cockpit-tasks.show-all"
                )
            }

            if !result.routeCandidates.isEmpty {
                TaskCockpitCandidateList(
                    title: UIStrings.taskCockpitRoutes,
                    empty: UIStrings.taskCockpitNoRows,
                    rows: result.routeCandidates,
                    systemImage: "arrow.triangle.branch",
                    accessibilityIdentifier: "task-cockpit-candidates.show-all"
                )
            }

            if !result.agentCandidates.isEmpty {
                TaskCockpitCandidateList(
                    title: UIStrings.taskCockpitAgents,
                    empty: UIStrings.taskCockpitNoRows,
                    rows: result.agentCandidates,
                    systemImage: "person.2",
                    accessibilityIdentifier: "task-cockpit-agents.show-all"
                )
            }

            if !result.skillCandidates.isEmpty {
                TaskCockpitCandidateList(
                    title: UIStrings.taskCockpitSkills,
                    empty: UIStrings.taskCockpitNoRows,
                    rows: result.skillCandidates,
                    systemImage: "square.stack.3d.up",
                    accessibilityIdentifier: "task-cockpit-skills.show-all"
                )
            }

            if !result.cockpitSections.isEmpty {
                TaskCockpitContextList(
                    title: UIStrings.taskCockpitSections,
                    empty: UIStrings.taskCockpitNoRows,
                    rows: result.cockpitSections,
                    systemImage: "rectangle.3.group",
                    accessibilityIdentifier: "task-cockpit-sections.show-all"
                )
            }

            if !result.readinessSignals.isEmpty {
                TaskCockpitContextList(
                    title: UIStrings.taskCockpitReadinessSignals,
                    empty: UIStrings.taskCockpitNoRows,
                    rows: result.readinessSignals,
                    systemImage: "checkmark.seal",
                    accessibilityIdentifier: "task-cockpit-readiness.show-all"
                )
            }

            if !result.providerObservabilityContext.isEmpty {
                TaskCockpitContextList(
                    title: UIStrings.taskCockpitProviderContext,
                    empty: UIStrings.taskCockpitNoRows,
                    rows: result.providerObservabilityContext,
                    systemImage: "waveform.path.ecg",
                    accessibilityIdentifier: "task-cockpit-provider-context.show-all"
                )
            }

            if !result.gapRows.isEmpty {
                TaskCockpitContextList(
                    title: UIStrings.text("taskCockpit.gaps", "Gaps"),
                    empty: UIStrings.taskCockpitNoRows,
                    rows: result.gapRows,
                    systemImage: "exclamationmark.triangle",
                    accessibilityIdentifier: "task-cockpit-context.show-all"
                )
            }

            if !result.blockerRows.isEmpty {
                TaskCockpitContextList(
                    title: UIStrings.text("taskCockpit.blockers", "Blockers"),
                    empty: UIStrings.taskCockpitNoRows,
                    rows: result.blockerRows,
                    systemImage: "exclamationmark.octagon",
                    accessibilityIdentifier: "task-cockpit-blockers.show-all"
                )
            }

            if !result.evidenceReferences.isEmpty {
                TaskCockpitEvidenceList(evidence: result.evidenceReferences)
            }

            TaskCockpitSafetyList(safety: result.safetyFlags)
        }
    }

    private var routeCount: Int {
        result.summary.routeCandidateCount > 0 ? result.summary.routeCandidateCount : result.routeCandidates.count
    }

    private var agentCount: Int {
        result.summary.agentCandidateCount > 0 ? result.summary.agentCandidateCount : result.agentCandidates.count
    }

    private var skillCount: Int {
        result.summary.skillCandidateCount > 0 ? result.summary.skillCandidateCount : result.skillCandidates.count
    }

    private var readinessSignalCount: Int {
        result.summary.readinessSignalCount > 0 ? result.summary.readinessSignalCount : result.readinessSignals.count
    }

    private var gapCount: Int {
        result.summary.gapCount > 0 ? result.summary.gapCount : result.gapRows.count
    }

    private var blockerCount: Int {
        result.summary.blockerCount > 0 ? result.summary.blockerCount : result.blockerRows.count
    }

}

private struct TaskCockpitMatchingProcessView: View {
    let result: TaskCockpitResult
    let routeCount: Int
    let skillCount: Int
    let agentCount: Int

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.taskCockpitDiagnosticsProcess)
                .font(.caption.bold())
                .foregroundStyle(.secondary)

            VStack(alignment: .leading, spacing: 7) {
                PrivacyEvidenceLabel(
                    value: scanSummary,
                    systemImage: "line.3.horizontal.decrease.circle",
                    font: .callout,
                    lineLimit: 2
                )

                if let topRoute = result.routeCandidates.first {
                    PrivacyEvidenceLabel(
                        value: topRouteSummary(topRoute),
                        systemImage: "arrow.triangle.branch",
                        font: .callout,
                        lineLimit: 2
                    )
                }

                ExpandableSummaryList(
                    processNoteRows,
                    visibleLimit: 3,
                    spacing: 7,
                    accessibilityIdentifier: "task-cockpit-process-notes.show-all"
                ) { note in
                    PrivacyEvidenceLabel(
                        value: note.value,
                        systemImage: "text.bubble",
                        font: .callout,
                        lineLimit: 2
                    )
                }
            }
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
        }
    }

    private var scanSummary: String {
        "\(UIStrings.taskCockpitDiagnosticsScanned): \(routeCount) \(UIStrings.taskCockpitRoutes), \(skillCount) \(UIStrings.taskCockpitSkills), \(agentCount) \(UIStrings.taskCockpitAgents)"
    }

    private func topRouteSummary(_ row: TaskCockpitCandidateRow) -> String {
        let score = row.routingScore ?? row.readinessScore ?? row.score
        let scoreText = score.map { " · \($0)" } ?? ""
        return "\(UIStrings.taskCockpitDiagnosticsTopRoute): \(row.title)\(scoreText)"
    }

    private var processNotes: [String] {
        TaskCockpitDecisionModel(result: result).processNotes
    }

    private var processNoteRows: [TaskCockpitSummaryTextRow] {
        TaskCockpitSummaryTextRow.rows(for: processNotes)
    }
}

private struct TaskCockpitCandidateList: View {
    let title: String
    let empty: String
    let rows: [TaskCockpitCandidateRow]
    let systemImage: String
    let accessibilityIdentifier: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if rows.isEmpty {
                Text(empty)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                ExpandableSummaryList(
                    rows,
                    visibleLimit: 8,
                    spacing: 8,
                    columns: [GridItem(.adaptive(minimum: 250), spacing: 8)],
                    accessibilityIdentifier: accessibilityIdentifier
                ) { row in
                    VStack(alignment: .leading, spacing: 7) {
                        HStack(alignment: .firstTextBaseline) {
                            Label(rowTitle(row), systemImage: systemImage)
                                .font(.callout.bold())
                                .lineLimit(1)
                            Spacer()
                            if let score = row.routingScore ?? row.readinessScore ?? row.score {
                                Text("\(score)")
                                    .font(.caption.monospacedDigit().bold())
                            }
                        }
                        HStack(spacing: 6) {
                            if let agent = row.agent, !agent.isEmpty {
                                Text(DisplayText.agent(agent))
                            }
                            if let band = row.band, !band.isEmpty {
                                Text(band)
                            }
                            if let status = row.status, !status.isEmpty {
                                Text(status)
                            }
                        }
                        .font(.caption2.bold())
                        .foregroundStyle(.secondary)

                        Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 3) {
                            MetadataRow(label: UIStrings.taskCockpitAgentReadinessScore, value: row.readinessScore.map(String.init) ?? UIStrings.unknown)
                            MetadataRow(label: UIStrings.taskCockpitAgentRoutingScore, value: row.routingScore.map(String.init) ?? UIStrings.unknown)
                            if let skill = row.skill {
                                MetadataRow(label: UIStrings.taskCockpitAgentBestSkill, value: skill.name)
                            }
                        }

                        if !row.summary.isEmpty {
                            PrivacyEvidenceText(value: row.summary, font: .caption, lineLimit: 3)
                        }
                        RoutingInlineList(title: UIStrings.taskCockpitAgentReasons, empty: UIStrings.taskCockpitAgentNoReasons, values: row.reasons, systemImage: "text.bubble")
                        RoutingInlineList(title: UIStrings.taskCockpitEvidence, empty: UIStrings.taskCockpitNoEvidence, values: row.evidenceRefs, systemImage: "checklist")
                        RoutingInlineList(title: UIStrings.safetyFlags, empty: UIStrings.noSafetyFlags, values: row.safetyFlags, systemImage: "checkmark.shield")
                    }
                    .padding(10)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
                }
            }
        }
    }

    private func rowTitle(_ row: TaskCockpitCandidateRow) -> String {
        if let rank = row.rank {
            return "#\(rank) \(row.title)"
        }
        return row.title
    }
}

private struct TaskCockpitContextList: View {
    let title: String
    let empty: String
    let rows: [TaskCockpitContextRow]
    let systemImage: String
    let accessibilityIdentifier: String

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if rows.isEmpty {
                Text(empty)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                ExpandableSummaryList(
                    rows,
                    visibleLimit: 6,
                    spacing: 6,
                    accessibilityIdentifier: accessibilityIdentifier
                ) { row in
                    VStack(alignment: .leading, spacing: 4) {
                        HStack(alignment: .firstTextBaseline) {
                            Label(row.title, systemImage: systemImage)
                                .font(.callout.bold())
                            Spacer()
                            if let count = row.count {
                                Text("\(count)")
                                    .font(.caption.monospacedDigit().bold())
                                    .foregroundStyle(.secondary)
                            }
                        }
                        HStack(spacing: 8) {
                            if let agent = row.agent, !agent.isEmpty {
                                Text(DisplayText.agent(agent))
                            }
                            if let status = row.status, !status.isEmpty {
                                Text(status)
                            }
                            if let severity = row.severity, !severity.isEmpty {
                                Text(severity)
                            }
                            if let source = row.source, !source.isEmpty {
                                PrivacyEvidenceText(value: source, font: .caption2, lineLimit: 1)
                            }
                        }
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        if !row.detail.isEmpty {
                            PrivacyEvidenceText(value: row.detail, font: .caption, lineLimit: nil)
                        }
                        RoutingInlineList(title: UIStrings.taskCockpitEvidence, empty: UIStrings.taskCockpitNoEvidence, values: row.evidenceRefs, systemImage: "checklist")
                        RoutingInlineList(title: UIStrings.safetyFlags, empty: UIStrings.noSafetyFlags, values: row.safetyFlags, systemImage: "checkmark.shield")
                    }
                    .padding(8)
                    .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
                }
            }
        }
    }
}

private struct TaskCockpitEvidenceList: View {
    let evidence: [ProviderObservabilityEvidenceReference]

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                Text(UIStrings.taskCockpitEvidence)
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                if !evidence.isEmpty {
                    DenseCountBadge(count: evidence.count)
                }
            }
            if evidence.isEmpty {
                Text(UIStrings.taskCockpitNoEvidence)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                ExpandableSummaryList(
                    evidence,
                    visibleLimit: 6,
                    spacing: 6,
                    accessibilityIdentifier: "task-cockpit-evidence.show-all"
                ) { item in
                    VStack(alignment: .leading, spacing: 2) {
                        Label(item.title, systemImage: "checklist")
                            .font(.callout)
                        HStack(spacing: 8) {
                            if let agent = item.agent, !agent.isEmpty {
                                Text(DisplayText.agent(agent))
                            }
                            if let source = item.source, !source.isEmpty {
                                PrivacyEvidenceText(value: source, font: .caption2, lineLimit: 1)
                            }
                        }
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        PrivacyEvidenceText(value: item.detail, font: .caption, lineLimit: nil)
                    }
                }
            }
        }
    }
}

private struct TaskCockpitSafetyList: View {
    let safety: ProviderObservabilitySafety

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.taskCockpitSafetyFlags)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            Label(
                safety.allReadOnlyFlagsClear ? UIStrings.safetyReadOnlyClear : UIStrings.safetyReadOnlyWarning,
                systemImage: safety.allReadOnlyFlagsClear ? "checkmark.shield" : "exclamationmark.triangle"
            )
            .font(.callout)
            .foregroundStyle(.secondary)

            LazyVGrid(columns: [GridItem(.adaptive(minimum: 185), spacing: 8)], alignment: .leading, spacing: 8) {
                ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                    SafetyPill(label: row.label, isBlocked: !row.isUnsafe)
                }
            }

            if !safety.notes.isEmpty {
                DenseDisclosureList(safety.notes, visibleLimit: 4, spacing: 4) { note in
                    PrivacyEvidenceLabel(value: note, systemImage: "info.circle", font: .caption, lineLimit: 2)
                }
            }
        }
    }

    private var rows: [(label: String, isUnsafe: Bool)] {
        [
            (UIStrings.safetyProviderNotSent, safety.providerRequestSent),
            (UIStrings.safetyWritesBlocked, safety.writeBackAllowed || safety.writeActionsAvailable),
            (UIStrings.safetyScriptsBlocked, safety.scriptExecutionAllowed || safety.executionActionsAvailable),
            (UIStrings.safetyMutationsBlocked, safety.configMutationAllowed || safety.snapshotCreated || safety.triageMutationAllowed),
            (UIStrings.safetyCredentialsBlocked, safety.credentialAccessed || safety.rawSecretReturned),
            (UIStrings.llmPromptRawPromptStored, safety.rawPromptPersisted),
            (UIStrings.llmPromptRawResponseStored, safety.rawResponsePersisted),
            (UIStrings.safetyRawTraceStored, safety.rawTracePersisted),
            (UIStrings.safetyCloudSync, safety.cloudSyncEnabled),
            (UIStrings.safetyTelemetry, safety.telemetryEnabled)
        ]
    }
}
