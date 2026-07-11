# List Completeness And Pagination Design

Date: 2026-07-11

Status: approved in conversation and approved after written-spec review

## Purpose

No user-visible list may silently omit records. Every formal list must either:

1. expose every matching record;
2. let the user continue loading until every matching record is exposed; or
3. state that the result is incomplete, explain why, and provide the safest
   available recovery action.

This design uses a hybrid policy. Local, inexpensive lists load completely and
render with lazy native controls. Remote or potentially large lists load in
bounded pages and offer **Load More**, **Load All**, and **Cancel**. Safety
budgets remain finite and are never replaced with unbounded reads.

## Current Gaps

The current native app has these completeness gaps:

- local-session pages are merged automatically, but candidate selection stops
  at 800 and the UI does not clearly expose candidate or inventory truncation;
- the config timeline receives the complete agent history but exposes only the
  newest five records and a hidden-count label;
- Skill Manager search exposes eight results, while installed and local-library
  sections expose twelve records, without a path to the remaining records;
- provider observability returns full-range aggregates but only a fixed set of
  detail rows;
- global search returns six shortcut results per kind without an explicit route
  to every matching item;
- several preview and detail surfaces use `prefix()` as presentation logic
  without consistently naming the section as a summary or exposing the rest.

The main Skills, Findings, Conflicts, Rule Tuning, and selected-skill event
collections already receive complete service results when their underlying
scan/catalog read is complete. They remain in scope for completeness metadata
and regression verification. A scanner budget or root failure must make the
corresponding list explicitly incomplete, but these collections must not be
converted to remote-style pagination without measured need.

## Product Contract

### Formal-list status

Every formal list presents a common status derived from this logical contract:

```text
loaded_count: Int
total_count: Int?
has_more: Bool
is_complete: Bool
completeness: complete | partial | incomplete | unknown
next_cursor: String?
incomplete_reason: safety_budget | source_changed | source_limited |
                   unreadable_source | page_failed | unsupported_protocol | null
can_load_more: Bool
can_load_all: Bool
```

`total_count = null` means the source did not provide a defensible total. The
UI says **Total unknown** and never substitutes `loaded_count` as the total.

The status copy uses these rules:

- complete: `Loaded X of X · Complete`;
- partial with known total: `Loaded X of Y`;
- partial with unknown total: `Loaded X · Total unknown`;
- loading: preserve current rows and append `Loading…`;
- incomplete: show the reason and the next safe action;
- stale: preserve current rows and identify the failed refresh or page.

Counts describe the current source, filters, and sort order. Changing any of
those inputs begins a new generation and invalidates the old cursor.

### No silent summaries

A presentation may intentionally show a top-N summary only when all of these
are true:

- its title says **Top N**, **Recent N**, or **Summary**;
- the full count is visible when known;
- **Show All** expands the complete in-memory collection or navigates to the
  canonical paged list;
- accessibility value and help text state that the section is a summary.

Decorative charts may retain a top-N visual series when an adjacent accessible
table exposes all loaded rows and the completeness status.

## List Policy By Surface

### Skills, findings, conflicts, rules, and selected-skill events

- Skills, findings, conflicts, and rule-tuning records continue to load as
  complete local collections from the startup/manual-refresh snapshot.
- The snapshot exposes a source-completeness summary derived from every scanned
  adapter root. A file, byte, depth, directory, entry, or skill-count budget;
  an unreadable root; or an incomplete catalog missing-sweep makes the affected
  list `incomplete` and identifies the affected agent/root using privacy-safe
  display text.
- Swift lists use `List`, `LazyVStack`, or equivalent lazy native containers;
  they do not use `prefix()` to define the accessible dataset.
- Selected-skill events move to an additive keyset-paged service method when
  the event history exceeds one response page. The app automatically requests
  all local pages in the background while preserving the first visible page.
- Existing unpaged service methods remain compatible for older clients.

### Local sessions

- Initial refresh returns at most 100 summary rows so the first useful screen
  appears promptly.
- Startup prewarm continues automatically through at most 800 summaries.
- When more candidates exist, the footer exposes **Load More** and **Load All**.
- Each additional request remains bounded to 100 summaries. **Load All** means
  repeated bounded requests, not a larger unbounded request.
