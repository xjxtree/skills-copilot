import { createHash, randomUUID } from "node:crypto";
import {
  closeSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { isAbsolute, join, resolve, sep } from "node:path";

const cacheSchema = 1;
const cachePrefix = "agent-copilot-check-v2";
const markerName = ".agent-copilot-check-cache.json";

export function checkCacheKey(repoRoot) {
  return createHash("sha256").update(resolve(repoRoot)).digest("hex").slice(0, 12);
}

function managedCacheMetadata({ repoRoot, cacheKey, kind, ephemeral }) {
  return {
    schema: cacheSchema,
    repoRoot: resolve(repoRoot),
    cacheKey,
    kind,
    ephemeral,
  };
}

function markerPath(cachePath) {
  return join(cachePath, markerName);
}

function readJSON(path) {
  try {
    return JSON.parse(readFileSync(path, "utf8"));
  } catch {
    return null;
  }
}

function metadataMatches(actual, expected) {
  return (
    actual?.schema === expected.schema &&
    actual?.repoRoot === expected.repoRoot &&
    actual?.cacheKey === expected.cacheKey &&
    actual?.kind === expected.kind &&
    actual?.ephemeral === expected.ephemeral
  );
}

function createManagedCache(path, metadata) {
  mkdirSync(path, { recursive: true });
  writeFileSync(markerPath(path), `${JSON.stringify(metadata, null, 2)}\n`, {
    encoding: "utf8",
    mode: 0o600,
  });
}

function isDirectory(path) {
  try {
    return statSync(path).isDirectory();
  } catch {
    return false;
  }
}

export function cargoCacheHealth(cachePath) {
  const buildRoot = join(cachePath, "debug", "build");
  if (!isDirectory(buildRoot)) return { healthy: true };

  const missingGeneratedFiles = [];
  for (const entry of readdirSync(buildRoot, { withFileTypes: true })) {
    if (!entry.isDirectory() || !entry.name.startsWith("libsqlite3-sys-")) continue;
    const outputRoot = join(buildRoot, entry.name, "out");
    if (isDirectory(outputRoot) && !existsSync(join(outputRoot, "bindgen.rs"))) {
      missingGeneratedFiles.push(join(outputRoot, "bindgen.rs"));
    }
  }
  return {
    healthy: missingGeneratedFiles.length === 0,
    missingGeneratedFiles,
  };
}

function isPathInsideTempRoot(path, tempRoot) {
  const absolutePath = resolve(path);
  const absoluteRoot = resolve(tempRoot);
  return absolutePath.startsWith(`${absoluteRoot}${sep}`);
}

function removeOwnedCache({ path, metadata, tempRoot }) {
  const actual = readJSON(markerPath(path));
  const expectedPrefix =
    metadata.kind === "cargo"
      ? `${cachePrefix}-target-${metadata.cacheKey}`
      : `${cachePrefix}-swift-${metadata.cacheKey}`;
  if (
    !isAbsolute(path) ||
    !isPathInsideTempRoot(path, tempRoot) ||
    !path.split(sep).at(-1)?.startsWith(expectedPrefix) ||
    !metadataMatches(actual, metadata)
  ) {
    return false;
  }
  rmSync(path, { recursive: true, force: true });
  return true;
}

function ensureStableCache({ path, metadata, tempRoot }) {
  if (!existsSync(path)) {
    createManagedCache(path, metadata);
    return { path, repaired: false };
  }

  if (!metadataMatches(readJSON(markerPath(path)), metadata)) {
    return null;
  }

  if (metadata.kind === "cargo") {
    const health = cargoCacheHealth(path);
    if (!health.healthy) {
      if (!removeOwnedCache({ path, metadata, tempRoot })) return null;
      createManagedCache(path, metadata);
      return {
        path,
        repaired: true,
        reason: `missing generated files: ${health.missingGeneratedFiles.join(", ")}`,
      };
    }
  }
  return { path, repaired: false };
}

function createEphemeralCache({ repoRoot, cacheKey, kind, tempRoot }) {
  const leafPrefix =
    kind === "cargo"
      ? `${cachePrefix}-target-${cacheKey}-run-`
      : `${cachePrefix}-swift-${cacheKey}-run-`;
  const path = mkdtempSync(join(tempRoot, leafPrefix));
  const metadata = managedCacheMetadata({
    repoRoot,
    cacheKey,
    kind,
    ephemeral: true,
  });
  writeFileSync(markerPath(path), `${JSON.stringify(metadata, null, 2)}\n`, {
    encoding: "utf8",
    mode: 0o600,
  });
  return { path, metadata };
}

function processIsAlive(pid) {
  if (!Number.isSafeInteger(pid) || pid <= 0) return false;
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return error?.code === "EPERM";
  }
}

