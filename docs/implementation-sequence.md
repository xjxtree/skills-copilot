# Implementation Sequence

This document is the normative dependency order for engineering the product
defined in [Product Design Contract](product-design.md). It is intentionally
not a schedule, milestone list, progress ledger, or validation log.

Coding agents must execute the tasks in numeric order. A task begins only after
the previous task's exit criteria are satisfied. Work inside the active task
may be split only when ownership and write sets are disjoint, the repository's
isolated-worktree rules are followed, and every subtask still converges on the
same task-level verification. Delivery state, command output, decisions, and
handoffs belong in the active task conversation, GitHub issue, or pull request,
not this file.

## Execution Rules

For every task:

1. Read `AGENTS.md`, `docs/product-design.md`, this document, and the focused
   contracts named by the task.
2. Inspect the implementation and fixtures before editing; documentation alone
   does not prove runtime behavior.
3. Preserve compatible wire contracts until all native consumers and fixtures
   have migrated.
4. Keep deterministic truth and write authorization in Rust. Swift presents
   typed projections and interaction state.
5. Keep optional AI behind preview, redaction, destination visibility, and
   explicit confirmation. Never make AI a prerequisite for a deterministic
   flow.
6. Update the focused current-contract docs in the same change as behavior.
7. Meet the task exit criteria and required verification before starting the
   next numbered task.

## Dependency Map

| Task | Depends on | Required outcome |
| --- | --- | --- |
| 1. Core Product Vocabulary | Product design contract | Shared no-I/O states, evidence, actions, and read-model types |
| 2. Deterministic Product Projections | Task 1 | Authoritative project, skill, attention, and continuation derivation |
| 3. Deterministic Action Lifecycle | Task 2 | Revision-bound preview, confirmation, apply, and read-back contract |
| 4. Product Read Service Protocol | Task 3 | Typed additive readiness, aggregate, and resume methods with fixtures |
| 5. Native State And Routing Foundation | Task 4 | Project-first routes, workspace stores, and typed client calls |
| 6. Project-First Shell And Overview | Task 5 | Primary shell and deterministic project overview |
| 7. Skills Workspace And Integrated Package Management | Task 6 | Aggregate capability workflow and one package-management entry point |
| 8. Sessions Workspace And Native Continuation | Task 7 | Project-grouped session evidence and safe copy-only resume preview |
| 9. Advanced And Settings Consolidation | Task 8 | Supporting config, diagnostics, raw metadata, and provider surfaces |
| 10. Evidence-Bound AI Contract | Task 9 | Structured revision/evidence/action-bound provider responses |
| 11. Contextual Intelligence And Search | Task 10 | Evidence-cited intelligence inside the three primary workspaces |
| 12. Legacy Surface Removal And Compatibility Cleanup | Task 11 | One entry point per product job with required compatibility retained |
| 13. Automated Contract Verification | Task 12 | CI/local enforcement of product, protocol, UI, list, and privacy rules |
| 14. Real-Local Cross-Agent Acceptance | Task 13 | `funnyaccount_system` parity evidence for all supported agents |

## Task 1 — Core Product Vocabulary

### Objective

Establish shared no-I/O types so every higher layer uses the same meaning for
health, effectiveness, evidence, actions, and source completeness.

### Work

- Add `EnvironmentHealthState` with `healthy`, `review`, and `blocked`.
- Add `SkillEffectivenessState` with `effective`, `disabled`, `shadowed`,
  `installed_unlinked`, `broken`, and `unavailable`.
- Add a typed `EvidenceRef` that can identify a fact without exposing a private
  path or embedding raw content.
- Add a typed `ActionDescriptor` containing action identity, target, impact,
  preview/apply service methods, source revision, and confirmation requirement.
- Define `ProjectReadinessRecord`, `SkillAggregateRecord`, and
  `SessionContinuationRecord` in the core model or adjacent no-I/O projection
  module.
- Reuse existing `SkillInstance`, `SkillDefinition`, `ConflictGroup`, paging,
  completeness, and revision types instead of creating parallel concepts.
- Define stable serialization and forward-compatible optional fields before
  exposing any record over the service boundary.

### Primary Areas

