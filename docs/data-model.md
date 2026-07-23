# Data Model

This file summarizes persisted and transient data.

## Core Domains

| Domain | Owner | Notes |
| --- | --- | --- |
| Agent and scope ids | `crates/core` | Stable wire strings; no I/O base layer |
| Skill catalog rows | Rust service | Derived from local roots and fixtures |
| Session preview rows | Rust service | Redacted bounded summaries/process samples plus transient paged user/final-Agent messages |
| Skill usage rows | Rust service | Derived from explicit invocation markers |
| Config snapshots | Rust service | Guarded reads/writes for supported adapters |
| Model-task matches | App-local JSON | Redacted metadata for Agent Copilot AI features only |
| Skill Manager inventory | Native startup/manual-refresh cache | One row per project/global installed source, enriched by matching guarded `.agents/skills` catalog instances (including nested package layouts) and merged with non-installed app-owned local sources; not persisted by the UI |

## Product Read Projections

The product rebuild uses additive Rust-owned read models. Their type names and
semantics are defined here; a projection is callable only after its method also
appears in `docs/service-protocol.md`, protocol fixtures, and the service
supported-method inventory.

| Model | Required content | Persistence |
| --- | --- | --- |
| `ProjectReadinessRecord` | Project/source revision, coverage, six-agent environment health, typed blocking reasons, attention queue, evidence-bound actions, and recent continuation summaries | Transient/cache only |
| `SkillAggregateRecord` | Definition fingerprint, logical source/package/scope/runtime identity, per-instance effectiveness, findings, conflicts, completeness, evidence, and supported actions | Transient/cache only |
| `SessionContinuationRecord` | Project/agent identity, stable session identity, timing, completeness, native source revision, accepted snapshot revision, evidence, and native resume capability | Transient selected-project state only |
| `EvidenceRef` | Stable typed id, evidence kind, source revision, and redacted display summary | Stored only when the owning existing metadata record is allowed to persist |
| `ActionDescriptor` | Deterministic id, target, impact, preview/apply method, revision binding, network posture, and confirmation requirement | Transient; never write authorization by itself |

`EnvironmentHealthState` is `healthy`, `review`, or `blocked`.
`SkillEffectivenessState` is `effective`, `disabled`, `shadowed`,
`installed_unlinked`, `broken`, or `unavailable`. Installed, enabled, and
effective remain independent facts. `effective` means verified by the adapter
projection; it does not prove runtime invocation.

Core product records enforce these invariants before service exposure:

- Evidence ids, revisions, and summaries are non-empty; summaries reject raw
  Unix/Windows absolute paths and control characters while allowing logical
  placeholders such as `$HOME` and `<project-root>`.
- Action descriptors name typed targets and service preview/apply methods,
  contain unique impacts and evidence references, and bind one source
  revision. A mutating impact requires both an apply method and explicit
  confirmation; a read-only action cannot expose an apply method.
- Healthy project or agent readiness requires complete source coverage.
- `partial`, stale, unavailable, source-limited, and uninspected required
  evidence maps to typed incomplete coverage and blocks healthy readiness.
- Skill aggregation keys include definition id/fingerprint, logical source
  identity and kind, publisher/package/version, scope, and runtime identity.
  Same-name records with any material identity difference remain separate.
  Logical source and runtime identities are typed opaque values and cannot
  contain filesystem path separators.
- Each current aggregate carries exactly one effectiveness record per current
  instance. Its effectiveness counts cover every projected instance, while
  installed/enabled/effective counts remain independent. Historical `missing`
  catalog rows are not current instances.
- `effective` requires complete installed, linked, enabled, and
  precedence-proven evidence. A proven loser is `shadowed`; unresolved
  precedence or incomplete evidence is `unavailable`. `installed_unlinked`
  requires complete manager inventory evidence.
- Aggregate effectiveness state counts, primary state, agent/scope membership,
  and coverage are recomputed from per-instance rows during validation.
  Project coverage is the same canonical merge over agent rows.
- Supported resume capability contains non-empty native argv and remains
  copy-only. Unsupported capability contains no argv and carries a typed
  reason; incomplete continuation coverage cannot carry supported resume. A
  continuation's evidence binds its native source revision, while actions bind
  its accepted product snapshot revision.
