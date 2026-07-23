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
    @State private var configConfirmationToApply: ConfigSaveConfirmation?
    @State private var configPreviewGeneration: UInt64 = 0
    @State private var configPreviewTask: Task<Void, Never>?
    @State private var configBaselineContent = ""
    @State private var configEditGeneration: UInt64 = 0

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
        draft != configBaselineContent
    }

    private var hasWritableConfigBinding: Bool {
        store.supportsConfigActionLifecycle
            && store.claudeSettings?.supportsCompareAndSwap == true
    }

    private var sensitiveTogglePolicy: AgentConfigSensitiveTogglePolicy {
        AgentConfigSensitiveTogglePolicy(
            isSensitiveVisible: revealsSensitiveConfig,
            hasLoadedDocument: store.claudeSettings != nil,
            hasWritableBinding: hasWritableConfigBinding,
            isLoading: store.isLoadingSettings,
            isSaving: store.isSavingSettings
        )
    }

    private var hasConfigConflict: Bool {
        if case .conflict = store.configMutationState {
            return true
        }
        return false
    }

    private var canSaveConfig: Bool {
        revealsSensitiveConfig
            && hasWritableConfigBinding
            && !hasConfigConflict
            && hasDraftChanges
            && validationMessage == nil
            && !store.isSavingSettings
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
            } else if store.visibleConfigDocuments.isEmpty && !store.isLoadingAgentConfigDocuments {
                EmptyState(
                    title: UIStrings.text("agentConfig.noMatchingDocuments", "No matching config documents"),
                    systemImage: "doc.text.magnifyingglass",
                    message: UIStrings.text(
                        "agentConfig.noMatchingDocuments.message",
                        "The selected agent, scope, and search filters do not include a config document."
                    )
                )
            } else if store.agentFilter == .claudeCode {
                claudeCurrentConfigSection
            } else {
                currentAgentConfigSection(documents: store.visibleConfigDocuments)
            }
        }
        .task(id: store.selectedAgentConfigRefreshKey) {
            await store.loadSelectedAgentConfigDataIfNeeded()
            if store.agentFilter == .claudeCode {
                hydrateConfigDraftFromStore()
            }
        }
        .onChange(of: store.claudeSettings) { _ in
            reconcileConfigDraftFromStore(revealsSensitive: revealsSensitiveConfig)
        }
        .onChange(of: store.selectedAgentConfigRefreshKey) { _ in
            revealsSensitiveConfig = false
            hydrateConfigDraftFromStore()
        }
        .onChange(of: selectedDocument?.target) { _ in
            revealsSensitiveConfig = false
            hydrateConfigDraftFromStore()
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
                isRevealDisabled: sensitiveTogglePolicy.isDisabled,
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
            } else if store.claudeSettings != nil && !store.supportsConfigActionLifecycle {
                ConfigInlineBanner(message: UIStrings.configConsistencyProtocolRequired, systemImage: "lock.fill", color: .orange)
            } else if store.claudeSettings != nil && !hasWritableConfigBinding {
                ConfigInlineBanner(message: UIStrings.configRevisionUnavailable, systemImage: "lock.fill", color: .orange)
            } else {
                switch store.configMutationState {
                case .previewing:
                    ConfigInlineBanner(
                        message: UIStrings.text(
                            "settings.agentConfig.previewing",
                            "Preparing a signed config preview..."
                        ),
                        systemImage: "doc.text.magnifyingglass",
                        color: .secondary
                    )
                case .awaitingConfirmation:
                    ConfigInlineBanner(
                        message: UIStrings.text(
                            "settings.agentConfig.awaitingConfirmation",
                            "Review the exact target, impact, revision, and read-back before saving."
                        ),
                        systemImage: "checkmark.shield",
                        color: .orange
                    )
                case .saving:
                    ConfigInlineBanner(
                        message: UIStrings.text(
                            "settings.agentConfig.savingConfirmed",
                            "Applying the confirmed config and verifying read-back..."
                        ),
                        systemImage: "hourglass",
                        color: .secondary
                    )
                case .idle, .conflict, .failed:
                    if canSaveConfig {
                        ConfigInlineBanner(
                            message: UIStrings.text(
                                "settings.agentConfig.dirty",
                                "Unsaved valid changes. Preview them before Save."
                            ),
                            systemImage: "pencil.and.list.clipboard",
                            color: .blue
                        )
                    }
                }
            }

            if revealsSensitiveConfig {
                HStack(spacing: 10) {
                    Spacer()
                    Button(UIStrings.text("settings.agentConfig.revert", "Revert")) {
                        resetDraftFromStore(revealsSensitive: true)
                    }
                    .disabled(!hasDraftChanges || store.isSavingSettings)

                    Button(UIStrings.text("settings.agentConfig.save", "Save")) {
                        previewConfigSave()
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(!canSaveConfig)
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
        .confirmationDialog(
            UIStrings.text("settings.agentConfig.confirmSave", "Confirm Config Save"),
            isPresented: Binding(
                get: { configConfirmationToApply != nil },
                set: { isPresented in
                    if !isPresented {
                        invalidateConfigPreview()
                    }
                }
            ),
            titleVisibility: .visible
        ) {
            Button(UIStrings.text("settings.agentConfig.save", "Save"), role: .destructive) {
                guard let confirmation = configConfirmationToApply else { return }
                guard draft == confirmation.content else {
                    invalidateConfigPreview()
                    return
                }
                let confirmedEditGeneration = configEditGeneration
                configConfirmationToApply = nil
                Task {
                    let succeeded = await store.applyClaudeSettingsSave(confirmation)
                    if succeeded,
                       ConfigDraftReconciliation.shouldHydrateAfterApply(
                           confirmedEditGeneration: confirmedEditGeneration,
                           currentEditGeneration: configEditGeneration,
                           currentDraft: draft,
                           confirmedCandidate: confirmation.content
                       ) {
                        hydrateConfigDraftFromStore(revealsSensitive: true)
                    }
                }
            }
            Button(UIStrings.cancel, role: .cancel) {
                invalidateConfigPreview()
            }
        } message: {
            if let confirmation = configConfirmationToApply {
                Text(configConfirmationSummary(confirmation.preview))
            }
        }
    }

    private func hydrateConfigDraftFromStore(revealsSensitive: Bool = false) {
        invalidateConfigPreview()
        let incoming = store.claudeSettings?.content ?? ""
        configBaselineContent = incoming
        configEditGeneration &+= 1
        draft = incoming
        revealsSensitiveConfig = revealsSensitive
    }

    private func reconcileConfigDraftFromStore(revealsSensitive: Bool) {
        invalidateConfigPreview()
        let incoming = store.claudeSettings?.content ?? ""
        let reconciledDraft = ConfigDraftReconciliation.draft(
            current: draft,
            previousBaseline: configBaselineContent,
            incomingBaseline: incoming,
            force: false
        )
        configBaselineContent = incoming
        draft = reconciledDraft
        revealsSensitiveConfig = revealsSensitive
    }

    private func resetDraftFromStore(revealsSensitive: Bool = false) {
        hydrateConfigDraftFromStore(revealsSensitive: revealsSensitive)
        store.clearSettingsFeedback()
    }

    private func reloadClaudeConfig() {
        invalidateConfigPreview()
        Task {
            await store.refreshSelectedAgentConfigData()
            resetDraftFromStore()
        }
    }

    private func toggleSensitiveEditing() {
        if revealsSensitiveConfig {
            revealsSensitiveConfig = false
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
        configEditGeneration &+= 1
        invalidateConfigPreview()
        if !store.isSavingSettings {
            store.clearSettingsFeedback()
        }
    }

    private func previewConfigSave() {
        guard canSaveConfig else { return }
        invalidateConfigPreview()
        let candidate = draft
        let generation = configPreviewGeneration
        configPreviewTask = Task {
            let confirmation = await store.previewClaudeSettingsSave(content: candidate)
            guard !Task.isCancelled,
                  generation == configPreviewGeneration,
                  draft == candidate,
                  confirmation?.content == candidate else {
                return
            }
            configConfirmationToApply = confirmation
        }
    }

    private func invalidateConfigPreview() {
        configPreviewGeneration &+= 1
        configPreviewTask?.cancel()
        configPreviewTask = nil
        configConfirmationToApply = nil
        store.invalidateConfigSavePreview()
    }

    private func configConfirmationSummary(_ preview: ConfigSavePreviewRecord) -> String {
        let action = preview.action
        let scope = action.target.scope ?? UIStrings.unknown
        let impacts = action.impacts.joined(separator: ", ")
        let readback = action.readback.joined(separator: ", ")
        let evidence = action.evidenceRefs.isEmpty
            ? UIStrings.unknown
            : action.evidenceRefs.joined(separator: "\n")
        return [
            "\(UIStrings.text("action.target", "Target")): \(action.target.id)",
            "\(UIStrings.text("action.scope", "Scope")): \(scope)",
            "\(UIStrings.text("action.impact", "Impact")): \(impacts)",
            "\(UIStrings.text("action.network", "Network")): \(action.network)",
            "\(UIStrings.text("action.revision", "Current revision")): \(preview.currentRevision)",
            "\(UIStrings.text("action.candidateDigest", "Candidate digest")): \(preview.candidateContentDigest)",
            "\(UIStrings.text("action.readback", "Read-back")): \(readback)",
            "\(UIStrings.text("action.evidence", "Evidence")):\n\(evidence)"
        ].joined(separator: "\n")
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

    @State private var previewPresentation = RollbackPreviewPresentationState<SnapshotRollbackPreviewRecord>()
    @State private var previewLoadTask: Task<Void, Never>?
    @State private var confirmationToApply: RollbackConfirmation?
    @State private var revealsSnapshotContent = false

    private var preview: SnapshotRollbackPreviewRecord? {
        previewPresentation.preview
    }

    private var previewError: String? {
        previewPresentation.errorMessage
    }

    private var confirmedPreview: SnapshotRollbackPreviewRecord? {
        guard store.supportsConfigActionLifecycle,
              let preview,
              preview.snapshot.id == snapshot.id,
              preview.rollbackSupported,
              let confirmation = store.rollbackConfirmation,
              confirmation == RollbackConfirmation(preview: preview) else {
            return nil
        }
        return preview
    }

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
                        revealsSnapshotContent.toggle()
                    } label: {
                        Label(
                            revealsSnapshotContent ? UIStrings.agentConfigHideSensitive : UIStrings.agentConfigShowSensitiveValues,
                            systemImage: revealsSnapshotContent ? "eye.slash" : "eye"
                        )
                    }
                    Button {
                        loadPreview()
                    } label: {
                        Label(UIStrings.previewDiff, systemImage: "doc.text.magnifyingglass")
                    }
                    .disabled(store.isWriting)

                    Button(role: .destructive) {
                        guard confirmedPreview != nil,
                              let confirmation = store.rollbackConfirmation else { return }
                        confirmationToApply = confirmation
                    } label: {
                        Label(UIStrings.rollback, systemImage: "arrow.uturn.backward")
                    }
                    .disabled(store.isWriting || confirmedPreview == nil)
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

                    if !store.supportsConfigActionLifecycle {
                        ErrorBanner(message: UIStrings.configConsistencyProtocolRequired)
                    } else if !preview.rollbackSupported {
                        ErrorBanner(message: UIStrings.rollbackBindingUnavailable)
                    }

                    ViewThatFits(in: .horizontal) {
                        HStack(alignment: .top, spacing: 14) {
                            SnapshotTextPane(
                                title: UIStrings.current,
                                content: snapshotDisplayContent(preview.currentContent)
                            )
                            SnapshotTextPane(
                                title: UIStrings.snapshot,
                                content: snapshotDisplayContent(preview.snapshot.content)
                            )
                        }
                        .frame(minHeight: 420)

                        VStack(alignment: .leading, spacing: 12) {
                            SnapshotTextPane(
                                title: UIStrings.current,
                                content: snapshotDisplayContent(preview.currentContent)
                            )
                            SnapshotTextPane(
                                title: UIStrings.snapshot,
                                content: snapshotDisplayContent(preview.snapshot.content)
                            )
                        }
                        .frame(minHeight: 360, idealHeight: 460)
                    }
                }
            } else {
                SnapshotTextPane(
                    title: UIStrings.snapshot,
                    content: snapshotDisplayContent(snapshot.content)
                )
                .frame(minHeight: 360)
            }
        }
        .confirmationDialog(
            UIStrings.rollbackSnapshotQuestion,
            isPresented: Binding(
                get: { confirmationToApply != nil },
                set: { isPresented in
                    if !isPresented {
                        confirmationToApply = nil
                    }
                }
            ),
            titleVisibility: .visible
        ) {
            Button(UIStrings.rollback, role: .destructive) {
                guard let confirmation = confirmationToApply else { return }
                confirmationToApply = nil
                Task {
                    let succeeded = await store.rollbackSnapshot(confirmation: confirmation)
                    guard !succeeded else { return }
                    previewPresentation.replaceWithError(
                        store.errorMessage,
                        selectedSnapshotID: snapshot.id
                    )
                }
            }
            Button(UIStrings.cancel, role: .cancel) {
                confirmationToApply = nil
            }
        } message: {
            if let confirmation = confirmationToApply {
                Text(rollbackConfirmationSummary(confirmation))
            }
        }
        .onChange(of: snapshot.id) { _ in
            revealsSnapshotContent = false
            invalidatePreviewLoad(selectedSnapshotID: snapshot.id)
        }
        .onDisappear {
            invalidatePreviewLoad(selectedSnapshotID: nil)
        }
    }

    private func snapshotDisplayContent(_ content: String) -> String {
        let value = content.isEmpty ? UIStrings.emptyPlaceholder : content
        return revealsSnapshotContent ? value : ConfigContentRedactor.redactedForDisplay(value)
    }

    private func rollbackConfirmationSummary(_ confirmation: RollbackConfirmation) -> String {
        let action = confirmation.action
        let scope = action.target.scope ?? UIStrings.unknown
        let impacts = action.impacts.joined(separator: ", ")
        let readback = action.readback.joined(separator: ", ")
        let evidence = action.evidenceRefs.isEmpty
            ? UIStrings.unknown
            : action.evidenceRefs.joined(separator: "\n")
        return [
            "\(UIStrings.text("action.target", "Target")): \(action.target.id)",
            "\(UIStrings.text("action.scope", "Scope")): \(scope)",
            "\(UIStrings.text("action.impact", "Impact")): \(impacts)",
            "\(UIStrings.text("action.network", "Network")): \(action.network)",
            "\(UIStrings.text("action.revision", "Source revision")): \(action.sourceRevision)",
            "\(UIStrings.text("action.readback", "Read-back")): \(readback)",
            "\(UIStrings.text("action.evidence", "Evidence")):\n\(evidence)"
        ].joined(separator: "\n")
    }

    private func loadPreview() {
        previewLoadTask?.cancel()
        store.clearRollbackConfirmation()
        let request = previewPresentation.begin(snapshotID: snapshot.id)
        confirmationToApply = nil
        previewLoadTask = Task { @MainActor in
            do {
                let loadedPreview = try await store.previewRollback(snapshotID: request.snapshotID)
                guard previewPresentation.publish(preview: loadedPreview, for: request) else { return }
                previewLoadTask = nil
            } catch is CancellationError {
                guard previewPresentation.activeRequest == request else { return }
                previewPresentation.invalidate(selectedSnapshotID: request.snapshotID)
                previewLoadTask = nil
            } catch {
                guard previewPresentation.publish(errorMessage: error.localizedDescription, for: request) else { return }
                previewLoadTask = nil
            }
        }
    }

    private func invalidatePreviewLoad(selectedSnapshotID: String?) {
        previewLoadTask?.cancel()
        previewLoadTask = nil
        previewPresentation.invalidate(selectedSnapshotID: selectedSnapshotID)
        confirmationToApply = nil
        store.clearRollbackConfirmation()
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
