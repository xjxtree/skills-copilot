import assert from "node:assert/strict";
import { Buffer } from "node:buffer";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const buildScript = join(repoRoot, "script", "build_and_run.sh");
const shellHelper = join(repoRoot, "script", "path_identity.sh");

function parseEncodedRecord(path) {
  const encodedPath = Buffer.from(path).toString("base64");
  const record = `42\t${encodedPath}`;
  return spawnSync(
    "bash",
    [
      "-c",
      String.raw`source "$1"
rows="$2"
count=0
decoded=""
decoded_pid=""
while IFS=$'\t' read -r pid bundle_path_base64; do
  count=$((count + 1))
  decode_base64_path "$bundle_path_base64"
  decoded="$DECODED_BASE64_PATH"
  decoded_pid="$pid"
done <<<"$rows"
if [[ "$count" == "1" && "$decoded_pid" == "42" && "$decoded" == "$3" ]]; then
  exit 0
fi
printf 'count=%q pid=%q decoded=%q expected=%q\n' "$count" "$decoded_pid" "$decoded" "$3" >&2
exit 1`,
      "build-run-path-record-test",
      shellHelper,
      record,
      path,
    ],
    { encoding: "utf8" },
  );
}

test("base64 process records preserve special bundle paths as one record", () => {
  const paths = [
    "/private/tmp/Agent Copilot.app",
    "/private/tmp/Agent\tCopilot *?[]'$`\\.app",
    "/private/tmp/Agent\nCopilot.app",
    "/private/tmp/AgentCopilot.app\n",
  ];

  for (const path of paths) {
    const encodedPath = Buffer.from(path).toString("base64");
    assert.doesNotMatch(encodedPath, /[\t\n]/);

    const result = parseEncodedRecord(path);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stderr, "");
  }
});

test("build gate encodes every Foundation bundle path before shell parsing", () => {
  const source = readFileSync(buildScript, "utf8");

  assert.equal(
    source.includes("let bundlePathBase64 = Data(bundlePath.utf8).base64EncodedString()"),
    true,
  );
  assert.equal(
    source.includes(String.raw`print("\(app.processIdentifier)\t\(bundlePathBase64)")`),
    true,
  );
  assert.equal(
    (source.match(/while IFS=\$'\\t' read -r pid _?bundle_path_base64; do/g) ?? []).length,
    4,
  );
  assert.equal(
    (source.match(/decode_base64_path "\$bundle_path_base64"/g) ?? []).length,
    3,
  );
  assert.doesNotMatch(source, /read -r pid _?bundle_path; do/);
  assert.equal(
    source.includes(String.raw`print("\(app.processIdentifier)\t\(bundlePath)")`),
    false,
  );
});
