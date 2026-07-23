# Service Protocol

The native app talks to the Rust service through typed JSON request/response
messages over stdio. `crates/service/src/protocol.rs` is the source of truth for
method names and typed payloads; this document is the human-readable contract
index.

## Runtime Shape

Request:

```json
{"id":"req-1","method":"catalog.listSkills","params":{}}
```

Success response:

```json
{"id":"req-1","ok":true,"result":[]}
```

Error response:

```json
{"id":"req-1","ok":false,"error":{"code":"unknown_method","message":"unknown method: x"}}
```

The stdio transport may change in the future, but method names, payloads,
fixtures, and stable error codes must remain synchronized with protocol drift
verification.

## Protocol Rules

- UI shells must call service methods instead of importing scanner/catalog
  internals.
- Provider calls require preview, redaction, destination visibility, and
  explicit confirmation.
- Skill scripts remain default-denied.
- Skill Manager may invoke supported external manager CLIs for search,
  install, remove, update, list, and local template creation when the request
  exposes command preview, target agents, network posture, telemetry-off env,
  and confirmation state. Calls must use argv arrays, not shell strings.
- App-local metadata writes must be redacted.
- Adapter config writes must use the guarded paths documented in
  `docs/adapters/agent-adapters.md`.
- Service method changes must update fixtures and pass
  `pnpm verify:service-protocol-drift`.

## Product Design Compatibility

The Methods table below is the callable service inventory. Product design or
implementation documents may define a required future projection, but a client
must not call it until `crates/service/src/protocol.rs`, method-effect tests,
fixtures, Swift wire models, and this table expose the same method.

The product rebuild reserves these responsibilities and preferred additive
method names:

- `project.getReadiness` for deterministic project coverage, per-agent health,
  evidence, and attention actions;
- `catalog.listSkillAggregates` for complete, evidence-backed skill
  projections;
- `session.previewResume` for an adapter-native copy-only continuation command
  or typed unsupported reason.

These names are not supported merely because they appear in this paragraph.
Existing lower-level catalog, project, session, search, and config methods
remain authoritative until each additive method is implemented and fixture
backed. `task_cockpit` remains the compatible LLM action identifier even when a
native UI labels the experience Task Readiness.

## Config Consistency

Protocol version 2 makes direct config saves and snapshot rollback confirmations
conditional on the exact local state that the client reviewed.

- `config.readClaudeSettings` and every row from `config.readAgentConfig`
  include an opaque tagged `revision`. The revision is `sha256:` plus a
  domain-separated digest of either `present\0` and the exact file bytes or
  `missing\0`. A missing file is therefore distinct from an existing empty
  file, while UI-only default content does not change the missing-file
  revision.
- `config.saveClaudeSettings` requires `content` and `expected_revision`. The
  service first performs a non-creating read preflight before initializing the
  catalog or preparing a lock path. If that passes, it acquires the existing
  config lock and authoritatively rereads and compares the target again before
  catalog initialization, snapshot creation, or a target write. An initial
  mismatch returns the stable `config_conflict` error with no filesystem
  entries created; either mismatch leaves the external bytes, catalog, and
  snapshot history unchanged.
- `snapshot.previewRollback` returns `current_revision` and an opaque
  `preview_token`. The token binds the snapshot id, target, a digest of the
  snapshot content, and the current target revision; it does not expose the
  snapshot content or config values.
- `snapshot.rollback` accepts only `snapshot_id` and `preview_token`. It checks
  the token before write preparation, acquires the config lock, reloads the
  snapshot by id, rereads the target, and checks the token again before writing
  the reloaded snapshot content. Snapshot replacement or target drift returns
  the stable `stale_preview_token` error without a target write, catalog
  refresh, or rollback-owned snapshot. A snapshot deleted after preview, or a
  snapshot whose agent, scope, or target no longer validates, is also reported
  as `stale_preview_token`; rollback does not read a rejected drifted target.
- Clients must surface either conflict and ask the user to read or preview
  again. They must not automatically retry a stale save or rollback. A bare
  revision is not a rollback authorization token. Toggle and batch operations
  continue to patch the latest content read under their own existing lock and
  do not accept client revisions or rollback preview tokens.

## Methods