- `crates/core/src/model.rs`
- `crates/core/src/lib.rs`
- focused core tests
- `docs/data-model.md`

### Exit Criteria

- The new types compile without adding I/O to `crates/core`.
- Unit tests cover every enum value, evidence/action validation, redacted
  display behavior, and serialization round trips.
- No Swift type independently invents different state semantics.

### Verification

```sh
cargo test -p skills-copilot-core
cargo clippy -p skills-copilot-core --all-targets --all-features
```

## Task 2 — Deterministic Product Projections

### Objective

Build authoritative Rust projections from existing catalog, adapter, session,
finding, conflict, config, manager, and completeness evidence.

### Work

- Derive per-agent environment health and project coverage without provider
  calls.
- Make partial, stale, unavailable, and source-limited required evidence block
  a healthy result.
- Derive skill effectiveness separately from package presence and config
  enablement.
- Aggregate skills by definition, source, package, scope, and runtime identity;
  keep same-name but materially distinct definitions separate.
- Derive the project attention queue from active findings, conflicts, broken
  records, incomplete sources, and existing typed remediation capability.
- Project session continuation records from the documented adapter session
  sources and project matching rules.
- Keep raw catalog audit records available while ensuring product counts use
  the same user-visible projection everywhere.
- Add fixture coverage for Claude Code, Codex, opencode, Pi, Hermes, and
  OpenClaw, including compatibility and plugin provenance.

### Primary Areas

- `crates/commands/src/analysis.rs`
- `crates/commands/src/lib.rs`
- `crates/ai-core/src/lib.rs` for deterministic local rules only
- adapter, scanner, catalog, and command fixtures
- `docs/architecture.md`
- `docs/adapters/agent-adapters.md`

### Exit Criteria

- The same fixture evidence produces the same health, aggregate, and
  continuation records independent of UI or provider configuration.
- Installed, enabled, effective, shadowed, broken, and unavailable cases are
  independently tested.
- Codex plugin and opencode compatibility skills retain correct logical
  provenance and never arise from generic cache scans.
- Incomplete evidence cannot produce a positive aggregate or project-health
  assertion.

### Verification

```sh
cargo test -p skills-copilot-commands
cargo test -p skills-copilot-adapters
cargo test -p skills-copilot-scanner
```

## Task 3 — Deterministic Action Lifecycle

### Objective

Make every suggested repair or continuation action conform to Detect, Explain,
Evidence, Preview, Confirm, Apply, and Read-back.

### Work

- Map each attention-queue item to zero or more existing typed action
  descriptors. Absence of a safe supported action is explicit.
- Bind action descriptors and previews to source revision, target, agent,
  project, scope, impact, and network posture where applicable.
- Reject stale, unknown, mismatched, or AI-invented action references before
  mutation.
- Route direct config save and snapshot rollback through typed preview,
  immutable confirmation, apply, and semantic read-back. Keep config drafts
  local until explicit confirmation; do not autosave or retry stale actions.
- Serialize config and catalog mutation with the shared cross-process app-data
  owner lock. Revalidate under that owner and keep catalog changes in an SQLite
  `IMMEDIATE` transaction through file read-back and commit.
- On config failure, roll back catalog state and compensate the file only when
  it is still the exact written candidate. Restore a prior missing state and
  its newly created empty ancestor chain; preserve any third state and report a
  typed partial effect.
- Standardize read-back so each apply refreshes only the affected domain and
  recomputes projections.
- Keep skill package management, config toggles, rollback, local ZIP, and
  continuation preview within their existing safety ownership.
- Do not broaden adapter write scopes or enable skill scripts.

### Primary Areas

- `crates/commands`
- `crates/service/src/protocol.rs`
- service method-effect tests
- `docs/security-model.md`
- `docs/service-protocol.md`

### Exit Criteria

- Every UI-visible write has a deterministic preview and explicit confirmation
  contract.
- Stale revision and target mismatch tests prove no write or hidden retry
  occurs.
- Read-back tests prove the returned projection reflects the applied mutation.
- Cross-process contention tests prove config mutation waits for the common
  owner. Fault-injection tests prove transaction rollback, exact filesystem
  restoration, and fail-closed third-state handling.
