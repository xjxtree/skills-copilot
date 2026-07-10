import Foundation
@testable import SkillsCopilot

@MainActor
struct RevisionAutosaveCoordinatorTests {
    func run() async throws {
        try await rapidEditsSaveOnlyLatestRevision()
        try await editDuringSaveRunsAfterCurrentSave()
        try await failedSaveDoesNotDropPendingRevision()
        try await invalidRevisionWaitsUntilInputIsValid()
        try await cancellingDebounceDoesNotCancelActiveSave()
        try await completionIdentifiesExactlyCommittedRevision()
        try await concurrentFlushesShareOnePendingSave()
        try await cancellingFlushCallerDoesNotCancelActiveSave()
        try await phaseCallbackPublishesCompleteLifecycle()
        try await failedSaveWithoutPendingIsNotRetriedByFlush()
        try await invalidEditDuringActiveSaveLeavesNoPendingRevision()
        try await editsBAndCDuringSaveRunOnlyLatestC()
        try persistedValueDuringActiveSaveMustSubmit()
    }

    private func rapidEditsSaveOnlyLatestRevision() async throws {
        let clock = ControlledAutosaveClock()
        let recorder = ControlledAutosaveSaveRecorder<String>(suspends: false)
        var completions: [RevisionAutosaveCompletion<String>] = []
        let coordinator = makeCoordinator(clock: clock, recorder: recorder) {
            completions.append($0)
        }

        try expectEqual(coordinator.submit("A", validationError: nil), 1, "First autosave revision should start at one.")
        try expectEqual(coordinator.submit("B", validationError: nil), 2, "Second autosave revision should increment.")
        try expectEqual(coordinator.submit("C", validationError: nil), 3, "Third autosave revision should increment.")

        try await waitForClockCount(1, clock: clock)
        await clock.releaseNext()
        try await waitUntil("latest rapid edit saved") {
            recorder.calls.map(\.value) == ["C"] && coordinator.phase == .idle
        }

        try expectEqual(recorder.calls.map(\.value), ["C"], "Rapid edits should save only the latest value.")
        try expectEqual(completions.map(\.revision), [3], "Completion should identify only the committed revision.")
    }

    private func editDuringSaveRunsAfterCurrentSave() async throws {
        let clock = ControlledAutosaveClock()
        let recorder = ControlledAutosaveSaveRecorder<String>(suspends: true)
        let coordinator = makeCoordinator(clock: clock, recorder: recorder)

        _ = coordinator.submit("A", validationError: nil)
        try await releaseNextDebounce(clock)
        try await waitUntil("save A starts") { recorder.calls.map(\.value) == ["A"] }

        let revisionB = coordinator.submit("B", validationError: nil)
        try expectEqual(coordinator.phase, .pendingAfterSave(revision: revisionB), "Edit B should remain pending while A saves.")
        recorder.resumeNext(success: true)

        try await releaseNextDebounce(clock)
        try await waitUntil("save B starts") { recorder.calls.map(\.value) == ["A", "B"] }
        recorder.resumeNext(success: true)
        try await waitUntil("save B finishes") { coordinator.phase == .idle }

        try expectEqual(recorder.calls.map(\.value), ["A", "B"], "An edit arriving during a save should run immediately after it.")
    }

    private func failedSaveDoesNotDropPendingRevision() async throws {
        let clock = ControlledAutosaveClock()
        let recorder = ControlledAutosaveSaveRecorder<String>(suspends: true)
        var completions: [RevisionAutosaveCompletion<String>] = []
        let coordinator = makeCoordinator(clock: clock, recorder: recorder) {
            completions.append($0)
        }

        _ = coordinator.submit("A", validationError: nil)
        try await releaseNextDebounce(clock)
        try await waitUntil("failed save A starts") { recorder.calls.map(\.value) == ["A"] }
        _ = coordinator.submit("B", validationError: nil)
        recorder.resumeNext(success: false)

        try await releaseNextDebounce(clock)
        try await waitUntil("save B follows failure") { recorder.calls.map(\.value) == ["A", "B"] }
        recorder.resumeNext(success: true)
        try await waitUntil("save B succeeds") { coordinator.phase == .idle }

        try expectEqual(recorder.calls.map(\.value), ["A", "B"], "A failed save must not discard a newer pending value.")
        try expectEqual(completions.map(\.succeeded), [false, true], "Completion should preserve each save outcome.")
    }

