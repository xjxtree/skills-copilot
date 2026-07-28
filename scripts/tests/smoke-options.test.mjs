import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { runSmokeFlow } from "../lib/smoke-flow.mjs";
import { parseSmokeOptions } from "../lib/smoke-options.mjs";

const emptyOptions = {
  allowStaleApp: false,
  bundleOnly: false,
  captureWindow: false,
  checkLogs: false,
  fixtureData: false,
  headlessSidecar: false,
  keepOpen: false,
};

test("returns every smoke option as a boolean when no flags are present", () => {
  assert.deepEqual(parseSmokeOptions([], {}), emptyOptions);
});

for (const [flag, property] of [
  ["--bundle-only", "bundleOnly"],
  ["--fixture-data", "fixtureData"],
  ["--keep-open", "keepOpen"],
  ["--capture-window", "captureWindow"],
  ["--check-logs", "checkLogs"],
  ["--allow-stale-app", "allowStaleApp"],
]) {
  test(`maps ${flag} to ${property}`, () => {
    assert.deepEqual(parseSmokeOptions([flag], {}), {
      ...emptyOptions,
      [property]: true,
    });
  });
}

test("preserves duplicate supported flag semantics", () => {
  assert.deepEqual(
    parseSmokeOptions(["--fixture-data", "--fixture-data"], {}),
    { ...emptyOptions, fixtureData: true },
  );
});

test("preserves the exact stale-app environment alias semantics", () => {
  assert.equal(
    parseSmokeOptions([], { SKILLS_COPILOT_ALLOW_STALE_APP: "1" }).allowStaleApp,
    true,
  );
  for (const value of ["", "0", "true", "yes"]) {
    assert.equal(
      parseSmokeOptions([], { SKILLS_COPILOT_ALLOW_STALE_APP: value }).allowStaleApp,
      false,
      `expected ${JSON.stringify(value)} to remain false`,
    );
  }
});

test("the stale-app flag remains an alias for the environment switch", () => {
  assert.equal(
    parseSmokeOptions(["--allow-stale-app"], {
      SKILLS_COPILOT_ALLOW_STALE_APP: "0",
    }).allowStaleApp,
    true,
  );
});

for (const unknown of ["--unknown", "fixture-data"]) {
  test(`rejects unknown argument ${unknown}`, () => {
    assert.throws(() => parseSmokeOptions([unknown], {}), /unknown smoke option/);
  });
}

test("accepts the leading pnpm argument separator", () => {
  assert.deepEqual(
    parseSmokeOptions(
      ["--", "--fixture-data", "--headless-sidecar"],
      {},
    ),
    {
      ...emptyOptions,
      fixtureData: true,
      headlessSidecar: true,
    },
  );
});

test("rejects a misplaced argument separator", () => {
  assert.throws(
    () => parseSmokeOptions(["--fixture-data", "--"], {}),
    /unknown smoke option/,
  );
});

test("headless sidecar requires fixture data", () => {
  assert.throws(
    () => parseSmokeOptions(["--headless-sidecar"], {}),
    /requires --fixture-data/,
  );
});

for (const incompatible of [
  "--bundle-only",
  "--keep-open",
  "--capture-window",
  "--check-logs",
]) {
  test(`headless sidecar rejects ${incompatible}`, () => {
    assert.throws(
      () =>
        parseSmokeOptions(
          ["--fixture-data", "--headless-sidecar", incompatible],
          {},
        ),
      /cannot be combined/,
    );
  });
}

test("accepts fixture-only headless mode", () => {
  assert.deepEqual(
    parseSmokeOptions(["--fixture-data", "--headless-sidecar"], {}),
    {
      ...emptyOptions,
      fixtureData: true,
      headlessSidecar: true,
    },
  );
});

test("headless mode may retain the existing stale-bundle override", () => {
  assert.deepEqual(
    parseSmokeOptions(
      ["--fixture-data", "--headless-sidecar", "--allow-stale-app"],
      {},
    ),
    {
      ...emptyOptions,
      allowStaleApp: true,
      fixtureData: true,
      headlessSidecar: true,
    },
  );
});

function headlessOptions() {
  return parseSmokeOptions(["--fixture-data", "--headless-sidecar"], {});
}

function recordingDependencies(trace, overrides = {}) {
  const fixture = {
    appData: "/fixture/app-data",
    home: "/fixture/home",
    realOpencodeConfigSnapshot: ["real-opencode-snapshot"],
    root: "/fixture/root",
  };
  const allowed = {
    assertRealOpencodeConfigUntouched(snapshot) {
      trace.push(["assert-real-opencode", snapshot]);
    },
    cleanupFixture(root) {
      trace.push(["cleanup-fixture", root]);
    },
    createFixtureEnvironment() {
      trace.push(["create-fixture"]);
      return fixture;
    },
    note(message) {
      trace.push(["note", message]);
    },
    runFixtureProjectContextSmoke(env, receivedFixture, status) {
      trace.push(["project-smoke", env, receivedFixture, status]);
    },
    runFixtureServiceSmoke(env) {
      trace.push(["service-smoke", env]);
      return { protocol_version: 1 };
    },
    verifyBundle() {
      trace.push(["verify-bundle"]);
    },
    verifyBundleFreshness(allowStaleApp) {
      trace.push(["verify-freshness", allowStaleApp]);
    },
    ...overrides,
  };
  const forbidden = new Set([
    "captureAppWindow",
    "checkSystemLogs",
    "launchApp",
    "queryRunningApps",
    "terminateExistingApp",
  ]);
  return new Proxy(allowed, {
    get(target, property, receiver) {
      if (forbidden.has(property)) {
        assert.fail(`headless flow reached forbidden dependency ${property}`);
      }
      return Reflect.get(target, property, receiver);
    },
  });
}