- AI output cannot directly authorize or synthesize a write.

### Verification

```sh
cargo test -p skills-copilot-commands
cargo test -p skills-copilot-service method_effects
cargo test -p skills-copilot-service --test action_lifecycle_process
pnpm test:macos-native-models
pnpm verify:service-protocol-drift
```

## Task 4 — Product Read Service Protocol

### Objective

Expose the Rust-owned product projections through small typed read methods
without removing compatible lower-level methods.

### Work

- Add `project.getReadiness` as a cache-backed, read-only project projection.
- Add `catalog.listSkillAggregates` as a cache-backed, read-only skill
  projection with explicit completeness metadata.
- Add `session.previewResume` as a read-only adapter-native continuation
  preview that returns a copyable command or a typed unsupported reason.
- Include stable evidence references, action references, source revision, and
  completeness in every new response.
- Decide whether `app.stateSnapshot` embeds compact project projections or
  leaves them as independently prewarmed calls; preserve one startup source of
  truth either way.
- Keep `app.search` deterministic and lexical. Do not hide provider calls in a
  search request.
- Update supported-method inventories, Swift wire models, JSON fixtures,
  method-effect declarations, and protocol drift checks together.

### Primary Areas

- `crates/service/src/protocol.rs`
- `crates/service/src/service_host.rs`
- `crates/service/src/service_app_search.rs`
- `crates/service/src/service_local_sessions.rs`
- `fixtures/service-protocol`
- `docs/service-protocol.md`

### Exit Criteria

- New methods are fully typed, fixture-backed, bounded, local-only, and listed
  in method-effect tests.
- Unknown or stale evidence/revision inputs fail with stable typed errors.
- Existing catalog, session, search, and `task_cockpit` consumers remain wire
  compatible.

### Verification

```sh
cargo test -p skills-copilot-service
pnpm verify:service-protocol-drift
pnpm verify:gate-parity
```

## Task 5 — Native State And Routing Foundation

### Objective

Replace skill-centric application routing with project-first workspace routing
while retaining one controlled composition root.

### Work

- Introduce `AppContextStore`, `SkillWorkspaceStore`, and
  `SessionWorkspaceStore` boundaries.
- Retain `SkillStore` as the composition root until call sites have migrated;
  do not perform an unrelated full rewrite.
- Introduce an `AppRoute` model for overview, skills, sessions, and advanced
  destinations.
- Move route selection out of skill-only `SidebarSelection` assumptions.
- Extend `ServiceClient` with typed calls for the product read methods.
- Define cache ownership, cancellation, accepted-revision behavior, and minimal
  domain refresh rules.
- Add native model tests before replacing visual surfaces.

### Primary Areas

- `apps/macos/Sources/SkillsCopilot/Stores`
- `apps/macos/Sources/SkillsCopilot/Models/SidebarSelection.swift`
- `apps/macos/Sources/SkillsCopilot/Services`
- `apps/macos/Sources/SkillsCopilot/Views/ContentView.swift`
- `docs/architecture.md`

### Exit Criteria

- Overview, skills, sessions, and advanced routes can be selected and restored
  without manufacturing a selected skill.
- Workspace stores consume typed service projections and do not reproduce Rust
  policy.
- Cancellation or a failed refresh preserves the last accepted data and
  selection.
- Existing config/provider flows remain reachable during migration.

### Verification

```sh
pnpm test:macos-native-models
swift test --package-path apps/macos
pnpm verify:module-size
```

## Task 6 — Project-First Shell And Overview

### Objective

Make project status, task readiness, attention, and continuity the product
entry point.

### Work

- Order toolbar controls as project, agent filter, global search, and settings.
- Keep individual recent-project removal and place a compact localized `Clear`
  action in the recent-project section header.
- Limit primary navigation to Project Overview, Skills, and Sessions.
- Implement the four overview sections in the product-design order.
- Render per-agent environment health, coverage, incomplete reasons, and last
  successful refresh from product projections.
- Route attention items to their evidence and typed action preview.
- Re-express Task Preflight as inline task readiness while retaining the
  existing `task_cockpit` wire identifier.
- Route recent sessions into the Sessions workspace or continuation preview.
- Add empty, loading, stale, partial, blocked, provider-disabled, and error
  presentations.

