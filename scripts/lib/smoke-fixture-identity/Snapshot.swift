import CryptoKit
import Darwin
import Foundation

struct SnapshotBounds {
    let maxDepth: Int
    let maxEntries: Int
    let maxFileBytes: Int
    let maxTotalBytes: Int
}

struct SnapshotResult: Codable {
    let digest: String
    let entryCount: Int
    let exists: Bool
    let totalFileBytes: Int
}

enum SnapshotEntryKind: String {
    case blockDevice = "block-device"
    case characterDevice = "character-device"
    case directory
    case fifo
    case file
    case other
    case socket
    case symlink
}

struct SnapshotObserver {
    let afterEntryMetadata: ((String, SnapshotEntryKind) throws -> Void)?
    let beforeDirectoryEnumeration: ((String) throws -> Void)?
    let beforeFileRead: ((String) throws -> Void)?
    let beforeSymlinkRead: ((String) throws -> Void)?

    init(
        afterEntryMetadata: ((String, SnapshotEntryKind) throws -> Void)? = nil,
        beforeDirectoryEnumeration: ((String) throws -> Void)? = nil,
        beforeFileRead: ((String) throws -> Void)? = nil,
        beforeSymlinkRead: ((String) throws -> Void)? = nil
    ) {
        self.afterEntryMetadata = afterEntryMetadata
        self.beforeDirectoryEnumeration = beforeDirectoryEnumeration
        self.beforeFileRead = beforeFileRead
        self.beforeSymlinkRead = beforeSymlinkRead
    }
}

enum SnapshotFailure: Error {
    case bounded
    case filesystem
}

private final class SnapshotState {
    let bounds: SnapshotBounds
    var entryCount = 0
    var hasher = SHA256()
    var totalFileBytes = 0

    init(bounds: SnapshotBounds) {
        self.bounds = bounds
    }
}

private let directoryOpenFlags = O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC
private let fileOpenFlags = O_RDONLY | O_NOFOLLOW | O_NONBLOCK | O_CLOEXEC
// Darwin's O_SYMLINK atomically opens the link inode itself. O_NOFOLLOW cannot
// be combined with it: the documented result for a final symlink is ELOOP.
private let symlinkOpenFlags = O_SYMLINK | O_NONBLOCK | O_CLOEXEC
private let readBufferBytes = 64 * 1_024
private let maxSymlinkBytes = 64 * 1_024
private let slashByte = UInt8(ascii: "/")

private func fileType(_ mode: mode_t) -> mode_t {
    mode & mode_t(S_IFMT)
}

private func entryKind(_ mode: mode_t) -> SnapshotEntryKind {
    switch fileType(mode) {
    case S_IFDIR: return .directory
    case S_IFREG: return .file
    case S_IFLNK: return .symlink
    case S_IFBLK: return .blockDevice
    case S_IFCHR: return .characterDevice
    case S_IFIFO: return .fifo
    case S_IFSOCK: return .socket
    default: return .other
    }
}

private func sameIdentityAndType(_ expected: stat, _ opened: stat) -> Bool {
    expected.st_dev == opened.st_dev &&
        expected.st_ino == opened.st_ino &&
        fileType(expected.st_mode) == fileType(opened.st_mode)
}

private func framedUpdate(_ data: Data, state: SnapshotState) {
    var length = UInt64(data.count).bigEndian
    withUnsafeBytes(of: &length) { bytes in
        state.hasher.update(data: Data(bytes))
    }
    state.hasher.update(data: data)
}

private func framedUpdate(_ value: String, state: SnapshotState) {
    framedUpdate(Data(value.utf8), state: state)
}

private func framedUpdate(_ value: Int, state: SnapshotState) {
    var encoded = UInt64(value).bigEndian
    withUnsafeBytes(of: &encoded) { bytes in
        framedUpdate(Data(bytes), state: state)
    }
}

private func displayPath(_ relativePath: [UInt8]) -> String {
    String(decoding: relativePath, as: UTF8.self)
}

private func appendPath(_ parent: [UInt8], _ name: [UInt8]) -> [UInt8] {
    if parent == [UInt8(ascii: ".")] {
        return name
    }
    return parent + [slashByte] + name
}

private func withNameCString<Result>(
    _ name: [UInt8],
    _ body: (UnsafePointer<CChar>) throws -> Result
) rethrows -> Result {
    var terminated = name.map { CChar(bitPattern: $0) }
    terminated.append(0)
    return try terminated.withUnsafeBufferPointer { buffer in
        try body(buffer.baseAddress!)
    }
}

