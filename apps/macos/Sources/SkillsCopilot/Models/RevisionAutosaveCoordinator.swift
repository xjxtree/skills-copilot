import Foundation

enum RevisionAutosavePhase: Equatable {
    case idle
    case debouncing(revision: UInt64)
    case saving(revision: UInt64)
    case pendingAfterSave(revision: UInt64)
    case failed(revision: UInt64, message: String)
}

struct RevisionAutosaveCompletion<Value> {
    let revision: UInt64
    let value: Value
    let succeeded: Bool
}

@MainActor
final class RevisionAutosaveCoordinator<Value: Equatable> {
    typealias Sleep = @Sendable (UInt64) async throws -> Void
    typealias Save = @MainActor (Value, UInt64) async -> Bool
    typealias Completion = @MainActor (RevisionAutosaveCompletion<Value>) -> Void
    typealias PhaseChanged = @MainActor (RevisionAutosavePhase) -> Void

    private let delayNanoseconds: UInt64
    private let sleep: Sleep
    private let save: Save
    private let completion: Completion
    private let phaseChanged: PhaseChanged
    private var nextRevision: UInt64 = 0
    private var pending: (revision: UInt64, value: Value)?
    private var debounceTask: Task<Void, Never>?
    private var workerTask: Task<Void, Never>?
    private var activeRevision: UInt64?
    private(set) var phase: RevisionAutosavePhase = .idle

    init(
        delayNanoseconds: UInt64,
        sleep: @escaping Sleep = { try await Task.sleep(nanoseconds: $0) },
        save: @escaping Save,
        phaseChanged: @escaping PhaseChanged,
        completion: @escaping Completion
    ) {
        self.delayNanoseconds = delayNanoseconds
        self.sleep = sleep
        self.save = save
        self.phaseChanged = phaseChanged
        self.completion = completion
    }

    @discardableResult
    func submit(_ value: Value, validationError: String?) -> UInt64 {
        nextRevision &+= 1
        let revision = nextRevision

        cancelDebounceTask()
        guard validationError == nil else {
            pending = nil
            publishPhaseWithoutPending()
            return revision
        }

        pending = (revision, value)
        if workerTask != nil {
            setPhase(.pendingAfterSave(revision: revision))
        } else {
            scheduleDebounce(revision: revision)
        }
        return revision
    }

    func cancelPendingDebounce() {
        cancelDebounceTask()
        pending = nil
        publishPhaseWithoutPending()
    }

    func flush() async {
        while true {
            cancelDebounceTask()

            if workerTask == nil, let pending {
                self.pending = nil
                startWorker(revision: pending.revision, value: pending.value)
            }

            guard let workerTask else { return }
            await workerTask.value
        }
    }

    private func scheduleDebounce(revision: UInt64) {
        cancelDebounceTask()
        setPhase(.debouncing(revision: revision))
        let delayNanoseconds = delayNanoseconds
        let sleep = sleep
        debounceTask = Task { @MainActor [weak self] in
            do {
                try await sleep(delayNanoseconds)
            } catch {
                return
            }
            guard !Task.isCancelled else { return }
            self?.debounceElapsed(revision: revision)
        }
    }

    private func debounceElapsed(revision: UInt64) {
        guard let pending, pending.revision == revision else { return }
        debounceTask = nil
        guard workerTask == nil else {
            setPhase(.pendingAfterSave(revision: revision))
            return
        }
        self.pending = nil
        startWorker(revision: revision, value: pending.value)
    }

    private func startWorker(revision: UInt64, value: Value) {
        activeRevision = revision
        setPhase(.saving(revision: revision))
        let save = save
        workerTask = Task { @MainActor [weak self] in
            let succeeded = await save(value, revision)
            self?.workerFinished(revision: revision, value: value, succeeded: succeeded)
        }
    }

    private func workerFinished(revision: UInt64, value: Value, succeeded: Bool) {
        guard activeRevision == revision else { return }
        activeRevision = nil
        workerTask = nil
        completion(
            RevisionAutosaveCompletion(
                revision: revision,
                value: value,
                succeeded: succeeded
            )
        )

        if let pending {
            scheduleDebounce(revision: pending.revision)
        } else if succeeded {
            setPhase(.idle)
        } else {
            setPhase(.failed(revision: revision, message: "Autosave failed."))
        }
    }

    private func cancelDebounceTask() {
        debounceTask?.cancel()
        debounceTask = nil
    }

    private func publishPhaseWithoutPending() {
        if let activeRevision {
            setPhase(.saving(revision: activeRevision))
        } else {
            setPhase(.idle)
        }
    }

    private func setPhase(_ newPhase: RevisionAutosavePhase) {
        phase = newPhase
        phaseChanged(newPhase)
    }
}
