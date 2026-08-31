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
    jump: { durationMs: 100, role: "stationary", scale: 1.5, holdAsset: "done.png" },
    rest: { durationMs: 100, role: "stationary" },
  },
  states: {
    idle: { completion: "restart", flow: { type: "hide", durationMs: 100 } },
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
    blocked: { completion: "restart", flow: { type: "wait", durationMs: 100 } },
    done: { completion: "hold", flow: { type: "play", clip: "jump" } },
    unknown: { completion: "restart", flow: { type: "play", clip: "rest" } },
  },
};

const compiled = compileBehaviorPack(manifest);
assert(compiled.pack, JSON.stringify(compiled.diagnostics));
equal(compiled.pack.clips.jump.scale, 1.5, "clip scale was not compiled");
const assetManifest = structuredClone(manifest);
assetManifest.clips.walk.asset = "walk.png";
assetManifest.clips.jump.asset = "jump.png";
assetManifest.clips.rest.asset = "rest.png";
const assetUrls = { "walk.png": "walk-url", "jump.png": "jump-url", "rest.png": "rest-url", "done.png": "done-url" };
const withAssets = compileBehaviorPack(assetManifest, assetUrls);
assert(withAssets.pack, JSON.stringify(withAssets.diagnostics));
equal(withAssets.pack.clips.jump.holdAssetUrl, "done-url", "hold asset was not compiled");
const missingHoldAssets = { ...assetUrls };
delete missingHoldAssets["done.png"];
assert(!compileBehaviorPack(assetManifest, missingHoldAssets).pack, "missing hold asset was accepted");
const assetless = structuredClone(assetManifest);
delete assetless.clips.rest.asset;
assert(!compileBehaviorPack(assetless, assetUrls).pack, "bundled assetless clip was accepted");
equal(normalizeHerdrState("future"), "unknown", "unknown-state normalization");
const machine = new BehaviorMachine(compiled.pack, "agent-a", "working");
const walking = machine.advance(50);
equal(walking.distancePx, 2.5, "logical movement");
const walkEpoch = walking.sample.clipEpoch;
assert(!machine.setStatus("working"), "same-state poll restarted the flow");
const jumping = machine.advance(50).sample;
equal(jumping.clip.name, "jump", "sequence did not advance");
assert(jumping.clipEpoch > walkEpoch, "new clip action did not advance its epoch");
const waiting = machine.advance(100).sample;
equal(waiting.clipEpoch, jumping.clipEpoch, "wait restarted the current clip epoch");
assert(machine.setStatus("blocked"), "state change did not interrupt");
equal(machine.advance(0).sample.state, "blocked", "replacement flow did not start");
machine.setStatus("done");
machine.advance(100);
assert(machine.sample().held, "done flow did not hold");
equal(machine.advance(50, true).remainingMs, 0, "held flow retained elapsed time");
const repeatMachine = new BehaviorMachine(compiled.pack, "agent-repeat", "unknown");
const firstRestEpoch = repeatMachine.advance(0).sample.clipEpoch;
const secondRestEpoch = repeatMachine.advance(100).sample.clipEpoch;
assert(secondRestEpoch > firstRestEpoch, "restarted play action reused its clip epoch");
const boundaryMachine = new BehaviorMachine(compiled.pack, "agent-boundary", "unknown");
boundaryMachine.advance(0);
const boundary = boundaryMachine.advance(150, true);
equal(boundary.remainingMs, 50, "clip-boundary remainder was lost");
equal(boundary.sample.clipElapsedMs, 0, "next clip advanced before readiness check");
const resumed = boundaryMachine.advance(boundary.remainingMs, true);
equal(resumed.sample.clipElapsedMs, 50, "boundary remainder was not resumed");
machine.setStatus("idle");
assert(!machine.advance(0).sample.visible, "hide flow did not hide the citizen");
machine.setStatus("blocked");
assert(machine.advance(0).sample.visible, "state entry did not restore visibility before wait");
const missingHold = structuredClone(manifest);
delete missingHold.clips.jump.holdAsset;
assert(!compileBehaviorPack(missingHold).pack, "visible hold clip without holdAsset was accepted");
const inheritedHold = structuredClone(manifest);
inheritedHold.states.done.flow = { type: "choose", choices: [
  { weight: 1, flow: { type: "hide", durationMs: 100 } },
  { weight: 1, flow: { type: "wait", durationMs: 100 } },
] };
assert(!compileBehaviorPack(inheritedHold).pack, "hold branch inherited an unfreezable clip");
const invalidScale = structuredClone(manifest);
invalidScale.clips.jump.scale = 3;
assert(!compileBehaviorPack(invalidScale).pack, "unsafe clip scale was accepted");
const invalidHide = structuredClone(manifest);
invalidHide.states.idle.flow.durationMs = 0;
assert(!compileBehaviorPack(invalidHide).pack, "zero-time hide was accepted");
const invalid = structuredClone(manifest);
delete invalid.states.unknown;
assert(!compileBehaviorPack(invalid).pack, "missing state was accepted");
const fallback = resolveBehaviorPack(invalid);
assert(fallback.usedFallback, "invalid pack did not fall back");
const fallbackMachine = new BehaviorMachine(fallback.pack, "fallback-agent", "working");
assert(!fallbackMachine.advance(0).sample.visible, "safe fallback rendered without artwork");
const bounce = advanceTrack(9, 1, 4, 10);
equal(bounce.x, 7, "edge overshoot was lost");
equal(bounce.direction, -1, "edge collision did not turn");
equal(remapTrackPosition(25, 100, 200), 50, "resize remapping changed position ratio");
console.log("flow runtime checks: pass");
CHECK
node "$TMP/check.cjs"
npx esbuild src/renderer-view.ts --bundle --platform=node --format=cjs --log-level=error --outfile="$TMP/renderer-view.cjs"
cat > "$TMP/check-renderer.cjs" <<'CHECK_RENDERER'
const {
  applyFlowSample,
  ASSET_READY_TIMEOUT_MS,
  distanceWhileAssetPending,
} = require("./renderer-view.cjs");
function assert(condition, message) { if (!condition) throw new Error(message); }
class FakeStyle {
  constructor() { this.values = new Map(); }
  getPropertyValue(key) { return this.values.get(key) ?? ""; }
  setProperty(key, value) { this.values.set(key, value); }
}
class FakePet {
  constructor() {
    this.className = "pet";
    this.dataset = {};
    this.hidden = true;
    this.style = new FakeStyle();
    this.source = "";
    this.srcAssignments = 0;
  }
  getAttribute(name) { return name === "src" && this.source ? this.source : null; }
  removeAttribute(name) { if (name === "src") this.source = ""; }
  get src() { return this.source; }
  set src(value) { this.source = value; this.srcAssignments += 1; }
}
class FakeLoader {
  static pending = [];
  constructor() {
    this.promise = new Promise((resolve, reject) => { this.resolve = resolve; this.reject = reject; });
    FakeLoader.pending.push(this);
  }
  decode() { return this.promise; }
}
global.Image = FakeLoader;
const clip = (scale, assetUrl = "walk.png") => ({
  name: "walk", durationMs: 100, role: "locomotion", assetUrl, holdAssetUrl: "done.png",
  sourceFacing: "right", mirror: true, scale,
});
const sample = (epoch, scale, moving = true, assetUrl = "walk.png") => ({
  state: "working", clip: clip(scale, assetUrl), clipElapsedMs: 0, clipEpoch: epoch,
  moving, speedPxPerSecond: moving ? 20 : 0, visible: true, held: false, failed: false,
});
const timers = [];
global.setTimeout = (callback, milliseconds) => {
  const timer = { callback, milliseconds, active: true };
  timers.push(timer);
  return timer;
};
global.clearTimeout = (timer) => { timer.active = false; };
const fireLatestTimer = () => {
  const timer = timers.findLast((candidate) => candidate.active);
  assert(timer, "no active asset-release timer");
  timer.active = false;
  timer.callback();
  return timer;
};
(async () => {
  const pet = new FakePet();
  const element = { dataset: {}, hidden: false, querySelector: () => pet };
  assert(!applyFlowSample(element, sample(1, 1)), "undecoded first asset was ready");
  FakeLoader.pending.shift().resolve();
  await new Promise(setImmediate);
  assert(applyFlowSample(element, sample(1, 1)), "decoded first asset was not ready");
  assert(pet.style.getPropertyValue("--clip-scale") === "1", "first scale was not committed");
  assert(!applyFlowSample(element, sample(2, 2)), "new clip epoch did not restart loading");
  assert(pet.style.getPropertyValue("--clip-scale") === "1", "presentation changed before decode");
  FakeLoader.pending.shift().resolve();
  await new Promise(setImmediate);
  assert(applyFlowSample(element, sample(2, 2)), "restarted clip was not ready");
  assert(pet.srcAssignments === 2, "same-URL clip action did not restart the image");
  assert(pet.style.getPropertyValue("--clip-scale") === "2", "new scale was not committed atomically");
  assert(applyFlowSample(element, sample(2, 2, false)), "wait changed asset readiness");
  assert(pet.className.includes("flow-stationary"), "wait did not preserve image with stationary metadata");
  const pending = sample(3, 1.7, true, "work.png");
  assert(!applyFlowSample(element, pending), "new pending asset was ready");
  assert(distanceWhileAssetPending(pending, 250) === 5, "pending move lost horizontal travel");
  assert(distanceWhileAssetPending(sample(3, 1.7, false), 250) === 0, "pending wait gained movement");
  const releaseTimer = fireLatestTimer();
  assert(releaseTimer.milliseconds === ASSET_READY_TIMEOUT_MS, "asset timeout changed unexpectedly");
  assert(applyFlowSample(element, pending), "asset timeout did not release logical flow");
  assert(pet.src === "walk.png", "asset timeout replaced the last ready artwork");
  FakeLoader.pending.shift().resolve();
  await new Promise(setImmediate);
  assert(pet.src === "work.png", "late matching decode did not commit atomically");
  assert(!applyFlowSample(element, sample(4, 1.7, true, "bad.png")), "new bad asset was ready");
  assert(pet.style.getPropertyValue("--clip-scale") === "1.7", "failed asset metadata leaked early");
  FakeLoader.pending.shift().reject(new Error("decode failed"));
  await new Promise(setImmediate);
  assert(applyFlowSample(element, sample(4, 1.7, true, "bad.png")), "decode failure stalled flow");
  assert(pet.hidden, "decode failure left stale artwork visible");
  const stale = sample(5, 1, true, "stale.png");
  assert(!applyFlowSample(element, stale), "stale request began ready");
  const staleLoader = FakeLoader.pending.shift();
  const staleTimer = timers.findLast((timer) => timer.active);
  const latest = sample(6, 1, true, "latest.png");
  assert(!applyFlowSample(element, latest), "latest request began ready");
  const latestLoader = FakeLoader.pending.shift();
  staleTimer.callback();
  assert(!applyFlowSample(element, latest), "stale timeout released the latest request");
  staleLoader.resolve();
  await new Promise(setImmediate);
  assert(pet.src !== "stale.png", "stale decode replaced the latest request");
  fireLatestTimer();
  assert(applyFlowSample(element, latest), "latest timeout did not release flow");
  latestLoader.resolve();
  await new Promise(setImmediate);
  assert(pet.src === "latest.png", "latest late decode did not commit");
  console.log("renderer asset transition checks: pass");
})().catch((error) => { console.error(error); process.exitCode = 1; });
CHECK_RENDERER
node "$TMP/check-renderer.cjs"
node - "$ROOT" "$TMP/flow-runtime.js" <<'CHECK_PACKS'
const fs = require("node:fs");
const path = require("node:path");
const root = process.argv[2];
const { compileBehaviorPack } = require(process.argv[3]);
const petsDirectory = path.join(root, "src/pets");
const petDirectories = fs.readdirSync(petsDirectory, { withFileTypes: true })
  .filter((entry) => entry.isDirectory())
  .map((entry) => entry.name)
  .sort();
