@testable import SkillsCopilot

struct ListCompletenessModelTests {
    struct Row: Identifiable, Equatable { let id: String; let value: String }

    func run() throws {
        try appendsPagesWithoutDuplicateIDs()
        try rejectsChangedSourceRevision()
        try knownTotalCompletesOnlyAtEOF()
        try unknownTotalNeverInventsKnownTotal()
        try cancellationKeepsAcceptedRowsPartial()
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
}
