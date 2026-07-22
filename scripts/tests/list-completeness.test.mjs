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
  owner: "SessionSidebarPanel",
  source: "preview.sessionRows",
  policy: "paged",
  total_count_source: "preview.totalMatchedCount",
  allowed_limitations: ["safety_budget", "source_changed", "page_failed"],
  control_scope: "body",
  control_anchor: "state: state",
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
    owner: "BatchTogglePreviewSummary",
    source: "preview.affectedSkills",
    policy: "summary_with_expand",
    total_count_source: "preview.affectedSkills.count",
    allowed_limitations: [],
    control_scope: "body",
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
        "struct SessionSidebarPanel: View {",
        '  let fake = """',
        '  .accessibilityIdentifier("sessions.completeness")',
        '  .accessibilityIdentifier("sessions.load-all")',
        '  """',
        '  // .accessibilityIdentifier("sessions.load-all")',
        "  var body: some View {",
        "    ForEach(preview.sessionRows) { row in Text(row.id) }",
        "  }",
        "}",
        "struct DeadControl: View {",
        "  var body: some View {",
        '    Text("Dead").accessibilityIdentifier("sessions.load-all")',
        "  }",
        "}",
      ].join("\n"),
    },
    (repoRoot) => {
      assert.deepEqual(
        verifyListSurfaceInventory(manifest(sessions), { repoRoot }),
        [
          "sessions.sidebar: declared status_id is not attached to an accessibility control in owner SessionSidebarPanel scope body: sessions.completeness",
          "sessions.sidebar: declared full_access_id is not attached to an accessibility control in owner SessionSidebarPanel scope body: sessions.load-all",
        ],
      );
    },
  );
});

test("generated paging identifiers are reachable through a declared footer prefix", () => {
  withRepository(
    {
      [sessions.file]: `struct SessionSidebarPanel: View {
      var body: some View {
      ForEach(preview.sessionRows) { row in Text(row.id) }
      ListCompletenessFooter(
        state: state,
        onLoadMore: loadMore,
        onLoadAll: loadAll,
        onCancel: cancel,
        accessibilityIdentifierPrefix: "sessions"
      )
      .accessibilityIdentifier("sessions.completeness")
      }
      }`,
    },
    (repoRoot) => {
      assert.deepEqual(
        verifyListSurfaceInventory(manifest(sessions), { repoRoot }),
        [],
      );
    },
  );
});

test("Unicode before an owner does not corrupt structural source offsets", () => {
  withRepository(
    {
      [sessions.file]: `let decorative = """
      😀 } struct FakeOwner {
      """
      struct SessionSidebarPanel: View {
      var body: some View {
      ForEach(preview.sessionRows) { row in Text(row.id) }
      ListCompletenessFooter(
        state: state,
        onLoadMore: loadMore,
        onLoadAll: loadAll,
        onCancel: cancel,
        accessibilityIdentifierPrefix: "sessions"
      )
      .accessibilityIdentifier("sessions.completeness")
      }
      }`,
    },
    (repoRoot) => {
      assert.deepEqual(
        verifyListSurfaceInventory(manifest(sessions), { repoRoot }),
        [],
      );
    },
  );
});

