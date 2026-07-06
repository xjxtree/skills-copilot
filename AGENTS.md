# AGENTS.md

Shared instruction entrypoint for Codex, Claude Code, Pi, opencode, and other
coding agents working in this repository.

## Purpose

- Keep this file short, operational, and safe.
- Put human-facing overview in `README.md`.
- Put future planning in `docs/plans/roadmap.md`.
- Put active task routing in `docs/plans/development-tasks.md`.
- Put version numbers, release notes, and main changelogs in GitHub tags and
  GitHub Releases.
- Put detailed procedures in `docs/`.

## Architecture Rules

- Keep product logic in Rust workspace crates, not in the UI shell.
- `crates/core` is the no-I/O base layer. Higher crates may depend on it; it
  must not depend on higher crates.
- UI work must call the typed Rust service protocol.
- SwiftUI/AppKit code stays in the native macOS shell and follows existing
  view/model/service patterns.
- App UI filters, scope pickers, sort/search controls, and navigation should
  derive from startup/manual-refresh cache and avoid expensive reads or scans by
  default. Fetch fresh data only for explicit refresh, startup prewarm, or
  consistency-bound flows such as config edit/write/rollback.
- Service behavior changes must keep `docs/service-protocol.md`, fixtures, and
  protocol drift verification in sync.
- Do not recreate `ui/`, `src-tauri/`, or Tauri IPC.

## Adapter Scope

- Supported adapter families: Claude Code, Codex, opencode, Pi, Hermes, and
  OpenClaw.
- Adapter scans may read only documented roots and explicitly configured local
  roots.
- Adapter writes are limited to the guarded toggle/install scopes documented in
  `docs/adapters/agent-adapters.md`.
- Network-backed installs outside the `skillManager.*` service path, scripts,
  credentials, cloud sync, telemetry, uncontrolled fetch, broad config writes,
  and release automation require a new scoped safety review. Skill Manager
  search/install/update may use the scoped external manager CLI path with
  command preview, target visibility, telemetry-off env, redaction, and explicit
  confirmation.

## Safety Boundaries

- No cloud sync, accounts, telemetry, anonymous crash reports, or uncontrolled
  outbound network calls.
- Optional LLM/provider features must be explicitly enabled by the user.
- Provider calls require prompt preview, redaction, destination visibility, and
  explicit confirmation.
- Credentials must prefer Keychain. Never write credentials to SQLite, project
  directories, logs, prompts, response artifacts, screenshots, or reports.
- LLM output is untrusted and copy-only unless a normal explicit user edit/save
  flow validates it.
- Skill scripts are untrusted. Script execution remains default-denied and must
  not be triggered by imports, LLM output, analyzer recommendations, previews,
  or cleanup guidance.
- Do not add hidden apply/write paths, hidden task state, raw
  prompt/response/trace persistence, public distribution automation, signing,
  notarization, DMG, or ZIP work unless explicitly scoped.

## Required Verification

- For small code changes, run focused checks for the touched area.
- For major changes, user-visible behavior, UI work, service protocol changes,
  or milestone completion, run `pnpm check:macos`.
- For docs that claim implementation status, screenshots, or validation
  results, run the relevant verifier or change the wording.
- Before committing, pushing, or handing off evidence, run `pnpm check:privacy`.
- Smoke validation uses fixture data and must not touch real user config.
- Real local validation uses the developer's real local HOME, app data, and
  agent configs.
- Completed UI screenshots must capture only the full app window. Full desktop
  screenshots are forbidden.
- If the macOS session is locked, cannot be confirmed interactive, or Computer
  Use/window capture is blocked, record the canonical blocker. Do not
  substitute a smoke screenshot for real local validation.

## Common Commands

```sh
cargo test --workspace
cargo clippy --workspace --all-targets --all-features
pnpm test:macos-native-models
swift test --package-path apps/macos
pnpm check:macos
pnpm check:privacy
pnpm verify:gate-parity
pnpm verify:service-protocol-drift
pnpm verify:module-size
pnpm verify:macos-ui-layout
pnpm smoke:macos-app -- --fixture-data --capture-window
pnpm dev:macos
```

## Read Before Editing

| Change area | Read first |
| --- | --- |
| Architecture | `docs/architecture.md` |
| Agent workflow / validation | `docs/ai-agent-workflow.md` |
| macOS run / smoke | `docs/runbooks/macos-app-runbook.md` |
| UI / screenshot standards | `docs/ui-delivery-standards.md` |
| Service protocol | `docs/service-protocol.md` |
| Data model | `docs/data-model.md` |
| Security / privacy | `docs/security-model.md` |
| Roadmap | `docs/plans/roadmap.md` |
| Active task ledger | `docs/plans/development-tasks.md` |
| Adapter scope | `docs/adapters/agent-adapters.md` |

## Git And Editing Rules

- Do not revert user changes unless explicitly asked.
- Keep edits scoped to the requested task and relevant architecture boundary.
- Prefer existing project patterns over new abstractions.
- Update docs when behavior, commands, architecture, validation flow, or UI
  state changes.
- Before committing, check the working tree and include only intended changes.
- For multi-agent parallel work, create one isolated git worktree and branch per
  task before assigning subagents. Subagents must stay in their assigned
  worktree, must not switch branches, and must not edit the coordinator
  checkout.
