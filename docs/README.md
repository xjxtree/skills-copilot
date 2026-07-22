# Documentation

Use this page to find the current contract or reusable procedure for a change.
Task state and future plans belong in GitHub issues or pull requests; release
history belongs in GitHub Releases and tags.

## Product And Architecture

| Document | Purpose |
| --- | --- |
| [Architecture](architecture.md) | Layer ownership, dependency direction, data flow, and extension points |
| [Data model](data-model.md) | Persisted, transient, redacted, and compatibility data |
| [Service protocol](service-protocol.md) | Typed app/service methods, effects, paging, and error contracts |
| [Security model](security-model.md) | Trust, privacy, credential, write, script, and capture boundaries |
| [AI layer](ai-layer.md) | Optional Agent Copilot provider workflow and confirmation boundary |
| [UI delivery standards](ui-delivery-standards.md) | Native UI, loading, list completeness, accessibility, and runtime validation |

## Agent Adapters

- [Shared adapter contract](adapters/agent-adapters.md)
- [Codex](adapters/codex-adapter-spec.md)
- [opencode](adapters/opencode-adapter-spec.md)
- [Pi](adapters/pi-adapter-spec.md)
- [Hermes](adapters/hermes-adapter-spec.md)
- [OpenClaw](adapters/openclaw-adapter-spec.md)

Claude Code behavior is covered by the shared adapter contract and its fixtures.

## Development And Operations

| Document | Purpose |
| --- | --- |
| [AI-agent workflow](ai-agent-workflow.md) | Instruction ownership, source priority, and validation workflow |
| [macOS app runbook](runbooks/macos-app-runbook.md) | Build, launch, fixture smoke, and real-local validation |
| [Release checklist](runbooks/release-checklist.md) | Maintainer release-readiness checks |
| [Distribution runbook](runbooks/distribution-runbook.md) | Public-distribution boundaries and release requirements |

## Maintenance Rules

- Update a focused contract when behavior changes; do not create a second
  overview for the same domain.
- Do not store plans, task ledgers, progress reports, validation transcripts,
  screenshots, temporary handoffs, release notes, or changelogs in `docs/`.
- Update existing focused contracts instead of adding dated or versioned
  snapshots of the same behavior.
- Keep Markdown links relative and run `pnpm verify:doc-governance` after
  reorganizing documents.