private func directoryNames(
    descriptor: Int32,
    state: SnapshotState
) throws -> [[UInt8]] {
    let duplicate = dup(descriptor)
    guard duplicate >= 0 else { throw SnapshotFailure.filesystem }
    guard let directory = fdopendir(duplicate) else {
        close(duplicate)
        throw SnapshotFailure.filesystem
    }
    defer { closedir(directory) }

    var names: [[UInt8]] = []
    errno = 0
    while let entry = readdir(directory) {
        let length = Int(entry.pointee.d_namlen)
        let bytes = withUnsafePointer(to: &entry.pointee.d_name) { pointer in
            pointer.withMemoryRebound(to: UInt8.self, capacity: length) { namePointer in
                Array(UnsafeBufferPointer(start: namePointer, count: length))
            }
        }
        if bytes == [UInt8(ascii: ".")] ||
            bytes == [UInt8(ascii: "."), UInt8(ascii: ".")]
        {
            continue
        }
        names.append(bytes)
        if state.entryCount + names.count > state.bounds.maxEntries {
            throw SnapshotFailure.bounded
        }
    }
    guard errno == 0 else { throw SnapshotFailure.filesystem }
    return names.sorted { $0.lexicographicallyPrecedes($1) }
}

private func readSymlink(descriptor: Int32) throws -> Data {
    var capacity = 256
    while capacity <= maxSymlinkBytes {
        var buffer = [UInt8](repeating: 0, count: capacity)
        let count = buffer.withUnsafeMutableBytes { bytes in
            freadlink(descriptor, bytes.baseAddress, capacity)
        }
        guard count >= 0 else { throw SnapshotFailure.filesystem }
        if count < capacity {
            return Data(buffer.prefix(count))
        }
        capacity *= 2
    }
    throw SnapshotFailure.bounded
}

private func hashFile(
    descriptor: Int32,
    relativePath: [UInt8],
    state: SnapshotState,
    observer: SnapshotObserver
) throws {
    try observer.beforeFileRead?(displayPath(relativePath))
    var fileHasher = SHA256()
    var fileBytes = 0
    var buffer = [UInt8](repeating: 0, count: readBufferBytes)

    while true {
        let remainingFileBytes = state.bounds.maxFileBytes - fileBytes
        let remainingTotalBytes = state.bounds.maxTotalBytes - state.totalFileBytes
        let bytesToRead = min(
            readBufferBytes,
            remainingFileBytes + 1,
            remainingTotalBytes + 1
        )
        let count = buffer.withUnsafeMutableBytes { bytes in
            Darwin.read(descriptor, bytes.baseAddress, bytesToRead)
        }
        if count == 0 {
            break
        }
        if count < 0 {
            if errno == EINTR { continue }
            throw SnapshotFailure.filesystem
        }
        fileBytes += count
        state.totalFileBytes += count
        if fileBytes > state.bounds.maxFileBytes ||
            state.totalFileBytes > state.bounds.maxTotalBytes
        {
            throw SnapshotFailure.bounded
        }
        fileHasher.update(data: Data(buffer.prefix(count)))
    }

    framedUpdate(fileBytes, state: state)
    framedUpdate(Data(fileHasher.finalize()), state: state)
}

private func recordEntry(
    relativePath: [UInt8],
    kind: SnapshotEntryKind,
    depth: Int,
    state: SnapshotState
) throws {
    if depth > state.bounds.maxDepth {
        throw SnapshotFailure.bounded
    }
    state.entryCount += 1
    if state.entryCount > state.bounds.maxEntries {
        throw SnapshotFailure.bounded
    }
    framedUpdate(Data(relativePath), state: state)
    framedUpdate(kind.rawValue, state: state)
}

