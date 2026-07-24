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
- Skill Manager may invoke supported external manager CLIs for confirmed
  search, remote-repository install, remove, update, and local template creation
  when the request
  exposes command preview, target agents, network posture, telemetry-off env,
  and confirmation state. Calls must use argv arrays, not shell strings.
- App-local metadata writes must be redacted.
- Adapter config writes must use the guarded paths documented in
  `docs/adapters/agent-adapters.md`.
- Service method changes must update fixtures and pass
  `pnpm verify:service-protocol-drift`.

## Typed Action Lifecycle

The typed lifecycle is an allowlisted authorization contract, not a convention
that clients may infer for arbitrary mutations.

- A supported preview returns `action`, `preconditions`, and an opaque
  `preview_token`. `action.id` is deterministic for kind, intent, target, and
  project; `action.source_revision` identifies the accepted snapshot.
- The token is an HMAC over the complete descriptor and sorted preconditions.
  It additionally binds impacts, network posture, read-back domains, and
  evidence references. It is opaque client state and must not be displayed,
  copied, logged, persisted, placed in accessibility metadata, or returned by
  an apply response after consumption.
- The macOS app generates one 32-byte random secret for its lifetime and passes
  it only in each sidecar child's environment. The sidecar consumes and removes
  it before reading stdin and explicitly removes it from external manager child
  environments. A missing or malformed secret returns
  `action_token_unavailable`; there is no production fallback.
- Apply accepts the exact preview reference plus token and explicit
  confirmation. `batch.applySkillToggles` uses `confirmation`;
  `skill.install`, provider profile actions, provider connection tests, and LLM
  prompt sends use `action_confirmation`; mutating `skillManager.*` methods use
  `action_reference`, `preview_token`, and `confirmed=true`.
- Before the first product, target, process, catalog, snapshot, or audit
  effect, apply reprojects the action, checks its method and intent ownership,
  locks every target, and revalidates each accepted revision. Authorized
  non-manager app-data initialization walks directory components with
  no-follow opens, creates missing components privately, and locks the final
  opened owner descriptor before catalog creation or migration. Final and
  intermediate symlinks fail closed without changing the linked target's mode,
  contents, or children. The confirmed fresh-filesystem manager owner
  bootstrap described under Skill Manager remains the only case where a stale
  race detected after a successful non-creating preflight may retain an
  otherwise empty owner directory. Success returns a typed `readback` covering
  every domain declared by the action. Stable failures are
  `unknown_action_reference`,
  `stale_action_reference`, `action_target_mismatch`,
  `confirmation_required`, `action_token_unavailable`, and
  `verification_failed`. Raw manager local-source input is rejected earlier as
  `local_source_requires_guarded_import`. `verification_failed` means
  execution completed but the
  required semantic postcondition could not be proved; it is never reported as
  a successful apply. A mutation that may have crossed its external effect
  boundary returns `partial_effect` with structured `details` containing
  `operation`, `state`, `cleanup_required`, and `retry_allowed=false`.
  `state` distinguishes `not_started`, `applied_verified`,
  `applied_unverified`, and `outcome_unknown`; clients must not automatically
  retry any such response.
- Service methods whose effect row declares no local writes open only a
  current-schema read-only catalog. An outdated catalog fails closed without
  migration or byte changes; explicit `catalog.scan*` and already-confirmed
  apply paths retain the writable migration boundary.
- The action-reference contract currently covers single and multi-skill UI
  toggles through `batch.*`, `skill.install`, confirmed Skill Manager search
  and remote-repository-install/remove/update/local-create/local-archive-import/
  local-archive-update, eligible app-owned local deletion, explicit Claude
  settings saves, config snapshot rollback, provider profile save/delete,
  provider connection tests, and confirmed LLM prompt sends.
  It also covers explicit cleanup of legacy AI/private-content files through
  `privacy.previewCleanupLegacyContent` and
  `privacy.cleanupLegacyContent`.
  Other callable mutations retain their documented
  method-specific consistency contracts until they explicitly adopt these
  fields.
- `fixtures/service-protocol/method-effects.json` schema version 2 is the
  machine-readable inventory for every supported method's effects, blocked
  compatibility methods, production Swift RPC bindings, and every effectful
  action lifecycle. Each lifecycle declares its preview method, allowed
  kind/intent/target kind, accepted precondition kinds, agent/scope/project
  binding rule, network posture, impacts, and read-back domains. Protocol drift
  verification rejects undeclared mutations, dynamic Swift dispatch,
  `unknown_method` fallback branches, or a fixture that does not bind these
  fields exactly.
- Confirmation material is request-only authority. A successful apply response
  may return the original action descriptor for correlation and must return its
  verified action-bound read-back, but must not echo `preview_token`,
  `action_confirmation`, `action_reference`, or an equivalent action-reference
  authorization envelope anywhere in the result. Human-readable confirmation
  metadata without a token or action reference is not authorization and may be
  returned for UI explanation. Native clients compare any returned action with
  the reviewed preview and validate the response using that original preview
  before publishing success.

`script.execute`, `catalog.importSkill`, and `skill.exportBundle` are
compatibility-only blocked methods. Each returns `mutation_disabled` before
parameter-dependent I/O; their zero-write rows in the method table are not an
alternative apply path.

## Product Read Projections

The additive product read methods expose Rust-owned projections without
removing lower-level catalog, project, session, search, or config contracts.
They are independently prewarmed after the project context is accepted;
`app.stateSnapshot` does not duplicate them. `app.search` remains deterministic
and lexical and never hides a provider call. `task_cockpit` remains the
compatible LLM action identifier even when a native UI labels the experience
Task Readiness.

- `project.getReadiness` requires `project_id` and
  `expected_project_context_revision`; optional `source_revision` binds a
  previously accepted projection. It returns one validated
  `ProjectReadinessRecord`. Projection input has fixed local safety bounds;
  reaching one returns typed limited coverage instead of an invented complete
  total.
