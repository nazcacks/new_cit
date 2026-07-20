const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");
const i18n = fs.readFileSync("frontend/app/i18n.js", "utf8");
const css = fs.readFileSync("frontend/app.css", "utf8");

function bodyOf(name, prefix = "function") {
  const start = screens.indexOf(`${prefix} ${name}`);
  assert(start >= 0, `${name} must exist`);
  const nextAsync = screens.indexOf("\nasync function ", start + 1);
  const nextPlain = screens.indexOf("\nfunction ", start + 1);
  const candidates = [nextAsync, nextPlain].filter((index) => index > start);
  const next = candidates.length ? Math.min(...candidates) : undefined;
  return screens.slice(start, next);
}

const loader = bodyOf("loadStdFsWorkbenchData(root)", "async function");
const mappingScreen = bodyOf("renderWorkInfoAccountMapping(env)", "async function");
const legacyWorkbench = bodyOf("renderWorkInfo(env)", "async function");
const support = [
  "renderDualMappingTabs(locale, active = \"tax\")",
  "renderTaxAccountMappingPanel(data, locale, hidden = false)",
  "renderStdFsMappingPanel(data, locale, hidden = true)",
  "renderStdFsWorkbench(data, locale)",
  "renderStdFsMetrics(stdFs, locale)",
  "renderStdFsStatementTabs(stdFs, locale)",
  "renderStdFsStatementTable(lines, locale, label)",
  "renderStdFsValidation(validation, locale)",
  "bindDualMappingTabs(container = document)",
  "activateMappingTab(tab, container = document)",
  "activateStdFsStatementTab(stmtType, container = document)",
  "bindStdFsWorkbenchActions(env, data, rerender)",
].map((name) => bodyOf(name)).join("\n");
const combined = `${loader}\n${mappingScreen}\n${legacyWorkbench}\n${support}`;

for (const endpoint of [
  'request(`${root}/std-fs/mappings`)',
  'request(`${root}/std-fs/statements?stmtType=STD_BS`)',
  'request(`${root}/std-fs/statements?stmtType=STD_IS`)',
  'request(`${root}/std-fs/validate`)',
  'request(`${root}/std-fs/${action}`',
  'request(`${root}/std-fs/mappings/${encodeURIComponent(accountCode)}`',
]) {
  assert(combined.includes(endpoint), `std-fs workbench must call ${endpoint}`);
}

for (const snippet of [
  'data-dual-mapping-tabs',
  'data-mapping-tab-button="tax"',
  'data-mapping-tab-button="std-fs"',
  'data-mapping-tab="tax"',
  'data-mapping-tab="std-fs"',
  'id="stdFsMappingForm"',
  'id="stdFsActionResult"',
  'data-std-fs-action="aggregate"',
  'data-std-fs-action="confirm"',
  'data-std-fs-stmt-tab-button="STD_BS"',
  'data-std-fs-stmt-tab-button="STD_IS"',
  'data-std-fs-empty',
  'data-std-fs-error',
  'data-std-fs-validation-empty',
  'renderStdFsWorkbench(data, locale)',
  'bindStdFsWorkbenchActions(env, data, () => renderWorkInfoAccountMapping(env))',
  'bindStdFsWorkbenchActions(env, mappingData, () => renderWorkInfo(env))',
]) {
  assert(combined.includes(snippet), `std-fs workbench must include ${snippet}`);
}

for (const ruleCode of [
  "CHK_STDBS",
  "CHK_STDFS",
  "stdFsValidation",
  "stdFsMappingSaved",
  "aggregateStdFs",
  "confirmStdFs",
]) {
  assert(i18n.includes(ruleCode), `${ruleCode} i18n/support text must exist`);
}

for (const snippet of [
  ".std-fs-table .amount-cell",
  ".std-fs-table .std-fs-subtotal td",
  ".mapping-tabs",
  ".std-fs-tabs",
]) {
  assert(css.includes(snippet), `${snippet} CSS must exist`);
}

assert(screens.includes('if (source.includes("std_fs")'), "std-fs validation issues must jump to mapping");
assert(screens.includes('if (source === "std-fs") return "ws/info:mapping";'), "std-fs source must route to mapping leaf");

console.log("frontend std_fs_workbench.spec.js passed");
