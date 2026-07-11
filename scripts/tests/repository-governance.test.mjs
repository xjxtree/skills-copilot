import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import {
  chmodSync,
  copyFileSync,
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
    "markdown-governance-parser.mjs",
    "html-character-entities.mjs",
    "html-character-entities-a-f.mjs",
    "html-character-entities-g-m.mjs",
    "html-character-entities-n-z.mjs",
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
  return root;
}

function runGovernanceFixture(root) {
  return spawnSync(process.execPath, ["scripts/verify-doc-governance.mjs"], {
    cwd: root,
    encoding: "utf8",
  });
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
