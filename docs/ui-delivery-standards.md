# UI Delivery Standards

This file captures the current native UI and runtime-validation contract.

## Product Shell

- The maintained UI is the native macOS app in `apps/macos`.
- Do not recreate `ui/`, `src-tauri/`, or Tauri IPC.
- User-facing behavior should use existing SwiftUI/AppKit view, model, service,
  localization, and fixture patterns.

## Current Surface Rules

- Primary navigation contains Sessions, Skills, and Config.
- Session, skill, and config detail panes should render only the selected item
  or overview for the selected mode.
- Skill Manager and Task Preflight are first-class workflow entries. Their
  navigation cards present contained workflow sheets rather than replacing the
  primary Sessions, Skills, or Config context.
- At widths below the regular three-column breakpoint, keep primary navigation
  at its standard readable width and keep a usable selected-list width visible
  while presenting detail as a dismissible overlay.
- Long-form detail content uses a bounded readable width; zero-state metrics
  collapse instead of reserving full metric cards.
- Retired surfaces should not reappear without an explicitly scoped change.

## Responsive Loading Defaults

- Startup prewarm and explicit refresh controls are the default places for
  expensive file scans, config reads, history reads, and analysis/statistics
  work. The global primary action is Refresh (`Command-R`); Deep Scan
  (`Shift-Command-R`) is its secondary full re-enumeration action.
- Native file events may add a path-free pending indicator to Refresh, but they
  must not start work automatically. Refresh performs full reconciliation when
  the watcher has invalidated the cache and otherwise uses cached catalog data.
  Deep Scan always re-enumerates supported roots, including roots that were
  missing when the current watch plan was created.
- Scope pickers, filters, search, sort controls, sidebar navigation, and list
  row selection should derive from already loaded app data and must not clear the
  current UI into a loading state for routine changes.
- While a refresh is running, keep the last successful data visible and show
  progress only on the explicit refresh/loading control.
- Watcher tooltips, accessibility values, notifications, and errors must report
  only status and bounded counts; they must never expose an event path.
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
