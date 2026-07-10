# Scanner and Local Sessions Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bound scanner and local-session filesystem work, enforce explicit root scope, preserve cache correctness under partial reads, and make local-session sorting and pagination deterministic and truthful.

**Architecture:** The scanner will resolve one explicit canonical allowlist per adapter scope, read skill content through an adapter content-parsing boundary under injectable budgets, and distinguish complete roots from partial roots before catalog missing-sweep. Local sessions will use a request-scoped I/O context with fixed head/tail reads, aggregate sidecar budgets, a bounded file inventory, request-local index caches, and typed server-side sorting before pagination.

**Tech Stack:** Rust workspace crates (`core`, `adapters`, `scanner`, `commands`, `service`), `std::fs`/`std::io`, SQLite catalog through `rusqlite`, serde JSON wire models, Swift `Decodable` macOS models, pnpm verification scripts.

## Global Constraints

- Keep `crates/core` free of filesystem I/O; its new adapter interface may accept already-read text but must not open files.
- Add no third-party dependencies. Use `std::io::{Read, Seek, SeekFrom}` and existing serde support.
- Adapter scan targets must be explicit walked roots or explicit link-target roots declared by the adapter for the same `Scope`; whole-home and whole-project fallbacks are forbidden.
- Preserve the public `AgentAdapter::parse(&Path)` entry point for existing callers and tests; add an owned-content parsing entry point for bounded scanner reads.
- Scanner production limits are: depth 64, 50,000 directories, 200,000 entries, 25,000 skill files, 2 MiB per skill, and 256 MiB total skill content per adapter scan.
- Local-session production limits are: 384 KiB primary head, 128 KiB primary tail, 64 KiB maximum retained line fragment, 64 KiB per sidecar file, 512 KiB sidecars per session, 240 sidecar files per session, 64 MiB total read bytes per request, 20,000 inventory directories, and 100,000 inventory entries.
- `max_files` selects the newest candidate files after metadata inventory; it must never stop an unsorted `read_dir` traversal.
- Summary/list requests must explicitly send `include_content_items=false`; a selected-row detail request sends `include_content_items=true` plus the row's stable `session_id`. Requests that omit `include_content_items` retain legacy behavior and include content.
- A detail `session_id` must be matched with `local_session_row_id(path)` immediately after inventory construction, before `max_files` newest-selection and before any primary session file is opened.
- Do not persist raw local-session, sidecar, prompt, response, or trace content. Local-session caches live for one service request only.
- Only roots that were fully enumerated may be supplied to catalog missing-sweep. Partial roots may upsert observed rows but may not mark unobserved rows missing.
- Service protocol changes must update `docs/service-protocol.md`, fixtures, Rust wire decoders, Swift models, and protocol drift verification together.
- Tests and smoke data must use disposable fixture directories and must not read or mutate real local agent configuration.
- Keep changes scoped to scanner and local-session behavior; do not add watchers, persistent session caches, background scans, or new network behavior.

---

## File Structure

- Modify `crates/core/src/adapter.rs`: declare the owned-content adapter parsing interface without adding I/O.
- Modify `crates/adapters/src/claude_code/mod.rs`: delegate path parsing to owned-content parsing and declare the supported Claude compatibility root explicitly.
- Modify `crates/adapters/src/codex/mod.rs`: delegate path parsing to owned-content parsing.
- Modify `crates/adapters/src/opencode/mod.rs`: delegate path parsing to owned-content parsing.
- Modify `crates/adapters/src/pi/mod.rs`: delegate path parsing to owned-content parsing.
- Modify `crates/adapters/src/hermes/mod.rs`: delegate path parsing to owned-content parsing.
- Modify `crates/adapters/src/openclaw/mod.rs`: delegate path parsing to owned-content parsing.
- Modify `crates/adapters/fuzz/fuzz_targets/frontmatter_parser.rs`: call the retained path entry point and add direct owned-content coverage where the harness already has bytes.
- Modify `crates/scanner/src/lib.rs`: own canonical-root resolution, scan budgets, bounded reads, issues, and complete-versus-partial root state.
- Modify `crates/commands/src/lib.rs`: consume only complete scanner roots for missing-sweep and propagate scan diagnostics.
- Modify `crates/commands/src/tests.rs`: verify partial-root cache behavior and multi-adapter continuation.
- Create `crates/service/src/service_local_session_io.rs`: own bounded head/tail reads, budgets, file inventory, and request-local index caches.
- Modify `crates/service/src/lib.rs`: register the new module and add the additive wire field.
- Modify `crates/service/src/service_local_sessions.rs`: orchestrate inventory, bounded parsing, sidecars, sorting, filtering, and pagination through the new I/O context.
- Modify `crates/service/src/tests/local_session_preview.rs`: cover bounded reads, sidecars, inventory selection, sorting, and pagination behavior.
- Modify `crates/service/src/tests/dispatch_fixtures.rs`: decode the additive local-session result field under `deny_unknown_fields`.
- Modify `fixtures/service-protocol/session.previewLocalSessions.request.json`: exercise explicit sort and direction.
- Modify `fixtures/service-protocol/session.previewLocalSessions.response.json`: include candidate-set completeness.
- Modify `apps/macos/Sources/SkillsCopilot/Models/LocalSessionPreview.swift`: decode and preserve candidate-set completeness.
- Modify `apps/macos/Sources/SkillsCopilot/Services/ServiceClient.swift`: encode summary/detail request fields.
- Modify `apps/macos/Sources/SkillsCopilot/Services/ServiceClientSessionRPC.swift`: send summary requests without content and selected-row detail requests with content.
- Modify `apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift`: keep startup/manual cache rows lightweight and replace only the selected row with detail.
- Modify `apps/macos/Tests/SkillsCopilotTests/LocalSessionPreviewModelTests.swift`: cover backward-compatible decoding and page merging.
- Modify `apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift`: assert summary/detail request separation.
- Modify `docs/architecture.md`, `docs/adapters/agent-adapters.md`, and `docs/service-protocol.md`: document root, cache, budget, sorting, and pagination semantics.

## Dependency Order

1. Task 1 establishes the adapter content boundary used by scanner bounded reads.
2. Task 2 establishes explicit roots and error degradation.
3. Task 3 adds scan budgets and connects root completeness to catalog cache updates.
4. Task 4 establishes the local-session bounded reader and request I/O context.
5. Tasks 5 and 6 both consume Task 4; implement Task 5 first to keep sidecar and index reads on the bounded path before reorganizing inventory and pagination.
6. Task 7 updates the service protocol and native model after Tasks 5 and 6 stabilize the result semantics.
7. Task 8 runs the repository-wide gates after both independently testable repair packages are complete.

---

### Task 1: Add an Owned-Content Adapter Parsing Boundary

**Files:**
- Modify: `crates/core/src/adapter.rs:5-12`
- Modify: `crates/adapters/src/lib.rs:1-18`
- Modify: `crates/adapters/src/claude_code/mod.rs:20-93`
- Modify: `crates/adapters/src/codex/mod.rs:30-110`
- Modify: `crates/adapters/src/opencode/mod.rs:24-100`
- Modify: `crates/adapters/src/pi/mod.rs:23-95`
- Modify: `crates/adapters/src/hermes/mod.rs:24-84`
- Modify: `crates/adapters/src/openclaw/mod.rs:21-100`
- Modify: `crates/adapters/fuzz/fuzz_targets/frontmatter_parser.rs:1-30`
- Test: the inline `#[cfg(test)]` modules in each adapter file listed above

**Interfaces:**
- Consumes: Existing `AgentAdapter::parse(&Path) -> Result<SkillInstance, AdapterError>` and each adapter's current parsing logic.
- Produces: `AgentAdapter::parse_content(&self, path: &Path, content: String) -> Result<SkillInstance, AdapterError>`; `parse` remains available and delegates to it.

- [ ] **Step 1: Add compile-failing equivalence tests to all six adapter modules**

Add this shared test helper to `crates/adapters/src/lib.rs`:

