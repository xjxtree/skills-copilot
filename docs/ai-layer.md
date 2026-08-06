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
- Task Preflight task text and untrusted provider output are returned for
  copy-only use in the current app session and are excluded from persisted
  prompt-run rows.
- Task Preflight confirmation shows a request summary and token/cost estimates.
  After a send attempt, its complete redacted request and complete untrusted
  response remain available through the paged, date-filtered in-memory view
  until the app exits; neither value is written to disk.
- Agent Copilot prompt builders include the complete available effective-skill,
  finding, and conflict inputs for their scoped action. They do not silently
  drop list suffixes to fit an internal item-count cap; an actual provider/model
  context failure remains visible as a request failure.
- Selected-skill Intelligent Analysis may render copy-only Agent Copilot
  provider output as Markdown for readability without changing the safety
  model.

## Local Signals

- Agent/session/skill/config summaries should prefer local typed service data.
- Skill usage summaries count explicit local invocation markers, not ordinary
  skill-name mentions.
- Provider Observability may display read-only redacted metadata for Agent
  Copilot AI requests, including model-task match rows, but must not add
  write/delete controls without an explicitly scoped safety review.

## Non-Goals

- No autonomous edits from LLM recommendations.
- No network-backed skill install flows.
- No script execution from imported skills, previews, cleanup guidance, or
  generated analysis.
