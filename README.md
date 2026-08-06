# Agent Copilot

[English](README.md) | [简体中文](README.zh-CN.md)

Agent Copilot is a native macOS workspace for inspecting and managing local
coding-agent data. It supports Claude Code, Codex, opencode, Pi, Hermes, and
OpenClaw.

## What It Does

- Finds local skills within documented agent roots.
- Browses redacted local session summaries and messages.
- Searches skills, sessions, configuration entries, and detail pages.
- Reviews configuration and applies supported guarded actions.
- Manages local skill packages through explicit preview and confirmation flows.
- Shows redacted usage signals for Agent Copilot's optional AI features.
- Keeps lists and navigation responsive through startup and manual-refresh
  caches.

Agent Copilot is local-first. It does not add cloud sync, accounts, telemetry,
or uncontrolled network access. Credentials prefer Keychain, skill scripts are
untrusted and default-denied, and provider calls require explicit user review
and confirmation.

## Architecture

- Rust workspace crates own product logic, scanning, adapters, persistence, and
  the typed service protocol.
- The native SwiftUI/AppKit shell presents state and sends typed service
  requests.
- Adapter reads and writes stay within documented local roots and guarded
  actions.
- The removed Tauri/React shell is not part of the project.

## Build

Requirements: macOS, Xcode Command Line Tools, Rust, Node.js, Corepack, and
pnpm.

```sh
git clone https://github.com/xjxtree/agent-copilot.git
cd agent-copilot
corepack enable
pnpm install
pnpm build:macos
open dist/AgentCopilot.app
```

For active development:

```sh
pnpm dev:macos
```

Skill Package Manager operations also require a local Node/npm installation
because they use the local `npx skills` manager.

## Documentation

| File | Purpose |
| --- | --- |
| `AGENTS.md` | Shared coding-agent workflow and safety rules |
| `docs/README.md` | Documentation index |
| `docs/architecture.md` | Architecture and ownership |
| `docs/data-model.md` | Persisted and transient data |
| `docs/adapters/agent-adapters.md` | Adapter roots and guarded operations |
| `docs/service-protocol.md` | App/service contract |
| `docs/security-model.md` | Security and privacy boundaries |
| `docs/ui-delivery-standards.md` | Native UI standards |
| `docs/runbooks/macos-app-runbook.md` | Local build and run commands |

See [CONTRIBUTING.md](CONTRIBUTING.md) before submitting a change.

## License

[MIT](LICENSE)