### Primary Areas

- `apps/macos/Sources/SkillsCopilot/Views/ContentView.swift`
- `apps/macos/Sources/SkillsCopilot/Views/SidebarView.swift`
- project/task models and localization resources
- `docs/ui-delivery-standards.md`

### Exit Criteria

- Selecting a project leads to one coherent overview rather than a skill
  detail.
- No incomplete source can render as healthy.
- The core overview remains complete with provider features disabled.
- Project history actions are keyboard accessible, localized, and do not clear
  active context implicitly.

### Verification

```sh
pnpm test:macos-native-models
pnpm verify:macos-ui-layout
pnpm verify:list-completeness
pnpm check:macos
```

## Task 7 — Skills Workspace And Integrated Package Management

### Objective

Present capabilities by user meaning while preserving complete instance-level
evidence and safe package actions.

### Work

- Consume `SkillAggregateRecord` for list counts, filters, rows, and detail.
- Implement Needs Attention, Project, Global, and All in that default order.
- Keep distinct definitions separate even when names match.
- Implement Answer, Evidence, and Advanced detail layers.
- Display installed, enabled, verified effective, shadowed, broken, unavailable,
  and installed-but-unlinked states without conflation.
- Explain plugin, compatibility, native, manager, and local provenance with
  logical labels rather than physical cache paths.
- Integrate Add, Update, Remove, import, and local-create flows into Skills while
  preserving the `skillManager.*` service boundary and confirmation contract.
- Keep config enable/disable separate from package ownership actions.
- Replace generic skill LLM buttons with one contextual intelligence entry
  point; leave implementation provider-safe until Task 10.

### Primary Areas

- skill views, models, stores, and manager panels under `apps/macos`
- `scripts/list-completeness-surfaces.json`
- localization and native model tests
- `docs/ui-delivery-standards.md`
- `docs/adapters/agent-adapters.md`

### Exit Criteria

- Aggregate totals equal their evidence instances and expose completeness.
- Plugin and compatibility sources are understandable without revealing a raw
  cache path.
- Package actions and agent enablement cannot be mistaken for each other.
- All list rows remain reachable through verified complete or explicitly paged
  controls.

### Verification

```sh
pnpm test:macos-native-models
pnpm verify:list-completeness
pnpm check:macos
```

## Task 8 — Sessions Workspace And Native Continuation

### Objective

Turn local session inventory into a trustworthy project continuity workflow.

### Work

- Group sessions by project while retaining agent, time, source revision, and
  completeness filters.
- Implement Summary, Timeline, and Evidence detail layers.
- Preserve bounded keyset summary paging and fixed-revision message paging.
- Keep local lexical search available over accepted rows.
- Add deterministic `session.previewResume` consumption and a copy-only resume
  command action.
- Show a typed unsupported reason when an adapter cannot prove a native resume
  command.
- Do not launch a terminal, automatically resume, or translate sessions across
  agents.
- Prepare evidence anchors for later AI digest and suggested-next-prompt output.

### Primary Areas

- `apps/macos/Sources/SkillsCopilot/Views/AgentSessionDetailPanel.swift`
- session models, stores, and service client RPC
- service local-session fixtures
- `docs/service-protocol.md`
- `docs/ui-delivery-standards.md`

### Exit Criteria

- A user can find a local conversation by project, agent, time, or content and
  inspect its source limitations.
- A resume command is shown only when the service verified the adapter-native
  form for that exact session and revision.
- Paging, cancellation, source change, and incomplete inventory tests preserve
  accepted rows without duplicates or silent gaps.

### Verification

```sh
cargo test -p skills-copilot-service local_session
pnpm test:macos-native-models
pnpm verify:list-completeness
pnpm check:macos
```

## Task 9 — Advanced And Settings Consolidation

### Objective

Keep expert mechanisms available without making them primary product concepts.

### Work

- Move configuration inspection, snapshots, rollback, raw metadata, diagnostics,
  and expert repair tools under Advanced.
- Keep Agent Copilot provider profiles and provider activity under Settings or
  Advanced, clearly scoped to this app's optional requests.
