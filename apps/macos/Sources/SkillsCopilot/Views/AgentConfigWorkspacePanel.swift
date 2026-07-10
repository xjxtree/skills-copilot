import SwiftUI

@MainActor
enum AgentConfigDisplay {
    static func targetPath(for agent: SkillAgentFilter, store: SkillStore) -> String {
        switch agent {
        case .claudeCode:
            return store.claudeSettings?.target ?? "~/.claude/settings.json"
        case .codex:
            return "~/.codex/config.toml"
        case .opencode:
            return "~/.config/opencode/opencode.json"
        case .pi:
            return "~/.pi/agent/settings.json / <project>/.pi/settings.json"
        case .hermes:
            return "~/.hermes/config.yaml"
        case .openclaw:
            return "~/.openclaw/openclaw.json"
        case .all:
            return UIStrings.unknown
        }
    }

    static func shortTargetPath(for agent: SkillAgentFilter, store: SkillStore) -> String {
        pathSummary(targetPath(for: agent, store: store))
    }

    static func pathSummary(_ path: String) -> String {
        let trimmed = path.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return UIStrings.unknown }
        return DisplayText.configPathSummary(trimmed)
    }

    static func supportText(_ capability: AdapterFeatureCapability?) -> String {
        capability?.supported == true ? UIStrings.supported : UIStrings.notSupported
    }

    static func supportSymbol(_ capability: AdapterFeatureCapability?) -> String {
        capability?.supported == true ? "checkmark.circle.fill" : "minus.circle"
    }

    static func supportColor(_ capability: AdapterFeatureCapability?) -> Color {
        capability?.supported == true ? .green : .secondary
    }

    static func disabledSkills(for agent: SkillAgentFilter, store: SkillStore) -> [SkillRecord] {
        guard agent != .all else { return [] }
        return store.skills
            .filter { skill in
                skill.agent == agent.rawValue
                    && (!skill.enabled || skill.state.caseInsensitiveCompare("disabled") == .orderedSame)
            }
            .sorted { lhs, rhs in
                lhs.name.localizedStandardCompare(rhs.name) == .orderedAscending
            }
    }

    static func disabledSkillNamesSummary(_ skills: [SkillRecord], limit: Int = 3) -> String {
        let names = skills.prefix(limit).map(\.name).joined(separator: ", ")
        let remaining = skills.count - min(skills.count, limit)
        guard remaining > 0 else { return names }
        return "\(names) · \(UIStrings.agentConfigDisabledSkillsMore(remaining))"
    }
}

struct AgentConfigDetailPanel: View {
    @EnvironmentObject private var store: SkillStore

    var body: some View {
        if let snapshot = store.selectedConfigSnapshot {
            AgentConfigSnapshotDetailPanel(snapshot: snapshot)
        } else {
            AgentConfigOverviewDetailPanel(selectedDocument: store.selectedConfigDocument)
        }
    }
}

private struct AgentConfigOverviewDetailPanel: View {
    @EnvironmentObject private var store: SkillStore
    let selectedDocument: ConfigDocumentRecord?

    @State private var draft = ""
    @State private var revealsSensitiveConfig = false
    @State private var isConfirmingConfigEdit = false

    private var validationMessage: String? {
        guard let data = draft.data(using: .utf8) else {
            return UIStrings.settingsInvalidUTF8
        }
        do {
            _ = try JSONSerialization.jsonObject(with: data)
            return nil
        } catch {
            return error.localizedDescription
        }
    }

    private var hasDraftChanges: Bool {
        draft != (store.claudeSettings?.content ?? "")
    }

    private var canAutosaveConfig: Bool {
        revealsSensitiveConfig
            && hasDraftChanges
            && validationMessage == nil
    }

    private var displayedDraft: Binding<String> {
        Binding {
            revealsSensitiveConfig ? draft : ConfigContentRedactor.redactedForDisplay(draft)
        } set: { newValue in
            guard revealsSensitiveConfig else { return }
            draft = newValue
            store.clearSettingsFeedback()
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            if let selectedDocument {
                if isEditableClaudeGlobalDocument(selectedDocument) {
                    claudeCurrentConfigSection
                } else {
                    currentAgentConfigSection(documents: [selectedDocument])
                }
            } else if store.agentFilter == .claudeCode {
                claudeCurrentConfigSection
            } else {
                currentAgentConfigSection(documents: store.currentAgentConfigDocuments)
            }
        }
        .task(id: store.selectedAgentConfigRefreshKey) {
            await store.loadSelectedAgentConfigDataIfNeeded()
            if store.agentFilter == .claudeCode {
                hydrateConfigDraftFromStore()
            }
        }
        .onChange(of: store.claudeSettings) { _ in
            hydrateConfigDraftFromStore(revealsSensitive: revealsSensitiveConfig)
        }
        .onChange(of: store.configAutosaveDraft) { _ in
            hydrateConfigDraftFromStore(revealsSensitive: revealsSensitiveConfig)
        }
        .onChange(of: draft) { _ in
            handleConfigDraftChange()
        }
    }

