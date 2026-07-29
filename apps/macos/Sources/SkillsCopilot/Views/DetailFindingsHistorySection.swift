import AppKit
import SwiftUI

struct FindingsSection: View {
    let skill: SkillRecord
    let findings: [RuleFindingRecord]
    let catalogCompleteness: ListCompletenessState
    @State private var ruleFilter = FindingDisplayModel.allFilterValue

    private var ruleIDOptions: [String] {
        FindingDisplayModel.ruleIDOptions(for: findings)
    }

    private var visibleGroups: [FindingSeverityGroup] {
        FindingDisplayModel.grouped(
            findings: findings,
            severityFilter: FindingDisplayModel.allFilterValue,
            ruleFilter: ruleFilter
        )
    }

    private var catalogStatusIssueKind: SkillStatusKind? {
        SkillListModel.catalogStatusIssueKind(for: skill)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if findings.isEmpty && catalogStatusIssueKind == nil {
                if catalogCompleteness.completeness == .complete {
                    EmptyState(
                        title: UIStrings.noFindings,
                        systemImage: "checkmark.seal",
                        message: UIStrings.noFindingsForSkillMessage(DisplayText.agent(skill.agent))
                    )
                }
            } else {
                if let catalogStatusIssueKind {
                    CatalogStatusIssueCard(skill: skill, status: catalogStatusIssueKind)
                }

                if !findings.isEmpty {
                    FindingsControlPanel(
                        showsFilters: true,
                        ruleFilter: $ruleFilter,
                        ruleIDOptions: ruleIDOptions
                    )

                    if visibleGroups.isEmpty {
                        EmptyState(
                            title: UIStrings.noMatchingFindings,
                            systemImage: "line.3.horizontal.decrease.circle",
                            message: UIStrings.noMatchingFindingsMessage
                        )
                    } else {
                        ForEach(visibleGroups) { group in
                            VStack(alignment: .leading, spacing: 10) {
                                FindingSeverityHeader(group: group)

                                ForEach(group.issues) { issue in
                                    FindingIssueCard(
                                        issue: issue,
                                        severityTitle: group.title
                                    )
                                }
                            }
                            .frame(maxWidth: .infinity, alignment: .leading)
                        }
                    }
                }
            }

            if catalogCompleteness.completeness != .complete {
                catalogCoverageFooter(
                    label: UIStrings.text("findings.catalogCoverage", "Visible issue scan coverage"),
                    state: catalogCompletenessState(loadedCount: findings.count)
                )
            }
        }
        .onAppear {
            clampFilters()
        }
        .onChange(of: findings) { _ in
            clampFilters()
        }
    }

    private func clampFilters() {
        if ruleFilter != FindingDisplayModel.allFilterValue && !ruleIDOptions.contains(ruleFilter) {
            ruleFilter = FindingDisplayModel.allFilterValue
        }
    }

    private func catalogCompletenessState(loadedCount: Int) -> ListCompletenessState {
        ListCompletenessState(
            loadedCount: loadedCount,
            totalCount: catalogCompleteness.completeness == .complete ? loadedCount : nil,
            hasMore: false,
            isComplete: catalogCompleteness.isComplete,
            completeness: catalogCompleteness.completeness,
            incompleteReason: catalogCompleteness.incompleteReason,
            loadingPhase: catalogCompleteness.loadingPhase,
            canLoadMore: false,
            canLoadAll: false
        )
    }

    @ViewBuilder
    private func catalogCoverageFooter(label: String, state: ListCompletenessState) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(label)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            ListCompletenessFooter(state: state, onLoadMore: {}, onLoadAll: {}, onCancel: {})
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

