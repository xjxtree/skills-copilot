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

Every supported mutation follows Detect, Explain, Evidence, Preview, Confirm,
Apply, and Read-back. A typed action descriptor may identify an existing
service path, but it is not authorization. Preview and apply must independently
validate project, agent, scope, target, revision, confirmation, and network
posture.

- Skill scripts are untrusted. Script execution is default-denied and must not
  be triggered by imports, LLM output, analyzer recommendations, previews, or
  cleanup guidance.
- Adapter writes stay limited to the documented guarded toggles and install
  roots in `AGENTS.md` and `docs/adapters/agent-adapters.md`.
- Skill Manager writes must use the manager tool when the tool supports the
  operation. The service may run `npx skills` with argv-only commands,
  telemetry-off env, redacted logs, and read-back catalog refresh; enable/
  disable still uses the existing guarded agent-config toggle APIs.
  Large installed-inventory stdout is captured only in a private `0600`
  temporary regular file, size-checked before reading, and removed by scoped
  cleanup on success and failure; it is never cataloged or retained.
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
