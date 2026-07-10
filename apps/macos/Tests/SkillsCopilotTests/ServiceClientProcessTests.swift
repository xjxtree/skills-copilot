import Darwin
import Foundation
@testable import SkillsCopilot

struct ServiceClientProcessTests {
    func run() async throws {
        try await cancelledCallTerminatesSidecarProcess()
        try await cancelledCallForceKillsTermIgnoringSidecarProcess()
        try await hangingCallTimesOutAndTerminatesSidecarProcess()
        try await largeStderrBeforeStdoutDoesNotDeadlock()
        try await concurrentLargeStdoutAndStderrAreDrained()
        try await largeInputAndEarlyOutputDoNotDeadlock()
        try await earlyExitWithoutReadingLargeInputDoesNotRaiseSIGPIPE()
        try await stdoutBoundaryIsExact()
        try await stdoutAboveSixteenMiBReturnsResponseTooLarge()
        try await oversizedFailingStderrIsBoundedAndRedacted()
        try diagnosticSanitizerCoversStandalonePatternsAndFallback()
        try await malformedStdoutNeverAppearsInDisplayError()
        try await invalidEnvelopeNeverAppearsInDisplayError()
        try await cancellationWhileDrainingReapsProcess()
        try await callCancelledBeforeRunnerStartStillCompletes()
        try configuredTimeoutRejectsOverflowAndInvalidValues()
        try await malformedOutputMapsToInvalidOutput()
        try await emptyOutputMapsToInvalidOutput()
        try await truncatedOutputMapsToInvalidOutput()
        try await stderrOnlyFailureMapsToProcessFailed()
    }

    private func cancelledCallTerminatesSidecarProcess() async throws {
        let fake = try CancellableServiceScript()
        defer { fake.cleanup() }
        fake.activate()

        let call = Task {
            try await fake.serviceClient().status()
        }

        let pid = try await fake.waitForPID()
        call.cancel()

        do {
            _ = try await call.value
            throw NativeModelTestFailure(description: "Cancelled service call should not return a status result.")
        } catch is CancellationError {
            // Expected: the process runner maps caller cancellation to Swift cancellation.
        }

        try await waitUntil("Cancelled sidecar process should be reaped.") {
            !processExists(pid)
        }
        try expectContains(fake.calls(), "\"method\":\"service.status\"", "Cancellation test should launch the expected service method.")
    }

    private func cancelledCallForceKillsTermIgnoringSidecarProcess() async throws {
        let fake = try CancellableServiceScript(ignoresTermination: true)
        defer { fake.cleanup() }
        fake.activate()

        let call = Task {
            try await fake.serviceClient().status()
        }

        let pid = try await fake.waitForPID()
        call.cancel()

        do {
            _ = try await call.value
            throw NativeModelTestFailure(description: "Cancelled stubborn service call should not return a status result.")
        } catch is CancellationError {
            // Expected: the process runner escalates from terminate() to SIGKILL after the cleanup timeout.
        }

        try await waitUntil("TERM-ignoring sidecar should be force-killed after the cleanup timeout.", timeout: 4) {
            !processExists(pid)
        }
        try expectContains(fake.calls(), "\"method\":\"service.status\"", "Force-kill test should launch the expected service method.")
    }

    private func hangingCallTimesOutAndTerminatesSidecarProcess() async throws {
        let fake = try CancellableServiceScript()
        defer { fake.cleanup() }
        fake.activate()

        let call = Task {
            try await fake.serviceClient(timeoutNanoseconds: 1_000_000_000).status()
        }

        let pid = try await fake.waitForPID()
        do {
            _ = try await call.value
            throw NativeModelTestFailure(description: "Hanging service call should time out.")
        } catch ServiceClient.ClientError.processTimedOut {
            // Expected: the runner maps a sidecar that never closes stdout to a bounded timeout.
        }

        try await waitUntil("Timed-out sidecar process should be reaped.", timeout: 4) {
            !processExists(pid)
        }
        try expectContains(fake.calls(), "\"method\":\"service.status\"", "Timeout test should launch the expected service method.")
    }