private struct CatalogStatusIssueCard: View {
    let skill: SkillRecord
    let status: SkillStatusKind

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Label(title, systemImage: DisplayText.stateSystemImage(skill.state, enabled: skill.enabled))
                    .font(.headline)
                    .foregroundStyle(.orange)
                Spacer()
                Text(UIStrings.text("issues.catalogStatus.badge", "Catalog status"))
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Color.agentCopilotPanelBackground, in: Capsule())
            }

            Text(message)
                .font(.callout)
                .foregroundStyle(.secondary)

            VStack(alignment: .leading, spacing: 4) {
                Text(UIStrings.findingRemediation)
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                Text(remediation)
                    .font(.callout)
            }
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color.orange.opacity(0.08), in: RoundedRectangle(cornerRadius: 8))
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }

    private var title: String {
        switch status {
        case .missing:
            return UIStrings.text("issues.catalogStatus.missing.title", "Skill source is missing")
        case .broken:
            return UIStrings.text("issues.catalogStatus.broken.title", "Skill could not be loaded")
        case .unknown:
            return UIStrings.text("issues.catalogStatus.unknown.title", "Skill state needs review")
        case .enabled, .disabled, .shadowed:
            return UIStrings.text("issues.catalogStatus.title", "Catalog status issue")
        }
    }

    private var message: String {
        switch status {
        case .missing:
            return UIStrings.text("issues.catalogStatus.missing.message", "The source file was not found during the last complete scan. The catalog keeps this historical record, but the skill cannot be used or toggled.")
        case .broken:
            return UIStrings.text("issues.catalogStatus.broken.message", "The source was found, but the skill could not be parsed or loaded as a usable skill.")
        case .unknown:
            return UIStrings.text("issues.catalogStatus.unknown.message", "The catalog returned a state this app cannot classify as enabled, disabled, broken, missing, or shadowed.")
        case .enabled, .disabled, .shadowed:
            return UIStrings.text("issues.catalogStatus.message", "Review the current catalog state before relying on this skill.")
        }
    }

    private var remediation: String {
        switch status {
        case .missing:
            return UIStrings.text("issues.catalogStatus.missing.remediation", "Restore SKILL.md at the recorded source path if the skill should still exist. If it was intentionally removed, keep this historical record as missing.")
        case .broken:
            return UIStrings.text("issues.catalogStatus.broken.remediation", "Repair the SKILL.md frontmatter or source content, then run Deep Scan again.")
        case .unknown:
            return UIStrings.text("issues.catalogStatus.unknown.remediation", "Inspect the source and scan diagnostics, then run Deep Scan again after correcting the underlying state.")
        case .enabled, .disabled, .shadowed:
            return UIStrings.text("issues.catalogStatus.remediation", "Inspect the skill source and scan again before relying on this record.")
        }
    }
}

struct FindingsControlPanel: View {
    let showsFilters: Bool
    @Binding var ruleFilter: String
    let ruleIDOptions: [String]

    var body: some View {
        if showsFilters {
            filterControls
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private var filterControls: some View {
        HStack(spacing: 10) {
            filterControl(label: UIStrings.findingRuleFilter) {
                rulePicker.frame(width: 250)
            }
            Spacer(minLength: 0)
        }
    }

    private func filterControl<Control: View>(
        label: String,
        @ViewBuilder control: () -> Control
    ) -> some View {
        HStack(spacing: 6) {
            Text(label)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            control()
        }
    }

    private var rulePicker: some View {
        Picker(UIStrings.findingRuleFilter, selection: $ruleFilter) {
            Text(UIStrings.allRuleIDs).tag(FindingDisplayModel.allFilterValue)
            ForEach(ruleIDOptions, id: \.self) { ruleID in
                Text(ruleID).tag(ruleID)
            }
        }
        .labelsHidden()
        .pickerStyle(.menu)
        .help(UIStrings.findingRuleFilter)
    }
}

struct FindingIssueCard: View {
    let issue: FindingIssueGroup
    let severityTitle: String
    @State private var didCopyRemediation = false

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Label(UIStrings.findingTrigger, systemImage: "exclamationmark.bubble")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                Spacer()
                if issue.isRiskRelated {
                    Label(UIStrings.findingRiskRelated, systemImage: "shield.lefthalf.filled")
                        .font(.caption.bold())
                        .foregroundStyle(.orange)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(.orange.opacity(0.14), in: Capsule())
                        .help(UIStrings.findingRiskRelatedHelp)
                }
                Text(severityTitle)
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Color.agentCopilotPanelBackground, in: Capsule())
            }

            Text(issue.message)
                .font(.headline)
                .textSelection(.enabled)

            VStack(alignment: .leading, spacing: 8) {
                Label(UIStrings.findingExplanation, systemImage: "list.bullet.clipboard")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)