private func hashChild(
    parentDescriptor: Int32,
    name: [UInt8],
    relativePath: [UInt8],
    depth: Int,
    state: SnapshotState,
    observer: SnapshotObserver
) throws {
    var expected = stat()
    let metadataStatus = withNameCString(name) { pointer in
        fstatat(parentDescriptor, pointer, &expected, AT_SYMLINK_NOFOLLOW)
    }
    guard metadataStatus == 0 else { throw SnapshotFailure.filesystem }
    let kind = entryKind(expected.st_mode)
    try observer.afterEntryMetadata?(displayPath(relativePath), kind)
    try recordEntry(relativePath: relativePath, kind: kind, depth: depth, state: state)

    switch kind {
    case .directory:
        let descriptor = withNameCString(name) { pointer in
            openat(parentDescriptor, pointer, directoryOpenFlags)
        }
        guard descriptor >= 0 else { throw SnapshotFailure.filesystem }
        defer { close(descriptor) }
        var opened = stat()
        guard fstat(descriptor, &opened) == 0,
              sameIdentityAndType(expected, opened),
              entryKind(opened.st_mode) == .directory
        else {
            throw SnapshotFailure.filesystem
        }
        try hashDirectoryContents(
            descriptor: descriptor,
            relativePath: relativePath,
            depth: depth,
            state: state,
            observer: observer
        )
    case .file:
        let descriptor = withNameCString(name) { pointer in
            openat(parentDescriptor, pointer, fileOpenFlags)
        }
        guard descriptor >= 0 else { throw SnapshotFailure.filesystem }
        defer { close(descriptor) }
        var opened = stat()
        guard fstat(descriptor, &opened) == 0,
              sameIdentityAndType(expected, opened),
              entryKind(opened.st_mode) == .file
        else {
            throw SnapshotFailure.filesystem
        }
        try hashFile(
            descriptor: descriptor,
            relativePath: relativePath,
            state: state,
            observer: observer
        )
    case .symlink:
        let descriptor = withNameCString(name) { pointer in
            openat(parentDescriptor, pointer, symlinkOpenFlags)
        }
        guard descriptor >= 0 else { throw SnapshotFailure.filesystem }
        defer { close(descriptor) }
        var opened = stat()
        guard fstat(descriptor, &opened) == 0,
              sameIdentityAndType(expected, opened),
              entryKind(opened.st_mode) == .symlink
        else {
            throw SnapshotFailure.filesystem
        }
        try observer.beforeSymlinkRead?(displayPath(relativePath))
        framedUpdate(try readSymlink(descriptor: descriptor), state: state)
    case .blockDevice, .characterDevice, .fifo, .other, .socket:
        break
    }
}

private func hashDirectoryContents(
    descriptor: Int32,
    relativePath: [UInt8],
    depth: Int,
    state: SnapshotState,
    observer: SnapshotObserver
) throws {
    try observer.beforeDirectoryEnumeration?(displayPath(relativePath))
    for name in try directoryNames(descriptor: descriptor, state: state) {
        try hashChild(
            parentDescriptor: descriptor,
            name: name,
            relativePath: appendPath(relativePath, name),
            depth: depth + 1,
            state: state,
            observer: observer
        )
    }
}

func snapshotBoundedPathIdentity(
    rootPath: String,
    bounds: SnapshotBounds,
    observer: SnapshotObserver = SnapshotObserver()
) throws -> SnapshotResult {
    var expected = stat()
    if lstat(rootPath, &expected) != 0 {
        if errno == ENOENT {
            let state = SnapshotState(bounds: bounds)
            framedUpdate("missing", state: state)
            return SnapshotResult(
                digest: state.hasher.finalize().map { String(format: "%02x", $0) }.joined(),
                entryCount: 0,
                exists: false,
                totalFileBytes: 0
            )
        }
        throw SnapshotFailure.filesystem
    }
    guard entryKind(expected.st_mode) == .directory else {
        throw SnapshotFailure.filesystem
    }

    let descriptor = open(rootPath, directoryOpenFlags)
    guard descriptor >= 0 else { throw SnapshotFailure.filesystem }
    defer { close(descriptor) }
    var opened = stat()
    guard fstat(descriptor, &opened) == 0,
          sameIdentityAndType(expected, opened),
          entryKind(opened.st_mode) == .directory
    else {
        throw SnapshotFailure.filesystem
    }

    let state = SnapshotState(bounds: bounds)
    let rootRelativePath = [UInt8(ascii: ".")]
    try recordEntry(
        relativePath: rootRelativePath,
        kind: .directory,
        depth: 0,
        state: state
    )
    try hashDirectoryContents(
        descriptor: descriptor,
        relativePath: rootRelativePath,
        depth: 0,
        state: state,
        observer: observer
    )
    let digest = state.hasher.finalize().map { String(format: "%02x", $0) }.joined()
    return SnapshotResult(
        digest: digest,
        entryCount: state.entryCount,
        exists: true,
        totalFileBytes: state.totalFileBytes
    )
}
