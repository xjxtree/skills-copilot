import AppKit
import SwiftUI

struct GuidedCleanupFlowPanel: View {
    let result: GuidedCleanupFlowResult?
    let recordResult: GuidedCleanupRecordStepResult?
    let isPlanning: Bool
    let isRecording: Bool
    let onLoad: () -> Void
    let onRecord: (GuidedCleanupFlowStep) -> Void
    let onOpenSafeLink: (GuidedCleanupSafeActionDeepLink, GuidedCleanupFlowStep?) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.guidedCleanupFlowTitle, systemImage: "sparkles.square.filled.on.square")
                    .font(.headline)
                Spacer()
                Label(UIStrings.guidedCleanupFlowAppLocalOnly, systemImage: "archivebox")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.guidedCleanupFlowBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            HStack(spacing: 8) {
                Button {
                    onLoad()
                } label: {
                    Label(UIStrings.guidedCleanupFlowAction, systemImage: "list.clipboard")
                }
                .disabled(isPlanning || isRecording)

                if let step = result?.recommendedStep, result?.isUnavailable != true {
                    Button {
                        onRecord(step)
                    } label: {
                        Label(UIStrings.guidedCleanupFlowRecordAction, systemImage: "archivebox")
                    }
                    .disabled(isPlanning || isRecording)
                    .help(UIStrings.guidedCleanupFlowRecordGuidance)
                }

                if isPlanning || isRecording {
                    Label(UIStrings.llmPreparing, systemImage: "hourglass")
                        .font(.callout)
                        .foregroundStyle(.secondary)
                }
            }

            if let recordResult {
                GuidedCleanupRecordResultView(result: recordResult)
            }

            if let result {
                GuidedCleanupFlowResultView(result: result, onOpenSafeLink: onOpenSafeLink)
            } else {
                Label(UIStrings.guidedCleanupFlowNoResult, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            Label(UIStrings.guidedCleanupFlowNoWriteBoundary, systemImage: "nosign")
                .font(.callout)
                .foregroundStyle(.secondary)
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

struct GuidedCleanupFlowResultView: View {
    let result: GuidedCleanupFlowResult
    let onOpenSafeLink: (GuidedCleanupSafeActionDeepLink, GuidedCleanupFlowStep?) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            DetailMetricGrid {
                SummaryChip(title: UIStrings.guidedCleanupFlowSteps, value: "\(stepCount)", systemImage: "list.clipboard")
                SummaryChip(title: UIStrings.guidedCleanupFlowIssueGroups, value: "\(issueGroupCount)", systemImage: "exclamationmark.triangle")
                SummaryChip(title: UIStrings.guidedCleanupFlowSafeActions, value: "\(safeActionCount)", systemImage: "arrow.right.circle")
                SummaryChip(title: UIStrings.guidedCleanupFlowRecordedSteps, value: "\(recordedStepCount)", systemImage: "archivebox")
                SummaryChip(title: UIStrings.guidedCleanupFlowRecommended, value: "\(recommendedStepCount)", systemImage: "star")
                SummaryChip(title: UIStrings.knowledgeGapNotes, value: "\(gapCount)", systemImage: "puzzlepiece.extension")
                SummaryChip(title: UIStrings.knowledgeBlockerNotes, value: "\(blockerCount)", systemImage: "lock.trianglebadge.exclamationmark")
            }

            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                MetadataRow(label: UIStrings.routingAccuracyGeneratedBy, value: result.generatedBy)
                MetadataRow(label: UIStrings.routingAccuracyCatalog, value: result.catalogAvailable ? UIStrings.routingAccuracyAvailable : UIStrings.routingAccuracyUnavailableShort)
                MetadataRow(label: UIStrings.agent, value: agentFilterLabel)
                if let selectedSkillID = result.filters.selectedSkillID, !selectedSkillID.isEmpty {
                    MetadataRow(label: UIStrings.localSkillMapSelectedContext, value: result.filters.selectedSkillName ?? selectedSkillID)
                }
                if let taskText = result.filters.taskText, !taskText.isEmpty {
                    MetadataRow(label: UIStrings.taskBenchmarkTaskPlaceholder, value: taskText)
                }
                if let limit = result.filters.limit {
                    MetadataRow(label: UIStrings.text("filter.limit", "Limit"), value: "\(limit)")
                }
                MetadataRow(label: UIStrings.guidedCleanupFlowIssueGroups, value: result.filters.includeIssueGroups ? UIStrings.stateEnabled : UIStrings.stateDisabled)
                MetadataRow(label: UIStrings.guidedCleanupFlowSafeActions, value: result.filters.includeSafeNextActions ? UIStrings.stateEnabled : UIStrings.stateDisabled)
                MetadataRow(label: UIStrings.guidedCleanupFlowRecordedSteps, value: result.filters.includeRecordedSteps ? UIStrings.stateEnabled : UIStrings.stateDisabled)
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

            GuidedCleanupStepList(steps: result.flowSteps, onOpenSafeLink: onOpenSafeLink)
            GuidedCleanupIssueGroupList(groups: result.issueGroups)
            GuidedCleanupSafeActionList(actions: result.safeNextActions, onOpenSafeLink: onOpenSafeLink)
            GuidedCleanupRecordedStepList(records: result.recordedSteps)
            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: result.gapNotes, systemImage: "puzzlepiece.extension")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: result.blockerNotes, systemImage: "exclamationmark.octagon")
            ProviderObservabilityEvidenceList(evidence: result.evidenceReferences)
            ProviderObservabilitySafetyList(safety: result.safetyFlags)
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 6))
    }

    private var stepCount: Int {
        result.summary.stepCount > 0 ? result.summary.stepCount : result.flowSteps.count
    }

    private var issueGroupCount: Int {
        result.summary.issueGroupCount > 0 ? result.summary.issueGroupCount : result.issueGroups.count
    }

    private var safeActionCount: Int {
        result.summary.safeActionCount > 0 ? result.summary.safeActionCount : result.safeNextActions.count
    }

    private var recordedStepCount: Int {
        result.summary.recordedStepCount > 0 ? result.summary.recordedStepCount : result.recordedSteps.count
    }

    private var recommendedStepCount: Int {
        result.summary.recommendedStepCount > 0 ? result.summary.recommendedStepCount : result.flowSteps.filter(\.recommended).count
    }

    private var gapCount: Int {
        result.summary.gapCount > 0 ? result.summary.gapCount : result.gapNotes.count
    }

    private var blockerCount: Int {
        result.summary.blockerCount > 0 ? result.summary.blockerCount : result.blockerNotes.count
    }

    private var agentFilterLabel: String {
        if !result.filters.agents.isEmpty {
            return result.filters.agents.map(DisplayText.agent).joined(separator: ", ")
        }
        return result.filters.agent.map(DisplayText.agent) ?? UIStrings.text("health.allAgents", "All Agents")
    }

    private func promptRequestLabel(_ promptRequest: GuidedCleanupFlowPromptRequest) -> String {
        let state = promptRequest.enabled ? UIStrings.llmEnabled : UIStrings.llmDisabled
        let copy = promptRequest.copyOnly ? UIStrings.llmPromptCopyOnly : UIStrings.llmSkillAnalysisEnabledUnsafe
        let redaction = promptRequest.redacted ? UIStrings.aiProviderAuditRedaction : UIStrings.llmSkillAnalysisEnabledUnsafe
        return "\(promptRequest.requestKind) · \(state) · \(copy) · \(redaction)"
    }
}

