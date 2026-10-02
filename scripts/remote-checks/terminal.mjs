import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createContext, SourceTextModule, SyntheticModule } from "node:vm";
import { visibleTownAgents } from "../../apps/pet-town-3d/src/pet-town/agents/snapshot.js";

const attachments = [];
let closes = 0;
const view = {
  root: { hidden: true, dataset: {}, contains: () => false },
  fields: { screen: { replaceChildren() {} } },
  identity() {},
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
const mocks = {
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
  "./dock-listeners.js": { bindDockListeners() {} },
};
const context = createContext({
  AbortController,
  document: { activeElement: null, hidden: false },
});
const source = await readFile(
  new URL("../../apps/pet-town-3d/src/pet-town/terminal/index.js", import.meta.url),
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
    },
    { id: "local", source: "herdr", status: "working", supportsTerminal: true },
    { id: "ephemeral", source: "herdr", status: "working", supportsTerminal: false },
    { id: "legacy", source: "herdr", status: "working" },
  ],
});
assert.equal(agents.get("remote").remoteMachine, "Build");
assert.equal(agents.get("legacy").supportsTerminal, true);
const dock = module.namespace.createTerminalDock({ context: {}, controller: {}, bridge: {} });
dock.update(agents.get("remote"));
assert.equal(dock.open, false);
assert.equal(attachments.length, 0, "remote selection attempted terminal attachment");
dock.update(agents.get("local"));
assert.equal(dock.open, true);
assert.equal(attachments.length, 1);
assert.equal(attachments[0][0], "local");
const before = closes;
dock.update(agents.get("remote"));
assert.ok(closes > before, "switching to remote did not release the local terminal");
assert.equal(dock.open, false);
dock.update(agents.get("ephemeral"));
assert.equal(attachments.length, 1);
dock.dispose();
console.log("remote terminal capability checks: pass");
