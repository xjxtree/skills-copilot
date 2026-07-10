function fixtureEnvironment(fixture) {
  return {
    SKILLS_COPILOT_APP_DATA_DIR: fixture.appData,
    SKILLS_COPILOT_HOME: fixture.home,
  };
}

export function runSmokeFlow(options, dependencies) {
  dependencies.verifyBundle();
  dependencies.verifyBundleFreshness(options.allowStaleApp);

  if (options.headlessSidecar) {
    const fixture = dependencies.createFixtureEnvironment();
    try {
      const env = fixtureEnvironment(fixture);
      dependencies.note(`headless fixture data enabled: ${fixture.root}`);
      const status = dependencies.runFixtureServiceSmoke(env);
      dependencies.runFixtureProjectContextSmoke(env, fixture, status);
    } finally {
      try {
        dependencies.assertRealOpencodeConfigUntouched(
          fixture.realOpencodeConfigSnapshot,
        );
      } finally {
        dependencies.cleanupFixture(fixture.root);
      }
    }
    dependencies.note("headless bundled-sidecar smoke completed");
    return;
  }

  if (options.bundleOnly) {
    dependencies.note("bundle-only mode; launch and fixture checks skipped");
    return;
  }

  let fixture = null;
  let pid = null;
  try {
    fixture = options.fixtureData
      ? dependencies.createFixtureEnvironment()
      : null;
    const env = fixture ? fixtureEnvironment(fixture) : {};
    if (fixture) {
      dependencies.note(`fixture data enabled: ${fixture.root}`);
    }
    dependencies.terminateExistingApp();
    const launched = dependencies.launchApp(env);
    pid = launched.pid;
    if (options.captureWindow) {
      dependencies.captureAppWindow(pid, launched.windowId);
    }
    if (fixture) {
      const status = dependencies.runFixtureServiceSmoke(env);
      dependencies.runFixtureProjectContextSmoke(env, fixture, status);
      dependencies.assertRealOpencodeConfigUntouched(
        fixture.realOpencodeConfigSnapshot,
      );
    }
    if (options.checkLogs) {
      dependencies.checkSystemLogs(pid);
    }
  } finally {
    if (!options.keepOpen) {
      dependencies.terminateExistingApp();
    }
    if (fixture && !options.keepOpen) {
      dependencies.cleanupFixture(fixture.root);
    }
  }
  dependencies.note("native macOS app smoke completed");
}
