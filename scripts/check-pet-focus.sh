#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TMP=$(mktemp -d "${TMPDIR:-/tmp}/herdr-pets-focus.XXXXXX")
trap 'rm -rf "$TMP"' EXIT INT TERM
cd "$ROOT"

npx esbuild src/pet-focus.ts --bundle --platform=node --format=cjs \
  --log-level=error --outfile="$TMP/pet-focus.cjs"
cat > "$TMP/check.cjs" <<'CHECK'
class FakeElement {
  constructor(classes = [], parent = null, agentId = null) {
    this.classes = new Set(classes);
    this.parent = parent;
    this.dataset = agentId ? { agentId } : {};
  }
  closest(selector) {
    if (this.classes.has(selector.slice(1))) return this;
    return this.parent?.closest(selector) ?? null;
  }
}
global.Element = FakeElement;
const { agentIdForPetClick, installPetFocus } = require("./pet-focus.cjs");
function assert(condition, message) { if (!condition) throw new Error(message); }
const citizen = new FakeElement(["citizen"], null, "session:w1:p2");
const pet = new FakeElement(["pet"], citizen);
const label = new FakeElement(["project"], citizen);
assert(agentIdForPetClick(pet) === "session:w1:p2", "pet did not resolve its agent");
assert(agentIdForPetClick(label) === null, "label unexpectedly focused an agent");
assert(agentIdForPetClick(null) === null, "null target resolved an agent");
let listener = null;
const root = {
  addEventListener(type, next) { if (type === "click") listener = next; },
  removeEventListener(type, next) { if (type === "click" && listener === next) listener = null; },
};
const focused = [];
const dispose = installPetFocus(root, (id) => focused.push(id));
listener({ target: label });
listener({ target: pet });
assert(JSON.stringify(focused) === JSON.stringify(["session:w1:p2"]), "click routing was not pet-only");
dispose();
assert(listener === null, "click listener was not removed");
console.log("pet focus checks: pass");
CHECK
node "$TMP/check.cjs"
