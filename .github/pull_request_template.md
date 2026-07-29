## Summary

What changed and why?

## Scope

Describe the single reviewable outcome of this pull request and list anything
intentionally deferred. An issue link is optional.

## Type

- [ ] Rust logic or service protocol
- [ ] Native macOS UI or model
- [ ] Security or privacy
- [ ] Documentation or validation tooling
- [ ] Adapter contract

## Checklist

- [ ] The change is independently reviewable and contains no unrelated cleanup
- [ ] I read `AGENTS.md` and the task-relevant documents
- [ ] I kept architecture, adapter, security, privacy, and write boundaries intact
- [ ] I added or updated focused regression coverage
- [ ] I updated related contracts or fixtures when behavior changed
- [ ] I ran the relevant focused checks and recorded their exact results below
- [ ] I ran `pnpm check:privacy`
- [ ] Any UI capture is a full app-window image outside the repository

## Evidence

List exact commands and outcomes. For UI changes, include real-local validation
or the canonical blocker. Link official adapter documentation or sanitized
fixture provenance when adapter behavior changes.