    private func invalidRevisionWaitsUntilInputIsValid() async throws {
        let clock = ControlledAutosaveClock()
        let recorder = ControlledAutosaveSaveRecorder<String>(suspends: false)
        let coordinator = makeCoordinator(clock: clock, recorder: recorder)

        _ = coordinator.submit("valid-but-superseded", validationError: nil)
        let invalidRevision = coordinator.submit("invalid", validationError: "invalid input")
        try expectEqual(invalidRevision, 2, "Invalid input should still advance revision identity.")
        try await waitUntil("invalid edit cancels debounce") { await clock.pendingCount == 0 }
        try expectEqual(coordinator.phase, .idle, "An invalid current value should leave no pending save.")

        _ = coordinator.submit("valid", validationError: nil)
        try await releaseNextDebounce(clock)
        try await waitUntil("valid edit saves") { recorder.calls.map(\.value) == ["valid"] }

        try expectEqual(recorder.calls.map(\.value), ["valid"], "A superseded valid draft must not save after an invalid edit.")
    }

    private func cancellingDebounceDoesNotCancelActiveSave() async throws {
        let clock = ControlledAutosaveClock()
        let recorder = ControlledAutosaveSaveRecorder<String>(suspends: true)
        let coordinator = makeCoordinator(clock: clock, recorder: recorder)

        let revisionA = coordinator.submit("A", validationError: nil)
        try await releaseNextDebounce(clock)
        try await waitUntil("active save starts") { recorder.calls.map(\.value) == ["A"] }
        _ = coordinator.submit("B", validationError: nil)
        coordinator.cancelPendingDebounce()

        try expectEqual(coordinator.phase, .saving(revision: revisionA), "Cancelling pending debounce must leave the active save running.")
        recorder.resumeNext(success: true)
        try await waitUntil("active save completes") { coordinator.phase == .idle }
        try expectEqual(recorder.calls.map(\.value), ["A"], "Cancelling debounce should remove only unsaved B.")
    }

    private func completionIdentifiesExactlyCommittedRevision() async throws {
        let clock = ControlledAutosaveClock()
        let recorder = ControlledAutosaveSaveRecorder<String>(suspends: false)
        var completions: [RevisionAutosaveCompletion<String>] = []
        let coordinator = makeCoordinator(clock: clock, recorder: recorder) {
            completions.append($0)
        }

        let revision = coordinator.submit("committed", validationError: nil)
        try await releaseNextDebounce(clock)
        try await waitUntil("completion is delivered") { completions.count == 1 }

        try expectEqual(completions.first?.revision, revision, "Completion should carry the exact committed revision.")
        try expectEqual(completions.first?.value, "committed", "Completion should carry the exact committed value.")
        try expectEqual(completions.first?.succeeded, true, "Completion should carry the save outcome.")
    }

    private func concurrentFlushesShareOnePendingSave() async throws {
        let clock = ControlledAutosaveClock()
        let recorder = ControlledAutosaveSaveRecorder<String>(suspends: true)
        let coordinator = makeCoordinator(clock: clock, recorder: recorder)

        _ = coordinator.submit("pending", validationError: nil)
        let firstFlush = Task { @MainActor in await coordinator.flush() }
        let secondFlush = Task { @MainActor in await coordinator.flush() }

        try await waitUntil("concurrent flush starts one pending save") {
            recorder.calls.map(\.value) == ["pending"]
        }
        try expectEqual(
            recorder.calls.map(\.value),
            ["pending"],
            "Concurrent termination flush callers must share one worker instead of duplicating a save."
        )

        recorder.resumeNext(success: true)
        await firstFlush.value
        await secondFlush.value
        try expectEqual(coordinator.phase, .idle, "Both concurrent flush callers should observe the settled worker.")
    }