    private func currentAgentConfigSection(documents: [ConfigDocumentRecord]) -> some View {
        AgentCurrentConfigDocumentsSection(
            documents: documents,
            isLoading: store.isLoadingAgentConfigDocuments,
            errorMessage: store.settingsErrorMessage,
            revealsSensitiveConfig: $revealsSensitiveConfig
        ) {
            Task {
                await store.loadCurrentAgentConfigDocuments(agent: store.agentFilter.rawValue)
            }
        }
    }

    private func isEditableClaudeGlobalDocument(_ document: ConfigDocumentRecord) -> Bool {
        document.agent == SkillAgentFilter.claudeCode.rawValue
            && document.scope.localizedCaseInsensitiveContains("global")
    }

    private var claudeCurrentConfigSection: some View {
        ConfigCodeCard(
            title: UIStrings.currentConfigFile,
            path: store.claudeSettings?.target ?? "~/.claude/settings.json",
            statusText: store.claudeSettings?.exists == true ? UIStrings.existingFile : UIStrings.willCreateFile,
            statusSystemImage: store.claudeSettings?.exists == true ? "doc.text" : "doc.badge.plus",
            sensitiveText: revealsSensitiveConfig ? UIStrings.agentConfigSensitiveValuesVisible : UIStrings.agentConfigSensitiveValuesHidden,
            sensitiveSystemImage: revealsSensitiveConfig ? "eye" : "eye.slash",
            sensitiveColor: revealsSensitiveConfig ? .orange : .secondary
        ) {
            ConfigCodeToolbar(
                isReloadDisabled: store.isLoadingSettings || store.isSavingSettings,
                isFormatDisabled: !revealsSensitiveConfig || validationMessage != nil || draft.isEmpty,
                isRevealDisabled: store.isLoadingSettings || store.isSavingSettings,
                isSensitiveVisible: revealsSensitiveConfig,
                revealHelp: revealsSensitiveConfig ? UIStrings.agentConfigHideSensitive : UIStrings.agentConfigShowSensitive,
                onReload: reloadClaudeConfig,
                onFormat: formatDraftJSON,
                onReveal: toggleSensitiveEditing
            )
        } content: {
            if revealsSensitiveConfig {
                JSONLineNumberedEditor(text: displayedDraft)
                    .frame(minHeight: CGFloat(UIOptimizationPresentation.configEditor.codeCardMinHeight))
                    .agentConfigTextSelection(enabled: true)
            } else {
                JSONSyntaxHighlightedText(content: displayedDraft.wrappedValue)
            }

            if let validationMessage {
                ConfigInlineBanner(message: validationMessage, systemImage: "exclamationmark.triangle.fill", color: .red)
            } else {
                switch store.configAutosavePhase {
                case .saving:
                    ConfigInlineBanner(message: UIStrings.configAutosaveSaving, systemImage: "hourglass", color: .secondary)
                case .debouncing, .pendingAfterSave:
                    ConfigInlineBanner(message: UIStrings.configAutosavePending, systemImage: "clock.arrow.circlepath", color: .secondary)
                case .idle:
                    if canAutosaveConfig {
                        ConfigInlineBanner(message: UIStrings.jsonValidSettingsWrite, systemImage: "checkmark.circle.fill", color: .green)
                    }
                case .failed:
                    EmptyView()
                }
            }

            if let message = store.settingsMessage {
                ConfigInlineBanner(message: message, systemImage: "checkmark.circle.fill", color: .green)
            }

            if let error = store.settingsErrorMessage {
                ConfigInlineBanner(message: error, systemImage: "exclamationmark.triangle.fill", color: .red)
            }
        }
        .confirmationDialog(
            UIStrings.agentConfigEditConfirmationTitle,
            isPresented: $isConfirmingConfigEdit,
            titleVisibility: .visible
        ) {
            Button(UIStrings.agentConfigShowSensitive, role: .destructive) {
                store.clearSettingsFeedback()
                revealsSensitiveConfig = true
            }
            Button(UIStrings.cancel, role: .cancel) {
                isConfirmingConfigEdit = false
            }
        } message: {
            Text(UIStrings.agentConfigEditConfirmationMessage)
        }
    }