struct GuidedCleanupRecordResultView: View {
    let result: GuidedCleanupRecordStepResult

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.guidedCleanupFlowRecordResult, systemImage: result.recorded ? "checkmark.seal" : "info.circle")
                    .font(.callout.bold())
                Spacer()
                Text(result.recorded ? UIStrings.remediationHistoryStatusRecorded : UIStrings.routingAccuracyUnavailableShort)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
            }

            if !result.message.isEmpty {
                Text(result.message)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            Grid(alignment: .leading, horizontalSpacing: 12, verticalSpacing: 5) {
                MetadataRow(label: UIStrings.routingAccuracyGeneratedBy, value: result.generatedBy)
                MetadataRow(label: UIStrings.guidedCleanupFlowAppLocalOnly, value: result.appLocalOnly ? UIStrings.stateEnabled : UIStrings.llmSkillAnalysisEnabledUnsafe)
                MetadataRow(label: UIStrings.guidedCleanupFlowMetadataRedacted, value: result.metadataRedacted ? UIStrings.stateEnabled : UIStrings.llmSkillAnalysisEnabledUnsafe)
            }

            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            if let record = result.record {
                GuidedCleanupRecordedStepCard(record: record)
            }
            GuidedCleanupRecordedStepList(records: result.records, title: UIStrings.guidedCleanupFlowRecordedSteps)
            ProviderObservabilityEvidenceList(evidence: result.evidenceReferences)
            ProviderObservabilitySafetyList(safety: result.safetyFlags)
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 6))
    }
}

