import AppKit
import SwiftUI

struct AgentTraceImportPanel: View {
    @Binding var traceText: String
    @Binding var title: String
    @Binding var taskText: String
    @Binding var expectedSkills: String
    let listResult: AgentTraceImportListResult
    let importResult: AgentTraceImportResult?
    let deleteResult: AgentTraceImportDeleteResult?
    let latestRecord: AgentTraceImportRecord?
    let isLoading: Bool
    let isImporting: Bool
    let isDeleting: (AgentTraceImportRecord) -> Bool
    let onLoad: () -> Void
    let onImport: () -> Void
    let onDelete: (AgentTraceImportRecord) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.traceImportTitle, systemImage: "tray.and.arrow.down.fill")
                    .font(.headline)
                Spacer()
                Label(UIStrings.readOnlyPreview, systemImage: "lock.shield")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.traceImportBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)
            Label(UIStrings.traceImportProviderBoundary, systemImage: "nosign")
                .font(.callout)
                .foregroundStyle(.secondary)

            VStack(alignment: .leading, spacing: 8) {
                TextField(UIStrings.traceImportTitlePlaceholder, text: $title)
                    .textFieldStyle(.roundedBorder)
                TextField(UIStrings.traceImportTaskPlaceholder, text: $taskText, axis: .vertical)
                    .textFieldStyle(.roundedBorder)
                    .lineLimit(1...3)
                TextField(UIStrings.traceImportExpectedPlaceholder, text: $expectedSkills, axis: .vertical)
                    .textFieldStyle(.roundedBorder)
                    .lineLimit(1...2)
                ZStack(alignment: .topLeading) {
                    if traceText.isEmpty {
                        Text(UIStrings.traceImportTextPlaceholder)
                            .font(.callout)
                            .foregroundStyle(.tertiary)
                            .padding(.horizontal, 7)
                            .padding(.vertical, 8)
                    }
                    TextEditor(text: $traceText)
                        .font(.system(.callout, design: .monospaced))
                        .scrollContentBackground(.hidden)
                        .frame(minHeight: 82, maxHeight: 120)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 6))
                }
            }

            HStack(spacing: 8) {
                Button {
                    onImport()
                } label: {
                    Label(UIStrings.traceImportImportAction, systemImage: "square.and.arrow.down")
                }
                .disabled(isImporting || traceText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)

                Button {
                    onLoad()
                } label: {
                    Label(UIStrings.traceImportLoadAction, systemImage: "arrow.clockwise")
                }
                .disabled(isLoading)

                Spacer()
            }

            if isLoading || isImporting {
                Label(UIStrings.llmPreparing, systemImage: "hourglass")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            if let reason = importResult?.fallbackReason, !reason.isEmpty {
                Label(reason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            if let latestRecord {
                VStack(alignment: .leading, spacing: 8) {
                    Text(UIStrings.traceImportLatest)
                        .font(.caption.bold())
                        .foregroundStyle(.secondary)
                    AgentTraceImportRecordView(record: latestRecord, compact: false)
                }
            } else if listResult.imports.isEmpty {
                Label(UIStrings.traceImportNoImports, systemImage: "clock.badge.questionmark")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            AgentTraceImportListView(
                result: listResult,
                deleteResult: deleteResult,
                isDeleting: isDeleting,
                onDelete: onDelete
            )
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

struct AgentSessionSkillReviewPanel: View {
    @Binding var transcriptText: String
    @Binding var taskText: String
    @Binding var expectedSkills: String
    @Binding var localSessionRoots: String
    let listResult: AgentSessionSkillReviewListResult
    let reviewResult: AgentSessionSkillReviewResult?
    let deleteResult: AgentSessionSkillReviewDeleteResult?
    let localSessionPreviewResult: LocalSessionPreviewResult
    let latestRecord: AgentSessionSkillReviewRecord?
    let isLoading: Bool
    let isReviewing: Bool
    let isPreviewingLocalSessions: Bool
    let isDeleting: (AgentSessionSkillReviewRecord) -> Bool
    let onLoad: () -> Void
    let onReview: () -> Void
    let onPreviewLocalSessions: () -> Void
    let onDelete: (AgentSessionSkillReviewRecord) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.agentSessionReviewTitle, systemImage: "person.crop.rectangle.stack")
                    .font(.headline)
                Spacer()
                Label(UIStrings.agentSessionReviewAppLocal, systemImage: "archivebox")
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.agentSessionReviewBoundary)
                .font(.callout)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)
            Label(UIStrings.agentSessionReviewNoWriteBoundary, systemImage: "nosign")
                .font(.callout)
                .foregroundStyle(.secondary)

            LocalSessionPreviewPanel(
                result: localSessionPreviewResult,
                isPreviewing: isPreviewingLocalSessions,
                onPreview: onPreviewLocalSessions
            )

            VStack(alignment: .leading, spacing: 8) {
                TextField(UIStrings.agentSessionReviewTaskPlaceholder, text: $taskText, axis: .vertical)
                    .textFieldStyle(.roundedBorder)
                    .lineLimit(1...3)
                TextField(UIStrings.agentSessionReviewExpectedPlaceholder, text: $expectedSkills, axis: .vertical)
                    .textFieldStyle(.roundedBorder)
                    .lineLimit(1...2)
                ZStack(alignment: .topLeading) {
                    if transcriptText.isEmpty {
                        Text(UIStrings.agentSessionReviewTranscriptPlaceholder)
                            .font(.callout)
                            .foregroundStyle(.tertiary)
                            .padding(.horizontal, 7)
                            .padding(.vertical, 8)
                    }
                    TextEditor(text: $transcriptText)
                        .font(.system(.callout, design: .monospaced))
                        .scrollContentBackground(.hidden)
                        .frame(minHeight: 84, maxHeight: 126)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 6))
                }
            }

            HStack(spacing: 8) {
                Button {
                    onReview()
                } label: {
                    Label(UIStrings.agentSessionReviewAction, systemImage: "checkmark.bubble")
                }
                .disabled(isReviewing || transcriptText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)

                Button {
                    onLoad()
                } label: {
                    Label(UIStrings.agentSessionReviewLoadAction, systemImage: "arrow.clockwise")
                }
                .disabled(isLoading || isReviewing)

                Spacer()
            }

            if isLoading || isReviewing {
                Label(UIStrings.llmPreparing, systemImage: "hourglass")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            if let reason = reviewResult?.fallbackReason, !reason.isEmpty {
                Label(reason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            if let latestRecord {
                VStack(alignment: .leading, spacing: 8) {
                    Text(UIStrings.agentSessionReviewLatest)
                        .font(.caption.bold())
                        .foregroundStyle(.secondary)
                    AgentSessionSkillReviewRecordView(record: latestRecord, compact: false)
                }
            } else if listResult.reviews.isEmpty {
                Label(UIStrings.agentSessionReviewNoReviews, systemImage: "clock.badge.questionmark")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }

            AgentSessionSkillReviewListView(
                result: listResult,
                deleteResult: deleteResult,
                isDeleting: isDeleting,
                onDelete: onDelete
            )
        }
        .padding()
        .frame(maxWidth: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}

struct LocalSessionPreviewPanel: View {
    let result: LocalSessionPreviewResult
    let isPreviewing: Bool
    let onPreview: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline) {
                Label(UIStrings.text("localSessionPreview.title", "Local Session Sources"), systemImage: "folder.badge.gearshape")
                    .font(.callout.bold())
                Spacer()
                Text(UIStrings.text("localSessionPreview.mode", "Auto discovery"))
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
            }

            Text(UIStrings.text("localSessionPreview.boundary", "Automatically scans supported local agent session stores for the selected agent. The preview reads redacted metadata/excerpts only and does not create trace or review records."))
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)

            HStack(spacing: 8) {
                Button {
                    onPreview()
                } label: {
                    Label(UIStrings.text("localSessionPreview.action", "Refresh Sessions"), systemImage: "arrow.clockwise")
                }
                .disabled(isPreviewing)

                if isPreviewing {
                    Label(UIStrings.llmPreparing, systemImage: "hourglass")
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }

                Spacer()
            }

            if result.authorizationRequired {
                Label(UIStrings.text("localSessionPreview.authorizationRequired", "No supported local session store was found for the selected agent."), systemImage: "folder.badge.questionmark")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }

            ForEach(result.blockerNotes.prefix(3), id: \.self) { note in
                PrivacyEvidenceText(value: note, font: .caption, lineLimit: 2)
            }

            if !result.roots.isEmpty {
                DenseDisclosureList(result.roots.map(rootLabel), visibleLimit: 3, spacing: 4) { root in
                    PrivacyEvidenceText(value: root, font: .caption2, lineLimit: 1)
                }
            }

            if result.sessionRows.isEmpty {
                Text(result.fallbackReason ?? UIStrings.text("localSessionPreview.noRows", "No redacted local session previews are loaded."))
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                if !result.skillUsageRows.isEmpty {
                    VStack(alignment: .leading, spacing: 5) {
                        Text(UIStrings.text("localSessionPreview.skillUsage", "Skill usage from sessions"))
                            .font(.caption.bold())
                            .foregroundStyle(.secondary)
                        LazyVGrid(columns: [GridItem(.adaptive(minimum: 180), spacing: 8)], alignment: .leading, spacing: 8) {
                            ForEach(result.skillUsageRows.prefix(6)) { row in
                                HStack(spacing: 8) {
                                    Image(systemName: "square.stack.3d.up")
                                        .foregroundStyle(.secondary)
                                    VStack(alignment: .leading, spacing: 2) {
                                        Text(row.skillName)
                                            .font(.caption.bold())
                                            .lineLimit(1)
                                        Text("\(row.callCount) · \(row.sessionCount)")
                                            .font(.caption2)
                                            .foregroundStyle(.secondary)
                                    }
                                }
                                .padding(8)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
                            }
                        }
                    }
                }

                LazyVGrid(columns: [GridItem(.adaptive(minimum: 260), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(result.sessionRows.prefix(6)) { row in
                        VStack(alignment: .leading, spacing: 6) {
                            HStack(alignment: .firstTextBaseline, spacing: 8) {
                                Text(row.title)
                                    .font(.caption.bold())
                                    .lineLimit(1)
                                Spacer()
                                if let agent = row.agent, !agent.isEmpty {
                                    Text(DisplayText.agent(agent))
                                        .font(.caption2.bold())
                                        .foregroundStyle(.secondary)
                                }
                            }
                            PrivacyEvidenceText(value: row.redactedPath, font: .caption2, lineLimit: 1)
                            PrivacyEvidenceText(value: row.excerpt, font: .caption, lineLimit: 3)
                            RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: row.evidenceRefs, systemImage: "checklist")
                        }
                        .padding(8)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
    }

    private func rootLabel(_ root: LocalSessionPreviewRoot) -> String {
        if let blocker = root.blocker, !blocker.isEmpty {
            return "\(root.root) · \(root.status) · \(blocker)"
        }
        return "\(root.root) · \(root.status) · \(root.candidateCount)"
    }
}

struct AgentSessionSkillReviewListView: View {
    let result: AgentSessionSkillReviewListResult
    let deleteResult: AgentSessionSkillReviewDeleteResult?
    let isDeleting: (AgentSessionSkillReviewRecord) -> Bool
    let onDelete: (AgentSessionSkillReviewRecord) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Text(UIStrings.agentSessionReviewReviews)
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                Spacer()
                Text("\(result.reviews.count)")
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(.secondary)
            }

            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            if result.reviews.isEmpty {
                Text(UIStrings.agentSessionReviewNoReviews)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 260), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(result.reviews.prefix(10)) { record in
                        VStack(alignment: .leading, spacing: 7) {
                            HStack(alignment: .firstTextBaseline) {
                                Text(record.title.isEmpty ? (record.taskText.isEmpty ? record.id : record.taskText) : record.title)
                                    .font(.caption.bold())
                                    .lineLimit(2)
                                    .textSelection(.enabled)
                                Spacer()
                                Button {
                                    onDelete(record)
                                } label: {
                                    Image(systemName: "trash")
                                }
                                .buttonStyle(.borderless)
                                .disabled(isDeleting(record))
                                .help(UIStrings.agentSessionReviewDeleteAction)
                            }
                            AgentSessionSkillReviewRecordView(record: record, compact: true)
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }

            if let deleteResult, let reason = deleteResult.fallbackReason, !reason.isEmpty {
                Label(reason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }
        }
    }
}

struct AgentSessionSkillReviewRecordView: View {
    let record: AgentSessionSkillReviewRecord
    let compact: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: compact ? 7 : 10) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(record.outcome.isEmpty ? UIStrings.unknown : record.outcome)
                    .font(.caption2.bold())
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(outcomeTint.opacity(0.16), in: Capsule())
                    .foregroundStyle(outcomeTint)
                if !record.taskText.isEmpty {
                    Text(record.taskText)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(compact ? 1 : 2)
                        .textSelection(.enabled)
                }
                Spacer()
            }

            if !compact {
                DetailMetricGrid {
                    SummaryChip(title: UIStrings.agentSessionReviewDetectedSkills, value: "\(record.detectedSkills.count)", systemImage: "wrench.and.screwdriver")
                    SummaryChip(title: UIStrings.agentSessionReviewInterference, value: "\(record.interference.count)", systemImage: "exclamationmark.triangle")
                    SummaryChip(title: UIStrings.agentSessionReviewSafeNextSteps, value: "\(record.safeNextSteps.count)", systemImage: "arrow.right.circle")
                    SummaryChip(title: UIStrings.knowledgeSafetyFlags, value: "\(record.safetyFlags.count + record.safety.notes.count)", systemImage: "checkmark.shield")
                }

                Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                    MetadataRow(label: UIStrings.agentSessionReviewOutcome, value: record.outcome.isEmpty ? UIStrings.unknown : record.outcome)
                    if let agent = record.agent, !agent.isEmpty {
                        MetadataRow(label: UIStrings.agent, value: DisplayText.agent(agent))
                    }
                    if let createdAt = record.createdAt, !createdAt.isEmpty {
                        MetadataRow(label: UIStrings.remediationHistoryRecordedAt, value: createdAt)
                    }
                    MetadataRow(label: UIStrings.skillQualityProviderNotSent, value: record.safety.providerRequestSent ? UIStrings.llmSkillAnalysisEnabledUnsafe : UIStrings.llmDisabled)
                    MetadataRow(label: UIStrings.skillQualityWritesBlocked, value: readOnlyValue(!record.safety.writeBackAllowed && !record.safety.writeActionsAvailable))
                    MetadataRow(label: UIStrings.skillQualityScriptsBlocked, value: readOnlyValue(!record.safety.scriptExecutionAllowed && !record.safety.executionActionsAvailable))
                    MetadataRow(label: UIStrings.skillQualityMutationsBlocked, value: readOnlyValue(!record.safety.configMutationAllowed && !record.safety.snapshotCreated && !record.safety.triageMutationAllowed))
                    MetadataRow(label: UIStrings.skillQualityCredentialsBlocked, value: readOnlyValue(!record.safety.credentialAccessed && !record.safety.rawSecretReturned))
                }
            }

            if !record.summary.isEmpty {
                PrivacyEvidenceText(value: record.summary, font: .caption, lineLimit: compact ? 3 : nil)
            }

            AgentSessionSkillRefList(title: UIStrings.agentSessionReviewDetectedSkills, skills: record.detectedSkills)
            AgentSessionSkillRefList(title: UIStrings.agentSessionReviewExpectedSkills, skills: record.expectedSkills)
            AgentSessionInterferenceList(items: record.interference, compact: compact)
            RoutingInlineList(title: UIStrings.agentSessionReviewSafeNextSteps, empty: UIStrings.agentSessionReviewNoSafeNextSteps, values: record.safeNextSteps, systemImage: "arrow.right.circle")

            if !compact || !record.redactedExcerpt.isEmpty {
                VStack(alignment: .leading, spacing: 5) {
                    Text(UIStrings.agentSessionReviewRedactedExcerpt)
                        .font(.caption.bold())
                        .foregroundStyle(.secondary)
                    Text(record.redactedExcerpt.isEmpty ? UIStrings.agentSessionReviewNoExcerpt : record.redactedExcerpt)
                        .font(.system(.caption, design: .monospaced))
                        .foregroundStyle(record.redactedExcerpt.isEmpty ? .secondary : .primary)
                        .lineLimit(compact ? 3 : nil)
                        .textSelection(.enabled)
                }
            }

            RoutingInlineList(title: UIStrings.agentSessionReviewReasons, empty: UIStrings.agentSessionReviewNoReasons, values: record.reasons, systemImage: "text.bubble")
            RoutingInlineList(title: UIStrings.knowledgeSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: record.safetyFlags, systemImage: "lock.shield")
            CrossAgentReadinessEvidenceList(evidence: record.evidenceReferences)
            if !compact {
                CrossAgentReadinessSafetyList(safety: record.safety)
            }
        }
        .padding(compact ? 0 : 12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(compact ? Color.clear : Color.white, in: RoundedRectangle(cornerRadius: 6))
    }

    private var outcomeTint: Color {
        switch record.outcome.lowercased() {
        case "hit", "matched", "expected_match", "expected-match", "correct":
            return .green
        case "miss", "wrong_pick", "wrong-pick", "interference":
            return .red
        case "ambiguous", "partial":
            return .orange
        default:
            return .secondary
        }
    }

    private func readOnlyValue(_ isBlocked: Bool) -> String {
        isBlocked ? UIStrings.llmSkillAnalysisBlocked : UIStrings.llmSkillAnalysisEnabledUnsafe
    }
}

