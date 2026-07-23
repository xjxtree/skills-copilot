# Architecture

Agent Copilot uses a native macOS shell over a typed Rust service. It is a
project-first readiness and continuity control center: product truth belongs in
Rust crates; the UI presents typed projections and sends typed requests.

## Goals

- Verify what supported coding agents can effectively use in the selected
  project.
- Explain deterministic problems and expose evidence-bound, guarded repair
  actions.
- Find local user-owned sessions and continue work through verified native
  adapter behavior.
- Keep deterministic local analysis useful without default Agent Copilot
  provider calls.
- Keep write, script, credential, cloud, telemetry, and release automation
  surfaces narrow and explicit.
- Share the same Rust service contract across current and future UI shells.

## Non-Goals

- Do not replace agent runtimes or proxy their tool calls.
- Do not become an IDE, general chat client, agent orchestrator, or generic
  plugin marketplace.
- Do not parse private prompts beyond explicitly authorized local preview
  flows.
- Do not add cloud sync, accounts, telemetry, or marketplace behavior by
  default.
- Do not reintroduce the removed Web/Tauri shell.

## Product Architecture Contract

Project is the primary context. Agent is a status dimension and filter; skills
are capabilities; sessions are continuity; config is a supporting mechanism;
optional AI interprets evidence but never creates product truth.

Product-facing changes follow this flow:

```text
documented agent sources
  -> bounded deterministic adapters and scanner
  -> catalog plus transient session evidence
  -> Rust-owned readiness, skill, action, and continuation projections
  -> typed service protocol
  -> native project, skill, session, and advanced workspaces
```

Optional intelligence branches only after deterministic projection:

```text
redacted revision-bound evidence
  -> prompt preview and explicit confirmation
  -> structured evidence-referencing interpretation
  -> copy-only presentation or reference to an existing typed action
```

Mutations follow Detect, Explain, Evidence, Preview, Confirm, Apply, and
Read-back. Preview/apply authority remains in Rust. A provider result cannot
invent an action, target, command, revision token, or write path.

Provider profile save/delete, connection tests, and LLM prompt sends use the
same signed action descriptor and exact confirmation contract. Their tokens are
reserved once under a canonical local lock and finish as not-started, verified,
or partial in one bounded replay-state record rather than an action history.
Verified results carry semantic typed read-back; unknown local, credential, or
remote effects are explicit partial outcomes and are never automatically
retried.

## Layers

| Layer | Owner | Notes |
| --- | --- | --- |
| macOS app | `apps/macos` | SwiftUI/AppKit shell, view models, service client |
| Service boundary | `crates/service` | Typed JSON stdio request/response protocol |
| Command orchestration | `crates/commands` | Scans, toggles, snapshots, reports, provider gates |
| Core model | `crates/core` | Pure types and traits; no I/O |
| Adapters | `crates/adapters` | Agent root/config semantics |
| Scanner | `crates/scanner` | Root walking, symlink guards, skill parsing |
| Catalog | `crates/catalog` | Local SQLite catalog and app-local metadata |
| AI core | `crates/ai-core` | Deterministic rules and local analysis contracts |

Read projections are domain models, not UI convenience calculations. Core owns
their stable types; commands compose adapter/catalog/session evidence; service
owns bounded wire exposure; Swift owns presentation and interaction state only.

## Dependency Direction

- `core` does not depend on higher crates.
- `adapters` and `scanner` depend on `core`.
- `catalog` and `ai-core` depend on `core`.
- `commands` composes scanner/catalog/adapter/AI behavior.
- `service` exposes the UI-independent protocol boundary.
- UI code must not call scanner/catalog internals directly.

## Data Flow

1. The app calls a typed service method such as `catalog.scanAll`.
2. Commands resolve project context and adapter roots.
3. Scanner enumerates candidate `SKILL.md` files inside allowed roots.
4. Adapters parse agent-specific metadata and enabled state.
5. Commands update the local catalog and derived findings.
6. The app reads typed service results for list, detail, config, session, and
   report surfaces.

Product read projections are additive over this flow. Environment health,
skill effectiveness, aggregate counts, evidence references, attention actions,
and continuation capability must be computed from the same source revision and
completeness evidence. Swift must not reinterpret a partial source as complete
or derive an alternative enablement/precedence policy.

