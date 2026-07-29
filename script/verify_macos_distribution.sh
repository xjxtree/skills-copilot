#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_BUNDLE="$ROOT_DIR/dist/AgentCopilot.app"
ALLOW_AD_HOC=0
REQUIRE_NOTARIZATION=0

usage() {
  cat >&2 <<USAGE
usage: $0 [--app path] [--allow-ad-hoc] [--require-notarization]

Validates a macOS release candidate without modifying it.

By default the candidate must use Developer ID Application signing and the
hardened runtime. --allow-ad-hoc is only for local architecture/privacy
qualification and is not distribution evidence. --require-notarization also
requires a stapled ticket and a successful Gatekeeper assessment.
USAGE
}

fail() {
  echo "distribution verification failed: $*" >&2
  exit 1
}

if [[ "${1:-}" == "--" ]]; then
  shift
fi

while [[ $# -gt 0 ]]; do
  case "$1" in
    --app)
      if [[ $# -lt 2 || -z "$2" ]]; then
        echo "--app requires a bundle path" >&2
        usage
        exit 2
      fi
      APP_BUNDLE="$2"
      shift 2
      ;;
    --app=*)
      APP_BUNDLE="${1#--app=}"
      if [[ -z "$APP_BUNDLE" ]]; then
        echo "--app requires a bundle path" >&2
        usage
        exit 2
      fi
      shift
      ;;
    --allow-ad-hoc)
      ALLOW_AD_HOC=1
      shift
      ;;
    --require-notarization)
      REQUIRE_NOTARIZATION=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done

if [[ "$ALLOW_AD_HOC" == "1" && "$REQUIRE_NOTARIZATION" == "1" ]]; then
  fail "--allow-ad-hoc cannot be combined with --require-notarization"
fi

for command in codesign file lipo plutil strings; do
  command -v "$command" >/dev/null 2>&1 || fail "required command is unavailable: $command"
done

[[ -d "$APP_BUNDLE" ]] || fail "app bundle does not exist"
[[ ! -L "$APP_BUNDLE" ]] || fail "app bundle cannot be a symbolic link"

INFO_PLIST="$APP_BUNDLE/Contents/Info.plist"
[[ -f "$INFO_PLIST" ]] || fail "Info.plist is missing"

BUNDLE_ID="$(plutil -extract CFBundleIdentifier raw -o - "$INFO_PLIST")"
APP_VERSION="$(plutil -extract CFBundleShortVersionString raw -o - "$INFO_PLIST")"
BUNDLE_VERSION="$(plutil -extract CFBundleVersion raw -o - "$INFO_PLIST")"
APP_EXECUTABLE="$(plutil -extract CFBundleExecutable raw -o - "$INFO_PLIST")"
MIN_SYSTEM_VERSION="$(plutil -extract LSMinimumSystemVersion raw -o - "$INFO_PLIST")"

[[ "$BUNDLE_ID" == "dev.agent-copilot.native" ]] || fail "unexpected bundle identifier"
[[ "$APP_VERSION" == "$BUNDLE_VERSION" ]] || fail "short and bundle versions differ"
[[ "$MIN_SYSTEM_VERSION" == "13.0" ]] || fail "unexpected minimum system version"

SOURCE_VERSION="$(awk -F'"' '/^version = / {print $2; exit}' "$ROOT_DIR/crates/service/Cargo.toml")"
[[ "$APP_VERSION" == "$SOURCE_VERSION" ]] || fail "bundle and service source versions differ"

APP_BINARY="$APP_BUNDLE/Contents/MacOS/$APP_EXECUTABLE"
SERVICE_BINARY="$APP_BUNDLE/Contents/Resources/skills-copilot-service"
ICON_FILE="$APP_BUNDLE/Contents/Resources/AppIcon.icns"

[[ -x "$APP_BINARY" ]] || fail "main executable is missing or not executable"
[[ -x "$SERVICE_BINARY" ]] || fail "service sidecar is missing or not executable"
[[ -f "$ICON_FILE" ]] || fail "packaged app icon is missing"

APP_ARCHES="$(lipo -archs "$APP_BINARY")"
SERVICE_ARCHES="$(lipo -archs "$SERVICE_BINARY")"
[[ "$APP_ARCHES" == "$SERVICE_ARCHES" ]] || fail "main executable and sidecar architectures differ"

for architecture in $APP_ARCHES; do
  case "$architecture" in
    arm64|x86_64)
      ;;
    *)
      fail "unsupported executable architecture"
      ;;
  esac
done

