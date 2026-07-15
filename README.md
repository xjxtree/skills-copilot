# Agent Copilot

[English](README.md) | [简体中文](README.zh-CN.md)

Agent Copilot is a native macOS app for people who work with multiple coding
agents. It brings local sessions, skill packages, configuration snapshots, and
app AI provider usage signals into one focused desktop workspace, so you can
inspect what is installed, search across local agent data, and manage common
workflows without jumping between hidden folders and terminal output.

## Project Overview

- **Platform:** native macOS desktop app.
- **Latest release:** use the
  [GitHub Releases](https://github.com/xjxtree/agent-copilot/releases/latest)
  page for the current app download, release notes, and checksums.
- **Supported agent families:** Claude Code, Codex, opencode, Pi, Hermes, and
  OpenClaw.
- **Primary use cases:** skill catalog review, local session lookup,
  configuration inspection, app AI provider usage review, project context
  management, and skill package workflows.
- **Distribution:** architecture-specific macOS ZIP downloads for Apple Silicon
  and Intel Macs.

## Architecture

Agent Copilot is organized as a local-first desktop product:

- The macOS app provides the main navigation, detail views, settings, and
  workflow panels.
- A local processing layer handles scanning, catalog updates, session previews,
  configuration reads, and package-manager operations.
- Local caches keep the app responsive while preserving explicit refresh
  controls for heavier scans.
- The repository includes focused fixtures and validation scripts so app,
  service, and documentation changes can be checked together before release.

This split keeps the user experience native and fast while leaving the
agent-specific parsing and workflow rules in shared project code.

## App Features

OpenAI's current desktop experience hosts Codex inside the ChatGPT app. Agent
Copilot keeps `codex` as the stable adapter identity, continues to read the
same safe `$CODEX_HOME` configuration and local session stores, and recognizes
skills delivered through ChatGPT's local plugin cache. Existing Codex projects
and configuration do not need to be renamed for Agent Copilot.

- **Skills:** scan supported agent roots, filter by agent/scope/status, inspect
  metadata, review findings, and enable or disable supported local skills.
  ChatGPT plugin-cache skills show their publisher, package, version, and
  read-only ownership in the detail view.
- **Sessions:** browse local Claude Code, Codex, opencode, and Pi session
  previews; search within supported history; open a selected session with
  redacted message and skill-usage summaries.
- **Global search:** search from the app toolbar and jump directly to matching
  skills, sessions, configuration entries, or detail pages.
- **Configuration:** review supported agent config snapshots, inspect current
  files, preview rollback changes, and apply guarded config actions where the
  app supports them.
- **Skill Package Manager:** search, preview, install, update, remove, and
  create local skill packages through the local `npx skills` manager.
- **Provider Observability:** view read-only usage summaries for AI requests
  made by Agent Copilot's own optional AI features, including model activity,
  latency, token estimates, and cost estimates over selectable date ranges. It
  does not report usage from provider profiles configured inside managed
  agents such as Claude Code, Codex, opencode, or Pi.
- **Task Preflight:** paste a task and review local readiness, matching skills,
  Agent Copilot AI provider context, and diagnostic notes before taking action.
- **Project Context:** pin or clear the current project root so app lists,
  searches, and previews stay aligned with the workspace you are reviewing.
- **Appearance:** follow the system appearance automatically, or choose light or
  dark mode in Settings.

## Download And Use

Download the latest macOS app from the
[GitHub Releases](https://github.com/xjxtree/agent-copilot/releases/latest)
page. Each release provides architecture-specific ZIP files and checksum
information.

Choose `arm64` for Apple Silicon Macs and `x86_64` for Intel Macs.

1. Download the ZIP for your Mac architecture.
2. Unzip it and move `AgentCopilot.app` to `/Applications` or another local
   folder you control.
3. Open `AgentCopilot.app`.
4. Use the sidebar to review skills, sessions, config, Agent Copilot AI
   provider activity, and settings.

If macOS blocks the first launch, open the app from Finder's context menu or
approve it in **System Settings > Privacy & Security**.

Skill Package Manager workflows require a local Node/npm install because they
use the local `npx skills` manager. The app detects common Homebrew, Volta,
asdf, and nvm paths when launched from Finder. Custom installs can set
`SKILLS_COPILOT_NPX_PATH`.

ChatGPT's Plugin Directory and Agent Copilot's Skill Package Manager are
separate sources. Agent Copilot only inventories ChatGPT's existing plugin
cache; it does not install, update, or remove ChatGPT plugins. Writable package
operations remain in the explicit, previewed `npx skills` workflow.

## Build From Source

Prerequisites:

- macOS 13 or newer.
- Xcode Command Line Tools.
- Rust toolchain with Cargo.
- Node.js with Corepack/pnpm.

Build and run the macOS app:

```sh
git clone https://github.com/xjxtree/agent-copilot.git
cd agent-copilot
corepack enable
pnpm install
pnpm build:macos
open dist/AgentCopilot.app
```

Build an architecture-specific app bundle without launching it:

```sh
pnpm build:macos:arm64
rustup target add x86_64-apple-darwin
pnpm build:macos:x86_64
```

Run the main local validation gates:

```sh
pnpm check:macos
pnpm check:privacy
```

## Documentation Guide

| File | Use |
| --- | --- |
| `docs/README.md` | Documentation index and ownership guide |
| `docs/architecture.md` | Repository architecture and code ownership |
| `docs/data-model.md` | Persisted and transient data overview |
| `docs/adapters/agent-adapters.md` | Supported agent roots, config behavior, and adapter scope |
| `docs/service-protocol.md` | Service method contract for app/service integration |
| `docs/security-model.md` | Security, privacy, credential, and local data rules |
| `docs/ai-layer.md` | Optional provider workflow boundary |
| `docs/ui-delivery-standards.md` | Native UI and screenshot validation standards |
| `docs/runbooks/macos-app-runbook.md` | Local macOS build, run, and smoke validation flow |
| `docs/runbooks/release-checklist.md` | Maintainer release-readiness checklist |
| `docs/plans/roadmap.md` | Future work and deferred scope |
| `docs/plans/development-tasks.md` | Active task routing |
| `AGENTS.md` | Coding-agent operating rules for this repository |

## Contributing

Contributions are welcome. For a smooth review:

- Read `AGENTS.md` and the relevant docs before editing.
- Keep changes scoped to the current app surface and existing architecture.
- Update fixtures and `docs/service-protocol.md` when service behavior changes.
- Run focused checks for small changes and `pnpm check:macos` for UI,
  protocol, or publishing work.
- Run `pnpm check:privacy` before committing or publishing evidence.
- Include the commands you ran in your PR or handoff notes.

## License

Agent Copilot is released under the [MIT License](LICENSE).
