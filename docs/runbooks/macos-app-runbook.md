# macOS App Runbook

This runbook describes local development, smoke validation, and real-local UI
validation for the native macOS app.

## App Bundle

`script/build_and_run.sh` builds the Rust service, builds the Swift app, and
regenerates `dist/AgentCopilot.app`.

Common entrypoints:

```sh
./script/build_and_run.sh run
./script/build_and_run.sh --verify
pnpm dev:macos
pnpm build:macos
pnpm verify:macos-launch
pnpm test:macos-native-models
pnpm check:macos
```

`pnpm smoke:macos-app` validates the existing bundle; it does not rebuild it.

## Scenarios

| Scenario | Command | Data environment | Use |
| --- | --- | --- | --- |
| Local App Run | `./script/build_and_run.sh run` or `pnpm dev:macos` | Real local HOME and app data | Manual behavior and visual checks |
| Bundle Build | `pnpm build:macos` | No app data access | Rebuild without launching or stopping an existing app |
| Launch Verify | `./script/build_and_run.sh --verify` or `pnpm verify:macos-launch` | Real local HOME and app data | Rebuild and confirm a visible window |
| Headless Sidecar Smoke | `pnpm smoke:macos-app -- --fixture-data --headless-sidecar` | Temporary fixture HOME, app data, and project roots | Validate the bundled Rust sidecar without GUI or Accessibility |
| Smoke App Run | `pnpm smoke:macos-app -- --fixture-data --capture-window` | Temporary fixture HOME, app data, and project roots | Automated validation without real config |
| Native Model Tests | `pnpm test:macos-native-models` | Temporary SwiftPM test package | Explicit native model runner without SwiftPM test-bundle loading |
| macOS Check | `pnpm check:macos` | Combined local gate | fmt/test/clippy/native model tests/build/launch/smoke/window screenshot |

## Gate Parity

`pnpm verify:gate-parity` is the deterministic local/CI shared gate. It runs
these members in this exact order:

1. `verify:service-protocol-drift`
2. `verify:module-size`
3. `verify:list-completeness`
4. `verify:doc-governance`
5. `verify:macos-native-test-registry`
6. `verify:js-syntax`
7. `verify:rust-docs`
8. `verify:validation-blockers`
9. `verify:screenshot-artifacts`

`verify:list-completeness` checks the schema-v1 formal-list inventory, owning
source paths, reachable full-access accessibility controls, and new raw
`prefix()`-defined `ForEach`/`List` presentations. It is a static declaration
and control-boundary check. Native model and UI tests remain responsible for
proving loading, continuation, expansion, routing, cancellation, and focus
behavior.

The live `pnpm benchmark:10k` and `pnpm benchmark:macos-list-model` checks are
separate CI/release commands, not members of the deterministic parity gate.
Fixture-only headless bundled-sidecar smoke is also separate from real-local
GUI validation and cannot substitute for operating the visible app window.

The parity gate does not replace real-local UI operation when a user-visible
change needs visual or interaction evidence.

## Smoke Rules

Smoke validation must use fixture data:

```sh
pnpm smoke:macos-app -- --fixture-data --headless-sidecar
pnpm smoke:macos-app -- --fixture-data --capture-window
```

The headless command is suitable for CI and exercises the bundled sidecar
directly. It does not launch the app and is not evidence of a real-local window
or interaction. The capture command remains the local fixture GUI check.

Smoke must:

- use temporary HOME, app data, and project roots;
- avoid real Claude, Codex, opencode, Pi, Hermes, and OpenClaw config mutation;
- validate the existing app bundle;
- capture only the app window;
- cover scan, toggle, settings save, snapshot preview, rollback, project
  context, and configured fixture roots when supported by the smoke script;
- avoid script execution through scan, import, export, install, LLM prepare,
  state snapshot, or detail loading.

## Screenshot Rules

- Completed screenshots must be app-window-only.
- Do not commit full desktop screenshots.
- Keep screenshot privacy mode enabled for committed evidence.
- Do not expose real HOME paths, app data, `/var/folders`, fixture temp roots,
  tokens, keys, or credential placeholders.
- Run `pnpm verify:screenshot-artifacts` after screenshot changes.
- Manually inspect new screenshots; the verifier is not OCR.

## Real-Local Validation

Use the developer's real environment only when the runbook or task explicitly
requires real-local validation:

```sh
pnpm dev:macos
```

or:

```sh
./script/build_and_run.sh run
```

Operate the visible app window with Computer Use/AX when available. If the app
process launches but the window cannot be resolved, record one canonical
blocker and do not substitute fixture smoke for real-local validation.

Useful classifier:

```sh
pnpm classify:validation-blocker -- "<tool output>"
```

Canonical blockers include locked session, timeout, window not found, remote
connection, missing AX window, activation failure, Screen Recording permission,
black/flat/transparent/invalid capture, stale bundle, and unknown tool-layer
failure.
