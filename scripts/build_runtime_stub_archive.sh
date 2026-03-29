#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE="debug"
TARGET_DIR=""
DRY_RUN=0

usage() {
  cat <<'EOF'
usage: scripts/build_runtime_stub_archive.sh [--profile <name>] [--target-dir <path>] [--dry-run]

Builds the dx-runtime-stub archive using the canonical build plan.

Options:
  --profile <name>    cargo profile to build (default: debug)
  --target-dir <path> override CARGO_TARGET_DIR
  --dry-run           print the planned command and archive path without executing
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

plan_cmd=(cargo run -q -p dx-runtime-stub --bin dx-runtime-stub-build-plan -- "$PROFILE")
if [[ -n "$TARGET_DIR" ]]; then
  plan_cmd+=("$TARGET_DIR")
fi

PLAN="$(
  cd "$ROOT_DIR"
  "${plan_cmd[@]}"
)"

if [[ $DRY_RUN -eq 1 ]]; then
  printf '%s\n' "$PLAN"
  exit 0
fi

env_prefix=()
build_cmd=()
archive_path=""
while IFS= read -r line; do
  [[ -z "$line" ]] && continue
  if [[ "$line" == archive\ * ]]; then
    archive_path="${line#archive }"
    continue
  fi
  if [[ "$line" == *=* && "$line" != cargo* ]]; then
    env_prefix+=("$line")
    continue
  fi
  build_cmd=("$line")
done <<< "$PLAN"

if [[ ${#build_cmd[@]} -eq 0 ]]; then
  echo "could not determine build command from plan" >&2
  exit 1
fi

echo "==> runtime stub build plan"
printf '%s\n' "$PLAN"

echo "==> building runtime stub archive"
(
  cd "$ROOT_DIR"
  if [[ ${#env_prefix[@]} -gt 0 ]]; then
    env "${env_prefix[@]}" bash -lc "${build_cmd[0]}"
  else
    bash -lc "${build_cmd[0]}"
  fi
)

if [[ -n "$archive_path" ]]; then
  echo "built runtime archive: $archive_path"
fi
