import SwiftUI

struct ContextualIntelligenceFact: Identifiable, Hashable {
    let label: String
    let value: String

    var id: String { "\(label):\(value)" }
}

struct ContextualEvidenceSelection: Identifiable {
    let reference: EvidenceRef

    var id: String { reference.id }
}

struct ContextualEvidenceSheet: View {
    let selection: ContextualEvidenceSelection

    var body: some View {
        WorkflowSheetShell(
            title: UIStrings.text("intelligence.evidence.title", "Exact evidence"),
            systemImage: "doc.text.magnifyingglass",
            subtitle: selection.reference.kind.rawValue
                .replacingOccurrences(of: "_", with: " ")
                .capitalized
        ) {
            VStack(alignment: .leading, spacing: 14) {
                Text(selection.reference.summary)
                    .fixedSize(horizontal: false, vertical: true)
                    .textSelection(.enabled)
                CompactMetadataGrid(rows: [
                    CompactMetadataRow(
                        label: UIStrings.text("intelligence.evidence.kind", "Kind"),
                        value: selection.reference.kind.rawValue
                    ),
                    CompactMetadataRow(
                        label: UIStrings.text("intelligence.evidence.revision", "Source revision"),
                        value: selection.reference.sourceRevision
                    ),
                    CompactMetadataRow(
                        label: UIStrings.text("intelligence.evidence.agent", "Agent"),
                        value: selection.reference.agent.map {
                            DisplayText.agent($0.rawValue)
                        } ?? UIStrings.text("intelligence.evidence.project", "Project")
                    ),
                ])
                PrivacyEvidenceText(
                    value: selection.reference.id,
                    font: .caption.monospaced(),
                    lineLimit: 2
                )
                Spacer()
            }
            .padding(18)
        }
        .frame(minWidth: 600, minHeight: 360)
    }
}

struct ContextualIntelligenceView: View {
    let kind: ContextualIntelligenceKind
    let deterministicTitle: String
    let deterministicFacts: [ContextualIntelligenceFact]
    let flow: ContextualIntelligenceFlow?
    let currentSourceRevision: String?
    let providerGateMessage: String?
    let onPreview: () -> Void
    let onConfirm: () -> Void
    let onDismissPreview: () -> Void
    let onOpenEvidence: (EvidenceRef) -> Void

    private var isStale: Bool {
        flow?.isStale(currentSourceRevision: currentSourceRevision) ?? false
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            deterministicCard
            aiCard
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("contextual-intelligence.\(kind.rawValue)")
    }

    private var deterministicCard: some View {
        VStack(alignment: .leading, spacing: 9) {
            Label(
                UIStrings.text(
                    "intelligence.deterministic.label",
                    "Verified deterministic facts"
                ),
                systemImage: "checkmark.seal"
            )
            .font(.headline)
            Text(deterministicTitle)
                .font(.callout)
                .foregroundStyle(.secondary)
            ForEach(deterministicFacts) { fact in
                HStack(alignment: .firstTextBaseline, spacing: 10) {
                    Text(fact.label)
                        .foregroundStyle(.secondary)
                    Spacer(minLength: 12)
                    Text(fact.value)
                        .multilineTextAlignment(.trailing)
                        .textSelection(.enabled)
                }
                .font(.callout)
            }
        }
        .padding(12)
        .nativePanelSurface()
    }

