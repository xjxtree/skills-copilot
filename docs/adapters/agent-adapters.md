# Agent Adapters

This document defines supported local-agent scan roots, write scopes, and
blocked operations. It is a current contract, not a version history.

## Global Rules

- Adapters are stateless. They discover roots, parse skill records, and report
  enabled state; they do not cache file contents.
- Local-session inventories are also request-scoped and stateless. Keyset first
  pages require the explicit wire opt-in `paging_mode="keyset"`; unmarked
  requests retain legacy offset/max-file behavior. Continuation re-inventories
  only authorized roots within the established directory, entry, file, sidecar,
  and byte budgets; any exhaustion is a terminal typed `safety_budget`
  limitation that retains accepted unique rows. Opaque cursors contain digests
  rather than raw paths. Adapters must not persist session cursors, inventories,
  summaries, details, or raw session content in SQLite or app data.
- Scan roots and symlink-target roots are explicit. A symlink target is readable
  only when its canonical path is under a declared root with the same scope;
  the rest of the user home or project tree does not authorize traversal.
- Scanner link resolution must retain that same-scope root provenance; a link
  cannot promote a project root into a global root or vice versa.
- Missing implicit built-in root candidates are ignored until created.
  Unavailable explicit configured/plugin/extra roots and existing invalid roots
  are skipped with typed issues, while authorized directory and entry failures
  mark a root partial and allow other roots to continue. Only fully enumerated
  roots are eligible for missing-record cleanup.
- Partial roots never participate in missing-sweep, including when another
  complete root resolves to the same physical location under a different scope.
- Writes must go through the service layer with preview, snapshot/read-back
  where applicable, atomic write, rollback, and rescan.
- Network fetch, package manager calls, script execution, credentials, cloud
  sync, and telemetry are blocked unless explicitly scoped. The `skillManager.*`
  service domain is the scoped exception for supported external manager CLIs.
- Configured or compatibility roots are scan-only unless the adapter table
  below explicitly names a guarded write path.
- Skill Manager exposes only the app-supported target set: Claude Code, Pi,
  opencode, Codex, Hermes, and OpenClaw. Agent selection appears after a skill
  and install/remove action are chosen; manager reads must not fan out into
  per-agent list commands or use wildcard targets.
- `npx skills list` is inventory rather than ownership evidence. Only a matching
  scope lock entry marks a row manager-backed; unlocked rows whose sources are
  guarded descendants of `.agents/skills` remain local. Confirmed ZIP
  replacement is limited to the selected catalog instance inside the active
  project/global shared root.

## Adapter Matrix

| Adapter | Scan roots | Guarded writes | Install targets | Blocked |
| --- | --- | --- | --- | --- |
| Claude Code | User/project `.claude/skills`; matching user/project `.agents/skills` directories are explicit same-scope symlink-target roots and are not walked directly | Private Claude settings toggle path | Verified native target paths | Shared project settings writes unless separately scoped |
| Codex | User/project `.agents/skills`; read-only `$CODEX_HOME/skills`, bounded ChatGPT plugin-cache and local plugin marketplace roots, `/etc/codex/skills`, and project `.codex/config.toml` diagnostics | User config override for native `.agents/skills` instances | Native `.agents/skills` roots | Project `.codex/config.toml`, plugin/admin/system/compat writes |
| opencode | Native roots, official `.claude` / `.agents` compatibility roots, and configured local `skills.paths` roots | Exact `permission.skill` overrides in verified config targets | Native opencode roots | `skills.urls` fetch, configured-root writes, compatibility-root installs |
| Pi | Native `~/.pi/agent/skills`, project `.pi/skills`, and `.agents/skills` compatibility roots | Guarded settings toggle for native and `.agents` compatibility instances | Native Pi roots only | Package install/remove, `.agents` direct installs, scripts, credentials |
| Hermes | Native `~/.hermes/skills` and explicit read-only `skills.external_dirs` | Global `skills.disabled` only | Native `~/.hermes/skills` | Project installs, `platform_disabled`, `external_dirs` writes, hub/URL/tap/update/uninstall/reset |
| OpenClaw | Native `~/.openclaw/skills`, shared `~/.agents/skills`, optional bundled system roots, confirmed workspace `<workspace>/skills`, and `<workspace>/.agents/skills` | `skills.entries.<key>.enabled` only | Native `~/.openclaw/skills` and confirmed workspace `<workspace>/skills` | `.agents` direct installs, allowlists, env/apiKey, install policy, load roots, ClawHub/Git/update/verify/workshop |

### Codex In ChatGPT

- `codex` remains the adapter ID even though the current desktop host is
  `ChatGPT.app` (bundle identifier `com.openai.codex`). App branding does not
  change the config, skill, session, or service protocol identities.
- The adapter and local-session service share the same guarded Codex home.
  `CODEX_HOME` must be an absolute, lexically normalized path beneath the active
  user home; unsafe or relative overrides fall back to `$HOME/.codex`.
- The adapter inventories
  `$CODEX_HOME/plugins/cache/<publisher>/<package>/<version>/skills` as
  read-only plugin roots. Enumeration is bounded, hidden/staging directories
  are skipped, manifests are size-bounded, paths must remain inside the
  canonical package root, and only the highest valid version per package is
  selected.
- Catalog list and detail records derive publisher, package, version,
  `source_kind=chatgpt-plugin-cache`, and a read-only reason from the validated
  persisted path. This adds no plugin state or raw manifest content to SQLite.
- ChatGPT plugin discovery is inventory only. It never runs plugin scripts or
  invokes plugin install/update/remove. Those cache roots remain blocked for
  adapter writes.
