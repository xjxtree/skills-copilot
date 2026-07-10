import { writeFileSync } from "node:fs";

import {
  allocateOwnedTemporaryRoot,
  cleanupOwnedTemporaryRoot,
} from "../../lib/smoke-lifecycle.mjs";

const root = process.env.SKILLS_COPILOT_CLEANUP_SIGNAL_ROOT;
const marker = process.env.SKILLS_COPILOT_CLEANUP_SIGNAL_MARKER;
if (!root || !marker) {
  throw new Error("missing cleanup signal fixture paths");
}

allocateOwnedTemporaryRoot(() => root);
process.stdout.write("ready\n", () => {
  setImmediate(() => {
    writeFileSync(marker, "cleanup-started\n");
    cleanupOwnedTemporaryRoot(root);
    setInterval(() => {}, 60_000);
  });
});
