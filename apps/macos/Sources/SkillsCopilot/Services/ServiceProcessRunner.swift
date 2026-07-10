import Darwin
import Foundation

protocol ServiceProcessRunning {
    func run(executableURL: URL, input: Data, timeoutNanoseconds: UInt64?) async throws -> Data
}

private struct BoundedPipeOutput {
    let data: Data
    let discardedByteCount: Int

    var wasTruncated: Bool { discardedByteCount > 0 }
}

private final class BoundedPipeDrain {
    static let chunkSize = 64 * 1_024

    private let maximumRetainedBytes: Int

    init(maximumRetainedBytes: Int) {
        self.maximumRetainedBytes = max(0, maximumRetainedBytes)
    }

    func readToEOF(from handle: FileHandle) throws -> BoundedPipeOutput {
        var retained = Data()
        var discardedByteCount = 0

        while let chunk = try handle.read(upToCount: Self.chunkSize), !chunk.isEmpty {
            let available = maximumRetainedBytes - retained.count
            let retainedCount = min(available, chunk.count)
            if retainedCount > 0 {
                retained.append(chunk.prefix(retainedCount))
            }
            let discarded = chunk.count - retainedCount
            let sum = discardedByteCount.addingReportingOverflow(discarded)
            discardedByteCount = sum.overflow ? Int.max : sum.partialValue
        }

        return BoundedPipeOutput(
            data: retained,
            discardedByteCount: discardedByteCount
        )
    }
}

private struct StdioOutputs {
    let stdout: BoundedPipeOutput
    let stderr: BoundedPipeOutput
}

private final class StdioPipeCollector {
    static let maximumStdoutBytes = 16 * 1_024 * 1_024
    static let maximumStderrBytes = 1 * 1_024 * 1_024

    private enum CollectorError: Error {
        case missingOutput
    }

    private let stdoutReader: FileHandle
    private let stderrReader: FileHandle
    private let group = DispatchGroup()
    private let queue = DispatchQueue(
        label: "com.agent-copilot.service-pipe-drain",
        qos: .utility,
        attributes: .concurrent
    )
    private let lock = NSLock()
    private var stdoutResult: Result<BoundedPipeOutput, Error>?
    private var stderrResult: Result<BoundedPipeOutput, Error>?
    private var started = false

    init(stdoutReader: FileHandle, stderrReader: FileHandle) {
        self.stdoutReader = stdoutReader
        self.stderrReader = stderrReader
    }

    func start() {
        lock.lock()
        guard !started else {
            lock.unlock()
            return
        }
        started = true
        lock.unlock()

        startDrain(
            handle: stdoutReader,
            maximumRetainedBytes: Self.maximumStdoutBytes,
            isStdout: true
        )
        startDrain(
            handle: stderrReader,
            maximumRetainedBytes: Self.maximumStderrBytes,
            isStdout: false
        )
    }

    func waitForOutputs() throws -> StdioOutputs {
        group.wait()
        lock.lock()
        let stdoutResult = self.stdoutResult
        let stderrResult = self.stderrResult
        lock.unlock()

        guard let stdoutResult, let stderrResult else {
            throw CollectorError.missingOutput
        }
        return try StdioOutputs(
            stdout: stdoutResult.get(),
            stderr: stderrResult.get()
        )
    }

    func cancel() {
        try? stdoutReader.close()
        try? stderrReader.close()
    }

    private func startDrain(
        handle: FileHandle,
        maximumRetainedBytes: Int,
        isStdout: Bool
    ) {
        group.enter()
        queue.async { [self] in
            defer { group.leave() }
            let result = Result {
                try BoundedPipeDrain(maximumRetainedBytes: maximumRetainedBytes)
                    .readToEOF(from: handle)
            }
            lock.lock()
            if isStdout {
                stdoutResult = result
            } else {
                stderrResult = result
            }
            lock.unlock()
        }
    }
}

final class StdioServiceProcessRunner: ServiceProcessRunning {
    private let timeoutNanoseconds: UInt64
    private let environmentOverrides: [String: String]

    init(
        timeoutNanoseconds: UInt64 = StdioServiceProcessRunner.configuredTimeoutNanoseconds(),
        environmentOverrides: [String: String] = [:]
    ) {
        self.timeoutNanoseconds = timeoutNanoseconds
        self.environmentOverrides = environmentOverrides
    }

    func run(executableURL: URL, input: Data, timeoutNanoseconds overrideTimeoutNanoseconds: UInt64? = nil) async throws -> Data {
        let invocation = StdioServiceProcessInvocation(
            executableURL: executableURL,
            input: input,
            environmentOverrides: environmentOverrides
        )
        let coordinator = StdioServiceProcessRunCoordinator(invocation: invocation)
        let effectiveTimeoutNanoseconds = overrideTimeoutNanoseconds ?? timeoutNanoseconds

        return try await withTaskCancellationHandler {
            try await withCheckedThrowingContinuation { continuation in
                coordinator.start(
                    timeoutNanoseconds: effectiveTimeoutNanoseconds,
                    continuation: continuation
                )
            }
        } onCancel: {
            coordinator.cancel()
        }
    }

