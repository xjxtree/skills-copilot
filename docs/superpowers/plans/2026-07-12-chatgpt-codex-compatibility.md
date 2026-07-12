# ChatGPT Desktop / Codex Compatibility Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Agent Copilot fully compatible with Codex inside the new ChatGPT desktop app, including safe plugin-skill discovery, complete large-session listing, shared `CODEX_HOME`, plugin provenance, current app identity, and accurate documentation.

**Architecture:** Keep `codex` as the stable agent/wire identity. Move Codex path and plugin-package resolution into focused Rust adapter modules, carry optional read-only plugin provenance through core/catalog/service into Swift, and split session classification from summary materialization so large files do not make ordinary lists incomplete. The native shell resolves the current desktop client by bundle identifier and keeps all network-backed Plugin Directory behavior out of scope.

**Tech Stack:** Rust workspace, rusqlite migrations, serde JSON service protocol, SwiftUI/AppKit, Swift native model tests, pnpm validation scripts.

## Global Constraints

- Keep product logic in Rust workspace crates; SwiftUI remains a typed service client.
- Preserve `AgentId::Codex`, wire value `codex`, existing catalog identities, `.codex` config formats, and current guarded write boundaries.
- Plugin cache discovery is bounded and read-only: no scripts, hooks, apps, templates, credentials, OAuth, network calls, install, update, remove, or toggle operations.
- ChatGPT Chat and ChatGPT Work histories remain outside the Codex adapter.
- `npx skills` Skill Package Manager behavior and naming remain distinct from ChatGPT Plugins.
- Session summaries/details stay in memory and are never persisted.
- Every behavior change follows red-green-refactor and every task ends with a focused commit.
- Major UI/service work must finish with `pnpm check:macos`, `pnpm check:privacy`, and real local Computer Use validation.

---

### Task 1: Shared safe Codex home and custom-home sessions

**Files:**
- Create: `crates/adapters/src/codex/paths.rs`
- Modify: `crates/adapters/src/codex/mod.rs`
- Modify: `crates/service/src/service_local_sessions.rs`
- Modify: `crates/service/src/service_local_sessions/paging.rs`
- Test: `crates/adapters/src/codex/mod.rs`
- Test: `crates/service/src/tests/local_session_inventory.rs`
- Test: `crates/service/src/tests/local_session_preview.rs`

**Interfaces:**
- Produces: `pub fn codex_home_dir(ctx: &AdapterContext) -> PathBuf`.
- Produces: `CodexSessionContext { home: PathBuf }` passed to session summary/index lookup.
- Preserves: unsafe or escaping `CODEX_HOME` falls back to `$HOME/.codex`.

- [ ] **Step 1: Write failing adapter resolver tests**

Add tests that set an absolute home-contained override, an outside override, a
relative override, and an override containing `..`. Serialize environment
mutation with the existing test lock pattern and assert:

```rust
assert_eq!(codex_home_dir(&ctx), home.join("custom-codex"));
assert_eq!(codex_home_dir(&ctx), home.join(".codex"));
```

- [ ] **Step 2: Write a failing service test for custom-home sessions and indexes**

Create `<home>/custom-codex/sessions/.../rollout.jsonl` plus
`<home>/custom-codex/session_index.jsonl`, set `CODEX_HOME`, call
`session.previewLocalSessions` for agent `codex`, and assert the returned row
and indexed title are present even though no path component is named `.codex`.

- [ ] **Step 3: Run the focused tests and verify RED**

Run:

```sh
cargo test -p skills-copilot-adapters codex_home
cargo test -p skills-copilot-service custom_codex_home
```

Expected: FAIL because the resolver is private and session discovery is fixed
to `$HOME/.codex/sessions`.

- [ ] **Step 4: Implement the shared resolver**

Create `paths.rs` with lexical normalization that accepts only absolute paths
beneath `ctx.user_home`:

```rust
pub fn codex_home_dir(ctx: &AdapterContext) -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .filter(|path| safe_home_override(path, &ctx.user_home))
        .unwrap_or_else(|| ctx.user_home.join(".codex"))
}
```

Use it from adapter skill/config roots and service session discovery. Pass the
resolved home explicitly to index-title loading instead of calling
`local_session_agent_store_root(path, ".codex")`.

- [ ] **Step 5: Run focused tests and verify GREEN**

Run the two focused commands from Step 3 plus:

```sh
cargo test -p skills-copilot-service local_session_project_scope
```

Expected: PASS with no environment leakage between tests.

- [ ] **Step 6: Commit**

```sh
git add crates/adapters/src/codex crates/service/src/service_local_sessions.rs crates/service/src/service_local_sessions/paging.rs crates/service/src/tests
git commit -m "fix: share safe Codex home across local data"
```

