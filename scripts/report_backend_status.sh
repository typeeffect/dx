#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT=""
JSON_MODE=0

usage() {
  cat <<'EOF'
usage: scripts/report_backend_status.sh [--json] [--output <path>]

Emits a consolidated backend status report containing:
  - toolchain preflight
  - runtime-stub metadata
  - canonical subset proof dry-run

Options:
  --json           emit a structured JSON summary instead of Markdown text
  --output <path>  write the result to a file instead of stdout
EOF
}

json_escape() {
  local s="${1//\\/\\\\}"
  s="${s//\"/\\\"}"
  s="${s//$'\n'/\\n}"
  s="${s//$'\r'/\\r}"
  s="${s//$'\t'/\\t}"
  printf '%s' "$s"
}

json_bool() {
  if [[ "$1" -eq 1 ]]; then
    printf 'true'
  else
    printf 'false'
  fi
}

tool_json() {
  local name="$1"
  if command -v "$name" >/dev/null 2>&1; then
    printf '{"ok":true,"path":"%s"}' "$(json_escape "$(command -v "$name")")"
  else
    printf '{"ok":false,"path":null}'
  fi
}

runtime_archive_json() {
  local archive_path=""
  if command -v cargo >/dev/null 2>&1; then
    archive_path="$(
      cd "$ROOT_DIR"
      cargo run -q -p dx-runtime-stub --bin dx-runtime-stub-build-plan \
        | awk '/^archive / { print $2; exit }'
    )"
  fi

  if [[ -n "$archive_path" && -f "$ROOT_DIR/$archive_path" ]]; then
    printf '{"ok":true,"path":"%s"}' "$(json_escape "$archive_path")"
  elif [[ -n "$archive_path" ]]; then
    printf '{"ok":false,"path":"%s"}' "$(json_escape "$archive_path")"
  else
    printf '{"ok":false,"path":null}'
  fi
}

emit_json_report() {
  mapfile -t demos < <(find "$ROOT_DIR/examples/backend" -maxdepth 1 -type f -name '*.dx' | sort)
  mapfile -t runtime_symbols < <(
    cd "$ROOT_DIR"
    cargo run -q -p dx-runtime-stub --bin dx-runtime-stub-symbols
  )

  printf '{\n'
  printf '  "repo": "%s",\n' "$(json_escape "$ROOT_DIR")"
  printf '  "canonical_demo_count": %d,\n' "${#demos[@]}"

  printf '  "canonical_demos": [\n'
  for i in "${!demos[@]}"; do
    suffix=","
    if [[ $i -eq $((${#demos[@]} - 1)) ]]; then
      suffix=""
    fi
    printf '    "%s"%s\n' "$(json_escape "${demos[$i]#$ROOT_DIR/}")" "$suffix"
  done
  printf '  ],\n'

  printf '  "toolchain": {\n'
  printf '    "cargo": %s,\n' "$(tool_json cargo)"
  printf '    "cc": %s,\n' "$(tool_json cc)"
  printf '    "llvm-as": %s,\n' "$(tool_json llvm-as)"
  printf '    "llc": %s,\n' "$(tool_json llc)"
  printf '    "opt": %s,\n' "$(tool_json opt)"
  printf '    "runtime_archive": %s\n' "$(runtime_archive_json)"
  printf '  },\n'

  printf '  "runtime_stub_symbol_count": %d,\n' "${#runtime_symbols[@]}"
  printf '  "runtime_stub_symbols": [\n'
  for i in "${!runtime_symbols[@]}"; do
    suffix=","
    if [[ $i -eq $((${#runtime_symbols[@]} - 1)) ]]; then
      suffix=""
    fi
    printf '    "%s"%s\n' "$(json_escape "${runtime_symbols[$i]}")" "$suffix"
  done
  printf '  ],\n'

  cargo_ok=0
  cc_ok=0
  llvm_as_ok=0
  llc_ok=0
  runtime_archive_ok=0
  command -v cargo >/dev/null 2>&1 && cargo_ok=1
  command -v cc >/dev/null 2>&1 && cc_ok=1
  command -v llvm-as >/dev/null 2>&1 && llvm_as_ok=1
  command -v llc >/dev/null 2>&1 && llc_ok=1
  if [[ "$(runtime_archive_json)" == '{"ok":true,"path":'* ]]; then
    runtime_archive_ok=1
  fi

  workflow_ready=0
  if [[ $cargo_ok -eq 1 && $cc_ok -eq 1 && $llvm_as_ok -eq 1 && $llc_ok -eq 1 && $runtime_archive_ok -eq 1 ]]; then
    workflow_ready=1
  fi
  printf '  "executable_workflow_ready": %s\n' "$(json_bool "$workflow_ready")"
  printf '}\n'
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --json)
      JSON_MODE=1
      shift
      ;;
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

emit() {
  if [[ $JSON_MODE -eq 1 ]]; then
    emit_json_report
  else
    emit_report
  fi
}

if [[ -n "$OUTPUT" ]]; then
  mkdir -p "$(dirname "$OUTPUT")"
  emit > "$OUTPUT"
  echo "wrote backend status report: $OUTPUT"
else
  emit
fi
