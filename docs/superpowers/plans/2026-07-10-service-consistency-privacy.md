# Service Consistency and Privacy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make service side effects machine-verifiable, keep read paths physically read-only, reject stale config saves with revisions and stale rollbacks with preview-bound tokens, scan all current and reachable Git blobs for privacy leaks, and retire the deprecated YAML dependency without leaving the excluded fuzz workspace stale.

**Architecture:** A checked-in method-effects manifest becomes the side-effect source of truth and is cross-checked against Rust dispatch, fixtures, status output, and the human protocol table. Read requests use a read-only SQLite connection or an initialized in-memory empty catalog. Config saves compare tagged SHA-256 revisions under the existing file lock, while rollback applies only a preview token bound to snapshot identity, target, snapshot-content hash, and the current revision. Privacy validation enumerates index, worktree, untracked, and reachable historical blobs by object ID. YAML migration is contract-first and regenerates both workspace lockfiles only after an implementation-time upstream verification gate.

**Tech Stack:** Rust 2021, rusqlite, serde/serde_json, sha2, Node.js 22 ESM, Bash, Git plumbing commands, pnpm 11.5.1, GitHub Actions.

## Global Constraints

- Work only in an isolated `codex/` worktree; do not switch branches or edit the coordinator checkout.
- Keep `crates/core` free of I/O and keep all product behavior behind the typed Rust service protocol.
- A method declared with no local writes must not create directories, SQLite files, lock files, audit records, temporary files, or config parents.
- Script execution remains default-denied; `script.execute` may record a redacted blocked-attempt audit record but must not execute the script.
- Config reads and previews must validate paths without creating them; config writes retain path guards, file locks, atomic rename, private permissions, snapshot, readback verification, and rollback behavior.
- Config revisions must distinguish a missing file from an existing empty file and must never expose config content.
- Rollback requests must carry only a preview-bound token, not a naked expected revision. A stale or forged token returns `stale_preview_token` before snapshot or filesystem mutation.
- Privacy tests must construct sensitive-looking samples dynamically so the regression test source and this plan remain safe to commit.
- `--no-history` skips only reachable-history scanning; it must still inspect the index, tracked worktree files, and untracked non-ignored files.
- OCR remains a manual screenshot review requirement and is not represented as automated coverage.
- Treat `serde_norway` only as a candidate. During implementation, re-check its official registry/repository metadata and the current RustSec recommendation before selecting a version; do not copy a version from this plan.
- Do not add network access outside the explicit dependency-metadata verification and normal dependency resolution steps.
- Every task ends with its focused RED/GREEN cycle before the next task begins.

---

## File Structure

### New files

- `fixtures/service-protocol/method-effects.json` — exhaustive machine-readable side-effect contract for every supported method.
- `scripts/tests/service-protocol-effects.test.mjs` — negative tests for missing, duplicate, malformed, or documentation-divergent effect entries.
- `crates/service/src/tests/method_effects.rs` — table-driven filesystem and process/network behavior checks.
- `crates/commands/src/config_consistency.rs` — tagged revision calculation, locked current-state reads, and conflict comparison.
- `scripts/test-privacy-check.mjs` — disposable-repository regression suite for index, worktree, untracked, and historical blobs.

### Existing files modified

- `scripts/verify-service-protocol-drift.mjs` — load and validate all manifest fields and all protocol table columns.
- `docs/service-protocol.md` — render truthful local-write, process, network, and confirmation columns.
- `crates/service/src/service_host.rs` — route read methods through `open_catalog_for_read` and propagate CAS parameters.
- `crates/service/src/lib.rs` — request/response fields, conflict error code, and test module wiring.
- `crates/service/src/protocol.rs` — bump the schema version after required CAS fields land.
- `crates/service/src/tests.rs` and protocol fixture tests — register and exercise the new contracts.
- `crates/commands/src/lib.rs` — use non-creating validation and the new config consistency module.
- `crates/commands/src/config_support.rs` — separate read/preview validation from write preparation.
- `crates/commands/src/skill_manager.rs` — avoid manager cwd creation for previews and prohibited searches.
- `fixtures/service-protocol/*.json` — update CAS payloads/results and protocol version fixtures.
- `script/check_privacy.sh` — scan current and reachable blob content by object ID.
- `package.json`, `.github/workflows/ci.yml`, and `docs/security-model.md` — expose, run, and document the privacy regression gate.
- Root and crate Cargo manifests, YAML call sites, `Cargo.lock`, and `crates/adapters/fuzz/Cargo.lock` — complete the dependency migration and fuzz lock refresh.

---

### Task 1: Establish the exhaustive method-effects contract

**Files:**
- Create: `fixtures/service-protocol/method-effects.json`
- Create: `scripts/tests/service-protocol-effects.test.mjs`
- Modify: `scripts/verify-service-protocol-drift.mjs`
- Modify: `docs/service-protocol.md`
- Modify: `fixtures/service-protocol/README.md`
- Modify: `package.json`

**Interfaces:**
- Produces: `parseMethodEffects(raw: string): Map<string, MethodEffect>` exported from `scripts/verify-service-protocol-drift.mjs`.
- Produces: `validateMethodEffects({ documentedRows, effects, supportedMethods }): string[]`.
- `MethodEffect` shape: `{ writes: string[], process: "never" | "conditional" | "always", network: "never" | "conditional" | "always", confirmation: "none" | "required" }`.
- Manifest write values are restricted to `app_data`, `audit`, `keychain`, `agent_config`, `agent_files`, `export`, and `external_manager_state`.
- Consumes: the existing `SUPPORTED_METHODS`, dispatch arms, request/response fixtures, and `service.status` fixture discovered by the verifier.

- [ ] **Step 1: Write failing manifest parser and drift tests**

Create `scripts/tests/service-protocol-effects.test.mjs` with pure tests that import the future exports:

```js
import assert from "node:assert/strict";
import test from "node:test";
import {
  parseMethodEffects,
  validateMethodEffects,
} from "../verify-service-protocol-drift.mjs";

const readOnly = {
  writes: [],
  process: "never",
  network: "never",
  confirmation: "none",
};

test("rejects a supported method missing from the effects manifest", () => {
  const errors = validateMethodEffects({
    documentedRows: new Map([["app.version", readOnly]]),
    effects: new Map(),
    supportedMethods: ["app.version"],
  });
  assert.deepEqual(errors, ["supported methods missing effect entries: app.version"]);
});

test("rejects effect entries that are absent from SUPPORTED_METHODS", () => {
  const errors = validateMethodEffects({
    documentedRows: new Map(),
    effects: new Map([["legacy.method", readOnly]]),
    supportedMethods: [],
  });
  assert.deepEqual(errors, ["effect entries missing from SUPPORTED_METHODS: legacy.method"]);
});

test("rejects a documentation row whose side effects differ", () => {
  const effects = new Map([["script.execute", { ...readOnly, writes: ["audit"], confirmation: "required" }]]);
  const errors = validateMethodEffects({
    documentedRows: new Map([["script.execute", readOnly]]),
    effects,
    supportedMethods: ["script.execute"],
  });
  assert.match(errors.join("\n"), /script\.execute.*writes.*confirmation/);
});

test("rejects unknown enum values", () => {
  assert.throws(
    () => parseMethodEffects(JSON.stringify({
      schema_version: 1,
      methods: {
        "app.version": { ...readOnly, process: "sometimes" },
      },
    })),
    /invalid process value for app\.version/,
  );
});
```

- [ ] **Step 2: Run the tests and verify RED**

Run:

```sh
node --test scripts/tests/service-protocol-effects.test.mjs
```

