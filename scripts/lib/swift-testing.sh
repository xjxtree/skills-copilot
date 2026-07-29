#!/usr/bin/env bash

# Apple Command Line Tools ships Swift Testing as a developer framework, but
# some SwiftPM releases do not add that framework search path automatically.
# Populate SWIFT_TESTING_EXTRA_ARGS without downloading a package dependency.
configure_swift_testing_args() {
  SWIFT_TESTING_EXTRA_ARGS=()

  local developer_dir
  developer_dir="$(xcode-select -p 2>/dev/null || true)"
  [[ -n "${developer_dir}" ]] || return 0

  local framework_root=""
  local interop_root=""
  local candidate
  for candidate in \
    "${developer_dir}/Library/Frameworks" \
    "${developer_dir}/Library/Developer/Frameworks"; do
    if [[ -d "${candidate}/Testing.framework" ]]; then
      framework_root="${candidate}"
      break
    fi
  done

  [[ -n "${framework_root}" ]] || return 0

  for candidate in \
    "${developer_dir}/usr/lib" \
    "${developer_dir}/Library/Developer/usr/lib"; do
    if [[ -f "${candidate}/lib_TestingInterop.dylib" ]]; then
      interop_root="${candidate}"
      break
    fi
  done

  SWIFT_TESTING_EXTRA_ARGS+=(
    -Xswiftc -F
    -Xswiftc "${framework_root}"
    -Xlinker -F
    -Xlinker "${framework_root}"
    -Xlinker -rpath
    -Xlinker "${framework_root}"
  )
  if [[ -n "${interop_root}" ]]; then
    SWIFT_TESTING_EXTRA_ARGS+=(
      -Xlinker -rpath
      -Xlinker "${interop_root}"
    )
  fi
}
