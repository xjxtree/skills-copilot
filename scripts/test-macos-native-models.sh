#!/usr/bin/env bash
set -euo pipefail

INHERITED_APP_DATA_SENTINEL="${SKILLS_COPILOT_NATIVE_MODEL_INHERITED_APP_DATA_SENTINEL:-}"
INHERITED_APP_DATA_SENTINEL_CONTENT=""
if [[ -n "${INHERITED_APP_DATA_SENTINEL}" ]]; then
  if [[ ! -f "${INHERITED_APP_DATA_SENTINEL}" ]]; then
    echo "native-model-runtime-isolation: inherited app-data sentinel is missing before the run" >&2
    exit 1
  fi
  INHERITED_APP_DATA_SENTINEL_CONTENT="$(<"${INHERITED_APP_DATA_SENTINEL}")"
fi

TEMP_ROOT="${TMPDIR:-/tmp}"
RUNTIME_ROOT="$(mktemp -d "${TEMP_ROOT%/}/skills-copilot-native-model-tests.XXXXXX")"
cleanup_runtime_root() {
  if [[ -n "${RUNTIME_ROOT:-}" && -d "${RUNTIME_ROOT}" ]]; then
    rm -rf -- "${RUNTIME_ROOT}"
  fi
}
trap cleanup_runtime_root EXIT

RUNTIME_HOME="${RUNTIME_ROOT}/home"
RUNTIME_APP_DATA_DIR="${RUNTIME_ROOT}/app-data"
SIMULATED_INHERITED_APP_DATA_DIR="${RUNTIME_ROOT}/inherited-app-data"
SIMULATED_INHERITED_SENTINEL="${SIMULATED_INHERITED_APP_DATA_DIR}/task-preflight-history.json"
SIMULATED_INHERITED_SENTINEL_CONTENT="NATIVE_MODEL_INHERITED_SENTINEL_73"
mkdir -p \
  "${RUNTIME_HOME}" \
  "${RUNTIME_APP_DATA_DIR}" \
  "${SIMULATED_INHERITED_APP_DATA_DIR}"
printf '%s' "${SIMULATED_INHERITED_SENTINEL_CONTENT}" > "${SIMULATED_INHERITED_SENTINEL}"

run_native_model_suite() {
  env \
    HOME="${RUNTIME_HOME}" \
    CFFIXED_USER_HOME="${RUNTIME_HOME}" \
    SKILLS_COPILOT_APP_DATA_DIR="${RUNTIME_APP_DATA_DIR}" \
    SKILLS_COPILOT_NATIVE_MODEL_ISOLATED=1 \
    SKILLS_COPILOT_NATIVE_REAL_SERVICE_PATH="${REAL_SERVICE_PATH}" \
    "$@"
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BUILD_ROOT="${RUNTIME_ROOT}/build"
PACKAGE_DIR="${BUILD_ROOT}/package"
TARGET_DIR="${PACKAGE_DIR}/Sources/SkillsCopilotNativeModelTests"
REAL_SERVICE_TARGET_DIR="${CARGO_TARGET_DIR:-${BUILD_ROOT}/cargo}"
REAL_SERVICE_PATH="${REAL_SERVICE_TARGET_DIR}/debug/skills-copilot-service"

cargo build \
  --package skills-copilot-service \
  --bin skills-copilot-service \
  --target-dir "${REAL_SERVICE_TARGET_DIR}"

if [[ ! -x "${REAL_SERVICE_PATH}" ]]; then
  echo "native-model-real-sidecar: built service is missing or not executable" >&2
  exit 1
fi

rm -rf "${PACKAGE_DIR}"
mkdir -p "${TARGET_DIR}"
mkdir -p "${BUILD_ROOT}/fixtures"
cp -R "${REPO_ROOT}/fixtures/service-protocol" "${BUILD_ROOT}/fixtures/"

rsync -a \
  --exclude='Views/**' \
  --exclude='App/**' \
  --exclude='Support/AgentIconProvider.swift' \
  --include='*/' \
  --include='*.swift' \
  --exclude='*' \
  "${REPO_ROOT}/apps/macos/Sources/SkillsCopilot/" \
  "${TARGET_DIR}/"

cp -R "${REPO_ROOT}/apps/macos/Sources/SkillsCopilot/Resources" "${TARGET_DIR}/Resources"

mkdir -p "${TARGET_DIR}/Tests"
rsync -a \
  --exclude='FullNativeModelSuiteTests.swift' \
  --include='*.swift' \
  --exclude='*' \
  "${REPO_ROOT}/apps/macos/Tests/SkillsCopilotTests/" \
  "${TARGET_DIR}/Tests/"

find "${TARGET_DIR}/Tests" -name '*.swift' -print0 \
  | xargs -0 perl -0pi -e 's/^\@testable import SkillsCopilot\n//mg'

# The disposable executable calls the model suites directly from main.swift.
# Remove the XCTest-only bridge wrappers copied from the real SwiftPM test
# target so this standalone runner does not acquire an Xcode-toolchain runtime
# dependency such as libXCTestSwiftSupport.dylib.
find "${TARGET_DIR}/Tests" -name '*.swift' -print0 \
  | xargs -0 perl -0pi -e 's/^#if canImport\(XCTest\)\R.*?^#endif\R?//gms'

if grep -R -n -E '^(import XCTest|#if canImport\(XCTest\))' "${TARGET_DIR}/Tests" >/dev/null; then
  grep -R -n -E '^(import XCTest|#if canImport\(XCTest\))' "${TARGET_DIR}/Tests"
  echo "Standalone native model tests must not link XCTest." >&2
  exit 1
fi

if grep -R -n -E '^import (AppKit|SwiftUI)$' "${TARGET_DIR}" >/dev/null; then
  grep -R -n -E '^import (AppKit|SwiftUI)$' "${TARGET_DIR}"
  echo "Native model tests must not link AppKit or SwiftUI." >&2
  exit 1
fi

cat > "${PACKAGE_DIR}/Package.swift" <<'SWIFT'
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "SkillsCopilotNativeModelTests",
    defaultLocalization: "en",
    platforms: [.macOS(.v13)],
    products: [
        .executable(
            name: "SkillsCopilotNativeModelTests",
            targets: ["SkillsCopilotNativeModelTests"]
        )
    ],
    targets: [
        .executableTarget(
            name: "SkillsCopilotNativeModelTests",
            path: "Sources/SkillsCopilotNativeModelTests",
            resources: [.process("Resources")]
        )
    ]
)
SWIFT

