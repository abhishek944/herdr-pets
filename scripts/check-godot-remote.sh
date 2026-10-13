#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TMP=$(mktemp -d "${TMPDIR:-/tmp}/pet-town-godot-check.XXXXXX")
trap 'rm -rf "$TMP"' EXIT HUP INT TERM
APP=$(sh "$ROOT/scripts/prepare-godot-engine.sh")
# A fresh checkout needs font/icon imports before toolbar scripts can load.
"$APP/Contents/MacOS/Godot" --headless --editor --import \
  --path "$ROOT/apps/pet-town-godot-sample" --log-file "$TMP/import.log" >"$TMP/import-result" 2>&1
if grep -q 'SCRIPT ERROR' "$TMP/import-result"; then cat "$TMP/import-result"; exit 1; fi
"$APP/Contents/MacOS/Godot" --headless --path "$ROOT/apps/pet-town-godot-sample" \
  --log-file "$TMP/godot.log" --quit-after 5 \
  --script res://tests/remote_observe.gd >"$TMP/result" 2>&1
cat "$TMP/result"
# Godot can exit zero after a script assertion; require its completion marker.
grep -q 'Godot remote Observe checks: pass' "$TMP/result"
if grep -q 'SCRIPT ERROR' "$TMP/result"; then exit 1; fi
