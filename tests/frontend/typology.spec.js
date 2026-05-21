const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");

for (const fn of [
  "renderTypologyGrid",
  "renderTypologyGridTree",
  "renderTypologyDashboard",
  "renderTypologyWizard",
  "renderTypologyForm",
  "renderTypologyChart",
  "renderTypologyDetail",
  "enrichLeafSpec",
  "leafTypology",
]) {
  assert(screens.includes(`function ${fn}`), `${fn} must exist`);
}

assert(screens.includes("TYPOLOGY_RENDERERS"), "typology renderer registry must exist");
assert(/function renderLeafTemplate\(state\)[\s\S]*TYPOLOGY_RENDERERS\[state\.spec\.typology\]/.test(screens), "renderLeafTemplate must dispatch by spec.typology");

for (const typology of ["grid", "grid-tree", "dashboard", "wizard", "form", "chart", "detail"]) {
  assert(screens.includes(`data-typology="${typology}"`), `${typology} must render a data-typology signature`);
}

for (const format of ["money", "bps", "date", "datetime", "biz", "corp", "tags", "status", "severity", "link", "boolean", "progress", "code", "email", "phone", "actions"]) {
  assert(screens.includes(`"${format}"`), `${format} column format must be registered`);
}

for (const mapping of [
  ["dashboard:overview", "TYPOLOGY_DASHBOARD"],
  ["dashboard:kpi-tax", "TYPOLOGY_CHART"],
  ["ws/file:precheck", "TYPOLOGY_WIZARD"],
  ["post/amend:unlock", "TYPOLOGY_FORM"],
  ["ws/start:snapshot", "TYPOLOGY_DETAIL"],
  ["admin/sec:menus", "TYPOLOGY_GRID_TREE"],
]) {
  assert(screens.includes(mapping[0]) && screens.includes(mapping[1]), `${mapping[0]} must be mapped through ${mapping[1]}`);
}

assert(screens.includes("panel-head-actions") && screens.includes("data-leaf-create"), "grid add button must be inline in the panel head");
assert(!screens.includes("leaf-row-actions-panel"), "separate row action panel must not exist");
assert(!screens.includes('data-leaf-block="row-actions"'), "row action block must not render");

console.log("frontend typology.spec.js passed");