- `catalog.listSkillAggregates` requires the same project and context binding.
  Optional `agent` filters aggregates that include one of the six project
  agents while preserving each aggregate's complete cross-agent instances.
  `limit` is clamped to 1 through 100. Optional `cursor` and
  `source_revision` bind keyset paging to the same project, agent filter, and
  accepted projection. The response contains `source_revision`, `coverage`,
  `page` metadata, and `aggregates`.
- `session.previewResume` revalidates the same bounded local session inventory
  used by startup prewarm. The request requires `authorized_roots`, one
  supported `agent`, `auto_discover`, `project_root`, `current_cwd`,
  `session_id`, `expected_source_revision`, and
  `expected_snapshot_revision`. These fields preserve the accepted discovery
  and project context. The response is one `SessionContinuationRecord`. A
  supported result obtains its copy-only command exclusively from
  `resume.argv`; an unsupported result has no argv and carries a typed reason.
  The service never launches a terminal or process.

All three methods are bounded, local-only, read-only, process-free, and
network-free. `project_context_required`, `project_context_mismatch`,
`stale_project_context`, `source_changed`, `session_not_found`, and
`invalid_request` are stable errors for their applicable context, revision,
identity, and parameter failures.

## Config Consistency

Protocol version 2 puts direct Claude settings saves and snapshot rollback on
the typed action lifecycle and binds both to the exact local state reviewed by
the client.

- `config.readClaudeSettings` and every row from `config.readAgentConfig`
  include an opaque tagged `revision`. The revision is `sha256:` plus a
  domain-separated digest of either `present\0` and the exact file bytes or
  `missing\0`. A missing file is therefore distinct from an existing empty
  file, while UI-only default content does not change the missing-file
  revision.
- `config.previewSaveClaudeSettings` accepts `content` and
  `expected_revision`. It validates JSON and the authorized target, then
  returns `action`, sorted `preconditions`, an opaque HMAC `preview_token`, the
  current document, the candidate-content digest, and whether bytes differ.
  It is read-only and creates no app-data directory, catalog, lock artifact,
  snapshot, config parent, or target.
- `config.saveClaudeSettings` accepts the exact `content` plus
  `confirmation`. The confirmation contains the preview's action reference,
  token, and `confirmed=true`; an expected revision alone is not
  authorization. The service completes a non-creating preflight before opening
  a writable catalog. A stale preflight returns `config_conflict` or
  `stale_action_reference` with no new filesystem or catalog artifacts.
- A valid save may initialize the app catalog, then acquires the shared
  cross-process mutation owner lock on the already existing app-data directory.
  Initialization itself performs a component-wise no-follow create-and-lock on
  the final owner descriptor before opening or migrating SQLite. Under the
  subsequently reacquired existing-owner lock, apply revalidates the target and
  complete action binding, begins an SQLite `IMMEDIATE` transaction, records
  the safety snapshot, atomically writes the config without a target `.lock`
  file, semantically reads back the config and snapshot, commits the
  transaction, and finally releases the owner lock.
- `snapshot.previewRollback` returns the same typed action fields plus the
  snapshot-content digest and current target revision. Its token binds snapshot
  identity and content, agent, scope, project, target, and current config
  revision. `snapshot.rollback` accepts `snapshot_id` plus the exact typed
  `confirmation`; a bare revision or token is insufficient.
- Rollback validates the confirmation against an existing read-only catalog
  before any writable open. A valid apply takes the same mutation owner lock,
  begins an SQLite `IMMEDIATE` transaction, reloads the snapshot and target,
  and revalidates all preconditions before the atomic write and semantic
  read-back. Deleted, replaced, retargeted, or otherwise drifted inputs return
  `stale_action_reference` or `action_target_mismatch` without a target write.
- A failed save or rollback first rolls back its open catalog transaction. File
  compensation occurs only when the target is still the exact candidate just
  written. It restores the exact prior state, including absence, and removes
  only empty ancestor directories created by that write. If the target is a
  third state, cannot be read, or compensation fails, the service never
  overwrites it and returns `partial_effect`; the outcome requires inspection
  and must not be retried automatically.
