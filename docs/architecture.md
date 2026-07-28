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

The native shell keeps `SkillStore` as the typed service-workflow coordinator,
while independently observable `SessionStore`, `ProviderStore`, and
`SkillManagerStore` own their surface state. Views subscribe only to the domain
state they render. Compatibility forwarding properties on `SkillStore` support
existing workflow code without making unrelated domain changes invalidate the
entire UI.

### Scanner Bounds And Catalog Completeness

- Scans are restricted to documented adapter roots and explicitly configured
  local roots. Symlinks may resolve only inside an authorized root of the same
  scope; discovery never expands that authorization.
- Production traversal is bounded to depth 64, 50,000 directories, 200,000
  entries, 25,000 skill files, 2 MiB per `SKILL.md`, and 256 MiB of aggregate
  skill content. Unreadable, oversized, unavailable, or rejected candidates
  produce bounded diagnostics without broadening the scan.
- Observed instances may always be upserted. Only roots proven completely
  enumerated may drive the catalog missing-sweep; partial or skipped roots must
  never turn unseen rows into missing records.
- Local-session summaries and selected details use bounded, source-scoped,
  progressively paged reads. They remain in memory and are not persisted by
  the native shell.
- UI filters, sorting, scope changes, and navigation project startup or
  manual-refresh caches. Rust includes a bounded `watch_plan` in the state
  snapshot from existing adapter roots, same-scope link targets, config-parent
  directories, and the app-owned local library. It rejects broad roots and any
  path with a symbolic-link component.
- The native shell uses FSEvents only to invalidate those caches. Event paths
  are ignored and are never logged or rendered. No event starts a scan.
  Explicit Refresh performs a full adapter reconciliation when invalidated and
  otherwise reloads cached catalog state; Deep Scan always re-enumerates the
  documented roots. Startup prewarm and consistency-bound write flows remain
  the other allowed sources of fresh filesystem work.

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
