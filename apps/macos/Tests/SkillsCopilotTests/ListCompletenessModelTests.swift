@testable import SkillsCopilot

struct ListCompletenessModelTests {
    struct Row: Identifiable, Equatable { let id: String; let value: String }

    func run() throws {
        try appendsPagesWithoutDuplicateIDs()
        try rejectsChangedSourceRevision()
        try knownTotalCompletesOnlyAtEOF()
        try unknownTotalNeverInventsKnownTotal()
        try cancellationKeepsAcceptedRowsPartial()
        try rejectsEnumerableContinuationWithoutCursor()
        try rejectsCursorOnTerminalPage()
        try rejectsTerminalKnownTotalMismatchWithoutMutation()
        try terminalLimitationsDisableContinuation()
        try pageFailureKeepsRetryContinuation()
    }

    private func appendsPagesWithoutDuplicateIDs() throws {
        var value = ListPageAccumulator<Row>()
        try value.append(ListPage(items: [.init(id: "a", value: "A"), .init(id: "b", value: "B")],
            returnedCount: 2, totalCount: 3, hasMore: true, nextCursor: "next", sourceRevision: "r1",
            sourceCompleteness: .enumerable, incompleteReason: nil))
        try value.append(ListPage(items: [.init(id: "b", value: "duplicate"), .init(id: "c", value: "C")],
            returnedCount: 2, totalCount: 3, hasMore: false, nextCursor: nil, sourceRevision: "r1",
            sourceCompleteness: .enumerable, incompleteReason: nil))
        try expectEqual(value.items.map(\.id), ["a", "b", "c"], "Stable IDs should deduplicate pages")
        try expectEqual(value.state.completeness, .complete, "EOF plus known total should complete")
    }

    private func rejectsChangedSourceRevision() throws {
        var value = ListPageAccumulator<Row>()
        try value.append(.init(items: [.init(id: "a", value: "A")], returnedCount: 1,
            totalCount: 2, hasMore: true, nextCursor: "next", sourceRevision: "r1",
            sourceCompleteness: .enumerable, incompleteReason: nil))
        do {
            try value.append(.init(items: [.init(id: "b", value: "B")], returnedCount: 1,
                totalCount: 2, hasMore: false, nextCursor: nil, sourceRevision: "r2",
                sourceCompleteness: .enumerable, incompleteReason: nil))
            throw NativeModelTestFailure(description: "Changed revision should fail")
        } catch ListPageAccumulatorError.sourceChanged {}
        try expectEqual(value.items.map(\.id), ["a"], "Rejected page must not mutate rows")
    }

    private func knownTotalCompletesOnlyAtEOF() throws {
        var value = ListPageAccumulator<Row>()
        try value.append(.init(items: [.init(id: "a", value: "A")], returnedCount: 1,
            totalCount: 2, hasMore: true, nextCursor: "next", sourceRevision: "r1",
            sourceCompleteness: .enumerable, incompleteReason: nil))
        try expectEqual(value.state.completeness, .partial, "Nonterminal page must stay partial")
    }

    private func unknownTotalNeverInventsKnownTotal() throws {
        var value = ListPageAccumulator<Row>()
        try value.append(.init(items: [.init(id: "a", value: "A")], returnedCount: 1,
            totalCount: nil, hasMore: false, nextCursor: nil, sourceRevision: "r1",
            sourceCompleteness: .enumerable, incompleteReason: nil))
        try expectNil(value.state.totalCount, "Unknown total must stay nil")
        try expectEqual(value.state.completeness, .complete, "Defensible EOF can complete unknown total")
    }

    private func cancellationKeepsAcceptedRowsPartial() throws {
        var value = ListPageAccumulator<Row>()
        value.begin(.all)
        try value.append(.init(items: [.init(id: "a", value: "A")], returnedCount: 1,
            totalCount: 2, hasMore: true, nextCursor: "next", sourceRevision: "r1",
            sourceCompleteness: .enumerable, incompleteReason: nil))
        value.cancel()
        try expectEqual(value.items.map(\.id), ["a"], "Cancel must retain accepted rows")
        try expectEqual(value.state.completeness, .partial, "Cancelled continuation is partial")
    }

    private func rejectsEnumerableContinuationWithoutCursor() throws {
        var value = ListPageAccumulator<Row>()
        do {
            try value.append(.init(items: [.init(id: "a", value: "A")], returnedCount: 1,
                totalCount: 2, hasMore: true, nextCursor: nil, sourceRevision: "r1",
                sourceCompleteness: .enumerable, incompleteReason: nil))
            throw NativeModelTestFailure(description: "Enumerable continuation without a cursor should fail")
        } catch ListPageAccumulatorError.invalidPage {}
        try expectEqual(value.items, [], "Rejected cursorless continuation must not mutate rows")
    }

    private func rejectsCursorOnTerminalPage() throws {
        var value = ListPageAccumulator<Row>()
        do {
            try value.append(.init(items: [.init(id: "a", value: "A")], returnedCount: 1,
                totalCount: 1, hasMore: false, nextCursor: "stale", sourceRevision: "r1",
                sourceCompleteness: .enumerable, incompleteReason: nil))
            throw NativeModelTestFailure(description: "Terminal page with a cursor should fail")
        } catch ListPageAccumulatorError.invalidPage {}
        try expectEqual(value.items, [], "Rejected terminal cursor must not mutate rows")
    }

    private func rejectsTerminalKnownTotalMismatchWithoutMutation() throws {
        var value = try enumerableContinuation()
        do {
            try value.append(.init(
                items: [.init(id: "a", value: "duplicate"), .init(id: "b", value: "B")],
                returnedCount: 2,
                totalCount: 3,
                hasMore: false,
                nextCursor: nil,
                sourceRevision: "r1",
                sourceCompleteness: .enumerable,
                incompleteReason: nil
            ))
            throw NativeModelTestFailure(description: "Terminal page must account for the known total")
        } catch ListPageAccumulatorError.invalidPage {}
        try expectEqual(value.items.map(\.id), ["a"], "Rejected terminal page must not mutate rows")
        try expectEqual(value.state.canLoadMore, true, "Rejected terminal page must preserve the accepted cursor")
    }

    private func terminalLimitationsDisableContinuation() throws {
        let reasons: [ListIncompleteReason] = [
            .safetyBudget,
            .sourceChanged,
            .sourceLimited,
            .unreadableSource,
            .unsupportedProtocol,
        ]
        for reason in reasons {
            var value = try enumerableContinuation()
            value.fail(reason: reason)
            try expectEqual(value.state.canLoadMore, false, "\(reason) must disable Load More")
            try expectEqual(value.state.canLoadAll, false, "\(reason) must disable Load All")
            try expectEqual(value.state.hasMore, false, "\(reason) must be terminal")
        }
    }

    private func pageFailureKeepsRetryContinuation() throws {
        var value = try enumerableContinuation()
        value.fail(reason: .pageFailed)
        try expectEqual(value.state.canLoadMore, true, "Page failure should keep retry continuation")
        try expectEqual(value.state.canLoadAll, true, "Page failure should keep retry-all continuation")
    }

    private func enumerableContinuation() throws -> ListPageAccumulator<Row> {
        var value = ListPageAccumulator<Row>()
        try value.append(.init(items: [.init(id: "a", value: "A")], returnedCount: 1,
            totalCount: 3, hasMore: true, nextCursor: "next", sourceRevision: "r1",
            sourceCompleteness: .enumerable, incompleteReason: nil))
        return value
    }
}
