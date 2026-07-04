import AppKit
import SwiftUI

struct RemediationPlanPanel: View {
    let result: RemediationPlanResult?
    let isPlanning: Bool
    let onPlan: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.remediationPlanTitle, systemImage: "wrench.and.screwdriver")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.remediationPlanBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            HStack(spacing: 8) {
                Button {
                    onPlan()
                } label: {
                    Label(UIStrings.remediationPlanAction, systemImage: "list.bullet.clipboard")
                }
                .disabled(isPlanning)

                if isPlanning {
                    Label(UIStrings.llmPreparing, systemImage: "hourglass")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
            }

            if let result {
                RemediationPlanResultView(result: result)
            } else {
                Label(UIStrings.remediationPlanNoResult, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            Label(UIStrings.llmReviewNoActions, systemImage: "nosign")
                .font(.callout)
                .foregroundStyle(.secondary)
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

struct RemediationPlanResultView: View {
    let result: RemediationPlanResult

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            DetailMetricGrid {
                SummaryChip(title: UIStrings.remediationPlanItems, value: "\(itemCount)", systemImage: "list.bullet")
                SummaryChip(title: UIStrings.remediationPlanCritical, value: "\(criticalCount)", systemImage: "exclamationmark.octagon")
                SummaryChip(title: UIStrings.cleanupPriorityHigh, value: "\(highCount)", systemImage: "exclamationmark.triangle")
                SummaryChip(title: UIStrings.cleanupPriorityMedium, value: "\(mediumCount)", systemImage: "circle.lefthalf.filled")
                SummaryChip(title: UIStrings.remediationPlanQuickWins, value: "\(quickWinCount)", systemImage: "bolt")
                SummaryChip(title: UIStrings.knowledgeBlockerNotes, value: "\(blockerCount)", systemImage: "lock.trianglebadge.exclamationmark")
                SummaryChip(title: UIStrings.knowledgeGapNotes, value: "\(gapCount)", systemImage: "puzzlepiece.extension")
                SummaryChip(title: UIStrings.remediationPlanAmbiguity, value: "\(ambiguityCount)", systemImage: "arrow.triangle.branch")
                SummaryChip(title: UIStrings.remediationPlanDrift, value: "\(driftCount)", systemImage: "clock.arrow.circlepath")
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.routingAccuracyGeneratedBy, value: result.generatedBy)
                MetadataRow(label: UIStrings.routingAccuracyCatalog, value: result.catalogAvailable ? UIStrings.routingAccuracyAvailable : UIStrings.routingAccuracyUnavailableShort)
                MetadataRow(label: UIStrings.agent, value: agentFilterLabel)
                if let projectRoot = result.filters.projectRoot, !projectRoot.isEmpty {
                    PrivacyPathRow(label: UIStrings.project, path: projectRoot)
                }
                if let workspace = result.filters.workspace, !workspace.isEmpty {
                    MetadataRow(label: UIStrings.workspaceReadinessTitle, value: workspace)
                }
                if let taskText = result.filters.taskText, !taskText.isEmpty {
                    MetadataRow(label: UIStrings.taskBenchmarkTaskPlaceholder, value: taskText)
                }
                if let limit = result.filters.limit {
                    MetadataRow(label: UIStrings.text("filter.limit", "Limit"), value: "\(limit)")
                }
                MetadataRow(label: UIStrings.remediationPlanGuidanceOnly, value: result.filters.includeGuidanceOnly ? UIStrings.stateEnabled : UIStrings.stateDisabled)
                if let promptRequest = result.promptRequest {
                    MetadataRow(label: UIStrings.routingAccuracyPromptRequest, value: promptRequestLabel(promptRequest))
                }
            }

            if !result.summary.summaryText.isEmpty {
                Text(result.summary.summaryText)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            RemediationPriorityList(rows: result.priorityRows)
            RemediationPlanItemList(items: result.items)
            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: result.gapNotes, systemImage: "puzzlepiece.extension")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: result.blockerNotes, systemImage: "exclamationmark.octagon")
            CrossAgentReadinessEvidenceList(evidence: result.evidenceReferences)
            StaleDriftSafetyList(safety: result.safetyFlags)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
    }

    private var itemCount: Int {
        result.summary.totalCount > 0 ? result.summary.totalCount : result.items.count
    }

    private var criticalCount: Int {
        result.summary.criticalCount > 0 ? result.summary.criticalCount : countItems(matching: "critical")
    }

    private var highCount: Int {
        result.summary.highCount > 0 ? result.summary.highCount : countItems(matching: "high")
    }

    private var mediumCount: Int {
        result.summary.mediumCount > 0 ? result.summary.mediumCount : countItems(matching: "medium")
    }

    private var quickWinCount: Int {
        result.summary.quickWinCount > 0 ? result.summary.quickWinCount : result.items.filter { item in
            item.category.localizedCaseInsensitiveContains("quick")
                || item.suggestedAction.localizedCaseInsensitiveContains("quick")
        }.count
    }

    private var blockerCount: Int {
        result.summary.blockerCount > 0 ? result.summary.blockerCount : result.blockerNotes.count + result.items.reduce(0) { $0 + $1.blockerNotes.count }
    }

    private var gapCount: Int {
        result.summary.gapCount > 0 ? result.summary.gapCount : result.gapNotes.count + result.items.reduce(0) { $0 + $1.gapNotes.count }
    }

    private var ambiguityCount: Int {
        result.summary.ambiguityCount > 0 ? result.summary.ambiguityCount : result.items.filter { item in
            item.category.localizedCaseInsensitiveContains("ambigu")
                || item.rationale.localizedCaseInsensitiveContains("ambigu")
        }.count
    }

    private var driftCount: Int {
        result.summary.driftCount > 0 ? result.summary.driftCount : result.items.filter { item in
            item.category.localizedCaseInsensitiveContains("drift")
                || item.category.localizedCaseInsensitiveContains("stale")
        }.count
    }

    private var agentFilterLabel: String {
        if !result.filters.agents.isEmpty {
            return result.filters.agents.map(DisplayText.agent).joined(separator: ", ")
        }
        return result.filters.agent.map(DisplayText.agent) ?? UIStrings.text("health.allAgents", "All Agents")
    }

    private func countItems(matching priority: String) -> Int {
        result.items.filter { item in
            item.priority.localizedCaseInsensitiveContains(priority)
        }.count
    }

    private func promptRequestLabel(_ promptRequest: RoutingAccuracyPromptRequest) -> String {
        let state = promptRequest.enabled ? UIStrings.llmEnabled : UIStrings.llmDisabled
        let copy = promptRequest.copyOnly ? UIStrings.llmPromptCopyOnly : UIStrings.llmSkillAnalysisEnabledUnsafe
        return "\(promptRequest.requestKind) · \(state) · \(copy)"
    }
}

