import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  allocateOwnedTemporaryRoot,
  cleanupOwnedTemporaryRoot,
  executeOwnedFile,
} from "./smoke-lifecycle.mjs";

const defaultSnapshotBounds = Object.freeze({
  maxDepth: 32,
  maxEntries: 8_192,
  maxFileBytes: 16 * 1_024 * 1_024,
  maxTotalBytes: 64 * 1_024 * 1_024,
});
const snapshotBoundNames = [
  "maxDepth",
  "maxEntries",
  "maxFileBytes",
  "maxTotalBytes",
];
const sourceDirectory = join(
  dirname(fileURLToPath(import.meta.url)),
  "smoke-fixture-identity",
);
const nativeHelperSources = [
  join(sourceDirectory, "Snapshot.swift"),
  join(sourceDirectory, "main.swift"),
];
let nativeHelperBuild = null;

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

function validatedSnapshotBounds(customBounds) {
  const bounds = {};
  for (const name of snapshotBoundNames) {
    const value = Object.hasOwn(customBounds, name)
      ? customBounds[name]
      : defaultSnapshotBounds[name];
    if (!Number.isSafeInteger(value) || value < 0) {
      throw new SmokeFixtureSafetyError(
        `invalid smoke snapshot bound ${name}`,
      );
    }
    bounds[name] = value;
  }
  return Object.freeze(bounds);
}

function cleanupNativeHelper() {
  if (!nativeHelperBuild) {
    return;
  }
  const build = nativeHelperBuild;
  nativeHelperBuild = null;
  try {
    cleanupOwnedTemporaryRoot(build.root);
  } catch {
    // Cleanup is best effort during process teardown; owned paths are private.
  }
}

async function nativeSnapshotHelper() {
  if (nativeHelperBuild?.ready) {
    return nativeHelperBuild.binary;
  }
  if (nativeHelperBuild?.compilation) {
    return nativeHelperBuild.compilation;
  }
  if (process.platform !== "darwin") {
    throw snapshotFailure();
  }

  const root = allocateOwnedTemporaryRoot(() =>
    mkdtempSync(join(tmpdir(), "smoke-fixture-identity-helper-")),
  );
  const binary = join(root, "smoke-fixture-identity");
  const build = { binary, compilation: null, ready: false, root };
  nativeHelperBuild = build;
  build.compilation = (async () => {
    try {
      await executeOwnedFile(
        "swiftc",
        [...nativeHelperSources, "-O", "-o", binary],
        {
          encoding: "utf8",
          maxBuffer: 1_024 * 1_024,
          timeout: 60_000,
        },
      );
      if (nativeHelperBuild !== build) {
        throw snapshotFailure();
      }
      build.ready = true;
      return binary;
    } catch {
      if (nativeHelperBuild === build) {
        cleanupNativeHelper();
      }
      throw snapshotFailure();
    }
  })();
  return build.compilation;
}

function validNativeSnapshotResult(result, bounds) {
  return (
    result &&
    typeof result === "object" &&
    typeof result.exists === "boolean" &&
    typeof result.digest === "string" &&
    /^[0-9a-f]{64}$/.test(result.digest) &&
    Number.isSafeInteger(result.entryCount) &&
    result.entryCount >= 0 &&
    result.entryCount <= bounds.maxEntries &&
    Number.isSafeInteger(result.totalFileBytes) &&
    result.totalFileBytes >= 0 &&
    result.totalFileBytes <= bounds.maxTotalBytes
  );
}

async function nativeSnapshot(rootPath, bounds) {
  let stdout;
  try {
    ({ stdout } = await executeOwnedFile(
      await nativeSnapshotHelper(),
      [
        rootPath,
        String(bounds.maxDepth),
        String(bounds.maxEntries),
        String(bounds.maxFileBytes),
        String(bounds.maxTotalBytes),
      ],
      {
        encoding: "utf8",
        maxBuffer: 1_024 * 1_024,
        timeout: 30_000,
      },
    ));
  } catch (error) {
    if (error?.code === 3) {
      throw boundedSnapshotFailure();
    }
    throw snapshotFailure();
  }

  let snapshot;
  try {
    snapshot = JSON.parse(stdout);
  } catch {
    throw snapshotFailure();
  }
  if (!validNativeSnapshotResult(snapshot, bounds)) {
    throw snapshotFailure();
  }
  return snapshot;
}

export async function snapshotBoundedPathIdentity(path, bounds = {}) {
  const rootPath = resolve(path);
  const resolvedBounds = validatedSnapshotBounds(bounds);
  const snapshot = await nativeSnapshot(rootPath, resolvedBounds);
  return {
    bounds: resolvedBounds,
    digest: snapshot.digest,
    entryCount: snapshot.entryCount,
    exists: snapshot.exists,
    rootPath,
    totalFileBytes: snapshot.totalFileBytes,
  };
}

export async function assertBoundedPathIdentityUnchanged(before) {
  const after = await snapshotBoundedPathIdentity(
    before.rootPath,
    before.bounds,
  );
  if (before.exists !== after.exists || before.digest !== after.digest) {
    throw new SmokeFixtureSafetyError(
      "real opencode config changed during fixture smoke",
    );
  }
}

export async function initializeAllocatedFixture({ allocateRoot, initializeRoot }) {
  const root = allocateOwnedTemporaryRoot(allocateRoot, {
    cleanupOnExit: false,
  });
  try {
    return await initializeRoot(root);
  } catch (error) {
    try {
      cleanupOwnedTemporaryRoot(root);
    } catch {
      throw new SmokeFixtureSafetyError(
        "failed to clean fixture root after initialization error",
      );
    }
    throw error;
  }
}
