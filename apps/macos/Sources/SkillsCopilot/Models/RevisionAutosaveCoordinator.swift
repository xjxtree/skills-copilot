import Foundation

enum RevisionAutosavePhase: Equatable {
    case idle
    case debouncing(revision: UInt64)
    case saving(revision: UInt64)
    case pendingAfterSave(revision: UInt64)
    case failed(revision: UInt64, message: String)
}

enum RevisionAutosaveSaveOutcome: Equatable {
    case succeeded
    case failed
    case cancelled
}

struct RevisionAutosaveCompletion<Value> {
    let revision: UInt64
    let value: Value
    let outcome: RevisionAutosaveSaveOutcome

    var succeeded: Bool { outcome == .succeeded }
}

enum AutosaveDraftSubmissionPolicy {
    static func shouldSubmit(
        hasChangesFromPersistedValue: Bool,
        hasActiveSave: Bool
    ) -> Bool {
        hasChangesFromPersistedValue || hasActiveSave
    }
}

enum AutosaveDraftPresentation {
    static func resolve<Value>(storeDraft: Value?, persistedValue: @autoclosure () -> Value) -> Value {
        storeDraft ?? persistedValue()
    }
}

enum ConfigAutosaveDraftEvent: Equatable {
    case hydrate(storeDraft: String?, persistedContent: String)
    case userChanged(
        storeDraft: String?,
        persistedContent: String,
        revealsSensitiveConfig: Bool,
        hasActiveSave: Bool,
        validationError: String?
    )
}

enum ConfigAutosaveDraftAction: Equatable {
    case none
    case cancelPending
    case submit(content: String, validationError: String?)
}

struct ConfigAutosaveDraftTransition: Equatable {
    let content: String
    let action: ConfigAutosaveDraftAction
}

enum ConfigAutosaveDraftReducer {
    static func reduce(
        content: String,
        event: ConfigAutosaveDraftEvent
    ) -> ConfigAutosaveDraftTransition {
        switch event {
        case let .hydrate(storeDraft, persistedContent):
            return ConfigAutosaveDraftTransition(
                content: AutosaveDraftPresentation.resolve(
                    storeDraft: storeDraft,
                    persistedValue: persistedContent
                ),
                action: .none
            )

        case let .userChanged(
            storeDraft,
            persistedContent,
            revealsSensitiveConfig,
            hasActiveSave,
            validationError
        ):
            guard revealsSensitiveConfig, storeDraft != content else {
                return ConfigAutosaveDraftTransition(content: content, action: .none)
            }
            guard AutosaveDraftSubmissionPolicy.shouldSubmit(
                hasChangesFromPersistedValue: content != persistedContent,
                hasActiveSave: hasActiveSave
            ) else {
                let action: ConfigAutosaveDraftAction = storeDraft == nil ? .none : .cancelPending
                return ConfigAutosaveDraftTransition(content: content, action: action)
            }
            return ConfigAutosaveDraftTransition(
                content: content,
                action: .submit(content: content, validationError: validationError)
            )
        }
    }
}

@MainActor
struct AutosaveMutationLaneToken: Hashable {
    enum Family: Hashable {
        case config
        case provider
    }

    let family: Family
    let revision: UInt64
}

enum AutosaveMutationLaneResult<Result> {
    case completed(Result)
    case cancelled
}

extension AutosaveMutationLaneResult: Equatable where Result: Equatable {}

@MainActor
final class AutosaveMutationLane {
    typealias Operation<Result> = @MainActor () async -> Result

    private struct Waiter {
        let token: AutosaveMutationLaneToken?
        let continuation: CheckedContinuation<Bool, Never>
    }

    private var isOccupied = false
    private var isShutdown = false
    private var waiters: [Waiter] = []
    private var currentOwnerToken: AutosaveMutationLaneToken?
    private var registeredTokens: Set<AutosaveMutationLaneToken> = []
    private var cancelledTokens: Set<AutosaveMutationLaneToken> = []

    var queuedCount: Int { waiters.count }

    @discardableResult
    func register(_ token: AutosaveMutationLaneToken) -> Bool {
        guard !isShutdown,
              currentOwnerToken != token,
              !registeredTokens.contains(token),
              !cancelledTokens.contains(token),
              !waiters.contains(where: { $0.token == token }) else {
            return false
        }
        registeredTokens.insert(token)
        return true
    }

    func perform<Result>(_ operation: Operation<Result>) async -> Result {
        let acquired = await acquire(token: nil)
        precondition(acquired, "Untracked mutation operations cannot start after lane shutdown.")
        defer { release() }
        return await operation()
    }

    func perform<Result>(
        token: AutosaveMutationLaneToken,
        _ operation: Operation<Result>
    ) async -> AutosaveMutationLaneResult<Result> {
        guard await acquire(token: token) else {
            cancelledTokens.remove(token)
            return .cancelled
        }
        defer { release() }
        return .completed(await operation())
    }

