# Documentation

Use this page to find the current contract or reusable procedure for a change.
Task notes do not belong in repository documentation.

## Product And Architecture

| Document | Purpose |
| --- | --- |
| [Architecture](architecture.md) | Layer ownership, dependency direction, data flow, and extension points |
| [Data model](data-model.md) | Persisted, transient, redacted, and compatibility data |
| [Service protocol](service-protocol.md) | Typed app/service methods, effects, paging, and error contracts |
| [Security model](security-model.md) | Trust, privacy, credential, write, script, and capture boundaries |
| [AI layer](ai-layer.md) | Optional Agent Copilot provider workflow and confirmation boundary |
| [UI delivery standards](ui-delivery-standards.md) | Native UI, loading, list completeness, and accessibility |

## Agent Adapters

- [Shared adapter contract](adapters/agent-adapters.md)
- [Codex](adapters/codex-adapter-spec.md)
- [opencode](adapters/opencode-adapter-spec.md)
- [Pi](adapters/pi-adapter-spec.md)
- [Hermes](adapters/hermes-adapter-spec.md)
- [OpenClaw](adapters/openclaw-adapter-spec.md)

Claude Code behavior is covered by the shared adapter contract.

## Development And Operations

| Document | Purpose |
| --- | --- |
| [AGENTS.md](../AGENTS.md) | Shared agent workflow and safety rules |
| [macOS app runbook](runbooks/macos-app-runbook.md) | Build and run commands |

## Maintenance Rules

- Update a focused contract when behavior changes; do not create a second
  overview for the same domain.
- Do not store plans, task ledgers, logs, or temporary handoffs in `docs/`.
- Update an existing focused contract instead of adding another overview.
- Keep Markdown links relative.