    private func largeStderrBeforeStdoutDoesNotDeadlock() async throws {
        let fake = try StaticServiceScript(mode: "stderr-before-stdout")
        defer { fake.cleanup() }

        let status = try await fake.serviceClient(timeoutNanoseconds: 5_000_000_000).status()
        try expectEqual(status.version, "test", "Large stderr should be drained before stdout is decoded.")
    }

    private func concurrentLargeStdoutAndStderrAreDrained() async throws {
        let fake = try StaticServiceScript(mode: "dual-large")
        defer { fake.cleanup() }

        let status = try await fake.serviceClient(timeoutNanoseconds: 5_000_000_000).status()
        try expectEqual(status.protocolVersion, 1, "Concurrent stdout and stderr should both drain without blocking.")
    }

    private func largeInputAndEarlyOutputDoNotDeadlock() async throws {
        let fake = try StaticServiceScript(mode: "large-input-early-output")
        defer { fake.cleanup() }

        let output = try await fake.run(
            input: Data(repeating: 0x49, count: 2 * 1_024 * 1_024),
            timeoutNanoseconds: 5_000_000_000
        )
        let envelope = try JSONDecoder().decode(ServiceEnvelope<ServiceStatus>.self, from: output)
        try expectEqual(envelope.result?.version, "test", "Early stdout must drain while a large request body is written.")
    }

    private func earlyExitWithoutReadingLargeInputDoesNotRaiseSIGPIPE() async throws {
        let fake = try StaticServiceScript(mode: "early-exit-without-stdin")
        defer { fake.cleanup() }

        do {
            _ = try await fake.run(
                input: Data(repeating: 0x49, count: 2 * 1_024 * 1_024),
                timeoutNanoseconds: 5_000_000_000
            )
            throw NativeModelTestFailure(description: "A sidecar that exits before reading stdin should fail safely.")
        } catch ServiceClient.ClientError.processFailed(let status, let diagnostic) {
            try expectEqual(status, 7, "Early sidecar exit should preserve its status instead of raising SIGPIPE.")
            try expectContains(diagnostic, "early sidecar failure", "Early sidecar exit should preserve its safe diagnostic.")
        }
    }

    private func stdoutAboveSixteenMiBReturnsResponseTooLarge() async throws {
        let fake = try StaticServiceScript(mode: "stdout-too-large")
        defer { fake.cleanup() }

        do {
            _ = try await fake.run(input: Data(), timeoutNanoseconds: 5_000_000_000)
            throw NativeModelTestFailure(description: "Oversized stdout should be rejected before decoding.")
        } catch ServiceClient.ClientError.responseTooLarge(let maxBytes) {
            try expectEqual(maxBytes, 16 * 1_024 * 1_024, "Oversized stdout should report the fixed response bound.")
        }
    }

    private func stdoutBoundaryIsExact() async throws {
        let atLimit = try StaticServiceScript(mode: "stdout-at-limit")
        defer { atLimit.cleanup() }
        let accepted = try await atLimit.run(input: Data(), timeoutNanoseconds: 5_000_000_000)
        try expectEqual(accepted.count, 16 * 1_024 * 1_024, "Exactly 16 MiB stdout should be retained.")

        let overLimit = try StaticServiceScript(mode: "stdout-over-limit-by-one")
        defer { overLimit.cleanup() }
        do {
            _ = try await overLimit.run(input: Data(), timeoutNanoseconds: 5_000_000_000)
            throw NativeModelTestFailure(description: "One byte above the stdout limit should fail.")
        } catch ServiceClient.ClientError.responseTooLarge(let maxBytes) {
            try expectEqual(maxBytes, 16 * 1_024 * 1_024, "The one-byte overflow should report the fixed response bound.")
        }
    }

