# V2.68 Task Cockpit IA Verification

Date: 2026-06-13

## Commands

- `cargo test -p skills-copilot-service --no-default-features task_cockpit -- --nocapture`
- `cargo test -p skills-copilot-service --no-default-features service_protocol_fixtures_decode -- --nocapture`
- Retired cleanup workflows are no longer part of the current app surface; current verification uses the service protocol and macOS checks.
- `swift test --package-path apps/macos`
- `pnpm verify:macos-ui-layout`
- `pnpm check:macos`
- `pnpm check:privacy`
- `git diff --check`

## Visual Evidence

- Completed fixture screenshot: `docs/ui-artifacts/v2.68-task-cockpit-ia/completed.png`.
- The screenshot is app-window-only and generated from fixture data.
- Manual inspection: Task Cockpit is selected by default, the duplicate detail picker label is gone, and Work surfaces appear before Adapter/Health diagnostics.

## Real Local Validation

The current `dist/SkillsCopilot.app` launched against real local app data and CG window metadata found the app window. The session was locked (`CGSSessionScreenIsLocked=Yes`), Computer Use timed out, and final direct capture was black. The black screenshot was rejected and not committed.

This is recorded as the V2.68 locked-session/window-capture blocker. V2.69 later added screenshot privacy plus black-image rejection, and V2.72 added canonical blocker classification; real-local visual validation should still be retried after unlocking.
