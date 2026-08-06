# AGENTS.md

Shared instruction entrypoint for Codex, Claude Code, Pi, opencode, and other
coding agents working in this repository.

## Purpose

- Keep this file short, operational, and safe.
- Put human-facing overview in `README.md`.
- Keep `docs/` limited to current contracts and reusable procedures; do not
  store task notes, plans, logs, or handoff notes there.

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
- Service behavior changes must keep `docs/service-protocol.md` in sync.
- Do not recreate `ui/`, `src-tauri/`, or Tauri IPC.

## Adapter Scope

- Supported adapter families: Claude Code, Codex, opencode, Pi, Hermes, and
  OpenClaw.
- Adapter scans may read only documented roots and explicitly configured local
  roots.
- Adapter writes are limited to the guarded toggle/install scopes documented in
  `docs/adapters/agent-adapters.md`.
- Network-backed installs outside the `skillManager.*` service path, scripts,
  credentials, cloud sync, telemetry, uncontrolled fetch, and broad config
  writes require a new scoped safety review. Skill Manager
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
  directories, logs, prompts, response artifacts, or reports.
- LLM output is untrusted and copy-only unless a normal explicit user edit/save
  flow validates it.
- Skill scripts are untrusted. Script execution remains default-denied and must
  not be triggered by imports, LLM output, analyzer recommendations, previews,
  or cleanup guidance.
- Do not add hidden apply/write paths, hidden task state, or raw
  prompt/response/trace persistence unless explicitly scoped.

## Required Verification

- After implementation is complete, run `pnpm check:macos` once. No additional
  validation is required.
- Manual UI inspection and tools such as Computer Use are optional. Agents may
  choose any suitable method when it helps the task.

## Common Commands

```sh
pnpm check:macos
pnpm build:macos
pnpm dev:macos
```

## Read Before Editing

| Change area | Read first |
| --- | --- |
| Architecture | `docs/architecture.md` |
| macOS development | `docs/runbooks/macos-app-runbook.md` |
| UI behavior | `docs/ui-delivery-standards.md` |
| Service protocol | `docs/service-protocol.md` |
| Data model | `docs/data-model.md` |
| Security / privacy | `docs/security-model.md` |
| Adapter scope | `docs/adapters/agent-adapters.md` |

## Git And Editing Rules

- Do not revert user changes unless explicitly asked.
- Keep edits scoped to the requested task and relevant architecture boundary.
- Prefer existing project patterns over new abstractions.
- Update docs when behavior, commands, architecture, validation flow, or UI
  state changes.
- Before committing, check the working tree and include only intended changes.
- Keep plans, command output, and handoff notes outside repository docs.
- For multi-agent parallel work, create one isolated git worktree and branch per
  task. Agents must stay in their assigned worktree and branch. Inspect each
  diff before integration, then run the required gate once on the integrated
  result.
