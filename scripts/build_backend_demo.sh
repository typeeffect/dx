#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERIFY_FLAG=""
DRY_RUN=0

usage() {
  cat <<'EOF'
usage: scripts/build_backend_demo.sh [--verify] [--dry-run] <input.dx> [build-dir]

Build flow:
  1. cargo build -p dx-runtime-stub
  2. dx-plan-exec <input.dx> [build-dir]
  3. execute each planned step

Options:
  --verify   ask dx-emit-llvm to verify with LLVM tools during emission
  --dry-run  print the commands without executing them
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --verify)
      VERIFY_FLAG="--verify"
      shift
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      break
      ;;
    -*)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
    *)
      break
      ;;
  esac
done

if [[ $# -lt 1 || $# -gt 2 ]]; then
  usage >&2
  exit 2
fi

INPUT="$1"
BUILD_DIR="${2:-build}"
INPUT_BASENAME="$(basename "$INPUT")"
DEMO_NAME="${INPUT_BASENAME%.dx}"

require_tool() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "missing required tool: $name" >&2
    echo "hint: use --dry-run to inspect the generated commands without executing them" >&2
    exit 1
  fi
}

plan_cmd=(cargo run -q -p dx-llvm-ir --bin dx-plan-exec --)
if [[ -n "$VERIFY_FLAG" ]]; then
  plan_cmd+=("$VERIFY_FLAG")
fi
plan_cmd+=("$INPUT" "$BUILD_DIR")

build_cmd=(cargo build -q -p dx-runtime-stub)

if [[ $DRY_RUN -eq 1 ]]; then
  echo "repo: $ROOT_DIR"
  printf '%q ' "${build_cmd[@]}"
  printf '\n'
  (
    cd "$ROOT_DIR"
    "${plan_cmd[@]}"
  )
  exit 0
fi

require_tool cargo
require_tool cc
require_tool llvm-as
require_tool llc

echo "==> building runtime stub"
(
  cd "$ROOT_DIR"
  "${build_cmd[@]}"
)

echo "==> planning executable"
PLAN="$(
  cd "$ROOT_DIR"
  "${plan_cmd[@]}"
)"
printf '%s\n' "$PLAN"

while IFS= read -r line; do
  [[ -z "$line" ]] && continue
  echo "==> $line"
  (
    cd "$ROOT_DIR"
    bash -lc "$line"
  )
done <<< "$PLAN"

echo "built executable: $BUILD_DIR/$DEMO_NAME"