if (petDirectories.length === 0) throw new Error("expected at least one bundled pet pack");
function collectAssets(directory, current = directory) {
  return Object.fromEntries(
    fs.readdirSync(current, { withFileTypes: true }).flatMap((entry) => {
      const absolute = path.join(current, entry.name);
      if (entry.isDirectory()) return Object.entries(collectAssets(directory, absolute));
      if (!entry.isFile() || !entry.name.endsWith(".png")) return [];
      return [[path.relative(directory, absolute).split(path.sep).join("/"), entry.name]];
    }),
  );
}
const workingFlowSignatures = new Set();
for (const pet of petDirectories) {
  const directory = path.join(petsDirectory, pet);
  const manifest = JSON.parse(fs.readFileSync(path.join(directory, "flow.json"), "utf8"));
  const assets = collectAssets(directory);
  const result = compileBehaviorPack(manifest, assets);
  if (!result.pack) throw new Error(`${pet}/flow.json: ${JSON.stringify(result.diagnostics)}`);
  if (result.pack.id !== pet) throw new Error(`${pet}/flow.json: pack id must match its folder`);
  if (result.pack.states.idle.flow.type !== "hide") throw new Error(`${pet}/flow.json: idle visibility is not flow-authored`);
  workingFlowSignatures.add(JSON.stringify(result.pack.states.working.flow));
}
if (workingFlowSignatures.size !== petDirectories.length) throw new Error("working pet flows are not distinct");
console.log("bundled pet pack checks: pass");
CHECK_PACKS
python3 scripts/check-apng-assets.py
