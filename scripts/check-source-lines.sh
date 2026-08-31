#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
FILES=$(mktemp "${TMPDIR:-/tmp}/herdr-pets-lines.XXXXXX")
trap 'rm -f "$FILES"' EXIT INT TERM
MAX_LINES=199
failed=0

find "$ROOT" \
  \( -type d \( -name .git -o -name node_modules -o -name dist -o -name target -o -name gen -o -name var \) -prune \) -o \
  \( -type f \( -name '*.ts' -o -name '*.tsx' -o -name '*.rs' \) -print \) |
  sort >"$FILES"

while IFS= read -r file; do
  lines=$(awk 'END { print NR }' "$file")
  if [ "$lines" -gt "$MAX_LINES" ]; then
    relative=${file#"$ROOT"/}
    printf '%s: %s lines (maximum %s)\n' "$relative" "$lines" "$MAX_LINES" >&2
    failed=1
  fi
done <"$FILES"

[ "$failed" -eq 0 ]
echo "TypeScript and Rust source line limits: pass"
