const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");

for (const snippet of [
  "const validationRunState = new Map();",
  'data-stage="forms"',
  'data-stage="validation"',
  'data-stage="approval"',
  'data-stage="print"',
  'data-stage="efiling"',
  'data-stage="post-history"',
  'data-stage="post-amend"',
  "renderStageRouteButtons(activeLeaf, stageRoutes, env.locale)",
  'data-form-code="${escapeHtml(selectedFormCode)}"',
  'data-form-pdf="${escapeHtml(selectedFormCode)}"',
  'data-form-edit-field="${escapeHtml(field.field_path)}"',
  'data-validation-jump="${escapeHtml(validationIssueLeaf(issue))}"',
  'data-dismiss-issue="${escapeHtml(issue.issue_id)}"',
  'data-download-form="${escapeHtml(item.form_code)}"',
  'data-efile-jump="${escapeHtml(efilingIssueLeaf(issue))}"',
  'data-download-efile="${item.efiling_id}"',
  'data-open-amend="${escapeHtml(by.by_id)}"',
  'request(`${root}/forms/linkage-check`)',
  'request(`${root}/forms/${selectedFormCode}/preview`)',
  'request(`${root}/validation/issues`)',
  'request(`${root}/workflow/request`',
  'request(`${root}/forms/print-history`)',
  'request(`${root}/efilings/latest`)',
  'request(`${root}/efilings/${latestHistory.efiling_id}/submit`',
  'request(`${root}/amendment-version-mode`)',
  'request(`${root}/resubmit`',
  'downloadBinary(`${root}/forms/pdf-bundle/download`, "forms.zip")',
  'downloadBinary(`${routeRoot(env)}/efilings/${button.dataset.downloadEfile}/file`',
  'env.navigate("ws/appr:request")',
  'env.navigate("post/amend:unlock")',
  'env.navigate("ws/val:run")',
]) {
  assert(screens.includes(snippet), `Phase 5~8 workbench must include ${snippet}`);
}

for (const fn of [
  "statusIn",
  "renderStageRouteButtons",
  "formatWorkbenchValue",
  "parseManualFieldValue",
  "formSourceLeaf",
  "validationIssueLeaf",
  "efilingIssueLeaf",
  "validationCounts",
]) {
  assert(screens.includes(`function ${fn}`), `${fn} helper must exist`);
}

console.log("frontend phase5_8_workbench.spec.js passed");