- Clients must discard a consumed or stale confirmation, load the latest
  config, and require another preview. The native editor never autosaves. After
  verified apply it publishes the returned config document and refreshes only
  config snapshots; failure of that later timeline refresh cannot reclassify
  the already verified write. The compatibility-only `config.toggleSkill`
  method returns `mutation_disabled`; native single and batch toggles use the
  typed `batch.*` lifecycle.

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
| `session.previewResume` | None | Never | Never | None |
| `llm.status` | None | Never | Never | None |
| `llm.listProviderProfiles` | None | Never | Never | None |
| `llm.previewSaveProviderProfile` | None | Never | Never | None |
| `llm.saveProviderProfile` | App-local data, Keychain | Never | Never | Required |
| `llm.previewDeleteProviderProfile` | None | Never | Never | None |
| `llm.deleteProviderProfile` | App-local data, Keychain | Never | Never | Required |
| `llm.previewProviderConnectionTest` | None | Never | Never | None |
| `llm.testProviderConnection` | App-local data | Never | Always | Required |
| `llm.previewPrompt` | None | Never | Never | None |
| `llm.confirmPromptAndSend` | App-local data | Never | Always | Required |
| `llm.listPromptRuns` | None | Never | Never | None |
| `llm.providerObservability` | None | Never | Never | None |
| `llm.listProviderActivity` | None | Never | Never | None |
| `llm.listModelTaskMatches` | None | Never | Never | None |
| `llm.recordModelTaskMatch` | None | Never | Never | None |
| `llm.deleteModelTaskMatch` | None | Never | Never | None |
| `llm.prepareAction` | None | Never | Never | None |
| `privacy.inspectLegacyContent` | None | Never | Never | None |
| `privacy.previewCleanupLegacyContent` | None | Never | Never | None |
| `privacy.cleanupLegacyContent` | App-local data | Never | Never | Required |
| `rules.listTuning` | None | Never | Never | None |
| `rules.setSeverityOverride` | None | Never | Never | None |
| `rules.clearSeverityOverride` | None | Never | Never | None |
| `rules.setSuppression` | None | Never | Never | None |
| `rules.clearSuppression` | None | Never | Never | None |
| `batch.previewSkillToggles` | None | Never | Never | None |
| `batch.applySkillToggles` | Agent config, App-local data | Never | Never | Required |
| `script.previewExecution` | None | Never | Never | None |
| `script.execute` | None | Never | Never | None |
| `skillManager.listTools` | None | Never | Never | None |
| `skillManager.search` | None | Never | Never | None |
| `skillManager.applySearch` | App-local data, External manager state may change when invoked | Always | Always | Required |
| `skillManager.listInstalled` | None | Never | Never | None |
| `skillManager.previewInstall` | None | Never | Never | None |
| `skillManager.applyInstall` | Agent skill files, App-local data, External manager state may change when invoked | Always | Always | Required |
| `skillManager.previewRemove` | None | Never | Never | None |
| `skillManager.applyRemove` | Agent skill files, App-local data, External manager state may change when invoked | Always | Always | Required |
| `skillManager.previewUpdate` | None | Never | Never | None |
| `skillManager.applyUpdate` | Agent skill files, App-local data, External manager state may change when invoked | Always | Always | Required |
| `skillManager.previewLocalCreate` | None | Never | Never | None |
| `skillManager.applyLocalCreate` | Agent skill files, App-local data | Always | Always | Required |
| `skillManager.deleteLocal` | App-local data | Never | Never | Required |
| `skillManager.previewLocalArchiveImport` | None | Never | Never | None |
| `skillManager.applyLocalArchiveImport` | App-local data | Never | Never | Required |
| `skillManager.previewLocalArchiveUpdate` | None | Never | Never | None |
| `skillManager.applyLocalArchiveUpdate` | Agent skill files, App-local data | Never | Never | Required |
| `project.getContext` | None | Never | Never | None |
| `project.getReadiness` | None | Never | Never | None |
| `project.previewSetContext` | None | Never | Never | None |
| `project.setContext` | App-local data | Never | Never | Required |
| `project.previewClearContext` | None | Never | Never | None |
| `project.clearContext` | App-local data | Never | Never | Required |
| `project.previewRemoveRecentContext` | None | Never | Never | None |
| `project.removeRecentContext` | App-local data | Never | Never | Required |
| `project.previewClearRecentContexts` | None | Never | Never | None |
| `project.clearRecentContexts` | App-local data | Never | Never | Required |
| `project.validateContext` | None | Never | Never | None |
| `catalog.listSkills` | None | Never | Never | None |
| `catalog.listSkillAggregates` | None | Never | Never | None |
| `catalog.getSkill` | None | Never | Never | None |
| `catalog.analysis` | None | Never | Never | None |
| `catalog.listFindings` | None | Never | Never | None |
| `catalog.listFindingTriage` | None | Never | Never | None |
| `catalog.setFindingTriage` | None | Never | Never | None |
| `catalog.clearFindingTriage` | None | Never | Never | None |
| `catalog.listConflicts` | None | Never | Never | None |
| `catalog.importSkill` | None | Never | Never | None |
| `catalog.scanClaude` | App-local data | Never | Never | None |
| `catalog.scanAll` | App-local data | Never | Never | None |
| `skill.exportBundle` | None | Never | Never | None |
| `skill.install` | Agent skill files, App-local data | Never | Never | Required |
| `skill.listEvents` | None | Never | Never | None |
| `skill.listEventsPage` | None | Never | Never | None |
| `config.toggleSkill` | None | Never | Never | None |
| `config.readAgentConfig` | None | Never | Never | None |
| `config.readClaudeSettings` | None | Never | Never | None |
| `config.previewSaveClaudeSettings` | None | Never | Never | None |
| `config.saveClaudeSettings` | Agent config, App-local data | Never | Never | Required |
| `snapshot.list` | None | Never | Never | None |
| `snapshot.listAgentConfig` | None | Never | Never | None |
| `snapshot.listAgentConfigPage` | None | Never | Never | None |
| `snapshot.previewRollback` | None | Never | Never | None |
| `snapshot.rollback` | Agent config, App-local data | Never | Never | Required |

## Disabled Compatibility Mutations And Script Preview

`llm.recordModelTaskMatch`, `llm.deleteModelTaskMatch`, the four `rules.*`
mutation methods, and the two finding-triage mutation methods remain recognized
wire identifiers but return `mutation_disabled` before parsing targets or
opening app data. They have no guarded product action and therefore perform no
I/O. Their corresponding list methods remain read-only.

`script.previewExecution` accepts either an explicit command preview request or
the native client's skill identity (`instance_id`, `definition_id`, `agent`).
For identity-only requests, the service returns a deterministic blocked preview
with empty `command_preview.argv` and an unavailable reason. It never infers a
command from skill metadata, reads a script to construct one, writes an audit,
or spawns a process. `script.execute` still requires explicit confirmation and
a non-empty command argv; confirmed attempts are audited as blocked and never
spawn a process.

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

## Provider Profile And Request Actions

Provider settings do not autosave. The native client first presents the typed
preview and sends an apply request only after explicit confirmation:

| Preview | Apply |
| --- | --- |
| `llm.previewSaveProviderProfile` | `llm.saveProviderProfile` |
| `llm.previewDeleteProviderProfile` | `llm.deleteProviderProfile` |
| `llm.previewProviderConnectionTest` | `llm.testProviderConnection` |
| `llm.previewPrompt` | `llm.confirmPromptAndSend` |

Each apply carries `action_confirmation` with the exact action reference,
opaque preview token, and `confirmed=true`. Provider previews bind the current
profile-store revision and bounded replay-state revision. Save previews also
bind normalized non-secret input; when an API key is supplied, an
authorization-keyed opaque secret binding distinguishes different submitted
secrets without serializing, displaying, logging, or persisting either the raw
secret or a reusable digest.

