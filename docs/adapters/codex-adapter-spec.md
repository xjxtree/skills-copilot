# Codex Adapter Spec

This document records the current Codex adapter contract.

The adapter ID stays `codex` when Codex is hosted inside the current ChatGPT
desktop app. The host app name does not change repository instructions,
configuration paths, session stores, or service wire identities.

## Scan Roots

- User `$CODEX_HOME/skills` and `$HOME/.agents/skills` where applicable.
- Project `.agents/skills` discovered from the selected working directory up to
  the project root.
- Local plugin marketplace roots,
  `$CODEX_HOME/plugins/cache/<publisher>/<package>/<version>/skills`, and
  `/etc/codex/skills` when present, as read-only diagnostics.
- Project `.codex/config.toml` as read-only diagnostics.

`CODEX_HOME` is shared by skill/config and local-session discovery. An override
must be absolute and lexically normalize beneath the active user home; otherwise
the adapter falls back to `$HOME/.codex`.

Plugin-cache discovery is bounded and deterministic: hidden/staging entries are
ignored, manifest reads are capped at 1 MiB, `skills` must resolve beneath the
canonical package root, and the highest valid numeric-aware version for each
publisher/package is selected. Plugin list/detail records expose optional
publisher, package, version, `chatgpt-plugin-cache` source kind, and read-only
reason derived from the cataloged path.

## Skill Format

- A skill is a directory containing `SKILL.md`.
- Required frontmatter: `name`, `description`.
- Optional directories such as `scripts/`, `references/`, `assets/`, and
  `agents/` are metadata only; importing or scanning must not execute scripts.
- Missing required frontmatter creates a broken record rather than aborting the
  scan.

## Writable Scope

- Toggles may patch only the verified user config override for native
  `.agents/skills` instances.
- The adapter uses absolute `SKILL.md` paths for disabled entries.
- Re-enable removes matching disabled entries and preserves non-target config.

## Blocked Scope

- Do not write project `.codex/config.toml`.
- Do not write plugin, admin, system, or compatibility roots.
- Do not install, update, remove, or execute ChatGPT plugin-cache content.
- Do not fetch marketplace/network skill indexes.
- Do not add hooks, MCP config writes, script execution, credentials, cloud
sync, or telemetry through the adapter.

ChatGPT's Plugin Directory is not Agent Copilot's Skill Manager. The latter is
the separate `skillManager.*` path backed by an explicit manager CLI preview,
target visibility, telemetry-off environment, and confirmation.

## Fixtures

Codex fixtures live under `fixtures/codex/` and cover valid and malformed skill
frontmatter plus read-only root behavior.