### Task 2: Bounded ChatGPT plugin-package discovery

**Files:**
- Create: `crates/adapters/src/codex/plugins.rs`
- Modify: `crates/adapters/src/codex/mod.rs`
- Modify: `crates/core/src/adapter.rs`
- Modify: `crates/core/src/model.rs`
- Modify: `crates/scanner/src/lib.rs`
- Test: `crates/adapters/src/codex/mod.rs`
- Test: `crates/scanner/src/lib.rs`
- Create fixtures beneath: `fixtures/codex/plugin-cache/`

**Interfaces:**
- Produces: `SkillSourceProvenance` with optional package name/version/publisher/source kind/read-only reason.
- Extends: `AdapterRoot` and `SkillInstance` with optional provenance.
- Produces: `codex_plugin_cache_roots(ctx) -> Vec<AdapterRoot>`.

- [ ] **Step 1: Add failing plugin discovery fixtures and adapter tests**

Build fixtures for one valid package, two versions of the same package, a
manifest with `skills = "../../escape"`, malformed JSON, a staging package,
and a package with omitted `skills`. Assert only the valid highest version and
default `./skills/` package become roots, with provenance such as:

```rust
SkillSourceProvenance {
    package_name: Some("build-macos-apps".into()),
    package_version: Some("1.4.0".into()),
    publisher: Some("openai-curated-remote".into()),
    source_kind: Some("chatgpt-plugin-cache".into()),
    read_only_reason: Some("ChatGPT plugin cache skills are read-only.".into()),
}
```

- [ ] **Step 2: Add a failing scanner propagation test**

Scan the valid fixture and assert the resulting `SkillInstance` carries the
root provenance without reading or executing the fixture's `scripts/` entry.

- [ ] **Step 3: Run focused tests and verify RED**

```sh
cargo test -p skills-copilot-adapters plugin_cache
cargo test -p skills-copilot-scanner plugin_provenance
```

Expected: FAIL because plugin-cache roots and provenance fields do not exist.

- [ ] **Step 4: Implement core provenance and bounded root discovery**

Add:

```rust
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct SkillSourceProvenance {
    pub package_name: Option<String>,
    pub package_version: Option<String>,
    pub publisher: Option<String>,
    pub source_kind: Option<String>,
    pub read_only_reason: Option<String>,
}
```

Enumerate exactly `plugins/cache/<publisher>/<package>/<version>`, reject
hidden/staging components, cap publishers/packages/versions and manifest bytes,
validate `.codex-plugin/plugin.json`, canonicalize the declared skills path
beneath the package root, and choose one numeric-aware highest version per
publisher/package. Keep legacy marketplace roots and label them
`legacy-local-plugin`.

- [ ] **Step 5: Propagate root provenance through scanner normalization**

After bounded adapter parsing and before catalog upsert, assign:

```rust
instance.source_provenance = root.provenance.clone();
```

All non-plugin root constructors explicitly use `None`.

- [ ] **Step 6: Run focused tests and verify GREEN**

Run the two Step 3 commands and the existing cross-adapter root tests.
Expected: PASS; no staging/escaping roots appear.

- [ ] **Step 7: Commit**

```sh
git add crates/core crates/adapters crates/scanner fixtures/codex/plugin-cache
git commit -m "feat: discover ChatGPT plugin skills read only"
```

### Task 3: Persist and expose plugin provenance

**Files:**
- Create: `crates/catalog/src/migrations/0006_add_skill_provenance.sql`
- Modify: `crates/catalog/src/schema.rs`
- Modify: `crates/catalog/src/lib.rs`
- Modify: `crates/catalog/src/mapping.rs`
- Modify: `crates/catalog/src/queries.rs`
- Modify: `crates/service/src/tests/dispatch_fixtures.rs`
- Modify: `fixtures/service-protocol/`
- Test: `crates/catalog/src/lib.rs`
- Test: `crates/service/src/tests/dispatch_fixtures.rs`

**Interfaces:**
- Extends `SkillRecord` and `SkillDetailRecord` with five optional snake-case wire fields.
- Older databases and fixtures decode with all provenance fields absent/null.

- [ ] **Step 1: Write failing catalog migration and round-trip tests**

Initialize an old schema, run `Catalog::init`, upsert a plugin skill, reopen the
catalog, and assert summary/detail/instance queries retain all five fields.
Also assert a native skill returns `None` for every field.

- [ ] **Step 2: Write failing service fixture compatibility tests**

Add one plugin skill record containing:

```json
{
  "package_name": "build-macos-apps",
  "package_version": "1.4.0",
  "publisher": "openai-curated-remote",
  "source_kind": "chatgpt-plugin-cache",
  "read_only_reason": "ChatGPT plugin cache skills are read-only."
}
```

