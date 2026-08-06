# Codex Adapter Spec

This document records the current Codex adapter contract.

The adapter ID stays `codex` when Codex is hosted inside the current ChatGPT
desktop app. The host app name does not change repository instructions,
configuration paths, session stores, or service wire identities.

## Scan Roots

- User `$CODEX_HOME/skills` and `$HOME/.agents/skills` where applicable.
- Project `.agents/skills` discovered from the selected working directory up to
  the project root.
- `/etc/codex/skills` when present, as read-only diagnostics.
- Project `.codex/config.toml` as read-only diagnostics.
- One deterministic active copy per installed plugin record whose
  `[plugins.<id>] enabled` value is explicitly `true` in
  `$CODEX_HOME/config.toml`. A missing entry and `enabled = false` both exclude
  the package. The plugin cache directory is never a scan root. The adapter
  reads the active copy's bounded regular
  `.codex-plugin/plugin.json` and scans only the safe relative root named by the
  manifest `skills` field; that root does not have to be named `skills`.
  Every valid skill below that declared root is part of the enabled plugin
  inventory unless its exact `SKILL.md` path is disabled by
  `[[skills.config]]`.

`CODEX_HOME` is shared by skill/config and local-session discovery. An override
must be absolute and lexically normalize beneath the active user home; otherwise
the adapter falls back to `$HOME/.codex`.

Codex product skill discovery is filesystem-only. The adapter does not start
`codex app-server` and does not call `skills/list`.
Plugin files are persisted installation state and appear in list, detail,
analysis, and conflict projections with a plugin namespace, logical display
path, and read-only package provenance; their physical cache path is hidden.

Local plugin marketplace directories are runtime implementation details rather
than installed skill roots. They are not walked. Ineffective cache packages,
staging directories, scripts, assets, and unrelated cache content are also
excluded and never executed.

## Skill Format

- A skill is a directory containing `SKILL.md`.
- Required frontmatter: `name`, `description`.
- Optional directories such as `scripts/`, `references/`, `assets/`, and
  `agents/` are metadata only; importing or scanning must not execute scripts.
- Missing required frontmatter creates a broken record rather than aborting the
  scan.
- Plugin skills use the runtime `<plugin>:<skill>` namespace. A colon-separated
  runtime name is not evaluated by the local lowercase-slug naming rule.

## Writable Scope

- Toggles may patch only the verified user config override for native
  `.agents/skills` instances.
- The adapter uses absolute `SKILL.md` paths for disabled entries.
- Re-enable removes matching disabled entries and preserves non-target config.

## Blocked Scope

- Do not write project `.codex/config.toml`.
- Do not write plugin, admin, system, or compatibility roots.
- Do not install, update, remove, or execute installed Codex plugin content.
- Do not fetch marketplace/network skill indexes.
- Do not add hooks, MCP config writes, script execution, credentials, cloud
sync, or telemetry through the adapter.

## Session Inventory

- Summary inventory reads the newest guarded `$CODEX_HOME/state_*.sqlite`
  thread index, matching Codex `thread/list` semantics for active interactive
  top-level tasks and exact cwd project scope.
- A selected session's messages are read on demand from its guarded rollout;
  summary rows never copy raw transcript content into Agent Copilot storage.
- Archived, exec, subagent, review, compact, memory, and other internal source
  kinds are excluded.

ChatGPT's Plugin Directory is not Agent Copilot's Skill Manager. The latter is
the separate `skillManager.*` path backed by an explicit manager CLI preview,
target visibility, telemetry-off environment, and confirmation.
