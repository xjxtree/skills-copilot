import AppKit
import SwiftUI

struct KnowledgeSearchPanel: View {
    @Binding var query: String
    let result: KnowledgeSearchResult?
    let isSearching: Bool
    let onSearch: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.knowledgeTitle, systemImage: "books.vertical")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.knowledgeBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            HStack(spacing: 8) {
                TextField(UIStrings.knowledgeQueryPlaceholder, text: $query)
                    .textFieldStyle(.roundedBorder)
                    .onSubmit(onSearch)
                Button {
                    onSearch()
                } label: {
                    Label(UIStrings.knowledgeSearchAction, systemImage: "magnifyingglass")
                }
                .disabled(isSearching)
            }

            if isSearching {
                Label(UIStrings.llmPreparing, systemImage: "hourglass")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            if let result {
                KnowledgeSearchResultView(result: result)
            } else {
                Label(UIStrings.knowledgeNoResult, systemImage: "info.circle")
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

struct KnowledgeSearchResultView: View {
    let result: KnowledgeSearchResult

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            DetailMetricGrid {
                SummaryChip(title: UIStrings.knowledgeMatches, value: "\(resultCount)", systemImage: "magnifyingglass")
                SummaryChip(title: UIStrings.agent, value: "\(agentCount)", systemImage: "person.3")
                SummaryChip(title: UIStrings.knowledgeFacets, value: "\(result.facetRows.count)", systemImage: "tag")
                SummaryChip(title: UIStrings.knowledgeGapNotes, value: "\(gapCount)", systemImage: "puzzlepiece.extension")
                SummaryChip(title: UIStrings.knowledgeBlockerNotes, value: "\(blockerCount)", systemImage: "exclamationmark.octagon")
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.routingAccuracyGeneratedBy, value: result.generatedBy)
                MetadataRow(label: UIStrings.routingAccuracyCatalog, value: result.catalogAvailable ? UIStrings.routingAccuracyAvailable : UIStrings.routingAccuracyUnavailableShort)
                MetadataRow(label: UIStrings.knowledgeQuery, value: result.filters.query.isEmpty ? UIStrings.unknown : result.filters.query)
                MetadataRow(label: UIStrings.agent, value: result.filters.agents.isEmpty ? (result.filters.agent.map(DisplayText.agent) ?? UIStrings.text("health.allAgents", "All Agents")) : result.filters.agents.map(DisplayText.agent).joined(separator: ", "))
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

            KnowledgeRowsList(rows: result.knowledgeRows)
            KnowledgeFacetList(facets: result.facetRows)
            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: result.gapNotes, systemImage: "puzzlepiece.extension")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: result.blockerNotes, systemImage: "exclamationmark.octagon")
            CrossAgentReadinessEvidenceList(evidence: result.evidenceReferences)
            StaleDriftSafetyList(safety: result.safetyFlags)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
    }

    private var resultCount: Int {
        result.summary.resultCount > 0 ? result.summary.resultCount : result.knowledgeRows.count
    }

    private var agentCount: Int {
        result.summary.agentCount > 0 ? result.summary.agentCount : Set(result.knowledgeRows.compactMap(\.agent)).count
    }

    private var gapCount: Int {
        result.summary.gapCount > 0 ? result.summary.gapCount : result.gapNotes.count
    }

    private var blockerCount: Int {
        result.summary.blockerCount > 0 ? result.summary.blockerCount : result.blockerNotes.count
    }

    private func promptRequestLabel(_ promptRequest: RoutingAccuracyPromptRequest) -> String {
        let state = promptRequest.enabled ? UIStrings.llmEnabled : UIStrings.llmDisabled
        let copy = promptRequest.copyOnly ? UIStrings.llmPromptCopyOnly : UIStrings.llmSkillAnalysisEnabledUnsafe
        return "\(promptRequest.requestKind) · \(state) · \(copy)"
    }
}

