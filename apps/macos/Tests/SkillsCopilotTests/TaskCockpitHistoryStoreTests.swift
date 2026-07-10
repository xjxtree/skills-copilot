import Darwin
import Foundation
@testable import SkillsCopilot

struct TaskCockpitHistoryStoreTests {
    private static let sensitiveSentinel = "SENSITIVE_SENTINEL_42"

    func run() throws {
        try missingHistoryFileNeedsNoCleanup()
        try legacyArrayIsDeleted()
        try versionOneEnvelopeIsDeleted()
        try versionTwoEnvelopeIsDeleted()
        try malformedHistoryFileIsDeleted()
        try symbolicLinkAtHistoryPathIsDeletedWithoutDeletingTarget()
        try danglingSymbolicLinkAtHistoryPathIsDeleted()
        try directoryAtHistoryPathIsNotDeleted()
        try directoryReplacementImmediatelyBeforeUnlinkIsNotRecursivelyDeleted()
        try purgeFailureDoesNotRenameOrCopySensitiveFile()
    }

    private func missingHistoryFileNeedsNoCleanup() throws {
        try withTemporaryHistoryStore { historyStore in
            let outcome = try historyStore.purgeLegacyHistoryIfPresent()

            try expectEqual(outcome, .noFile, "A missing legacy history file should need no cleanup.")
            try expectEqual(
                FileManager.default.fileExists(atPath: historyStore.fileURL.path),
                false,
                "Cleanup must not create a Task Preflight history file."
            )
        }
    }

    private func legacyArrayIsDeleted() throws {
        try assertFixtureIsDeleted("[\(sensitiveRecordFixture)]")
    }

    private func versionOneEnvelopeIsDeleted() throws {
        try assertFixtureIsDeleted("{\"version\":1,\"records\":[\(sensitiveRecordFixture)]}")
    }

    private func versionTwoEnvelopeIsDeleted() throws {
        try assertFixtureIsDeleted("{\"version\":2,\"records\":[\(sensitiveRecordFixture)]}")
    }

    private func malformedHistoryFileIsDeleted() throws {
        try assertFixtureIsDeleted(
            """
            taskText=\(Self.sensitiveSentinel)
            operationState=\(Self.sensitiveSentinel)
            filters=\(Self.sensitiveSentinel)
            summary=\(Self.sensitiveSentinel)
            resultText=\(Self.sensitiveSentinel)
            """
        )
    }

    private func symbolicLinkAtHistoryPathIsDeletedWithoutDeletingTarget() throws {
        try withTemporaryHistoryStore { historyStore in
            let targetURL = historyStore.fileURL
                .deletingLastPathComponent()
                .appendingPathComponent("legacy-history-target.json", isDirectory: false)
            let fixture = "[\(sensitiveRecordFixture)]"
            try Data(fixture.utf8).write(to: targetURL)
            try FileManager.default.createSymbolicLink(
                at: historyStore.fileURL,
                withDestinationURL: targetURL
            )

            let outcome = try historyStore.purgeLegacyHistoryIfPresent()

            try expectEqual(outcome, .removed, "A legacy history symlink should be removed.")
            try expectEqual(
                FileManager.default.fileExists(atPath: historyStore.fileURL.path),
                false,
                "Cleanup should remove only the history symlink."
            )
            try expectEqual(
                try String(contentsOf: targetURL, encoding: .utf8),
                fixture,
                "Cleanup must not follow a history symlink and delete its target."
            )
        }
    }

    private func danglingSymbolicLinkAtHistoryPathIsDeleted() throws {
        try withTemporaryHistoryStore { historyStore in
            let missingTargetURL = historyStore.fileURL
                .deletingLastPathComponent()
                .appendingPathComponent("missing-history-target.json", isDirectory: false)
            try FileManager.default.createSymbolicLink(
                at: historyStore.fileURL,
                withDestinationURL: missingTargetURL
            )

            let outcome = try historyStore.purgeLegacyHistoryIfPresent()

            try expectEqual(outcome, .removed, "A dangling legacy history symlink should be removed.")
            do {
                _ = try FileManager.default.destinationOfSymbolicLink(atPath: historyStore.fileURL.path)
                throw NativeModelTestFailure(
                    description: "Cleanup should remove a dangling history symlink."
                )
            } catch is NativeModelTestFailure {
                throw NativeModelTestFailure(
                    description: "Cleanup should remove a dangling history symlink."
                )
            } catch {
                // The symlink no longer exists, which is the intended cleanup result.
            }
        }
    }

    private func directoryAtHistoryPathIsNotDeleted() throws {
        try withTemporaryHistoryStore { historyStore in
            try FileManager.default.createDirectory(
                at: historyStore.fileURL,
                withIntermediateDirectories: false
            )
            let nestedSentinelURL = historyStore.fileURL
                .appendingPathComponent("SENSITIVE_SENTINEL_42.txt", isDirectory: false)
            try Data(Self.sensitiveSentinel.utf8).write(to: nestedSentinelURL)

            do {
                _ = try historyStore.purgeLegacyHistoryIfPresent()
                throw NativeModelTestFailure(
                    description: "A directory at the legacy history filename must be rejected."
                )
            } catch is NativeModelTestFailure {
                throw NativeModelTestFailure(
                    description: "A directory at the legacy history filename must be rejected."
                )
            } catch {
                try expectFalse(
                    String(describing: error).contains(historyStore.fileURL.path),
                    "A rejected history item must not expose its local path in the cleanup error."
                )
            }

            var isDirectory: ObjCBool = false
            try expectEqual(
                FileManager.default.fileExists(
                    atPath: historyStore.fileURL.path,
                    isDirectory: &isDirectory
                ),
                true,
                "Cleanup must leave an unexpected directory at the history filename untouched."
            )
            try expectEqual(
                isDirectory.boolValue,
                true,
                "The unexpected history-path item should remain a directory."
            )
            try expectEqual(
                try String(contentsOf: nestedSentinelURL, encoding: .utf8),
                Self.sensitiveSentinel,
                "Cleanup must not recursively delete or mutate directory contents."
            )
        }
    }