Commands accepts only one adapter-normalized, immutable product snapshot at a
time. Every agent source, skill, finding, conflict, action, and session
snapshot membership must carry the accepted source revision; mixed revisions
fail closed. Projection is pure and deterministic: it performs no filesystem,
catalog, provider, or config reads, and input order cannot change serialized
output. A continuation separately retains its native session-source revision:
its evidence binds that native revision, while any action descriptor binds the
accepted product snapshot revision. The core model validates both relationships.

The normalized skill facts distinguish definition fingerprint, logical source
identity, publisher/package/version, scope, and runtime identity. These are
material aggregation dimensions, so same-name rows are never merged merely
because their display names or physical files resemble one another. Logical
source identity is adapter-owned provenance such as a native root,
compatibility root, or manifest-declared plugin package; it is not inferred
from a cache path or content hash. Physical paths remain scanner/audit
evidence and do not become product identity.

Current projections omit historical `missing` skill rows. Each retained
instance has a separate effectiveness row covering installed, linked, enabled,
precedence-proven, completeness, and the derived effective/disabled/shadowed/
installed-unlinked/broken/unavailable state. Partial, stale, unreadable,
source-limited, or uninspected required evidence cannot yield an effective
instance or healthy agent/project result. Session continuation follows the
same rule: incomplete source evidence cannot expose supported native resume.
Aggregate and project coverage are the canonical merge of their child rows,
not a separately trusted count.

Typed actions remain capabilities, not authorization. A projection attaches an
action only when its target and evidence belong to the declaring skill, agent,
or session. Action ids and evidence references are canonical sets, and impacts
use fixed enum order; semantic sequences such as native resume argv retain
adapter order. Conflict precedence is equally fail-closed: a winner must be a
current, complete, enabled candidate in the same agent/runtime identity;
otherwise every member is unavailable and the conflict blocks readiness.

Startup and manual reload reuse the catalog inventory, but read-only skill
responses project the current guarded Codex `[[skills.config]]` overrides onto
cached list, detail, analysis, conflict, and health records in memory. This
keeps external ChatGPT enable/disable changes current without a directory scan
or catalog write; filesystem inventory changes still require an explicit scan.

### Scanner Bounds And Catalog Completeness

- A scan follows only explicit canonical adapter roots and explicitly declared
  same-scope link-target roots. It does not treat the whole home or project
  directory as an implicit symlink allowlist.
- Built-in user, project, compatibility, admin, system, and implicit package
  convention root candidates are optional until their directory exists. A
  candidate that was never created is omitted without degrading the scan;
  explicit configured, manifest-declared plugin, and extra roots still report
  unavailability.
- Scanner links may resolve only beneath those explicit roots with the same
  scope; link discovery never expands authorization to a neighboring scope.
- An unavailable symlink target is reported as a per-entry
  `dangling_symlink` diagnostic and skipped without making the surrounding
  authorized root partial. Resolvable targets outside the same-scope allowlist
  remain rejected as `root_outside_allowlist`.
- Production scans are bounded to depth 64, 50,000 directories, 200,000
  entries, 25,000 skill files, 2 MiB per `SKILL.md`, and 256 MiB of aggregate
  skill content. Skill content is read through the bounded scanner reader
  before adapter parsing.
- Oversized or unreadable skill files become broken catalog candidates when
  their canonical path is known, allowing enumeration to continue. Filesystem
  traversal failures or exhausted traversal budgets mark the root partial.
- Commands upsert every observed instance, but catalog missing-sweep receives
  only `(scope, canonical root)` pairs that were completely enumerated. A
  complete global root cannot sweep a partial project root that resolves to the
  same path. Partial and skipped roots never mark unseen rows missing. A first
  transition to missing and its compact `missing` event are committed
  atomically; repeated complete scans do not duplicate the event.
- Local-session UI state follows the same bounded-read posture: startup/manual
  refresh stores source-scoped summaries in memory, local criteria project that
  snapshot, and one selected stable ID may load a bounded process sample plus
  progressively paged user messages and final Agent replies. Each page has
  item, scan-byte, and returned-text bounds; accepted pages publish between
  sidecar calls and SwiftUI renders them lazily. Session summary/detail content
  is not persisted.
