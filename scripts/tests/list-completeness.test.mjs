import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";

import {
  findUndeclaredPrefixLists,
  loadListCompletenessManifest,
  verifyListSurfaceInventory,
} from "../lib/list-completeness.mjs";

const sessions = {
  id: "sessions.sidebar",
  file: "apps/macos/Sources/SkillsCopilot/Views/SidebarView.swift",
  source: "session.previewLocalSessions",
  policy: "paged",
  status_id: "sessions.completeness",
  full_access_id: "sessions.load-all",
};

function manifest(...surfaces) {
  return { schema_version: 1, surfaces };
}

function withRepository(files, run) {
  const root = mkdtempSync(join(tmpdir(), "list-completeness-test-"));
  try {
    for (const [relativePath, source] of Object.entries(files)) {
      const path = join(root, relativePath);
      mkdirSync(dirname(path), { recursive: true });
      writeFileSync(path, source);
    }
    return run(root);
  } finally {
    rmSync(root, { force: true, recursive: true });
  }
}

test("paged surfaces require a full-access control", () => {
  const { full_access_id: _, ...missingContinuation } = sessions;
  assert.deepEqual(
    verifyListSurfaceInventory(manifest(missingContinuation)),
    ["sessions.sidebar: paged surface is missing full_access_id"],
  );
});

test("summary surfaces require a full-access control", () => {
  const missingShowAll = {
    id: "batch-toggle.items",
    file: "BatchSkillOperationSheet.swift",
    source: "preview.affectedSkills",
    policy: "summary_with_expand",
  };
  assert.deepEqual(verifyListSurfaceInventory(manifest(missingShowAll)), [
    "batch-toggle.items: summary_with_expand is missing full_access_id",
  ]);
});

test("surface IDs must be unique", () => {
  assert.deepEqual(verifyListSurfaceInventory(manifest(sessions, sessions)), [
    "duplicate list completeness surface id: sessions.sidebar",
  ]);
});

test("unknown policies name the manifest entry", () => {
  assert.deepEqual(
    verifyListSurfaceInventory(
      manifest({ ...sessions, policy: "unbounded_magic" }),
    ),
    [
      "sessions.sidebar: unknown list completeness policy: unbounded_magic",
    ],
  );
});

test("stale manifest paths name the manifest entry", () => {
  withRepository({}, (repoRoot) => {
    assert.deepEqual(
      verifyListSurfaceInventory(manifest(sessions), { repoRoot }),
      [
        "sessions.sidebar: declared file is missing: apps/macos/Sources/SkillsCopilot/Views/SidebarView.swift",
      ],
    );
  });
});

test("paged controls must be reachable accessibility identifiers", () => {
  withRepository(
    {
      [sessions.file]: [
        'let deadStatus = "sessions.completeness"',
        'let deadAction = "sessions.load-all"',
        'Text("Unrelated").accessibilityIdentifier("other.control")',
      ].join("\n"),
    },
    (repoRoot) => {
      assert.deepEqual(
        verifyListSurfaceInventory(manifest(sessions), { repoRoot }),
        [
          "sessions.sidebar: declared status_id is not attached to an accessibility control: sessions.completeness",
          "sessions.sidebar: declared full_access_id is not attached to an accessibility control: sessions.load-all",
        ],
      );
    },
  );
});

test("generated paging identifiers are reachable through a declared footer prefix", () => {
  const generated = { ...sessions, status_id: undefined };
  withRepository(
    {
      [sessions.file]: `ListCompletenessFooter(
        state: state,
        onLoadMore: loadMore,
        onLoadAll: loadAll,
        onCancel: cancel,
        accessibilityIdentifierPrefix: "sessions"
      )`,
    },
    (repoRoot) => {
      assert.deepEqual(
        verifyListSurfaceInventory(manifest(generated), { repoRoot }),
        [],
      );
    },
  );
});

test("identifier-returning accessibility helpers are reachable controls", () => {
  const search = {
    id: "global-search.skills",
    file: "ContentView.swift",
    source: "AppSearchIndex.search",
    policy: "summary_with_expand",
    full_access_id: "global-search.skills.view-all",
  };
  withRepository(
    {
      [search.file]: [
        ".accessibilityIdentifier(viewAllAccessibilityIdentifier(for: kind))",
        "private func viewAllAccessibilityIdentifier(for kind: Kind) -> String {",
        '  return "global-search.skills.view-all"',
        "}",
      ].join("\n"),
    },
    (repoRoot) => {
      assert.deepEqual(
        verifyListSurfaceInventory(manifest(search), { repoRoot }),
        [],
      );
    },
  );
});

