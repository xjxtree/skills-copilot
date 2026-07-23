# AI Layer

This file defines the optional intelligence boundary. Product truth remains
deterministic and local; AI interprets already-collected evidence.

## Default Mode

- The app works without Agent Copilot provider calls.
- Optional LLM/provider features inside Agent Copilot are disabled unless
  explicitly enabled by the user.
- Provider calls made by Agent Copilot require prompt preview, redaction,
  destination visibility, and explicit confirmation.
- Provider usage shown in the app refers only to Agent Copilot's own optional AI
  features. It is not a billing, usage, or telemetry view for provider profiles
  configured inside managed agents such as Claude Code, Codex, opencode, or Pi.

The provider-off experience must still support project health, skill and
session inspection, local lexical search, typed action preview, and native
resume preview.

## Intelligence Allocation

AI may help with:

- explaining project health and prioritizing an existing attention queue;
- ranking relevant effective skills for an explicit task and explaining
  missing capability;
- explaining skill overlap or a bounded skill change;
- producing an evidence-cited session digest and suggested next prompt;
- explicitly reranking an already-returned local lexical result set.

AI must not determine scan completeness, source precedence, plugin enablement,
skill effectiveness, conflict membership, session identity, write authority,
or resume command syntax.

The product exposes contextual intelligence in Project Overview, Skills, and
Sessions. It does not add a general chat destination. Skill detail uses one
contextual explanation entry point; frontmatter drafting remains an advanced,
copy-only authoring aid.

## Request Contract

- Existing `llm.previewPrompt` and `llm.confirmPromptAndSend` remain the only
  provider request boundary.
- The wire action `task_cockpit` remains compatible while the UI presents it as
  inline task readiness.
- New product actions such as `session_digest` and `skill_change_review` are not
  callable until they appear in the service method/action inventory, fixtures,
  and `docs/service-protocol.md`.
- Semantic search is a separate explicit provider action over bounded local
  candidates. `app.search` remains local and lexical and must never trigger a
  provider call.

Every new product-intelligence response must bind:

- the input or source revision;
- typed evidence references resolvable in the selected project projection;
- optional typed action references created independently by deterministic
  product logic;
- a structured response schema and safety flags.

Unknown evidence/action references or a changed revision invalidate the
response. The app must display deterministic facts and AI interpretation as
different kinds of content.

## Output Handling

- LLM output is untrusted and copy-only by default.
- LLM output must not create hidden writes, hidden task state, script execution,
  credential access, cloud sync, telemetry, or raw prompt/response persistence.
- Selected-skill Intelligent Analysis may render copy-only Agent Copilot
  provider output as Markdown for readability without changing the safety
  model.
- LLM output cannot synthesize a preview token, resume command, target path,
  manager command, config patch, or action authorization. It may reference only
  a currently valid typed action descriptor.

## Local Signals

- Agent/session/skill/config summaries should prefer local typed service data
  and fixture-backed evidence.
- Skill usage summaries count explicit local invocation markers, not ordinary
  skill-name mentions.
- Provider Observability may display read-only redacted metadata for Agent
  Copilot AI requests, including model-task history rows, but must not add
  write/delete controls without an explicitly scoped safety review.

## Non-Goals

- No autonomous edits from LLM recommendations.
- No AI-triggered or hidden network-backed skill install flow; supported
  manager operations remain separate typed, previewed, confirmed actions.
- No script execution from imported skills, previews, cleanup guidance, or
  generated analysis.
- No persistent embeddings, raw transcript index, cross-agent session
  translation, or autonomous continuation without a new scoped review.
