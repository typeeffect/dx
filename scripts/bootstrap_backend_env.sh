#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE="debug"
TARGET_DIR=""
REPORT_OUTPUT=""
DRY_RUN=0

usage() {
  cat <<'EOF'
usage: scripts/bootstrap_backend_env.sh [--profile <name>] [--target-dir <path>] [--report-output <path>] [--dry-run]

Bootstrap the local backend executable environment:
  1. build the dx-runtime-stub archive
  2. emit a consolidated backend status report

Options:
  --profile <name>       cargo profile for dx-runtime-stub (default: debug)
  --target-dir <path>    override CARGO_TARGET_DIR for the runtime archive
  --report-output <path> write the backend status report to a file
  --dry-run              print the commands without executing them
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --profile)
      if [[ $# -lt 2 ]]; then
        echo "--profile requires a value" >&2
        exit 2
      fi
      PROFILE="$2"
      shift 2
      ;;
    --target-dir)
      if [[ $# -lt 2 ]]; then
        echo "--target-dir requires a path" >&2
        exit 2
      fi
      TARGET_DIR="$2"
      shift 2
      ;;
    --report-output)
      if [[ $# -lt 2 ]]; then
        echo "--report-output requires a path" >&2
        exit 2
      fi
      REPORT_OUTPUT="$2"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=1
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

build_cmd=("$ROOT_DIR/scripts/build_runtime_stub_archive.sh" --profile "$PROFILE")
if [[ -n "$TARGET_DIR" ]]; then
  build_cmd+=(--target-dir "$TARGET_DIR")
fi

report_cmd=("$ROOT_DIR/scripts/report_backend_status.sh")
if [[ -n "$REPORT_OUTPUT" ]]; then
  report_cmd+=(--output "$REPORT_OUTPUT")
fi

if [[ $DRY_RUN -eq 1 ]]; then
  printf '%q ' "${build_cmd[@]}" --dry-run
  printf '\n'
  printf '%q ' "${report_cmd[@]}"
  printf '\n'
  exit 0
fi

echo "==> bootstrap runtime stub archive"
"${build_cmd[@]}"

echo
echo "==> backend status report"
"${report_cmd[@]}"
