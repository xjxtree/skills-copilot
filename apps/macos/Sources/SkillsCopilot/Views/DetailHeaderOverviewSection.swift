import AppKit
import SwiftUI

struct HeaderView: View {
    let skill: SkillRecord
    let adoptingAgentSummary: String
    let sessionUsage: LocalSessionSkillUsageRow?
    let issueCount: Int
    let isWriting: Bool
    let adapterCapability: AdapterCapabilityRecord?
    let onSelectSection: (DetailSection) -> Void
    let onToggle: (Bool) -> Void

    var body: some View {
        let disabledReason = toggleDisabledReason
        let isEffectivelyEnabled = DisplayText.statusKind(skill.state, enabled: skill.enabled) == .enabled

        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .center, spacing: 12) {
                Text(skill.name)
                    .font(.title2.bold())
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .help(skill.name)

                Spacer(minLength: 12)

                statusTag(isEffectivelyEnabled: isEffectivelyEnabled)

                Menu {
                    Button(role: isEffectivelyEnabled ? .destructive : nil) {
                        onToggle(!isEffectivelyEnabled)
                    } label: {
                        Label(
                            isEffectivelyEnabled ? UIStrings.disable : UIStrings.enable,
                            systemImage: isEffectivelyEnabled ? "pause.circle" : "play.circle"
                        )
                    }
                    .disabled(disabledReason != nil)
                } label: {
                    Image(systemName: "ellipsis.circle")
                }
                .menuStyle(.borderlessButton)
                .menuIndicator(.hidden)
                .help(disabledReason ?? UIStrings.text("detail.actions", "Skill actions"))
                .accessibilityLabel(UIStrings.text("detail.actions", "Skill actions"))
                .accessibilityHint(disabledReason ?? "")
            }
            .frame(height: CGFloat(UIOptimizationPresentation.detailHeader.height), alignment: .center)

            CompactMetadataGrid(rows: headerMetadataRows)

