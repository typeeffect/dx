#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERIFY_FLAG=""

if [[ "${1:-}" == "--verify" ]]; then
  VERIFY_FLAG="--verify"
  shift
fi

if [[ $# -ne 0 ]]; then
  echo "usage: scripts/audit_backend_demos.sh [--verify]" >&2
  exit 2
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

declare -a DEMOS=(
  "arithmetic"
  "thunk"
  "closure_call_int"
  "closure_call_str"
  "closure_call_two_args"
  "match_nominal"
)

expected_ir_symbols() {
  case "$1" in
    arithmetic)
      return 0
      ;;
    thunk)
      printf '%s\n' "dx_rt_closure_create" "dx_rt_thunk_call_i64"
      ;;
    closure_call_int)
      printf '%s\n' "dx_rt_closure_create" "dx_rt_closure_call_i64_1_i64"
      ;;
    closure_call_str)
      printf '%s\n' "dx_rt_closure_create" "dx_rt_closure_call_ptr_1_ptr"
      ;;
    closure_call_two_args)
      printf '%s\n' "dx_rt_closure_create" "dx_rt_closure_call_i64_2_i64_i64"
      ;;
    match_nominal)
      printf '%s\n' "dx_rt_match_tag"
      ;;
    *)
      echo "unknown demo: $1" >&2
      exit 1
      ;;
  esac
}

expected_runtime_symbols() {
  case "$1" in
    arithmetic)
      return 0
      ;;
    thunk)
      printf '%s\n' "dx_rt_closure_create" "dx_rt_thunk_call_i64"
      ;;
    closure_call_int)
      printf '%s\n' "dx_rt_closure_create" "dx_rt_closure_call_i64_1_i64"
      ;;
    closure_call_str)
      printf '%s\n' "dx_rt_closure_create" "dx_rt_closure_call_ptr_1_ptr"
      ;;
    closure_call_two_args)
      printf '%s\n' "dx_rt_closure_create" "dx_rt_closure_call_i64_2_i64_i64"
      ;;
    match_nominal)
      printf '%s\n' "dx_rt_match_tag"
      ;;
    *)
      echo "unknown demo: $1" >&2
      exit 1
      ;;
  esac
}

runtime_symbols="$tmpdir/runtime_symbols.txt"
(cd "$ROOT_DIR" && cargo run -q -p dx-runtime-stub --bin dx-runtime-stub-symbols) > "$runtime_symbols"

echo "Backend demo audit"
echo "repo: $ROOT_DIR"
if [[ -n "$VERIFY_FLAG" ]]; then
  echo "mode: verify"
else
  echo "mode: emit+plan"
fi

for demo in "${DEMOS[@]}"; do
  src="examples/backend/${demo}.dx"
  ll="$tmpdir/${demo}.ll"
  echo
  echo "==> $src"

  if [[ -n "$VERIFY_FLAG" ]]; then
    (cd "$ROOT_DIR" && cargo run -q -p dx-llvm-ir --bin dx-emit-llvm -- --verify "$src" "$ll")
    (cd "$ROOT_DIR" && cargo run -q -p dx-llvm-ir --bin dx-plan-exec -- --verify "$src" "$tmpdir/${demo}_build") > /dev/null
  else
    (cd "$ROOT_DIR" && cargo run -q -p dx-llvm-ir --bin dx-emit-llvm -- "$src" "$ll")
    (cd "$ROOT_DIR" && cargo run -q -p dx-llvm-ir --bin dx-plan-exec -- "$src" "$tmpdir/${demo}_build") > /dev/null
  fi

  while IFS= read -r symbol; do
    [[ -z "$symbol" ]] && continue
    rg -q "$symbol" "$ll"
  done < <(expected_ir_symbols "$demo")

  while IFS= read -r symbol; do
    [[ -z "$symbol" ]] && continue
    rg -qx "$symbol" "$runtime_symbols"
  done < <(expected_runtime_symbols "$demo")

  echo "ok: emit, plan, symbols"
done

echo
echo "audit ok: ${#DEMOS[@]} demos"
