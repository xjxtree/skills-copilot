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
BUILD_ROOT="${REPO_ROOT}/apps/macos/.build/native-model-tests"
PACKAGE_DIR="${BUILD_ROOT}/package"
TARGET_DIR="${PACKAGE_DIR}/Sources/SkillsCopilotNativeModelTests"

rm -rf "${PACKAGE_DIR}"
mkdir -p "${TARGET_DIR}"

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
  --include='*.swift' \
  --exclude='*' \
  "${REPO_ROOT}/apps/macos/Tests/SkillsCopilotTests/" \
  "${TARGET_DIR}/Tests/"

find "${TARGET_DIR}/Tests" -name '*.swift' -print0 \
  | xargs -0 perl -0pi -e 's/^\@testable import SkillsCopilot\n//mg'

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
await runNativeModelTestsMain()
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

# Keep a disposable inherited value in the shell environment. Every runner must
# override it through run_native_model_suite or this sentinel will be purged.
export SKILLS_COPILOT_APP_DATA_DIR="${SIMULATED_INHERITED_APP_DATA_DIR}"

run_native_model_suite \
  SKILLS_COPILOT_NATIVE_MODEL_TEST_SUITE=service-process \
  "${BINARY_DIR}/SkillsCopilotNativeModelTests"

run_native_model_suite \
  SKILLS_COPILOT_NATIVE_MODEL_TEST_SUITE=service-rpc \
  "${BINARY_DIR}/SkillsCopilotNativeModelTests"

run_native_model_suite \
  SKILLS_COPILOT_NATIVE_MODEL_TEST_SUITE=main \
  "${BINARY_DIR}/SkillsCopilotNativeModelTests"

SKILL_STORE_GROUP_COUNT=64
for group in $(seq 0 $((SKILL_STORE_GROUP_COUNT - 1))); do
  run_native_model_suite \
    SKILLS_COPILOT_NATIVE_MODEL_TEST_SUITE="skill-store-${group}" \
    SKILLS_COPILOT_SKILL_STORE_GROUP_COUNT="${SKILL_STORE_GROUP_COUNT}" \
    "${BINARY_DIR}/SkillsCopilotNativeModelTests"
done

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