test("identifier-returning accessibility helpers are reachable controls", () => {
  const search = {
    id: "global-search.skills",
    file: "ContentView.swift",
    owner: "GlobalSearchResultsOverlay",
    source: "kindResults",
    policy: "summary_with_expand",
    total_count_source: "count(for: kind)",
    allowed_limitations: [],
    control_scope: "body",
    full_access_id: "global-search.skills.view-all",
  };
  withRepository(
    {
      [search.file]: [
        "struct GlobalSearchResultsOverlay: View {",
        "var body: some View {",
        "let kindResults = results.filter { $0.kind == kind }",
        "ForEach(kindResults) { result in Text(result.title) }",
        'Button("View All") {}',
        "  .accessibilityIdentifier(viewAllAccessibilityIdentifier(for: kind))",
        "}",
        "private func viewAllAccessibilityIdentifier(for kind: Kind) -> String {",
        '  return "global-search.skills.view-all"',
        "}",
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
      manifest(
        { policy: "complete" },
        {
          id: "skills.sidebar",
          policy: "complete",
          owner: "SkillSidebarPanel",
          total_count_source: "skills.count",
          allowed_limitations: [],
        },
      ),
    ),
    [
      "list completeness surface is missing id",
      "skills.sidebar: surface is missing file",
      "skills.sidebar: surface is missing source",
    ],
  );
});

test("schema requires owner, total-count source, allowed limitations, and control scope", () => {
  const {
    owner: _,
    total_count_source: __,
    allowed_limitations: ___,
    control_scope: ____,
    ...missing
  } = sessions;
  assert.deepEqual(verifyListSurfaceInventory(manifest(missing)), [
    "sessions.sidebar: surface is missing owner",
    "sessions.sidebar: surface is missing total_count_source",
    "sessions.sidebar: surface is missing allowed_limitations",
    "sessions.sidebar: paged surface is missing control_scope",
  ]);
});

test("paged surfaces require a status accessibility identifier", () => {
  const { status_id: _, ...missingStatus } = sessions;
  assert.deepEqual(verifyListSurfaceInventory(manifest(missingStatus)), [
    "sessions.sidebar: paged surface is missing status_id",
  ]);
});

test("paged surfaces require an exact target control anchor", () => {
  const { control_anchor: _, ...missingAnchor } = sessions;
  assert.deepEqual(verifyListSurfaceInventory(manifest(missingAnchor)), [
    "sessions.sidebar: paged surface is missing control_anchor",
  ]);
});

test("source anchors must exist inside the declared owner", () => {
  withRepository(
    {
      [sessions.file]: [
        "struct SessionSidebarPanel: View {",
        "  var body: some View { Text(\"No rows\") }",
        "}",
      ].join("\n"),
    },
    (repoRoot) => {
      assert.deepEqual(
        verifyListSurfaceInventory(manifest(sessions), { repoRoot }),
        [
          "sessions.sidebar: declared source anchor is not reachable in owner SessionSidebarPanel scope body: preview.sessionRows",
          "sessions.sidebar: declared status_id is not attached to an accessibility control in owner SessionSidebarPanel scope body: sessions.completeness",
          "sessions.sidebar: declared full_access_id is not attached to an accessibility control in owner SessionSidebarPanel scope body: sessions.load-all",
        ],
      );
    },
  );
});

test("source anchors inside string literals are not reachable", () => {
  withRepository(
    {
      [sessions.file]: [
        "struct SessionSidebarPanel: View {",
        "  var body: some View {",
        '    Text("preview.sessionRows")',
        "    ListCompletenessFooter(",
        "      state: state, onLoadMore: {}, onLoadAll: {}, onCancel: {},",
        '      accessibilityIdentifierPrefix: "sessions"',
        "    )",
        '    .accessibilityIdentifier("sessions.completeness")',
        "  }",
        "}",
      ].join("\n"),
    },
    (repoRoot) => {
      assert.deepEqual(
        verifyListSurfaceInventory(manifest(sessions), { repoRoot }),
        [
          "sessions.sidebar: declared source anchor is not reachable in owner SessionSidebarPanel scope body: preview.sessionRows",
        ],
      );
    },
  );
});

test("full-access accessibility identifiers cannot be reused", () => {
  const config = {
    ...sessions,
    id: "config.history",
    owner: "ConfigSidebarPanel",
    source: "selectedSnapshots",
    status_id: "config-history.completeness",
  };
  assert.deepEqual(verifyListSurfaceInventory(manifest(sessions, config)), [
    "duplicate list completeness full_access_id: sessions.load-all",
  ]);
});

test("controls in an unrelated owner member are not reachable", () => {
  withRepository(
    {
      [sessions.file]: [
        "struct SessionSidebarPanel: View {",
        "  var body: some View {",
        "    ForEach(preview.sessionRows) { row in Text(row.id) }",
        "  }",
        "  private func deadControls() -> some View {",
        "    ListCompletenessFooter(",
        "      state: state, onLoadMore: {}, onLoadAll: {}, onCancel: {},",
        '      accessibilityIdentifierPrefix: "sessions"',
        "    )",
        '    .accessibilityIdentifier("sessions.completeness")',
        "  }",
        "}",
      ].join("\n"),
    },
    (repoRoot) => {
      assert.deepEqual(
        verifyListSurfaceInventory(manifest(sessions), { repoRoot }),
        [
          "sessions.sidebar: declared status_id is not attached to an accessibility control in owner SessionSidebarPanel scope body: sessions.completeness",
          "sessions.sidebar: declared full_access_id is not attached to an accessibility control in owner SessionSidebarPanel scope body: sessions.load-all",
        ],
      );
    },
  );
});

test("controls in compile-time false branches are not reachable", () => {
  withRepository(
    {
      [sessions.file]: [
        "struct SessionSidebarPanel: View {",
        "  var body: some View {",
        "    ForEach(preview.sessionRows) { row in Text(row.id) }",
        "    if false {",
        "      ListCompletenessFooter(",
        "        state: state, onLoadMore: {}, onLoadAll: {}, onCancel: {},",
        '        accessibilityIdentifierPrefix: "sessions"',
        "      )",
        '      .accessibilityIdentifier("sessions.completeness")',
        "    }",
        "  }",
        "}",
      ].join("\n"),
    },
    (repoRoot) => {
      assert.deepEqual(
        verifyListSurfaceInventory(manifest(sessions), { repoRoot }),
        [
          "sessions.sidebar: declared status_id is not attached to an accessibility control in owner SessionSidebarPanel scope body: sessions.completeness",
          "sessions.sidebar: declared full_access_id is not attached to an accessibility control in owner SessionSidebarPanel scope body: sessions.load-all",
        ],
      );
    },
  );
});

test("controls in parenthesized compile-time false branches are not reachable", () => {
  withRepository(
    {
      [sessions.file]: [
        "struct SessionSidebarPanel: View {",
        "  var body: some View {",
        "    ForEach(preview.sessionRows) { row in Text(row.id) }",
        "    if (false) {",
        "      ListCompletenessFooter(",
        "        state: state, onLoadMore: {}, onLoadAll: {}, onCancel: {},",
        '        accessibilityIdentifierPrefix: "sessions"',
        "      )",
        '      .accessibilityIdentifier("sessions.completeness")',
        "    }",
        "  }",
        "}",
      ].join("\n"),
    },
    (repoRoot) => {
      assert.deepEqual(
        verifyListSurfaceInventory(manifest(sessions), { repoRoot }),
        [
          "sessions.sidebar: declared status_id is not attached to an accessibility control in owner SessionSidebarPanel scope body: sessions.completeness",
          "sessions.sidebar: declared full_access_id is not attached to an accessibility control in owner SessionSidebarPanel scope body: sessions.load-all",
        ],
      );
    },
  );
});

test("paged status and full access must belong to the same footer", () => {
  withRepository(
    {
      [sessions.file]: [
        "struct SessionSidebarPanel: View {",
        "  var body: some View {",
        "    ForEach(preview.sessionRows) { row in Text(row.id) }",
        "    ListCompletenessFooter(",
        "      state: unrelatedState, onLoadMore: {}, onLoadAll: {}, onCancel: {},",
        '      accessibilityIdentifierPrefix: "unrelated"',
        "    )",
        '    .accessibilityIdentifier("sessions.completeness")',
        "    ListCompletenessFooter(",
        "      state: targetState, onLoadMore: {}, onLoadAll: {}, onCancel: {},",
        '      accessibilityIdentifierPrefix: "sessions"',
        "    )",
        '    .accessibilityIdentifier("other.completeness")',
        "  }",
        "}",
      ].join("\n"),
    },
    (repoRoot) => {
      assert.deepEqual(
        verifyListSurfaceInventory(manifest(sessions), { repoRoot }),
        [
          "sessions.sidebar: declared status_id and full_access_id are not attached to the same target control in owner SessionSidebarPanel scope body",
        ],
      );
    },
  );
});

test("paged control must match the declared target anchor", () => {
  withRepository(
    {
      [sessions.file]: [
        "struct SessionSidebarPanel: View {",
        "  var body: some View {",
        "    ForEach(preview.sessionRows) { row in Text(row.id) }",
        "    ListCompletenessFooter(",
        "      state: unrelatedState, onLoadMore: {}, onLoadAll: {}, onCancel: {},",
        '      accessibilityIdentifierPrefix: "sessions"',
        "    )",
        '    .accessibilityIdentifier("sessions.completeness")',
        "  }",
        "}",
      ].join("\n"),
    },
    (repoRoot) => {
      assert.deepEqual(
        verifyListSurfaceInventory(manifest(sessions), { repoRoot }),
        [
          "sessions.sidebar: declared status_id and full_access_id are not attached to the same target control in owner SessionSidebarPanel scope body",
        ],
      );
    },
  );
});

test("summary control must expand the declared source rather than another collection", () => {
  const summary = {
    id: "target.rows",
    file: "Target.swift",
    owner: "TargetView",
    source: "targetRows",
    policy: "summary_with_expand",
    total_count_source: "targetRows.count",
    allowed_limitations: [],
    control_scope: "body",
    full_access_id: "target.rows.show-all",
  };
  withRepository(
    {
      [summary.file]: [
        "struct TargetView: View {",
        "  var body: some View {",
        "    ForEach(targetRows) { row in Text(row.name) }",
        "    ExpandableSummaryList(",
        "      otherRows,",
        "      visibleLimit: 3,",
        '      accessibilityIdentifier: "target.rows.show-all"',
        "    ) { row in Text(row.name) }",
        "  }",
        "}",
      ].join("\n"),
    },
    (repoRoot) => {
      assert.deepEqual(
        verifyListSurfaceInventory(manifest(summary), { repoRoot }),
        [
          "target.rows: declared full_access_id is not attached to the declared source control in owner TargetView scope body: target.rows.show-all",
        ],
      );
    },
  );
});

test("helper control arguments must bind source and identifier in the same invocation", () => {
  const summary = {
    id: "batch-toggle.items",
    file: "BatchSkillOperationSheet.swift",
    owner: "BatchTogglePreviewSummary",
    source: "preview.affectedSkills",
    policy: "summary_with_expand",
    total_count_source: "preview.affectedSkills.count",
    allowed_limitations: [],
    control_scope: "body",
    full_access_id: "batch-toggle-items.show-all",
  };
  withRepository(
    {
      [summary.file]: [
        "struct BatchTogglePreviewSummary: View {",
        "  var body: some View {",
        "    ForEach(preview.affectedSkills) { row in Text(row.name) }",
        "    BatchToggleItemList(",
        "      items: preview.skippedItems,",
        '      showAllAccessibilityIdentifier: "batch-toggle-items.show-all"',
        "    )",
        "  }",
        "}",
        "struct BatchToggleItemList: View {",
        "  let items: [Item]",
        "  let showAllAccessibilityIdentifier: String",
        "  var body: some View {",
        "    ExpandableSummaryList(",
        "      items, visibleLimit: 3,",
        "      accessibilityIdentifier: showAllAccessibilityIdentifier",
        "    ) { row in Text(row.name) }",
        "  }",
        "}",
      ].join("\n"),
    },
    (repoRoot) => {
      assert.deepEqual(
        verifyListSurfaceInventory(manifest(summary), { repoRoot }),
        [
          "batch-toggle.items: declared full_access_id is not attached to the declared source control in owner BatchTogglePreviewSummary scope body: batch-toggle-items.show-all",
        ],
      );
    },
  );
});

test("unrelated Text and destructive Button identifiers are not full-access controls", () => {
  const summary = {
    id: "batch-toggle.items",
    file: "BatchSkillOperationSheet.swift",
    owner: "BatchTogglePreviewSummary",
    source: "preview.affectedSkills",
    policy: "summary_with_expand",
    total_count_source: "preview.affectedSkills.count",
    allowed_limitations: [],
    control_scope: "body",
    full_access_id: "batch-toggle-items.show-all",
  };
  withRepository(
    {
      [summary.file]: [
        "struct BatchTogglePreviewSummary: View {",
        "  var body: some View {",
        "    ForEach(preview.affectedSkills) { row in Text(row.name) }",
        '    Text("Decorative").accessibilityIdentifier("batch-toggle-items.show-all")',
        '    Button("Delete", role: .destructive) {}',
        '      .accessibilityIdentifier("batch-toggle-items.show-all")',
        "  }",
        "}",
      ].join("\n"),
    },
    (repoRoot) => {
      assert.deepEqual(
        verifyListSurfaceInventory(manifest(summary), { repoRoot }),
        [
          "batch-toggle.items: declared full_access_id is not attached to an accessibility control in owner BatchTogglePreviewSummary scope body: batch-toggle-items.show-all",
        ],
      );
    },
  );
});

test("raw Swift strings cannot authenticate accessibility controls", () => {
  withRepository(
    {
      [sessions.file]: [
        "struct SessionSidebarPanel: View {",
        "  var body: some View {",
        "    ForEach(preview.sessionRows) { row in Text(row.id) }",
        '    let fake = ##"""',
        '      .accessibilityIdentifier("sessions.completeness")',
        '      accessibilityIdentifierPrefix: "sessions"',
        '    """##',
        "  }",
        "}",
      ].join("\n"),
    },
    (repoRoot) => {
      assert.deepEqual(
        verifyListSurfaceInventory(manifest(sessions), { repoRoot }),
        [
          "sessions.sidebar: declared status_id is not attached to an accessibility control in owner SessionSidebarPanel scope body: sessions.completeness",
          "sessions.sidebar: declared full_access_id is not attached to an accessibility control in owner SessionSidebarPanel scope body: sessions.load-all",
        ],
      );
    },
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

test("finds prefix-defined aliases consumed by a formal list", () => {
  const source = [
    "let visible = records.prefix(8)",
    "ForEach(visible) { row in rowView(row) }",
  ].join("\n");
  assert.deepEqual(findUndeclaredPrefixLists(source), [
    "undeclared prefix-defined formal list at line 2",
  ]);
});

test("finds prefix-defined computed properties consumed by a formal list", () => {
  const source = [
    "private var processNotes: [String] {",
    "  Array(values.prefix(3))",
    "}",
    "ForEach(processNotes, id: \\.self) { note in row(note) }",
  ].join("\n");
  assert.deepEqual(findUndeclaredPrefixLists(source), [
    "undeclared prefix-defined formal list at line 4",
  ]);
});

test("finds multiline chained prefix aliases", () => {
  const source = [
    "let visible = records",
    "    .lazy",
    "    .prefix(8)",
    "ForEach(visible) { row in rowView(row) }",
  ].join("\n");
  assert.deepEqual(findUndeclaredPrefixLists(source), [
    "undeclared prefix-defined formal list at line 4",
  ]);
});

test("propagates prefix taint through wrapper aliases", () => {
  const source = [
    "let first = records.prefix(8)",
    "let copied = Array(first)",
    "let visible = copied.map { $0 }",
    "ForEach(visible) { row in rowView(row) }",
  ].join("\n");
  assert.deepEqual(findUndeclaredPrefixLists(source), [
    "undeclared prefix-defined formal list at line 4",
  ]);
});

test("finds formal lists fed by prefix-returning helpers", () => {
  const source = [
    "func visibleRows(_ records: [Row]) -> [Row] {",
    "  Array(records.prefix(8))",
    "}",
    "ForEach(visibleRows(records)) { row in rowView(row) }",
  ].join("\n");
  assert.deepEqual(findUndeclaredPrefixLists(source), [
    "undeclared prefix-defined formal list at line 4",
  ]);
});

test("propagates helper prefix taint through computed properties", () => {
  const source = [
    "func firstRows(_ records: [Row]) -> [Row] { Array(records.prefix(8)) }",
    "var visibleRows: [Row] {",
    "  firstRows(records)",
    "}",
    "ForEach(visibleRows) { row in rowView(row) }",
  ].join("\n");
  assert.deepEqual(findUndeclaredPrefixLists(source), [
    "undeclared prefix-defined formal list at line 5",
  ]);
});

test("finds multiline Array-wrapped prefix aliases", () => {
  const source = [
    "let visible = Array(",
    "  records.prefix(8)",
    ")",
    "ForEach(visible) { row in rowView(row) }",
  ].join("\n");
  assert.deepEqual(findUndeclaredPrefixLists(source), [
    "undeclared prefix-defined formal list at line 4",
  ]);
});

test("finds prefix aliases returned by closure initializers", () => {
  const source = [
    "let visible: [Row] = {",
    "  let first = records.prefix(8)",
    "  return Array(first)",
    "}()",
    "List(visible) { row in rowView(row) }",
  ].join("\n");
  assert.deepEqual(findUndeclaredPrefixLists(source), [
    "undeclared prefix-defined formal list at line 5",
  ]);
});

test("finds tainted collections passed to expandable and dense sinks", () => {
  const source = [
    "struct TargetView: View {",
    "  var truncated: [Row] { Array(records.prefix(8)) }",
    "  var body: some View {",
    "    ExpandableSummaryList(truncated, visibleLimit: 3) { row in rowView(row) }",
    "    let wrapped = Array(truncated)",
    "    DenseDisclosureList(wrapped) { row in rowView(row) }",
    "  }",
    "}",
  ].join("\n");
  assert.deepEqual(findUndeclaredPrefixLists(source), [
    "undeclared prefix-defined formal list at line 4",
    "undeclared prefix-defined formal list at line 6",
  ]);
});

test("same-name aliases in different lexical types and members do not leak taint", () => {
  const source = [
    "struct TruncatedView: View {",
    "  var body: some View {",
    "    let visible = records.prefix(8)",
    "    Text(String(visible.count))",
    "  }",
    "}",
    "struct CompleteView: View {",
    "  var visible: [Row] { records }",
    "  var body: some View {",
    "    ForEach(visible) { row in rowView(row) }",
    "  }",
    "  func unrelated() {",
    "    let rows = records.prefix(2)",
    "    consume(rows)",
    "  }",
    "  func complete() -> some View {",
    "    let rows = records",
    "    return List(rows) { row in rowView(row) }",
    "  }",
    "}",
  ].join("\n");
  assert.deepEqual(findUndeclaredPrefixLists(source), []);
});

test("formal-list syntax inside ordinary multiline and raw strings is ignored", () => {
  const source = [
    'let ordinary = "ForEach(records.prefix(8))"',
    'let multiline = """',
    "ForEach(records.prefix(8))",
    '"""',
    'let raw = #"ForEach(records.prefix(8))"#',
    'let rawMultiline = ##"""',
    "ForEach(records.prefix(8))",
    '"""##',
  ].join("\n");
  assert.deepEqual(findUndeclaredPrefixLists(source), []);
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
  assert.deepEqual(
    findUndeclaredPrefixLists(denseDisclosureSource, {
      relativePath: "apps/macos/Sources/SkillsCopilot/Views/DetailPresentationPrimitives.swift",
    }),
    [],
  );
});

test("a prefix-only component cannot impersonate DenseDisclosureList", () => {
  const fake = `struct DenseDisclosureList<Item, RowContent: View>: View {
    var body: some View {
        ForEach(items.prefix(visibleLimit)) { item in rowContent(item) }
    }
}`;
  assert.deepEqual(findUndeclaredPrefixLists(fake), [
    "undeclared prefix-defined formal list at line 3",
  ]);
});

test("DenseDisclosureList remainder must feed the disclosure ForEach", () => {
  const fake = `struct DenseDisclosureList<Item, RowContent: View>: View {
    var body: some View {
        let unused = items.dropFirst(visibleLimit)
        ForEach(items.prefix(visibleLimit)) { item in rowContent(item) }
        DisclosureGroup(isExpanded: $isExpanded) {
            Text("No remainder")
        }
    }
}`;
  assert.deepEqual(
    findUndeclaredPrefixLists(fake, {
      relativePath: "apps/macos/Sources/SkillsCopilot/Views/DetailPresentationPrimitives.swift",
    }),
    ["undeclared prefix-defined formal list at line 4"],
  );
});

test("DenseDisclosureList must render every remainder item with rowContent", () => {
  const fake = `struct DenseDisclosureList<Item, RowContent: View>: View {
    var body: some View {
        ForEach(Array(items.prefix(visibleLimit).enumerated()), id: \.offset) { _, item in rowContent(item) }
        DisclosureGroup(isExpanded: $isExpanded) {
            ForEach(Array(items.dropFirst(visibleLimit).enumerated()), id: \.offset) { _, item in Text(String(describing: item)) }
        }
    }
}`;
  assert.deepEqual(
    findUndeclaredPrefixLists(fake, {
      relativePath: "apps/macos/Sources/SkillsCopilot/Views/DetailPresentationPrimitives.swift",
    }),
    ["undeclared prefix-defined formal list at line 3"],
  );
});

test("ExpandableSummaryList must expand from its Button action", () => {
  const fake = `struct ExpandableSummaryList<Item: Identifiable, RowContent: View>: View {
    let items: [Item]
    @State private var isExpanded = false
    var body: some View {
        ForEach(visibleItems) { item in rowContent(item) }
        Button("Show All") { doNothing() }
            .accessibilityIdentifier(accessibilityIdentifier)
    }
    private var visibleItems: [Item] {
        isExpanded ? items : Array(items.prefix(visibleLimit))
    }
    private func unused() { isExpanded = true }
}`;
  assert.deepEqual(
    findUndeclaredPrefixLists(fake, {
      relativePath: "apps/macos/Sources/SkillsCopilot/Views/ListCompletenessControls.swift",
    }),
    ["undeclared prefix-defined formal list at line 5"],
  );
});

test("ExpandableSummaryList expansion mutation must belong to the identified Button", () => {
  const fake = `struct ExpandableSummaryList<Item: Identifiable, RowContent: View>: View {
    let items: [Item]
    @State private var isExpanded = false
    var body: some View {
        ForEach(visibleItems) { item in rowContent(item) }
        Button("Show All") { doNothing() }
            .accessibilityIdentifier(accessibilityIdentifier)
        Button("Unrelated") { isExpanded.toggle() }
    }
    private var visibleItems: [Item] {
        isExpanded ? items : Array(items.prefix(visibleLimit))
    }
}`;
  assert.deepEqual(
    findUndeclaredPrefixLists(fake, {
      relativePath: "apps/macos/Sources/SkillsCopilot/Views/ListCompletenessControls.swift",
    }),
    ["undeclared prefix-defined formal list at line 5"],
  );
});

test("DenseDisclosureList does not exempt later undeclared formal lists", () => {
  const mixedSource = `
struct DenseDisclosureList<Item, RowContent: View>: View {
    var body: some View {
        ForEach(Array(items.prefix(visibleLimit).enumerated()), id: \\.offset) { _, item in
            rowContent(item)
        }
        DisclosureGroup(isExpanded: $isExpanded) {
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
    "sessions.detail-messages",
    "sessions.sidebar",
    "skill-events.history",
    "skill-manager.agents",
    "skill-manager.inventory",
    "skill-manager.risks",
    "skill-manager.search",
    "task-cockpit.agents",
    "task-cockpit.blockers",
    "task-cockpit.candidate-alternatives",
    "task-cockpit.decision-reasons",
    "task-cockpit.evidence",
    "task-cockpit.gaps",
    "task-cockpit.inline-details",
    "task-cockpit.process-notes",
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