    private func hydrateConfigDraftFromStore(revealsSensitive: Bool = false) {
        let transition = ConfigAutosaveDraftReducer.reduce(
            content: draft,
            event: .hydrate(
                storeDraft: store.configAutosaveDraft,
                persistedContent: store.claudeSettings?.content ?? ""
            )
        )
        draft = transition.content
        revealsSensitiveConfig = revealsSensitive
    }

    private func resetDraftFromStore(revealsSensitive: Bool = false) {
        store.cancelPendingConfigAutosave()
        hydrateConfigDraftFromStore(revealsSensitive: revealsSensitive)
    }

    private func reloadClaudeConfig() {
        store.cancelPendingConfigAutosave()
        Task {
            await store.refreshSelectedAgentConfigData()
            resetDraftFromStore()
        }
    }

    private func toggleSensitiveEditing() {
        if revealsSensitiveConfig {
            resetDraftFromStore()
        } else {
            isConfirmingConfigEdit = true
        }
    }

    private func formatDraftJSON() {
        guard revealsSensitiveConfig,
              let formatted = Self.formattedJSON(draft),
              formatted != draft else {
            return
        }
        draft = formatted
        store.clearSettingsFeedback()
    }

    private func handleConfigDraftChange() {
        let transition = ConfigAutosaveDraftReducer.reduce(
            content: draft,
            event: .userChanged(
                storeDraft: store.configAutosaveDraft,
                persistedContent: store.claudeSettings?.content ?? "",
                revealsSensitiveConfig: revealsSensitiveConfig,
                hasActiveSave: store.configAutosaveHasActiveSave,
                validationError: validationMessage
            )
        )
        switch transition.action {
        case .none:
            return
        case .cancelPending:
            store.cancelPendingConfigAutosave()
        case let .submit(content, validationError):
            store.submitConfigAutosave(
                content: content,
                validationError: validationError
            )
        }
    }

    private static func formattedJSON(_ content: String) -> String? {
        guard let data = content.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data),
              JSONSerialization.isValidJSONObject(json),
              let formattedData = try? JSONSerialization.data(
                withJSONObject: json,
                options: [.prettyPrinted, .sortedKeys]
              ),
              let formatted = String(data: formattedData, encoding: .utf8) else {
            return nil
        }
        return formatted + (content.hasSuffix("\n") ? "\n" : "")
    }
}

private struct AgentCurrentConfigDocumentsSection: View {
    let documents: [ConfigDocumentRecord]
    let isLoading: Bool
    let errorMessage: String?
    @Binding var revealsSensitiveConfig: Bool
    let reload: () -> Void

    private var primaryDocument: ConfigDocumentRecord? {
        documents.first
    }

    private var displayedContent: String {
        guard let primaryDocument else {
            return isLoading ? UIStrings.loading : UIStrings.agentConfigNoReadableDocuments
        }
        let content = primaryDocument.content.isEmpty ? UIStrings.emptyPlaceholder : primaryDocument.content
        guard !revealsSensitiveConfig else { return content }
        return ConfigContentRedactor.redactedForDisplay(content)
    }

    var body: some View {
        ConfigCodeCard(
            title: UIStrings.currentConfigFile,
            path: primaryDocument?.target ?? UIStrings.unknown,
            statusText: primaryDocument?.exists == true ? UIStrings.existingFile : UIStrings.willCreateFile,
            statusSystemImage: primaryDocument?.exists == true ? "doc.text" : "doc.badge.plus",
            sensitiveText: revealsSensitiveConfig ? UIStrings.agentConfigSensitiveValuesVisible : UIStrings.agentConfigSensitiveValuesHidden,
            sensitiveSystemImage: revealsSensitiveConfig ? "eye" : "eye.slash",
            sensitiveColor: revealsSensitiveConfig ? .orange : .secondary
        ) {
            ConfigCodeToolbar(
                isReloadDisabled: isLoading,
                isFormatDisabled: true,
                isRevealDisabled: documents.isEmpty,
                isSensitiveVisible: revealsSensitiveConfig,
                revealHelp: documents.isEmpty
                    ? UIStrings.text("agentConfig.noDocumentsHint", "No config documents are loaded.")
                    : (revealsSensitiveConfig ? UIStrings.agentConfigHideSensitive : UIStrings.agentConfigShowSensitiveValues),
                onReload: reload,
                onFormat: {},
                onReveal: { revealsSensitiveConfig.toggle() }
            )
        } content: {
            if let errorMessage {
                ConfigInlineBanner(message: errorMessage, systemImage: "exclamationmark.triangle.fill", color: .red)
            } else if isLoading {
                Label(UIStrings.loading, systemImage: "arrow.clockwise")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            JSONSyntaxHighlightedText(content: displayedContent)
        }
    }
}

private struct AgentConfigSnapshotDetailPanel: View {
    @EnvironmentObject private var store: SkillStore
    let snapshot: ConfigSnapshotRecord