    private var aiCard: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Label(kind.title, systemImage: "sparkles")
                    .font(.headline)
                Text(UIStrings.text("intelligence.ai.label", "AI interpretation"))
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(.purple)
                if isStale {
                    Text(UIStrings.text("intelligence.stale", "Stale"))
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(.orange)
                }
                Spacer()
            }

            if let flow {
                flowContent(flow)
            } else {
                idleContent
            }
        }
        .padding(12)
        .background(
            Color.purple.opacity(0.045),
            in: RoundedRectangle(cornerRadius: 10)
        )
        .overlay {
            RoundedRectangle(cornerRadius: 10)
                .stroke(Color.purple.opacity(0.18), lineWidth: 1)
        }
    }

    @ViewBuilder
    private func flowContent(_ flow: ContextualIntelligenceFlow) -> some View {
        switch flow.phase {
        case .idle:
            idleContent
        case .previewing:
            progress(
                UIStrings.text(
                    "intelligence.previewing",
                    "Preparing a redacted provider preview..."
                )
            )
        case .sending:
            progress(
                UIStrings.text(
                    "intelligence.sending",
                    "Waiting for evidence-bound interpretation..."
                )
            )
        case .awaitingConfirmation:
            if let preview = flow.preview {
                promptPreview(preview)
            }
        case .complete:
            if let output = flow.output {
                outputView(output, citations: flow.citations)
            }
        case .failed:
            Label(
                flow.errorMessage
                    ?? UIStrings.text(
                        "intelligence.failed",
                        "Contextual intelligence failed."
                    ),
                systemImage: "exclamationmark.triangle"
            )
            .foregroundStyle(.orange)
            Button(
                UIStrings.text("intelligence.retry", "Preview again"),
                action: onPreview
            )
            .disabled(providerGateMessage != nil)
        }
    }

    private var idleContent: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(
                UIStrings.text(
                    "intelligence.optional",
                    "Optional and contextual. The verified workspace remains fully usable without a provider request."
                )
            )
            .font(.callout)
            .foregroundStyle(.secondary)
            if let providerGateMessage {
                Label(providerGateMessage, systemImage: "nosign")
                    .font(.callout)
                    .foregroundStyle(.secondary)
            } else {
                Button(action: onPreview) {
                    Label(
                        UIStrings.text(
                            "intelligence.preview",
                            "Preview provider request"
                        ),
                        systemImage: "eye"
                    )
                }
                .accessibilityIdentifier("contextual-intelligence.preview")
            }
        }
    }

    private func progress(_ message: String) -> some View {
        HStack(spacing: 8) {
            ProgressView().controlSize(.small)
            Text(message)
                .font(.callout)
                .foregroundStyle(.secondary)
        }
    }

    private func promptPreview(_ preview: LLMPromptPreview) -> some View {
        VStack(alignment: .leading, spacing: 9) {
            Label(
                UIStrings.text(
                    "intelligence.confirm.title",
                    "Review before sending"
                ),
                systemImage: "network.badge.shield.half.filled"
            )
            .font(.subheadline.bold())
            CompactMetadataGrid(rows: [
                CompactMetadataRow(
                    label: UIStrings.text("intelligence.destination", "Destination"),
                    value: preview.destinationHost
                        ?? preview.endpoint
                        ?? UIStrings.unknown
                ),
                CompactMetadataRow(
                    label: UIStrings.text("intelligence.model", "Model"),
                    value: preview.model ?? UIStrings.unknown
                ),
                CompactMetadataRow(
                    label: UIStrings.text("intelligence.redaction", "Redaction"),
                    value: preview.redaction.summary.isEmpty
                        ? preview.redaction.status
                        : preview.redaction.summary
                ),
            ])
            DisclosureGroup(
                UIStrings.text(
                    "intelligence.preview.details",
                    "Included and excluded data"
                )
            ) {
                VStack(alignment: .leading, spacing: 8) {
                    fieldList(
                        title: UIStrings.text("intelligence.included", "Included"),
                        fields: preview.includedFields
                    )
                    fieldList(
                        title: UIStrings.text("intelligence.excluded", "Excluded"),
                        fields: preview.excludedFields
                    )
                    if let prompt = preview.promptPreview, !prompt.isEmpty {
                        Text(prompt)
                            .font(.caption.monospaced())
                            .textSelection(.enabled)
                            .padding(8)
                            .background(
                                Color.agentCopilotPanelBackground,
                                in: RoundedRectangle(cornerRadius: 7)
                            )
                    }
                }
                .padding(.top, 7)
            }
            if isStale {
                Label(
                    UIStrings.text(
                        "intelligence.stale.preview",
                        "The evidence changed. Preview the provider request again."
                    ),
                    systemImage: "clock.arrow.circlepath"
                )
                .foregroundStyle(.orange)
            }
            HStack {
                Button(UIStrings.cancel, action: onDismissPreview)
                Spacer()
                if isStale {
                    Button(
                        UIStrings.text("intelligence.previewAgain", "Preview again"),
                        action: onPreview
                    )
                } else {
                    Button(action: onConfirm) {
                        Label(
                            UIStrings.text(
                                "intelligence.confirm.send",
                                "Confirm and send"
                            ),
                            systemImage: "paperplane"
                        )
                    }
                    .keyboardShortcut(.defaultAction)
                    .accessibilityIdentifier("contextual-intelligence.confirm")
                }
            }
        }
    }

    private func fieldList(title: String, fields: [LLMPromptField]) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(title)
                .font(.caption.bold())
            Text(fields.isEmpty ? UIStrings.none : fields.map(\.label).joined(separator: " · "))
                .font(.caption)
                .foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    private func outputView(
        _ output: ContextualIntelligenceOutput,
        citations: [EvidenceRef]
    ) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            if isStale {
                Label(
                    UIStrings.text(
                        "intelligence.stale.output",
                        "This interpretation belongs to an older evidence revision. It remains visible for comparison but cannot drive any action."
                    ),
                    systemImage: "clock.arrow.circlepath"
                )
                .font(.callout)
                .foregroundStyle(.orange)
            }
            Text(output.summary)
                .fixedSize(horizontal: false, vertical: true)
                .textSelection(.enabled)
            ForEach(output.sections) { section in
                VStack(alignment: .leading, spacing: 4) {
                    Text(section.title)
                        .font(.subheadline.bold())
                    ForEach(Array(section.rows.enumerated()), id: \.offset) { _, row in
                        Text("• \(row)")
                            .fixedSize(horizontal: false, vertical: true)
                    }
                }
                .font(.callout)
            }
            if let prompt = output.suggestedNextPrompt {
                VStack(alignment: .leading, spacing: 4) {
                    Text(
                        UIStrings.text(
                            "intelligence.nextPrompt",
                            "Suggested next prompt"
                        )
                    )
                    .font(.subheadline.bold())
                    Text(prompt)
                        .font(.callout.monospaced())
                        .textSelection(.enabled)
                }
            }
            if !output.unsupportedClaims.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    Text(
                        UIStrings.text(
                            "intelligence.unsupported",
                            "Unsupported claims"
                        )
                    )
                    .font(.subheadline.bold())
                    ForEach(Array(output.unsupportedClaims.enumerated()), id: \.offset) { _, row in
                        Label(row, systemImage: "questionmark.diamond")
                            .font(.callout)
                            .foregroundStyle(.orange)
                    }
                }
            }
            if !citations.isEmpty {
                VStack(alignment: .leading, spacing: 6) {
                    Text(UIStrings.text("intelligence.citations", "Evidence citations"))
                        .font(.subheadline.bold())
                    ForEach(citations) { reference in
                        Button {
                            onOpenEvidence(reference)
                        } label: {
                            Label(reference.summary, systemImage: "arrow.up.right.square")
                                .lineLimit(2)
                                .frame(maxWidth: .infinity, alignment: .leading)
                        }
                        .buttonStyle(.link)
                    }
                }
            }
            Button(
                UIStrings.text("intelligence.refreshInterpretation", "Preview a new interpretation"),
                action: onPreview
            )
            .disabled(providerGateMessage != nil)
        }
    }
}