| Method | Local writes | External process | Network | Confirmation |
| --- | --- | --- | --- | --- |
| `app.version` | None | Never | Never | None |
| `app.stateSnapshot` | None | Never | Never | None |
| `app.search` | None | Never | Never | None |
| `service.status` | None | Never | Never | None |
| `adapter.listCapabilities` | None | Never | Never | None |
| `adapter.listDiagnostics` | None | Never | Never | None |
| `session.previewLocalSessions` | None | Never | Never | None |
| `session.listLocalSessionMessages` | None | Never | Never | None |
| `llm.status` | None | Never | Never | None |
| `llm.listProviderProfiles` | None | Never | Never | None |
| `llm.saveProviderProfile` | App-local data, Keychain | Never | Never | None |
| `llm.deleteProviderProfile` | App-local data, Keychain | Never | Never | None |
| `llm.testProviderConnection` | App-local data | Never | Always | Required |
| `llm.previewPrompt` | None | Never | Never | None |
| `llm.confirmPromptAndSend` | App-local data | Never | Always | Required |
| `llm.listPromptRuns` | None | Never | Never | None |
| `llm.providerObservability` | None | Never | Never | None |
| `llm.listProviderActivity` | None | Never | Never | None |
| `llm.listModelTaskMatches` | None | Never | Never | None |
| `llm.recordModelTaskMatch` | App-local data | Never | Never | None |
| `llm.deleteModelTaskMatch` | App-local data | Never | Never | None |
| `llm.prepareAction` | None | Never | Never | None |
| `rules.listTuning` | None | Never | Never | None |
| `rules.setSeverityOverride` | App-local data | Never | Never | None |
| `rules.clearSeverityOverride` | App-local data | Never | Never | None |
| `rules.setSuppression` | App-local data | Never | Never | None |
| `rules.clearSuppression` | App-local data | Never | Never | None |
| `batch.previewSkillToggles` | None | Never | Never | None |
| `batch.applySkillToggles` | Agent config, App-local data | Never | Never | Required |
| `script.previewExecution` | None | Never | Never | None |
| `script.execute` | Blocked-attempt audit only | Never | Never | Required |
| `skillManager.listTools` | None | Never | Never | None |
| `skillManager.search` | External manager state may change when invoked | Conditional | Conditional | None |
| `skillManager.listInstalled` | External manager state may change when invoked | Always | Never | None |
| `skillManager.previewInstall` | None | Never | Never | None |
| `skillManager.applyInstall` | App-local data, External manager state may change when invoked | Always | Conditional | Required |
| `skillManager.previewRemove` | None | Never | Never | None |
| `skillManager.applyRemove` | App-local data, External manager state may change when invoked | Always | Never | Required |
| `skillManager.previewUpdate` | None | Never | Never | None |
| `skillManager.applyUpdate` | App-local data, External manager state may change when invoked | Always | Conditional | Required |
| `skillManager.previewLocalCreate` | None | Never | Never | None |
| `skillManager.applyLocalCreate` | App-local data, External manager state may change when invoked | Always | Never | Required |
| `skillManager.deleteLocal` | App-local data | Never | Never | Required |
| `skillManager.previewLocalArchiveImport` | None | Never | Never | None |
| `skillManager.applyLocalArchiveImport` | App-local data | Never | Never | Required |
| `skillManager.previewLocalArchiveUpdate` | None | Never | Never | None |
| `skillManager.applyLocalArchiveUpdate` | App-local data | Never | Never | Required |
| `project.getContext` | None | Never | Never | None |
| `project.setContext` | App-local data | Never | Never | None |
| `project.clearContext` | App-local data | Never | Never | None |
| `project.removeRecentContext` | App-local data | Never | Never | None |
| `project.clearRecentContexts` | App-local data | Never | Never | None |
| `project.validateContext` | None | Never | Never | None |
| `catalog.listSkills` | None | Never | Never | None |
| `catalog.getSkill` | None | Never | Never | None |
| `catalog.analysis` | None | Never | Never | None |
| `catalog.listFindings` | None | Never | Never | None |
| `catalog.listFindingTriage` | None | Never | Never | None |
| `catalog.setFindingTriage` | App-local data | Never | Never | None |
| `catalog.clearFindingTriage` | App-local data | Never | Never | None |
| `catalog.listConflicts` | None | Never | Never | None |
| `catalog.importSkill` | App-local data | Never | Never | None |
| `catalog.scanClaude` | App-local data | Never | Never | None |
| `catalog.scanAll` | App-local data | Never | Never | None |
| `skill.exportBundle` | Export destination | Never | Never | None |
| `skill.install` | Agent skill files, App-local data | Never | Never | Required |
| `skill.listEvents` | None | Never | Never | None |
| `skill.listEventsPage` | None | Never | Never | None |
| `config.toggleSkill` | Agent config, App-local data | Never | Never | None |
| `config.readAgentConfig` | None | Never | Never | None |
| `config.readClaudeSettings` | None | Never | Never | None |
| `config.saveClaudeSettings` | Agent config, App-local data | Never | Never | None |
| `snapshot.list` | None | Never | Never | None |
| `snapshot.listAgentConfig` | None | Never | Never | None |
| `snapshot.listAgentConfigPage` | None | Never | Never | None |
| `snapshot.previewRollback` | None | Never | Never | None |
| `snapshot.rollback` | Agent config, App-local data | Never | Never | Required |

