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
  "dashboard:duesoon": "renderDashboardDueSoon",
  "dashboard:inbox": "renderDashboardInbox",
  "dashboard:recent": "renderDashboardRecent",
  "dashboard:kpi-tax": "renderDashboardKpiTax",
};
const reportRendererContract = {
  "report:year-compare": "renderYearCompare",
  "report:tax-burden": "renderTaxBurden",
  "report:reserve-trend": "renderReserveTrend",
  "report:loss-expiry": "renderLossExpiryReport",
  "report:industry-stats": "renderIndustryStatsReport",
  "report:custom": "renderCustomReports",
};
const adminRendererContract = {
  "admin/tenant:list": "renderAdminTenantLeaf",
  "admin/cust:list": "renderAdminCustomerList",
  "admin/cust:by-master": "renderAdminBusinessYearMaster",
  "admin/cust:agent": "renderAdminTaxAgentContracts",
  "admin/sec:users": "renderAdminUsers",
  "admin/sec:roles": "renderAdminRoleCatalog",
  "admin/sec:matrix": "renderAdminPermissionMatrix",
  "admin/sec:menus": "renderAdminMenuManagement",
  "admin/sec:functions": "renderAdminFunctionCatalog",
  "admin/sec:mask": "renderAdminFieldMasking",
  "admin/sec:scope": "renderAdminDataScope",
  "admin/cacc:assign": "renderAdminCustomerAssignment",
  "admin/cacc:groups": "renderAdminCustomerGroups",
  "admin/cacc:rules": "renderAdminCustomerRules",
  "admin/cacc:delegate": "renderAdminCustomerDelegation",
  "admin/cacc:override": "renderAdminCustomerOverrides",
  "admin/law:master": "renderAdminLawMaster",
  "admin/law:rates": "renderAdminTaxRates",
  "admin/law:limits": "renderAdminLawLimits",
  "admin/law:credits": "renderAdminLawCredits",
  "admin/law:depr-lives": "renderAdminLawDepreciationLives",
  "admin/law:sme": "renderAdminLawSmeCriteria",
  "admin/law:loss-rule": "renderAdminLawLossRules",
  "admin/law:snapshots": "renderAdminLawSnapshots",
  "admin/law:impact": "renderAdminLawImpact",
  "admin/law:history": "renderAdminLawHistory",
  "admin/form:master": "renderAdminFormMaster",
  "admin/form:versions": "renderAdminFormVersions",
  "admin/form:fields": "renderAdminFormFields",
  "admin/form:validations": "renderAdminFormValidations",
  "admin/form:linkage-rule": "renderAdminFormLinkageRules",
  "admin/form:migration": "renderAdminFormMigration",
  "admin/form:efile-map": "renderAdminFormEfileMap",
  "admin/form:by-set": "renderAdminFormBySet",
  "admin/form:impact": "renderAdminFormImpact",
  "admin/code:manage": "renderAdminCodes",
  "admin/audit:events": "renderAdminAuditEvents",
  "admin/audit:login": "renderAdminLoginHistory",
  "admin/audit:perm": "renderAdminPermissionChangeHistory",
  "admin/audit:settings": "renderAdminSystemSettingsAudit",
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
    } else if (reportRendererContract[match[1]]) {
      assert(
        match[2].includes(`${reportRendererContract[match[1]]}(env)`),
        `report leaf ${match[1]} must use ${reportRendererContract[match[1]]}`
      );
      assert(!match[2].includes("renderLeafScreen"), `report leaf ${match[1]} must not use generic renderer`);
    } else if (adminRendererContract[match[1]] && adminRendererContract[match[1]] !== "renderLeafScreen") {
      assert(
        match[2].includes(`${adminRendererContract[match[1]]}(env)`),
        `admin leaf ${match[1]} must use ${adminRendererContract[match[1]]}`
      );
      assert(!match[2].includes("renderLeafScreen"), `admin leaf ${match[1]} must not use generic renderer`);
    } else {
      assert.fail(`leaf ${match[1]} must be assigned to a dedicated renderer`);
    }
    return match[1];
  });

assert(!objectBlock("screenByLeaf").includes("renderLeafScreen"), "screenByLeaf must not dispatch active menu leaves to the generic renderer");

for (const signature of [
  'data-dashboard="duesoon"',
  'data-dashboard="inbox"',
  'data-dashboard="recent"',
  'data-dashboard="kpi-tax"',
  'data-admin-stage="security-users"',
  'data-admin-stage="security-roles"',
  'data-admin-stage="security-matrix"',
  'data-admin-stage="security-mask"',
  'data-admin-stage="security-scope"',
  'data-admin-stage="customer-list"',
  'data-admin-stage="business-year-master"',
  'data-admin-stage="tax-agent-contracts"',
  'data-admin-stage="menu-management"',
  'data-admin-stage="function-catalog"',
  'data-admin-stage="customer-access-assign"',
  'data-admin-stage="customer-access-groups"',
  'data-admin-stage="customer-access-rules"',
  'data-admin-stage="customer-access-delegate"',
  'data-admin-stage="customer-access-override"',
  'data-admin-stage="law-master"',
  'data-admin-stage="law-rates"',
  'stage: "law-limits"',
  'stage: "law-credits"',
  'stage: "law-depr-lives"',
  'stage: "law-sme"',
  'stage: "law-loss-rule"',
  'data-admin-stage="law-snapshots"',
  'data-admin-stage="law-impact"',
  'data-admin-stage="law-history"',
  'data-admin-stage="form-master"',
  'data-admin-stage="form-versions"',
  'data-admin-stage="form-fields"',
  'data-admin-stage="form-validations"',
  'data-admin-stage="form-linkage-rule"',
  'data-admin-stage="form-migration"',
  'data-admin-stage="form-efile-map"',
  'data-admin-stage="form-by-set"',
  'data-admin-stage="form-impact"',
  'data-admin-stage="audit-events"',
  'data-admin-stage="audit-login"',
  'data-admin-stage="audit-permission"',
  'data-admin-stage="audit-settings"',
  'data-report-leaf="year-compare"',
  'data-report-leaf="tax-burden"',
  'data-report-leaf="reserve-trend"',
  'data-report-leaf="loss-expiry"',
  'data-report-leaf="industry-stats"',
  'data-report-leaf="custom"',
  'data-admin-stage="codes"',
]) {
  assert(screens.includes(signature), `${signature} must identify a dedicated menu screen`);
}

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
