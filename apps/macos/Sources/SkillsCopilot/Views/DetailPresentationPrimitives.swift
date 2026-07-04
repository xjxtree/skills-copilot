import AppKit
import SwiftUI

struct SafetyPill: View {
    let label: String
    let isBlocked: Bool

    var body: some View {
        Label(label, systemImage: isBlocked ? "lock" : "exclamationmark.triangle")
            .font(.caption2.bold())
            .padding(.horizontal, 7)
            .padding(.vertical, 3)
            .background(Color.agentCopilotPanelBackground, in: Capsule())
            .foregroundStyle(.secondary)
    }
}

struct SummaryChip: View {
    let title: String
    let value: String
    let systemImage: String
    let valueLineLimit: Int?
    let valueTruncationMode: Text.TruncationMode

    init(
        title: String,
        value: String,
        systemImage: String,
        valueLineLimit: Int? = 2,
        valueTruncationMode: Text.TruncationMode = .middle
    ) {
        self.title = title
        self.value = value
        self.systemImage = systemImage
        self.valueLineLimit = valueLineLimit
        self.valueTruncationMode = valueTruncationMode
    }

    var body: some View {
        HStack(alignment: .center, spacing: 10) {
            Image(systemName: systemImage)
                .font(.title3)
                .foregroundStyle(.secondary)
                .frame(width: 24)

            VStack(alignment: .leading, spacing: 1) {
                Text(title)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                Text(value)
                    .font(.callout.bold())
                    .lineLimit(valueLineLimit)
                    .truncationMode(valueTruncationMode)
                    .fixedSize(horizontal: false, vertical: valueLineLimit == nil)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.horizontal, 10)
        .frame(maxWidth: .infinity, minHeight: 54, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .combine)
        .accessibilityLabel(title)
        .accessibilityValue(value)
    }
}

struct DetailMetricGrid<Content: View>: View {
    let maxColumns: Int
    let minColumnWidth: CGFloat
    let spacing: CGFloat
    @ViewBuilder let content: () -> Content

    init(
        maxColumns: Int = 3,
        minColumnWidth: CGFloat = 170,
        spacing: CGFloat = 10,
        @ViewBuilder content: @escaping () -> Content
    ) {
        self.maxColumns = max(1, min(maxColumns, 4))
        self.minColumnWidth = minColumnWidth
        self.spacing = spacing
        self.content = content
    }

    var body: some View {
        ViewThatFits(in: .horizontal) {
            if maxColumns >= 4 {
                grid(columnCount: 4)
            }
            if maxColumns >= 3 {
                grid(columnCount: 3)
            }
            if maxColumns >= 2 {
                grid(columnCount: 2)
            }
            grid(columnCount: 1)
        }
    }

    private func grid(columnCount: Int) -> some View {
        LazyVGrid(
            columns: Array(repeating: GridItem(.flexible(minimum: minColumnWidth), spacing: spacing), count: columnCount),
            alignment: .leading,
            spacing: spacing
        ) {
            content()
        }
    }
}

struct CompactMetadataGrid: View {
    let rows: [CompactMetadataRow]
    var labelWidth = CGFloat(UIOptimizationPresentation.detailHeader.metadataLabelWidth)

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            ForEach(Array(rows.enumerated()), id: \.offset) { index, row in
                CompactMetadataRowView(row: row, labelWidth: labelWidth)
                    .id("\(row.label)-\(index)")
            }
        }
        .accessibilityElement(children: .contain)
    }
}

struct CompactMetadataRowView: View {
    let row: CompactMetadataRow
    let labelWidth: CGFloat

    var body: some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            HStack(spacing: 5) {
                if let systemImage = row.systemImage {
                    Image(systemName: systemImage)
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                        .frame(width: 13)
                }
                Text(row.label)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
            .frame(width: labelWidth, alignment: .leading)

            Text(row.value)
                .font(.system(.caption, design: .monospaced))
                .foregroundStyle(.primary)
                .lineLimit(1)
                .truncationMode(.middle)
                .textSelection(.enabled)
                .help(row.value)
                .frame(maxWidth: .infinity, alignment: .leading)

            if row.isCopyable {
                Button {
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(row.value, forType: .string)
                } label: {
                    Image(systemName: "doc.on.doc")
                }
                .buttonStyle(.borderless)
                .controlSize(.small)
                .help(UIStrings.text("action.copy", "Copy"))
                .accessibilityLabel("\(UIStrings.text("action.copy", "Copy")) \(row.label)")
            }
        }
        .frame(minHeight: CGFloat(UIOptimizationPresentation.detailHeader.metadataRowHeight), alignment: .center)
        .contentShape(Rectangle())
    }
}

struct DenseCountBadge: View {
    let count: Int

