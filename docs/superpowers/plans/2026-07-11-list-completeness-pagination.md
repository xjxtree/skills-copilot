# Complete List Access And Pagination Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every user-visible list complete, continuable, or explicitly incomplete, with no silent presentation or service truncation.

**Architecture:** Add a shared Rust/Swift completeness contract, keyset cursor utilities for dynamic local sources, and pure native accumulators that keep accepted rows visible across paging, cancellation, and failure. Local collections automatically reach a defensible EOF; remote or large collections use bounded continuation controls, and intentionally compact summaries always expose the canonical full collection.

**Tech Stack:** Rust 2021 workspace, serde/serde_json/sha2, SQLite via rusqlite, Swift 5.9/SwiftUI/AppKit on macOS 13+, Node.js ESM repository verifiers, pnpm.

## Global Constraints

- No user-visible formal list may silently omit records.
- Every omitted row must be reachable through Load More, Load All, Show All, or canonical-list navigation, unless a visible typed source/safety limitation prevents it.
- `total_count = nil` must render as “Total unknown”; never substitute the loaded count as a total.
- Load All is repeated bounded page work, never one unbounded request.
- Local-session pages contain at most 100 summaries and never persist raw session content, cursor state, inventory metadata, or detail rows.
- Preserve current scanner file, byte, depth, directory, entry, skill-count, sidecar, and request budgets.
- Preserve existing Skill Manager command preview, explicit network permission, target visibility, telemetry-off environment, redaction, and mutation confirmation boundaries.
- Service changes are additive; existing unpaged methods, `limit`/`offset` requests, fixtures, and older native decoders remain supported.
- Product logic stays in Rust workspace crates; Swift owns transient presentation and request-generation state only.
- Every behavior change follows strict RED → verified RED → minimal GREEN → verified GREEN.
- Every protocol task updates `docs/service-protocol.md`, fixtures, method effects, dispatch coverage, and protocol-drift verification in the same commit.
- Security/deep scanning stays skipped by the user’s explicit instruction; ordinary privacy checks remain required.
- Major/UI/protocol packages run `pnpm check:macos`; every commit/handoff runs `pnpm check:privacy`.

## File And Responsibility Map

- `crates/core/src/list_page.rs`: dependency-light shared completeness enums and page metadata invariants.
- `crates/service/src/service_keyset_cursor.rs`: versioned request-bound opaque cursor encoding/decoding and source-revision validation.
- `apps/macos/Sources/SkillsCopilot/Models/ListCompleteness.swift`: pure page, accumulator, state, and action models.
- `apps/macos/Sources/SkillsCopilot/Views/ListCompletenessControls.swift`: reusable badge, footer, paging buttons, and expandable summary.
- Domain service/model/store files: only translate domain rows and own request generations.
- `scripts/list-completeness-surfaces.json`: governed inventory of formal lists.
- `scripts/lib/list-completeness.mjs`: manifest and source-policy verification helpers.
- `scripts/verify-list-completeness.mjs`: repository entrypoint for the deterministic completeness gate.

---

### Task 1: Add the shared completeness contract and native accumulator

**Files:**
- Create: `crates/core/src/list_page.rs`
- Modify: `crates/core/src/lib.rs`
- Modify: `crates/core/Cargo.toml`
- Create: `crates/core/tests/list_page.rs`
- Create: `apps/macos/Sources/SkillsCopilot/Models/ListCompleteness.swift`
- Create: `apps/macos/Sources/SkillsCopilot/Views/ListCompletenessControls.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Support/UIStrings.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Resources/en.lproj/Localizable.strings`
- Modify: `apps/macos/Sources/SkillsCopilot/Resources/zh-Hans.lproj/Localizable.strings`
- Create: `apps/macos/Tests/SkillsCopilotTests/ListCompletenessModelTests.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/NativeModelTestRunner.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/FullNativeModelSuiteTests.swift`
- Modify: `scripts/test-macos-native-models.sh`
- Modify: `scripts/verify-macos-native-test-registry.mjs`
- Modify: `scripts/tests/macos-native-test-registry.test.mjs`
- Modify: `scripts/verify-native-ui-layout.mjs`

**Interfaces:**
- Produces Rust `ListSourceCompleteness`, `ListIncompleteReason`, and `ListPageMetadata::validate(returned_len)`.
- Produces Swift `ListPage<Item>`, `ListPageAccumulator<Item>`, `ListCompletenessState`, and `ListLoadingPhase`.
- Produces `ListCompletenessBadge`, `ListCompletenessFooter`, `ListPagingActions`, and `ExpandableSummaryList` for later UI tasks.

- [ ] **Step 1: Write Rust contract tests before adding the types**

Create `crates/core/tests/list_page.rs` with the exact behaviors:

```rust
use skills_copilot_core::{
    ListIncompleteReason, ListPageMetadata, ListSourceCompleteness,
};

#[test]
fn page_metadata_requires_returned_count_to_match_rows() {
    let page = ListPageMetadata::enumerable(2, Some(5), Some("v1:next".into()));
    assert_eq!(page.validate(1), Err("returned_count does not match rows"));
}

#[test]
fn enumerable_more_page_requires_cursor() {
    let page = ListPageMetadata {
        returned_count: 2,
        total_count: Some(5),
        has_more: true,
        next_cursor: None,
        source_completeness: ListSourceCompleteness::Enumerable,
        incomplete_reason: None,
    };
    assert_eq!(page.validate(2), Err("enumerable page with more rows requires next_cursor"));
}

#[test]
fn source_limited_page_is_honest_without_cursor() {
    let page = ListPageMetadata::incomplete(
        8,
        None,
        ListIncompleteReason::SourceLimited,
    );
    assert_eq!(page.validate(8), Ok(()));
    assert_eq!(page.source_completeness, ListSourceCompleteness::Limited);
}
```

- [ ] **Step 2: Run the Rust tests and verify RED**

Run:

```sh
cargo test -p skills-copilot-core --test list_page -- --nocapture
```

Expected: compilation fails because `list_page` and its exported types do not exist.

- [ ] **Step 3: Implement the minimal Rust contract**