## Provider Observability

`llm.providerObservability` is a read-only, app-local dashboard over redacted
prompt-run and provider-call metadata created by Agent Copilot's own optional
AI features. It does not read, infer, or aggregate provider usage from managed
agent configuration or from provider profiles used directly by Claude Code,
Codex, opencode, Pi, or other managed agents.

Requests may pass `window_days` for a rolling range or explicit `start_at` /
`end_at` Unix-millisecond bounds. When both are present, explicit bounds define
the applied range. The response `filters` echoes the applied range and includes
`aggregation_uses_full_range`. `limit` bounds returned evidence rows such as
`history_rows` and `call_rows`; summary metrics, grouping rows, status rows,
and budget hints are computed from all app AI metadata matching the applied
date/filter range before evidence rows are limited.

`llm.listProviderActivity` is the read-only paged detail companion to that
full-range aggregate. It merges the same redacted prompt-run and provider-call
metadata into `(timestamp DESC, id ASC)` order without changing the aggregate
summary. Page limits are clamped to `1...100`; opaque cursors bind provider,
model, action, and time filters. A rolling `window_days` request resolves fixed
start/end bounds on page one; the opaque cursor carries those bounds so later
pages do not move when the clock advances. Explicit start/end bounds remain
exactly filter-bound.

Activity reads use a bounded, retrying before/after snapshot of the complete raw
bytes for both app-local metadata sources. On Unix, each source is opened with
no-follow and nonblocking descriptor flags, then validated as a regular file by
the opened handle. Handle length and an 8 MiB+1 bounded read enforce the 8 MiB
per-source limit even if a file grows or its path is replaced during the read.
Symlinks and nonregular sources fail closed without blocking. A page is returned
only when the two sources were jointly stable during one read window. Prompt-run
JSON and every non-empty provider-call JSONL row must parse completely;
unreadable, oversized, truncated, or malformed sources fail closed with
`provider_activity_source_unreadable` or `provider_activity_source_invalid`
without returning raw error text. The opaque `source_revision` hashes the
presence and complete bounded raw bytes of both sources, including bytes that do
not become display rows. Any later source-byte change returns `source_changed`.

Row IDs derive from the prompt-run ID or provider-call confirmation ID, with a
source-prefixed `fallback-v1` digest for older records. Fallback digests use an
explicit canonical field set with length-prefixed labels and values rather than
typed-record serialization. They do not depend on filter position or array
index, and remain distinct across the two source kinds. IDs are validated for
global uniqueness across both complete sources before filtering, totals, or
paging. Duplicate prompt IDs, provider confirmation IDs, or fallback identities
fail closed as `provider_activity_source_invalid`; duplicate rows are never
silently removed by the client. Only redacted titles, subtitles, status, stable
IDs, and evidence references cross the service boundary. The method does not
read credentials, send provider traffic, persist raw prompts/responses/traces,
or expose write controls.

## Full-Access Local Lists

- `llm.listPromptRuns` and `llm.listModelTaskMatches` do not apply a default
  row limit. Omit `limit` when the UI needs the full local list. Passing
  `limit` explicitly requests a bounded page/preview.
- Bounded responses expose returned-vs-total metadata where the protocol
  supports it (`total_count` / `returned_count` or
  `total_*_count` / `returned_*_count`, plus `truncated`) so clients do not
  mistake a limited page for full history.
- `snapshot.listAgentConfigPage` and `skill.listEventsPage` provide complete,
  additive paged access while the legacy unpaged methods and response shapes
  remain supported. Page limits are clamped to `1...100`.
- Config snapshot records include optional `project_root`. Project-scoped
  records are returned only when that canonical root matches the active
  service project context; legacy project rows without a binding are hidden.
  Preview and rollback repeat the same binding check and include it in the
  confirmation token, so a snapshot from project A cannot be used in project B.
- Config pages use `(created_at DESC, id DESC)` and event pages use
  `(occurred_at DESC, id DESC)` stable keyset order. Continuations carry an
  opaque `v1:` cursor and the first page's `source_revision`; a changed ordered
  source returns `source_changed` before returning continuation rows.
- Page results contain `records`, `source_revision`, `returned_count`,
  `total_count`, `has_more`, `next_cursor`, `source_completeness`, and optional
  `incomplete_reason`. Clients should accumulate by stable record ID and retain
  accepted rows if loading is cancelled or a later page fails.

