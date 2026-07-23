import AppKit
import Foundation
import SwiftUI

enum SettingsTab: String, CaseIterable, Identifiable, Hashable {
    case appearance
    case provider
    case providerObservability
    case service

    var id: String { rawValue }

    var title: String {
        switch self {
        case .appearance:
            return UIStrings.appearanceSettings
        case .provider:
            return UIStrings.aiProviderSettings
        case .providerObservability:
            return UIStrings.providerObservabilityTitle
        case .service:
            return UIStrings.service
        }
    }

    var subtitle: String {
        switch self {
        case .appearance:
            return UIStrings.settingsNavAppearanceSubtitle
        case .provider:
            return UIStrings.settingsNavProviderSubtitle
        case .providerObservability:
            return UIStrings.settingsNavObservabilitySubtitle
        case .service:
            return UIStrings.settingsNavServiceSubtitle
        }
    }

    var systemImage: String {
        switch self {
        case .appearance:
            return "paintpalette"
        case .provider:
            return "key"
        case .providerObservability:
            return "waveform.path.ecg.rectangle"
        case .service:
            return "wrench.and.screwdriver"
        }
    }
}

struct SettingsView: View {
    @EnvironmentObject private var store: SkillStore
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @AppStorage(AppLanguage.storageKey) private var appLanguageRawValue = AppLanguage.defaultLanguage.rawValue
    @AppStorage(AppTheme.storageKey) private var appThemeRawValue = AppTheme.defaultTheme.rawValue
    @AppStorage(DisplayText.screenshotPrivacyModeStorageKey) private var screenshotPrivacyModeEnabled = true
    @State private var providerDraft = AIProviderSettingsDraft(status: .unavailable())
    @State private var hasEditedProviderDraft = false
    @State private var showsServiceDiagnostics = false
    @AppStorage(SettingsNavigation.selectionStorageKey)
    private var selectedSettingsTab: SettingsTab = .appearance

    private var providerValidationMessage: String? {
        providerDraft.validationMessage
    }

    private var providerActionsDisabled: Bool {
        store.isLoadingAIProvider || store.isSavingAIProvider || store.isTestingAIProvider || !store.aiProviderStatus.serviceAvailable
    }

    private var canTestProvider: Bool {
        providerValidationMessage == nil
            && !providerActionsDisabled
            && !hasEditedProviderDraft
            && store.aiProviderStatus.activeProfile != nil
    }

    private var selectedLanguage: Binding<AppLanguage> {
        Binding {
            AppLanguage.fromStorage(appLanguageRawValue)
        } set: { language in
            appLanguageRawValue = language.rawValue
            UIStrings.use(language)
        }
    }

    private var selectedTheme: Binding<AppTheme> {
        Binding {
            AppTheme.fromStorage(appThemeRawValue)
        } set: { theme in
            appThemeRawValue = theme.rawValue
            MainWindowCoordinator.applyAppearance(theme)
        }
    }

