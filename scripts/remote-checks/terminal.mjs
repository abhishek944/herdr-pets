import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createContext, SourceTextModule, SyntheticModule } from "node:vm";
import { visibleTownAgents } from "../../apps/pet-town-3d/src/pet-town/agents/snapshot.js";

async function load(file, mocks, globals) {
  const context = createContext(globals);
  const source = await readFile(new URL(file, import.meta.url), "utf8");
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
  return module.namespace;
}

const attachments = [];
let closes = 0;
let callbacks;
const view = {
  root: { hidden: true, dataset: {}, contains: () => false },
  fields: { screen: { replaceChildren() {} } },
  identity() {},
  showTerminal() {},
  error() {},
  status() {},
  dispose() {},
};
const renderer = {
  dimensions: () => ({ cols: 80, rows: 24 }),
  blur() {},
  dispose() {},
  setInput() {},
};
const { createTerminalDock } = await load(
  "../../apps/pet-town-3d/src/pet-town/terminal/index.js",
  {
    "./view.js": { createTerminalView: () => view },
    "./renderer.js": { createTerminalRenderer: () => renderer },
    "./session.js": {
      createTerminalSession: () => ({
        close() {
          closes++;
        },
        attach: (...arguments_) => attachments.push(arguments_),
        dispose() {},
      }),
    },
    "./dock-listeners.js": {
      bindDockListeners(_view, bound) {
        callbacks = bound;
      },
    },
  },
  { AbortController, document: { activeElement: null, hidden: false } },
);
const agents = visibleTownAgents({
  v: 1,
  type: "snapshot",
  available: true,
  agents: [
    {
      id: "remote",
      source: "herdr",
      status: "working",
      remoteMachine: "Build",
      supportsTerminal: false,
      supportsObserve: true,
    },
    { id: "local", source: "herdr", status: "working", supportsTerminal: true },
    { id: "ephemeral", source: "herdr", status: "working", supportsTerminal: false },
    { id: "legacy", source: "herdr", status: "working" },
  ],
});
assert.equal(agents.get("remote").remoteMachine, "Build");
assert.equal(agents.get("legacy").supportsTerminal, true);

const dock = createTerminalDock({ context: {}, controller: {}, bridge: {} });
// Remote observation is explicit and never grants input or takeover.
dock.update(agents.get("remote"));
assert.equal(dock.open, true);
callbacks.attach(true);
assert.equal(attachments.length, 0, "remote input attempted attachment");
callbacks.attach(false);
assert.equal(attachments.length, 1);
assert.equal(attachments[0][0], "remote");
assert.equal(attachments[0][2], false);
// Local observation/control stay explicit and selection releases the prior stream.
const before = closes;
dock.update(agents.get("local"));
assert.ok(closes > before);
callbacks.attach(false);
assert.equal(attachments.length, 2);
assert.equal(attachments[1][0], "local");
dock.update(agents.get("remote"));
callbacks.attach(false);
assert.equal(attachments.length, 3);
dock.update(agents.get("ephemeral"));
callbacks.attach(false);
assert.equal(attachments.length, 3, "agent without observation support attached");
dock.update({ id: "mayor", source: "mayor", isMayor: true });
assert.equal(dock.open, false);
dock.dispose();

// The profile names the remote action and hides the terminal controls it cannot offer.
const control = () => ({ textContent: "", hidden: false, setAttribute() {} });
const actions = Object.fromEntries(
  ["open", "observe", "interact", "control", "camera", "leave"].map((name) => [
    name,
    Object.assign(control(), { dataset: { action: name } }),
  ]),
);
const hint = control();
const profileRoot = {
  innerHTML: "",
  querySelectorAll: () => Object.values(actions),
  querySelector: (selector) => (selector.includes("hint") ? hint : control()),
};
const { createDockProfile } = await load(
  "../../apps/pet-town-3d/src/pet-town/terminal/profile.js",
  { "./profile.css": {} },
  { document: { createElement: () => profileRoot } },
);
const profile = createDockProfile({ querySelector: () => ({ after() {} }) });
profile.render(agents.get("remote"), { firstPerson: false });
assert.equal(actions.open.textContent, "Select remote pane");
assert.ok(
  !actions.observe.hidden && actions.interact.hidden,
  "remote profile must offer observation without input",
);
assert.match(hint.textContent, /Build/);
profile.render(agents.get("local"), { firstPerson: false });
assert.equal(actions.open.textContent, "Open in Herdr ↗");
assert.ok(!actions.observe.hidden && !actions.interact.hidden);
profile.render(agents.get("ephemeral"), { firstPerson: false });
assert.equal(actions.open.textContent, "Open agent ↗");
assert.ok(actions.observe.hidden && actions.interact.hidden);
console.log("remote terminal capability checks: pass");
