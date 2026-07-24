# UI Delivery Standards

This file captures the native UI and runtime-validation contract. Product
meaning and acceptance criteria are defined in `docs/product-design.md`.

## Product Shell

- The maintained UI is the native macOS app in `apps/macos`.
- Do not recreate `ui/`, `src-tauri/`, or Tauri IPC.
- User-facing behavior should use existing SwiftUI/AppKit view, model, service,
  localization, and fixture patterns.

## Product Surface Contract

- Toolbar order is project selector, agent filter, global search, and settings.
- The recent-project section supports per-entry removal and a compact localized
  `Clear` action in its header. Clearing history does not implicitly clear the
  active project.
- Primary navigation contains exactly Project Overview, Skills, and Sessions.
- Settings uses a dedicated macOS scene. Its Advanced page is the discoverable
  entry for Agent configuration, recovery history, and privacy-safe service
  diagnostics; opening configuration activates the non-primary Advanced
  workspace in the main window without an implicit read or scan.
- Project Overview renders, in order, project/agent environment health, inline
  task readiness, a needs-attention action queue, and continue-work sessions.
  It covers empty-project, empty-snapshot, loading, ready, stale, partial,
  blocked, provider-disabled, and error presentation without replacing a last
  accepted snapshot with an ambiguous loading screen.
- `Last successful refresh` is the local time at which the visible typed
  readiness record was accepted. The source revision is displayed separately
  as snapshot identity.
- Overview evidence opens as read-only evidence. Attention actions open a
  typed capability preview and route to the owning workspace; the preview
  never applies an action. Session continuation is copy-only, never launches a
  terminal, and appears only when the service provides supported ordered argv.
- Skill Package Manager actions are integrated into Skills. The compatibility
  action `task_cockpit` is presented as inline task readiness rather than a
  separate destination.
- Overview attention targets open the smallest owning surface: config opens
  Advanced configuration, provider profiles open Provider Settings, and
  app-local provider history opens Provider Activity.
- Skills presents Needs Attention, Project, Global, and All in that order.
  The list reads `SkillAggregateRecord` rows, labels aggregate and instance
  counts separately, and keeps same-name definitions separate when their
  definition, logical source, package, scope, or runtime identity differs.
  Search, filters, sorting, and selection use the accepted aggregate cache;
  loading the route or invalidating a filter must not select the first skill.
- Skill detail is ordered Answer, Evidence, and Advanced. Answer explains
  purpose, verified effective locations, and attention state. Evidence retains
  every instance, effectiveness state, finding/conflict total, coverage,
  evidence reference, and service-owned action. Advanced shows safe logical
  metadata rather than a physical cache path.
- Package ownership actions open one contextual Skill Manager presentation.
  The entry context may choose a cached workflow, scope, or uniquely matched
  inventory row, but it is not authorization and never bypasses the existing
  preview, confirmation, apply, and read-back lifecycle. Ambiguous same-name
  inventory matches remain unselected.
- Sessions groups accepted rows by project and retains Agent, activity time,
  native source revision, and inventory completeness. Project/all scope,
  Agent, lexical search, and sort changes project accepted rows locally and
  never manufacture a selected session.
- Session detail is ordered Summary, Timeline, and Evidence. Timeline pages
  contain only bounded user messages and final Agent replies from one fixed
  native source revision; Load More, Load All, cancellation, and page failure
  preserve already accepted messages. Evidence uses logical source identity,
  typed coverage, revisions, and evidence references without revealing a
  physical transcript path.
- Continue first requests `session.previewResume` for the exact selected
  session, native source revision, and accepted product snapshot revision.
  Supported ordered argv is copy-only. Unsupported capability renders its
  typed reason; the app never synthesizes argv, launches a terminal, resumes
  automatically, or translates a conversation between Agents.
- Package Add/Update/Remove/import/local-create and agent config
  enable/disable are separate action groups. Skill Manager never presents
  config enablement as package state.
- Config, raw metadata, diagnostics, provider profiles, and Agent Copilot
  provider activity are Advanced or Settings surfaces.
