import AppKit
import SwiftUI

struct ProjectOverviewEvidenceSelection: Identifiable {
    let item: AttentionItem
    let evidence: [EvidenceRef]

    var id: String { item.id }
}

struct ProjectOverviewActionSelection: Identifiable {
    let item: AttentionItem
    let action: ActionDescriptorWire

    var id: String { "\(item.id):\(action.id)" }
}

struct ProjectOverviewResumeSelection: Identifiable {
    let session: SessionContinuationRecord

    var id: String { session.id }
}

struct ProjectOverviewEvidenceSheet: View {
    let selection: ProjectOverviewEvidenceSelection

    var body: some View {
        WorkflowSheetShell(
            title: UIStrings.text("overview.evidence.title", "Evidence"),
            systemImage: "doc.text.magnifyingglass",
            subtitle: selection.item.title
        ) {
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 10) {
                    Text(selection.item.summary)
                        .font(.callout)
                        .foregroundStyle(.secondary)

                    if selection.evidence.isEmpty {
                        Text(
                            UIStrings.text(
                                "overview.evidence.empty",
                                "No current evidence reference is available."
                            )
                        )
                        .foregroundStyle(.secondary)
                    } else {
                        ForEach(selection.evidence, id: \.id) { evidence in
                            VStack(alignment: .leading, spacing: 5) {
                                HStack {
                                    Text(evidence.kind.rawValue.replacingOccurrences(
                                        of: "_",
                                        with: " "
                                    ).capitalized)
                                    .font(.headline)
                                    Spacer()
                                    if let agent = evidence.agent {
                                        Text(DisplayText.agent(agent.rawValue))
                                            .font(.caption)
                                            .foregroundStyle(.secondary)
                                    }
                                }
                                Text(evidence.summary)
                                    .font(.callout)
                                    .fixedSize(horizontal: false, vertical: true)
                                PrivacyEvidenceText(
                                    value: evidence.id,
                                    font: .caption2,
                                    lineLimit: 1
                                )
                            }
                            .padding(12)
                            .nativePanelSurface()
                        }
                    }
                }
                .padding(18)
            }
        }
        .frame(minWidth: 620, minHeight: 420)
    }
}

struct ProjectOverviewActionPreviewSheet: View {
    @Environment(\.dismiss) private var dismiss
    let selection: ProjectOverviewActionSelection
    let onOpenTarget: () -> Void

    var body: some View {
        WorkflowSheetShell(
            title: UIStrings.text("overview.actionPreview.title", "Typed action preview"),
            systemImage: "eye",
            subtitle: selection.item.title
        ) {
            ScrollView {
                VStack(alignment: .leading, spacing: 14) {
                    Label(
                        UIStrings.text(
                            "overview.actionPreview.readOnly",
                            "This overview shows the service-owned action capability. It never applies the action."
                        ),
                        systemImage: "lock.shield"
                    )
                    .foregroundStyle(.secondary)

                    Text(selection.item.summary)
                        .font(.callout)

                    GroupBox(UIStrings.text("overview.actionPreview.contract", "Action contract")) {
                        VStack(alignment: .leading, spacing: 8) {
                            MetadataRow(
                                label: UIStrings.text("overview.actionPreview.target", "Target"),
                                value: "\(selection.action.target.kind): \(selection.action.target.id)"
                            )
                            MetadataRow(
                                label: UIStrings.text("overview.actionPreview.previewMethod", "Preview method"),
                                value: selection.action.previewMethod
                            )
                            MetadataRow(
                                label: UIStrings.text("overview.actionPreview.impacts", "Impacts"),
                                value: selection.action.impacts.joined(separator: ", ")
                            )
                            MetadataRow(
                                label: UIStrings.text("overview.actionPreview.network", "Network"),
                                value: selection.action.network
                            )
                            MetadataRow(
                                label: UIStrings.text("overview.actionPreview.confirmation", "Confirmation"),
                                value: selection.action.confirmationRequired
                                    ? UIStrings.text("overview.actionPreview.required", "Required")
                                    : UIStrings.text("overview.actionPreview.notRequired", "Not required")
                            )
                        }
                        .padding(.vertical, 4)
                    }

                    HStack {
                        Spacer()
                        Button {
                            onOpenTarget()
                            dismiss()
                        } label: {
                            Label(
                                UIStrings.text("overview.actionPreview.openTarget", "Open target"),
                                systemImage: "arrow.right.circle"
                            )
                        }
                        .buttonStyle(.borderedProminent)
                    }
                }
                .padding(18)
            }
        }
        .frame(minWidth: 620, minHeight: 440)
    }
}

struct ProjectOverviewResumePreviewSheet: View {
    let selection: ProjectOverviewResumeSelection

    private var command: SessionResumeCommandPresentation? {
        SessionResumeCommandPresentation(session: selection.session)
    }

    var body: some View {
        WorkflowSheetShell(
            title: UIStrings.text("overview.resume.title", "Continuation preview"),
            systemImage: "doc.on.clipboard",
            subtitle: selection.session.title
        ) {
            VStack(alignment: .leading, spacing: 14) {
                Label(
                    UIStrings.text(
                        "overview.resume.copyOnly",
                        "Copy-only: Agent Copilot will not launch a terminal or resume the session."
                    ),
                    systemImage: "hand.raised"
                )
                .foregroundStyle(.secondary)

                if let command {
                    Text(command.command)
                        .font(.system(.body, design: .monospaced))
                        .textSelection(.enabled)
                        .padding(14)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .nativePanelSurface()

                    HStack {
                        Label(
                            DisplayText.agent(selection.session.agent.rawValue),
                            systemImage: "person.crop.circle"
                        )
                        .foregroundStyle(.secondary)
                        Spacer()
                        Button {
                            NSPasteboard.general.clearContents()
                            NSPasteboard.general.setString(command.command, forType: .string)
                        } label: {
                            Label(
                                UIStrings.text("overview.resume.copy", "Copy command"),
                                systemImage: "doc.on.doc"
                            )
                        }
                        .buttonStyle(.borderedProminent)
                    }
                } else {
                    Text(
                        ProjectOverviewPresentation.unsupportedResumeText(
                            selection.session.resume.unsupportedReason
                        )
                    )
                    .foregroundStyle(.secondary)
                }
            }
            .padding(18)
        }
        .frame(minWidth: 620, minHeight: 360)
    }
}