cat > "${TARGET_DIR}/main.swift" <<'SWIFT'
import Foundation

do {
    let summary = try await runAllNativeModelTestsAsync()
    try expectEqual(summary.serviceSuiteCount, 2, "Service suite count")
    try expectEqual(summary.mainSuiteCount, 34, "Main suite count")
    try expectEqual(summary.skillStoreGroupCount, 64, "SkillStore group count")
    try expectEqual(summary.namedExecutionCount, 100, "Named execution count")
} catch {
    fputs("SkillsCopilotTests: \(error)\n", stderr)
    exit(1)
}
SWIFT

cd "${REPO_ROOT}"
export MallocNanoZone=0

swift build \
  --package-path "${PACKAGE_DIR}" \
  --scratch-path "${BUILD_ROOT}/swiftpm"

BINARY_DIR="$(swift build \
  --package-path "${PACKAGE_DIR}" \
  --scratch-path "${BUILD_ROOT}/swiftpm" \
  --show-bin-path)"

if otool -L "${BINARY_DIR}/SkillsCopilotNativeModelTests" \
  | grep -F 'libXCTestSwiftSupport.dylib' >/dev/null; then
  otool -L "${BINARY_DIR}/SkillsCopilotNativeModelTests"
  echo "Standalone native model tests unexpectedly link the XCTest Swift runtime." >&2
  exit 1
fi

# Keep a disposable inherited value in the shell environment. Every runner must
# override it through run_native_model_suite or this sentinel will be purged.
export SKILLS_COPILOT_APP_DATA_DIR="${SIMULATED_INHERITED_APP_DATA_DIR}"

run_native_model_suite "${BINARY_DIR}/SkillsCopilotNativeModelTests"

if [[ ! -f "${SIMULATED_INHERITED_SENTINEL}" ]] \
  || [[ "$(<"${SIMULATED_INHERITED_SENTINEL}")" != "${SIMULATED_INHERITED_SENTINEL_CONTENT}" ]]; then
  echo "native-model-runtime-isolation: simulated inherited app-data sentinel changed during the run" >&2
  exit 1
fi
echo "native-model-runtime-isolation: simulated inherited app-data sentinel preserved"

if [[ -n "${INHERITED_APP_DATA_SENTINEL}" ]]; then
  if [[ ! -f "${INHERITED_APP_DATA_SENTINEL}" ]] \
    || [[ "$(<"${INHERITED_APP_DATA_SENTINEL}")" != "${INHERITED_APP_DATA_SENTINEL_CONTENT}" ]]; then
    echo "native-model-runtime-isolation: inherited app-data sentinel changed during the run" >&2
    exit 1
  fi
  echo "native-model-runtime-isolation: inherited app-data sentinel preserved"
fi