Add `serde.workspace = true` to `crates/core/Cargo.toml`, export `list_page` from `lib.rs`, and implement:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ListSourceCompleteness { Enumerable, Limited, Unknown }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ListIncompleteReason {
    SafetyBudget,
    SourceChanged,
    SourceLimited,
    UnreadableSource,
    PageFailed,
    UnsupportedProtocol,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListPageMetadata {
    pub returned_count: usize,
    pub total_count: Option<usize>,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub source_completeness: ListSourceCompleteness,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incomplete_reason: Option<ListIncompleteReason>,
}

impl ListPageMetadata {
    pub fn enumerable(returned_count: usize, total_count: Option<usize>, next_cursor: Option<String>) -> Self {
        Self { returned_count, total_count, has_more: next_cursor.is_some(), next_cursor,
            source_completeness: ListSourceCompleteness::Enumerable, incomplete_reason: None }
    }

    pub fn incomplete(returned_count: usize, total_count: Option<usize>, reason: ListIncompleteReason) -> Self {
        Self { returned_count, total_count, has_more: false, next_cursor: None,
            source_completeness: ListSourceCompleteness::Limited, incomplete_reason: Some(reason) }
    }

    pub fn validate(&self, returned_len: usize) -> Result<(), &'static str> {
        if self.returned_count != returned_len { return Err("returned_count does not match rows"); }
        if self.has_more && self.source_completeness == ListSourceCompleteness::Enumerable && self.next_cursor.is_none() {
            return Err("enumerable page with more rows requires next_cursor");
        }
        if !self.has_more && self.next_cursor.is_some() { return Err("terminal page cannot expose next_cursor"); }
        Ok(())
    }
}
```

- [ ] **Step 4: Write Swift accumulator RED tests**

Create and register `ListCompletenessModelTests` with:

```swift
struct ListCompletenessModelTests {
    struct Row: Identifiable, Equatable { let id: String; let value: String }

    func run() throws {
        try appendsPagesWithoutDuplicateIDs()
        try rejectsChangedSourceRevision()
        try knownTotalCompletesOnlyAtEOF()
        try unknownTotalNeverInventsKnownTotal()
        try cancellationKeepsAcceptedRowsPartial()
    }

    private func appendsPagesWithoutDuplicateIDs() throws {
        var value = ListPageAccumulator<Row>()
        try value.append(ListPage(items: [.init(id: "a", value: "A"), .init(id: "b", value: "B")],
            returnedCount: 2, totalCount: 3, hasMore: true, nextCursor: "next", sourceRevision: "r1",
            sourceCompleteness: .enumerable, incompleteReason: nil))
        try value.append(ListPage(items: [.init(id: "b", value: "duplicate"), .init(id: "c", value: "C")],
            returnedCount: 2, totalCount: 3, hasMore: false, nextCursor: nil, sourceRevision: "r1",
            sourceCompleteness: .enumerable, incompleteReason: nil))
        try expectEqual(value.items.map(\.id), ["a", "b", "c"], "Stable IDs should deduplicate pages")
        try expectEqual(value.state.completeness, .complete, "EOF plus known total should complete")
    }
}
```

Add the remaining methods exactly as follows:

```swift
private func rejectsChangedSourceRevision() throws {
    var value = ListPageAccumulator<Row>()
    try value.append(.init(items: [.init(id: "a", value: "A")], returnedCount: 1,
        totalCount: 2, hasMore: true, nextCursor: "next", sourceRevision: "r1",
        sourceCompleteness: .enumerable, incompleteReason: nil))
    do {
        try value.append(.init(items: [.init(id: "b", value: "B")], returnedCount: 1,
            totalCount: 2, hasMore: false, nextCursor: nil, sourceRevision: "r2",
            sourceCompleteness: .enumerable, incompleteReason: nil))
        throw NativeModelTestFailure(description: "Changed revision should fail")
    } catch ListPageAccumulatorError.sourceChanged {}
    try expectEqual(value.items.map(\.id), ["a"], "Rejected page must not mutate rows")
}

private func knownTotalCompletesOnlyAtEOF() throws {
    var value = ListPageAccumulator<Row>()
    try value.append(.init(items: [.init(id: "a", value: "A")], returnedCount: 1,
        totalCount: 2, hasMore: true, nextCursor: "next", sourceRevision: "r1",
        sourceCompleteness: .enumerable, incompleteReason: nil))
    try expectEqual(value.state.completeness, .partial, "Nonterminal page must stay partial")
}

private func unknownTotalNeverInventsKnownTotal() throws {
    var value = ListPageAccumulator<Row>()
    try value.append(.init(items: [.init(id: "a", value: "A")], returnedCount: 1,
        totalCount: nil, hasMore: false, nextCursor: nil, sourceRevision: "r1",
        sourceCompleteness: .enumerable, incompleteReason: nil))
    try expectNil(value.state.totalCount, "Unknown total must stay nil")
    try expectEqual(value.state.completeness, .complete, "Defensible EOF can complete unknown total")
}

private func cancellationKeepsAcceptedRowsPartial() throws {
    var value = ListPageAccumulator<Row>()
    value.begin(.all)
    try value.append(.init(items: [.init(id: "a", value: "A")], returnedCount: 1,
        totalCount: 2, hasMore: true, nextCursor: "next", sourceRevision: "r1",
        sourceCompleteness: .enumerable, incompleteReason: nil))
    value.cancel()
    try expectEqual(value.items.map(\.id), ["a"], "Cancel must retain accepted rows")
    try expectEqual(value.state.completeness, .partial, "Cancelled continuation is partial")
}
```

- [ ] **Step 5: Run native tests and verify RED**

Run:

```sh
pnpm test:macos-native-models
```

Expected: compilation fails because the Swift completeness types do not exist and the registry count has not been updated.

- [ ] **Step 6: Implement the Swift pure models and common controls**

Implement `ListCompleteness.swift` with these exact public-internal signatures:

```swift
enum ListSourceCompleteness: String, Codable, Hashable { case enumerable, limited, unknown }
enum ListIncompleteReason: String, Codable, Hashable {
    case safetyBudget = "safety_budget", sourceChanged = "source_changed", sourceLimited = "source_limited"
    case unreadableSource = "unreadable_source", pageFailed = "page_failed", unsupportedProtocol = "unsupported_protocol"
}
enum ListCompleteness: Equatable { case complete, partial, incomplete, unknown }
enum ListLoadingPhase: Equatable { case idle, initial, more, all }

struct ListCompletenessState: Equatable {
    let loadedCount: Int
    let totalCount: Int?
    let hasMore: Bool
    let isComplete: Bool
    let completeness: ListCompleteness
    let incompleteReason: ListIncompleteReason?
    let loadingPhase: ListLoadingPhase
    let canLoadMore: Bool
    let canLoadAll: Bool
}

struct ListPage<Item> {
    let items: [Item]; let returnedCount: Int; let totalCount: Int?; let hasMore: Bool
    let nextCursor: String?; let sourceRevision: String?
    let sourceCompleteness: ListSourceCompleteness; let incompleteReason: ListIncompleteReason?
}