    static func configuredTimeoutNanoseconds(
        environment: [String: String] = ProcessInfo.processInfo.environment
    ) -> UInt64 {
        let defaultMilliseconds: UInt64 = 30_000
        let maximumMilliseconds: UInt64 = 300_000
        guard let raw = environment["SKILLS_COPILOT_SERVICE_TIMEOUT_MS"],
              let parsed = UInt64(raw),
              parsed <= maximumMilliseconds else {
            return defaultMilliseconds * 1_000_000
        }
        let milliseconds = max(parsed, 50)
        let product = milliseconds.multipliedReportingOverflow(by: 1_000_000)
        return product.overflow
            ? defaultMilliseconds * 1_000_000
            : product.partialValue
    }
}

private final class StdioServiceProcessRunCoordinator {
    private let invocation: StdioServiceProcessInvocation
    private let lock = NSLock()
    private var continuation: CheckedContinuation<Data, Error>?
    private var operationTask: Task<Void, Never>?
    private var timeoutTask: Task<Void, Never>?
    private var completed = false

    init(invocation: StdioServiceProcessInvocation) {
        self.invocation = invocation
    }

    func start(
        timeoutNanoseconds: UInt64,
        continuation: CheckedContinuation<Data, Error>
    ) {
        lock.lock()
        self.continuation = continuation
        lock.unlock()

        operationTask = Task.detached(priority: .userInitiated) { [weak self] in
            guard let self else { return }
            do {
                let data = try self.invocation.run()
                self.finish(.success(data))
            } catch {
                self.finish(.failure(error))
            }
        }

        timeoutTask = Task { [weak self] in
            do {
                try await Task.sleep(nanoseconds: timeoutNanoseconds)
            } catch {
                return
            }
            guard let self else { return }
            self.invocation.cancel()
            self.operationTask?.cancel()
            self.finish(.failure(ServiceClient.ClientError.processTimedOut))
        }
    }

    func cancel() {
        invocation.cancel()
        operationTask?.cancel()
        finish(.failure(CancellationError()))
    }

    private func finish(_ result: Result<Data, Error>) {
        let continuation: CheckedContinuation<Data, Error>?
        let timeoutTask: Task<Void, Never>?
        lock.lock()
        if completed {
            continuation = nil
            timeoutTask = nil
        } else {
            completed = true
            continuation = self.continuation
            self.continuation = nil
            timeoutTask = self.timeoutTask
            self.timeoutTask = nil
        }
        lock.unlock()

        timeoutTask?.cancel()
        switch result {
        case .success(let data):
            continuation?.resume(returning: data)
        case .failure(let error):
            continuation?.resume(throwing: error)
        }
    }
}

private final class StdioServiceProcessInvocation {
    private let executableURL: URL
    private let input: Data
    private let environmentOverrides: [String: String]
    private let lock = NSLock()

    private var process: Process?
    private var stdinWriter: FileHandle?
    private var stdoutReader: FileHandle?
    private var stderrReader: FileHandle?
    private var cancelled = false
    private var terminationRequested = false
    private var cleanedUp = false

    init(executableURL: URL, input: Data, environmentOverrides: [String: String]) {
        self.executableURL = executableURL
        self.input = input
        self.environmentOverrides = environmentOverrides
    }

    func run() throws -> Data {
        try Task.checkCancellation()

        let process = Process()
        process.executableURL = executableURL
        if !environmentOverrides.isEmpty {
            var environment = ProcessInfo.processInfo.environment
            environmentOverrides.forEach { key, value in
                environment[key] = value
            }
            process.environment = environment
        }

        let stdin = Pipe()
        let stdout = Pipe()
        let stderr = Pipe()
        process.standardInput = stdin
        process.standardOutput = stdout
        process.standardError = stderr
        let collector = StdioPipeCollector(
            stdoutReader: stdout.fileHandleForReading,
            stderrReader: stderr.fileHandleForReading
        )

        register(
            process: process,
            stdinWriter: stdin.fileHandleForWriting,
            stdoutReader: stdout.fileHandleForReading,
            stderrReader: stderr.fileHandleForReading
        )

        do {
            try process.run()
            collector.start()
            try Task.checkCancellation()
            try stdin.fileHandleForWriting.write(contentsOf: input)
            try stdin.fileHandleForWriting.close()
            clearStdinWriter(stdin.fileHandleForWriting)

            waitUntilExit(process)
            let outputs = try collector.waitForOutputs()

            try Task.checkCancellation()
            guard !isCancelled else {
                throw CancellationError()
            }

            cleanup(closePipes: true)

            if outputs.stdout.wasTruncated {
                throw ServiceClient.ClientError.responseTooLarge(
                    maxBytes: StdioPipeCollector.maximumStdoutBytes
                )
            }
            if process.terminationStatus != 0 {
                let retainedStderr = String(data: outputs.stderr.data, encoding: .utf8) ?? ""
                throw ServiceClient.ClientError.processFailed(
                    process.terminationStatus,
                    ServiceDiagnosticSanitizer.displayMessage(retainedStderr)
                )
            }
            return outputs.stdout.data
        } catch is CancellationError {
            cancel()
            collector.cancel()
            cleanup(closePipes: true)
            throw CancellationError()
        } catch {
            if Task.isCancelled || isCancelled {
                cancel()
                collector.cancel()
                cleanup(closePipes: true)
                throw CancellationError()
            }
            collector.cancel()
            terminateForFailure()
            if process.isRunning {
                waitUntilExit(process)
            }
            cleanup(closePipes: true)
            throw error
        }
    }

