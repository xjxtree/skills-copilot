import AppKit
import SwiftUI

struct AgentSessionDetailPanel: View {
    @EnvironmentObject private var store: SkillStore
    @EnvironmentObject private var sessionStore: SessionStore

    var body: some View {
        AgentSessionContentPanel(
            session: store.selectedLocalSession,
            detailState: sessionStore.selectedLocalSessionDetailState,
            messageCompleteness: sessionStore.selectedLocalSessionMessageCompleteness,
            diagnosticNotes: sessionStore.localSessionPreviewResult.blockerNotes
                + sessionStore.localSessionPreviewResult.gapNotes,
            isRefreshing: sessionStore.isPreviewingLocalSessions,
            showsLoadedRowsFilterNotice: sessionStore.localSessionCompleteness.hasMore,
            onRefresh: {
                Task {
                    await store.previewLocalSessions()
                }
            },
            onLoadDetail: {
                guard let sessionID = sessionStore.selectedLocalSessionID else { return }
                Task { await store.loadLocalSessionDetailIfNeeded(sessionID: sessionID) }
            },
            onCancelMessageLoad: store.cancelLocalSessionMessageLoad,
            onOpenSkill: { name, agent in
                _ = store.navigateToSkill(name: name, agent: agent)
            },
            onInstallSkill: { name in
                store.presentSkillManager(searchQuery: name)
            }
        )
    }
}

private struct AgentSessionContentPanel: View {
    let session: LocalSessionPreviewRow?
    let detailState: LocalSessionDetailState?
    let messageCompleteness: ListCompletenessState
    let diagnosticNotes: [String]
    let isRefreshing: Bool
    let showsLoadedRowsFilterNotice: Bool
    let onRefresh: () -> Void
    let onLoadDetail: () -> Void
    let onCancelMessageLoad: () -> Void
    let onOpenSkill: (String, String?) -> Void
    let onInstallSkill: (String) -> Void

