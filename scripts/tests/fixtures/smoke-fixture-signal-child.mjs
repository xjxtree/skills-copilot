import { snapshotBoundedPathIdentity } from "../../lib/smoke-fixture-safety.mjs";

const root = process.env.SKILLS_COPILOT_SIGNAL_TEST_ROOT;
if (!root) {
  throw new Error("missing signal test root");
}

const watchedEvents = ["exit", "SIGHUP", "SIGINT", "SIGTERM"];
const baselineListeners = new Map(
  watchedEvents.map((event) => [event, process.listenerCount(event)]),
);
snapshotBoundedPathIdentity(root);
const installedListeners = new Map(
  watchedEvents.map((event) => [event, process.listenerCount(event)]),
);
snapshotBoundedPathIdentity(root);
for (const event of watchedEvents) {
  if (installedListeners.get(event) !== baselineListeners.get(event) + 1) {
    throw new Error(`missing cleanup listener for ${event}`);
  }
  if (process.listenerCount(event) !== installedListeners.get(event)) {
    throw new Error(`cleanup listener leak for ${event}`);
  }
}
process.stdout.write("ready\n");
setInterval(() => {}, 60_000);
