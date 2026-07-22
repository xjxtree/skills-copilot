# OpenClaw Fixtures

These files are executable parser and scanner inputs for
`docs/adapters/openclaw-adapter-spec.md`.

- `skill-evidence/sample-openclaw-skill/SKILL.md` covers a valid skill.
- `user-home/.openclaw/skills/managed-global/SKILL.md` covers the managed
  global root.
- `user-home/.agents/skills/personal-shared/SKILL.md` covers the shared global
  root.
- `user-home/.openclaw/workspace/skills/workspace-override/SKILL.md` and the
  workspace `.agents/skills` fixture cover confirmed workspace scope.
- `broken/missing-name/SKILL.md` covers directory-name fallback.
- `broken/missing-description/SKILL.md` covers description-tolerant discovery.

The scanner must not infer arbitrary repositories as OpenClaw workspaces or
invoke OpenClaw/network operations while reading these fixtures.
