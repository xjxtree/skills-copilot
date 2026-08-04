import Darwin
import Foundation

enum TaskCockpitHistoryPurgeOutcome: Equatable {
    case noFile
    case removed
}

private enum TaskCockpitHistoryPurgeError: Error {
    case couldNotRemoveItem
}

struct TaskCockpitHistoryStore {
    enum UnlinkResult: Equatable {
        case removed
        case failed(errorNumber: Int32)
    }

    let fileURL: URL
    private let unlinkItem: (String) -> UnlinkResult

    init(
        fileURL: URL = TaskCockpitHistoryStore.defaultFileURL,
        unlinkItem: @escaping (String) -> UnlinkResult = TaskCockpitHistoryStore.unlinkHistoryItem(atPath:)
    ) {
        self.fileURL = fileURL
        self.unlinkItem = unlinkItem
    }

    func purgeLegacyHistoryIfPresent() throws -> TaskCockpitHistoryPurgeOutcome {
        switch unlinkItem(fileURL.path) {
        case .removed:
            return .removed
        case .failed(let errorNumber) where errorNumber == ENOENT:
            return .noFile
        case .failed:
            throw TaskCockpitHistoryPurgeError.couldNotRemoveItem
        }
    }

    static func unlinkHistoryItem(atPath path: String) -> UnlinkResult {
        path.withCString { fileSystemPath in
            if Darwin.unlink(fileSystemPath) == 0 {
                return .removed
            }
            return .failed(errorNumber: errno)
        }
    }

    static var defaultFileURL: URL {
        appDataURL.appendingPathComponent("task-preflight-history.json", isDirectory: false)
    }

    private static var appDataURL: URL {
        let environment = ProcessInfo.processInfo.environment
        if let override = environment["SKILLS_COPILOT_APP_DATA_DIR"], !override.isEmpty {
            return URL(fileURLWithPath: override, isDirectory: true).standardizedFileURL
        }
        return FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library", isDirectory: true)
            .appendingPathComponent("Application Support", isDirectory: true)
            .appendingPathComponent("dev.agent-copilot.native", isDirectory: true)
            .standardizedFileURL
    }
}