```rust
#[cfg(test)]
pub(crate) fn assert_parse_equivalent(
    adapter: &dyn skills_copilot_core::AgentAdapter,
    fixture: &std::path::Path,
) {
    let content = std::fs::read_to_string(fixture).expect("read fixture");
    let from_path = adapter.parse(fixture).expect("path parse");
    let from_content = adapter
        .parse_content(fixture, content)
        .expect("content parse");

    assert_eq!(from_content.name, from_path.name);
    assert_eq!(from_content.description, from_path.description);
    assert_eq!(from_content.frontmatter_raw, from_path.frontmatter_raw);
    assert_eq!(from_content.body, from_path.body);
    assert_eq!(from_content.state, from_path.state);
    assert_eq!(from_content.enabled, from_path.enabled);
    assert_eq!(from_content.permissions, from_path.permissions);
}
```

Call the helper from each adapter test with these inputs: Claude's `fixtures/claude-code/personal/valid-summarize/SKILL.md`; Codex's existing `write_skill("valid-codex", valid_frontmatter)` path; opencode's `fixtures/opencode/user-home/.config/opencode/skills/global-review/SKILL.md`; Pi's `fixtures/pi/global/agent/skills/global-pdf/SKILL.md`; Hermes's `fixtures/hermes/active-home/.hermes/skills/nested/research-brief/SKILL.md`; and OpenClaw's `fixtures/openclaw/skill-evidence/sample-openclaw-skill/SKILL.md`. Do not compare generated timestamps because adapter parsing initializes them to zero before scanner normalization.

- [ ] **Step 2: Run the adapter tests to verify the interface is absent**

Run:

```sh
cargo test -p skills-copilot-adapters
```

Expected: compilation fails with `no method named parse_content found` for each new equivalence test.

- [ ] **Step 3: Add the trait method and delegate each adapter's path reader**

Change `AgentAdapter` to the following exact interface:

```rust
pub trait AgentAdapter: Send + Sync {
    fn id(&self) -> AgentId;
    fn display_name(&self) -> &'static str;
    fn roots(&self, ctx: &AdapterContext) -> Vec<AdapterRoot>;
    fn parse(&self, path: &std::path::Path) -> Result<SkillInstance, AdapterError>;
    fn parse_content(
        &self,
        path: &std::path::Path,
        content: String,
    ) -> Result<SkillInstance, AdapterError>;
    fn is_enabled(&self, instance: &SkillInstance) -> bool;
    fn config_paths(&self, ctx: &AdapterContext) -> Vec<PathBuf>;
}
```

In each adapter, make `parse` contain only the bounded-independent path read and delegation:

```rust
fn parse(&self, path: &Path) -> Result<SkillInstance, AdapterError> {
    let content = std::fs::read_to_string(path)
        .map_err(|err| AdapterError::new(format!("failed to read skill: {err}")))?;
    self.parse_content(path, content)
}
```

For each adapter, create the exact `parse_content(&self, path: &Path, content: String)` signature declared in the trait, then move every existing statement after that adapter's `read_to_string` statement into it without changing statement order. The first moved statement is `let display_name = path` for Claude Code, `let fallback_name = path` for Codex, `let fallback_name = containing_dir_name(path)` for opencode/Hermes/OpenClaw, and `let fallback_name = fallback_skill_name(path)` for Pi. The final moved expression is that adapter's existing `SkillInstance` constructor and `Ok` return. The resulting method consumes the supplied `String` and performs no filesystem read.

In the fuzz target, preserve the existing path call and add a direct call when UTF-8 decoding succeeds:

```rust
if let Ok(content) = String::from_utf8(data.to_vec()) {
    let _ = ClaudeCodeAdapter.parse_content(&skill_path, content);
}
```

- [ ] **Step 4: Run focused tests and the core dependency check**

Run:

```sh
cargo test -p skills-copilot-core
cargo test -p skills-copilot-adapters
cargo clippy -p skills-copilot-core -p skills-copilot-adapters --all-targets --all-features -- -D warnings
```

Expected: all tests pass; clippy emits no warnings; `crates/core` contains no `std::fs` or `File::open` use.

- [ ] **Step 5: Commit the adapter boundary**

```sh
git add crates/core/src/adapter.rs crates/adapters/src/lib.rs crates/adapters/src/claude_code/mod.rs crates/adapters/src/codex/mod.rs crates/adapters/src/opencode/mod.rs crates/adapters/src/pi/mod.rs crates/adapters/src/hermes/mod.rs crates/adapters/src/openclaw/mod.rs crates/adapters/fuzz/fuzz_targets/frontmatter_parser.rs
git commit -m "refactor: add bounded adapter content parsing"
```

---

### Task 2: Resolve Explicit Scanner Roots and Degrade Filesystem Errors

**Files:**
- Modify: `crates/core/src/adapter.rs:5-12`
- Modify: `crates/adapters/src/claude_code/mod.rs:20-37`
- Modify: `crates/scanner/src/lib.rs:13-208`
- Modify: `docs/adapters/agent-adapters.md:6-32`
- Test: `crates/scanner/src/lib.rs` inline test module around the existing symlink tests

**Interfaces:**
- Consumes: `AgentAdapter::roots`, `AdapterRoot`, `RootSource`, `Scope`, and Task 1's `parse_content` method.
- Produces: `AgentAdapter::link_target_roots`, typed `ScanIssue`, `ScanIssueKind`, `partial_roots`, and the invariant that `scanned_roots` contains only completely enumerated roots.

- [ ] **Step 1: Replace the broad-root expectation with explicit-root and continuation tests**

Add these test cases to the scanner test module:

| Test | Exact fixture | Required assertions |
| --- | --- | --- |
| `rejects_symlink_target_outside_declared_adapter_roots` | disposable home with `.claude/skills/linked` pointing to `home/private-skill`, where `private-skill/SKILL.md` is valid and no adapter root names `private-skill` | `report.instances` contains no canonical path beneath `private-skill`; neither `scanned_roots` nor `partial_roots` contains that target |
| `allows_symlink_target_inside_another_declared_root_with_same_scope` | valid skill at `.agents/skills/shared/SKILL.md` and `.claude/skills/shared` pointing to that directory | exactly one Claude instance exists, its canonical path is the `.agents` file, and its display path is the `.claude` link path |
| `unavailable_root_is_reported_and_other_roots_continue` | a test adapter returns an ordinary file as its first `Extra` root and a valid skill directory as its second `Extra` root | the valid instance is returned; the ordinary file is in `skipped_roots`; one `RootUnavailable` issue names it |

Use the existing disposable-directory cleanup pattern and the valid Claude fixture text. Rename `follows_user_home_symlinks_that_stay_inside_user_home` to `allows_symlink_target_inside_another_declared_root_with_same_scope` and change its expected boundary from the entire home to the explicit compatibility root.

- [ ] **Step 2: Run scanner tests to verify current behavior violates the cases**

Run:

```sh
cargo test -p skills-copilot-scanner rejects_symlink_target_outside_declared_adapter_roots -- --exact
cargo test -p skills-copilot-scanner unavailable_root_is_reported_and_other_roots_continue -- --exact
```

Expected: the first test finds an out-of-allowlist instance and the second returns `ScannerError` before returning the valid instance.

- [ ] **Step 3: Add scanner issue types and two-pass canonical root resolution**

Add these exact report types near `ScanReport`:

```rust
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ScanIssueKind {
    RootUnavailable,
    RootOutsideAllowlist,
    DirectoryUnreadable,
    EntryUnreadable,
    FileUnreadable,
    FileTooLarge,
    BudgetExceeded,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScanIssue {
    pub path: PathBuf,
    pub kind: ScanIssueKind,
    pub detail: String,
}

#[derive(Debug, Default)]
pub struct ScanReport {
    pub instances: Vec<SkillInstance>,
    pub skipped_roots: Vec<PathBuf>,
    pub scanned_roots: Vec<PathBuf>,
    pub partial_roots: Vec<PathBuf>,
    pub issues: Vec<ScanIssue>,
}
```

Add private resolved-root and allowlist state:

```rust
#[derive(Debug, Clone)]
struct ResolvedScanRoot {
    declared: AdapterRoot,
    canonical: PathBuf,
}

#[derive(Debug, Clone)]
struct ResolvedAllowedRoot {
    scope: Scope,
    canonical: PathBuf,
}

fn target_is_allowlisted(
    path: &Path,
    scope: Scope,
    roots: &[ResolvedAllowedRoot],
) -> bool {
    roots
        .iter()
        .filter(|root| root.scope == scope)
        .any(|root| path.starts_with(&root.canonical))
}
```