- The service adds an opaque keyset cursor ordered by candidate modified time,
  canonical stable row ID, and deterministic path tie-breaker.
- The first response returns a metadata-only inventory revision. Later pages
  must match it. If the source changes, the service returns `source_changed`
  instead of returning a page that could skip or duplicate records.
- The service stores no static session cache and persists no inventory, cursor,
  path, or raw session content. A cursor contains only versioned pagination
  fields, a normalized-query digest, and an inventory revision digest. The
  decoder validates every field against the current request; diagnostics redact
  the cursor. The cursor is not an authorization credential.
- `candidate_set_truncated` continues to represent safety-budget truncation,
  not ordinary pagination. The UI renders it as incomplete and offers scope or
  authorized-root narrowing rather than claiming completion.
- Detail loading remains stable-ID targeted and reads only the selected primary
  file. Raw detail remains in the bounded, source-scoped in-memory detail cache.

### Config history

- The timeline no longer hard-codes five accessible records.
- A new additive `snapshot.listAgentConfigPage` method uses a keyset cursor over
  `(created_at, id)` and returns completeness metadata.
- The native app renders the first page immediately and automatically obtains
  remaining local pages. The disclosure contains the complete lazy timeline.
- Large histories expose load progress and cancellation. They never replace
  the full timeline with a hidden-count label.
- Existing `snapshot.listAgentConfig` remains supported for protocol
  compatibility and is treated as a complete legacy response.

### Skill Manager

- Search, installed, and local-library sections remove raw `prefix(8)` and
  `prefix(12)` presentation limits.
- The scoped manager CLI request and service result gain optional page token,
  loaded count, total count, `has_more`, and completeness fields when the
  manager can provide them.
- The app requests one remote page at a time. It provides **Load More** and
  **Load All**, with command preview, target visibility, telemetry-off
  environment, redaction, and existing confirmation boundaries unchanged.
- If an external manager cannot prove the total or cannot request another
  page, the result is `unknown` or `source_limited`. The app displays that
  limitation; it does not describe the returned rows as all results.
- Local-library results are local and inexpensive, so the app exposes the
  complete lazy collection without a twelve-row cap.
- Installed records use paging only when the manager exposes a real paging
  mechanism; otherwise all returned records remain visible with honest
  completeness state.

### Provider observability

- `llm.providerObservability` remains the full-range aggregation endpoint. Its
  summary continues to use every matching metadata record.
- Add `llm.listProviderActivity` for paged detail rows. It accepts the same
  provider/model/action/date filters plus an opaque cursor and a bounded limit.
- The activity method returns a unified stable row ID, activity kind, timestamp,
  redacted display fields, and completeness metadata.
- The native settings panel initially loads 50 activity rows. It exposes
  **Load More**, **Load All**, and cancellation while retaining charts and
  aggregates.
- The top-five chart is explicitly labelled as a visual summary. Its adjacent
  activity list is the canonical path to all available rows.

### Global search

- Six results per kind remain the lightweight overlay limit.
- Each kind reports its full in-memory match count.
- **View All N Skills**, **View All N Sessions**, and **View All N Config
  Records** navigate to the canonical list, apply the query there, and dismiss
  the overlay.
- The overlay never labels its six shortcuts as the complete result set.

### Preview and workflow collections

The following surfaces are audited as part of this work:

- session top-skill rows;
- batch-toggle affected records;
- permission summary rows;
- Task Cockpit candidates, gap rows, evidence, risks, and safety notes;
- Skill Manager agent summaries and mutation risks;
- Markdown tables and other compact detail collections.

Each surface must use one of three explicit policies:

1. complete lazy collection;
2. `DenseDisclosureList` or an equivalent accessible inline expansion; or
3. named summary plus a route to the canonical complete list.

There is no raw `prefix()`-defined formal list without a full-access control.

## Service Architecture

### Additive page metadata

Rust exposes one shared serializable page metadata shape. Concrete result types
embed it with `#[serde(flatten)]` only where that preserves existing wire
names; otherwise they expose the same explicit snake-case fields.

```rust
pub struct ListPageMetadata {
    pub returned_count: usize,
    pub total_count: Option<usize>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
    pub source_completeness: ListSourceCompleteness,
    pub incomplete_reason: Option<ListIncompleteReason>,
}
```

Every result invariant is validated before serialization:

- `returned_count` equals the number of rows in this response;
- `has_more` requires either `next_cursor` or a typed source limitation;
- `source_completeness = enumerable` means the source can provide every row by
  following cursors; it does not mean the current page contains every row;
- an unknown total is allowed to end at a defensible source EOF, but it cannot
  be presented as a known `X of X` result;
- cursors are versioned and bound to normalized filters, sort, scope, source,
  and inventory/query revision.

The native accumulator, not an individual page response, computes
`loaded_count` and `is_complete`. It may mark the list complete only when
`has_more == false`, the source is enumerable, and either the accumulated
stable-ID count equals the known total or the source contract supplies a
defensible EOF with an unknown total.

### Cursor behavior

- Local SQLite collections use keyset cursors, never offset-only continuation.
- Session inventory uses a stateless metadata cursor plus inventory revision;
  it does not persist raw or derived session data between requests.
- Remote manager page tokens remain opaque and are never logged verbatim.
- Malformed, mismatched, stale, or replayed cursors fail with a typed
  `invalid_cursor` or `source_changed` error.
- Legacy `limit` and `offset` fields remain accepted where already documented.

### Response and memory budgets

- Page size is clamped per method and covered by tests.
- **Load All** is implemented only in the native client loop.
- One domain has at most one in-flight next-page request per generation.
- Page accumulation uses stable-ID de-duplication and bounded raw-detail caches.
- Summary rows may contain redacted excerpts and metrics but never raw content
  items unless an exact detail request explicitly asks for them.

## Native Architecture

### Shared pure models

Swift adds pure, testable models rather than embedding paging logic in views:

```swift
struct ListCompletenessState: Equatable {
    let loadedCount: Int
    let totalCount: Int?
    let hasMore: Bool
    let isComplete: Bool
    let completeness: ListCompleteness
    let incompleteReason: ListIncompleteReason?
    let canLoadMore: Bool
    let canLoadAll: Bool
}

struct ListPageAccumulator<Item: Identifiable> where Item.ID: Hashable {
    private(set) var items: [Item]
    private(set) var nextCursor: String?
    private(set) var sourceRevision: String?
    mutating func append(_ page: ListPage<Item>) throws
}

struct ListPage<Item> {
    let items: [Item]
    let returnedCount: Int
    let totalCount: Int?
    let hasMore: Bool
    let nextCursor: String?
    let sourceRevision: String?
    let sourceCompleteness: ListSourceCompleteness
    let incompleteReason: ListIncompleteReason?
}
```

The accumulator rejects a filter/source revision mismatch, retains first-seen
stable order, and ignores duplicate stable IDs. Domain stores retain their own
request parameters and generations; views consume only published rows,
completeness state, and explicit actions.

### State machine

Every paged domain follows:

```text
empty -> loadingInitial -> partial -> loadingMore -> partial|complete
                              \-> loadingAll -> partial|complete
```

Failure states are:

- `stale`: a refresh failed after a previously complete result;
- `partial`: at least one page is visible and a later page failed or loading
  was cancelled;
- `incomplete`: a safety budget or source limitation prevents completion;
- `failed`: the initial page failed and no prior rows exist.

Loading another page never clears visible rows. Retry continues from the last
accepted cursor. Refresh starts a new generation and cannot be overwritten by
an older page response.

### Common UI components

The native shell adds reusable presentation primitives:

- `ListCompletenessBadge`;
- `ListCompletenessFooter`;
- `ListPagingActions`;
- `ExpandableSummaryList`.

All controls have stable accessibility identifiers, labels, values, and help.
**Load All** changes to **Cancel Loading All** while active. Keyboard focus
returns to the first appended row after **Load More** and remains on the action
after cancellation or failure.

## Error Handling

- A page decode, service, or source error remains local to that list.
- An initial error shows the normal empty/error state and a retry action.
- A later-page error preserves accepted pages and displays **Retry Page**.
- `source_changed` preserves loaded rows as stale and offers **Restart
  Refresh**; it never merges pages from different source revisions.
- `safety_budget` explains the bounded resource and offers narrower agent,
  scope, project, or authorized-root controls.
- `source_limited` and `unsupported_protocol` state that the external source or
  older sidecar cannot prove completeness.
