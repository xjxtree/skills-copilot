#!/usr/bin/env bash
set -euo pipefail

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

SKILLS_COPILOT_NATIVE_MODEL_TEST_SUITE=service-process \
  "${BINARY_DIR}/SkillsCopilotNativeModelTests"

SKILLS_COPILOT_NATIVE_MODEL_TEST_SUITE=service-rpc \
  "${BINARY_DIR}/SkillsCopilotNativeModelTests"

SKILLS_COPILOT_NATIVE_MODEL_TEST_SUITE=main \
  "${BINARY_DIR}/SkillsCopilotNativeModelTests"

SKILL_STORE_GROUP_COUNT=64
for group in $(seq 0 $((SKILL_STORE_GROUP_COUNT - 1))); do
  SKILLS_COPILOT_NATIVE_MODEL_TEST_SUITE="skill-store-${group}" \
  SKILLS_COPILOT_SKILL_STORE_GROUP_COUNT="${SKILL_STORE_GROUP_COUNT}" \
    "${BINARY_DIR}/SkillsCopilotNativeModelTests"
done
