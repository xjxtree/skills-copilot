import SwiftUI

struct SkillSummaryCard: View {
    let skill: SkillRecord
    let detail: SkillDetailRecord?
    let scriptPreview: ScriptExecutionPreview?
    let isLoading: Bool
    @AppStorage(DisplayText.screenshotPrivacyModeStorageKey) private var screenshotPrivacyModeEnabled = true

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            OverviewPurposeContent(
                summaryText: summaryText,
                isEmpty: summaryText == UIStrings.noDescription,
                isLoading: isLoading
            )

            CompactMetadataGrid(rows: diagnosticRows)

            if showsOverviewRiskPanel {
                OverviewRiskPanel(
                    permissionSummary: PermissionDisplayModel.summary(for: permissionPayload),
                    scriptPreview: scriptPreview
                )
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }

    private var summaryText: String {
        guard let description = detail?.description.trimmingCharacters(in: .whitespacesAndNewlines), !description.isEmpty else {
            return UIStrings.noDescription
        }
        return description
    }

    private var permissionPayload: JSONValue {
        detail?.permissions ?? .null
    }

    private var showsOverviewRiskPanel: Bool {
        PermissionDisplayModel.hasOverviewSignal(for: permissionPayload)
            || scriptPreview?.hasOverviewSignal == true
    }

    private var diagnosticRows: [CompactMetadataRow] {
        [
            CompactMetadataRow(label: UIStrings.agent, value: DisplayText.agent(skill.agent), systemImage: "person.crop.circle"),
            CompactMetadataRow(label: UIStrings.scope, value: DisplayText.scope(for: skill), systemImage: "folder"),
            CompactMetadataRow(label: UIStrings.provenanceKind, value: SkillProvenanceDisplay.kind(for: skill), systemImage: "tag"),
            CompactMetadataRow(
                label: UIStrings.source,
                value: DisplayText.privacyPath(skill.displayPath, privacyModeEnabled: screenshotPrivacyModeEnabled),
                systemImage: "doc",
                isCopyable: true
            )
        ]
    }
}

