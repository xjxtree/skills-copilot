import { Buffer } from "node:buffer";
import { createHash } from "node:crypto";
import {
  closeSync,
  lstatSync,
  openSync,
  opendirSync,
  readlinkSync,
  readSync,
  rmSync,
} from "node:fs";
import { join, resolve } from "node:path";

const defaultSnapshotBounds = Object.freeze({
  maxDepth: 32,
  maxEntries: 8_192,
  maxFileBytes: 16 * 1_024 * 1_024,
  maxTotalBytes: 64 * 1_024 * 1_024,
});
const readBufferBytes = 64 * 1_024;

class SmokeFixtureSafetyError extends Error {
  constructor(message) {
    super(message);
    this.name = "SmokeFixtureSafetyError";
  }
}

function boundedSnapshotFailure() {
  return new SmokeFixtureSafetyError(
    "bounded real opencode config snapshot limit exceeded",
  );
}

function snapshotFailure() {
  return new SmokeFixtureSafetyError(
    "unable to capture bounded real opencode config snapshot",
  );
}

function updateFramed(hash, value) {
  const bytes = Buffer.isBuffer(value) ? value : Buffer.from(String(value));
  const length = Buffer.allocUnsafe(8);
  length.writeBigUInt64BE(BigInt(bytes.length));
  hash.update(length);
  hash.update(bytes);
}

function entryKind(stat) {
  if (stat.isDirectory()) return "directory";
  if (stat.isFile()) return "file";
  if (stat.isSymbolicLink()) return "symlink";
  if (stat.isBlockDevice()) return "block-device";
  if (stat.isCharacterDevice()) return "character-device";
  if (stat.isFIFO()) return "fifo";
  if (stat.isSocket()) return "socket";
  return "other";
}

function sortedDirectoryNames(path, state) {
  const directory = opendirSync(path);
  const names = [];
  try {
    for (let entry = directory.readSync(); entry; entry = directory.readSync()) {
      names.push(entry.name);
      if (state.entryCount + names.length > state.bounds.maxEntries) {
        throw boundedSnapshotFailure();
      }
    }
  } finally {
    directory.closeSync();
  }
  return names.sort((left, right) =>
    Buffer.compare(Buffer.from(left), Buffer.from(right)),
  );
}

function hashFile(path, hash, state) {
  const fileHash = createHash("sha256");
  const buffer = Buffer.allocUnsafe(readBufferBytes);
  const descriptor = openSync(path, "r");
  let fileBytes = 0;
  try {
    while (true) {
      const remainingFileBytes = state.bounds.maxFileBytes - fileBytes;
      const remainingTotalBytes =
        state.bounds.maxTotalBytes - state.totalFileBytes;
      const bytesToRead = Math.min(
        buffer.length,
        remainingFileBytes + 1,
        remainingTotalBytes + 1,
      );
      const bytesRead = readSync(
        descriptor,
        buffer,
        0,
        bytesToRead,
        null,
      );
      if (bytesRead === 0) {
        break;
      }
      fileBytes += bytesRead;
      state.totalFileBytes += bytesRead;
      if (
        fileBytes > state.bounds.maxFileBytes ||
        state.totalFileBytes > state.bounds.maxTotalBytes
      ) {
        throw boundedSnapshotFailure();
      }
      fileHash.update(buffer.subarray(0, bytesRead));
    }
  } finally {
    closeSync(descriptor);
  }
  updateFramed(hash, fileBytes);
  updateFramed(hash, fileHash.digest());
}

function hashEntry(path, relativePath, depth, hash, state, knownStat) {
  if (depth > state.bounds.maxDepth) {
    throw boundedSnapshotFailure();
  }
  state.entryCount += 1;
  if (state.entryCount > state.bounds.maxEntries) {
    throw boundedSnapshotFailure();
  }

  const stat = knownStat ?? lstatSync(path);
  const kind = entryKind(stat);
  updateFramed(hash, relativePath);
  updateFramed(hash, kind);

  if (kind === "file") {
    hashFile(path, hash, state);
    return;
  }
  if (kind === "symlink") {
    updateFramed(hash, readlinkSync(path, { encoding: "buffer" }));
    return;
  }
  if (kind !== "directory") {
    return;
  }

  for (const name of sortedDirectoryNames(path, state)) {
    const childRelativePath = relativePath ? join(relativePath, name) : name;
    hashEntry(
      join(path, name),
      childRelativePath,
      depth + 1,
      hash,
      state,
    );
  }
}

export function snapshotBoundedPathIdentity(path, bounds = {}) {
  const rootPath = resolve(path);
  const resolvedBounds = { ...defaultSnapshotBounds, ...bounds };
  const hash = createHash("sha256");
  const state = {
    bounds: resolvedBounds,
    entryCount: 0,
    totalFileBytes: 0,
  };

  let rootStat;
  try {
    rootStat = lstatSync(rootPath);
  } catch (error) {
    if (error?.code === "ENOENT") {
      updateFramed(hash, "missing");
      return {
        bounds: resolvedBounds,
        digest: hash.digest("hex"),
        entryCount: 0,
        exists: false,
        rootPath,
        totalFileBytes: 0,
      };
    }
    throw snapshotFailure();
  }

  try {
    hashEntry(rootPath, ".", 0, hash, state, rootStat);
  } catch (error) {
    if (error instanceof SmokeFixtureSafetyError) {
      throw error;
    }
    throw snapshotFailure();
  }

  return {
    bounds: resolvedBounds,
    digest: hash.digest("hex"),
    entryCount: state.entryCount,
    exists: true,
    rootPath,
    totalFileBytes: state.totalFileBytes,
  };
}

export function assertBoundedPathIdentityUnchanged(before) {
  const after = snapshotBoundedPathIdentity(before.rootPath, before.bounds);
  if (before.exists !== after.exists || before.digest !== after.digest) {
    throw new SmokeFixtureSafetyError(
      "real opencode config changed during fixture smoke",
    );
  }
}

export function initializeAllocatedFixture({ allocateRoot, initializeRoot }) {
  const root = allocateRoot();
  try {
    return initializeRoot(root);
  } catch (error) {
    try {
      rmSync(root, { force: true, recursive: true });
    } catch {
      throw new SmokeFixtureSafetyError(
        "failed to clean fixture root after initialization error",
      );
    }
    throw error;
  }
}
