const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");

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
const screenKeys = [...objectBlock("screenByLeaf").matchAll(/^\s*"([^"]+)": \(env\) => renderLeafScreen\(env, "([^"]+)"\)/gm)]
  .map((match) => {
    assert.strictEqual(match[1], match[2], `screenByLeaf registration mismatch for ${match[1]}`);
    return match[1];
  });

assert.strictEqual(routes.length, 99, "leafRoutes must expose 99 active leaves");
assert.strictEqual(specKeys.length, 99, "leafScreenSpecs must expose 99 active leaves");
assert.strictEqual(screenKeys.length, 99, "screenByLeaf must register 99 active leaves");
assert.deepStrictEqual([...specKeys].sort(), [...routes].sort(), "leafScreenSpecs must match active leafRoutes");
assert.deepStrictEqual([...screenKeys].sort(), [...routes].sort(), "screenByLeaf must match active leafRoutes");

for (const key of routes) {
  const escaped = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  assert(new RegExp(`"${escaped}": leafSpec\\("`, "m").test(screens), `${key} missing primary API spec`);
  assert(new RegExp(`"${escaped}": \\(env\\) => renderLeafScreen\\(env, "${escaped}"\\)`, "m").test(screens), `${key} missing screenByLeaf registration`);
}

for (const needle of [
  "data-leaf-key",
  "data-primary-api",
  "data-action-api",
  "data-leaf-action",
  "empty-state",
  "leafGate",
  "request(primaryApi",
  "request(actionApi",
]) {
  assert(screens.includes(needle), `${needle} is required for v1.3 leaf screens`);
}

console.log("frontend leaf_screens.spec.js passed");