    private func oversizedFailingStderrIsBoundedAndRedacted() async throws {
        let sentinel = "SENSITIVE" + "_SENTINEL_42"
        let diagnostic = [
            "TOKEN" + "=" + sentinel,
            "/" + "Users/fixture/private-config.json",
            "sk-" + String(repeating: "a", count: 24)
        ].joined(separator: " ")
        let fake = try StaticServiceScript(mode: "stderr-too-large-failure", diagnostic: diagnostic)
        defer { fake.cleanup() }

        do {
            _ = try await fake.run(input: Data(), timeoutNanoseconds: 5_000_000_000)
            throw NativeModelTestFailure(description: "Nonzero service exit should fail.")
        } catch ServiceClient.ClientError.processFailed(let status, let displayDiagnostic) {
            try expectEqual(status, 7, "Bounded stderr should preserve exit status.")
            try expectFalse(displayDiagnostic.count > 512, "Displayed stderr should be at most 512 characters.")
            try expectFalse(displayDiagnostic.contains(sentinel), "Displayed stderr should redact secret values.")
            try expectFalse(displayDiagnostic.contains("private-config.json"), "Displayed stderr should redact local paths.")
            try expectFalse(displayDiagnostic.contains("sk-"), "Displayed stderr should redact provider token shapes.")
        }
    }

    private func diagnosticSanitizerCoversStandalonePatternsAndFallback() throws {
        let sentinel = "SENSITIVE" + "_SENTINEL_42"
        let raw = [
            "prefix api_key=\(sentinel)",
            "token " + "sk-" + String(repeating: "b", count: 24),
            "user /" + "Users/fixture/private-user.conf",
            "path /" + "home/fixture/private.conf",
            "temp /" + "private/" + "var/folders/example",
            "cache /" + "var/folders/example"
        ].joined(separator: "\n")
        let sanitized = ServiceDiagnosticSanitizer.displayMessage(raw)
        try expectFalse(sanitized.contains(sentinel), "Standalone API key shapes should be redacted.")
        try expectFalse(sanitized.contains("sk-"), "Standalone provider token shapes should be redacted.")
        try expectFalse(sanitized.contains("private.conf"), "Standalone home paths should be redacted.")
        try expectFalse(sanitized.contains("private-user.conf"), "Standalone user paths should be redacted.")
        try expectFalse(sanitized.contains("folders/example"), "Standalone private paths should be redacted.")
        try expectFalse(sanitized.contains("\n"), "Displayed diagnostics should collapse whitespace.")
        for key in ["API" + "_KEY", "TO" + "KEN"] {
            for whitespace in [" ", "\t", "\n"] {
                let prefixed = "error: " + key + "=" + whitespace + sentinel
                try expectFalse(
                    ServiceDiagnosticSanitizer.displayMessage(prefixed).contains(sentinel),
                    "Prefixed credential values should redact optional whitespace after equals."
                )
            }
        }
        try expectFalse(
            ServiceDiagnosticSanitizer.displayMessage(" \n\t ").isEmpty,
            "Empty diagnostics should use a stable fallback."
        )
    }

    private func malformedStdoutNeverAppearsInDisplayError() async throws {
        let sentinel = "SENSITIVE" + "_SENTINEL_42"
        let fake = try StaticServiceScript(
            mode: "malformed-sensitive",
            diagnostic: "TOKEN" + "=" + sentinel
        )
        defer { fake.cleanup() }

        do {
            _ = try await fake.serviceClient().status()
            throw NativeModelTestFailure(description: "Malformed service output should fail.")
        } catch ServiceClient.ClientError.invalidOutput(let message) {
            try expectContains(message, "response bytes", "Malformed output should report only bounded metadata.")
            try expectFalse(message.contains(sentinel), "Malformed stdout must not enter a display error.")
            try expectFalse(message.contains("TOKEN"), "Malformed stdout keys must not enter a display error.")
        }
    }

