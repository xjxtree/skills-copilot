# Roadmap

This file tracks future work and deferred scope. Completed release history,
version numbers, and main changelogs belong in GitHub tags and GitHub
Releases.

## Near-Term Work

- Add an in-app preview/confirmation action that removes only a proven dangling
  Agent skill link; do not add a general filesystem delete surface.
- Strengthen full-uninstall recovery across the external manager, lock state,
  Agent links, and app-owned source cleanup. The existing single-confirmation
  sequence is guarded but cannot provide a cross-process atomic rollback.
- Replace or remove UI evidence only after a fresh app-window-only capture
  proves the corresponding surface. Fixture smoke remains supporting evidence,
  not a substitute for required real-local validation.
- Add focused tests when view models, RPC wrappers, or adapter helpers gain
  behavior beyond type-safe forwarding.
- Keep service protocol, fixtures, and drift verification synchronized whenever
  service behavior changes.

## Scope That Requires A New Safety Review

- Network-backed skill installs or uncontrolled fetch.
- Skill script execution.
- Signing, notarization, packaging, public distribution, updater feeds, release
  automation, DMG, or ZIP output.
- Raw multi-agent config editors.
- Broader adapter config writes.
- Broader session parsing without confirmed local-store evidence.
- Credential storage outside Keychain.
- Cloud sync, telemetry, anonymous crash reports, or accounts.

## Planning Rules

- Prefer small scoped changes with focused validation.
- Do not mark work complete from docs alone; link completion claims to real
  verifier output or release evidence.
- Keep historical status out of entry docs. Use GitHub Releases and tags for
  release history.
