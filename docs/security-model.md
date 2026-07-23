# Security Model

This file describes security and privacy boundaries.

## Trust Boundaries

- Rust crates own product logic and policy decisions.
- The native macOS app presents state and sends typed requests to the Rust stdio
  service.
- Local agent files, skills, transcripts, LLM output, screenshots, and generated
  reports are untrusted inputs.
- Fixture smoke validation must not touch real user config. Real-local
  validation may read the developer's real HOME and app data only when the
  runbook explicitly calls for it.

## Product Truth And Evidence

- Environment health, skill effectiveness, scan coverage, conflicts, session
  identity, and continuation capability are deterministic Rust projections.
  LLM output cannot originate or override them.
- An incomplete, stale, partial, unavailable, or source-limited required input
  cannot produce a healthy status.
- Installed, enabled, and effective are separate facts. `effective` means the
  documented adapter projection selected the skill; it is not runtime
  telemetry or proof of invocation.
- Every product status, explanation, and action must resolve to typed evidence
  from the same project/source revision. Unknown or stale evidence references
  fail closed.

## Privacy Rules

- No cloud sync, accounts, telemetry, anonymous crash reports, or uncontrolled
  outbound network calls.
- Skill Manager search/install/update may make outbound network calls only
  through a supported external manager CLI. Search first produces a local-only
  signed preview and starts the external process only after a separate explicit
  confirmation. Write commands likewise expose destination, scope, affected
  targets, and a confirmation-bound preview in typed service requests.
- Provider calls made by Agent Copilot's optional AI features require user
  enablement, prompt preview, redaction, destination visibility, and explicit
  confirmation.
- Raw transcripts, prompts, responses, traces, credentials, screenshots, and
  reports must not persist secrets.
- Session preview data is redacted and bounded before it crosses the service
  boundary.
- AI summaries, semantic reranks, readiness interpretations, and evidence
  envelopes remain transient. Persisted embeddings, transcript indexes, or raw
  prompt/response caches require a new scoped privacy review.

## Credentials

- Credentials must prefer Keychain.
- Never write credentials to SQLite, project directories, logs, prompts,
  response artifacts, screenshots, or reports.
- Provider Observability may show redacted metadata only for Agent Copilot's own
  optional AI requests. It must not expose raw secrets, infer usage from managed
  agent provider profiles, or add write/delete controls without a new scoped
  safety review.

## Writes And Scripts

Every mutation admitted to the typed action lifecycle follows Detect, Explain,
Evidence, Preview, Confirm, Apply, and Read-back. A typed action descriptor
identifies one service-owned operation, but it is not authorization. Preview
and apply independently validate project, agent, scope, target, revision,
confirmation, and network posture.

### Action Authorization

- A preview returns a deterministic action descriptor, explicit preconditions,
  and an opaque HMAC authorization token. The stable action ID binds kind,
  intent, target, and project; the token additionally binds the accepted source
  revision, impacts, network posture, read-back domains, evidence references,
  and sorted preconditions.
- The macOS app creates one cryptographically random 32-byte secret for its
  lifetime. It passes only the hex-encoded secret to each Rust sidecar child in
  that child's environment. The sidecar consumes and removes the variable
  before reading stdin. Missing or invalid secret material returns
  `action_token_unavailable`; production code never substitutes a public hash
  or process-local random fallback.
- The authorization token may cross only the typed app-to-sidecar preview and
  confirmation payloads. It is never rendered, copied, persisted, logged,
  included in accessibility metadata, or inherited by an external manager
  child process.
- Apply requires the exact confirmed action reference and token. The service
  reprojects the action, acquires every required target lock, and revalidates
  all preconditions before the first write. Unknown, mismatched, forged, or
  already-stale references rejected by the non-creating preflight have zero
  write, process, catalog, snapshot, and audit side effects. The sole
  coordination bootstrap exception is the confirmed fresh-filesystem manager
  apply described below: a race detected by locked revalidation may retain only
  its empty private owner directory.
- Catalog-sensitive apply paths acquire the shared app-data owner lock, then
  hold a SQLite immediate transaction while revalidating the accepted catalog
  record, writing targets, proving read-back, and committing. Batch targets are
  precomputed in stable order and `skill.install` validates a non-creating
  target before locking. A changed record, reference set, source tree, or
  target revision returns a stale-action failure before any owned write.
