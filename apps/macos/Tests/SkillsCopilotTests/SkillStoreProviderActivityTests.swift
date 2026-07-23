import Combine
import Foundation
@testable import SkillsCopilot

@MainActor
extension SkillStoreTests {
    func providerObservabilitySurfacesMethodUnavailable() async throws {
        let fake = try FakeServiceScript()
        defer { fake.cleanup() }
        fake.activate(scenario: "normal")

        let store = SkillStore(service: fake.serviceClient())
        store.selectedSkillID = "beta"
        await store.reload()
        await store.loadProviderObservability()

        try expectEqual(store.providerObservabilityResult?.isUnavailable, true, "Provider observability should expose the service failure as unavailable.")
        try expectContains(store.providerObservabilityResult?.fallbackReason, "unknown_method", "Unknown methods should remain visible instead of being rewritten as a compatibility fallback.")
        try expectFalse(store.isLoadingProviderObservability, "Unavailable provider observability should reset loading state.")
        try expectContains(fake.calls(), "llm.providerObservability", "The unavailable result should still prove the intended method was attempted.")
    }

    func providerActivityAccumulatesAllPagesWithoutChangingSummary() async throws {
        let runner = ProviderActivityPageRunner(totalCount: 130)
        let store = SkillStore(service: runner.serviceClient())

        await store.loadProviderObservability()
        try expectEqual(store.providerActivityRows.count, 50, "Initial provider activity page")
        try expectEqual(store.providerActivityCompleteness.totalCount, 130, "Provider activity total")
        let summary = store.providerObservabilityResult?.summary

        await store.loadMoreProviderActivity(loadAll: false)
        try expectEqual(store.providerActivityRows.count, 100, "Provider activity Load More")
        await store.loadMoreProviderActivity(loadAll: true)
        try expectEqual(store.providerActivityRows.count, 130, "Provider activity Load All")
        try expectEqual(store.providerActivityCompleteness.completeness, .complete, "Provider activity should become complete")
        try expectEqual(store.providerObservabilityResult?.summary, summary, "Paging must not change aggregate summary")
        try expectEqual(runner.activityRequestCount(), 3, "Provider activity requests should be serialized into three 50-row pages.")
        try await providerActivityMethodUnavailableUsesRetryableFailure()
        try await providerActivityInitialFailureRetriesFromNilCursor()
    }

    private func providerActivityMethodUnavailableUsesRetryableFailure() async throws {
        let runner = ProviderActivityPageRunner(
            totalCount: 50,
            activityFailureCode: "unknown_method",
            activityFailureCount: .max
        )
        let store = SkillStore(service: runner.serviceClient())
        await store.loadProviderObservability()

        try expectEqual(store.providerObservabilityResult?.isUnavailable, false, "Activity method fallback must not discard the available aggregate summary.")
        try expectEqual(store.providerActivityRows, [], "Unavailable activity method must not invent rows.")
        try expectEqual(store.providerActivityCompleteness.isComplete, false, "Unavailable activity method must remain incomplete.")
        try expectEqual(store.providerActivityCompleteness.incompleteReason, .pageFailed, "Unknown activity methods should use the same visible page_failed state as other service failures.")
        try expectEqual(store.providerActivityCompleteness.canLoadAll, true, "An initial activity failure should offer a safe retry from the nil cursor.")
    }

    private func providerActivityInitialFailureRetriesFromNilCursor() async throws {
        let runner = ProviderActivityPageRunner(
            totalCount: 50,
            activityFailureCode: "temporary_failure",
            activityFailureCount: 1
        )
        let store = SkillStore(service: runner.serviceClient())
        await store.loadProviderObservability()

        try expectEqual(store.providerActivityRows, [], "Failed initial activity page must retain an empty accepted set.")
        try expectEqual(store.providerActivityCompleteness.incompleteReason, .pageFailed, "Initial activity failure must be typed as page_failed.")
        try expectEqual(store.providerActivityCompleteness.canLoadAll, true, "Initial activity failure must offer a safe retry from the nil cursor.")

        await store.loadMoreProviderActivity(loadAll: true)
        try expectEqual(store.providerActivityRows.count, 50, "Retry must obtain the full first page from the nil cursor.")
        try expectEqual(store.providerActivityCompleteness.completeness, .complete, "Successful retry must reach complete EOF.")
        try expectEqual(runner.activityRequestCount(), 2, "Initial retry must issue exactly one replacement first-page request.")
    }

