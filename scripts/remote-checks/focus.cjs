const assert = require("node:assert/strict");
const { createAgentFocus } = require(process.argv[2]);
let notice;
global.document = { createElement: () => ({ setAttribute() {} }) };
global.setTimeout = () => 0;
global.clearTimeout = () => {};
const root = {
  append(element) {
    notice = element;
  },
};
const requests = [];
const focus = createAgentFocus(
  root,
  (id) =>
    new Promise((resolve, reject) => {
      requests.push({ id, resolve, reject });
    }),
);

(async () => {
  const remote = focus("remote");
  assert.equal(notice.hidden, true, "remote focus displayed a progress bubble");
  await focus("remote");
  assert.equal(requests.length, 1, "rapid clicks started duplicate helpers");
  requests.shift().resolve();
  await remote;
  assert.equal(notice.hidden, true, "remote focus displayed a success bubble");

  const local = focus("local");
  assert.equal(notice.hidden, true);
  requests.shift().resolve();
  await local;
  assert.equal(notice.hidden, true, "local focus displayed a success bubble");

  const failed = focus("remote");
  requests.shift().reject("agent is no longer available");
  await failed;
  assert.equal(notice.textContent, "agent is no longer available");
  assert.equal(notice.hidden, false);

  const outdated = focus("remote");
  assert.equal(notice.hidden, true, "a new interaction did not dismiss the previous error");
  const current = focus("local");
  requests.shift().reject("outdated error");
  await outdated;
  assert.equal(notice.hidden, true, "an older remote error replaced the current interaction");
  requests.shift().resolve();
  await current;
  console.log("remote pet focus checks: pass");
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
