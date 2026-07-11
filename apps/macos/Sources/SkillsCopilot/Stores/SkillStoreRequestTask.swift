import Foundation

final class SkillManagerRequestTaskWaiter: @unchecked Sendable {
    private let lock = NSLock()
    private var continuation: CheckedContinuation<Void, Never>?
    private var finished = false

    func wait() async {
        await withCheckedContinuation { continuation in
            let shouldResume: Bool
            lock.lock()
            if finished {
                shouldResume = true
            } else {
                self.continuation = continuation
                shouldResume = false
            }
            lock.unlock()
            if shouldResume {
                continuation.resume()
            }
        }
    }

    func finish() {
        let continuation: CheckedContinuation<Void, Never>?
        lock.lock()
        if finished {
            continuation = nil
        } else {
            finished = true
            continuation = self.continuation
            self.continuation = nil
        }
        lock.unlock()
        continuation?.resume()
    }
}

final class SkillManagerRequestTaskHandle: @unchecked Sendable {
    private let task: Task<Void, Never>
    private let waiter = SkillManagerRequestTaskWaiter()

    init(task: Task<Void, Never>) {
        self.task = task
        let waiter = self.waiter
        Task {
            await task.value
            waiter.finish()
        }
    }

    func wait() async {
        await withTaskCancellationHandler {
            await waiter.wait()
        } onCancel: {
            self.cancel()
        }
    }

    func cancel() {
        task.cancel()
        waiter.finish()
    }
}
