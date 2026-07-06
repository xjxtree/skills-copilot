# AI Layer

This file summarizes the LLM/provider boundary.

## Default Mode

- The app works without Agent Copilot provider calls.
- Optional LLM/provider features inside Agent Copilot are disabled unless
  explicitly enabled by the user.
- Provider calls made by Agent Copilot require prompt preview, redaction,
  destination visibility, and explicit confirmation.
- Provider usage shown in the app refers only to Agent Copilot's own optional AI
  features. It is not a billing, usage, or telemetry view for provider profiles
  configured inside managed agents such as Claude Code, Codex, opencode, or Pi.

## Output Handling

- LLM output is untrusted and copy-only by default.
- LLM output must not create hidden writes, hidden task state, script execution,
  credential access, cloud sync, telemetry, or raw prompt/response persistence.
- Selected-skill Intelligent Analysis may render copy-only Agent Copilot
  provider output as Markdown for readability without changing the safety
  model.

## Local Signals

- Agent/session/skill/config summaries should prefer local typed service data
  and fixture-backed evidence.
- Skill usage summaries count explicit local invocation markers, not ordinary
  skill-name mentions.
- Provider Observability may display read-only redacted metadata for Agent
  Copilot AI requests, including model-task history rows, but must not add
  write/delete controls without a new scoped version and safety review.

## Non-Goals

- No autonomous edits from LLM recommendations.
- No network-backed skill install flows.
- No script execution from imported skills, previews, cleanup guidance, or
  generated analysis.