struct RemediationPriorityList: View {
    let rows: [RemediationPlanPriorityRow]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.remediationPlanPriorities)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if rows.isEmpty {
                Text(UIStrings.remediationPlanNoPriorities)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 220), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(rows.prefix(6)) { row in
                        VStack(alignment: .leading, spacing: 6) {
                            HStack(alignment: .firstTextBaseline) {
                                Text(row.title)
                                    .font(.caption.bold())
                                    .lineLimit(1)
                                Spacer()
                                Text("\(row.count)")
                                    .font(.caption2.bold())
                                    .foregroundStyle(.secondary)
                            }
                            MetadataRow(label: UIStrings.cleanupFilterPriority, value: row.priority)
                            if !row.rationale.isEmpty {
                                Text(row.rationale)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                    .textSelection(.enabled)
                            }
                            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: row.evidenceRefs, systemImage: "checklist")
                        }
                        .padding(8)
                        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
                    }
                }
            }
        }
    }
}

struct RemediationPlanItemList: View {
    let items: [RemediationPlanItem]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.remediationPlanItems)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if items.isEmpty {
                Text(UIStrings.remediationPlanNoItems)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 360), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(items.prefix(10)) { item in
                        RemediationPlanItemCard(item: item)
                    }
                }
            }
        }
    }
}

struct RemediationPlanItemCard: View {
    let item: RemediationPlanItem

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Label(item.title, systemImage: iconName)
                    .font(.callout.bold())
                    .lineLimit(1)
                Spacer()
                Text(item.priority)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
            }

            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                MetadataRow(label: UIStrings.remediationPlanCategory, value: item.category)
                MetadataRow(label: UIStrings.state, value: item.status)
                MetadataRow(label: UIStrings.remediationPlanGuidanceOnly, value: item.guidanceOnly ? UIStrings.stateEnabled : UIStrings.stateDisabled)
                if let agent = item.agent, !agent.isEmpty {
                    MetadataRow(label: UIStrings.agent, value: DisplayText.agent(agent))
                }
                if let capability = item.capability, !capability.isEmpty {
                    MetadataRow(label: UIStrings.capabilityTaxonomyCapability, value: capability)
                }
                if let nextArea = item.nextArea, !nextArea.isEmpty {
                    MetadataRow(label: UIStrings.remediationPlanNextArea, value: nextArea)
                }
            }

            if let skill = item.skill {
                CapabilitySkillList(skills: [skill])
            }

            if !item.rationale.isEmpty {
                Text(item.rationale)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            Label(item.suggestedAction, systemImage: "lightbulb")
                .font(.caption)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            if let impact = item.impact, !impact.isEmpty {
                Label(impact, systemImage: "chart.line.uptrend.xyaxis")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: item.gapNotes, systemImage: "puzzlepiece.extension")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: item.blockerNotes, systemImage: "exclamationmark.octagon")
            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: item.evidenceRefs, systemImage: "checklist")
            RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: item.safetyFlags, systemImage: "checkmark.shield")
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
    }

    private var iconName: String {
        if item.category.localizedCaseInsensitiveContains("drift") || item.category.localizedCaseInsensitiveContains("stale") {
            return "clock.arrow.circlepath"
        }
        if item.category.localizedCaseInsensitiveContains("ambigu") {
            return "arrow.triangle.branch"
        }
        if item.category.localizedCaseInsensitiveContains("block") {
            return "lock.trianglebadge.exclamationmark"
        }
        if item.category.localizedCaseInsensitiveContains("gap") {
            return "puzzlepiece.extension"
        }
        return "wrench.and.screwdriver"
    }
}

struct RemediationPreviewDraftsPanel: View {
    let result: RemediationPreviewDraftsResult?
    let isPreviewing: Bool
    let onPreview: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.fixPreviewTitle, systemImage: "doc.text.magnifyingglass")
                    .font(.headline)
                Spacer()
                Label(UIStrings.llmPromptCopyOnly, systemImage: "doc.on.doc")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.fixPreviewBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            HStack(spacing: 8) {
                Button {
                    onPreview()
                } label: {
                    Label(UIStrings.fixPreviewAction, systemImage: "wand.and.stars")
                }
                .disabled(isPreviewing)

                if isPreviewing {
                    Label(UIStrings.llmPreparing, systemImage: "hourglass")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
            }

            if let result {
                RemediationPreviewDraftsResultView(result: result)
            } else {
                Label(UIStrings.fixPreviewNoResult, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            Label(UIStrings.fixPreviewCopyOnlyBoundary, systemImage: "nosign")
                .font(.callout)
                .foregroundStyle(.secondary)
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

struct RemediationPreviewDraftsResultView: View {
    let result: RemediationPreviewDraftsResult

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            DetailMetricGrid {
                SummaryChip(title: UIStrings.fixPreviewDrafts, value: "\(draftCount)", systemImage: "doc.text")
                SummaryChip(title: UIStrings.fixPreviewFrontmatter, value: "\(frontmatterCount)", systemImage: "list.bullet.rectangle")
                SummaryChip(title: UIStrings.fixPreviewDescription, value: "\(descriptionCount)", systemImage: "text.alignleft")
                SummaryChip(title: UIStrings.fixPreviewPermissions, value: "\(permissionsCount)", systemImage: "lock.shield")
                SummaryChip(title: UIStrings.fixPreviewDependency, value: "\(dependencyCount)", systemImage: "shippingbox")
                SummaryChip(title: UIStrings.fixPreviewPolicy, value: "\(policyCount)", systemImage: "checkmark.shield")
                SummaryChip(title: UIStrings.knowledgeBlockerNotes, value: "\(blockerCount)", systemImage: "lock.trianglebadge.exclamationmark")
                SummaryChip(title: UIStrings.llmPromptCopyOnly, value: "\(copyOnlyCount)", systemImage: "doc.on.doc")
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.routingAccuracyGeneratedBy, value: result.generatedBy)
                MetadataRow(label: UIStrings.routingAccuracyCatalog, value: result.catalogAvailable ? UIStrings.routingAccuracyAvailable : UIStrings.routingAccuracyUnavailableShort)
                MetadataRow(label: UIStrings.agent, value: agentFilterLabel)
                if let taskText = result.filters.taskText, !taskText.isEmpty {
                    MetadataRow(label: UIStrings.taskBenchmarkTaskPlaceholder, value: taskText)
                }
                if let limit = result.filters.limit {
                    MetadataRow(label: UIStrings.text("filter.limit", "Limit"), value: "\(limit)")
                }
                if let promptRequest = result.promptRequest {
                    MetadataRow(label: UIStrings.routingAccuracyPromptRequest, value: promptRequestLabel(promptRequest))
                }
            }

            if !result.summary.summaryText.isEmpty {
                Text(result.summary.summaryText)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            RemediationPreviewDraftGroupList(groups: draftGroups)
            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: result.gapNotes, systemImage: "puzzlepiece.extension")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: result.blockerNotes, systemImage: "exclamationmark.octagon")
            CrossAgentReadinessEvidenceList(evidence: result.evidenceReferences)
            StaleDriftSafetyList(safety: result.safetyFlags)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
    }

    private var draftCount: Int {
        result.summary.totalCount > 0 ? result.summary.totalCount : result.draftItems.count
    }

    private var frontmatterCount: Int {
        result.summary.frontmatterCount > 0 ? result.summary.frontmatterCount : countDrafts(matching: "frontmatter")
    }

    private var descriptionCount: Int {
        result.summary.descriptionCount > 0 ? result.summary.descriptionCount : countDrafts(matching: "description")
    }

    private var permissionsCount: Int {
        result.summary.permissionsCount > 0 ? result.summary.permissionsCount : countDrafts(matching: "permission")
    }

    private var dependencyCount: Int {
        result.summary.dependencyCount > 0 ? result.summary.dependencyCount : countDrafts(matching: "depend")
    }

    private var policyCount: Int {
        result.summary.policyCount > 0 ? result.summary.policyCount : countDrafts(matching: "policy")
    }

    private var blockerCount: Int {
        result.summary.blockerCount > 0 ? result.summary.blockerCount : result.blockerNotes.count + result.draftItems.reduce(0) { $0 + $1.blockerNotes.count }
    }

    private var copyOnlyCount: Int {
        result.summary.copyOnlyCount > 0 ? result.summary.copyOnlyCount : result.draftItems.count
    }

    private var agentFilterLabel: String {
        if !result.filters.agents.isEmpty {
            return result.filters.agents.map(DisplayText.agent).joined(separator: ", ")
        }
        return result.filters.agent.map(DisplayText.agent) ?? UIStrings.text("health.allAgents", "All Agents")
    }

    private var draftGroups: [(type: String, items: [RemediationPreviewDraftItem])] {
        let grouped = Dictionary(grouping: result.draftItems, by: \.draftType)
        return grouped.keys.sorted { lhs, rhs in
            draftTypeSortIndex(lhs) < draftTypeSortIndex(rhs)
        }.map { type in
            (type: type, items: grouped[type] ?? [])
        }
    }

    private func countDrafts(matching draftType: String) -> Int {
        result.draftItems.filter { item in
            item.draftType.localizedCaseInsensitiveContains(draftType)
        }.count
    }

    private func draftTypeSortIndex(_ draftType: String) -> Int {
        let normalized = draftType.lowercased()
        if normalized.contains("frontmatter") { return 0 }
        if normalized.contains("description") { return 1 }
        if normalized.contains("permission") { return 2 }
        if normalized.contains("depend") { return 3 }
        if normalized.contains("policy") { return 4 }
        return 5
    }

    private func promptRequestLabel(_ promptRequest: RoutingAccuracyPromptRequest) -> String {
        let state = promptRequest.enabled ? UIStrings.llmEnabled : UIStrings.llmDisabled
        let copy = promptRequest.copyOnly ? UIStrings.llmPromptCopyOnly : UIStrings.llmSkillAnalysisEnabledUnsafe
        return "\(promptRequest.requestKind) · \(state) · \(copy)"
    }
}