struct KnowledgeRowsList: View {
    let rows: [KnowledgeSearchRow]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.knowledgeRows)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if rows.isEmpty {
                Text(UIStrings.knowledgeNoRows)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 300), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(rows.prefix(10)) { row in
                        VStack(alignment: .leading, spacing: 8) {
                            HStack(alignment: .firstTextBaseline) {
                                Label(row.skillName, systemImage: "doc.text.magnifyingglass")
                                    .font(.callout.bold())
                                    .lineLimit(1)
                                Spacer()
                                Text(row.displayRank)
                                    .font(.caption2.bold())
                                    .foregroundStyle(.secondary)
                            }

                            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                                MetadataRow(label: UIStrings.agent, value: row.agent.map(DisplayText.agent) ?? UIStrings.unknown)
                                MetadataRow(label: UIStrings.scope, value: row.scope ?? UIStrings.unknown)
                                MetadataRow(label: UIStrings.state, value: row.statusLabel)
                                if let definitionID = row.definitionID, !definitionID.isEmpty {
                                    MetadataRow(label: UIStrings.definition, value: definitionID)
                                }
                            }

                            if !row.purpose.isEmpty {
                                Text(row.purpose)
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                                    .textSelection(.enabled)
                            }

                            KnowledgeTokenFlow(title: UIStrings.knowledgeMatchedFields, values: row.matchedFields)
                            RoutingInlineList(title: UIStrings.routingConfidenceMatchReasons, empty: UIStrings.routingConfidenceNoReasons, values: row.matchReasons, systemImage: "text.bubble")
                            KnowledgeTokenFlow(title: UIStrings.knowledgeKeywords, values: row.keywords)
                            KnowledgeTokenFlow(title: UIStrings.knowledgeTools, values: row.tools)
                            KnowledgeTokenFlow(title: UIStrings.knowledgeRules, values: row.rules)
                            KnowledgeTokenFlow(title: UIStrings.knowledgeCapabilities, values: row.capabilityTags)
                            KnowledgeTokenFlow(title: UIStrings.knowledgeRisks, values: row.riskTags)
                            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: row.evidenceRefs, systemImage: "checklist")
                            RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: row.safetyFlags, systemImage: "checkmark.shield")
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }
        }
    }
}

struct KnowledgeTokenFlow: View {
    let title: String
    let values: [String]

    var body: some View {
        if !values.isEmpty {
            VStack(alignment: .leading, spacing: 5) {
                Text(title)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 70), spacing: 6)], alignment: .leading, spacing: 6) {
                    ForEach(values.prefix(10), id: \.self) { value in
                        Text(value)
                            .font(.caption2)
                            .padding(.horizontal, 7)
                            .padding(.vertical, 3)
                            .background(Color.agentCopilotPanelBackground, in: Capsule())
                    }
                }
            }
        }
    }
}

struct KnowledgeFacetList: View {
    let facets: [KnowledgeFacetRow]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.knowledgeFacets)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if facets.isEmpty {
                Text(UIStrings.knowledgeNoFacets)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 150), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(facets.prefix(12)) { facet in
                        HStack {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(facet.value)
                                    .font(.caption.bold())
                                Text(facet.facet)
                                    .font(.caption2)
                                    .foregroundStyle(.secondary)
                            }
                            Spacer()
                            Text("\(facet.count)")
                                .font(.caption.bold())
                                .foregroundStyle(.secondary)
                        }
                        .padding(8)
                        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
                    }
                }
            }
        }
    }
}