    var body: some View {
        HStack(spacing: 0) {
            settingsSidebar

            Divider()

            selectedSettingsPane
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .frame(
            minWidth: CGFloat(UIOptimizationPresentation.settings.minimumWidth),
            idealWidth: CGFloat(UIOptimizationPresentation.settings.idealWidth),
            minHeight: CGFloat(UIOptimizationPresentation.settings.minimumHeight),
            idealHeight: CGFloat(UIOptimizationPresentation.settings.idealHeight)
        )
        .background(SettingsWindowConfigurator(theme: AppTheme.fromStorage(appThemeRawValue)))
        .task(id: selectedSettingsTab) {
            switch selectedSettingsTab {
            case .appearance, .service:
                break
            case .provider:
                await store.loadAIProviderStatusIfNeeded()
                hydrateProviderDraftFromStore()
            case .providerObservability:
                break
            }
        }
        .onChange(of: store.aiProviderStatus) { _ in
            if !hasEditedProviderDraft {
                hydrateProviderDraftFromStore()
            }
        }
        .onChange(of: providerDraft) { _ in
            handleProviderDraftChange()
        }
        .onReceive(
            NotificationCenter.default.publisher(
                for: SettingsNavigation.providerObservabilityRequested
            )
        ) { _ in
            selectedSettingsTab = .providerObservability
        }
        .transaction { transaction in
            if reduceMotion {
                transaction.animation = nil
            }
        }
        .onExitCommand {
            NSApp.keyWindow?.close()
        }
    }

    private var settingsSidebar: some View {
        VStack(alignment: .leading, spacing: 14) {
            VStack(alignment: .leading, spacing: 4) {
                Text(UIStrings.settingsWindowTitle)
                    .font(.headline)
                Text(UIStrings.settingsSidebarSubtitle)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .padding(.horizontal, 14)
            .padding(.top, 18)

            VStack(spacing: 4) {
                ForEach(SettingsTab.allCases) { tab in
                    SettingsSidebarItem(
                        tab: tab,
                        isSelected: selectedSettingsTab == tab
                    ) {
                        selectedSettingsTab = tab
                    }
                }
            }
            .padding(.horizontal, 10)

            Spacer(minLength: 0)
        }
        .frame(width: CGFloat(UIOptimizationPresentation.settings.sidebarWidth))
        .frame(maxHeight: .infinity)
        .background(.bar)
    }

    @ViewBuilder
    private var selectedSettingsPane: some View {
        switch selectedSettingsTab {
        case .appearance:
            appearanceSection
        case .provider:
            providerSection
        case .providerObservability:
            ProviderObservabilitySettingsPanel()
        case .service:
            serviceSection
        }
    }

    private var appearanceSection: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                SettingsPageHeader(
                    title: UIStrings.appearanceSettings,
                    systemImage: "paintpalette",
                    boundary: UIStrings.appearanceBoundary,
                    badge: UIStrings.text("settings.localOnly", "App-local")
                )

                SettingsSectionCard(
                    title: UIStrings.themeSettings,
                    systemImage: "circle.lefthalf.filled"
                ) {
                    Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 12) {
                        GridRow {
                            Text(UIStrings.themeSelection)
                                .foregroundStyle(.secondary)
                            Picker(UIStrings.themeSelection, selection: selectedTheme) {
                                ForEach(AppTheme.allCases) { theme in
                                    Text(theme.title).tag(theme)
                                }
                            }
                            .pickerStyle(.segmented)
                            .labelsHidden()
                            .frame(width: 360)
                        }
                    }
                }

                SettingsSectionCard(
                    title: UIStrings.text("settings.preferences", "Preferences"),
                    systemImage: "slider.horizontal.3"
                ) {
                    Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 12) {
                        GridRow {
                            Text(UIStrings.languageSelection)
                                .foregroundStyle(.secondary)
                            Picker(UIStrings.languageSelection, selection: selectedLanguage) {
                                ForEach(AppLanguage.allCases) { language in
                                    Text(language.title).tag(language)
                                }
                            }
                            .pickerStyle(.segmented)
                            .labelsHidden()
                            .frame(width: 280)
                        }

                        GridRow {
                            Text(UIStrings.privacyScreenshotMode)
                                .foregroundStyle(.secondary)
                            Toggle(UIStrings.privacyScreenshotMode, isOn: $screenshotPrivacyModeEnabled)
                                .toggleStyle(.switch)
                                .labelsHidden()
                        }
                    }

                    Text(UIStrings.privacyScreenshotBoundary)
                        .font(.callout)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }

                SettingsBanner(message: UIStrings.appearanceAppliesImmediately, systemImage: "checkmark.circle.fill", color: .green)