struct RemediationPreviewDraftGroupList: View {
    let groups: [(type: String, items: [RemediationPreviewDraftItem])]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.fixPreviewDrafts)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if groups.isEmpty {
                Text(UIStrings.fixPreviewNoDrafts)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(groups, id: \.type) { group in
                    VStack(alignment: .leading, spacing: 8) {
                        Text(draftTypeLabel(group.type))
                            .font(.caption.bold())
                            .foregroundStyle(.secondary)
                        LazyVGrid(columns: [GridItem(.adaptive(minimum: 360), spacing: 8)], alignment: .leading, spacing: 8) {
                            ForEach(group.items.prefix(8)) { item in
                                RemediationPreviewDraftCard(item: item)
                            }
                        }
                    }
                }
            }
        }
    }

    private func draftTypeLabel(_ draftType: String) -> String {
        let normalized = draftType.lowercased()
        if normalized.contains("frontmatter") { return UIStrings.fixPreviewFrontmatter }
        if normalized.contains("description") { return UIStrings.fixPreviewDescription }
        if normalized.contains("permission") { return UIStrings.fixPreviewPermissions }
        if normalized.contains("depend") { return UIStrings.fixPreviewDependency }
        if normalized.contains("policy") { return UIStrings.fixPreviewPolicy }
        return draftType.isEmpty ? UIStrings.unknown : draftType
    }
}

struct RemediationPreviewDraftCard: View {
    let item: RemediationPreviewDraftItem

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Label(item.title, systemImage: iconName)
                    .font(.callout.bold())
                    .lineLimit(1)
                Spacer()
                if let confidenceLabel {
                    Text(confidenceLabel)
                        .font(.caption2.bold())
                        .foregroundStyle(.secondary)
                }
            }

            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                MetadataRow(label: UIStrings.fixPreviewDraftType, value: draftTypeLabel)
                if let agent = item.agent, !agent.isEmpty {
                    MetadataRow(label: UIStrings.agent, value: DisplayText.agent(agent))
                }
                if let findingID = item.findingID, !findingID.isEmpty {
                    MetadataRow(label: UIStrings.fixPreviewFinding, value: findingID)
                }
                if let ruleID = item.ruleID, !ruleID.isEmpty {
                    MetadataRow(label: UIStrings.knowledgeRules, value: ruleID)
                }
            }

            if let skill = item.affectedSkill {
                CapabilitySkillList(skills: [skill])
            }

            if let currentText = item.currentText, !currentText.isEmpty {
                DraftSnippetBlock(title: UIStrings.fixPreviewCurrentSnippet, text: currentText, allowsCopy: false, copyLabel: item.copyLabel)
            }

            DraftSnippetBlock(title: UIStrings.fixPreviewProposedSnippet, text: item.proposedText, allowsCopy: true, copyLabel: item.copyLabel)

            if !item.editGuidance.isEmpty {
                Label(item.editGuidance, systemImage: "pencil.and.list.clipboard")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            if !item.rationale.isEmpty {
                Text(item.rationale)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: item.evidenceRefs, systemImage: "checklist")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: item.blockerNotes, systemImage: "exclamationmark.octagon")
            RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: item.safetyFlags, systemImage: "checkmark.shield")
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
    }

    private var confidenceLabel: String? {
        if let confidenceScore = item.confidenceScore, let band = item.confidenceBand, !band.isEmpty {
            return "\(confidenceScore) · \(band)"
        }
        if let confidenceScore = item.confidenceScore {
            return "\(confidenceScore)"
        }
        return item.confidenceBand?.isEmpty == false ? item.confidenceBand : nil
    }

    private var draftTypeLabel: String {
        let normalized = item.draftType.lowercased()
        if normalized.contains("frontmatter") { return UIStrings.fixPreviewFrontmatter }
        if normalized.contains("description") { return UIStrings.fixPreviewDescription }
        if normalized.contains("permission") { return UIStrings.fixPreviewPermissions }
        if normalized.contains("depend") { return UIStrings.fixPreviewDependency }
        if normalized.contains("policy") { return UIStrings.fixPreviewPolicy }
        return item.draftType
    }

    private var iconName: String {
        let normalized = item.draftType.lowercased()
        if normalized.contains("frontmatter") { return "list.bullet.rectangle" }
        if normalized.contains("description") { return "text.alignleft" }
        if normalized.contains("permission") { return "lock.shield" }
        if normalized.contains("depend") { return "shippingbox" }
        if normalized.contains("policy") { return "checkmark.shield" }
        return "doc.text.magnifyingglass"
    }
}