struct AgentSessionSkillRefList: View {
    let title: String
    let skills: [TaskBenchmarkSkillRef]

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            Text(title)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if skills.isEmpty {
                Text(UIStrings.agentSessionReviewNoSkills)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                Text(skills.map(skillLabel).joined(separator: ", "))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
                    .textSelection(.enabled)
            }
        }
    }

    private func skillLabel(_ skill: TaskBenchmarkSkillRef) -> String {
        if skill.agent == UIStrings.unknown || skill.agent.isEmpty {
            return skill.name
        }
        return "\(skill.name) (\(DisplayText.agent(skill.agent)))"
    }
}

struct AgentSessionInterferenceList: View {
    let items: [AgentSessionInterferenceSignal]
    let compact: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(UIStrings.agentSessionReviewInterference)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if items.isEmpty {
                Text(UIStrings.agentSessionReviewNoInterference)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                ForEach(items.prefix(compact ? 2 : 6)) { item in
                    VStack(alignment: .leading, spacing: 4) {
                        HStack(alignment: .firstTextBaseline) {
                            Label(item.title, systemImage: "exclamationmark.triangle")
                                .font(.caption.bold())
                                .lineLimit(1)
                            Spacer()
                            Text(item.severity)
                                .font(.caption2.bold())
                                .foregroundStyle(.secondary)
                        }
                        if let agent = item.agent, !agent.isEmpty {
                            Text(DisplayText.agent(agent))
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                        }
                        if let skill = item.skill {
                            Text(skillLabel(skill))
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                        }
                        PrivacyEvidenceText(value: item.detail, font: .caption, lineLimit: compact ? 2 : nil)
                        RoutingInlineList(title: UIStrings.crossAgentReadinessEvidence, empty: UIStrings.crossAgentReadinessNoEvidence, values: item.evidenceRefs, systemImage: "checklist")
                    }
                    .padding(compact ? 0 : 8)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(compact ? Color.clear : Color.white, in: RoundedRectangle(cornerRadius: 6))
                }
            }
        }
    }

    private func skillLabel(_ skill: TaskBenchmarkSkillRef) -> String {
        if skill.agent == UIStrings.unknown || skill.agent.isEmpty {
            return skill.name
        }
        return "\(skill.name) (\(DisplayText.agent(skill.agent)))"
    }
}

