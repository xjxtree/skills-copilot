import SwiftUI

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
                    Text(preview.snapshot.target)
                        .foregroundStyle(.secondary)
                        .textSelection(.enabled)
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
                SnapshotTextPane(
                    title: UIStrings.current,
                    content: displayContent(preview.currentContent)
                )
                SnapshotTextPane(
                    title: UIStrings.snapshot,
                    content: displayContent(preview.snapshot.content)
                )
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
            .background(
                Color(nsColor: .textBackgroundColor),
                in: RoundedRectangle(cornerRadius: 6)
            )
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
        .nativePanelSurface()
    }
}
