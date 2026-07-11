import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  renameSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import {
  collectDeclaredCreatePaths,
  collectHeadingSlugs,
  collectMarkdownReferences,
  parseGateMembers,
  validateGateMembers,
  validateReferences,
} from "../lib/repository-governance.mjs";
import { validateGovernanceManifest } from "../verify-doc-governance.mjs";

const TEST_REPOSITORY_ROOT = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../..",
);
const VALID_MANIFEST = {
  schema_version: 1,
  policy_documents: ["README.md"],
  required_text: { "README.md": ["Required marker"] },
  forbidden_patterns: ["FORBIDDEN"],
  forbidden_paths: ["blocked"],
  gate: { script: "verify:gate", members: ["verify:a"] },
};

function createGovernanceFixture(manifest = VALID_MANIFEST) {
  const root = mkdtempSync(join(tmpdir(), "agent-copilot-governance-test-"));
  mkdirSync(join(root, "scripts", "lib"), { recursive: true });
  for (const moduleName of [
    "repository-governance.mjs",
    "markdown-ast-governance.mjs",
  ]) {
    copyFileSync(
      join(TEST_REPOSITORY_ROOT, "scripts", "lib", moduleName),
      join(root, "scripts", "lib", moduleName),
    );
  }
  copyFileSync(
    join(TEST_REPOSITORY_ROOT, "scripts", "verify-doc-governance.mjs"),
    join(root, "scripts", "verify-doc-governance.mjs"),
  );
  writeFileSync(
    join(root, "scripts", "repository-governance.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
  writeFileSync(join(root, "README.md"), "# Fixture\n\nRequired marker\n");
  writeFileSync(
    join(root, "package.json"),
    `${JSON.stringify({ scripts: { "verify:gate": "pnpm verify:a" } }, null, 2)}\n`,
  );
  execFileSync("git", ["init", "-q"], { cwd: root });
  execFileSync("git", ["config", "user.email", "fixture@example.invalid"], {
    cwd: root,
  });
  execFileSync("git", ["config", "user.name", "Governance Fixture"], {
    cwd: root,
  });
  execFileSync("git", ["add", "--all"], { cwd: root });
  execFileSync("git", ["commit", "-qm", "fixture"], { cwd: root });
  symlinkSync(
    join(TEST_REPOSITORY_ROOT, "node_modules"),
    join(root, "node_modules"),
    "dir",
  );
  return root;
}

function runGovernanceFixture(root, environment = process.env) {
  return spawnSync(process.execPath, ["scripts/verify-doc-governance.mjs"], {
    cwd: root,
    encoding: "utf8",
    env: environment,
  });
}

function addBrokenMarkdown(root) {
  mkdirSync(join(root, "docs"), { recursive: true });
  writeFileSync(join(root, "docs", "hidden.md"), "[missing](missing.md)\n");
  execFileSync("git", ["add", "docs/hidden.md"], { cwd: root });
  execFileSync("git", ["commit", "-qm", "add broken markdown"], { cwd: root });
}

function createMinimalGitRepository() {
  const root = mkdtempSync(join(tmpdir(), "agent-copilot-git-decoy-"));
  writeFileSync(join(root, "README.md"), "# Decoy\n");
  execFileSync("git", ["init", "-q"], { cwd: root });
  execFileSync("git", ["config", "user.email", "fixture@example.invalid"], {
    cwd: root,
  });
  execFileSync("git", ["config", "user.name", "Governance Fixture"], {
    cwd: root,
  });
  execFileSync("git", ["add", "README.md"], { cwd: root });
  execFileSync("git", ["commit", "-qm", "decoy"], { cwd: root });
  return root;
}

function writeFsmonitorProbe(root, name) {
  const marker = join(root, `${name}.executed`);
  const executable = join(root, `${name}.mjs`);
  writeFileSync(
    executable,
    [
      "#!/usr/bin/env node",
      'import { writeFileSync } from "node:fs";',
      `writeFileSync(${JSON.stringify(marker)}, "executed\\n");`,
      'process.stdout.write("builtin:governance-test\\0");',
      "",
    ].join("\n"),
  );
  chmodSync(executable, 0o755);
  return { executable, marker };
}

function assertMissingMarkdownTarget(markdown, target, line) {
  const source = "docs/index.md";
  const references = collectMarkdownReferences(markdown, source);
  assert.deepEqual(references, [
    {
      source,
      target: `docs/${target}`,
      pathname: `docs/${target}`,
      fragment: "",
      line,
      kind: "markdown",
    },
  ]);
  assert.deepEqual(
    validateReferences({
      references,
      trackedFiles: new Set([source]),
      headingsByFile: new Map(),
    }),
    [`${source}:${line} -> docs/${target} is missing`],
  );
}

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

test("HTML blocks cannot grant plan-local Create allowances", () => {
  const hiddenBlocks = [
    ["pre", "<pre>\n- Create: `scripts/pre.mjs`\n</pre>"],
    ["script", "<script>\n- Create: `scripts/script.mjs`\n</script>"],
    ["comment", "<!--\n- Create: `scripts/comment.mjs`\n-->"],
    ["generic", "<div>\n- Create: `scripts/generic.mjs`\n</div>"],
    ["fenced", "```md\n- Create: `scripts/fenced.mjs`\n```"],
    ["indented", "    - Create: `scripts/indented.mjs`"],
  ];

  for (const [name, block] of hiddenBlocks) {
    assert.deepEqual(
      [...collectDeclaredCreatePaths(
        `${block}\n\nUse \`scripts/${name}.mjs\`.`,
        "docs/superpowers/plans/example.md",
      )],
      [],
      name,
    );
  }

  assert.deepEqual(
    [...collectDeclaredCreatePaths(
      [
        "<span>ordinary inline HTML</span>",
        "",
        "- Create: `scripts/visible.mjs`",
      ].join("\n"),
      "docs/superpowers/plans/example.md",
    )],
    ["scripts/visible.mjs"],
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

test("resolves full collapsed and shortcut reference links", () => {
  const markdown = [
    "[full][Run   Book] [collapsed][] [shortcut] ![asset][image]",
    "",
    "[run book]: <runbooks/app.md#Launch Check> 'Runbook'",
    "[collapsed]: missing.md",
    "[shortcut]: ../README.md",
    "[image]: data:image/png;base64,AA==",
    "[undefined]",
  ].join("\n");

  assert.deepEqual(
    collectMarkdownReferences(markdown, "docs/index.md").map((reference) => ({
      target: reference.target,
      line: reference.line,
    })),
    [
      { target: "docs/runbooks/app.md#Launch Check", line: 1 },
      { target: "docs/missing.md", line: 1 },
      { target: "README.md", line: 1 },
    ],
  );
});

test("unmatched backticks cannot hide links across CommonMark leaf blocks", () => {
  assertMissingMarkdownTarget(
    "`open\n\n[x](missing-after-blank.md)\n`",
    "missing-after-blank.md",
    3,
  );
  assertMissingMarkdownTarget(
    "`open\n# Boundary\n[x](missing-after-heading.md)\n`",
    "missing-after-heading.md",
    3,
  );
});

test("collects a missing inline link whose title spans lines", () => {
  assertMissingMarkdownTarget(
    '[x](missing-multiline-title.md "multi\nline")',
    "missing-multiline-title.md",
    1,
  );
});

test("collects a missing reference whose definition label spans lines", () => {
  assertMissingMarkdownTarget(
    "[Baz][Foo bar]\n\n[Foo\n  bar]: missing-multiline-label.md",
    "missing-multiline-label.md",
    1,
  );
});

test("collects a missing reference with an escaped closing bracket label", () => {
  assertMissingMarkdownTarget(
    "[x][foo\\]]\n\n[foo\\]]: missing-escaped-label.md",
    "missing-escaped-label.md",
    1,
  );
});

test("uses Unicode full case folding for reference labels", () => {
  assertMissingMarkdownTarget(
    "[ẞ]\n\n[SS]: missing-unicode-casefold.md",
    "missing-unicode-casefold.md",
    1,
  );
});

test("reports exact lines after a multiline inline title", () => {
  assertMissingMarkdownTarget(
    '[first](https://example.com "multi\nline") [x](missing-after-title.md)',
    "missing-after-title.md",
    2,
  );
});

test("reports exact lines after a multiline reference label", () => {
  assertMissingMarkdownTarget(
    [
      "[first][foo",
      "bar] [x](missing-after-reference-label.md)",
      "",
      "[foo bar]: https://example.com",
    ].join("\n"),
    "missing-after-reference-label.md",
    2,
  );
});

test("honors container fences and rejects invalid backtick fence info", () => {
  const markdown = [
    "```bad`info",
    "[visible](visible.md)",
    "> ```markdown",
    "> [quoted-hidden](quoted-hidden.md)",
    "> ```",
    "- ~~~ markdown",
    "  [listed-hidden](listed-hidden.md)",
    "  ~~~",
    "~~~ title `is allowed for tilde fences`",
    "[tilde-hidden](tilde-hidden.md)",
    "~~~",
  ].join("\n");

  assert.deepEqual(
    collectMarkdownReferences(markdown, "docs/index.md").map(
      (reference) => reference.target,
    ),
    ["docs/visible.md"],
  );
});

test("container fences end when their quote or list container ends", () => {
  assert.deepEqual(
    collectMarkdownReferences(
      "> ```markdown\n> [quoted-hidden](quoted-hidden.md)\n[after-quote](after-quote.md)",
      "docs/index.md",
    ).map((reference) => reference.target),
    ["docs/after-quote.md"],
  );
  assert.deepEqual(
    collectMarkdownReferences(
      "- ```markdown\n  [listed-hidden](listed-hidden.md)\n[after-list](after-list.md)",
      "docs/index.md",
    ).map((reference) => reference.target),
    ["docs/after-list.md"],
  );
});

test("final3 A exposes a root link after a list-contained fence ends", () => {
  const markdown = [
    "- item",
    "",
    "  ```md",
    "  [hidden](hidden.md)",
    "[go](missing.md)",
  ].join("\n");

  assert.deepEqual(
    collectMarkdownReferences(markdown, "docs/index.md").map(
      (reference) => reference.target,
    ),
    ["docs/missing.md"],
  );
});

test("final3 B keeps a wide ordered-list fence hidden", () => {
  const markdown = [
    "10. ```md",
    "    [hidden](missing.md)",
    "    ```",
    "[go](visible.md)",
  ].join("\n");

  assert.deepEqual(
    collectMarkdownReferences(markdown, "docs/index.md").map(
      (reference) => reference.target,
    ),
    ["docs/visible.md"],
  );
});

test("final3 C does not let a deeper blockquote close an outer fence", () => {
  const markdown = [
    "> ```md",
    ">> ```",
    "> [hidden](missing.md)",
    "> ```",
    "[go](visible.md)",
  ].join("\n");

  assert.deepEqual(
    collectMarkdownReferences(markdown, "docs/index.md").map(
      (reference) => reference.target,
    ),
    ["docs/visible.md"],
  );
});

test("final3 D resolves continued reference destinations and titles", () => {
  const markdown = [
    "[destination][dest] [title][titled]",
    "",
    "[dest]:",
    "  missing-destination.md",
    "[titled]: missing-title.md",
    "  \"Continued [hidden](title-only.md)\"",
  ].join("\n");

  assert.deepEqual(
    collectMarkdownReferences(markdown, "docs/index.md").map(
      (reference) => reference.target,
    ),
    ["docs/missing-destination.md", "docs/missing-title.md"],
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

test("keeps encoded hashes in markdown pathnames separate from anchors", () => {
  const source = "docs/index.md";
  const references = collectMarkdownReferences(
    [
      "[literal hash](tracked.md%23details)",
      "[missing literal hash](missing.md%23details)",
      "[raw anchor](prefix.md#details)",
    ].join("\n"),
    source,
  );

  assert.deepEqual(
    references.map(({ target, pathname, fragment }) => ({
      target,
      pathname,
      fragment,
    })),
    [
      {
        target: "docs/tracked.md#details",
        pathname: "docs/tracked.md#details",
        fragment: "",
      },
      {
        target: "docs/missing.md#details",
        pathname: "docs/missing.md#details",
        fragment: "",
      },
      {
        target: "docs/prefix.md#details",
        pathname: "docs/prefix.md",
        fragment: "details",
      },
    ],
  );
  assert.deepEqual(
    validateReferences({
      references,
      trackedFiles: new Set([
        source,
        "docs/tracked.md#details",
        "docs/missing.md",
        "docs/prefix.md",
      ]),
      headingsByFile: new Map([
        ["docs/missing.md", new Set(["details"])],
        ["docs/prefix.md", new Set(["details"])],
      ]),
    }),
    ["docs/index.md:2 -> docs/missing.md#details is missing"],
  );
});

test("markdown links use literal paths while backticks retain repository globs", () => {
  const source = "docs/index.md";
  const markdown = [
    "[raw star](literal-*.md)",
    "[encoded star](encoded-%2A.md)",
    "[raw question](query-?.md)",
    "[encoded question](encoded-%3F.md)",
    "[raw brackets](literal-[ab].md)",
    "[encoded brackets](encoded-%5Bab%5D.md)",
    "[raw braces](literal-{ab}.md)",
    "[encoded braces](encoded-%7Bab%7D.md)",
    "[valid query](target.md?plain=1#Launch)",
    "`fixtures/api/*.json`",
  ].join("\n");
  const references = collectMarkdownReferences(markdown, source);

  assert.deepEqual(references.map((reference) => reference.target), [
    "docs/literal-*.md",
    "docs/encoded-*.md",
    "docs/query-",
    "docs/encoded-?.md",
    "docs/literal-[ab].md",
    "docs/encoded-[ab].md",
    "docs/literal-{ab}.md",
    "docs/encoded-{ab}.md",
    "docs/target.md#Launch",
    "fixtures/api/*.json",
  ]);

  const wildcardOnlyFiles = new Set([
    source,
    "docs/literal-a.md",
    "docs/encoded-a.md",
    "docs/query-x.md",
    "docs/encoded-x.md",
    "docs/literal-a.md",
    "docs/encoded-a.md",
    "docs/literal-ab.md",
    "docs/encoded-ab.md",
    "docs/target.md",
    "fixtures/api/request.json",
  ]);
  assert.deepEqual(
    validateReferences({
      references,
      trackedFiles: wildcardOnlyFiles,
      headingsByFile: new Map([["docs/target.md", new Set(["launch"])]]),
    }),
    references.slice(0, 8).map(
      (reference) =>
        `${reference.source}:${reference.line} -> ${reference.target} is missing`,
    ),
  );

  const literalFiles = new Set([
    source,
    ...references.slice(0, 8).map((reference) => reference.target),
    "docs/target.md",
    "fixtures/api/request.json",
  ]);
  assert.deepEqual(
    validateReferences({
      references,
      trackedFiles: literalFiles,
      headingsByFile: new Map([["docs/target.md", new Set(["launch"])]]),
    }),
    [],
  );
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

test("resolves duplicate heading suffix collisions like GitHub", () => {
  const markdown = [
    "# Alpha",
    "# Alpha",
    "# Alpha-1",
    "# Alpha",
    "# Alpha-2",
  ].join("\n");
  assert.deepEqual([...collectHeadingSlugs(markdown)], [
    "alpha",
    "alpha-1",
    "alpha-1-1",
    "alpha-2",
    "alpha-2-1",
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

test("builds rendered heading slugs with block context and entities", () => {
  const markdown = [
    "> Not a heading",
    "---",
    "- Also not a heading",
    "---",
    "> Quoted heading",
    "> ===",
    "> # Quoted ATX",
    "# Fish &amp; Chips",
    "# Café &#x26; Tea",
    "# Cafe\u0301",
    "## [Save][action] and `Rollback`",
    "",
    "[action]: actions/save.md",
    "# Fish &amp; Chips",
  ].join("\n");

  assert.deepEqual([...collectHeadingSlugs(markdown)], [
    "quoted-heading",
    "quoted-atx",
    "fish--chips",
    "café--tea",
    "cafe\u0301",
    "save-and-rollback",
    "fish--chips-1",
  ]);
});

test("final3 E renders named entities and intraword underscores in slugs", () => {
  assert.deepEqual(
    [...collectHeadingSlugs("# Caf&eacute;\n# foo_bar")],
    ["café", "foo_bar"],
  );
});

test("uses canonical GitHub slugs for entities and Unicode edge cases", () => {
  const markdown = [
    "# a&nbsp;b",
    "# &#32;Alpha&#32;",
    "# &ensp;Beta&ensp;",
    "# ⓖ",
    "# Hello, world!",
    "# Cafe\u0301",
    "# !!!",
    "# !!!",
  ].join("\n");

  assert.deepEqual([...collectHeadingSlugs(markdown)], [
    "ab",
    "-alpha-",
    "beta",
    "ⓖ",
    "hello-world",
    "cafe\u0301",
    "",
    "-1",
  ]);
});

test("handles deeply nested CommonMark blocks without call-stack overflow", () => {
  const markdown = `${"> ".repeat(10_000)}[x](missing.md)`;
  assert.deepEqual(collectMarkdownReferences(markdown, "docs/index.md"), [
    {
      source: "docs/index.md",
      target: "docs/missing.md",
      pathname: "docs/missing.md",
      fragment: "",
      line: 1,
      kind: "markdown",
    },
  ]);
});

test("renders deeply nested inline headings without call-stack overflow", () => {
  const depth = 4_000;
  const markdown = `# ${"*a ".repeat(depth)}x${" z*".repeat(depth)}`;
  const expected = `${"a-".repeat(depth)}x${"-z".repeat(depth)}`;
  assert.deepEqual([...collectHeadingSlugs(markdown)], [expected]);
});

test("final3 F honors multi-backtick code span delimiters", () => {
  assert.deepEqual(
    collectMarkdownReferences(
      "`` `docs/missing.md` ``",
      "docs/index.md",
    ),
    [],
  );
});

test("create declarations reject non-file and non-repository paths", () => {
  const markdown = [
    "- Create: `scripts/./nested/../future.mjs`",
    "- Create: `scripts\\nested\\..\\windows.mjs`",
    "- Create: `/absolute.mjs`",
    "- Create: `%2Fencoded-absolute.mjs`",
    "- Create: `C:\\absolute.mjs`",
    "- Create: `https://example.com/future.mjs`",
    "- Create: `file:///tmp/future.mjs`",
    "- Create: `scripts/../../outside.mjs`",
    "- Create: `scripts/%2e%2e/%2e%2e/encoded.mjs`",
    "- Create: `scripts%5c..%5c..%5cencoded-backslash.mjs`",
    "- Create: `scripts/future-*.mjs`",
    "- Create: `scripts/future?.mjs`",
    "- Create: `scripts/future/`",
    "- Create: `scripts/future/.`",
    "- Create: `scripts/future/..`",
    "- Create: `scripts/malformed%2.mjs`",
    "- Create: `scripts/control\u0001.mjs`",
    "- Create: `scripts/nul\u0000.mjs`",
  ].join("\n");

  assert.deepEqual(
    [...collectDeclaredCreatePaths(markdown, "docs/superpowers/plans/example.md")],
    ["scripts/future.mjs", "scripts/windows.mjs"],
  );
});

test("requires every gate term to be an exact pnpm script invocation", () => {
  assert.deepEqual(
    parseGateMembers("pnpm verify:a && pnpm run verify:b && pnpm verify:c"),
    ["verify:a", "verify:b", "verify:c"],
  );

  for (const command of [
    "pnpm verify:a && node ignored.mjs",
    "pnpm verify:a || true",
    "pnpm verify:a # skipped",
    "pnpm verify:a ; echo skipped",
    "pnpm verify:a > /dev/null",
    "pnpm verify:a --flag",
    "pnpm verify:a | cat",
    "pnpm verify:a & echo skipped",
    "\u00a0pnpm verify:a",
  ]) {
    assert.throws(
      () => parseGateMembers(command),
      /gate term must be/,
      command,
    );
  }
});

test("rejects gate line terminators before splitting terms", () => {
  assert.deepEqual(
    parseGateMembers("\tpnpm verify:a\t && \tpnpm run verify:b\t"),
    ["verify:a", "verify:b"],
  );
  for (const command of [
    "\npnpm verify:a && pnpm verify:b",
    "pnpm verify:a\r&& pnpm verify:b",
    "pnpm verify:a &&\r\npnpm verify:b",
    "pnpm verify:a && pnpm verify:b\u2028",
    "pnpm verify:a\u2029&& pnpm verify:b",
    "pnpm verify:a \\\n&& pnpm verify:b",
  ]) {
    assert.throws(
      () => parseGateMembers(command),
      /gate command must be a single line/u,
      JSON.stringify(command),
    );
  }
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

test("forbidden governance patterns are Unicode case-insensitive", () => {
  const manifest = {
    ...VALID_MANIFEST,
    forbidden_patterns: [
      "\\bV\\d+\\.\\d+\\b",
      "MVP\\s*(?:/|\\|)\\s*V1",
      "Current (?:Status|State|Baseline)",
      "Completed baseline",
      "Current phase",
    ],
  };
  const cases = [
    [manifest.forbidden_patterns[0], ["v1.2", "V1.2"]],
    [manifest.forbidden_patterns[1], ["mvp / v1", "MvP | v1", "MVP/V1"]],
    [
      manifest.forbidden_patterns[2],
      ["current status", "CURRENT STATE", "CuRrEnT BaSeLiNe"],
    ],
    [
      manifest.forbidden_patterns[3],
      ["completed baseline", "COMPLETED BASELINE", "CoMpLeTeD BaSeLiNe"],
    ],
    [
      manifest.forbidden_patterns[4],
      ["current phase", "CURRENT PHASE", "CuRrEnT PhAsE"],
    ],
  ];
  const root = createGovernanceFixture(manifest);
  try {
    for (const [pattern, variants] of cases) {
      for (const policyText of variants) {
        writeFileSync(
          join(root, "README.md"),
          `# Fixture\n\nRequired marker\n\n${policyText}\n`,
        );
        const result = runGovernanceFixture(root);
        assert.equal(result.status, 1, policyText);
        assert.match(
          result.stderr,
          new RegExp(
            `contains forbidden pattern: ${pattern.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}`,
          ),
          policyText,
        );
      }
    }
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("case-insensitive policy matching does not fold repository paths", () => {
  const manifest = { ...VALID_MANIFEST, forbidden_paths: ["blocked"] };
  const root = createGovernanceFixture(manifest);
  try {
    mkdirSync(join(root, "Blocked"));
    writeFileSync(join(root, "Blocked", "kept.txt"), "case exact\n");
    execFileSync("git", ["add", "Blocked/kept.txt"], { cwd: root });
    const mixedCase = runGovernanceFixture(root);
    assert.equal(mixedCase.status, 0, mixedCase.stderr);

    renameSync(join(root, "Blocked"), join(root, "blocked"));
    const exactCase = runGovernanceFixture(root);
    assert.equal(exactCase.status, 1, exactCase.stdout || exactCase.stderr);
    assert.match(exactCase.stderr, /forbidden path exists: blocked/u);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("Git inventory ignores inherited repository and pathspec selectors", () => {
  const root = createGovernanceFixture();
  const decoy = createMinimalGitRepository();
  try {
    addBrokenMarkdown(root);
    const alternateIndex = join(root, "alternate.index");
    const alternateEnvironment = {
      ...process.env,
      GIT_INDEX_FILE: alternateIndex,
    };
    execFileSync("git", ["read-tree", "--empty"], {
      cwd: root,
      env: alternateEnvironment,
    });
    execFileSync("git", ["add", "README.md"], {
      cwd: root,
      env: alternateEnvironment,
    });

    const attacks = [
      {
        name: "repository redirect",
        environment: {
          GIT_DIR: join(decoy, ".git"),
          GIT_WORK_TREE: decoy,
        },
      },
      {
        name: "alternate index",
        environment: { GIT_INDEX_FILE: alternateIndex },
      },
      {
        name: "literal pathspec",
        environment: { GIT_LITERAL_PATHSPECS: "1" },
      },
      {
        name: "no-glob pathspec",
        environment: { GIT_NOGLOB_PATHSPECS: "1" },
      },
    ];

    for (const attack of attacks) {
      const result = runGovernanceFixture(root, {
        ...process.env,
        ...attack.environment,
      });
      assert.equal(result.status, 1, attack.name);
      assert.match(
        result.stderr,
        /docs\/hidden\.md:1 -> docs\/missing\.md is missing/u,
        attack.name,
      );
    }
  } finally {
    rmSync(root, { recursive: true, force: true });
    rmSync(decoy, { recursive: true, force: true });
  }
});

test("Git inventory disables fsmonitor and inherited trace side effects", () => {
  const root = createGovernanceFixture();
  const results = [];
  const markers = [];
  try {
    addBrokenMarkdown(root);

    const localProbe = writeFsmonitorProbe(root, "local-fsmonitor");
    markers.push(localProbe.marker);
    execFileSync("git", ["config", "core.fsmonitor", localProbe.executable], {
      cwd: root,
    });
    results.push(["repository fsmonitor", runGovernanceFixture(root)]);
    execFileSync("git", ["config", "--unset", "core.fsmonitor"], { cwd: root });

    const globalProbe = writeFsmonitorProbe(root, "global-fsmonitor");
    const globalConfig = join(root, "injected-global.gitconfig");
    writeFileSync(
      globalConfig,
      `[core]\n\tfsmonitor = ${globalProbe.executable}\n`,
    );
    markers.push(globalProbe.marker);
    results.push([
      "global fsmonitor",
      runGovernanceFixture(root, {
        ...process.env,
        GIT_CONFIG_GLOBAL: globalConfig,
      }),
    ]);

    const systemProbe = writeFsmonitorProbe(root, "system-fsmonitor");
    const systemConfig = join(root, "injected-system.gitconfig");
    writeFileSync(
      systemConfig,
      `[core]\n\tfsmonitor = ${systemProbe.executable}\n`,
    );
    markers.push(systemProbe.marker);
    results.push([
      "system fsmonitor",
      runGovernanceFixture(root, {
        ...process.env,
        GIT_CONFIG_SYSTEM: systemConfig,
      }),
    ]);

    const countProbe = writeFsmonitorProbe(root, "count-fsmonitor");
    markers.push(countProbe.marker);
    results.push([
      "command environment fsmonitor",
      runGovernanceFixture(root, {
        ...process.env,
        GIT_CONFIG_COUNT: "1",
        GIT_CONFIG_KEY_0: "core.fsmonitor",
        GIT_CONFIG_VALUE_0: countProbe.executable,
      }),
    ]);

    const trace = join(root, "git-trace.log");
    const trace2 = join(root, "git-trace2.json");
    markers.push(trace, trace2);
    results.push([
      "trace outputs",
      runGovernanceFixture(root, {
        ...process.env,
        GIT_TRACE: trace,
        GIT_TRACE2_EVENT: trace2,
      }),
    ]);

    for (const [name, result] of results) {
      assert.equal(result.status, 1, name);
      assert.match(
        result.stderr,
        /docs\/hidden\.md:1 -> docs\/missing\.md is missing/u,
        name,
      );
    }
    for (const marker of markers) {
      assert.equal(existsSync(marker), false, `${marker} must not be created`);
    }
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("Git inventory discovers linked worktrees after clearing selectors", () => {
  const root = createGovernanceFixture();
  const decoy = createMinimalGitRepository();
  const linkedParent = mkdtempSync(
    join(tmpdir(), "agent-copilot-governance-linked-"),
  );
  const linkedRoot = join(linkedParent, "worktree");
  try {
    addBrokenMarkdown(root);
    execFileSync(
      "git",
      ["worktree", "add", "-q", "-b", "governance-linked", linkedRoot],
      { cwd: root },
    );
    symlinkSync(
      join(TEST_REPOSITORY_ROOT, "node_modules"),
      join(linkedRoot, "node_modules"),
      "dir",
    );

    const result = runGovernanceFixture(linkedRoot, {
      ...process.env,
      GIT_DIR: join(decoy, ".git"),
      GIT_WORK_TREE: decoy,
    });
    assert.equal(result.status, 1, result.stdout || result.stderr);
    assert.match(
      result.stderr,
      /docs\/hidden\.md:1 -> docs\/missing\.md is missing/u,
    );
  } finally {
    try {
      execFileSync("git", ["worktree", "remove", "--force", linkedRoot], {
        cwd: root,
      });
    } catch {
      // A partially created linked fixture is removed below.
    }
    rmSync(linkedParent, { recursive: true, force: true });
    rmSync(root, { recursive: true, force: true });
    rmSync(decoy, { recursive: true, force: true });
  }
});

test("manifest shape failures are deterministic and fail closed", async (t) => {
  const cases = [
    {
      name: "null root",
      manifest: null,
      message: "scripts/repository-governance.json must be a plain object",
    },
    {
      name: "wrong forbidden_patterns type",
      mutate(manifest) {
        manifest.forbidden_patterns = "";
      },
      message:
        "scripts/repository-governance.json.forbidden_patterns must be an array of nonempty strings",
    },
    {
      name: "wrong required_text type",
      mutate(manifest) {
        manifest.required_text = [];
      },
      message:
        "scripts/repository-governance.json.required_text must be a plain object",
    },
    {
      name: "missing gate",
      mutate(manifest) {
        delete manifest.gate;
      },
      message: "scripts/repository-governance.json is missing required key: gate",
    },
    {
      name: "unexpected root key and duplicate member",
      mutate(manifest) {
        manifest.extra = true;
        manifest.gate.members.push("verify:a");
      },
      messages: [
        "scripts/repository-governance.json has unexpected key: extra",
        "scripts/repository-governance.json.gate.members contains duplicate value: verify:a",
      ],
    },
  ];

  for (const fixtureCase of cases) {
    await t.test(fixtureCase.name, () => {
      const manifest = Object.hasOwn(fixtureCase, "manifest")
        ? fixtureCase.manifest
        : structuredClone(VALID_MANIFEST);
      fixtureCase.mutate?.(manifest);
      const root = createGovernanceFixture(manifest);
      try {
        const result = runGovernanceFixture(root);
        assert.equal(result.status, 1, result.stdout || result.stderr);
        for (const message of fixtureCase.messages ?? [fixtureCase.message]) {
          assert.match(result.stderr, new RegExp(message.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
        }
        assert.doesNotMatch(result.stderr, /TypeError|\n\s+at /u);
        assert.doesNotMatch(result.stderr, new RegExp(root.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
      } finally {
        rmSync(root, { recursive: true, force: true });
      }
    });
  }
});

test("manifest validator reports exact-key type and duplicate errors in order", () => {
  assert.deepEqual(validateGovernanceManifest(structuredClone(VALID_MANIFEST)), []);

  const manifest = structuredClone(VALID_MANIFEST);
  manifest.extra = true;
  manifest.policy_documents.push("README.md");
  manifest.required_text["README.md"] = [];
  manifest.forbidden_patterns.push("FORBIDDEN");
  manifest.forbidden_paths = [42];
  manifest.gate.extra = true;
  manifest.gate.script = " ";
  manifest.gate.members.push("verify:a");

  assert.deepEqual(validateGovernanceManifest(manifest), [
    "scripts/repository-governance.json has unexpected key: extra",
    "scripts/repository-governance.json.policy_documents contains duplicate value: README.md",
    "scripts/repository-governance.json.required_text.README.md must be an array of nonempty strings",
    "scripts/repository-governance.json.forbidden_patterns contains duplicate value: FORBIDDEN",
    "scripts/repository-governance.json.forbidden_paths must be an array of nonempty strings",
    "scripts/repository-governance.json.gate has unexpected key: extra",
    "scripts/repository-governance.json.gate.script must be a nonempty string",
    "scripts/repository-governance.json.gate.members contains duplicate value: verify:a",
  ]);
});

test("JSON load failures use stable repository-relative diagnostics", async (t) => {
  const cases = [
    {
      name: "missing manifest",
      prepare(root) {
        rmSync(join(root, "scripts", "repository-governance.json"));
      },
      message: "scripts/repository-governance.json is unreadable",
    },
    {
      name: "invalid manifest JSON",
      prepare(root) {
        writeFileSync(join(root, "scripts", "repository-governance.json"), "{\n");
      },
      message: "scripts/repository-governance.json is invalid JSON",
    },
  ];

  for (const fixtureCase of cases) {
    await t.test(fixtureCase.name, () => {
      const root = createGovernanceFixture();
      try {
        fixtureCase.prepare(root);
        const result = runGovernanceFixture(root);
        assert.equal(result.status, 1, result.stdout || result.stderr);
        assert.match(result.stderr, new RegExp(`- ${fixtureCase.message}`));
        assert.doesNotMatch(
          result.stderr,
          new RegExp(root.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")),
        );
      } finally {
        rmSync(root, { recursive: true, force: true });
      }
    });
  }
});

test("tracked file type failures are stable and deduplicated", async (t) => {
  const cases = [
    {
      name: "symlink policy",
      prepare(root) {
        mkdirSync(join(root, "docs"));
        writeFileSync(join(root, "docs", "safe.md"), "Required marker\n");
        rmSync(join(root, "README.md"));
        symlinkSync("docs/safe.md", join(root, "README.md"));
        execFileSync("git", ["add", "--all"], { cwd: root });
      },
      message: "README.md is not a regular tracked file: index mode 120000",
    },
    {
      name: "gitlink policy",
      prepare(root) {
        const commit = execFileSync("git", ["rev-parse", "HEAD"], {
          cwd: root,
          encoding: "utf8",
        }).trim();
        execFileSync(
          "git",
          ["update-index", "--add", "--cacheinfo", "160000", commit, "README.md"],
          { cwd: root },
        );
      },
      message: "README.md is not a regular tracked file: index mode 160000",
    },
    {
      name: "deleted regular policy",
      prepare(root) {
        rmSync(join(root, "README.md"));
      },
      message: "README.md is missing from the working tree",
    },
    {
      name: "unreadable regular policy",
      prepare(root) {
        chmodSync(join(root, "README.md"), 0o000);
      },
      message: "README.md is unreadable in the working tree",
    },
    {
      name: "directory replaces regular policy",
      prepare(root) {
        rmSync(join(root, "README.md"));
        mkdirSync(join(root, "README.md"));
      },
      message: "README.md is not a regular working-tree file",
    },
    {
      name: "symlinked policy parent",
      manifest: {
        ...VALID_MANIFEST,
        policy_documents: ["docs/policy.md"],
        required_text: { "docs/policy.md": ["Required marker"] },
      },
      prepare(root) {
        mkdirSync(join(root, "docs"));
        writeFileSync(join(root, "docs", "policy.md"), "Required marker\n");
        execFileSync("git", ["add", "docs/policy.md"], { cwd: root });
        execFileSync("git", ["commit", "-qm", "add nested policy"], {
          cwd: root,
        });
        renameSync(join(root, "docs"), join(root, "real-docs"));
        symlinkSync("real-docs", join(root, "docs"));
      },
      message: "docs/policy.md is not a regular working-tree file",
    },
  ];

  for (const fixtureCase of cases) {
    await t.test(fixtureCase.name, () => {
      const root = createGovernanceFixture(fixtureCase.manifest);
      try {
        fixtureCase.prepare(root);
        const result = runGovernanceFixture(root);
        assert.equal(result.status, 1, result.stdout || result.stderr);
        const diagnostics = result.stderr
          .split("\n")
          .filter((line) => line === `- ${fixtureCase.message}`);
        assert.equal(diagnostics.length, 1, result.stderr);
        assert.doesNotMatch(
          result.stderr,
          new RegExp(root.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")),
        );
      } finally {
        try {
          chmodSync(join(root, "README.md"), 0o600);
        } catch {
          // Missing/nonregular fixtures do not need permission restoration.
        }
        rmSync(root, { recursive: true, force: true });
      }
    });
  }
});