                Spacer(minLength: 0)
            }
            .padding(20)
        }
    }

    private var providerSection: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                SettingsPageHeader(
                    title: UIStrings.aiProviderSettings,
                    systemImage: "key",
                    boundary: UIStrings.aiProviderBoundary,
                    badge: store.aiProviderStatus.configured ? UIStrings.aiProviderConfigured : UIStrings.aiProviderUnconfigured,
                    badgeSystemImage: store.aiProviderStatus.configured ? "checkmark.circle" : "circle.dashed",
                    badgeTint: store.aiProviderStatus.configured ? .green : .secondary
                )

                if !store.aiProviderStatus.serviceAvailable {
                    SettingsBanner(
                        message: UIStrings.localizedServiceMessage(store.aiProviderStatus.disabledReason ?? UIStrings.aiProviderUnavailable),
                        systemImage: "exclamationmark.triangle.fill",
                        color: .orange
                    )
                }

                SettingsSectionCard(title: UIStrings.text("settings.aiProvider.connection", "Connection"), systemImage: "network") {
                    providerConnectionForm
                }

                SettingsSectionCard(title: UIStrings.text("settings.aiProvider.limits", "Limits"), systemImage: "gauge.with.dots.needle.67percent") {
                    providerLimitsForm
                }

                SettingsSectionCard(title: UIStrings.text("settings.aiProvider.credentialSafety", "Credential Safety"), systemImage: "lock.shield") {
                    providerCredentialSafety
                }

                if let providerValidationMessage, hasEditedProviderDraft {
                    SettingsBanner(message: providerValidationMessage, systemImage: "exclamationmark.triangle.fill", color: .red)
                }

                if let message = store.aiProviderMessage {
                    SettingsBanner(message: UIStrings.localizedServiceMessage(message), systemImage: "checkmark.circle.fill", color: .green)
                }

                if let error = store.aiProviderErrorMessage {
                    SettingsBanner(message: UIStrings.localizedServiceMessage(error), systemImage: "exclamationmark.triangle.fill", color: .red)
                }

                if store.isTestingAIProvider {
                    SettingsBanner(message: UIStrings.aiProviderTesting, systemImage: "network", color: .secondary)
                } else if store.isSavingAIProvider {
                    SettingsBanner(message: UIStrings.aiProviderSaving, systemImage: "hourglass", color: .secondary)
                } else if hasEditedProviderDraft, providerValidationMessage == nil {
                    SettingsBanner(
                        message: UIStrings.text(
                            "settings.aiProvider.unsaved",
                            "Unsaved changes. Review a signed preview before saving."
                        ),
                        systemImage: "pencil.and.list.clipboard",
                        color: .secondary
                    )
                }

                SettingsSectionCard(title: UIStrings.text("settings.actions", "Actions"), systemImage: "command") {
                    providerActions
                }

                if let preview = store.aiProviderActionPreview,
                   let pendingAction = store.aiProviderPendingAction {
                    SettingsSectionCard(
                        title: UIStrings.text(
                            "settings.aiProvider.signedPreview",
                            "Signed Action Preview"
                        ),
                        systemImage: "checkmark.shield"
                    ) {
                        providerActionPreview(preview, pendingAction: pendingAction)
                    }
                }

                if let result = store.aiProviderTestResult {
                    SettingsSectionCard(title: UIStrings.aiProviderTestResult, systemImage: "checkmark.seal") {
                        providerTestResult(result)
                    }
                }

                Spacer(minLength: 0)
            }
            .padding(20)
        }
    }

    private var providerConnectionForm: some View {
        Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 12) {
            GridRow {
                Text(UIStrings.llmProvider)
                    .foregroundStyle(.secondary)
                Picker(UIStrings.llmProvider, selection: $providerDraft.kind) {
                    ForEach([AIProviderKind.openAICompatible, .claudeCompatible]) { kind in
                        Text(kind.title).tag(kind)
                    }
                }
                .pickerStyle(.segmented)
                .labelsHidden()
            }

            GridRow {
                Text(UIStrings.aiProviderEndpoint)
                    .foregroundStyle(.secondary)
                TextField(UIStrings.aiProviderEndpointPlaceholder, text: $providerDraft.endpoint)
                    .textFieldStyle(.roundedBorder)
            }

            GridRow {
                Text(UIStrings.aiProviderModel)
                    .foregroundStyle(.secondary)
                TextField(UIStrings.aiProviderModelPlaceholder, text: $providerDraft.model)
                    .textFieldStyle(.roundedBorder)
            }

            GridRow {
                Text(UIStrings.aiProviderAPIVersion)
                    .foregroundStyle(.secondary)
                TextField(UIStrings.aiProviderOptionalPlaceholder, text: $providerDraft.apiVersion)
                    .textFieldStyle(.roundedBorder)
            }

            GridRow {
                Text(UIStrings.aiProviderAPIKey)
                    .foregroundStyle(.secondary)
                SecureField(UIStrings.aiProviderAPIKeyPlaceholder, text: $providerDraft.apiKey)
                    .textFieldStyle(.roundedBorder)
            }
        }
    }

    private var providerLimitsForm: some View {
        Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 12) {
            GridRow {
                Text(UIStrings.aiProviderMonthlyBudget)
                    .foregroundStyle(.secondary)
                VStack(alignment: .leading, spacing: 4) {
                    TextField(UIStrings.aiProviderMonthlyBudgetPlaceholder, text: $providerDraft.monthlyBudgetUSD)
                        .textFieldStyle(.roundedBorder)
                    Text(UIStrings.aiProviderMonthlyBudgetHelp)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }

            GridRow {
                Text(UIStrings.aiProviderTokenLimit)
                    .foregroundStyle(.secondary)
                VStack(alignment: .leading, spacing: 4) {
                    TextField(UIStrings.aiProviderTokenLimitPlaceholder, text: $providerDraft.singleRequestTokenLimit)
                        .textFieldStyle(.roundedBorder)
                    Text(UIStrings.aiProviderTokenLimitHelp)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
        }
    }

    private var providerCredentialSafety: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(UIStrings.aiProviderKeychainFirst)
                .font(.callout)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 8) {
                SettingsMetadataRow(label: UIStrings.aiProviderStorage, value: store.aiProviderStatus.credentialStorage ?? UIStrings.notLoaded)
                SettingsMetadataRow(label: UIStrings.llmEnabled, value: store.aiProviderStatus.enabled ? UIStrings.llmEnabled : UIStrings.llmDisabled)
                if let disabledReason = store.aiProviderStatus.disabledReason, !disabledReason.isEmpty {
                    SettingsMetadataRow(label: UIStrings.aiProviderDisabledReason, value: UIStrings.localizedServiceMessage(disabledReason))
                }
            }
        }
    }

    private var providerActions: some View {
        HStack(spacing: 10) {
            Button {
                Task {
                    store.cancelAIProviderActionPreview()
                    await store.loadAIProviderStatus()
                    resetProviderDraftFromStore()
                }
            } label: {
                Label(UIStrings.reload, systemImage: "arrow.clockwise")
            }
            .disabled(store.isLoadingAIProvider || store.isSavingAIProvider || store.isTestingAIProvider)

            Button(role: .destructive) {
                Task {
                    await store.previewDeleteAIProviderSettings()
                }
            } label: {
                Label(
                    UIStrings.text("settings.aiProvider.delete", "Delete"),
                    systemImage: "trash"
                )
            }
            .disabled(providerActionsDisabled || store.aiProviderStatus.activeProfile == nil)

            Spacer()

            Button {
                Task {
                    await store.previewAIProviderConnectionTest()
                }
            } label: {
                Label(UIStrings.aiProviderTest, systemImage: "network")
            }
            .disabled(!canTestProvider)

            Button {
                Task {
                    await store.previewSaveAIProviderSettings(draft: providerDraft)
                }
            } label: {
                Label(
                    UIStrings.text("common.save", "Save"),
                    systemImage: "square.and.arrow.down"
                )
            }
            .buttonStyle(.borderedProminent)
            .disabled(
                providerActionsDisabled
                    || providerValidationMessage != nil
                    || !hasEditedProviderDraft
            )
        }
    }

    private func providerActionPreview(
        _ preview: AIProviderActionPreview,
        pendingAction: AIProviderPendingAction
    ) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 8) {
                SettingsMetadataRow(
                    label: UIStrings.text("action.target", "Target"),
                    value: "\(preview.action.target.kind): \(preview.action.target.id)"
                )
                SettingsMetadataRow(
                    label: UIStrings.text("action.scope", "Scope"),
                    value: preview.action.target.scope
                        ?? UIStrings.text("action.scope.providerGlobal", "Provider global")
                )
                SettingsMetadataRow(
                    label: UIStrings.text("action.impact", "Impact"),
                    value: preview.action.impacts.joined(separator: ", ")
                )
                SettingsMetadataRow(
                    label: UIStrings.text("action.network", "Network"),
                    value: preview.action.network
                )
                SettingsMetadataRow(
                    label: UIStrings.aiProviderEndpoint,
                    value: preview.destinationHost
                )
                SettingsMetadataRow(label: UIStrings.aiProviderModel, value: preview.model)
                SettingsMetadataRow(
                    label: UIStrings.text("action.expectedRevision", "Expected revision"),
                    value: preview.expectedRevision
                )
                SettingsMetadataRow(
                    label: UIStrings.text("action.readback", "Read-back"),
                    value: preview.action.readback.joined(separator: ", ")
                )
                SettingsMetadataRow(
                    label: UIStrings.text("action.evidence", "Evidence"),
                    value: preview.action.evidenceRefs.joined(separator: ", ")
                )
            }

            Text(
                UIStrings.text(
                    "settings.aiProvider.previewBoundary",
                    "Confirming applies only this signed target and revision. A changed target or profile is rejected; the signing token is never displayed."
                )
            )
            .font(.caption)
            .foregroundStyle(.secondary)
            .fixedSize(horizontal: false, vertical: true)

            HStack {
                Button(UIStrings.cancel, role: .cancel) {
                    store.cancelAIProviderActionPreview()
                }
                Spacer()
                Button(role: pendingAction == .delete ? .destructive : nil) {
                    confirmProviderAction(pendingAction)
                } label: {
                    Label(
                        providerConfirmLabel(pendingAction),
                        systemImage: pendingAction == .test ? "network" : "checkmark.shield"
                    )
                }
                .buttonStyle(.borderedProminent)
                .disabled(store.isSavingAIProvider || store.isTestingAIProvider)
            }
        }
    }

    private func providerConfirmLabel(_ action: AIProviderPendingAction) -> String {
        switch action {
        case .save:
            return UIStrings.text("settings.aiProvider.confirmSave", "Confirm Save")
        case .delete:
            return UIStrings.text("settings.aiProvider.confirmDelete", "Confirm Delete")
        case .test:
            return UIStrings.text("settings.aiProvider.confirmTest", "Confirm Test")
        }
    }

    private func confirmProviderAction(_ action: AIProviderPendingAction) {
        Task {
            switch action {
            case .save:
                if await store.confirmSaveAIProviderSettings(draft: providerDraft) {
                    providerDraft = AIProviderSettingsDraft(status: store.aiProviderStatus)
                    hasEditedProviderDraft = false
                }
            case .delete:
                if await store.confirmDeleteAIProviderSettings() {
                    providerDraft = AIProviderSettingsDraft(status: store.aiProviderStatus)
                    hasEditedProviderDraft = false
                }
            case .test:
                _ = await store.confirmAIProviderConnectionTest()
            }
        }
    }

    private func providerTestResult(_ result: AIProviderTestResult) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Label(UIStrings.localizedServiceMessage(result.message), systemImage: result.success ? "checkmark.circle.fill" : "exclamationmark.triangle.fill")
                .font(.headline)
                .foregroundStyle(result.success ? .green : .orange)

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 8) {
                SettingsMetadataRow(label: UIStrings.aiProviderTestResult, value: result.status)
                if let audit = result.audit {
                    SettingsMetadataRow(label: UIStrings.aiProviderAuditMetadata, value: audit.auditID ?? UIStrings.unknown)
                    SettingsMetadataRow(label: UIStrings.llmProvider, value: audit.provider ?? UIStrings.unknown)
                    SettingsMetadataRow(label: UIStrings.llmModel, value: audit.model ?? UIStrings.unknown)
                    SettingsMetadataRow(label: UIStrings.aiProviderEndpoint, value: audit.endpoint ?? UIStrings.unknown)
                    SettingsMetadataRow(label: UIStrings.aiProviderAuditDuration, value: audit.durationMS.map { "\($0) ms" } ?? UIStrings.unknown)
                    SettingsMetadataRow(label: UIStrings.aiProviderAuditRedaction, value: audit.redactionApplied ? UIStrings.aiProviderAuditApplied : UIStrings.aiProviderAuditNotApplied)
                    SettingsMetadataRow(label: UIStrings.aiProviderAuditPromptStored, value: audit.promptStored ? UIStrings.aiProviderAuditStored : UIStrings.aiProviderAuditNotStored)
                    SettingsMetadataRow(label: UIStrings.aiProviderAuditResponseStored, value: audit.responseStored ? UIStrings.aiProviderAuditStored : UIStrings.aiProviderAuditNotStored)
                    if let input = audit.inputTokens, let output = audit.outputTokens {
                        SettingsMetadataRow(label: UIStrings.llmTokens, value: "\(input) in / \(output) out")
                    }
                    if let cost = audit.estimatedCostUSD {
                        SettingsMetadataRow(label: UIStrings.llmCost, value: String(format: "$%.4f", cost))
                    }
                    if let errorCode = audit.errorCode {
                        SettingsMetadataRow(label: UIStrings.aiProviderAuditErrorCode, value: errorCode)
                    }
                } else {
                    SettingsMetadataRow(label: UIStrings.aiProviderAuditMetadata, value: UIStrings.aiProviderNoAudit)
                }
            }
        }
        .padding(.top, 4)
    }

    private var serviceSection: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                SettingsPageHeader(
                    title: UIStrings.service,
                    systemImage: "wrench.and.screwdriver",
                    boundary: UIStrings.text(
                        "settings.service.boundary",
                        "Review local sidecar health and privacy-safe diagnostics. This page does not write configuration or call providers."
                    ),
                    badge: UIStrings.text("settings.advanced", "Advanced"),
                    badgeSystemImage: "gearshape"
                )

                DetailMetricGrid(maxColumns: 3, minColumnWidth: 150) {
                    SummaryChip(title: UIStrings.version, value: store.status?.version ?? UIStrings.unknown, systemImage: "number")
                    SummaryChip(title: UIStrings.protocolLabel, value: "\(store.status?.protocolVersion ?? 0)", systemImage: "point.3.connected.trianglepath.dotted")
                    SummaryChip(title: UIStrings.methods, value: "\(store.status?.supportedMethods.count ?? 0)", systemImage: "list.bullet.rectangle")
                }

                DisclosureGroup(isExpanded: $showsServiceDiagnostics) {
                    Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 8) {
                        SettingsMetadataRow(label: UIStrings.version, value: store.status?.version ?? UIStrings.unknown)
                        SettingsMetadataRow(label: UIStrings.protocolLabel, value: "\(store.status?.protocolVersion ?? 0)")
                        PrivacyPathRow(label: UIStrings.catalog, path: store.status?.catalogPath ?? UIStrings.notLoaded)
                        PrivacyPathRow(label: UIStrings.userHome, path: store.status?.userHome ?? UIStrings.unknown)
                        SettingsMetadataRow(label: UIStrings.methods, value: "\(store.status?.supportedMethods.count ?? 0)")
                    }
                    .padding(.top, 8)
                } label: {
                    Label(UIStrings.text("settings.serviceDiagnostics", "Service Diagnostics"), systemImage: "wrench.and.screwdriver")
                        .font(.headline)
                }
                .padding(12)
                .frame(maxWidth: .infinity, alignment: .leading)
                .nativePanelSurface()
                Spacer(minLength: 0)
            }
            .padding(20)
        }
    }

    private func hydrateProviderDraftFromStore() {
        providerDraft = AIProviderSettingsDraft(status: store.aiProviderStatus)
        hasEditedProviderDraft = false
    }

    private func resetProviderDraftFromStore() {
        store.cancelAIProviderActionPreview()
        providerDraft = AIProviderSettingsDraft(status: store.aiProviderStatus)
        hasEditedProviderDraft = false
    }

    private func resetProviderEditedState() {
        hasEditedProviderDraft = providerDraft != AIProviderSettingsDraft(status: store.aiProviderStatus)
    }

    private func handleProviderDraftChange() {
        resetProviderEditedState()
        store.cancelAIProviderActionPreview()
    }
}

