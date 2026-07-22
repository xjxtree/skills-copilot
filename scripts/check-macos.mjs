#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "..");
const cacheKey = createHash("sha1").update(repoRoot).digest("hex").slice(0, 10);
const cargoTargetDir =
  process.env.CARGO_TARGET_DIR ?? join(tmpdir(), `agent-copilot-check-target-${cacheKey}`);
const swiftScratchRoot =
  process.env.SWIFTPM_SCRATCH_PATH ?? join(tmpdir(), `agent-copilot-swift-check-${cacheKey}`);
const baseEnv = {
  ...process.env,
  CARGO_TARGET_DIR: cargoTargetDir,
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

for (const [command, args, env] of steps) {
  console.log(`$ ${command} ${args.join(" ")}`);
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    env,
    stdio: "inherit",
  });
  if (result.error) {
    console.error(`check:macos failed to start ${command}: ${result.error.message}`);
    process.exit(1);
  }
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}
