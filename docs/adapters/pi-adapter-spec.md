# Pi Adapter Spec

This document records the current Pi adapter contract.

## Scan Roots

- Project and global installed package skill roots, then configured `skills`
  paths, in Pi's first-name-wins precedence.
- Cwd `.pi/skills` only; do not infer `.pi/skills` from every ancestor.
- Project `.agents/skills` from cwd through the git root.
- Native global `~/.pi/agent/skills` and shared `~/.agents/skills`.

Directory skills with `SKILL.md` are cataloged recursively. Direct root
Markdown skills are accepted in Pi-native roots and ignored in `.agents`
compatibility roots. Declared skill symlinks authorize only their exact
canonical targets.

## Skill Format

- A skill is a directory containing `SKILL.md`.
- Required frontmatter: `name`, `description`.
- Missing required frontmatter creates a broken record rather than aborting the
  scan.

## Writable Scope

- Guarded toggles may update supported disabled-skill collections in Pi
  settings.
- Project settings writes must be project-bound, snapshot-backed, read back,
  and rollback-capable.
- Tool-global installs may copy confirmed local `SKILL.md` records only into
  native Pi roots.

## Blocked Scope

- No package install/remove.
- No `.agents` direct skill-file installs.
- No scripts, credentials, cloud sync, telemetry, or AI write-back.
- Explicit untrusted project markers block project writes.

## Session Inventory

- Use `PI_CODING_AGENT_SESSION_DIR`, configured `sessionDir`, or the native
  `<agent-dir>/sessions` fallback, matching Pi's own lookup order.
- Extension subagent artifacts are excluded; normal parent transcript branches
  remain part of the user-owned session.
