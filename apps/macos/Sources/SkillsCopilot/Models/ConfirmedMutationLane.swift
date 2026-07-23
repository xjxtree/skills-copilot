import Foundation

@MainActor
final class ConfirmedMutationLane {
    typealias Operation<Result> = @MainActor () async -> Result

    private var isOccupied = false
    private var isShutdown = false
    private var waiters: [CheckedContinuation<Bool, Never>] = []

    func perform<Result>(_ operation: Operation<Result>) async -> Result? {
        let acquired = await acquire()
        guard acquired else { return nil }
        defer { release() }
        return await operation()
    }

    @discardableResult
    func shutdown() -> Int {
        isShutdown = true
        let queued = waiters
        waiters.removeAll()
        for waiter in queued {
            waiter.resume(returning: false)
        }
        return queued.count
    }

    private func acquire() async -> Bool {
        guard !isShutdown else { return false }
        if !isOccupied {
            isOccupied = true
            return true
        }
        return await withCheckedContinuation { continuation in
            guard !isShutdown else {
                continuation.resume(returning: false)
                return
            }
            waiters.append(continuation)
        }
    }

    private func release() {
        guard !isShutdown else {
            isOccupied = false
            return
        }
        guard !waiters.isEmpty else {
            isOccupied = false
            return
        }
        waiters.removeFirst().resume(returning: true)
    }
}
