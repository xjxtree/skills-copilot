# Architecture

Agent Copilot uses a native macOS shell over a typed Rust service. Product logic
belongs in Rust crates; the UI presents state and sends typed requests.

## Goals

- Inspect local agent sessions, skills, config snapshots, and validation
  evidence.
- Keep deterministic local analysis useful without default Agent Copilot
  provider calls.
- Keep write, script, credential, cloud, telemetry, and release automation
  surfaces narrow and explicit.
- Share the same Rust service contract across current and future UI shells.

## Non-Goals

- Do not replace agent runtimes or proxy their tool calls.
- Do not parse private prompts beyond explicitly authorized local preview
  flows.
- Do not add cloud sync, accounts, telemetry, or marketplace behavior by
  default.
- Do not reintroduce the removed Web/Tauri shell.

## Layers

| Layer | Owner | Notes |
| --- | --- | --- |
| macOS app | `apps/macos` | SwiftUI/AppKit shell, view models, service client |
| Service boundary | `crates/service` | Typed JSON stdio request/response protocol |
| Command orchestration | `crates/commands` | Scans, toggles, snapshots, reports, provider gates |
| Core model | `crates/core` | Pure types and traits; no I/O |
| Adapters | `crates/adapters` | Agent root/config semantics |
| Scanner | `crates/scanner` | Root walking, symlink guards, skill parsing |
| Catalog | `crates/catalog` | Local SQLite catalog and app-local metadata |
| AI core | `crates/ai-core` | Deterministic rules and local analysis contracts |

## Dependency Direction

- `core` does not depend on higher crates.
- `adapters` and `scanner` depend on `core`.
- `catalog` and `ai-core` depend on `core`.
- `commands` composes scanner/catalog/adapter/AI behavior.
- `service` exposes the UI-independent protocol boundary.
- UI code must not call scanner/catalog internals directly.

## Data Flow

1. The app calls a typed service method such as `catalog.scanAll`.
2. Commands resolve project context and adapter roots.
3. Scanner enumerates candidate `SKILL.md` files inside allowed roots.
4. Adapters parse agent-specific metadata and enabled state.
5. Commands update the local catalog and derived findings.
6. The app reads typed service results for list, detail, config, session, and
   report surfaces.

## Extension Points

| Change | Add it here |
| --- | --- |
| New agent | `crates/adapters/src/<agent>/`, scanner/catalog tests, adapter docs |
| New service method | `crates/service`, fixtures, `docs/service-protocol.md` |
| New local rule | `crates/ai-core` |
| New macOS surface | `apps/macos` view/model/service patterns |
| New provider behavior | Provider profile gate with preview/redaction/confirmation |

## Compatibility

The displayed product name is Agent Copilot. Some module names, crate names,
sidecar names, AX identifiers, environment variables, or legacy app-data ids may
retain `SkillsCopilot` / `skills-copilot` compatibility where migration or
fixtures require it.