- Preserve typed config conflict, snapshot revision, rollback, reveal, and
  redaction behavior.
- Route normal users from overview or workspace answers to the smallest
  relevant advanced evidence rather than presenting config first.
- Ensure global search still routes to advanced records when explicitly
  matched.

### Primary Areas

- config and settings views/models under `apps/macos`
- `ProviderObservabilitySettingsPanel.swift`
- app search routing
- `docs/ui-delivery-standards.md`
- `docs/security-model.md`

### Exit Criteria

- Project Overview, Skills, and Sessions remain the only primary navigation.
- Every existing guarded config and provider function remains reachable and
  correctly labeled.
- Raw metadata and sensitive path reveal stay explicit and redacted by default.

### Verification

```sh
pnpm test:macos-native-models
pnpm verify:macos-ui-layout
pnpm check:macos
```

## Task 10 — Evidence-Bound AI Contract

### Objective

Make optional AI responses structured, revision-bound interpretations of local
product evidence.

### Work

- Define one response envelope containing input/source revision, evidence
  references, optional action references, structured result schema, and safety
  flags.
- Reject unknown evidence references, unknown action references, target drift,
  and stale source revisions.
- Preserve `task_cockpit` as the wire-compatible task-readiness action.
- Add `session_digest` and `skill_change_review` prompt actions.
- Keep prompt preview, redaction, destination visibility, and explicit
  confirmation mandatory for every provider request.
- Keep output copy-only and prevent it from creating commands, mutations,
  scripts, prompt persistence, or hidden task state.
- Store only existing redacted app-local observability metadata; do not persist
  raw prompts, responses, transcripts, or embeddings.

### Primary Areas

- `crates/service/src/service_llm.rs`
- `crates/service/src/service_llm_prompt_helpers.rs`
- `crates/service/src/provider.rs`
- `crates/ai-core/src/lib.rs`
- LLM fixtures and Swift wire models
- `docs/ai-layer.md`
- `docs/security-model.md`

### Exit Criteria

- Each accepted result resolves every evidence and action reference against the
  bound project revision.
- Provider-disabled, cancelled, rejected, stale, malformed, and unsupported
  cases have typed non-destructive behavior.
- No AI result can cross the deterministic action preview boundary.

### Verification

```sh
cargo test -p skills-copilot-service llm
cargo test -p skills-copilot-ai-core
pnpm verify:service-protocol-drift
pnpm check:privacy
```

## Task 11 — Contextual Intelligence And Search

### Objective

Apply the Task 10 contract only where it reduces interpretation cost in the
three primary workspaces.

### Work

- Add plain-language project health and attention explanations grounded in
  evidence.
- Add task-specific agent/skill relevance, missing-capability interpretation,
  and concise rationale to inline task readiness.
- Add one contextual skill explanation/change-review flow.
- Add evidence-cited session digest and suggested next prompt.
- Keep `app.search` lexical and local; offer semantic reranking only as an
  explicit provider action over the already-returned candidate set.
- Visibly distinguish deterministic facts, AI interpretation, stale output,
  and unsupported claims.
- Keep every primary flow useful when AI is off or network access is denied.

### Primary Areas

- overview, skill, session, and search native models/views
- service LLM prompt actions and fixtures
- localization and accessibility tests
- `docs/ai-layer.md`

### Exit Criteria

- The app exposes contextual intelligence, not a general chat destination.
- AI citations navigate to the exact evidence represented by the response.
- Revision changes visibly stale prior output and prevent its actions from
  applying.
- Lexical search and deterministic workspaces remain unaffected when AI is off.

### Verification

```sh
cargo test -p skills-copilot-service llm
pnpm test:macos-native-models
pnpm check:macos
pnpm check:privacy
```

## Task 12 — Legacy Surface Removal And Compatibility Cleanup

### Objective

Remove duplicate product surfaces after their capabilities have migrated,
without breaking required compatibility identifiers or wire consumers.

### Work

- Remove standalone Task Preflight and Skill Manager navigation/panels only
  after overview and Skills contain complete replacements.
- Remove Config from primary navigation only after Advanced routing is complete.
- Remove generic duplicate LLM entry points after contextual intelligence is
  available.
