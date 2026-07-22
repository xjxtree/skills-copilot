# opencode Adapter Spec

This document records the current opencode adapter contract.

## Scan Roots

- Native global and project `.opencode/skills` roots.
- Official `.claude/skills` and `.agents/skills` compatibility roots.
- Configured local `skills.paths` roots from readable JSON/JSONC config.

The `.claude` root is part of opencode's documented compatibility contract. A
skill stored there is an effective opencode skill and must be labeled as an
opencode compatibility source; it is not cross-agent filter leakage. Direct
skill-directory links authorize only their exact canonical targets.

`skills.paths` roots are scan-only. Paths are expanded relative to the declaring
config scope, canonicalized, deduped, and bounded to the expected project/user
context.

## Skill Format

- A skill is a directory containing `SKILL.md`.
- Required frontmatter: `name`, `description`.
- The `name` should match the containing directory. Mismatch creates a broken
  record rather than aborting scanning.
- Lowercase colon namespaces may appear at runtime when the containing
  directory uses the colon-normalized form.

## Writable Scope

- Toggles may patch exact `permission.skill` overrides in verified config
  targets.
- Disable writes exact `deny`; re-enable removes only the matching exact deny.
- Wildcard and unrelated permission rules must be preserved.
- Effective access uses the last matching `permission.skill` rule. Claude
  `skillOverrides` never apply to opencode compatibility rows.
- Tool-global installs are limited to native opencode roots.

## Blocked Scope

- `skills.urls` is metadata-only and must not fetch remote indexes.
- Configured local roots and compatibility roots are not install targets.
- Managed config, environment-provided config content, and network-backed
  installs need separate evidence before write support.

## Session Inventory

- The canonical local store is the XDG data `opencode/opencode.db` SQLite
  database.
- Archived rows and child sessions with a parent are excluded from the
  user-facing session list. Legacy JSON sidecars are not an active source when
  the database exists.

## Fixtures

opencode fixtures live under `fixtures/opencode/` and cover valid, malformed,
configured-root, and permission behavior.