- Writable catalog creation and schema migration take that same owner before
  opening the mutation connection. Confirmed non-manager initialization walks
  each app-data component with no-follow directory opens, creates only missing
  components with private permissions, and locks the final opened owner
  descriptor. A final or intermediate symlink fails closed before its target
  can be chmodded, populated, or used for catalog migration. A confirmed action
  then reacquires the existing no-follow owner and revalidates its
  preconditions before changing catalog or target state; read-only RPCs never
  migrate an outdated catalog.
- Skill Manager lock projections and one-time discovery replay state are read
  once from a bounded, no-follow regular-file descriptor. Symlinks,
  non-regular files, oversized inputs, and replacement races fail closed.
- Batch toggles, project-context applies, catalog scans, `skill.install`,
  confirmed Skill Manager search, install/remove/update/local-create, local
  archive import/update, and app-owned local deletion share one
  cross-sidecar mutation lock on the app-data owner directory; the lock creates
  no persistent lock file. If a confirmed manager apply is the first durable
  action on a fresh filesystem, only after its exact confirmation passes may
  it create the single missing app-data leaf as a private `0700` directory
  beneath an existing non-symlink parent and lock the opened no-follow
  directory handle. Preview and any confirmation or full preflight rejection
  remain zero-write. If state races stale only after that bootstrap, locked
  revalidation stops before replay reservation or process start and leaves the
  private owner empty; it never unlinks the coordination inode while another
  sidecar may be waiting on it. Missing parent chains and symlink owners fail
  closed. Under the lock, the service rechecks the complete bounded target
  trees, archive bytes, relevant catalog identity, and manager inventory
  accepted at preview. It holds the lock through process execution or staged
  filesystem replacement, catalog refresh, semantic verification, and
  read-back. A stale preview runs no manager process and writes neither targets
  nor replay, catalog, or audit state; only the already-authorized empty owner
  bootstrap may remain after a post-preflight race.
- App-owned private-file helpers apply the same component-wise no-follow rule
  before creating a parent directory or opening a write destination. This
  covers project context, provider replay/profile/activity state, prompt-run
  metadata, and migration staging. Unsafe parent links are rejected without
  changing the linked directory's mode, contents, or children.
- External config and direct skill-file mutations hold a capability borrowed
  from the app-data mutation lock. It binds the allowed root, every parent
  directory, and the target entry by descriptor identity and requires each to
  be owned by the current effective user; later reads, replacement, read-back,
  and compensation stay relative to those descriptors. Regular-file proof
  includes owner, device, inode, mode, link count, length, mtime, and ctime
  around each bounded read. New files and directories are ownership-verified
  before chmod, sync, or activation. Temporary, backup, and quarantine names
  use cryptographic randomness. Existing targets retain their exact displaced
  inode until catalog commit; missing targets and newly created private parents
  are removed only through identity-bound atomic quarantine. Errors after a
  namespace effect are typed `partial_effect`; cleanup uncertainty preserves
  recovery material instead of following a changed display path.
- The one-time legacy bundle app-data migration locks the already-opened common
  parent directory across processes and keeps the legacy source, private
  staging tree, and final target descriptor-relative. It accepts only
  current-user-owned regular files and directories, bounded to 100,000
  entries, depth 128, 1 GiB per file, and 8 GiB total; symlinks, special files,
  hardlinks, and cross-filesystem directory traversal fail closed. Copied files
  and directories are synced with `0600` and `0700` permissions, the path-free
  migration marker is written through the staging descriptor, and activation
  uses an atomic no-replace rename.
  Failure cleanup removes a staging tree only while its root still has the
  inode created by that attempt. The legacy source is preserved, target races
  are never overwritten, and any binding or read-back uncertainty after
  activation returns non-retryable `partial_effect`.
- Every catalog commit after a config, skill-file, quarantine, archive, or
  external-manager effect classifies a proven non-commit separately from an
  uncertain commit outcome. Exact reverse compensation is allowed only after a
  proven non-commit and only for wholly app-owned reversible state. An
  uncertain commit never triggers reverse compensation; it preserves the
  candidate, private quarantine or backup, or manager target state and returns
  a non-retryable `partial_effect` with `state=outcome_unknown`. Once an
  external manager process has crossed its effect boundary, even a proven
  catalog non-commit preserves the manager target state and requires explicit
  catalog recovery rather than automatic process replay.