struct DraftSnippetBlock: View {
    let title: String
    let text: String
    let allowsCopy: Bool
    let copyLabel: String

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Label(title, systemImage: "doc.text")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                Spacer()
                if allowsCopy && !text.isEmpty {
                    Button {
                        NSPasteboard.general.clearContents()
                        NSPasteboard.general.setString(text, forType: .string)
                    } label: {
                        Label(copyLabel.isEmpty ? UIStrings.fixPreviewCopyDraft : copyLabel, systemImage: "doc.on.doc")
                    }
                    .buttonStyle(.borderless)
                }
            }
            Text(text.isEmpty ? UIStrings.emptyPlaceholder : text)
                .font(.system(.caption, design: .monospaced))
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(8)
                .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
        }
    }
}

struct RemediationImpactPreviewPanel: View {
    let result: RemediationImpactPreviewResult?
    let isPreviewing: Bool
    let onPreview: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.impactPreviewTitle, systemImage: "chart.line.uptrend.xyaxis")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.impactPreviewBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            HStack(spacing: 8) {
                Button {
                    onPreview()
                } label: {
                    Label(UIStrings.impactPreviewAction, systemImage: "scope")
                }
                .disabled(isPreviewing)

                if isPreviewing {
                    Label(UIStrings.llmPreparing, systemImage: "hourglass")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
            }

            if let result {
                RemediationImpactPreviewResultView(result: result)
            } else {
                Label(UIStrings.impactPreviewNoResult, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            Label(UIStrings.impactPreviewNoWriteBoundary, systemImage: "nosign")
                .font(.callout)
                .foregroundStyle(.secondary)
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

struct RemediationImpactPreviewResultView: View {
    let result: RemediationImpactPreviewResult

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            DetailMetricGrid {
                SummaryChip(title: UIStrings.impactPreviewImpacts, value: "\(impactCount)", systemImage: "scope")
                SummaryChip(title: UIStrings.impactPreviewTaskImpacts, value: "\(taskImpactCount)", systemImage: "checklist")
                SummaryChip(title: UIStrings.impactPreviewAgentImpacts, value: "\(agentImpactCount)", systemImage: "person.2")
                SummaryChip(title: UIStrings.impactPreviewSkillImpacts, value: "\(skillImpactCount)", systemImage: "wrench.and.screwdriver")
                SummaryChip(title: UIStrings.impactPreviewRiskDeltas, value: "\(riskDeltaCount)", systemImage: "arrow.up.arrow.down")
                SummaryChip(title: UIStrings.impactPreviewSnapshotRollback, value: "\(snapshotRollbackCount)", systemImage: "arrow.uturn.backward.circle")
                SummaryChip(title: UIStrings.knowledgeBlockerNotes, value: "\(blockerCount)", systemImage: "lock.trianglebadge.exclamationmark")
                SummaryChip(title: UIStrings.impactPreviewNoWrite, value: "\(noWriteCount)", systemImage: "lock.shield")
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.routingAccuracyGeneratedBy, value: result.generatedBy)
                MetadataRow(label: UIStrings.routingAccuracyCatalog, value: result.catalogAvailable ? UIStrings.routingAccuracyAvailable : UIStrings.routingAccuracyUnavailableShort)
                MetadataRow(label: UIStrings.agent, value: agentFilterLabel)
                if let projectRoot = result.filters.projectRoot, !projectRoot.isEmpty {
                    PrivacyPathRow(label: UIStrings.project, path: projectRoot)
                }
                if let workspace = result.filters.workspace, !workspace.isEmpty {
                    MetadataRow(label: UIStrings.workspaceReadinessTitle, value: workspace)
                }
                if let taskText = result.filters.taskText, !taskText.isEmpty {
                    MetadataRow(label: UIStrings.taskBenchmarkTaskPlaceholder, value: taskText)
                }
                if let limit = result.filters.limit {
                    MetadataRow(label: UIStrings.text("filter.limit", "Limit"), value: "\(limit)")
                }
                if let promptRequest = result.promptRequest {
                    MetadataRow(label: UIStrings.routingAccuracyPromptRequest, value: promptRequestLabel(promptRequest))
                }
            }

            if !result.summary.summaryText.isEmpty {
                Text(result.summary.summaryText)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            RemediationImpactRowGroupList(title: UIStrings.impactPreviewImpacts, rows: result.impactRows, empty: UIStrings.impactPreviewNoImpacts, systemImage: "scope")
            RemediationImpactRowGroupList(title: UIStrings.impactPreviewTaskImpacts, rows: result.taskImpactRows, empty: UIStrings.impactPreviewNoTaskImpacts, systemImage: "checklist")
            RemediationImpactRowGroupList(title: UIStrings.impactPreviewAgentImpacts, rows: result.agentImpactRows, empty: UIStrings.impactPreviewNoAgentImpacts, systemImage: "person.2")
            RemediationImpactRowGroupList(title: UIStrings.impactPreviewSkillImpacts, rows: result.skillImpactRows, empty: UIStrings.impactPreviewNoSkillImpacts, systemImage: "wrench.and.screwdriver")
            RemediationImpactRowGroupList(title: UIStrings.impactPreviewRiskDeltas, rows: result.riskDeltaRows, empty: UIStrings.impactPreviewNoRiskDeltas, systemImage: "arrow.up.arrow.down")
            RemediationImpactRowGroupList(title: UIStrings.impactPreviewSnapshotRollback, rows: result.snapshotRollbackRows, empty: UIStrings.impactPreviewNoSnapshotRollback, systemImage: "arrow.uturn.backward.circle")
            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: result.gapNotes, systemImage: "puzzlepiece.extension")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: result.blockerNotes, systemImage: "exclamationmark.octagon")
            CrossAgentReadinessEvidenceList(evidence: result.evidenceReferences)
            StaleDriftSafetyList(safety: result.safetyFlags)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
    }

    private var allRows: [RemediationImpactRow] {
        result.impactRows + result.taskImpactRows + result.agentImpactRows + result.skillImpactRows + result.riskDeltaRows + result.snapshotRollbackRows
    }

    private var impactCount: Int {
        result.summary.totalCount > 0 ? result.summary.totalCount : allRows.count
    }

    private var taskImpactCount: Int {
        result.summary.taskImpactCount > 0 ? result.summary.taskImpactCount : result.taskImpactRows.count
    }

    private var agentImpactCount: Int {
        result.summary.agentImpactCount > 0 ? result.summary.agentImpactCount : result.agentImpactRows.count
    }

    private var skillImpactCount: Int {
        result.summary.skillImpactCount > 0 ? result.summary.skillImpactCount : result.skillImpactRows.count
    }

    private var riskDeltaCount: Int {
        result.summary.riskDeltaCount > 0 ? result.summary.riskDeltaCount : result.riskDeltaRows.count
    }

    private var snapshotRollbackCount: Int {
        result.summary.snapshotRollbackCount > 0 ? result.summary.snapshotRollbackCount : result.snapshotRollbackRows.count
    }

    private var blockerCount: Int {
        result.summary.blockerCount > 0 ? result.summary.blockerCount : result.blockerNotes.count
    }

    private var noWriteCount: Int {
        result.summary.noWriteCount > 0 ? result.summary.noWriteCount : result.safetyFlags.notes.filter { note in
            note.localizedCaseInsensitiveContains("write") || note.localizedCaseInsensitiveContains("read")
        }.count
    }

    private var agentFilterLabel: String {
        if !result.filters.agents.isEmpty {
            return result.filters.agents.map(DisplayText.agent).joined(separator: ", ")
        }
        return result.filters.agent.map(DisplayText.agent) ?? UIStrings.text("health.allAgents", "All Agents")
    }

    private func promptRequestLabel(_ promptRequest: RoutingAccuracyPromptRequest) -> String {
        let state = promptRequest.enabled ? UIStrings.llmEnabled : UIStrings.llmDisabled
        let copy = promptRequest.copyOnly ? UIStrings.llmPromptCopyOnly : UIStrings.llmSkillAnalysisEnabledUnsafe
        return "\(promptRequest.requestKind) · \(state) · \(copy)"
    }
}