struct LocalSkillMapPanel: View {
    let skill: SkillRecord
    let result: LocalSkillMapResult?
    let isBuilding: Bool
    let onBuild: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.localSkillMapTitle, systemImage: "point.3.connected.trianglepath.dotted")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.localSkillMapBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            HStack(spacing: 8) {
                Button {
                    onBuild()
                } label: {
                    Label(UIStrings.localSkillMapAction, systemImage: "map")
                }
                .disabled(isBuilding)
                .help(UIStrings.localSkillMapBoundary)

                if isBuilding {
                    Label(UIStrings.llmPreparing, systemImage: "hourglass")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
            }

            LocalSkillMapSelectedContext(skill: skill, selectedSkill: result?.selectedSkill)

            if let result {
                LocalSkillMapResultView(result: result)
            } else {
                Label(UIStrings.localSkillMapNoResult, systemImage: "info.circle")
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

struct SkillLifecycleTimelinePanel: View {
    let skill: SkillRecord
    let result: SkillLifecycleTimelineResult?
    let isLoading: Bool
    let onLoad: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.skillLifecycleTimelineTitle, systemImage: "timeline.selection")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.skillLifecycleTimelineBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            HStack(spacing: 8) {
                Button {
                    onLoad()
                } label: {
                    Label(UIStrings.skillLifecycleTimelineAction, systemImage: "clock.arrow.circlepath")
                }
                .disabled(isLoading)
                .help(UIStrings.skillLifecycleTimelineBoundary)

                if isLoading {
                    Label(UIStrings.loading, systemImage: "hourglass")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
            }

            LocalSkillMapSelectedContext(skill: skill, selectedSkill: nil)

            if let result {
                SkillLifecycleTimelineResultView(result: result)
            } else {
                Label(UIStrings.skillLifecycleTimelineNoResult, systemImage: "info.circle")
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

struct SkillLifecycleTimelineResultView: View {
    let result: SkillLifecycleTimelineResult

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            DetailMetricGrid {
                SummaryChip(title: UIStrings.skillLifecycleTimelineEvents, value: "\(eventCount)", systemImage: "timeline.selection")
                SummaryChip(title: UIStrings.skillLifecycleTimelineSkillRows, value: "\(skillCount)", systemImage: "target")
                SummaryChip(title: UIStrings.skillLifecycleTimelineAgentRows, value: "\(agentCount)", systemImage: "person.3")
                SummaryChip(title: UIStrings.skillLifecycleTimelineEventTypes, value: "\(eventTypeCount)", systemImage: "tag")
                SummaryChip(title: UIStrings.skillLifecycleTimelineStages, value: "\(stageCount)", systemImage: "flag")
                SummaryChip(title: UIStrings.knowledgeGapNotes, value: "\(gapCount)", systemImage: "puzzlepiece.extension")
                SummaryChip(title: UIStrings.knowledgeBlockerNotes, value: "\(blockerCount)", systemImage: "exclamationmark.octagon")
                SummaryChip(title: UIStrings.crossAgentReadinessEvidence, value: "\(evidenceCount)", systemImage: "checklist")
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.routingAccuracyGeneratedBy, value: result.generatedBy)
                MetadataRow(label: UIStrings.routingAccuracyCatalog, value: result.catalogAvailable ? UIStrings.routingAccuracyAvailable : UIStrings.routingAccuracyUnavailableShort)
                MetadataRow(label: UIStrings.agent, value: agentFilterLabel)
                if let selectedSkillID = result.filters.selectedSkillID, !selectedSkillID.isEmpty {
                    MetadataRow(label: UIStrings.localSkillMapSelectedContext, value: result.filters.selectedSkillName ?? selectedSkillID)
                }
                if let projectRoot = result.filters.projectRoot, !projectRoot.isEmpty {
                    PrivacyPathRow(label: UIStrings.text("projectContext.root", "Project root"), path: projectRoot)
                }
                if let currentCWD = result.filters.currentCWD, !currentCWD.isEmpty {
                    MetadataRow(label: UIStrings.text("projectContext.currentCWD", "Current CWD"), value: currentCWD)
                }
                if let limit = result.filters.limit {
                    MetadataRow(label: UIStrings.text("filter.limit", "Limit"), value: "\(limit)")
                }
                if let firstEventAt = result.summary.firstEventAt, !firstEventAt.isEmpty {
                    MetadataRow(label: UIStrings.text("skillLifecycleTimeline.firstEvent", "First event"), value: firstEventAt)
                }
                if let latestEventAt = result.summary.latestEventAt, !latestEventAt.isEmpty {
                    MetadataRow(label: UIStrings.text("skillLifecycleTimeline.latestEvent", "Latest event"), value: latestEventAt)
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

            SkillLifecycleTimelineRowList(
                title: UIStrings.skillLifecycleTimelineEvents,
                rows: result.timelineRows,
                systemImage: "timeline.selection"
            )
            SkillLifecycleTimelineRowList(
                title: UIStrings.skillLifecycleTimelineSkillRows,
                rows: result.skillRows,
                systemImage: "target"
            )
            SkillLifecycleTimelineRowList(
                title: UIStrings.skillLifecycleTimelineAgentRows,
                rows: result.agentRows,
                systemImage: "person.3"
            )
            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: result.gapNotes, systemImage: "puzzlepiece.extension")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: result.blockerNotes, systemImage: "exclamationmark.octagon")
            ProviderObservabilityEvidenceList(evidence: result.evidenceReferences)
            ProviderObservabilitySafetyList(safety: result.safetyFlags)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
    }

    private var eventCount: Int {
        result.summary.eventCount > 0 ? result.summary.eventCount : result.timelineRows.count
    }

    private var skillCount: Int {
        result.summary.skillCount > 0 ? result.summary.skillCount : result.skillRows.count
    }

    private var agentCount: Int {
        result.summary.agentCount > 0 ? result.summary.agentCount : result.agentRows.count
    }

    private var eventTypeCount: Int {
        if result.summary.eventTypeCount > 0 {
            return result.summary.eventTypeCount
        }
        return Set(result.timelineRows.map(\.eventType)).count
    }

    private var stageCount: Int {
        if result.summary.stageCount > 0 {
            return result.summary.stageCount
        }
        return Set(result.timelineRows.map(\.lifecycleStage)).count
    }

    private var gapCount: Int {
        result.summary.gapCount > 0 ? result.summary.gapCount : result.gapNotes.count
    }

    private var blockerCount: Int {
        result.summary.blockerCount > 0 ? result.summary.blockerCount : result.blockerNotes.count
    }

    private var evidenceCount: Int {
        result.summary.evidenceCount > 0 ? result.summary.evidenceCount : result.evidenceReferences.count
    }

    private var agentFilterLabel: String {
        if !result.filters.agents.isEmpty {
            return result.filters.agents.map(DisplayText.agent).joined(separator: ", ")
        }
        return result.filters.agent.map(DisplayText.agent) ?? UIStrings.text("health.allAgents", "All Agents")
    }

    private func promptRequestLabel(_ promptRequest: SkillLifecycleTimelinePromptRequest) -> String {
        let state = promptRequest.enabled ? UIStrings.llmEnabled : UIStrings.llmDisabled
        let copy = promptRequest.copyOnly ? UIStrings.llmPromptCopyOnly : UIStrings.llmSkillAnalysisEnabledUnsafe
        let redaction = promptRequest.redacted ? UIStrings.aiProviderAuditRedaction : UIStrings.llmSkillAnalysisEnabledUnsafe
        return "\(promptRequest.requestKind) · \(state) · \(copy) · \(redaction)"
    }
}

struct SkillLifecycleTimelineRowList: View {
    let title: String
    let rows: [SkillLifecycleTimelineRow]
    let systemImage: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if rows.isEmpty {
                Text(UIStrings.skillLifecycleTimelineNoRows)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 300), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(rows.prefix(10)) { row in
                        SkillLifecycleTimelineRowItem(row: row, systemImage: systemImage)
                    }
                }
            }
        }
    }
}

