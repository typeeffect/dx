#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERIFY_FLAG=""
DRY_RUN_ONLY=0

usage() {
  cat <<'EOF'
usage: scripts/prove_backend_subset.sh [--verify] [--dry-run]

Runs the canonical backend subset proof workflow:
  1. audit all backend demos
  2. dry-run build planning for every canonical backend demo

Options:
  --verify   forwards verification to the audit step
  --dry-run  print the commands that would be executed, without running them
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --verify)
      VERIFY_FLAG="--verify"
      shift
      ;;
    --dry-run)
      DRY_RUN_ONLY=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    -*)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
    *)
      usage >&2
      exit 2
      ;;
  esac
done

declare -a DEMOS=(
  "examples/backend/arithmetic.dx"
  "examples/backend/thunk.dx"
  "examples/backend/closure_call_int.dx"
  "examples/backend/closure_call_str.dx"
  "examples/backend/closure_call_two_args.dx"
  "examples/backend/match_nominal.dx"
)

audit_cmd=("$ROOT_DIR/scripts/audit_backend_demos.sh")
if [[ -n "$VERIFY_FLAG" ]]; then
  audit_cmd+=("$VERIFY_FLAG")
fi

if [[ $DRY_RUN_ONLY -eq 1 ]]; then
  printf '%q ' "$ROOT_DIR/scripts/check_backend_toolchain.sh"
  printf '\n'
  printf '%q ' "${audit_cmd[@]}"
  printf '\n'
  for demo in "${DEMOS[@]}"; do
    build_dir="build/$(basename "${demo%.dx}")"
    printf '%q ' "$ROOT_DIR/scripts/build_backend_demo.sh" --dry-run "$demo" "$build_dir"
    printf '\n'
  done
  exit 0
fi

echo "==> backend toolchain preflight"
"$ROOT_DIR/scripts/check_backend_toolchain.sh"

echo
echo "==> audit backend demos"
"${audit_cmd[@]}"

for demo in "${DEMOS[@]}"; do
  build_dir="build/$(basename "${demo%.dx}")"
  echo
  echo "==> build dry-run $demo"
  "$ROOT_DIR/scripts/build_backend_demo.sh" --dry-run "$demo" "$build_dir"
done

echo
echo "proof ok: canonical backend subset"
