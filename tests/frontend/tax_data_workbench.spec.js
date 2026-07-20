const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");
const i18n = fs.readFileSync("frontend/app/i18n.js", "utf8");

function bodyOf(name, prefix = "async function") {
  const start = screens.indexOf(`${prefix} ${name}`);
  assert(start >= 0, `${name} must exist`);
  const nextAsync = screens.indexOf("\nasync function ", start + 1);
  const nextPlain = screens.indexOf("\nfunction ", start + 1);
  const candidates = [nextAsync, nextPlain].filter((index) => index > start);
  const next = candidates.length ? Math.min(...candidates) : undefined;
  return screens.slice(start, next);
}

const workInfo = bodyOf("renderWorkInfo(env)");
const mappingSupport = [
  "renderTaxAccountMappingPanel(data, locale, hidden = false)",
  "renderStdFsMappingPanel(data, locale, hidden = true)",
].map((name) => bodyOf(name, "function")).join("\n");
const workInfoAndSupport = `${workInfo}\n${mappingSupport}`;
const dedicatedTaxData = [
  "renderWorkInfoFinancialStatements",
  "renderWorkInfoAccountMapping",
  "renderWorkInfoAssets",
  "renderWorkInfoTransactions",
  "renderWorkInfoVehicleUsage",
  "renderWorkInfoConsistency",
].map((name) => bodyOf(`${name}(env)`)).join("\n");
const taxDataSupport = `${bodyOf("taxDataHeader(env, activeLeaf, title, description, validation)", "function")}\n${bodyOf("taxDataRouteForSource(source)", "function")}`;
const dedicatedTaxDataAndSupport = `${dedicatedTaxData}\n${taxDataSupport}`;

for (const snippet of [
  'data-workbench="tax-data"',
  'request(`${root}/tax-data/validation`)',
  'request(`${root}/tax-data/financial-statements`)',
  'request(`${root}/tax-data/assets`)',
  'request(`${root}/tax-data/transactions`)',
  'request(`${root}/vehicle-usage-logs`)',
  'request(`${root}/tax-data/import-batches`)',
  'data-tax-template="financial-statements"',
  "downloadBinary(`${routeRoot(env)}/tax-data/templates/",
  'id="importForm"',
  'data-import-errors',
  "/errors",
  'id="mappingForm"',
  "/account-mappings",
  'id="vehicleLogForm"',
  "/vehicle-usage-logs",
  "data-tax-tab-button",
  "data-tax-tab",
  "data-source-jump",
  'id="taxDataComplete"',
  "runPhaseFValidationGate(env, root, { navigateOnPass: true })",
]) {
  assert(workInfoAndSupport.includes(snippet), `renderWorkInfo must include ${snippet}`);
}

assert(screens.includes("function activateTaxDataTab(tab)"), "tax-data tabs must have an activation helper");
assert(screens.includes("function sourceTabForIssue(issue)"), "validation issue source jump helper must exist");

for (const snippet of [
  'data-tax-data-stage="financial-statements"',
  'data-tax-data-stage="account-mapping"',
  'data-tax-data-stage="assets"',
  'data-tax-data-stage="transactions"',
  'data-tax-data-stage="vehicle-usage"',
  'data-tax-data-stage="consistency"',
  'data-leaf-key="ws/info:fs"',
  'data-leaf-key="ws/info:mapping"',
  'data-leaf-key="ws/info:assets"',
  'data-leaf-key="ws/info:transactions"',
  'data-leaf-key="ws/info:vehicle"',
  'data-leaf-key="ws/info:consistency"',
  'renderStageRouteButtons(activeLeaf, TAX_DATA_ROUTES, locale)',
  'env.navigate(taxDataRouteForSource(button.dataset.sourceJump))',
]) {
  assert(dedicatedTaxDataAndSupport.includes(snippet), `dedicated tax-data screens must include ${snippet}`);
}

for (const dispatch of [
  '"ws/info:fs": (env) => renderWorkInfoFinancialStatements(env)',
  '"ws/info:mapping": (env) => renderWorkInfoAccountMapping(env)',
  '"ws/info:assets": (env) => renderWorkInfoAssets(env)',
  '"ws/info:transactions": (env) => renderWorkInfoTransactions(env)',
  '"ws/info:vehicle": (env) => renderWorkInfoVehicleUsage(env)',
  '"ws/info:consistency": (env) => renderWorkInfoConsistency(env)',
]) {
  assert(screens.includes(dispatch), `${dispatch} must dispatch to a dedicated tax-data screen`);
}

for (const key of [
  "taxData.title",
  "taxData.templates",
  "taxData.upload",
  "taxData.importHistory",
  "taxData.issueDrilldown",
  "taxData.mapping",
  "taxData.vehicleEditor",
  "taxData.consistency",
  "taxData.sourceJump",
  "taxData.downloadTemplate",
  "taxData.completeInput",
]) {
  assert(i18n.includes(`"${key}"`), `${key} must exist in i18n.js`);
}

console.log("frontend tax_data_workbench.spec.js passed");
