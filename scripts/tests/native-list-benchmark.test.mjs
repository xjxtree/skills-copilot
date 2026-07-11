import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("native list benchmark compiles the pagination model before SkillRecord", async () => {
  const source = await readFile(
    new URL("../benchmark-native-list-model.mjs", import.meta.url),
    "utf8",
  );
  const paginationModel = source.indexOf(
    "apps/macos/Sources/SkillsCopilot/Models/ListCompleteness.swift",
  );
  const skillRecord = source.indexOf(
    "apps/macos/Sources/SkillsCopilot/Models/SkillRecord.swift",
  );

  assert.notEqual(
    paginationModel,
    -1,
    "benchmark swiftc sources must include ListCompleteness.swift",
  );
  assert.ok(
    paginationModel < skillRecord,
    "ListCompleteness.swift must be compiled before its SkillRecord.swift consumer",
  );
});