Expected: FAIL because `parseMethodEffects` and `validateMethodEffects` are not exported.

- [ ] **Step 3: Add the complete manifest**

Create `fixtures/service-protocol/method-effects.json` with `schema_version: 1` and exactly these method keys and values:

```json
{
  "schema_version": 1,
  "methods": {
    "app.version": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "app.stateSnapshot": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "app.search": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "service.status": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "adapter.listCapabilities": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "adapter.listDiagnostics": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "session.previewLocalSessions": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "llm.status": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "llm.listProviderProfiles": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "llm.saveProviderProfile": { "writes": ["app_data", "keychain"], "process": "never", "network": "never", "confirmation": "none" },
    "llm.deleteProviderProfile": { "writes": ["app_data", "keychain"], "process": "never", "network": "never", "confirmation": "none" },
    "llm.testProviderConnection": { "writes": ["app_data"], "process": "never", "network": "always", "confirmation": "required" },
    "llm.previewPrompt": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "llm.confirmPromptAndSend": { "writes": ["app_data"], "process": "never", "network": "always", "confirmation": "required" },
    "llm.listPromptRuns": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "llm.providerObservability": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "llm.listModelTaskMatches": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "llm.recordModelTaskMatch": { "writes": ["app_data"], "process": "never", "network": "never", "confirmation": "none" },
    "llm.deleteModelTaskMatch": { "writes": ["app_data"], "process": "never", "network": "never", "confirmation": "none" },
    "llm.prepareAction": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "rules.listTuning": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "rules.setSeverityOverride": { "writes": ["app_data"], "process": "never", "network": "never", "confirmation": "none" },
    "rules.clearSeverityOverride": { "writes": ["app_data"], "process": "never", "network": "never", "confirmation": "none" },
    "rules.setSuppression": { "writes": ["app_data"], "process": "never", "network": "never", "confirmation": "none" },
    "rules.clearSuppression": { "writes": ["app_data"], "process": "never", "network": "never", "confirmation": "none" },
    "batch.previewSkillToggles": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "batch.applySkillToggles": { "writes": ["app_data", "agent_config"], "process": "never", "network": "never", "confirmation": "required" },
    "script.previewExecution": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "script.execute": { "writes": ["audit"], "process": "never", "network": "never", "confirmation": "required" },
    "skillManager.listTools": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "skillManager.search": { "writes": ["external_manager_state"], "process": "conditional", "network": "conditional", "confirmation": "none" },
    "skillManager.listInstalled": { "writes": ["external_manager_state"], "process": "always", "network": "never", "confirmation": "none" },
    "skillManager.previewInstall": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "skillManager.applyInstall": { "writes": ["app_data", "external_manager_state"], "process": "always", "network": "conditional", "confirmation": "required" },
    "skillManager.previewRemove": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "skillManager.applyRemove": { "writes": ["app_data", "external_manager_state"], "process": "always", "network": "never", "confirmation": "required" },
    "skillManager.previewUpdate": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "skillManager.applyUpdate": { "writes": ["app_data", "external_manager_state"], "process": "always", "network": "conditional", "confirmation": "required" },
    "skillManager.previewLocalCreate": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "skillManager.applyLocalCreate": { "writes": ["app_data", "external_manager_state"], "process": "always", "network": "never", "confirmation": "required" },
    "skillManager.deleteLocal": { "writes": ["app_data"], "process": "never", "network": "never", "confirmation": "required" },
    "project.getContext": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "project.setContext": { "writes": ["app_data"], "process": "never", "network": "never", "confirmation": "none" },
    "project.clearContext": { "writes": ["app_data"], "process": "never", "network": "never", "confirmation": "none" },
    "project.validateContext": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "catalog.listSkills": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "catalog.getSkill": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "catalog.analysis": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "catalog.listFindings": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "catalog.listFindingTriage": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "catalog.setFindingTriage": { "writes": ["app_data"], "process": "never", "network": "never", "confirmation": "none" },
    "catalog.clearFindingTriage": { "writes": ["app_data"], "process": "never", "network": "never", "confirmation": "none" },
    "catalog.listConflicts": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "catalog.importSkill": { "writes": ["app_data"], "process": "never", "network": "never", "confirmation": "none" },
    "catalog.scanClaude": { "writes": ["app_data"], "process": "never", "network": "never", "confirmation": "none" },
    "catalog.scanAll": { "writes": ["app_data"], "process": "never", "network": "never", "confirmation": "none" },
    "skill.exportBundle": { "writes": ["export"], "process": "never", "network": "never", "confirmation": "none" },
    "skill.install": { "writes": ["app_data", "agent_files"], "process": "never", "network": "never", "confirmation": "required" },
    "skill.listEvents": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "config.toggleSkill": { "writes": ["app_data", "agent_config"], "process": "never", "network": "never", "confirmation": "none" },
    "config.readAgentConfig": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "config.readClaudeSettings": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "config.saveClaudeSettings": { "writes": ["app_data", "agent_config"], "process": "never", "network": "never", "confirmation": "none" },
    "snapshot.list": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "snapshot.listAgentConfig": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "snapshot.previewRollback": { "writes": [], "process": "never", "network": "never", "confirmation": "none" },
    "snapshot.rollback": { "writes": ["app_data", "agent_config"], "process": "never", "network": "never", "confirmation": "required" }
  }
}
```

- [ ] **Step 4: Implement strict parsing and table-column comparison**

Refactor `scripts/verify-service-protocol-drift.mjs` so it exports the two tested functions, loads `method-effects.json`, parses the Markdown columns `Method`, `Local writes`, `External process`, `Network`, and `Confirmation`, and appends validation errors to the existing drift report. Keep the executable guard explicit:

```js
const processValues = new Set(["never", "conditional", "always"]);
const networkValues = new Set(["never", "conditional", "always"]);
const confirmationValues = new Set(["none", "required"]);
const writeValues = new Set([
  "app_data",
  "audit",
  "keychain",
  "agent_config",
  "agent_files",
  "export",
  "external_manager_state",
]);

export function parseMethodEffects(raw) {
  const parsed = JSON.parse(raw);
  if (parsed.schema_version !== 1 || typeof parsed.methods !== "object" || parsed.methods === null) {
    throw new Error("method effects manifest must use schema_version 1 and an object-valued methods field");
  }
  return new Map(Object.entries(parsed.methods).map(([method, effect]) => {
    if (!Array.isArray(effect.writes) || effect.writes.some((value) => !writeValues.has(value))) {
      throw new Error(`invalid writes value for ${method}`);
    }
    if (!processValues.has(effect.process)) throw new Error(`invalid process value for ${method}`);
    if (!networkValues.has(effect.network)) throw new Error(`invalid network value for ${method}`);
    if (!confirmationValues.has(effect.confirmation)) throw new Error(`invalid confirmation value for ${method}`);
    return [method, { ...effect, writes: [...new Set(effect.writes)].sort() }];
  }));
}

export function validateMethodEffects({ documentedRows, effects, supportedMethods }) {
  const errors = [];
  const supported = new Set(supportedMethods);
  const missing = supportedMethods.filter((method) => !effects.has(method));
  const extra = [...effects.keys()].filter((method) => !supported.has(method));
  if (missing.length) errors.push(`supported methods missing effect entries: ${missing.sort().join(", ")}`);
  if (extra.length) errors.push(`effect entries missing from SUPPORTED_METHODS: ${extra.sort().join(", ")}`);
  for (const method of supportedMethods) {
    const expected = effects.get(method);
    const documented = documentedRows.get(method);
    if (expected && documented && JSON.stringify(expected) !== JSON.stringify(documented)) {
      errors.push(`${method} documentation differs in writes/process/network/confirmation`);
    }
  }
  return errors;
}
```

