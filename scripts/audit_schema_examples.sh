#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MANIFEST="$ROOT/scripts/schema_examples.txt"
SOURCE_MANIFEST="$ROOT/scripts/schema_source_examples.txt"

usage() {
  cat <<'EOF'
usage: scripts/audit_schema_examples.sh [--verify]

Checks the schema example package end-to-end:
- validates annotated .dxschema examples
- contract-matches them against expected name/provider/source
- checks canonical artifacts for canonical form
- checks DX source schema declarations against locked artifacts
EOF
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
  exit 0
fi

VERIFY=0
if [[ "${1:-}" == "--verify" ]]; then
  VERIFY=1
fi

count=0
while IFS='|' read -r raw_name provider source example canonical; do
  [[ -z "$raw_name" ]] && continue
  name="${raw_name^}"

  cargo run -q -p dx-schema --bin dx-schema-validate -- \
    --expect-name "$name" \
    --expect-provider "$provider" \
    --expect-source "$source" \
    "$ROOT/$example" >/dev/null

  cargo run -q -p dx-schema --bin dx-schema-match -- \
    --name "$name" \
    --provider "$provider" \
    --source "$source" \
    "$ROOT/$example" >/dev/null

  cargo run -q -p dx-schema --bin dx-schema-validate -- \
    --check-canonical \
    "$ROOT/$canonical" >/dev/null

  if [[ "$VERIFY" -eq 1 ]]; then
    generated="$(mktemp)"
    trap 'rm -f "$generated"' EXIT
    cargo run -q -p dx-schema --bin dx-schema-new -- \
      --name "$name" \
      --provider "$provider" \
      --source "$source" \
      --source-fingerprint "sha256:source" \
      --schema-fingerprint "sha256:schema" \
      --generated-at "2026-03-29T10:00:00Z" \
      --field id=Int \
      --output "$generated" >/dev/null
    cargo run -q -p dx-schema --bin dx-schema-validate -- --check-canonical "$generated" >/dev/null
    rm -f "$generated"
    trap - EXIT
  fi

  echo "ok schema-example $raw_name"
  count=$((count + 1))
done < "$MANIFEST"

while IFS='|' read -r name source_file artifact_file; do
  [[ -z "$name" ]] && continue

  cargo run -q -p dx-schema --bin dx-schema-check-source -- \
    --name "$name" \
    "$ROOT/$source_file" \
    "$ROOT/$artifact_file" >/dev/null

  echo "ok schema-source $name"
done < "$SOURCE_MANIFEST"

echo "ok audited $count schema examples"
