import Darwin
import Foundation

enum RaceTestFailure: Error {
    case assertion
}

func expect(_ condition: @autoclosure () -> Bool) throws {
    if !condition() {
        throw RaceTestFailure.assertion
    }
}

func temporaryDirectory() throws -> URL {
    let root = FileManager.default.temporaryDirectory
        .appendingPathComponent("smoke-fixture-native-race-\(UUID().uuidString)")
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    return root
}

func write(_ content: String, to url: URL) throws {
    try Data(content.utf8).write(to: url)
}

func expectSnapshotFailure(
    root: URL,
    observer: SnapshotObserver
) throws {
    do {
        _ = try snapshotBoundedPathIdentity(
            rootPath: root.path,
            bounds: SnapshotBounds(
                maxDepth: 8,
                maxEntries: 64,
                maxFileBytes: 1_024,
                maxTotalBytes: 4_096
            ),
            observer: observer
        )
        throw RaceTestFailure.assertion
    } catch RaceTestFailure.assertion {
        throw RaceTestFailure.assertion
    } catch {
        return
    }
}

func testFileReplacementSymlinkNeverReadsExternalContent() throws {
    let root = try temporaryDirectory()
    defer { try? FileManager.default.removeItem(at: root) }
    let config = root.appendingPathComponent("config")
    let outside = root.appendingPathComponent("external-secret")
    try FileManager.default.createDirectory(at: config, withIntermediateDirectories: true)
    try write("safe", to: config.appendingPathComponent("victim"))
    try write("EXTERNAL_SECRET_SHOULD_NOT_BE_READ", to: outside)

    var swapped = false
    var readPaths: [String] = []
    let observer = SnapshotObserver(
        afterEntryMetadata: { relativePath, kind in
            guard relativePath == "victim", kind == .file, !swapped else { return }
            swapped = true
            try FileManager.default.removeItem(at: config.appendingPathComponent("victim"))
            try FileManager.default.createSymbolicLink(
                at: config.appendingPathComponent("victim"),
                withDestinationURL: outside
            )
        },
        beforeDirectoryEnumeration: nil,
        beforeFileRead: { relativePath in readPaths.append(relativePath) }
    )

    try expectSnapshotFailure(root: config, observer: observer)
    try expect(swapped)
    try expect(!readPaths.contains("victim"))
}

func testFileReplacementFIFONeverBlocksOrReads() throws {
    let root = try temporaryDirectory()
    defer { try? FileManager.default.removeItem(at: root) }
    let config = root.appendingPathComponent("config")
    let victim = config.appendingPathComponent("victim")
    try FileManager.default.createDirectory(at: config, withIntermediateDirectories: true)
    try write("safe", to: victim)

    var swapped = false
    var readPaths: [String] = []
    let observer = SnapshotObserver(
        afterEntryMetadata: { relativePath, kind in
            guard relativePath == "victim", kind == .file, !swapped else { return }
            swapped = true
            try FileManager.default.removeItem(at: victim)
            guard mkfifo(victim.path, S_IRUSR | S_IWUSR) == 0 else {
                throw RaceTestFailure.assertion
            }
        },
        beforeDirectoryEnumeration: nil,
        beforeFileRead: { relativePath in readPaths.append(relativePath) }
    )

    try expectSnapshotFailure(root: config, observer: observer)
    try expect(swapped)
    try expect(!readPaths.contains("victim"))
}

func testDirectoryReplacementNeverEnumeratesExternalChildren() throws {
    let root = try temporaryDirectory()
    defer { try? FileManager.default.removeItem(at: root) }
    let config = root.appendingPathComponent("config")
    let victim = config.appendingPathComponent("victim")
    let savedVictim = root.appendingPathComponent("saved-victim")
    let outside = root.appendingPathComponent("external-directory")
    try FileManager.default.createDirectory(at: victim, withIntermediateDirectories: true)
    try FileManager.default.createDirectory(at: outside, withIntermediateDirectories: true)
    try write("inside", to: victim.appendingPathComponent("inside-child"))
    try write("external", to: outside.appendingPathComponent("external-child"))

    var swapped = false
    var metadataPaths: [String] = []
    var enumeratedDirectories: [String] = []
    let observer = SnapshotObserver(
        afterEntryMetadata: { relativePath, kind in
            metadataPaths.append(relativePath)
            guard relativePath == "victim", kind == .directory, !swapped else { return }
            swapped = true
            try FileManager.default.moveItem(at: victim, to: savedVictim)
            try FileManager.default.createSymbolicLink(at: victim, withDestinationURL: outside)
        },
        beforeDirectoryEnumeration: { relativePath in
            enumeratedDirectories.append(relativePath)
        },
        beforeFileRead: nil
    )

    try expectSnapshotFailure(root: config, observer: observer)
    try expect(swapped)
    try expect(!enumeratedDirectories.contains("victim"))
    try expect(!metadataPaths.contains("victim/external-child"))
}

do {
    try testFileReplacementSymlinkNeverReadsExternalContent()
    try testFileReplacementFIFONeverBlocksOrReads()
    try testDirectoryReplacementNeverEnumeratesExternalChildren()
    print("native fixture identity race tests passed")
} catch {
    fputs("native fixture identity race tests failed\n", stderr)
    exit(1)
}