    private func cancellingFlushCallerDoesNotCancelActiveSave() async throws {
        let clock = ControlledAutosaveClock()
        let recorder = ControlledAutosaveSaveRecorder<String>(suspends: true)
        let coordinator = makeCoordinator(clock: clock, recorder: recorder)

        _ = coordinator.submit("active", validationError: nil)
        let flush = Task { @MainActor in await coordinator.flush() }
        try await waitUntil("flush starts active save") {
            recorder.calls.map(\.value) == ["active"]
        }

        flush.cancel()
        await Task.yield()
        try expectEqual(
            recorder.calls.map(\.value),
            ["active"],
            "Cancelling a termination waiter must not duplicate or cancel the worker it is observing."
        )

        recorder.resumeNext(success: true)
        await flush.value
        try await waitUntil("active save survives flush caller cancellation") {
            coordinator.phase == .idle
        }
    }

    private func phaseCallbackPublishesCompleteLifecycle() async throws {
        let clock = ControlledAutosaveClock()
        let recorder = ControlledAutosaveSaveRecorder<String>(suspends: true)
        var phases: [RevisionAutosavePhase] = []
        let coordinator = RevisionAutosaveCoordinator(
            delayNanoseconds: 900_000_000,
            sleep: { nanoseconds in try await clock.sleep(nanoseconds) },
            save: { value, revision in await recorder.save(value, revision: revision) },
            phaseChanged: { phases.append($0) },
            completion: { _ in }
        )

        let revision = coordinator.submit("phase", validationError: nil)
        try await releaseNextDebounce(clock)
        try await waitUntil("phase callback reaches saving") {
            recorder.calls.map(\.value) == ["phase"]
        }
        recorder.resumeNext(success: true)
        try await waitUntil("phase callback reaches idle") { coordinator.phase == .idle }

        try expectEqual(
            phases,
            [.debouncing(revision: revision), .saving(revision: revision), .idle],
            "Phase callbacks should publish the exact debounce, active-save, and settled lifecycle."
        )
    }

    private func failedSaveWithoutPendingIsNotRetriedByFlush() async throws {
        let clock = ControlledAutosaveClock()
        let recorder = ControlledAutosaveSaveRecorder<String>(suspends: true)
        let coordinator = makeCoordinator(clock: clock, recorder: recorder)

        let revision = coordinator.submit("fails", validationError: nil)
        try await releaseNextDebounce(clock)
        try await waitUntil("failed save starts") { recorder.calls.map(\.value) == ["fails"] }
        recorder.resumeNext(success: false)
        try await waitUntil("failed save settles") {
            coordinator.phase == .failed(revision: revision, message: "Autosave failed.")
        }

        await coordinator.flush()
        try expectEqual(
            recorder.calls.map(\.value),
            ["fails"],
            "Flush must not silently retry a failed revision when no newer value is pending."
        )
    }

    private func invalidEditDuringActiveSaveLeavesNoPendingRevision() async throws {
        let clock = ControlledAutosaveClock()
        let recorder = ControlledAutosaveSaveRecorder<String>(suspends: true)
        let coordinator = makeCoordinator(clock: clock, recorder: recorder)

        let revisionA = coordinator.submit("A", validationError: nil)
        try await releaseNextDebounce(clock)
        try await waitUntil("active save A starts before invalid edit") {
            recorder.calls.map(\.value) == ["A"]
        }
        _ = coordinator.submit("invalid", validationError: "invalid")
        try expectEqual(
            coordinator.phase,
            .saving(revision: revisionA),
            "An invalid current edit should remove pending work without relabeling the active save."
        )

        recorder.resumeNext(success: true)
        await coordinator.flush()
        try expectEqual(
            recorder.calls.map(\.value),
            ["A"],
            "An invalid edit arriving during A must not revive or enqueue an older valid draft."
        )
    }

    private func editsBAndCDuringSaveRunOnlyLatestC() async throws {
        let clock = ControlledAutosaveClock()
        let recorder = ControlledAutosaveSaveRecorder<String>(suspends: true)
        let coordinator = makeCoordinator(clock: clock, recorder: recorder)

        _ = coordinator.submit("A", validationError: nil)
        try await releaseNextDebounce(clock)
        try await waitUntil("A starts before B and C") { recorder.calls.map(\.value) == ["A"] }
        _ = coordinator.submit("B", validationError: nil)
        let revisionC = coordinator.submit("C", validationError: nil)
        try expectEqual(
            coordinator.phase,
            .pendingAfterSave(revision: revisionC),
            "C should replace B as the only pending revision while A is active."
        )

        recorder.resumeNext(success: true)
        try await releaseNextDebounce(clock)
        try await waitUntil("C starts after A") { recorder.calls.map(\.value) == ["A", "C"] }
        recorder.resumeNext(success: true)
        await coordinator.flush()

        try expectEqual(
            recorder.calls.map(\.value),
            ["A", "C"],
            "A/B/C ordering should coalesce the edits made during A to only the latest C."
        )
    }