codesign --verify --strict --verbose=2 "$SERVICE_BINARY"
codesign --verify --strict --verbose=2 "$APP_BINARY"
codesign --verify --deep --strict --verbose=2 "$APP_BUNDLE"

EXPECTED_TEAM_ID=""
for signed_item in "$SERVICE_BINARY" "$APP_BINARY" "$APP_BUNDLE"; do
  SIGNING_DETAILS="$(codesign -dvvv "$signed_item" 2>&1)"
  ENTITLEMENTS="$(codesign -d --entitlements - "$signed_item" 2>/dev/null || true)"
  ENTITLEMENTS_COMPACT="${ENTITLEMENTS//$'\n'/}"
  GET_TASK_ALLOW_PATTERN='<key>com\.apple\.security\.get-task-allow</key>[[:space:]]*<true[[:space:]]*/>'
  if [[ "$ENTITLEMENTS_COMPACT" =~ $GET_TASK_ALLOW_PATTERN ]]; then
    fail "get-task-allow must be absent or false for every executable and bundle"
  fi

  if [[ "$ALLOW_AD_HOC" == "1" ]]; then
    grep -Fq "Signature=adhoc" <<<"$SIGNING_DETAILS" \
      || grep -Fq "Authority=Developer ID Application:" <<<"$SIGNING_DETAILS" \
      || fail "candidate code has neither an ad-hoc nor Developer ID Application signature"
  else
    grep -Fq "Authority=Developer ID Application:" <<<"$SIGNING_DETAILS" \
      || fail "Developer ID Application signature is required for every executable and bundle"
    grep -Eq 'flags=.*runtime' <<<"$SIGNING_DETAILS" \
      || fail "hardened runtime is required for every executable and bundle"
    grep -Fq "Timestamp=" <<<"$SIGNING_DETAILS" \
      || fail "a secure signing timestamp is required for every executable and bundle"

    TEAM_ID="$(
      sed -nE 's/^TeamIdentifier=(.+)$/\1/p' <<<"$SIGNING_DETAILS" | head -n 1
    )"
    [[ -n "$TEAM_ID" && "$TEAM_ID" != "not set" ]] \
      || fail "Developer ID TeamIdentifier is missing"
    if [[ -z "$EXPECTED_TEAM_ID" ]]; then
      EXPECTED_TEAM_ID="$TEAM_ID"
    elif [[ "$TEAM_ID" != "$EXPECTED_TEAM_ID" ]]; then
      fail "nested code and the app bundle use different Developer ID teams"
    fi
  fi
done

for executable in "$APP_BINARY" "$SERVICE_BINARY"; do
  if grep -Fq "$ROOT_DIR" < <(strings "$executable"); then
    fail "release executable contains the repository path"
  fi
  if [[ -n "${HOME:-}" ]] && grep -Fq "$HOME" < <(strings "$executable"); then
    fail "release executable contains a maintainer home path"
  fi
done

if grep -R -I -Fq "$ROOT_DIR" "$APP_BUNDLE"; then
  fail "app resources contain the repository path"
fi
if [[ -n "${HOME:-}" ]] && grep -R -I -Fq "$HOME" "$APP_BUNDLE"; then
  fail "app resources contain a maintainer home path"
fi

UNSAFE_PAYLOAD="$(
  find "$APP_BUNDLE" -type f \
    \( -name '*.log' \
      -o -name '*.trace' \
      -o -name '*.sqlite' \
      -o -name '*.sqlite3' \
      -o -name '*.db' \
      -o -name '*.pem' \
      -o -name '*.key' \
      -o -name '*.p12' \
      -o -name '*.mobileprovision' \
    \) \
    -print -quit
)"
[[ -z "$UNSAFE_PAYLOAD" ]] || fail "app bundle contains a prohibited release payload"

if [[ "$REQUIRE_NOTARIZATION" == "1" ]]; then
  command -v xcrun >/dev/null 2>&1 || fail "xcrun is required for notarization checks"
  command -v spctl >/dev/null 2>&1 || fail "spctl is required for Gatekeeper checks"
  xcrun stapler validate "$APP_BUNDLE"
  spctl --assess --type execute --verbose=4 "$APP_BUNDLE"
fi

if [[ "$ALLOW_AD_HOC" == "1" ]]; then
  QUALIFICATION="local-only"
elif [[ "$REQUIRE_NOTARIZATION" == "1" ]]; then
  QUALIFICATION="notarized"
else
  QUALIFICATION="developer-id-signed"
fi

echo "distribution verification: ok ($QUALIFICATION, version $APP_VERSION, architectures $APP_ARCHES)"