    @State private var preview: SnapshotRollbackPreviewRecord?
    @State private var previewError: String?
    @State private var snapshotToRollback: ConfigSnapshotRecord?

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            VStack(alignment: .leading, spacing: 14) {
                HStack(alignment: .top, spacing: 12) {
                    Image(systemName: "doc.text.magnifyingglass")
                        .font(.title2)
                        .foregroundStyle(.secondary)
                    VStack(alignment: .leading, spacing: 4) {
                        Text(UIStrings.snapshotPreview)
                            .font(.title2.bold())
                        Text(snapshot.reason.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? UIStrings.agentConfigTimelineDefaultAction : snapshot.reason)
                            .font(.headline)
                    }
                    Spacer()
                    Button {
                        loadPreview()
                    } label: {
                        Label(UIStrings.previewDiff, systemImage: "doc.text.magnifyingglass")
                    }
                    .disabled(store.isWriting)

                    Button(role: .destructive) {
                        snapshotToRollback = snapshot
                    } label: {
                        Label(UIStrings.rollback, systemImage: "arrow.uturn.backward")
                    }
                    .disabled(store.isWriting)
                }
            }
            .padding()
            .frame(maxWidth: .infinity, alignment: .leading)
            .nativePanelSurface()

            if let previewError {
                ErrorBanner(message: previewError)
            }

            if let preview {
                VStack(alignment: .leading, spacing: 12) {
                    Label(
                        preview.changed ? UIStrings.currentDiffersFromSnapshot : UIStrings.currentMatchesSnapshot,
                        systemImage: preview.changed ? "exclamationmark.triangle" : "checkmark.circle"
                    )
                    .foregroundStyle(preview.changed ? .orange : .green)

                    if let readError = preview.currentReadError {
                        ErrorBanner(message: readError)
                    }

                    ViewThatFits(in: .horizontal) {
                        HStack(alignment: .top, spacing: 14) {
                            SnapshotTextPane(
                                title: UIStrings.current,
                                content: preview.currentContent.isEmpty ? UIStrings.emptyPlaceholder : preview.currentContent
                            )
                            SnapshotTextPane(
                                title: UIStrings.snapshot,
                                content: preview.snapshot.content.isEmpty ? UIStrings.emptyPlaceholder : preview.snapshot.content
                            )
                        }
                        .frame(minHeight: 420)

                        VStack(alignment: .leading, spacing: 12) {
                            SnapshotTextPane(
                                title: UIStrings.current,
                                content: preview.currentContent.isEmpty ? UIStrings.emptyPlaceholder : preview.currentContent
                            )
                            SnapshotTextPane(
                                title: UIStrings.snapshot,
                                content: preview.snapshot.content.isEmpty ? UIStrings.emptyPlaceholder : preview.snapshot.content
                            )
                        }
                        .frame(minHeight: 360, idealHeight: 460)
                    }
                }
            } else {
                SnapshotTextPane(
                    title: UIStrings.snapshot,
                    content: snapshot.content.isEmpty ? UIStrings.emptyPlaceholder : snapshot.content
                )
                .frame(minHeight: 360)
            }
        }
        .confirmationDialog(
            UIStrings.rollbackSnapshotQuestion,
            isPresented: Binding(
                get: { snapshotToRollback != nil },
                set: { isPresented in
                    if !isPresented {
                        snapshotToRollback = nil
                    }
                }
            ),
            titleVisibility: .visible
        ) {
            Button(UIStrings.rollback, role: .destructive) {
                guard let snapshotID = snapshotToRollback?.id else { return }
                Task { await store.rollbackSnapshot(snapshotID: snapshotID) }
                snapshotToRollback = nil
            }
            Button(UIStrings.cancel, role: .cancel) {
                snapshotToRollback = nil
            }
        } message: {
            Text(UIStrings.agentConfigTimelineRollbackConfirm(
                AgentConfigDisplay.pathSummary(snapshotToRollback?.target ?? "")
            ))
        }
    }

    private func loadPreview() {
        previewError = nil
        Task {
            do {
                preview = try await store.previewRollback(snapshotID: snapshot.id)
            } catch {
                previewError = error.localizedDescription
            }
        }
    }
}