- Remove raw five-tab detail structures after Answer, Evidence, and Advanced
  cover every supported function.
- Delete dead Swift presentation/state code and obsolete localization keys.
- Retain `task_cockpit`, `codex`, bundle/app-data migration names,
  `SkillsCopilot` module/AX identifiers, and other compatibility values where
  fixtures or migration contracts still require them.
- Update formal-list manifests, module-size budgets only when justified by real
  ownership changes, and all affected docs in the same change.

### Primary Areas

- native views, models, stores, resources, and tests
- compatibility fixtures
- documentation index and README files

### Exit Criteria

- There is one user-facing entry point for each product job and no orphaned
  route to a removed surface.
- Search, deep links, selection restoration, keyboard navigation, localization,
  and accessibility all resolve through the new route model.
- Compatibility identifiers are retained only for an inspected, documented
  reason.

### Verification

```sh
pnpm verify:module-size
pnpm verify:macos-ui-layout
pnpm verify:list-completeness
pnpm check:macos
pnpm check:privacy
```

## Task 13 — Automated Contract Verification

### Objective

Make the redesigned product contract enforceable through deterministic local
and CI checks.

### Work

- Add cross-agent fixtures covering global/project skills, plugin and
  compatibility sources, missing/partial roots, session inventories, and
  resume capability.
- Add protocol fixtures for readiness, skill aggregates, resume preview, AI
  evidence envelopes, and stale-reference errors.
- Add native model tests for route restoration, coverage semantics, status
  language, workspace selection, action lifecycle, provider-off behavior, and
  complete-list access.
- Update smoke fixture flows for overview, skills, sessions, advanced config,
  and non-destructive continuation preview.
- Keep smoke data isolated from real agent config and keep screenshots outside
  the repository.

### Primary Areas

- Rust and Swift tests
- `fixtures`
- smoke and governance scripts
- `docs/runbooks/macos-app-runbook.md`

### Exit Criteria

- Protocol drift, list completeness, UI layout, privacy, fixture smoke, and the
  full macOS gate cover the new product surfaces.
- A deliberate incomplete scan cannot pass a healthy-state assertion.
- A deliberate stale AI/action/session revision cannot pass validation or
  mutate local state.

### Verification

```sh
pnpm verify:gate-parity
pnpm check:macos
pnpm check:privacy
```

## Task 14 — Real-Local Cross-Agent Acceptance

### Objective

Prove that app projections match the local agent environments and that the
redesigned workflows are operable in the real native app.

### Work

- Build and launch the current bundle against the developer's real local HOME.
- Select the `funnyaccount_system` project.
- For Claude Code, Codex, opencode, Pi, Hermes, and OpenClaw, compare app global
  and project skill instances, counts, effectiveness states, provenance, and
  completeness with the agent's documented native effective inventory.
- Obtain any runtime-native skill inventory only temporarily for comparison;
  do not persist it, add it as an app source, or broaden scan roots.
- Compare app session lists, project filtering, search results, detail evidence,
  and continuation capability with each adapter's documented native session
  source and command behavior.
- Exercise Overview, Skills, Sessions, integrated manager entry points,
  Advanced, task readiness with provider off, and any explicitly confirmed AI
  flow included in the acceptance scope.
- Capture only full app-window images outside the repository and inspect them
  for private paths or secrets.
- Record exact comparisons, commands, results, and any canonical Computer Use
  blocker in the GitHub issue or pull request.

### Primary Areas

- built `dist/AgentCopilot.app`
- real local adapter sources under documented read boundaries
- `docs/runbooks/macos-app-runbook.md`

### Exit Criteria

- Every displayed positive status and count is reconcilable with local source
  evidence, or the UI exposes an accurate incomplete/unsupported reason.
- Every supported resume command is adapter-native, revision-bound, copy-only,
  and reproducible; unsupported adapters are labeled accurately.
- Project and agent switching never leaks rows from another projection.
- Required real-local interaction is completed, or one canonical environment
  blocker is recorded without substituting fixture evidence.
- Final handoff includes automated gate results and real-local comparison
  evidence outside repository documentation.

### Verification

```sh
pnpm check:macos
pnpm dev:macos
pnpm check:privacy
```