Resolve all existing walked roots and link-target roots before walking any root. For each unavailable or unresolvable walked root, push its declared path to `skipped_roots`, append `ScanIssueKind::RootUnavailable`, and continue. A missing link-target root is absent from the allowlist because it is not itself a requested walk. A built-in walked root whose leaf is itself a symlink is accepted only when its canonical target is contained by another allowed root of the same scope; `Configured`, `Admin`, `Plugin`, `System`, and `Extra` walked roots remain explicit self-authorizing roots.

Add this default interface to `AgentAdapter`:

```rust
fn link_target_roots(&self, _ctx: &AdapterContext) -> Vec<AdapterRoot> {
    Vec::new()
}
```

Override it for Claude so the supported shared link remains explicit without walking every shared skill directly:

```rust
fn link_target_roots(&self, ctx: &AdapterContext) -> Vec<AdapterRoot> {
    vec![AdapterRoot {
        scope: Scope::AgentGlobal,
        path: ctx.user_home.join(".agents/skills"),
        source: RootSource::Compatibility,
    }]
}
```

Remove `allowed_target_base` and make every descendant link check call `target_is_allowlisted(&resolved, root.scope, allowed_roots)`.

- [ ] **Step 4: Make directory and entry failures mark a root partial without returning early**

Change `visit_root` to return completion state instead of propagating ordinary filesystem errors:

```rust
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum RootWalkStatus {
    Complete,
    Partial,
}

fn visit_root(
    adapter: &dyn AgentAdapter,
    ctx: &AdapterContext,
    root: &ResolvedScanRoot,
    allowed_roots: &[ResolvedAllowedRoot],
    overrides: &SkillConfigOverrides,
    report: &mut ScanReport,
) -> RootWalkStatus
```

On `read_dir`, `entry.file_type`, or required canonicalization failure, append the matching issue, set a local `partial = true`, and continue with other queued directories and roots. After the walk, push the root canonical path to `partial_roots` when partial, otherwise to `scanned_roots`. Do not push a root to `scanned_roots` before traversal.

- [ ] **Step 5: Run scanner tests and adapter-root regression tests**

Run:

```sh
cargo test -p skills-copilot-scanner
cargo test -p skills-copilot-adapters claude_code
```

Expected: all scanner tests pass; the valid second root survives the invalid first root; only explicit same-scope roots accept link targets.

- [ ] **Step 6: Commit explicit roots and degradation behavior**

```sh
git add crates/core/src/adapter.rs crates/adapters/src/claude_code/mod.rs crates/scanner/src/lib.rs docs/adapters/agent-adapters.md
git commit -m "fix: bound scanner roots and degrade read errors"
```

---

### Task 3: Enforce Scanner Budgets and Protect Catalog Missing-Sweep

**Files:**
- Modify: `crates/scanner/src/lib.rs:41-347`
- Modify: `crates/commands/src/lib.rs:404-467`
- Modify: `crates/commands/src/tests.rs`
- Modify: `docs/architecture.md:29-56`
- Test: `crates/scanner/src/lib.rs` inline tests
- Test: `crates/commands/src/tests.rs`

**Interfaces:**
- Consumes: Task 1's `parse_content`; Task 2's `ResolvedScanRoot`, `RootWalkStatus`, `ScanIssue`, and complete-root invariant.
- Produces: `ScanLimits`, `ScanBudget`, `ScanStats`, bounded file reads, and cache updates that sweep only complete roots.

- [ ] **Step 1: Add failing scanner budget tests with injected small limits**

Add a private test entry point and tests that use small values rather than large fixtures:

```rust
let limits = ScanLimits {
    max_depth: 8,
    max_directories: 8,
    max_entries: 8,
    max_skill_files: 8,
    max_skill_bytes: 32,
    max_total_skill_bytes: 48,
};
let report = scan_agent_with_limits(&ClaudeCodeAdapter, &ctx, limits)
    .expect("bounded scan returns a report");
```

Add four named tests with the following exact setup and assertions:

| Test | Fixture setup | Required assertions |
| --- | --- | --- |
| `oversized_skill_becomes_broken_without_stopping_scan` | `oversized/SKILL.md` contains 129 ASCII bytes; `valid/SKILL.md` contains a valid frontmatter document shorter than 128 bytes; `max_skill_bytes=128` | `oversized` is `SkillState::Broken`, `valid` is `SkillState::Loaded`, and a `FileTooLarge` issue names the oversized canonical path |
| `total_byte_budget_marks_root_partial_without_unbounded_read` | two valid skill files each shorter than 128 bytes; `max_total_skill_bytes` is one byte less than their combined metadata lengths | at least one instance is returned, `BudgetExceeded` is present, `budget_exhausted=true`, and the canonical root is in `partial_roots` but not `scanned_roots` |
| `entry_budget_marks_root_partial_and_records_budget_issue` | nine lexically named child directories with `max_entries=8` | `entries_seen=8`, `BudgetExceeded` is present, and the root is partial |
| `complete_root_remains_eligible_for_missing_sweep` | one valid skill with every limit above fixture size | there are no issues, `partial_roots` is empty, and the canonical root is the only `scanned_roots` entry |

Use a shared `test_scan_limits()` returning depth 8, directories 8, entries 8, skill files 8, 128 bytes per skill, and 1,024 total bytes; override only the field named in each row.

- [ ] **Step 2: Add failing catalog-cache tests**

In `crates/commands/src/tests.rs`, add two tests:

| Test | Exact fixture | Required assertions |
| --- | --- | --- |
| `partial_scan_upserts_seen_rows_without_marking_unseen_rows_missing` (`#[cfg(unix)]`) | seed catalog rows `observed` and `unobserved`; keep `observed/SKILL.md`, remove `unobserved/SKILL.md`, and add a dangling directory link under the same root so canonicalization records a partial-root issue; run refresh | `observed.last_seen` advances and remains loaded; `unobserved.state` remains its pre-refresh state; no missing event is created for it |
| `complete_scan_marks_removed_rows_missing` | seed the same rows, remove `unobserved/SKILL.md`, and complete refresh without issues | `observed` remains loaded; `unobserved.state == "missing"`; one missing event exists for `unobserved` |

- [ ] **Step 3: Run the focused tests to verify missing types and current cache behavior**

Run:

```sh
cargo test -p skills-copilot-scanner oversized_skill_becomes_broken_without_stopping_scan -- --exact
cargo test -p skills-copilot-commands partial_scan_upserts_seen_rows_without_marking_unseen_rows_missing -- --exact
```

Expected: scanner compilation fails before `ScanLimits` exists, and the command test demonstrates that current scan-root bookkeeping is not sufficient to preserve the unseen row.

- [ ] **Step 4: Implement exact scanner limits, counters, and bounded reads**

Add these production defaults and counters:

```rust
#[derive(Debug, Clone, Copy)]
struct ScanLimits {
    max_depth: usize,
    max_directories: usize,
    max_entries: usize,
    max_skill_files: usize,
    max_skill_bytes: u64,
    max_total_skill_bytes: u64,
}

impl Default for ScanLimits {
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_directories: 50_000,
            max_entries: 200_000,
            max_skill_files: 25_000,
            max_skill_bytes: 2 * 1024 * 1024,
            max_total_skill_bytes: 256 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct ScanStats {
    pub directories_visited: usize,
    pub entries_seen: usize,
    pub skill_files_seen: usize,
    pub bytes_read: u64,
    pub budget_exhausted: bool,
}

#[derive(Debug, Default)]
struct ScanBudget {
    stats: ScanStats,
}
```

Add `pub stats: ScanStats` to `ScanReport` in this task, and add the test-only entry point used by Step 1:

```rust
fn scan_agent_with_limits(
    adapter: &dyn AgentAdapter,
    ctx: &AdapterContext,
    limits: ScanLimits,
) -> Result<ScanReport, ScannerError>
```

Keep `scan_agent` as the production wrapper:

```rust
pub fn scan_agent(
    adapter: &dyn AgentAdapter,
    ctx: &AdapterContext,
) -> Result<ScanReport, ScannerError> {
    scan_agent_with_limits(adapter, ctx, ScanLimits::default())
}
```

Extend Task 2's `visit_root` signature in this task with `limits: &ScanLimits` and `budget: &mut ScanBudget` immediately before `report`.

Add a bounded read that never allocates beyond the per-file cap plus one byte:

```rust
fn read_skill_content_bounded(path: &Path, max_bytes: u64) -> std::io::Result<Option<String>> {
    use std::io::Read;

    let file = fs::File::open(path)?;
    let mut bytes = Vec::with_capacity((max_bytes.min(64 * 1024)) as usize);
    file.take(max_bytes + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Ok(None);
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}
```

Before opening a skill, reserve its metadata length against both byte limits. Call `adapter.parse_content(&canonical_path, content)` only after the bounded read succeeds. A file that is too large or unreadable becomes a broken instance when its canonical path is known, so enumeration may still complete. Directory, entry, depth, and traversal-budget exhaustion mark the root partial.

- [ ] **Step 5: Restrict catalog missing-sweep to complete roots**

Keep the command call shape but pass only the now-strict `report.scanned_roots`:

```rust
catalog.mark_missing_except_for_project_context(
    adapter.id().as_str(),
    ctx.project_root.as_deref(),
    &report.scanned_roots,
    &seen,
)?;
```

Add `partial_roots` and typed issue summaries to `AgentCatalogScanReport` so refresh diagnostics can distinguish skipped, partial, and complete roots. Upsert `report.instances` before missing-sweep exactly as today; do not sweep when `scanned_roots` is empty.

- [ ] **Step 6: Run focused tests and the existing 10k benchmark**

Run:

```sh
cargo test -p skills-copilot-scanner
cargo test -p skills-copilot-commands
pnpm benchmark:10k
```

Expected: all tests pass; the benchmark prints `scanned=10000 records=10000`; `budget_exhausted` remains false under production defaults.

- [ ] **Step 7: Commit scanner budgets and cache protection**

```sh
git add crates/scanner/src/lib.rs crates/commands/src/lib.rs crates/commands/src/tests.rs docs/architecture.md
git commit -m "fix: bound scanner work and protect partial cache refreshes"
```

---

### Task 4: Add a Bounded Head-and-Tail Local-Session Reader

**Files:**
- Create: `crates/service/src/service_local_session_io.rs`
- Modify: `crates/service/src/lib.rs:42-52`
- Modify: `crates/service/src/service_local_sessions.rs:1-5,757-963`
- Test: inline `#[cfg(test)]` module in `crates/service/src/service_local_session_io.rs`
- Test: `crates/service/src/tests/local_session_preview.rs`

**Interfaces:**
- Consumes: `ServiceError`, local-session redaction/parsing helpers, and seekable regular files.
- Produces: `LocalSessionReadLimits`, `LocalSessionReadBudget`, `BoundedReadSpec`, `BoundedText`, and `read_bounded_text` without `BufRead::read_line`.

- [ ] **Step 1: Create the module with failing bounded-reader tests**

Register the module in `crates/service/src/lib.rs`:

```rust
mod service_local_session_io;
```

Create the new file with tests for this exact interface:

```rust
#[derive(Debug, Clone, Copy)]
pub(crate) struct BoundedReadSpec {
    pub(crate) head_bytes: usize,
    pub(crate) tail_bytes: usize,
    pub(crate) line_fragment_bytes: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct BoundedText {
    pub(crate) head: String,
    pub(crate) tail: String,
    pub(crate) truncated: bool,
    pub(crate) bytes_read: usize,
}

pub(crate) fn read_bounded_text(
    path: &Path,
    spec: BoundedReadSpec,
    budget: &mut LocalSessionReadBudget,
) -> Result<BoundedText, ServiceError>;

fn read_bounded_from<R: Read + Seek>(
    reader: &mut R,
    file_len: u64,
    spec: BoundedReadSpec,
    budget: &mut LocalSessionReadBudget,
) -> std::io::Result<BoundedText>;
```

Add these tests with exact outcomes:

| Test | Input/specification | Required assertions |
| --- | --- | --- |
| `bounded_reader_reads_disjoint_head_and_tail_windows` | `head\n` + 128 `m` bytes + `\ntail\n`; head 5, tail 6, fragment 8 | `head == "head\n"`, `tail == "tail\n"`, `truncated=true`, and `bytes_read <= 19` |
| `bounded_reader_never_exceeds_request_byte_budget` | a synthetic 1 MiB seekable reader; head 64, tail 32, fragment 16; request budget 80 | `bytes_read <= 80`, remaining request bytes are zero, and neither retained string exceeds its granted window |
| `bounded_reader_keeps_utf8_boundaries_valid` | `"开始"` + 128 ASCII bytes + `"结束"` with both windows splitting a multibyte character | both returned strings pass UTF-8 validation and contain no partial scalar |
| `bounded_reader_handles_one_line_larger_than_both_windows` | a synthetic 8 MiB line; head 64, tail 32, fragment 16 | `truncated=true`, retained bytes are at most 96, and the largest single `Read::read` buffer requested by the implementation is at most 64 KiB |

Implement the synthetic reader as an inline test `RecordingReadSeek { len, position, max_requested }` that yields `b'x'`, implements `SeekFrom::{Start, End, Current}`, and records `buf.len()` on every `read`. This proves the reader does not allocate its logical file size.

- [ ] **Step 2: Run the new unit-test target and verify the implementation is absent**

Run:

```sh
cargo test -p skills-copilot-service service_local_session_io::tests -- --nocapture
```

Expected: compilation fails because `LocalSessionReadBudget` and `read_bounded_text` are not implemented.

- [ ] **Step 3: Implement the limits, budget, and fixed-window reader**

Add these exact defaults:

```rust
#[derive(Debug, Clone, Copy)]
pub(crate) struct LocalSessionReadLimits {
    pub(crate) primary_head_bytes: usize,
    pub(crate) primary_tail_bytes: usize,
    pub(crate) max_line_fragment_bytes: usize,
    pub(crate) max_sidecar_file_bytes: usize,
    pub(crate) max_sidecar_session_bytes: usize,
    pub(crate) max_sidecar_files: usize,
    pub(crate) max_preview_read_bytes: usize,
    pub(crate) max_inventory_directories: usize,
    pub(crate) max_inventory_entries: usize,
}

impl Default for LocalSessionReadLimits {
    fn default() -> Self {
        Self {
            primary_head_bytes: 384 * 1024,
            primary_tail_bytes: 128 * 1024,
            max_line_fragment_bytes: 64 * 1024,
            max_sidecar_file_bytes: 64 * 1024,
            max_sidecar_session_bytes: 512 * 1024,
            max_sidecar_files: 240,
            max_preview_read_bytes: 64 * 1024 * 1024,
            max_inventory_directories: 20_000,
            max_inventory_entries: 100_000,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LocalSessionReadBudget {
    remaining_bytes: usize,
}

impl LocalSessionReadBudget {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            remaining_bytes: limit,
        }
    }

    fn claim(&mut self, requested: usize) -> usize {
        let granted = requested.min(self.remaining_bytes);
        self.remaining_bytes -= granted;
        granted
    }
}

pub(crate) struct LocalSessionIoContext {
    pub(crate) limits: LocalSessionReadLimits,
    pub(crate) budget: LocalSessionReadBudget,
}

impl LocalSessionIoContext {
    pub(crate) fn new(limits: LocalSessionReadLimits) -> Self {
        Self {
            budget: LocalSessionReadBudget::new(limits.max_preview_read_bytes),
            limits,
        }
    }
}
```

`read_bounded_text` must open the file once, obtain its length, and delegate to `read_bounded_from`. The generic helper reads a bounded head with `Read::take`, seeks to a tail start no earlier than the end of the head, and reads at most `tail_bytes + line_fragment_bytes` for newline alignment. Convert bounded windows with `String::from_utf8_lossy`, trimming incomplete UTF-8 at window boundaries. Never call `read_line`.