    var body: some View {
        Text("\(count)")
            .font(.caption2.monospacedDigit().bold())
            .foregroundStyle(.secondary)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(Color.agentCopilotPanelBackground, in: Capsule())
    }
}

struct DenseDisclosureList<Item, RowContent: View>: View {
    let items: [Item]
    let visibleLimit: Int
    let spacing: CGFloat
    let rowContent: (Item) -> RowContent
    @State private var isExpanded = false

    init(
        _ items: [Item],
        visibleLimit: Int = 6,
        spacing: CGFloat = 4,
        @ViewBuilder rowContent: @escaping (Item) -> RowContent
    ) {
        self.items = items
        self.visibleLimit = max(0, visibleLimit)
        self.spacing = spacing
        self.rowContent = rowContent
    }

    var body: some View {
        VStack(alignment: .leading, spacing: spacing) {
            ForEach(Array(items.prefix(visibleLimit).enumerated()), id: \.offset) { _, item in
                rowContent(item)
            }

            if hiddenCount > 0 {
                DisclosureGroup(isExpanded: $isExpanded) {
                    VStack(alignment: .leading, spacing: spacing) {
                        ForEach(Array(items.dropFirst(visibleLimit).enumerated()), id: \.offset) { _, item in
                            rowContent(item)
                        }
                    }
                    .padding(.top, 2)
                } label: {
                    Label("+\(hiddenCount)", systemImage: "ellipsis.circle")
                        .font(.caption.bold())
                        .foregroundStyle(.secondary)
                }
            }
        }
    }

    private var hiddenCount: Int {
        max(0, items.count - visibleLimit)
    }
}

struct RoutingInlineList: View {
    let title: String
    let empty: String
    let values: [String]
    let systemImage: String

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            HStack(spacing: 6) {
                Text(title)
                    .font(.caption2.bold())
                    .foregroundStyle(.secondary)
                if !values.isEmpty {
                    DenseCountBadge(count: values.count)
                }
            }
            if values.isEmpty {
                Text(empty)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            } else {
                DenseDisclosureList(values, visibleLimit: 3, spacing: 3) { value in
                    PrivacyEvidenceLabel(value: value, systemImage: systemImage, font: .caption, lineLimit: 2)
                }
            }
        }
    }
}

struct MetadataRow: View {
    let label: String
    let value: String

    var body: some View {
        GridRow {
            Text(label)
                .foregroundStyle(.secondary)
            Text(value)
                .textSelection(.enabled)
                .lineLimit(3)
        }
    }
}

struct MetadataLine: View {
    let label: String
    let value: String

    var body: some View {
        HStack(alignment: .firstTextBaseline) {
            Text(label)
                .foregroundStyle(.secondary)
                .frame(minWidth: 80, alignment: .leading)
            Text(value)
                .textSelection(.enabled)
                .lineLimit(3)
        }
    }
}

struct EmptyState: View {
    let title: String
    let systemImage: String
    let message: String

    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: systemImage)
                .font(.system(size: 34))
                .foregroundStyle(.secondary)
            Text(title)
                .font(.title3.bold())
            Text(message)
                .foregroundStyle(.secondary)
        }
        .padding(28)
        .frame(maxWidth: 900, minHeight: 220)
        .nativePanelSurface()
    }
}

struct ErrorBanner: View {
    let message: String

    var body: some View {
        Label(message, systemImage: "exclamationmark.triangle.fill")
            .foregroundStyle(.red)
            .padding(.vertical, 10)
            .padding(.horizontal, 12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .nativePanelSurface()
            .overlay(alignment: .leading) {
                Rectangle()
                    .fill(Color.red)
                    .frame(width: 3)
                    .clipShape(Capsule())
            }
    }
}

struct DetailFeedbackToast: View {
    let message: String
    let systemImage: String
    let color: Color

    var body: some View {
        Label(message, systemImage: systemImage)
            .font(.caption)
            .foregroundStyle(color)
            .lineLimit(3)
            .padding(.horizontal, 12)
            .padding(.vertical, 9)
            .frame(maxWidth: CGFloat(UIOptimizationPresentation.detailFeedback.maximumWidth), alignment: .leading)
            .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: CGFloat(UIOptimizationPresentation.detailFeedback.cornerRadius)))
            .overlay(
                RoundedRectangle(cornerRadius: CGFloat(UIOptimizationPresentation.detailFeedback.cornerRadius))
                    .stroke(color.opacity(0.18), lineWidth: 1)
            )
    }
}

struct SuccessBanner: View {
    let message: String

    var body: some View {
        Label(message, systemImage: "checkmark.circle.fill")
            .foregroundStyle(.green)
            .padding(.vertical, 10)
            .padding(.horizontal, 12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .nativePanelSurface()
            .overlay(alignment: .leading) {
                Rectangle()
                    .fill(Color.green)
                    .frame(width: 3)
                    .clipShape(Capsule())
            }
    }
}