After confirmation is accepted under the provider lock, the service writes a
one-time reservation to private `provider-action-state.json`. This is a bounded
single replay-state record, not action history: it contains only a monotonic
generation, token digest, action id, source revision, phase, state, and
timestamp. Atomic `0600` replacement changes `reservation/not_started` to one
`outcome/not_started|verified|partial`, and the next action replaces the same
record at a higher generation. Replacement uses one fixed controlled path under
the provider lock, opens or creates its private parent component-wise with
no-follow semantics, removes bounded legacy crash residue, and verifies the
parent-directory sync after rename. A symlinked owner or intermediate component
is rejected before any provider effect and cannot chmod or populate its target.
A failed post-rename durability check is `applied_unverified`; an atomic-write
error followed by candidate read-back remains `applied_unverified` because
read-back cannot prove directory durability. Candidate, accepted-original, and
successful read-back classifications require the exact content-and-storage
snapshot; equal bytes on a different inode are `outcome_unknown`. An unchanged
reservation is `action_not_started`, while an unchanged terminal record after
a provider effect is partial. Malformed, oversized, symlinked, non-regular,
broadly permissioned, or unbounded state storage fails closed.
Every consumed token is rejected on replay. The native client clears the
pending token after every confirmed attempt and requires a new preview for
recovery. Stale or mismatched confirmations are rejected before the lock or
replay-state file can create an app-data directory.

Provider profile saves stage and verify credential replacement in Keychain
before committing the profile. If the profile commit fails, the service
restores the previous credential and verifies the restoration. Delete and save
responses classify unverified local or credential effects as `partial` with
`effect=applied_unverified`; callers must reload provider state and must not
retry automatically. The bounded profile-store candidate is checked before
credential staging. An unverified store outcome keeps the verified target
credential and removes its duplicate staging copy.

Connection tests and prompt sends classify all failures that occur after a
request may have left the process as `partial` with
`remote_effect=remote_unknown`. This includes response-body reads, response
JSON/schema parsing, and post-request metadata persistence. Provider HTTP
clients do not follow redirects, so credentials are sent only to the exact
previewed destination and are never forwarded to another host. A verified
provider apply returns typed `readback` observations for every domain declared
in the action: provider profiles, provider credentials, provider activity,
and/or prompt runs as applicable. Credential read-back is a semantic Keychain
presence/value or absence verification, not secret exposure.

App-owned provider files are read only through the locked owner descriptor.
On Unix every file must be regular, owned by the app-data owner UID, have one
link, fit its byte bound, and grant no group/other permission bits; `0600` and
owner-only read modes such as `0400` are accepted. Every traversed nested
directory must have the same owner and no group/other permission bits. Reads
fail closed without chmod. Bounded residue cleanup atomically moves each name
to an unpredictable same-directory no-replace quarantine, verifies the moved
device/inode identity, and only then unlinks it. If a raced replacement was
moved, it is restored only by an atomic no-replace rename while the original
name remains absent; otherwise both names are retained. Any failure after a
removal or uncertain move, including directory sync failure, is
`partial_effect` with `state=applied_unverified` and `cleanup_required=true`.
On macOS the final quarantine identity recheck and path-based `unlinkat` cannot
be combined into one identity-conditioned syscall. Random no-replace quarantine
names and complete metadata rechecks substantially narrow that interval, and
detected drift is retained as a partial outcome, but the contract does not
claim atomic protection from an actively adversarial same-UID replacement in
the final check-to-unlink gap.

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

## Legacy Private-Content Cleanup

`privacy.inspectLegacyContent` is the startup-safe, read-only projection for
legacy Agent Copilot AI content. It opens only an existing app-data owner and
does not create directories, replay state, locks on disk, or cleanup residue.
It returns source names, file kinds, cleanup operations, and counts, but never
returns raw file bytes, task text, prompt text, provider output, or paths.

The cleanup inventory is limited to three historical sources:

- valid `prompt-runs.json` is preserved as metadata after `task`,
  `draft_output`, and all raw prompt/response persistence flags are cleared;
  `draft_requires_user_copy` is normalized to `false`;
- malformed `prompt-runs.json` is eligible for confirmed deletion;
- `model-task-matches.json` and `task-preflight-history.json` are eligible for
  confirmed deletion.

Generated quarantine residue for those sources is included in the same
inventory. Directories, special files, foreign-owner files, and hard links
fail closed. A symlink is treated as the leaf itself; confirmed cleanup can
remove the bound link but never follows or changes its target.

`privacy.previewCleanupLegacyContent` is zero-write. It returns a typed action,
opaque token, and one HMAC-bound precondition for every present cleanup leaf,
including its complete bounded content/type/identity revision, plus the
provider-action replay-state revision. The native client must show the
persistent cleanup-required state and obtain an explicit confirmation from
this preview.

`privacy.cleanupLegacyContent` reprojects under the single app-data owner lock,
reserves the one-time action token, and applies only the confirmed plan.
Every replacement or deletion first moves the accepted leaf to an unpredictable
same-directory no-replace quarantine and verifies its identity. Candidate
activation also uses no-replace semantics. If another state appears at the
original name, it is preserved together with the quarantine and the method
returns non-retryable `partial_effect`. Delete success requires the original
name to remain absent through parent-directory durability verification.
Verified success performs a semantic inspection read-back over
`private_content`; an uncertain effect or read-back failure is partial and
requires another inspection and preview.

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

## Project Context Consistency

- `project.getContext` returns the active project, recent projects, and a
  stable `revision` derived from the exact app-local state bytes. Missing state
  has a distinct stable revision. Reads accept only a bounded regular
  `project-context.json`; symlinks, oversized content, malformed JSON, and
  unsupported schemas fail closed.
- Set, clear-active, remove-recent, and clear-recents each use a dedicated
  `project.preview*`/apply pair. Preview is read-only, accepts the current
  expected revision, and returns the current/candidate states, exact
  `affected_count`, preconditions, and a signed preview token. Apply requires
  that exact `action_confirmation`.
