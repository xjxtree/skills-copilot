#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
source "${SCRIPT_DIR}/lib/swift-testing.sh"
configure_swift_testing_args

TEMP_ROOT="${TMPDIR:-/tmp}"
RUNTIME_ROOT="$(mktemp -d "${TEMP_ROOT%/}/skills-copilot-swift-tests.XXXXXX")"
cleanup_runtime_root() {
  if [[ -n "${RUNTIME_ROOT:-}" && -d "${RUNTIME_ROOT}" ]]; then
    rm -rf -- "${RUNTIME_ROOT}"
  fi
}
trap cleanup_runtime_root EXIT

RUNTIME_HOME="${RUNTIME_ROOT}/home"
RUNTIME_APP_DATA_DIR="${RUNTIME_ROOT}/app-data"
SWIFT_SCRATCH_PATH="${SWIFTPM_SCRATCH_PATH:-${RUNTIME_ROOT}/swiftpm}"
mkdir -p "${RUNTIME_HOME}" "${RUNTIME_APP_DATA_DIR}"

run_swift_tests() {
  env \
    HOME="${RUNTIME_HOME}" \
    CFFIXED_USER_HOME="${RUNTIME_HOME}" \
    SKILLS_COPILOT_APP_DATA_DIR="${RUNTIME_APP_DATA_DIR}" \
    SKILLS_COPILOT_REPOSITORY_ROOT="${REPO_ROOT}" \
    swift test \
      --enable-swift-testing \
      --package-path "${REPO_ROOT}/apps/macos" \
      --scratch-path "${SWIFT_SCRATCH_PATH}" \
      "${SWIFT_TESTING_EXTRA_ARGS[@]}" \
      "$@"
}

if (( $# > 0 )); then
  run_swift_tests --no-parallel "$@"
  exit 0
fi

run_swift_tests \
  --parallel \
  --skip 'LocalizationModelTests|SkillManagerModelTests'
run_swift_tests \
  --skip-build \
  --no-parallel \
  --filter 'LocalizationModelTests|SkillManagerModelTests'