- [ ] **Step 4: Route primary session files through the bounded reader**

Replace `read_local_session_file_content` with a function that consumes the request context:

```rust
fn read_local_session_file_content(
    path: &Path,
    io: &mut LocalSessionIoContext,
) -> Result<BoundedText, ServiceError> {
    read_bounded_text(
        path,
        BoundedReadSpec {
            head_bytes: io.limits.primary_head_bytes,
            tail_bytes: io.limits.primary_tail_bytes,
            line_fragment_bytes: io.limits.max_line_fragment_bytes,
        },
        &mut io.budget,
    )
}
```

Construct compacted content from complete head records, a single truncation marker, and complete tail records. For a truncated one-line JSON record, retain only supported scalar fragments (`type`, `role`, `text`, `content`, `title`, `aiTitle`, `timestamp`, `sessionId`, `id`, `cwd`) found in the bounded head/tail windows. This preserves the existing large-image title test without reconstructing the omitted blob.

- [ ] **Step 5: Add behavior tests for late events**

In `crates/service/src/tests/local_session_preview.rs`, add the tests `local_session_preview_keeps_tail_message_after_large_middle_record`, `local_session_preview_keeps_tail_timestamp_after_read_cap`, and `local_session_preview_bounds_single_line_json`. For the first two, write an initial user event, a 600 KiB middle record, and a final user event with timestamp `2026-07-10T08:09:10Z`; assert the final text is a title/content item and `ended_at == 1_783_670_950_000`. For the one-line case, place a 600 KiB `data` string before a final text field in one JSON object; assert the final text is present and the serialized response contains neither the filler nor the complete `data` value.

- [ ] **Step 6: Run focused local-session tests**

Run:

```sh
cargo test -p skills-copilot-service service_local_session_io::tests
cargo test -p skills-copilot-service local_session_preview_reads_past_large_claude_file_history_snapshots -- --exact
cargo test -p skills-copilot-service local_session_preview_compacts_large_claude_image_messages_for_titles -- --exact
cargo test -p skills-copilot-service local_session_preview_keeps_tail_message_after_large_middle_record -- --exact
```

Expected: all tests pass; the bounded-reader tests prove requested and retained bytes stay within their configured windows.

- [ ] **Step 7: Commit bounded local-session reads**

```sh
git add crates/service/src/lib.rs crates/service/src/service_local_session_io.rs crates/service/src/service_local_sessions.rs crates/service/src/tests/local_session_preview.rs
git commit -m "fix: bound local session head and tail reads"
```

---

### Task 5: Bound Sidecars and Cache Codex Indexes Per Request

**Files:**
- Modify: `crates/service/src/service_local_session_io.rs`
- Modify: `crates/service/src/service_local_sessions.rs:757-1080,1328-1367`
- Test: `crates/service/src/tests/local_session_preview.rs`

**Interfaces:**
- Consumes: Task 4's `LocalSessionIoContext`, limits, request byte budget, bounded reader, and compacted primary content.
- Produces: `LocalSessionRequestCache`, `SessionSidecarBudget`, bounded opencode enrichment, and one bounded Codex index load per store per request.

- [ ] **Step 1: Add failing sidecar and index-cache tests**

Add five tests to `local_session_preview.rs` with these exact fixtures:

| Test | Fixture and assertion |
| --- | --- |
| `opencode_sidecars_share_one_file_budget_per_session` | one session, one message, and 241 lexically named part files; assert only 240 sidecars contribute content and a truncation gap note is returned |
| `opencode_sidecars_share_one_byte_budget_per_session` | three 256 KiB sidecars under one session; assert aggregated retained sidecar content is at most 512 KiB and the primary row still exists |
| `oversized_opencode_sidecar_does_not_drop_the_primary_session` | one 1 MiB sidecar and a primary session titled `Primary title`; assert exactly one row with that title and a sidecar truncation gap note |
| `codex_index_cache_loads_once_per_store` | inline I/O-module test calls `codex_titles_or_load` twice for one canonical root with a closure that increments `Cell<usize>`; assert the counter equals one and both lookups return the same map |
| `request_local_index_cache_is_empty_on_the_next_preview` | call preview, rewrite the same index title, call preview again; assert the second title reflects the rewrite, proving the first request's map was dropped |

Place `codex_index_cache_loads_once_per_store` in the inline test module of `service_local_session_io.rs`; no cache diagnostic is added to the service wire result.

- [ ] **Step 2: Run the focused tests and verify unbounded sidecar behavior remains**

Run:

```sh
cargo test -p skills-copilot-service opencode_sidecars_share_one_file_budget_per_session -- --exact
cargo test -p skills-copilot-service codex_index_cache_loads_once_per_store -- --exact
```

Expected: tests fail because sidecar counts are independently capped per directory and Codex indexes are reread for each session.

- [ ] **Step 3: Implement the request context and per-session sidecar budget**

Add these exact types to `service_local_session_io.rs`:

```rust
#[derive(Debug, Default)]
pub(crate) struct LocalSessionRequestCache {
    pub(crate) codex_titles: HashMap<PathBuf, HashMap<String, String>>,
}

struct SessionSidecarBudget {
    remaining_files: usize,
    remaining_bytes: usize,
}
```

Extend Task 4's `LocalSessionIoContext` with `cache: LocalSessionRequestCache`, initialize it with `LocalSessionRequestCache::default()`, and add this cache method:

```rust
impl LocalSessionRequestCache {
    pub(crate) fn codex_titles_or_load<F>(
        &mut self,
        root: PathBuf,
        load: F,
    ) -> &HashMap<String, String>
    where
        F: FnOnce() -> HashMap<String, String>,
    {
        self.codex_titles.entry(root).or_insert_with(load)
    }
}
```

Create one `SessionSidecarBudget` per primary opencode session with 240 files and 512 KiB. Sort message and part paths lexically, consume the same budget across both levels, and read each file with a 32 KiB head plus 32 KiB tail `BoundedReadSpec`. Remove all sidecar `fs::read_to_string` calls and the `message.clone()` allocation.

- [ ] **Step 4: Cache bounded Codex index maps for one request**

Change title lookup to this interface:

```rust
fn codex_session_index_title(
    io: &mut LocalSessionIoContext,
    path: &Path,
    session_id: &str,
) -> Option<String>
```

Resolve the `.codex` root once. On cache miss, read `session_index.jsonl` and then `history.jsonl` through `read_bounded_text`, parse supported complete head/tail lines into `HashMap<session_id, title>`, and insert the map under the canonical root. On cache hit, perform no filesystem read. If the bounded index omits an ID, retain the existing session-derived title fallback.

- [ ] **Step 5: Pass one I/O context through the full preview request**

Construct the context once at the start of preview:

```rust
let mut io = LocalSessionIoContext::new(LocalSessionReadLimits::default());
```

Pass `&mut io` through `local_session_preview_row`, `read_local_session_file_content`, `enrich_local_session_content`, `append_opencode_parts`, and `codex_session_index_title`. Do not place the context on `ServiceHost` and do not use static state.

- [ ] **Step 6: Run service tests**

Run:

```sh
cargo test -p skills-copilot-service opencode_sidecars
cargo test -p skills-copilot-service codex_index
cargo test -p skills-copilot-service local_session_preview
```

Expected: all focused and existing local-session tests pass; the second request sees index changes; no service source path contains sidecar or index `fs::read_to_string` calls.

- [ ] **Step 7: Commit sidecar bounds and request cache**

```sh
git add crates/service/src/service_local_session_io.rs crates/service/src/service_local_sessions.rs crates/service/src/tests/local_session_preview.rs
git commit -m "fix: bound session sidecars and cache indexes per request"
```

---

### Task 6: Build a Deterministic Inventory and Sort Before Pagination

**Files:**
- Modify: `crates/service/src/service_local_session_io.rs`
- Modify: `crates/service/src/service_local_sessions.rs:6-250,669-755`
- Modify: `crates/service/src/tests/local_session_preview.rs`

**Interfaces:**
- Consumes: Task 4's limits/context and Task 5's request-scoped I/O path.
- Produces: `LocalSessionFileCandidate`, `LocalSessionInventory`, `LocalSessionSort`, `SortDirection`, deterministic newest-candidate selection, and server-side sort-before-page behavior.

