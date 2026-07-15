# Development Tasks

This file is the active task-routing ledger. Future product work belongs in
`roadmap.md`; completed release history, version numbers, and changelogs belong
in GitHub tags and GitHub Releases.

## Active Task Rules

- Create a scoped entry here before changing user-visible behavior, protocol
  methods, adapter scope, validation policy, packaging, signing, or safety
  boundaries.
- Name the owning architecture area, intended behavior, safety boundary, and
  required verification. Link a focused design only when the task cannot be
  expressed concisely here.
- Keep documentation-only cleanup unversioned when it does not claim new
  product capability or validation evidence.
- If documentation claims completion, screenshot evidence, or verifier results,
  run the relevant command and record the real outcome in the task handoff or
  pull request, not as a permanent log here.
- Treat `scripts/list-completeness-surfaces.json` as the routing ledger for
  formal list ownership. List changes must update the matching manifest entry
  and pass `pnpm verify:list-completeness`.
- Remove completed entries from this file after their durable contracts are
  reflected in focused docs, tests, fixtures, and release history.

## Active Work

No follow-on implementation task is recorded. Add a scoped entry before
starting unrelated product work.

## Task Entry Template

```md
### <Task name>

- Owner: `<crate, service domain, or native surface>`
- Outcome: <observable result>
- Safety boundary: <writes, network, credentials, or none>
- Documentation: <focused contracts that must change>
- Verification: <focused tests and required gates>
```

## Durable Routing

- Future and deferred work: `roadmap.md`.
- Architecture and ownership: `../architecture.md`.
- Service behavior: `../service-protocol.md` plus protocol fixtures.
- Adapter behavior: `../adapters/agent-adapters.md` and focused adapter specs.
- Security and privacy boundaries: `../security-model.md`.
- UI evidence: `../ui-artifacts/`.
