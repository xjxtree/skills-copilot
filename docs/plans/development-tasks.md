# Development Tasks

This file is for active task routing. Completed release history, version
numbers, and main changelogs belong in GitHub tags and GitHub Releases.

## Active Task Rules

- Create a scoped task before changing user-visible behavior, protocol methods,
  adapter scope, validation policy, packaging, signing, or safety boundaries.
- Keep documentation-only cleanup unversioned when it does not claim new product
  capability or new validation evidence.
- If documentation claims completion, screenshot evidence, or verifier results,
  run the relevant command and record the real outcome.
- Keep work items close to the owning architecture boundary.

## Backlog

- Reduce duplicated historical prose in active documentation.
- Add focused tests for view models and RPC wrappers when they gain behavior.
- Keep adapter capability text aligned with `docs/adapters/agent-adapters.md`.
- Keep runbooks focused on commands and decision rules, not closeout history.

## Active Comprehensive Audit

- Resume the repository-wide audit from the verified cutoff, branch ledger,
  review blockers, and ordered next actions in the
  [2026-07-11 comprehensive audit handoff](comprehensive-audit-handoff-2026-07-11.md).
- The handoff is the active routing source for this audit. Update its status
  ledger when a task is approved or integrated; do not infer completion from an
  implementation branch alone.

## Done Elsewhere

- User-facing release notes and main changelogs live in GitHub Releases.
- Release versions are determined from GitHub tags or an explicit maintainer
  instruction.
- UI artifact evidence lives under `docs/ui-artifacts/`.
