#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT_DIR/build/backend-demo-artifacts"

usage() {
  cat <<'EOF'
usage: scripts/export_backend_demo_artifacts.sh [output-dir]

Exports backend demo artifacts for every canonical demo under examples/backend/:
  - emitted LLVM IR when validation passes
  - executable plan text
  - stderr logs for failing emit/plan steps
  - a summary report
EOF
}

if [[ $# -gt 1 ]]; then
  usage >&2
  exit 2
fi

if [[ ${1:-} == "--help" || ${1:-} == "-h" ]]; then
  usage
  exit 0
fi

if [[ $# -eq 1 ]]; then
  OUT_DIR="$1"
fi

mkdir -p "$OUT_DIR"

mapfile -t DEMOS < <(find "$ROOT_DIR/examples/backend" -maxdepth 1 -type f -name '*.dx' | sort)

summary="$OUT_DIR/SUMMARY.md"
{
  echo "# Backend Demo Artifact Export"
  echo
  echo "repo: $ROOT_DIR"
  echo "output: $OUT_DIR"
  echo
  echo "| Demo | Emit | Plan | Files |"
  echo "|---|---|---|---|"
} > "$summary"

for demo in "${DEMOS[@]}"; do
  name="$(basename "${demo%.dx}")"
  ll_file="$OUT_DIR/$name.ll"
  plan_file="$OUT_DIR/$name.plan"
  emit_err="$OUT_DIR/$name.emit.stderr.txt"
  plan_err="$OUT_DIR/$name.plan.stderr.txt"

  emit_status="ok"
  plan_status="ok"

  if ! (
    cd "$ROOT_DIR"
    cargo run -q -p dx-llvm-ir --bin dx-emit-llvm -- "$demo" "$ll_file"
  ) > /dev/null 2> "$emit_err"; then
    emit_status="fail"
    rm -f "$ll_file"
  elif [[ ! -s "$emit_err" ]]; then
    rm -f "$emit_err"
  fi

  if ! (
    cd "$ROOT_DIR"
    cargo run -q -p dx-llvm-ir --bin dx-plan-exec -- "$demo" "$OUT_DIR/$name.build"
  ) > "$plan_file" 2> "$plan_err"; then
    plan_status="fail"
    rm -f "$plan_file"
  elif [[ ! -s "$plan_err" ]]; then
    rm -f "$plan_err"
  fi

  files=()
  [[ -f "$ll_file" ]] && files+=("$(basename "$ll_file")")
  [[ -f "$plan_file" ]] && files+=("$(basename "$plan_file")")
  [[ -f "$emit_err" ]] && files+=("$(basename "$emit_err")")
  [[ -f "$plan_err" ]] && files+=("$(basename "$plan_err")")

  files_cell=""
  if [[ ${#files[@]} -gt 0 ]]; then
    for file in "${files[@]}"; do
      if [[ -n "$files_cell" ]]; then
        files_cell+=" "
      fi
      files_cell+="\`$file\`"
    done
  fi
  {
    printf '| `%s` | %s | %s | %s |\n' \
      "$(basename "$demo")" \
      "$emit_status" \
      "$plan_status" \
      "$files_cell"
  } >> "$summary"
done

echo "wrote backend demo artifacts to: $OUT_DIR"
echo "summary: $summary"
