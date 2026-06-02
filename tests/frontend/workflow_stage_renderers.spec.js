const fs = require("fs");
const assert = require("assert");

const screens = fs.readFileSync("frontend/app/screens.js", "utf8");

function objectBlock(name) {
  const match = screens.match(new RegExp(`export const ${name} = Object\\.freeze\\(\\{([\\s\\S]*?)\\n\\}\\);`));
  assert(match, `${name} block is missing`);
  return match[1];
}

function screenRegistrations() {
  return Object.fromEntries(
    [...objectBlock("screenByLeaf").matchAll(/^\s*"([^"]+)": \(env\) => (.+)$/gm)]
      .map((match) => [match[1], match[2]])
  );
}

function contractRegistrations() {
  return Object.fromEntries(
    [...objectBlock("workflowLeafRendererContract").matchAll(/^\s*"([^"]+)": "([^"]+)"/gm)]
      .map((match) => [match[1], match[2]])
  );
}

const expectedGroups = {
  renderWorkStartCustomerPick: ["ws/start:customer-pick"],
  renderWorkStartBusinessYearPick: ["ws/start:by-pick"],
  renderWorkStartSnapshot: ["ws/start:snapshot"],
  renderWorkInfoFinancialStatements: ["ws/info:fs"],
  renderWorkInfoAccountMapping: ["ws/info:mapping"],
  renderWorkInfoAssets: ["ws/info:assets"],
  renderWorkInfoTransactions: ["ws/info:transactions"],
  renderWorkInfoVehicleUsage: ["ws/info:vehicle"],
  renderWorkInfoConsistency: ["ws/info:consistency"],
  ...Object.fromEntries(Array.from({ length: 17 }, (_, index) => [`renderAdjustmentB${index + 1}`, [`ws/adj:B${index + 1}`]])),
  renderFormsForm3: ["ws/form:form3"],
  renderFormsAttachments: ["ws/form:attachments"],
  renderFormsPreview: ["ws/form:preview"],
  renderFormsLinkage: ["ws/form:linkage"],
  renderValidationRun: ["ws/val:run"],
  renderValidationIssues: ["ws/val:issues"],
  renderValidationRules: ["ws/val:rules"],
  renderApprovalRequest: ["ws/appr:request"],
  renderApprovalInbox: ["ws/appr:inbox"],
  renderApprovalRejected: ["ws/appr:rejected"],
  renderPrintPreview: ["ws/print:preview"],
  renderPrintBulk: ["ws/print:bulk"],
  renderPrintHistory: ["ws/print:history"],
  renderEfilingPrecheck: ["ws/file:precheck"],
  renderEfilingGenerate: ["ws/file:generate"],
  renderEfilingSubmit: ["ws/file:submit"],
  renderEfilingDone: ["ws/file:done"],
  renderPostHistoryLeaf: ["post/hist:list"],
  renderPostAmendUnlock: ["post/amend:unlock"],
  renderPostAmendVersion: ["post/amend:version"],
  renderPostAmendDiff: ["post/amend:diff"],
  renderPostAmendResubmit: ["post/amend:resubmit"],
  renderPostCorrection: ["post/correction"],
};

const delegateByRenderer = {
  renderPostHistoryLeaf: "renderPostHistory",
};

const screensByLeaf = screenRegistrations();
const contract = contractRegistrations();
const stageContract = objectBlock("workflowStageContract");

for (const [renderer, keys] of Object.entries(expectedGroups)) {
  assert(screens.includes(`async function ${renderer}(env)`), `${renderer} must exist`);
  if (delegateByRenderer[renderer]) {
    assert(screens.includes(`await ${delegateByRenderer[renderer]}(env);`), `${renderer} must call ${delegateByRenderer[renderer]}`);
    assert(stageContract.includes(`renderer: "${renderer}"`), `workflowStageContract must reference ${renderer}`);
  }
  for (const key of keys) {
    assert.strictEqual(contract[key], renderer, `${key} contract must use ${renderer}`);
    assert(stageContract.includes(`"${key}"`), `workflowStageContract must include ${key}`);
    assert(screensByLeaf[key], `${key} must be registered in screenByLeaf`);
    assert(screensByLeaf[key].includes(`${renderer}(env)`), `${key} must dispatch to ${renderer}`);
    assert(!screensByLeaf[key].includes("renderLeafScreen"), `${key} must not dispatch to generic renderLeafScreen`);
  }
}

