import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  initializeAllocatedFixture,
  snapshotBoundedPathIdentity,
} from "../../lib/smoke-fixture-safety.mjs";
import { runSmokeFlow } from "../../lib/smoke-flow.mjs";
import { cleanupOwnedTemporaryRoot } from "../../lib/smoke-lifecycle.mjs";

const configRoot = process.env.SKILLS_COPILOT_SIGNAL_TEST_ROOT;
if (!configRoot) {
  throw new Error("missing signal test root");
}

await runSmokeFlow(
  {
    allowStaleApp: false,
    bundleOnly: false,
    captureWindow: false,
    checkLogs: false,
    fixtureData: true,
    headlessSidecar: true,
    keepOpen: false,
  },
  {
    assertRealOpencodeConfigUntouched() {},
    cleanupFixture(root) {
      cleanupOwnedTemporaryRoot(root);
    },
    createFixtureEnvironment() {
      return initializeAllocatedFixture({
        allocateRoot: () =>
          mkdtempSync(join(tmpdir(), "skills-copilot-native-smoke-")),
        async initializeRoot(root) {
          const realOpencodeConfigSnapshot =
            await snapshotBoundedPathIdentity(configRoot);
          return {
            appData: join(root, "app-data"),
            home: join(root, "home"),
            realOpencodeConfigSnapshot,
            root,
          };
        },
      });
    },
    note() {},
    runFixtureProjectContextSmoke() {
      throw new Error("headless signal child unexpectedly reached project smoke");
    },
    runFixtureServiceSmoke() {
      throw new Error("headless signal child unexpectedly reached service smoke");
    },
    verifyBundle() {},
    verifyBundleFreshness() {},
  },
);
