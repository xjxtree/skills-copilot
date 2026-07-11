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
| `llm.status` | None | Never | Never | None |
| `llm.listProviderProfiles` | None | Never | Never | None |
| `llm.saveProviderProfile` | App-local data, Keychain | Never | Never | None |
| `llm.deleteProviderProfile` | App-local data, Keychain | Never | Never | None |
| `llm.testProviderConnection` | App-local data | Never | Always | Required |
| `llm.previewPrompt` | None | Never | Never | None |
| `llm.confirmPromptAndSend` | App-local data | Never | Always | Required |
| `llm.listPromptRuns` | None | Never | Never | None |
| `llm.providerObservability` | None | Never | Never | None |
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
| `project.getContext` | None | Never | Never | None |
| `project.setContext` | App-local data | Never | Never | None |
| `project.clearContext` | App-local data | Never | Never | None |
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
| `config.toggleSkill` | Agent config, App-local data | Never | Never | None |
| `config.readAgentConfig` | None | Never | Never | None |
| `config.readClaudeSettings` | None | Never | Never | None |
| `config.saveClaudeSettings` | Agent config, App-local data | Never | Never | None |
| `snapshot.list` | None | Never | Never | None |
| `snapshot.listAgentConfig` | None | Never | Never | None |
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

## Full-Access Local Lists

- `llm.listPromptRuns` and `llm.listModelTaskMatches` do not apply a default
  row limit. Omit `limit` when the UI needs the full local list. Passing
  `limit` explicitly requests a bounded page/preview.
- Bounded responses expose returned-vs-total metadata where the protocol
  supports it (`total_count` / `returned_count` or
  `total_*_count` / `returned_*_count`, plus `truncated`) so clients do not
  mistake a limited page for full history.

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
- `scan_issues[]` contains typed `kind`, redacted `path`, and a stable
  privacy-safe `detail` field; raw filesystem error text does not cross the
  service boundary. Stable kinds are `root_unavailable`,
  `root_outside_allowlist`, `directory_unreadable`, `entry_unreadable`,
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

## Skill Manager

- `npx skills` is the first writable manager. `skills-npm` is listed as a
  registry capability, with write execution deferred to a future adapter.
- Default targets are exactly the supported app agents: `claude-code`, `pi`,
  `opencode`, `codex`, `hermes-agent`, and `openclaw`. The service never uses
  wildcard agent targeting.
- Install defaults to symlink distribution. `--copy` is sent only when the user
  explicitly selects copy.
- Search, install, and update may require external network access through the
  manager CLI. Requests must carry `network_allowed`; previews show whether a
  command will run.
- The Skill Manager UI does not expose agent-layer enable/disable controls.
  Skill removal is manager-backed unlink/removal from the currently selected
  agent targets, using the same explicit confirmation flow as install/update.
- Enable/disable remains in `config.toggleSkill`,
  `batch.previewSkillToggles`, and `batch.applySkillToggles` because it is
  agent config state, not manager package state.

## Session Preview

- `session.previewLocalSessions` returns event-derived session timing when the
  local store exposes it. Each `session_rows[]` item includes `started_at` and
  `ended_at` in Unix epoch milliseconds, with `ended_at` representing the last
  parsed session message/content event. Each `content_items[]` item includes
  `timestamp` when its source event has a timestamp.
- `session.previewLocalSessions` supports server-side `scope`, `search`,
  `sort`, `direction`, `limit`, and `offset`. Responses include
  `total_matched_count`, `has_more`, and `next_offset`; UI shells should request
  additional pages instead of treating the first page as the full local session
  list.
- `sort` accepts `recent`, `modified_at`, and `title`. `direction` accepts
  `asc` and `desc`; recent/modified time defaults descending and title defaults
  ascending.
- `max_files` selects the newest metadata candidates before any primary session
  content read. `total_candidate_count` is the number discovered within the
  bounded inventory, while `total_matched_count`, `has_more`, and `next_offset`
  describe the selected, filtered candidate set after sorting.
- `candidate_set_truncated=true` means additional disk candidates were omitted
  by `max_files` or the request-owned inventory limits. A false value does not
  weaken per-file, sidecar, or aggregate read bounds.
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
- The native app prewarms and manually refreshes source-scoped summary snapshots
  only. Scope, search, sort, and global search project those summaries in memory.
  At most a selected row's bounded detail is held in the in-memory detail cache;
  neither summaries nor details persist raw session content.
- When a session store has no parseable event timestamp, the service falls back
  to the redacted read-only file metadata timestamp for row-level timing only.

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

## LLM Prompt Actions

- `llm.previewPrompt` action `task_cockpit` accepts `agents: string[]` plus
  `instance_ids: string[]` and `user_intent`/`task_text`. The service renders a
  redacted task preflight prompt from selected agent names, adapter capability
  summaries, and current effective skill names/descriptions only. Raw skill
  bodies, frontmatter, config contents, paths, credentials, raw prompts, raw
  responses, traces, writes, scripts, snapshots, and rollback commands are
  excluded.

## Environment Overrides

| Variable | Purpose |
| --- | --- |
| `SKILLS_COPILOT_APP_DATA_DIR` | Override app data/catalog directory for tests and screenshots |
| `SKILLS_COPILOT_HOME` | Override user home used by adapters |
| `SKILLS_COPILOT_PROJECT_CWD` | Provide current project working directory |
| `SKILLS_COPILOT_PROJECT_ROOT` | Provide project safety root |
| `SKILLS_COPILOT_CLAUDE_EXTRA_ROOTS` | Add fixture Claude skill roots |
| `SKILLS_COPILOT_SERVICE_PATH` | Override sidecar path for local debugging |
| `CODEX_HOME` | Override Codex user config home when safe for the active context |

## Fixtures

Protocol fixtures live under `fixtures/service-protocol/`. Each supported method
must have dispatch coverage, status fixture coverage, and request/response
fixture coverage where applicable.
