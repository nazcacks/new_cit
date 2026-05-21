const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");
const api = fs.readFileSync("frontend/app/api.js", "utf8");
const frontendSources = `${screens}\n${api}`;

function objectBlock(name) {
  const match = screens.match(new RegExp(`export const ${name} = Object\\.freeze\\(\\{([\\s\\S]*?)\\n\\}\\);`));
  assert(match, `${name} block is missing`);
  return match[1];
}

function leafRouteKeys() {
  const block = objectBlock("leafRoutes");
  return [...block.matchAll(/\["([^"]+)"/g)]
    .map((match) => match[1])
    .map((key) => (/^B\d+$/.test(key) ? `ws/adj:${key}` : key));
}

const routes = leafRouteKeys();
const specKeys = [...objectBlock("leafScreenSpecs").matchAll(/^\s*"([^"]+)": leafSpec\(/gm)].map((match) => match[1]);
const screenKeys = [...objectBlock("screenByLeaf").matchAll(/^\s*"([^"]+)": \(env\) => (.+)$/gm)]
  .map((match) => {
    if (match[1] !== "admin/tenant:list") {
      assert(match[2].includes(`renderLeafScreen(env, "${match[1]}")`), `screenByLeaf registration mismatch for ${match[1]}`);
    } else {
      assert(match[2].includes("renderAdminTenantLeaf(env)"), "admin/tenant:list must use tenant CRUD renderer");
    }
    return match[1];
  });

assert.strictEqual(routes.length, 100, "leafRoutes must expose 100 active leaves");
assert.strictEqual(specKeys.length, 100, "leafScreenSpecs must expose 100 active leaves");
assert.strictEqual(screenKeys.length, 100, "screenByLeaf must register 100 active leaves");
assert.deepStrictEqual([...specKeys].sort(), [...routes].sort(), "leafScreenSpecs must match active leafRoutes");
assert.deepStrictEqual([...screenKeys].sort(), [...routes].sort(), "screenByLeaf must match active leafRoutes");

for (const key of routes) {
  const escaped = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  assert(new RegExp(`"${escaped}": leafSpec\\("`, "m").test(screens), `${key} missing primary API spec`);
  assert(new RegExp(`"${escaped}": \\(env\\) =>`, "m").test(screens), `${key} missing screenByLeaf registration`);
}

for (const needle of [
  "data-leaf-key",
  "data-primary-api",
  "data-action-api",
  "data-leaf-block=\"summary\"",
  "data-leaf-block=\"filters\"",
  "data-leaf-block=\"table\"",
  "data-typology=\"grid\"",
  "data-typology=\"grid-tree\"",
  "data-typology=\"dashboard\"",
  "data-typology=\"wizard\"",
  "data-typology=\"form\"",
  "data-typology=\"chart\"",
  "data-typology=\"detail\"",
  "data-leaf-create",
  "data-leaf-row-action",
  "data-row-edit",
  "empty-state",
  "leafGate",
  "request(primaryApi",
  "loadLeafRecords",
  "request(state.actionApi",
]) {
  assert(screens.includes(needle), `${needle} is required for v1.5 leaf screens`);
}

for (const needle of [
  "leaf-api-summary",
  "leaf-response",
  "1차 API 응답",
  "응답 데이터",
  "jsonBlock",
  "jsonBlock(payload)",
  "<pre",
  "data-leaf-action",
  "기능 실행",
  "leaf-row-actions-panel",
  "data-leaf-block=\"row-actions\"",
]) {
  assert(!frontendSources.includes(needle), `${needle} must not be rendered on leaf or API response screens`);
}

console.log("frontend leaf_screens.spec.js passed");
