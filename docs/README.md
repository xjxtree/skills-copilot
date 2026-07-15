# Documentation

Use this page to find the durable contract or procedure for a change. Keep
release history in GitHub Releases, future work in `plans/roadmap.md`, and only
active routing in `plans/development-tasks.md`.

## Product And Architecture

| Document | Purpose |
| --- | --- |
| [Architecture](architecture.md) | Layer ownership, dependency direction, data flow, and extension points |
| [Data model](data-model.md) | Persisted, transient, redacted, and compatibility data |
| [Service protocol](service-protocol.md) | Typed app/service methods, effects, paging, and error contracts |
| [Security model](security-model.md) | Trust, privacy, credential, write, script, and evidence boundaries |
| [AI layer](ai-layer.md) | Optional Agent Copilot provider workflow and confirmation boundary |
| [UI delivery standards](ui-delivery-standards.md) | Native UI, loading, list completeness, accessibility, and screenshot rules |

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
| [Distribution runbook](runbooks/distribution-runbook.md) | Deferred public-distribution decisions and requirements |
| [Roadmap](plans/roadmap.md) | Future work and scope requiring a new safety review |
| [Development tasks](plans/development-tasks.md) | Active task-routing ledger only |
| [UI artifacts](ui-artifacts/README.md) | Durable app-window evidence and capture rules |

## Maintenance Rules

- Update a focused contract when behavior changes; do not create a second
  overview for the same domain.
- Remove completed implementation plans after their behavior is represented by
  code, tests, fixtures, and focused contracts. Git history preserves the plan.
- Do not store validation transcripts, temporary handoffs, release notes, or
  changelogs in `docs/`.
- Keep Markdown links relative and run `pnpm verify:doc-governance` after
  reorganizing documents.