- Apply performs a non-creating preflight, takes the shared app-data mutation
  owner lock, creating an authorized missing owner chain through
  component-wise no-follow directory descriptors when necessary, reprojects
  and revalidates the confirmation under the lock, atomically replaces the
  private state file, and returns a minimal verified project-context read-back.
  The action source revision and target-file precondition bind both logical
  project content and the complete private-file storage snapshot. Successful
  read-back requires the exact installed candidate inode; the original is
  considered stable only when its complete snapshot is unchanged. Candidate
  bytes observed after a replacement or parent-directory sync error are
  returned as non-retryable `applied_unverified`, while equal bytes on another
  inode are `outcome_unknown`, never verified success.
  A stale or mismatched action does not create app data or write state; an
  unsafe owner link does not chmod or populate its target.
- Clearing the active project preserves Recent Projects. Removing a recent row
  and clearing all recent rows preserve the active project even when it no
  longer appears in the recent list. Clear-recents confirmation reports the
  exact previewed row count.
- The native app publishes a successful project apply before requesting the
  follow-up catalog scan. A later scan or supporting-snapshot failure is
  reported separately and never reclassifies the project write as failed.
  Late scan responses whose accepted context revision no longer matches the
  current project are discarded.

## Catalog Scan Diagnostics

- `catalog.scanAll` and `catalog.scanClaude` accept an optional
  `expected_context_revision` and require `explicit_refresh: true`. A catalog
  scan rebuilds derived app-local cache state, so the explicit refresh
  invocation is its confirmation and there is no secondary preview/confirm
  prompt. Missing or false `explicit_refresh` and stale explicit revisions are
  rejected by a read-only preflight before app-data creation. The context
  revision is checked again while the shared mutation owner lock is held.
- One SQLite `IMMEDIATE` transaction owns all skill rows, missing-state
  reconciliation, findings, conflicts, and the scan revision for a refresh.
  The explicit refresh creates and locks an authorized missing app-data chain
  on the final no-follow owner descriptor before opening or migrating SQLite;
  an unsafe owner component fails before catalog creation. Any preparation or
  commit failure rolls back the whole scan. The response returns
  `accepted_context_revision`, `catalog_scan_revision`, and a matching verified
  `readback`.
- `catalog_scan_revision` versions successful catalog scans only. It is not a
  global version for every catalog mutation and must not be used to authorize
  config, install, manager, triage, or other catalog-backed actions.
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
  pre-diagnostics service. Scan revision/read-back fields are required for a
  scan result to be published as current catalog state.

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
- Search first returns a signed, local-only preview. The preview binds the
  normalized query and owner, manager executable, command, working directory,
  exact allowlisted environment, project/scope target, network posture, and
  the bounded one-time action state. `skillManager.applySearch` accepts only
  that exact explicit confirmation, revalidates under the app-data owner lock,
  and then starts the network-backed manager process. On a fresh filesystem,
  preview and any confirmation or full non-creating preflight rejection create
  nothing. After those checks succeed, the first manager apply may create only
  the single missing app-data leaf beneath an existing non-symlink parent, set
  it to `0700`, and lock its opened no-follow directory handle; the lock never
  creates a lock file. If state races stale after this bootstrap, locked
  revalidation stops before replay reservation and process start. The empty
  owner remains so existing waiters and future calls keep one coordination
  inode; no discovery state, catalog, audit, target, or manager effect occurs.
  Install, remove, update, and local-create perform their applicable target and
  manager-inventory revalidation while that same owner is held, then open or
  initialize the writable catalog beneath it without releasing or reacquiring
  the lock. A locked stale result therefore precedes SQLite creation. An
  unverified failure after writable catalog initialization crosses its local
  effect boundary is returned as a non-retryable partial outcome.
- Every admitted raw `npx --yes skills@1.5.20`
  search/remote-repository-install/remove/update/local-create operation is
  `network_required=true`, including local template creation, because `npx`
  may resolve or download the manager package before
  dispatching the subcommand. Its typed request must explicitly set
  `network_allowed=true`; false or omitted permission is rejected before
  app-data owner bootstrap, catalog access, target writes, or process creation.
  The signed preview exposes this network posture and exact command before
  confirmation. Process-free installed projection and local archive/delete
  operations retain `network_required=false`.
- Every manager child starts from `env_clear()`. The previewed exact allowlist
  includes `HOME`, `PATH`, locale and telemetry-off values; a dedicated
  app-owned npm cache; disabled npm user/global configuration and lifecycle
  scripts; the fixed `https://registry.npmjs.org/` registry; and Git/credential
  helper/interactive SSH discovery disablement. Parent secret environment
  values are not inherited. The external package is not sandboxed, receives
  the real `HOME` to preserve global target semantics, and runs with the user's
  filesystem authority; confirmation therefore accepts an explicit external
  code trust boundary rather than asserting that HOME files are inaccessible.
- The exact `skills@1.5.20` package specifier and executable revision are
  signed with the preview. Search, install, remove, update, and local-create
  revalidate that executable under the app-data owner lock and at the final
  safe point before command construction. macOS has no usable descriptor-exec
  primitive for this script/runtime chain, so observed drift is rejected but a
  malicious same-UID check-to-exec replacement cannot be claimed impossible.
- A manager working directory must already exist. On Unix the service opens
  every component without following symlinks, retains the accepted directory
  descriptor, rechecks its path binding immediately before spawn, and uses
  descriptor-based `fchdir` in the child. A retained clone is checked again
  after process exit and before catalog scan or target read-back; external cwd,
  app-data owner, or app-owned relative-cwd drift is a non-retryable partial
  outcome and never a verified success. It never creates an external project
  or home working-directory chain. Machine-readable stdout uses a
  descriptor-created, current-user-owned, single-link `0600` regular file;
  chmod and reads are descriptor-based. Cleanup atomically moves the current
  entry to a cryptographically random same-directory quarantine with
  no-replace semantics, verifies the quarantined inode against the retained
  descriptor, and unlinks only a match. A mismatch is restored without
  replacement or retained on conflict, so an entry substituted at the original
  path is never deleted. Quarantine, restore, and unlink transitions sync the
  parent directory; any post-quarantine verification, restore, unlink, or sync
  failure is a typed partial outcome with retained cleanup material when its
  removal cannot be proved. Explicit read completion returns cleanup failures;
  every post-create early return explicitly finalizes the capture and combines
  cleanup failure with its original error. Destructor cleanup is only a
  no-misdelete best-effort unwind fallback.