- [ ] **Step 5: Replace the protocol table with manifest-equivalent columns**

Add a deterministic renderer to the verifier so the 67-row table is derived from the complete manifest rather than copied a second time:

```js
const writeLabels = new Map([
  ["app_data", "App-local data"],
  ["audit", "Blocked-attempt audit"],
  ["keychain", "Keychain"],
  ["agent_config", "Agent config"],
  ["agent_files", "Agent skill files"],
  ["export", "Export destination"],
  ["external_manager_state", "External manager state"],
]);

function titleCase(value) {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

export function renderMethodEffectsTable(effects) {
  const rows = [
    "| Method | Local writes | External process | Network | Confirmation |",
    "| --- | --- | --- | --- | --- |",
  ];
  for (const [method, effect] of effects) {
    const writes = effect.writes.length === 0
      ? "None"
      : effect.writes.map((value) => writeLabels.get(value)).join(", ");
    rows.push(
      `| \`${method}\` | ${writes} | ${titleCase(effect.process)} | ${titleCase(effect.network)} | ${titleCase(effect.confirmation)} |`,
    );
  }
  return rows.join("\n");
}
```

Support `node scripts/verify-service-protocol-drift.mjs --write-doc-table` to replace only the content between the `## Methods` heading and the next level-two heading. Run it once, then review the generated table. The resulting heading and representative rows are:

```markdown
| Method | Local writes | External process | Network | Confirmation |
| --- | --- | --- | --- | --- |
| `app.version` | None | Never | Never | None |
| `script.execute` | Blocked-attempt audit only | Never | Never | Required |
| `skillManager.search` | External manager state may change when invoked | Conditional | Conditional | None |
```

The normal verifier mode recomputes this exact table and fails if the checked-in Markdown differs; it never writes files.

Add one paragraph to `fixtures/service-protocol/README.md` stating that `method-effects.json` is exhaustive for `SUPPORTED_METHODS`, while request/response JSON files remain the wire-shape fixtures.

- [ ] **Step 6: Wire and run GREEN checks**

Add this package script:

```json
"test:service-protocol-effects": "node --test scripts/tests/service-protocol-effects.test.mjs"
```

Run:

```sh
node scripts/verify-service-protocol-drift.mjs --write-doc-table
pnpm test:service-protocol-effects
pnpm verify:service-protocol-drift
```

Expected: both commands exit 0; the drift verifier reports 67 documented, supported, dispatched, status, and effect-manifest methods.

- [ ] **Step 7: Commit the contract**

```sh
git add fixtures/service-protocol/method-effects.json fixtures/service-protocol/README.md scripts/verify-service-protocol-drift.mjs scripts/tests/service-protocol-effects.test.mjs docs/service-protocol.md package.json
git commit -m "test: enforce service method effect contract"
```

---

### Task 2: Make catalog-backed reads and config previews physically read-only

**Files:**
- Create: `crates/service/src/tests/method_effects.rs`
- Modify: `crates/service/src/tests.rs`
- Modify: `crates/service/src/service_host.rs`
- Modify: `crates/commands/src/config_support.rs`
- Modify: `crates/commands/src/lib.rs`
- Modify: `crates/commands/src/skill_manager.rs`

**Interfaces:**
- Produces: `ServiceHost::open_catalog_for_read(&self) -> Result<Catalog, ServiceError>`.
- Promotes and strengthens the existing `config_support::validate_config_read_target(ctx, agent, scope, path) -> Result<(), CommandError>` so `lib.rs` can reuse it without filesystem creation.
- Retains: `ServiceHost::open_catalog(&self)` as the only initializing/migrating catalog opener.
- Retains: `validate_config_write_target` as the creating/canonicalizing write guard.
- Consumes: `Catalog::open_read_only`, `Catalog::in_memory`, and `Catalog::init`.

- [ ] **Step 1: Add filesystem snapshot helpers and failing read-only tests**

Register `mod method_effects;` in `crates/service/src/tests.rs`. In the new module, add a deterministic tree snapshot and the first RED tests:

```rust
use super::*;
use std::collections::BTreeMap;

fn tree_snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, dir: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        let mut entries = entries.flatten().collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.path());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, files);
            } else if path.is_file() {
                files.insert(path.strip_prefix(root).unwrap().to_path_buf(), fs::read(path).unwrap());
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}

#[test]
fn catalog_list_skills_does_not_create_app_data_or_catalog() {
    let root = temp_test_dir("effects-list-skills");
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();
    let host = ServiceHost {
        app_data_dir: root.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        },
    };
    let before = tree_snapshot(&root);
    let response = host.handle(ServiceRequest {
        id: Some("effects-list".to_string()),
        method: "catalog.listSkills".to_string(),
        params: json!({}),
    });
    assert!(response.ok, "{response:?}");
    assert_eq!(tree_snapshot(&root), before);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn read_claude_settings_does_not_create_claude_directory() {
    let root = temp_test_dir("effects-read-settings");
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();
    let host = ServiceHost {
        app_data_dir: root.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home: home.clone(),
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        },
    };
    let response = host.handle(ServiceRequest {
        id: Some("effects-read-settings".to_string()),
        method: "config.readClaudeSettings".to_string(),
        params: json!({}),
    });
    assert!(response.ok, "{response:?}");
    assert!(!home.join(".claude").exists());
    assert!(!host.app_data_dir.exists());
    let _ = fs::remove_dir_all(root);
}
```

- [ ] **Step 2: Run focused tests and verify RED**

```sh
cargo test -p skills-copilot-service tests::method_effects::catalog_list_skills_does_not_create_app_data_or_catalog -- --exact
cargo test -p skills-copilot-service tests::method_effects::read_claude_settings_does_not_create_claude_directory -- --exact
```

Expected: both tests fail because the current paths create app-data/catalog or `.claude`.

- [ ] **Step 3: Add a non-filesystem empty catalog and read-only existing catalog path**

Implement this method in `crates/service/src/service_host.rs`:

```rust
pub(crate) fn open_catalog_for_read(&self) -> Result<Catalog, ServiceError> {
    let path = self.catalog_path();
    if path.exists() {
        return Catalog::open_read_only(&path).map_err(Into::into);
    }
    let catalog = Catalog::in_memory()?;
    catalog.init()?;
    Ok(catalog)
}
```

Replace `open_catalog()` with `open_catalog_for_read()` only in these handlers and helpers:

```text
app.stateSnapshot
rules.listTuning
batch.previewSkillToggles
catalog.listSkills
catalog.getSkill
catalog.analysis
catalog.listFindings
catalog.listFindingTriage
catalog.listConflicts
skill.listEvents
snapshot.list
snapshot.listAgentConfig
snapshot.previewRollback
```

Keep every set/clear/apply/scan/import/install/save/rollback handler on `open_catalog()`.

- [ ] **Step 4: Reuse and strengthen read validation instead of write preparation**

`config_support.rs` already has a private non-creating validator used by `read_agent_config`. Make it `pub(super)`, retain `read_config_target` so Codex project reads keep their special target, normalize the comparison, and reject symlinks at every existing or missing path component up to the allowed root without canonicalizing or creating anything:

```rust
pub(super) fn validate_config_read_target(
    ctx: &AdapterContext,
    agent: AgentId,
    scope: Scope,
    path: &Path,
) -> Result<(), CommandError> {
    let expected = read_config_target(ctx, agent, scope)?;
    if normalize_path_lexically(path) != normalize_path_lexically(&expected.path) {
        return Err(CommandError::UnsafeConfigPath(format!(
            "{} does not match expected {} config path {}",
            path.display(),
            agent.as_str(),
            expected.path.display()
        )));
    }

    let allowed_root = match scope {
        Scope::AgentGlobal => &ctx.user_home,
        Scope::AgentProject
            if matches!(
                agent,
                AgentId::ClaudeCode | AgentId::Codex | AgentId::Opencode | AgentId::Pi
            ) => ctx.project_root.as_ref().ok_or(CommandError::UnsupportedScope(scope))?,
        _ => return Err(CommandError::UnsupportedScope(scope)),
    };
    let parent = path.parent().ok_or_else(|| {
        CommandError::UnsafeConfigPath("config path has no parent".to_string())
    })?;
    if !normalize_path_lexically(parent).starts_with(&normalize_path_lexically(allowed_root)) {
        return Err(CommandError::UnsafeConfigPath(format!(
            "config directory {} is outside allowed root {}",
            parent.display(),
            allowed_root.display()
        )));
    }

    let mut cursor = Some(path);
    while let Some(candidate) = cursor {
        let label = if candidate == path { "config file" } else { "config path ancestor" };
        reject_symlink(candidate, label)?;
        if normalize_path_lexically(candidate) == normalize_path_lexically(allowed_root) {
            break;
        }
        cursor = candidate.parent();
    }
    Ok(())
}
```

Import it into the parent module and use it from `read_claude_settings` and `preview_snapshot_rollback_for_record`; `read_agent_config` remains on the same helper. Leave `save_claude_settings`, toggle, batch apply, install, and rollback on `validate_config_write_target`.

- [ ] **Step 5: Prevent manager previews and prohibited searches from creating cwd**

Change `run_previewed_command` so directory creation occurs only immediately before an allowed process invocation. Ensure `search_skills_with_manager` returns before calling it when `network_allowed` is false. Add these tests beside the existing skill-manager tests:

```rust
#[test]
fn search_without_network_does_not_create_manager_cwd() {
    let root = std::env::temp_dir().join(format!(
        "skill-manager-search-no-network-{}",
        std::process::id()
    ));
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };
    let record = search_skills_with_manager(
        &ctx,
        &SkillManagerSearchParams {
            query: "local-only".to_string(),
            owner: None,
            network_allowed: false,
        },
    ).unwrap();
    assert!(record.output.is_none());
    assert!(!ctx.user_home.exists());
    let _ = fs::remove_dir_all(root);
}
```

- [ ] **Step 6: Expand the manifest-driven read-only sweep**

In `method_effects.rs`, parse `fixtures/service-protocol/method-effects.json`, select entries with `writes: []`, `process: never`, and `network: never`, execute their normal request fixtures against a fresh host, and compare `tree_snapshot` before/after. Seed a catalog once for methods that need a record, close it, then hash every catalog byte before and after the read-only call. Add dedicated assertions that a valid `snapshot.previewRollback` does not create the missing target parent.

- [ ] **Step 7: Run GREEN checks**

```sh
cargo test -p skills-copilot-service method_effects
cargo test -p skills-copilot-commands search_without_network_does_not_create_manager_cwd
pnpm verify:service-protocol-drift
```

Expected: all commands exit 0; fresh-root reads leave no files and seeded read-only calls leave catalog bytes unchanged.

- [ ] **Step 8: Commit the read-only behavior**

```sh
git add crates/service/src/service_host.rs crates/service/src/tests.rs crates/service/src/tests/method_effects.rs crates/commands/src/config_support.rs crates/commands/src/lib.rs crates/commands/src/skill_manager.rs
git commit -m "fix: keep service read paths side-effect free"
```

---

### Task 3: Add Rust config revisions and preview-bound rollback tokens

**Files:**
- Create: `crates/commands/src/config_consistency.rs`
- Modify: `crates/commands/src/lib.rs`
- Modify: `crates/service/src/lib.rs`
- Modify: `crates/service/src/service_host.rs`
- Modify: `crates/service/src/protocol.rs`
- Modify: `docs/service-protocol.md`
- Modify: `crates/service/src/tests/dispatch_fixtures.rs`
- Modify: `crates/service/src/tests/protocol_fixtures.rs`
- Modify: `fixtures/service-protocol/config.readClaudeSettings.response.json`
- Modify: `fixtures/service-protocol/config.readAgentConfig.response.json`
- Modify: `fixtures/service-protocol/config.saveClaudeSettings.request.json`
- Modify: `fixtures/service-protocol/config.saveClaudeSettings.response.json`
- Modify: `fixtures/service-protocol/snapshot.previewRollback.response.json`
- Modify: `fixtures/service-protocol/snapshot.rollback.request.json`
- Modify: `fixtures/service-protocol/app.version.response.json`
- Modify: `fixtures/service-protocol/service.status.response.json`
- Modify: `fixtures/service-protocol/app.stateSnapshot.response.json`

**Interfaces:**
- Produces: `config_revision(exists: bool, content: &str) -> String`.
- Produces: `read_config_state(path: &Path) -> Result<ConfigState, CommandError>` where `ConfigState { exists: bool, content: String, revision: String }`.
- Produces: `ensure_expected_revision(expected: &str, actual: &ConfigState) -> Result<(), CommandError>`.
- Produces: `rollback_preview_token(snapshot: &ConfigSnapshotRecord, current_revision: &str) -> String`.
- Produces: `ensure_rollback_preview_token(provided: &str, snapshot: &ConfigSnapshotRecord, current_revision: &str) -> Result<(), CommandError>`.
- Changes: `ConfigDocumentRecord` adds `revision: String`.
- Changes: `save_claude_settings(catalog, ctx, content, expected_revision)`.
- Changes: `SnapshotRollbackPreviewRecord` adds `current_revision` and `preview_token`.
- Changes: `rollback_snapshot(catalog, ctx, snapshot_id, preview_token)`.
- Adds service error codes: `config_conflict` for stale saves and `stale_preview_token` for rollback preview drift.
- Produces a typed protocol contract consumed by Task 1 of `2026-07-10-quality-ci-governance.md`; this Rust task does not implement Swift behavior.

- [ ] **Step 1: Define the tagged-revision/token helper contract and direct tests**

Create `config_consistency.rs` with the exact helpers and direct tests below, add `mod config_consistency;` to `crates/commands/src/lib.rs`, and add the two error variants shown after the module. This makes the pure hashing/token contract green in isolation while leaving the command boundary unwired for the RED run in Step 2:

```rust
use crate::CommandError;
use sha2::{Digest, Sha256};
use skills_copilot_catalog::ConfigSnapshotRecord;
use std::{fs, io, path::Path};

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct ConfigState {
    pub exists: bool,
    pub content: String,
    pub revision: String,
}

pub(crate) fn config_revision(exists: bool, content: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(if exists { b"present\0" } else { b"missing\0" });
    if exists {
        digest.update(content.as_bytes());
    }
    format!("sha256:{:x}", digest.finalize())
}

pub(crate) fn read_config_state(path: &Path) -> Result<ConfigState, CommandError> {
    let (exists, content) = match fs::read_to_string(path) {
        Ok(content) => (true, content),
        Err(error) if error.kind() == io::ErrorKind::NotFound => (false, String::new()),
        Err(error) => return Err(error.into()),
    };
    Ok(ConfigState {
        exists,
        revision: config_revision(exists, &content),
        content,
    })
}

pub(crate) fn ensure_expected_revision(
    expected: &str,
    actual: &ConfigState,
) -> Result<(), CommandError> {
    if expected == actual.revision {
        Ok(())
    } else {
        Err(CommandError::ConfigConflict {
            expected: expected.to_string(),
            actual: actual.revision.clone(),
        })
    }
}

pub(crate) fn rollback_preview_token(
    snapshot: &ConfigSnapshotRecord,
    current_revision: &str,
) -> String {
    let mut snapshot_content = Sha256::new();
    snapshot_content.update(snapshot.content.as_bytes());
    let snapshot_content_hash = format!("{:x}", snapshot_content.finalize());
    let mut token = Sha256::new();
    token.update(b"snapshot-rollback-preview\0");
    token.update(snapshot.id.as_bytes());
    token.update(b"\0");
    token.update(snapshot.target.as_bytes());
    token.update(b"\0");
    token.update(snapshot_content_hash.as_bytes());
    token.update(b"\0");
    token.update(current_revision.as_bytes());
    format!("sha256:{:x}", token.finalize())
}

pub(crate) fn ensure_rollback_preview_token(
    provided: &str,
    snapshot: &ConfigSnapshotRecord,
    current_revision: &str,
) -> Result<(), CommandError> {
    let expected = rollback_preview_token(snapshot, current_revision);
    if provided == expected {
        Ok(())
    } else {
        Err(CommandError::StalePreviewToken)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_snapshot(id: &str, target: &str, content: &str) -> ConfigSnapshotRecord {
        ConfigSnapshotRecord {
            id: id.to_string(),
            agent: "claude-code".to_string(),
            scope: "agent-global".to_string(),
            target: target.to_string(),
            content: content.to_string(),
            reason: "test".to_string(),
            created_at: 1,
        }
    }

    #[test]
    fn missing_and_present_empty_files_have_different_revisions() {
        assert_ne!(config_revision(false, ""), config_revision(true, ""));
    }

    #[test]
    fn missing_revision_ignores_display_only_default_content() {
        assert_eq!(config_revision(false, ""), config_revision(false, "{}\n"));
    }

    #[test]
    fn revision_never_contains_config_content() {
        let content = ["private", "-", "value"].join("");
        let revision = config_revision(true, &content);
        assert!(revision.starts_with("sha256:"));
        assert!(!revision.contains(&content));
    }

    #[test]
    fn stale_revision_returns_config_conflict() {
        let actual = ConfigState {
            exists: true,
            content: "{}\n".to_string(),
            revision: config_revision(true, "{}\n"),
        };
        assert!(matches!(
            ensure_expected_revision("sha256:stale", &actual),
            Err(CommandError::ConfigConflict { .. })
        ));
    }

    #[test]
    fn rollback_token_changes_with_snapshot_target_content_or_current_revision() {
        let base = config_snapshot("snap-1", "/tmp/config.json", "{}\n");
        let base_token = rollback_preview_token(&base, "sha256:current-a");
        assert_ne!(base_token, rollback_preview_token(&base, "sha256:current-b"));
        assert_ne!(base_token, rollback_preview_token(
            &config_snapshot("snap-1", "/tmp/other.json", "{}\n"),
            "sha256:current-a",
        ));
        assert_ne!(base_token, rollback_preview_token(
            &config_snapshot("snap-1", "/tmp/config.json", "{\"changed\":true}\n"),
            "sha256:current-a",
        ));
    }
}
```

Add this error variant to `CommandError`:

```rust
#[error("config changed since it was read (expected {expected}, actual {actual})")]
ConfigConflict { expected: String, actual: String },
#[error("snapshot rollback preview is stale; preview again before confirming")]
StalePreviewToken,
```

- [ ] **Step 2: Run the consistency tests and verify RED at the command boundary**

First run the helper tests:

```sh
cargo test -p skills-copilot-commands config_consistency::tests
```

Then add a command test that reads settings, externally changes the file, calls save with the old revision, and asserts the external content remains unchanged and the snapshot count remains zero. Run:

```sh
cargo test -p skills-copilot-commands tests::stale_claude_settings_save_is_rejected_without_snapshot_or_write -- --exact
```

Expected: the helper tests pass, while the command test fails because `save_claude_settings` does not yet accept or compare an expected revision.

- [ ] **Step 3: Enforce CAS under the existing config lock**

Import the Step 1 helpers in `crates/commands/src/lib.rs`. Change `ConfigDocumentRecord` and the save path to this shape:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ConfigDocumentRecord {
    pub agent: String,
    pub scope: String,
    pub target: String,
    pub format: String,
    pub content: String,
    pub exists: bool,
    pub revision: String,
}

pub fn save_claude_settings(
    catalog: &Catalog,
    ctx: &AdapterContext,
    content: &str,
    expected_revision: &str,
) -> Result<ConfigDocumentRecord, CommandError> {
    serde_json::from_str::<serde_json::Value>(content)
        .map_err(|error| CommandError::InvalidJson(error.to_string()))?;
    let target = claude_global_settings_path(ctx);
    validate_config_write_target(ctx, AgentId::ClaudeCode, Scope::AgentGlobal, &target)?;
    let lock_file = lock_config(ctx, AgentId::ClaudeCode, Scope::AgentGlobal, &target)?;
    let current = read_config_state(&target)?;
    if let Err(error) = ensure_expected_revision(expected_revision, &current) {
        lock_file.unlock()?;
        return Err(error);
    }
    let snapshot_id = generate_snapshot_id();
    let snapshot_content = redact_snapshot_content(&current.content);
    catalog.create_config_snapshot(ConfigSnapshotDraft {
        id: &snapshot_id,
        agent: ClaudeCodeAdapter.id().as_str(),
        scope: Scope::AgentGlobal.as_str(),
        target: &target.to_string_lossy(),
        content: &snapshot_content,
        reason: "pre-config-edit",
        created_at_ms: current_time_ms(),
    })?;
    write_config_atomic(ctx, AgentId::ClaudeCode, Scope::AgentGlobal, &target, content)?;
    let written = read_config_state(&target)?;
    if written.content != content {
        let _ = write_config_atomic(
            ctx,
            AgentId::ClaudeCode,
            Scope::AgentGlobal,
            &target,
            &current.content,
        );
        lock_file.unlock()?;
        return Err(CommandError::VerificationFailed);
    }
    lock_file.unlock()?;
    scan_claude_to_catalog(ctx, catalog)?;
    read_claude_settings(ctx)
}
```

Construct every `ConfigDocumentRecord` with `revision: config_revision(exists, &content)`. The helper intentionally ignores `content` when `exists == false`, so a UI-only default such as Claude's `{}\n` receives the same missing-file revision as `read_config_state`, while an existing empty file remains distinct.

- [ ] **Step 4: Bind rollback execution to its preview**

Add both fields to `SnapshotRollbackPreviewRecord`:

```rust
pub struct SnapshotRollbackPreviewRecord {
    pub snapshot: ConfigSnapshotRecord,
    pub current_content: String,
    pub current_read_error: Option<String>,
    pub current_revision: String,
    pub preview_token: String,
    pub changed: bool,
    pub redacted: bool,
    pub rollback_supported: bool,
}
```

In `preview_snapshot_rollback_for_record`, call `read_config_state`, calculate `preview_token = rollback_preview_token(&snapshot, &current.revision)`, and return both values without creating the target or its parent.

Change rollback to accept `preview_token: &str`. Its mutation order must be: load snapshot, perform non-creating target validation, read current state and reject an already-stale token before any write preparation, acquire the existing config lock, reload the snapshot by ID, read current target state again, recompute and compare the token from the reloaded snapshot/current revision under lock, reject redacted snapshot content, write that reloaded snapshot content, read back, unlock, then rescan. The critical pre-write section is:

```rust
validate_config_read_target(ctx, agent, scope, &target)?;
let before_lock = read_config_state(&target)?;
ensure_rollback_preview_token(preview_token, &snapshot, &before_lock.revision)?;
validate_config_write_target(ctx, agent, scope, &target)?;
let lock_file = lock_config(ctx, agent, scope, &target)?;
let locked_snapshot = catalog
    .get_config_snapshot(snapshot_id)?
    .ok_or_else(|| CommandError::StalePreviewToken)?;