struct RemediationImpactRowGroupList: View {
    let title: String
    let rows: [RemediationImpactRow]
    let empty: String
    let systemImage: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(title, systemImage: systemImage)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if rows.isEmpty {
                Text(empty)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 320), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(rows.prefix(8)) { row in
                        RemediationImpactRowCard(row: row, fallbackIcon: systemImage)
                    }
                }
            }
        }
    }
}

struct RemediationImpactRowCard: View {
    let row: RemediationImpactRow
    let fallbackIcon: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Label(row.title, systemImage: iconName)
                    .font(.callout.bold())
                    .lineLimit(1)
                Spacer()
                Text(row.severity)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
            }

            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                MetadataRow(label: UIStrings.remediationPlanCategory, value: row.category)
                if let agent = row.agent, !agent.isEmpty {
                    MetadataRow(label: UIStrings.agent, value: DisplayText.agent(agent))
                }
                if let before = row.before, !before.isEmpty {
                    MetadataRow(label: UIStrings.impactPreviewBefore, value: before)
                }
                if let after = row.after, !after.isEmpty {
                    MetadataRow(label: UIStrings.impactPreviewAfter, value: after)
                }
                if let delta = row.delta, !delta.isEmpty {
                    MetadataRow(label: UIStrings.impactPreviewDelta, value: delta)
                }
            }

            if let skill = row.skill {
                CapabilitySkillList(skills: [skill])
            }

            if !row.impact.isEmpty {
                Label(row.impact, systemImage: "chart.line.uptrend.xyaxis")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            if !row.rationale.isEmpty {
                Text(row.rationale)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: row.evidenceRefs, systemImage: "checklist")
            RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: row.safetyFlags, systemImage: "checkmark.shield")
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
    }

    private var iconName: String {
        let normalized = row.category.lowercased()
        if normalized.contains("risk") { return "arrow.up.arrow.down" }
        if normalized.contains("snapshot") || normalized.contains("rollback") { return "arrow.uturn.backward.circle" }
        if normalized.contains("agent") { return "person.2" }
        if normalized.contains("skill") { return "wrench.and.screwdriver" }
        if normalized.contains("task") { return "checklist" }
        return fallbackIcon
    }
}

