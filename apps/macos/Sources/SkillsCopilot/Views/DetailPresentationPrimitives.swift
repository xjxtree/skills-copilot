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

struct LongTextDetailSheet: View {
    let title: String
    let text: String
    let renderMode: LongTextRenderMode
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .firstTextBaseline, spacing: 12) {
                Text(title)
                    .font(.headline)
                Spacer()
                Button {
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(text, forType: .string)
                } label: {
                    Label(UIStrings.llmPromptCopyFullText, systemImage: "doc.on.doc")
                }
                Button(UIStrings.llmPromptCloseDetails) {
                    dismiss()
                }
                .keyboardShortcut(.cancelAction)
            }

            ScrollView {
                RenderedLongText(
                    text: text,
                    renderMode: renderMode,
                    isEmpty: false,
                    lineLimit: nil
                )
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(12)
            }
            .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
        }
        .padding()
        .frame(minWidth: 680, minHeight: 460)
    }
}

struct RenderedLongText: View {
    let text: String
    let renderMode: LongTextRenderMode
    let isEmpty: Bool
    let lineLimit: Int?

    var body: some View {
        Group {
            if renderMode == .markdown {
                RenderedMarkdownDocument(
                    text: text,
                    isEmpty: isEmpty,
                    maxBlocks: lineLimit
                )
            } else {
                Text(text)
                    .font(.system(.callout, design: .monospaced))
                    .lineLimit(lineLimit)
            }
        }
        .foregroundStyle(isEmpty ? .secondary : .primary)
        .textSelection(.enabled)
    }
}

struct RenderedMarkdownDocument: View {
    let text: String
    let isEmpty: Bool
    let maxBlocks: Int?

    private var document: MarkdownRenderDocument {
        MarkdownRenderDocument(text: text, maxBlocks: maxBlocks)
    }

    private var compactTableRowLimit: Int? {
        maxBlocks == nil ? nil : 4
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            ForEach(Array(document.blocks.enumerated()), id: \.offset) { _, block in
                blockView(block)
            }
            if document.isTruncated {
                Text("...")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
        .foregroundStyle(isEmpty ? .secondary : .primary)
    }

    @ViewBuilder
    private func blockView(_ block: MarkdownRenderBlock) -> some View {
        switch block {
        case let .heading(level, value):
            MarkdownInlineText(value, font: level <= 2 ? .headline : .subheadline.bold())
        case let .paragraph(value):
            MarkdownInlineText(value, font: .callout)
        case let .bullet(value):
            HStack(alignment: .firstTextBaseline, spacing: 7) {
                Text("*")
                    .font(.callout.bold())
                MarkdownInlineText(value, font: .callout)
            }
        case let .numbered(marker, value):
            HStack(alignment: .firstTextBaseline, spacing: 7) {
                Text(marker)
                    .font(.callout.monospacedDigit())
                    .foregroundStyle(.secondary)
                MarkdownInlineText(value, font: .callout)
            }
        case let .quote(value):
            HStack(alignment: .top, spacing: 8) {
                RoundedRectangle(cornerRadius: 1)
                    .fill(.secondary.opacity(0.5))
                    .frame(width: 3)
                MarkdownInlineText(value, font: .callout)
                    .foregroundStyle(.secondary)
            }
            .padding(.vertical, 2)
        case let .table(rows):
            if maxBlocks == nil {
                MarkdownTableView(rows: rows, maxRows: compactTableRowLimit)
            } else {
                MarkdownTableSummaryView(rows: rows)
            }
        case .rule:
            Divider()
        case let .code(value):
            MarkdownCodeBlockView(
                value: value,
                wrapsLines: maxBlocks != nil,
                lineLimit: maxBlocks == nil ? nil : 8
            )
        }
    }
}

struct MarkdownCodeBlockView: View {
    let value: String
    var wrapsLines = false
    var lineLimit: Int? = nil

    var body: some View {
        if wrapsLines {
            Text(value)
                .font(.system(.callout, design: .monospaced))
                .lineLimit(lineLimit)
                .fixedSize(horizontal: false, vertical: true)
                .padding(8)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 4))
        } else {
            ScrollView(.horizontal) {
                Text(value)
                    .font(.system(.callout, design: .monospaced))
                    .fixedSize(horizontal: true, vertical: false)
                    .padding(8)
            }
            .scrollIndicators(.automatic)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 4))
        }
    }
}

struct MarkdownInlineText: View {
    let value: String
    let font: Font

    init(_ value: String, font: Font) {
        self.value = value
        self.font = font
    }

    var body: some View {
        if let attributed = try? AttributedString(markdown: value) {
            Text(attributed)
                .font(font)
                .fixedSize(horizontal: false, vertical: true)
        } else {
            Text(value)
                .font(font)
                .fixedSize(horizontal: false, vertical: true)
        }
    }
}