enum ListPageAccumulatorError: Error, Equatable { case sourceChanged; case invalidPage }

struct ListPageAccumulator<Item: Identifiable> where Item.ID: Hashable {
    private(set) var items: [Item] = []
    private(set) var nextCursor: String?
    private(set) var sourceRevision: String?
    private(set) var totalCount: Int?
    private(set) var sourceCompleteness: ListSourceCompleteness = .unknown
    private(set) var incompleteReason: ListIncompleteReason?
    private(set) var loadingPhase: ListLoadingPhase = .idle
    private var seenIDs = Set<Item.ID>()
    mutating func append(_ page: ListPage<Item>) throws
    mutating func begin(_ phase: ListLoadingPhase)
    mutating func cancel()
    mutating func fail(reason: ListIncompleteReason)
    var state: ListCompletenessState { get }
}
```

`append` rejects a different non-nil source revision, rejects `returnedCount != items.count`, appends first-seen IDs in page order, and computes completion only after enumerable EOF. Implement the four SwiftUI controls in `ListCompletenessControls.swift` with stable IDs `list-completeness.badge`, `list-completeness.load-more`, `list-completeness.load-all`, `list-completeness.cancel`, and `list-completeness.show-all`.

- [ ] **Step 7: Verify GREEN and commit Task 1**

Run:

```sh
cargo test -p skills-copilot-core --test list_page
pnpm test:macos-native-models
swift test --package-path apps/macos
pnpm verify:macos-native-test-registry
pnpm verify:macos-ui-layout
pnpm check:privacy
```

Expected: all pass; native registry reports `service=2 main=24 skill-store-groups=64 named=90`.

Commit:

```sh
git add crates/core apps/macos scripts/test-macos-native-models.sh scripts/verify-macos-native-test-registry.mjs scripts/tests/macos-native-test-registry.test.mjs scripts/verify-native-ui-layout.mjs Cargo.lock
git commit -m "feat: add shared list completeness state"
```

---

### Task 2: Page local histories and expose catalog scan completeness

**Files:**
- Modify: `crates/catalog/src/queries.rs`
- Modify: `crates/commands/src/lib.rs`
- Create: `crates/service/src/service_keyset_cursor.rs`
- Modify: `crates/service/src/lib.rs`
- Modify: `crates/service/src/service_host.rs`
- Modify: `crates/service/src/tests.rs`
- Create: `crates/service/src/tests/list_page_local_catalog.rs`
- Modify: `crates/service/src/tests/dispatch_fixtures.rs`
- Modify: `crates/service/src/tests/protocol_fixtures.rs`
- Modify: `crates/service/src/tests/method_effects.rs`
- Modify: `fixtures/service-protocol/method-effects.json`
- Create: `fixtures/service-protocol/snapshot.listAgentConfigPage.request.json`
- Create: `fixtures/service-protocol/snapshot.listAgentConfigPage.response.json`
- Create: `fixtures/service-protocol/skill.listEventsPage.request.json`
- Create: `fixtures/service-protocol/skill.listEventsPage.response.json`
- Modify: `docs/service-protocol.md`
- Modify: `apps/macos/Sources/SkillsCopilot/Models/AgentConfigTimelineModel.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Models/SkillRecord.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Services/ServiceClient.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Services/ServiceClientCatalogConfigRPC.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/SidebarView.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/AgentConfigTimelineModelTests.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/ServiceClientRPCTests.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift`

**Interfaces:**
- Consumes Task 1 `ListPageMetadata` and `ListPageAccumulator`.
- Produces `snapshot.listAgentConfigPage` and `skill.listEventsPage` with keyset cursor order `(created_at DESC, id DESC)` and `(occurred_at DESC, id DESC)`.
- Produces store methods `loadMoreAgentConfigSnapshots(loadAll:)`, `cancelAgentConfigSnapshotLoadAll()`, `loadMoreSkillEvents(instanceID:loadAll:)`, and `cancelSkillEventLoadAll(instanceID:)`.
- Produces `catalogListCompleteness`: unknown after a catalog-only startup/reload, complete after a fully enumerated Scan, and incomplete after any partial/skipped/budget-exhausted scan report.

- [ ] **Step 1: Write failing Rust keyset and mutation tests**

Add `config_and_event_pages_match_legacy_order` with this assertion body after seeding six equal-timestamp snapshots and events through the existing catalog fixture helpers:

```rust
let legacy_snapshots = list_agent_config_snapshots(&catalog, "claude-code", None)?;
let paged_snapshots = collect_config_pages(&host, "claude-code", 2)?;
assert_eq!(
    paged_snapshots.iter().map(|row| &row.id).collect::<Vec<_>>(),
    legacy_snapshots.iter().map(|row| &row.id).collect::<Vec<_>>(),
);
let legacy_events = list_skill_events(&catalog, &instance_id, None)?;
let paged_events = collect_event_pages(&host, &instance_id, 2)?;
assert_eq!(
    paged_events.iter().map(|row| &row.id).collect::<Vec<_>>(),
    legacy_events.iter().map(|row| &row.id).collect::<Vec<_>>(),
);
assert_eq!(paged_snapshots.iter().map(|row| &row.id).collect::<HashSet<_>>().len(), 6);
assert_eq!(paged_events.iter().map(|row| &row.id).collect::<HashSet<_>>().len(), 6);
```

Add `changed_catalog_revision_rejects_continuation` with this mutation assertion:

```rust
let first = host.dispatch_value("snapshot.listAgentConfigPage", json!({
    "agent": "claude-code", "limit": 2
}))?;
seed_agent_config_snapshot(&catalog, "inserted-after-first-page", 1_900_000_000_000)?;
let error = host.dispatch_value("snapshot.listAgentConfigPage", json!({
    "agent": "claude-code", "limit": 2,
    "cursor": first["next_cursor"], "source_revision": first["source_revision"]
})).expect_err("changed source must reject continuation");
assert_eq!(error.code(), "source_changed");
```

- [ ] **Step 2: Verify Rust RED**

Run:

```sh
cargo test -p skills-copilot-service tests::list_page_local_catalog -- --nocapture
```

Expected: FAIL because the methods, params, cursor module, and fixtures do not exist.

- [ ] **Step 3: Implement catalog keyset queries and service cursor validation**

Add catalog methods with a `limit + 1` query and stable predicate:

```sql
WHERE agent = ?1
  AND (?2 IS NULL OR scope = ?2)
  AND reason IN ('pre-toggle', 'pre-batch-toggle', 'pre-config-edit')
  AND (?3 IS NULL OR created_at < ?3 OR (created_at = ?3 AND id < ?4))