## Catalog Skill Provenance

- `catalog.listSkills`, `catalog.getSkill`, toggle responses, and batch-toggle
  skill records include optional `publisher`, `package_name`,
  `package_version`, `source_kind`, and `read_only_reason` fields.
- Installed Codex plugin skills are scanned only from bounded roots declared by
  effectively enabled installed-plugin records and their manifests. The plugin
  store/cache is never a generic root. These skills appear in
  `catalog.listSkills` and current instance/analysis/conflict projections with
  publisher, package, version, and read-only provenance.
- `source_kind="chatgpt-plugin-cache"` remains the backward-compatible wire
  value for those installed files; clients should present it as an installed
  Codex plugin source, not as a runtime-only record. Legacy synthetic
  `codex-runtime` rows are removed by schema migration and are excluded from
  current projections if encountered.
- Provenance is derived deterministically from the cataloged path at read time.
  The service does not persist plugin manifests, introduce a plugin write path,
  or merge Codex plugin ownership with `skillManager.*` package ownership.
- The product discovery path never invokes the Codex runtime inventory API and
  does not use it as an authoritative source. Release validation may compare a
  temporary read-only runtime inventory against the filesystem result without
  persisting runtime rows.
- Read-only startup/reload, list, detail, analysis, conflict, app-search, and
  LLM skill-selection paths project current guarded Codex `[[skills.config]]`
  path overrides and `[plugins.<id>] enabled` values over cached native/plugin
  records. Removing an override restores cached `loaded`/`disabled` records to
  loaded; broken, missing, and shadowed states remain unchanged. The projection
  does not scan directories, write config, or mutate the catalog.

## Catalog Scan Diagnostics

- `catalog.scanAll` returns `activity.agent_summaries[]` with separate
  `roots_scanned`, `roots_partial`, and `roots_skipped` arrays. Only
  `roots_scanned` were completely enumerated and are eligible for catalog
  missing-sweep.
- Agent summary status is `completed` only when every considered existing root
  was completely enumerated. `completed-partial`,
  `completed-with-skipped-roots`, and `completed-no-roots-scanned` distinguish
  degraded outcomes; the enclosing activity is `completed-partial` when any
  adapter has a partial root.
- Missing implicit built-in and package-convention root candidates do not
  populate `roots_skipped` or `scan_issues` and do not degrade status. Missing
  explicit or manifest-declared roots, existing invalid roots, and authorized
  traversal failures retain typed diagnostics.
- Unavailable skill symlink targets are reported as `dangling_symlink` without
  making the surrounding root partial. The single link is skipped, the rest of
  the root remains eligible for exact stale-row reconciliation, and recovery
  text tells the user to confirm the source is unavailable before removing only
  that link. Resolvable targets outside the same-scope allowlist remain
  `root_outside_allowlist`.
- Native catalog completeness is exact when the enclosing activity is
  `completed` and every summary has empty `roots_partial` and `roots_skipped`.
  A `completed-no-roots-scanned` adapter is an exact empty source, and
  non-degrading issues such as rejected outside-allowlist entries do not turn
  the authorized catalog into an unknown or incomplete list.
- After an exact adapter scan, unseen rows for that agent and current project
  context transition to `missing`, including rows beneath roots that no longer
  exist or are no longer selected. A partial or skipped scan performs only
  complete-root-bounded cleanup and preserves uncertain rows.
- `scan_issues[]` contains typed `kind`, redacted `path`, and a stable
  privacy-safe `detail` field; raw filesystem error text does not cross the
  service boundary. Stable kinds are `root_unavailable`,
  `root_outside_allowlist`, `dangling_symlink`, `directory_unreadable`, `entry_unreadable`,
  `file_unreadable`, `file_too_large`, and `budget_exceeded`.
- Paths under the active home, project, project CWD, or app-data roots use
  placeholders such as `$HOME` and `<project-root>`. Explicit adapter roots
  outside those locations use `<adapter-root>`; raw private absolute paths are
  not returned in scan issue summaries or refresh logs.
- Redaction uses the declared/canonical aliases captured by the scanner. It is
  lexical, longest-path-first, and does not re-canonicalize roots while
  serializing a response, so a removed or redirected symlink cannot expose the
  former target.
- `catalog.scanClaude` returns the same single-entry `agent_summaries` contract
  as `catalog.scanAll`, including partial roots, typed issues, and recovery
  actions.
- `roots_partial` and `scan_issues` are additive summary arrays. Native clients
  decode either field as an empty array when reading a legal response from a
  pre-diagnostics service, while all pre-existing summary fields remain
  required and keep their original wire keys.

