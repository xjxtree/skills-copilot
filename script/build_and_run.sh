#!/usr/bin/env bash
set -euo pipefail

MODE="run"
TARGET_ARCH="${AGENT_COPILOT_ARCH:-}"
BUILD_CONFIGURATION="${AGENT_COPILOT_BUILD_CONFIGURATION:-debug}"
APP_NAME="AgentCopilot"
BUNDLE_ID="dev.agent-copilot.native"
LEGACY_APP_NAME="SkillsCopilot"
LEGACY_BUNDLE_ID="dev.skills-copilot.native"
SWIFT_PRODUCT_NAME="SkillsCopilot"
MIN_SYSTEM_VERSION="13.0"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/script/path_identity.sh"
APP_VERSION="$(awk -F'"' '/^version = / {print $2; exit}' "$ROOT_DIR/crates/service/Cargo.toml")"
MACOS_DIR="$ROOT_DIR/apps/macos"
DIST_DIR="$ROOT_DIR/dist"
APP_BUNDLE="$DIST_DIR/$APP_NAME.app"
APP_CONTENTS="$APP_BUNDLE/Contents"
APP_MACOS="$APP_CONTENTS/MacOS"
APP_RESOURCES="$APP_CONTENTS/Resources"
APP_BINARY="$APP_MACOS/$APP_NAME"
SERVICE_BINARY="$APP_RESOURCES/skills-copilot-service"
INFO_PLIST="$APP_CONTENTS/Info.plist"
ICON_SOURCE="$MACOS_DIR/Sources/SkillsCopilot/Resources/AppIcon.icns"
ICON_TARGET="$APP_RESOURCES/AppIcon.icns"
SWIFT_RESOURCES="$MACOS_DIR/Sources/SkillsCopilot/Resources"
LAUNCHED_PID=""
CARGO_TARGET_ROOT="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"
CARGO_BIN="${CARGO:-cargo}"
CARGO_ENV=()
if command -v rustup >/dev/null 2>&1; then
  if [[ -z "${CARGO:-}" ]]; then
    CARGO_BIN="$(rustup which cargo)"
  fi
  if [[ -z "${RUSTC:-}" ]]; then
    CARGO_ENV+=(RUSTC="$(rustup which rustc)")
  fi
fi

usage() {
  cat >&2 <<USAGE
usage: $0 [run|--debug|--logs|--telemetry|--verify|--build-only] [--arch arm64|x86_64] [--configuration debug|release]

Builds dist/$APP_NAME.app before running the selected mode.
Use "pnpm build:macos" for a build that neither launches nor stops the app.
Use "pnpm verify:macos-launch" only for interactive launch/window proof.
Set AGENT_COPILOT_ARCH or pass --arch to cross-build architecture-specific bundles.
Set AGENT_COPILOT_BUILD_CONFIGURATION or pass --configuration for debug or release builds.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --arch)
      if [[ $# -lt 2 ]]; then
        echo "--arch requires arm64 or x86_64" >&2
        usage
        exit 2
      fi
      TARGET_ARCH="$2"
      shift 2
      ;;
    --arch=*)
      TARGET_ARCH="${1#--arch=}"
      shift
      ;;
    --configuration)
      if [[ $# -lt 2 ]]; then
        echo "--configuration requires debug or release" >&2
        usage
        exit 2
      fi
      BUILD_CONFIGURATION="$2"
      shift 2
      ;;
    --configuration=*)
      BUILD_CONFIGURATION="${1#--configuration=}"
      shift
      ;;
    run|--debug|debug|--logs|logs|--telemetry|telemetry|--verify|verify|--build-only|build-only)
      MODE="$1"
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

RUST_TARGET=""
SWIFT_TRIPLE=""
case "$TARGET_ARCH" in
  "")
    ;;
  arm64|aarch64)
    TARGET_ARCH="arm64"
    RUST_TARGET="aarch64-apple-darwin"
    SWIFT_TRIPLE="arm64-apple-macosx$MIN_SYSTEM_VERSION"
    ;;
  x86_64|x64|amd64|intel)
    TARGET_ARCH="x86_64"
    RUST_TARGET="x86_64-apple-darwin"
    SWIFT_TRIPLE="x86_64-apple-macosx$MIN_SYSTEM_VERSION"
    ;;
  *)
    echo "unsupported architecture: $TARGET_ARCH" >&2
    usage
    exit 2
    ;;