struct AgentTraceImportListView: View {
    let result: AgentTraceImportListResult
    let deleteResult: AgentTraceImportDeleteResult?
    let isDeleting: (AgentTraceImportRecord) -> Bool
    let onDelete: (AgentTraceImportRecord) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Text(UIStrings.traceImportImports)
                    .font(.caption.bold())
                    .foregroundStyle(.secondary)
                Spacer()
                Text("\(result.imports.count)")
                    .font(.caption.monospacedDigit())
                    .foregroundStyle(.secondary)
            }

            if let fallbackReason = result.fallbackReason, !fallbackReason.isEmpty {
                Label(fallbackReason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }

            if result.imports.isEmpty {
                Text(UIStrings.traceImportNoImports)
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 250), spacing: 8)], alignment: .leading, spacing: 8) {
                    ForEach(result.imports) { record in
                        VStack(alignment: .leading, spacing: 7) {
                            HStack(alignment: .firstTextBaseline) {
                                Text(record.title.isEmpty ? (record.taskText.isEmpty ? record.id : record.taskText) : record.title)
                                    .font(.caption.bold())
                                    .lineLimit(2)
                                    .textSelection(.enabled)
                                Spacer()
                                Button {
                                    onDelete(record)
                                } label: {
                                    Image(systemName: "trash")
                                }
                                .buttonStyle(.borderless)
                                .disabled(isDeleting(record))
                                .help(UIStrings.traceImportDeleteAction)
                            }
                            AgentTraceImportRecordView(record: record, compact: true)
                        }
                        .padding(10)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color.white, in: RoundedRectangle(cornerRadius: 8))
                    }
                }
            }

            if let deleteResult, let reason = deleteResult.fallbackReason, !reason.isEmpty {
                Label(reason, systemImage: "info.circle")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            }
        }
    }
}