## Finding Visibility Semantics

- `catalog.listFindings`, `catalog.getStateSnapshot.findings`, and scan
  `activity.finding_count` expose retained current catalog records. These raw
  records support local audit and compatibility and are not user-visible issue
  totals.
- The derived user-visible rule-finding projection excludes suppressed records,
  reviewed/ignored triage, records without a current `instance_id`, and
  `name.collision`. It also excludes warning/information records for
  `frontmatter.tools-not-empty`, `permissions.network-declared`, and
  `permissions.exec-needs-human`; error/critical records for those rules remain
  visible. Open and `needs-follow-up` findings remain active.
- `skill.getHealth` finding counts and agent summaries, plus related findings
  included by `llm.previewPrompt`, use the derived projection. Runtime
  collisions remain available through conflict groups and conflict summaries,
  without duplicating generated `name.collision` findings.
- Native issue totals, the Issues filter, row/header counts, detail issue cards,
  and refresh feedback use the same projection and separately include
  missing/broken/unknown catalog-state problems. The independent Conflicts
  filter and conflict counts use loaded/enabled same-agent runtime conflict
  groups, with Codex plugin package namespaces applied as described above.

## Skill Manager

- `npx skills` is the first writable manager. `skills-npm` is listed as a
  registry capability, with write execution deferred to a future adapter.
- Default targets are exactly the supported app agents: `claude-code`, `pi`,
  `opencode`, `codex`, `hermes-agent`, and `openclaw`. The service never uses
  wildcard agent targeting.
- Install uses symlink distribution. The native Skill Manager does not expose a
  copy-mode choice.
- Search, install, and update may require external network access through the
  manager CLI. The native client allows those scoped operations by default;
  previews still show the destination command before any confirmed write.
- `skillManager.search` returns every row emitted by the invoked manager and
  flattens list metadata into the response. Because the current manager output
  does not advertise an authoritative total or continuation token, search uses
  `total_count=null`, `has_more=false`, `source_completeness=unknown`, and
  `incomplete_reason=source_limited`; `returned_count` is only the number of
  rows actually returned and is never presented as the source total. A
  network-blocked preview uses the same source semantics with zero rows.
- `skillManager.listInstalled` runs once per project/global scope rather than
  once per agent, consumes the CLI's complete JSON inventory, and
  reports `returned_count=total_count`, `source_completeness=enumerable`, and
  no incomplete reason. Machine-readable output is parsed in full behind a
  4 MiB fail-closed limit; only bounded diagnostic output is returned. A
  truncated, malformed, or unrecognized list fails closed with a stable error
  instead of becoming an exact empty list.
  Because the Node CLI can lose bytes when a large `console.log` exits while
  stdout is a pipe, installed JSON is captured through a private `0600`
  temporary regular file, bounded before reading, and removed on every return
  path. This prevents the observed 64 KiB pipe-buffer truncation without
  persisting inventory output.
  A row is `source_kind=manager` only when the matching scope lock file proves
  a managed source. Unlocked `.agents/skills` rows are `source_kind=local` and
  retain a redacted local source path; every row also retains a dedicated,
  redacted `path` identity so the native cache can associate the CLI row with
  its scanned canonical source without treating that source as another package.
  Appearing in `skills list` alone is not manager ownership evidence. No raw
  manager payload is included in that
  error. No pagination flags or
  tokens are invented for either command. The native client may reveal an
  already-returned search collection in 20-row steps without issuing another
  manager or network request, while installed JSON and the app-owned local
  library remain fully accessible.
- The native inventory emits one row per installed package source. A matching
  catalog row enriches the CLI row and is consumed rather than appended again.
  Catalog-only fallback rows are limited to skill sources beneath the selected
  project/global `.agents/skills` root (including nested package layouts);
  plugin caches, configured read-only roots, and other catalog discovery paths
  never become editable local-package rows.
  Installed local rows outside those guarded roots remain visible as external
  local sources, but do not expose ZIP replacement; only their manager-backed
  agent unlink/removal action remains available.
- Native clients validate method-specific metadata, not only generic page
  invariants. Search accepts only terminal unknown/source-limited metadata;
  installed accepts only terminal exact enumerable metadata. Invalid refresh
  metadata is rejected without replacing a current record for the same inputs.
  Empty and network-blocked searches still display zero loaded rows, unknown
  total, the typed source limitation and recovery guidance, with no load action.
- The Skill Manager UI does not expose agent-layer enable/disable controls.
  Skill removal is manager-backed unlink/removal from the currently selected
  agent targets, using the same explicit confirmation flow as install/update.