assert(screens.includes("async function renderWorkStartLeaf(env)"), "renderWorkStartLeaf compatibility wrapper must exist");
assert(screens.includes('if (activeLeaf === "ws/start:by-pick") return renderWorkStartBusinessYearPick(env);'), "work-start wrapper must dispatch business-year pick");
assert(screens.includes('if (activeLeaf === "ws/start:snapshot") return renderWorkStartSnapshot(env);'), "work-start wrapper must dispatch snapshot");
assert(stageContract.includes('renderer: "renderWorkStartLeaf"'), "workflowStageContract must keep the work-start stage wrapper");
assert(screens.includes("async function renderWorkInfoLeaf(env)"), "renderWorkInfoLeaf compatibility wrapper must exist");
assert(screens.includes('if (activeLeaf === "ws/info:mapping") return renderWorkInfoAccountMapping(env);'), "tax-data wrapper must dispatch account mapping");
assert(screens.includes('if (activeLeaf === "ws/info:assets") return renderWorkInfoAssets(env);'), "tax-data wrapper must dispatch assets");
assert(screens.includes('if (activeLeaf === "ws/info:transactions") return renderWorkInfoTransactions(env);'), "tax-data wrapper must dispatch transactions");
assert(screens.includes('if (activeLeaf === "ws/info:vehicle") return renderWorkInfoVehicleUsage(env);'), "tax-data wrapper must dispatch vehicle usage");
assert(screens.includes('if (activeLeaf === "ws/info:consistency") return renderWorkInfoConsistency(env);'), "tax-data wrapper must dispatch consistency");
assert(stageContract.includes('renderer: "renderWorkInfoLeaf"'), "workflowStageContract must keep the tax-data stage wrapper");
assert(screens.includes("async function renderAdjustmentLeaf(env)"), "renderAdjustmentLeaf compatibility wrapper must exist");
assert(screens.includes('if (moduleCode === "B2") return renderAdjustmentB2(env);'), "adjustment wrapper must dispatch B2");
assert(screens.includes('if (moduleCode === "B17") return renderAdjustmentB17(env);'), "adjustment wrapper must dispatch B17");
assert(stageContract.includes('renderer: "renderAdjustmentLeaf"'), "workflowStageContract must keep the adjustment stage wrapper");
assert(screens.includes("async function renderFormsLeaf(env)"), "renderFormsLeaf compatibility wrapper must exist");
assert(screens.includes('if (activeLeaf === "ws/form:attachments") return renderFormsAttachments(env);'), "forms wrapper must dispatch attachments");
assert(screens.includes('if (activeLeaf === "ws/form:preview") return renderFormsPreview(env);'), "forms wrapper must dispatch preview");
assert(screens.includes('if (activeLeaf === "ws/form:linkage") return renderFormsLinkage(env);'), "forms wrapper must dispatch linkage");
assert(stageContract.includes('renderer: "renderFormsLeaf"'), "workflowStageContract must keep the forms stage wrapper");
assert(screens.includes("async function renderValidationLeaf(env)"), "renderValidationLeaf compatibility wrapper must exist");
assert(screens.includes('if (activeLeaf === "ws/val:issues") return renderValidationIssues(env);'), "validation wrapper must dispatch issues");
assert(screens.includes('if (activeLeaf === "ws/val:rules") return renderValidationRules(env);'), "validation wrapper must dispatch rules");
assert(stageContract.includes('renderer: "renderValidationLeaf"'), "workflowStageContract must keep the validation stage wrapper");
assert(screens.includes("async function renderApprovalLeaf(env)"), "renderApprovalLeaf compatibility wrapper must exist");
assert(screens.includes('if (activeLeaf === "ws/appr:inbox") return renderApprovalInbox(env);'), "approval wrapper must dispatch inbox");
assert(screens.includes('if (activeLeaf === "ws/appr:rejected") return renderApprovalRejected(env);'), "approval wrapper must dispatch rejected");
assert(stageContract.includes('renderer: "renderApprovalLeaf"'), "workflowStageContract must keep the approval stage wrapper");
assert(screens.includes("async function renderPrintLeaf(env)"), "renderPrintLeaf compatibility wrapper must exist");
assert(screens.includes('if (activeLeaf === "ws/print:bulk") return renderPrintBulk(env);'), "print wrapper must dispatch bulk");
assert(screens.includes('if (activeLeaf === "ws/print:history") return renderPrintHistory(env);'), "print wrapper must dispatch history");
assert(stageContract.includes('renderer: "renderPrintLeaf"'), "workflowStageContract must keep the print stage wrapper");
assert(screens.includes("async function renderEfilingLeaf(env)"), "renderEfilingLeaf compatibility wrapper must exist");
assert(screens.includes('if (activeLeaf === "ws/file:generate") return renderEfilingGenerate(env);'), "efiling wrapper must dispatch generate");
assert(screens.includes('if (activeLeaf === "ws/file:submit") return renderEfilingSubmit(env);'), "efiling wrapper must dispatch submit");
assert(screens.includes('if (activeLeaf === "ws/file:done") return renderEfilingDone(env);'), "efiling wrapper must dispatch done");
assert(stageContract.includes('renderer: "renderEfilingLeaf"'), "workflowStageContract must keep the efiling stage wrapper");
assert(screens.includes("async function renderPostAmendLeaf(env)"), "renderPostAmendLeaf compatibility wrapper must exist");
assert(screens.includes('if (activeLeaf === "post/amend:version") return renderPostAmendVersion(env);'), "post-amend wrapper must dispatch version");
assert(screens.includes('if (activeLeaf === "post/amend:diff") return renderPostAmendDiff(env);'), "post-amend wrapper must dispatch diff");
assert(screens.includes('if (activeLeaf === "post/amend:resubmit") return renderPostAmendResubmit(env);'), "post-amend wrapper must dispatch resubmit");
assert(screens.includes('if (activeLeaf === "post/correction") return renderPostCorrection(env);'), "post-amend wrapper must dispatch correction");
assert(stageContract.includes('renderer: "renderPostAmendLeaf"'), "workflowStageContract must keep the post-amend stage wrapper");

assert(!/generic:\s*true/.test(stageContract), "core workflow stage contract must not mark stages generic");

for (const key of Object.keys(contract)) {
  assert(
    Object.values(expectedGroups).some((keys) => keys.includes(key)),
    `${key} contract entry must belong to a known workflow stage`
  );
}

console.log("frontend workflow_stage_renderers.spec.js passed");
