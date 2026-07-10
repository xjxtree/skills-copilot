import Foundation

let arguments = Array(CommandLine.arguments.dropFirst())
guard arguments.count == 5,
      let maxDepth = Int(arguments[1]),
      let maxEntries = Int(arguments[2]),
      let maxFileBytes = Int(arguments[3]),
      let maxTotalBytes = Int(arguments[4])
else {
    fputs("snapshot helper arguments invalid\n", stderr)
    exit(2)
}

do {
    let result = try snapshotBoundedPathIdentity(
        rootPath: arguments[0],
        bounds: SnapshotBounds(
            maxDepth: maxDepth,
            maxEntries: maxEntries,
            maxFileBytes: maxFileBytes,
            maxTotalBytes: maxTotalBytes
        )
    )
    let data = try JSONEncoder().encode(result)
    FileHandle.standardOutput.write(data)
    FileHandle.standardOutput.write(Data("\n".utf8))
} catch SnapshotFailure.bounded {
    fputs("snapshot helper bound exceeded\n", stderr)
    exit(3)
} catch {
    fputs("snapshot helper failed\n", stderr)
    exit(1)
}