- Inventory rows do not collide merely because separate plugin packages reuse
  a raw skill name. Runtime conflicts include only loaded/enabled native rows
  and cache packages explicitly enabled by plugin id in the guarded Codex user
  config. Each enabled package is a separate runtime namespace; a native row
  with the same name still conflicts with an enabled plugin row.
- Startup and manual reload project the current guarded Codex
  `[[skills.config]]` enable/disable state over cached native list, detail, and
  analysis records. This read-only projection makes changes performed by
  ChatGPT visible without a rescan or catalog write; inventory changes still
  require an explicit scan.

## Skill Manager Tooling

- `npx skills` is the first writable manager tool. It owns search, list,
  install, remove, update, and local template creation when the app calls
  `skillManager.*`.
- `skills-npm` is registered for capability discovery only in this slice; write
  execution needs a future scoped adapter.
- Manager-backed search/install/update use the scoped external network path by
  default. Writes still require a command preview and explicit confirmation.
- Commands must be executed as argv arrays with telemetry-off env
  (`DISABLE_TELEMETRY=1`, `DO_NOT_TRACK=1`) and redacted output logging. Shell
  string concatenation is forbidden.
- Search preserves every row returned by the manager, but the current CLI does
  not prove a remote total or advertise a continuation token. Responses must
  therefore distinguish returned rows from an unknown, source-limited total;
  clients may reveal the in-memory returned collection progressively but must
  not label its count as complete or make a hidden follow-up network request.
  Installed JSON is read once per project/global scope and merged with the
  app-owned local library. The CLI row is authoritative for an installed
  package; its redacted source path associates matching catalog instances and
  consumes them, so the same `.agents/skills` source is never rendered once as
  manager-owned and again as local. Catalog fallback is limited to guarded
  skill sources beneath the selected `.agents/skills` root, including nested
  package layouts; plugin caches and other read-only discovery roots are not
  package-manager inventory. Machine JSON is parsed in
  full behind a 4 MiB limit;
  the smaller diagnostic capture is never fed back into the parser. Truncated,
  malformed, or unrecognized list output fails closed and must never become an
  exact empty list or expose raw manager output in an error.
  Installed JSON stdout uses a private `0600` temporary regular-file capture
  so the Node CLI cannot drop bytes at the 64 KiB pipe-buffer boundary; the
  capture is bounded and removed on every return path.
  An installed local source outside the selected guarded `.agents/skills` root
  remains visible as external local and may be unlinked, but cannot be replaced
  from ZIP by the app.
  Do not add page flags or alter the manager command shape without a parsed
  token fixture that proves the external manager contract.
- Install uses the manager symlink flow; the native UI does not offer copy
  distribution.
- Skill removal uses manager-backed agent link removal for the targets selected
  after the skill is selected. Removing every linked target allows the manager
  to remove an unreferenced canonical source; a partial removal removes only
  those links. The panel does not expose agent-layer enable/disable controls.
- Manager update operates on the shared package source and does not accept
  per-agent targeting. The confirmation shows all currently linked supported
  agents affected by that source update.
- The app prewarms project/global inventories at startup. Opening Skill Manager
  is cache-only; its Load Data button is the page-local manual refresh.
- App-owned local source replacement accepts a confirmed ZIP only through
  `skillManager.previewLocalArchiveUpdate` / `applyLocalArchiveUpdate`. The
  archive must contain exactly one matching skill root; traversal, symlinks,
  special files, duplicates, files outside the root, and size/count overflow
  fail closed. Extraction never executes packaged scripts.
- Local ZIP import rejects a name already present in the app-owned library or
  an installed shared `.agents/skills` source. The existing package must use its
  update flow instead of creating an ambiguous second package row.
- Agent enable/disable remains in `config.toggleSkill`,
  `batch.previewSkillToggles`, and `batch.applySkillToggles` outside the Skill
  Manager surface; package manager state and agent config state are separate.
- ChatGPT's Plugin Directory is also separate from Skill Manager. Existing
  ChatGPT cache content may appear in the catalog as read-only provenance, but
  `skillManager.*` continues to operate only through its explicit supported
  manager CLI, target selection, preview, and confirmation contract.

## Discovery Requirements

New or expanded adapter support needs verified evidence for:

- skill discovery roots;
- skill file/directory format;
- project inheritance behavior;
- config file path and schema;
- enable/disable semantics;
- fixture data;
- malformed input behavior;
- read-only fallback behavior when write semantics are absent.

Do not infer support from neighboring tools, guessed paths, or generic project
root conventions.

## Identity And Dedupe

- Same physical file exposed by multiple agents may appear as multiple
  `SkillInstance` rows.
- Same-agent runtime/name collisions require at least two distinct physical
  paths that are loaded and enabled in the same effective runtime namespace.
- A complete exact adapter scan retires unseen rows for that agent and current
  project context even when their former source root was removed. Partial or
  skipped scans retain the root-bounded preservation behavior.
- Cross-agent duplicate, overlap, or enabled-state mismatch belongs in analysis,
  not conflict counts.
- Path/provenance labels should explain why two rows exist without changing
  conflict semantics.

## Safety Notes

- `skills.urls`, hubs, taps, package managers, Git-backed installs, cloud
  scanners, security scans, and update commands are metadata-only or blocked
  outside the scoped `skillManager.*` manager path.
- Import/install must copy only confirmed local `SKILL.md` records into verified
  app-controlled or native roots.
- Adapter config snapshots must redact secrets before persistence or display.
- Any future write expansion must include disposable evidence, fixture tests,
  rollback tests, and privacy verification.