    func providerActivityCancellationAndStaleGenerationPreserveAcceptedRows() async throws {
        let cancelRunner = ProviderActivityPageRunner(totalCount: 130, delayedOffsets: [100])
        let cancelStore = SkillStore(service: cancelRunner.serviceClient())
        await cancelStore.loadProviderObservability()
        await cancelStore.loadMoreProviderActivity(loadAll: false)
        let cancelTask = Task { @MainActor in
            await cancelStore.loadMoreProviderActivity(loadAll: true)
        }
        try await waitUntil("Provider activity Load All should reach the delayed third page.") {
            cancelRunner.activityRequestCount() == 3
        }
        cancelStore.cancelProviderActivityLoadAll()
        cancelRunner.release(offset: 100)
        await cancelTask.value
        try expectEqual(cancelStore.providerActivityRows.count, 100, "Cancelling provider activity Load All must retain accepted pages.")
        try expectEqual(cancelStore.providerActivityCompleteness.loadingPhase, .idle, "Cancellation should restore idle paging controls.")

        let staleRunner = ProviderActivityPageRunner(totalCount: 130, delayedOffsets: [50])
        let staleStore = SkillStore(service: staleRunner.serviceClient())
        await staleStore.loadProviderObservability()
        let staleTask = Task { @MainActor in
            await staleStore.loadMoreProviderActivity(loadAll: false)
        }
        try await waitUntil("Provider activity continuation should be delayed before refresh.") {
            staleRunner.activityRequestCount() == 2
        }
        await staleStore.loadProviderObservability()
        try expectEqual(staleStore.providerActivityRows.count, 50, "Manual refresh should publish a replacement first page.")
        try expectFalse(
            !staleStore.providerActivityRows.allSatisfy { $0.id.hasPrefix("g2-") },
            "Replacement provider activity should use the latest generation."
        )
        staleRunner.release(offset: 50)
        await staleTask.value
        try expectEqual(staleStore.providerActivityRows.count, 50, "A stale delayed page must not append after replacement.")
        try expectFalse(
            !staleStore.providerActivityRows.allSatisfy { $0.id.hasPrefix("g2-") },
            "Stale generation rows must never publish."
        )
    }

    func providerActivityNotifiesAfterEachAcceptedPage() async throws {
        let runner = ProviderActivityPageRunner(totalCount: 130, delayedOffsets: [100])
        let store = SkillStore(service: runner.serviceClient())
        await store.loadProviderObservability()

        var notifiedRowCounts: [Int] = []
        let observation = store.objectWillChange.sink {
            notifiedRowCounts.append(store.providerActivityRows.count)
        }
        defer { observation.cancel() }

        let loadAll = Task { @MainActor in
            await store.loadMoreProviderActivity(loadAll: true)
        }
        try await waitUntil("Provider activity should accept page two before page three is released.") {
            runner.activityRequestCount() == 3
                && store.providerActivityRows.count == 100
        }
        try expectFalse(
            !notifiedRowCounts.contains(100),
            "The Store must notify observers as soon as each intermediate provider page is accepted."
        )

        runner.release(offset: 100)
        await loadAll.value
        try expectEqual(store.providerActivityRows.count, 130, "The released final page should still reach complete EOF.")
    }
}

private final class ProviderActivityPageRunner: ServiceProcessRunning, @unchecked Sendable {
    private let lock = NSLock()
    private let totalCount: Int
    private let delayedOffsets: Set<Int>
    private let activityFailureCode: String?
    private var activityFailuresRemaining: Int
    private var generation = 0
    private var activityRequests = 0
    private var releaseContinuations: [Int: [CheckedContinuation<Void, Never>]] = [:]
    private var releasedOffsets = Set<Int>()

    init(
        totalCount: Int,
        delayedOffsets: Set<Int> = [],
        activityFailureCode: String? = nil,
        activityFailureCount: Int = 0
    ) {
        self.totalCount = totalCount
        self.delayedOffsets = delayedOffsets
        self.activityFailureCode = activityFailureCode
        activityFailuresRemaining = max(0, activityFailureCount)
    }

