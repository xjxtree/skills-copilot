# UI Artifacts

This directory stores durable UI evidence: completed app-window screenshots and
verification notes.

Rules:

- Completed screenshots must be complete app-window-only captures.
- Use `script/capture_app_window.sh` for macOS artifacts when a real window is
  available.
- Do not commit full-desktop screenshots.
- Run `pnpm verify:screenshot-artifacts` after adding, regenerating, deleting,
  or reorganizing PNG evidence.
- App screenshots should be taken with screenshot privacy mode enabled unless a
  maintainer explicitly needs full paths for local debugging.
- If a session is locked or app-window capture is blocked, record the canonical
  blocker in the relevant verification record.

Current artifacts:

| Artifact | Contents |
| --- | --- |
| `native-macos-shell/` | Initial native macOS completed screenshot and verification record |
| `service-protocol/` | Historical service-boundary artifact directory |
| `v2.68-task-cockpit-ia/` | V2.68 task cockpit IA screenshot and verification |
| `v2.69-privacy-screenshot-mode/` | V2.69 privacy screenshot verification record |
| `v2.73-task-cockpit-timeout-recovery/` | V2.73 timeout recovery app-window evidence |
| `v2.74-launch-window-targeting/` | V2.74 launch/window targeting app-window evidence |
| `v2.75-task-input-resilience/` | V2.75 task input resilience app-window evidence |
| `v2.76-progressive-cockpit-feedback/` | V2.76 progressive feedback app-window evidence |
| `v2.77-validation-workbench/` | V2.77 validation workbench app-window evidence |
| `v2.79-privacy-localization/` | V2.79 privacy/localization app-window evidence |
| `v2.80-detail-density/` | V2.80 detail density app-window evidence |
| `v2.86-real-local-2026-06-16/` | V2.86 real-local window capture evidence |
| `v2.89-brand-assets/` | V2.89 brand asset evidence |
| `v2.90-identifier-migration/` | V2.90 identifier migration evidence |
| `v2.91-model-task-history/` | V2.91 model-task history evidence |
| `v2.92-codex-expanded-roots/` | V2.92 Codex expanded roots evidence |
| `v2.93-opencode-custom-roots/` | V2.93 opencode custom roots verification notes |
| `v2.94-pi-install-compat-writes/` | V2.94 Pi install/compat write evidence |
| `v2.95-hermes-native-install/` | V2.95 Hermes native install evidence |
| `v2.96-openclaw-native-workspace-install/` | V2.96 OpenClaw native/workspace install evidence |