- Enable/disable remains in `config.toggleSkill`,
  `batch.previewSkillToggles`, and `batch.applySkillToggles` because it is
  agent config state, not manager package state.
- The native client prewarms project and global skill inventories during app
  startup. Opening the Skill Manager performs no read; its Load Data button is
  the only page-local refresh trigger. Local app-owned skills are merged into
  the same skill-centric inventory.
- Search starts with a keyword and result selection. Install/remove agent
  targets and the shared install scope are chosen only after a skill is
  selected. Manager update changes a shared source and therefore reports all
  linked supported agents instead of accepting a misleading per-agent target.
- Remote search also exposes a local ZIP import entry. Import validates one
  skill and copies it into the app-owned local library after confirmation; the
  imported skill then uses the normal project/global and agent install flow.
- Local app-owned and guarded descendant project/global `.agents/skills`
  sources use a replacement ZIP selected after the skill. The
  service requires an absolute regular archive with exactly one matching
  `SKILL.md`, rejects path traversal, symlinks, special files, oversize entries,
  and files outside the skill root, and binds apply to the archive and current
  source digest through a preview token. Imported scripts are never executed.
- When a removal selects every linked Agent target for an app-owned local
  source, the native confirmation identifies it as a full uninstall. After the
  confirmed manager unlink succeeds, the same guarded mutation flow previews
  and deletes the app-owned source; the manager removes its matching lock
  entry. Partial target removal keeps the shared source.

## Session Preview

- `session.previewLocalSessions` returns event-derived session timing when the
  local store exposes it. Each `session_rows[]` item includes `started_at` and
  `ended_at` in Unix epoch milliseconds, with `ended_at` representing the last
  parsed session message/content event. Each `content_items[]` item includes
  `timestamp` when its source event has a timestamp.
- Codex summary rows come from the guarded `state_*.sqlite` thread index used by
  current `thread/list`, represent active interactive user-owned top-level
  tasks, and apply exact cwd matching for project scope. Archived, structured
  subagent/review/compact, memory, host-created internal, and non-interactive
  `exec` carriers are excluded. Selected detail and message pages still read
  the guarded rollout on demand; the index is never treated as transcript
  content. Legacy homes without a compatible index retain the bounded rollout
  and `session_index.jsonl`/`history.jsonl` fallback.
- Agent-specific inventory filtering excludes internal conversation stores at
  discovery and metadata boundaries: Claude Code sidechains, `subagents`, tool
  results, and runtime lock state; OpenCode child sessions with a `parent_id`;
  Pi extension subagent transcripts/artifacts and context-mode state; Hermes
  cron, batch, subagent, and memory workflows; and OpenClaw cron, hook,
  heartbeat, ACP, and subagent session keys. Pi transcript branching and Hermes
  compression lineage remain part of their user-facing parent conversations.
- Current OpenClaw summaries and messages come from each agent's
  `<state>/agents/<id>/agent/openclaw-agent.sqlite`. Legacy per-agent
  `sessions.json` and JSONL transcripts are migration/archive material and do
  not enter the active list.
- `session.previewLocalSessions` supports complete, stateless summary paging.
  A first page explicitly sends `paging_mode="keyset"`; a continuation sends
  the opaque `cursor` and matching `source_revision` (and may repeat the mode).
  Cursor pages are limited to 100 rows,
  inventory every authorized root within the existing request budgets, and use
  canonical `(modified_at DESC, stable row id ASC, normalized path digest ASC)`
  order. Responses include `next_cursor`, `source_revision`,
  `source_completeness`, and optional `incomplete_reason`. A changed candidate
  inventory returns `source_changed` before continuation rows are returned.
- Cursors are opaque `v1:` values containing only stable metadata and digests;
  they never contain a raw path. Cursor pages read primary content only for the
  first `limit` candidates after the cursor boundary; rejected or empty
  candidates still consume that page's candidate window. Determining whether
  later candidates remain is metadata-only and does not open or read the next
  primary file. No cursor, inventory, summary, detail, or raw session content is
  written to app data, SQLite, or another persistent cache.
- The session `source_revision` binds the stable candidate identity set, while
  each cursor also binds the identities in its processed prefix. An already
  processed active session may grow or receive a newer modification timestamp
  without aborting later pages. Adding/removing candidates, moving an
  unprocessed candidate ahead of the cursor, or moving a processed candidate
  behind it still returns `source_changed`; this prevents silent skips or
  duplicates while allowing the current live Codex transcript to keep growing.
- Keyset requests reject `session_id`, `offset`, `max_files`, and
  `include_content_items=true` instead of silently ignoring them. They require
  all-scope recent descending summaries without server search, and bind the
  normalized agent, roots, project context, scope, sort, direction, search
  shape, excerpt bound, and content shape into the cursor query digest.