    func serviceClient() -> ServiceClient {
        ServiceClient(
            processRunner: self,
            serviceURL: URL(fileURLWithPath: "/tmp/provider-activity-page-service")
        )
    }

    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) async throws -> Data {
        guard let request = try JSONSerialization.jsonObject(with: input) as? [String: Any],
              let method = request["method"] as? String else {
            return Self.error(code: "invalid_request", message: "invalid test request")
        }
        switch method {
        case "llm.providerObservability":
            return Self.aggregateResponse(requestID: request["id"])
        case "llm.listProviderActivity":
            let params = request["params"] as? [String: Any] ?? [:]
            let cursor = params["cursor"] as? String
            let binding = recordActivityRequest(cursor: cursor)
            if let failureCode = consumeActivityFailure() {
                return Self.error(code: failureCode, message: "fixture activity failure")
            }
            if delayedOffsets.contains(binding.offset) {
                await waitForRelease(offset: binding.offset)
            }
            return Self.activityResponse(
                requestID: request["id"],
                generation: binding.generation,
                offset: binding.offset,
                totalCount: totalCount
            )
        default:
            return Self.error(code: "unknown_method", message: "unknown method: \(method)")
        }
    }

    func activityRequestCount() -> Int {
        lock.lock()
        let count = activityRequests
        lock.unlock()
        return count
    }

    func release(offset: Int) {
        lock.lock()
        let continuations = releaseContinuations.removeValue(forKey: offset) ?? []
        if continuations.isEmpty {
            releasedOffsets.insert(offset)
        }
        lock.unlock()
        continuations.forEach { $0.resume() }
    }

    private func consumeActivityFailure() -> String? {
        lock.lock()
        defer { lock.unlock() }
        guard activityFailuresRemaining > 0 else { return nil }
        activityFailuresRemaining -= 1
        return activityFailureCode
    }

    private func recordActivityRequest(cursor: String?) -> (generation: Int, offset: Int) {
        lock.lock()
        defer { lock.unlock() }
        activityRequests += 1
        if let cursor {
            let parts = cursor.split(separator: "-")
            let cursorGeneration = Int(String(parts.first?.dropFirst() ?? "")) ?? generation
            let offset = parts.last.flatMap { Int($0) } ?? 0
            return (cursorGeneration, offset)
        }
        generation += 1
        return (generation, 0)
    }

    private func waitForRelease(offset: Int) async {
        await withCheckedContinuation { continuation in
            lock.lock()
            let wasReleased = releasedOffsets.remove(offset) != nil
            if !wasReleased {
                releaseContinuations[offset, default: []].append(continuation)
            }
            lock.unlock()
            if wasReleased {
                continuation.resume()
            }
        }
    }

    private static func aggregateResponse(requestID: Any?) -> Data {
        response(
            requestID: requestID,
            result: [
                "generated_by": "local-v2.64",
                "filters": ["aggregation_uses_full_range": true],
                "summary": [
                    "call_count": 130,
                    "success_count": 130,
                    "estimated_total_tokens": 13_000,
                    "summary": "Stable full-range aggregate summary.",
                ],
                "safety_flags": readOnlySafetyFlags,
            ]
        )
    }

    private static func activityResponse(
        requestID: Any?,
        generation: Int,
        offset: Int,
        totalCount: Int
    ) -> Data {
        let end = min(offset + 50, totalCount)
        let rows: [[String: Any]] = (offset..<end).map { index in
            [
                "id": "g\(generation)-activity-\(index)",
                "kind": index.isMultiple(of: 2) ? "provider_call" : "prompt_run",
                "timestamp": totalCount - index,
                "title": "Activity \(index)",
                "subtitle": "redacted metadata",
                "status": "succeeded",
                "evidence_refs": ["activity:g\(generation):\(index)"],
            ]
        }
        let hasMore = end < totalCount
        return response(
            requestID: requestID,
            result: [
                "generated_by": "local-v2.64",
                "rows": rows,
                "source_revision": "sha256:activity-g\(generation)",
                "returned_count": rows.count,
                "total_count": totalCount,
                "has_more": hasMore,
                "next_cursor": hasMore ? "g\(generation)-cursor-\(end)" : NSNull(),
                "source_completeness": "enumerable",
                "incomplete_reason": NSNull(),
                "safety_flags": readOnlySafetyFlags,
            ]
        )
    }

    private static let readOnlySafetyFlags: [String: Any] = [
        "provider_request_sent": false,
        "credential_accessed": false,
        "write_back_allowed": false,
        "write_actions_available": false,
        "script_execution_allowed": false,
        "execution_actions_available": false,
        "config_mutation_allowed": false,
        "snapshot_created": false,
        "triage_mutation_allowed": false,
        "raw_secret_returned": false,
        "raw_prompt_persisted": false,
        "raw_response_persisted": false,
        "raw_trace_persisted": false,
        "cloud_sync_performed": false,
        "telemetry_emitted": false,
    ]

    private static func response(requestID: Any?, result: [String: Any]) -> Data {
        try! JSONSerialization.data(withJSONObject: [
            "id": requestID ?? "test",
            "ok": true,
            "result": result,
        ])
    }

    private static func error(code: String, message: String) -> Data {
        try! JSONSerialization.data(withJSONObject: [
            "id": "test",
            "ok": false,
            "error": ["code": code, "message": message],
        ])
    }
}
