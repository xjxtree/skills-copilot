# Agent Copilot

[English](README.md) | [简体中文](README.zh-CN.md)

Agent Copilot is a native macOS readiness and continuity control center for
people who work with multiple coding agents. It helps you verify what each agent
can effectively use in the selected project, understand what needs attention,
and continue local work from where it stopped.

## Project Overview

- **Platform:** native macOS desktop app.
- **Latest release:** use the
  [GitHub Releases](https://github.com/xjxtree/agent-copilot/releases/latest)
  page for the current app download, release notes, and checksums.
- **Supported agent families:** Claude Code, Codex, opencode, Pi, Hermes, and
  OpenClaw.
- **Primary use cases:** project/agent environment verification, effective
  skill review, local session continuity, evidence-backed repair, and guarded
  skill/config workflows.
- **Distribution:** architecture-specific macOS ZIP downloads for Apple Silicon
  and Intel Macs.

## Architecture

Agent Copilot is organized as a local-first desktop product:

- Bounded adapters and scanners read only documented local agent sources.
- Rust product logic derives project health, skill state, session evidence,
  and guarded actions through a typed service protocol.
- The native macOS app presents project, skill, session, settings, and workflow
  views without reimplementing adapter policy.
- Local caches keep the app responsive while preserving explicit refresh
  controls for heavier scans.
- Optional AI interprets redacted, revision-bound evidence after prompt preview
  and explicit confirmation; it is never the source of environment truth or
  write authority.
- The repository includes focused fixtures and validation scripts so app,
  service, and documentation changes can be checked together before release.

This split keeps the user experience native and fast while leaving the
agent-specific parsing and workflow rules in shared project code.

The durable product and interaction contract is
[`docs/product-design.md`](docs/product-design.md). Engineering work for that
contract follows the numeric dependency order in
[`docs/implementation-sequence.md`](docs/implementation-sequence.md); execution
state and validation results stay in the active task conversation, GitHub issue,
or pull request.

## App Features

OpenAI's current desktop experience hosts Codex inside the ChatGPT app. Agent
Copilot keeps `codex` as the stable adapter identity, continues to read the
same safe `$CODEX_HOME` configuration and local session stores. Skill inventory
comes from persisted `SKILL.md` files, including manifest-declared skill roots
of enabled, installed Codex plugins. The plugin store/cache is never walked as
a generic source, and physical cache paths are hidden. Agent Copilot does not query
the Codex runtime for skill inventory. Existing Codex projects and configuration
do not need to be renamed for Agent Copilot.

- **Project context:** select the workspace whose agent evidence, skills,
  sessions, search, and config history should be inspected. Recent projects can
  be removed individually or cleared without silently changing the active
  project.
- **Skills:** scan supported agent roots, distinguish installed, enabled, and
  effective state, filter by agent/scope/status, inspect evidence and findings,
  and toggle supported local skills. Filesystem-discovered skills retain source
  and read-only ownership information; installed Codex plugin copies are
  visible but never writable.
- **Sessions:** browse local session previews for supported adapters, search
  within bounded history, and inspect redacted user/final-agent messages,
  timing, source coverage, and skill-usage summaries.
- **Global search:** search from the app toolbar and jump directly to matching
  skills, sessions, configuration entries, or detail pages.
- **Readiness:** deterministic local evidence reports environment coverage;
  task-specific preflight can rank matching effective skills without claiming
  universal readiness when no task is supplied.
- **Configuration and evidence:** review supported agent config snapshots,
  inspect current files and diagnostics, preview rollback changes, and apply
  guarded config actions where the app supports them.
- **Skill Package Manager:** search, preview, install, update, remove, and
  create local skill packages through the local `npx skills` manager.
- **Provider Observability:** view read-only usage summaries for AI requests
  made by Agent Copilot's own optional AI features, including model activity,
  latency, token estimates, and cost estimates over selectable date ranges. It
  does not report usage from provider profiles configured inside managed
  agents such as Claude Code, Codex, opencode, or Pi.
- **Optional contextual AI:** preview and explicitly confirm redacted Agent
  Copilot requests that explain or rank existing local evidence. Output remains
  untrusted and copy-only and cannot authorize writes or execute scripts.
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
4. Select a project and review its local skills, sessions, configuration
   evidence, Agent Copilot AI provider activity, and settings.

If macOS blocks the first launch, open the app from Finder's context menu or
approve it in **System Settings > Privacy & Security**.

Skill Package Manager workflows require a local Node/npm install because they
use the local `npx skills` manager. The app detects common Homebrew, Volta,
asdf, and nvm paths when launched from Finder. Custom installs can set
`SKILLS_COPILOT_NPX_PATH`.

ChatGPT's Plugin Directory and Agent Copilot's Skill Package Manager are
separate sources. Agent Copilot reads only enabled-plugin records and their
manifest-declared local skill files. It does not treat the plugin store/cache as
a general source, or install, update, remove, or execute plugin content. Writable package operations
remain in the explicit, previewed `npx skills` workflow.

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
| `docs/product-design.md` | Product promise, ontology, information architecture, truth model, and acceptance contract |
| `docs/implementation-sequence.md` | Required engineering task order, boundaries, exit criteria, and verification |
| `docs/architecture.md` | Repository architecture and code ownership |
| `docs/data-model.md` | Persisted and transient data overview |
| `docs/adapters/agent-adapters.md` | Supported agent roots, config behavior, and adapter scope |
| `docs/service-protocol.md` | Service method contract for app/service integration |
| `docs/security-model.md` | Security, privacy, credential, and local data rules |
| `docs/ai-layer.md` | Optional provider workflow boundary |
| `docs/ui-delivery-standards.md` | Native UI, interaction, and runtime validation standards |
| `docs/runbooks/macos-app-runbook.md` | Local macOS build, run, and smoke validation flow |
| `docs/runbooks/release-checklist.md` | Maintainer release-readiness checklist |
| `AGENTS.md` | Coding-agent operating rules for this repository |

## Contributing

Contributions are welcome. For a smooth review:

- Read `AGENTS.md` and the relevant docs before editing.
- For product-rebuild work, follow `docs/implementation-sequence.md` in numeric
  order and keep task state in the issue or pull request.
- Keep changes scoped to the current app surface and existing architecture.
- Update fixtures and `docs/service-protocol.md` when service behavior changes.
- Run focused checks for small changes and `pnpm check:macos` for UI,
  protocol, or publishing work.
- Run `pnpm check:privacy` before committing or pushing changes.
- Include the commands you ran in your PR or handoff notes.

## License

Agent Copilot is released under the [MIT License](LICENSE).