- A pre-commit error after a skill/config candidate, archive replacement,
  backup, or quarantine exists must explicitly roll back the open catalog
  transaction before filesystem compensation begins. Compensation is allowed
  only when that rollback returns success. A failed or unprovable rollback
  preserves the candidate and all private recovery material and returns
  non-retryable `partial_effect` with `state=outcome_unknown`; transaction-drop
  cleanup is never treated as proof that compensation is safe.
- Catalog scans are the narrow derived-cache exception to the signed
  preview/apply lifecycle. `catalog.scanAll` and `catalog.scanClaude` require
  `explicit_refresh: true`; the explicit refresh invocation itself is the
  confirmation, no secondary prompt is shown, and missing authorization is
  rejected before app-data creation. Accepted project-context and scan
  revisions plus verified read-back make the committed cache generation
  inspectable.
- The lifecycle currently covers single and batch agent-config toggles through
  `batch.*`, all project-context mutations, `skill.install`, confirmed Skill
  Manager search, install/remove/update/local create, local archive
  import/update,
  physical deletion of an eligible app-owned local source, explicit Claude
  settings saves, config snapshot rollback, provider profile save/delete,
  provider connection tests, and confirmed LLM prompt sends.
  `config.toggleSkill` is compatibility-only and cannot mutate.

### Project Context And Catalog Scan Atomicity

- Project-context reads are bounded, regular-file-only, and no-follow.
  Preview binds the exact current bytes and candidate state to a signed action;
  apply creates and locks any authorized missing owner chain through the same
  component-wise no-follow descriptor walk, then revalidates under the shared
  cross-process owner lock before atomic private-file replacement and semantic
  read-back. Candidate bytes observed after a replacement or parent-directory
  sync error are `applied_unverified`, never a verified success.
- A stale project preview or stale explicit scan context is rejected by a
  non-creating preflight. It must not create the app-data directory, catalog,
  target file, lock artifact, snapshot, or audit record.
- Catalog scans hold the same owner lock used by config, manager, install, and
  project mutations. Skill rows, stale-row reconciliation, findings,
  conflicts, and the scan-only revision commit in one SQLite `IMMEDIATE`
  transaction or roll back together.
- The scan revision proves only the accepted scan transaction. It is not a
  global catalog mutation revision and cannot authorize unrelated writes.

### Config Mutation Atomicity

- Config save and rollback perform a non-creating, read-only preflight before a
  writable catalog is opened. Rejected or stale preflight must not create the
  app-data directory, catalog, lock artifact, config parent, target, snapshot,
  or audit row.
- A valid apply may initialize the app catalog, then takes the cross-process
  mutation owner lock on the existing app-data directory. Catalog
  initialization creates and locks the no-follow owner on one descriptor;
  confirmed config apply then reacquires that existing owner, revalidates every
  action precondition, begins an SQLite `IMMEDIATE` transaction, performs the
  atomic file replacement, semantically reads back every declared domain,
  commits, and releases the owner. It never creates a target-side `.lock` file.
- Failure rolls back the open catalog transaction before file compensation.
  Compensation may restore the original config only when the current target is
  still byte-for-byte the candidate written by that apply. An originally
  missing target returns to missing, including removal of only the empty
  ancestor directories created for the failed write.
- A third target state, unreadable target, or failed compensation is an unknown
  outcome. The service must not overwrite that state; it returns
  `partial_effect` with cleanup required, and clients must stop automatic
  retries and require inspection.

### Provider And Prompt Mutation Atomicity

- Provider previews bind the current profile store, the bounded replay state,
  and the normalized non-secret input. A save that replaces a credential also
  binds a keyed opaque value derived from the submitted secret. The raw secret
  and its reusable digest never enter descriptors, tokens, replay state, logs,
  responses, or UI state.
- A confirmed provider/profile or prompt action consumes its token exactly
  once. The private `provider-action-state.json` file is a bounded, atomic,
  `0600` single-record replay guard, not action history. It stores only a
  monotonic generation, token digest, action id, source revision, phase, state,
  and timestamp. A `not_started` reservation is atomically replaced by its
  terminal `not_started`, `verified`, or `partial` outcome; the next action
  replaces that record with a higher generation. Replacement uses one
  controlled path under the provider lock, removes bounded legacy crash
  residue, and verifies the parent-directory sync after rename. A consumed
  action is never automatically retried; recovery starts from a fresh preview.
  Malformed, oversized, symlinked, non-regular, or unbounded replay storage
  fails closed.
