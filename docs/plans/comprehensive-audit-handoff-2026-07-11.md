# Comprehensive Audit Continuation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` (recommended) or
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Continue the repository-wide audit from the last verified integration
baseline, finish every reviewed finding, and end with one independently reviewed
and fully verified integration branch.

**Architecture:** Keep `main` untouched and use the existing
`codex/comprehensive-audit-fixes` branch only as the integration branch. Finish,
review, and verify each architecture-bounded task in its isolated branch before
ordered cherry-pick integration; never mix a later task into an earlier review
range.

**Tech Stack:** Rust workspace, SwiftPM/SwiftUI/AppKit, Node.js/pnpm validation
scripts, Git worktrees, JSON service protocol fixtures, fixture-only macOS smoke.

## Global Constraints

- The user explicitly asked to skip the Codex/deep security scan. Do not run it.
  Ordinary repository privacy checks remain required.
- Preserve the architecture and safety boundaries in `AGENTS.md`; especially,
  product logic stays in Rust and macOS UI uses the typed Rust service protocol.
- Do not edit or reset `main`, revert user work, push, publish, sign, notarize,
  package, or add network behavior.
- Each implementation or review task uses its own existing or newly created Git
  worktree. A worker must not switch branches or edit the integration checkout.
- Critical or Important review findings block integration. Resolve or explicitly
  disposition every Minor finding before integration.
- Protocol changes must update documentation, fixtures, effect metadata, and
  drift verification together.
- Run focused checks for each task. Run `pnpm check:macos` for major, UI,
  protocol, or milestone changes, and run `pnpm check:privacy` before every
  commit or handoff.
- Smoke validation must use fixture data. Do not read or mutate real user agent
  configuration.

---

## Handoff Snapshot

- Cutoff date: 2026-07-11 (Asia/Shanghai).
- Untouched base: `main` at `216b47befe2bbb2accb0143033cc42c6ed5835be`.
- Integration branch: `codex/comprehensive-audit-fixes`.
- Last verified code head before this handoff document:
  `09901bdc13e6ea7001cda7a7973416fea9fc01bd`.
- The current task was Skill Manager Task 6 integration. It is complete and no
  later autosave or protocol-consumer commit has been started on the integration
  branch.
- Source plans with exact implementation steps:
  [scanner/local sessions](../superpowers/plans/2026-07-10-scanner-local-sessions.md),
  [service/privacy](../superpowers/plans/2026-07-10-service-consistency-privacy.md),
  [macOS reliability/UX](../superpowers/plans/2026-07-10-macos-reliability-ux.md),
  and [quality/CI/governance](../superpowers/plans/2026-07-10-quality-ci-governance.md).

### Fresh evidence for the cutoff head

The following commands were run sequentially on clean head `09901bdc` and all
exited with status 0:

```sh
swift test --package-path apps/macos
pnpm test:macos-native-models
pnpm verify:macos-native-test-registry
pnpm verify:macos-ui-layout
pnpm verify:gate-parity
pnpm check:privacy
git diff --check
git status --short --branch
```

The native registry reported service suites `2`, main suites `21`, SkillStore
groups `64`, and named tests `87`; the UI layout verifier reported 105 checks.
The final status contained only the integration branch header.

## Findings Summary

| Area | Main finding classes | Handoff state |
| --- | --- | --- |
| Scanner and paths | Unbounded traversal/content work, implicit roots, partial scans deleting valid cache entries, lexical/canonical alias ambiguity, and symlink races | Scanner Tasks 1–3 and path identity are integrated; local-session Task 4 awaits final review; Scanner Tasks 5–8 remain |
| Service consistency | No exhaustive read/write effect contract, read-only calls causing physical writes, stale config save/rollback races, and a deprecated YAML dependency | Tasks 1–3 and 5–6 are integrated; final service/privacy gate waits for privacy Task 4 |
| Privacy history gate | Tracked-only coverage, Git environment/config redirection, object-type and case-folding bypasses, unbounded metadata, raw diagnostic leakage, and a stderr FIFO lifecycle hang | Semantic and bounded-supervisor fixes are committed through `d8205da3`; independent final review and integration remain |
| Local sessions | Unbounded reads, system/developer/summary leakage, ambiguous role carriers, empty rows, path check/use races, unbounded sidecars/indexes, nondeterministic inventory, and paging before sort | Task 4 final fix is committed but needs independent approval; Tasks 5–8 remain |
| macOS reliability | Sensitive history persistence, pipe deadlocks/unbounded diagnostics, autosave ordering/cancellation races, stale Skill Manager generations, stale protocol-v2 bindings, and pending navigation/AX/layout issues | Tasks 1, 2, and 6 are integrated; Tasks 3 and 4 are approved but not integrated; Tasks 5 and 7–9 remain |
| CI and governance | Partial native-test registration, GUI-dependent smoke/CI, incomplete branch triggers, bypassable documentation checks, and absent runtime/RSS/module ratchets | Native registry, headless smoke, and CI are integrated; governance Tasks 5–6 await review; budget Task 7 and final Task 8 remain |