struct GuidedCleanupStepList: View {
    let steps: [GuidedCleanupFlowStep]
    let onOpenSafeLink: (GuidedCleanupSafeActionDeepLink, GuidedCleanupFlowStep?) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.guidedCleanupFlowSteps)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if steps.isEmpty {
                Text(UIStrings.guidedCleanupFlowNoSteps)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 320), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(steps.prefix(10)) { step in
                        GuidedCleanupStepCard(step: step, onOpenSafeLink: onOpenSafeLink)
                    }
                }
            }
        }
    }
}

struct GuidedCleanupStepCard: View {
    let step: GuidedCleanupFlowStep
    let onOpenSafeLink: (GuidedCleanupSafeActionDeepLink, GuidedCleanupFlowStep?) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Label(step.title, systemImage: guidedCleanupIcon(for: step.kind))
                    .font(.callout.bold())
                    .lineLimit(1)
                Spacer()
                if step.recommended {
                    Text(UIStrings.guidedCleanupFlowRecommended)
                        .font(.caption2.bold())
                        .foregroundStyle(.secondary)
                }
            }

            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                MetadataRow(label: UIStrings.remediationPlanCategory, value: step.kind)
                MetadataRow(label: UIStrings.state, value: step.status)
                MetadataRow(label: UIStrings.cleanupFilterPriority, value: step.priority)
                MetadataRow(label: UIStrings.guidedCleanupFlowAppLocalOnly, value: step.appLocalRecordOnly ? UIStrings.stateEnabled : UIStrings.llmSkillAnalysisEnabledUnsafe)
                if let order = step.order {
                    MetadataRow(label: UIStrings.guidedCleanupFlowOrder, value: "\(order)")
                }
                if let reviewArea = step.reviewArea, !reviewArea.isEmpty {
                    MetadataRow(label: UIStrings.remediationBatchReviewReviewArea, value: reviewArea)
                }
                if let agent = step.agent, !agent.isEmpty {
                    MetadataRow(label: UIStrings.agent, value: DisplayText.agent(agent))
                }
            }

            if let skill = step.skill {
                CapabilitySkillList(skills: [skill])
            }

            if !step.rationale.isEmpty {
                Text(step.rationale)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }
            if !step.detail.isEmpty {
                PrivacyEvidenceText(value: step.detail, font: .caption, lineLimit: nil)
            }

            Label(step.actionLabel, systemImage: "arrow.right.circle")
                .font(.caption)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)

            GuidedCleanupSafeLinkButton(link: step.safeActionDeepLink) {
                onOpenSafeLink(step.safeActionDeepLink, step)
            }

            RoutingInlineList(title: UIStrings.knowledgeGapNotes, empty: UIStrings.routingAccuracyNoGaps, values: step.gapNotes, systemImage: "puzzlepiece.extension")
            RoutingInlineList(title: UIStrings.knowledgeBlockerNotes, empty: UIStrings.routingAccuracyNoBlockers, values: step.blockerNotes, systemImage: "exclamationmark.octagon")
            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: step.evidenceRefs, systemImage: "checklist")
            RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: step.safetyFlags, systemImage: "checkmark.shield")
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
    }
}

struct GuidedCleanupIssueGroupList: View {
    let groups: [GuidedCleanupIssueGroup]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.guidedCleanupFlowIssueGroups)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if groups.isEmpty {
                Text(UIStrings.guidedCleanupFlowNoIssueGroups)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 300), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(groups.prefix(8)) { group in
                        VStack(alignment: .leading, spacing: 8) {
                            HStack(alignment: .firstTextBaseline) {
                                Label(group.title, systemImage: guidedCleanupIcon(for: group.category))
                                    .font(.callout.bold())
                                    .lineLimit(1)
                                Spacer()
                                Text(group.severity)
                                    .font(.caption2.bold())
                                    .foregroundStyle(.secondary)
                            }
                            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                                MetadataRow(label: UIStrings.remediationPlanCategory, value: group.category)
                                MetadataRow(label: UIStrings.state, value: group.status)
                                MetadataRow(label: UIStrings.providerObservabilityCalls, value: "\(group.count)")
                            }
                            if !group.summary.isEmpty {
                                PrivacyEvidenceText(value: group.summary, font: .caption, lineLimit: nil)
                            }
                            RoutingInlineList(title: UIStrings.guidedCleanupFlowIssueGroups, empty: UIStrings.guidedCleanupFlowNoIssueGroups, values: group.issueRefs, systemImage: "exclamationmark.triangle")
                            RoutingInlineList(title: UIStrings.guidedCleanupFlowSafeActions, empty: UIStrings.guidedCleanupFlowNoSafeActions, values: group.safeNextActionIDs, systemImage: "arrow.right.circle")
                            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: group.evidenceRefs, systemImage: "checklist")
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }
        }
    }
}