struct MarkdownTableView: View {
    let model: MarkdownTableDisplayModel

    init(rows: [[String]], maxRows: Int? = nil) {
        self.model = MarkdownTableDisplayModel(rows: rows, maxVisibleRows: maxRows)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            if model.usesCardLayout {
                MarkdownTableCardList(model: model)
                    .padding(8)
            } else {
                ScrollView(.horizontal) {
                    ExpandableSummaryList(
                        model.identifiedRows,
                        visibleLimit: model.visibleRowLimit,
                        spacing: 6,
                        accessibilityIdentifier: "markdown-table.show-all"
                    ) { displayRow in
                        Grid(alignment: .leading, horizontalSpacing: 10, verticalSpacing: 6) {
                            GridRow {
                                ForEach(Array(model.normalizedRow(displayRow.values).enumerated()), id: \.offset) { columnIndex, value in
                                    MarkdownInlineText(
                                        value.isEmpty ? " " : value,
                                        font: displayRow.id == 0 ? .caption.bold() : .caption
                                    )
                                    .frame(width: model.columnWidth(at: columnIndex), alignment: .leading)
                                    .padding(.vertical, 3)
                                }
                            }
                            if displayRow.id == 0 && model.identifiedRows.count > 1 {
                                Divider()
                                    .gridCellColumns(model.columnCount)
                            }
                        }
                    }
                    .fixedSize(horizontal: true, vertical: false)
                    .padding(8)
                }
                .scrollIndicators(.automatic)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 4))
    }
}

struct MarkdownTableSummaryView: View {
    let model: MarkdownTableDisplayModel

    init(rows: [[String]]) {
        self.model = MarkdownTableDisplayModel(rows: rows, maxVisibleRows: nil)
    }

    var body: some View {
        Label(
            UIStrings.markdownTablePreviewSummary,
            systemImage: "tablecells"
        )
        .font(.caption)
        .foregroundStyle(.secondary)
        .padding(8)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 4))
    }
}

struct MarkdownTableCardList: View {
    let model: MarkdownTableDisplayModel

    var body: some View {
        ExpandableSummaryList(
            model.identifiedCardRows,
            visibleLimit: model.visibleCardRowLimit,
            spacing: 8,
            accessibilityIdentifier: "markdown-table.show-all"
        ) { displayRow in
            MarkdownTableCard(row: model.normalizedRow(displayRow.values), headers: model.headerRow)
        }
    }
}

struct MarkdownTableCard: View {
    let row: [String]
    let headers: [String]

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            if !titleText.isEmpty {
                MarkdownInlineText(titleText, font: .caption.bold())
            }

            ForEach(fieldRows, id: \.index) { field in
                VStack(alignment: .leading, spacing: 2) {
                    Text(field.label)
                        .font(.caption2.bold())
                        .foregroundStyle(.secondary)
                    MarkdownInlineText(field.value, font: .caption)
                }
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.agentCopilotPanelBackground, in: RoundedRectangle(cornerRadius: 6))
    }

    private var titleText: String {
        row.first?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
    }

    private var fieldRows: [MarkdownTableCardField] {
        row.enumerated().compactMap { index, value in
            guard index > 0 else { return nil }
            let cleanValue = value.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !cleanValue.isEmpty else { return nil }
            return MarkdownTableCardField(
                index: index,
                label: headerLabel(at: index),
                value: cleanValue
            )
        }
    }

    private func headerLabel(at index: Int) -> String {
        guard index < headers.count else {
            return "#\(index + 1)"
        }
        let cleanHeader = headers[index].trimmingCharacters(in: .whitespacesAndNewlines)
        return cleanHeader.isEmpty ? "#\(index + 1)" : cleanHeader
    }
}

struct MarkdownTableCardField {
    let index: Int
    let label: String
    let value: String
}

extension JSONValue {
    func boolValue(forAnyKey keys: [String]) -> Bool? {
        guard case .object(let object) = self else { return nil }
        for key in keys {
            if let payloadValue = object[key], case .bool(let value) = payloadValue {
                return value
            }
        }
        return nil
    }

    var compactDisplayString: String {
        switch self {
        case .string(let value):
            return value
        case .number(let value):
            return String(value)
        case .bool(let value):
            return value ? "true" : "false"
        case .object(let object):
            return object.keys.sorted().map { key in
                "\(key)=\(object[key]?.compactDisplayString ?? "")"
            }.joined(separator: ", ")
        case .array(let values):
            return values.map(\.compactDisplayString).joined(separator: ", ")
        case .null:
            return ""
        }
    }
}
