import assert from "node:assert/strict";
import test from "node:test";

import {
  collectDeclaredCreatePaths,
  collectHeadingSlugs,
  collectMarkdownReferences,
  parseGateMembers,
  validateGateMembers,
  validateReferences,
} from "../lib/repository-governance.mjs";

test("finds markdown links and backticked repository paths", () => {
  const refs = collectMarkdownReferences(
    "Read [runbook](runbooks/app.md#smoke) and `docs/missing.md`.",
    "docs/index.md",
  );
  assert.deepEqual(refs.map((ref) => ref.target), [
    "docs/runbooks/app.md#smoke",
    "docs/missing.md",
  ]);
});

test("reports missing files and anchors", () => {
  const errors = validateReferences({
    references: [
      { source: "docs/index.md", target: "docs/missing.md", line: 1 },
      { source: "docs/index.md", target: "docs/runbooks/app.md#missing", line: 1 },
    ],
    trackedFiles: new Set(["docs/index.md", "docs/runbooks/app.md"]),
    headingsByFile: new Map([["docs/runbooks/app.md", new Set(["smoke"])]]),
  });
  assert.deepEqual(errors, [
    "docs/index.md:1 -> docs/missing.md is missing",
    "docs/index.md:1 -> docs/runbooks/app.md#missing has no matching anchor",
  ]);
});

test("implementation plans allow declared creates but reject undeclared future paths", () => {
  const source = "docs/superpowers/plans/example.md";
  const markdown = [
    "- Create: `scripts/future.mjs`",
    "Use `scripts/future.mjs` and `scripts/typo.mjs`.",
  ].join("\n");
  const declaredCreates = new Map([
    [source, collectDeclaredCreatePaths(markdown, source)],
  ]);
  const errors = validateReferences({
    references: collectMarkdownReferences(markdown, source),
    trackedFiles: new Set([source]),
    headingsByFile: new Map(),
    declaredCreates,
  });
  assert.deepEqual(errors, [
    "docs/superpowers/plans/example.md:2 -> scripts/typo.mjs is missing",
  ]);
});

test("declared creates do not excuse markdown links or another plan", () => {
  const first = "docs/superpowers/plans/first.md";
  const second = "docs/superpowers/plans/second.md";
  const declaredCreates = new Map([
    [first, new Set(["scripts/future.mjs"])],
  ]);
  const references = [
    ...collectMarkdownReferences(
      "- Create: `scripts/future.mjs`\n[future](../../../scripts/future.mjs)",
      first,
    ),
    ...collectMarkdownReferences("Use `scripts/future.mjs`.", second),
  ];
  assert.deepEqual(
    validateReferences({
      references,
      trackedFiles: new Set([first, second]),
      headingsByFile: new Map(),
      declaredCreates,
    }),
    [
      "docs/superpowers/plans/first.md:2 -> scripts/future.mjs is missing",
      "docs/superpowers/plans/second.md:1 -> scripts/future.mjs is missing",
    ],
  );
});

test("ignores fenced examples and external destinations", () => {
  const markdown = [
    "```md",
    "[missing](missing.md)",
    "`docs/in-a-fence.md`",
    "```",
    "[web](https://example.com/x)",
    "[mail](mailto:owner@example.com)",
    "![inline](data:image/png;base64,AA==)",
    "[local](<runbooks/local app.md#Launch Check>)",
    "`README.md` and `sibling.md` and `cargo test`",
  ].join("\n");
  assert.deepEqual(
    collectMarkdownReferences(markdown, "docs/index.md").map((ref) => ref.target),
    [
      "docs/runbooks/local app.md#Launch Check",
      "README.md",
      "docs/sibling.md",
    ],
  );
});

test("normalizes dot segments and same-document anchors", () => {
  const refs = collectMarkdownReferences(
    "[root](../README.md) [same](#Details)",
    "docs/index.md",
  );
  assert.deepEqual(refs.map((ref) => ref.target), [
    "README.md",
    "docs/index.md#Details",
  ]);
});

test("validates repository globs and strips backticked line locations", () => {
  const references = collectMarkdownReferences(
    "Read `docs/service-protocol.md:100-115`, `fixtures/api/*.json`, and `SKILL.md` records.",
    "docs/index.md",
  );
  assert.deepEqual(references.map((reference) => reference.target), [
    "docs/service-protocol.md",
    "fixtures/api/*.json",
  ]);
  assert.deepEqual(
    validateReferences({
      references,
      trackedFiles: new Set([
        "docs/index.md",
        "docs/service-protocol.md",
        "fixtures/api/request.json",
      ]),
      headingsByFile: new Map(),
    }),
    [],
  );
  assert.deepEqual(
    validateReferences({
      references: [
        {
          source: "docs/index.md",
          target: "fixtures/missing/*.json",
          line: 2,
          kind: "backtick",
        },
      ],
      trackedFiles: new Set(["docs/index.md"]),
      headingsByFile: new Map(),
    }),
    ["docs/index.md:2 -> fixtures/missing/*.json is missing"],
  );
});

test("builds lowercase GitHub-style slugs for ATX, duplicate, and setext headings", () => {
  const markdown = [
    "# Launch Check",
    "## `Save` and [Rollback](rollback.md)",
    "# Launch Check",
    "详细信息",
    "----",
  ].join("\n");
  assert.deepEqual([...collectHeadingSlugs(markdown)], [
    "launch-check",
    "save-and-rollback",
    "launch-check-1",
    "详细信息",
  ]);
});

test("create declarations must be exact and outside fences", () => {
  const markdown = [
    "- Create: `scripts/real.mjs`",
    "- Modify: `scripts/not-create.mjs`",
    "Before - Create: `scripts/not-exact.mjs`",
    "```",
    "- Create: `scripts/in-fence.mjs`",
    "```",
  ].join("\n");
  assert.deepEqual(
    [...collectDeclaredCreatePaths(markdown, "docs/superpowers/plans/example.md")],
    ["scripts/real.mjs"],
  );
});

test("requires every gate term to be a pnpm script", () => {
  assert.deepEqual(
    parseGateMembers("pnpm verify:a && pnpm verify:b --flag && pnpm verify:c"),
    ["verify:a", "verify:b", "verify:c"],
  );
  assert.throws(
    () => parseGateMembers("pnpm verify:a && node ignored.mjs"),
    /gate term must be pnpm <script>/,
  );
});

test("requires exact gate order and membership", () => {
  assert.deepEqual(
    validateGateMembers(["verify:a", "verify:c"], ["verify:a", "verify:b"]),
    [
      "gate members differ: expected verify:a -> verify:b; actual verify:a -> verify:c",
    ],
  );
  assert.deepEqual(validateGateMembers(["verify:a"], ["verify:a"]), []);
});
