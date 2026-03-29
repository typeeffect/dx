#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STRICT=0

usage() {
  cat <<'EOF'
usage: scripts/check_backend_toolchain.sh [--strict]

Reports whether the local machine is ready for the current backend executable
workflow.

Checks:
  - cargo
  - cc
  - llvm-as
  - llc
  - opt
  - current dx-runtime-stub archive path

Options:
  --strict   exit non-zero when required executable-path tools are missing
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --strict)
      STRICT=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

check_cmd() {
  local name="$1"
  if command -v "$name" >/dev/null 2>&1; then
    printf 'ok       %s -> %s\n' "$name" "$(command -v "$name")"
    return 0
  fi
  printf 'missing  %s\n' "$name"
  return 1
}

required_ok=1

echo "Backend toolchain check"
echo "repo: $ROOT_DIR"

check_cmd cargo || required_ok=0
check_cmd cc || required_ok=0
check_cmd llvm-as || required_ok=0
check_cmd llc || required_ok=0
check_cmd opt || true

archive_path=""
if command -v cargo >/dev/null 2>&1; then
  archive_path="$(
    cd "$ROOT_DIR"
    cargo run -q -p dx-runtime-stub --bin dx-runtime-stub-build-plan \
      | awk '/^archive / { print $2; exit }'
  )"
fi

if [[ -n "$archive_path" ]]; then
  if [[ -f "$ROOT_DIR/$archive_path" ]]; then
    printf 'ok       runtime-archive -> %s\n' "$archive_path"
  else
    printf 'missing  runtime-archive -> %s\n' "$archive_path"
    echo "hint     build it with: cargo build -p dx-runtime-stub"
    required_ok=0
  fi
else
  echo "missing  runtime-archive-path"
  required_ok=0
fi

echo
if [[ $required_ok -eq 1 ]]; then
  echo "status: executable workflow prerequisites available"
  exit 0
fi

echo "status: executable workflow prerequisites incomplete"
if [[ $STRICT -eq 1 ]]; then
  exit 1
fi
