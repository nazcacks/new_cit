const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");
const i18n = fs.readFileSync("frontend/app/i18n.js", "utf8");

function bodyOf(name) {
  const start = screens.indexOf(`async function ${name}(env)`);
  assert(start >= 0, `${name} must exist`);
  const next = screens.indexOf("\nasync function ", start + 1);
  return screens.slice(start, next > start ? next : undefined);
}

const workStart = bodyOf("renderWorkStart");

for (const snippet of [
  'data-stage="work-start"',
  'request(`${root}/customers`)',
  'request(`${root}/business-years`)',
  'id="workStartSearch"',
  'data-select-by',
  'id="customerForm"',
  'request(`${root}/customers`, {',
  'id="businessYearForm"',
  'id="byCarryForward"',
  'id="byCarryForwardSource"',
  'id="snapshotPreview"',
  "refreshContextFromBy(env, by, customer)",
  'env.navigate("ws-info"',
]) {
  assert(workStart.includes(snippet), `renderWorkStart must include ${snippet}`);
}

assert(screens.includes("syncCarryForwardOptions()"), "work-start screen must synchronize carryforward candidates");
assert(screens.includes('"carry_forward_from_by_id"') || screens.includes("carry_forward_from_by_id:"), "business-year creation must submit carry-forward source");

for (const key of [
  "workStart.title",
  "workStart.recent",
  "workStart.customerSearch",
  "workStart.newCustomer",
  "workStart.newBusinessYear",
  "workStart.carryForward",
  "workStart.carryForwardSource",
  "workStart.snapshotPreview",
  "workStart.selectWork",
]) {
  assert(i18n.includes(`"${key}"`), `${key} must exist in i18n.js`);
}

assert(
  screens.includes('"ws/start:customer-pick": (env) => renderWorkStartLeaf(env)') &&
    screens.includes('"ws/start:by-pick": (env) => renderWorkStartLeaf(env)') &&
    screens.includes('"ws/start:snapshot": (env) => renderWorkStartLeaf(env)'),
  "all work-start active routes must dispatch to the work-start renderer"
);

console.log("frontend work_start.spec.js passed");