## Status Ledger

### Integrated and freshly verified

| Plan area | Tasks/results | Integration commits |
| --- | --- | --- |
| Scanner/local sessions | Scanner Tasks 1–3 | `0ae91860`, `114b333d`, `a51818f1`, `b206abac`, `4e997432` |
| Service/privacy | Service Tasks 1–3 | `19cf3317`, `74429c07`, `0811bf44`, `bde1d5e6` |
| Service/privacy | YAML behavior freeze and maintained dependency migration, Tasks 5–6 | `d9fabd50`, `c5b6c098` |
| Cross-cutting filesystem | Bundle/path filesystem identity and race handling | `cdc927ff` plus the headless-smoke hardening series |
| macOS reliability | Session-only preflight history, Task 1 | `a388f09a` |
| macOS reliability | Concurrent bounded service pipe drains and diagnostic redaction, Task 2 | `0a2d8aed` through `67ed914c` |
| Quality/CI | Complete native registry, headless bundled-sidecar smoke, and build-only/all-push CI, Tasks 2–4 | `16eec9af`, `947d6eef` through `1cce2879`, `a863d8e6` |
| macOS reliability | Skill Manager immutable request generations, Task 6 | `060711f6`, `965bb60e`, `32d47705`, `09901bdc` |

Skill Manager Task 6 received a complete-range independent review with Critical
0, Important 0, Minor 0 before integration. Its post-integration verification
is the fresh cutoff evidence recorded above.

### Implemented and approved, not yet integrated

| Task | Branch/head | Review result | Integration rule |
| --- | --- | --- | --- |
| macOS autosave Task 3 | `codex/macos-revision-autosave` at `93e21c93` | Final independent review: 0/0/0, specification pass, approved | Cherry-pick only `dcb8d758`, `4661a7f9`, `12a33b10`, `93e21c93`; do not replay its older Skill Manager base commit |
| macOS protocol-v2 consumer / Quality Task 1 / macOS Task 4 | `codex/macos-protocol-v2` at `df6970d2` | Final independent review: 0/0/0, specification pass, approved, including a real v2 fixture smoke | Integrate only after autosave: `7ef0a89b`, `a80a2ffc`, `6f27e0b7`, `df6970d2`; then review the combined Swift state |

### Implemented, not approved for integration

| Task | Branch/head | What remains |
| --- | --- | --- |
| Local-session bounded reader Task 4 | `codex/bounded-local-session-reader` at `e4ff087b` | Final20 fixes all three final19 Important findings plus mtime/row-ID reopen variants and passed its author gates. A fresh independent complete-range final21 review is still required. Do not start Task 5 from this branch before approval and integration. |
| Repository governance Quality Tasks 5–6 | `codex/repository-governance` at `99e71980` | Final5 addresses five Important and one Minor finding and passed author gates. A fresh independent complete-range review is still required. |
| Privacy history Service Task 4 | `codex/privacy-history-gate` at `d8205da3` | Final5 closes the reproducible inherited-writer FIFO hang with one bounded supervisor and passed one explicit complete run under Bash 3.2 and Bash 5.3 plus focused timeout/high-volume tests. Additional repeated full runs and a fresh independent full-range review remain before integration. |

### Not started or not yet integrated

- Scanner Tasks 5–8: aggregate sidecar bounds/request-local Codex cache;
  deterministic inventory/sort-before-page; summary/detail protocol and macOS
  cache; then full scanner review/gates.
- macOS Task 5: cache session summaries and fetch one bounded detail by stable
  ID. Execute with Scanner Task 7, not as an independent competing protocol.