- [ ] **Step 1: Add failing pure inventory and sorting tests**

Add unit tests in `service_local_session_io.rs` using synthetic candidates:

```rust
#[test]
fn inventory_selects_newest_candidates_independent_of_input_order() {
    let candidates = vec![
        candidate("old.jsonl", 100),
        candidate("new.jsonl", 300),
        candidate("middle.jsonl", 200),
    ];
    let selected = select_newest_candidates(candidates, 2);
    assert_eq!(paths(selected), vec!["new.jsonl", "middle.jsonl"]);
}
```

Add service behavior tests in `local_session_preview.rs` using three rows titled `Alpha`, `Bravo`, and `Charlie` with modified times 100, 300, and 200 respectively:

| Test | Required assertion |
| --- | --- |
| `preview_sorts_title_ascending_and_descending` | ascending titles are `Alpha, Bravo, Charlie`; descending titles are `Charlie, Bravo, Alpha` |
| `preview_sorts_modified_at_ascending_and_descending` | ascending times are `100, 200, 300`; descending times are `300, 200, 100` |
| `pagination_is_applied_after_server_side_sort` | title-ascending offset 1/limit 1 returns only `Bravo` |
| `consecutive_pages_equal_unpaged_order_without_duplicates` | two title-ascending pages concatenate to the unpaged IDs and the ID set size equals the vector size |
| `max_files_selects_newest_candidates` | five candidates with times 100 through 500 and `max_files=2` return times 500 and 400; inventory retains `total_candidate_count=5` and `truncated=true` |
| `invalid_sort_or_direction_is_rejected` | `sort="size"` and `direction="sideways"` each return `error.code == "invalid_request"` |

Use pure candidate/row helpers for ordering assertions rather than relying on `read_dir` order. The filesystem integration covers only inventory counts and response flags.

- [ ] **Step 2: Run sorting tests and verify current hard-coded order**

Run:

```sh
cargo test -p skills-copilot-service preview_sorts_title_ascending_and_descending -- --exact
cargo test -p skills-copilot-service max_files_selects_newest_candidates -- --exact
```

Expected: title ascending and descending return the same hard-coded modified-desc order, and the inventory test cannot observe omitted candidates truthfully.

- [ ] **Step 3: Implement bounded metadata inventory without early file-count return**

Add these exact types and selector:

```rust
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct LocalSessionFileCandidate {
    pub(crate) path: PathBuf,
    pub(crate) modified_at: i64,
}

#[derive(Debug, Default)]
pub(crate) struct LocalSessionInventory {
    pub(crate) candidates: Vec<LocalSessionFileCandidate>,
    pub(crate) total_candidate_count: usize,
    pub(crate) truncated: bool,
}

pub(crate) fn select_newest_candidates(
    mut candidates: Vec<LocalSessionFileCandidate>,
    max_files: usize,
) -> Vec<LocalSessionFileCandidate> {
    candidates.sort_by(|left, right| {
        right
            .modified_at
            .cmp(&left.modified_at)
            .then_with(|| left.path.cmp(&right.path))
    });
    candidates.truncate(max_files);
    candidates
}
```

Replace `collect_local_session_files` with `collect_local_session_inventory`. Walk until the directory/entry budget completes or is exhausted; never return because the selected file vector reached `max_files`. Count every supported canonical candidate encountered, record metadata only, set `truncated` when traversal budgets stop discovery, then call `select_newest_candidates`.

Carry a local `candidate_set_was_truncated` boolean when either inventory traversal was truncated or `total_candidate_count > selected_candidates.len()`. Task 7 serializes this already-computed value without changing inventory selection.

- [ ] **Step 4: Parse and validate sort parameters**

Add these exact enums and parsers:

```rust
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum LocalSessionSort {
    ModifiedAt,
    Title,
}

impl LocalSessionSort {
    fn parse(value: Option<&str>) -> Result<Self, ServiceError> {
        match value.map(str::trim).filter(|value| !value.is_empty()) {
            None | Some("recent") | Some("modified_at") => Ok(Self::ModifiedAt),
            Some("title") => Ok(Self::Title),
            Some(value) => Err(ServiceError::InvalidRequest(format!(
                "unsupported local session sort '{value}'"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    fn parse(value: Option<&str>, default: Self) -> Result<Self, ServiceError> {
        match value.map(str::trim).filter(|value| !value.is_empty()) {
            None => Ok(default),
            Some("asc") => Ok(Self::Asc),
            Some("desc") => Ok(Self::Desc),
            Some(value) => Err(ServiceError::InvalidRequest(format!(
                "unsupported local session direction '{value}'"
            ))),
        }
    }
}
```

Default modified/recent to descending. Default title to ascending when direction is absent.

- [ ] **Step 5: Sort all matched rows before offset and limit**

Implement one comparator function:

```rust
fn sort_local_session_rows(
    rows: &mut [LocalSessionPreviewRow],
    sort: LocalSessionSort,
    direction: SortDirection,
) {
    rows.sort_by(|left, right| {
        let primary = match sort {
            LocalSessionSort::ModifiedAt => left.modified_at.cmp(&right.modified_at),
            LocalSessionSort::Title => left
                .title
                .to_lowercase()
                .cmp(&right.title.to_lowercase()),
        };
        let primary = match direction {
            SortDirection::Asc => primary,
            SortDirection::Desc => primary.reverse(),
        };
        primary
            .then_with(|| left.title.cmp(&right.title))
            .then_with(|| right.modified_at.cmp(&left.modified_at))
            .then_with(|| left.id.cmp(&right.id))
    });
}
```

Call it after filtering and deduplication, then compute `total_matched_count`, `offset`, `page_end`, `has_more`, and `next_offset`. `has_more` must describe only the selected candidate set; `next_offset` is `Some(page_end)` only when `page_end < total_matched_count`.

- [ ] **Step 6: Run inventory, sorting, and pagination tests**

Run:

```sh
cargo test -p skills-copilot-service inventory_
cargo test -p skills-copilot-service preview_sorts_
cargo test -p skills-copilot-service pagination_
cargo test -p skills-copilot-service consecutive_pages_
cargo test -p skills-copilot-service invalid_sort_or_direction_is_rejected -- --exact
```

Expected: all tests pass; ascending and descending differ; concatenated pages exactly equal the unpaged ID order; omitted candidates set the truncation flag without producing an unreachable offset.

- [ ] **Step 7: Commit inventory and server sorting**

```sh
git add crates/service/src/service_local_session_io.rs crates/service/src/service_local_sessions.rs crates/service/src/tests/local_session_preview.rs
git commit -m "fix: sort and page bounded session inventories"
```

---

### Task 7: Synchronize the Summary/Detail Service Protocol and macOS Cache

**Files:**
- Modify: `crates/service/src/lib.rs:281-399`
- Modify: `crates/service/src/tests/dispatch_fixtures.rs:387-409`
- Modify: `crates/service/src/tests/local_session_preview.rs`
- Modify: `fixtures/service-protocol/session.previewLocalSessions.request.json`
- Modify: `fixtures/service-protocol/session.previewLocalSessions.response.json`
- Modify: `apps/macos/Sources/SkillsCopilot/Services/ServiceClient.swift:121-151`
- Modify: `apps/macos/Sources/SkillsCopilot/Services/ServiceClientSessionRPC.swift:3-38`
- Modify: `apps/macos/Sources/SkillsCopilot/Models/LocalSessionPreview.swift:140-620`
- Modify: `apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift:751-758,1982-2061`
- Modify: `apps/macos/Tests/SkillsCopilotTests/LocalSessionPreviewModelTests.swift`
- Modify: `apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift`
- Modify: `docs/service-protocol.md:49-175`
- Modify: `docs/adapters/agent-adapters.md:6-32`
- Modify: `docs/architecture.md:45-60`

**Interfaces:**
- Consumes: Task 6's `candidate_set_was_truncated` value, existing snake/camel decoding compatibility, and existing pagination merge logic.
- Produces: additive `candidate_set_truncated: bool`, row-level `content_included: bool`, optional request `include_content_items: Option<bool>` and `session_id: Option<String>` in Rust; matching Swift request/model fields; documented summary/detail, sort, and count semantics.