esac

case "$BUILD_CONFIGURATION" in
  debug|release)
    ;;
  *)
    echo "unsupported build configuration: $BUILD_CONFIGURATION" >&2
    usage
    exit 2
    ;;
esac

SWIFT_BUILD_ARGS=(--package-path "$MACOS_DIR")
SWIFT_BUILD_ARGS+=(-Xswiftc -DAGENT_COPILOT_APP_BUNDLE)
if [[ -n "${SWIFTPM_SCRATCH_PATH:-}" ]]; then
  SWIFT_BUILD_ARGS+=(--scratch-path "$SWIFTPM_SCRATCH_PATH")
fi
if [[ -n "$SWIFT_TRIPLE" ]]; then
  SWIFT_BUILD_ARGS+=(--triple "$SWIFT_TRIPLE")
fi
SWIFT_BUILD_ARGS+=(-c "$BUILD_CONFIGURATION")

CARGO_BUILD_ARGS=(-p skills-copilot-service)
if [[ -n "$RUST_TARGET" ]]; then
  if command -v rustup >/dev/null 2>&1 && ! rustup target list --installed | grep -qx "$RUST_TARGET"; then
    echo "missing Rust target $RUST_TARGET; run: rustup target add $RUST_TARGET" >&2
    exit 1
  fi
  CARGO_BUILD_ARGS+=(--target "$RUST_TARGET")
fi
if [[ "$BUILD_CONFIGURATION" == "release" ]]; then
  CARGO_BUILD_ARGS+=(--release)
  RELEASE_RUSTFLAGS="${RUSTFLAGS:-}"
  RELEASE_RUSTFLAGS="${RELEASE_RUSTFLAGS:+$RELEASE_RUSTFLAGS }--remap-path-prefix=$ROOT_DIR=/agent-copilot-source"
  if [[ -n "${HOME:-}" ]]; then
    RELEASE_RUSTFLAGS="$RELEASE_RUSTFLAGS --remap-path-prefix=$HOME=/agent-copilot-home"
  fi
  CARGO_ENV+=(RUSTFLAGS="$RELEASE_RUSTFLAGS")
fi

canonical_app_bundle() {
  if [[ -d "$APP_BUNDLE" ]]; then
    (cd "$APP_BUNDLE" && pwd -P)
  else
    printf '%s\n' "$APP_BUNDLE"
  fi
}

list_running_app_instances() {
  swift -Xfrontend -disable-availability-checking -e '
import AppKit
import Foundation

let args = Array(CommandLine.arguments.dropFirst())
let bundleId = args.indices.contains(0) ? args[0] : ""
let appName = args.indices.contains(1) ? args[1] : ""
let legacyBundleId = args.indices.contains(2) ? args[2] : ""
let legacyAppName = args.indices.contains(3) ? args[3] : ""

for app in NSWorkspace.shared.runningApplications {
    let identifierMatches = app.bundleIdentifier == bundleId || app.bundleIdentifier == legacyBundleId
    let nameMatches = app.localizedName == appName || app.localizedName == legacyAppName
    guard identifierMatches || nameMatches else { continue }
    let bundlePath = app.bundleURL?.resolvingSymlinksInPath().standardizedFileURL.path ?? ""
    let bundlePathBase64 = Data(bundlePath.utf8).base64EncodedString()
    print("\(app.processIdentifier)\t\(bundlePathBase64)")
}
' "$BUNDLE_ID" "$APP_NAME" "$LEGACY_BUNDLE_ID" "$LEGACY_APP_NAME"
}

