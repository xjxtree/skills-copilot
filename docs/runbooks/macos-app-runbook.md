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
pnpm build:macos:release:arm64
pnpm build:macos:release:x86_64
pnpm verify:macos-launch
pnpm verify:macos-distribution
pnpm test:macos-native-models
pnpm check:macos
```

`pnpm smoke:macos-app` validates the existing bundle; it does not rebuild it.

Release-candidate builds use optimized Rust and Swift products. The generated
app build script owns copying packaged resources into `Contents/Resources`, and
runtime icon loading uses `Bundle.main` so it never depends on the maintainer's
build tree or a SwiftPM resource accessor. Release builds also remap Rust source
and Cargo registry paths so maintainer-specific absolute paths are not embedded
in the service binary, and strip executable debug symbols before signing:

```sh
SWIFTPM_SCRATCH_PATH=/tmp/agent-copilot-release-arm64 \
  ./script/build_and_run.sh --build-only --configuration release --arch arm64
SWIFTPM_SCRATCH_PATH=/tmp/agent-copilot-release-x86_64 \
  ./script/build_and_run.sh --build-only --configuration release --arch x86_64
```

Without an explicit signing identity these are local-only, ad-hoc candidates
that can be checked with:

```sh
pnpm verify:macos-distribution -- --allow-ad-hoc
```

For distribution, pass a Developer ID Application identity through
`AGENT_COPILOT_SIGNING_IDENTITY` or `--signing-identity`, then use the
notarization command in `docs/runbooks/distribution-runbook.md`. The verifier
scans release executables for maintainer-specific source and home paths.

## Scenarios

| Scenario | Command | Data environment | Use |
| --- | --- | --- | --- |
| Local App Run | `./script/build_and_run.sh run` or `pnpm dev:macos` | Real local HOME and app data | Manual behavior and visual checks |
| Bundle Build | `pnpm build:macos` | No app data access | Rebuild without launching or stopping an existing app |
| Release Candidate | `pnpm build:macos:release:arm64` or `pnpm build:macos:release:x86_64` | No app data access; optional explicit signing identity | Build a stripped architecture-specific candidate |
| Distribution Verify | `pnpm verify:macos-distribution` | Existing bundle only | Verify versions, architectures, privacy, Developer ID signing, hardened runtime, and optional notarization trust |
| Launch Verify | `./script/build_and_run.sh --verify` or `pnpm verify:macos-launch` | Real local HOME and app data | Rebuild and confirm a visible window |
| Headless Sidecar Smoke | `pnpm smoke:macos-app -- --fixture-data --headless-sidecar` | Temporary fixture HOME, app data, and project roots | Validate the bundled Rust sidecar without GUI or Accessibility |
| Smoke App Run | `pnpm smoke:macos-app -- --fixture-data --capture-window` | Temporary fixture HOME, app data, and project roots | Automated validation without real config |
| Native Model Tests | `pnpm test:macos-native-models` | Temporary HOME, app data, and SwiftPM package | Swift Testing model suites without AppKit or SwiftUI linkage |
| Full Swift Tests | `pnpm test:macos-swift` | Temporary HOME and app data | Swift Testing against the complete macOS package |
| macOS Check | `pnpm check:macos` | Combined local gate | fmt/test/clippy/native model tests/build/launch/fixture smoke |

## Gate Parity

`pnpm verify:gate-parity` is the deterministic local/CI shared gate. It runs
these members in this exact order:

1. `verify:service-protocol-drift`
2. `verify:module-size`
3. `verify:quality-budgets`
4. `verify:list-completeness`
5. `verify:doc-governance`
6. `test:check-macos-cache`
7. `test:distribution-workflow`
8. `verify:js-syntax`
9. `verify:rust-docs`
10. `verify:validation-blockers`

`verify:quality-budgets` checks the parser, exact 10k count contract, and
checked-in elapsed/RSS/p95 ceilings. The live benchmark commands separately
measure and enforce those ceilings against the current machine.

`verify:list-completeness` checks the schema-v1 formal-list inventory, owning
source paths and Swift type/member anchors, unique reachable full-access
accessibility controls, paged `control_anchor` bindings, and new direct or
lexically alias-defined `prefix()` collections passed to `ForEach`, `List`,
`DenseDisclosureList`, or `ExpandableSummaryList`. Its canonical component
exceptions validate the rendered remainder and the identified Button's own
expansion action. It is a static declaration and control-boundary
check. Native model and UI tests remain responsible for proving loading,
continuation, expansion, routing, cancellation, and focus behavior.

The live `pnpm benchmark:10k` and `pnpm benchmark:macos-list-model` checks run
after tests and bundle build in the macOS CI job, and remain separate from the
deterministic parity gate. Their checked-in sampling minimums and metric
ceilings cannot be weakened by CI environment overrides.
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

## Screen Capture Rules

- Runtime screenshots must be app-window-only and remain outside the repository.
- Do not commit full desktop screenshots.
- Keep screenshot privacy mode enabled.
- Do not expose real HOME paths, app data, `/var/folders`, fixture temp roots,
  tokens, keys, or credential placeholders.
- Manually inspect task screenshots before handoff.

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
process launches but the window cannot be resolved, report one canonical
blocker in the task or pull request and do not substitute fixture smoke for
real-local validation.

Useful classifier:

```sh
pnpm classify:validation-blocker -- "<tool output>"
```

Canonical blockers include locked session, timeout, window not found, remote
connection, missing AX window, activation failure, Screen Recording permission,
black/flat/transparent/invalid capture, stale bundle, and unknown tool-layer
failure.
