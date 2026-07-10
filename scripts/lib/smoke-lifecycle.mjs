import { execFile } from "node:child_process";
import { rmSync } from "node:fs";

const cleanupSignals = ["SIGHUP", "SIGINT", "SIGTERM"];
const ownedRoots = new Map();
const ownedChildren = new Set();
let cleanupHooks = null;
let terminating = false;

function removeCleanupHooks() {
  if (!cleanupHooks) {
    return;
  }
  const hooks = cleanupHooks;
  cleanupHooks = null;
  process.removeListener("exit", hooks.exit);
  for (const [signal, listener] of hooks.signals) {
    process.removeListener(signal, listener);
  }
}

function terminateChildProcessGroup(child) {
  const pid = child?.pid;
  if (!Number.isSafeInteger(pid) || pid <= 0) {
    return;
  }

  if (process.platform !== "win32") {
    try {
      process.kill(-pid, "SIGKILL");
      return;
    } catch (error) {
      if (error?.code !== "ESRCH") {
        return;
      }
    }
  }

  try {
    child.kill("SIGKILL");
  } catch {
    // The child may already have exited between observation and cleanup.
  }
}

function terminateAllChildren() {
  const children = [...ownedChildren];
  ownedChildren.clear();
  for (const child of children) {
    terminateChildProcessGroup(child);
  }
}

function cleanupRoots({ exitOnly = false } = {}) {
  for (const [root, ownership] of [...ownedRoots]) {
    if (exitOnly && !ownership.cleanupOnExit) {
      continue;
    }
    try {
      rmSync(root, { force: true, recursive: true });
    } catch {
      // Signal/exit cleanup is best effort for private, explicitly owned roots.
    } finally {
      ownedRoots.delete(root);
    }
  }
}

function removeHooksIfIdle() {
  if (!terminating && ownedRoots.size === 0 && ownedChildren.size === 0) {
    removeCleanupHooks();
  }
}

function installCleanupHooks() {
  if (cleanupHooks) {
    return;
  }
  const hooks = {
    exit() {
      terminating = true;
      terminateAllChildren();
      cleanupRoots({ exitOnly: true });
      removeCleanupHooks();
    },
    signals: new Map(),
  };
  cleanupHooks = hooks;
  process.once("exit", hooks.exit);
  for (const signal of cleanupSignals) {
    const listener = () => {
      if (terminating) {
        return;
      }
      terminating = true;
      terminateAllChildren();
      cleanupRoots();
      removeCleanupHooks();
      process.kill(process.pid, signal);
    };
    hooks.signals.set(signal, listener);
    process.once(signal, listener);
  }
}

export function allocateOwnedTemporaryRoot(
  allocateRoot,
  { cleanupOnExit = true } = {},
) {
  installCleanupHooks();
  try {
    const root = allocateRoot();
    ownedRoots.set(root, { cleanupOnExit });
    return root;
  } catch (error) {
    removeHooksIfIdle();
    throw error;
  }
}

export function cleanupOwnedTemporaryRoot(root) {
  try {
    rmSync(root, { force: true, recursive: true });
    ownedRoots.delete(root);
  } finally {
    removeHooksIfIdle();
  }
}

export function executeOwnedFile(command, args, options = {}) {
  installCleanupHooks();
  return new Promise((resolve, reject) => {
    let child;
    try {
      child = execFile(
        command,
        args,
        {
          ...options,
          detached: process.platform !== "win32",
        },
        (error, stdout, stderr) => {
          if (error) {
            terminateChildProcessGroup(child);
          }
          ownedChildren.delete(child);
          removeHooksIfIdle();
          if (error) {
            error.stdout = stdout;
            error.stderr = stderr;
            reject(error);
          } else {
            resolve({ stderr, stdout });
          }
        },
      );
      ownedChildren.add(child);
    } catch (error) {
      removeHooksIfIdle();
      reject(error);
    }
  });
}