            if let disabledReason {
                Label(disabledReason, systemImage: "lock.fill")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else if isGuardedToggleAvailable {
                Label(guardedToggleBoundary, systemImage: "shield")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            headerMetrics
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }

    private var headerMetrics: some View {
        ViewThatFits(in: .horizontal) {
            HStack(alignment: .top, spacing: 10) {
                if let sessionUsage {
                    sessionUsageChip(sessionUsage)
                        .frame(width: 220)
                }
                SummaryChip(title: UIStrings.scope, value: DisplayText.scope(for: skill), systemImage: "folder")
                    .frame(width: 220)
                issueBadge
                    .frame(width: 220)
            }

            VStack(alignment: .leading, spacing: 10) {
                DetailMetricGrid(maxColumns: 2, minColumnWidth: 190) {
                    if let sessionUsage {
                        sessionUsageChip(sessionUsage)
                    }
                    SummaryChip(title: UIStrings.scope, value: DisplayText.scope(for: skill), systemImage: "folder")
                    issueBadge
                }
            }
        }
    }

    private var issueBadge: some View {
        CountBadge(
            label: UIStrings.text("detail.issueGroups", "Issue groups"),
            value: issueCount,
            systemImage: "exclamationmark.triangle",
            tint: .orange,
            action: { onSelectSection(.findings) }
        )
    }

    private var headerMetadataRows: [CompactMetadataRow] {
        [
            CompactMetadataRow(
                label: UIStrings.text("detail.adoptingAgents.short", "Agents"),
                value: adoptingAgentSummary,
                systemImage: "person.2"
            )
        ]
    }

    private func statusTag(isEffectivelyEnabled: Bool) -> some View {
        Label(
            DisplayText.isToolGlobal(skill) ? UIStrings.readOnlyPreview : DisplayText.state(skill.state, enabled: skill.enabled),
            systemImage: DisplayText.isToolGlobal(skill) ? "eye" : DisplayText.stateSystemImage(skill.state, enabled: skill.enabled)
        )
        .font(.caption.bold())
        .lineLimit(1)
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(statusBackground(isEffectivelyEnabled: isEffectivelyEnabled), in: Capsule())
        .foregroundStyle(statusForeground)
    }

    private func statusBackground(isEffectivelyEnabled: Bool) -> Color {
        if DisplayText.isToolGlobal(skill) {
            return Color.secondary.opacity(0.12)
        }
        if isEffectivelyEnabled {
            return Color.green.opacity(0.12)
        }
        return Color.secondary.opacity(0.12)
    }

    private var statusForeground: Color {
        DisplayText.isToolGlobal(skill) ? .secondary : DisplayText.stateColor(skill.state, enabled: skill.enabled)
    }

    private func sessionUsageChip(_ row: LocalSessionSkillUsageRow) -> some View {
        SummaryChip(
            title: UIStrings.text("detail.sessionSkillUsage", "Session calls"),
            value: "\(row.callCount) \(UIStrings.text("detail.sessionSkillUsage.calls", "calls")) · \(row.sessionCount) \(UIStrings.text("detail.sessionSkillUsage.sessions", "sessions"))",
            systemImage: "bubble.left.and.text.bubble.right"
        )
    }

    private var toggleDisabledReason: String? {
        if let catalogReason = DisplayText.catalogToggleDisabledReason(for: skill, isWriting: isWriting) {
            return catalogReason
        }
        guard !isWriting else {
            return UIStrings.toggleUnavailableBusy
        }
        guard let adapterCapability else {
            if DisplayText.isReadOnlyAdapter(skill.agent) {
                return UIStrings.toggleUnavailableReadOnlyAdapter(DisplayText.agent(skill.agent))
            }
            return DisplayText.requiresGuardedToggleCapability(skill.agent) ? guardedToggleBoundary : nil
        }
        guard !adapterCapability.configToggle.supported else { return nil }
            if skill.agent == "openclaw" {
                return UIStrings.openClawToggleBlocked
            }
            return adapterCapability.configToggle.reason ?? UIStrings.readOnlyAdapterStatus(adapterCapability.displayName)
    }

    private var isGuardedToggleAvailable: Bool {
        DisplayText.requiresGuardedToggleCapability(skill.agent) && adapterCapability?.configToggle.supported == true
    }

    private var guardedToggleBoundary: String {
        DisplayText.guardedToggleBoundary(for: skill.agent)
    }

    private var showsReadOnlyPreviewBadge: Bool {
        DisplayText.isReadOnlyPreview(skill) && !isGuardedToggleAvailable
    }

}

struct RecentActivityCard: View {
    let events: [SkillEventRecord]
    let isLoading: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.recentActivity, systemImage: "clock.badge")
                    .font(.headline)
                Spacer()
                if isLoading {
                    Label(UIStrings.loadingRecentActivity, systemImage: "hourglass")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }

            if events.isEmpty {
                Text(isLoading ? UIStrings.loadingRecentActivity : UIStrings.noRecentActivity)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                VStack(alignment: .leading, spacing: 10) {
                    ForEach(events) { event in
                        SkillActivityRow(event: event)
                    }
                }
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

struct HistorySection: View {
    let events: [SkillEventRecord]
    let isLoading: Bool
    let completeness: ListCompletenessState
    let onLoadMore: () -> Void
    let onLoadAll: () -> Void
    let onCancel: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            VStack(alignment: .leading, spacing: 8) {
                Label(UIStrings.text("history.activity", "Configuration activity"), systemImage: "clock.arrow.circlepath")
                    .font(.headline)
                Text(UIStrings.text("history.activity.summary", "History shows lightweight enable, disable, and config-action events that the service already records. Skill-content snapshots are intentionally not shown here."))
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
            .padding()
            .frame(maxWidth: .infinity, alignment: .leading)
            .nativePanelSurface()

            RecentActivityCard(events: events, isLoading: isLoading)

            if completeness.completeness != .complete || completeness.loadingPhase != .idle {
                ListCompletenessFooter(
                    state: completeness,
                    onLoadMore: onLoadMore,
                    onLoadAll: onLoadAll,
                    onCancel: onCancel
                )
            }
        }
    }
}

struct SkillActivityRow: View {
    let event: SkillEventRecord

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: "switch.2")
                .foregroundStyle(.secondary)
                .frame(width: 18)

