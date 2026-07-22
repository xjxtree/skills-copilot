# AI Agent Workflow

Shared workflow for Codex, Claude Code, Pi, opencode, and other coding agents
used on this repository.

## Instruction File Layout

`AGENTS.md` is the canonical shared entrypoint for coding agents.

`CLAUDE.md` is a Claude Code compatibility layer. It imports `AGENTS.md` and only adds Claude Code-specific behavior.

Do not duplicate the full project rules across multiple instruction files. Put shared rules in `AGENTS.md`, tool-specific behavior in that tool's compatibility file, and detailed procedures in `docs/`.

Layout:

```text
AGENTS.md                         # Shared agent entrypoint
CLAUDE.md                         # Claude Code compatibility layer
docs/ai-agent-workflow.md         # Multi-agent workflow and validation rules
docs/runbooks/macos-app-runbook.md # macOS app run, smoke, and bundle freshness rules
docs/ui-delivery-standards.md     # UI structure and Computer Use rules
```

## Compatibility Notes

- Codex reads `AGENTS.md` before work and supports global, project, and nested
  project instruction files. The current ChatGPT desktop app hosts Codex work,
  but repository instructions, `$CODEX_HOME/config.toml`, project
  `.codex/config.toml`, and the stable `codex` adapter identity remain the
  compatibility boundary for this repository.
- Claude Code reads `CLAUDE.md`; when a project also uses `AGENTS.md`, Claude's official recommendation is to import it from `CLAUDE.md`.
- opencode reads `AGENTS.md` and falls back to `CLAUDE.md` only when `AGENTS.md` is absent.
- Pi's default resource loader discovers context files named `AGENTS.md` from the current working directory and global agent directory.

References:

- OpenAI Codex AGENTS.md: <https://developers.openai.com/codex/guides/agents-md>
- Claude Code memory / CLAUDE.md: <https://code.claude.com/docs/en/memory>
- opencode rules: <https://open-code.ai/en/docs/rules>
- Pi SDK context file discovery: <https://pi.dev/docs/latest/sdk>
- AGENTS.md format: <https://agents.md>

## Source of Truth

Use this priority order when information conflicts:

1. Code and scripts that have been inspected or executed.
2. Current user instructions in the active task.
3. `AGENTS.md` and `CLAUDE.md`.
4. Focused docs under `docs/`.
5. README and higher-level summary docs.

If docs conflict with code, fix the docs or code as part of the task when the requested scope allows it. Do not silently proceed with stale documentation.

## Documentation Ownership Matrix

Use this table before adding or moving documentation. Repository docs contain
current contracts and reusable procedures, not task state or validation logs.

| Document | Primary audience | Put here | Do not put here |
| --- | --- | --- | --- |
| `AGENTS.md` | AI coding agents | Shared rules, hard boundaries, validation expectations, compact gate anchors | Plans, task state, changelogs, release notes |
| `CLAUDE.md` | Claude Code | Claude-specific behavior, Computer Use defaults | Shared project rules already in `AGENTS.md` |
| `README.md` | Humans | Product overview, app features, download/build guide, document map | Validation dumps, plans, or task ledgers |
| `docs/README.md` | Humans + agents | Focused documentation index and ownership navigation | Product marketing, task state, or duplicated contracts |
| GitHub issues and pull requests | Contributors | Plans, task state, decisions, command results, and handoffs | Durable product contracts |
| GitHub Releases/tags | Humans + maintainers | Release versions, user-facing release notes, downloadable assets, checksums | Planning and implementation details |
| Focused specs (`docs/service-protocol.md`, adapter specs, security/data/AI docs) | Implementers | Current contracts and domain-specific rules | Progress reports or dated validation results |

## Validation Rules

Use focused validation for small code changes. Use full macOS validation for larger or user-visible work.

| Change type | Required validation |
| --- | --- |
| Planning discussion in an issue or pull request | None |
| Docs-only contract change | `git diff --check` and `pnpm verify:doc-governance` |
| Rust logic or service protocol change | Focused Rust tests; for service-visible behavior, `pnpm check:macos` |
| Native macOS UI change | `pnpm check:macos` plus real Local App Run when Computer Use is available |
| Major or user-visible change | `pnpm check:macos` plus real Local App Run |
| Runtime screenshot | App-window-only capture outside the repository; full desktop screenshots are forbidden |
| Privacy-sensitive docs or repository cleanup | `pnpm check:privacy` |

`verify:macos-ui-layout` is intentionally reached through `pnpm check:macos`
instead of `pnpm verify:gate-parity`, because it is a native UI layout guard
rather than a protocol/docs parity gate.

## Smoke App Run vs Local App Run

Smoke App Run is an automated fixture-data regression check:

```sh
pnpm smoke:macos-app -- --fixture-data --capture-window
```

It validates the existing `dist/AgentCopilot.app` with temporary HOME, temporary app data, synthetic Claude skills/settings, and window-only screenshot capture. It must not touch real user config.

Local App Run is the real environment check:

```sh
pnpm dev:macos
```

It rebuilds and launches `dist/AgentCopilot.app` with the developer's real local HOME, app data, and Claude config. Use this to inspect actual product behavior and visual quality.

For major, user-visible, UI, or service protocol work, run both in this order:

```sh
pnpm check:macos
pnpm dev:macos
```

Then operate the real app with macOS Computer Use when the macOS session is confirmed unlocked and interactive.

A valid `get_app_state` result for the target app window is enough to proceed.
If it returns `remoteConnection`, `cgWindowNotFound`, `timeoutReached`,
an activation error, or another non-interactive signal, stop Computer Use
attempts for that pass and record the canonical blocker.
Use `pnpm classify:validation-blocker -- "<tool output>"` when the raw tool
text is ambiguous.
If Computer Use can observe the window but a specific action primitive fails,
record that tool-action limitation and use another macOS AX path only when
each operation is followed by Computer Use state read-back.

## Screenshot Rules

Use the temporary default output:

```sh
pnpm capture:macos-window
```

The capture helper and fixture smoke write to `/tmp`. Do not point them into the
repository.

Only complete app-window captures are allowed. Full desktop screenshots are forbidden.

If the macOS session is locked, cannot be confirmed interactive, or Computer Use cannot resolve the app window, report real-local validation as blocked in the task or pull request. Do not replace it with a smoke screenshot.

Inspect screenshots visually, keep them outside the repository, and run before handoff:

```sh
pnpm check:privacy
```

Screenshots and task output must not expose real local usernames, home paths,
app-data paths, temp directories, credentials, tokens,
or proxy-managed credential placeholders.
Use placeholders such as `$HOME`, `<repo>`, `<worktree>`,
`<project-root>`, `<app-data-dir>`, and `<redacted>`.

## Multi-Agent Use

When another coding agent is used:

- Do not assume subagents are isolated by default. Isolation is a coordinator responsibility.
- Create an isolated git worktree and branch for each parallel task before starting the agent.
- Start the agent from its assigned worktree root unless the task intentionally targets a subdirectory with scoped instructions.
- Ask it to read `AGENTS.md` and the task-relevant docs before editing.
- Tell it to work only in the assigned worktree, not switch branches, not edit other worktrees, and not touch known unrelated dirty files.
- Require concrete validation output, not only a prose summary.
- Require a list of changed files and any remaining blockers.
- Re-check its changes against code, docs, and current project rules before committing.

### Parallel Worktree Procedure

Use this sequence for multi-agent parallel development:

Do this before dispatch, not after a worker has already begun. Each subtask gets exactly one isolated worktree plus one branch, and that worker must stay inside that assigned checkout for the whole task.

1. Inspect the coordinator checkout first:

```sh
git status --short --branch
git worktree list --porcelain
```

2. Decide the task split so each agent has a disjoint write set. Prefer splits by ownership area, such as docs, SwiftUI app, Rust service, security hardening, or adapter evidence.

3. Create the branch and worktree in the coordinator shell before assigning the task:

```sh
git worktree add -b gd-ops/<task-name> /path/to/agent-copilot-<task-name> main
```

Use an existing branch instead of `-b` only when intentionally resuming that branch:

```sh
git worktree add /path/to/agent-copilot-<task-name> gd-ops/<task-name>
```

4. Put the assigned worktree path, branch, allowed write set, and validation command in the agent prompt. Explicitly say:

```text
Only work in /path/to/assigned-worktree.
Do not switch branches.
Do not edit other worktrees.
Do not touch unrelated dirty files.
You are not alone in the codebase; do not revert changes made by others.
```

5. After assigning agents, immediately verify isolation:

```sh
git worktree list --porcelain
git -C /path/to/assigned-worktree status --short --branch
```

6. If an agent starts in the coordinator checkout or switches the shared checkout, interrupt it immediately. Ask it to stop, report any edits, and do not continue until a clean isolated worktree has been created.

7. When a worker finishes, inspect its boundary before integration:

```sh
git -C /path/to/assigned-worktree status --short --branch
git -C /path/to/assigned-worktree diff --name-status
```

Then review the diff, run the relevant validation, and only commit, push, or merge after confirming the change stayed within its assigned ownership.

Use this handoff shape for non-trivial work:

```text
Task:
Worktree and branch:
Changed files:
Implementation summary:
Validation commands and results:
Computer Use result or blocker:
Known blockers:
Docs updated:
Commit hash, if committed:
```

For adapter research or implementation, cite the relevant section of `docs/adapters/agent-adapters.md` or the adapter-specific spec and state whether the adapter is blocked, read-only, guarded writable, or install-only.

## Documentation Sync

Update docs when any of the following changes:

- App run commands or validation flow.
- Architecture boundaries.
- Service protocol behavior.
- UI behavior or validation flow.
- Adapter scope or verified external agent specs.

Keep README focused on human navigation. Keep `AGENTS.md` focused on rules that every coding agent must follow. Keep detailed procedures in `docs/`.