    func cancel() {
        let snapshot = markCancelled()
        try? snapshot.stdinWriter?.close()
        try? snapshot.stdoutReader?.close()
        try? snapshot.stderrReader?.close()
        if snapshot.shouldTerminate, let process = snapshot.process, process.isRunning {
            process.terminate()
            forceTerminate(process, after: .milliseconds(250))
        }
    }

    private func waitUntilExit(_ process: Process) {
        while process.isRunning {
            if Task.isCancelled || isCancelled {
                cancel()
            }
            Thread.sleep(forTimeInterval: 0.01)
        }
        process.waitUntilExit()
    }

    private func register(
        process: Process,
        stdinWriter: FileHandle,
        stdoutReader: FileHandle,
        stderrReader: FileHandle
    ) {
        lock.lock()
        self.process = process
        self.stdinWriter = stdinWriter
        self.stdoutReader = stdoutReader
        self.stderrReader = stderrReader
        let shouldCancel = cancelled
        lock.unlock()

        if shouldCancel {
            cancel()
        }
    }

    private func clearStdinWriter(_ handle: FileHandle) {
        lock.lock()
        if stdinWriter === handle {
            stdinWriter = nil
        }
        lock.unlock()
    }

    private var isCancelled: Bool {
        lock.lock()
        let value = cancelled
        lock.unlock()
        return value
    }

    private func markCancelled() -> ProcessSnapshot {
        lock.lock()
        cancelled = true
        let shouldTerminate = process?.isRunning == true && !terminationRequested
        if shouldTerminate {
            terminationRequested = true
        }
        let stdinWriter = stdinWriter
        self.stdinWriter = nil
        let snapshot = ProcessSnapshot(
            process: process,
            stdinWriter: stdinWriter,
            stdoutReader: stdoutReader,
            stderrReader: stderrReader,
            shouldTerminate: shouldTerminate
        )
        lock.unlock()
        return snapshot
    }

    private func terminateForFailure() {
        let snapshot: ProcessSnapshot
        lock.lock()
        let shouldTerminate = process?.isRunning == true && !terminationRequested
        if shouldTerminate {
            terminationRequested = true
        }
        snapshot = ProcessSnapshot(
            process: process,
            stdinWriter: stdinWriter,
            stdoutReader: stdoutReader,
            stderrReader: stderrReader,
            shouldTerminate: shouldTerminate
        )
        lock.unlock()

        try? snapshot.stdinWriter?.close()
        if snapshot.shouldTerminate, let process = snapshot.process, process.isRunning {
            process.terminate()
            forceTerminate(process, after: .milliseconds(250))
        }
    }

    private func cleanup(closePipes: Bool) {
        let snapshot: ProcessSnapshot?
        lock.lock()
        if cleanedUp {
            snapshot = nil
        } else {
            cleanedUp = true
            snapshot = ProcessSnapshot(
                process: process,
                stdinWriter: stdinWriter,
                stdoutReader: stdoutReader,
                stderrReader: stderrReader,
                shouldTerminate: false
            )
            process = nil
            stdinWriter = nil
            stdoutReader = nil
            stderrReader = nil
        }
        lock.unlock()

        guard closePipes, let snapshot else { return }
        try? snapshot.stdinWriter?.close()
        try? snapshot.stdoutReader?.close()
        try? snapshot.stderrReader?.close()
    }

    private func forceTerminate(_ process: Process, after delay: DispatchTimeInterval) {
        let pid = process.processIdentifier
        DispatchQueue.global(qos: .utility).asyncAfter(deadline: .now() + delay) {
            if process.isRunning {
                kill(pid, SIGKILL)
            }
        }
    }

    private struct ProcessSnapshot {
        let process: Process?
        let stdinWriter: FileHandle?
        let stdoutReader: FileHandle?
        let stderrReader: FileHandle?
        let shouldTerminate: Bool
    }
}