            VStack(alignment: .leading, spacing: 4) {
                Text(activityTitle)
                    .font(.subheadline.bold())
                Text(DisplayText.timestamp(event.occurredAt))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                if let payloadSummary {
                    Text(payloadSummary)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(2)
                        .textSelection(.enabled)
                }
            }
            Spacer()
        }
        .padding(10)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
    }

    private var activityTitle: String {
        if let enabled = event.payload.boolValue(forAnyKey: ["on", "enabled"]) {
            return UIStrings.activityToggleState(enabled: enabled)
        }
        return event.kind
    }

    private var payloadSummary: String? {
        let summary = event.payload.compactDisplayString
        return summary.isEmpty ? nil : "\(UIStrings.activityPayload): \(summary)"
    }
}

struct CountBadge: View {
    let label: String
    let value: Int
    let systemImage: String
    let tint: Color
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(alignment: .center, spacing: 10) {
                Image(systemName: systemImage)
                    .font(.title3)
                    .foregroundStyle(value > 0 ? tint : .secondary)
                    .frame(width: 24)

                VStack(alignment: .leading, spacing: 1) {
                    Text(label)
                        .font(.caption2.bold())
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                    Text("\(value)")
                        .font(.callout.bold())
                        .foregroundStyle(value > 0 ? .primary : .secondary)
                        .monospacedDigit()
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .padding(.horizontal, 10)
            .frame(maxWidth: .infinity, minHeight: 54, alignment: .leading)
            .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
        }
        .buttonStyle(.plain)
        .help(UIStrings.text("detail.countBadge.help", "Show \(label)"))
        .accessibilityElement(children: .combine)
    }
}

struct EmptyDetailView: View {
    let title: String
    let message: String
    var systemImage: String = "sparkle.magnifyingglass"

    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: systemImage)
                .font(.system(size: 42))
                .foregroundStyle(.secondary)
            Text(title)
                .font(.title2.bold())
            Text(message)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

struct SkillDetailCard: View {
    let skill: SkillRecord
    let detail: SkillDetailRecord?
    let adapterCapability: AdapterCapabilityRecord?
    let isLoading: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 18) {
            Grid(alignment: .leading, horizontalSpacing: 18, verticalSpacing: 12) {
                MetadataRow(label: UIStrings.agent, value: DisplayText.agent(skill.agent))
                MetadataRow(label: UIStrings.scope, value: DisplayText.scope(for: skill))
                MetadataRow(label: UIStrings.provenanceRoot, value: SkillProvenanceDisplay.rootClass(for: skill))
                MetadataRow(label: UIStrings.provenanceKind, value: SkillProvenanceDisplay.kind(for: skill))
                MetadataRow(label: UIStrings.definition, value: skill.definitionId)
                MetadataRow(label: UIStrings.catalogID, value: skill.id)
                PrivacyPathRow(label: UIStrings.source, path: skill.displayPath)
                if DisplayText.isToolGlobal(skill) {
                    MetadataRow(label: UIStrings.access, value: UIStrings.toolGlobalAccessStatus(DisplayText.agent(skill.agent)))
                } else if DisplayText.isReadOnlyAdapter(skill.agent) || DisplayText.requiresGuardedToggleCapability(skill.agent) {
                    MetadataRow(label: UIStrings.access, value: adapterAccessStatus)
                }
                if let detail {
                    MetadataRow(label: UIStrings.fingerprint, value: detail.fingerprint)
                    MetadataRow(label: UIStrings.description, value: detail.description.isEmpty ? UIStrings.noDescription : detail.description)
                }
            }

            if isLoading {
                ProgressView(UIStrings.loadingSkillDetail)
            }

            if let detail {
                PermissionSummaryCard(summary: PermissionDisplayModel.summary(for: detail.permissions))

                if !detail.frontmatterRaw.isEmpty {
                    TextBlock(title: UIStrings.frontmatter, content: detail.frontmatterRaw)
                }
                if !detail.body.isEmpty {
                    TextBlock(title: UIStrings.body, content: detail.body)
                }
            }

            Text(UIStrings.connectedProtocolNote)
                .font(.callout)
                .foregroundStyle(.secondary)
                .padding()
                .frame(maxWidth: .infinity, alignment: .leading)
                .nativePanelSurface()
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private var adapterAccessStatus: String {
        if DisplayText.requiresGuardedToggleCapability(skill.agent) && adapterCapability?.configToggle.supported == true {
            return DisplayText.guardedToggleBoundary(for: skill.agent)
        }
        if skill.agent == "hermes" {
            if skill.provenance.rootKind == .external || skill.provenance.scopeKind == .external {
                return UIStrings.hermesExternalAccess
            }
            return UIStrings.hermesHomeProfileAccess
        }
        if skill.agent == "openclaw" {
            return UIStrings.openClawReadOnlyAccess
        }
        if skill.agent == "pi" {
            return UIStrings.piGuardedToggleBoundary
        }
        return UIStrings.readOnlyAdapterStatus(DisplayText.agent(skill.agent))
    }
}

