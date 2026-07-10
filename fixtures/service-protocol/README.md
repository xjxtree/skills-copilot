# Service Protocol Fixtures

These fixtures are the shared contract examples for native macOS and future Windows/Linux UI shells.

`method-effects.json` is the exhaustive side-effect contract for every method in
`SUPPORTED_METHODS`. The request and response JSON files remain wire-shape
fixtures for typed protocol examples and error cases.

Rules:

- `*.request.json` must decode as `ServiceRequest`.
- `*.response.json` must decode as `ServiceResponse`.
- Fixture methods should stay additive and match `service.status.supported_methods`.
- `service.status.response.json` must include the current `protocol_version`.
- Use temporary paths in examples; do not encode a contributor's real home directory.

Current catalog scan fixture contract:

- `catalog.scanAll.response.json` covers all six supported adapter families:
  Claude Code, Codex, opencode, Pi, OpenClaw, and Hermes.
- The fixture intentionally combines a complete root, a partial root with a
  typed issue, and a skipped root. Its enclosing status, warning logs, and
  recovery actions must agree with those outcomes rather than merely decode as
  the right JSON shape.
- `catalog.scanClaude.response.json` exercises the same mixed-root semantics in
  its single Claude Code summary, including stable issue detail, recovery copy,
  warning logs, and both `$HOME` and `<adapter-root>` redaction.
- Diagnostic paths use `$HOME` and `<adapter-root>` exactly as the service does;
  contributor-specific absolute paths are forbidden in scan activity.
- V2.5 did not add a separate opencode scan method; `catalog.scanAll` remains
  the single multi-agent scan method. Fixture and smoke coverage keep opencode
  visible for global and selected-project scans while respecting its current
  guarded write capability.

V2.9 local export note:

- `skill.exportBundle.response.json` includes local response paths, but the bundle manifest itself must keep reproducible metadata path-relative.
- Export fixtures cover directory-form bundles only; signed or zipped distribution artifacts are intentionally out of scope.

V2.9 tool-global import note:

- `catalog.importSkill` imports a local directory containing `SKILL.md` into the app-controlled tool-global staging area and returns the imported `SkillRecord`, `instance_id`, `staging_path`, import findings, and audit summary.
- Imported tool-global content is read-only preview content; installing or writing to an agent config remains a separate confirmed adapter write flow.
- GitHub repo import is explicitly deferred in the service contract. The `catalog.importSkill.github.error.*` fixtures document the unsupported path; callers must provide a local `source_path` after any user-controlled clone or unpack step.

V2.11 adapter capability matrix note:

- `adapter.listCapabilities.response.json` is the direct capability matrix fixture for Claude Code, Codex, opencode, Pi, Hermes, and OpenClaw.
- `service.status.response.json` and `app.stateSnapshot.response.json` include `adapter_capabilities` so native UI shells can render adapter status without guessing from agent names.
- The matrix is descriptive for unsupported write paths: opencode is writable for native roots after V2.12 validation, Pi is guarded after V2.94 validation, Hermes native install landed in V2.95 and guarded config toggles landed in V2.97, and OpenClaw native/workspace install landed in V2.96 with guarded config toggles in V2.97 while unsupported config/network paths stay blocked.

V2.13 / V2.94 Pi note:

- V2.13 kept Pi read-only. V2.37 added guarded native toggles, and V2.94
  adds `.agents/skills` compatibility toggles plus native-root direct install.
- `adapter.listCapabilities`, `service.status`, and `app.stateSnapshot`
  fixtures expose Pi as guarded with native-root install support.
- Pi package install/remove, `.agents` direct skill-file installs, scripts,
  provider writes, credentials, cloud sync, and telemetry remain blocked.

V2.95 / V2.97 Hermes native install and config-toggle note:

- Hermes is guarded in `adapter.listCapabilities`, `service.status`, and `app.stateSnapshot`.
- Hermes native skill-file install may copy a local ToolGlobal `SKILL.md` into `~/.hermes/skills` after confirmation.
- V2.97 allows guarded skill toggles by patching only the documented global `skills.disabled` list in `~/.hermes/config.yaml` with snapshot/read-back/rollback.
- Hermes generic project scan remains blocked; `skills.external_dirs` are explicit read-only external roots and not project or install targets.
- Hermes `platform_disabled`, `skills.external_dirs` writes, hub/URL/tap/update/uninstall/reset operations, scripts, credentials, cloud sync, telemetry, and uncontrolled network fetch remain blocked.
- The Hermes fixture directory contains active-home scanner contract fixtures plus evidence-only cron samples.
- `catalog.scanAll` includes Hermes after the read-only scanner implementation lands.

V2.96 / V2.97 OpenClaw native/workspace install and config-toggle note:

- OpenClaw is guarded in `adapter.listCapabilities`, `service.status`, and `app.stateSnapshot`.
- OpenClaw project scan is workspace-scoped only for confirmed OpenClaw workspace roots; arbitrary repo roots must not be inferred as OpenClaw projects.
- V2.96 allows confirmed local ToolGlobal `SKILL.md` copies into `~/.openclaw/skills` and confirmed OpenClaw workspace `<workspace>/skills` only.
- V2.97 allows guarded skill toggles by patching only documented `skills.entries.<key>.enabled` in `~/.openclaw/openclaw.json`; JSON5 input is parsed and rewritten as strict JSON.
- OpenClaw `.agents` roots remain scan-only and are not direct install targets.
- OpenClaw agent allowlists, env/apiKey, install policy, load roots, ClawHub, Git, update, verify, workshop, and network-backed operations remain blocked.
- The OpenClaw fixture directory contains read-only scanner contract fixtures plus evidence samples.
- `catalog.scanAll` includes OpenClaw after V2.16.

V2.18 cross-agent analysis note:

- `catalog.analysis` is a read-only service method and is also embedded in `app.stateSnapshot.analysis`.
- Analysis groups duplicate names, canonical-name overlap, source-path overlap, enabled-state mismatch, malformed/broken rows, and same-agent precedence/shadowing where adapter evidence supports it.
- The analysis fixture must not imply writes, config changes, CLI calls, execution, or unsupported-root inference.

V2.12 opencode writable note:

- V2.12 updates opencode capability fixtures to writable after code implementation plus disposable local evidence verification completed.
- Regression coverage includes exact `permission.skill` patch, re-enable, snapshot/rollback, install, guarded UI/service gating, fixture HOME writes, and real HOME isolation.
