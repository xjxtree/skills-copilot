# Service Protocol Fixtures

These JSON files are executable wire-contract examples for the Rust service and
native clients. `crates/service/src/protocol.rs` remains the method source of
truth, and `docs/service-protocol.md` describes the current semantics.

## Rules

- Every `*.request.json` must decode as `ServiceRequest`.
- Every `*.response.json` must decode as `ServiceResponse`.
- `method-effects.json` must cover every supported method and its side effects.
- Supported methods must stay synchronized across status/state fixtures,
  dispatch, documentation, request/response fixtures, and effect declarations.
- Examples use `$HOME`, `<adapter-root>`, `<project-root>`, or temporary paths;
  contributor-specific absolute paths are forbidden.

## Covered Contracts

- `catalog.scanAll.response.json` covers all supported adapter families and the
  complete, partial, skipped, and empty-root diagnostic shapes.
- `catalog.scanClaude.response.json` covers the single-agent form of the same
  typed scan diagnostics and redacted paths.
- `adapter.listCapabilities.response.json`, `service.status.response.json`, and
  `app.stateSnapshot.response.json` expose the same adapter capability matrix.
- Catalog import/export fixtures cover local directory and app-owned staging
  flows. Network repository import, script execution, and hidden agent writes
  remain outside the contract.
- Skill Manager fixtures cover previewed manager operations and the guarded
  local archive path described in the service protocol.
- Session fixtures cover bounded local inventory, summary/detail separation,
  paging, redaction, and supported agent stores.
- Analysis and finding fixtures are read-only and must not imply config writes,
  CLI calls, execution, or unsupported-root inference.

Run `pnpm verify:service-protocol-drift` after any method, payload, fixture, or
effect-contract change.
