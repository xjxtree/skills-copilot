import { spawn } from "node:child_process";
import { rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const cleanupSignals = ["SIGHUP", "SIGINT", "SIGTERM"];
const controlMaxBytes = 8 * 1_024;
const defaultMaxBuffer = 1_024 * 1_024;
const groupTerminationGraceMs = 25;
const orphanDrainGraceMs = 2_000;
const signalCleanupTimeoutMs = 2_000;
const signalPollIntervalMs = 10;
const supervisorPath = join(
  dirname(fileURLToPath(import.meta.url)),
  "smoke-child-supervisor.mjs",
);
const synchronousWaitState = new Int32Array(new SharedArrayBuffer(4));
const ownedRoots = new Map();
const ownedChildren = new Set();
let cleanupHooks = null;
let idleRemovalHandle = null;
let ownershipGeneration = 0;
let terminationState = null;

function boundedDelay(milliseconds) {
  return new Promise((resolveDelay) => setTimeout(resolveDelay, milliseconds));
}

function waitSynchronously(milliseconds) {
  Atomics.wait(synchronousWaitState, 0, 0, milliseconds);
}

function markOwnershipChanged() {
  ownershipGeneration += 1;
}

function cancelIdleRemoval() {
  if (idleRemovalHandle !== null) {
    clearImmediate(idleRemovalHandle);
    idleRemovalHandle = null;
  }
}

function removeCleanupHooks() {
  cancelIdleRemoval();
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

function requestGroupTermination(
  state,
  failure,
  { markKilled = Boolean(failure) } = {},
) {
  if (failure && !state.failure) {
    state.failure = failure;
  }
  if (markKilled) {
    state.killed = true;
  }
  if (state.terminationRequested) {
    return;
  }
  state.terminationRequested = true;
  clearTimeout(state.targetCleanupTimer);

  const initialSignal = state.killSignal ?? "SIGTERM";
  signalOwnedProcessGroup(state, initialSignal);
  if (initialSignal !== "SIGKILL" && initialSignal !== 9) {
    // The owned supervisor never exits voluntarily. Blocking this short grace
    // prevents Node from reaping its PID between TERM and KILL, so the immutable
    // group identity cannot be reused for an unrelated process group.
    waitSynchronously(groupTerminationGraceMs);
    signalOwnedProcessGroup(state, "SIGKILL");
  }
}

function processGroupExists(pgid) {
  if (process.platform === "win32" || !Number.isSafeInteger(pgid)) {
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
  while (processGroupExists(state.pid) && Date.now() < deadline) {
    await boundedDelay(
      Math.min(signalPollIntervalMs, Math.max(1, deadline - Date.now())),
    );
  }
  return state.closed && !processGroupExists(state.pid);
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
      if (ownedRoots.delete(root)) {
        markOwnershipChanged();
      }
    }
  }
}

function removeHooksIfIdle() {
  if (
    terminationState ||
    ownedRoots.size !== 0 ||
    ownedChildren.size !== 0 ||
    idleRemovalHandle !== null
  ) {
    return;
  }
  const expectedGeneration = ownershipGeneration;
  idleRemovalHandle = setImmediate(() => {
    idleRemovalHandle = null;
    if (
      terminationState ||
      expectedGeneration !== ownershipGeneration ||
      ownedRoots.size !== 0 ||
      ownedChildren.size !== 0
    ) {
      removeHooksIfIdle();
      return;
    }
    removeCleanupHooks();
  });
}

async function handleTerminationSignal(signal) {
  const states = [...ownedChildren];
  const deadline = Date.now() + signalCleanupTimeoutMs;
  try {
    for (const state of states) {
      requestGroupTermination(state, null, { markKilled: true });
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
      if (ownedChildren.delete(state)) {
        markOwnershipChanged();
      }
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
      for (const state of ownedChildren) {
        requestGroupTermination(state, null, { markKilled: true });
      }
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

function protocolFailure(message) {
  const error = new Error(message);
  error.code = "ERR_SMOKE_CHILD_SUPERVISOR";
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

function appendControlOutput(state, chunk) {
  const remaining = Math.max(0, controlMaxBytes - state.controlBytes);
  const kept = Math.min(remaining, chunk.length);
  if (kept > 0) {
    state.controlChunks.push(chunk.subarray(0, kept));
    state.controlBytes += kept;
  }
  if (kept < chunk.length) {
    requestGroupTermination(
      state,
      protocolFailure("smoke child supervisor control limit exceeded"),
    );
    return;
  }
  const control = Buffer.concat(state.controlChunks, state.controlBytes);
  const newline = control.indexOf(10);
  if (newline < 0 || state.targetFinished) {
    return;
  }
  let message;
  try {
    message = JSON.parse(control.subarray(0, newline).toString("utf8"));
  } catch {
    requestGroupTermination(
      state,
      protocolFailure("invalid smoke child supervisor response"),
    );
    return;
  }
  acceptTargetResult(state, message);
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

function errorFromSupervisor(payload) {
  const error = new Error(
    String(payload?.message ?? "unable to spawn smoke child process").slice(
      0,
      512,
    ),
  );
  error.name = String(payload?.name ?? "Error").slice(0, 64);
  for (const field of ["code", "errno", "path", "syscall"]) {
    if (payload?.[field] !== undefined) {
      error[field] = payload[field];
    }
  }
  return error;
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

function maybeFinishTarget(state) {
  if (
    !state.targetFinished ||
    state.terminationRequested ||
    terminationState
  ) {
    return;
  }
  if (state.stdoutEnded && state.stderrEnded) {
    requestGroupTermination(state, null, { markKilled: false });
    return;
  }
  if (state.targetCleanupTimer === null) {
    state.targetCleanupTimer = setTimeout(() => {
      requestGroupTermination(state, null, { markKilled: false });
    }, orphanDrainGraceMs);
  }
}

function acceptTargetResult(state, message) {
  if (state.targetFinished) {
    return;
  }
  if (message?.type === "exit") {
    state.targetCode = message.code;
    state.targetSignal = message.signal;
  } else if (message?.type === "error") {
    state.targetError = errorFromSupervisor(message.error);
  } else {
    requestGroupTermination(
      state,
      protocolFailure("unexpected smoke child supervisor response"),
    );
    return;
  }
  state.targetFinished = true;
  maybeFinishTarget(state);
}

function clearStateTimers(state) {
  clearTimeout(state.targetCleanupTimer);
  clearTimeout(state.timeoutTimer);
  if (state.abortSignal && state.abortListener) {
    state.abortSignal.removeEventListener("abort", state.abortListener);
  }
}

function finalizeClosedChild(state, groupConfirmed) {
  if (terminationState || state.settled) {
    return;
  }
  state.settled = true;
  clearStateTimers(state);
  if (ownedChildren.delete(state)) {
    markOwnershipChanged();
  }
  removeHooksIfIdle();

  const stdout = boundedOutput(state, "stdout");
  const stderr = boundedOutput(state, "stderr");
  if (!groupConfirmed && !state.failure) {
    state.failure = protocolFailure(
      "smoke child process group cleanup deadline exceeded",
    );
  }
  if (
    !state.failure &&
    !state.spawnError &&
    !state.targetError &&
    state.targetFinished &&
    state.targetCode === 0 &&
    state.targetSignal === null
  ) {
    state.resolve({ stderr, stdout });
    return;
  }

  const error =
    state.failure ??
    state.spawnError ??
    state.targetError ??
    naturalExitFailure(state.targetCode, state.targetSignal);
  error.killed = state.killed;
  error.signal = state.targetFinished
    ? state.targetSignal
    : state.anchorCloseSignal;
  error.stderr = stderr;
  error.stdout = stdout;
  state.reject(error);
}

async function settleClosedChild(state) {
  if (terminationState || state.settled || state.settling) {
    return;
  }
  state.settling = true;
  const groupConfirmed = await waitForChildAndGroup(
    state,
    Date.now() + signalCleanupTimeoutMs,
  );
  state.settling = false;
  finalizeClosedChild(state, groupConfirmed);
}

export function allocateOwnedTemporaryRoot(
  allocateRoot,
  { cleanupOnExit = true } = {},
) {
  installCleanupHooks();
  try {
    const root = allocateRoot();
    ownedRoots.set(root, { cleanupOnExit });
    markOwnershipChanged();
    return root;
  } catch (error) {
    removeHooksIfIdle();
    throw error;
  }
}

export function cleanupOwnedTemporaryRoot(root) {
  try {
    rmSync(root, { force: true, recursive: true });
    if (ownedRoots.delete(root)) {
      markOwnershipChanged();
    }
  } finally {
    // Delay hook removal until libuv has dispatched signals queued while the
    // synchronous recursive delete blocked this JavaScript turn.
    removeHooksIfIdle();
  }
}

export function executeOwnedFile(command, args, options = {}) {
  if (typeof command !== "string" || !Array.isArray(args)) {
    throw new TypeError("invalid smoke child-process command");
  }
  if (args.some((argument) => typeof argument !== "string")) {
    throw new TypeError("invalid smoke child-process argument");
  }
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
      child = spawn(
        process.execPath,
        [
          supervisorPath,
          JSON.stringify({
            args,
            argv0: options.argv0 ?? null,
            command,
          }),
        ],
        {
          cwd: options.cwd,
          detached: process.platform !== "win32",
          env: options.env,
          gid: options.gid,
          shell: false,
          stdio: ["ignore", "pipe", "pipe", "pipe"],
          uid: options.uid,
          windowsHide: options.windowsHide,
          windowsVerbatimArguments: options.windowsVerbatimArguments,
        },
      );
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
      anchorCloseSignal: null,
      child,
      closePromise,
      closed: false,
      controlBytes: 0,
      controlChunks: [],
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
      settling: false,
      spawnError: null,
      stderrBytes: 0,
      stderrChunks: [],
      stderrEnded: false,
      stdoutBytes: 0,
      stdoutChunks: [],
      stdoutEnded: false,
      targetCleanupTimer: null,
      targetCode: null,
      targetError: null,
      targetFinished: false,
      targetSignal: null,
      terminationRequested: false,
      timeoutTimer: null,
    };
    ownedChildren.add(state);
    markOwnershipChanged();

    child.stdout.on("data", (chunk) => {
      appendBoundedOutput(state, "stdout", chunk);
    });
    child.stdout.once("end", () => {
      state.stdoutEnded = true;
      maybeFinishTarget(state);
    });
    child.stderr.on("data", (chunk) => {
      appendBoundedOutput(state, "stderr", chunk);
    });
    child.stderr.once("end", () => {
      state.stderrEnded = true;
      maybeFinishTarget(state);
    });
    child.stdio[3].on("data", (chunk) => {
      appendControlOutput(state, chunk);
    });
    child.stdio[3].once("end", () => {
      if (!state.targetFinished && !state.failure && !terminationState) {
        requestGroupTermination(
          state,
          protocolFailure("smoke child supervisor closed without a result"),
        );
      }
    });
    child.once("error", (error) => {
      state.spawnError = error;
    });
    child.once("exit", () => {
      state.exitObserved = true;
    });
    child.once("close", (_code, signal) => {
      state.closed = true;
      state.anchorCloseSignal = signal;
      state.resolveClose();
      void settleClosedChild(state);
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
