#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: script/check_privacy.sh [--no-history]

Checks the repository for privacy leaks before commit or push.

The check blocks real local paths, common token/key shapes, known local
credential placeholders, and sensitive strings embedded in tracked binary
artifacts. It intentionally allows fixture paths such as /tmp/home and
documentation placeholders such as $HOME, <repo>, and <project-root>.
Tracked text also blocks fixed local host-port fingerprints. Use non-local
fixture hosts for provider examples; loopback port zero is allowed only for
intentional ephemeral listener binds.

Note: binary string scanning does not perform OCR. Screenshots used for a task
must stay outside the repository and be inspected before handoff.
EOF
}

check_history=1
while [[ $# -gt 0 ]]; do
  case "$1" in
    --)
      shift
      ;;
    --no-history)
      check_history=0
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

mac_home='/'Users'/'
linux_home='/'home'/'
var_folders='/'var'/folders/'
private_var='/'private'/var/'

text_pattern="(${mac_home}[^[:space:]\`\"<>)]*|(^|[[:space:]\`\"'=:,({[])\${linux_home}[A-Za-z0-9._-]+[^[:space:]\`\"<>)]*|${var_folders}[^[:space:]\`\"<>)]*|${private_var}[^[:space:]\`\"<>)]*|fixture[-]secret|PROXY[_-]MANAGED|ANTHROPIC_AUTH_TOKEN[=:][^<[:space:]]+|OPENAI_API_KEY[=:][^<[:space:]]+|DASHSCOPE_API_KEY[=:][^<[:space:]]+|API[_-]?KEY[=:][^<[:space:]]+|TOKEN[=:][^<[:space:]]+|SECRET[=:][^<[:space:]]+|PASSWORD[=:][^<[:space:]]+|(^|[^A-Za-z0-9_-])sk-[A-Za-z0-9_-]{20,}|ghp_[A-Za-z0-9_]{20,}|BEGIN [A-Z ]*PRIVATE KEY|ssh-rsa[[:space:]])"
local_host_port_pattern="(localhost:[0-9]+|127[.]0[.]0[.]1:([1-9][0-9]*|0[0-9]+))"

echo "privacy check: tracked text"
if git grep -n -I -E "$text_pattern" -- . ':(exclude)dist' ':(exclude)target'; then
  echo "privacy check failed: tracked text contains sensitive-looking content" >&2
  exit 1
fi

echo "privacy check: fixed local host-port fingerprints"
if git grep -n -I -E "$local_host_port_pattern" -- . ':(exclude)dist' ':(exclude)target'; then
  echo "privacy check failed: tracked text contains fixed local host-port fingerprints" >&2
  echo "use non-local fixture hosts, or loopback port zero for deliberate ephemeral listener binds" >&2
  exit 1
fi

echo "privacy check: binary string metadata"
binary_failed=0
while IFS= read -r path; do
  [[ -f "$path" ]] || continue
  if strings "$path" | grep -n -E "$text_pattern"; then
    echo "privacy check failed: binary artifact contains sensitive-looking strings: $path" >&2
    binary_failed=1
  fi
done < <(git ls-files '*.png' '*.jpg' '*.jpeg' '*.gif' '*.webp' '*.pdf' '*.icns')
if [[ "$binary_failed" -ne 0 ]]; then
  exit 1
fi

if [[ "$check_history" -eq 1 ]]; then
  echo "privacy check: reachable history"
  if git log --all --oneline -G"$text_pattern" -- . ':(exclude)dist' ':(exclude)target' | grep .; then
    echo "privacy check failed: reachable history contains sensitive-looking content" >&2
    echo "rewrite or remove the affected history before pushing" >&2
    exit 1
  fi
fi

echo "privacy check: ok"
