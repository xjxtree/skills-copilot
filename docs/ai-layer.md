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
  provider prompt-request boundary. Provider profile save/delete and connection
  tests use their own paired preview/apply methods.
- Every provider profile mutation, connection test, and prompt send uses the
  signed typed action lifecycle. Apply requires the exact
  `action_confirmation`; its one-time token is cleared by the native client
  after a confirmed attempt and cannot be replayed or retried automatically.
- Profile save previews bind non-secret normalized input. Credential
  replacement is represented only by an authorization-keyed opaque binding;
  API keys never appear in the preview, action descriptor, bounded replay
  state, log, or response.
- Verified applies return semantic read-back for all declared profile,
  credential, activity, and prompt-run domains. Unverified local effects are
  `applied_unverified`; any failure after provider traffic may have left the
  process is `remote_unknown`. Both are explicit partial outcomes that require
  state inspection and a fresh preview.
- The wire action `task_cockpit` remains compatible while the UI presents it as
  inline task readiness.
- `session_digest` binds one revalidated native session inventory revision and
  one accepted product snapshot revision. It may summarize bounded session
  metadata and suggest copy-only next-prompt text, but it never receives or
  returns resume argv.
- `skill_change_review` binds one selected skill aggregate from the accepted
  project projection. It describes only observed current evidence and must not
  invent a prior version when comparison evidence is unavailable.
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

`llm.previewPrompt` returns the complete response contract that will be sent
after confirmation. The contract contains schema version 1, request kind,
selected project ID, accepted product source revision, one result schema,
bounded typed evidence, optional deterministic actions, and exact copy-only
safety flags. Evidence may retain its own native provenance revision, such as a
session inventory revision, while the enclosing contract remains bound to the
accepted product source revision.

The supported result schemas are:

- `copy_only_markdown` for bounded explanations and advanced frontmatter help;
- `task_readiness` for the wire-compatible `task_cockpit` action;
- `session_digest` for one selected session;
- `skill_change_review` for one selected skill aggregate.

The provider must return only the matching JSON response envelope. Transport
success without a valid envelope is `parse_failed` /
`response_schema_invalid`; a request that may have left the process but whose
response contract or current product revision cannot be verified is
`remote_unknown`. Native clients validate the same contract again before
publishing a successful result.

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
- Result objects containing command, argv, script, tool-call, apply,
  confirmation, preview-token, mutation, or write-back fields are rejected at
  the provider boundary.

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
