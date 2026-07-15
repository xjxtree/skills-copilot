# UI Artifacts

This directory stores durable UI evidence: completed app-window screenshots
used by screenshot validation. It is not a release-history record.

An artifact proves the reviewed surface at the commit that introduced it; it
must not be cited as evidence for later UI changes without a fresh capture.

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
  blocker in task or release handoff notes rather than adding version history
  here.

Screenshot artifacts:

| Artifact | Contents |
| --- | --- |
| `native-macos-shell/` | Shared native macOS app-window screenshots |
| `task-cockpit-ia/` | Task Cockpit information architecture screenshot |
| `task-cockpit-timeout-recovery/` | Task Cockpit timeout recovery screenshot |
| `launch-window-targeting/` | Launch/window targeting screenshot |
| `task-input-resilience/` | Task input resilience screenshot |
| `progressive-cockpit-feedback/` | Progressive cockpit feedback screenshot |
| `validation-workbench/` | Validation workbench screenshot |
| `privacy-localization/` | Privacy/localization screenshot |
| `detail-density/` | Detail density screenshot |
| `real-local-2026-06-16/` | Real-local app-window capture |
| `brand-assets/` | Brand asset screenshot |
| `identifier-migration/` | Identifier migration screenshot |
| `model-task-history/` | Model-task history screenshot |
| `codex-expanded-roots/` | Codex expanded roots screenshot |
| `pi-install-compat-writes/` | Pi install/compat write screenshot |
| `hermes-native-install/` | Hermes native install screenshot |
| `openclaw-native-workspace-install/` | OpenClaw native/workspace install screenshot |
