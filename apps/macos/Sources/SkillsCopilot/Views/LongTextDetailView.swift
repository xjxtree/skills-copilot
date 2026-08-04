import AppKit
import SwiftUI

struct LongTextPreviewCard: View {
    let title: String
    let text: String
    var tint: Color = .secondary

    @State private var showsDetail = false
    @State private var didCopy = false

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 8) {
                Text(title)
                    .font(.caption.bold())
                Spacer()
                Button {
                    copyText()
                } label: {
                    Label(
                        didCopy ? UIStrings.text("action.copied", "Copied") : UIStrings.text("action.copy", "Copy"),
                        systemImage: didCopy ? "checkmark" : "doc.on.doc"
                    )
                }
                .buttonStyle(.borderless)
                .controlSize(.small)

                Button {
                    showsDetail = true
                } label: {
                    Label(UIStrings.text("action.viewComplete", "View complete"), systemImage: "arrow.up.left.and.arrow.down.right")
                }
                .buttonStyle(.borderless)
                .controlSize(.small)
            }

            ScrollView([.horizontal, .vertical]) {
                Text(text)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(tint)
                    .textSelection(.enabled)
                    .fixedSize(horizontal: true, vertical: true)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .frame(minHeight: 72, maxHeight: 180)
            .padding(8)
            .background(Color.agentCopilotWindowBackground.opacity(0.7), in: RoundedRectangle(cornerRadius: 6))
        }
        .sheet(isPresented: $showsDetail) {
            LongTextDetailSheet(title: title, text: text, renderMode: .plain)
        }
    }

    private func copyText() {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
        didCopy = true
        Task { @MainActor in
            try? await Task.sleep(nanoseconds: 1_500_000_000)
            didCopy = false
        }
    }
}
