# Agent Adapters

This document defines supported local-agent scan roots, write scopes, and
blocked operations. It is a current contract, not a version history.

## Global Rules

- Adapters are stateless. They discover roots, parse skill records, and report
  enabled state; they do not cache file contents.
- Local-session inventories are also request-scoped and stateless. Keyset first
  pages require the explicit wire opt-in `paging_mode="keyset"`; unmarked
  requests retain legacy offset/max-file behavior. Continuation re-inventories
  only authorized roots within the established directory, entry, file, sidecar,
  and byte budgets; any exhaustion is a terminal typed `safety_budget`
  limitation that retains accepted unique rows. Opaque cursors contain digests
  rather than raw paths. Adapters must not persist session cursors, inventories,
  summaries, details, or raw session content in SQLite or app data.
- Scan roots and symlink-target roots are explicit. A symlink target is readable
  only when its canonical path is under a declared root with the same scope;
  the rest of the user home or project tree does not authorize traversal.
- Scanner link resolution must retain that same-scope root provenance; a link
  cannot promote a project root into a global root or vice versa.
- Unavailable roots are skipped with typed issues, while directory and entry
  failures mark a root partial and allow other roots to continue. Only fully
  enumerated roots are eligible for missing-record cleanup.
- Partial roots never participate in missing-sweep, including when another
  complete root resolves to the same physical location under a different scope.
- Writes must go through the service layer with preview, snapshot/read-back
  where applicable, atomic write, rollback, and rescan.
- Network fetch, package manager calls, script execution, credentials, cloud
  sync, and telemetry are blocked unless explicitly scoped. The `skillManager.*`
  service domain is the scoped exception for supported external manager CLIs.
- Configured or compatibility roots are scan-only unless the adapter table
  below explicitly names a guarded write path.
- Skill Manager defaults to the app-supported agent set only: Claude Code, Pi,
  opencode, Codex, Hermes, and OpenClaw. It must not use wildcard manager
  targets that could reach unsupported agents.

## Adapter Matrix

| Adapter | Scan roots | Guarded writes | Install targets | Blocked |
| --- | --- | --- | --- | --- |
| Claude Code | User/project `.claude/skills`; user `.agents/skills` is an explicit same-scope symlink-target root and is not walked directly | Private Claude settings toggle path | Verified native target paths | Shared project settings writes unless separately scoped |
| Codex | User/project `.agents/skills`; read-only `$CODEX_HOME/skills`, local plugin marketplace roots, `/etc/codex/skills`, and project `.codex/config.toml` diagnostics | User config override for native `.agents/skills` instances | Native `.agents/skills` roots | Project `.codex/config.toml`, plugin/admin/system/compat writes |
| opencode | Native roots, official `.claude` / `.agents` compatibility roots, and configured local `skills.paths` roots | Exact `permission.skill` overrides in verified config targets | Native opencode roots | `skills.urls` fetch, configured-root writes, compatibility-root installs |
| Pi | Native `~/.pi/agent/skills`, project `.pi/skills`, and `.agents/skills` compatibility roots | Guarded settings toggle for native and `.agents` compatibility instances | Native Pi roots only | Package install/remove, `.agents` direct installs, scripts, credentials |
| Hermes | Native `~/.hermes/skills` and explicit read-only `skills.external_dirs` | Global `skills.disabled` only | Native `~/.hermes/skills` | Project installs, `platform_disabled`, `external_dirs` writes, hub/URL/tap/update/uninstall/reset |
| OpenClaw | Native `~/.openclaw/skills`, shared `~/.agents/skills`, bundled roots, confirmed workspace `<workspace>/skills`, and `<workspace>/.agents/skills` | `skills.entries.<key>.enabled` only | Native `~/.openclaw/skills` and confirmed workspace `<workspace>/skills` | `.agents` direct installs, allowlists, env/apiKey, install policy, load roots, ClawHub/Git/update/verify/workshop |

## Skill Manager Tooling

- `npx skills` is the first writable manager tool. It owns search, list,
  install, remove, update, and local template creation when the app calls
  `skillManager.*`.
- `skills-npm` is registered for capability discovery only in this slice; write
  execution needs a future scoped adapter.
- Manager-backed search/install/update may use external network access only
  when the request marks network access allowed and the app has shown command
  preview and confirmation state.
- Commands must be executed as argv arrays with telemetry-off env
  (`DISABLE_TELEMETRY=1`, `DO_NOT_TRACK=1`) and redacted output logging. Shell
  string concatenation is forbidden.
- Search preserves every row returned by the manager, but the current CLI does
  not prove a remote total or advertise a continuation token. Responses must
  therefore distinguish returned rows from an unknown, source-limited total;
  clients may reveal the in-memory returned collection progressively but must
  not label its count as complete or make a hidden follow-up network request.
  Installed JSON and the app-owned local library are enumerable exact lists.
  Do not add page flags or alter the manager command shape without a parsed
  token fixture that proves the external manager contract.
- Install uses the manager default symlink flow. `--copy` is sent only for an
  explicit copy selection.
- Skill removal uses manager-backed agent link removal for the targets selected
  in the Skill Manager panel; the panel does not expose agent-layer
  enable/disable controls.
- Agent enable/disable remains in `config.toggleSkill`,
  `batch.previewSkillToggles`, and `batch.applySkillToggles` outside the Skill
  Manager surface; package manager state and agent config state are separate.

## Discovery Requirements

New or expanded adapter support needs verified evidence for:

- skill discovery roots;
- skill file/directory format;
- project inheritance behavior;
- config file path and schema;
- enable/disable semantics;
- fixture data;
- malformed input behavior;
- read-only fallback behavior when write semantics are absent.

Do not infer support from neighboring tools, guessed paths, or generic project
root conventions.

## Identity And Dedupe

- Same physical file exposed by multiple agents may appear as multiple
  `SkillInstance` rows.
- Same-agent runtime/name collisions are conflicts.
- Cross-agent duplicate, overlap, or enabled-state mismatch belongs in analysis,
  not conflict counts.
- Path/provenance labels should explain why two rows exist without changing
  conflict semantics.

## Safety Notes

- `skills.urls`, hubs, taps, package managers, Git-backed installs, cloud
  scanners, security scans, and update commands are metadata-only or blocked
  outside the scoped `skillManager.*` manager path.
- Import/install must copy only confirmed local `SKILL.md` records into verified
  app-controlled or native roots.
- Adapter config snapshots must redact secrets before persistence or display.
- Any future write expansion must include disposable evidence, fixture tests,
  rollback tests, and privacy verification.