- Project and skill aggregate records reject evidence or actions bound to their
  source revision. Session continuation records enforce the native/snapshot
  split above. Every record rejects actions that refer to unknown evidence.
- Projection action ids are unique. Skill, agent, and session inputs may attach
  an action only when its target and every evidence reference belong to that
  exact owner. Action ids and evidence references serialize in lexical order,
  impacts serialize in fixed enum order, and semantic sequences such as native
  resume argv preserve adapter order.
- Product projection input is accepted only when all component snapshot
  revisions match. The projection performs no provider or mutable-source reads,
  and its serialized ordering is deterministic.

## Persistence

- `model-task-matches.json` stores redacted app-local metadata for Agent
  Copilot's optional AI workflows. It is not a store for provider usage
  generated by managed agents.
- Agent config history and snapshots must not store credentials or raw Agent
  Copilot provider output.
- Project-scoped config snapshots persist their canonical `project_root` and
  are filtered and rollback-validated against the active project context.
  Legacy project snapshots without this binding are retained but not exposed.
- Session preview rows model user-facing, top-level conversations rather than
  every transcript-like file. Adapter-specific subagent, scheduled, host-owned,
  runtime-state, and other synthetic/internal records are filtered from both
  summaries and details using source metadata and documented directory
  boundaries. Session preview data, transient selected-session message pages,
  and skill usage summaries are read-only diagnostics and must not persist raw
  transcript content. The message-page cursor fixes one source snapshot and is
  retained only in native in-memory loading state.
- Scan root and issue diagnostics are transient response data. Partial roots
  preserve unseen catalog rows, and diagnostic paths/details are redacted
  before crossing the service boundary.
- An exact complete adapter scan marks unseen rows for that agent and current
  project context as `missing`, including rows whose old root disappeared.
  These historical rows appear only through the explicit Deleted filter; the
  default All projection and current totals exclude them. Runtime conflict
  groups are derived only from current loaded/enabled rows.
- Codex catalog rows retain the last scanned state. Read-only service
  projections overlay the current guarded `skills.config` disabled paths in
  memory and can restore stale loaded/disabled rows when an external override
  is removed; this does not mutate SQLite.
- Installed Codex plugin rows are derived from persisted manifest-declared
  `SKILL.md` files and appear in current projections with read-only package
  provenance. The compatibility wire value remains
  `source_kind="chatgpt-plugin-cache"`; legacy synthetic runtime rows and
  missing Pi reference-document noise are removed by schema migration.
- Fixture data is test input; it must keep its wire shape unless protocol drift
  work is explicitly scoped.
- Installed manager rows expose compact skill identity, source kind, scope, and
  supported-agent links. Raw manager JSON and repeated filesystem paths remain
  internal to parsing and are not duplicated across the service boundary.
- Local ZIP update previews are transient. Archive paths, hashes, counts, and
  preview tokens are not catalog payloads; only the validated replacement skill
  is registered through the existing app-owned ToolGlobal catalog path.
- Product readiness, skill aggregate, action-queue, session continuation, AI
  evidence-envelope, and semantic rerank results are transient. They must not
  create a second catalog, task ledger, transcript store, raw prompt store, or
  embedding database without a separately scoped persistence and privacy
  review.

## Redaction

- Paths, hosts, ports, transcript snippets, Agent Copilot provider metadata,
  and config values should be collapsed or redacted before display when they
  could expose private local state.
- Reveal flows must be explicit and local to the UI surface.
- Reports, screenshots, and response artifacts must not contain credentials.
- Evidence references carry redacted summaries and stable ids, not raw local
  paths, config values, transcript bodies, prompts, or provider responses.

## Compatibility

- The displayed product name and package identity are Agent Copilot.
- Existing catalog files are schema-migrated once before a read-only service
  view opens; catalogs already at the current schema stay on the no-write read
  path.
- Compatibility names such as `SkillsCopilot`, `skills-copilot`, legacy
  Keychain service ids, AX ids, and environment variables remain where required
  for migration and existing fixtures.