let locked_agent = agent_from_snapshot(&locked_snapshot.agent)?;
let locked_scope = scope_from_snapshot(&locked_snapshot.scope)?;
if locked_agent != agent || locked_scope != scope {
    lock_file.unlock()?;
    return Err(CommandError::StalePreviewToken);
}
let current = read_config_state(&target)?;
if let Err(error) = ensure_rollback_preview_token(
    preview_token,
    &locked_snapshot,
    &current.revision,
) {
    lock_file.unlock()?;
    return Err(error);
}
if is_redacted_snapshot_content(&locked_snapshot.content) {
    lock_file.unlock()?;
    return Err(CommandError::UnsafeConfigPath(
        "snapshot content was redacted and cannot be rolled back directly".to_string(),
    ));
}
```

After this block, write `locked_snapshot.content`, not the pre-lock copy. No config snapshot, target write, temporary config file, catalog update, or rescan may occur before either token check. The pre-lock check guarantees the ordinary stale-token path has no write preparation; the snapshot reload plus second current-state check closes races between preview/read and lock acquisition. Toggle and batch paths continue to patch the latest content read under their lock and therefore do not accept client revisions or preview tokens.

- [ ] **Step 5: Propagate the typed protocol and stable error code**

Change the Rust request types:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct SaveClaudeSettingsParams {
    pub content: String,
    pub expected_revision: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RollbackSnapshotParams {
    pub snapshot_id: String,
    pub preview_token: String,
}
```

Keep `SnapshotParams` for preview only. Map the command conflict before the general command case:

```rust
Self::Command(skills_copilot_commands::CommandError::ConfigConflict { .. }) => "config_conflict",
Self::Command(skills_copilot_commands::CommandError::StalePreviewToken) => "stale_preview_token",
Self::Command(_) => "command_error",
```

Because required request fields and response fields change, set `SERVICE_PROTOCOL_VERSION` to `2` and update the three protocol-version response fixtures. The native client and its fake responses are updated by Task 1 of `2026-07-10-quality-ci-governance.md` before the combined macOS gate runs.

Add a `## Config Consistency` section to `docs/service-protocol.md` that defines the tagged revision, the save `expected_revision`, the preview-bound rollback token fields, the lock-time recheck, and the stable `config_conflict`/`stale_preview_token` errors. State explicitly that clients must preview again after a stale rollback token and must not retry either mutation automatically.

- [ ] **Step 6: Update protocol fixtures and prove stale writes over JSON RPC**

Use these payload shapes:

```json
{"id":"req-config-save","method":"config.saveClaudeSettings","params":{"content":"{}\n","expected_revision":"sha256:0000000000000000000000000000000000000000000000000000000000000000"}}
```

```json
{"id":"req-snapshot-rollback","method":"snapshot.rollback","params":{"snapshot_id":"snapshot-1","preview_token":"sha256:0000000000000000000000000000000000000000000000000000000000000000"}}
```

Add one service test that changes the target after `config.readClaudeSettings`, sends the old revision to `config.saveClaudeSettings`, and asserts `error.code == "config_conflict"`, no snapshot row, and unchanged external bytes. Add another that obtains a rollback preview, changes the target, sends the old preview token, and asserts `error.code == "stale_preview_token"`, no new snapshot row, unchanged target bytes, and no rescan/catalog mutation. Route rollback through an internal `rollback_snapshot_with_after_lock(..., after_lock: impl FnOnce())` helper whose public wrapper passes an empty closure; a commands-layer test callback rewrites the target immediately after lock acquisition and proves the lock-time reread returns `StalePreviewToken` without any rollback-owned snapshot, target, temporary-file, or catalog write. Separately assert that reloading a snapshot record with changed content or target invalidates the token before its content can be written.

- [ ] **Step 7: Run GREEN Rust CAS/token checks**

```sh
cargo test -p skills-copilot-commands config_consistency
cargo test -p skills-copilot-commands stale_claude_settings_save_is_rejected_without_snapshot_or_write
cargo test -p skills-copilot-commands rollback_rechecks_state_after_lock
cargo test -p skills-copilot-service config_conflict
cargo test -p skills-copilot-service stale_preview_token
pnpm verify:service-protocol-drift
```

Expected: all commands exit 0; stale saves return `config_conflict`; stale, forged, target-mismatched, snapshot-content-mismatched, and current-revision-mismatched rollback tokens return `stale_preview_token`; all rejection cases leave target bytes and snapshot counts unchanged.

- [ ] **Step 8: Commit the Rust protocol slice**

```sh
git add crates/commands/src/config_consistency.rs crates/commands/src/lib.rs crates/service/src/lib.rs crates/service/src/service_host.rs crates/service/src/protocol.rs crates/service/src/tests fixtures/service-protocol docs/service-protocol.md
git commit -m "fix: bind config writes to fresh state"
```

---

### Task 4: Scan current, untracked, and historical Git blobs for privacy leaks

**Files:**
- Create: `scripts/test-privacy-check.mjs`
- Modify: `script/check_privacy.sh`
- Modify: `package.json`
- Modify: `.github/workflows/ci.yml`
- Modify: `docs/security-model.md`

**Interfaces:**
- Current candidates: index blobs, tracked worktree files, and `git ls-files --others --exclude-standard` files.
- Historical candidates: unique blob OIDs from `git rev-list --objects --all`, read with `git cat-file --batch` or `git cat-file blob`.
- Exit contract: `0` clean, `1` detected content, `2` invalid CLI argument or missing Git precondition.
- `--no-history` affects only historical candidates.

- [ ] **Step 1: Create a disposable-repository RED suite**

Create `scripts/test-privacy-check.mjs`. Resolve the real checker path before changing cwd, create each case with `mkdtemp`, and construct sample strings from pieces:

```js
#!/usr/bin/env node
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const checker = join(repoRoot, "script/check_privacy.sh");
const localPathSample = ["/", "Users", "example", "private-note"].join("/").replace("//", "/");

function git(cwd, ...args) {
  return execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
}

function makeRepo(name) {
  const cwd = mkdtempSync(join(tmpdir(), `agent-copilot-privacy-${name}-`));
  git(cwd, "init", "-q");
  git(cwd, "config", "user.email", "privacy-test@example.invalid");
  git(cwd, "config", "user.name", "Privacy Test");
  writeFileSync(join(cwd, "clean.txt"), "clean\n");
  git(cwd, "add", "clean.txt");
  git(cwd, "commit", "-qm", "clean baseline");
  return cwd;
}

function runChecker(cwd, ...args) {
  return spawnSync("bash", [checker, ...args], { cwd, encoding: "utf8" });
}

function expectFailure(label, arrange, args = []) {
  const cwd = makeRepo(label);
  try {
    arrange(cwd);
    const result = runChecker(cwd, ...args);
    assert.equal(result.status, 1, `${label}\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
  } finally {
    rmSync(cwd, { recursive: true, force: true });
  }
}

expectFailure("untracked-text", (cwd) => {
  writeFileSync(join(cwd, "note.txt"), `${localPathSample}\n`);
});

