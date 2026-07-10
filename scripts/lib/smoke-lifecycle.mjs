import { spawn } from "node:child_process";
import { rmSync } from "node:fs";

const cleanupSignals = ["SIGHUP", "SIGINT", "SIGTERM"];
const defaultMaxBuffer = 1_024 * 1_024;
const groupTerminationGraceMs = 25;
const signalCleanupTimeoutMs = 2_000;
const signalPollIntervalMs = 10;
const synchronousWaitState = new Int32Array(new SharedArrayBuffer(4));
const ownedRoots = new Map();
const ownedChildren = new Set();
let cleanupHooks = null;
let terminationState = null;

function boundedDelay(milliseconds) {
  return new Promise((resolveDelay) => setTimeout(resolveDelay, milliseconds));
}

function waitSynchronously(milliseconds) {
  Atomics.wait(synchronousWaitState, 0, 0, milliseconds);
}

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

function directChildCanOwnGroup(state) {
  return (
    Number.isSafeInteger(state.pid) &&
    state.pid > 0 &&
    !state.exitObserved &&
    state.child.exitCode === null &&
    state.child.signalCode === null
  );
}

function signalOwnedProcessGroup(state, signal) {
  if (!directChildCanOwnGroup(state)) {
    return false;
  }
  try {
    if (process.platform === "win32") {
      return state.child.kill(signal);
    }
    process.kill(-state.pid, signal);
    return true;
  } catch {
    return false;
  }
}

function requestGroupTermination(state, failure) {
  if (failure && !state.failure) {
    state.failure = failure;
  }
  if (state.terminationRequested) {
    return;
  }
  state.terminationRequested = true;
  state.killed = true;

  const initialSignal = state.killSignal ?? "SIGTERM";
  signalOwnedProcessGroup(state, initialSignal);
  if (initialSignal !== "SIGKILL" && initialSignal !== 9) {
    // Keep the event loop from reaping the direct child between TERM and KILL.
    // Its unreaped PID therefore cannot be reused for an unrelated group.
    waitSynchronously(groupTerminationGraceMs);
    signalOwnedProcessGroup(state, "SIGKILL");
  }
}

function processGroupExists(pgid) {
  if (process.platform === "win32") {
    return false;
  }
  try {
    process.kill(-pgid, 0);
    return true;
  } catch (error) {
    return error?.code !== "ESRCH";
  }
}

async function waitForChildAndGroup(state, deadline) {
  const closeRemaining = Math.max(0, deadline - Date.now());
  if (!state.closed && closeRemaining > 0) {
    await Promise.race([
      state.closePromise,
      boundedDelay(closeRemaining),
    ]);
  }
  while (
    Number.isSafeInteger(state.pid) &&
    processGroupExists(state.pid) &&
    Date.now() < deadline
  ) {
    await boundedDelay(
      Math.min(signalPollIntervalMs, Math.max(1, deadline - Date.now())),
    );
  }
  const groupGone =
    !Number.isSafeInteger(state.pid) || !processGroupExists(state.pid);
  return state.closed && groupGone;
}

