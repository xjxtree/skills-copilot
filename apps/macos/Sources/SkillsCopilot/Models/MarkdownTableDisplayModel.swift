import CoreGraphics
import Foundation

struct MarkdownTableDisplayRow: Identifiable, Equatable {
    let id: Int
    let values: [String]
}

struct MarkdownTableDisplayModel {
    static let minimumColumnWidth: CGFloat = 120
    static let maximumColumnWidth: CGFloat = 280
    private static let approximateCharacterWidth: CGFloat = 7
    private static let horizontalPadding: CGFloat = 28

    let rows: [[String]]
    let maxVisibleRows: Int?

    var nonEmptyRows: [[String]] {
        rows.filter { row in
            !row.allSatisfy { $0.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }
        }
    }

    var displayRows: [[String]] {
        guard let maxVisibleRows, nonEmptyRows.count > maxVisibleRows else {
            return nonEmptyRows
        }
        return Array(nonEmptyRows.prefix(maxVisibleRows))
    }

    var identifiedRows: [MarkdownTableDisplayRow] {
        nonEmptyRows.enumerated().map { index, values in
            MarkdownTableDisplayRow(id: index, values: values)
        }
    }

    var visibleRowLimit: Int {
        min(maxVisibleRows ?? nonEmptyRows.count, nonEmptyRows.count)
    }

    var usesCardLayout: Bool {
        columnCount > 3 || (columnCount > 2 && containsLongBodyCell)
    }

    var headerRow: [String] {
        normalizedRow(nonEmptyRows.first ?? [])
    }

    var displayCardRows: [[String]] {
        let bodyRows = cardBodyRows
        guard let maxVisibleRows else {
            return bodyRows
        }
        let visibleBodyCount = max(1, maxVisibleRows - 1)
        return Array(bodyRows.prefix(visibleBodyCount))
    }

    var identifiedCardRows: [MarkdownTableDisplayRow] {
        cardBodyRows.enumerated().map { index, values in
            MarkdownTableDisplayRow(id: index, values: values)
        }
    }

    var visibleCardRowLimit: Int {
        guard let maxVisibleRows else { return cardBodyRows.count }
        return min(max(1, maxVisibleRows - 1), cardBodyRows.count)
    }

    var hiddenRowCount: Int {
        if usesCardLayout {
            return max(0, cardBodyRows.count - displayCardRows.count)
        }
        return max(0, nonEmptyRows.count - displayRows.count)
    }

    var bodyRowCount: Int {
        cardBodyRows.count
    }

    var columnCount: Int {
        max(nonEmptyRows.map(\.count).max() ?? 0, 1)
    }

    func normalizedRow(_ row: [String]) -> [String] {
        guard row.count < columnCount else { return row }
        return row + Array(repeating: "", count: columnCount - row.count)
    }

    func columnWidth(at columnIndex: Int) -> CGFloat {
        let maxWeight = nonEmptyRows
            .compactMap { row -> String? in
                guard columnIndex < row.count else { return nil }
                return row[columnIndex]
            }
            .map(Self.displayWeight)
            .max() ?? 0
        let clampedWeight = min(max(maxWeight, 12), 36)
        let estimatedWidth = CGFloat(clampedWeight) * Self.approximateCharacterWidth + Self.horizontalPadding
        return min(Self.maximumColumnWidth, max(Self.minimumColumnWidth, estimatedWidth))
    }

    func totalWidth(horizontalSpacing: CGFloat = 10) -> CGFloat {
        let columnsWidth = (0..<columnCount).reduce(CGFloat.zero) { partial, index in
            partial + columnWidth(at: index)
        }
        let spacingWidth = max(CGFloat(columnCount - 1), 0) * horizontalSpacing
        return columnsWidth + spacingWidth
    }

    private var cardBodyRows: [[String]] {
        let rows = nonEmptyRows
        guard rows.count > 1 else {
            return rows
        }
        return Array(rows.dropFirst())
    }

    private var containsLongBodyCell: Bool {
        cardBodyRows
            .flatMap { $0 }
            .contains { Self.displayWeight($0) > 48 }
    }

    private static func displayWeight(_ text: String) -> Int {
        text.reduce(0) { partial, character in
            let isASCII = character.unicodeScalars.allSatisfy { scalar in
                scalar.value < 128
            }
            return partial + (isASCII ? 1 : 2)
        }
    }
}