wait_for_no_running_app_instances() {
  local quiet="${1:-}"
  local deadline=$((SECONDS + 5))
  while (( SECONDS < deadline )); do
    if [[ -z "$(list_running_app_instances || true)" ]]; then
      return 0
    fi
    sleep 0.25
  done
  if [[ "$quiet" != "--quiet" ]]; then
    echo "stale-bundle: timed out waiting for existing $APP_NAME instances to exit" >&2
  fi
  return 1
}

terminate_existing_app_instances() {
  local rows
  rows="$(list_running_app_instances || true)"
  if [[ -z "$rows" ]]; then
    return 0
  fi
  local target_bundle
  target_bundle="$(canonical_app_bundle)"
  local pid bundle_path_base64 bundle_path
  while IFS=$'\t' read -r pid bundle_path_base64; do
    [[ -z "$pid" ]] && continue
    if ! decode_base64_path "$bundle_path_base64"; then
      echo "tool-layer-unknown: invalid bundle path encoding for $APP_NAME pid $pid" >&2
      return 1
    fi
    bundle_path="$DECODED_BASE64_PATH"
    if [[ -n "$bundle_path" ]] && ! same_filesystem_entry "$bundle_path" "$target_bundle"; then
      echo "Stopping stale same-bundle $APP_NAME pid $pid from $bundle_path (target $target_bundle)." >&2
    fi
    kill "$pid" >/dev/null 2>&1 || true
  done <<<"$rows"
  if ! wait_for_no_running_app_instances --quiet; then
    rows="$(list_running_app_instances || true)"
    while IFS=$'\t' read -r pid _bundle_path_base64; do
      [[ -n "$pid" ]] && kill -9 "$pid" >/dev/null 2>&1 || true
    done <<<"$rows"
    wait_for_no_running_app_instances
  fi
}

wait_for_current_bundle_process() {
  local deadline=$((SECONDS + 10))
  local target_bundle
  target_bundle="$(canonical_app_bundle)"
  local pid bundle_path_base64 bundle_path
  while (( SECONDS < deadline )); do
    local rows exact_pids
    rows="$(list_running_app_instances || true)"
    exact_pids=""
    if [[ -n "$rows" ]]; then
      while IFS=$'\t' read -r pid bundle_path_base64; do
        [[ -z "$pid" ]] && continue
        if ! decode_base64_path "$bundle_path_base64"; then
          echo "tool-layer-unknown: invalid bundle path encoding for $APP_NAME pid $pid" >&2
          return 1
        fi
        bundle_path="$DECODED_BASE64_PATH"
        if same_filesystem_entry "$bundle_path" "$target_bundle"; then
          exact_pids+="${pid}"$'\n'
        fi
      done <<<"$rows"
    fi
    local exact_count
    exact_count="$(printf '%s' "$exact_pids" | sed '/^$/d' | wc -l | tr -d ' ')"
    if [[ "$exact_count" == "1" ]]; then
      printf '%s\n' "$exact_pids" | sed '/^$/d' | head -n 1
      return 0
    fi
    if [[ "$exact_count" != "0" ]]; then
      echo "activation-failed: duplicate current bundle processes for $target_bundle: $(printf '%s' "$exact_pids" | tr '\n' ' ')" >&2
      return 1
    fi
    sleep 0.25
  done
  local rows stale_rows
  rows="$(list_running_app_instances || true)"
  stale_rows=""
  if [[ -n "$rows" ]]; then
    while IFS=$'\t' read -r pid bundle_path_base64; do
      [[ -z "$pid" ]] && continue
      if ! decode_base64_path "$bundle_path_base64"; then
        echo "tool-layer-unknown: invalid bundle path encoding for $APP_NAME pid $pid" >&2
        return 1
      fi
      bundle_path="$DECODED_BASE64_PATH"
      if ! same_filesystem_entry "$bundle_path" "$target_bundle"; then
        stale_rows+="${pid} ${bundle_path}"$'\n'
      fi
    done <<<"$rows"
  fi
  if [[ -n "$stale_rows" ]]; then
    echo "stale-bundle: running $APP_NAME instances are from different bundle path than target $target_bundle: $(printf '%s' "$stale_rows" | tr '\n' '; ')" >&2
  else
    echo "activation-failed: timed out waiting for $APP_NAME to launch from $target_bundle" >&2
  fi
  return 1
}

