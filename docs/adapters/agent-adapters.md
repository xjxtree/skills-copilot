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
- Missing implicit built-in and package-convention root candidates are ignored
  until created. Unavailable explicit configured, manifest-declared plugin, and
  extra roots plus existing invalid roots are skipped with typed issues, while
  authorized directory and entry failures mark a root partial and allow other
  roots to continue. Only fully enumerated roots are eligible for
  missing-record cleanup.
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

### Product Projection Boundary

- Adapters report documented local facts: roots, scope, config state,
  precedence inputs, source provenance, completeness, session identity, and
  supported native continuation behavior.
- Snapshot assembly normalizes those facts under one immutable source revision
  before product projection. Plugin enablement, compatibility-root ownership,
  config state, manager linkage, precedence, and project/session matching are
  adapter facts; the projector does not rediscover them from mutable files.
- Rust product projections combine those facts into environment health, skill
  effectiveness, aggregates, evidence references, and typed action capability.
  Native UI code and optional AI must not reimplement adapter policy.
- Installed, enabled, and effective remain separate. An installed source may be
  disabled, shadowed, unlinked, broken, or unavailable. `effective` means the
  adapter projection selects it; it does not mean a runtime invocation was
  observed.
- Every skill fact carries a logical source identity and runtime identity.
  Logical source identity names the adapter-declared native, compatibility, or
  plugin source; it must not be derived from a physical cache path or content
  hash. Definition fingerprint, source identity, publisher/package/version,
  scope, and runtime identity are all material aggregation dimensions.
- Precedence-sensitive adapters retain every current candidate. A loser becomes
  `shadowed` only when adapter evidence proves a current, complete, installed,
  linked, enabled winner in the same agent/runtime identity. An absent,
  historical, cross-runtime, or otherwise unproved winner leaves every member
  `unavailable`; it never produces a guessed effective result.
- Missing, partial, stale, or source-limited required adapter evidence blocks a
  healthy product status and must carry a typed reason and recovery action.
- Historical `missing` rows are excluded from current product skill inventory.
  Session continuation is limited to exact normalized project matches and
  retains only adapter-verified copy-only native resume capability; incomplete
  session evidence removes supported resume. Continuations retain both the
  native session-source revision and their accepted product snapshot revision.

## Adapter Matrix

| Adapter | Scan roots | Guarded writes | Install targets | Blocked |
| --- | --- | --- | --- | --- |
| Claude Code | User/project-ancestor `.claude/skills`, existing legacy `.claude/commands/*.md`, direct skill-directory links, and skill roots declared by effectively enabled installed plugins; matching `.agents/skills` directories only authorize same-scope link targets | Private Claude settings toggle path | Verified native target paths | Plugin writes, shared project settings writes unless separately scoped |
| Codex | Verified user/project `.agents/skills`, read-only `$CODEX_HOME/skills`, manifest-declared roots in installed versioned `$CODEX_HOME/plugins/cache` copies, `/etc/codex/skills`, and project `.codex/config.toml` diagnostics; no runtime inventory call | User config override for native `.agents/skills` instances | Native `.agents/skills` roots | Project `.codex/config.toml` and plugin/admin/system/compat writes |
| opencode | Native roots, official `.claude` / `.agents` compatibility roots, exact direct skill-link targets, and configured local `skills.paths` roots | Exact `permission.skill` overrides in verified config targets | Native opencode roots | `skills.urls` fetch, configured-root writes, compatibility-root installs |
| Pi | Native user roots, cwd `.pi/skills`, ancestor `.agents/skills`, local `skills` settings paths, and installed local package manifests/convention roots in runtime precedence order | Guarded official `skills` array exact `+path`/`-path` overrides | Native Pi roots only | Package install/remove, `.agents` direct installs, scripts, credentials |
| Hermes | Native `~/.hermes/skills` and explicit read-only `skills.external_dirs` | Global `skills.disabled` only | Native `~/.hermes/skills` | Project installs, `platform_disabled`, `external_dirs` writes, hub/URL/tap/update/uninstall/reset |
| OpenClaw | Workspace and personal `.agents` roots, native managed skills, optional bundled roots, enabled plugin manifest roots, and configured local extra dirs in runtime precedence order | `skills.entries.<key>.enabled` only | Native managed and confirmed workspace skill roots | `.agents` direct installs, allowlist/env/apiKey/install-policy writes, ClawHub/Git/update/verify/workshop |

## Verified Effective Source Inventory

The source inventory below is implemented from installed runtime contracts,
official documentation, or both. Environment overrides are accepted only when
they resolve to absolute local paths; network configuration is never fetched.