    @State private var selectedKinds = LocalSessionContentKind.defaultDetailKinds

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Label(UIStrings.text("agentCopilot.sessions.title", "Sessions"), systemImage: "bubble.left.and.text.bubble.right")
                    .font(.headline)
                Spacer()
                Button {
                    onRefresh()
                } label: {
                    Label(UIStrings.text("sidebar.sessions.preview", "Refresh Sessions"), systemImage: "arrow.clockwise")
                }
                .controlSize(.small)
                .disabled(isRefreshing)
            }

            if let session {
                VStack(alignment: .leading, spacing: 10) {
                    HStack(alignment: .firstTextBaseline, spacing: 8) {
                        VStack(alignment: .leading, spacing: 3) {
                            Text(session.title)
                                .font(.callout.bold())
                                .lineLimit(1)
                            PrivacyEvidenceText(value: session.redactedPath, font: .caption2, lineLimit: 1)
                            if let timeRange = sessionTimeRangeText(session) {
                                Label(timeRange, systemImage: "clock")
                                    .font(.caption2)
                                    .foregroundStyle(.secondary)
                                    .lineLimit(1)
                            }
                        }
                        Spacer()
                        if let agent = session.agent, !agent.isEmpty {
                            Text(DisplayText.agent(agent))
                                .font(.caption.bold())
                                .foregroundStyle(.secondary)
                        }
                    }

                    if case .loading = detailState {
                        HStack(spacing: 8) {
                            ProgressView().controlSize(.small)
                            Text(UIStrings.text("agentCopilot.sessions.loadingDetail", "Loading selected session detail..."))
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    } else if case .failed(let displayError) = detailState {
                        VStack(alignment: .leading, spacing: 8) {
                            Text(displayError)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            Button(UIStrings.text("agentCopilot.sessions.retryDetail", "Retry Detail"), action: onLoadDetail)
                                .controlSize(.small)
                        }
                    } else if session.contentIncluded {
                        LocalSessionContentFilterBar(items: session.contentItems, selectedKinds: $selectedKinds)
                        let visibleItems = session.contentItems.filter { selectedKinds.contains($0.kind) }
                        if visibleItems.isEmpty {
                            Text(emptyFilteredContentMessage)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        } else {
                            LazyVStack(alignment: .leading, spacing: 8) {
                                ForEach(visibleItems) { item in
                                    LocalSessionContentItemRow(
                                        item: item,
                                        sessionAgent: session.agent,
                                        onOpenSkill: onOpenSkill,
                                        onInstallSkill: onInstallSkill
                                    )
                                }
                            }
                        }
                        if messageCompleteness.completeness != .complete
                            || messageCompleteness.loadingPhase != .idle {
                            ListCompletenessFooter(
                                state: messageCompleteness,
                                onLoadMore: onLoadDetail,
                                onLoadAll: onLoadDetail,
                                onCancel: onCancelMessageLoad,
                                accessibilityIdentifierPrefix: "session-messages"
                            )
                            .accessibilityIdentifier("session-messages.completeness")
                        }
                    } else {
                        Button(UIStrings.text("agentCopilot.sessions.loadDetail", "Load Session Detail"), action: onLoadDetail)
                            .controlSize(.small)
                    }
                }
            } else {
                Text(emptySessionMessage)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            if !diagnosticNotes.isEmpty {
                DisclosureGroup(
                    UIStrings.text("agentCopilot.sessions.diagnostics", "Source diagnostics")
                ) {
                    VStack(alignment: .leading, spacing: 5) {
                        ForEach(Array(diagnosticNotes.enumerated()), id: \.offset) { _, note in
                            Text("• \(note)")
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                                .textSelection(.enabled)
                                .fixedSize(horizontal: false, vertical: true)
                        }
                    }
                    .padding(.top, 4)
                }
                .font(.caption.bold())
            }

            if showsLoadedRowsFilterNotice {
                Text(UIStrings.text(
                    "agentCopilot.sessions.loadedRowsFilterNotice",
                    "Search, scope, and sort currently cover loaded session summaries; load more to include remaining rows."
                ))
                .font(.caption2)
                .foregroundStyle(.secondary)
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
        .onChange(of: session?.id) { _ in
            selectedKinds = LocalSessionContentKind.defaultDetailKinds
        }
    }

    private var emptySessionMessage: String {
        return UIStrings.text("agentCopilot.sessions.empty", "No local sessions are loaded for the selected agent.")
    }

    private var emptyFilteredContentMessage: String {
        if selectedKinds.isEmpty {
            return UIStrings.text("agentCopilot.sessions.noSelectedFilters", "Select at least one content filter.")
        }
        return UIStrings.text("agentCopilot.sessions.noFilteredContent", "No session content matches the selected filters.")
    }

    private func sessionTimeRangeText(_ session: LocalSessionPreviewRow) -> String? {
        switch (session.startedAt, session.endedAt) {
        case let (started?, ended?) where started != ended:
            let startLabel = UIStrings.text("agentCopilot.sessions.started", "Started")
            let endLabel = UIStrings.text("agentCopilot.sessions.ended", "Last")
            return "\(startLabel) \(DisplayText.timestamp(started)) · \(endLabel) \(DisplayText.timestamp(ended))"
        case let (started?, _):
            let startLabel = UIStrings.text("agentCopilot.sessions.started", "Started")
            return "\(startLabel) \(DisplayText.timestamp(started))"
        case let (_, ended?):
            let endLabel = UIStrings.text("agentCopilot.sessions.ended", "Last")
            return "\(endLabel) \(DisplayText.timestamp(ended))"
        default:
            return nil
        }
    }
}

private struct LocalSessionContentFilterBar: View {
    let items: [LocalSessionContentItem]
    @Binding var selectedKinds: Set<LocalSessionContentKind>

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 6) {
                Button {
                    selectedKinds = Set(LocalSessionContentKind.allCases)
                } label: {
                    filterLabel(
                        title: UIStrings.text("agentCopilot.sessions.filterAll", "All"),
                        count: items.count,
                        systemImage: "line.3.horizontal.decrease.circle",
                        tint: .accentColor,
                        isSelected: selectedKinds.count == LocalSessionContentKind.allCases.count,
                        isDisabled: false
                    )
                }
                .buttonStyle(.plain)
                .accessibilityAddTraits(selectedKinds.count == LocalSessionContentKind.allCases.count ? .isSelected : [])

                ForEach(LocalSessionContentKind.allCases) { kind in
                    let count = contentCount(for: kind)
                    let isSelected = selectedKinds.contains(kind)
                    Button {
                        toggle(kind)
                    } label: {
                        filterLabel(
                            title: kind.title,
                            count: count,
                            systemImage: kind.systemImage,
                            tint: kind.semanticTint,
                            isSelected: isSelected,
                            isDisabled: count == 0
                        )
                    }
                    .buttonStyle(.plain)
                    .disabled(count == 0)
                    .accessibilityAddTraits(isSelected ? .isSelected : [])
                }
            }
        }
    }

    private func contentCount(for kind: LocalSessionContentKind) -> Int {
        items.filter { $0.kind == kind }.count
    }

    private func filterLabel(
        title: String,
        count: Int,
        systemImage: String,
        tint: Color,
        isSelected: Bool,
        isDisabled: Bool
    ) -> some View {
        Label {
            HStack(spacing: 4) {
                Text(title)
                    .font(.caption.bold())
                    .lineLimit(1)
                Text("\(count)")
                    .font(.caption2.monospacedDigit())
                    .foregroundStyle(isSelected ? tint : .secondary)
            }
        } icon: {
            Image(systemName: systemImage)
        }
        .foregroundStyle(filterForeground(tint: tint, isSelected: isSelected, isDisabled: isDisabled))
        .padding(.horizontal, 9)
        .padding(.vertical, 5)
        .background(
            isSelected ? tint.opacity(0.16) : Color.agentCopilotPanelBackground,
            in: RoundedRectangle(cornerRadius: 6)
        )
        .overlay(
            RoundedRectangle(cornerRadius: 6)
                .stroke(isSelected ? tint.opacity(0.45) : Color.secondary.opacity(0.14), lineWidth: 1)
        )
    }

    private func filterForeground(tint: Color, isSelected: Bool, isDisabled: Bool) -> Color {
        if isDisabled {
            return Color.secondary.opacity(0.55)
        }
        if isSelected {
            return tint
        }
        return .secondary
    }

    private func toggle(_ kind: LocalSessionContentKind) {
        if selectedKinds.contains(kind) {
            selectedKinds.remove(kind)
        } else {
            selectedKinds.insert(kind)
        }
    }
}

