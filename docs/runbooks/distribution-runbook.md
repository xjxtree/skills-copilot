# Distribution Runbook

This runbook records the maintainer-operated distribution path for the macOS
app. It is not evidence that public distribution automation exists.

For local release-readiness, use `docs/runbooks/release-checklist.md`.

## Distribution Boundary

The repository provides explicit local commands for Developer ID signing,
notarization, stapling, optional post-staple ZIP creation, and candidate
verification. It does not select an identity, provision credentials, run those
commands automatically, or publish their output.

Public distribution still requires a maintainer-owned decision and credentials
for:

- the Developer ID identity and notarization Keychain profile;
- artifact architecture and naming;
- checksum publication;
- DMG packaging;
- updater feeds;
- public download or release artifact automation.

Do not describe local app bundles as public release artifacts.

Ad-hoc signing remains the development default and is valid only for local
bundle-integrity validation. `--allow-ad-hoc` qualification is never release
evidence and does not replace Developer ID signing or notarization.

## Version Strategy

- Keep source version, bundle version, release notes, and tag names aligned.
- Use SemVer for user-facing release versions.
- Treat GitHub tags and GitHub Releases as the release-history source of truth.
- Treat a standalone tag without a published GitHub Release as non-distributable.
  If a withdrawn tag must remain for traceability, publish a clearly named
  pre-release note with no assets that points users to the current supported
  release.
- Default the next release to the next patch version after the latest published
  SemVer release unless a maintainer specifies another version.
- Use pre-release identifiers only when all downstream tooling accepts them.
- Do not tag or publish when source, bundle, and notes disagree.

## Signing

Normal builds apply ad-hoc signatures to the Rust service sidecar, native app
executable, and app bundle. A release build opts into identity-bearing signing
only when the maintainer supplies
`AGENT_COPILOT_SIGNING_IDENTITY` or `--signing-identity`. The build refuses that
option for debug configuration, signs nested code before the outer bundle,
requests a secure timestamp, enables hardened runtime, and verifies the result
is a Developer ID Application signature. No entitlements are added unless a
real capability later requires them.

Example:

```sh
AGENT_COPILOT_SIGNING_IDENTITY="Developer ID Application: <holder> (<team>)" \
  pnpm build:macos:release:arm64
pnpm verify:macos-distribution
```

Signing remains maintainer-local. CI has no signing identity or notarization
credential path.

## Notarization

Create a `notarytool` credential profile in the macOS Keychain outside this
repository. The notarization command accepts only the profile name; it never
accepts or stores an Apple ID, password, API key, issuer id, or private key.
It performs signed-candidate verification (including rejection of
`get-task-allow=true`), creates a temporary submission ZIP, waits for Apple
acceptance, requires the returned notarization log to contain no issues,
staples the app, validates the ticket and Gatekeeper result, and optionally
writes a new post-staple ZIP and checksum. It refuses to overwrite an existing
ZIP and does not publish anything.

```sh
pnpm notarize:macos -- \
  --keychain-profile <profile> \
  --output-zip dist/AgentCopilot-<version>-<arch>.zip
```

Record the printed notarization request id, artifact checksum, architecture,
and release commit in the GitHub Release. An accepted request without a stapled
and Gatekeeper-approved artifact is incomplete.

## Packaging

The supported local output is an optional architecture-specific ZIP created
only after successful stapling. DMG creation remains out of scope. Build with a
non-user-specific SwiftPM scratch path as documented in
`docs/runbooks/macos-app-runbook.md`; the verifier checks bundle/source version
agreement, icon presence, executable architecture parity, signature integrity,
hardened runtime, local-path absence, stapling, and Gatekeeper according to the
requested stage.

Before publishing, independently confirm screenshots, reports, fixture data,
logs, credentials, private catalog data, and local config files are absent from
the archive. Update-feed behavior remains absent.

## Privacy Requirements

Distribution work must preserve the repository privacy stance:

- no cloud sync by default;
- no telemetry or anonymous crash reporting by default;
- no uncontrolled outbound network calls;
- no credentials in project files, logs, reports, screenshots, or prompts;
- no public artifact containing local catalog data or real user config.