expectFailure("untracked-binary", (cwd) => {
  writeFileSync(join(cwd, "artifact.bin"), Buffer.concat([
    Buffer.from([0, 1, 2, 0]),
    Buffer.from(localPathSample),
    Buffer.from([0, 3, 0]),
  ]));
});

expectFailure("historical-binary", (cwd) => {
  writeFileSync(join(cwd, "artifact.bin"), Buffer.concat([Buffer.from([0]), Buffer.from(localPathSample)]));
  git(cwd, "add", "artifact.bin");
  git(cwd, "commit", "-qm", "add artifact");
  writeFileSync(join(cwd, "artifact.bin"), Buffer.from([0, 1, 2]));
  git(cwd, "add", "artifact.bin");
  git(cwd, "commit", "-qm", "replace artifact");
});

const clean = makeRepo("clean");
try {
  assert.equal(runChecker(clean).status, 0);
} finally {
  rmSync(clean, { recursive: true, force: true });
}
```

Extend the same file with staged/index-only, tracked worktree, deleted historical text, ignored untracked, and `--no-history` cases. For index-only, stage the sensitive sample and restore only the worktree copy before invoking the checker.

- [ ] **Step 2: Run the regression suite and verify RED**

```sh
node scripts/test-privacy-check.mjs
```

Expected: FAIL on at least untracked text, untracked binary, and historical binary because the current checker does not enumerate those blobs.

- [ ] **Step 3: Refactor the checker around blob streams**

Keep the existing sensitive-content and fixed-local-host patterns and CLI. Build `combined_pattern="(${text_pattern}|${local_host_port_pattern})"`, preserving the loopback-port-zero exception already encoded in `local_host_port_pattern`. Add Bash functions with these contracts:

```bash
scan_stream() {
  local label="$1"
  if strings | grep -E "$combined_pattern" >/dev/null; then
    echo "privacy check failed: sensitive-looking content in $label" >&2
    return 1
  fi
}

scan_blob_oid() {
  local oid="$1"
  local label="$2"
  git cat-file blob "$oid" | scan_stream "$label"
}
```

Enumerate and de-duplicate index/history OIDs with associative arrays. Enumerate current paths with NUL delimiters:

```bash
while IFS= read -r -d '' path; do
  if [[ -L "$path" ]]; then
    readlink "$path" | scan_stream "worktree-symlink:$path" || current_failed=1
    continue
  fi
  [[ -f "$path" ]] || continue
  scan_stream "worktree:$path" < "$path" || current_failed=1
done < <(git ls-files -z --cached --others --exclude-standard)
```

Read index OIDs from `git ls-files -s -z`; split each NUL-delimited record at its first tab, then parse `mode oid stage` and scan each unique nonzero OID even when the worktree differs. Enumerate history with `git rev-list --objects --all --no-object-names` so unusual historical filenames cannot corrupt parsing; confirm each unique object's type is `blob`, then scan it. Report the candidate class and OID plus a repository-relative path for index/worktree candidates, but never print matched content. Feed raw blob bytes into `scan_stream`; do not pre-run `strings`, because `scan_stream` owns the single extraction pass. Never follow a worktree symlink outside the repository: scan only its `readlink` text while the index/history loops scan the symlink blob itself.

- [ ] **Step 4: Preserve exclusions and option semantics explicitly**

Ignored untracked files remain excluded through `--exclude-standard`. Continue excluding tracked `dist/` and `target/` paths from current/path-based checks. `--no-history` must bypass only the reachable-object loop. Do not skip a historical blob merely because its current path no longer exists.

- [ ] **Step 5: Wire tests and CI**

Add:

```json
"test:privacy-check": "node scripts/test-privacy-check.mjs"
```

In the full-history privacy job, run:

```yaml
- name: Test privacy leak guard
  run: pnpm test:privacy-check

- name: Check privacy leaks
  run: pnpm check:privacy
```

Retain `actions/checkout` with `fetch-depth: 0`.

- [ ] **Step 6: Document exact automated and manual coverage**

In `docs/security-model.md`, state that `pnpm check:privacy` scans index, tracked worktree, untracked non-ignored files, and all reachable Git blobs; state separately that binary string extraction is not OCR and new screenshots require manual inspection.

- [ ] **Step 7: Run GREEN privacy checks**

```sh
pnpm test:privacy-check
pnpm check:privacy
```

Expected: regression cases produce their expected pass/fail status, then the real repository reports `privacy check: ok`.

- [ ] **Step 8: Commit the privacy guard**

```sh
git add script/check_privacy.sh scripts/test-privacy-check.mjs package.json .github/workflows/ci.yml docs/security-model.md
git commit -m "fix: scan untracked and historical privacy blobs"
```

---

### Task 5: Freeze YAML behavior before changing dependencies

**Files:**
- Modify: `crates/adapters/src/claude_code/mod.rs`
- Modify: `crates/adapters/src/codex/mod.rs`
- Modify: `crates/adapters/src/opencode/mod.rs`
- Modify: `crates/adapters/src/pi/mod.rs`
- Modify: `crates/adapters/src/hermes/mod.rs`
- Modify: `crates/adapters/src/openclaw/mod.rs`
- Modify: `crates/scanner/src/lib.rs`
- Modify: `crates/ai-core/src/lib.rs`
- Modify: `crates/commands/src/tests.rs`

**Interfaces:**
- Preserves: frontmatter scalar/sequence/bool/nested mapping behavior across all six adapters.
- Preserves: Hermes config parsing and serialization of unrelated keys.
- Preserves: scanner disabled-skill resolution and AI required-field findings.
- Produces: focused compatibility test names usable before and after the dependency change.

- [ ] **Step 1: Add compatibility tests against the current parser**

Add tests covering:

```rust
#[test]
fn yaml_contract_preserves_scalar_sequence_bool_and_nested_mapping() {
    let raw = r#"
name: sample-skill
description: Sample
enabled: true
allowed-tools:
  - Read
  - Search
metadata:
  openclaw:
    skillKey: routed-key
"#;
    let value: serde_yaml::Value = serde_yaml::from_str(raw).unwrap();
    assert_eq!(value.get("name").and_then(serde_yaml::Value::as_str), Some("sample-skill"));
    assert_eq!(value.get("enabled").and_then(serde_yaml::Value::as_bool), Some(true));
    assert_eq!(value.get("allowed-tools").and_then(serde_yaml::Value::as_sequence).unwrap().len(), 2);
    assert_eq!(
        value.get("metadata")
            .and_then(|item| item.get("openclaw"))
            .and_then(|item| item.get("skillKey"))
            .and_then(serde_yaml::Value::as_str),
        Some("routed-key")
    );
}
```

Add a Hermes round-trip test that patches `skills.disabled`, reparses the result, and asserts unrelated mappings and scalar values survive. Add malformed YAML tests that preserve the current broken-skill/error behavior instead of panicking.

- [ ] **Step 2: Run the contract suite on the old dependency**

```sh
cargo test -p skills-copilot-adapters yaml_contract
cargo test -p skills-copilot-scanner yaml_contract
cargo test -p skills-copilot-ai-core frontmatter
cargo test -p skills-copilot-commands yaml_contract
```

Expected: all tests pass before dependency changes, establishing the behavior baseline.

- [ ] **Step 3: Commit behavior-only tests**

```sh
git add crates/adapters/src crates/scanner/src/lib.rs crates/ai-core/src/lib.rs crates/commands/src/tests.rs
git commit -m "test: freeze yaml parsing contracts"
```

---

### Task 6: Replace the deprecated YAML crate and refresh the fuzz lock

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/adapters/Cargo.toml`
- Modify: `crates/scanner/Cargo.toml`
- Modify: `crates/ai-core/Cargo.toml`
- Modify: `crates/commands/Cargo.toml`
- Modify: `crates/adapters/src/shared.rs`
- Modify: `crates/adapters/src/claude_code/mod.rs`
- Modify: `crates/adapters/src/codex/mod.rs`
- Modify: `crates/adapters/src/opencode/mod.rs`
- Modify: `crates/adapters/src/pi/mod.rs`
- Modify: `crates/adapters/src/hermes/mod.rs`
- Modify: `crates/adapters/src/openclaw/mod.rs`
- Modify: `crates/scanner/src/lib.rs`
- Modify: `crates/ai-core/src/lib.rs`
- Modify: `crates/commands/src/lib.rs`
- Regenerate: `crates/adapters/fuzz/Cargo.lock`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Candidate package: `serde_norway`, subject to the implementation-time verification gate below.
- Source symbol after migration: `serde_norway::{Value, Mapping, from_str, to_string}`.
- Fuzz workspace remains excluded from the root workspace but is checked independently with its own lockfile.

