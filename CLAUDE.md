@AGENTS.md

# CLAUDE.md

This file provides Claude Code-specific guidance for this repository. Shared project rules live in `AGENTS.md`; detailed multi-agent workflow rules live in `docs/ai-agent-workflow.md`.

## Claude Code Specific Rules

- Treat `AGENTS.md` as the canonical shared instruction entrypoint.
- Treat Agent Copilot as the displayed product name, app bundle, and repository identity. Swift/Rust module names, sidecar names, AX identifiers, env vars, and legacy data ids may still use `SkillsCopilot` / `skills-copilot` for compatibility.
- Keep task state, plans, and validation results out of repository docs.
- Read the relevant `docs/` file before architecture, UI, validation, or adapter changes.
- Use macOS Computer Use for real app validation when the macOS session is unlocked and the app window can be resolved.
- If Computer Use cannot resolve the app window, report the real-local blocker in the task or pull request.
- Do not treat Smoke App Run screenshots as real local effect validation.
- Do not capture full desktop screenshots. Validation images must be full app-window-only captures and remain outside the repository.
- The deprecated Tauri/React UI has been removed. Do not recreate `ui/`, `src-tauri/`, or Tauri IPC for product work.

## Claude Code Validation Defaults

For substantial, user-visible, UI, or service protocol work:

```sh
pnpm check:macos
pnpm dev:macos
```

Then operate the real local `dist/AgentCopilot.app` with Computer Use when available.

For docs-only contract cleanup, use the documentation and governance checks; app validation is unnecessary unless product behavior also changes.