- `skillManager.previewInstall` accepts only credential-free remote repository
  sources for the raw manager path: HTTPS URLs, SSH URLs with no username or
  the literal `git` username, the literal `git@host:path` SCP form, or GitHub
  `owner/repository` shorthand. HTTP, `git://`, FTP, data, JavaScript, and
  custom schemes are rejected. URL/SCP sources require a non-empty host and
  repository path and reject empty, `.` or `..` path segments and backslashes.
  Absolute or existing relative paths, all `file://` URLs including
  percent-encoded spellings, and symlinks resolving to local content fail
  before action creation with the
  stable path-free error `local_source_requires_guarded_import`; no file URL
  reaches the manager. A GitHub
  `owner/repository` shorthand is converted to an explicit credential-free
  HTTPS URL before argv, source revision, and token are created; a later
  same-name path in the retained manager `cwd` therefore cannot substitute
  local bytes. Local content is admitted only by the bounded
  `skillManager.previewLocalArchiveImport` /
  `skillManager.applyLocalArchiveImport` lifecycle, then distributed from the
  locked app-data owner's `tool-global/skills/<skill>/SKILL.md` namespace by
  the no-process `skill.install` action. The protocol intentionally does not
  pass a user-controlled local source or an app snapshot to `npx`, because
  pre/post hashing cannot prove which bytes an unsandboxed external process
  opened. URL sources with disallowed userinfo, query, fragment, percent
  encoding, or credential data are rejected without echo; only the literal
  `git` SSH username is accepted. Credential-shaped URLs in manager output are
  replaced before parsing, diagnostics, or response serialization.
- A confirmed `skillManager.applySearch` returns every row emitted by the
  invoked manager and flattens list metadata into the response. Because the current manager output
  does not advertise an authoritative total or continuation token, search uses
  `total_count=null`, `has_more=false`, `source_completeness=unknown`, and
  `incomplete_reason=source_limited`; `returned_count` is only the number of
  rows actually returned and is never presented as the source total. Search
  previews must declare `network_allowed=true` so the confirmation shows the
  actual network posture; `network_allowed=false` is rejected locally and
  never starts the manager. Exit-zero output is accepted only when it matches
  a recognized result collection or the manager's explicit empty-result
  shape. Unknown, malformed, or structurally changed output after process start
  is `partial_effect` with `state=outcome_unknown` and automatic retry disabled.
  If the one-time reservation replacement itself cannot be proved durable, the
  apply returns non-retryable `partial_effect` before starting the manager.
  Its replay revision and write classification bind the complete private-file
  snapshot; equal JSON bytes at a new inode are an unowned third state, not an
  unchanged reservation.
- `skillManager.listInstalled` is a process-free projection over the accepted
  project-context catalog plus the applicable project/global manager lock. It
  never invokes `npx`, performs network access, or treats a generic plugin or
  cache path as package inventory. A missing lock is an exact empty manager
  projection; malformed, oversized, symlinked, or non-regular lock state fails
  closed without replacing the last accepted native cache.
  A row is `source_kind=manager` only when the matching scope lock file proves
  a valid source identity and package path. Lock-proven rows remain manager
  rows even when the manager source type is local. Their agent linkage comes
  only from the one physical source anchored by the selected scope's guarded
  `.agents/skills` display entry; same-name plugin caches, configured read-only
  roots, and unrelated sources cannot contribute agents or paths. Unlocked
  physical `.agents/skills` rows are `source_kind=local` and
  retain a redacted local source path. No raw manager payload is stored or
  returned by the local projection. No pagination flags or tokens are invented
  for either command. The native client may reveal an
  already-returned search collection in 20-row steps without issuing another
  manager or network request, while installed JSON and the app-owned local
  library remain fully accessible.
- The native inventory emits one row per lock-proven package source. Matching
  catalog rows supply supported-agent linkage and are consumed rather than
  appended again. Catalog-only fallback rows are limited to skill sources beneath the selected
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
  Empty completed searches display zero loaded rows, unknown total, and the
  typed source limitation with no load action. A preview alone is displayed as
  a pending confirmation, not as an empty search result.
- The Skill Manager UI does not expose agent-layer enable/disable controls.
  Skill removal is manager-backed unlink/removal from the currently selected
  agent targets, using the same explicit confirmation flow as install/update.
- Mutating manager actions, local archive import/update, and app-owned local
  deletion serialize through one cross-sidecar app-data owner lock that creates
  no lock artifact. A confirmed first manager apply may create the private
  app-data owner directory under the preceding fresh-filesystem rule; the lock
  itself remains the opened directory handle. Batch toggles and `skill.install`
  use the same lock, acquire it before the SQLite immediate transaction, and
  retain it through writes, semantic read-back, and commit.
  Install/remove/update previews bind the complete bounded target skill trees
  and manager inventory, including the applicable manager lock file;
  local-create binds its exact destination tree; archive actions bind the
  archive bytes and inspected archive tree from one bounded no-follow file
  snapshot, the complete destination/source tree, and relevant catalog
  identity/reference set. Apply revalidates these facts after taking the lock
  and before creating a process or replacing a tree. The lock remains held
  through catalog scan, semantic verification, and read-back.
- `skill.install` accepts source bytes only from the locked app-data owner's
  direct `tool-global/skills/<skill>/SKILL.md` namespace. Apply reopens that
  owner-relative source with no-follow semantics, requires a single-link
  bounded regular file, compares its complete device/inode/mode/link/length/
  mtime/ctime stamp with the preview, and installs those descriptor-read bytes.
  After the target scan it compares catalog list and detail semantics (agent,
  scope, path, definition, name, description, frontmatter, body, fingerprint,
  state, and project binding), then performs a final capability-relative
  target read. A same-path catalog row alone never proves success.
