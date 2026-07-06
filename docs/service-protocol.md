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

## Methods

| Method | Mutates local state |
| --- | --- |
| `app.version` | No |
| `app.stateSnapshot` | No |
| `app.search` | No |
| `service.status` | No |
| `adapter.listCapabilities` | No |
| `adapter.listDiagnostics` | No |
| `llm.status` | No |
| `llm.listProviderProfiles` | No |
| `llm.saveProviderProfile` | Yes, writes Agent Copilot provider metadata and Keychain secret when supplied |
| `llm.deleteProviderProfile` | Yes, updates app-local Agent Copilot provider metadata |
| `llm.testProviderConnection` | Yes, records minimal redacted Agent Copilot provider call metadata |
| `llm.previewPrompt` | No |
| `llm.confirmPromptAndSend` | Yes, records redacted Agent Copilot prompt-run metadata |
| `llm.listPromptRuns` | No |
| `llm.prepareAction` | No |
| `llm.providerObservability` | No |
| `llm.listModelTaskMatches` | No |
| `llm.recordModelTaskMatch` | Yes, writes app-local redacted metadata only |
| `llm.deleteModelTaskMatch` | Yes, updates app-local redacted metadata only |
| `script.previewExecution` | No |
| `script.execute` | No |
| `skillManager.listTools` | No |
| `skillManager.search` | No, may run a network-backed manager search only when allowed by the request |
| `skillManager.listInstalled` | No, reads external manager state |
| `skillManager.previewInstall` | No |
| `skillManager.applyInstall` | Yes, after confirmation through the external manager CLI and catalog refresh |
| `skillManager.previewRemove` | No |
| `skillManager.applyRemove` | Yes, after confirmation through the external manager CLI and catalog refresh |
| `skillManager.previewUpdate` | No |
| `skillManager.applyUpdate` | Yes, after confirmation through the external manager CLI and catalog refresh |
| `skillManager.previewLocalCreate` | No |
| `skillManager.applyLocalCreate` | Yes, creates a manager template and imports it into app-owned local library |
| `skillManager.deleteLocal` | Yes, physically deletes only app-owned local skills with no supported-agent references |
| `project.getContext` | No |
| `project.setContext` | Yes, writes app state |
| `project.clearContext` | Yes, writes app state |
| `project.validateContext` | No |
| `catalog.listSkills` | No |
| `catalog.getSkill` | No |
| `catalog.analysis` | No |
| `catalog.listFindings` | No |
| `catalog.listFindingTriage` | No |
| `catalog.setFindingTriage` | Yes, writes app-local triage metadata only |
| `catalog.clearFindingTriage` | Yes, clears app-local triage metadata only |
| `catalog.listConflicts` | No |
| `catalog.importSkill` | Yes, writes app-controlled staging/catalog only |
| `catalog.scanAll` | Yes, refreshes catalog |
| `catalog.scanClaude` | Yes, refreshes catalog |
| `skill.exportBundle` | Yes, writes app-controlled export files |
| `skill.install` | Yes, after confirmation |
| `skill.listEvents` | No |
| `config.toggleSkill` | Yes, writes agent config |
| `config.readAgentConfig` | No |
| `config.readClaudeSettings` | No |
| `config.saveClaudeSettings` | Yes, writes Claude settings and rescans |
| `snapshot.list` | No |
| `snapshot.listAgentConfig` | No |
| `snapshot.previewRollback` | No |
| `snapshot.rollback` | Yes, writes agent config snapshot content and rescans |
| `session.previewLocalSessions` | No |
| `rules.listTuning` | No |
| `rules.setSeverityOverride` | Yes, writes app-local rule tuning metadata only |
| `rules.clearSeverityOverride` | Yes, clears app-local rule tuning metadata only |
| `rules.setSuppression` | Yes, writes app-local rule tuning metadata only |
| `rules.clearSuppression` | Yes, clears app-local rule tuning metadata only |
| `batch.previewSkillToggles` | No |
| `batch.applySkillToggles` | Yes, writes through verified per-agent toggle paths after confirmation |

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
