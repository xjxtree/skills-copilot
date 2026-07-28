import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import test from "node:test";

const repoRoot = resolve(new URL("../..", import.meta.url).pathname);
const buildScript = readFileSync(
  resolve(repoRoot, "script/build_and_run.sh"),
  "utf8",
);
const verifyScript = readFileSync(
  resolve(repoRoot, "script/verify_macos_distribution.sh"),
  "utf8",
);
const notarizeScript = readFileSync(
  resolve(repoRoot, "script/notarize_macos.sh"),
  "utf8",
);

function run(script, args = []) {
  return spawnSync("bash", [resolve(repoRoot, script), ...args], {
    cwd: repoRoot,
    encoding: "utf8",
  });
}

test("distribution shell scripts pass syntax validation", () => {
  const result = spawnSync(
    "bash",
    [
      "-n",
      resolve(repoRoot, "script/build_and_run.sh"),
      resolve(repoRoot, "script/verify_macos_distribution.sh"),
      resolve(repoRoot, "script/notarize_macos.sh"),
    ],
    { cwd: repoRoot, encoding: "utf8" },
  );
  assert.equal(result.status, 0, result.stderr);
});

test("Developer ID signing is rejected for development builds before build work", () => {
  const result = run("script/build_and_run.sh", [
    "--configuration",
    "debug",
    "--signing-identity",
    "Developer ID Application: Test",
    "--build-only",
  ]);
  assert.equal(result.status, 2);
  assert.match(result.stderr, /requires --configuration release/);
  assert.doesNotMatch(result.stderr, /Compiling|Building/);
});

test("build signing is explicit, inside-out, timestamped, and hardened", () => {
  assert.match(buildScript, /AGENT_COPILOT_SIGNING_IDENTITY/);
  const sidecarIndex = buildScript.indexOf(
    'codesign --force --sign "$SIGNING_IDENTITY" --options runtime --timestamp "$SERVICE_BINARY"',
  );
  const executableIndex = buildScript.indexOf(
    'codesign --force --sign "$SIGNING_IDENTITY" --options runtime --timestamp "$APP_BINARY"',
  );
  const bundleIndex = buildScript.indexOf(
    'codesign --force --sign "$SIGNING_IDENTITY" --options runtime --timestamp "$APP_BUNDLE"',
  );
  assert.ok(sidecarIndex >= 0);
  assert.ok(executableIndex > sidecarIndex);
  assert.ok(bundleIndex > executableIndex);
  assert.match(buildScript, /Authority=Developer ID Application:/);
  assert.match(buildScript, /flags=\.\*runtime/);
  assert.match(buildScript, /security find-identity -p codesigning -v/);
});

test("notarization requires a named Keychain profile", () => {
  const result = run("script/notarize_macos.sh");
  assert.equal(result.status, 2);
  assert.match(result.stderr, /--keychain-profile is required/);
  assert.match(notarizeScript, /notarytool submit/);
  assert.match(notarizeScript, /--keychain-profile "\$KEYCHAIN_PROFILE"/);
});

test("notarization never accepts raw credentials or publishes artifacts", () => {
  for (const forbidden of [
    "--apple-id",
    "--password",
    "--team-id",
    "gh release",
    "curl ",
  ]) {
    assert.doesNotMatch(notarizeScript, new RegExp(forbidden));
  }
  assert.match(notarizeScript, /refusing to overwrite an existing output ZIP/);
  assert.match(notarizeScript, /stapler staple/);
  assert.match(notarizeScript, /--require-notarization/);
  assert.match(notarizeScript, /notarytool log/);
  assert.match(notarizeScript, /accepted but its log contains issues/);
});

test("distribution verification separates local ad-hoc proof from release trust", () => {
  assert.match(verifyScript, /--allow-ad-hoc/);
  assert.match(verifyScript, /Developer ID Application signature is required/);
  assert.match(verifyScript, /hardened runtime is required/);
  assert.match(verifyScript, /stapler validate/);
  assert.match(verifyScript, /spctl --assess/);
  assert.match(verifyScript, /maintainer home path/);
  assert.match(verifyScript, /secure signing timestamp/);
  assert.match(verifyScript, /different Developer ID teams/);
  assert.match(verifyScript, /prohibited release payload/);
  assert.match(verifyScript, /get-task-allow must be absent or false/);

  const result = run("script/verify_macos_distribution.sh", [
    "--allow-ad-hoc",
    "--require-notarization",
  ]);
  assert.equal(result.status, 1);
  assert.match(
    result.stderr,
    /--allow-ad-hoc cannot be combined with --require-notarization/,
  );
});
