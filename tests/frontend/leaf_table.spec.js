const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");
const menu = fs.readFileSync("frontend/app/menu.js", "utf8");
const app = fs.readFileSync("frontend/app.js", "utf8");

for (const fn of [
  "renderLeafTemplate",
  "renderLeafSummaryBlock",
  "renderLeafTableBlock",
  "renderLeafTableShell",
  "renderLeafTableRows",
  "renderLeafPrimaryRowAction",
  "renderLeafRowActions",
  "bindLeafTemplate",
  "openEditModal",
  "readLeafFormValues",
  "selectLeafCustomer",
  "selectLeafBusinessYear",
  "updateLeafRow",
  "deleteLeafRow",
]) {
  assert(screens.includes(`function ${fn}`), `${fn} must exist`);
}

for (const block of ["summary", "filters", "table", "row-actions", "toolbar"]) {
  assert(screens.includes(`data-leaf-block="${block}"`), `${block} block must render`);
}

assert(screens.includes("/leaf-records"), "leaf CRUD must use persistent leaf-records API");
assert(screens.includes("refreshLeafRows"), "CRUD actions must refresh the table without page reload");
assert(!screens.includes("location.reload"), "leaf CRUD must not reload the page");
assert(!screens.includes("leaf-row-actions-panel"), "separate row action panel must be removed");
assert(!screens.includes("leaf-toolbar"), "separate toolbar panel must be removed");
assert(screens.includes("panel-head-actions"), "grid controls must be inline in panel head");
assert(screens.includes("row-actions-th"), "table must keep a single management column");
assert(screens.includes('data-leaf-row-action="select-customer"'), "customer-pick must expose a customer selection row action");
assert(screens.includes('data-leaf-row-action="select-by"'), "by-pick must expose a business-year selection row action");
assert(screens.includes('env.navigate("ws/start:by-pick"'), "customer selection must navigate to business-year pick");
assert(screens.includes('env.navigate("ws/info:fs"'), "business-year selection must navigate to the first work leaf");
assert(menu.includes("menu-progress-dot"), "sidebar group progress dot must render");
assert(menu.includes("groupProgress("), "sidebar group progress must update from context");
assert(menu.includes("renderStepper"), "workspace stepper renderer must exist");
assert(menu.includes("renderStateBadge"), "state pill renderer must exist");
assert(app.includes("renderStateBadge"), "topbar state pill must be refreshed");
assert(screens.includes("renderLawBanner"), "law banner renderer must exist");
assert(screens.includes("appendNextStepCard"), "workspace next step card must render");

console.log("frontend leaf_table.spec.js passed");