activate_current_app() {
  local pid="$1"
  swift -Xfrontend -disable-availability-checking -e '
import AppKit
import Foundation

let rawPid = CommandLine.arguments.dropFirst().first ?? ""
guard let pid = Int32(rawPid),
      let app = NSRunningApplication(processIdentifier: pid_t(pid)) else {
    fputs("activation-failed: unable to resolve running app pid \(rawPid).\n", stderr)
    exit(2)
}

let deadline = Date().addingTimeInterval(5)
while Date() < deadline {
    if app.isActive || app.activate(options: [.activateAllWindows, .activateIgnoringOtherApps]) {
        exit(0)
    }
    Thread.sleep(forTimeInterval: 0.25)
}
fputs("activation-failed: failed to activate \(app.localizedName ?? "target app") pid \(pid).\n", stderr)
exit(3)
' "$pid"
}

wait_for_visible_window() {
  local pid="$1"
  local deadline=$((SECONDS + 10))
  local output status
  while (( SECONDS < deadline )); do
    if output="$(swift -Xfrontend -disable-availability-checking -e '
import AppKit
import CoreGraphics
import Foundation

let args = Array(CommandLine.arguments.dropFirst())
let rawPid = args.indices.contains(0) ? args[0] : ""
let appName = args.indices.contains(1) ? args[1] : "AgentCopilot"
guard let expectedPid = Int32(rawPid) else {
    fputs("window-not-found: invalid app pid \(rawPid).\n", stderr)
    exit(2)
}

if let session = CGSessionCopyCurrentDictionary() as? [String: Any],
   let locked = session["CGSSessionScreenIsLocked"] as? Bool,
   locked {
    fputs("locked-session: macOS session is locked; refusing UI evidence.\n", stderr)
    exit(6)
}

guard let windows = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as? [[String: Any]] else {
    fputs("tool-layer-unknown: unable to read window list.\n", stderr)
    exit(3)
}

let matches = windows.compactMap { window -> UInt32? in
    guard let layer = window[kCGWindowLayer as String] as? Int, layer == 0 else { return nil }
    guard let pid = window[kCGWindowOwnerPID as String] as? Int32, pid == expectedPid else { return nil }
    guard let bounds = window[kCGWindowBounds as String] as? [String: Any],
          let width = bounds["Width"] as? Double,
          let height = bounds["Height"] as? Double,
          width > 0,
          height > 0 else {
        return nil
    }
    return window[kCGWindowNumber as String] as? UInt32
}

if matches.isEmpty {
    fputs("window-not-found: No visible \(appName) app window found for pid \(expectedPid).\n", stderr)
    exit(1)
}
if matches.count > 1 {
    let ids = matches.map(String.init).joined(separator: ",")
    fputs("window-not-found: multiple visible \(appName) windows create window ambiguity for pid \(expectedPid): \(ids)\n", stderr)
    exit(1)
}
print(matches[0])
' "$pid" "$APP_NAME" 2>&1)"; then
      status=0
      printf '%s\n' "$output"
      return 0
    else
      status=$?
    fi
    if [[ "$output" == locked-session:* || "$output" == tool-layer-unknown:* || "$output" == *"multiple visible"* ]]; then
      echo "$output" >&2
      return "$status"
    fi
    sleep 0.25
  done
  echo "${output:-window-not-found: timed out waiting for visible $APP_NAME window for pid $pid}" >&2
  return 1
}

ad_hoc_sign_app_bundle() {
  if ! command -v codesign >/dev/null 2>&1; then
    echo "codesign is required to build $APP_NAME.app" >&2
    exit 1
  fi

  codesign --force --sign - "$SERVICE_BINARY"
  codesign --force --sign - "$APP_BINARY"
  codesign --force --sign - "$APP_BUNDLE"
  codesign --verify --deep --strict --verbose=2 "$APP_BUNDLE"
}

case "$MODE" in
  --build-only|build-only)
    ;;
  *)
    terminate_existing_app_instances
    ;;
