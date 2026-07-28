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
    "$@"
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
source "${SCRIPT_DIR}/lib/swift-testing.sh"
configure_swift_testing_args
BUILD_ROOT="${RUNTIME_ROOT}/build"
PACKAGE_DIR="${BUILD_ROOT}/package"
SOURCE_DIR="${PACKAGE_DIR}/Sources/SkillsCopilot"
TEST_DIR="${PACKAGE_DIR}/Tests/SkillsCopilotTests"

rm -rf "${PACKAGE_DIR}"
mkdir -p "${SOURCE_DIR}" "${TEST_DIR}"

rsync -a \
  --exclude='Views/**' \
  --exclude='App/**' \
  --exclude='Support/AgentIconProvider.swift' \
  --include='*/' \
  --include='*.swift' \
  --exclude='*' \
  "${REPO_ROOT}/apps/macos/Sources/SkillsCopilot/" \
  "${SOURCE_DIR}/"

cp -R "${REPO_ROOT}/apps/macos/Sources/SkillsCopilot/Resources" "${SOURCE_DIR}/Resources"

rsync -a \
  --exclude='NativeUILayoutTests.swift' \
  --include='*.swift' \
  --exclude='*' \
  "${REPO_ROOT}/apps/macos/Tests/SkillsCopilotTests/" \
  "${TEST_DIR}/"

if grep -R -n -E '^import (AppKit|SwiftUI)$' "${SOURCE_DIR}" "${TEST_DIR}" >/dev/null; then
  grep -R -n -E '^import (AppKit|SwiftUI)$' "${SOURCE_DIR}" "${TEST_DIR}"
  echo "Native model tests must not link AppKit or SwiftUI." >&2
  exit 1
fi

cat > "${PACKAGE_DIR}/Package.swift" <<'SWIFT'
// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "SkillsCopilotNativeModelTests",
    defaultLocalization: "en",
    platforms: [.macOS(.v13)],
    targets: [
        .target(
            name: "SkillsCopilot",
            path: "Sources/SkillsCopilot",
            resources: [.process("Resources")]
        ),
        .testTarget(
            name: "SkillsCopilotTests",
            dependencies: ["SkillsCopilot"],
            path: "Tests/SkillsCopilotTests"
        )
    ],
    swiftLanguageModes: [.v5]
)
SWIFT

cd "${REPO_ROOT}"
export MallocNanoZone=0

# Keep a disposable inherited value in the shell environment. Every runner must
# override it through run_native_model_suite or this sentinel will be purged.
export SKILLS_COPILOT_APP_DATA_DIR="${SIMULATED_INHERITED_APP_DATA_DIR}"
export SKILLS_COPILOT_REPOSITORY_ROOT="${REPO_ROOT}"

run_native_model_suite swift test \
  --enable-swift-testing \
  --package-path "${PACKAGE_DIR}" \
  --scratch-path "${BUILD_ROOT}/swiftpm" \
  --parallel \
  --skip 'LocalizationModelTests|SkillManagerModelTests' \
  "${SWIFT_TESTING_EXTRA_ARGS[@]}"

# These two suites intentionally switch the process-wide UI language. Run them
# in a second serialized pass while the remaining suites keep parallel execution.
run_native_model_suite swift test \
  --enable-swift-testing \
  --package-path "${PACKAGE_DIR}" \
  --scratch-path "${BUILD_ROOT}/swiftpm" \
  --skip-build \
  --no-parallel \
  --filter 'LocalizationModelTests|SkillManagerModelTests' \
  "${SWIFT_TESTING_EXTRA_ARGS[@]}"

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