| Adapter | Effective skill/config sources | Effective local-session source | Explicit exclusions |
| --- | --- | --- | --- |
| Claude Code | `CLAUDE_CONFIG_DIR` (default `~/.claude`) user skills/settings and existing legacy `commands/*.md`; selected-project ancestor `.claude/skills` and `.claude/commands/*.md`; project-root settings; managed settings; skill roots from effectively enabled entries in `installed_plugins.json` | `<CLAUDE_CONFIG_DIR>/projects` | Disabled/uninstalled plugins, nested command files, generic cache walks, generated outputs, and undeclared roots |
| Codex | `$CODEX_HOME/skills`, user/project `.agents/skills`, user/project/admin config diagnostics, `/etc/codex/skills`, and complete manifest-declared skill roots from bounded installed plugin packages explicitly enabled in `$CODEX_HOME/config.toml` | `$CODEX_HOME/state_*.sqlite` thread index for summaries; selected `$CODEX_HOME/sessions` rollout for message detail | Runtime skill inventory as a product data source, generic plugin-store/cache walks, unconfigured or disabled plugin packages, staging/source-marketplace trees, archived/internal sessions, and arbitrary project roots |
| opencode | XDG/`OPENCODE_CONFIG_DIR` native roots; absolute `OPENCODE_CONFIG`; inline `OPENCODE_CONFIG_CONTENT`; selected-project ancestor native/official compatibility roots; local `skills.paths` | XDG data `opencode/opencode.db` | `skills.urls`, remote organization config, disabled external/Claude compatibility sources, caches, and legacy JSON sidecar stores when the SQLite DB exists |
| Pi | Installed project/global package skill roots, project/global configured `skills` paths, cwd `.pi/skills`, ancestor `.agents/skills` to the git root, `~/.pi/agent/skills`, and `~/.agents/skills`; first effective name wins | `PI_CODING_AGENT_SESSION_DIR`, configured `sessionDir`, or `<agent-dir>/sessions` | `--skill` process-only CLI arguments, `--no-skills` process state, temporary package trials, remote package indexes, caches, and generated outputs |
| Hermes | `HERMES_HOME` (default `~/.hermes`) native skills/config plus explicit local `skills.external_dirs` | `<HERMES_HOME>/state.db` | Hubs, taps, URLs, legacy JSON stores when the canonical DB exists, caches, and inferred project roots |
| OpenClaw | `OPENCLAW_STATE_DIR`/profile state, `OPENCLAW_CONFIG_PATH`, workspace/personal shared roots, managed skills, runtime-bundled roots, effectively enabled plugin manifest roots, configured extra dirs, agent/bundled allowlists, and eligibility metadata | `<state>/agents/<id>/agent/openclaw-agent.sqlite`; legacy JSON/JSONL is ignored | Arbitrary selected repositories, disabled plugins, ClawHub/Git/network sources, broad extension/cache walks, temporary workspaces, and guessed installation paths |

All filesystem walkers prune VCS, cache, temporary, build, distribution,
coverage, quarantine, archive, and language-cache directories before inspecting
skill files. A configured root that is itself a skill file is accepted only by
an adapter whose official format allows it (currently Pi Markdown skills).

### Codex In ChatGPT

- `codex` remains the adapter ID even though the current desktop host is
  `ChatGPT.app` (bundle identifier `com.openai.codex`). App branding does not
  change the config, skill, session, or service protocol identities.
- The adapter and local-session service share the same guarded Codex home.
  `CODEX_HOME` must be an absolute, lexically normalized path beneath the active
  user home; unsafe or relative overrides fall back to `$HOME/.codex`.
- The adapter never calls `codex app-server` or `skills/list` for skill
  discovery. Persisted files are the only catalog inventory source.
- The plugin store/cache directory is never a generic scan root. Installed
  plugin discovery considers only bounded marketplace/package/version records,
  keeps only packages whose config state is explicitly `enabled = true`, reads
  a bounded regular
  `.codex-plugin/plugin.json`, rejects absolute, parent-traversing, and escaping
  manifest paths, and selects one deterministic current version per
  marketplace/plugin pair. It scans every valid skill in the manifest-declared
  root. Unconfigured packages, staging, marketplace source, scripts, and
  unrelated cache files are not scanned or executed.
- Plugin skills are cataloged with the plugin namespace and a logical
  `$CODEX_HOME/plugins/...` display path; the physical cache path is not shown
  as the skill source. Rows retain `source_kind="chatgpt-plugin-cache"` only as
  a wire-compatibility value, and the UI labels them as installed Codex plugin
  files. Legacy synthetic runtime rows are removed by catalog migration.
- Startup and manual reload project the current guarded Codex
  `[[skills.config]]` path overrides and `[plugins.<id>] enabled` state over
  current native/plugin list, detail, and analysis records. This read-only
  projection makes external config changes visible without a catalog write.

### opencode Compatibility Roots

- opencode officially loads skills from its native roots plus `.claude/skills`
  and `.agents/skills` compatibility roots. A row whose physical file is under
  `~/.claude/skills` is therefore an effective opencode skill, not a Claude row
  leaking across the selected-agent filter.
- Compatibility provenance remains explicit in list/detail metadata and the UI
  labels it as an opencode compatibility source. Claude `skillOverrides` do not
  apply; opencode's own last-matching `permission.skill` rule controls access.