    private func invalidEnvelopeNeverAppearsInDisplayError() async throws {
        let sentinel = "SENSITIVE" + "_SENTINEL_42"
        let fake = try StaticServiceScript(
            mode: "invalid-envelope-sensitive",
            diagnostic: "TOKEN" + "=" + sentinel
        )
        defer { fake.cleanup() }

        do {
            _ = try await fake.serviceClient().status()
            throw NativeModelTestFailure(description: "An envelope without a result or error should fail.")
        } catch ServiceClient.ClientError.invalidOutput(let message) {
            try expectContains(message, "response bytes", "Invalid envelopes should report only bounded metadata.")
            try expectFalse(message.contains(sentinel), "Valid but unusable stdout must not enter a display error.")
        }
    }

    private func cancellationWhileDrainingReapsProcess() async throws {
        let fake = try CancellableServiceScript(emitsOutput: true)
        defer { fake.cleanup() }

        let call = Task {
            try await fake.serviceClient().status()
        }
        let pid = try await fake.waitForPID()
        call.cancel()
        do {
            _ = try await call.value
            throw NativeModelTestFailure(description: "Cancellation while draining should not return a result.")
        } catch is CancellationError {
            // Expected.
        }
        try await waitUntil("Cancelled streaming sidecar should be reaped.", timeout: 4) {
            !processExists(pid)
        }
    }

    private func callCancelledBeforeRunnerStartStillCompletes() async throws {
        let gate = ServiceRunnerStartGate()
        let outcome = ServiceRunnerCancellationOutcome()
        let fake = try StaticServiceScript(mode: "empty")
        defer { fake.cleanup() }

        let call = Task {
            await gate.wait()
            do {
                _ = try await fake.run(input: Data(), timeoutNanoseconds: 5_000_000_000)
                await outcome.record("returned")
            } catch is CancellationError {
                await outcome.record("cancelled")
            } catch {
                await outcome.record("error")
            }
        }
        try await waitUntil("The pre-cancel gate should be reached.") {
            await gate.isWaiting
        }
        call.cancel()
        await gate.open()
        try await waitUntil("An already-cancelled runner call must resume its continuation.", timeout: 1) {
            await outcome.value != nil
        }
        try expectEqual(await outcome.value, "cancelled", "Pre-cancelled runner calls should complete as cancellation.")
    }

    private func configuredTimeoutRejectsOverflowAndInvalidValues() throws {
        try expectEqual(StdioServiceProcessRunner.configuredTimeoutNanoseconds(environment: [:]), 30_000_000_000, "Missing timeout should use 30 seconds.")
        try expectEqual(StdioServiceProcessRunner.configuredTimeoutNanoseconds(environment: ["SKILLS_COPILOT_SERVICE_TIMEOUT_MS": "1"]), 50_000_000, "Timeout should clamp to 50 ms.")
        try expectEqual(StdioServiceProcessRunner.configuredTimeoutNanoseconds(environment: ["SKILLS_COPILOT_SERVICE_TIMEOUT_MS": "300000"]), 300_000_000_000, "Five minutes should be accepted.")
        try expectEqual(StdioServiceProcessRunner.configuredTimeoutNanoseconds(environment: ["SKILLS_COPILOT_SERVICE_TIMEOUT_MS": "300001"]), 30_000_000_000, "Out-of-range timeout should use the default.")
        try expectEqual(StdioServiceProcessRunner.configuredTimeoutNanoseconds(environment: ["SKILLS_COPILOT_SERVICE_TIMEOUT_MS": String(UInt64.max)]), 30_000_000_000, "Overflow should use the default.")
        try expectEqual(StdioServiceProcessRunner.configuredTimeoutNanoseconds(environment: ["SKILLS_COPILOT_SERVICE_TIMEOUT_MS": "not-a-number"]), 30_000_000_000, "Invalid timeout should use the default.")
    }

