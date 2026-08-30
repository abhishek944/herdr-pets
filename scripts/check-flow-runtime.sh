#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TMP=$(mktemp -d "${TMPDIR:-/tmp}/herdr-pets-flow.XXXXXX")
trap 'rm -rf "$TMP"' EXIT INT TERM

cd "$ROOT"
npx tsc \
  --target ES2020 \
  --module commonjs \
  --strict \
  --skipLibCheck \
  --outDir "$TMP" \
  src/flow-runtime.ts
printf '%s\n' '{"type":"commonjs"}' > "$TMP/package.json"
cat > "$TMP/check.cjs" <<'CHECK'
const {
  advanceTrack,
  BehaviorMachine,
  compileBehaviorPack,
  normalizeHerdrState,
  remapTrackPosition,
  resolveBehaviorPack,
} = require("./flow-runtime.js");

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
function equal(actual, expected, message) {
  if (!Object.is(actual, expected)) {
    throw new Error(`${message}: expected ${expected}, got ${actual}`);
  }
}

const manifest = {
  formatVersion: 1,
  id: "runtime-check",
  packVersion: "1",
  clips: {
    walk: { durationMs: 100, role: "locomotion", mirror: true },
    jump: { durationMs: 100, role: "stationary" },
    rest: { durationMs: 100, role: "stationary" },
  },
  states: {
    idle: { completion: "restart", flow: { type: "play", clip: "rest" } },
    working: { completion: "restart", flow: { type: "sequence", steps: [
      { type: "move", clip: "walk", durationMs: 100, speedPxPerSecond: 50 },
      { type: "play", clip: "jump" },
      { type: "wait", durationMs: 100 },
      { type: "repeat", count: 2, flow: { type: "play", clip: "rest" } },
      { type: "choose", choices: [
        { weight: 1, flow: { type: "play", clip: "jump" } },
        { weight: 1, flow: { type: "play", clip: "rest" } },
      ] },
    ] } },
    blocked: { completion: "restart", flow: { type: "play", clip: "jump" } },
    done: { completion: "hold", flow: { type: "play", clip: "jump" } },
    unknown: { completion: "restart", flow: { type: "play", clip: "rest" } },
  },
};

const compiled = compileBehaviorPack(manifest);
assert(compiled.pack, JSON.stringify(compiled.diagnostics));
equal(normalizeHerdrState("future"), "unknown", "unknown-state normalization");
const machine = new BehaviorMachine(compiled.pack, "agent-a", "working");
equal(machine.advance(50).distancePx, 2.5, "logical movement");
assert(!machine.setStatus("working"), "same-state poll restarted the flow");
equal(machine.advance(50).sample.clip.name, "jump", "sequence did not advance");
assert(machine.setStatus("blocked"), "state change did not interrupt");
equal(machine.advance(0).sample.state, "blocked", "replacement flow did not start");
machine.setStatus("done");
machine.advance(100);
assert(machine.sample().held, "done flow did not hold");
const invalid = structuredClone(manifest);
delete invalid.states.unknown;
assert(!compileBehaviorPack(invalid).pack, "missing state was accepted");
assert(resolveBehaviorPack(invalid).usedFallback, "invalid pack did not fall back");
const bounce = advanceTrack(9, 1, 4, 10);
equal(bounce.x, 7, "edge overshoot was lost");
equal(bounce.direction, -1, "edge collision did not turn");
equal(remapTrackPosition(25, 100, 200), 50, "resize remapping changed position ratio");
console.log("flow runtime checks: pass");
CHECK
node "$TMP/check.cjs"
node - "$ROOT" "$TMP/flow-runtime.js" <<'CHECK_PACKS'
const fs = require("node:fs");
const path = require("node:path");
const root = process.argv[2];
const { compileBehaviorPack } = require(process.argv[3]);
const spriteDirectory = path.join(root, "src/assets/sprites");
const assets = Object.fromEntries(
  fs.readdirSync(spriteDirectory)
    .filter((name) => name.endsWith(".png"))
    .map((name) => [`sprites/${name}`, name]),
);
const packDirectory = path.join(root, "src/packs");
const files = fs.readdirSync(packDirectory).filter((name) => name.endsWith(".json")).sort();
if (files.length !== 4) throw new Error(`expected 4 bundled character packs, found ${files.length}`);
for (const file of files) {
  const manifest = JSON.parse(fs.readFileSync(path.join(packDirectory, file), "utf8"));
  const result = compileBehaviorPack(manifest, assets);
  if (!result.pack) throw new Error(`${file}: ${JSON.stringify(result.diagnostics)}`);
}
console.log("bundled character pack checks: pass");
CHECK_PACKS