- Aggregate skill-byte accounting is based on bytes actually read. The bounded
  reader never retains more than the remaining aggregate allowance, including
  its limit-detection byte; stale file metadata cannot enlarge the read or
  allocation budget.
- Scanner reports snapshot declared/canonical root aliases during traversal.
  Service diagnostics lexically normalize and redact those immutable aliases
  longest-path-first without resolving the filesystem again after a scan.

### Raw Findings And User-Visible Issues

- The catalog retains current rule-finding records for local audit, including
  declaration-baseline warnings, collision-member records, local triage state,
  and rule-tuning suppressions. Raw catalog totals therefore are not UI issue
  totals.
- Rust owns the shared derived-finding policy used by health summaries and LLM
  prompt context. The native shell mirrors that policy for cached list/detail
  presentation: suppressions, reviewed/ignored triage, records without a
  current skill instance, and `name.collision` are excluded. Built-in
  declaration-baseline findings are excluded at warning/information severity
  but remain visible when raised to error/critical.
- Multi-skill runtime collisions appear only through the independent conflict
  projection. Broken and unknown current catalog states remain navigable
  single-skill issues even when no rule finding is attached. Missing rows are
  historical Deleted records and are excluded from the default All and Issues
  projections.
- Runtime collision membership is limited to loaded, enabled instances that
  share an agent and an effective runtime namespace. Installed Codex plugin
  skills use a package-specific namespace; separate plugins do not collide
  with each other, while a loaded native skill and loaded plugin skill with the
  same effective name can still surface a real same-agent conflict.
- Plugin runtime names use a colon-separated namespace such as
  `plugin:skill`. Namespaced values are excluded from the local
  `name.canonical-case` finding because the separator is runtime identity, not
  invalid skill authoring.
- Skill detail preserves the same boundary through independent `Skill Issues`
  and `Same-Agent Conflicts` sections; neither section renders the other's
  projection, and each header metric routes to its matching section.
- Sidebar totals, default filters, row/header badges, detail cards, refresh
  feedback, health summaries, and LLM related-finding context must use these
  current-skill projections instead of raw catalog counts. The explicit Deleted
  filter is the only skill-list filter that shows general historical missing
  rows.

Skill Manager follows a skill-first cache model. Startup and explicit manual
refresh load project/global package inventories; opening the panel and changing
its display scope or action targets perform no scan. Rust parses full bounded
manager JSON into compact skill rows and owns local archive validation and
replacement. Swift renders cached rows and sends typed, confirmation-bound
actions only after a skill is selected. Manager CLI rows are the primary
installed-package identity; matching catalog instances enrich and are consumed
by that row. Only guarded sources beneath the selected shared `.agents/skills`
root participate in the editable catalog fallback, including nested package
layouts. Plugin caches and other read-only discovery roots remain outside
package operations. Skill Manager loading, search, inventory, and preview use
surface-local busy state and do not block unrelated app actions; only a
confirmed Skill Manager write participates in the app-wide mutation gate.

## Extension Points

| Change | Add it here |
| --- | --- |
| New agent | `crates/adapters/src/<agent>/`, scanner/catalog tests, adapter docs |
| New service method | `crates/service`, fixtures, `docs/service-protocol.md` |
| New readiness, skill aggregate, or continuation projection | `crates/core` types, `crates/commands` composition, service fixtures, native wire models |
| New user-visible action | Deterministic `ActionDescriptor`, preview/apply/read-back service path, security review |
| New local rule | `crates/ai-core` |
| New macOS surface | `apps/macos` view/model/service patterns |
| New provider behavior | Provider profile gate with preview/redaction/confirmation |

## Compatibility

The displayed product name is Agent Copilot. Some module names, crate names,
sidecar names, AX identifiers, environment variables, or legacy app-data ids may
retain `SkillsCopilot` / `skills-copilot` compatibility where migration or
fixtures require it.

`codex` is likewise a stable adapter and protocol identity even when the
desktop runtime is hosted by `ChatGPT.app`. The adapter and local-session
service resolve one guarded `$CODEX_HOME`; neither assumes the former
`Codex.app` bundle name. Codex skill discovery is filesystem-only: it reads
verified native roots plus the manifest-declared skill roots of installed,
versioned plugin copies and never calls the Codex runtime inventory API.
Plugin files remain read-only, and installation stays isolated in the
separately confirmed `skillManager.*` command path.
