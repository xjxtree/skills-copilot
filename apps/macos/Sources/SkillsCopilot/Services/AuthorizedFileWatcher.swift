import CoreServices
import Foundation

struct FileSystemChangeSummary: Equatable, Sendable {
    let eventCount: Int
    let requiresDeepScan: Bool
}

protocol FileSystemWatching: AnyObject {
    @discardableResult
    func start(
        paths: [String],
        onChange: @escaping @Sendable (FileSystemChangeSummary) -> Void
    ) -> Bool

    func stop()
}

enum AuthorizedWatchRootSanitizer {
    static let maximumRootCount = 256

    static func sanitizedPaths(
        from paths: [String],
        fileManager: FileManager = .default
    ) -> [String] {
        var seen = Set<String>()
        var sanitized: [String] = []

        for rawPath in paths where sanitized.count < maximumRootCount {
            let components = rawPath.split(separator: "/", omittingEmptySubsequences: true)
            guard rawPath.hasPrefix("/"),
                  !components.contains(where: { $0 == "." || $0 == ".." }) else {
                continue
            }

            let path = "/" + components.joined(separator: "/")
            guard path != "/",
                  seen.insert(path).inserted,
                  isExistingDirectory(path, fileManager: fileManager),
                  !hasSymbolicLinkComponent(path, fileManager: fileManager) else {
                continue
            }
            sanitized.append(path)
        }

        return sanitized.sorted()
    }

    private static func isExistingDirectory(
        _ path: String,
        fileManager: FileManager
    ) -> Bool {
        var isDirectory = ObjCBool(false)
        return fileManager.fileExists(atPath: path, isDirectory: &isDirectory)
            && isDirectory.boolValue
    }

    private static func hasSymbolicLinkComponent(
        _ path: String,
        fileManager: FileManager
    ) -> Bool {
        var current = "/"
        for component in (path as NSString).pathComponents where component != "/" {
            current = (current as NSString).appendingPathComponent(component)
            do {
                let attributes = try fileManager.attributesOfItem(atPath: current)
                if attributes[.type] as? FileAttributeType == .typeSymbolicLink {
                    return true
                }
            } catch {
                return true
            }
        }
        return false
    }
}

private final class FSEventsCallbackBox: @unchecked Sendable {
    let onChange: @Sendable (FileSystemChangeSummary) -> Void

    init(onChange: @escaping @Sendable (FileSystemChangeSummary) -> Void) {
        self.onChange = onChange
    }
}

private let authorizedFileWatchCallback: FSEventStreamCallback = {
    _,
    clientCallBackInfo,
    eventCount,
    _,
    eventFlags,
    _
in
    guard let clientCallBackInfo else { return }
    let callbackBox = Unmanaged<FSEventsCallbackBox>
        .fromOpaque(clientCallBackInfo)
        .takeUnretainedValue()
    let deepScanFlags = FSEventStreamEventFlags(
        kFSEventStreamEventFlagMustScanSubDirs
            | kFSEventStreamEventFlagUserDropped
            | kFSEventStreamEventFlagKernelDropped
            | kFSEventStreamEventFlagEventIdsWrapped
            | kFSEventStreamEventFlagRootChanged
    )
    let requiresDeepScan = (0..<eventCount).contains { index in
        eventFlags[index] & deepScanFlags != 0
    }
    callbackBox.onChange(
        FileSystemChangeSummary(
            eventCount: eventCount,
            requiresDeepScan: requiresDeepScan
        )
    )
}

final class FSEventsFileSystemWatcher: FileSystemWatching, @unchecked Sendable {
    private let callbackQueue = DispatchQueue(
        label: "dev.agent-copilot.authorized-file-watcher",
        qos: .utility
    )
    private var stream: FSEventStreamRef?
    private var callbackBox: FSEventsCallbackBox?

    @discardableResult
    func start(
        paths: [String],
        onChange: @escaping @Sendable (FileSystemChangeSummary) -> Void
    ) -> Bool {
        stop()
        let paths = AuthorizedWatchRootSanitizer.sanitizedPaths(from: paths)
        guard !paths.isEmpty else { return false }

        let callbackBox = FSEventsCallbackBox(onChange: onChange)
        var context = FSEventStreamContext(
            version: 0,
            info: Unmanaged.passUnretained(callbackBox).toOpaque(),
            retain: nil,
            release: nil,
            copyDescription: nil
        )
        let flags = FSEventStreamCreateFlags(
            kFSEventStreamCreateFlagFileEvents
                | kFSEventStreamCreateFlagWatchRoot
                | kFSEventStreamCreateFlagNoDefer
        )
        guard let stream = FSEventStreamCreate(
            nil,
            authorizedFileWatchCallback,
            &context,
            paths as CFArray,
            FSEventStreamEventId(kFSEventStreamEventIdSinceNow),
            0.35,
            flags
        ) else {
            return false
        }

        self.callbackBox = callbackBox
        self.stream = stream
        FSEventStreamSetDispatchQueue(stream, callbackQueue)
        guard FSEventStreamStart(stream) else {
            stop()
            return false
        }
        return true
    }

    func stop() {
        guard let stream else {
            callbackBox = nil
            return
        }
        FSEventStreamStop(stream)
        FSEventStreamInvalidate(stream)
        FSEventStreamRelease(stream)
        self.stream = nil
        callbackQueue.sync {}
        callbackBox = nil
    }

    deinit {
        stop()
    }
}