function terminateAllChildrenForExit() {
  for (const state of ownedChildren) {
    requestGroupTermination(state);
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
  if (
    !terminationState &&
    ownedRoots.size === 0 &&
    ownedChildren.size === 0
  ) {
    removeCleanupHooks();
  }
}

async function handleTerminationSignal(signal) {
  const states = [...ownedChildren];
  const deadline = Date.now() + signalCleanupTimeoutMs;
  try {
    for (const state of states) {
      requestGroupTermination(state);
    }
    const cleanupConfirmed = await Promise.all(
      states.map((state) => waitForChildAndGroup(state, deadline)),
    );
    if (cleanupConfirmed.some((confirmed) => !confirmed)) {
      process.stderr.write(
        "smoke lifecycle: child cleanup deadline exceeded\n",
      );
    }
  } finally {
    for (const state of states) {
      ownedChildren.delete(state);
    }
    cleanupRoots();
    removeCleanupHooks();
    process.kill(process.pid, signal);
  }
}

function installCleanupHooks() {
  if (cleanupHooks) {
    return;
  }
  const hooks = {
    exit() {
      terminationState = { kind: "exit" };
      terminateAllChildrenForExit();
      cleanupRoots({ exitOnly: true });
      removeCleanupHooks();
    },
    signals: new Map(),
  };
  cleanupHooks = hooks;
  process.once("exit", hooks.exit);
  for (const signal of cleanupSignals) {
    const listener = () => {
      if (terminationState) {
        return;
      }
      terminationState = { kind: "signal", signal };
      void handleTerminationSignal(signal);
    };
    hooks.signals.set(signal, listener);
    process.once(signal, listener);
  }
}

function normalizedEncoding(encoding) {
  if (encoding === null || encoding === "buffer") {
    return null;
  }
  const resolved = encoding ?? "utf8";
  if (!Buffer.isEncoding(resolved)) {
    throw new TypeError(`invalid smoke child-process encoding ${resolved}`);
  }
  return resolved;
}

function validatedMaxBuffer(maxBuffer) {
  const resolved = maxBuffer ?? defaultMaxBuffer;
  if (!Number.isSafeInteger(resolved) || resolved < 0) {
    throw new RangeError("invalid smoke child-process maxBuffer");
  }
  return resolved;
}

function validatedTimeout(timeout) {
  const resolved = timeout ?? 0;
  if (!Number.isSafeInteger(resolved) || resolved < 0) {
    throw new RangeError("invalid smoke child-process timeout");
  }
  return resolved;
}

function outputLimitFailure(stream) {
  const error = new RangeError(`${stream} maxBuffer length exceeded`);
  error.code = "ERR_CHILD_PROCESS_STDIO_MAXBUFFER";
  return error;
}

function timeoutFailure(timeout) {
  const error = new Error(`Command timed out after ${timeout}ms`);
  error.code = null;
  error.timedOut = true;
  return error;
}

function abortFailure(reason) {
  const error = new Error("The operation was aborted", { cause: reason });
  error.code = "ABORT_ERR";
  error.name = "AbortError";
  return error;
}

function appendBoundedOutput(state, stream, chunk) {
  const chunks = state[`${stream}Chunks`];
  const byteKey = `${stream}Bytes`;
  const remaining = Math.max(0, state.maxBuffer - state[byteKey]);
  const kept = Math.min(remaining, chunk.length);
  if (kept > 0) {
    chunks.push(chunk.subarray(0, kept));
    state[byteKey] += kept;
  }
  if (kept < chunk.length) {
    requestGroupTermination(state, outputLimitFailure(stream));
  }
}

function boundedOutput(state, stream) {
  const output = Buffer.concat(
    state[`${stream}Chunks`],
    state[`${stream}Bytes`],
  );
  if (state.encoding === null) {
    return output;
  }
  let decoded = output.toString(state.encoding);
  while (Buffer.byteLength(decoded, state.encoding) > state.maxBuffer) {
    decoded = decoded.slice(0, -1);
  }
  return decoded;
}

function naturalExitFailure(code, signal) {
  const error = new Error(
    signal
      ? `Command terminated by ${signal}`
      : `Command failed with exit code ${code}`,
  );
  error.code = code;
  return error;
}

function settleClosedChild(state) {
  if (terminationState || state.settled) {
    return;
  }
  state.settled = true;
  clearTimeout(state.timeoutTimer);
  if (state.abortSignal && state.abortListener) {
    state.abortSignal.removeEventListener("abort", state.abortListener);
  }
  ownedChildren.delete(state);
  removeHooksIfIdle();

  const stdout = boundedOutput(state, "stdout");
  const stderr = boundedOutput(state, "stderr");
  if (
    !state.failure &&
    !state.spawnError &&
    state.closeCode === 0 &&
    state.closeSignal === null
  ) {
    state.resolve({ stderr, stdout });
    return;
  }

  const error =
    state.failure ??
    state.spawnError ??
    naturalExitFailure(state.closeCode, state.closeSignal);
  error.killed = state.killed;
  error.signal = state.closeSignal;
  error.stderr = stderr;
  error.stdout = stdout;
  state.reject(error);
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
  const encoding = normalizedEncoding(options.encoding);
  const maxBuffer = validatedMaxBuffer(options.maxBuffer);
  const timeout = validatedTimeout(options.timeout);
  if (options.signal?.aborted) {
    return Promise.reject(abortFailure(options.signal.reason));
  }

  installCleanupHooks();
  return new Promise((resolve, reject) => {
    let child;
    try {
      child = spawn(command, args, {
        argv0: options.argv0,
        cwd: options.cwd,
        detached: process.platform !== "win32",
        env: options.env,
        gid: options.gid,
        shell: false,
        stdio: ["ignore", "pipe", "pipe"],
        uid: options.uid,
        windowsHide: options.windowsHide,
        windowsVerbatimArguments: options.windowsVerbatimArguments,
      });
    } catch (error) {
      removeHooksIfIdle();
      reject(error);
      return;
    }

    let resolveClose;
    const closePromise = new Promise((resolveChildClose) => {
      resolveClose = resolveChildClose;
    });
    const state = {
      abortListener: null,
      abortSignal: options.signal ?? null,
      child,
      closeCode: null,
      closePromise,
      closeSignal: null,
      closed: false,
      encoding,
      exitObserved: false,
      failure: null,
      killed: false,
      killSignal: options.killSignal ?? "SIGTERM",
      maxBuffer,
      pid: child.pid,
      reject,
      resolve,
      resolveClose,
      settled: false,
      spawnError: null,
      stderrBytes: 0,
      stderrChunks: [],
      stdoutBytes: 0,
      stdoutChunks: [],
      terminationRequested: false,
      timeoutTimer: null,
    };
    ownedChildren.add(state);

    child.stdout.on("data", (chunk) => {
      appendBoundedOutput(state, "stdout", chunk);
    });
    child.stderr.on("data", (chunk) => {
      appendBoundedOutput(state, "stderr", chunk);
    });
    child.once("error", (error) => {
      state.spawnError = error;
    });
    child.once("exit", (code, signal) => {
      state.exitObserved = true;
      state.closeCode = code;
      state.closeSignal = signal;
    });
    child.once("close", (code, signal) => {
      state.closed = true;
      state.closeCode = code;
      state.closeSignal = signal;
      state.resolveClose();
      settleClosedChild(state);
    });

    if (timeout > 0) {
      state.timeoutTimer = setTimeout(() => {
        requestGroupTermination(state, timeoutFailure(timeout));
      }, timeout);
    }
    if (state.abortSignal) {
      state.abortListener = () => {
        requestGroupTermination(
          state,
          abortFailure(state.abortSignal.reason),
        );
      };
      state.abortSignal.addEventListener("abort", state.abortListener, {
        once: true,
      });
    }
  });
}