- [ ] **Step 1: Add failing Rust and Swift decoding assertions**

In the Rust service behavior test, assert both complete and truncated cases expose the field:

```rust
assert_eq!(
    result
        .get("candidate_set_truncated")
        .and_then(Value::as_bool),
    Some(true),
);
```

In `LocalSessionPreviewModelTests.swift`, add payloads with and without the field:

```swift
let explicit = try decodePreview("""
{"candidate_set_truncated":true,"session_rows":[]}
""")
try expectTrue(explicit.candidateSetTruncated, "Explicit truncation should decode.")

let legacy = try decodePreview("""
{"session_rows":[]}
""")
try expectFalse(legacy.candidateSetTruncated, "Missing additive field should default false.")
```

Add a page-merge assertion that `mergingPage` retains `true` when either page is truncated.

Add four Rust behavior tests to `local_session_preview.rs`:

| Test | Required assertion |
| --- | --- |
| `summary_rows_omit_content_items_and_mark_not_included` | request sends `include_content_items=false`; every row has `content_included=false` and an empty `content_items` array |
| `missing_include_content_items_preserves_legacy_detail` | request omits the field; returned row has `content_included=true` and at least one parsed content item |
| `detail_session_id_reads_only_target_candidate` | inventory contains two sessions; request sends the first row ID as `session_id`; the result contains that row only and the test I/O context records exactly its canonical path |
| `session_preview_never_persists_raw_session_content` | run both summary and detail requests against fresh app data; no file containing session content is created under app data |

For the read-path assertion, extend `LocalSessionIoContext` under `#[cfg(test)]` with `primary_paths_read: Vec<PathBuf>`, append the canonical path immediately before a primary file read, and split the host implementation into this wrapper plus injectable context:

```rust
pub(crate) struct LocalSessionIoContext {
    pub(crate) limits: LocalSessionReadLimits,
    pub(crate) budget: LocalSessionReadBudget,
    pub(crate) cache: LocalSessionRequestCache,
    #[cfg(test)]
    pub(crate) primary_paths_read: Vec<PathBuf>,
}
```

Add `#[cfg(test)] primary_paths_read: Vec::new(),` to `LocalSessionIoContext::new` and push `path.to_path_buf()` at the start of `read_local_session_file_content`.

```rust
pub fn preview_local_sessions(
    &self,
    params: LocalSessionPreviewParams,
) -> Result<LocalSessionPreviewResult, ServiceError> {
    let mut io = LocalSessionIoContext::new(LocalSessionReadLimits::default());
    self.preview_local_sessions_with_io(params, &mut io)
}

fn preview_local_sessions_with_io(
    &self,
    params: LocalSessionPreviewParams,
    io: &mut LocalSessionIoContext,
) -> Result<LocalSessionPreviewResult, ServiceError>
```

In the Swift model test, add a summary row with `"content_included":false` and no items, plus a legacy row with neither content flag nor items. Assert the summary decodes `contentIncluded == false` and the legacy row decodes `contentIncluded == true`.

In `SkillStoreTests.swift`, add `startupPreviewRequestsSummaryRows` and `selectingSummaryRequestsOnlySelectedDetail`. The mock service must record encoded params; assert list calls contain `include_content_items=false` and no `session_id`, while selection produces one call containing `include_content_items=true` and exactly the selected row ID.

- [ ] **Step 2: Run protocol/model tests and verify the field is absent**

Run:

```sh
cargo test -p skills-copilot-service max_files_reports_candidate_set_truncated -- --exact
cargo test -p skills-copilot-service summary_rows_omit_content_items_and_mark_not_included -- --exact
cargo test -p skills-copilot-service detail_session_id_reads_only_target_candidate -- --exact
pnpm test:macos-native-models
```

Expected: Rust cannot find the serialized completeness/content fields or pre-read detail filter; Swift compilation fails because `candidateSetTruncated` and `contentIncluded` are not defined.

- [ ] **Step 3: Add Rust summary/detail fields and filter detail before content reads**

Extend `LocalSessionPreviewParams` with optional fields so omitted requests retain legacy detail behavior:

```rust
#[serde(default)]
pub include_content_items: Option<bool>,
#[serde(default)]
pub session_id: Option<String>,
```

At the start of preview, resolve them once:

```rust
let include_content_items = params.include_content_items.unwrap_or(true);
let requested_session_id = params
    .session_id
    .as_deref()
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(ToOwned::to_owned);
```

After metadata inventory, but before newest-candidate selection and before the loop that calls `local_session_preview_row`, apply the stable row-ID filter:

```rust
let candidates = if let Some(session_id) = requested_session_id.as_deref() {
    inventory
        .candidates
        .into_iter()
        .filter(|candidate| local_session_row_id(&candidate.path) == session_id)
        .collect::<Vec<_>>()
} else {
    select_newest_candidates(inventory.candidates, max_files)
};
```

This filter must run before `read_local_session_file_content`; detail requests set `limit=1` and `offset=0`. Do not match native transcript IDs because the public row identifier is `local_session_row_id(path)`.

Add this field to `LocalSessionPreviewResult`:

```rust
pub candidate_set_truncated: bool,
```

Set it to `false` in the no-root early return and to the aggregated inventory state in the normal return. Add `candidate_set_truncated: bool` to `WireLocalSessionPreviewResult` so `deny_unknown_fields` continues to validate the complete response fixture.

Add this field to `LocalSessionPreviewRow` and `WireLocalSessionPreviewRow`:

```rust
pub content_included: bool,
```

When `include_content_items` is false, skip the `local_session_content_items` constructor and return `Vec::new()` while retaining summary title, excerpt, timing, and counts derived from bounded content. Set `content_included` to the resolved request boolean. When the request field is omitted, content items and `content_included=true` preserve the existing response behavior.

Update the request fixture to include:

```json
"sort": "modified_at",
"direction": "desc",
"include_content_items": false
```

Update the response fixture to include:

```json
"candidate_set_truncated": false,
"session_rows": [
  {
    "content_included": false,
    "content_items": []
  }
]
```

Keep all existing row fields in the fixture; replace only its former non-empty `content_items` value and add `content_included`.

- [ ] **Step 4: Add Swift summary/detail requests, decoding, and selected-row replacement**

Add the property and coding keys:

```swift
let candidateSetTruncated: Bool

case candidateSetTruncated = "candidate_set_truncated"
case candidateSetTruncatedAlt = "candidateSetTruncated"
```

Add an initializer parameter defaulting to `false`, assign it, and decode with:

```swift
candidateSetTruncated: try container.decodeIfPresent(
    Bool.self,
    forKey: .candidateSetTruncated
) ?? container.decodeIfPresent(
    Bool.self,
    forKey: .candidateSetTruncatedAlt
) ?? false,
```

In `mergingPage`, set the merged value to:

```swift
candidateSetTruncated || page.candidateSetTruncated
```

Propagate the current value through `ensuringSession` and other explicit `LocalSessionPreviewResult` reconstructions.

Extend `LocalSessionPreviewParams` and its coding keys:

```swift
let includeContentItems: Bool?
let sessionID: String?

case includeContentItems = "include_content_items"
case sessionID = "session_id"
```

Extend `ServiceClient.previewLocalSessions` with these arguments and pass them into params:

```swift
includeContentItems: Bool = false,
sessionID: String? = nil
```

The existing list call always encodes `includeContentItems: false`. Add a selected-row detail method that reuses the same RPC with `includeContentItems: true`, `sessionID` equal to `LocalSessionPreviewRow.id`, `limit: 1`, `offset: 0`, and no search text.

Add row-level decoding with legacy default `true`:

```swift
let contentIncluded: Bool

case contentIncluded = "content_included"
case contentIncludedAlt = "contentIncluded"

contentIncluded = try container.decodeIfPresent(Bool.self, forKey: .contentIncluded)
    ?? container.decodeIfPresent(Bool.self, forKey: .contentIncludedAlt)
    ?? true
```

Change the existing `ensuringSession` method so it keeps the summary collection but substitutes detail for one matching row:

```swift
func ensuringSession(_ session: LocalSessionPreviewRow) -> LocalSessionPreviewResult {
    var rows = sessionRows
    if let index = rows.firstIndex(where: { $0.id == session.id }) {
        rows[index] = session
    } else {
        rows.insert(session, at: 0)
    }
    return LocalSessionPreviewResult(
        generatedBy: generatedBy,
        authorized: true,
        authorizationRequired: authorizationRequired,
        roots: roots,
        sessionRows: rows,
        skillUsageRows: skillUsageRows,
        count: rows.count,
        totalCandidateCount: max(totalCandidateCount, rows.count),
        totalMatchedCount: max(totalMatchedCount, rows.count),
        offset: offset,
        limit: limit,
        hasMore: hasMore,
        nextOffset: nextOffset,
        candidateSetTruncated: candidateSetTruncated,
        gapNotes: gapNotes,
        blockerNotes: blockerNotes,
        redactionSummary: redactionSummary,
        safetyFlags: safetyFlags,
        fallbackReason: fallbackReason
    )
}
```

Do not put detail `contentItems` into `scopedLocalSessionSummaryCache`.

In `SkillStore.selectLocalSession`, preserve selection state and start `Task { await loadLocalSessionDetail(sessionID: session.id) }`. The async loader calls the detail RPC, checks that `selectedLocalSessionID` still equals the requested ID, then calls `localSessionPreviewResult = localSessionPreviewResult.ensuringSession(detailRow)`. A later summary refresh may discard detail and reload it only if the row remains selected.

- [ ] **Step 5: Document the exact semantics**

Document all of the following in `docs/service-protocol.md`:

- `sort` accepts `recent`, `modified_at`, and `title`.
- `direction` accepts `asc` and `desc`; recent defaults descending and title defaults ascending.
- `max_files` selects the newest metadata candidates before content reads.
- `total_candidate_count` is the number discovered within inventory limits.
- `total_matched_count`, `has_more`, and `next_offset` describe the selected candidate set.
- `candidate_set_truncated=true` means additional disk candidates were omitted by `max_files` or inventory limits.
- `include_content_items` defaults to `true` when omitted for old clients; list clients send `false` and rows return `content_included=false` with `content_items=[]`.
- `session_id` is the stable service row ID, not an agent-native transcript ID. The service filters it immediately after inventory, before `max_files` selection and before opening primary content.
- Detail requests send `include_content_items=true`, one `session_id`, `limit=1`, and `offset=0`; rows return `content_included=true`.
- Startup/manual cache stores summary rows only. At most the currently selected row is replaced with an in-memory detail row, and neither summary nor detail persists raw session content.

Document in the adapter and architecture files that scanner links may resolve only under explicit same-scope adapter roots, and that partial roots never participate in missing-sweep.

- [ ] **Step 6: Run protocol and native-model verification**

Run:

```sh
cargo test -p skills-copilot-service
pnpm verify:service-protocol-drift
pnpm test:macos-native-models
```

Expected: service tests pass, including summary/detail pre-read filtering and non-persistence; protocol verification reports equal documented, supported, dispatch, and fixture method sets; native model and store tests pass legacy decoding, summary requests, and selected-row detail replacement.

- [ ] **Step 7: Commit the protocol synchronization**

```sh
git add crates/service/src/lib.rs crates/service/src/service_local_session_io.rs crates/service/src/service_local_sessions.rs crates/service/src/tests/dispatch_fixtures.rs crates/service/src/tests/local_session_preview.rs fixtures/service-protocol/session.previewLocalSessions.request.json fixtures/service-protocol/session.previewLocalSessions.response.json apps/macos/Sources/SkillsCopilot/Services/ServiceClient.swift apps/macos/Sources/SkillsCopilot/Services/ServiceClientSessionRPC.swift apps/macos/Sources/SkillsCopilot/Models/LocalSessionPreview.swift apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift apps/macos/Tests/SkillsCopilotTests/LocalSessionPreviewModelTests.swift apps/macos/Tests/SkillsCopilotTests/SkillStoreTests.swift docs/service-protocol.md docs/adapters/agent-adapters.md docs/architecture.md
git commit -m "docs: define bounded scanner and session query semantics"
```

---

### Task 8: Run Full Verification and Review the Repair Boundaries

**Files:**
- Verify only: all files changed in Tasks 1-7

**Interfaces:**
- Consumes: both complete repair packages and their additive service field.
- Produces: repository-wide evidence that scanner, service, macOS models, privacy, and the 10k fixture remain healthy.

- [ ] **Step 1: Run format and focused lint checks**

Run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: both commands exit zero with no formatting diff and no warnings.

- [ ] **Step 2: Run the complete Rust test suite**

Run:

```sh
cargo test --workspace
```

Expected: every non-ignored workspace test passes; the 10k benchmark remains the only intentionally ignored benchmark test.

- [ ] **Step 3: Run protocol, native model, and benchmark checks**

Run:

```sh
pnpm verify:service-protocol-drift
pnpm test:macos-native-models
pnpm benchmark:10k
```

Expected: protocol sets and fixtures are aligned; native model tests pass; benchmark output contains `scanned=10000 records=10000` without budget exhaustion.

- [ ] **Step 4: Run the required macOS and privacy gates**

Run:

```sh
pnpm check:macos
pnpm check:privacy
```

Expected: both gates exit zero. Validation uses fixture data and does not modify real agent configuration.

- [ ] **Step 5: Review cache and persistence boundaries in the final diff**

Run:

```sh
rg -n "read_to_string|read_line|static .*LocalSession|OnceLock|LazyLock" crates/scanner/src crates/service/src/service_local_session_io.rs crates/service/src/service_local_sessions.rs
git diff --check
git status --short
```

Expected: scanner skill reads use the bounded content path; primary/sidecar/index session reads use `read_bounded_text`; no static local-session cache exists; `git diff --check` is clean; status lists only intended implementation and documentation files.

- [ ] **Step 6: Commit any verification-only corrections, if the gates required them**

If format or wire synchronization changed tracked files, commit only those corrections:

```sh
git add crates apps/macos docs fixtures
git commit -m "chore: finalize scanner and session hardening checks"
```

If no tracked correction was required, leave the Task 7 commit as the final implementation commit.

## Acceptance Checklist

- [ ] Every adapter can parse bounded owned content while retaining its existing path-based API.
- [ ] Scanner descendants resolve only beneath explicit same-scope adapter roots.
- [ ] One unavailable root or unreadable directory does not abort other roots or adapters.
- [ ] Per-file, total-byte, depth, directory, entry, and skill-count scanner budgets are enforced.
- [ ] Oversized known skill files become broken records without allocating their full contents.
- [ ] Only completely enumerated roots participate in catalog missing-sweep.
- [ ] Primary local-session reads retain bounded head and tail data without `read_line` allocation.
- [ ] Opencode messages and parts share per-session file and byte budgets.
- [ ] Codex title indexes are loaded at most once per store per request.
- [ ] No raw session or sidecar content is persisted or cached beyond the request.
- [ ] File inventory completes or explicitly reports truncation before `max_files` selection.
- [ ] `sort` and `direction` are validated and applied before `offset` and `limit`.
- [ ] Consecutive pages contain no duplicates and concatenate to the unpaged selected-set order.
- [ ] `candidate_set_truncated` distinguishes candidate omission from ordinary pagination completion.
- [ ] Summary requests explicitly exclude content items, and legacy requests that omit the field still include them.
- [ ] Every row reports whether content is included.
- [ ] Detail requests filter the stable row ID before reading content and open only the selected primary file.
- [ ] Startup/manual macOS cache holds summary rows; only the selected in-memory row may contain detail items.
- [ ] Rust fixtures, Swift decoding, service documentation, and protocol drift checks agree.
- [ ] Workspace tests, clippy, the 10k benchmark, macOS checks, and privacy checks all pass.

## Execution Handoff

Execute this plan with one of the required implementation workflows:

1. **Subagent-Driven (recommended):** use `superpowers:subagent-driven-development`, assign one fresh agent per task in its own worktree, and review each task before starting its dependent task.
2. **Inline Execution:** use `superpowers:executing-plans`, execute Tasks 1-8 in order, and stop at each task's verification checkpoint before continuing.