struct SkillLifecycleTimelineRowItem: View {
    let row: SkillLifecycleTimelineRow
    let systemImage: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Label(row.title, systemImage: iconName)
                    .font(.callout.bold())
                    .lineLimit(1)
                Spacer()
                if let status = row.displayStatus, !status.isEmpty {
                    Text(status)
                        .font(.caption2.bold())
                        .foregroundStyle(.secondary)
                }
            }

            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                if let occurredAt = row.occurredAt, !occurredAt.isEmpty {
                    MetadataRow(label: UIStrings.skillLifecycleTimelineOccurredAt, value: occurredAt)
                }
                MetadataRow(label: UIStrings.skillLifecycleTimelineEventType, value: row.eventType)
                MetadataRow(label: UIStrings.skillLifecycleTimelineLifecycleStage, value: row.lifecycleStage)
                if let agent = row.agent, !agent.isEmpty {
                    MetadataRow(label: UIStrings.agent, value: DisplayText.agent(agent))
                }
                if let skillName = row.skillName, !skillName.isEmpty {
                    MetadataRow(label: UIStrings.text("metadata.name", "Name"), value: skillName)
                }
                if let definitionID = row.definitionID, !definitionID.isEmpty {
                    MetadataRow(label: UIStrings.definition, value: definitionID)
                }
                if let instanceID = row.instanceID, !instanceID.isEmpty {
                    MetadataRow(label: UIStrings.text("metadata.instance", "Instance"), value: instanceID)
                }
                if let source = row.source, !source.isEmpty {
                    MetadataRow(label: UIStrings.remediationHistorySourceMethod, value: source)
                }
                if let count = row.count {
                    MetadataRow(label: UIStrings.providerObservabilityCalls, value: "\(count)")
                }
            }

            if !row.summary.isEmpty {
                PrivacyEvidenceText(value: row.summary, font: .caption, lineLimit: nil)
            }

            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: row.evidenceRefs, systemImage: "checklist")
            RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: row.safetyFlags, systemImage: "checkmark.shield")
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
    }

    private var iconName: String {
        let normalized = "\(row.eventType) \(row.lifecycleStage)".lowercased()
        if normalized.contains("block") || normalized.contains("risk") || normalized.contains("finding") {
            return "exclamationmark.triangle"
        }
        if normalized.contains("route") || normalized.contains("task") || normalized.contains("session") {
            return "point.topleft.down.curvedto.point.bottomright.up"
        }
        if normalized.contains("remediation") || normalized.contains("fix") || normalized.contains("cleanup") {
            return "wand.and.sparkles"
        }
        if normalized.contains("provider") || normalized.contains("prompt") {
            return "waveform.path.ecg.rectangle"
        }
        if normalized.contains("agent") {
            return "person.3"
        }
        if normalized.contains("skill") {
            return "target"
        }
        return systemImage
    }
}
