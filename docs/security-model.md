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
- Task Preflight task text and provider draft output are session-only. Its
  persisted prompt-run row contains diagnostic metadata without those fields;
  service startup also removes them from legacy Task Preflight rows without
  changing unrelated prompt-run content.
- Task Preflight confirmation exposes a redacted request summary plus
  destination and token/cost estimates. The complete redacted request and
  complete untrusted response are available only from current-session memory
  after a send attempt, are copy-only, and disappear when the app exits.
- Session preview data is redacted and bounded before it crosses the service
  boundary.
- Native file watching is limited to the Rust-provided bounded authorization
  plan. Broad roots and symbolic-link components fail closed. FSEvents paths
  are used transiently only to match authorized recursive roots and exact
  config files, then discarded; they are never logged, rendered, or persisted.
  Matching events only invalidate local cache state until the user explicitly
  chooses Refresh.

## Credentials

- Credentials must prefer Keychain.
- Never write credentials to SQLite, project directories, logs, prompts,
  response artifacts, screenshots, or reports.
- Provider Observability may show redacted metadata only for Agent Copilot's own
  optional AI requests. It must not expose raw secrets, infer usage from managed
  agent provider profiles, or add write/delete controls without a new scoped
  safety review.

## Writes And Scripts

- Skill scripts are untrusted. Script execution is default-denied and must not
  be triggered by imports, LLM output, analyzer recommendations, previews, or
  cleanup guidance.
- Adapter writes stay limited to the documented guarded toggles and install
  roots in `AGENTS.md` and `docs/adapters/agent-adapters.md`.
- Skill Manager writes must use the manager tool when the tool safely supports
  the operation. Complete uninstall runs `npx skills remove` with no
  `--agent` restriction, argv-only commands, telemetry-off env, redacted logs,
  and catalog plus manager-inventory read-back. Selected-Agent uninstall does
  not call the manager's unsafe partial-remove path or write Agent
  enable/disable configuration. It requires every exact catalog identity from
  the package row, accepts only a separable direct-child symlink or copied
  directory under that Agent's documented install root, binds confirmation to
  the target type and bounded tree revision, stages removal outside scanned
  roots, verifies selected entries disappeared while preserved paths remain,
  and rolls back failed verification. The shared source and manager lock remain
  unchanged.
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
- Direct local-folder installation is a separate manager path, not a ZIP
  import. Inspection accepts one absolute non-symlink directory, rejects
  nested symlinks, special files, duplicate skill names, and bounded-tree
  overflow, then invokes only the manager's network-free `--list --full-depth`
  operation. It does not copy or execute source content. A later install uses
  the ordinary explicit command preview and confirmation, with the complete
  source digest bound into the preview token so changed content cannot inherit
  an earlier confirmation.
- Catalog discovery never grants package-manager write authority: plugin
  caches, configured read-only roots, and native roots outside the guarded
  selected `.agents/skills` roots are excluded from editable inventory.
  Installed local sources outside those roots remain visible but are
  limited to exact selected-Agent physical uninstall when a documented
  per-Agent target exists, or complete external-manager uninstall; they never
  receive a ZIP replacement action. A direct source shared by selected and
  unselected Agents is never deleted by partial uninstall.
- Developer ID signing, notarization, stapling, and optional post-staple ZIP
  creation are explicit maintainer-only release actions. They require an
  identity selected at release-build invocation and a named `notarytool`
  Keychain profile; raw notarization credentials are not accepted. The scripts
  never run from normal builds, never publish an artifact, and refuse to
  overwrite an existing output ZIP.
- Hidden apply/write paths, hidden task state, raw prompt/response/trace
  persistence, public distribution automation, DMG creation, updater feeds,
  and other ZIP creation/distribution work require explicit new scope.

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
