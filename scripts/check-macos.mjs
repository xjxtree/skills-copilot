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
  ["cargo", ["fmt", "--all", "--", "--check"], baseEnv],
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
    "swift",
    [
      "build",
      "--package-path",
      "apps/macos",
      "--scratch-path",
      join(swiftScratchRoot, "build"),
    ],
    baseEnv,
  ],
  [
    "./script/build_and_run.sh",
    ["--verify"],
    {
      ...baseEnv,
      SWIFTPM_SCRATCH_PATH: join(swiftScratchRoot, "bundle"),
    },
  ],
  ["pnpm", ["smoke:macos-app", "--", "--fixture-data", "--capture-window"], baseEnv],
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