struct RemediationBatchReviewPanel: View {
    let result: RemediationBatchReviewResult?
    let isReviewing: Bool
    let onReview: (RemediationBatchReviewOptions) -> Void
    @State private var options = RemediationBatchReviewOptions()

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.remediationBatchReviewTitle, systemImage: "rectangle.stack.badge.checkmark")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.remediationBatchReviewBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            RemediationBatchReviewControls(options: $options)

            HStack(spacing: 8) {
                Button {
                    onReview(options)
                } label: {
                    Label(UIStrings.remediationBatchReviewAction, systemImage: "rectangle.stack.badge.checkmark")
                }
                .disabled(isReviewing || options.dimensions.isEmpty)

                if isReviewing {
                    Label(UIStrings.llmPreparing, systemImage: "hourglass")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
            }

            if let result {
                RemediationBatchReviewResultView(result: result)
            } else {
                Label(UIStrings.remediationBatchReviewNoResult, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            Label(UIStrings.remediationBatchReviewNoWriteBoundary, systemImage: "nosign")
                .font(.callout)
                .foregroundStyle(.secondary)
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

struct RemediationBatchReviewControls: View {
    @Binding var options: RemediationBatchReviewOptions

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(UIStrings.remediationBatchReviewControls, systemImage: "slider.horizontal.3")
                .font(.caption.bold())
                .foregroundStyle(.secondary)

            LazyVGrid(columns: [GridItem(.adaptive(minimum: 128), spacing: 8)], alignment: .leading, spacing: 6) {
                Toggle(UIStrings.remediationBatchReviewControlTask, isOn: $options.includeTask)
                Toggle(UIStrings.remediationBatchReviewControlRisk, isOn: $options.includeRisk)
                Toggle(UIStrings.remediationBatchReviewControlRule, isOn: $options.includeRule)
                Toggle(UIStrings.remediationBatchReviewControlAgent, isOn: $options.includeAgent)
                Toggle(UIStrings.remediationBatchReviewControlWorkspace, isOn: $options.includeWorkspace)
                Toggle(UIStrings.remediationBatchReviewControlBlocked, isOn: $options.includeBlocked)
            }
            .toggleStyle(.checkbox)
            .font(.callout)
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
    }
}

struct RemediationBatchReviewResultView: View {
    let result: RemediationBatchReviewResult

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            DetailMetricGrid {
                SummaryChip(title: UIStrings.remediationBatchReviewItems, value: "\(itemCount)", systemImage: "checklist")
                SummaryChip(title: UIStrings.remediationBatchReviewGroups, value: "\(groupCount)", systemImage: "rectangle.stack")
                SummaryChip(title: UIStrings.remediationBatchReviewTaskRows, value: "\(taskCount)", systemImage: "text.badge.checkmark")
                SummaryChip(title: UIStrings.remediationBatchReviewRiskRows, value: "\(riskCount)", systemImage: "exclamationmark.triangle")
                SummaryChip(title: UIStrings.remediationBatchReviewRuleRows, value: "\(ruleCount)", systemImage: "ruler")
                SummaryChip(title: UIStrings.remediationBatchReviewAgentRows, value: "\(agentCount)", systemImage: "person.2")
                SummaryChip(title: UIStrings.remediationBatchReviewWorkspaceRows, value: "\(workspaceCount)", systemImage: "folder")
                SummaryChip(title: UIStrings.knowledgeBlockerNotes, value: "\(blockerCount)", systemImage: "lock.trianglebadge.exclamationmark")
                SummaryChip(title: UIStrings.remediationBatchReviewSafeNextSteps, value: "\(safeNextStepCount)", systemImage: "arrow.right.circle")
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.routingAccuracyGeneratedBy, value: result.generatedBy)
                MetadataRow(label: UIStrings.routingAccuracyCatalog, value: result.catalogAvailable ? UIStrings.routingAccuracyAvailable : UIStrings.routingAccuracyUnavailableShort)
                MetadataRow(label: UIStrings.agent, value: agentFilterLabel)
                if !result.filters.dimensions.isEmpty {
                    MetadataRow(label: UIStrings.remediationBatchReviewDimensions, value: result.filters.dimensions.joined(separator: ", "))
                }
                if !result.filters.riskLevels.isEmpty {
                    MetadataRow(label: UIStrings.remediationBatchReviewRiskLevels, value: result.filters.riskLevels.joined(separator: ", "))
                }
                if !result.filters.ruleIDs.isEmpty {
                    MetadataRow(label: UIStrings.remediationBatchReviewRuleIDs, value: result.filters.ruleIDs.joined(separator: ", "))
                }
                if let projectRoot = result.filters.projectRoot, !projectRoot.isEmpty {
                    PrivacyPathRow(label: UIStrings.project, path: projectRoot)
                }
                if let workspace = result.filters.workspace, !workspace.isEmpty {
                    MetadataRow(label: UIStrings.workspaceReadinessTitle, value: workspace)
                }
                if let taskText = result.filters.taskText, !taskText.isEmpty {
                    MetadataRow(label: UIStrings.taskBenchmarkTaskPlaceholder, value: taskText)
                }
                if let limit = result.filters.limit {
                    MetadataRow(label: UIStrings.text("filter.limit", "Limit"), value: "\(limit)")
                }
                MetadataRow(label: UIStrings.remediationBatchReviewControlBlocked, value: result.filters.includeBlocked ? UIStrings.stateEnabled : UIStrings.stateDisabled)
                if let promptRequest = result.promptRequest {
                    MetadataRow(label: UIStrings.routingAccuracyPromptRequest, value: promptRequestLabel(promptRequest))
                }
            }

            if !result.summary.summaryText.isEmpty {
                Text(result.summary.summaryText)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            RoutingInlineList(title: UIStrings.remediationBatchReviewSafeNextSteps, empty: UIStrings.remediationBatchReviewSafeNextStepFallback, values: result.safeNextStepLabels, systemImage: "arrow.right.circle")
            RemediationBatchReviewGroupList(groups: result.groups)
            RemediationBatchReviewItemList(title: UIStrings.remediationBatchReviewItems, items: result.items, empty: UIStrings.remediationBatchReviewNoItems)
            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: result.gapNotes, systemImage: "puzzlepiece.extension")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: result.blockerNotes, systemImage: "exclamationmark.octagon")
            CrossAgentReadinessEvidenceList(evidence: result.evidenceReferences)
            StaleDriftSafetyList(safety: result.safetyFlags)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
    }

    private var allItems: [RemediationBatchReviewItem] {
        result.items + result.groups.flatMap(\.items)
    }

    private var itemCount: Int {
        result.summary.totalCount > 0 ? result.summary.totalCount : allItems.count
    }

    private var groupCount: Int {
        result.summary.groupCount > 0 ? result.summary.groupCount : result.groups.count
    }

    private var taskCount: Int {
        result.summary.taskCount > 0 ? result.summary.taskCount : countItems(matching: "task")
    }

    private var riskCount: Int {
        result.summary.riskCount > 0 ? result.summary.riskCount : countItems(matching: "risk")
    }

    private var ruleCount: Int {
        result.summary.ruleCount > 0 ? result.summary.ruleCount : allItems.filter { item in
            !(item.ruleID ?? "").isEmpty || item.category.localizedCaseInsensitiveContains("rule")
        }.count
    }

    private var agentCount: Int {
        result.summary.agentCount > 0 ? result.summary.agentCount : allItems.filter { item in
            !(item.agent ?? "").isEmpty || item.category.localizedCaseInsensitiveContains("agent")
        }.count
    }

    private var workspaceCount: Int {
        result.summary.workspaceCount > 0 ? result.summary.workspaceCount : allItems.filter { item in
            !(item.workspace ?? "").isEmpty || item.category.localizedCaseInsensitiveContains("workspace")
        }.count
    }

    private var blockerCount: Int {
        result.summary.blockerCount > 0 ? result.summary.blockerCount : result.blockerNotes.count + allItems.reduce(0) { $0 + $1.blockerNotes.count }
    }

    private var safeNextStepCount: Int {
        result.summary.safeNextStepCount > 0 ? result.summary.safeNextStepCount : result.safeNextStepLabels.count + allItems.filter { !$0.safeNextStepLabel.isEmpty }.count
    }

    private var agentFilterLabel: String {
        if !result.filters.agents.isEmpty {
            return result.filters.agents.map(DisplayText.agent).joined(separator: ", ")
        }
        return result.filters.agent.map(DisplayText.agent) ?? UIStrings.text("health.allAgents", "All Agents")
    }

    private func countItems(matching value: String) -> Int {
        allItems.filter { item in
            item.category.localizedCaseInsensitiveContains(value)
                || (item.reviewArea ?? "").localizedCaseInsensitiveContains(value)
        }.count
    }

    private func promptRequestLabel(_ promptRequest: RoutingAccuracyPromptRequest) -> String {
        let state = promptRequest.enabled ? UIStrings.llmEnabled : UIStrings.llmDisabled
        let copy = promptRequest.copyOnly ? UIStrings.llmPromptCopyOnly : UIStrings.llmSkillAnalysisEnabledUnsafe
        return "\(promptRequest.requestKind) · \(state) · \(copy)"
    }
}

struct RemediationBatchReviewGroupList: View {
    let groups: [RemediationBatchReviewGroup]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.remediationBatchReviewGroups)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if groups.isEmpty {
                Text(UIStrings.remediationBatchReviewNoGroups)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 360), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(groups.prefix(8)) { group in
                        RemediationBatchReviewGroupCard(group: group)
                    }
                }
            }
        }
    }
}

struct RemediationBatchReviewGroupCard: View {
    let group: RemediationBatchReviewGroup

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Label(group.title, systemImage: iconName)
                    .font(.callout.bold())
                    .lineLimit(1)
                Spacer()
                Text(group.priority)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
            }

            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                MetadataRow(label: UIStrings.remediationPlanCategory, value: group.category)
                MetadataRow(label: UIStrings.remediationBatchReviewItems, value: "\(group.items.count)")
            }

            if !group.summary.isEmpty {
                PrivacyEvidenceText(value: group.summary, font: .caption, lineLimit: nil)
            }

            RoutingInlineList(title: UIStrings.remediationBatchReviewSafeNextSteps, empty: UIStrings.remediationBatchReviewSafeNextStepFallback, values: group.safeNextStepLabels, systemImage: "arrow.right.circle")
            RemediationBatchReviewItemList(title: UIStrings.remediationBatchReviewItems, items: group.items, empty: UIStrings.remediationBatchReviewNoItems)
            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: group.gapNotes, systemImage: "puzzlepiece.extension")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: group.blockerNotes, systemImage: "exclamationmark.octagon")
            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: group.evidenceRefs, systemImage: "checklist")
            RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: group.safetyFlags, systemImage: "checkmark.shield")
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
    }

    private var iconName: String {
        batchReviewIcon(for: group.category)
    }
}

struct RemediationBatchReviewItemList: View {
    let title: String
    let items: [RemediationBatchReviewItem]
    let empty: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if items.isEmpty {
                Text(empty)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 320), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(items.prefix(10)) { item in
                        RemediationBatchReviewItemCard(item: item)
                    }
                }
            }
        }
    }
}