private struct OverviewPurposeContent: View {
    let summaryText: String
    let isEmpty: Bool
    let isLoading: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Label(UIStrings.text("detail.skillPurpose", "Purpose"), systemImage: "text.quote")
                    .font(.subheadline.bold())
                Spacer()
                if isLoading {
                    Label(UIStrings.loadingSkillDetail, systemImage: "hourglass")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                } else {
                    Text(isEmpty ? UIStrings.noDescription : UIStrings.text("detail.skillPurposeSource", "Description"))
                        .font(.caption2.bold())
                        .foregroundStyle(.secondary)
                }
            }

            VStack(alignment: .leading, spacing: 8) {
                ForEach(summaryItems.indices, id: \.self) { index in
                    Text(summaryItems[index])
                        .font(.callout)
                        .lineSpacing(2)
                        .foregroundStyle(isEmpty ? .secondary : .primary)
                        .fixedSize(horizontal: false, vertical: true)
                        .textSelection(.enabled)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private var summaryItems: [String] {
        let normalized = summaryText
            .replacingOccurrences(of: "\r\n", with: "\n")
            .trimmingCharacters(in: .whitespacesAndNewlines)
        guard !normalized.isEmpty else { return [UIStrings.noDescription] }

        let lineItems = normalized
            .split(whereSeparator: \.isNewline)
            .map { String($0).trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { !$0.isEmpty }
        if lineItems.count > 1 {
            return lineItems
        }

        return sentenceItems(from: normalized)
    }

    private func sentenceItems(from value: String) -> [String] {
        var items: [String] = []
        let characters = Array(value)
        var startIndex = 0
        var index = 0
        let terminalPunctuation: Set<Character> = [".", "。", "!", "！", "?", "？"]
        let closingPunctuation: Set<Character> = [")", "]", "}", "\"", "'", "”", "’"]

        while index < characters.count {
            guard terminalPunctuation.contains(characters[index]),
                  !isInlineAbbreviationEnding(at: index, in: characters)
            else {
                index += 1
                continue
            }

            var endIndex = index + 1
            while endIndex < characters.count, closingPunctuation.contains(characters[endIndex]) {
                endIndex += 1
            }

            if endIndex == characters.count || isWhitespace(characters[endIndex]) {
                let item = String(characters[startIndex..<endIndex]).trimmingCharacters(in: .whitespacesAndNewlines)
                if !item.isEmpty {
                    items.append(item)
                }
                startIndex = endIndex
                index = endIndex
            } else {
                index += 1
            }
        }

        if startIndex < characters.count {
            let tail = String(characters[startIndex...]).trimmingCharacters(in: .whitespacesAndNewlines)
            if !tail.isEmpty {
                items.append(tail)
            }
        }
        return items.isEmpty ? [value] : items
    }

    private func isInlineAbbreviationEnding(at index: Int, in characters: [Character]) -> Bool {
        let start = max(0, index - 4)
        let prefix = String(characters[start...index]).lowercased()
        return prefix.hasSuffix("e.g.") || prefix.hasSuffix("i.e.") || prefix.hasSuffix("etc.")
    }

    private func isWhitespace(_ character: Character) -> Bool {
        String(character).trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }
}

struct CleanupQueueSection: View {
    let result: CleanupQueueResult
    let items: [CleanupQueueItem]
    @Binding var kindFilter: CleanupQueueKindFilter
    @Binding var priorityFilter: CleanupQueuePriorityFilter
    let agentTitle: String
    let isLoading: Bool
    let onOpen: (CleanupQueueItem) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            VStack(alignment: .leading, spacing: 12) {
                HStack(alignment: .firstTextBaseline) {
                    Label(UIStrings.cleanupQueue, systemImage: "tray.full")
                        .font(.headline)
                    Spacer()
                    if isLoading {
                        Label(UIStrings.loading, systemImage: "hourglass")
                            .font(.caption)
                            .foregroundStyle(.secondary)
                    }
                }

                Text(UIStrings.cleanupQueueReadOnlyBoundary)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)

                DetailMetricGrid {
                    SummaryChip(title: UIStrings.text("cleanup.summary.total", "Open queue"), value: "\(result.summary.total)", systemImage: "number")
                    SummaryChip(title: CleanupQueueKind.finding.title, value: "\(result.summary.findingCount)", systemImage: CleanupQueueKind.finding.systemImage)
                    SummaryChip(title: CleanupQueueKind.integrity.title, value: "\(result.summary.integrityCount)", systemImage: CleanupQueueKind.integrity.systemImage)
                    SummaryChip(title: CleanupQueueKind.conflict.title, value: "\(result.summary.conflictCount)", systemImage: CleanupQueueKind.conflict.systemImage)
                    SummaryChip(title: CleanupQueueKind.analysis.title, value: "\(result.summary.analysisCount)", systemImage: CleanupQueueKind.analysis.systemImage)
                }

                HStack {
                    Picker(UIStrings.cleanupFilterKind, selection: $kindFilter) {
                        ForEach(CleanupQueueKindFilter.allCases) { filter in
                            Text(filter.title).tag(filter)
                        }
                    }
                    .pickerStyle(.menu)

                    Picker(UIStrings.cleanupFilterPriority, selection: $priorityFilter) {
                        ForEach(CleanupQueuePriorityFilter.allCases) { filter in
                            Text(filter.title).tag(filter)
                        }
                    }
                    .pickerStyle(.menu)

                    Spacer()

                    Text(UIStrings.cleanupAgentFilterNote(agentTitle))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            .padding()
            .frame(maxWidth: .infinity, alignment: .leading)
            .nativePanelSurface()

            if let fallbackReason = result.fallbackReason ?? result.summary.unavailableReason {
                CleanupNoticeCard(message: fallbackReason)
            }

            if items.isEmpty {
                CleanupEmptyCard(
                    title: UIStrings.cleanupEmptyTitle,
                    message: result.summary.total == 0 ? UIStrings.cleanupEmptyMessage : UIStrings.cleanupNoFilteredItems
                )
            } else {
                VStack(alignment: .leading, spacing: 12) {
                    ForEach(items) { item in
                        CleanupQueueItemCard(item: item, onOpen: { onOpen(item) })
                    }
                }
            }
        }
    }
}

private struct CleanupQueueItemCard: View {
    let item: CleanupQueueItem
    let onOpen: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .top, spacing: 12) {
                Image(systemName: item.kind.systemImage)
                    .font(.title3)
                    .foregroundStyle(.secondary)
                    .frame(width: 24)

                VStack(alignment: .leading, spacing: 6) {
                    HStack(alignment: .firstTextBaseline, spacing: 8) {
                        Text(item.title)
                            .font(.headline)
                        Text(item.priority.title)
                            .font(.caption.bold())
                            .padding(.horizontal, 8)
                            .padding(.vertical, 3)
                            .background(priorityTint.opacity(0.16), in: Capsule())
                            .foregroundStyle(priorityTint)
                    }

                    Text(affectedLabel)
                        .font(.caption)
                        .foregroundStyle(.secondary)

                    if !item.detail.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                        Text(item.detail)
                            .font(.callout)
                            .textSelection(.enabled)
                    }
                }

                Spacer()
            }

            HStack(alignment: .center, spacing: 10) {
                Label(item.kind.title, systemImage: item.kind.systemImage)
                    .font(.caption)
                    .foregroundStyle(.secondary)

                SafetyPill(label: UIStrings.readOnlyPreview, isBlocked: item.readOnly)
                SafetyPill(label: UIStrings.executionBlocked, isBlocked: item.scriptExecutionBlocked)
                SafetyPill(label: UIStrings.cleanupAIBlocked, isBlocked: item.aiProviderCallBlocked)
                SafetyPill(label: UIStrings.cleanupCredentialsBlocked, isBlocked: item.credentialStorageBlocked)

                Spacer()

                Button(item.nextActionLabel) {
                    onOpen()
                }
                .controlSize(.small)
                .help(UIStrings.cleanupOpenExistingDetailHelp)
            }
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }

    private var affectedLabel: String {
        let values = [
            item.skillName?.trimmingCharacters(in: .whitespacesAndNewlines),
            item.agent.map(DisplayText.agent),
            item.skillScope.map(DisplayText.scope),
        ]
        let label = values.compactMap { value -> String? in
            guard let value, !value.isEmpty else { return nil }
            return value
        }.joined(separator: " · ")
        return label.isEmpty ? UIStrings.unknown : label
    }

    private var priorityTint: Color {
        switch item.priority {
        case .critical, .high:
            return .red
        case .medium:
            return .orange
        case .low:
            return .blue
        case .info, .unknown:
            return .secondary
        }
    }
}