struct ToolGlobalPreviewCard: View {
    @EnvironmentObject private var store: SkillStore
    let skill: SkillRecord
    @State private var target: ToolInstallTarget = .claudeCode
    @State private var preview: ToolGlobalInstallPreview?
    @State private var isPreviewing = false
    @State private var isConfirming = false

    var body: some View {
        let targets = ToolInstallTarget.supportedTargets(from: store.adapterCapabilities)
        let selectedTarget = targets.contains(target) ? target : (targets.first ?? target)

        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.toolGlobalPreviewTitle, systemImage: "shippingbox")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.toolGlobalPreviewNote)
                .font(.callout)
                .foregroundStyle(.secondary)

            HStack(spacing: 10) {
                Picker(UIStrings.toolGlobalTargetAgent, selection: $target) {
                    ForEach(targets) { target in
                        Text(target.title).tag(target)
                    }
                }
                .pickerStyle(.menu)
                .frame(width: 180)

                Button {
                    Task {
                        isPreviewing = true
                        defer { isPreviewing = false }
                        preview = await store.previewToolInstall(skill: skill, target: selectedTarget)
                    }
                } label: {
                    Label(UIStrings.installToAgent, systemImage: "square.and.arrow.down")
                }
                .disabled(store.isRefreshBusy || isPreviewing || targets.isEmpty)
                .help(UIStrings.toolGlobalInstallConfirmation(skill.name, selectedTarget.title))
            }
        }
        .onAppear {
            if let first = targets.first, !targets.contains(target) {
                target = first
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
        .sheet(item: $preview) { preview in
            ToolGlobalInstallPreviewSheet(
                preview: preview,
                isConfirming: isConfirming,
                onConfirm: {
                    Task {
                        isConfirming = true
                        defer { isConfirming = false }
                        if let result = await store.confirmToolInstall(skill: skill, target: preview.target) {
                            self.preview = result
                        }
                    }
                }
            )
        }
    }
}

struct ToolGlobalInstallPreviewSheet: View {
    let preview: ToolGlobalInstallPreview
    let isConfirming: Bool
    let onConfirm: () -> Void
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text(UIStrings.toolGlobalInstallPreviewTitle)
                        .font(.title2.bold())
                    Text(preview.summary)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Button(UIStrings.done) {
                    dismiss()
                }
                .keyboardShortcut(.defaultAction)
            }

            Grid(alignment: .leading, horizontalSpacing: 18, verticalSpacing: 10) {
                PrivacyPathRow(label: UIStrings.source, path: preview.sourcePath)
                MetadataRow(label: UIStrings.toolGlobalTargetAgent, value: preview.target.title)
                if let targetPath = preview.targetPath {
                    PrivacyPathRow(label: UIStrings.target, path: targetPath)
                }
            }

            Label(preview.confirmationMessage, systemImage: "checkmark.shield")
                .foregroundStyle(.secondary)

            if !preview.risks.isEmpty {
                VStack(alignment: .leading, spacing: 6) {
                    ForEach(preview.risks, id: \.self) { risk in
                        Label(risk, systemImage: "exclamationmark.triangle")
                            .font(.callout)
                            .foregroundStyle(.secondary)
                    }
                }
            }

            Label(UIStrings.toolGlobalInstallReady, systemImage: "checkmark.shield")
                .foregroundStyle(preview.wrote ? .green : .secondary)

            HStack {
                Spacer()
                Button(UIStrings.cancel) {
                    dismiss()
                }
                Button(preview.wrote ? UIStrings.done : UIStrings.confirmInstall) {
                    if preview.wrote {
                        dismiss()
                    } else {
                        onConfirm()
                    }
                }
                    .buttonStyle(.borderedProminent)
                    .disabled((!preview.writeBackEnabled && !preview.wrote) || isConfirming)
                    .help(UIStrings.toolGlobalInstallReady)
            }
        }
        .padding(24)
        .frame(width: 720, height: 420)
    }
}
