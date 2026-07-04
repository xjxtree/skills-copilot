# Agent Copilot

[English](README.md) | [简体中文](README.zh-CN.md)

Agent Copilot is a native macOS control surface for inspecting local coding-agent
sessions, skills, configuration snapshots, and validation evidence without
expanding the repository's write, script, credential, cloud, or telemetry
surface.

## What It Does

- Shows local agent sessions, skill catalogs, and supported config snapshots.
- Uses a typed Rust JSON stdio service behind the native macOS app.
- Keeps local analysis deterministic by default.
- Gates optional provider calls behind preview, redaction, destination
  visibility, and explicit confirmation.
- Treats skill scripts, transcripts, LLM output, and config files as untrusted
  input.

## What It Does Not Do

- No cloud sync, accounts, telemetry, anonymous crash reports, or uncontrolled
  outbound network calls.
- No default provider calls.
- No hidden apply/write paths.
- No skill-script execution from scans, imports, previews, recommendations, or
  LLM output.
- No credential storage in project directories, SQLite, logs, prompts,
  screenshots, reports, or response artifacts.
- No Developer ID signing, notarization, DMG, updater, or release automation by
  default. The v0.1.1 macOS ZIPs are manually scoped release artifacts with
  ad-hoc signed app bundles.

## Download

Download the latest macOS app from the GitHub release page:

- [Agent Copilot v0.1.1](https://github.com/xjxtree/agent-copilot/releases/tag/v0.1.1)
- Apple Silicon ZIP:
  [AgentCopilot-0.1.1-macos-arm64.zip](https://github.com/xjxtree/agent-copilot/releases/download/v0.1.1/AgentCopilot-0.1.1-macos-arm64.zip)
- Intel ZIP:
  [AgentCopilot-0.1.1-macos-x86_64.zip](https://github.com/xjxtree/agent-copilot/releases/download/v0.1.1/AgentCopilot-0.1.1-macos-x86_64.zip)

Architecture note: choose `arm64` for Apple Silicon Macs and `x86_64` for Intel
Macs.

The v0.1.1 app is distributed as an ad-hoc signed and unnotarized macOS app
bundle inside a ZIP file. On first launch, macOS Gatekeeper may require explicit
user approval. Use Finder's **Open** action from the context menu, or approve the
app from **System Settings > Privacy & Security** if macOS blocks the first
launch.

## Use The App

1. Download the ZIP that matches your Mac architecture from the release page.
2. Unzip it and move `AgentCopilot.app` to `/Applications` or another local
   folder you control.
3. Open `AgentCopilot.app`.
4. Use **Scan** or project context controls inside the app to inspect supported
   local agent sessions, skill roots, and config snapshots.

Agent Copilot is local-first. It does not send provider requests unless optional
provider features are configured and a prompt is previewed and explicitly
confirmed.

Skill Package Manager workflows use the local `npx skills` manager and require a
local Node/npm install. The macOS app detects common Homebrew, Volta, asdf, and
nvm paths when launched from Finder; custom installs can set
`SKILLS_COPILOT_NPX_PATH`.

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

## Documentation

| File | Use |
| --- | --- |
| `AGENTS.md` | Agent-facing operating rules |
| `CLAUDE.md` | Claude Code-specific compatibility behavior |
| `docs/architecture.md` | Repository architecture |
| `docs/adapters/agent-adapters.md` | Adapter roots, write scopes, and blocked operations |
| `docs/service-protocol.md` | Typed service method contract |
| `docs/security-model.md` | Security and privacy rules |
| `docs/data-model.md` | Persisted and transient data model |
| `docs/ai-layer.md` | Provider and LLM safety boundary |
| `docs/ui-delivery-standards.md` | UI and screenshot validation standards |
| `docs/plans/roadmap.md` | Future work and non-goals |
| `docs/plans/development-tasks.md` | Active task routing |
| `CHANGELOG.md` | Versioned release-impact notes |
| `docs/verification/` | Version checklists and benchmark trends |

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
