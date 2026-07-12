# ChatGPT Desktop / Codex Compatibility Design

Date: 2026-07-12

## Purpose

Update Agent Copilot for the ChatGPT desktop application that now hosts Codex,
while preserving the existing `codex` agent identity, local-first safety model,
and compatibility with existing Codex configuration and session data.

The change covers four concrete gaps observed against the current local app:

1. ChatGPT-hosted plugin skills are not represented by the current Codex
   adapter's legacy marketplace discovery.
2. Large Codex session stores can exhaust the summary-read budget before the
   project-filtered list is completely classified.
3. Codex session discovery does not share the adapter's safe `CODEX_HOME`
   resolution.
4. The native shell still looks only for `Codex.app`, so the agent icon falls
   back to the CLI icon after the desktop application rename.

## Product Semantics

- `AgentId::Codex`, the service wire value `codex`, catalog identities, filter
  values, and persisted rows remain unchanged. Codex is still the coding mode
  and agent family inside ChatGPT.
- General agent labels remain `Codex`. Text that refers specifically to the
  desktop client uses `ChatGPT Codex` so it does not imply that a standalone
  `Codex.app` must exist.
- ChatGPT Chat and ChatGPT Work conversations are not Codex sessions and are
  outside this adapter. The adapter continues to read only documented Codex
  local stores.
- The existing Skill Package Manager remains an `npx skills` workflow. It is
  not renamed or presented as the ChatGPT Plugin Directory.

## Architecture

### Shared Codex path resolution

Add one Rust-owned resolver that derives the effective Codex home from the
adapter context:

- default: `$HOME/.codex`;
- override: absolute `CODEX_HOME` that remains beneath the verified user home;
- invalid, relative, or escaping overrides: fall back to `$HOME/.codex`.

Adapters, command/config targets, local-session roots, and Codex index loading
use this resolver. Session index lookup receives the resolved Codex home
directly instead of trying to rediscover it from a path segment named
`.codex`. This keeps custom safe homes working without widening any write
boundary.

### Read-only ChatGPT plugin discovery

Extend Codex skill discovery with a bounded plugin-package resolver. It keeps
the existing local marketplace support and adds installed package discovery
under the effective Codex home.

A package version is eligible only when all of the following hold:

- it is beneath `$CODEX_HOME/plugins/cache`;
- it contains a regular `.codex-plugin/plugin.json` file;
- the manifest is valid JSON and declares a relative `skills` directory, or
  uses the documented `./skills/` default;
- the canonical skills directory remains beneath the canonical package root;
- the package is not beneath a staging, temporary, hidden-work, or app-server
  directory.

Discovery enumerates only the bounded publisher/package/version hierarchy. It
does not recursively treat the entire Codex home as authorized, follow an
arbitrary cache symlink, read skill scripts, or access the network. When more
than one valid version of the same publisher/package is present, the resolver
selects one deterministic highest version using numeric-aware component
comparison and emits a diagnostic for superseded versions. It does not claim
that cached package presence grants workspace permission or that an included
app connection is active.

Plugin skill instances remain scan-only. They cannot be toggled, removed,
updated, or installed through the Codex adapter. Existing `skillManager.*`
operations continue to target only their current verified `npx skills` paths.

### Plugin provenance model

Add optional plugin provenance to the Rust skill/catalog/service model:

- plugin/package name;
- package version;
- publisher/cache family;
- source kind `chatgpt-plugin-cache` or `legacy-local-plugin`;
- read-only reason.

The fields are optional for wire compatibility and empty for native skills.
The Swift detail surface presents them in the metadata/source area. This gives
users a distinct, inspectable Plugin → Skill relationship without pretending
that Agent Copilot manages plugin Apps, App Templates, OAuth connections, or
workspace installation policy.

No network-backed Plugin Directory search, installation, update, OAuth,
workspace mutation, or app-action execution is added. Those behaviors remain
deferred pending a separate safety and service-protocol review.

### Two-stage Codex session loading

Split session list work into metadata classification and page materialization:

1. Inventory regular session files and retain stable path, size, and modified
   time under the existing directory and entry budgets.
2. Read a small bounded head sufficient to recover Codex `session_meta`
   identity and `cwd` for project filtering. Do not read the normal 384 KiB /
   128 KiB summary window for every candidate.
3. Apply agent/scope/project filtering and filesystem-metadata sorting.
4. Materialize summary content only for rows needed by the current keyset page.
5. Read the bounded detail window only for the selected stable session ID.

