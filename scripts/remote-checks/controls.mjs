import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createContext, SourceTextModule, SyntheticModule } from "node:vm";

const remote = { id: "remote", source: "herdr", label: "Build: project", remoteMachine: "Build" };
const local = { id: "local", source: "herdr", label: "project" };
const records = new Map([
  [remote.id, remote],
  [local.id, local],
]);
const elements = new Map();
const root = {
  querySelector(selector) {
    if (!elements.has(selector))
      elements.set(selector, {
        textContent: "",
        hidden: false,
        value: "remote",
        options: [{ value: "" }, { value: "remote" }, { value: "local" }],
        querySelector: () => ({}),
        setAttribute() {},
        replaceChildren() {},
      });
    return elements.get(selector);
  },
};
const requests = [];
const town = {
  agents: { records },
  controller: { selected: remote },
  bridge: {
    action: (action, payload) =>
      new Promise((resolve) => {
        requests.push({ action, payload, resolve });
      }),
  },
};
const mocks = {
  "./pet-gallery.js": { bindPetGallery: () => ({ render() {}, setVisible() {} }) },
  "../../pet-town/ui/portrait.js": {
    createPortrait: () => ({}),
    companionNames: () => new Map([...records].map(([id, agent]) => [id, agent.label])),
  },
};
const context = createContext({ document: { activeElement: null } });
const source = await readFile(
  new URL("../../apps/pet-town-3d/src/hud/settings/companion-controls.js", import.meta.url),
  "utf8",
);
const module = new SourceTextModule(source, { context });
await module.link((specifier) => {
  const exports = mocks[specifier];
  return new SyntheticModule(
    Object.keys(exports),
    function () {
      for (const [name, value] of Object.entries(exports)) this.setExport(name, value);
    },
    { context },
  );
});
await module.evaluate();
const controls = module.namespace.bindCompanionControls(root, { petTown: town });
const button = root.querySelector('[data-control="focus"]');
const feedback = root.querySelector('[data-field="feedback"]');
const hint = root.querySelector('[data-field="remoteHint"]');
controls.render();
assert.equal(button.textContent, "Select remote pane");
assert.equal(hint.hidden, false);
assert.match(hint.textContent, /select “Build”/);
const action = button.onclick();
assert.equal(button.textContent, "Selecting…");
assert.equal(button.disabled, true);
assert.equal(requests.length, 1);
assert.equal(requests[0].action, "focusAgent");
assert.equal(requests[0].payload.id, "remote");
requests.shift().resolve();
await action;
assert.equal(feedback.hidden, false);
assert.equal(feedback.textContent, "Pane selected. Open Herdr and select “Build” to view it.");
town.controller.selected = local;
controls.render();
assert.equal(button.textContent, "Open in Herdr ↗");
assert.equal(hint.hidden, true);
console.log("remote companion action checks: pass");