- Legacy `scope`, `search`, `sort`, `direction`, `offset`, and `max_files`
  behavior remains available when neither `paging_mode="keyset"` nor a cursor
  is sent. An unmarked summary request does not silently enter keyset mode; the
  native legacy wrapper retains `offset=0` and `max_files=800`. New paged summary
  clients explicitly select keyset, omit `offset` and `max_files`, and
  apply scope, search, and sort locally over accepted summaries.
- A legacy request with one exact non-empty `session_id` is a bounded detail
  read, not a fan-out summary scan. It may use up to a 4 MiB primary head and a
  512 KiB tail within the unchanged aggregate request budget so large Codex
  JSONL wrappers do not hide early user/tool events beyond the summary window.
  This bounded detail remains the source for sampled process items such as
  thinking, tool calls, and skill calls; it is not the completeness contract
  for user-facing conversation messages.
- `session.listLocalSessionMessages` is the read-only, selected-session
  completeness path for user messages and final Agent replies. It accepts the
  same authorized-root, agent, and project context plus one stable `session_id`.
  A continuation sends the opaque `cursor` and matching `source_revision`.
  The response contains only `user_message` and `agent_reply` content items;
  thinking, tool calls, tool results, progress events, and mirrored Codex
  `event_msg` copies are excluded. Host-injected user-role blocks such as
  recommended plugin/app/skill catalogs and runtime instruction envelopes are
  excluded from message counts, excerpts, inferred titles, and detail pages.
  Codex goal context is normalized to its user-authored `<objective>` text so a
  goal-setting message remains visible; repeated internal reinjection of the
  unchanged active goal is collapsed until the objective changes.
- Message pages default to 40 items and allow at most 100. Each request scans at
  most 32 MiB of the fixed transcript snapshot and normally returns at most
  2 MiB of message text. A single larger final message is returned intact on
  its own page rather than truncated or omitted. A page that crosses only a
  large non-message record may return zero items with `has_more=true`; its
  cursor must still advance. This keeps sidecar work bounded while allowing the
  native app to publish accepted messages between pages and yield to UI work.
- The first message page fixes `snapshot_bytes`. Appends to a live Codex JSONL
  are allowed during continuation but are excluded until the user refreshes;
  shrink, replacement, or detected fixed-snapshot drift returns
  `source_changed`. `total_count` is `null` until EOF and exact on the terminal
  page. Responses also report `scanned_bytes`, `scanned_through_bytes`, and
  `snapshot_bytes` for local progress without exposing a raw path.
- The native detail loader first obtains the bounded process sample, then
  automatically consumes message pages to EOF and publishes every accepted
  page. Cancellation and page failure retain already accepted messages and
  expose retry. The detail view uses lazy rows and defaults its selected
  filters to User and Agent Reply; Thinking, Tool, and Skill remain available
  but unselected. Exact final-message counts replace the bounded preview counts
  as pages arrive. Neither pages nor merged detail are persisted.
- `sort` accepts `recent`, `modified_at`, and `title`. `direction` accepts
  `asc` and `desc`; recent/modified time defaults descending and title defaults
  ascending.
- `max_files` selects the newest metadata candidates before any primary session
  content read. `total_candidate_count` is the number discovered within the
  bounded inventory, while `total_matched_count`, `has_more`, and `next_offset`
  describe the selected, filtered candidate set after sorting.
- `candidate_set_truncated=true` means additional disk candidates or required
  enrichment were omitted by `max_files`, inventory directory/entry limits,
  sidecar file/count/byte bounds, or aggregate request bytes. Deliberately
  retaining a bounded head/tail view of one large primary session file does not
  by itself make the candidate inventory incomplete; a genuinely insufficient
  request-level primary-read budget still reports the typed limitation. Keyset
  inventory de-duplicates overlapping roots by stable row identity before
  revision, total, order, and continuation metadata are calculated.
- Any keyset inventory or read-budget stop retains accepted rows and reports
  `candidate_set_truncated=true`, `source_completeness=limited`, and
  `incomplete_reason=safety_budget`; it is terminal and does not offer an unsafe
  continuation. An enumerable page processes at most `limit` candidates. If all
  candidates in that window are rejected or empty but metadata shows later
  candidates, the page may contain zero accepted rows and still return
  `has_more=true`; its cursor advances to the last processed candidate. Clients
  accept that zero-row continuation only when the cursor advances and reject a
  repeated cursor as no progress. While rejected candidates are being excluded,
  `total_matched_count` may decrease from an earlier candidate upper bound; at
  EOF it is the exact accepted total.