struct RemediationBatchReviewItemCard: View {
    let item: RemediationBatchReviewItem

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Label(item.title, systemImage: batchReviewIcon(for: item.category))
                    .font(.callout.bold())
                    .lineLimit(1)
                Spacer()
                Text(item.priority)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
            }

            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                MetadataRow(label: UIStrings.remediationPlanCategory, value: item.category)
                MetadataRow(label: UIStrings.state, value: item.status)
                if let reviewArea = item.reviewArea, !reviewArea.isEmpty {
                    MetadataRow(label: UIStrings.remediationBatchReviewReviewArea, value: reviewArea)
                }
                if let agent = item.agent, !agent.isEmpty {
                    MetadataRow(label: UIStrings.agent, value: DisplayText.agent(agent))
                }
                if let workspace = item.workspace, !workspace.isEmpty {
                    MetadataRow(label: UIStrings.workspaceReadinessTitle, value: workspace)
                }
                if let ruleID = item.ruleID, !ruleID.isEmpty {
                    MetadataRow(label: UIStrings.remediationBatchReviewRuleIDs, value: ruleID)
                }
                if let riskLevel = item.riskLevel, !riskLevel.isEmpty {
                    MetadataRow(label: UIStrings.remediationBatchReviewRiskLevels, value: riskLevel)
                }
            }

            if let skill = item.skill {
                CapabilitySkillList(skills: [skill])
            }

            if let taskText = item.taskText, !taskText.isEmpty {
                Label(taskText, systemImage: "text.badge.checkmark")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            if !item.rationale.isEmpty {
                Text(item.rationale)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            Label(item.safeNextStepLabel, systemImage: "arrow.right.circle")
                .font(.caption)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: item.gapNotes, systemImage: "puzzlepiece.extension")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: item.blockerNotes, systemImage: "exclamationmark.octagon")
            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: item.evidenceRefs, systemImage: "checklist")
            RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: item.safetyFlags, systemImage: "checkmark.shield")
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
    }
}

func batchReviewIcon(for category: String) -> String {
    let normalized = category.lowercased()
    if normalized.contains("risk") { return "exclamationmark.triangle" }
    if normalized.contains("rule") { return "ruler" }
    if normalized.contains("agent") { return "person.2" }
    if normalized.contains("workspace") { return "folder" }
    if normalized.contains("task") { return "text.badge.checkmark" }
    if normalized.contains("block") { return "lock.trianglebadge.exclamationmark" }
    return "checklist"
}

struct RemediationHistoryPanel: View {
    let result: RemediationHistoryResult?
    let recordResult: RemediationHistoryRecordResult?
    let isLoading: Bool
    let isRecording: Bool
    let onLoad: () -> Void
    let onRecord: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.remediationHistoryTitle, systemImage: "clock.arrow.circlepath")
                    .font(.headline)
                Spacer()
                Label(UIStrings.remediationHistoryRecorded, systemImage: "archivebox")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.remediationHistoryBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            HStack(spacing: 8) {
                Button {
                    onLoad()
                } label: {
                    Label(UIStrings.remediationHistoryLoadAction, systemImage: "clock.arrow.circlepath")
                }
                .disabled(isLoading || isRecording)

                Button {
                    onRecord()
                } label: {
                    Label(UIStrings.remediationHistoryRecordAction, systemImage: "archivebox")
                }
                .disabled(isLoading || isRecording)

                if isLoading || isRecording {
                    Label(UIStrings.llmPreparing, systemImage: "hourglass")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
            }

            if let recordResult {
                RemediationHistoryRecordResultView(result: recordResult)
            }

            if let result {
                RemediationHistoryResultView(result: result)
            } else {
                Label(UIStrings.remediationHistoryNoResult, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            Label(UIStrings.remediationHistoryNoWriteBoundary, systemImage: "nosign")
                .font(.callout)
                .foregroundStyle(.secondary)
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

struct RemediationHistoryRecordResultView: View {
    let result: RemediationHistoryRecordResult

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.remediationHistoryRecordResult, systemImage: result.recorded ? "checkmark.seal" : "info.circle")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                Spacer()
                Text(result.recorded ? UIStrings.remediationHistoryStatusRecorded : UIStrings.routingAccuracyUnavailableShort)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
            }

            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            if !result.message.isEmpty {
                Text(result.message)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            if let record = result.record {
                RemediationHistoryRecordCard(record: record)
            } else if !result.records.isEmpty {
                RemediationHistoryRecordList(records: result.records, title: UIStrings.remediationHistoryRecords)
            }

            CrossAgentReadinessEvidenceList(evidence: result.evidenceReferences)
            StaleDriftSafetyList(safety: result.safetyFlags)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
    }
}

struct RemediationHistoryResultView: View {
    let result: RemediationHistoryResult

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            DetailMetricGrid {
                SummaryChip(title: UIStrings.remediationHistoryRecords, value: "\(recordCount)", systemImage: "archivebox")
                SummaryChip(title: UIStrings.remediationHistoryRecorded, value: "\(recordedCount)", systemImage: "checkmark.seal")
                SummaryChip(title: UIStrings.remediationHistoryRecurrence, value: "\(recurrenceCount)", systemImage: "arrow.triangle.2.circlepath")
                SummaryChip(title: UIStrings.remediationHistoryReopened, value: "\(reopenedCount)", systemImage: "arrow.uturn.backward.circle")
                SummaryChip(title: UIStrings.remediationHistoryReadinessImprovement, value: "\(readinessImprovementCount)", systemImage: "chart.line.uptrend.xyaxis")
                SummaryChip(title: UIStrings.remediationHistoryDecisions, value: "\(decisionCount)", systemImage: "checklist.checked")
                SummaryChip(title: UIStrings.remediationHistoryStatuses, value: "\(statusCount)", systemImage: "tag")
                SummaryChip(title: UIStrings.knowledgeBlockerNotes, value: "\(blockerCount)", systemImage: "lock.trianglebadge.exclamationmark")
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.routingAccuracyGeneratedBy, value: result.generatedBy)
                MetadataRow(label: UIStrings.routingAccuracyCatalog, value: result.catalogAvailable ? UIStrings.routingAccuracyAvailable : UIStrings.routingAccuracyUnavailableShort)
                MetadataRow(label: UIStrings.agent, value: agentFilterLabel)
                if !result.filters.ruleIDs.isEmpty {
                    MetadataRow(label: UIStrings.remediationBatchReviewRuleIDs, value: result.filters.ruleIDs.joined(separator: ", "))
                }
                if !result.filters.riskLevels.isEmpty {
                    MetadataRow(label: UIStrings.remediationBatchReviewRiskLevels, value: result.filters.riskLevels.joined(separator: ", "))
                }
                if !result.filters.decisions.isEmpty {
                    MetadataRow(label: UIStrings.remediationHistoryDecisions, value: result.filters.decisions.joined(separator: ", "))
                }
                if !result.filters.statuses.isEmpty {
                    MetadataRow(label: UIStrings.remediationHistoryStatuses, value: result.filters.statuses.joined(separator: ", "))
                }
                if let projectRoot = result.filters.projectRoot, !projectRoot.isEmpty {
                    PrivacyPathRow(label: UIStrings.project, path: projectRoot)
                }
                if let workspace = result.filters.workspace, !workspace.isEmpty {
                    MetadataRow(label: UIStrings.workspaceReadinessTitle, value: workspace)
                }
                if let taskText = result.filters.taskText, !taskText.isEmpty {
                    MetadataRow(label: UIStrings.taskBenchmarkTaskPlaceholder, value: taskText)
                }
                if let limit = result.filters.limit {
                    MetadataRow(label: UIStrings.text("filter.limit", "Limit"), value: "\(limit)")
                }
                if let promptRequest = result.promptRequest {
                    MetadataRow(label: UIStrings.routingAccuracyPromptRequest, value: promptRequestLabel(promptRequest))
                }
            }

            if !result.summary.summaryText.isEmpty {
                Text(result.summary.summaryText)
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            RoutingInlineList(title: UIStrings.remediationHistoryDecisions, empty: UIStrings.routingAccuracyNoGaps, values: result.decisions, systemImage: "checklist.checked")
            RoutingInlineList(title: UIStrings.remediationHistoryStatuses, empty: UIStrings.routingAccuracyNoGaps, values: result.statuses, systemImage: "tag")
            RemediationHistoryRecordList(records: result.records, title: UIStrings.remediationHistoryRecords)
            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: result.gapNotes, systemImage: "puzzlepiece.extension")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: result.blockerNotes, systemImage: "exclamationmark.octagon")
            CrossAgentReadinessEvidenceList(evidence: result.evidenceReferences)
            StaleDriftSafetyList(safety: result.safetyFlags)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
    }

    private var recordCount: Int {
        result.summary.totalCount > 0 ? result.summary.totalCount : result.records.count
    }

    private var recordedCount: Int {
        result.summary.recordedCount > 0 ? result.summary.recordedCount : result.records.filter { $0.status.localizedCaseInsensitiveContains("record") }.count
    }

    private var recurrenceCount: Int {
        result.summary.recurrenceCount > 0 ? result.summary.recurrenceCount : result.records.reduce(0) { $0 + $1.recurrenceCount }
    }

    private var reopenedCount: Int {
        result.summary.reopenedCount > 0 ? result.summary.reopenedCount : result.records.reduce(0) { $0 + $1.reopenedCount }
    }

    private var readinessImprovementCount: Int {
        result.summary.readinessImprovementCount > 0 ? result.summary.readinessImprovementCount : result.records.filter { record in
            !(record.readinessImprovement ?? "").isEmpty
        }.count
    }

    private var decisionCount: Int {
        result.summary.decisionCount > 0 ? result.summary.decisionCount : Set(result.records.map(\.decision)).count
    }

    private var statusCount: Int {
        result.summary.statusCount > 0 ? result.summary.statusCount : Set(result.records.map(\.status)).count
    }

    private var blockerCount: Int {
        result.summary.blockerCount > 0 ? result.summary.blockerCount : result.blockerNotes.count + result.records.reduce(0) { $0 + $1.blockerNotes.count }
    }

    private var agentFilterLabel: String {
        if !result.filters.agents.isEmpty {
            return result.filters.agents.map(DisplayText.agent).joined(separator: ", ")
        }
        return result.filters.agent.map(DisplayText.agent) ?? UIStrings.text("health.allAgents", "All Agents")
    }

    private func promptRequestLabel(_ promptRequest: RoutingAccuracyPromptRequest) -> String {
        let state = promptRequest.enabled ? UIStrings.llmEnabled : UIStrings.llmDisabled
        let copy = promptRequest.copyOnly ? UIStrings.llmPromptCopyOnly : UIStrings.llmSkillAnalysisEnabledUnsafe
        return "\(promptRequest.requestKind) · \(state) · \(copy)"
    }
}