private struct CleanupNoticeCard: View {
    let message: String

    var body: some View {
        Label(message, systemImage: "info.circle")
            .font(.callout)
            .foregroundStyle(.secondary)
            .padding()
            .frame(maxWidth: .infinity, alignment: .leading)
            .nativePanelSurface()
    }
}

private struct CleanupEmptyCard: View {
    let title: String
    let message: String

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Label(title, systemImage: "checkmark.seal")
                .font(.headline)
            Text(message)
                .font(.callout)
                .foregroundStyle(.secondary)
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

private struct OverviewRiskPanel: View {
    let permissionSummary: PermissionSummary
    let scriptPreview: ScriptExecutionPreview?

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.text("detail.permissionScriptRisk", "Permissions & script risk"), systemImage: "shield.lefthalf.filled")
                    .font(.subheadline.bold())
                Spacer()
                Label(scriptState, systemImage: scriptPreview == nil ? "nosign" : "checkmark.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            DetailMetricGrid(minColumnWidth: 150, spacing: 8) {
                ForEach(permissionSummary.rows.prefix(5)) { row in
                    VStack(alignment: .leading, spacing: 2) {
                        Text(row.label)
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                        Text(row.value)
                            .font(.caption.bold())
                            .lineLimit(2)
                            .truncationMode(.middle)
                    }
                    .padding(9)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
                }
            }

            Text(permissionSummary.note)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.orange.opacity(0.08), in: RoundedRectangle(cornerRadius: 10))
        .overlay(alignment: .leading) {
            Rectangle()
                .fill(Color.orange)
                .frame(width: 3)
                .clipShape(RoundedRectangle(cornerRadius: 1.5))
                .padding(.vertical, 10)
        }
    }

    private var scriptState: String {
        if let preview = scriptPreview {
            return preview.executionAllowed ? UIStrings.executionBlocked : UIStrings.scriptExecutionPreviewOnly
        }
        return UIStrings.scriptExecutionPreviewOnly
    }
}

struct DetailSectionSwitcher: View {
    @Binding var selection: DetailSection

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .center, spacing: 12) {
                Label(UIStrings.detailSection, systemImage: selection.systemImage)
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)

                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 6) {
                        ForEach(DetailSection.visibleCases) { item in
                            DetailSectionTagButton(
                                item: item,
                                isSelected: selection == item,
                                action: {
                                    withAnimation(.easeInOut(duration: 0.16)) {
                                        selection = item
                                    }
                                }
                            )
                        }
                    }
                    .padding(.vertical, 1)
                }
                .frame(maxWidth: .infinity, alignment: .leading)

                Spacer()
            }

            Text(selection.summary)
                .font(.caption)
                .foregroundStyle(.secondary)
                .lineLimit(3)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

private struct DetailSectionTagButton: View {
    let item: DetailSection
    let isSelected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(spacing: 4) {
                Label(item.title, systemImage: item.systemImage)
                    .font(.caption.bold())
                    .lineLimit(1)
                    .foregroundStyle(isSelected ? Color.primary : Color.secondary)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 5)
                    .background(background, in: Capsule())
                    .overlay(
                        Capsule()
                            .stroke(isSelected ? Color.secondary.opacity(0.18) : Color.secondary.opacity(0.12), lineWidth: 1)
                    )

                Rectangle()
                    .fill(isSelected ? Color.accentColor : Color.clear)
                    .frame(height: 2)
                    .clipShape(Capsule())
                    .padding(.horizontal, 10)
            }
        }
        .buttonStyle(.plain)
        .accessibilityLabel(item.title)
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }

    private var background: some ShapeStyle {
        isSelected ? AnyShapeStyle(Color(nsColor: .selectedContentBackgroundColor).opacity(0.12)) : AnyShapeStyle(Color.agentCopilotPanelBackground)
    }
}