    private func malformedOutputMapsToInvalidOutput() async throws {
        let fake = try StaticServiceScript(mode: "malformed")
        defer { fake.cleanup() }
        fake.activate()

        do {
            _ = try await fake.serviceClient().status()
            throw NativeModelTestFailure(description: "Malformed service output should fail.")
        } catch ServiceClient.ClientError.invalidOutput(let output) {
            try expectContains(output, "decode failed", "Malformed output should include decode context.")
            try expectFalse(output.contains("not-json"), "Malformed output should not include a raw output snippet.")
        }
    }

    private func emptyOutputMapsToInvalidOutput() async throws {
        let fake = try StaticServiceScript(mode: "empty")
        defer { fake.cleanup() }
        fake.activate()

        do {
            _ = try await fake.serviceClient().status()
            throw NativeModelTestFailure(description: "Empty service output should fail.")
        } catch ServiceClient.ClientError.invalidOutput(let output) {
            try expectContains(output, "decode failed", "Empty output should include decode context.")
        }
    }

    private func truncatedOutputMapsToInvalidOutput() async throws {
        let fake = try StaticServiceScript(mode: "truncated")
        defer { fake.cleanup() }
        fake.activate()

        do {
            _ = try await fake.serviceClient().status()
            throw NativeModelTestFailure(description: "Truncated service output should fail.")
        } catch ServiceClient.ClientError.invalidOutput(let output) {
            try expectContains(output, "decode failed", "Truncated output should include decode context.")
            try expectFalse(output.contains("\"result\":"), "Truncated output should not include a raw output snippet.")
        }
    }


    private func stderrOnlyFailureMapsToProcessFailed() async throws {
        let fake = try StaticServiceScript(mode: "failure")
        defer { fake.cleanup() }
        fake.activate()

        do {
            _ = try await fake.serviceClient().status()
            throw NativeModelTestFailure(description: "Nonzero service exit should fail.")
        } catch ServiceClient.ClientError.processFailed(let status, let stderr) {
            try expectEqual(status, 7, "Process failure should preserve exit status.")
            try expectContains(stderr, "sidecar failed", "Process failure should preserve stderr.")
        }
    }