function tryAcquireLock(lockPath, repoRoot) {
  const token = `${process.pid}-${Date.now()}-${randomUUID()}`;
  const payload = {
    schema: cacheSchema,
    repoRoot: resolve(repoRoot),
    pid: process.pid,
    token,
  };

  const attempt = () => {
    let descriptor;
    try {
      descriptor = openSync(lockPath, "wx", 0o600);
      writeFileSync(descriptor, `${JSON.stringify(payload)}\n`, "utf8");
      closeSync(descriptor);
      return { lockPath, token };
    } catch (error) {
      if (descriptor !== undefined) closeSync(descriptor);
      if (error?.code !== "EEXIST") throw error;
      return null;
    }
  };

  const acquired = attempt();
  if (acquired) return acquired;

  const existing = readJSON(lockPath);
  if (
    existing?.schema === cacheSchema &&
    existing?.repoRoot === resolve(repoRoot) &&
    processIsAlive(existing.pid)
  ) {
    return null;
  }

  try {
    unlinkSync(lockPath);
  } catch (error) {
    if (error?.code !== "ENOENT") return null;
  }
  return attempt();
}

function releaseLock(lock) {
  if (!lock) return;
  const current = readJSON(lock.lockPath);
  if (current?.token !== lock.token) return;
  try {
    unlinkSync(lock.lockPath);
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
}

export function allocateCheckMacOSCaches({
  repoRoot,
  env = process.env,
  tempRoot = tmpdir(),
} = {}) {
  if (!repoRoot) throw new Error("repoRoot is required");
  mkdirSync(tempRoot, { recursive: true });

  const absoluteRepoRoot = resolve(repoRoot);
  const cacheKey = checkCacheKey(absoluteRepoRoot);
  const cargoOverride = env.CARGO_TARGET_DIR;
  const swiftOverride = env.SWIFTPM_SCRATCH_PATH;
  const needsManagedCache = !cargoOverride || !swiftOverride;
  const lockPath = join(tempRoot, `${cachePrefix}-${cacheKey}.lock`);
  const lock = needsManagedCache ? tryAcquireLock(lockPath, absoluteRepoRoot) : null;
  const ephemeralCaches = [];
  const messages = [];

  const allocate = (kind, override) => {
    if (override) return override;

    const stablePath =
      kind === "cargo"
        ? join(tempRoot, `${cachePrefix}-target-${cacheKey}`)
        : join(tempRoot, `${cachePrefix}-swift-${cacheKey}`);
    const stableMetadata = managedCacheMetadata({
      repoRoot: absoluteRepoRoot,
      cacheKey,
      kind,
      ephemeral: false,
    });
    if (lock) {
      const stable = ensureStableCache({
        path: stablePath,
        metadata: stableMetadata,
        tempRoot,
      });
      if (stable) {
        if (stable.repaired) {
          messages.push(`Rebuilt unhealthy ${kind} check cache (${stable.reason}).`);
        }
        return stable.path;
      }
      messages.push(`Ignored unowned ${kind} cache at ${stablePath}.`);
    } else {
      messages.push(`Another check owns the shared cache; using an isolated ${kind} cache.`);
    }

    const ephemeral = createEphemeralCache({
      repoRoot: absoluteRepoRoot,
      cacheKey,
      kind,
      tempRoot,
    });
    ephemeralCaches.push(ephemeral);
    return ephemeral.path;
  };

  const cargoTargetDir = allocate("cargo", cargoOverride);
  const swiftScratchRoot = allocate("swift", swiftOverride);
  let released = false;

  return {
    cargoTargetDir,
    swiftScratchRoot,
    messages,
    release() {
      if (released) return;
      released = true;
      for (const cache of ephemeralCaches) {
        removeOwnedCache({
          path: cache.path,
          metadata: cache.metadata,
          tempRoot,
        });
      }
      releaseLock(lock);
    },
  };
}
