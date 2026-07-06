# Manual Release Checklist

This checklist is for maintainer-run local release-readiness review. It is not
a public distribution checklist.

## Preflight

- [ ] Work from the intended branch or release-candidate commit.
- [ ] Confirm `git status --short` contains only intended changes.
- [ ] Read `README.md`, `AGENTS.md`, `docs/plans/roadmap.md`, and
  `docs/runbooks/distribution-runbook.md`.
- [ ] Confirm public distribution, signing, notarization, DMG, ZIP, updater,
  and release automation remain deferred unless explicitly scoped.
- [ ] Confirm script execution remains default-denied unless separately scoped
  with preview, confirmation, audit, and LLM separation.
- [ ] Record the exact commit hash.

## Version Sanity

- [ ] Inspect the service crate version.
- [ ] Build or verify the app bundle through the normal macOS gate.
- [ ] Confirm bundle version, GitHub Release draft, tag name, PR title, and
  tracking issue agree.
- [ ] Do not tag or announce a version when source, bundle, and notes disagree.

## Required Gates

```sh
pnpm check:macos
pnpm check:privacy
```

For screenshot changes, also run:

```sh
pnpm verify:screenshot-artifacts
```

For documentation-only cleanup, at minimum run:

```sh
git diff --check
pnpm verify:doc-governance
pnpm verify:gate-parity
pnpm check:privacy
```

## Fixture Smoke Boundary

The smoke pass must use fixture data:

```sh
pnpm smoke:macos-app -- --fixture-data --capture-window
```

Confirm it:

- [ ] uses temporary HOME, app data, and project roots;
- [ ] does not read, create, or modify real user agent config;
- [ ] validates the existing app bundle rather than rebuilding it;
- [ ] captures only the app window;
- [ ] avoids visible local paths, secrets, token shapes, and credential
  placeholders;
- [ ] does not execute skill scripts.

## Real-Local UI Evidence

For user-visible, UI, or service-protocol candidates, operate the real app in an
unlocked interactive macOS session and record the result. If Computer Use/AX
cannot resolve the app window, record the canonical blocker and keep real-local
validation pending.

Fixture smoke is supporting evidence only; it is not a substitute for required
real-local interaction evidence.

## Abort Conditions

- `pnpm check:macos` fails.
- `pnpm check:privacy` fails.
- `git diff --check` fails.
- Smoke uses or mutates real user config.
- A screenshot includes the desktop, wallpaper, menu bar, Dock, or unrelated
  windows.
- A candidate includes credentials, raw local config, logs, private catalog
  data, or unredacted local paths.
- Docs or scripts claim public distribution automation or real script execution
  exists without explicit scoped implementation and validation.

## Closeout

- [ ] Record commands run and exact outcomes.
- [ ] Put the version number, release notes, assets, and checksums in the
  GitHub Release.
- [ ] Confirm the GitHub tag, release title, bundle version, and asset names
  agree before publishing.
- [ ] Do not hand off or publish artifacts when any abort condition applies.
