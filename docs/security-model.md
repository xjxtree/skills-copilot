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
  through a supported external manager CLI. The UI enables that scoped path by
  default; write commands still expose destination, scope, affected targets,
  and a confirmation-bound preview in typed service requests.
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
  stale references have zero write, process, catalog, snapshot, and audit side
  effects.
- Catalog-sensitive apply paths acquire the shared app-data owner lock, then
  hold a SQLite immediate transaction while revalidating the accepted catalog
  record, writing targets, proving read-back, and committing. Batch targets are
  precomputed in stable order and `skill.install` validates a non-creating
  target before locking. A changed record, reference set, source tree, or
  target revision returns a stale-action failure before any owned write.
- Batch toggles, `skill.install`, Skill Manager
  install/remove/update/local-create, and app-owned local deletion share one
  cross-sidecar mutation lock on the existing app-data owner directory; the
  lock creates no persistent filesystem artifact. Under that lock, the service rechecks
  the complete bounded target trees and manager inventory accepted at preview.
  It holds the lock through process execution, catalog refresh, semantic
  verification, and read-back. A stale preview runs no manager process and
  writes neither targets nor app data.
- The lifecycle currently covers single and batch agent-config toggles through
  `batch.*`, `skill.install`, Skill Manager install/remove/update/local create,
  physical deletion of an eligible app-owned local source, explicit Claude
  settings saves, and config snapshot rollback. Local archive import/update,
  provider/profile calls, and LLM send keep their existing method-specific
  guards and must not be presented as action-reference-backed until their
  contracts are migrated. `config.toggleSkill` is compatibility-only and
  cannot mutate.

### Config Mutation Atomicity

- Config save and rollback perform a non-creating, read-only preflight before a
  writable catalog is opened. Rejected or stale preflight must not create the
  app-data directory, catalog, lock artifact, config parent, target, snapshot,
  or audit row.
- A valid apply may initialize the app catalog, then takes the cross-process
  mutation owner lock on the existing app-data directory. While holding that
  owner it revalidates every action precondition, begins an SQLite `IMMEDIATE`
  transaction, performs the atomic file replacement, semantically reads back
  every declared domain, commits, and releases the owner. It never creates a
  target-side `.lock` file.
- Failure rolls back the open catalog transaction before file compensation.
  Compensation may restore the original config only when the current target is
  still byte-for-byte the candidate written by that apply. An originally
  missing target returns to missing, including removal of only the empty
  ancestor directories created for the failed write.
- A third target state, unreadable target, or failed compensation is an unknown
  outcome. The service must not overwrite that state; it returns
  `partial_effect` with cleanup required, and clients must stop automatic
  retries and require inspection.

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
  Large installed-inventory stdout is captured only in a private `0600`
  temporary regular file, size-checked before reading, and removed by scoped
  cleanup on success and failure; it is never cataloged or retained.
  Process-start failure restores any manager working-directory components
  created for the attempt. After a process starts, an unobserved/nonzero
  result, failed semantic proof, refresh failure, or uncertain commit returns
  structured `partial_effect` with `retry_allowed=false`; compensation never
  overwrites a target whose current revision is neither the accepted original
  nor this operation's candidate revision.
  A local delete whose catalog and missing-source read-back committed remains a
  successful delete if only private quarantine cleanup fails; its typed
  path-free `follow_up` reports `cleanup_required=true` so the client cannot
  claim complete physical erasure.
- Local ZIP import and updates are the explicitly scoped ZIP exception. Import
  writes only to the app-owned local library. Update may replace either an
  app-owned source or one canonical descendant of the active project/global
  `.agents/skills` root after the catalog proves the selected instance and
  scope. Preview
  validates a regular bounded archive, one matching `SKILL.md`, safe paths,
  file types, counts, and expanded sizes; apply is bound to both ZIP and current
  source digests and uses staged replacement with rollback. Imported scripts
  remain data and are never run.
- Catalog discovery never grants package-manager write authority: plugin
  caches, configured read-only roots, and native roots outside the guarded
  selected `.agents/skills` roots are excluded from editable inventory.
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
