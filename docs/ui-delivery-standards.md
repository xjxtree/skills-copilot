# UI Delivery Standards

This file captures UI and evidence standards.

## Product Shell

- The maintained UI is the native macOS app in `apps/macos`.
- Do not recreate `ui/`, `src-tauri/`, or Tauri IPC.
- User-facing behavior should use existing SwiftUI/AppKit view, model, service,
  localization, and fixture patterns.

## Current Surface Rules

- Primary navigation contains Sessions, Skills, and Config.
- Session, skill, and config detail panes should render only the selected item
  or overview for the selected mode.
- Agent Usage Report and Task Preflight are compact preview tools.
- Retired surfaces should not reappear without a new scoped version.

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

## Evidence Screenshots

- Completed screenshots must capture only the full app window.
- Full desktop screenshots are forbidden.
- Fixture smoke screenshots are not substitutes for required real-local UI
  evidence unless the checklist explicitly says no fresh UI evidence is needed.
- If the session is locked or app-window capture is blocked, record the
  canonical blocker.

## Validation

- For UI changes, run focused Swift tests when appropriate and `pnpm
  check:macos` for milestone or user-visible changes.
- Use `pnpm verify:macos-ui-layout` through `pnpm check:macos`, not as a
  replacement for the full gate.
- Use `pnpm verify:screenshot-artifacts` when adding, removing, or reorganizing
  screenshot evidence.

## Formal List Completeness

- Declare every user-visible formal list in
  `scripts/list-completeness-surfaces.json` with its owning Swift type, real
  source anchor, completeness policy, total-count source, allowed limitations,
  and full-access accessibility control when the policy is paged or summarized.
  Paged and summarized entries also declare the exact live member scope that
  contains both the source anchor and control; comments, strings, unrelated
  members, and duplicate control IDs do not satisfy the verifier.
- Run `pnpm test:list-completeness` while changing the verifier and `pnpm
  verify:list-completeness` for every list-surface change. New raw
  `ForEach(...prefix(...))` and `List(...prefix(...))` presentations, including
  prefix-defined aliases and computed properties, are rejected unless the
  complete collection remains reachable through a canonical, verified
  disclosure component.
- The manifest verifier proves declaration, source-path, and reachable-control
  wiring. Native model/UI tests separately prove pagination state, Load More,
  Load All, cancellation, Show All, canonical-list routing, and accessibility
  behavior; neither layer substitutes for the other.