    private func waitUntil(_ label: String, timeout: TimeInterval = 2, predicate: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while !predicate() {
            if Date() > deadline {
                throw NativeModelTestFailure(description: label)
            }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
    }

    private func waitUntil(_ label: String, timeout: TimeInterval = 2, predicate: () async -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while !(await predicate()) {
            if Date() > deadline {
                throw NativeModelTestFailure(description: label)
            }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
    }

    private func processExists(_ pid: pid_t) -> Bool {
        if kill(pid, 0) == 0 {
            return true
        }
        return errno == EPERM
    }
}

private actor ServiceRunnerStartGate {
    private var continuation: CheckedContinuation<Void, Never>?
    private(set) var isWaiting = false

    func wait() async {
        isWaiting = true
        await withCheckedContinuation { continuation in
            self.continuation = continuation
        }
    }

    func open() {
        let continuation = continuation
        self.continuation = nil
        continuation?.resume()
    }
}

private actor ServiceRunnerCancellationOutcome {
    private(set) var value: String?

    func record(_ value: String) {
        guard self.value == nil else { return }
        self.value = value
    }
}

private final class StaticServiceScript {
    private let directory: URL
    private let executableURL: URL
    private let mode: String
    private let diagnostic: String

    init(mode: String, diagnostic: String = "") throws {
        self.mode = mode
        self.diagnostic = diagnostic
        directory = URL(fileURLWithPath: NSTemporaryDirectory())
            .appendingPathComponent("skills-copilot-static-service-\(UUID().uuidString)", isDirectory: true)
        executableURL = directory.appendingPathComponent("fake-static-service.sh")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try script.write(to: executableURL, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes(
            [.posixPermissions: NSNumber(value: Int16(0o755))],
            ofItemAtPath: executableURL.path
        )
    }

    func activate() {
        // Kept for test readability; ServiceClient injection carries the fake sidecar state.
    }

    func cleanup() {
        try? FileManager.default.removeItem(at: directory)
    }

    func serviceClient(timeoutNanoseconds: UInt64 = 30_000_000_000) -> ServiceClient {
        ServiceClient(
            processRunner: StdioServiceProcessRunner(
                timeoutNanoseconds: timeoutNanoseconds,
                environmentOverrides: environment
            ),
            serviceURL: executableURL
        )
    }

    func run(input: Data, timeoutNanoseconds: UInt64) async throws -> Data {
        try await StdioServiceProcessRunner(environmentOverrides: environment).run(
            executableURL: executableURL,
            input: input,
            timeoutNanoseconds: timeoutNanoseconds
        )
    }

    private var environment: [String: String] {
        [
            "SKILLS_COPILOT_STATIC_SERVICE_MODE": mode,
            "SKILLS_COPILOT_STATIC_SERVICE_DIAGNOSTIC": diagnostic
        ]
    }

    private var script: String {
        """
        #!/bin/sh
        write_mebibytes() {
          dd if=/dev/zero bs=1048576 count="$1" 2>/dev/null | tr '\\000' "$2"
        }
        status_prefix='{"id":"test","ok":true,"result":{"protocol_version":1,"version":"test","app_data_dir":"/tmp/app","catalog_path":"/tmp/catalog","user_home":"/tmp/home","supported_methods":[]'
        status_suffix='}}'
        case "${SKILLS_COPILOT_STATIC_SERVICE_MODE:-malformed}" in
          malformed)
            cat >/dev/null
            printf 'not-json'
            exit 0
            ;;
          empty)
            cat >/dev/null
            exit 0
            ;;
          failure)
            cat >/dev/null
            printf 'sidecar failed' >&2
            exit 7
            ;;
          truncated)
            cat >/dev/null
            printf '{"id":"test","ok":true,"result":'
            exit 0
            ;;
          stderr-before-stdout)
            cat >/dev/null
            write_mebibytes 2 E >&2
            printf '%s%s' "$status_prefix" "$status_suffix"
            ;;
          dual-large)
            cat >/dev/null
            (write_mebibytes 2 E >&2) &
            stderr_pid=$!
            printf '%s,"padding":"' "$status_prefix"
            write_mebibytes 2 O
            printf '"%s' "$status_suffix"
            wait "$stderr_pid"
            ;;
          large-input-early-output)
            printf '%s,"padding":"' "$status_prefix"
            write_mebibytes 2 O
            printf '"%s' "$status_suffix"
            cat >/dev/null
            ;;
          early-exit-without-stdin)
            printf 'early sidecar failure' >&2
            exit 7
            ;;
          stdout-too-large)
            cat >/dev/null
            write_mebibytes 17 O
            ;;
          stdout-at-limit)
            cat >/dev/null
            write_mebibytes 16 O
            ;;
          stdout-over-limit-by-one)
            cat >/dev/null
            write_mebibytes 16 O
            printf 'X'
            ;;
          stderr-too-large-failure)
            cat >/dev/null
            printf '%s\\n' "$SKILLS_COPILOT_STATIC_SERVICE_DIAGNOSTIC" >&2
            write_mebibytes 2 E >&2
            exit 7
            ;;
          malformed-sensitive)
            cat >/dev/null
            printf 'not-json %s' "$SKILLS_COPILOT_STATIC_SERVICE_DIAGNOSTIC"
            ;;
          invalid-envelope-sensitive)
            cat >/dev/null
            printf '{"id":"test","ok":true,"diagnostic":"%s"}' "$SKILLS_COPILOT_STATIC_SERVICE_DIAGNOSTIC"
            ;;
          *)
            cat >/dev/null
            printf 'not-json'
            exit 0
            ;;
        esac
        """
    }
}

private final class CancellableServiceScript {
    private let directory: URL
    private let executableURL: URL
    private let callsURL: URL
    private let pidURL: URL
    private let ignoresTermination: Bool
    private let emitsOutput: Bool

    init(ignoresTermination: Bool = false, emitsOutput: Bool = false) throws {
        self.ignoresTermination = ignoresTermination
        self.emitsOutput = emitsOutput
        directory = URL(fileURLWithPath: NSTemporaryDirectory())
            .appendingPathComponent("skills-copilot-cancellable-service-\(UUID().uuidString)", isDirectory: true)
        executableURL = directory.appendingPathComponent("fake-cancellable-service.sh")
        callsURL = directory.appendingPathComponent("calls.log")
        pidURL = directory.appendingPathComponent("pid")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        FileManager.default.createFile(atPath: callsURL.path, contents: nil)
        try script.write(to: executableURL, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes(
            [.posixPermissions: NSNumber(value: Int16(0o755))],
            ofItemAtPath: executableURL.path
        )
    }

    func activate() {
        // Kept for test readability; ServiceClient injection carries the fake sidecar state.
    }

    func cleanup() {
        if let pid = try? currentPID(), kill(pid, 0) == 0 {
            kill(pid, SIGKILL)
        }
        try? FileManager.default.removeItem(at: directory)
    }

    func serviceClient(timeoutNanoseconds: UInt64 = 30_000_000_000) -> ServiceClient {
        ServiceClient(
            processRunner: StdioServiceProcessRunner(
                timeoutNanoseconds: timeoutNanoseconds,
                environmentOverrides: [
                    "SKILLS_COPILOT_CANCELLABLE_SERVICE_CALLS": callsURL.path,
                    "SKILLS_COPILOT_CANCELLABLE_SERVICE_PID": pidURL.path,
                    "SKILLS_COPILOT_CANCELLABLE_SERVICE_IGNORE_TERM": ignoresTermination ? "1" : "0",
                    "SKILLS_COPILOT_CANCELLABLE_SERVICE_EMIT_OUTPUT": emitsOutput ? "1" : "0"
                ]
            ),
            serviceURL: executableURL
        )
    }

    func calls() -> String {
        (try? String(contentsOf: callsURL, encoding: .utf8)) ?? ""
    }

    func waitForPID(timeout: TimeInterval = 2) async throws -> pid_t {
        let deadline = Date().addingTimeInterval(timeout)
        while true {
            if let pid = try? currentPID() {
                return pid
            }
            if Date() > deadline {
                throw NativeModelTestFailure(description: "Fake sidecar should publish its PID before cancellation.")
            }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
    }

    private func currentPID() throws -> pid_t {
        let raw = try String(contentsOf: pidURL, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)
        guard let value = Int32(raw) else {
            throw NativeModelTestFailure(description: "Fake sidecar PID should be numeric.")
        }
        return pid_t(value)
    }

    private var script: String {
        """
        #!/bin/sh
        input=$(cat)
        printf '%s\\n' "$input" >> "$SKILLS_COPILOT_CANCELLABLE_SERVICE_CALLS"
        printf '%s\\n' "$$" > "$SKILLS_COPILOT_CANCELLABLE_SERVICE_PID"
        if [ "${SKILLS_COPILOT_CANCELLABLE_SERVICE_EMIT_OUTPUT:-0}" = "1" ]; then
          while :; do
            printf 'streaming-stdout-data'
            printf 'streaming-stderr-data' >&2
          done
        fi
        if [ "${SKILLS_COPILOT_CANCELLABLE_SERVICE_IGNORE_TERM:-0}" = "1" ]; then
          trap '' TERM
          while :; do
            :
          done
        fi
        while :; do
          sleep 1
        done
        """
    }
}
