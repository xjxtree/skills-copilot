# Distribution Runbook

This runbook records the public-distribution path for the macOS app. It is not
evidence that public distribution automation exists.

For local release-readiness, use `docs/runbooks/release-checklist.md`.

## Distribution Boundary

Public distribution requires an explicit scoped implementation for:

- Developer ID signing;
- notarization and stapling;
- DMG or ZIP packaging;
- checksum publishing;
- updater feeds;
- public download or release artifact automation.

Do not describe local app bundles as public release artifacts.

The manually scoped v0.1.x ZIP releases are an exception: generated app bundles
are ad-hoc signed for local bundle-integrity validation only. Ad-hoc signing is
not Developer ID signing and does not replace notarization.

## Version Strategy

- Keep source version, bundle version, release notes, and tag names aligned.
- Use SemVer for user-facing release versions.
- Use pre-release identifiers only when all downstream tooling accepts them.
- Do not tag or publish when source, bundle, and notes disagree.

## Signing

The current build script applies ad-hoc signing to the Rust service sidecar, the
native app executable, and the app bundle. This prevents invalid bundle
signatures in ZIP releases, but it is not identity-bearing distribution signing.

Before Developer ID signing is enabled:

- Confirm Apple Developer Team ID and certificate holder.
- Confirm signing identity name.
- Decide whether signing runs only on a maintainer machine or also in CI with
  protected secrets.
- Decide whether the Rust service sidecar is signed before the app bundle.
- Add entitlements only for real required capabilities.

Expected verification once signing exists:

```sh
codesign --verify --deep --strict --verbose=2 dist/AgentCopilot.app
codesign -dv --verbose=4 dist/AgentCopilot.app
spctl --assess --type execute --verbose=4 dist/AgentCopilot.app
```

## Notarization

Before notarization is enabled:

- Confirm notarization account and credential storage.
- Store notarization credentials outside the repository.
- Decide whether submission targets the app, ZIP, DMG, or multiple artifacts.
- Record notarization request ids in release notes or an internal release log.
- Staple tickets to user-facing artifacts where supported.

Expected verification once notarization exists:

```sh
xcrun stapler validate dist/AgentCopilot.app
spctl --assess --type execute --verbose=4 dist/AgentCopilot.app
```

## Packaging

Before packaging is enabled:

- Choose DMG, ZIP, or both.
- Define artifact naming.
- Define checksum generation and publication rules.
- Confirm screenshots, reports, fixture data, logs, credentials, and local
  config files are excluded.
- Confirm update-feed behavior is either implemented and validated or absent.

## Privacy Requirements

Distribution work must preserve the repository privacy stance:

- no cloud sync by default;
- no telemetry or anonymous crash reporting by default;
- no uncontrolled outbound network calls;
- no credentials in project files, logs, reports, screenshots, or prompts;
- no public artifact containing local catalog data or real user config.