test("every surface requires an ID, file, and source declaration", () => {
  assert.deepEqual(
    verifyListSurfaceInventory(
      manifest({ policy: "complete" }, { id: "skills.sidebar", policy: "complete" }),
    ),
    [
      "list completeness surface is missing id",
      "skills.sidebar: surface is missing file",
      "skills.sidebar: surface is missing source",
    ],
  );
});

test("loads a schema-v1 manifest", () => {
  withRepository(
    {
      "scripts/list-completeness-surfaces.json": `${JSON.stringify(
        manifest(sessions),
      )}\n`,
    },
    (repoRoot) => {
      assert.deepEqual(loadListCompletenessManifest({ repoRoot }), manifest(sessions));
    },
  );
});

test("finds undeclared prefix-defined formal lists", () => {
  assert.deepEqual(
    findUndeclaredPrefixLists("ForEach(records.prefix(8)) { row in"),
    ["undeclared prefix-defined formal list at line 1"],
  );
});

test("finds multiline prefix-defined formal list expressions", () => {
  const source = [
    "ForEach(",
    "    records.prefix(8)",
    ") { row in",
    "    rowView(row)",
    "}",
  ].join("\n");
  assert.deepEqual(findUndeclaredPrefixLists(source), [
    "undeclared prefix-defined formal list at line 1",
  ]);
});

test("approved complete-prefix components are not formal-list findings", () => {
  const denseDisclosureSource = `
struct DenseDisclosureList<Item, RowContent: View>: View {
    var body: some View {
        ForEach(Array(items.prefix(visibleLimit).enumerated()), id: \\.offset) { _, item in
            rowContent(item)
        }
        DisclosureGroup(isExpanded: $isExpanded) {
            ForEach(Array(items.dropFirst(visibleLimit).enumerated()), id: \\.offset) { _, item in
                rowContent(item)
            }
        }
    }
}`;
  assert.deepEqual(findUndeclaredPrefixLists(denseDisclosureSource), []);
});

test("DenseDisclosureList does not exempt later undeclared formal lists", () => {
  const mixedSource = `
struct DenseDisclosureList<Item, RowContent: View>: View {
    var body: some View {
        ForEach(Array(items.prefix(visibleLimit).enumerated()), id: \\.offset) { _, item in
            rowContent(item)
        }
        DisclosureGroup {
            ForEach(items.dropFirst(visibleLimit)) { item in rowContent(item) }
        }
    }
}
ForEach(records.prefix(8)) { row in rowView(row) }`;
  assert.deepEqual(findUndeclaredPrefixLists(mixedSource), [
    "undeclared prefix-defined formal list at line 12",
  ]);
});

test("non-list text prefix operations are ignored", () => {
  assert.deepEqual(
    findUndeclaredPrefixLists("let excerpt = String(message.prefix(80))"),
    [],
  );
});

test("repository manifest inventories every planned formal list", () => {
  const expectedIDs = [
    "batch-toggle.items",
    "batch-toggle.skipped-items",
    "batch-toggle.snapshot-targets",
    "catalog.conflicts",
    "catalog.findings",
    "catalog.rules",
    "catalog.skills",
    "config.history",
    "global-search.config-history",
    "global-search.sessions",
    "global-search.skills",
    "markdown.tables",
    "permission.summary",
    "provider.activity",
    "session.top-skills",
    "sessions.sidebar",
    "skill-events.history",
    "skill-manager.agents",
    "skill-manager.installed",
    "skill-manager.local-library",
    "skill-manager.risks",
    "skill-manager.search",
    "task-cockpit.agents",
    "task-cockpit.blockers",
    "task-cockpit.evidence",
    "task-cockpit.gaps",
    "task-cockpit.inline-details",
    "task-cockpit.provider-context",
    "task-cockpit.readiness",
    "task-cockpit.routes",
    "task-cockpit.safety-notes",
    "task-cockpit.sections",
    "task-cockpit.skills",
    "task-cockpit.tasks",
  ];
  const repositoryManifest = loadListCompletenessManifest();
  assert.deepEqual(
    repositoryManifest.surfaces.map((surface) => surface.id).sort(),
    expectedIDs,
  );
  assert.deepEqual(
    verifyListSurfaceInventory(repositoryManifest, {
      repoRoot: join(dirname(new URL(import.meta.url).pathname), "../.."),
    }),
    [],
  );
});

test("repository verifier accepts the governed inventory and prefix scan", () => {
  const result = spawnSync(process.execPath, ["scripts/verify-list-completeness.mjs"], {
    cwd: join(dirname(new URL(import.meta.url).pathname), "../.."),
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /list completeness verification passed/u);
});