- Config, direct skill-file, and external archive-tree writes retain
  descriptor-bound recovery state under the mutation lock. The allowed root,
  parent chain, target, every archive-tree entry, and each newly created entry
  must be owned by the current effective user. Private temporary, backup,
  recovery, and quarantine entries use cryptographically random names.
  Complete-tree archive recovery validates a bounded owner/device/inode
  identity manifest and atomically quarantines before recursive removal.
  Post-namespace failures return `partial_effect`; uncertain cleanup preserves
  private recovery material and never follows a replacement display path.
- Catalog commit results after any config, skill-file, quarantine, archive, or
  manager effect are classified. A proven non-commit permits exact
  compensation only for wholly app-owned reversible state. An uncertain commit
  preserves the candidate or private recovery material and returns
  non-retryable `partial_effect` with `state=outcome_unknown`; it never runs
  reverse compensation. Manager install/update/local-create preserve their
  target state after either commit class because the external process boundary
  has already been crossed. A proven catalog non-commit is
  `applied_unverified`; an uncertain commit is `outcome_unknown`, and neither
  may be retried automatically.
- Pre-commit failure after any reversible filesystem effect explicitly rolls
  back the open catalog transaction before exact compensation. Only a
  successful rollback permits restoration. Rollback failure or an unknown
  rollback result preserves the skill/config candidate and any archive backup
  or delete quarantine, and returns non-retryable `partial_effect` with
  `state=outcome_unknown`; implicit transaction drop is not a proven rollback.
  External-manager process errors keep their existing not-started versus
  possible-effect classification after a proven rollback, while rollback
  uncertainty always upgrades the result to `outcome_unknown`.
- Install/remove/update success declares and verifies `catalog_skills`,
  `skill_files`, and `manager_inventory` read-back. Install proves every named
  skill's manager-lock source and safe package `skillPath`, then requires every
  selected agent target to resolve to that lock-proven canonical shared source.
  Its refreshed catalog record/detail must equal the post-process `SKILL.md`
  path, definition id, name, frontmatter, body, and fingerprint. Remove proves
  selected target links/records are gone; update repeats the lock/source proof
  and proves the preview-bound skill content fingerprint changed without
  rebinding to another source. An unrelated tree change or a same-path
  scan-window third state cannot satisfy any operation. Local-create verifies
  `catalog_skills` and
  `skill_files`, including the exact app-owned source and imported catalog ID.
  Install and update previews reject an empty skill selection. Every operation
  must also change at least one preview-bound target tree, so a preexisting
  install, absent remove, unchanged update, or uncreated template cannot turn
  exit code zero into verified read-back; it returns `verification_failed`.
  Multi-agent results emit separate catalog and skill-file observations for
  every selected target, including an empty catalog projection after a
  successful removal; one aggregate observation never stands in for target
  coverage.
- App-owned local deletion commits only after its missing-file and missing-row
  read-back verifies. If removal of the private quarantine then fails, the
  method remains a successful verified delete and returns typed
  `follow_up={kind:"quarantine_cleanup",
  state:"delete_applied_cleanup_pending",cleanup_required:true,...}`. The
  follow-up never contains the quarantine path; clients must show it as
  “deletion applied, cleanup pending,” not as either complete physical erasure
  or a failed delete.
- Local-create may create only its app-owned working-directory components under
  the retained owner and removes those components back to their original
  missing state if process creation fails; external project/home working
  directories are never created. Once a manager process starts, nonzero exit,
  output/read or output-cleanup failure, catalog refresh failure, semantic
  verification failure, or commit uncertainty is `partial_effect` with
  `retry_allowed=false`; it is never returned as an ordinary retryable command
  failure.
- Enable/disable is agent config state, not manager package state. The native
  UI routes both single and multi-skill changes through
  `batch.previewSkillToggles` and `batch.applySkillToggles`;
  `config.toggleSkill` is compatibility-only and returns `mutation_disabled`.
- Startup, manual reload, project switching, ordinary catalog refresh, and
  opening Skill Manager do not invoke an external manager or load manager
  inventory. The explicit Load Data action reads project/global catalog and
  lock projections without spawning a process, using the network, or writing
  agent config. Local app-owned skills are then merged into the same
  skill-centric inventory.
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
  and files outside the skill root. Preview returns a signed action reference
  and token bound to the archive bytes and inspected archive tree from the same
  no-follow file snapshot, complete target tree, catalog identity, references,
  agent, scope, and project. Apply reprojects under the shared owner lock, uses
  staged replacement and exact-state rollback, and must return
  verified `catalog_skills` and `skill_files` read-back. Any tree or catalog
  drift fails stale before mutation; replay cannot reimport a duplicate or
  replace an already identical complete tree. Imported scripts are never
  executed.
- Local archive and composite-uninstall failures classify both explicit
  pre-commit rollback and catalog commit results. Only a proven rollback or
  proven non-commit may restore the original tree. An uncertain result retains
  the imported tree or private backup/quarantine and returns non-retryable
  `partial_effect` with `state=outcome_unknown` and `cleanup_required=true`.