    private func directoryReplacementImmediatelyBeforeUnlinkIsNotRecursivelyDeleted() throws {
        try withTemporaryHistoryStore { historyStore in
            try Data("legacy file".utf8).write(to: historyStore.fileURL)
            let nestedSentinelURL = historyStore.fileURL
                .appendingPathComponent("SENSITIVE_SENTINEL_42.txt", isDirectory: false)
            let racingStore = TaskCockpitHistoryStore(
                fileURL: historyStore.fileURL,
                unlinkItem: { path in
                    let url = URL(fileURLWithPath: path, isDirectory: false)
                    do {
                        try FileManager.default.removeItem(at: url)
                        try FileManager.default.createDirectory(
                            at: url,
                            withIntermediateDirectories: false
                        )
                        try Data(Self.sensitiveSentinel.utf8).write(to: nestedSentinelURL)
                    } catch {
                        return .failed(errorNumber: EIO)
                    }
                    return TaskCockpitHistoryStore.unlinkHistoryItem(atPath: path)
                }
            )

            let didThrow: Bool
            do {
                _ = try racingStore.purgeLegacyHistoryIfPresent()
                didThrow = false
            } catch {
                didThrow = true
            }

            try expectEqual(
                didThrow,
                true,
                "A directory swapped in immediately before removal must be rejected."
            )
            var isDirectory: ObjCBool = false
            try expectEqual(
                FileManager.default.fileExists(
                    atPath: historyStore.fileURL.path,
                    isDirectory: &isDirectory
                ),
                true,
                "Atomic cleanup must leave a replacement directory untouched."
            )
            try expectEqual(
                isDirectory.boolValue,
                true,
                "The replacement history item should remain a directory."
            )
            try expectEqual(
                try String(contentsOf: nestedSentinelURL, encoding: .utf8),
                Self.sensitiveSentinel,
                "Atomic cleanup must not recursively delete replacement directory contents."
            )
        }
    }

    private func purgeFailureDoesNotRenameOrCopySensitiveFile() throws {
        try withTemporaryHistoryStore { historyStore in
            let fixture = "[\(sensitiveRecordFixture)]"
            try Data(fixture.utf8).write(to: historyStore.fileURL)
            let failingStore = TaskCockpitHistoryStore(
                fileURL: historyStore.fileURL,
                unlinkItem: { _ in .failed(errorNumber: EACCES) }
            )

            let thrownError: Error
            do {
                _ = try failingStore.purgeLegacyHistoryIfPresent()
                throw NativeModelTestFailure(description: "A deterministic removal failure should be thrown.")
            } catch is NativeModelTestFailure {
                throw NativeModelTestFailure(description: "A deterministic removal failure should be thrown.")
            } catch {
                thrownError = error
            }

            try expectEqual(
                FileManager.default.fileExists(atPath: historyStore.fileURL.path),
                true,
                "The original sensitive file should remain solely because deletion failed."
            )
            try expectEqual(
                try String(contentsOf: historyStore.fileURL, encoding: .utf8),
                fixture,
                "A failed purge must not mutate the original sensitive file."
            )
            let siblingNames = try FileManager.default.contentsOfDirectory(
                atPath: historyStore.fileURL.deletingLastPathComponent().path
            )
            try expectEqual(
                siblingNames.filter { $0.contains("task-preflight-history") },
                [historyStore.fileURL.lastPathComponent],
                "Cleanup failure must not retain a renamed or backup history file."
            )
            try expectFalse(
                String(describing: thrownError).contains(Self.sensitiveSentinel),
                "Cleanup errors must not contain file contents."
            )
        }
    }

    private func assertFixtureIsDeleted(_ fixture: String) throws {
        try withTemporaryHistoryStore { historyStore in
            try Data(fixture.utf8).write(to: historyStore.fileURL)

            let outcome = try historyStore.purgeLegacyHistoryIfPresent()

            try expectEqual(outcome, .removed, "An existing legacy history file should be removed.")
            try expectEqual(
                FileManager.default.fileExists(atPath: historyStore.fileURL.path),
                false,
                "Legacy Task Preflight history must be removed."
            )
            let siblingNames = try FileManager.default.contentsOfDirectory(
                atPath: historyStore.fileURL.deletingLastPathComponent().path
            )
            try expectFalse(
                siblingNames.contains { $0.contains("task-preflight-history") },
                "Cleanup must not retain a renamed or backup history file."
            )
        }
    }

    private func withTemporaryHistoryStore(
        _ body: (TaskCockpitHistoryStore) throws -> Void
    ) throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("skills-copilot-history-purge-tests-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }

        try body(TaskCockpitHistoryStore(
            fileURL: directory.appendingPathComponent("task-preflight-history.json", isDirectory: false)
        ))
    }

    private var sensitiveRecordFixture: String {
        """
        {
          "taskText":"\(Self.sensitiveSentinel) task text",
          "operationState":{"message":"\(Self.sensitiveSentinel) operation state"},
          "filters":{"taskText":"\(Self.sensitiveSentinel) filters"},
          "summary":{"summaryText":"\(Self.sensitiveSentinel) summary"},
          "result":{"resultText":"\(Self.sensitiveSentinel) result text"}
        }
        """
    }
}
