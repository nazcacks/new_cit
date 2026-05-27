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
const workflowRendererContract = Object.fromEntries(
  [...objectBlock("workflowLeafRendererContract").matchAll(/^\s*"([^"]+)": "([^"]+)"/gm)]
    .map((match) => [match[1], match[2]])
);
const dashboardRendererContract = {
  "dashboard:overview": "renderDashboard",
  "dashboard:duesoon": "renderDashboard",
};
const adminRendererContract = {
  "admin/tenant:list": "renderAdminTenantLeaf",
  "admin/cust:list": "renderAdminCustomers",
  "admin/cust:by-master": "renderAdminCustomers",
  "admin/cust:agent": "renderAdminCustomers",
  "admin/sec:users": "renderAdminRoles",
  "admin/sec:roles": "renderAdminRoles",
  "admin/sec:matrix": "renderAdminRoles",
  "admin/sec:menus": "renderAdminMenus",
  "admin/sec:functions": "renderAdminMenus",
  "admin/sec:mask": "renderAdminRoles",
  "admin/sec:scope": "renderAdminRoles",
  "admin/cacc:assign": "renderAdminCustomerAccess",
  "admin/cacc:groups": "renderAdminCustomerAccess",
  "admin/cacc:rules": "renderAdminCustomerAccess",
  "admin/cacc:delegate": "renderAdminCustomerAccess",
  "admin/cacc:override": "renderAdminCustomerAccess",
  "admin/law:master": "renderAdminLaw",
  "admin/law:rates": "renderAdminLaw",
  "admin/law:limits": "renderAdminLaw",
  "admin/law:credits": "renderAdminLaw",
  "admin/law:depr-lives": "renderAdminLaw",
  "admin/law:sme": "renderAdminLaw",
  "admin/law:loss-rule": "renderAdminLaw",
  "admin/law:snapshots": "renderAdminLaw",
  "admin/law:impact": "renderAdminLaw",
  "admin/law:history": "renderAdminLaw",
  "admin/form:master": "renderAdminForms",
  "admin/form:versions": "renderAdminForms",
  "admin/form:fields": "renderAdminForms",
  "admin/form:validations": "renderAdminForms",
  "admin/form:linkage-rule": "renderAdminForms",
  "admin/form:migration": "renderAdminForms",
  "admin/form:efile-map": "renderAdminForms",
  "admin/form:by-set": "renderAdminForms",
  "admin/form:impact": "renderAdminForms",
  "admin/code:manage": "renderLeafScreen",
  "admin/audit:events": "renderAdminAudit",
  "admin/audit:login": "renderAdminAudit",
  "admin/audit:perm": "renderAdminAudit",
  "admin/audit:settings": "renderAdminAudit",
};
const screenKeys = [...objectBlock("screenByLeaf").matchAll(/^\s*"([^"]+)": \(env\) => (.+)$/gm)]
  .map((match) => {
    if (workflowRendererContract[match[1]]) {
      assert(
        match[2].includes(`${workflowRendererContract[match[1]]}(env)`),
        `workflow leaf ${match[1]} must use ${workflowRendererContract[match[1]]}`
      );
      assert(!match[2].includes("renderLeafScreen"), `workflow leaf ${match[1]} must not use generic renderer`);
    } else if (dashboardRendererContract[match[1]]) {
      assert(
        match[2].includes(`${dashboardRendererContract[match[1]]}(env)`),
        `dashboard leaf ${match[1]} must use ${dashboardRendererContract[match[1]]}`
      );
      assert(!match[2].includes("renderLeafScreen"), `dashboard leaf ${match[1]} must not use generic renderer`);
    } else if (adminRendererContract[match[1]] && adminRendererContract[match[1]] !== "renderLeafScreen") {
      assert(
        match[2].includes(`${adminRendererContract[match[1]]}(env)`),
        `admin leaf ${match[1]} must use ${adminRendererContract[match[1]]}`
      );
      assert(!match[2].includes("renderLeafScreen"), `admin leaf ${match[1]} must not use generic renderer`);
    } else if (match[1] !== "admin/tenant:list") {
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
assert.strictEqual(Object.keys(workflowRendererContract).length, 49, "workflow renderer contract must cover 49 core workflow/post leaves");

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
  "data-leaf-block=\"row-actions\"",
  "data-leaf-block=\"toolbar\"",
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
]) {
  assert(!frontendSources.includes(needle), `${needle} must not be rendered on leaf or API response screens`);
}

console.log("frontend leaf_screens.spec.js passed");