esac

env "${CARGO_ENV[@]}" "$CARGO_BIN" build "${CARGO_BUILD_ARGS[@]}"
swift build "${SWIFT_BUILD_ARGS[@]}"

SWIFT_BIN_DIR="$(swift build "${SWIFT_BUILD_ARGS[@]}" --show-bin-path)"
if [[ -n "$RUST_TARGET" ]]; then
  RUST_SERVICE="$CARGO_TARGET_ROOT/$RUST_TARGET/$BUILD_CONFIGURATION/skills-copilot-service"
else
  RUST_SERVICE="$CARGO_TARGET_ROOT/$BUILD_CONFIGURATION/skills-copilot-service"
fi
rm -rf "$APP_BUNDLE"
mkdir -p "$APP_MACOS" "$APP_RESOURCES"
cp "$SWIFT_BIN_DIR/$SWIFT_PRODUCT_NAME" "$APP_BINARY"
cp "$RUST_SERVICE" "$SERVICE_BINARY"
if [[ -d "$SWIFT_RESOURCES" ]]; then
  cp -R "$SWIFT_RESOURCES"/. "$APP_RESOURCES"/
fi
if [[ ! -f "$ICON_SOURCE" ]]; then
  echo "missing native app icon: $ICON_SOURCE" >&2
  exit 1
fi
cp "$ICON_SOURCE" "$ICON_TARGET"
chmod +x "$APP_BINARY" "$SERVICE_BINARY"
if [[ "$BUILD_CONFIGURATION" == "release" ]]; then
  if ! command -v strip >/dev/null 2>&1; then
    echo "strip is required for release builds" >&2
    exit 1
  fi
  strip -S -x "$APP_BINARY" "$SERVICE_BINARY"
fi

cat >"$INFO_PLIST" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>$APP_NAME</string>
  <key>CFBundleIdentifier</key>
  <string>$BUNDLE_ID</string>
  <key>CFBundleName</key>
  <string>$APP_NAME</string>
  <key>CFBundleIconFile</key>
  <string>AppIcon</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>$APP_VERSION</string>
  <key>CFBundleVersion</key>
  <string>$APP_VERSION</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.developer-tools</string>
  <key>LSMinimumSystemVersion</key>
  <string>$MIN_SYSTEM_VERSION</string>
  <key>NSPrincipalClass</key>
  <string>NSApplication</string>
</dict>
</plist>
PLIST

ad_hoc_sign_app_bundle

LAUNCH_ENV_VARS=(
  SKILLS_COPILOT_HOME
  SKILLS_COPILOT_APP_DATA_DIR
  SKILLS_COPILOT_SERVICE_PATH
)

set_launch_env() {
  for name in "${LAUNCH_ENV_VARS[@]}"; do
    if [[ -n "${!name:-}" ]]; then
      /bin/launchctl setenv "$name" "${!name}"
    fi
  done
}

clear_launch_env() {
  for name in "${LAUNCH_ENV_VARS[@]}"; do
    /bin/launchctl unsetenv "$name" >/dev/null 2>&1 || true
  done
}

open_app() {
  trap clear_launch_env EXIT
  clear_launch_env
  set_launch_env
  /usr/bin/open -n "$APP_BUNDLE"
  LAUNCHED_PID="$(wait_for_current_bundle_process)"
  activate_current_app "$LAUNCHED_PID"
  clear_launch_env
  trap - EXIT
}

case "$MODE" in
  --build-only|build-only)
    ;;
  run)
    open_app
    ;;
  --debug|debug)
    lldb -- "$APP_BINARY"
    ;;
  --logs|logs)
    open_app
    /usr/bin/log stream --info --style compact --predicate "process == \"$APP_NAME\""
    ;;
  --telemetry|telemetry)
    open_app
    /usr/bin/log stream --info --style compact --predicate "subsystem == \"$BUNDLE_ID\""
    ;;
  --verify|verify)
    open_app
    wait_for_visible_window "$LAUNCHED_PID" >/dev/null
    ;;
  *)
    usage
    exit 2
    ;;
esac