    @discardableResult
    func cancelQueued(_ token: AutosaveMutationLaneToken) -> Bool {
        guard !isShutdown, currentOwnerToken != token else { return false }
        guard !cancelledTokens.contains(token) else { return false }
        if let index = waiters.firstIndex(where: { $0.token == token }) {
            cancelledTokens.insert(token)
            let waiter = waiters.remove(at: index)
            waiter.continuation.resume(returning: false)
            return true
        }
        guard registeredTokens.remove(token) != nil else { return false }
        cancelledTokens.insert(token)
        return true
    }

    @discardableResult
    func shutdown() -> Int {
        isShutdown = true
        registeredTokens.removeAll()
        cancelledTokens.removeAll()
        let queued = waiters
        waiters.removeAll()
        for waiter in queued {
            waiter.continuation.resume(returning: false)
        }
        return queued.count
    }

    private func acquire(token: AutosaveMutationLaneToken?) async -> Bool {
        guard !isShutdown else { return false }
        if let token {
            if Task.isCancelled {
                registeredTokens.remove(token)
                return false
            }
            if cancelledTokens.remove(token) != nil {
                registeredTokens.remove(token)
                return false
            }
            registeredTokens.insert(token)
        }
        if !isOccupied {
            isOccupied = true
            if let token {
                registeredTokens.remove(token)
            }
            currentOwnerToken = token
            return true
        }
        guard let token else {
            return await enqueue(token: nil)
        }
        return await withTaskCancellationHandler {
            await enqueue(token: token)
        } onCancel: {
            Task { @MainActor [weak self] in
                self?.cancelQueued(token)
            }
        }
    }

    private func enqueue(token: AutosaveMutationLaneToken?) async -> Bool {
        await withCheckedContinuation { continuation in
            guard !isShutdown else {
                continuation.resume(returning: false)
                return
            }
            if let token, cancelledTokens.remove(token) != nil {
                registeredTokens.remove(token)
                continuation.resume(returning: false)
                return
            }
            if let token {
                registeredTokens.remove(token)
            }
            waiters.append(Waiter(token: token, continuation: continuation))
        }
    }

    private func release() {
        currentOwnerToken = nil
        guard !isShutdown else {
            isOccupied = false
            return
        }
        guard !waiters.isEmpty else {
            isOccupied = false
            return
        }
        currentOwnerToken = waiters.first?.token
        waiters.removeFirst().continuation.resume(returning: true)
    }
}

@MainActor
final class RevisionAutosaveCoordinator<Value: Equatable> {
    typealias Sleep = @Sendable (UInt64) async throws -> Void
    typealias Save = @MainActor (Value, UInt64) async -> RevisionAutosaveSaveOutcome
    typealias WorkerWillStart = @MainActor (UInt64) -> Void
    typealias Completion = @MainActor (RevisionAutosaveCompletion<Value>) -> Void
    typealias PhaseChanged = @MainActor (RevisionAutosavePhase) -> Void

    private let delayNanoseconds: UInt64
    private let sleep: Sleep
    private let workerWillStart: WorkerWillStart
    private let save: Save
    private let completion: Completion
    private let phaseChanged: PhaseChanged
    private var nextRevision: UInt64 = 0
    private var pending: (revision: UInt64, value: Value)?
    private var debounceTask: Task<Void, Never>?
    private var workerTask: Task<Void, Never>?
    private var activeRevision: UInt64?
    private(set) var phase: RevisionAutosavePhase = .idle

    var hasActiveSave: Bool { workerTask != nil }
    var activeSaveRevision: UInt64? { activeRevision }

    init(
        delayNanoseconds: UInt64,
        sleep: @escaping Sleep = { try await Task.sleep(nanoseconds: $0) },
        workerWillStart: @escaping WorkerWillStart = { _ in },
        save: @escaping Save,
        phaseChanged: @escaping PhaseChanged,
        completion: @escaping Completion
    ) {
        self.delayNanoseconds = delayNanoseconds
        self.sleep = sleep
        self.workerWillStart = workerWillStart
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
        workerWillStart(revision)
        setPhase(.saving(revision: revision))
        let save = save
        workerTask = Task { @MainActor [weak self] in
            let outcome = await save(value, revision)
            self?.workerFinished(revision: revision, value: value, outcome: outcome)
        }
    }

    private func workerFinished(
        revision: UInt64,
        value: Value,
        outcome: RevisionAutosaveSaveOutcome
    ) {
        guard activeRevision == revision else { return }
        activeRevision = nil
        workerTask = nil
        completion(
            RevisionAutosaveCompletion(
                revision: revision,
                value: value,
                outcome: outcome
            )
        )

        if let pending {
            scheduleDebounce(revision: pending.revision)
        } else if outcome != .failed {
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
