# Pi Fixtures

These files are executable parser and scanner inputs for
`docs/adapters/pi-adapter-spec.md`.

- `global/agent/skills/global-pdf/SKILL.md` covers the native global root.
- `project/.pi/skills/project-plan/SKILL.md` covers the native project root.
- `broken/missing-description/SKILL.md` covers required frontmatter failure.

Package filtering, compatibility roots, toggles, and write isolation are
exercised with temporary settings and homes inside the Rust tests.