Keep an older fixture without the fields and require both shapes to decode.

- [ ] **Step 3: Run tests and verify RED**

```sh
cargo test -p skills-copilot-catalog provenance
cargo test -p skills-copilot-service dispatch_fixtures
```

Expected: FAIL because schema/query/wire fields are absent.

- [ ] **Step 4: Add migration and query plumbing**

Add nullable TEXT columns, guarded by `apply_column_migration_if_missing`, and
extend every insert/update/select/row mapper. Keep field order identical across
SQL and Rust row decoders. Increment `migration_count()`.

- [ ] **Step 5: Run tests and verify GREEN**

Run Step 3 plus `cargo test -p skills-copilot-catalog`.
Expected: PASS for fresh and migrated catalogs.

- [ ] **Step 6: Commit**

```sh
git add crates/catalog crates/service/src/tests fixtures/service-protocol
git commit -m "feat: expose plugin skill provenance"
```

### Task 4: Complete large Codex session lists without unbounded reads

**Files:**
- Modify: `crates/service/src/service_local_session_io.rs`
- Modify: `crates/service/src/service_local_sessions.rs`
- Modify: `crates/service/src/service_local_sessions/paging.rs`
- Test: `crates/service/src/tests/local_session_inventory.rs`
- Test: `crates/service/src/tests/local_session_summary_detail.rs`
- Test: `crates/service/src/tests/local_session_project_scope.rs`

**Interfaces:**
- Produces: `LocalSessionCandidateMetadata` from a small bounded head probe.
- Separates: `content_truncated` from `request_budget_exhausted`.
- Preserves: content search remains explicitly limited if its aggregate read budget is exhausted.

- [ ] **Step 1: Add failing single-large-file completeness test**

Create a session larger than 512 KiB whose first line contains Codex
`session_meta`. Request an unsearched list and assert
`source_completeness="enumerable"`, `candidate_set_truncated=false`, and the row
is returned with a bounded excerpt.

- [ ] **Step 2: Add failing aggregate-large-project test**

Create more than 140 sessions whose combined bodies exceed 64 MiB, with only
several matching the selected project. Request project/recent pages and assert
the exact matching total and continuation are available without a
`safety_budget` terminal result.

- [ ] **Step 3: Run focused tests and verify RED**

```sh
cargo test -p skills-copilot-service large_codex_session
cargo test -p skills-copilot-service project_scope_large
```

Expected: FAIL because per-file truncation is treated as candidate truncation
and legacy project listing materializes summaries before pagination.

- [ ] **Step 4: Separate truncation causes**

Change primary reads to record both:

```rust
struct LocalSessionPrimaryRead {
    content: String,
    modified_at: Option<i64>,
    content_truncated: bool,
    request_budget_exhausted: bool,
}
```

Only `request_budget_exhausted` can limit the candidate set. A deliberately
bounded head/tail summary is normal and does not make the list incomplete.

- [ ] **Step 5: Implement metadata-first project classification**

Probe only the small head needed for `session_meta`, retain guarded candidate
metadata, filter by project, sort, and select the requested page before calling
`local_session_preview_row`. For content search, retain the existing bounded
full-summary scan and its typed limitation.

- [ ] **Step 6: Run focused and paging tests and verify GREEN**

```sh
cargo test -p skills-copilot-service large_codex_session
cargo test -p skills-copilot-service local_session_project_scope
cargo test -p skills-copilot-service local_session_inventory
```

Expected: PASS with exact ordinary list metadata and bounded detail behavior.

- [ ] **Step 7: Commit**

```sh
git add crates/service/src/service_local_session_io.rs crates/service/src/service_local_sessions crates/service/src/tests
git commit -m "fix: classify Codex sessions before reading summaries"
```

### Task 5: Native ChatGPT/Codex identity and plugin presentation

**Files:**
- Modify: `apps/macos/Sources/SkillsCopilot/Support/AgentIconProvider.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Models/SkillRecord.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/DetailOverviewSection.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Views/DetailHeaderOverviewSection.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Support/UIStrings.swift`
- Modify: `apps/macos/Sources/SkillsCopilot/Resources/en.lproj/Localizable.strings`
- Modify: `apps/macos/Sources/SkillsCopilot/Resources/zh-Hans.lproj/Localizable.strings`
- Test: `apps/macos/Tests/SkillsCopilotTests/SkillListModelTests.swift`
- Test: `apps/macos/Tests/SkillsCopilotTests/LocalizationModelTests.swift`
- Test: `apps/macos/Tests/SkillsCopilotTests/FullNativeModelSuiteTests.swift`

**Interfaces:**
- Swift `SkillRecord`/`SkillDetailRecord` decode the five optional provenance fields.
- `AgentIconProvider` resolves bundle ID `com.openai.codex` before fixed paths.

