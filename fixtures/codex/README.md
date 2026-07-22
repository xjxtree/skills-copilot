# Codex Fixtures

These files are executable parser and scanner inputs for
`docs/adapters/codex-adapter-spec.md`.

- `user-home/.agents/skills/user-alpha/SKILL.md` covers a global shared skill.
- `project/.agents/skills/repo-beta/SKILL.md` and
  `project/nested/.agents/skills/nested-gamma/SKILL.md` cover project discovery
  from the selected working directory up to the project root.
- `conflict/` contains same-name global and project skills for precedence and
  conflict tests.
- `broken/missing-description/SKILL.md` covers required frontmatter failure.

Configuration and plugin behavior are exercised with isolated temporary homes
inside the Rust tests so fixture paths never need machine-specific rewriting.