Search that requires message content retains explicit bounded/incomplete
semantics when all candidate content cannot be searched. Normal unsearched
project and global lists must not become incomplete merely because many
sessions have large bodies. Raw session summaries, metadata probes, cursors,
and details remain in memory and are not persisted.

### macOS application identity and icon

Resolve the desktop client by bundle identifier `com.openai.codex` through
`NSWorkspace` before trying fixed paths. This supports the current
`ChatGPT.app`, the former `Codex.app`, and nonstandard application locations.

For Codex, the icon order is:

1. Codex-specific icon resources inside the resolved ChatGPT/Codex bundle;
2. the resolved application bundle icon;
3. fixed `ChatGPT.app` and legacy `Codex.app` compatibility paths;
4. the Codex CLI file icon.

The resolver is separated from image loading so native tests can verify
candidate ordering without depending on which applications are installed on
the test machine.

## UI And Documentation

- Keep sidebar, filters, task-preflight agent values, and skill headings named
  `Codex`.
- Change desktop-client-specific restart/help copy to `ChatGPT Codex or the
  Codex CLI may need to restart ...`.
- Display plugin provenance and a read-only badge/reason for plugin-cache
  skills.
- Explain in README and adapter documentation that Codex is hosted by the
  ChatGPT desktop app, local `.codex` data remains the adapter boundary, and
  ChatGPT plugins are distinct from the Skill Package Manager.
- Update service protocol fixtures and docs for optional plugin provenance and
  shared Codex-home session discovery.

## Error Handling And Safety

- Malformed plugin manifests or escaping skills paths produce typed scan
  diagnostics and do not abort other roots.
- Missing or stale cache versions are skipped; they never cause native skill
  rows to be swept as missing unless their own declared root was completely
  enumerated.
- Plugin roots are read-only regardless of manifest content. Scripts, hooks,
  apps, templates, credentials, and executable declarations are metadata only
  and are not loaded or run.
- All displayed paths continue through existing redaction. New diagnostics use
  `$HOME`, `<project-root>`, or `<adapter-root>` placeholders.
- Session metadata probing uses descriptor-relative guarded reads, the existing
  symlink protections, bounded UTF-8 recovery, and an independent request
  budget. Budget exhaustion remains visible and never converts an incomplete
  source into an exact empty list.
- No cloud sync, account access, telemetry, OAuth, hidden write path, or
  uncontrolled outbound request is introduced.

## Test Strategy

Follow red-green-refactor for each behavior:

1. Adapter tests for safe `CODEX_HOME`, plugin-cache manifests, manifest path
   escape rejection, staging exclusion, deterministic version choice, legacy
   marketplace compatibility, provenance, and read-only toggle rejection.
2. Service tests proving custom-home session and index discovery without a
   literal `.codex` path component.
3. Large-session tests proving an unsearched project page is complete when the
   aggregate session bodies exceed the summary budget, while selected detail
   remains bounded.
4. Protocol/catalog migration tests for optional provenance and older fixture
   compatibility.
5. Native model tests for `com.openai.codex` / `ChatGPT.app` candidate order,
   legacy fallbacks, plugin metadata presentation, and updated localized copy.
6. Focused Rust and Swift tests during implementation, followed by
   `pnpm check:macos`, `pnpm check:privacy`, and a real local launch verified
   through Computer Use.

## Acceptance Criteria

- Existing native Codex skills, config documents, snapshots, and stable catalog
  identities remain intact.
- Installed ChatGPT plugin skills from valid cache packages appear exactly once
  with parent plugin/version/source and a clear read-only status; staging and
  superseded versions do not appear.
- Agent Copilot never performs a Plugin Directory network call or plugin write.
- A safe custom `CODEX_HOME` supplies skills, configuration, sessions, and
  session indexes consistently; unsafe overrides fall back to `$HOME/.codex`.
- A real unsearched Codex session list with hundreds of large files remains
  completely pageable within the inventory limits, instead of terminating
  after the 64 MiB summary budget.
- The toolbar shows the current ChatGPT/Codex icon when the new desktop app is
  installed, while legacy installations still work.
- User-facing documentation clearly separates Codex, ChatGPT Work/Chat,
  ChatGPT plugins, and the existing `npx skills` Skill Package Manager.
- Full macOS, privacy, protocol-drift, list-completeness, and real-launch gates
  pass without touching real agent configuration during automated smoke tests.

