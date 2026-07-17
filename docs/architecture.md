# Architecture

Agent Copilot uses a native macOS shell over a typed Rust service. Product logic
belongs in Rust crates; the UI presents state and sends typed requests.

## Goals

- Inspect local agent sessions, skills, config snapshots, and validation
  evidence.
- Keep deterministic local analysis useful without default Agent Copilot
  provider calls.
- Keep write, script, credential, cloud, telemetry, and release automation
  surfaces narrow and explicit.
- Share the same Rust service contract across current and future UI shells.

## Non-Goals

- Do not replace agent runtimes or proxy their tool calls.
- Do not parse private prompts beyond explicitly authorized local preview
  flows.
- Do not add cloud sync, accounts, telemetry, or marketplace behavior by
  default.
- Do not reintroduce the removed Web/Tauri shell.

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

Startup and manual reload reuse the catalog inventory, but read-only skill
responses project the current guarded Codex `[[skills.config]]` overrides onto
cached list, detail, analysis, conflict, and health records in memory. This
keeps external ChatGPT enable/disable changes current without a directory scan
or catalog write; filesystem inventory changes still require an explicit scan.

### Scanner Bounds And Catalog Completeness

- A scan follows only explicit canonical adapter roots and explicitly declared
  same-scope link-target roots. It does not treat the whole home or project
  directory as an implicit symlink allowlist.
- Built-in user, project, compatibility, admin, and system root candidates are
  optional until their directory exists. A candidate that was never created is
  omitted without degrading the scan; explicit configured, plugin, and extra
  roots still report unavailability.
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
  share an agent and an effective runtime namespace. `$CODEX_HOME/plugins/cache`
  is excluded from adapter scans and all current projections, so it cannot
  participate in conflict analysis.
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
`Codex.app` bundle name. ChatGPT's plugin cache is not an adapter root and never
participates in current skill or conflict projections. Installation remains
isolated in the separately confirmed `skillManager.*` command path.