test("headless flow reaches only bundle and fixture-sidecar dependencies", async () => {
  const trace = [];

  await runSmokeFlow(headlessOptions(), recordingDependencies(trace));

  const env = {
    SKILLS_COPILOT_APP_DATA_DIR: "/fixture/app-data",
    SKILLS_COPILOT_HOME: "/fixture/home",
  };
  assert.deepEqual(trace, [
    ["verify-bundle"],
    ["verify-freshness", false],
    ["create-fixture"],
    ["note", "headless fixture data enabled: /fixture/root"],
    ["service-smoke", env],
    [
      "project-smoke",
      env,
      {
        appData: "/fixture/app-data",
        home: "/fixture/home",
        realOpencodeConfigSnapshot: ["real-opencode-snapshot"],
        root: "/fixture/root",
      },
      { protocol_version: 1 },
    ],
    ["assert-real-opencode", ["real-opencode-snapshot"]],
    ["cleanup-fixture", "/fixture/root"],
    ["note", "headless bundled-sidecar smoke completed"],
  ]);
});

test("headless flow always cleans its fixture after a sidecar failure", async () => {
  const trace = [];
  const failure = new Error("fixture sidecar failed");
  const dependencies = recordingDependencies(trace, {
    runFixtureServiceSmoke(env) {
      trace.push(["service-smoke", env]);
      throw failure;
    },
  });

  await assert.rejects(
    () => runSmokeFlow(headlessOptions(), dependencies),
    (error) => error === failure,
  );
  assert.deepEqual(trace.slice(-2), [
    ["assert-real-opencode", ["real-opencode-snapshot"]],
    ["cleanup-fixture", "/fixture/root"],
  ]);
});

test("smoke script delegates argument parsing instead of inspecting argv directly", () => {
  const source = readFileSync("scripts/smoke-macos-app.mjs", "utf8");
  assert.doesNotMatch(source, /process\.argv\.includes/);
});

test("CI builds without launching and validates the bundled sidecar headlessly", () => {
  const workflow = readFileSync(".github/workflows/ci.yml", "utf8");
  const packageJson = JSON.parse(readFileSync("package.json", "utf8"));
  const buildScript = readFileSync("script/build_and_run.sh", "utf8");
  const localGate = readFileSync("scripts/check-macos.mjs", "utf8");
  const smokeScript = readFileSync("scripts/smoke-macos-app.mjs", "utf8");

  assert.match(workflow, /^on:\n  pull_request:\n  push:\n\n/m);
  assert.match(
    workflow,
    /run: pnpm smoke:macos-app -- --fixture-data --headless-sidecar/,
  );
  assert.doesNotMatch(workflow, /--bundle-only/);
  assert.doesNotMatch(workflow, /verify:macos-native-test-registry/);
  assert.match(
    workflow,
    /run: SWIFTPM_SCRATCH_PATH="\$RUNNER_TEMP\/swift-tests" pnpm test:macos-swift/,
  );
  assert.doesNotMatch(workflow, /run: swift build --package-path apps\/macos/);
  assert.ok(
    workflow.indexOf("uses: actions/setup-node@v4") <
      workflow.indexOf("run: corepack enable"),
    "Node 22 must be installed before Corepack is invoked",
  );
  assert.doesNotMatch(workflow, /cache: pnpm/);
  assert.equal(packageJson.scripts.build, "./script/build_and_run.sh --build-only");
  assert.equal(
    packageJson.scripts["build:macos"],
    "./script/build_and_run.sh --build-only",
  );
  assert.equal(
    packageJson.scripts["verify:macos-launch"],
    "./script/build_and_run.sh --verify",
  );
  assert.match(
    buildScript,
    /if \[\[ \$\{#CARGO_ENV\[@\]\} -gt 0 \]\]; then\n  env "\$\{CARGO_ENV\[@\]\}" "\$CARGO_BIN" build/,
  );
  const cargoBuildIndex = buildScript.indexOf(
    'if [[ ${#CARGO_ENV[@]} -gt 0 ]]; then',
  );
  const signingIndex = buildScript.indexOf("sign_app_bundle\n\ncase \"$MODE\"");
  const terminationIndex = buildScript.indexOf(
    'case "$MODE" in\n  --build-only|build-only)\n    ;;\n  *)\n    terminate_existing_app_instances',
  );
  assert.ok(cargoBuildIndex >= 0, "The Cargo build guard should exist.");
  assert.ok(
    signingIndex > cargoBuildIndex,
    "Bundle signing should happen after the build.",
  );
  assert.ok(
    terminationIndex > signingIndex,
    "Running app instances should stop only after a successful signed bundle build.",
  );
  assert.match(localGate, /"\.\/script\/build_and_run\.sh",\s*\["--verify"\]/);
  assert.match(
    localGate,
    /\["pnpm", \["smoke:macos-app", "--", "--fixture-data", "--capture-window"\]/,
  );
  assert.match(smokeScript, /Run pnpm build:macos before Smoke App Run/);
  assert.doesNotMatch(
    smokeScript,
    /Run \.\/script\/build_and_run\.sh --verify or pnpm check:macos before Smoke App Run/,
  );
});
