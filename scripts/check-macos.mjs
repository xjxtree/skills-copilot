#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { allocateCheckMacOSCaches } from "./check-macos-cache.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "..");
const cacheSession = allocateCheckMacOSCaches({ repoRoot });
const { cargoTargetDir, swiftScratchRoot } = cacheSession;
for (const message of cacheSession.messages) console.log(`check:macos cache: ${message}`);
const baseEnv = {
  ...process.env,
  CARGO_TARGET_DIR: cargoTargetDir,
  SWIFTPM_SCRATCH_PATH: swiftScratchRoot,
};

const steps = [
  ["git", ["diff", "--check"], baseEnv],
  ["cargo", ["fmt", "--all", "--", "--check"], baseEnv],
  ["pnpm", ["check:privacy"], baseEnv],
  ["cargo", ["test", "--workspace"], baseEnv],
  [
    "cargo",
    ["clippy", "--workspace", "--all-targets", "--all-features", "--", "-D", "warnings"],
    baseEnv,
  ],
  ["pnpm", ["test:macos-list-model"], baseEnv],
  ["pnpm", ["verify:macos-ui-layout"], baseEnv],
  ["pnpm", ["verify:gate-parity"], baseEnv],
  ["pnpm", ["test:macos-native-models"], baseEnv],
  [
    "./script/build_and_run.sh",
    ["--build-only"],
    {
      ...baseEnv,
      SWIFTPM_SCRATCH_PATH: join(swiftScratchRoot, "bundle"),
    },
  ],
  ["pnpm", ["smoke:macos-app", "--", "--fixture-data", "--headless-sidecar"], baseEnv],
];

let exitCode = 0;
try {
  for (const [command, args, env] of steps) {
    console.log(`$ ${command} ${args.join(" ")}`);
    const result = spawnSync(command, args, {
      cwd: repoRoot,
      env,
      stdio: "inherit",
    });
    if (result.error) {
      console.error(`check:macos failed to start ${command}: ${result.error.message}`);
      exitCode = 1;
      break;
    }
    if (result.status !== 0) {
      exitCode = result.status ?? 1;
      break;
    }
  }
} finally {
  cacheSession.release();
}
process.exitCode = exitCode;
