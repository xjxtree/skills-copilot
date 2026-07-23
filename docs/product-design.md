# Product Design Contract

This document defines the product Agent Copilot is being shaped into. It is a
normative design contract for product, domain, service, and native UI work. It
does not record delivery status. The dependency-ordered engineering contract is
in [Implementation Sequence](implementation-sequence.md).

## Product Promise

Agent Copilot is a local, verifiable readiness and continuity control center
for coding agents on macOS.

The product promise is:

> Make every coding agent verifiably ready in the selected project, and make it
> easy to continue work from where it stopped.

Every primary surface must help the user complete one of three jobs:

1. Confirm what is available and effective in this project.
2. Understand and resolve anything that needs attention.
3. Find and continue previous work with trustworthy local evidence.

## Product Boundaries

Agent Copilot is not:

- an agent runtime, tool-call proxy, or autonomous agent orchestrator;
- an IDE, general chat client, or provider dashboard;
- a generic plugin marketplace or an alternate source of agent truth;
- cloud memory, cloud sync, or an account system;
- an autonomous AI editor or skill-script execution environment.

The app may inspect and operate supported local agent state only through the
documented adapter, service, security, and confirmation boundaries.

## Product Ontology

The following terms are canonical across code, protocol, documentation, and UI.

| Object | Product meaning | UI role |
| --- | --- | --- |
| Project | The selected local workspace and the context in which evidence is interpreted | Primary context and home |
| Agent | A supported coding runtime whose local capabilities and sessions are inspected | Actor, status dimension, and filter |
| Skill | A local capability definition plus one or more agent-specific instances | Capability |
| Session | A user-owned local conversation that can be found, understood, and continued | Memory and continuity |
| Config | The mechanism that influences agent behavior | Advanced diagnostic and repair surface |
| AI | An optional interpreter of already-collected evidence | Explanation and ranking aid, never source of truth |

Project is the organizing object. Agent is not a separate universe that hides
project context; it is a filter and status dimension inside the selected
project. Skills and sessions are the two primary workspaces. Config, raw
metadata, provider settings, and diagnostics support those workspaces and live
under Advanced or Settings.

## Truth Model

### Environment Health

Environment health is deterministic and task-independent. It answers whether
the app could completely inspect the selected project and whether each selected
agent has a coherent, usable local setup.

Canonical states are:

| State | Meaning |
| --- | --- |
| `healthy` | Required evidence is complete and no blocking deterministic issue is present |
| `review` | Evidence is complete enough to assess, but one or more non-blocking issues need attention |
| `blocked` | A blocking issue or incomplete required evidence prevents a trustworthy positive result |

An incomplete, partial, stale, unavailable, or source-limited inspection must
never be rendered as healthy. The UI must show coverage, reason, and recovery
action instead of manufacturing a complete total.

### Task Readiness

Task readiness is task-specific. It exists only when the user supplies a task
or selects an equivalent explicit work intent. It answers which agents and
skills appear relevant, which prerequisites are missing, and what evidence
supports that interpretation.

Without task context, the product may say that an environment was verified. It
must not claim that an agent is ready for any task.

Task readiness combines deterministic project evidence with optional AI
interpretation. Deterministic coverage and blocking conditions remain
authoritative even when AI is enabled.

### Skill Presence And Effectiveness

Installed, enabled, and effective are separate facts:

- Installed means a valid source exists in an authorized adapter or manager
  location.
- Enabled means the applicable agent configuration permits that source.
- Effective means the deterministic adapter projection says the instance is
  available after scope, precedence, plugin, compatibility, and config rules.
- Verified effective describes adapter-projected local state, not runtime
  telemetry or proof that a model invoked the skill.

The canonical effectiveness states are:

| State | Meaning |
| --- | --- |
| `effective` | Installed, permitted, valid, and selected by deterministic precedence |
| `disabled` | Installed but explicitly disabled by the owning agent configuration |
| `shadowed` | Valid, but another instance wins the same agent runtime identity |
| `installed_unlinked` | Installed package source exists but is not linked to the selected agent/context |
| `broken` | The source was found but cannot produce a valid skill definition |
| `unavailable` | Required source or evidence is missing, partial, stale, or otherwise unreadable |

The UI must not collapse these states into a single enabled badge.

## Information Architecture

### Product Shell

The toolbar order is:

1. Project selector.
2. Agent filter.
3. Global search.
4. Settings.

