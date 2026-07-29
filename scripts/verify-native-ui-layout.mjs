#!/usr/bin/env node

import { spawnSync } from "node:child_process";

const result = spawnSync(
  "bash",
  ["scripts/test-macos-swift-package.sh", "--filter", "NativeUILayoutTests"],
  {
    cwd: process.cwd(),
    env: process.env,
    stdio: "inherit",
  },
);

if (result.error) {
  console.error(`native UI layout verification could not start: ${result.error.message}`);
  process.exitCode = 1;
} else {
  process.exitCode = result.status ?? 1;
}
