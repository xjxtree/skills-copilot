# Changelog

This file keeps versioned release-impact notes. Longer historical notes are
available from git history when needed.

## Unreleased

- No unreleased public-release notes yet.

## 0.1.0 - 2026-07-04

- First manually scoped public macOS ZIP release for Agent Copilot, with
  architecture-specific `arm64` and `x86_64` release assets.
- Ships `AgentCopilot.app` version `0.1.0` with the Rust stdio service bundled
  behind the native macOS shell.
- Includes local-first agent session, skill catalog, config snapshot, validation
  evidence, and guarded optional provider preview surfaces.
- Keeps safety boundaries in force: no telemetry, cloud sync, default provider
  calls, hidden apply/write paths, credential persistence in project files, or
  skill-script execution from scans/previews.
- Distribution note: the v0.1.0 ZIPs are unsigned and unnotarized; Developer ID
  signing, notarization, DMG packaging, updater feeds, and release automation
  remain future scoped work.

## Internal V2.98 Line

- Added the `skillManager.*` service protocol surface for manager-backed skill
  search, list, install, remove, update, local template creation, and guarded
  local deletion across Claude Code, Pi, opencode, Codex, Hermes, and OpenClaw.
- Scoped `npx skills` as the first writable Skill Manager tool: symlink
  distribution is default, copy is opt-in, network-backed search/install/update
  require command preview and confirmation, and `skills-npm` is discovery-only
  for now.
- Updated the Skill Manager UI so deletion removes manager-installed skill
  links from the selected agent targets; agent config enable/disable stays out
  of the Skill Manager panel.
- Keep V2.98 safety boundaries in force: no provider default calls, hidden
  writes, script execution, credential persistence, cloud sync, telemetry, or
  signing/release automation without a new scoped version.
- Current documentation work may move or compress historical prose, but must
  not change public API, service protocol methods, Swift/Rust wire types, or
  fixture JSON wire shape.

## V2.98

- Added automatic local session discovery for supported Claude Code, Codex,
  opencode, and Pi local session stores when no explicit roots are supplied.
- Added redacted, bounded `content_items` and `skill_usage_rows` to session
  preview results.
- Kept Hermes/OpenClaw session parsing deferred until confirmed session-store
  evidence exists.

## V2.97

- Added the Agent Config center and guarded Hermes/OpenClaw config toggles.
- Hermes writes remain limited to global `skills.disabled`.
- OpenClaw writes remain limited to `skills.entries.<key>.enabled` with JSON5
  input parsing, strict JSON write-back, snapshot/read-back, and rollback.

## V2.96

- Added OpenClaw native/workspace skill install support for confirmed local
  `SKILL.md` records.
- Kept `.agents` roots scan-only and blocked ClawHub, Git, update, verify,
  workshop, network-backed operations, scripts, credentials, cloud sync, and
  telemetry.

## V2.95

- Added Hermes native-root install support for confirmed local `SKILL.md`
  copies into `~/.hermes/skills`.
- Kept Hermes project installs, config toggle expansion, hub/URL/tap/update/
  uninstall/reset operations, scripts, credentials, cloud sync, telemetry, and
  uncontrolled network fetch blocked.

## V2.94

- Added Pi native and `.agents` compatibility-root scanning plus guarded native
  and compatibility settings toggles.
- Kept Pi package install/remove and `.agents` direct skill-file installs
  blocked.

## V2.93

- Added opencode configured local `skills.paths` scanning with
  canonicalization/dedupe.
- Kept `skills.urls` metadata-only/no-fetch and installs limited to native
  roots.

## V2.92

- Added Codex expanded read-only roots and a native `.agents/skills` write
  allowlist.
- Kept plugin/admin/system roots and project `.codex/config.toml` diagnostics
  read-only.

## V2.91

- Added redacted model-task history via `llm.listModelTaskMatches`,
  `llm.recordModelTaskMatch`, and `llm.deleteModelTaskMatch`.
- Kept provider observability read-only for `model_task_history_rows`.

## V2.90

- Migrated packaged app identity to `dist/AgentCopilot.app` and
  `dev.agent-copilot.native`.
- Preserved app-data compatibility with `dev.skills-copilot.native`.

## V2.89

- Refreshed Agent Copilot app icon assets.
- Preserved internal `SkillsCopilot` / `skills-copilot` compatibility
  identifiers.

## V2.88

- Added per-surface handoff evidence for Agent Copilot surfaces under
  `docs/ui-artifacts/v2.88-handoff-evidence/`.

## V2.87

- Introduced the Agent Copilot first pass with native macOS surfaces and service
  methods for local session and MCP evidence previews.
- Historical V2.87 implementation design notes were superseded by the V2.87
  and V2.88 verification checklists, `docs/service-protocol.md`, and this
  changelog.