ORDER BY created_at DESC, id DESC
LIMIT ?5
```

`service_keyset_cursor.rs` defines:

```rust
#[derive(Serialize, Deserialize)]
pub(crate) struct KeysetCursor {
    version: u8,
    method: String,
    query_digest: String,
    source_revision: String,
    sort_value: i64,
    stable_id: String,
    tie_breaker_digest: Option<String>,
}
pub(crate) fn encode_cursor(value: &KeysetCursor) -> Result<String, ServiceError>;
pub(crate) fn decode_cursor(text: &str, method: &str, query_digest: &str) -> Result<KeysetCursor, ServiceError>;
```

Encode canonical JSON bytes as lowercase hexadecimal prefixed by `v1:`. Clamp page limits to `1...100`, compute a SHA-256 revision over ordered `(id,timestamp)` metadata, and return `source_changed` before any page rows when the supplied revision differs.

- [ ] **Step 4: Add additive dispatch, effects, protocol docs, and exact fixtures**

Define these wire types in `crates/service/src/lib.rs`:

```rust
pub struct ListAgentConfigPageParams { pub agent: String, pub scope: Option<String>, pub limit: Option<usize>, pub cursor: Option<String>, pub source_revision: Option<String> }
pub struct ConfigSnapshotPageResult { pub records: Vec<ConfigSnapshotRecord>, pub source_revision: String, #[serde(flatten)] pub page: ListPageMetadata }
pub struct ListSkillEventsPageParams { pub instance_id: String, pub limit: Option<usize>, pub cursor: Option<String>, pub source_revision: Option<String> }
pub struct SkillEventPageResult { pub records: Vec<SkillEventRecord>, pub source_revision: String, #[serde(flatten)] pub page: ListPageMetadata }
```

Register both methods as read-only and side-effect free. Fixtures use two returned rows, known total three, `has_more=true`, and a nonempty redacted opaque cursor.

- [ ] **Step 5: Write Swift RED tests for auto-complete local histories**

Add these assertions to `SkillStoreTests` after the fake returns three config pages and three event pages:

```swift
try expectEqual(store.agentConfigSnapshots.count, 205, "Config history should auto-load every local page")
try expectEqual(store.agentConfigSnapshotCompleteness.loadedCount, 205, "Loaded count")
try expectEqual(store.agentConfigSnapshotCompleteness.completeness, .complete, "Config completeness")
try expectEqual(store.agentConfigTimeline.items.count, 205, "Timeline must not cap at five")
```

For events, seed 201 rows and assert every stable ID is present. Add a second-page failure fixture and assert first-page rows remain visible with `.partial` and retry continues from the accepted cursor.
Start each local Load All against a delayed third page, cancel it, release the page, and assert the accepted IDs/count remain unchanged and the state stays partial.

Add scan-completeness assertions:

```swift
await store.loadAppStartupDataIfNeeded()
try expectEqual(store.catalogListCompleteness.completeness, .unknown, "Catalog-only startup cannot prove scan completeness")
await store.scanAll()
try expectEqual(store.catalogListCompleteness.completeness, .complete, "Complete scan should prove catalog completeness")
fake.activate(scenario: "partial-scan-budget")
await store.scanAll()
try expectEqual(store.catalogListCompleteness.completeness, .incomplete, "Budgeted scan must be visible as incomplete")
try expectEqual(store.catalogListCompleteness.incompleteReason, .safetyBudget, "Budget reason")
```

- [ ] **Step 6: Verify Swift RED**

Run:

```sh
pnpm test:macos-native-models
```

Expected: failures at the five-row timeline assertion and absent page RPC/model types.

- [ ] **Step 7: Implement native local-history paging**

Add RPCs with these signatures:

```swift
func listAgentConfigSnapshotPage(agent: String, scope: String?, limit: Int = 100,
    cursor: String?, sourceRevision: String?) async throws -> ConfigSnapshotPageResult
func listSkillEventPage(instanceID: String, limit: Int = 100,
    cursor: String?, sourceRevision: String?) async throws -> SkillEventPageResult
```

Then:

- decode page metadata with snake/camel aliases;
- issue pages serially with limit 100;
- use one accumulator and generation per selected agent/skill;
- preserve rows on cancellation or failure;
- render all config timeline items in a lazy stack;
- render `ListCompletenessFooter` while loading or incomplete;
- remove `AgentConfigTimelineModel.visibleLimit` and `hiddenCount`.
- initialize catalog-backed Skills/Findings/Conflicts completeness as unknown on catalog-only load, then derive complete/incomplete and privacy-safe recovery copy from `ScanResult.activity` after explicit Scan.

- [ ] **Step 8: Verify and commit Task 2**

Run:

```sh
cargo test -p skills-copilot-service tests::list_page_local_catalog
cargo test -p skills-copilot-service tests::protocol_fixtures::service_protocol_fixtures_decode -- --exact
cargo test -p skills-copilot-service tests::method_effects -- --nocapture
pnpm verify:service-protocol-drift
pnpm test:macos-native-models
pnpm verify:macos-ui-layout
cargo clippy --workspace --all-targets --all-features -- -D warnings
pnpm check:privacy
```

Commit:

```sh
git add crates/catalog crates/commands crates/service apps/macos fixtures/service-protocol docs/service-protocol.md
git commit -m "feat: expose complete config and event histories"
```

---

### Task 3: Continue local sessions beyond the 800-row prewarm boundary

**Files:**
- Modify: `crates/service/src/lib.rs`
- Modify: `crates/service/src/service_local_session_io.rs`
- Modify: `crates/service/src/service_local_sessions.rs`
- Modify: `crates/service/src/tests/local_session_inventory.rs`
- Modify: `crates/service/src/tests/local_session_summary_detail.rs`
- Modify: `crates/service/src/tests/protocol_fixtures.rs`
- Modify: `crates/service/src/tests/method_effects.rs`
- Modify: `fixtures/service-protocol/session.previewLocalSessions.request.json`
- Modify: `fixtures/service-protocol/session.previewLocalSessions.response.json`
- Modify: `docs/service-protocol.md`
- Modify: `docs/adapters/agent-adapters.md`
- Modify: `apps/macos/Sources/SkillsCopilot/Models/LocalSessionPreview.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Models/LocalSessionCache.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Services/ServiceClient.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Services/ServiceClientSessionRPC.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/SidebarView.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/AgentSessionDetailPanel.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/LocalSessionCacheTests.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/ServiceClientRPCTests.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift`

**Interfaces:**
- Consumes Task 1 completeness models and Task 2 cursor encoding.
- Extends `session.previewLocalSessions` with optional `cursor` and `source_revision`, plus response `next_cursor`, `source_revision`, `source_completeness`, and `incomplete_reason`.
- Produces `loadMoreLocalSessions()`, `loadAllLocalSessions()`, and `cancelLocalSessionLoadAll()`.

- [ ] **Step 1: Add RED tests for 1,205 candidates, stable continuation, and budget truncation**

Create 1,205 metadata-only session fixtures and request pages of 100. Assert:

```rust
assert_eq!(all_ids.len(), 1_205);
assert_eq!(all_ids.iter().collect::<HashSet<_>>().len(), 1_205);
assert_eq!(all_ids, unpaged_expected_ids);
assert!(!final_page.has_more);
assert_eq!(final_page.page.incomplete_reason, None);
```

Mutate one candidate after page one and require `source_changed`. Run a separate low-directory-budget fixture and require `candidate_set_truncated=true`, `source_completeness=limited`, and `incomplete_reason=safety_budget` while retaining accepted rows.

- [ ] **Step 2: Verify service RED**

Run:

```sh
cargo test -p skills-copilot-service tests::local_session_inventory -- --nocapture
```

Expected: the 1,205-row assertion fails at 800 and no cursor/source-revision fields exist.

- [ ] **Step 3: Implement stateless session keyset pages**

Add optional request fields:

```rust
pub cursor: Option<String>,
pub source_revision: Option<String>,
```

Inventory every authorized root within existing request budgets, calculate a metadata revision over canonical stable row ID, modified milliseconds, file size, and normalized scope, then sort candidates by `(modified_at DESC, row_id ASC, normalized_path_digest ASC)`. A cursor stores the last modified time, stable row ID, path digest, query digest, and revision; it never contains a raw path. For cursor requests:

```rust
let start = candidates.partition_point(|candidate| !is_after_cursor(candidate, &cursor));
let page = candidates.into_iter().skip(start).take(limit + 1).collect::<Vec<_>>();
```

Read primary content only for the first `limit` selected candidates. Keep legacy offset/max-files behavior only when the new cursor is absent. Do not add a static cache, global, `OnceLock`, SQLite persistence, or app-data write.

- [ ] **Step 4: Write Swift RED tests for prewarm, more, all, cancel, and source change**

Use fake pages totaling 1,205 rows and add these exact assertions:

```swift
await store.refreshLocalSessionSnapshot(reason: .startup)
try expectEqual(store.localSessionPreviewResult.sessionRows.count, 800, "Prewarm boundary")
try expectEqual(store.localSessionCompleteness.loadedCount, 800, "Prewarm loaded count")
try expectEqual(store.localSessionCompleteness.hasMore, true, "Prewarm should expose continuation")
await store.loadMoreLocalSessions()
try expectEqual(store.localSessionPreviewResult.sessionRows.count, 900, "One more page")
await store.loadAllLocalSessions()
try expectEqual(store.localSessionPreviewResult.sessionRows.count, 1_205, "Load all count")
try expectEqual(store.localSessionCompleteness.completeness, .complete, "Load all completeness")
try expectEqual(Set(store.localSessionPreviewResult.sessionRows.map(\.id)).count, 1_205, "No duplicates")
```

In a separate delayed fake, start Load All, release two pages, call `cancelLocalSessionLoadAll()`, and assert the accepted count remains unchanged after releasing the old response. Change `agentFilter` before releasing another delayed page and assert its IDs never enter the active source.

- [ ] **Step 5: Implement native prewarm and explicit continuation**

Change the first request limit to 100. Continue serially until EOF or 800 accepted summaries. Store the cursor/revision in `LocalSessionSnapshot`, not AppStorage. Implement:

```swift
func loadMoreLocalSessions() async
func loadAllLocalSessions() async
func cancelLocalSessionLoadAll()
```

Each method captures source key and generation, calls only one page at a time, and publishes accepted pages through `LocalSessionCache`. Sidebar status shows loaded/total, typed incompleteness, and paging buttons. Search/sort/scope remain local over every loaded summary; the footer says that filters cover loaded rows while more rows remain.

- [ ] **Step 6: Synchronize wire docs and verify Task 3**

Run:

```sh
cargo test -p skills-copilot-service tests::local_session_inventory
cargo test -p skills-copilot-service tests::local_session_summary_detail
pnpm verify:service-protocol-drift
pnpm test:macos-native-models
swift test --package-path apps/macos
pnpm verify:macos-ui-layout
pnpm check:macos
pnpm check:privacy
```

Expected: service and native suites pass; fixture smoke uses only fixture HOME and captures the full app window.

- [ ] **Step 7: Commit Task 3**

```sh
git add crates/service apps/macos fixtures/service-protocol docs/service-protocol.md docs/adapters/agent-adapters.md
git commit -m "feat: continue bounded session inventories"
```

---

### Task 4: Remove Skill Manager display caps and expose source completeness

**Files:**
- Modify: `crates/commands/src/skill_manager.rs`
- Modify: `crates/commands/src/skill_manager/effects_tests.rs`
- Modify: `crates/service/src/tests/skill_manager_fixtures.rs`
- Modify: `crates/service/src/tests/protocol_fixtures.rs`
- Modify: `fixtures/service-protocol/skillManager.search.response.json`
- Modify: `fixtures/service-protocol/skillManager.listInstalled.response.json`
- Modify: `docs/service-protocol.md`
- Modify: `docs/adapters/agent-adapters.md`
- Modify: `apps/macos/Sources/SkillsCopilot/Models/SkillManager.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Services/ServiceClientSkillManagerRPC.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/SkillManagerPanel.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/SkillManagerModelTests.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/SkillManagerRequestGenerationTests.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift`

**Interfaces:**
- Adds page metadata to `SkillManagerSearchRecord` and `SkillManagerInstalledListRecord`.
- Produces `SkillManagerVisibleResults` for client-side reveal of a complete returned CLI result without a second network request.
- Preserves current external manager command shapes unless the manager advertises an actual page token.

- [ ] **Step 1: Write RED command/parser tests**

Feed 35 parsed search rows and 27 installed rows, then assert:

```rust
assert_eq!(search.results.len(), 35);
assert_eq!(search.page.returned_count, 35);
assert_eq!(search.page.total_count, None);
assert_eq!(search.page.source_completeness, ListSourceCompleteness::Unknown);
assert_eq!(search.page.incomplete_reason, Some(ListIncompleteReason::SourceLimited));
assert_eq!(installed.installed.len(), 27);
assert_eq!(installed.page.total_count, Some(27));
assert_eq!(installed.page.source_completeness, ListSourceCompleteness::Enumerable);
```

For the network-blocked preview assert zero rows and `SourceLimited`, never `Enumerable`.

- [ ] **Step 2: Verify Rust RED**

Run:

```sh
cargo test -p skills-copilot-commands skill_manager -- --nocapture
cargo test -p skills-copilot-service tests::skill_manager_fixtures -- --nocapture
```

Expected: result structs do not contain page metadata.

- [ ] **Step 3: Add honest manager completeness metadata**

Extend command records with `#[serde(flatten)] pub page: ListPageMetadata`. Use:

```rust
let page = ListPageMetadata {
    returned_count: results.len(),
    total_count: None,
    has_more: false,
    next_cursor: None,
    source_completeness: ListSourceCompleteness::Unknown,
    incomplete_reason: Some(ListIncompleteReason::SourceLimited),
};
```

for remote search without a manager total, and exact enumerable totals for installed JSON and app-owned local-library data. Do not invent CLI flags. If a future parsed JSON field contains `next_page_token`, pass it through only after a fixture proves the manager contract.

- [ ] **Step 4: Write native RED tests for 35/27/31 visible rows**

Add the exact visible-state assertions:

```swift
var search = SkillManagerVisibleResults<String>()
try expectEqual(search.visibleItems(in: Array(0..<35).map(String.init)).count, 20, "Initial search page")
search.loadMore(totalReturned: 35)
try expectEqual(search.visibleItems(in: Array(0..<35).map(String.init)).count, 35, "Search Load More")
search.reset()
search.loadAll(totalReturned: 35)
try expectEqual(search.visibleItems(in: Array(0..<35).map(String.init)).count, 35, "Search Load All")
```

Store tests assert installed visible IDs count 27, local-library IDs count 31, search status has `totalCount == nil`, and a delayed old generation cannot change the current visible IDs after query/owner inputs change.

- [ ] **Step 5: Implement client-side bounded reveal and remove prefixes**

Add:

```swift
struct SkillManagerVisibleResults<ID: Hashable>: Equatable {
    private(set) var visibleCount: Int = 20
    mutating func loadMore(totalReturned: Int) { visibleCount = min(totalReturned, visibleCount + 20) }
    mutating func loadAll(totalReturned: Int) { visibleCount = totalReturned }
    mutating func reset() { visibleCount = 20 }
}
```

Remove `prefix(8)` and `prefix(12)`. Slice only through the explicit visible-state model, show loaded/known-or-unknown total, and label Load All as “Show all returned results” when source completeness is unknown. Local library renders its complete lazy collection directly.

- [ ] **Step 6: Verify and commit Task 4**

Run:

```sh
cargo test -p skills-copilot-commands skill_manager
cargo test -p skills-copilot-service tests::skill_manager_fixtures
pnpm verify:service-protocol-drift
pnpm test:macos-native-models
pnpm verify:macos-ui-layout
pnpm check:macos
pnpm check:privacy
```

Commit:

```sh
git add crates/commands crates/service apps/macos fixtures/service-protocol docs/service-protocol.md docs/adapters/agent-adapters.md
git commit -m "feat: expose complete skill manager results"
```

---

### Task 5: Add paged provider activity beneath full-range aggregates

**Files:**
- Modify: `crates/service/src/lib.rs`
- Modify: `crates/service/src/service_host.rs`
- Modify: `crates/service/src/service_llm.rs`
- Modify: `crates/service/src/service_observability_helpers.rs`
- Modify: `crates/service/src/tests/llm_provider.rs`
- Modify: `crates/service/src/tests/protocol_fixtures.rs`
- Modify: `crates/service/src/tests/method_effects.rs`
- Modify: `fixtures/service-protocol/method-effects.json`
- Create: `fixtures/service-protocol/llm.listProviderActivity.request.json`
- Create: `fixtures/service-protocol/llm.listProviderActivity.response.json`
- Modify: `docs/service-protocol.md`
- Modify: `apps/macos/Sources/SkillsCopilot/Models/ProviderObservability.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Services/ServiceClient.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Services/ServiceClientLLMRPC.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/ProviderObservabilitySettingsPanel.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/ProviderObservabilityModelTests.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/ServiceClientRPCTests.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift`

**Interfaces:**
- Produces read-only `llm.listProviderActivity`.
- Produces `ProviderActivityRow`, `ProviderActivityPageResult`, and store actions `loadMoreProviderActivity(loadAll:)`/`cancelProviderActivityLoadAll()`.

- [ ] **Step 1: Write Rust RED tests for unified activity ordering**

Seed 75 provider-call metadata rows and 55 prompt-run rows with colliding timestamps. Collect pages with limit 50 and assert:

```rust
assert_eq!(rows.len(), 130);
assert_eq!(rows.iter().map(|row| &row.id).collect::<HashSet<_>>().len(), 130);
assert!(rows.windows(2).all(|pair| {
    pair[0].timestamp > pair[1].timestamp
        || (pair[0].timestamp == pair[1].timestamp && pair[0].id <= pair[1].id)
}));
assert!(rows.iter().all(|row| !row.title.contains(user_home.to_string_lossy().as_ref())));
assert!(!final_page.safety_flags.provider_request_sent);
assert!(!final_page.safety_flags.raw_prompt_persisted);
```

After page one, append one metadata record and assert the next request returns error code `source_changed`.

- [ ] **Step 2: Verify RED**

Run:

```sh
cargo test -p skills-copilot-service tests::llm_provider::provider_activity -- --nocapture
```

Expected: unknown method/type failures.

- [ ] **Step 3: Implement the read-only activity method**

Define:

```rust
pub struct ListProviderActivityParams {
    pub provider: Option<String>, pub model: Option<String>, pub action: Option<String>,
    pub window_days: Option<i64>, pub start_at: Option<i64>, pub end_at: Option<i64>,
    pub limit: Option<usize>, pub cursor: Option<String>, pub source_revision: Option<String>,
}
pub struct ProviderActivityRow {
    pub id: String, pub kind: String, pub timestamp: i64, pub title: String,
    pub subtitle: String, pub status: String, pub evidence_refs: Vec<String>,
}
pub struct ProviderActivityPageResult {
    pub generated_by: &'static str, pub rows: Vec<ProviderActivityRow>,
    pub source_revision: String, #[serde(flatten)] pub page: ListPageMetadata,
    pub safety_flags: LlmProviderObservabilitySafetyFlags,
}
```

Reuse existing redacted call/history row builders, map them into activity rows, clamp limit to `1...100`, and use Task 2 cursor validation. Keep `llm.providerObservability` aggregate calculations unchanged.

- [ ] **Step 4: Write Swift RED tests for activity accumulation**

Add:

```swift
await store.loadProviderObservability()
try expectEqual(store.providerActivityRows.count, 50, "Initial activity page")
try expectEqual(store.providerActivityCompleteness.totalCount, 130, "Activity total")
let summary = store.providerObservabilityResult?.summary
await store.loadMoreProviderActivity(loadAll: false)
try expectEqual(store.providerActivityRows.count, 100, "Activity Load More")
await store.loadMoreProviderActivity(loadAll: true)
try expectEqual(store.providerActivityRows.count, 130, "Activity Load All")
try expectEqual(store.providerObservabilityResult?.summary, summary, "Paging must not change aggregate summary")
```

Add delayed-page cancellation and stale-generation cases using the existing fake service release-file pattern.

- [ ] **Step 5: Implement activity accumulation and canonical paged UI**

Decode activity page fields with snake/camel aliases, store one accumulator per resolved filter key, serialize page requests, preserve page errors locally, and expose `loadMoreProviderActivity(loadAll:)` plus `cancelProviderActivityLoadAll()`. Keep the top-five chart but label it “Top 5 summary”. Render all loaded activity rows below it with `ListCompletenessFooter`. Add stable IDs `provider-activity.load-more`, `provider-activity.load-all`, and `provider-activity.cancel`.

- [ ] **Step 6: Verify and commit Task 5**

Run:

```sh
cargo test -p skills-copilot-service tests::llm_provider
pnpm verify:service-protocol-drift
pnpm test:macos-native-models
pnpm verify:macos-ui-layout
pnpm check:macos
pnpm check:privacy
```

Commit:

```sh
git add crates/service apps/macos fixtures/service-protocol docs/service-protocol.md
git commit -m "feat: page provider activity details"
```

---

### Task 6: Route global-search totals and expand every intentional summary

**Files:**
- Modify: `apps/macos/Sources/SkillsCopilot/Models/AppSearch.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Models/AppSearchIndex.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/ContentView.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/SidebarView.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/BatchSkillOperationSheet.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/DetailOverviewSection.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/DetailPresentationPrimitives.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/TaskCockpitPanel.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/SkillManagerPanel.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Models/MarkdownTableDisplayModel.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/LocalSessionPreviewModelTests.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/TaskCockpitModelTests.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/UIOptimizationModelTests.swift`
- Modify: `scripts/verify-native-ui-layout.mjs`

**Interfaces:**
- Extends `AppSearchResult` with exact per-kind match counts.
- Produces `showAllAppSearchResults(kind:query:) async` routing to canonical Skills, Sessions, or Config lists.
- Converts every formal raw-prefix surface to complete lazy data, `ExpandableSummaryList`, or canonical navigation.

- [ ] **Step 1: Write RED global-search count and routing tests**

Index 20 matching records per kind with overlay limit six and assert:

```swift
let result = index.search(query: "match", limitPerKind: 6)
for kind in AppSearchItemKind.allCases {
    try expectEqual(result.items.filter { $0.kind == kind }.count, 6, "Shortcut count for \(kind)")
    try expectEqual(result.count(for: kind), 20, "Full count for \(kind)")
}
```

Invoke `showAllAppSearchResults(kind:query:)` for each kind and assert destination mode/domain search text equal `"match"`, the overlay selection clears, and fake service call count does not increase.

- [ ] **Step 2: Write RED surface tests for every current raw prefix**

Add native/layout assertions requiring these full-access IDs:

```text
session-top-skills.show-all
batch-toggle-items.show-all
permission-summary.show-all
task-cockpit-candidates.show-all
task-cockpit-context.show-all
skill-manager-agents.show-all
markdown-table.show-all
global-search.skills.view-all
global-search.sessions.view-all
global-search.config-history.view-all
```

Expected RED: current prefix-only surfaces lack the controls.

- [ ] **Step 3: Implement exact per-kind counts and canonical routing**

Add:

```swift
struct AppSearchKindCount: Decodable, Hashable { let kind: AppSearchItemKind; let count: Int }
```

`AppSearchIndex.search` counts the full filtered arrays before taking six shortcuts. `showAllAppSearchResults` sets `sidebarContentMode`, the matching domain query, scope required to include the indexed record set, and selection normalization, then dismisses the overlay through its callback.

- [ ] **Step 4: Replace every formal raw prefix with explicit expansion**

Use `ExpandableSummaryList` for batch items, permission rows, Task Cockpit candidate/context rows, manager agents, and compact Markdown rows. Session skill usage says “Top 3 of N” and expands every `skillUsageRows` entry. Keep `DenseDisclosureList` where it already renders both the prefix and `dropFirst` remainder. Remove hidden-count-only copy that lacks an action.

- [ ] **Step 5: Verify and commit Task 6**

Run:

```sh
pnpm test:macos-native-models
swift test --package-path apps/macos
pnpm verify:macos-ui-layout
pnpm verify:module-size
pnpm check:macos
pnpm check:privacy
```

Commit:

```sh
git add apps/macos scripts/verify-native-ui-layout.mjs
git commit -m "feat: expose full search and summary results"
```

---

### Task 7: Govern every formal list and forbid new silent truncation

**Files:**
- Create: `scripts/list-completeness-surfaces.json`
- Create: `scripts/lib/list-completeness.mjs`
- Create: `scripts/verify-list-completeness.mjs`
- Create: `scripts/tests/list-completeness.test.mjs`
- Modify: `scripts/repository-governance.json`
- Modify: `package.json`
- Modify: `docs/runbooks/macos-app-runbook.md`
- Modify: `docs/ui-delivery-standards.md`
- Modify: `docs/plans/development-tasks.md`

**Interfaces:**
- Produces `loadListCompletenessManifest`, `verifyListSurfaceInventory`, and `findUndeclaredPrefixLists`.
- Produces `pnpm test:list-completeness` and `pnpm verify:list-completeness`.
- Adds the verifier to repository gate parity after module-size verification.

- [ ] **Step 1: Write RED verifier tests**

Create table-driven fixtures with these exact expected messages:

```js
assert.deepEqual(verifyListSurfaceInventory(missingContinuation), [
  "sessions.sidebar: paged surface is missing full_access_id",
]);
assert.deepEqual(verifyListSurfaceInventory(missingShowAll), [
  "batch-toggle.items: summary_with_expand is missing full_access_id",
]);
assert.deepEqual(verifyListSurfaceInventory(duplicateIDs), [
  "duplicate list completeness surface id: sessions.sidebar",
]);
assert.deepEqual(findUndeclaredPrefixLists("ForEach(records.prefix(8)) { row in"), [
  "undeclared prefix-defined formal list at line 1",
]);
assert.deepEqual(findUndeclaredPrefixLists(denseDisclosureSource), []);
```

Add stale-path and unknown-policy cases with exact messages naming the manifest entry.

- [ ] **Step 2: Verify RED**

Run:

```sh
node --test scripts/tests/list-completeness.test.mjs
```

Expected: module-not-found for `scripts/lib/list-completeness.mjs`.

- [ ] **Step 3: Create the complete manifest and verifier**

The JSON schema is:

```json
{
  "schema_version": 1,
  "surfaces": [
    {
      "id": "sessions.sidebar",
      "file": "apps/macos/Sources/SkillsCopilot/Views/SidebarView.swift",
      "source": "session.previewLocalSessions",
      "policy": "paged",
      "status_id": "sessions.completeness",
      "full_access_id": "sessions.load-all"
    }
  ]
}
```

Include every list named in the design: skills, findings, conflicts, rules, events, sessions, config history, three Skill Manager lists, provider activity, three global-search kinds, session top skills, batch affected rows, permissions, Task Cockpit collections, manager agents/risks, and Markdown tables.

The verifier loads source files, requires declared accessibility IDs, rejects missing files/duplicates, and scans `ForEach`/`List` expressions containing `.prefix(` unless the manifest policy is `summary_with_expand` and its full-access ID exists in the same owning file.

- [ ] **Step 4: Wire deterministic gates and documentation**

Add package scripts:

```json
"test:list-completeness": "node --test scripts/tests/list-completeness.test.mjs",
"verify:list-completeness": "node scripts/verify-list-completeness.mjs"
```

Add `pnpm verify:list-completeness` immediately after `pnpm verify:module-size` in gate parity and `repository-governance.json`. Document that the verifier enforces declaration/controls while native tests enforce behavior.

- [ ] **Step 5: Verify and commit Task 7**

Run:

```sh
pnpm test:list-completeness
pnpm verify:list-completeness
pnpm verify:gate-parity
pnpm verify:doc-governance
pnpm check:privacy
```

Commit:

```sh
git add scripts package.json docs/runbooks/macos-app-runbook.md docs/ui-delivery-standards.md docs/plans/development-tasks.md
git commit -m "test: govern complete list access"
```

---

### Task 8: Run integrated completeness, performance, protocol, and UI verification

**Files:**
- Verify: all files changed in Tasks 1–7
- Modify only on failure: the owning task’s exact source/test/doc files

**Interfaces:**
- Consumes every prior contract and surface.
- Produces final evidence that no formal list silently omits accessible data.

- [ ] **Step 1: Run formatting, unit, and strict lint gates**

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
pnpm test:macos-native-models
swift test --package-path apps/macos
```

Expected: all pass with no warnings; native registry counts match the checked-in registry.

- [ ] **Step 2: Run protocol, governance, and privacy gates**

```sh
pnpm verify:service-protocol-drift
pnpm verify:list-completeness
pnpm verify:module-size
pnpm verify:gate-parity
pnpm check:privacy
```

Expected: all methods/fixtures/effects/docs agree, every manifest surface resolves, and privacy reports `ok`.

- [ ] **Step 3: Run performance budgets**

```sh
pnpm benchmark:10k
pnpm benchmark:macos-list-model
```

Expected: 10k scan/catalog counts remain exact, runtime/RSS stay within checked-in budgets, and list p95 stays within its checked-in maximum.

- [ ] **Step 4: Run the full macOS gate and inspect fixture UI**

```sh
pnpm check:macos
pnpm smoke:macos-app -- --fixture-data --capture-window
```

Operate only the fixture-data app window. Verify Sessions, Config history, Skill Manager, Provider activity, and global-search View All controls display loaded/total/incomplete state and can reach final rows. Capture only the full app window; do not capture the desktop or real local session content.

- [ ] **Step 5: Review final source boundaries**

```sh
rg -n "ForEach\([^\n]*\.prefix|ForEach\([^\n]*prefix" apps/macos/Sources/SkillsCopilot
rg -n "read_to_string|read_line|OnceLock|LazyLock|static .*LocalSession" crates/service/src/service_local_session_io.rs crates/service/src/service_local_sessions.rs
git diff --check
git status --short
```

Expected: every remaining UI prefix is an expandable/named summary declared in the manifest; no static session cache or unbounded session read exists; diff is clean.

- [ ] **Step 6: Correct failures only through the owning task’s RED/GREEN loop**

For each failure, add a regression to the exact task test file, verify RED, make the smallest correction, rerun the focused gate, then restart Task 8 from Step 1. Do not raise limits, weaken completeness assertions, or add manifest exceptions to make a gate pass.

- [ ] **Step 7: Commit verification-only corrections if any**

If no tracked correction was required, create no commit. Otherwise:

```sh
git add crates apps/macos scripts fixtures docs package.json Cargo.lock
git commit -m "chore: finalize complete list verification"
```

## Spec Coverage Map

| Design requirement | Owning task |
| --- | --- |
| Shared known/unknown/partial/incomplete contract | Task 1 |
| Common native controls and accessibility IDs | Task 1 |
| Catalog scan completeness, config history, skill events | Task 2 |
| Session prewarm, continuation beyond 800, source revision, safety budgets | Task 3 |
| Skill Manager full returned rows and honest remote-source limits | Task 4 |
| Full-range provider aggregates plus paged activity details | Task 5 |
| Global-search View All and every summary/preview expansion | Task 6 |
| Governed surface inventory and no-new-prefix enforcement | Task 7 |
| 10k performance, compact UI, protocol, smoke, privacy, and full gates | Task 8 |

## Final Acceptance Checklist

- [ ] Every formal list is declared in `scripts/list-completeness-surfaces.json`.
- [ ] Every declared list is complete, continuable, or visibly incomplete.
- [ ] Config history, Skill Manager, provider activity, sessions beyond 800, global search, and every named summary expose all safely available rows.
- [ ] No source or presentation cap is silently described as a total.
- [ ] Dynamic data changes reject stale cursors instead of duplicating or omitting rows.
- [ ] Load All remains serial, bounded, cancellable, and retryable.
- [ ] Legacy wire methods and decoders remain compatible.
- [ ] Session summary/detail privacy boundaries remain intact.
- [ ] Full Rust, Swift, protocol, governance, performance, macOS, smoke, and privacy gates pass.
- [ ] Security/deep scanning remains skipped as explicitly requested.