struct RemediationHistoryRecordList: View {
    let records: [RemediationHistoryRecord]
    let title: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if records.isEmpty {
                Text(UIStrings.remediationHistoryNoRecords)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 320), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(records.prefix(10)) { record in
                        RemediationHistoryRecordCard(record: record)
                    }
                }
            }
        }
    }
}

struct RemediationHistoryRecordCard: View {
    let record: RemediationHistoryRecord

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Label(record.title, systemImage: historyIcon(for: record.category))
                    .font(.callout.bold())
                    .lineLimit(1)
                Spacer()
                Text(record.status)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
            }

            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                MetadataRow(label: UIStrings.remediationPlanCategory, value: record.category)
                MetadataRow(label: UIStrings.remediationHistoryDecision, value: record.decision)
                MetadataRow(label: UIStrings.state, value: record.status)
                if let reviewArea = record.reviewArea, !reviewArea.isEmpty {
                    MetadataRow(label: UIStrings.remediationBatchReviewReviewArea, value: reviewArea)
                }
                if let sourceMethod = record.sourceMethod, !sourceMethod.isEmpty {
                    MetadataRow(label: UIStrings.remediationHistorySourceMethod, value: sourceMethod)
                }
                if let agent = record.agent, !agent.isEmpty {
                    MetadataRow(label: UIStrings.agent, value: DisplayText.agent(agent))
                }
                if let workspace = record.workspace, !workspace.isEmpty {
                    MetadataRow(label: UIStrings.workspaceReadinessTitle, value: workspace)
                }
                if let ruleID = record.ruleID, !ruleID.isEmpty {
                    MetadataRow(label: UIStrings.remediationBatchReviewRuleIDs, value: ruleID)
                }
                if let riskLevel = record.riskLevel, !riskLevel.isEmpty {
                    MetadataRow(label: UIStrings.remediationBatchReviewRiskLevels, value: riskLevel)
                }
                if let recordedAt = record.recordedAt, !recordedAt.isEmpty {
                    MetadataRow(label: UIStrings.remediationHistoryRecordedAt, value: recordedAt)
                }
                if let updatedAt = record.updatedAt, !updatedAt.isEmpty {
                    MetadataRow(label: UIStrings.remediationHistoryUpdatedAt, value: updatedAt)
                }
                if record.recurrenceCount > 0 {
                    MetadataRow(label: UIStrings.remediationHistoryRecurrence, value: "\(record.recurrenceCount)")
                }
                if record.reopenedCount > 0 {
                    MetadataRow(label: UIStrings.remediationHistoryReopened, value: "\(record.reopenedCount)")
                }
                if let readinessImprovement = record.readinessImprovement, !readinessImprovement.isEmpty {
                    MetadataRow(label: UIStrings.remediationHistoryReadinessImprovement, value: readinessImprovement)
                }
            }

            if let skill = record.skill {
                CapabilitySkillList(skills: [skill])
            }

            if let taskText = record.taskText, !taskText.isEmpty {
                Label(taskText, systemImage: "text.badge.checkmark")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            if !record.rationale.isEmpty {
                Text(record.rationale)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            if !record.note.isEmpty {
                Label(record.note, systemImage: "note.text")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: record.gapNotes, systemImage: "puzzlepiece.extension")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: record.blockerNotes, systemImage: "exclamationmark.octagon")
            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: record.evidenceRefs, systemImage: "checklist")
            RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: record.safetyFlags, systemImage: "checkmark.shield")
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
    }

    private func historyIcon(for category: String) -> String {
        let normalized = category.lowercased()
        if normalized.contains("reopen") { return "arrow.uturn.backward.circle" }
        if normalized.contains("readiness") { return "chart.line.uptrend.xyaxis" }
        if normalized.contains("risk") { return "exclamationmark.triangle" }
        if normalized.contains("rule") { return "ruler" }
        if normalized.contains("agent") { return "person.2" }
        if normalized.contains("workspace") { return "folder" }
        if normalized.contains("task") { return "text.badge.checkmark" }
        return "archivebox"
    }
}
