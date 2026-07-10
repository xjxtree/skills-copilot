# Contributing to Agent Copilot

Agent Copilot uses a Rust core, a typed service protocol, and a native macOS
SwiftUI/AppKit shell. The native shell lives in `apps/macos`; the retired
Tauri/React shell must not be recreated.

## Current Contribution Scope

Good contributions right now:

- Fix contradictions, unclear wording, or stale assumptions in `docs/`
- Improve verified adapter specs and fixtures for Claude Code, Codex, opencode, Pi, Hermes, or OpenClaw
- Provide real sample layouts and config files for supported agents
- Improve tests, docs, service protocol fixtures, and native macOS UI behavior
- Help define service protocol fixtures for native and future desktop shells
- Help harden the native macOS shell described in `docs/architecture.md` and `docs/runbooks/macos-app-runbook.md`
- Improve native validation assets and app-window-only screenshots
- Add narrowly scoped implementation PRs that preserve the architecture,
  adapter, security, and write boundaries

## Design Rules

- `docs/` describe the intended architecture and must be kept in sync with implementation changes.
- The current adapter families are Claude Code, Codex, opencode, Pi, Hermes, and OpenClaw.
- Claude Code and Codex have verified scan/toggle boundaries; Codex writes stay limited to the documented user/project `.agents` compatibility path.
- opencode scans native, compatibility, and configured local `skills.paths` roots; writes are limited to verified managed `permission.skill` overrides and native install roots.
- Pi scans native plus `.agents` compatibility roots; guarded toggles and native-root installs are supported inside the documented boundaries.
- Hermes and OpenClaw have native/workspace install and guarded config-toggle slices only where documented; broader config edits and network-backed operations remain blocked.
- Do not guess agent directory layouts or config keys.
- No cloud sync, telemetry, or automatic crash reporting.
- Provider-backed AI support is implemented but remains explicit, preview/redaction/confirmation-gated, destination-visible, and copy-only unless a normal user edit/save flow validates output.
- Do not bind new core behavior to one UI shell. Rust service logic should be callable from native macOS now and future Windows/Linux shells later.
- Do not recreate `ui/`, `src-tauri/`, or Tauri IPC for product work.
- The macOS native UI should use SwiftUI/AppKit system components first. Liquid Glass is a functional material for navigation, toolbars, inspectors, popovers, and command bars; do not use it as decorative blur over dense content.
- Follow `docs/ui-delivery-standards.md`: major versions/features need UI prototypes before implementation; finished UI work needs completed screenshots; code changes must be verified by launching and operating the macOS app with macOS Computer Use.

## Agent Spec Evidence

When contributing an adapter spec, include:

- Official documentation link, if available
- Exact skill directory layout
- Exact config file path and schema, if any
- Enable/disable semantics
- One minimal fixture: global skill, project skill, and any related config
- Notes about precedence, live reload, and unsupported platforms

If only local evidence exists, label it as a local sample and include the agent version or commit.

## Pull Request Checklist

This checklist is a reusable PR template. Unchecked items here are intentional and do not represent current roadmap progress.

Before opening a PR:

- [ ] I read `README.md`, `CLAUDE.md`, and all files under `docs/`
- [ ] I kept implementation changes narrowly scoped and covered by tests where risk warrants it
- [ ] Any new agent behavior is backed by a source or fixture
- [ ] I updated related docs when changing scope, lifecycle, or security behavior
- [ ] I checked for broken internal links by inspection
- [ ] I kept the architecture, adapter, security, and write boundaries intact
- [ ] For code changes, I launched and operated the macOS app with macOS Computer Use, or documented the blocker
- [ ] For UI changes, I updated prototype and completed UI artifacts under `docs/ui-artifacts/`

## Style

- Prefer concise, implementation-ready wording.
- Use stable terms from `docs/data-model.md`.
- Keep examples small and reproducible.
- Use English for future code identifiers; Chinese documentation is fine.

## Testing

- `cargo test --workspace` must pass before opening a PR.
- `cargo clippy --workspace --all-targets --all-features` target: 0 warnings.
- `pnpm verify:gate-parity` must pass for protocol/docs/gate governance changes.
- `pnpm check:privacy` must pass before committing validation evidence, screenshots, or privacy-sensitive docs.
- `pnpm verify:macos-launch` should pass for native macOS UI/service changes; this builds the Rust sidecar, builds SwiftPM, assembles `dist/AgentCopilot.app`, launches it, and verifies a visible window.
- Run macOS Computer Use against `dist/AgentCopilot.app` for affected runtime flows and record the result under `docs/ui-artifacts/` when UI behavior changes.
- `pnpm build:macos` builds the bundle without launching or stopping an existing app. Pair it with `pnpm smoke:macos-app -- --fixture-data --headless-sidecar` for fixture-only sidecar validation that needs no GUI.
- `pnpm smoke:macos-app -- --fixture-data --capture-window` remains the local fixture app/window check and must not touch real config.
- `pnpm smoke:macos-app -- --fixture-data --check-logs` should pass before macOS release candidates to validate deterministic fixture launch and unknown app error/fault filtering.
- `pnpm benchmark:10k` and `pnpm benchmark:macos-list-model` should be rerun before performance-sensitive releases.
- CI builds the app without launching it, runs both complete native-test entrypoints, and validates the bundled Rust sidecar with fixture data in `--headless-sidecar` mode. This is not real-local GUI evidence.
- New adapters must include at least 3 fixture tests (happy path + broken cases).

## Code layout

- Native macOS UI belongs under `apps/macos/`, should call the shared Rust service protocol, and should not import or depend on UI shell internals.
- All Rust crates use `thiserror` for error enums; no `anyhow!` at crate boundaries.