- Raw cursors, local absolute paths, command stderr, credentials, session
  content, and provider payloads never enter user-facing errors or logs.

## Compatibility

- Service changes are additive. Existing unpaged methods and request fields
  remain supported.
- Swift decoders accept snake-case, camel-case, and legacy responses without
  page metadata.
- A legacy response from a method whose old contract returned the complete
  local collection is treated as complete.
- A legacy remote or bounded response is treated as `unknown`, not complete.
- `docs/service-protocol.md`, method effects, fixtures, dispatch checks, and
  protocol-drift verification change in the same task as each wire change.

## Governance

Task 7 of the approved implementation plan creates a governed manifest under
`scripts/` as the inventory of formal lists; the implementation plan owns the
exact future path declaration.
Each entry declares:

- surface ID and owning view/model;
- data source method;
- policy: `complete`, `paged`, or `summary_with_expand`;
- total-count source;
- full-access control accessibility ID;
- allowed safety/source limitations.

Add `pnpm verify:list-completeness` to validate the manifest, accessibility
control IDs, protocol metadata, and source inventory. It rejects:

- a formal list missing from the manifest;
- a paged list without loaded/total state and a continuation action;
- a summary without an expansion or canonical-list route;
- a new raw `prefix()` formal-list presentation not declared as a summary;
- stale manifest paths or accessibility identifiers.

The verifier joins the deterministic gate-parity manifest. It does not replace
native behavior tests.

## Verification

### Rust

- page size validation and EOF metadata;
- total, loaded, `has_more`, and cursor invariants;
- keyset ordering with equal sort keys;
- consecutive pages concatenate to the unpaged order without duplicates;
- inserts, deletes, and modifications between pages produce the documented
  stable result or `source_changed`, never silent omission;
- malformed and cross-query cursor rejection;
- session inventory and byte-budget truncation reasons;
- no raw session/app/provider data persistence;
- old request and response fixture compatibility.

### Swift models and stores

- initial, more, all, cancel, retry, stale, incomplete, and complete states;
- known and unknown totals;
- stable-ID de-duplication and generation rejection;
- source/filter/sort changes invalidate prior cursors;
- search and sort cover the canonical source rather than only visible rows;
- global-search **View All** routing;
- legacy sidecar response classification;
- no raw detail content enters summary accumulators or search indexes.

### Native UI and accessibility

- every manifest surface renders the correct completeness copy;
- every partial paged list exposes **Load More** and **Load All** when allowed;
- every named summary exposes **Show All** or a canonical-list route;
- VoiceOver values announce loaded/total/incomplete state;
- keyboard activation and focus behavior for continuation and cancellation;
- 960-point compact layout remains usable without clipped paging actions.

### Performance and integration

- 10,000-row list/model benchmark stays within checked-in runtime and RSS
  budgets;
- session pages remain bounded to 100 summaries per response;
- loading all does not create concurrent page fan-out;
- full Rust workspace, strict Clippy, SwiftPM, native registry, service protocol
  drift, module-size, list-completeness, gate parity, `pnpm check:macos`, fixture
  full-window smoke, and `pnpm check:privacy` pass;
- security/deep scanning remains outside this scope per the user's explicit
  instruction.

## Delivery Decomposition

Implementation is split into independently reviewable packages:

1. shared service/native completeness contract and governance verifier;
2. config history plus local complete collections;
3. local-session cursor continuation beyond prewarm;
4. Skill Manager paging and honest external-source completeness;
5. provider activity paging;
6. global search and all remaining summary/preview expansions;
7. integrated performance, protocol, UI, accessibility, privacy, and macOS
   release gates.

Each package uses strict RED/GREEN tests, an isolated worktree, an independent
specification/code-quality review, and a fresh integration verification.

## Acceptance Criteria

The work is complete only when:

- every formal list is present in the governed surface inventory;
- every governed list is complete, continuable, or explicitly incomplete;
- no formal list silently hides records behind a hard-coded presentation cap;
- all known remaining records are reachable through **Load More**, **Load
  All**, **Show All**, or canonical-list navigation;
- changing data during paging cannot silently duplicate or omit records;
- safety and external-source limits remain finite and visible;
- loaded rows survive continuation errors and cancellation;
- legacy clients and sidecars retain documented behavior;
- all required focused and repository-wide gates pass on a clean tree.
