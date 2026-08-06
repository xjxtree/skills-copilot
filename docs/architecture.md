# Architecture

Agent Copilot uses a native macOS shell over a typed Rust service. Product logic
belongs in Rust crates; the UI presents state and sends typed requests.

## Goals

- Inspect local agent sessions, skills, and config snapshots.
- Keep deterministic local analysis useful without default Agent Copilot
  provider calls.
- Keep write, script, credential, cloud, and telemetry surfaces narrow and
  explicit.
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
- Native service diagnostics are redacted and collapsed for display. Values
  longer than the 512-character display boundary end with an explicit localized
  truncation marker so a shortened diagnostic is never presented as complete.
- UI filters, sorting, scope changes, and navigation project startup or
  manual-refresh caches. Rust includes a bounded `watch_plan` in the state
  snapshot from existing adapter roots, same-scope link targets, exact config
  files plus their subscription parents, and the app-owned local library. It
  rejects broad roots and any path with a symbolic-link component.
- The native shell uses FSEvents only to invalidate those caches. Event paths
  are matched in memory against the plan's recursive roots and exact files,
  immediately discarded, and never logged or rendered. Unrelated Agent
  session, log, database, and WAL writes are ignored. No event starts a scan.
  Explicit Refresh is the single manual refresh action and always performs a
  full adapter reconciliation across the documented roots. A successful scan
  clears only invalidations that existed
  when that scan began; a newer event remains pending for the next Refresh.
  Committing a project-context transition stops and invalidates the old watcher
  before validation or follow-up scanning, so a failed transition scan cannot
  leave stale project roots active. Startup prewarm and consistency-bound write
  flows remain the other allowed sources of fresh filesystem work.

### Raw Findings And User-Visible Issues

- The catalog retains current rule-finding records for local audit, including
  declaration warnings, collision-member records, local triage state,
  and rule-tuning suppressions. Raw catalog totals therefore are not UI issue
  totals.
- Rust owns the shared derived-finding policy used by health summaries and LLM
  prompt context. The native shell mirrors that policy for cached list/detail
  presentation: suppressions, reviewed/ignored triage, records without a
  current skill instance, and `name.collision` are excluded. Built-in
  declaration findings are excluded at warning/information severity
  but remain visible when raised to error/critical.
- Multi-skill runtime collisions appear only through the independent conflict
  projection. Broken and unknown current catalog states remain navigable
  single-skill issues even when no rule finding is attached. Missing rows are
  excluded from the default All and Issues projections.
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
  filter is the only skill-list filter that shows missing rows.

Skill Manager follows a skill-first cache model. Startup and explicit manual
refresh load project/global package inventories; opening the panel and changing
its display scope or action targets perform no scan. Rust parses full bounded
manager JSON into compact skill rows and owns local archive validation and
replacement. Rust also validates bounded local source directories, asks the
manager to enumerate them without installing, and binds their content revision
to later install confirmation. Swift renders cached rows, presents direct
folder installation separately from app-owned ZIP snapshot import, and sends
typed, confirmation-bound actions only after a skill is selected. Manager CLI
rows are the primary
installed-package identity; matching catalog instances enrich and are consumed
by that row. Only guarded sources beneath the selected shared `.agents/skills`
root participate in the editable catalog fallback, including nested package
layouts. Plugin caches and other read-only discovery roots remain outside
package operations. Skill Manager loading, search, inventory, and preview use
surface-local busy state and do not block unrelated app actions; only a
confirmed Skill Manager write participates in the app-wide mutation gate.
For a shared `.agents/skills` source, the inventory combines the manager row
with every physically installed supported Agent target found by the catalog,
regardless of that instance's enable/disable configuration, and retains every
exact instance identity for verification. The read-only inventory also runs
preview-equivalent checks over those identities and the documented install
roots to derive which Agents have an effective separable target: a link or copy
does not qualify when that Agent also reads the shared source or lacks an exact
catalog identity. Swift uses that capability set to block impossible partial
previews before RPC. Removing a proper subset is a guarded physical uninstall:
only a selected Agent's separable target may be removed. Apply moves exact
preview-bound entries outside scanned roots, rescans and verifies the entries are
gone while the shared source and unselected targets remain, then commits the
removal; verification failure restores the entries. If selected and unselected
Agents directly read the same `.agents/skills` directory, no separable
filesystem target exists and partial uninstall fails closed instead of writing
an enable/disable override. Removing all linked Agents is an explicit complete
uninstall: the external manager receives no `--agent` restriction, then catalog
and manager inventory are both read back before the write is reported as
successful.

## Extension Points

| Change | Add it here |
| --- | --- |
| New agent | `crates/adapters/src/<agent>/` and adapter docs |
| New service method | `crates/service` and `docs/service-protocol.md` |
| New local rule | `crates/ai-core` |
| New macOS surface | `apps/macos` view/model/service patterns |
| New provider behavior | Provider profile gate with preview/redaction/confirmation |

## Codex Integration

`codex` is a stable adapter and protocol identity even when the
desktop runtime is hosted by `ChatGPT.app`. The adapter and local-session
service resolve one guarded `$CODEX_HOME`; neither assumes the former
`Codex.app` bundle name. Codex skill discovery is filesystem-only: it reads
verified native roots plus the manifest-declared skill roots of installed,
enabled plugin copies and never calls the Codex runtime inventory API.
Plugin files remain read-only, and installation stays isolated in the
separately confirmed `skillManager.*` command path.