- [ ] **Step 1: Re-verify the candidate from primary sources**

Run on the implementation date:

```sh
cargo search serde_norway --limit 1
cargo info serde_norway
```

Open and compare the current official crate repository/registry metadata with the current RustSec advisory for the deprecated YAML chain. Continue only if the candidate is maintained, supports the repository's Rust toolchain, exposes the required dynamic `Value`/`Mapping` and serialization API, and has no active advisory that invalidates the migration. If any condition fails, stop this task and record the upstream evidence rather than selecting a different crate silently.

- [ ] **Step 2: Replace the workspace dependency and symbols**

Use Cargo tooling to add the verified current candidate release to `[workspace.dependencies]`; remove `serde_yaml`. Change the four crate manifests to `serde_norway.workspace = true`. Replace source paths mechanically:

```sh
rg -l 'serde_yaml' crates --glob '*.rs' | xargs sed -i.bak 's/serde_yaml/serde_norway/g'
find crates -name '*.bak' -delete
```

Inspect every changed file and confirm only the crate path changed; do not accept formatter or content rewrites in fixture strings.

- [ ] **Step 3: Run the frozen contracts and verify GREEN**

```sh
cargo fmt --all -- --check
cargo test -p skills-copilot-adapters yaml_contract
cargo test -p skills-copilot-scanner yaml_contract
cargo test -p skills-copilot-ai-core frontmatter
cargo test -p skills-copilot-commands yaml_contract
```

Expected: all pre-migration behavior tests still pass. If serialization text differs while parsed semantics match, keep assertions semantic; do not weaken path, boolean, sequence, or unrelated-key assertions.

- [ ] **Step 4: Regenerate and verify both lockfiles**

```sh
cargo check --workspace
cargo generate-lockfile --manifest-path crates/adapters/fuzz/Cargo.toml
cargo check --manifest-path crates/adapters/fuzz/Cargo.toml --locked
cargo check --manifest-path crates/adapters/fuzz/Cargo.toml --locked --offline
```

Expected: both checks exit 0. `crates/adapters/fuzz/Cargo.lock` resolves local `skills-copilot-adapters` and `skills-copilot-core` at the same package version as their manifests.

- [ ] **Step 5: Add the excluded fuzz lock gate to CI**

Add after workspace clippy in the Rust job:

```yaml
- name: Check adapter fuzz workspace lock
  if: runner.os != 'Windows'
  run: cargo check --manifest-path crates/adapters/fuzz/Cargo.toml --locked
```

- [ ] **Step 6: Prove the deprecated dependency is absent**

```sh
! cargo tree --workspace | rg 'serde_yaml'
! cargo tree --manifest-path crates/adapters/fuzz/Cargo.toml | rg 'serde_yaml'
! rg -n 'name = "serde_yaml"' Cargo.lock crates/adapters/fuzz/Cargo.lock
cargo audit
```

Expected: the three negated searches find nothing and `cargo audit` exits 0.

- [ ] **Step 7: Run the complete Rust gate**

```sh
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo doc --workspace --no-deps
```

Expected: all commands exit 0.

- [ ] **Step 8: Commit dependency and fuzz consistency**

```sh
git add Cargo.toml Cargo.lock crates/adapters/Cargo.toml crates/adapters/fuzz/Cargo.lock crates/adapters/src crates/scanner/Cargo.toml crates/scanner/src/lib.rs crates/ai-core/Cargo.toml crates/ai-core/src/lib.rs crates/commands/Cargo.toml crates/commands/src/lib.rs .github/workflows/ci.yml
git commit -m "build: replace deprecated yaml dependency"
```

---

### Task 7: Run integrated service/privacy verification

**Files:**
- Modify only if a command exposes a real contract mismatch: the file owned by the task that introduced the mismatch.

**Interfaces:**
- Consumes all outputs from Tasks 1–6.
- Produces no new runtime API.

- [ ] **Step 1: Run deterministic protocol and privacy gates**

```sh
pnpm test:service-protocol-effects
pnpm verify:service-protocol-drift
pnpm test:privacy-check
pnpm check:privacy
pnpm verify:gate-parity
```

Expected: every command exits 0.

- [ ] **Step 2: Run full Rust verification**

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo check --manifest-path crates/adapters/fuzz/Cargo.toml --locked
```

Expected: formatting is clean, all tests pass, clippy reports no warnings, and the excluded fuzz workspace resolves its committed lock.

- [ ] **Step 3: Inspect the final diff and privacy-sensitive output**

```sh
git diff --check
git status --short
git diff --stat
```

Expected: no whitespace errors; only files named in this plan are modified; no generated `target`, `.build`, `dist`, or dependency cache is staged.

- [ ] **Step 4: Commit any integration-only correction**

Skip this commit when Step 1–3 require no correction. If a correction was necessary, stage only its owning task files and use:

```sh
git commit -m "test: align service consistency gates"
```

---

## Final Acceptance Criteria

- `fixtures/service-protocol/method-effects.json` has exactly one valid entry for every supported method and no unsupported entries.
- Protocol documentation, status, dispatch, fixtures, and side-effect metadata agree under a deterministic verifier.
- All methods declared fully read-only leave a fresh filesystem unchanged and do not mutate an existing catalog.
- Catalog initialization and legacy migration occur only on explicit write/scan paths.
- Config reads and rollback previews do not create missing directories.
- Stale save requests return `config_conflict`; stale, forged, or mismatched rollback tokens return `stale_preview_token`. Both paths leave the target, snapshots, audit records, and catalog unchanged.
- Rollback preview tokens bind snapshot ID, target, snapshot-content hash, and current config revision, and are recomputed after acquiring the config lock.
- Fresh save and rollback requests retain locking, atomic write, private permission, snapshot, readback, and rescan behavior.
- Privacy validation catches tracked, staged-only, untracked text, untracked binary, deleted historical text, and replaced historical binary samples.
- `--no-history` does not weaken current/index/untracked checks.
- The deprecated YAML crate is absent from root and fuzz dependency graphs, while the frozen YAML behavior suite remains green.
- The excluded fuzz workspace passes `cargo check --locked` and resolves local crates at current manifest versions.

## Execution Handoff

Execute this plan with `superpowers:subagent-driven-development` for task-by-task implementation and two-stage review. Run Task 1 of `2026-07-10-quality-ci-governance.md` immediately after this plan's Task 3 so the native client consumes protocol version 2 before the combined macOS gate. Use `superpowers:executing-plans` only when a single worker must run the tasks sequentially with checkpoints after Tasks 2, 4, and 6.