private struct SettingsMetadataRow: View {
    let label: String
    let value: String

    var body: some View {
        GridRow {
            Text(label)
                .foregroundStyle(.secondary)
            Text(value)
                .textSelection(.enabled)
                .lineLimit(2)
        }
    }
}

private struct SettingsSidebarItem: View {
    let tab: SettingsTab
    let isSelected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 10) {
                Image(systemName: tab.systemImage)
                    .font(.system(size: 15, weight: .semibold))
                    .foregroundStyle(isSelected ? Color.accentColor : Color.secondary)
                    .frame(width: 18)

                VStack(alignment: .leading, spacing: 2) {
                    Text(tab.title)
                        .font(.callout.weight(isSelected ? .semibold : .regular))
                        .foregroundStyle(.primary)
                        .lineLimit(1)

                    Text(tab.subtitle)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }

                Spacer(minLength: 0)
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background {
                if isSelected {
                    RoundedRectangle(cornerRadius: CGFloat(UIOptimizationPresentation.settings.sectionCornerRadius))
                        .fill(Color.agentCopilotPanelBackground)
                }
            }
        }
        .buttonStyle(.plain)
        .accessibilityLabel(tab.title)
    }
}

private struct SettingsPageHeader: View {
    let title: String
    let systemImage: String
    let boundary: String
    var badge: String? = nil
    var badgeSystemImage = "lock.shield"
    var badgeTint: Color = .secondary

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline, spacing: 10) {
                Label(title, systemImage: systemImage)
                    .font(.headline)
                Spacer()
                if let badge {
                    Label(badge, systemImage: badgeSystemImage)
                        .font(.caption.bold())
                        .foregroundStyle(badgeTint)
                }
            }
            Text(boundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

private struct SettingsWindowConfigurator: NSViewRepresentable {
    let theme: AppTheme

    func makeNSView(context: Context) -> NSView {
        let view = NSView(frame: .zero)
        view.isHidden = true
        configure(windowFor: view)
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        configure(windowFor: nsView)
    }

    private func configure(windowFor view: NSView) {
        DispatchQueue.main.async {
            guard let window = view.window else { return }
            window.appearance = theme.nsAppearance
            window.title = UIStrings.settingsWindowTitle
            window.minSize = NSSize(
                width: CGFloat(UIOptimizationPresentation.settings.minimumWidth),
                height: CGFloat(UIOptimizationPresentation.settings.minimumHeight)
            )
            window.titlebarAppearsTransparent = true
            window.toolbarStyle = .unifiedCompact
            window.styleMask.remove(.miniaturizable)
            window.standardWindowButton(.miniaturizeButton)?.isEnabled = false
            window.standardWindowButton(.miniaturizeButton)?.isHidden = true
            window.standardWindowButton(.zoomButton)?.isEnabled = false
            window.standardWindowButton(.zoomButton)?.isHidden = true
        }
    }
}

private struct SettingsSectionCard<Content: View>: View {
    let title: String
    let systemImage: String
    @ViewBuilder let content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Label(title, systemImage: systemImage)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            content()
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

private struct SettingsBanner: View {
    let message: String
    let systemImage: String
    let color: Color

    var body: some View {
        Label(message, systemImage: systemImage)
            .foregroundStyle(color)
            .padding(.vertical, 10)
            .padding(.horizontal, 12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .nativePanelSurface()
            .overlay(alignment: .leading) {
                Rectangle()
                    .fill(color)
                    .frame(width: 3)
                    .clipShape(Capsule())
            }
    }
}
