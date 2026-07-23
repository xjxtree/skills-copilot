import Foundation
@testable import SkillsCopilot

@MainActor
struct ConfirmedMutationLaneTests {
    func run() async throws {
        try await operationsAreFIFOAndNeverOverlap()
        try await ordinaryFailureIsNotRetried()
        try await shutdownSafelyCancelsQueuedOperations()
    }

    private func operationsAreFIFOAndNeverOverlap() async throws {
        let lane = ConfirmedMutationLane()
        let probe = ConfirmedMutationProbe()
        let gate = ConfirmedMutationGate()

        let first = Task { @MainActor in
            await lane.perform {
                await probe.begin("first")
                await gate.wait(for: "first")
                await probe.end("first")
                return "first"
            }
        }
        try await waitFor("first confirmed mutation starts") {
            await probe.events == ["start:first"]
        }

        let second = Task { @MainActor in
            await lane.perform {
                await probe.begin("second")
                await gate.wait(for: "second")
                await probe.end("second")
                return "second"
            }
        }
        let third = Task { @MainActor in
            await lane.perform {
                await probe.begin("third")
                await gate.wait(for: "third")
                await probe.end("third")
                return "third"
            }
        }

        await Task.yield()
        try expectEqual(
            await probe.events,
            ["start:first"],
            "Queued confirmed mutations must not overlap the active mutation."
        )

        await gate.release("first")
        try await waitFor("second confirmed mutation starts in FIFO order") {
            await probe.events == ["start:first", "end:first", "start:second"]
        }
        await gate.release("second")
        try await waitFor("third confirmed mutation starts in FIFO order") {
            await probe.events == [
                "start:first", "end:first",
                "start:second", "end:second",
                "start:third",
            ]
        }
        await gate.release("third")

        try expectEqual(await first.value, Optional("first"), "The first mutation should complete.")
        try expectEqual(await second.value, Optional("second"), "The second mutation should complete.")
        try expectEqual(await third.value, Optional("third"), "The third mutation should complete.")
        try expectEqual(
            await probe.maximumConcurrentCount,
            1,
            "The confirmed mutation lane must serialize service writes."
        )
    }

    private func ordinaryFailureIsNotRetried() async throws {
        let lane = ConfirmedMutationLane()
        var callCount = 0

        let result = await lane.perform {
            callCount += 1
            return false
        }
        await Task.yield()

        try expectEqual(result, Optional(false), "The caller must receive the failed mutation result.")
        try expectEqual(callCount, 1, "A failed confirmed mutation must not be retried implicitly.")
    }

    private func shutdownSafelyCancelsQueuedOperations() async throws {
        let lane = ConfirmedMutationLane()
        let gate = ConfirmedMutationGate()
        var queuedOperationRan = false

        let active = Task { @MainActor in
            await lane.perform {
                await gate.wait(for: "active")
                return true
            }
        }
        try await waitFor("active mutation occupies the lane") {
            await gate.hasWaiter(for: "active")
        }

        let queued = Task { @MainActor in
            await lane.perform {
                queuedOperationRan = true
                return true
            }
        }
        await Task.yield()

        try expectEqual(lane.shutdown(), 1, "Shutdown should release the queued waiter.")
        try expectNil(await queued.value, "A queued mutation should cancel safely after shutdown.")
        try expectFalse(queuedOperationRan, "Shutdown must not run a queued write closure.")

        await gate.release("active")
        try expectEqual(await active.value, Optional(true), "The already-running mutation may settle normally.")
        try expectNil(
            await lane.perform { true },
            "New mutations must be rejected safely after shutdown."
        )
    }

    private func waitFor(
        _ label: String,
        condition: () async -> Bool
    ) async throws {
        for _ in 0..<500 {
            if await condition() {
                return
            }
            await Task.yield()
        }
        throw NativeModelTestFailure(description: "Timed out waiting for \(label).")
    }
}

private actor ConfirmedMutationProbe {
    private(set) var events: [String] = []
    private var concurrentCount = 0
    private(set) var maximumConcurrentCount = 0

    func begin(_ label: String) {
        concurrentCount += 1
        maximumConcurrentCount = max(maximumConcurrentCount, concurrentCount)
        events.append("start:\(label)")
    }

    func end(_ label: String) {
        events.append("end:\(label)")
        concurrentCount -= 1
    }
}

private actor ConfirmedMutationGate {
    private var released: Set<String> = []
    private var waiters: [String: CheckedContinuation<Void, Never>] = [:]

    func wait(for label: String) async {
        if released.remove(label) != nil {
            return
        }
        await withCheckedContinuation { continuation in
            waiters[label] = continuation
        }
    }

    func release(_ label: String) {
        if let waiter = waiters.removeValue(forKey: label) {
            waiter.resume()
        } else {
            released.insert(label)
        }
    }

    func hasWaiter(for label: String) -> Bool {
        waiters[label] != nil
    }
}
