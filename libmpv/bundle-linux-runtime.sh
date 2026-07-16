#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${1:-"$ROOT/libmpv/linux-x86_64"}"
MPV_LIB="${2:-"$DEST/libmpv.so.2"}"

if [[ ! -f "$MPV_LIB" ]]; then
  echo "libmpv runtime was not found: $MPV_LIB" >&2
  exit 1
fi

mkdir -p "$DEST"

declare -A queued=()
declare -A scanned=()
declare -a queue=("$MPV_LIB")
declare -a missing=()
queued["$MPV_LIB"]=1

is_system_runtime() {
  local name="$1"
  case "$name" in
    linux-vdso.so.*|ld-linux*.so.*|ld-musl*.so.*) return 0 ;;
    libc.so.*|libm.so.*|libpthread.so.*|libdl.so.*|librt.so.*) return 0 ;;
    libresolv.so.*|libutil.so.*|libnsl.so.*|libanl.so.*) return 0 ;;
  esac
  return 1
}

enqueue_dependency() {
  local source="$1"
  local name="$2"

  if is_system_runtime "$name"; then
    return
  fi
  if [[ ! -f "$source" ]]; then
    return
  fi

  local target="$DEST/$name"
  if [[ "$source" != "$target" ]]; then
    cp -L "$source" "$target"
    chmod 0644 "$target"
  fi
  if [[ -z "${queued[$target]:-}" ]]; then
    queued["$target"]=1
    queue+=("$target")
  fi
}

scan_library() {
  local library="$1"
  while IFS= read -r line; do
    if [[ "$line" =~ ^[[:space:]]*([^[:space:]]+)[[:space:]]+\=\>[[:space:]]+not[[:space:]]+found ]]; then
      missing+=("${BASH_REMATCH[1]} needed by $library")
      continue
    fi
    if [[ "$line" =~ ^[[:space:]]*([^[:space:]]+)[[:space:]]+\=\>[[:space:]]+([^[:space:]]+) ]]; then
      enqueue_dependency "${BASH_REMATCH[2]}" "${BASH_REMATCH[1]}"
      continue
    fi
    if [[ "$line" =~ ^[[:space:]]*(/[^[:space:]]+) ]]; then
      local source="${BASH_REMATCH[1]}"
      enqueue_dependency "$source" "$(basename "$source")"
    fi
  done < <(ldd "$library")
}

index=0
while (( index < ${#queue[@]} )); do
  library="${queue[$index]}"
  index=$((index + 1))
  if [[ -n "${scanned[$library]:-}" ]]; then
    continue
  fi
  scanned["$library"]=1
  scan_library "$library"
done

if (( ${#missing[@]} > 0 )); then
  printf 'Missing Linux libmpv runtime dependencies:\n' >&2
  printf '  %s\n' "${missing[@]}" >&2
  exit 1
fi

if command -v patchelf >/dev/null 2>&1; then
  while IFS= read -r library; do
    patchelf --set-rpath '$ORIGIN' "$library"
  done < <(find "$DEST" -maxdepth 1 -type f \( -name '*.so' -o -name '*.so.*' \) | sort)
fi

echo "Bundled Linux libmpv runtime files in $DEST"