The project selector is the first control because every projection depends on
project context. Its recent-project section supports removing one entry and a
compact `Clear` action in the section header. Clearing history never clears the
active project unless the user separately chooses the explicit clear-project
action.

Primary navigation contains exactly:

1. Project Overview
2. Skills
3. Sessions

Config, diagnostics, raw metadata, Agent Copilot provider settings and
activity, and expert repair tools are grouped under Advanced or Settings. They
must not compete with the three primary user jobs.

### Project Overview

Project Overview is the default destination after a project is selected. It
contains four sections in this order:

1. Project status: evidence coverage, last refresh, and per-agent environment
   health.
2. Task readiness: an inline task field and task-specific interpretation. The
   existing `task_cockpit` wire action remains compatible, but Task Preflight is
   not a separate product destination.
3. Needs attention: a deterministic action queue ordered by severity and user
   impact.
4. Continue work: recent project sessions and verified continuation actions.

The overview answers the project-level question before showing inventory. It
must not begin with a wall of raw counts.

### Skills Workspace

The default skill views are ordered:

1. Needs Attention
2. Project
3. Global
4. All

The list uses skill aggregates for comprehension while preserving distinct
instances as evidence. Aggregation is based on definition identity, source,
and package provenance. Same-name skills with different content, source,
publisher, package, scope, or effective runtime identity must not be silently
merged.

Skill detail uses three disclosure layers:

- Answer: what the skill is, where it is effective, and whether attention is
  required.
- Evidence: instances, agents, scopes, precedence, provenance, findings,
  conflicts, and coverage.
- Advanced: raw metadata, history, config mechanisms, and expert diagnostics.

Skill Package Manager is integrated into the Skills workspace as Add, Update,
Remove, and package-detail actions. The existing `skillManager.*` service
boundary remains the only manager execution path. Plugin discovery and package
management remain separate ownership domains.

### Sessions Workspace

Sessions are grouped by project and described by intent, agent, activity time,
and continuation availability. The default detail hierarchy is:

- Summary: title, intent, participants, outcome, and suggested continuation.
- Timeline: bounded user messages and final agent replies, progressively
  loaded from the fixed source snapshot.
- Evidence: source coverage, revision, adapter, project match, and diagnostic
  limitations.

Local keyword search is always available. Optional AI may summarize or rerank
already-returned candidates only after the normal provider preview and
confirmation flow. AI must cite local evidence references.

The first continuation action is a copy-only, adapter-native command produced
by a deterministic preview. The app does not automatically launch a terminal,
resume a session, translate a session between agents, or claim continuation is
available when the adapter cannot verify it.

## Interaction Contract

All mutating or consequential actions follow:

```text
Detect -> Explain -> Evidence -> Preview -> Confirm -> Apply -> Read-back
```

- Detect and status derivation are deterministic.
- Explain states the reason in user language and links it to evidence.
- Preview is generated by typed product logic, not free-form AI output.
- Confirm identifies target, scope, impact, network posture, and expected
  revision where applicable.
- Apply uses the existing guarded service path.
- Read-back refreshes the smallest affected domain and recomputes projections.

AI may recommend an existing typed action reference. It cannot create an
arbitrary command, preview token, write target, or hidden apply path.

Routine filters, sort, navigation, selection, and scope changes use prewarmed
or manually refreshed caches. Fresh reads occur only at startup, explicit
refresh, bounded detail paging, or consistency-bound preview/write/read-back
flows.

## Intelligent Product Layer

AI is useful where interpretation is expensive and deterministic truth is
already available. It is not useful as a substitute for scanning, precedence,
enablement, source completeness, or write authorization.

| Experience | Deterministic foundation | Optional AI contribution |
| --- | --- | --- |
| Project status | Adapter evidence, coverage, findings, conflicts | Plain-language explanation and prioritization |
| Task readiness | Selected project, agents, effective skills, blockers | Relevance ranking, missing-capability interpretation, concise rationale |
| Skill review | Definition, provenance, instances, findings, changes | Contextual explanation, change review, overlap analysis |
| Session continuity | Session metadata, bounded messages, source revision, resume capability | Evidence-cited digest, intent inference, suggested next prompt |
| Search | Complete local lexical candidates | Explicit semantic rerank of those candidates |
| Action queue | Typed findings and action descriptors | Explanation of why an existing action may help |

