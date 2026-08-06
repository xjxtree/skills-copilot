# Hermes Adapter Spec

This document records the current Hermes adapter contract.

## Scan Roots

- Native `~/.hermes/skills`.
- Explicit read-only `skills.external_dirs`.

Do not infer generic project roots. Cron jobs, logs, auth files, `.env`, and hub
metadata are not skill instances.

## Skill Format

- A skill is a directory containing `SKILL.md`.
- Frontmatter is parsed as local metadata.
- Malformed frontmatter creates a broken record rather than aborting the scan.

## Writable Scope

- Tool-global install may copy confirmed local `SKILL.md` records into
  `~/.hermes/skills`.
- Guarded toggles may update only global `skills.disabled` in
  `~/.hermes/config.yaml`.
- Writes require snapshot/read-back/rollback and secret redaction.

## Blocked Scope

- No project installs.
- No `platform_disabled` writes.
- No `external_dirs` writes.
- No hub, URL, tap, update, uninstall, reset, package, script, credential, cloud
  sync, telemetry, or uncontrolled network operations.

## Session Inventory

- The canonical local source is `<HERMES_HOME>/state.db`.
- Cron, batch, subagent, and memory-consolidation workflows are excluded from
  the user-facing list.