struct AgentTraceImportRecordView: View {
    let record: AgentTraceImportRecord
    let compact: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: compact ? 7 : 10) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(record.outcome.isEmpty ? UIStrings.unknown : record.outcome)
                    .font(.caption2.bold())
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(outcomeTint.opacity(0.16), in: Capsule())
                    .foregroundStyle(outcomeTint)
                if !record.taskText.isEmpty {
                    Text(record.taskText)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(compact ? 1 : 2)
                        .textSelection(.enabled)
                }
                Spacer()
            }

            if !compact {
                Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                    MetadataRow(label: UIStrings.traceImportOutcome, value: record.outcome.isEmpty ? UIStrings.unknown : record.outcome)
                    MetadataRow(label: UIStrings.skillQualityProviderNotSent, value: record.safety.providerRequestSent ? UIStrings.llmSkillAnalysisEnabledUnsafe : UIStrings.llmDisabled)
                    MetadataRow(label: UIStrings.skillQualityWritesBlocked, value: readOnlyValue(!record.safety.writeBackAllowed && !record.safety.writeActionsAvailable))
                    MetadataRow(label: UIStrings.skillQualityScriptsBlocked, value: readOnlyValue(!record.safety.scriptExecutionAllowed && !record.safety.executionActionsAvailable))
                    MetadataRow(label: UIStrings.skillQualityMutationsBlocked, value: readOnlyValue(!record.safety.configMutationAllowed && !record.safety.snapshotCreated && !record.safety.triageMutationAllowed))
                    MetadataRow(label: UIStrings.skillQualityCredentialsBlocked, value: readOnlyValue(!record.safety.credentialAccessed && !record.safety.rawSecretReturned))
                }
            }

            AgentTraceSkillList(title: UIStrings.traceImportDetectedSkills, skills: record.detectedSkills)
            AgentTraceSkillList(title: UIStrings.traceImportExpectedSkills, skills: record.expectedSkills)

            if !compact || !record.redactedExcerpt.isEmpty {
                VStack(alignment: .leading, spacing: 5) {
                    Text(UIStrings.traceImportRedactedExcerpt)
                        .font(.caption.bold())
                        .foregroundStyle(.secondary)
                    Text(record.redactedExcerpt.isEmpty ? UIStrings.traceImportNoExcerpt : record.redactedExcerpt)
                        .font(.system(.caption, design: .monospaced))
                        .foregroundStyle(record.redactedExcerpt.isEmpty ? .secondary : .primary)
                        .lineLimit(compact ? 3 : nil)
                        .textSelection(.enabled)
                }
            }

            AgentTraceRedactionView(redaction: record.redaction)
            RoutingInlineList(title: UIStrings.traceImportReasons, empty: UIStrings.traceImportNoReasons, values: record.reasons, systemImage: "text.bubble")
            RoutingInlineList(title: UIStrings.taskBenchmarkSafetyFlags, empty: UIStrings.taskBenchmarkNoSafetyFlags, values: record.safetyFlags, systemImage: "lock.shield")
            RoutingEvidenceList(evidence: record.evidence)
        }
        .padding(compact ? 0 : 12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(compact ? Color.clear : Color.white, in: RoundedRectangle(cornerRadius: 6))
    }

    private var outcomeTint: Color {
        switch record.outcome.lowercased() {
        case "hit", "matched", "expected_match":
            return .green
        case "miss", "wrong_pick", "wrong-pick":
            return .red
        case "ambiguous":
            return .orange
        default:
            return .secondary
        }
    }

    private func readOnlyValue(_ isBlocked: Bool) -> String {
        isBlocked ? UIStrings.llmSkillAnalysisBlocked : UIStrings.llmSkillAnalysisEnabledUnsafe
    }
}