The app exposes contextual intelligence rather than a general chat surface.
Skill detail should present one relevant intelligent explanation entry point
instead of several generic LLM buttons. Frontmatter drafting remains an
advanced, copy-only authoring aid.

Every accepted AI response is bound to:

- the input or source revision it interpreted;
- typed evidence references that resolve in the current project projection;
- typed action references, if any, that were independently created by product
  logic;
- a structured response schema and safety flags.

Responses become stale when the bound revision changes. Unknown evidence or
action references invalidate the result. The provider-off experience remains
complete and useful.

## Required Read Models

The user experience is built from Rust-owned read projections rather than
Swift-side reinterpretation of raw catalog rows.

| Read model | Responsibility |
| --- | --- |
| `ProjectReadinessRecord` | Project coverage, per-agent environment health, blocking reasons, attention queue, and recent continuation summaries |
| `SkillAggregateRecord` | Definition identity, package/source identity, agent/scope coverage, instances, effectiveness, findings, conflicts, and supported actions |
| `SessionContinuationRecord` | Project/agent identity, intent summary inputs, source revision, coverage, and deterministic resume capability |
| `EvidenceRef` | Stable typed reference to the local fact supporting a status, explanation, or action |
| `ActionDescriptor` | Deterministic action id, target, impact, preview/apply method, revision binding, and confirmation requirement |

These projections are additive. Existing catalog, session, config, and manager
contracts remain available during migration and are removed only when protocol
fixtures and native consumers no longer depend on them.

## Native Application Composition

The native app uses one composition root and domain-focused stores:

- `AppContextStore` owns project, agent filter, routing, startup state, and
  cross-workspace refresh coordination.
- `SkillWorkspaceStore` owns skill projections, selection, package operations,
  and skill-local intelligence.
- `SessionWorkspaceStore` owns session summaries, message paging, selection,
  resume preview, and session-local intelligence.

The existing `SkillStore` remains the composition root while behavior moves
behind these boundaries. Config and provider models remain specialized and are
routed through Advanced or Settings.

The route model distinguishes overview, skills, sessions, and advanced
destinations. `ContentView` chooses a workspace; it must not assume that every
selection ends in a skill detail view.

User-facing branding is Agent Copilot. Compatibility identifiers containing
`SkillsCopilot` may remain where migration, fixtures, bundle behavior, or AX
contracts require them.

## Experience Standards

- Lead with answers and next actions, not implementation metadata.
- Show coverage beside every aggregate count that could be incomplete.
- Use human source labels; keep physical cache paths and private local paths
  behind redacted evidence/reveal controls.
- Explain why a compatibility or plugin skill belongs to an agent instead of
  making directory names carry that meaning.
- Preserve the last successful data while refresh runs.
- Keep keyboard navigation, accessibility labels, localization, complete-list
  disclosure, and stable selection behavior for every primary workspace.
- Empty, loading, incomplete, blocked, and error states are distinct.
- A provider-disabled or network-blocked state never blocks deterministic
  project inspection, lexical search, or native resume preview.

## Acceptance Contract

The product design is satisfied only when all of the following are true:

### Clarity

- The app opens on project status after project selection.
- Only Project Overview, Skills, and Sessions are primary navigation.
- Skill management, task readiness, config, provider activity, and raw metadata
  appear in their defined supporting locations rather than duplicate top-level
  surfaces.
- A normal user can answer what is effective and what needs attention without
  reading raw paths or metadata.

### Trust

- Incomplete inspection never produces a healthy result.
- Installed, enabled, and effective remain visibly distinct.
- Every status, intelligent explanation, and supported action resolves to local
  evidence.
- Every write has deterministic preview, explicit confirmation, and read-back.
- The app remains useful with provider features disabled.

### Continuity

- Users can find sessions by project, agent, time, and local content.
- Supported adapters return verified native continuation commands; unsupported
  cases state a typed reason.
- AI session summaries cite evidence and become stale when source revision
  changes.

### Cross-Agent Accuracy

- Real-local validation uses the selected `funnyaccount_system` project and all
  six supported adapter families.
- App global/project skill projections are compared with each agent's native
  effective inventory without persisting temporary runtime inventory.
- App session lists and continuation capability are compared with each agent's
  documented native session source and command behavior.
- Computer Use captures only the complete app window outside the repository;
  fixture smoke never substitutes for required real-local evidence.
