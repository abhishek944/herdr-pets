#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TMP=$(mktemp -d "${TMPDIR:-/tmp}/pet-town-flow.XXXXXX")
trap 'rm -rf "$TMP"' EXIT INT TERM

cd "$ROOT/apps/pet-town"
pnpm exec tsc \
  --target ES2020 \
  --module commonjs \
  --strict \
  --skipLibCheck \
  --outDir "$TMP" \
  src/flow-runtime.ts
printf '%s\n' '{"type":"commonjs"}' >"$TMP/package.json"
cp "$ROOT/scripts/flow-checks/check.cjs" "$TMP/check.cjs"
node "$TMP/check.cjs"
# The renderer bundle reaches Vite's import.meta.glob through the character packs; stub that
# build-time helper so the Node harness can drive renderer-view with synthetic samples instead.
pnpm exec esbuild src/renderer-view.ts --bundle --platform=node --format=cjs --log-level=error \
  --define:import.meta.glob=globalThis.__viteGlob \
  --banner:js='globalThis.__viteGlob=(...args)=>({});' \
  --outfile="$TMP/renderer-view.cjs"
cp "$ROOT/scripts/flow-checks/check-renderer.cjs" "$TMP/check-renderer.cjs"
node "$TMP/check-renderer.cjs"
node "$ROOT/scripts/flow-checks/check-packs.cjs" "$ROOT/apps/pet-town" "$TMP/flow-runtime.js"
python3 "$ROOT/scripts/check-pet-assets.py"