private struct AgentConfigAgentIcon: View {
    let filter: SkillAgentFilter

    var body: some View {
        ZStack {
            if let image = AgentIconProvider.image(for: filter) {
                Image(nsImage: image)
                    .resizable()
                    .scaledToFit()
                    .frame(width: 30, height: 30)
                    .clipShape(RoundedRectangle(cornerRadius: 6))
            } else {
                Image(systemName: "slider.horizontal.3")
                    .font(.title2)
                    .foregroundStyle(Color.accentColor)
            }
        }
        .frame(width: 36, height: 36)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 9))
        .accessibilityLabel(filter.title)
    }
}

private struct ConfigInlineBanner: View {
    let message: String
    let systemImage: String
    let color: Color

    var body: some View {
        Label(message, systemImage: systemImage)
            .font(.caption)
            .foregroundStyle(color)
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(color.opacity(0.08), in: RoundedRectangle(cornerRadius: 8))
    }
}

private struct ConfigCodeCard<Toolbar: View, Content: View>: View {
    let title: String
    let path: String
    let statusText: String?
    let statusSystemImage: String?
    let sensitiveText: String
    let sensitiveSystemImage: String
    let sensitiveColor: Color
    @ViewBuilder let toolbar: () -> Toolbar
    @ViewBuilder let content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .top, spacing: 12) {
                Image(systemName: "curlybraces.square")
                    .font(.title3)
                    .foregroundStyle(Color.accentColor)
                    .frame(width: 32, height: 32)
                    .background(Color.accentColor.opacity(0.10), in: RoundedRectangle(cornerRadius: 8))

                VStack(alignment: .leading, spacing: 4) {
                    Text(title)
                        .font(.headline)
                    PrivacyPathText(path: path, font: .callout, lineLimit: 1)
                }

                Spacer(minLength: 12)

                toolbar()
            }

            HStack(spacing: 10) {
                if let statusText, let statusSystemImage {
                    Label(statusText, systemImage: statusSystemImage)
                        .foregroundStyle(.secondary)
                }

                Label(sensitiveText, systemImage: sensitiveSystemImage)
                    .foregroundStyle(sensitiveColor)
            }
            .font(.caption)

            content()
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

private struct ConfigCodeToolbar: View {
    let isReloadDisabled: Bool
    let isFormatDisabled: Bool
    let isRevealDisabled: Bool
    let isSensitiveVisible: Bool
    let revealHelp: String
    let onReload: () -> Void
    let onFormat: () -> Void
    let onReveal: () -> Void

    var body: some View {
        HStack(spacing: 5) {
            ConfigToolbarIconButton(
                systemImage: "arrow.clockwise",
                label: UIStrings.reload,
                isDisabled: isReloadDisabled,
                action: onReload
            )
            ConfigToolbarIconButton(
                systemImage: "wand.and.sparkles",
                label: UIStrings.formatJSON,
                isDisabled: isFormatDisabled,
                action: onFormat
            )
            ConfigToolbarIconButton(
                systemImage: isSensitiveVisible ? "eye.slash" : "eye",
                label: revealHelp,
                isDisabled: isRevealDisabled,
                action: onReveal
            )
        }
    }
}

private struct ConfigToolbarIconButton: View {
    let systemImage: String
    let label: String
    let isDisabled: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Image(systemName: systemImage)
                .font(.system(size: 13, weight: .semibold))
                .frame(width: 28, height: 28)
                .contentShape(RoundedRectangle(cornerRadius: 7))
        }
        .buttonStyle(.plain)
        .foregroundStyle(isDisabled ? Color.secondary.opacity(0.45) : Color.secondary)
        .background(
            isDisabled ? Color.secondary.opacity(0.04) : Color.secondary.opacity(0.10),
            in: RoundedRectangle(cornerRadius: 7)
        )
        .overlay(
            RoundedRectangle(cornerRadius: 7)
                .stroke(Color.secondary.opacity(0.10), lineWidth: 1)
        )
        .disabled(isDisabled)
        .help(label)
        .accessibilityLabel(label)
    }
}

