#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VERIFY_FLAG=""
DRY_RUN=0

usage() {
  cat <<'EOF'
usage: scripts/prove_executable_entry_subset.sh [--verify] [--dry-run]

Runs the currently runnable executable-entry subset proof workflow:
  1. strict backend toolchain preflight
  2. build the runtime stub archive
  3. run each runnable executable-entry demo via dx-run-exec
  4. verify the observed exit code against the source-of-truth manifest

Options:
  --verify   forwards verification to dx-run-exec
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
      DRY_RUN=1
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

declare -A EXPECTED_EXIT_CODES=()
while read -r name code; do
  [[ -z "${name:-}" ]] && continue
  EXPECTED_EXIT_CODES["$name"]="$code"
done < "$ROOT_DIR/scripts/runnable_entry_expected_exit_codes.txt"

declare -a DEMOS=()
while IFS= read -r name; do
  [[ -z "$name" ]] && continue
  DEMOS+=("$name")
done < "$ROOT_DIR/scripts/runnable_entry_demos.txt"

for demo in "${DEMOS[@]}"; do
  if [[ -z "${EXPECTED_EXIT_CODES[$demo]:-}" ]]; then
    echo "missing expected exit code for runnable executable-entry demo: $demo" >&2
    exit 1
  fi
done

run_cmd() {
  local demo="$1"
  local demo_path="examples/backend/${demo}.dx"
  local build_dir="build/${demo}"
  local -a cmd=(cargo run -q -p dx-llvm-ir --bin dx-run-exec --)
  if [[ -n "$VERIFY_FLAG" ]]; then
    cmd+=("$VERIFY_FLAG")
  fi
  cmd+=(--json "$demo_path" "$build_dir")
  printf '%q ' "${cmd[@]}"
  printf '\n'
}

extract_exit_code() {
  local json="$1"
  sed -n 's/.*"exit_code":\([0-9][0-9]*\).*/\1/p' <<< "$json"
}

if [[ $DRY_RUN -eq 1 ]]; then
  printf '%q ' "$ROOT_DIR/scripts/check_backend_toolchain.sh" --strict
  printf '\n'
  printf '%q ' "$ROOT_DIR/scripts/build_runtime_stub_archive.sh"
  printf '\n'
  for demo in "${DEMOS[@]}"; do
    run_cmd "$demo"
  done
  exit 0
fi

echo "==> backend toolchain preflight"
"$ROOT_DIR/scripts/check_backend_toolchain.sh" --strict

echo
echo "==> building runtime stub archive"
"$ROOT_DIR/scripts/build_runtime_stub_archive.sh"

for demo in "${DEMOS[@]}"; do
  echo
  echo "==> run $demo"
  output="$(
    cd "$ROOT_DIR"
    run_cmd "$demo" | bash
  )"
  exit_code="$(extract_exit_code "$output")"
  expected="${EXPECTED_EXIT_CODES[$demo]}"
  if [[ -z "$exit_code" ]]; then
    echo "failed to parse exit code from dx-run-exec output for $demo" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
  if [[ "$exit_code" != "$expected" ]]; then
    echo "unexpected exit code for $demo: got $exit_code, expected $expected" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
  echo "ok $demo -> exit code $exit_code"
done

echo
echo "proof ok: runnable executable-entry subset"
