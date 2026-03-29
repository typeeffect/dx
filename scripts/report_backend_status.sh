#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT=""

usage() {
  cat <<'EOF'
usage: scripts/report_backend_status.sh [--output <path>]

Emits a consolidated backend status report containing:
  - toolchain preflight
  - runtime-stub metadata
  - canonical subset proof dry-run
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --output)
      if [[ $# -lt 2 ]]; then
        echo "--output requires a path" >&2
        exit 2
      fi
      OUTPUT="$2"
      shift 2
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

emit_report() {
  echo "# Backend Status Report"
  echo
  echo "repo: $ROOT_DIR"
  echo
  echo "## Toolchain Preflight"
  "$ROOT_DIR/scripts/check_backend_toolchain.sh" || true
  echo
  echo "## Runtime Stub Info"
  (
    cd "$ROOT_DIR"
    cargo run -q -p dx-runtime-stub --bin dx-runtime-stub-info
  )
  echo
  echo "## Canonical Subset Proof (Dry Run)"
  "$ROOT_DIR/scripts/prove_backend_subset.sh" --dry-run
}

if [[ -n "$OUTPUT" ]]; then
  mkdir -p "$(dirname "$OUTPUT")"
  emit_report > "$OUTPUT"
  echo "wrote backend status report: $OUTPUT"
else
  emit_report
fi