- When a removal selects every linked Agent target for an app-owned local
  source, the native confirmation identifies it as a full uninstall. One
  composite preview/action binds both manager unlink and the eligible local
  source/catalog/reference cleanup. One confirmed apply performs both under the
  same owner lock and returns combined manager, catalog, and missing-file
  read-back; the native client must not send a second local-delete preview or
  apply. Partial target removal keeps the shared source. If an external effect
  has crossed its boundary but combined completion cannot be safely proved,
  the service returns typed `partial_effect` with automatic retry disabled.

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
  With no agent filter and canonical auto-discovery, the service merges
  Claude Code, Codex, opencode, Pi, Hermes, and OpenClaw through their
  adapter-specific bounded readers into one revision-bound keyset. A missing
  canonical store remains visible through source notes; it is never silently
  replaced by a four-agent filesystem-only result.
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
- The Sessions workspace reads message pages progressively and publishes every
  accepted page. Explicit Load More or Load All continues with the accepted
  cursor and source revision. Cancellation and page failure retain already
  accepted messages and expose retry. The product Timeline contains only User
  and final Agent Reply rows; the legacy bounded process sample remains
  available to lower-level diagnostics during migration but is not mixed into
  the primary continuity timeline. Exact final-message counts replace bounded
  summary counts as pages arrive. Neither pages nor merged detail are
  persisted.
- SQLite-backed opencode, Hermes, and OpenClaw detail fixes the ordered raw-row
  endpoint on the first page. Its source revision hashes the database/session
  identity plus every raw row identity and content in that fixed range, so an
  append remains outside the snapshot while deletion, reordering, or same-count
  in-place replacement returns `source_changed`. One page advances through at
  most 1,000 raw rows and 32 MiB; pages containing only non-message rows may
  return zero items with an advancing cursor. Establishing the fixed SQLite
  snapshot is itself bounded to 20,000 raw rows and 32 MiB. A larger source
  retains the bounded prefix and reports `limited` with
  `incomplete_reason="safety_budget"` instead of claiming complete coverage.
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
- Canonical SQLite inventories and the Codex thread index probe one row beyond
  their 10,000-row metadata bound. Detecting another row beyond that bound is terminal
  `safety_budget`/`limited` evidence with no continuation cursor; it is never
  reported as an enumerable 10,000-row total.
- Hermes `state.db` sessions do not expose a verified project path. All-scope
  inventory retains them with an unassigned project, project scope excludes
  them, and `session.previewResume` returns typed
  `invalid_project_context` with no argv. The service does not infer a project
  from title, source, time, or the selected app project.
- Project session membership requires normalized equality with either the
  selected project root or its accepted current cwd. A different descendant
  directory is not silently promoted into the selected project.
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
- `session.previewResume` is the only resume-command preview. Clients must not
  infer a command from session metadata, titles, source paths, or agent names.
  It revalidates the exact session against the accepted native inventory
  revision and product snapshot revision. Supported responses contain ordered,
  adapter-native `resume.argv` with `copy_only=true`; unsupported responses
  contain no argv and one typed reason. The method is read-only, process-free,
  network-free, and never launches a terminal.

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

- Every `llm.previewPrompt` request requires an active selected project.
  Optional top-level `source_revision` binds the request to a previously
  accepted product projection; mismatch returns `source_changed` without
  provider traffic.
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
- Action `skill_change_review` requires `skill_instance_id` and returns the
  `skill_change_review` result schema over the selected skill aggregate and its
  current evidence. It never invents comparison history.
- Action `session_digest` requires a nested `session` object containing
  `authorized_roots`, `auto_discover`, `agent`, `project_root`, `current_cwd`,
  `session_id`, native `source_revision`, and product `snapshot_revision`.
  The service reuses `session.previewResume` validation to bind the exact
  session identity and both revisions, but excludes resume argv and raw
  transcript content from the provider prompt.
- Action `project_health` accepts the optional product `source_revision`,
  returns `copy_only_markdown`, and includes only the accepted readiness
  coverage, per-agent, blocker, and attention evidence.
- Action `semantic_search` requires `user_intent` plus
  `search_candidates: [{id, kind, title, subtitle}]`. It accepts 1–18 unique
  candidates whose kinds are `skill`, `session`, or `config_history`.
  Candidate evidence has an opaque ID and a local-only `target_id`; the
  rendered provider contract omits `target_id`, redacts candidate display
  fields, performs no additional read or scan, and returns
  `semantic_rerank`. Ranked IDs and rationale IDs must be unique, known
  evidence references from that exact candidate set.
- `llm.previewPrompt` returns `response_contract` with schema version 1,
  `request_kind`, `project_id`, accepted product `source_revision`,
  `result_schema`, typed `evidence`, optional deterministic `actions`, and
  exact `required_safety_flags`. The supported result schemas are
  `copy_only_markdown`, `task_readiness`, `session_digest`,
  `skill_change_review`, and `semantic_rerank`.
- A successful HTTP transport is not sufficient for any prompt action. The
  provider must return only a JSON `response_envelope` whose identity,
  revision, schema, safety flags, and referenced evidence/actions match the
  confirmed contract. Unknown or duplicate references, target drift, stale
  source revision, unsafe flags, malformed schema, and command/mutation-shaped
  results use `parse_failed` with `response_schema_invalid`. Provider metadata
  timestamps are epoch milliseconds; plausible legacy epoch-second records are
  normalized on read.
- `llm.previewPrompt` returns the same signed action descriptor, preconditions,
  and opaque token used by the provider profile lifecycle.
  `llm.confirmPromptAndSend` accepts only the exact `action_confirmation`; a
  consumed, stale, mismatched, or forged reference is never retried
  automatically.
- A verified send returns read-back over `provider_activity` and
  `prompt_runs`. When the request may have reached the provider but either
  local metadata record cannot be verified, the result is `status=partial`
  with a typed `partial_outcome`, `remote_effect=remote_unknown`, no verified
  read-back, and recovery guidance requiring inspection and a new preview.
- A successful confirmed send includes the validated transient
  `response_envelope`. Native clients validate it again against the previewed
  `response_contract` before publishing the structured copy-only result.
- Provider output is returned only by the immediate confirmed-send response and
  remains copy-only. `prompt-runs.json` is bounded metadata history; it never
  stores task text, `user_intent`, `task_text`, rendered prompt text, provider
  output, or response bodies. The backward-compatible `task` and
  `draft_output` fields decode but are always returned and persisted as
  `null`. Reads suppress legacy body values in memory without rewriting the
  file; the next explicit confirmed metadata write serializes body-free
  records. Observability therefore reports `draft_output_available=false`.

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