- Provider mutations use an existing canonical parent-directory lock, so a
  rejected stale or mismatched confirmation creates no lock or replay-state
  artifact. Provider replay and metadata parents are opened or created
  component-wise with no-follow semantics before file replacement; a
  symlinked app-data owner or intermediate component is rejected before any
  provider effect and without changing the link target. Credential replacement
  is staged in Keychain, verified, and compensated if the profile write fails.
  A credential or local metadata effect that cannot be semantically verified is
  `partial` and `applied_unverified`.
- After a provider request may have left the process, post-request transport,
  response-body read, JSON/schema parsing, audit, or prompt-run persistence
  failures are returned as a typed partial outcome with
  `remote_effect=remote_unknown`. Provider HTTP clients do not follow
  redirects, so Bearer and API-key credentials never leave the previewed
  destination. The client must not retry automatically. A verified apply
  returns read-back observations for every domain declared in its action
  descriptor, including credential semantics when Keychain is in scope.

- Skill scripts are untrusted. Script execution is default-denied and must not
  be triggered by imports, LLM output, analyzer recommendations, previews, or
  cleanup guidance.
- `script.execute`, `catalog.importSkill`, and `skill.exportBundle` are
  compatibility-only blocked RPCs. They return `mutation_disabled` before
  parameter-dependent filesystem, catalog, process, network, or audit I/O.
- Adapter writes stay limited to the documented guarded toggles and install
  roots in `AGENTS.md` and `docs/adapters/agent-adapters.md`.
- Skill Manager writes must use the manager tool when the tool supports the
  operation. The service may run `npx skills` with argv-only commands,
  a cleared environment, an explicit `HOME`/`PATH`/locale/telemetry-off
  allowlist, redacted logs, and read-back catalog refresh; enable/disable uses
  the guarded batch agent-config lifecycle. Previewed environment keys are the
  exact runtime allowlist. Parent credentials, provider tokens, cloud
  credentials, and action secrets are never inherited by the manager.
  Relative local install sources resolve and canonicalize against the displayed
  manager working directory. Standard and SCP-style sources containing
  userinfo other than the literal `git` SCP form, query, fragment, percent
  encoding, or credential material are rejected without echo, and
  credential-shaped URLs in child output are replaced as a whole before
  parsing or returning diagnostics.
  Process exit zero is insufficient: install/remove/update must prove their
  selected agent/scope postconditions in the refreshed catalog, while local
  create must prove the exact source, `SKILL.md`, imported file, and catalog
  identity. Install and update require an explicit non-empty skill selection.
  Every mutating manager operation must also change at least one preview-bound
  target tree; an exit-zero no-op, including a preinstalled install, absent
  remove, unchanged update, or uncreated local template, returns
  `verification_failed` without verified read-back. Multi-agent success records
  separate catalog and skill-file evidence for every selected target.
  Remote-search stdout/stderr is bounded and redacted before parsing or
  returning diagnostics. Its one-time reservation is a bounded private
  app-local record; a stale query, owner, executable, working directory,
  environment, target, or consumed confirmation is rejected before process
  creation. A reservation whose replacement durability cannot be proved is a
  non-retryable `partial_effect`, and no manager process starts. After process
  start, an exit-zero response still requires a
  recognized result collection or explicit empty-result shape; unknown or
  malformed output is a non-retryable `partial_effect`, never verified empty
  evidence.
  Process-start failure restores any manager working-directory components
  created for the attempt. After a process starts, an unobserved/nonzero
  result, failed semantic proof, refresh failure, or uncertain commit returns
  structured `partial_effect` with `retry_allowed=false`; compensation never
  overwrites a target whose current revision is neither the accepted original
  nor this operation's candidate revision.
  Every manager path explicitly rolls back its open catalog transaction on
  process or post-process failure. A proven rollback preserves the original
  process error classification; an unprovable rollback is `outcome_unknown`.
  Manager target effects are preserved rather than replayed or automatically
  reversed.
  A local delete whose catalog and missing-source read-back committed remains a
  successful delete if only private quarantine cleanup fails; its typed
  path-free `follow_up` reports `cleanup_required=true` so the client cannot
  claim complete physical erasure.