- [ ] **Step 1: Add failing native model tests**

Assert plugin JSON decodes and yields a read-only ChatGPT plugin provenance
label, old JSON still decodes, and desktop restart copy mentions both ChatGPT
Codex and the CLI.

- [ ] **Step 2: Add a failing pure icon candidate-order test**

Extract candidate construction behind an injectable resolved-app URL and
assert this order: resolved Codex resource, resolved app bundle,
`/Applications/ChatGPT.app`, legacy `/Applications/Codex.app`, CLI.

- [ ] **Step 3: Run native models and verify RED**

```sh
pnpm test:macos-native-models
```

Expected: FAIL on missing fields, label, copy, and ChatGPT icon candidates.

- [ ] **Step 4: Implement optional decoding and detail presentation**

Add optional `packageName`, `packageVersion`, `publisher`, `sourceKind`, and
`readOnlyReason`, render a Plugin source row plus lock/read-only explanation,
and continue deriving legacy provenance when fields are absent.

- [ ] **Step 5: Implement bundle-ID icon resolution**

Use:

```swift
NSWorkspace.shared.urlForApplication(withBundleIdentifier: "com.openai.codex")
```

Probe `icon-codex-dark-color.png`, `icon-codex-light.png`, `app.icns`, and the
resolved bundle icon before compatibility fallbacks.

- [ ] **Step 6: Run native tests and verify GREEN**

Run `pnpm test:macos-native-models`.
Expected: PASS with both current and legacy wire fixtures.

- [ ] **Step 7: Commit**

```sh
git add apps/macos/Sources apps/macos/Tests
git commit -m "feat: present Codex inside the ChatGPT desktop app"
```

### Task 6: Protocol, adapter, and user documentation

**Files:**
- Modify: `docs/service-protocol.md`
- Modify: `docs/adapters/codex-adapter-spec.md`
- Modify: `docs/adapters/agent-adapters.md`
- Modify: `docs/ai-agent-workflow.md`
- Modify: `README.md`
- Modify: `README.zh-CN.md`

**Interfaces:**
- Documents optional wire provenance and exact read/write boundaries.
- Documents ChatGPT Plugins versus `npx skills` packages.

- [ ] **Step 1: Update durable contracts**

Describe shared safe `CODEX_HOME`, bounded plugin-cache discovery, optional
provenance fields, staging/version exclusion, and metadata-first sessions.

- [ ] **Step 2: Update user-facing overview**

Explain that Codex is hosted in the ChatGPT desktop app, existing `.codex`
local data remains supported, and the Skill Package Manager does not manage
ChatGPT Plugins or app connections.

- [ ] **Step 3: Run documentation/protocol guards**

```sh
pnpm verify:service-protocol-drift
pnpm verify:doc-governance
pnpm check:privacy
```

Expected: all exit 0 with no stale fixture or private-path finding.

- [ ] **Step 4: Commit**

```sh
git add README.md README.zh-CN.md docs fixtures/service-protocol
git commit -m "docs: describe ChatGPT hosted Codex support"
```

### Task 7: Full verification and real-local acceptance

**Files:**
- Modify only if a verifier identifies a scoped regression.

**Interfaces:**
- Produces fresh full-gate and real-window evidence.

- [ ] **Step 1: Run Rust formatting and focused workspace checks**

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: all exit 0.

- [ ] **Step 2: Run the required macOS gate**

```sh
pnpm check:macos
```

Expected: format, Rust, native models, bundle, launch, fixture smoke, protocol,
list-completeness, and screenshot checks all pass.

- [ ] **Step 3: Run final privacy verification**

```sh
pnpm check:privacy
```

Expected: `privacy check: ok`.

- [ ] **Step 4: Perform real-local Computer Use acceptance**

Launch the latest bundle, select Codex, and verify:

- the toolbar uses the current ChatGPT/Codex icon;
- native and plugin-cache skills are present with read-only plugin metadata;
- the current project's Codex session list no longer reports false safety-budget incompleteness;
- user and project configuration remain readable;
- no plugin write or network action is exposed.

Capture only the full AgentCopilot app window if evidence is retained.

- [ ] **Step 5: Audit the plan line by line**

Compare every acceptance criterion in
`docs/superpowers/specs/2026-07-12-chatgpt-codex-compatibility-design.md` against
tests, code, docs, and real-local evidence. Record any genuine external blocker
instead of weakening a criterion.

- [ ] **Step 6: Commit verifier-only fixes if needed**

```sh
git status --short
git add <only-scoped-files>
git commit -m "test: close ChatGPT Codex compatibility gates"
```

Do not create an empty commit when no verifier-only fix is required.