private struct JSONSyntaxHighlightedText: View {
    let content: String

    var body: some View {
        ScrollView([.vertical, .horizontal]) {
            VStack(alignment: .leading, spacing: 1) {
                ForEach(Array(Self.lines(in: content).enumerated()), id: \.offset) { index, line in
                    HStack(alignment: .firstTextBaseline, spacing: 10) {
                        Text("\(index + 1)")
                            .font(.system(.caption, design: .monospaced))
                            .foregroundStyle(.tertiary)
                            .frame(width: CGFloat(UIOptimizationPresentation.configEditor.lineNumberGutterWidth), alignment: .trailing)
                            .textSelection(.disabled)

                        Text(Self.highlighted(line.isEmpty ? " " : line))
                            .font(.system(.body, design: .monospaced))
                            .textSelection(.enabled)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
            .padding(.vertical, 10)
            .padding(.trailing, 10)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(minHeight: CGFloat(UIOptimizationPresentation.configEditor.codeCardMinHeight))
        .background(Color(nsColor: .textBackgroundColor), in: RoundedRectangle(cornerRadius: 6))
        .overlay(
            RoundedRectangle(cornerRadius: 6)
                .stroke(Color.secondary.opacity(0.12), lineWidth: 1)
        )
    }

    private static func highlighted(_ content: String) -> AttributedString {
        let pattern = #""(?:\\.|[^"\\])*"\s*:|"(?:\\.|[^"\\])*"|\btrue\b|\bfalse\b|\bnull\b|-?\b\d+(?:\.\d+)?(?:[eE][+-]?\d+)?\b"#
        guard let regex = try? NSRegularExpression(pattern: pattern) else {
            return AttributedString(content)
        }

        var attributed = AttributedString(content)
        let fullRange = NSRange(content.startIndex..<content.endIndex, in: content)
        for match in regex.matches(in: content, range: fullRange) {
            guard let stringRange = Range(match.range, in: content),
                  let lower = AttributedString.Index(stringRange.lowerBound, within: attributed),
                  let upper = AttributedString.Index(stringRange.upperBound, within: attributed) else {
                continue
            }

            let token = String(content[stringRange])
            let color = highlightColor(for: token)
            attributed[lower..<upper].foregroundColor = color
        }

        return attributed
    }

    private static func highlightColor(for token: String) -> Color {
        let trimmed = token.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.hasSuffix(":") {
            return .accentColor
        }
        if trimmed == "true" || trimmed == "false" || trimmed == "null" {
            return .purple
        }
        if trimmed.first == "\"" {
            return .green
        }
        return .orange
    }

    private static func lines(in content: String) -> [String] {
        let lines = content.components(separatedBy: .newlines)
        return lines.isEmpty ? [""] : lines
    }
}

private struct JSONLineNumberedEditor: View {
    @Binding var text: String

    private var lineCount: Int {
        max(1, text.components(separatedBy: .newlines).count)
    }

    var body: some View {
        HStack(alignment: .top, spacing: 0) {
            ConfigLineNumberColumn(lineCount: lineCount)

            Divider()
                .opacity(0.35)

            TextEditor(text: $text)
                .font(.system(.body, design: .monospaced))
                .lineSpacing(2)
                .padding(.vertical, 4)
                .padding(.horizontal, 8)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .background(Color(nsColor: .textBackgroundColor), in: RoundedRectangle(cornerRadius: 6))
        .overlay(
            RoundedRectangle(cornerRadius: 6)
                .stroke(Color.secondary.opacity(0.12), lineWidth: 1)
        )
    }
}

private struct ConfigLineNumberColumn: View {
    let lineCount: Int

    var body: some View {
        VStack(alignment: .trailing, spacing: 1) {
            ForEach(1...max(lineCount, 1), id: \.self) { line in
                Text("\(line)")
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.tertiary)
                    .frame(height: 18)
            }
        }
        .padding(.vertical, 10)
        .padding(.horizontal, 8)
        .frame(width: CGFloat(UIOptimizationPresentation.configEditor.lineNumberGutterWidth), alignment: .trailing)
        .background(Color.secondary.opacity(0.05))
        .textSelection(.disabled)
    }
}

private extension View {
    @ViewBuilder
    func agentConfigTextSelection(enabled: Bool) -> some View {
        if enabled {
            textSelection(.enabled)
        } else {
            textSelection(.disabled)
        }
    }
}