- macOS Tasks 7–9: unified navigation/global-search keyboard state;
  privacy-safe presentation, stable Accessibility semantics, 960-point compact
  layout; then the integrated reliability/UX release gate.
- Quality Task 7: runtime/RSS budgets and module splitting/ratchets. Re-inventory
  only after all preceding merges and split oversized files; never raise or add
  a ratchet to absorb growth.
- Service Task 7 and Quality Task 8: integrated verification after their owning
  implementations land.
- Final whole-branch independent review, full repository gates, and the final
  audit report.

## Ordered Continuation Tasks

### Task 1: Independently approve and integrate local-session Task 4

**Files:** Use the exact Task 4 files and tests in the
[scanner/local sessions plan](../superpowers/plans/2026-07-10-scanner-local-sessions.md).

**Interfaces:** Consumes the integrated scanner limits; produces the
descriptor-bound primary reader required by Scanner Task 5.

- [ ] Review the complete range `16eec9af..e4ff087b` read-only, including the
  final19 external harness and the final20 empty-row, role-evidence, final-file,
  mtime, row-ID, FIFO/socket/device, and parent/static symlink regressions.
- [ ] Confirm no Task 5 sidecar aggregation or Codex index cache work is mixed
  into the range and record Critical/Important/Minor counts.
- [ ] If approved, cherry-pick the ordered range onto the integration branch;
  preserve the integration branch's scanner compatibility registration when
  resolving `crates/service/src/lib.rs`.
- [ ] Run focused local-session tests, `cargo test --workspace`, strict Clippy,
  `pnpm check:privacy`, and `pnpm check:macos` before marking it integrated.

### Task 2: Finish, review, and integrate the privacy history gate

**Files:**

- Modify: `script/check_privacy.sh`
- Modify: `docs/security-model.md`
- Create the matcher and regression files declared by Task 4 in the
  [service/privacy plan](../superpowers/plans/2026-07-10-service-consistency-privacy.md).
- Modify the Task 4 workflow/package wiring introduced by the privacy branch.

**Interfaces:** Produces a fail-closed, bounded current/index/history gate that
leaves no process, FIFO, or temporary artifact.

- [x] Finish the single-supervisor stderr lifecycle change on
  `codex/privacy-history-gate` as `d8205da3`; the rejected detached PID-target
  watchdog design is not present.
- [x] Prove inherited-writer timeout, 32 MiB stderr without backpressure, no
  orphan/temporary residue, and one complete run under Bash 3.2 and Bash 5.3
  with explicit success. Repeat full runs during independent review.
- [ ] Independently review the full range from `b42aa025` through the new head,
  including case folding, mode-160000 object types, raw metadata charging, Git
  environment/config neutralization, OID binding, shallow history, and resource
  limits.
- [ ] If approved, integrate the full range. Resolve workflow/package conflicts
  by preserving the integration branch's all-push, build-only, headless smoke
  behavior while adding the privacy test job and command.
- [ ] Run `pnpm test:privacy-check`, `pnpm check:privacy`, gate parity, Rust
  workspace tests, strict Clippy, and `pnpm check:macos`.

### Task 3: Review and integrate repository governance

**Files:** Use Quality Tasks 5–6 in the
[quality/CI/governance plan](../superpowers/plans/2026-07-10-quality-ci-governance.md).

**Interfaces:** Produces manifest-driven Markdown/path/anchor/gate governance
consumed by Quality Task 7.

- [ ] Independently review `b42aa025..99e71980`, specifically HTML/code block
  exclusion, literal Markdown links versus backtick globs, exact GitHub slugs,
  case-insensitive forbidden patterns, controlled Git state, linked worktrees,
  and iterative deep-AST traversal.
- [ ] If approved, cherry-pick the full ordered range and preserve current
  package scripts, CI behavior, protocol v2, and exact lockfile dependencies.
- [ ] Run repository-governance tests, all Node tests, documentation governance,
  JS syntax, gate parity, privacy, Rust workspace tests, strict Clippy, Swift
  tests, and standalone native model tests.

### Task 4: Compose the approved Swift branches in strict order

**Files:** Use macOS Tasks 3–4 in the
[macOS reliability/UX plan](../superpowers/plans/2026-07-10-macos-reliability-ux.md)
and Quality Task 1 in the
[quality/CI/governance plan](../superpowers/plans/2026-07-10-quality-ci-governance.md).

