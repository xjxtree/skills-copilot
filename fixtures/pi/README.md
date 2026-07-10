# Pi Evidence Fixtures

These fixtures provide parser, scan, and guarded configuration contract
evidence for `docs/adapters/pi-adapter-spec.md`. The original disposable-tool
validation used Pi 0.78.1, which remains the minimum evidenced tool version.

- `global/agent/skills/global-pdf/SKILL.md` mirrors `~/.pi/agent/skills/global-pdf/SKILL.md`.
- `project/.pi/skills/project-plan/SKILL.md` mirrors `.pi/skills/project-plan/SKILL.md`.
- `config/settings-package-filter-disabled.json` shows official package resource filtering syntax for disabling package-provided skills.
- `broken/missing-description/SKILL.md` is intentionally invalid because Pi docs say skills with missing descriptions are not loaded.

Revalidate parser or guarded-write changes against a disposable `agentDir` and
fixture project with Pi 0.78.1 or newer. Never use real Pi settings for fixture
validation.