- `ContentView` routes to a workspace and must not assume every selection is a
  skill detail.

## Truth And Disclosure

- Environment health is distinct from task readiness. Without a task, the UI
  may report verified environment state but not universal agent readiness.
- Installed, enabled, and verified effective are distinct labels. Partial,
  stale, unavailable, and source-limited evidence never renders as healthy.
- Disabled, shadowed, broken, unavailable, and installed-but-unlinked are
  distinct skill effectiveness states and are never collapsed into a generic
  disabled or missing label.
- Aggregate counts display completeness or a typed limitation.
- Skill detail and session detail lead with Answer/Summary, then Evidence, then
  Advanced detail. Raw paths and metadata are never the default explanation.
- Deterministic facts, AI interpretation, and action preview are visually and
  semantically distinct.
- Every action follows Detect, Explain, Evidence, Preview, Confirm, Apply, and
  Read-back. The UI never treats AI prose as a write control.
- The advanced config editor never autosaves. Editing creates only a local
  draft; Save requests a typed preview, shows the reviewed impact for explicit
  confirmation, and applies that immutable confirmation once. Revert restores
  the loaded document locally without a write.
- A stale config save or rollback discards its confirmation, loads current
  config evidence, and requires another preview; it is never retried
  automatically. Verified apply publishes the returned config document and
  refreshes only the affected config-snapshot timeline. A later timeline
  refresh failure must not be presented as failure of the verified write.

## Responsive Loading Defaults

- Startup prewarm and explicit refresh buttons are the default places for
  expensive file scans, config reads, history reads, and analysis/statistics
  work.
- Scope pickers, filters, search, sort controls, sidebar navigation, and list
  row selection should derive from already loaded app data and must not clear the
  current UI into a loading state for routine changes.
- While a refresh is running, keep the last successful data visible and show
  progress only on the explicit refresh/loading control.
- Exceptions are consistency-bound flows that must read current data before a
  mutation or irreversible preview, such as config edit, save, rollback, and
  guarded write operations.
- When a fresh read is needed after user action, request the smallest domain
  needed for that action instead of rescanning unrelated agents, projects, or
  surfaces.

## Runtime UI Validation

- Screenshots used during a task must capture only the full app window and stay
  outside the repository.
- Full desktop screenshots are forbidden.
- Fixture smoke is not a substitute for required real-local UI interaction.
- If the session is locked or app-window capture is blocked, report the
  canonical blocker in the task or pull request.

## Validation

- For UI changes, run focused Swift tests when appropriate and `pnpm
  check:macos` for substantial or user-visible changes.
- Use `pnpm verify:macos-ui-layout` through `pnpm check:macos`, not as a
  replacement for the full gate.

## Formal List Completeness

- Declare every user-visible formal list in
  `scripts/list-completeness-surfaces.json` with its owning Swift type, real
  source anchor, completeness policy, total-count source, allowed limitations,
  and full-access accessibility control when the policy is paged or summarized.
  Paged and summarized entries also declare the exact live member scope that
  contains both the source anchor and control; comments, strings, unrelated
  members, and duplicate control IDs do not satisfy the verifier. Every paged
  entry also declares the exact target `control_anchor`; its status and
  full-access action must bind the same anchored footer/helper invocation.
- Run `pnpm test:list-completeness` while changing the verifier and `pnpm
  verify:list-completeness` for every list-surface change. New raw
  prefix-defined collections passed to `ForEach`, `List`,
  `DenseDisclosureList`, or `ExpandableSummaryList`, including multiline,
  closure-initialized, wrapper, helper, and computed-property aliases, are
  rejected unless the complete collection remains reachable through a
  canonical, verified disclosure component. Taint propagation stays within the
  owning Swift type/member/function scope.
- The manifest verifier proves declaration, source-path, and reachable-control
  wiring. Native model/UI tests separately prove pagination state, Load More,
  Load All, cancellation, Show All, canonical-list routing, and accessibility
  behavior; neither layer substitutes for the other.