                DetailMetricGrid(maxColumns: 4, minColumnWidth: 190, spacing: 8) {
                    FindingExplanationField(title: UIStrings.findingRuleID, value: issue.ruleId, systemImage: "number")
                    FindingExplanationField(title: UIStrings.findingRuleSource, value: issue.ruleSource, systemImage: "scope")
                }
            }
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))

            VStack(alignment: .leading, spacing: 5) {
                HStack(spacing: 8) {
                    Label(UIStrings.findingRemediation, systemImage: "wrench.and.screwdriver")
                        .font(.caption.bold())
                        .foregroundStyle(.blue)
                    Spacer()
                    Button {
                        copyRemediation()
                    } label: {
                        Label(
                            didCopyRemediation
                                ? UIStrings.text("action.copied", "Copied")
                                : UIStrings.text("finding.action.copyRemediation", "Copy Fix"),
                            systemImage: didCopyRemediation ? "checkmark" : "doc.on.doc"
                        )
                    }
                    .buttonStyle(.borderless)
                    .controlSize(.small)
                    .accessibilityHint(UIStrings.text(
                        "finding.action.copyRemediation.hint",
                        "Copy this remediation for use in your editor or agent."
                    ))
                }
                Text(issue.remediation)
                    .foregroundStyle(.primary)
            }
            .padding(10)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }

    private func copyRemediation() {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(issue.remediation, forType: .string)
        didCopyRemediation = true
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
            didCopyRemediation = false
        }
    }
}

struct FindingExplanationField: View {
    let title: String
    let value: String
    let systemImage: String

    var body: some View {
        HStack(alignment: .top, spacing: 7) {
            Image(systemName: systemImage)
                .font(.caption)
                .foregroundStyle(.secondary)
                .frame(width: 14)

            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                Text(value)
                    .font(.caption)
                    .foregroundStyle(.primary)
                    .lineLimit(2)
                    .truncationMode(.middle)
                    .textSelection(.enabled)
            }
        }
        .padding(8)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 7))
    }
}

struct FindingSeverityHeader: View {
    let group: FindingSeverityGroup

    var body: some View {
        HStack(spacing: 8) {
            Label(group.title, systemImage: systemImage)
                .font(.subheadline.bold())
                .foregroundStyle(tint)
            Text(UIStrings.findingSeverityGroupCount(group.issues.count))
                .font(.caption)
                .foregroundStyle(.secondary)
            Spacer(minLength: 0)
        }
        .padding(.top, 6)
    }

    private var systemImage: String {
        switch group.severityKey {
        case "critical", "error":
            return "xmark.octagon"
        case "warning", "warn":
            return "exclamationmark.triangle"
        case "info", "notice":
            return "info.circle"
        default:
            return "questionmark.circle"
        }
    }

    private var tint: Color {
        switch group.severityKey {
        case "critical", "error":
            return .red
        case "warning", "warn":
            return .orange
        case "info", "notice":
            return .blue
        default:
            return .secondary
        }
    }
}

struct PermissionSummaryCard: View {
    let summary: PermissionSummary

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Label(UIStrings.permissions, systemImage: "hand.raised")
                .font(.headline)

            Grid(alignment: .leading, horizontalSpacing: 18, verticalSpacing: 8) {
                ForEach(summary.rows) { row in
                    MetadataRow(label: row.label, value: row.value)
                }
            }

            Text(summary.note)
                .font(.callout)
                .foregroundStyle(.secondary)

