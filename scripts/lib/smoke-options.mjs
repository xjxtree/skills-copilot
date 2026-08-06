const smokeFlags = new Map([
  ["--bundle-only", "bundleOnly"],
  ["--fixture-data", "fixtureData"],
  ["--headless-sidecar", "headlessSidecar"],
  ["--keep-open", "keepOpen"],
  ["--check-logs", "checkLogs"],
  ["--allow-stale-app", "allowStaleApp"],
]);

export function parseSmokeOptions(argv, env) {
  const enabled = new Set();
  const argumentsToParse = argv[0] === "--" ? argv.slice(1) : argv;
  for (const argument of argumentsToParse) {
    const property = smokeFlags.get(argument);
    if (!property) {
      throw new Error(`unknown smoke option: ${argument}`);
    }
    enabled.add(property);
  }

  const options = {
    allowStaleApp:
      enabled.has("allowStaleApp") ||
      env.SKILLS_COPILOT_ALLOW_STALE_APP === "1",
    bundleOnly: enabled.has("bundleOnly"),
    checkLogs: enabled.has("checkLogs"),
    fixtureData: enabled.has("fixtureData"),
    headlessSidecar: enabled.has("headlessSidecar"),
    keepOpen: enabled.has("keepOpen"),
  };

  if (options.headlessSidecar && !options.fixtureData) {
    throw new Error("--headless-sidecar requires --fixture-data");
  }

  if (options.headlessSidecar) {
    const incompatible = [
      ["bundleOnly", "--bundle-only"],
      ["keepOpen", "--keep-open"],
      ["checkLogs", "--check-logs"],
    ].find(([property]) => options[property]);
    if (incompatible) {
      throw new Error(
        `--headless-sidecar cannot be combined with ${incompatible[1]}`,
      );
    }
  }

  return options;
}
