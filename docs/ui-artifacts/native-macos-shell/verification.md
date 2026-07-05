# Native macOS Shell Verification

Date: 2026-06-17

## Build

- Ran `cargo test --workspace`: passed.
- Ran `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- Ran `cargo fmt --all -- --check`: passed.
- Ran `swift build --package-path apps/macos`: passed.
- Ran `./script/build_and_run.sh --verify`: passed; built `skills-copilot-service`, built SwiftPM app, assembled `dist/SkillsCopilot.app`, launched it, and verified the process exists.
- Ran `pnpm check:macos`: passed; this wraps Rust fmt/test/clippy, native list model test, native layout check, Swift build, Local App Launch Verify, Smoke App Run, and app-window-only screenshot capture.
- 2026-06-17 V2.87/V2.88 Agent Copilot validation:
  - `pnpm check:macos` passed end to end in an unlocked interactive session.
  - `./script/build_and_run.sh --verify` launched the current workspace `dist/SkillsCopilot.app`.
  - Fixture-data smoke captured the full Agent Copilot app window at `docs/ui-artifacts/native-macos-shell/completed.png`.
  - Computer Use resolved the current app window. Legacy V2.88 per-surface screenshots were later pruned because those pages no longer match the current UI.
- Latest deprecated Web/Tauri removal pass deleted `ui/` and `src-tauri/`, removed Tauri workspace membership/dependencies/scripts, moved the icon source to `apps/macos/Sources/SkillsCopilot/Resources/AppIcon.icns`, and verified the Cargo workspace now contains only the Rust crates used by the native service boundary.
- After the removal pass, `pnpm install --frozen-lockfile` passed and `pnpm check:macos` passed again: Rust reported 30 passed + 1 ignored, clippy was clean, native list model/layout checks passed, SwiftPM built the app, Local App Launch Verify passed, and Smoke App Run passed against the rebuilt `dist/SkillsCopilot.app`.
- Latest Smoke App Run verified bundle freshness, `service.status.protocol_version = 1`, bundle id `dev.skills-copilot.native`, bundled `AppIcon.icns`, visible `SkillsCopilot` window, scan, Enable/Disable, Claude Settings save, Snapshot Preview, and Snapshot Rollback.
- Latest UI productization pass added native sidebar State and Sort controls. `swift build --package-path apps/macos` passed before the full macOS check.
- Ran `pnpm test:macos-list-model`: passed; compiled the real Swift list model sources and verified search, state filter, findings/conflicts filter, and sort behavior.
- Ran `pnpm verify:macos-ui-layout`: passed; 11 native SwiftUI layout checks.
- Ran `pnpm benchmark:macos-list-model`: passed; 10k native Swift list model p95 max was 33.15ms across search/filter/sort scenarios.
- Ran `plutil -lint dist/SkillsCopilot.app/Contents/Resources/en.lproj/Localizable.strings`: passed.
- Latest localized resource check confirmed `Localizable.strings` is copied into `dist/SkillsCopilot.app/Contents/Resources/en.lproj/`.
- Latest CI positioning check: `.github/workflows/ci.yml` now uses Rust fmt/test/clippy plus native macOS list-model, layout, SwiftPM build, `dist/SkillsCopilot.app` build, and bundle-only smoke. Deprecated Web UI lint/test/build is no longer a product gate.
- Final session closeout check ran `pnpm check:macos`: passed. Rust reported 30 passed + 1 ignored, clippy was clean, native list model/layout checks passed, SwiftPM built the app, Local App Launch Verify passed, and Smoke App Run passed against the rebuilt `dist/SkillsCopilot.app`.
- V2 Prep integration check ran `pnpm run audit`: passed. `cargo audit` loaded 1122 RustSec advisories and scanned `Cargo.lock`; `pnpm audit --audit-level high` returned No known vulnerabilities found.
- V2 Prep integration check ran `pnpm check:macos`: passed. This now includes Rust fmt/test/clippy, native list model test, native layout check, `swift test --package-path apps/macos`, Swift build, Local App Launch Verify, and Smoke App Run with app-window-only screenshot capture. Rust reported 34 passed + 1 ignored across workspace tests, and SwiftPM reported `SkillsCopilotTests: native list/model checks passed`.
- V2 Prep refresh-experience slice:
  - Ran `cargo test -p skills-copilot-service`: passed with 9 tests; service protocol fixture decode and runtime scan activity tests covered the additive refresh metadata.
  - Ran `swift build --package-path apps/macos`: passed.
  - Ran `pnpm check:macos`: passed; this rebuilt `dist/SkillsCopilot.app`, ran Rust fmt/test/clippy, native list model/layout checks, SwiftPM tests/build, Local App Launch Verify, and fixture Smoke App Run. Smoke captured the full app window to `docs/ui-artifacts/native-macos-shell/completed.png`.
- V2.4 opencode read-only integration:
  - Ran `pnpm check:macos`: passed; this included Rust fmt/test/clippy, native list model test, native layout check, SwiftPM tests, Swift build, Local App Launch Verify, fixture Smoke App Run, and app-window-only screenshot capture.
  - Smoke verified opencode global native root scanning, project `.opencode/skills` scanning under project context, and read-only toggle rejection.
  - Fixture mode used temporary HOME/app data/project roots and did not touch real user opencode config.
  - Real local Computer Use operation was still pending at V2.4 closeout because the macOS/AX session could not resolve the SkillsCopilot window. The current mainline app later passed real local validation on 2026-06-09.
- 2026-06-09 current mainline validation:
  - Ran `pnpm check:macos`: passed; this included Rust fmt/test/clippy, native list model test, native layout check, SwiftPM tests/build, Local App Launch Verify, fixture Smoke App Run, and app-window-only fixture screenshot capture.
  - Ran the real local app from `<repo>/dist/SkillsCopilot.app` against the developer's real HOME/app data/Claude/Codex/opencode roots.
  - Captured app-window-only evidence at `docs/ui-artifacts/native-macos-shell/real-local-computer-use-2026-06-09.png`.
- 2026-06-09 final real local revalidation:
  - Ran `pnpm check:macos`: passed end to end.
  - Relaunched the real local app from `<repo>/dist/SkillsCopilot.app` after stopping duplicate bundle-id instances; Computer Use confirmed the observed window belonged to the main checkout app bundle.
  - Computer Use clicked Scan and read back `341 scanned, 341 in catalog, 866 findings, 170 conflicts`.
  - Verified detail tabs for Findings severity grouping, Conflicts instance comparison, and Snapshot Preview without running rollback.
  - Verified LLM actions remain disabled by default.
  - Verified Script Execution Safety stays preview-only; the missing-command preview error did not execute a script.
  - Verified agent filters: All 341 visible, Claude Code 154 visible, Codex 171 visible, and opencode 16 visible. opencode rows stayed read-only and Disable remained disabled.
  - Verified project context set to `skills-copilot` through Recent Projects, refreshed catalog, then Clear Project returned the UI to no-project.
  - Captured app-window-only evidence with the sidebar hidden to avoid visible local path disclosure: `docs/ui-artifacts/native-macos-shell/completed.png` and `docs/ui-artifacts/native-macos-shell/real-local-computer-use-2026-06-09.png`.

## macOS Computer Use

- Opened `dist/SkillsCopilot.app` with fixture environment:
  - `SKILLS_COPILOT_HOME=/tmp/skills-copilot-native-write-home`
  - `SKILLS_COPILOT_APP_DATA_DIR=/tmp/skills-copilot-native-write-data`
- Verified the first screen reads the temporary catalog path and starts with 0 skills.
- Clicked Scan and verified `catalog.scanClaude` renders one fixture skill, `demo-toggle`, with full `catalog.getSkill` detail.
- Clicked Disable and verified `config.toggleSkill` updates the sidebar count to `0 of 1 enabled`, changes detail state to Disabled, and creates one snapshot.
- Switched to Snapshots and verified the snapshot card renders from `snapshot.list`.
- Clicked Preview and verified `snapshot.previewRollback` shows current `skillOverrides` content beside the `{}` snapshot content.
- Confirmed Rollback and verified `snapshot.rollback` restores the skill to Loaded and refreshes the sidebar count to `1 of 1 enabled`.
- Reopened the app with `SKILLS_COPILOT_HOME=/tmp/skills-copilot-settings-home` and `SKILLS_COPILOT_APP_DATA_DIR=/tmp/skills-copilot-settings-data`.
- Opened the system Settings window and verified Claude Settings loads from `config.readClaudeSettings`.
- Edited the fixture JSON, saved through `config.saveClaudeSettings`, and verified the main window refreshed to `0 of 1 enabled`, Disabled state, and one `pre-config-edit` snapshot.
- Verified native search filters the sidebar and shows the no-match state.
- Verified the Skills menu exposes Overview, Findings, Conflicts, Snapshots, and Clear Search; Show Snapshots switches the detail segment.
- Latest attempt after protocol/smoke hardening: Computer Use listed `SkillsCopilot` as running but `get_app_state` returned `cgWindowNotFound` for app name, bundle id, and app path. A system window-list check confirmed the visible `SkillsCopilot` app window existed. The likely cause was a locked or otherwise non-interactive macOS UI session. Because Computer Use could not resolve the window, this pass used `pnpm smoke:macos-app -- --fixture-data --capture-window` and the window-only screenshot artifact as validation evidence. Do not treat this as a replacement for future Computer Use validation once the macOS session is unlocked and the tool can resolve the generated app bundle again.
- Latest attempt after sidebar filter/sort work:
  - Ran `pnpm dev:macos`; it rebuilt and launched the real local `dist/SkillsCopilot.app` with the developer's real HOME/app data/Claude config.
  - `pgrep -ax SkillsCopilot` confirmed the real local app process was running.
  - `script/capture_app_window.sh SkillsCopilot docs/ui-artifacts/native-macos-shell/completed.png` captured the complete app window and showed the new State and Sort controls.
  - Computer Use still returned `cgWindowNotFound` for app name, app path, and bundle id, so real Computer Use operation of scan/filter/sort/toggle remains blocked by window resolution. This is recorded as a validation gap, not as a completed manual operation pass.
- Latest attempt after Swift string resource work:
  - Ran `pnpm dev:macos`; it rebuilt and launched the real local `dist/SkillsCopilot.app`.
  - Computer Use resolved the app window successfully.
  - Clicked Scan and verified the real local catalog scanned 154 Claude skills.
  - Set Search to `analyticdb` and verified the sidebar visible count changed to 5.
  - Opened Sort and selected Scope; the Sort picker updated to Scope.
  - After the final localized `View` label change, reran `pnpm dev:macos`, Computer Use resolved the app window, set Search to `analyticdb`, and verified the sidebar visible count stayed at 5.
  - Captured `docs/ui-artifacts/native-macos-shell/completed.png` with the app-window-only capture script.
- Latest unlocked real Local App Run after CI/documentation cleanup:
  - Ran `pnpm dev:macos`; it rebuilt and launched the real local `dist/SkillsCopilot.app` with the developer's real HOME, app data, and Claude config.
  - Computer Use resolved the real app window and showed 154 Claude skills in the local catalog.
  - Clicked Scan and verified the success banner `Scanned 154 Claude skills.`
  - Clicked Disable for the selected real local skill and verified the sidebar count changed to `153 of 154 enabled`, the skill state changed to disabled, and the snapshot count changed to 1.
  - Opened the Snapshots section, clicked Preview, and verified the preview sheet compared current settings against the pre-toggle snapshot.
  - Confirmed Snapshot Rollback and verified the sidebar returned to `154 of 154 enabled`, the skill returned to Loaded, and the app rescanned 154 skills.
  - Opened the native Settings scene with Command-Comma, verified `config.readClaudeSettings` loaded the real settings target, made a semantic no-op JSON edit, saved, and saw `Saved Claude settings and refreshed catalog.`
  - Rolled back the resulting `pre-config-edit` snapshot to restore the real settings file after the save-path test.
  - Ran a JSON sanity check against `~/.claude/settings.json`: parsing passed and `skillOverrides` was absent.
- Latest real Local App Run after deprecated Web/Tauri removal:
  - Ran `pnpm dev:macos`; it rebuilt and launched the real local `dist/SkillsCopilot.app` from the native SwiftUI shell and packaged Rust sidecar after `ui/` and `src-tauri/` were deleted.
  - Computer Use resolved the real app window, showed 154 local Claude skills, clicked Scan, and verified the success banner `Scanned 154 Claude skills.`
  - Captured `docs/ui-artifacts/native-macos-shell/completed.png` with `pnpm capture:macos-window`, which delegates to the app-window-only Quartz capture script.
- Final session closeout real Local App Run:
  - Ran `pnpm dev:macos`; it rebuilt and launched the real local `dist/SkillsCopilot.app` with the developer's real HOME, app data, and Claude config.
  - Computer Use resolved the real app window and showed 154 local Claude skills in the catalog.
  - Computer Use action calls for click/key events returned an activation error even immediately after `get_app_state`; this is recorded as a tool action blocker.
  - Used macOS System Events to click the Scan toolbar button, then used Computer Use to read back the real app window and verify the success banner `Scanned 154 Claude skills.`
  - Captured `docs/ui-artifacts/native-macos-shell/completed.png` with `pnpm capture:macos-window`.
- Latest V2 Prep integration real Local App Run:
  - Ran `pnpm dev:macos`; it rebuilt and launched the real local `dist/SkillsCopilot.app` with the developer's real HOME, app data, and Claude config.
  - `pgrep` and System Events confirmed `SkillsCopilot` processes were running.
  - `script/capture_app_window.sh SkillsCopilot /tmp/skillscopilot-main-window.png` captured a complete 920x652 app window, confirming the real app window existed.
  - Computer Use could not resolve the real app window: app name returned `remoteConnection`, and full app path `<repo>/dist/SkillsCopilot.app` returned `cgWindowNotFound`.
  - After closing duplicate `SkillsCopilot` processes and reopening only the main worktree app, Computer Use still returned `remoteConnection` / `cgWindowNotFound`. Real Computer Use operation is therefore blocked by window resolution for this pass; the smoke screenshot and window capture are recorded as supporting evidence, not as a replacement for manual operation.
- Latest unlocked Computer Use retry:
  - After the macOS session was unlocked, Computer Use resolved `<repo>/dist/SkillsCopilot.app` successfully.
  - The real local app window showed 154 Claude skills in the local catalog.
  - Clicked the Scan toolbar button with Computer Use and verified the success banner `Scanned 154 Claude skills.`
- 2026-06-09 current mainline real local pass:
  - Computer Use resolved the real app window for observation and state read-back. `mcp__computer_use.click` returned an activation error, so UI operation used macOS AX/System Events clicks followed by Computer Use read-back.
  - Toolbar Scan completed against real local roots and showed `341 scanned, 341 in catalog, 869 findings, 170 conflicts`.
  - Findings severity filtering, conflicts, snapshot preview, Codex/opencode agent filters, project context set/clear, opencode read-only disabled toggle, disabled-by-default LLM controls, and V2.10 script safety preview were operated in the real app.
  - Script safety remained default-deny. The real catalog had no structured script command records, so the preview showed a safe missing-command preview-only state and did not execute anything.
  - Captured `docs/ui-artifacts/native-macos-shell/real-local-computer-use-2026-06-09.png` with the app-window-only capture script.
- 2026-06-09 final real local revalidation:
  - Computer Use resolved the main checkout app at `<repo>/dist/SkillsCopilot.app`.
  - Clicked Scan, Findings, Conflicts, Snapshots, Snapshot Preview, Preview Gate, agent filter menu entries, Recent Project, and Clear Project with Computer Use; each operation was followed by Computer Use state read-back.
  - Did not click real rollback or real writable Codex/Claude toggle in this pass, to avoid changing live user configuration.
  - Captured app-window-only evidence with sidebar hidden after validation.
- Latest V2 Prep refresh-experience Computer Use pass:
  - Computer Use resolved `<worktree>/dist/SkillsCopilot.app`.
  - Verified the sidebar shows the new refresh summary, refresh log disclosure, catalog path, and watcher state text.
  - Clicked the Scan toolbar button with Computer Use and verified the sidebar changed to `Scan complete: 154 scanned, 154 in catalog, 3 findings, 0 conflicts.` while the detail pane still showed `Scanned 154 Claude skills.`
  - Verified the watcher copy reports that the current stdio sidecar provides completed refresh summaries and native automatic watcher events are not running in this process.

## Screenshot

- Completed UI screenshot: `docs/ui-artifacts/native-macos-shell/completed.png`
- Screenshot capture rule: window-only capture. Full-desktop screenshots are forbidden.
- Latest shared smoke screenshot was regenerated on 2026-06-17 after V2.87 Agent Copilot validation. The capture script used the `SkillsCopilot` window id and `screencapture -l`, so the evidence contains the complete app window only.
- Legacy V2.88 per-surface screenshot artifacts were pruned during current
  app-surface cleanup; maintained evidence now centers on the current full app
  window.
- V2.68 screenshot was regenerated on 2026-06-13 by `pnpm check:macos` fixture smoke after cockpit-first IA changes. Manual inspection confirmed Task Cockpit is selected by default, Work surfaces appear before Adapter/Health diagnostics, and the detail picker no longer duplicates the "Detail Section" label.
- V2.68 real-local launch found the current `SkillsCopilot` window via CG metadata, but the macOS session was locked (`CGSSessionScreenIsLocked=Yes`), Computer Use timed out, and the direct real-local capture was black. The black screenshot was rejected and not committed.
- V2.69 screenshot evidence is generated with screenshot privacy mode available by default and checked by `pnpm verify:screenshot-artifacts`. The capture helper rejects locked-session, black-capture, mostly transparent, and near-flat screenshots before accepting evidence.
- V2.72 validation hardening adds canonical blocker classification, `pnpm classify:validation-blocker`, `pnpm verify:validation-blockers`, smoke lock-session preflight, and screenshot verifier canonical failure labels. Current locked-session capture attempts are blockers, not completed screenshot evidence.

## Notes

- `script/build_and_run.sh` now forwards `SKILLS_COPILOT_HOME`, `SKILLS_COPILOT_APP_DATA_DIR`, `SKILLS_COPILOT_CLAUDE_EXTRA_ROOTS`, and `SKILLS_COPILOT_SERVICE_PATH` through `launchctl` so LaunchServices-started `.app` processes can use fixture roots.
- Native Claude settings editing, toggle, and snapshot rollback write paths are exposed and verified against fixture data.