- A direct skill-directory symlink authorizes only its exact resolved target.
  The scanner never treats the target's parent or the rest of the user home as
  an allowed opencode root.

## Skill Manager Tooling

- `npx skills` is the first writable manager tool. It owns search, list,
  install, remove, update, and local template creation when the app calls
  `skillManager.*`.
- `skills-npm` is registered for capability discovery only in this slice; write
  execution needs a future scoped adapter.
- Manager-backed search/install/update use the scoped external network path by
  default. Writes still require a command preview and explicit confirmation.
- Commands must be executed as argv arrays after `env_clear()`, with only the
  previewed `HOME`, derived `PATH`, fixed locale, CI, telemetry-off, and npm
  audit/fund/update-notifier values. Parent credentials and action secrets are
  not inherited. Output logging replaces credential-shaped URLs before
  parsing; shell string concatenation is forbidden.
- Mutating manager previews expose a typed action, accepted target revision,
  and opaque HMAC token. Apply requires the exact action reference and token,
  then returns the declared catalog, skill-file, and manager-inventory
  read-back. Install/remove/update/local-create, local archive import/update,
  and app-owned local deletion share the same artifact-free cross-sidecar
  app-data owner lock used by batch toggles and direct `skill.install`. Apply
  rechecks the bounded complete target trees, archive bytes, inventory, and
  relevant catalog facts after taking the lock; stale state runs no process and
  writes no target or app data. The lock is held through semantic read-back and
  catalog commit. Exit zero is accepted only when each selected skill's
  preview-bound source identity and content fingerprint proves the
  operation-specific postcondition; an unrelated tree change is insufficient.
  Install and update require an explicit non-empty skill list. Multi-agent
  read-back retains one catalog and skill-file observation per selected target.
- Project manager targets are `.claude/skills` for Claude Code,
  `.agents/skills` for Codex and opencode, `.pi/skills` for Pi,
  `.hermes/skills` for Hermes, and `skills` for OpenClaw. Global targets use
  each adapter's verified native global root; Codex manager installs use the
  shared user `.agents/skills` root rather than `$CODEX_HOME/skills`. Inventory revisions also include
  project `skills-lock.json` or global `~/.agents/.skill-lock.json`.
- A relative local install source is resolved against the previewed manager
  working directory and canonicalized before it is classified as local.
  Credential-bearing standard or SCP-style URL forms with disallowed userinfo,
  query, fragment, percent encoding, or credential material are rejected
  without including the source in an error. The only accepted SCP username
  form is the literal `git@host:path`.
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
  those links. For an app-owned local source, full uninstall is one composite
  action and confirmation that binds both manager unlink targets and the
  complete local tree/catalog/reference cleanup. Apply returns combined
  read-back and the client must not issue a hidden second local-delete call.
  The panel does not expose agent-layer enable/disable controls.
- Manager update operates on the shared package source and does not accept
  per-agent targeting. The confirmation shows all currently linked supported
  agents affected by that source update.
- The app prewarms project/global inventories at startup. Opening Skill Manager
  is cache-only; its Load Data button is the page-local manual refresh.
- App-owned local source replacement accepts a confirmed ZIP only through
  `skillManager.previewLocalArchiveUpdate` / `applyLocalArchiveUpdate`. The
  archive must contain exactly one matching skill root; traversal, symlinks,
  special files, duplicates, files outside the root, and size/count overflow
  fail closed. Its signed action binds the exact archive bytes, complete current
  source tree, and catalog/reference identity. Apply revalidates under the
  shared owner lock, performs staged replacement with exact-state rollback, and
  returns verified catalog/skill-file read-back. Drift fails stale and replay
  cannot replace an already identical tree. Extraction never executes packaged
  scripts.
- Local ZIP import rejects a name already present in the app-owned library or
  an installed shared `.agents/skills` source. The existing package must use its
  update flow instead of creating an ambiguous second package row. Import uses
  the same signed action lifecycle, binds its missing destination tree and
  relevant catalog name state, and cannot be replayed after a successful
  registration.
- Agent enable/disable stays outside the Skill Manager surface; package manager
  state and agent config state are separate. Native single and multi-skill UI
  changes use `batch.previewSkillToggles` and
  `batch.applySkillToggles`. `config.toggleSkill` is a compatibility-only
  blocked method and returns `mutation_disabled`.
- ChatGPT's Plugin Directory is also separate from Skill Manager. Only enabled,
  installed, manifest-declared plugin skills participate in current projections;
  arbitrary plugin cache content and Deleted cache noise remain excluded;
  `skillManager.*` operates only through its explicit supported manager CLI,
  target selection, preview, and confirmation contract.

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
- Installed Codex plugin rows participate in current `SkillInstance` and
  `catalog.listSkills` projections with read-only provenance. Exact-path dedupe
  remains unchanged and does not merge distinct native or plugin copies.
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