- Startup, reload, project switching, catalog refresh, and installed-package
  reads use the accepted catalog and manager-lock projection. They never start
  the external manager. A malformed or unsafe lock fails closed while the
  native client keeps its last accepted projection.
- Zero-write RPCs use a current-schema read-only catalog handle. They never
  migrate an outdated catalog as a side effect; outdated state fails closed
  until an explicit catalog scan or confirmed writable lifecycle performs the
  migration.
- Local ZIP import and updates are the explicitly scoped ZIP exception. Import
  writes only to the app-owned local library. Update may replace either an
  app-owned source or one canonical descendant of the active project/global
  `.agents/skills` root after the catalog proves the selected instance and
  scope. Preview opens the archive with no-follow semantics and uses one
  bounded in-memory snapshot for hashing, inspection, and extraction. It
  validates one matching `SKILL.md`, safe paths, file types, counts, and
  expanded sizes. Its signed action binds the archive bytes, inspected archive
  tree, complete destination or selected source tree, and relevant catalog
  identity/reference set. Apply reprojects those facts under
  the shared owner lock and SQLite immediate transaction, uses staged
  replacement with exact-state rollback, and returns minimal verified
  `catalog_skills` plus `skill_files` read-back. Tree drift fails stale before
  replacement, and a repeated confirmation cannot reapply an already imported
  or identical tree. A proven non-commit restores the original filesystem
  state; an uncertain commit retains the imported tree or private backup and
  returns non-retryable `outcome_unknown` with cleanup required. Imported
  scripts remain data and are never run.
- Archive replacement treats the complete directory as one capability rather
  than trusted path strings. Staging, activation, rollback, and cleanup use
  no-follow directory descriptors plus a bounded manifest of every directory
  and regular-file identity and full file stamp. Symlink, special-file,
  hardlink, same-length mutation, parent replacement, and extra-entry races
  fail closed. Cleanup first atomically moves the verified tree to a
  cryptographically random private quarantine and removes only entries that
  still match the manifest; uncertainty leaves that quarantine available for
  explicit recovery.
- A full uninstall of an app-owned local package is one composite manager
  action. The preview and single confirmation bind the manager unlink targets,
  complete local source tree, catalog row, and reference set. Apply performs
  the manager removal and guarded local quarantine/catalog cleanup under the
  same owner lock and returns one combined read-back; the client must not issue
  a hidden second local-delete preview or apply. Any effect that crossed the
  manager boundary but cannot be proved or safely completed returns typed
  `partial_effect` with automatic retry disabled. An uncertain catalog commit
  retains the private local quarantine; only a proven non-commit restores it.
- Catalog discovery never grants package-manager write authority: plugin
  caches, configured read-only roots, and native roots outside the guarded
  selected `.agents/skills` roots are excluded from editable inventory.
  Manager-lock projection validates each source identity and package path,
  anchors linkage to one guarded `.agents/skills` display source, and then
  aggregates only catalog rows with that same physical source. Every lock-proven
  row remains manager-owned, including local-source lock entries; only
  unlocked physical descendants of the guarded root are local inventory.
  Installed local sources outside those roots remain visible but are
  unlink-only; they never receive a ZIP replacement action.
- Hidden apply/write paths, hidden task state, raw prompt/response/trace
  persistence, public distribution automation, signing, notarization, DMG, and
  other ZIP creation/distribution work require explicit new scope.

## Session Continuation

- Resume capability is derived by a supported adapter and bound to the selected
  session and source revision.
- The initial product action is copy-only. The app must not automatically open
  a terminal, launch an agent, translate a session between agents, or execute a
  resume command.
- Unsupported or unverifiable continuation returns a typed reason; the native
  UI must not construct a best-guess command.

## Screen Capture

- UI screenshots used during validation must capture only the full app window
  and remain outside the repository.
- Full desktop screenshots are forbidden.
- If the macOS session is locked, cannot be confirmed interactive, or window
  capture is blocked, report the canonical blocker in the task or pull request
  instead of substituting fixture output.

## Verification

Use `pnpm check:privacy` before committing or pushing changes. Use `pnpm
check:macos` for substantial user-visible, UI, or service-protocol changes.