struct GuidedCleanupSafeActionList: View {
    let actions: [GuidedCleanupSafeAction]
    let onOpenSafeLink: (GuidedCleanupSafeActionDeepLink, GuidedCleanupFlowStep?) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(UIStrings.guidedCleanupFlowSafeActions)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if actions.isEmpty {
                Text(UIStrings.guidedCleanupFlowNoSafeActions)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 300), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(actions.prefix(8)) { action in
                        VStack(alignment: .leading, spacing: 8) {
                            HStack(alignment: .firstTextBaseline) {
                                Label(action.title, systemImage: guidedCleanupIcon(for: action.kind))
                                    .font(.callout.bold())
                                    .lineLimit(1)
                                Spacer()
                                Text(action.kind)
                                    .font(.caption2.bold())
                                    .foregroundStyle(.secondary)
                            }
                            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                                if let reviewArea = action.reviewArea, !reviewArea.isEmpty {
                                    MetadataRow(label: UIStrings.remediationBatchReviewReviewArea, value: reviewArea)
                                }
                                if let entryMethod = action.entryMethod, !entryMethod.isEmpty {
                                    MetadataRow(label: UIStrings.guidedCleanupSafeActionEntryMethod, value: entryMethod)
                                }
                                MetadataRow(label: UIStrings.guidedCleanupSafeActionPreviewRequired, value: action.requiresPreview ? UIStrings.stateEnabled : UIStrings.stateDisabled)
                                MetadataRow(label: UIStrings.guidedCleanupSafeActionConfirmationRequired, value: action.requiresConfirmation ? UIStrings.stateEnabled : UIStrings.stateDisabled)
                                MetadataRow(label: UIStrings.llmPromptCopyOnly, value: action.copyOnly ? UIStrings.stateEnabled : UIStrings.stateDisabled)
                                MetadataRow(label: UIStrings.guidedCleanupFlowExistingSafeEntry, value: action.requiresExistingSafeEntry ? UIStrings.stateEnabled : UIStrings.stateDisabled)
                                MetadataRow(label: UIStrings.guidedCleanupFlowAppLocalOnly, value: action.appLocalOnly ? UIStrings.stateEnabled : UIStrings.llmSkillAnalysisEnabledUnsafe)
                                MetadataRow(label: UIStrings.guidedCleanupFlowCanApplyFix, value: action.canApplyFix ? UIStrings.llmSkillAnalysisEnabledUnsafe : UIStrings.stateDisabled)
                            }
                            if !action.description.isEmpty {
                                PrivacyEvidenceText(value: action.description, font: .caption, lineLimit: nil)
                            }
                            GuidedCleanupSafeLinkButton(link: action.deepLink) {
                                onOpenSafeLink(action.deepLink, nil)
                            }
                            RoutingInlineList(title: UIStrings.guidedCleanupFlowSteps, empty: UIStrings.guidedCleanupFlowNoSteps, values: action.relatedStepIDs, systemImage: "list.clipboard")
                            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: action.evidenceRefs, systemImage: "checklist")
                            RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: action.safetyFlags, systemImage: "checkmark.shield")
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }
        }
    }
}

struct GuidedCleanupSafeLinkButton: View {
    let link: GuidedCleanupSafeActionDeepLink
    let onOpen: () -> Void
    @State private var isConfirmingOpen = false