            VStack(alignment: .leading, spacing: 6) {
                Text(UIStrings.permissionRaw)
                    .font(.subheadline.bold())
                Text(summary.rawText)
                    .font(.system(.callout, design: .monospaced))
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(10)
                    .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

struct SameAgentConflictIssuesView: View {
    @EnvironmentObject private var store: SkillStore
    let conflicts: [ConflictGroupRecord]
    let selectedSkillID: String
    let currentAgentSkills: [SkillRecord]
    let onSelectSkill: (SkillRecord) -> Void
    let onDisableDuplicate: (SkillRecord) -> Void
    var showsEmptyState = false

    var body: some View {
        if conflicts.isEmpty {
            if showsEmptyState {
                EmptyState(title: UIStrings.noConflicts, systemImage: "checkmark.circle", message: UIStrings.noConflictsMessage)
            }
        } else {
            VStack(alignment: .leading, spacing: 8) {
                Label(UIStrings.text("conflicts.issueSection", "Same-agent conflicts"), systemImage: "person.crop.circle.badge.exclamationmark")
                    .font(.headline)
                Text(UIStrings.text("conflicts.issueSection.summary", "Current-agent runtime/name collisions are shown separately from single-skill issues because each conflict spans multiple skill instances."))
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
            .padding()
            .frame(maxWidth: .infinity, alignment: .leading)
            .nativePanelSurface()

            ForEach(conflicts) { conflict in
                let currentAgentInstanceIDs = conflict.instanceIds.filter {
                    currentAgentSkillByID[$0] != nil
                }
                VStack(alignment: .leading, spacing: 10) {
                    HStack(alignment: .firstTextBaseline) {
                        Text(conflict.reason)
                            .font(.headline)
                        Spacer()
                        Text(UIStrings.text("conflicts.currentAgentOnlyBadge", "current agent only"))
                            .font(.caption.bold())
                            .foregroundStyle(.secondary)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .background(Color.agentCopilotPanelBackground, in: Capsule())
                    }
                    DetailMetricGrid {
                        SummaryChip(title: UIStrings.definition, value: conflict.definitionId, systemImage: "number")
                        SummaryChip(title: UIStrings.winner, value: conflict.winnerId ?? UIStrings.none, systemImage: "crown")
                        SummaryChip(title: UIStrings.instances, value: "\(currentAgentInstanceIDs.count)", systemImage: "rectangle.stack")
                        SummaryChip(title: UIStrings.text("conflicts.selectedInstance", "Selected"), value: selectedSkillID, systemImage: "target")
                    }

                    if !currentAgentInstanceIDs.isEmpty {
                        VStack(alignment: .leading, spacing: 6) {
                            Text(UIStrings.instances)
                                .font(.caption.bold())
                                .foregroundStyle(.secondary)

                            ForEach(currentAgentInstanceIDs, id: \.self) { instanceID in
                                ConflictInstanceActionRow(
                                    instanceID: instanceID,
                                    skill: currentAgentSkillByID[instanceID],
                                    winnerID: conflict.winnerId,
                                    selectedSkillID: selectedSkillID,
                                    toggleDisabledReason: currentAgentSkillByID[instanceID].flatMap {
                                        store.toggleDisabledReason(for: $0)
                                    },
                                    onSelectSkill: onSelectSkill,
                                    onDisableDuplicate: onDisableDuplicate
                                )
                            }
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
                .padding()
                .frame(maxWidth: .infinity, alignment: .leading)
                .nativePanelSurface()
            }
        }
    }

    private var currentAgentSkillByID: [String: SkillRecord] {
        Dictionary(uniqueKeysWithValues: currentAgentSkills.map { ($0.id, $0) })
    }
}

private struct ConflictInstanceActionRow: View {
    let instanceID: String
    let skill: SkillRecord?
    let winnerID: String?
    let selectedSkillID: String
    let toggleDisabledReason: String?
    let onSelectSkill: (SkillRecord) -> Void
    let onDisableDuplicate: (SkillRecord) -> Void

    var body: some View {
        HStack(alignment: .center, spacing: 8) {
            Image(systemName: statusImage)
                .foregroundStyle(isWinner ? Color.green : Color.secondary)
                .frame(width: 16)
                .accessibilityHidden(true)

            VStack(alignment: .leading, spacing: 2) {
                Text(skill?.name ?? instanceID)
                    .font(.caption.bold())
                    .lineLimit(1)
                    .truncationMode(.middle)
                Text(statusText)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            if let skill {
                Button {
                    onSelectSkill(skill)
                } label: {
                    Label(
                        UIStrings.text("conflicts.action.open", "Open"),
                        systemImage: "arrow.right.circle"
                    )
                }
                .buttonStyle(.bordered)
                .controlSize(.small)
                .accessibilityHint(UIStrings.text(
                    "conflicts.action.open.hint",
                    "Show this conflicting skill."
                ))

                if canDisableAsDuplicate {
                    Button(role: .destructive) {
                        onDisableDuplicate(skill)
                    } label: {
                        Label(
                            UIStrings.text("conflicts.action.disableDuplicate", "Disable Duplicate…"),
                            systemImage: "pause.circle"
                        )
                    }
                    .buttonStyle(.bordered)
                    .controlSize(.small)
                    .disabled(toggleDisabledReason != nil)
                    .help(toggleDisabledReason ?? UIStrings.text(
                        "conflicts.action.disableDuplicate.help",
                        "Preview disabling this duplicate while keeping the selected winner active."
                    ))
                }
            }
        }
        .padding(8)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 7))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(skill?.name ?? instanceID)
        .accessibilityValue(statusText)
    }

    private var isWinner: Bool {
        winnerID == instanceID
    }

    private var isSelected: Bool {
        selectedSkillID == instanceID
    }

    private var canDisableAsDuplicate: Bool {
        guard let winnerID, winnerID != instanceID, skill != nil else { return false }
        return DisplayText.statusKind(skill?.state ?? "", enabled: skill?.enabled ?? false) == .enabled
    }

    private var statusImage: String {
        if isWinner {
            return "crown.fill"
        }
        if isSelected {
            return "target"
        }
        return "circle"
    }

    private var statusText: String {
        if isWinner {
            return UIStrings.text("conflicts.instance.winner", "Active winner")
        }
        if isSelected {
            return UIStrings.text("conflicts.instance.selected", "Selected duplicate")
        }
        return UIStrings.text("conflicts.instance.duplicate", "Conflicting duplicate")
    }
}

struct ConflictsSection: View {
    let conflicts: [ConflictGroupRecord]
    let selectedSkillID: String
    let currentAgentSkills: [SkillRecord]
    let catalogCompleteness: ListCompletenessState
    let onSelectSkill: (SkillRecord) -> Void
    let onDisableDuplicate: (SkillRecord) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            SameAgentConflictIssuesView(
                conflicts: conflicts,
                selectedSkillID: selectedSkillID,
                currentAgentSkills: currentAgentSkills,
                onSelectSkill: onSelectSkill,
                onDisableDuplicate: onDisableDuplicate,
                showsEmptyState: catalogCompleteness.completeness == .complete
            )

            if catalogCompleteness.completeness != .complete {
                VStack(alignment: .leading, spacing: 6) {
                    Text(UIStrings.text("conflicts.catalogCoverage", "Conflict scan coverage"))
                        .font(.caption.bold())
                        .foregroundStyle(.secondary)
                    ListCompletenessFooter(
                        state: catalogCompletenessState,
                        onLoadMore: {},
                        onLoadAll: {},
                        onCancel: {}
                    )
                }
                .padding()
                .frame(maxWidth: .infinity, alignment: .leading)
                .nativePanelSurface()
            }
        }
    }

    private var catalogCompletenessState: ListCompletenessState {
        ListCompletenessState(
            loadedCount: conflicts.count,
            totalCount: catalogCompleteness.completeness == .complete ? conflicts.count : nil,
            hasMore: false,
            isComplete: catalogCompleteness.isComplete,
            completeness: catalogCompleteness.completeness,
            incompleteReason: catalogCompleteness.incompleteReason,
            loadingPhase: catalogCompleteness.loadingPhase,
            canLoadMore: false,
            canLoadAll: false
        )
    }
}

struct AgentConfigHistorySection: View {
    let snapshots: [ConfigSnapshotRecord]
    let isWriting: Bool
    let onPreview: (String) async throws -> SnapshotRollbackPreviewRecord
    let onRollback: (String) async -> Void
    @State private var preview: SnapshotRollbackPreviewRecord?
    @State private var previewError: String?
    @State private var snapshotToRollback: ConfigSnapshotRecord?
    @AppStorage(DisplayText.screenshotPrivacyModeStorageKey) private var privacyModeEnabled = true

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let previewError {
                ErrorBanner(message: previewError)
            }

            if snapshots.isEmpty {
                EmptyState(title: UIStrings.noSnapshots, systemImage: "clock.badge.questionmark", message: UIStrings.noSnapshotsMessage)
            } else {
                ForEach(snapshots) { snapshot in
                    VStack(alignment: .leading, spacing: 8) {
                        HStack(alignment: .top) {
                            VStack(alignment: .leading, spacing: 8) {
                                Text(snapshot.reason)
                                    .font(.headline)
                                    .lineLimit(2)
                                DetailMetricGrid {
                                    SummaryChip(
                                        title: UIStrings.target,
                                        value: DisplayText.privacyPath(
                                            snapshot.target,
                                            privacyModeEnabled: privacyModeEnabled
                                        ),
                                        systemImage: "scope"
                                    )
                                    SummaryChip(title: UIStrings.scope, value: DisplayText.scope(snapshot.scope), systemImage: "folder")
                                    SummaryChip(title: UIStrings.text("history.created", "Created"), value: DisplayText.timestamp(snapshot.createdAt), systemImage: "calendar")
                                    SummaryChip(title: UIStrings.text("history.characters", "Captured"), value: UIStrings.charactersCaptured(snapshot.content.count), systemImage: "textformat.size")
                                }
                            }
                            Spacer()

                            HStack(spacing: 8) {
                                Button {
                                    loadPreview(snapshot.id)
                                } label: {
                                    Label(UIStrings.preview, systemImage: "eye")
                                }
                                .disabled(isWriting)

                                Button(role: .destructive) {
                                    snapshotToRollback = snapshot
                                } label: {
                                    Label(UIStrings.rollback, systemImage: "arrow.uturn.backward")
                                }
                                .disabled(isWriting)
                            }
                        }
                    }
                    .padding()
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .nativePanelSurface()
                }
            }
        }
        .sheet(item: $preview) { preview in
            SnapshotPreviewSheet(preview: preview)
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
                if let snapshotID = snapshotToRollback?.id {
                    Task { await onRollback(snapshotID) }
                }
                snapshotToRollback = nil
            }
            Button(UIStrings.cancel, role: .cancel) {
                snapshotToRollback = nil
            }
        } message: {
            Text(
                DisplayText.privacyPath(
                    snapshotToRollback?.target ?? "",
                    privacyModeEnabled: privacyModeEnabled
                )
            )
        }
    }

