#!/usr/bin/env node

console.error(
  "The manual native test registry was retired. Swift Testing discovers suites automatically; run `pnpm test:macos-native-models` or `pnpm test:macos-swift`.",
);
process.exitCode = 1;