**Interfaces:** Produces one Store/UI state containing Skill Manager
generations, revision autosave, and protocol-v2 CAS/token binding.

- [ ] Cherry-pick the four autosave-only commits listed in the status ledger;
  resolve Store, localization, registry, and test conflicts while preserving
  Skill Manager generation ownership.
- [ ] Run autosave/Store focused suites plus both Swift entrypoints and privacy.
- [ ] Cherry-pick the four protocol-consumer commits listed in the status
  ledger; preserve autosave, process/pipe, Skill Manager, and service protocol
  v2 behavior in every conflict.
- [ ] Run focused config/rollback/model/RPC suites, protocol drift, both Swift
  entrypoints, `pnpm check:macos`, and privacy.
- [ ] Obtain a fresh independent complete-range review of the combined Swift
  state before starting macOS Tasks 5, 7, or 8.

### Task 5: Execute Scanner Tasks 5–8 sequentially

**Files and code:** Follow Tasks 5–8 verbatim in the
[scanner/local sessions plan](../superpowers/plans/2026-07-10-scanner-local-sessions.md).

**Interfaces:** Task 5 consumes approved Task 4; Task 6 consumes Task 5; Task 7
changes the service protocol and jointly implements macOS Task 5; Task 8 is the
combined gate.

- [ ] Task 5: enforce 240 sidecar files and 512 KiB per session, 32 KiB
  head/tail per sidecar, and one request-local bounded Codex index load.
- [ ] Task 6: inventory all bounded candidates, select newest deterministically,
  validate typed sort/direction, and sort before pagination.
- [ ] Task 7 with macOS Task 5: add truthful candidate truncation and
  content-included fields, summary-only list requests, one-detail-by-stable-ID,
  fixtures/docs/drift updates, and cache race coverage.
- [ ] Task 8: run focused, workspace, strict Clippy, protocol, Swift, macOS,
  fixture smoke, privacy, and independent boundary review gates.

### Task 6: Finish remaining macOS reliability and UX work

**Files and code:** Follow Tasks 7–9 in the
[macOS reliability/UX plan](../superpowers/plans/2026-07-10-macos-reliability-ux.md).

- [ ] Task 7: unify navigation and deterministic global-search keyboard state.
- [ ] Task 8: enforce redacted presentation, stable AX semantics, and compact
  960-point layout using fixture-only accessibility validation.
- [ ] Task 9: run the integrated reliability/UX release gate and independent
  review. Capture only the full app window if a screenshot is required.

### Task 7: Enforce final quality budgets and integrated gates

**Files and code:** Follow Tasks 7–8 in the
[quality/CI/governance plan](../superpowers/plans/2026-07-10-quality-ci-governance.md)
and Task 7 in the
[service/privacy plan](../superpowers/plans/2026-07-10-service-consistency-privacy.md).

- [ ] Re-inventory every covered Rust, Swift, and Node module after all feature
  merges. Split/simplify every file above its fixed ceiling; do not increase a
  ceiling or create a new legacy ratchet.
- [ ] Measure only the warmed 10k test executable: elapsed at most 8000 ms and
  RSS at most 640 MiB. Enforce native-list p95 at most 80 ms.
- [ ] Run Service Task 7, Quality Task 8, and a fresh full `pnpm check:macos`.

### Task 8: Final whole-branch review and handoff

**Files:** Modify only files owned by a reproduced finding.

- [ ] Review the entire `main..codex/comprehensive-audit-fixes` range by
  architecture boundary and reconcile every item in this ledger.
- [ ] Run formatting, workspace tests, strict Clippy, excluded fuzz check,
  protocol/effect drift, Node/governance/quality gates, both Swift entrypoints,
  build-only bundle validation, fixture-only smoke, performance budgets,
  `pnpm check:privacy`, `git diff --check`, and a clean status check.
- [ ] Keep the security scan explicitly recorded as skipped by user request;
  do not present it as executed or passed.
- [ ] Update this active ledger, then prepare the concise final findings/fixes,
  remaining-risk, and verification report.

## Resume Command Checklist

```sh
git worktree list
git -C /private/tmp/agent-copilot-comprehensive status --short --branch
git -C /private/tmp/agent-copilot-comprehensive log -1 --oneline
```

Before any cherry-pick, compare the actual branch head with the hashes in this
document. If a finishing commit was added after the cutoff, review and record
that exact new range rather than assuming the old hash is current.