    private func loadPreview(_ snapshotID: String) {
        previewError = nil
        Task {
            do {
                preview = try await onPreview(snapshotID)
            } catch {
                previewError = error.localizedDescription
            }
        }
    }
}

struct SnapshotPreviewSheet: View {
    let preview: SnapshotRollbackPreviewRecord
    @Environment(\.dismiss) private var dismiss
    @State private var revealsSnapshotContent = false

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text(UIStrings.snapshotPreview)
                        .font(.title2.bold())
                    PrivacyPathText(
                        path: preview.snapshot.target,
                        font: .callout,
                        lineLimit: 2
                    )
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
                Button(UIStrings.done) {
                    dismiss()
                }
                .keyboardShortcut(.defaultAction)
            }

            Label(
                preview.changed ? UIStrings.currentDiffersFromSnapshot : UIStrings.currentMatchesSnapshot,
                systemImage: preview.changed ? "exclamationmark.triangle" : "checkmark.circle"
            )
            .foregroundStyle(preview.changed ? .orange : .green)

            if let readError = preview.currentReadError {
                ErrorBanner(message: readError)
            }

            HStack(alignment: .top, spacing: 14) {
                SnapshotTextPane(title: UIStrings.current, content: displayContent(preview.currentContent))
                SnapshotTextPane(title: UIStrings.snapshot, content: displayContent(preview.snapshot.content))
            }
            .frame(minHeight: 420)
        }
        .padding(24)
        .frame(width: 980, height: 680)
    }

    private func displayContent(_ content: String) -> String {
        let value = content.isEmpty ? UIStrings.emptyPlaceholder : content
        return revealsSnapshotContent ? value : ConfigContentRedactor.redactedForDisplay(value)
    }
}

struct SnapshotTextPane: View {
    let title: String
    let content: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.headline)
            ScrollView([.vertical, .horizontal]) {
                Text(content)
                    .font(.system(.body, design: .monospaced))
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(12)
            }
            .frame(minWidth: 430, maxWidth: .infinity, maxHeight: .infinity)
            .background(Color(nsColor: .textBackgroundColor), in: RoundedRectangle(cornerRadius: 6))
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

struct TextBlock: View {
    let title: String
    let content: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.headline)
            Text(content)
                .font(.system(.body, design: .monospaced))
                .textSelection(.enabled)
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}
