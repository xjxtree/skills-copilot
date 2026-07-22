# opencode Fixtures

These files are executable parser and scanner inputs for
`docs/adapters/opencode-adapter-spec.md`.

- `user-home/.config/opencode/skills/global-review/SKILL.md` covers the native
  global root.
- `project/.opencode/skills/project-release/SKILL.md` and the nested package
  fixture cover project discovery from the selected working directory upward.
- `broken/name-mismatch/SKILL.md`, `broken/missing-description/SKILL.md`, and
  `broken/missing-name/SKILL.md` cover required format failures.

Compatibility-root, permission override, and write-isolation cases construct
temporary homes in the Rust tests rather than storing manual validation state.
