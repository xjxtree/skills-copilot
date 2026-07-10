#!/usr/bin/env bash

_normalized_absolute_path_hex() {
  local path="${1-}"
  local absolute_path
  if [[ "$path" == /* ]]; then
    absolute_path="$path"
  else
    absolute_path="${PWD%/}/$path"
  fi

  local -a normalized=()
  local remainder="${absolute_path#/}"
  local component
  local has_more
  local last_index
  while :; do
    if [[ "$remainder" == */* ]]; then
      component="${remainder%%/*}"
      remainder="${remainder#*/}"
      has_more=1
    else
      component="$remainder"
      has_more=0
    fi

    case "$component" in
      ""|.)
        ;;
      ..)
        if (( ${#normalized[@]} > 0 )); then
          last_index=$((${#normalized[@]} - 1))
          unset "normalized[$last_index]"
        fi
        ;;
      *)
        normalized[${#normalized[@]}]="$component"
        ;;
    esac

    [[ "$has_more" == "0" ]] && break
  done

  {
    printf '/'
    local separator=""
    for component in "${normalized[@]}"; do
      printf '%s%s' "$separator" "$component"
      separator="/"
    done
    # Encode before command substitution so trailing path newlines remain data.
  } | LC_ALL=C /usr/bin/od -An -v -tx1 | LC_ALL=C /usr/bin/tr -d '[:space:]'
}

filesystem_identity() {
  local path="$1"
  local identity
  # Follow a final symlink so file and directory aliases identify the target entry.
  if identity="$(/usr/bin/stat -Lf '%d:%i' -- "$path" 2>/dev/null)"; then
    printf 'stat:%s\n' "$identity"
  else
    printf 'path:'
    _normalized_absolute_path_hex "$path"
    printf '\n'
  fi
}

same_filesystem_entry() {
  [[ "$(filesystem_identity "$1")" == "$(filesystem_identity "$2")" ]]
}

decode_base64_path() {
  local encoded_path="$1"
  local decoded_with_sentinel
  DECODED_BASE64_PATH=""
  # The sentinel prevents command substitution from trimming decoded newlines.
  if ! decoded_with_sentinel="$(
    printf '%s' "$encoded_path" | /usr/bin/base64 -D || exit 1
    printf '.'
  )"; then
    return 1
  fi
  DECODED_BASE64_PATH="${decoded_with_sentinel%.}"
}