    private func persistedValueDuringActiveSaveMustSubmit() throws {
        try expectFalse(
            AutosaveDraftSubmissionPolicy.shouldSubmit(
                hasChangesFromPersistedValue: false,
                hasActiveSave: false
            ),
            "An unchanged idle draft should not create a redundant autosave."
        )
        try expectEqual(
            AutosaveDraftSubmissionPolicy.shouldSubmit(
                hasChangesFromPersistedValue: false,
                hasActiveSave: true
            ),
            true,
            "Reverting to the old persisted value while A saves must enqueue a newer revision."
        )
        try expectEqual(
            AutosaveDraftSubmissionPolicy.shouldSubmit(
                hasChangesFromPersistedValue: true,
                hasActiveSave: false
            ),
            true,
            "A changed valid draft should autosave while the worker is idle."
        )
    }

    private func makeCoordinator(
        clock: ControlledAutosaveClock,
        recorder: ControlledAutosaveSaveRecorder<String>,
        completion: @escaping @MainActor (RevisionAutosaveCompletion<String>) -> Void = { _ in }
    ) -> RevisionAutosaveCoordinator<String> {
        RevisionAutosaveCoordinator(
            delayNanoseconds: 900_000_000,
            sleep: { nanoseconds in try await clock.sleep(nanoseconds) },
            save: { value, revision in await recorder.save(value, revision: revision) },
            phaseChanged: { _ in },
            completion: completion
        )
    }

    private func releaseNextDebounce(_ clock: ControlledAutosaveClock) async throws {
        try await waitForClockCount(1, clock: clock)
        await clock.releaseNext()
    }

    private func waitForClockCount(_ count: Int, clock: ControlledAutosaveClock) async throws {
        try await waitUntil("autosave clock waiter count \(count)") {
            await clock.pendingCount == count
        }
    }

    private func waitUntil(
        _ label: String,
        timeout: TimeInterval = 2,
        condition: @escaping @MainActor () async -> Bool
    ) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while !(await condition()) {
            if Date() > deadline {
                throw NativeModelTestFailure(description: "Timed out waiting for \(label).")
            }
            try await Task.sleep(nanoseconds: 5_000_000)
        }
    }
}

private actor ControlledAutosaveClock {
    private struct Waiter {
        let id: UUID
        let continuation: CheckedContinuation<Void, Error>
    }

    private var waiters: [Waiter] = []

    var pendingCount: Int { waiters.count }

    func sleep(_ nanoseconds: UInt64) async throws {
        _ = nanoseconds
        try Task.checkCancellation()
        let id = UUID()
        try await withTaskCancellationHandler {
            try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
                if Task.isCancelled {
                    continuation.resume(throwing: CancellationError())
                } else {
                    waiters.append(Waiter(id: id, continuation: continuation))
                }
            }
        } onCancel: {
            Task { await self.cancel(id) }
        }
    }

    func releaseNext() {
        guard !waiters.isEmpty else { return }
        waiters.removeFirst().continuation.resume()
    }

    private func cancel(_ id: UUID) {
        guard let index = waiters.firstIndex(where: { $0.id == id }) else { return }
        waiters.remove(at: index).continuation.resume(throwing: CancellationError())
    }
}

@MainActor
private final class ControlledAutosaveSaveRecorder<Value: Equatable> {
    struct Call: Equatable {
        let value: Value
        let revision: UInt64
    }

    private(set) var calls: [Call] = []
    private var continuations: [CheckedContinuation<Bool, Never>] = []
    private let suspends: Bool

    init(suspends: Bool) {
        self.suspends = suspends
    }

    func save(_ value: Value, revision: UInt64) async -> Bool {
        calls.append(Call(value: value, revision: revision))
        guard suspends else { return true }
        return await withCheckedContinuation { continuation in
            continuations.append(continuation)
        }
    }

    func resumeNext(success: Bool) {
        guard !continuations.isEmpty else { return }
        continuations.removeFirst().resume(returning: success)
    }
}