struct AgentTraceSkillList: View {
    let title: String
    let skills: [TaskBenchmarkSkillRef]

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            Text(title)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            if skills.isEmpty {
                Text(UIStrings.traceImportNoSkills)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                Text(skills.map(skillLabel).joined(separator: ", "))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(2)
                    .textSelection(.enabled)
            }
        }
    }

    private func skillLabel(_ skill: TaskBenchmarkSkillRef) -> String {
        if skill.agent == UIStrings.unknown || skill.agent.isEmpty {
            return skill.name
        }
        return "\(skill.name) (\(DisplayText.agent(skill.agent)))"
    }
}

struct AgentTraceRedactionView: View {
    let redaction: AgentTraceRedactionSummary

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            Text(UIStrings.traceImportRedactionSummary)
                .font(.caption.bold())
                .foregroundStyle(.secondary)
            Text(redaction.summary.isEmpty ? redaction.status : "\(redaction.status): \(redaction.summary)")
                .font(.caption)
                .foregroundStyle(.secondary)
                .textSelection(.enabled)
            if !redaction.redactedFields.isEmpty {
                Text(redaction.redactedFields.joined(separator: ", "))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }
            if !redaction.placeholders.isEmpty {
                Text(redaction.placeholders.joined(separator: ", "))
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }
            ForEach(redaction.warnings, id: \.self) { warning in
                Label(warning, systemImage: "exclamationmark.triangle")
                    .font(.caption)
                    .foregroundStyle(.orange)
            }
        }
    }
}
