# OpenClaw Adapter Spec

This document records the current OpenClaw adapter contract.

## Scan Roots

- Confirmed workspace `<workspace>/skills` and
  `<workspace>/.agents/skills`, then personal `~/.agents/skills`, managed
  `<state>/skills`, bundled roots, enabled plugin manifest roots, and configured
  local extra dirs in runtime precedence order.
- `OPENCLAW_STATE_DIR`, valid `OPENCLAW_PROFILE`, `OPENCLAW_CONFIG_PATH`, and
  configured agent workspaces are resolved consistently.
- Direct managed/personal skill links and configured
  `skills.load.allowSymlinkTargets` authorize exact targets only.

OpenClaw project scope is workspace-scoped. Do not infer arbitrary repository
roots or `.openclaw/skills` directories.

## Skill Format

- A skill is a directory containing `SKILL.md`.
- `name` is read from YAML frontmatter when present; directory name is the
  fallback.
- Missing description may remain loaded with an empty description when the
  adapter has enough local evidence to identify the skill.
- Agent skill lists, `skills.allowBundled`, disabled entries, and
  `metadata.openclaw.requires` eligibility are projected into effective enabled
  state without reading or returning credential values.
- Plugin extensions are never walked broadly. Only effectively enabled plugin
  IDs and safe relative `skills` directories from `openclaw.plugin.json` are
  scanned; workspace extensions fail closed unless explicitly enabled.

## Writable Scope

- Tool-global install may copy confirmed local `SKILL.md` records into
  `~/.openclaw/skills`.
- Workspace install may copy confirmed local `SKILL.md` records into confirmed
  `<workspace>/skills`.
- Guarded toggles may update only `skills.entries.<key>.enabled` in
  `~/.openclaw/openclaw.json`.
- JSON5 input is parsed and strict JSON is written back.

## Blocked Scope

- No `.agents` direct installs.
- No allowlist, env/apiKey, install policy, or load-root writes.
- No ClawHub, Git, update, verify, workshop, cloud, telemetry, script,
  credential, or network-backed operations.

## Session Inventory

- The canonical source is each agent's
  `<state>/agents/<id>/agent/openclaw-agent.sqlite` database.
- Only active routed user conversations are returned. Archived/deleted rows and
  cron, hook, heartbeat, ACP, and subagent keys are excluded.
- Old `<state>/agents/<id>/sessions` JSON/JSONL files are legacy migration or
  archive material and are never used as the active list source.

## Fixtures

OpenClaw fixtures live under `fixtures/openclaw/` and cover read-only roots,
install boundaries, malformed records, and guarded config-toggle behavior.
