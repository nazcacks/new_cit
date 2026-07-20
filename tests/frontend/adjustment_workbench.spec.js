const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");

for (const snippet of [
  'const adjustmentRunState = new Map();',
  'data-stage="adjustment"',
  'data-adjustment-stage="${escapeHtml(selectedCode)}"',
  'data-module-code="${escapeHtml(selectedCode)}"',
  'data-leaf-key="ws/adj:${escapeHtml(selectedCode)}"',
  "renderAdjustmentModuleLeaf",
  "ensurePhaseFAdjustmentGate(env, moduleCode)",
  "renderAdjustmentModuleNavigator(selectedCode, env.locale)",
  "renderAdjustmentModuleHighlights(selectedModule",
  "renderAdjustmentModuleForm(selectedModule",
  "renderAdjustmentRunSummary(selectedModule, lastRun, env.locale)",
  "const shellContext = {",
  "renderAdjustmentDataContext(shellContext, env.locale)",
  'data-adjustment-route="ws/adj:',
  'data-adjustment-route="ws/form:form3"',
  'data-adjustment-route="ws/form:linkage"',
  'data-adjustment-route="ws/val:issues"',
  "Workflow status",
  "Lock mode",
  "collectAdjustmentPayload(moduleCode)",
  "adjustmentRunState.set(moduleCode, result)",
]) {
  assert(screens.includes(snippet), `adjustment workbench must include ${snippet}`);
}

for (const code of ["B2", "B3", "B5", "B6", "B7", "B8", "B9", "B10", "B11", "B12", "B13", "B14", "B16", "B17"]) {
  assert(screens.includes(`case "${code}":`), `collectAdjustmentPayload must support ${code}`);
}

for (let index = 1; index <= 17; index += 1) {
  assert(screens.includes(`async function renderAdjustmentB${index}(env)`), `renderAdjustmentB${index} screen must exist`);
  assert(screens.includes(`renderAdjustmentModuleLeaf(env, "B${index}")`), `renderAdjustmentB${index} must select B${index}`);
}

for (const inputId of [
  "adjB2TaxableIncome",
  "adjB3GrossRevenue",
  "adjB5BookReserve",
  "adjB6ReceivableBalance",
  "adjB7PositionCode",
  "adjB8PositionCode",
  "adjB9LoanBalance",
  "adjB10BusinessUseBps",
  "adjB11TaxableIncome",
  "adjB12TaxBase",
  "adjB13TaxBase",
  "adjB14PenaltyType",
  "adjB15ChangeType",
  "adjB16IncomeType",
  "adjB17EntityCode",
]) {
  assert(screens.includes(inputId), `${inputId} must exist in adjustment module forms`);
}

console.log("frontend adjustment_workbench.spec.js passed");
