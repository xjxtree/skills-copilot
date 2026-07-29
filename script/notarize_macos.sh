#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_BUNDLE="$ROOT_DIR/dist/AgentCopilot.app"
KEYCHAIN_PROFILE=""
OUTPUT_ZIP=""
TEMP_DIR=""
TEMP_BASE="${TMPDIR:-/tmp}"
TEMP_BASE="${TEMP_BASE%/}"

usage() {
  cat >&2 <<USAGE
usage: $0 --keychain-profile name [--app path] [--output-zip path]

Submits an already Developer ID-signed AgentCopilot app to Apple's notary
service, waits for acceptance, staples the ticket, and verifies Gatekeeper.

The profile must already exist in Keychain through notarytool. This script does
not accept Apple IDs, passwords, API keys, or issuer credentials. The optional
ZIP is created only after stapling and is never uploaded or published by this
repository.
USAGE
}

fail() {
  echo "notarization failed: $*" >&2
  exit 1
}

cleanup() {
  if [[ -n "$TEMP_DIR" && -d "$TEMP_DIR" ]]; then
    case "$TEMP_DIR" in
      "$TEMP_BASE"/agent-copilot-notary.*)
        rm -rf -- "$TEMP_DIR"
        ;;
    esac
  fi
}

trap cleanup EXIT

if [[ "${1:-}" == "--" ]]; then
  shift
fi

while [[ $# -gt 0 ]]; do
  case "$1" in
    --keychain-profile)
      if [[ $# -lt 2 || -z "$2" ]]; then
        echo "--keychain-profile requires a profile name" >&2
        usage
        exit 2
      fi
      KEYCHAIN_PROFILE="$2"
      shift 2
      ;;
    --keychain-profile=*)
      KEYCHAIN_PROFILE="${1#--keychain-profile=}"
      if [[ -z "$KEYCHAIN_PROFILE" ]]; then
        echo "--keychain-profile requires a profile name" >&2
        usage
        exit 2
      fi
      shift
      ;;
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
    --output-zip)
      if [[ $# -lt 2 || -z "$2" ]]; then
        echo "--output-zip requires a path" >&2
        usage
        exit 2
      fi
      OUTPUT_ZIP="$2"
      shift 2
      ;;
    --output-zip=*)
      OUTPUT_ZIP="${1#--output-zip=}"
      if [[ -z "$OUTPUT_ZIP" ]]; then
        echo "--output-zip requires a path" >&2
        usage
        exit 2
      fi
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

[[ -n "$KEYCHAIN_PROFILE" ]] || {
  echo "--keychain-profile is required" >&2
  usage
  exit 2
}

for command in ditto plutil shasum xcrun; do
  command -v "$command" >/dev/null 2>&1 || fail "required command is unavailable: $command"
done

if [[ -n "$OUTPUT_ZIP" ]]; then
  [[ "$OUTPUT_ZIP" == *.zip ]] || fail "--output-zip must end in .zip"
  [[ ! -e "$OUTPUT_ZIP" ]] || fail "refusing to overwrite an existing output ZIP"
  OUTPUT_PARENT="$(dirname "$OUTPUT_ZIP")"
  [[ -d "$OUTPUT_PARENT" ]] || fail "output ZIP parent directory does not exist"
fi

"$ROOT_DIR/script/verify_macos_distribution.sh" --app "$APP_BUNDLE"

TEMP_DIR="$(mktemp -d "$TEMP_BASE/agent-copilot-notary.XXXXXX")"
SUBMISSION_ZIP="$TEMP_DIR/AgentCopilot-notary-submission.zip"
NOTARY_RESULT="$TEMP_DIR/notary-result.json"
NOTARY_LOG="$TEMP_DIR/notary-log.json"

ditto -c -k --keepParent --sequesterRsrc "$APP_BUNDLE" "$SUBMISSION_ZIP"
xcrun notarytool submit "$SUBMISSION_ZIP" \
  --keychain-profile "$KEYCHAIN_PROFILE" \
  --wait \
  --output-format json >"$NOTARY_RESULT"

NOTARY_STATUS="$(plutil -extract status raw -o - "$NOTARY_RESULT")"
NOTARY_ID="$(plutil -extract id raw -o - "$NOTARY_RESULT")"
if [[ "$NOTARY_STATUS" != "Accepted" ]]; then
  echo "notarization request $NOTARY_ID finished with status $NOTARY_STATUS" >&2
  echo "inspect it with: xcrun notarytool log <request-id> --keychain-profile <profile>" >&2
  exit 1
fi

xcrun notarytool log "$NOTARY_ID" \
  --keychain-profile "$KEYCHAIN_PROFILE" \
  "$NOTARY_LOG" >/dev/null
NOTARY_ISSUES="$(plutil -extract issues json -o - "$NOTARY_LOG" 2>/dev/null || true)"
case "$NOTARY_ISSUES" in
  ""|"null"|"[]")
    ;;
  *)
    echo "notarization request $NOTARY_ID was accepted but its log contains issues" >&2
    echo "inspect it with: xcrun notarytool log <request-id> --keychain-profile <profile>" >&2
    exit 1
    ;;
esac

xcrun stapler staple "$APP_BUNDLE"
"$ROOT_DIR/script/verify_macos_distribution.sh" \
  --app "$APP_BUNDLE" \
  --require-notarization

if [[ -n "$OUTPUT_ZIP" ]]; then
  ditto -c -k --keepParent --sequesterRsrc "$APP_BUNDLE" "$OUTPUT_ZIP"
  shasum -a 256 "$OUTPUT_ZIP"
fi

echo "notarization: accepted and stapled (request $NOTARY_ID)"