private struct LocalSessionContentItemRow: View {
    let item: LocalSessionContentItem
    let sessionAgent: String?
    let onOpenSkill: (String, String?) -> Void
    let onInstallSkill: (String) -> Void

    @State private var isShowingFullText = false
    @State private var didCopy = false

    private var isLongMessage: Bool {
        item.charCount > 600 || item.text.split(whereSeparator: \.isNewline).count > 8
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .top, spacing: 8) {
                VStack(alignment: .leading, spacing: 2) {
                    Label(item.title.isEmpty ? item.kind.title : item.title, systemImage: item.kind.systemImage)
                        .font(.caption.bold())
                        .foregroundStyle(item.kind.semanticTint)
                        .lineLimit(1)
                    if let timestamp = item.timestamp {
                        Text(DisplayText.timestamp(timestamp))
                            .font(.caption2.monospacedDigit())
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                    }
                }
                Spacer(minLength: 8)
                Text(UIStrings.localSessionContentCharacters(item.charCount))
                    .font(.caption2.monospacedDigit())
                    .foregroundStyle(.secondary)
                if didCopy {
                    Label(UIStrings.text("action.copied", "Copied"), systemImage: "checkmark")
                        .font(.caption2.bold())
                        .foregroundStyle(.green)
                        .transition(.opacity)
                }
                HStack(spacing: 4) {
                    if let skillName = item.referencedSkillName {
                        Button {
                            onOpenSkill(skillName, sessionAgent)
                        } label: {
                            Label(
                                UIStrings.text("session.skillCall.open", "Open Skill"),
                                systemImage: "arrow.right.circle"
                            )
                            .labelStyle(.iconOnly)
                        }
                        .controlSize(.small)
                        .buttonStyle(.borderless)
                        .help(UIStrings.text("session.skillCall.open", "Open Skill"))
                        .accessibilityLabel(UIStrings.text("session.skillCall.open", "Open Skill"))

                        Button {
                            onInstallSkill(skillName)
                        } label: {
                            Label(
                                UIStrings.text("session.skillCall.installAnother", "Install in Another Agent…"),
                                systemImage: "shippingbox.and.arrow.backward"
                            )
                            .labelStyle(.iconOnly)
                        }
                        .controlSize(.small)
                        .buttonStyle(.borderless)
                        .help(UIStrings.text("session.skillCall.installAnother", "Install in Another Agent…"))
                        .accessibilityLabel(UIStrings.text("session.skillCall.installAnother", "Install in Another Agent…"))
                    }
                    if isLongMessage {
                        detailButton
                    }
                    copyButton
                }
            }
            RenderedLongText(
                text: item.text,
                renderMode: .plain,
                isEmpty: item.text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
                lineLimit: isLongMessage ? 6 : 8
            )
            if !item.evidenceRefs.isEmpty {
                RoutingInlineList(title: UIStrings.taskCockpitEvidence, empty: UIStrings.taskCockpitNoEvidence, values: item.evidenceRefs, systemImage: "checklist")
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
        .contextMenu {
            if let skillName = item.referencedSkillName {
                Button {
                    onOpenSkill(skillName, sessionAgent)
                } label: {
                    Label(
                        UIStrings.text("session.skillCall.open", "Open Skill"),
                        systemImage: "arrow.right.circle"
                    )
                }
                Button {
                    onInstallSkill(skillName)
                } label: {
                    Label(
                        UIStrings.text("session.skillCall.installAnother", "Install in Another Agent…"),
                        systemImage: "shippingbox.and.arrow.backward"
                    )
                }
                Divider()
            }
            Button {
                copyToPasteboard(item.text)
            } label: {
                Label(UIStrings.llmPromptCopyFullText, systemImage: "doc.on.doc")
            }
            Button {
                isShowingFullText = true
            } label: {
                Label(UIStrings.llmPromptViewDetails, systemImage: "arrow.up.left.and.arrow.down.right")
            }
        }
        .sheet(isPresented: $isShowingFullText) {
            LongTextDetailSheet(
                title: item.title.isEmpty ? item.kind.title : item.title,
                text: item.text,
                renderMode: .plain
            )
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(item.title.isEmpty ? item.kind.title : item.title)
        .accessibilityValue(item.text)
    }

    private var detailButton: some View {
        Button {
            isShowingFullText = true
        } label: {
            Label(UIStrings.llmPromptViewDetails, systemImage: "arrow.up.left.and.arrow.down.right")
        }
        .controlSize(.small)
        .buttonStyle(.borderless)
        .help(UIStrings.llmPromptViewDetails)
    }

    private var copyButton: some View {
        Button {
            copyToPasteboard(item.text)
        } label: {
            if isLongMessage {
                Label(UIStrings.llmPromptCopyFullText, systemImage: "doc.on.doc")
            } else {
                Image(systemName: "doc.on.doc")
            }
        }
        .controlSize(.small)
        .buttonStyle(.borderless)
        .help(UIStrings.llmPromptCopyFullText)
        .accessibilityValue(didCopy ? UIStrings.text("action.copied", "Copied") : "")
    }

    private func copyToPasteboard(_ value: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(value, forType: .string)
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
        withAnimation(.easeInOut(duration: 0.12)) {
            didCopy = true
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) {
            withAnimation(.easeInOut(duration: 0.12)) {
                didCopy = false
            }
        }
    }
}

private extension LocalSessionContentKind {
    var semanticTint: Color {
        switch self {
        case .userMessage:
            return .blue
        case .agentReply:
            return .purple
        case .thinking:
            return .indigo
        case .toolCall:
            return .orange
        case .skillCall:
            return .green
        }
    }
}