    private var needsConfirmation: Bool {
        link.requiresConfirmation
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 8) {
                Button {
                    if needsConfirmation && !isConfirmingOpen {
                        isConfirmingOpen = true
                    } else {
                        isConfirmingOpen = false
                        onOpen()
                    }
                } label: {
                    Label(
                        isConfirmingOpen ? UIStrings.guidedCleanupSafeLinkConfirmOpen : (link.label.isEmpty ? UIStrings.guidedCleanupSafeLinkOpen : link.label),
                        systemImage: isConfirmingOpen ? "checkmark.shield" : "arrowshape.turn.up.right"
                    )
                }
                .buttonStyle(.bordered)
                .disabled(link.canApply)
                .help(link.canApply ? UIStrings.guidedCleanupSafeLinkApplyBlocked : UIStrings.guidedCleanupSafeLinkHelp)

                if isConfirmingOpen {
                    Button(UIStrings.guidedCleanupSafeLinkCancelOpen) {
                        isConfirmingOpen = false
                    }
                    .buttonStyle(.borderless)
                }

                Label(UIStrings.guidedCleanupFlowPreviewOnly, systemImage: "eye")
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
                if link.requiresConfirmation {
                    Label(UIStrings.scriptExecutionConfirmationRequired, systemImage: "checkmark.shield")
                        .font(.caption2.bold())
                        .foregroundStyle(.secondary)
                }
                if link.copyOnly {
                    Label(UIStrings.llmPromptCopyOnly, systemImage: "doc.on.doc")
                        .font(.caption2.bold())
                        .foregroundStyle(.secondary)
                }
            }

            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                MetadataRow(label: UIStrings.guidedCleanupSafeLinkTarget, value: link.target)
                MetadataRow(label: UIStrings.guidedCleanupSafeLinkTrigger, value: link.trigger)
                if let method = link.method, !method.isEmpty {
                    MetadataRow(label: UIStrings.guidedCleanupSafeActionEntryMethod, value: method)
                }
                if let detailSection = link.detailSection, !detailSection.isEmpty {
                    MetadataRow(label: UIStrings.detailSection, value: detailSection)
                }
                MetadataRow(label: UIStrings.guidedCleanupFlowCanApplyFix, value: link.canApply ? UIStrings.llmSkillAnalysisEnabledUnsafe : UIStrings.stateDisabled)
            }
        }
        .padding(8)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 6))
    }
}

struct GuidedCleanupRecordedStepList: View {
    let records: [GuidedCleanupRecordedStep]
    var title: String = UIStrings.guidedCleanupFlowRecordedSteps

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(title)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if records.isEmpty {
                Text(UIStrings.guidedCleanupFlowNoRecordedSteps)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 300), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(records.prefix(8)) { record in
                        GuidedCleanupRecordedStepCard(record: record)
                    }
                }
            }
        }
    }
}

struct GuidedCleanupRecordedStepCard: View {
    let record: GuidedCleanupRecordedStep

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Label(record.title, systemImage: "archivebox")
                    .font(.callout.bold())
                    .lineLimit(1)
                Spacer()
                Text(record.status)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
            }
            Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 4) {
                if let stepID = record.stepID, !stepID.isEmpty {
                    MetadataRow(label: UIStrings.guidedCleanupFlowStep, value: stepID)
                }
                if let decision = record.decision, !decision.isEmpty {
                    MetadataRow(label: UIStrings.remediationHistoryDecision, value: decision)
                }
                if let sourceMethod = record.sourceMethod, !sourceMethod.isEmpty {
                    MetadataRow(label: UIStrings.remediationHistorySourceMethod, value: sourceMethod)
                }
                if let recordedAt = record.recordedAt, !recordedAt.isEmpty {
                    MetadataRow(label: UIStrings.remediationHistoryRecordedAt, value: recordedAt)
                }
                MetadataRow(label: UIStrings.guidedCleanupFlowMetadataRedacted, value: record.redacted ? UIStrings.stateEnabled : UIStrings.llmSkillAnalysisEnabledUnsafe)
                MetadataRow(label: UIStrings.guidedCleanupFlowAppLocalOnly, value: record.appLocalOnly ? UIStrings.stateEnabled : UIStrings.llmSkillAnalysisEnabledUnsafe)
            }
            if !record.note.isEmpty {
                Text(record.note)
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }
            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: record.evidenceRefs, systemImage: "checklist")
            RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: record.safetyFlags, systemImage: "checkmark.shield")
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
    }
}

func guidedCleanupIcon(for kind: String) -> String {
    let normalized = kind.lowercased()
    if normalized.contains("risk") || normalized.contains("finding") || normalized.contains("issue") {
        return "exclamationmark.triangle"
    }
    if normalized.contains("history") || normalized.contains("record") {
        return "archivebox"
    }
    if normalized.contains("batch") || normalized.contains("review") {
        return "rectangle.stack.badge.checkmark"
    }
    if normalized.contains("impact") {
        return "scope"
    }
    if normalized.contains("draft") || normalized.contains("fix") {
        return "doc.text.magnifyingglass"
    }
    if normalized.contains("plan") || normalized.contains("remediation") {
        return "wrench.and.screwdriver"
    }
    if normalized.contains("safe") {
        return "checkmark.shield"
    }
    return "list.clipboard"
}
