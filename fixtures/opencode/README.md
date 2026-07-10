# opencode Parser/Scan Contract Fixtures

These fixtures define the opencode parser, scan, and guarded-write contract
from `docs/adapters/opencode-adapter-spec.md`.

- `user-home/.config/opencode/skills/global-review/SKILL.md` mirrors `~/.config/opencode/skills/global-review/SKILL.md`.
- `project/.opencode/skills/project-release/SKILL.md` mirrors `.opencode/skills/project-release/SKILL.md`.
- `project/packages/app/.opencode/skills/nested-local/SKILL.md` confirms project scanning walks from `project_cwd` upward to `project_root`.
- `broken/name-mismatch/SKILL.md` is intentionally invalid because opencode requires the frontmatter `name` to match the containing directory or its colon-normalized form.
- `broken/missing-description/SKILL.md` is intentionally invalid because `description` is required.
- `broken/missing-name/SKILL.md` is intentionally invalid because `name` is required.

The opencode adapter scans first-class native roots plus official compatibility roots: global `~/.config/opencode/skills`, project `.opencode/skills` from `project_cwd` upward to `project_root`, global/project `.claude/skills`, and global/project `.agents/skills`. Compatibility roots are scan-only sources; tool-global installs still target native opencode roots. Runtime names may include colon namespaces, such as `ce:compound`, when the directory uses the colon-normalized form `ce-compound`.

`config/opencode-deny-skill.json` records the managed `permission.skill`
configuration shape used by the guarded opencode toggle flow.

Writable validation rules:

- Keep this directory as the parser/scan contract.
- Validate writes only with a disposable local `HOME` and fixture project so
  real opencode configuration remains isolated.
- Managed writes use exact `permission.skill.<name> = "deny"` disable semantics and re-enable by removing only that exact deny.
- Commented JSONC configs are not rewritten because comments cannot be preserved by the current managed writer.
