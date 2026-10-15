#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TMP=$(mktemp -d "${TMPDIR:-/tmp}/pet-town-remote.XXXXXX")
trap 'rm -rf "$TMP"' EXIT INT TERM
cd "$ROOT/apps/pet-town"
pnpm exec esbuild src/agent-focus.ts --bundle --platform=node --format=cjs \
  --log-level=error --outfile="$TMP/agent-focus.cjs"
node "$ROOT/scripts/remote-checks/focus.cjs" "$TMP/agent-focus.cjs"
node --experimental-vm-modules "$ROOT/scripts/remote-checks/terminal.mjs"
node --experimental-vm-modules "$ROOT/scripts/remote-checks/controls.mjs"
