# Roadmap

This file tracks future work and deferred scope. Completed release history,
version numbers, and main changelogs belong in GitHub tags and GitHub
Releases.

## Near-Term Work

- Keep documentation lean: entry docs should describe product purpose,
  boundaries, commands, and navigation rather than version history.
- Add focused tests when view models, RPC wrappers, or adapter helpers gain
  behavior beyond type-safe forwarding.
- Keep screenshot and UI artifact indexes limited to durable evidence files.
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