- `include_content_items` defaults to `true` when omitted for compatibility.
  Summary/list clients send `false`; every returned row then has
  `content_included=false` and `content_items=[]` while retaining bounded title,
  excerpt, timing, and aggregate counts. Detail clients send `true` and receive
  `content_included=true`.
- `session_id` is the stable service row ID derived from the candidate path,
  not an agent-native transcript ID. The service applies this filter immediately
  after metadata inventory, before newest-candidate selection and before opening
  any primary session file. A detail request sends one `session_id`,
  `include_content_items=true`, `limit=1`, and `offset=0`.
- The native app requests 100 summaries at a time and prewarms serially to at
  most 800 accepted summaries. The 800-row mark is only the automatic startup
  boundary: explicit Load More and Load All continue with the snapshot's
  in-memory cursor/revision. Every accepted prewarm page is published before
  the next request; a later page failure retains those rows and retries from the
  last accepted cursor, while an initial failure retries from a nil cursor.
  A zero-row nonterminal page may continue when its cursor advances; a repeated
  cursor is rejected before another request. Session accumulation adopts each
  page's current matched total, clamped to the unique rows already loaded, so a
  decreasing total reaches exact `loaded == total` completeness at EOF.
  Cancellation or a source-key/generation change retains accepted rows and
  rejects late successes and errors. Scope, search, sort, and
  global search project all loaded summaries in memory. At most one selected
  row's bounded process sample and progressively paged final messages are held
  in the bounded in-memory detail cache; neither summaries nor details persist
  raw session content.
- When a session store has no parseable event timestamp, the service falls back
  to the redacted read-only file metadata timestamp for row-level timing only.
- The callable inventory does not yet expose a resume-command preview. Clients
  must not infer a command from session metadata. `session.previewResume`
  becomes supported only when it is added to the Methods table and fixtures
  under the product implementation sequence.

## App Search

- `app.search` is a read-only, local-only search across catalog skills,
  app-loaded config snapshots, and local sessions. It accepts `query`, optional
  `agent`, optional `limit_per_kind`, local-session roots/discovery settings,
  and project context.
- Results are grouped by `kind` (`skill`, `session`, or `config_history`) and
  include `target_id` plus an embedded record when available. UI shells should
  use the embedded record to insert the result into the corresponding list page,
  select it, and show its detail even when that item was not present in the
  currently loaded frontend page.
- Result subtitles carry stable disambiguation context: skill Agent/scope and
  package provenance, session Agent/project/time, and config Agent/scope/target
  time. Search remains local and does not read skill files on each keystroke.
- Optional semantic reranking must use a separate, explicitly previewed and
  confirmed LLM action over the already-returned bounded candidates.
  `app.search` never calls a provider or changes its lexical result contract.

## LLM Prompt Actions

- `llm.previewPrompt` action `task_cockpit` accepts `agents: string[]` plus
  `instance_ids: string[]` and `user_intent`/`task_text`. The service renders a
  redacted task preflight prompt from selected agent names, adapter capability
  summaries, and current effective skill names/descriptions only. Raw skill
  bodies, frontmatter, config contents, paths, credentials, raw prompts, raw
  responses, traces, writes, scripts, snapshots, and rollback commands are
  excluded.
- Native UI may present `task_cockpit` as inline Task Readiness, but it must not
  change the wire action or imply universal readiness without a task.
- Task Preflight first ranks cached effective skills against the task, includes
  at most 24 candidates, and blocks confirmation when the estimated request
  exceeds the 12,000-token safety budget (in addition to the provider profile
  limit).
- A successful HTTP transport is not sufficient for a successful Task
  Preflight. The provider output must be valid JSON with the required business
  result sections; otherwise prompt-run and provider-call metadata use
  `parse_failed` with `response_schema_invalid`. Provider metadata timestamps
  are epoch milliseconds; plausible legacy epoch-second records are normalized
  on read.

## Environment Overrides

| Variable | Purpose |
| --- | --- |
| `SKILLS_COPILOT_APP_DATA_DIR` | Override app data/catalog directory for tests and screenshots |
| `SKILLS_COPILOT_HOME` | Override user home used by adapters |
| `SKILLS_COPILOT_PROJECT_CWD` | Provide current project working directory |
| `SKILLS_COPILOT_PROJECT_ROOT` | Provide project safety root |
| `SKILLS_COPILOT_SERVICE_PATH` | Override sidecar path for local debugging |
| `CODEX_HOME` | Override Codex user config home when safe for the active context |

## Fixtures

Protocol fixtures live under `fixtures/service-protocol/`. Each supported method
must have dispatch coverage, status fixture coverage, and request/response
fixture coverage where applicable.
